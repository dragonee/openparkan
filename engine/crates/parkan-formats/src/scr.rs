//! The behaviour scripts, `MISSIONS/SCRIPTS/*.scr`, the variable table `varset.var`
//! they index, and the formula files `.fml` beside them. See `docs/15-behaviour.md`
//! and `openparkan/behaviour.py`.
//!
//! A script is three words, then one record per handler, each carrying its own flat
//! node list (`ai.dll:0x10011b20`). Everything is little-endian and nothing is
//! aligned.

use crate::cursor::{Cursor, FormatError, latin1};

/// The first word: the length of the function table the script was compiled against.
pub const MAGIC: i32 = 73;
/// The byte after a handler's name, its terminator.
pub const NAME_PAD: u8 = 0;
/// A name longer than this is taken for a bad record.
pub const MAX_NAME: i32 = 64;
/// What a head field or a trailer holds where it holds nothing.
pub const NULL: i32 = -1;
/// The fifth word on every node that is not a comparison.
pub const NO_RELATION: i32 = 6;

/// `head[3]`, what a node is (`ai.dll:0x10012038`).
pub const STATEMENT: i32 = -1;
pub const IF: i32 = 0;
pub const END: i32 = 1;
pub const LABEL: i32 = 2;
pub const GOTO: i32 = 3;
pub const SWITCH: i32 = 4;
pub const RETURN: i32 = 5;
pub const CONST: i32 = 6;

/// The shared symbol table's file name, in `MISSIONS/SCRIPTS`.
pub const VARSET: &str = "varset.var";

/// One node of a handler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// Function id, destination, source variable or number, and kind.
    pub head: [i32; 4],
    /// An `if`'s relation, 0..5; 6 on every other node.
    pub opcode: i32,
    /// Variable indices, or on the two jumps a node or handler index.
    pub operands: Vec<i32>,
    /// A formula index into the script's `.fml`, or -1.
    pub trailer: i32,
}

impl Node {
    pub fn function(&self) -> i32 {
        self.head[0]
    }

    pub fn destination(&self) -> i32 {
        self.head[1]
    }

    pub fn source(&self) -> i32 {
        self.head[2]
    }

    pub fn kind(&self) -> i32 {
        self.head[3]
    }

    /// True when the node calls a function.
    pub fn calls(&self) -> bool {
        self.head[0] != NULL
    }
}

/// A named entry point and the nodes behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handler {
    pub name: String,
    pub index: i32,
    pub nodes: Vec<Node>,
}

/// One `.scr` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub magic: i32,
    pub handlers: Vec<Handler>,
}

impl Script {
    /// The index of the handler of that name.
    pub fn handler(&self, name: &str) -> Option<usize> {
        self.handlers.iter().position(|h| h.name == name)
    }
}

fn name(r: &mut Cursor<'_>) -> Result<String, FormatError> {
    let at = r.pos;
    let length = r.i32()?;
    if !(1..=MAX_NAME).contains(&length) || r.remaining() < length as usize {
        return Err(FormatError::invalid(r.source(), format!("bad name length {length} at {at}")));
    }
    Ok(latin1(r.bytes(length as usize)?))
}

/// Read one script. Fails unless it is consumed exactly.
pub fn parse(data: &[u8], source: &str) -> Result<Script, FormatError> {
    let mut r = Cursor::new(data, source);
    let magic = r.i32()?;
    let count = r.i32()?;
    if magic != MAGIC {
        return Err(FormatError::invalid(source, format!("magic {magic}, not {MAGIC}")));
    }
    if count < 0 {
        return Err(FormatError::invalid(source, format!("handler count {count}")));
    }
    let mut handlers = Vec::new();
    for expected in 0..count {
        let name = name(&mut r)?;
        let pad = r.bytes(1)?[0];
        if pad != NAME_PAD {
            return Err(FormatError::invalid(source, format!("{name} padded with {pad}, not {NAME_PAD}")));
        }
        let index = r.i32()?;
        let node_count = r.i32()?;
        if index != expected {
            return Err(FormatError::invalid(source, format!("{name} indexed {index}, not {expected}")));
        }
        if node_count < 0 {
            return Err(FormatError::invalid(source, format!("{name} declares {node_count} nodes")));
        }
        let mut nodes = Vec::new();
        for _ in 0..node_count {
            let head = [r.i32()?, r.i32()?, r.i32()?, r.i32()?];
            let opcode = r.i32()?;
            let operand_count = r.i32()?;
            if !(0..=NO_RELATION).contains(&opcode) {
                return Err(FormatError::invalid(source, format!("{name} node opcode {opcode}")));
            }
            if operand_count < 0 {
                return Err(FormatError::invalid(
                    source,
                    format!("{name} node takes {operand_count} operands"),
                ));
            }
            let operands = (0..operand_count).map(|_| r.i32()).collect::<Result<Vec<_>, _>>()?;
            let trailer = r.i32()?;
            nodes.push(Node { head, opcode, operands, trailer });
        }
        handlers.push(Handler { name, index, nodes });
    }
    if r.remaining() != 0 {
        return Err(FormatError::invalid(
            source,
            format!("{} bytes left over after {count} handlers", r.remaining()),
        ));
    }
    Ok(Script { magic, handlers })
}

/// One declaration of `varset.var`: `VAR( Type, Name, DefValue, …)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    /// `VAR` or `STRING`.
    pub kind: String,
    pub type_name: String,
    pub name: String,
    /// The default as written, trimmed.
    pub default: String,
}

/// Python's `str.strip` whitespace, near enough for Latin-1 text.
fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// `rest` is `\s*;?\s*` to its end.
fn only_terminator(rest: &str) -> bool {
    let rest = rest.trim_start_matches(is_space);
    let rest = rest.strip_prefix(';').unwrap_or(rest);
    rest.trim_start_matches(is_space).is_empty()
}

fn declaration(body: &str) -> Option<Variable> {
    let s = body.trim_start_matches(is_space);
    let (kind, s) = ["VAR", "STRING"].iter().find_map(|k| s.strip_prefix(k).map(|rest| (*k, rest)))?;
    let s = s.strip_prefix('(')?;
    let comma = s.find([',', ')'])?;
    if s.as_bytes()[comma] != b',' {
        return None;
    }
    let type_name = trim(&s[..comma]);
    if type_name.is_empty() {
        return None;
    }
    let s = s[comma + 1..].trim_start_matches(is_space);
    s.chars().next().filter(|c| c.is_ascii_alphabetic() || *c == '_')?;
    let length = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(s.len());
    let name = &s[..length];
    let s = s[length..].trim_start_matches(is_space);
    let default = if let Some(rest) = s.strip_prefix(',') {
        // The lazy `(.*?)` ends at the first `)` that only a terminator follows.
        let close = rest.char_indices().find(|&(i, c)| c == ')' && only_terminator(&rest[i + 1..]))?.0;
        trim(&rest[..close]).to_owned()
    } else {
        let rest = s.strip_prefix(')')?;
        if !only_terminator(rest) {
            return None;
        }
        String::new()
    };
    Some(Variable { kind: kind.to_owned(), type_name: type_name.to_owned(), name: name.to_owned(), default })
}

/// Read `varset.var`'s declarations in file order: the index is the position.
/// Comment-only lines are skipped and a trailing `//` comment is cut off.
pub fn parse_variables(data: &[u8], source: &str) -> Result<Vec<Variable>, FormatError> {
    // Read as text is read: every line ending becomes a newline.
    let text = latin1(data).replace("\r\n", "\n").replace('\r', "\n");
    let out: Vec<Variable> = text
        .split('\n')
        .filter_map(|line| {
            let body = if line.trim_start_matches(is_space).starts_with("//") {
                ""
            } else {
                line.split("//").next().unwrap_or("")
            };
            declaration(body)
        })
        .collect();
    if out.is_empty() {
        return Err(FormatError::invalid(source, "no declarations"));
    }
    Ok(out)
}

/// The expressions of one `.fml`, in file order: one `FUNCTION( , <expression>, )`
/// line each. Blank lines and `//` lines are skipped.
pub fn parse_formulas(data: &[u8], source: &str) -> Result<Vec<String>, FormatError> {
    // Read as text is read: every line ending becomes a newline.
    let text = latin1(data).replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();
    for line in text.split('\n') {
        let lead = line.trim_start_matches(is_space);
        if trim(line).is_empty() || lead.starts_with("//") {
            continue;
        }
        let bad = || FormatError::invalid(source, format!("not a formula: {:?}", trim(line)));
        let rest = lead.strip_prefix("FUNCTION(").ok_or_else(bad)?;
        // The first field runs to the first comma; the expression to the last comma,
        // which only a field and the closing bracket may follow.
        let rest = &rest[rest.find(',').ok_or_else(bad)? + 1..];
        let last = rest.rfind(',').ok_or_else(bad)?;
        let tail = rest[last + 1..].trim_end_matches(is_space);
        let tail = tail.strip_suffix(';').unwrap_or(tail).trim_end_matches(is_space);
        if !tail.ends_with(')') {
            return Err(bad());
        }
        out.push(trim(&rest[..last]).to_owned());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(out: &mut Vec<u8>, values: &[i32]) {
        for v in values {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn script() -> Vec<u8> {
        let mut b = Vec::new();
        words(&mut b, &[MAGIC, 2]);
        for (i, name) in ["Init", "Mission"].iter().enumerate() {
            words(&mut b, &[name.len() as i32]);
            b.extend_from_slice(name.as_bytes());
            b.push(0);
            words(&mut b, &[i as i32, 2]);
            words(&mut b, &[19, -1, -1, STATEMENT, NO_RELATION, 3, 224, 225, 226, -1]);
            words(&mut b, &[-1, 30, -1, STATEMENT, NO_RELATION, 0, 4]);
        }
        b
    }

    #[test]
    fn a_script_reads_to_its_last_byte() {
        let s = parse(&script(), "t.scr").unwrap();
        assert_eq!(s.handlers.len(), 2);
        assert_eq!(s.handler("Mission"), Some(1));
        let n = &s.handlers[0].nodes[0];
        assert_eq!(
            (n.function(), n.kind(), n.operands.clone(), n.trailer),
            (19, STATEMENT, vec![224, 225, 226], -1)
        );
        assert_eq!(s.handlers[1].nodes[1].trailer, 4);
    }

    #[test]
    fn a_script_with_bytes_left_over_or_a_wrong_magic_is_refused() {
        let mut extra = script();
        extra.push(0);
        assert!(parse(&extra, "t.scr").is_err());
        let mut magic = script();
        magic[0] = 72;
        assert!(parse(&magic, "t.scr").is_err());
    }

    #[test]
    fn declarations_keep_their_order_through_comments_and_semicolons() {
        let text = b"//VAR( Type, Name, DefValue, Minimum, Maximum, Comment)\r\n\r\nVAR( float, f0, 0)\r\nVAR( DWORD, ClanBaseX, 950);\r\nVAR( DWORD, ERROR,\t\t0xffffffff)\t// all bits\r\n  // VAR( DWORD, gone, 1)\r\nVAR( DWORD, dTemp)\r\n";
        let v = parse_variables(text, "varset.var").unwrap();
        let names: Vec<&str> = v.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["f0", "ClanBaseX", "ERROR", "dTemp"]);
        assert_eq!((v[1].type_name.as_str(), v[1].default.as_str()), ("DWORD", "950"));
        assert_eq!(v[2].default, "0xffffffff");
        assert_eq!(v[3].default, "");
    }

    #[test]
    fn formulas_are_the_middle_field() {
        let text = b"//FormulaSet export file\r\n\r\nFUNCTION( , 0,  )\r\nFUNCTION( , ( df5 + 4 ),  )\r\nFUNCTION( , 20 + 55*fDifficulty,  );\r\n";
        assert_eq!(parse_formulas(text, "t.fml").unwrap(), ["0", "( df5 + 4 )", "20 + 55*fDifficulty"]);
        assert!(parse_formulas(b"FUNCTION( 1 )\n", "t.fml").is_err());
    }
}
