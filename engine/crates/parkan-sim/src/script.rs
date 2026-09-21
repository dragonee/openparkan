//! A behaviour script run the way `ai.dll`'s interpreter runs it: the one each clan's
//! SuperAI embeds. See `docs/15-behaviour.md`, "How a handler runs", "Blocks" and
//! "The jumps", and `docs/34-progression.md` for when the engine runs which handler.
//!
//! The interpreter owns the script's variables, so they keep their values from one
//! run to the next. A call reaches the engine through [`Host`]: function *n* of the
//! 73-slot table, handed its node's operands to fetch and write as it likes.

use parkan_formats::FormatError;
use parkan_formats::scr::{self, Node, Script, Variable};

/// How many slots the function table has; a call's `head[0]` indexes it.
pub const FUNCTION_TABLE: usize = 73;
/// The node index a switch leaves (`ai.dll:0x10012242`): one step on it is
/// [`RERUN`], which ends the handler and runs the one the switch named.
const SWITCHED: i32 = 0x00FF_FFFE;
/// A handler that stops on this node index runs again, from node 0, as whichever
/// handler is current (`0x10011fff`, `0x10011f2f`).
const RERUN: i32 = 0x00FF_FFFF;
/// Nodes one [`Interpreter::run`] steps through before it gives up. The game has no
/// such guard: a script that loops for ever hangs it. No shipped script comes near.
pub const MAX_STEPS: usize = 1_000_000;

/// A variable's type tag (`ai.dll:0x10037688`): 1 `int`, 2 `BOOL`, 3 `float`,
/// 5 `DWORD`. `void` and `char*` are declared by no shipped script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    Float,
    Dword,
}

impl Type {
    pub fn from_name(name: &str) -> Option<Type> {
        Some(match name {
            "int" => Type::Int,
            "BOOL" => Type::Bool,
            "float" => Type::Float,
            "DWORD" => Type::Dword,
            _ => return None,
        })
    }
}

/// A value truncated toward zero to 64 bits, low word kept (`_ftol`, `0x1001df70`);
/// out of range it is the x87's indefinite integer, whose low word is 0.
fn truncate(f: f64) -> u32 {
    // 2^63: the first float no i64 holds.
    if f.is_finite() && f.abs() < 9_223_372_036_854_775_808.0 { f.trunc() as i64 as u32 } else { 0 }
}

/// A float truncated the same way, which is how the getters and setters read one.
fn ftol(f: f32) -> u32 {
    truncate(f64::from(f))
}

/// One variable: its type and its four bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Var {
    pub ty: Type,
    pub bits: u32,
}

impl Var {
    /// Its value as a `DWORD` (`0x10013570`): a float truncated, a `BOOL` 0 or 1.
    pub fn dword(&self) -> u32 {
        match self.ty {
            Type::Int | Type::Dword => self.bits,
            Type::Bool => u32::from(self.bits != 0),
            Type::Float => ftol(f32::from_bits(self.bits)),
        }
    }

    /// Its value as a float (`0x10013190`): an `int` signed, a `DWORD` unsigned.
    pub fn float(&self) -> f32 {
        match self.ty {
            Type::Int => self.bits as i32 as f32,
            Type::Bool => f32::from(u8::from(self.bits != 0)),
            Type::Float => f32::from_bits(self.bits),
            Type::Dword => self.bits as f32,
        }
    }

    /// Set from a `DWORD` (`0x10013770`, `0x10012fe0`).
    pub fn set_dword(&mut self, w: u32) {
        self.bits = match self.ty {
            Type::Int | Type::Dword => w,
            Type::Bool => u32::from(w != 0),
            Type::Float => (w as f32).to_bits(),
        };
    }

    /// Set from a float (`0x10013650`, `0x10012c00`): an integer takes it truncated,
    /// a `BOOL` whether it compares unequal to 0.
    pub fn set_float(&mut self, f: f32) {
        self.bits = match self.ty {
            Type::Int | Type::Dword => ftol(f),
            Type::Bool => u32::from(!(f == 0.0 || f.is_nan())),
            Type::Float => f.to_bits(),
        };
    }

    /// A declaration's starting value.
    ///
    /// STAND-IN: docs/15-behaviour.md#the-vocabulary-varsetvar -- how the loader turns a
    /// `DefValue` into a value is not read. An integer type takes hex after `0x`, a
    /// decimal, or a float truncated; a float takes its decimal; an empty or unreadable
    /// default is 0. Every shipped default reads either way.
    pub fn declared(ty: Type, default: &str) -> Var {
        let text = default.trim();
        let mut v = Var { ty, bits: 0 };
        if ty == Type::Float {
            v.set_float(text.parse().unwrap_or(0.0));
        } else if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            v.set_dword(u32::from_str_radix(hex, 16).unwrap_or(0));
        } else if let Ok(n) = text.parse::<i64>() {
            v.set_dword(n as u32);
        } else {
            v.set_float(text.parse().unwrap_or(0.0));
        }
        v
    }
}

/// A call's operands, which the function fetches and writes itself.
pub struct Args<'a> {
    operands: &'a [i32],
    vars: &'a mut [Var],
    names: &'a [String],
}

impl Args<'_> {
    pub fn len(&self) -> usize {
        self.operands.len()
    }

    /// The name of the variable operand `i` is. Function 2 reads it: a raise looks its
    /// handler pair up by the name of the variable its first operand names
    /// (`ai.dll:0x1000f6d0`), not by the code's value.
    pub fn name(&self, i: usize) -> Option<&str> {
        let at = usize::try_from(*self.operands.get(i)?).ok()?;
        self.names.get(at).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.operands.is_empty()
    }

    /// Operand `i` as written: a variable index.
    pub fn index(&self, i: usize) -> Option<i32> {
        self.operands.get(i).copied()
    }

    /// The variable operand `i` names.
    pub fn var(&self, i: usize) -> Option<Var> {
        let at = usize::try_from(*self.operands.get(i)?).ok()?;
        self.vars.get(at).copied()
    }

    fn var_mut(&mut self, i: usize) -> Option<&mut Var> {
        let at = usize::try_from(*self.operands.get(i)?).ok()?;
        self.vars.get_mut(at)
    }

    /// Operand `i` as a `DWORD`, or 0 where there is none.
    pub fn dword(&self, i: usize) -> u32 {
        self.var(i).map_or(0, |v| v.dword())
    }

    /// Operand `i` as a float, or 0 where there is none.
    pub fn float(&self, i: usize) -> f32 {
        self.var(i).map_or(0.0, |v| v.float())
    }

    /// Write operand `i`, as functions 18, 19, 25 and the others marked "out" do.
    pub fn set_dword(&mut self, i: usize, w: u32) {
        if let Some(v) = self.var_mut(i) {
            v.set_dword(w);
        }
    }

    pub fn set_float(&mut self, i: usize, f: f32) {
        if let Some(v) = self.var_mut(i) {
            v.set_float(f);
        }
    }
}

/// What the script's calls reach: the engine below the SuperAI.
pub trait Host {
    /// Run function `function` of the table on `args`, and return what it leaves in
    /// the result slot, which is cleared to 0 before every call (`0x100122c2`). A
    /// function answering a float leaves its bits: return `value.to_bits()`.
    fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32;
}

/// A formula, parsed.
#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Number(f64),
    Var(usize),
    Unary(char, Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

/// A binary operator's priority, by the evaluator's own operator table
/// (`ai.dll:0x10047c70`, 13 records the count at `0x10047c68` gives): `+ - |` 1,
/// `* / &` 2, `^` 3.
fn binary_priority(op: char) -> Option<u8> {
    match op {
        '+' | '-' | '|' => Some(1),
        '*' | '/' | '&' => Some(2),
        '^' => Some(3),
        _ => None,
    }
}

/// The unary operators and their priorities: `-` and `!` 1; the one-letter `N`, `S`,
/// `B` and `A` 100.
fn unary_priority(op: char) -> Option<u8> {
    match op {
        '-' | '!' => Some(1),
        'N' | 'S' | 'B' | 'A' => Some(100),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Name(String),
    Op(char),
    Open,
    Close,
}

fn tokens(text: &str) -> Option<Vec<Token>> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            out.push(Token::Number(chars[start..i].iter().collect::<String>().parse().ok()?));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else if c == '(' {
            out.push(Token::Open);
            i += 1;
        } else if c == ')' {
            out.push(Token::Close);
            i += 1;
        } else if "+-*/^&|!".contains(c) {
            out.push(Token::Op(c));
            i += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    at: usize,
    names: &'a [String],
}

impl Parser<'_> {
    fn expression(&mut self, least: u8) -> Option<Expr> {
        let mut left = self.operand()?;
        while let Some(Token::Op(op)) = self.tokens.get(self.at).cloned() {
            let Some(priority) = binary_priority(op).filter(|&p| p >= least) else { break };
            self.at += 1;
            let right = self.expression(priority + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn operand(&mut self) -> Option<Expr> {
        let token = self.tokens.get(self.at).cloned()?;
        self.at += 1;
        match token {
            Token::Number(n) => Some(Expr::Number(n)),
            Token::Open => {
                let inner = self.expression(0)?;
                (self.tokens.get(self.at) == Some(&Token::Close)).then(|| self.at += 1)?;
                Some(inner)
            }
            Token::Op(op) => {
                let priority = unary_priority(op)?;
                Some(Expr::Unary(op, Box::new(self.expression(priority + 1)?)))
            }
            Token::Name(name) if name.len() == 1 && unary_priority(name.chars().next()?).is_some() => {
                let op = name.chars().next()?;
                Some(Expr::Unary(op, Box::new(self.expression(unary_priority(op)? + 1)?)))
            }
            // The tokeniser knows these two without the symbol table (`0x10015973`,
            // `0x100159a6`); `varset.var` declares them 1 and 0 as well.
            Token::Name(name) if name == "TRUE" => Some(Expr::Number(1.0)),
            Token::Name(name) if name == "FALSE" => Some(Expr::Number(0.0)),
            Token::Name(name) => self.names.iter().position(|n| *n == name).map(Expr::Var),
            Token::Close => None,
        }
    }
}

/// STAND-IN: docs/15-behaviour.md#the-fml-operators--read-and-measured -- the thirteen
/// arms of the evaluator are read and [`evaluate`] follows them, but the parser above
/// them is not: the tokeniser hands the four flagged operators (`N`, `S`, `B`, `A`,
/// priority 100) to a branch of its own rather than to the precedence stack
/// (`0x1001543b`), and what that branch does differently is not followed. Here
/// everything parses off the table's
/// priorities, left to right among equals, and evaluates in doubles where the engine
/// keeps its stack in 4-byte floats. A name that is neither an operator, `TRUE`,
/// `FALSE` nor a variable is refused. The shipped formulas use numbers, variables,
/// `+ - *` and brackets alone, so only those three arms are exercised in play.
fn parse_formula(text: &str, names: &[String]) -> Option<Expr> {
    let mut parser = Parser { tokens: tokens(text)?, at: 0, names };
    let expr = parser.expression(0)?;
    (parser.at == parser.tokens.len()).then_some(expr)
}

/// What the three arms that fold a value down to a flag test (`0x10015e53`,
/// `0x10015e82`, `0x10015eb1`, all *read*): the value through `_ftol` first, so
/// anything that truncates to zero -- 0.5, -0.9 -- is false.
fn truthy(v: f64) -> bool {
    truncate(v) != 0
}

fn flag(held: bool) -> f64 {
    if held { 1.0 } else { 0.0 }
}

/// One expression, arm for arm as the evaluator's 13-entry jump table runs them
/// (`ai.dll:0x10016064`, *read*). Every comparison below is the x87's, so a NaN is
/// unordered and falls to the arm's else.
fn evaluate(expr: &Expr, vars: &[Var]) -> f64 {
    match expr {
        Expr::Number(n) => *n,
        Expr::Var(i) => vars.get(*i).map_or(0.0, |v| f64::from(v.float())),
        Expr::Unary(op, a) => {
            let a = evaluate(a, vars);
            match op {
                // 7 Sign change: `fchs`.
                '-' => -a,
                // 8 Not.
                '!' => flag(!truthy(a)),
                // 9 Normalisator: 0 below -1, the ramp (x + 1)/2 between, 1 above 1.
                'N' => {
                    if a > 1.0 {
                        1.0
                    } else if a >= -1.0 {
                        (a + 1.0) * 0.5
                    } else {
                        0.0
                    }
                }
                // 10 Significator: the value itself, but only where it is positive.
                'S' => {
                    if a > 0.0 {
                        a
                    } else {
                        0.0
                    }
                }
                // 11 Booleanisator: positive, not merely non-zero.
                'B' => flag(a > 0.0),
                // 12 Absolute.
                _ => a.abs(),
            }
        }
        Expr::Binary(op, a, b) => {
            let (a, b) = (evaluate(a, vars), evaluate(b, vars));
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                // 3 Division tests the divisor against 0 first and answers 0 rather
                // than dividing (`0x10015e1f`), so no formula can raise an exception.
                '/' => {
                    if b == 0.0 || b.is_nan() {
                        0.0
                    } else {
                        a / b
                    }
                }
                // 4 Power, through the CRT's `pow` (`0x10021030`).
                '^' => a.powf(b),
                // 5 And, 6 Or.
                '&' => flag(truthy(a) && truthy(b)),
                _ => flag(truthy(a) || truthy(b)),
            }
        }
    }
}

/// How an `if` compares (`0x1001203f`): 0 `<`, 1 `==`, 2 `>`, 3 `<=`, 4 `>=`, 5 `!=`.
/// Floats compare as the x87 does, so an unordered pair passes `<`, `==` and `<=`.
fn compare_floats(relation: i32, a: f32, b: f32) -> bool {
    let unordered = a.is_nan() || b.is_nan();
    match relation {
        0 => unordered || a < b,
        1 => unordered || a == b,
        2 => !unordered && a > b,
        3 => unordered || a <= b,
        4 => !unordered && a >= b,
        5 => !unordered && a != b,
        _ => false,
    }
}

fn compare_dwords(relation: i32, a: u32, b: u32) -> bool {
    match relation {
        0 => a < b,
        1 => a == b,
        2 => a > b,
        3 => a <= b,
        4 => a >= b,
        5 => a != b,
        _ => false,
    }
}

/// What a node leaves the stepping loop to do.
enum Flow {
    Next,
    Stop,
    /// Leave the running node index at this, for the step to move on from.
    Leave(i32),
}

/// A script loaded into its own interpreter: the script, its formulas, and a copy of
/// every `varset.var` variable at its declared value.
#[derive(Clone, Debug)]
pub struct Interpreter {
    pub script: Script,
    names: Vec<String>,
    vars: Vec<Var>,
    formulas: Vec<Expr>,
    /// The condition bytes of the open blocks, innermost last (`+0x4c`).
    conditions: Vec<bool>,
    /// The running handler (`+0x40`).
    running: i32,
}

impl Interpreter {
    /// Load `script` with its `.fml` expressions against the variable table.
    pub fn new(script: Script, formulas: &[String], table: &[Variable]) -> Result<Self, FormatError> {
        let names: Vec<String> = table.iter().map(|v| v.name.clone()).collect();
        let vars = table
            .iter()
            .map(|v| {
                let ty = Type::from_name(&v.type_name).ok_or_else(|| {
                    FormatError::invalid(scr::VARSET, format!("{} has type {:?}", v.name, v.type_name))
                })?;
                Ok(Var::declared(ty, &v.default))
            })
            .collect::<Result<Vec<_>, FormatError>>()?;
        let formulas = formulas
            .iter()
            .enumerate()
            .map(|(i, f)| {
                parse_formula(f, &names)
                    .ok_or_else(|| FormatError::invalid("fml", format!("formula {i} does not parse: {f:?}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { script, names, vars, formulas, conditions: Vec::new(), running: 0 })
    }

    /// The handler of that name.
    pub fn handler(&self, name: &str) -> Option<usize> {
        self.script.handler(name)
    }

    /// The variable table's index of `name`.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// The variable called `name`, as it stands.
    pub fn get(&self, name: &str) -> Option<Var> {
        self.index_of(name).and_then(|i| self.vars.get(i)).copied()
    }

    /// `name` as a `DWORD`: how a caller looks up a constant such as `MESSAGE_INFO`.
    pub fn dword(&self, name: &str) -> Option<u32> {
        self.get(name).map(|v| v.dword())
    }

    pub fn float(&self, name: &str) -> Option<f32> {
        self.get(name).map(|v| v.float())
    }

    /// Write a variable the engine owns, such as `fDifficulty` or `dCurrentSender`.
    pub fn set_dword(&mut self, name: &str, w: u32) {
        if let Some(v) = self.index_of(name).and_then(|i| self.vars.get_mut(i)) {
            v.set_dword(w);
        }
    }

    pub fn set_float(&mut self, name: &str, f: f32) {
        if let Some(v) = self.index_of(name).and_then(|i| self.vars.get_mut(i)) {
            v.set_float(f);
        }
    }

    /// Run handler `handler` (`0x10011f10`) and every handler its switches pass to.
    ///
    /// STAND-IN: docs/15-behaviour.md#how-a-handler-runs -- a node naming a variable,
    /// operand, formula or handler that does not exist has the executor read past its
    /// tables; here such a node does nothing, and a switch to no handler stops the run.
    pub fn run(&mut self, handler: usize, host: &mut dyn Host) {
        let Ok(start) = i32::try_from(handler) else { return };
        self.running = start;
        let mut steps = 0;
        while steps < MAX_STEPS && self.step_handler(host, &mut steps) {}
    }

    /// Run a handler by name; false when the script has none.
    pub fn run_named(&mut self, name: &str, host: &mut dyn Host) -> bool {
        let Some(h) = self.handler(name) else { return false };
        self.run(h, host);
        true
    }

    /// The running handler from node 0, with no block open (`0x10011fb0`). True when it
    /// ended on [`RERUN`], to run again as whichever handler is current.
    fn step_handler(&mut self, host: &mut dyn Host, steps: &mut usize) -> bool {
        self.conditions.clear();
        let Some(h) = usize::try_from(self.running).ok().filter(|&h| h < self.script.handlers.len()) else {
            return false;
        };
        let count = i32::try_from(self.script.handlers[h].nodes.len()).unwrap_or(i32::MAX);
        let mut node: i32 = 0;
        while node < count {
            if *steps >= MAX_STEPS {
                return false;
            }
            *steps += 1;
            let n = self.script.handlers[h].nodes[node as usize].clone();
            match self.execute(&n, host) {
                Flow::Stop => return false,
                Flow::Leave(at) => node = at,
                Flow::Next => {}
            }
            node = node.wrapping_add(1);
        }
        node == RERUN
    }

    /// A statement, jump or return acts only while the innermost block holds. Four of
    /// the executor's eight arms open with the same condition test -- the statement,
    /// goto, switch and return, table entries 0, 4, 5 and 6 (*read*). The constant's,
    /// entry 7, does not; nor does the label's, entry 3, which has nothing to guard.
    fn blocked(&self) -> bool {
        self.conditions.last() == Some(&false)
    }

    fn var(&self, index: i32) -> Option<Var> {
        usize::try_from(index).ok().and_then(|i| self.vars.get(i)).copied()
    }

    fn var_mut(&mut self, index: i32) -> Option<&mut Var> {
        usize::try_from(index).ok().and_then(|i| self.vars.get_mut(i))
    }

    /// One node (`0x10012020`), by its kind.
    fn execute(&mut self, n: &Node, host: &mut dyn Host) -> Flow {
        match n.kind() {
            scr::IF => {
                let holds = match (
                    n.operands.first().and_then(|&i| self.var(i)),
                    n.operands.get(1).and_then(|&i| self.var(i)),
                ) {
                    (Some(a), Some(b)) if a.ty == Type::Dword => {
                        compare_dwords(n.opcode, a.dword(), b.dword())
                    }
                    (Some(a), Some(b)) => compare_floats(n.opcode, a.float(), b.float()),
                    _ => false,
                };
                // The comparison runs even inside a false block -- it can read a
                // variable a suppressed constant clobbered -- but the byte it pushes is
                // forced to 0 there, so it cannot open a live one (`0x1001219f`).
                let enclosing = self.conditions.last().copied().unwrap_or(true);
                self.conditions.push(holds && enclosing);
                Flow::Next
            }
            // A spare end clamps at zero rather than going negative (`0x100121cb`).
            scr::END => {
                self.conditions.pop();
                Flow::Next
            }
            // Nothing: the arm pops its frame and returns (`0x10012376`). Five of the
            // corpus's 57 labels are reached only by falling into one.
            scr::LABEL => Flow::Next,
            scr::GOTO | scr::SWITCH | scr::RETURN if self.blocked() => Flow::Next,
            // A taken goto zeroes the open-block count before it sets the node
            // (`0x1001226e`), so a label inside an `if` lands at depth 0 and the `end`
            // behind it decrements to -1 and is clamped back.
            scr::GOTO => match n.operands.first() {
                Some(&target) => {
                    self.conditions.clear();
                    Flow::Leave(target.wrapping_sub(1))
                }
                None => Flow::Next,
            },
            scr::SWITCH => match n.operands.first() {
                Some(&handler) => {
                    self.conditions.clear();
                    self.running = handler;
                    Flow::Leave(SWITCHED)
                }
                None => Flow::Next,
            },
            scr::RETURN => Flow::Stop,
            scr::CONST => {
                // Written whatever the condition (`0x100121e2`): the one arm of the
                // eight that goes straight to the destination without testing the
                // condition byte. As a DWORD into a DWORD, and its bits through the
                // float setter into anything else.
                let word = n.source() as u32;
                if let Some(v) = self.var_mut(n.destination()) {
                    if v.ty == Type::Dword {
                        v.set_dword(word);
                    } else {
                        v.set_float(f32::from_bits(word));
                    }
                }
                Flow::Next
            }
            scr::STATEMENT if self.blocked() => Flow::Next,
            // A kind outside -1..6 runs as a statement with no condition test (`0x100122b5`).
            _ => {
                self.statement(n, host);
                Flow::Next
            }
        }
    }

    /// A call, a copy or a formula (`0x100122b5`).
    fn statement(&mut self, n: &Node, host: &mut dyn Host) {
        if n.calls() {
            let result = host.call(
                n.function(),
                &mut Args { operands: &n.operands, vars: &mut self.vars, names: &self.names },
            );
            // A float destination takes the result slot's bits as a float, any other
            // type takes it as a DWORD (`0x100122ea`).
            if let Some(v) = self.var_mut(n.destination()) {
                if v.ty == Type::Float {
                    v.set_float(f32::from_bits(result));
                } else {
                    v.set_dword(result);
                }
            }
        } else if n.source() != scr::NULL {
            // The variable is copied whole, its type with it (`0x10013a80`).
            if let (Some(source), Some(_)) = (self.var(n.source()), self.var(n.destination())) {
                *self.var_mut(n.destination()).expect("checked") = source;
            }
        } else if let Some(expr) = usize::try_from(n.trailer).ok().and_then(|i| self.formulas.get(i)) {
            let value = evaluate(expr, &self.vars) as f32;
            if let Some(v) = self.var_mut(n.destination()) {
                v.set_float(value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::scr::{Handler, NO_RELATION};

    fn table() -> Vec<Variable> {
        let v = |ty: &str, name: &str, default: &str| Variable {
            kind: "VAR".into(),
            type_name: ty.into(),
            name: name.into(),
            default: default.into(),
        };
        vec![
            v("DWORD", "d0", "0"),
            v("DWORD", "d1", "1"),
            v("DWORD", "ERROR", "0xffffffff"),
            v("float", "fDifficulty", "0.5"),
            v("DWORD", "dA", "0"),
            v("DWORD", "dB", "0"),
            v("float", "fT", "0"),
            v("DWORD", "MESSAGE_INFO", "0x1"),
        ]
    }

    const D0: i32 = 0;
    const D1: i32 = 1;
    const ERROR: i32 = 2;
    const F_DIFFICULTY: i32 = 3;
    const DA: i32 = 4;
    const DB: i32 = 5;
    const FT: i32 = 6;

    fn node(head: [i32; 4], opcode: i32, operands: &[i32], trailer: i32) -> Node {
        Node { head, opcode, operands: operands.to_vec(), trailer }
    }
    fn call(function: i32, dest: i32, operands: &[i32]) -> Node {
        node([function, dest, -1, scr::STATEMENT], NO_RELATION, operands, -1)
    }
    fn if_(relation: i32, a: i32, b: i32) -> Node {
        node([-1, -1, -1, scr::IF], relation, &[a, b], -1)
    }
    fn end() -> Node {
        node([-1, -1, -1, scr::END], NO_RELATION, &[], -1)
    }
    fn constant(dest: i32, value: i32) -> Node {
        node([-1, dest, value, scr::CONST], NO_RELATION, &[], -1)
    }
    fn formula(dest: i32, index: i32) -> Node {
        node([-1, dest, -1, scr::STATEMENT], NO_RELATION, &[], index)
    }
    fn jump(kind: i32, target: i32) -> Node {
        node([-1, -1, -1, kind], NO_RELATION, &[target], -1)
    }
    /// A label is bare: `head = (-1, -1, -1, 2)`, no operands, no trailer, on all 57.
    fn label() -> Node {
        node([-1, -1, -1, scr::LABEL], NO_RELATION, &[], -1)
    }

    fn interpreter(handlers: Vec<Vec<Node>>, formulas: &[&str]) -> Interpreter {
        let handlers = handlers
            .into_iter()
            .enumerate()
            .map(|(i, nodes)| Handler { name: format!("H{i}"), index: i as i32, nodes })
            .collect();
        let formulas: Vec<String> = formulas.iter().map(|f| f.to_string()).collect();
        Interpreter::new(Script { magic: scr::MAGIC, handlers }, &formulas, &table()).unwrap()
    }

    /// Records every call; function 1 answers its first operand plus one, function 2 a
    /// float, function 19 writes 7 into its first operand.
    #[derive(Default)]
    struct Calls(Vec<(i32, Vec<u32>)>);

    impl Host for Calls {
        fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32 {
            self.0.push((function, (0..args.len()).map(|i| args.dword(i)).collect()));
            match function {
                1 => args.dword(0) + 1,
                2 => 2.75_f32.to_bits(),
                19 => {
                    args.set_dword(0, 7);
                    0
                }
                _ => 0,
            }
        }
    }

    #[test]
    fn a_block_runs_only_while_every_enclosing_condition_holds() {
        let mut i = interpreter(
            vec![vec![
                if_(1, D0, D1), // false
                if_(1, D0, D0), // true, but inside a false block
                call(30, -1, &[D1]),
                end(),
                end(),
                if_(0, D0, D1), // true
                call(31, -1, &[D0]),
                end(),
            ]],
            &[],
        );
        let mut calls = Calls::default();
        i.run(0, &mut calls);
        assert_eq!(calls.0, vec![(31, vec![0])]);
    }

    #[test]
    fn a_constant_lands_inside_a_false_block_and_a_spare_end_is_harmless() {
        let mut i =
            interpreter(vec![vec![end(), if_(1, D0, D1), constant(DA, 5), call(1, DB, &[D1]), end()]], &[]);
        i.run(0, &mut Calls::default());
        assert_eq!(i.dword("dA"), Some(5));
        assert_eq!(i.dword("dB"), Some(0));
    }

    #[test]
    fn a_dword_comparison_is_unsigned_and_a_float_one_is_not() {
        let mut i = interpreter(
            vec![vec![
                if_(2, ERROR, D1),
                constant(DA, 1),
                call(30, -1, &[]),
                end(),
                if_(0, FT, F_DIFFICULTY),
                call(31, -1, &[]),
                end(),
            ]],
            &[],
        );
        let mut calls = Calls::default();
        i.run(0, &mut calls);
        assert_eq!(calls.0.iter().map(|c| c.0).collect::<Vec<_>>(), [30, 31]);
    }

    #[test]
    fn a_result_is_written_by_the_destinations_type_and_operands_can_be_written() {
        let mut i = interpreter(
            vec![vec![call(1, DA, &[D1]), call(2, FT, &[]), call(2, DB, &[]), call(19, -1, &[DA])]],
            &[],
        );
        i.run(0, &mut Calls::default());
        assert_eq!(i.float("fT"), Some(2.75));
        // A float's bits land in a DWORD as they are.
        assert_eq!(i.dword("dB"), Some(2.75_f32.to_bits()));
        assert_eq!(i.dword("dA"), Some(7));
    }

    #[test]
    fn a_copy_takes_the_sources_type_and_a_formula_truncates_into_a_dword() {
        let copy = node([-1, DA, F_DIFFICULTY, scr::STATEMENT], NO_RELATION, &[], -1);
        let mut i = interpreter(
            vec![vec![copy, formula(DB, 0), formula(FT, 1), formula(FT, 2)]],
            &["20 + 55*fDifficulty", "( 2 - 3 )", "fT*2 - 1"],
        );
        i.run(0, &mut Calls::default());
        assert_eq!(i.get("dA"), Some(Var { ty: Type::Float, bits: 0.5_f32.to_bits() }));
        assert_eq!(i.dword("dB"), Some(47));
        assert_eq!(i.float("fT"), Some(-3.0));
    }

    #[test]
    fn goto_switch_and_return_move_the_run() {
        let mut i = interpreter(
            vec![
                vec![
                    jump(scr::GOTO, 2),
                    call(30, -1, &[]),
                    call(31, -1, &[]),
                    jump(scr::SWITCH, 1),
                    call(32, -1, &[]),
                ],
                vec![
                    call(33, -1, &[]),
                    if_(1, D0, D1),
                    jump(scr::RETURN, 0),
                    end(),
                    jump(scr::RETURN, 0),
                    call(34, -1, &[]),
                ],
            ],
            &[],
        );
        let mut calls = Calls::default();
        i.run(0, &mut calls);
        assert_eq!(calls.0.iter().map(|c| c.0).collect::<Vec<_>>(), [31, 33]);
    }

    #[test]
    fn variables_keep_their_values_from_one_run_to_the_next() {
        let mut i = interpreter(vec![vec![call(1, DA, &[DA])]], &[]);
        for _ in 0..3 {
            i.run(0, &mut Calls::default());
        }
        assert_eq!(i.dword("dA"), Some(3));
        assert_eq!(i.dword("MESSAGE_INFO"), Some(1));
    }

    #[test]
    fn a_goto_that_loops_is_cut_off() {
        let mut i = interpreter(vec![vec![jump(scr::GOTO, 0)]], &[]);
        i.run(0, &mut Calls::default());
    }

    #[test]
    fn a_formula_with_an_unknown_name_is_refused() {
        let script = Script { magic: scr::MAGIC, handlers: Vec::new() };
        assert!(Interpreter::new(script, &["nowhere + 1".to_string()], &table()).is_err());
    }

    /// `c5m1p`'s `PBM_BASE_DEFENCE_Start`: the whole body is one block, and the goto at
    /// node 3 lands on the label at node 9 -- which is inside it. The goto zeroes the
    /// open-block count, so the `end` behind the label clamps rather than going
    /// negative, and the next handler starts clean.
    #[test]
    fn a_goto_to_a_label_inside_a_block_leaves_no_block_open() {
        let mut i = interpreter(
            vec![vec![
                if_(1, D0, D0), // true: the handler's whole body
                jump(scr::GOTO, 3),
                call(30, -1, &[]), // skipped over
                label(),
                end(), // clamped: the goto already closed the block
                call(31, -1, &[]),
            ]],
            &[],
        );
        let mut calls = Calls::default();
        i.run(0, &mut calls);
        assert_eq!(calls.0.iter().map(|c| c.0).collect::<Vec<_>>(), [31]);
    }

    /// The five unaimed labels are `PBM_BUILDING_PROTECT_Continue`'s last node: reached
    /// by falling in from the node before, and doing nothing when it is.
    #[test]
    fn a_label_fallen_into_does_nothing() {
        let mut i = interpreter(vec![vec![call(30, -1, &[]), label()], vec![call(31, -1, &[])]], &[]);
        let mut calls = Calls::default();
        i.run(0, &mut calls);
        assert_eq!(calls.0.iter().map(|c| c.0).collect::<Vec<_>>(), [30]);
    }

    /// The thirteen operators, as their arms compute them. Ten are never reached by a
    /// shipped formula, so only `+`, `-` and `*` are exercised in play.
    #[test]
    fn the_operators_are_the_evaluators() {
        let names: Vec<String> = table().iter().map(|v| v.name.clone()).collect();
        let value = |text: &str| evaluate(&parse_formula(text, &names).expect("parses"), &[]);
        assert_eq!(value("3 + 4"), 7.0);
        assert_eq!(value("3 - 4"), -1.0);
        assert_eq!(value("3 * 4"), 12.0);
        assert_eq!(value("3 / 4"), 0.75);
        assert_eq!(value("2 ^ 10"), 1024.0);
        // Division tests the divisor first and answers 0 rather than dividing.
        assert_eq!(value("3 / 0"), 0.0);
        assert_eq!(value("0 / 0"), 0.0);
        // And, Or and Not truncate first, so a fraction is false.
        assert_eq!(value("0.5 & 1"), 0.0);
        assert_eq!(value("2 & 1"), 1.0);
        assert_eq!(value("0.5 | 0.5"), 0.0);
        assert_eq!(value("0.5 | 1"), 1.0);
        assert_eq!(value("!0.5"), 1.0);
        assert_eq!(value("!1"), 0.0);
        // Sign change, and the four flagged ones.
        assert_eq!(value("0 - -4"), 4.0);
        assert_eq!(value("N 2"), 1.0);
        assert_eq!(value("N 0"), 0.5);
        assert_eq!(value("N(0 - 2)"), 0.0);
        assert_eq!(value("S 3"), 3.0);
        assert_eq!(value("S(0 - 3)"), 0.0);
        assert_eq!(value("B 3"), 1.0);
        assert_eq!(value("B(0 - 3)"), 0.0);
        assert_eq!(value("A(0 - 3)"), 3.0);
        // The tokeniser's own two literals.
        assert_eq!(value("TRUE + FALSE"), 1.0);
        // Priority: `^` above `*` and `/` above `+` and `-`.
        assert_eq!(value("1 + 2 * 3 ^ 2"), 19.0);
        assert_eq!(value("1 + ( 2 * 3 ) ^ 2"), 37.0);
    }
}
