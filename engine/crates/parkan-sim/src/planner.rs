//! The clan's planner: the problem list a SuperAI keeps, the groups inside a problem, and
//! the four passes one clan takt makes over it. See `docs/15-behaviour.md`, "The planner:
//! when a `_Start` runs, and when a `_Continue`", and "What the functions do".
//!
//! Nothing here runs a script. The planner owns the records the script's calls read and
//! write — a problem's weight, its three parameters, its units and its groups — and says
//! which handler the takt runs next; the caller runs it through [`crate::script`].

use std::collections::{BTreeMap, BTreeSet};

/// A problem's state (`varset.var`'s `ST_*`, the record's `+0x18`).
pub const ST_NONE: u32 = 0;
pub const ST_SOLVING: u32 = 1;
pub const ST_SOLVED: u32 = 2;
pub const ST_UNSOLVED: u32 = 3;

/// Function 25's fills (`varset.var`'s `TAKE_*`); `CREATE_EMPTY` is 0 and no script uses it.
pub const CREATE_EMPTY: u32 = 0;
pub const TAKE_ALL_FREE: u32 = 1;
pub const TAKE_BY_HITS: u32 = 2;
pub const TAKE_ALL_BATTLE_UNITS: u32 = 3;

/// Functions 13 and 14's modes (`varset.var`'s `UNIT_*`).
pub const UNIT_FREE_UNIT: u32 = 0;
pub const UNIT_FREE_CAPTURER: u32 = 1;
pub const UNIT_ANY_UNIT: u32 = 2;
pub const UNIT_ANY_CAPTURER: u32 = 3;
pub const UNIT_ANY_NEAREST_CAPTURER: u32 = 4;

/// Function 6's second operand.
pub const UNIT_NORMAL: u32 = 0;
pub const UNIT_CRITICAL: u32 = 1;

/// Function 27's actions and target kinds (`varset.var`'s `ACTION_*` and `EXP_TARGET_*`).
pub const ACTION_DESTROY: u32 = 1;
pub const ACTION_NOTHING_DOING: u32 = 2;
pub const ACTION_CAPTURE_BUILDING: u32 = 3;
pub const ACTION_GROUP_DESTROY: u32 = 4;
pub const ACTION_GROUP_NOTHING_DOING: u32 = 5;
pub const EXP_TARGET_BY_LOGIC_ID: u32 = 0;
pub const EXP_TARGET_BY_GROUP_ID: u32 = 1;

/// Function 38's flag.
pub const FREE_UNITS: u32 = 0;
pub const ALL_UNITS: u32 = 1;

/// The design picks (`varset.var`'s `SELECT_*`), which a build order carries as its
/// `TARGET_BY_NAME` target.
pub const SELECT_BEST_WEAPON: u32 = 1;
pub const SELECT_BEST_ARMOR: u32 = 2;
pub const SELECT_BEST_RANGE: u32 = 3;
pub const SELECT_FASTEST: u32 = 4;
pub const SELECT_BEST_COMBAT: u32 = 5;
pub const SELECT_SMALLEST: u32 = 6;

/// `varset.var`'s robot types.
pub const ROBOT_TRANSPORT: u32 = 0x0100_2000;
pub const ROBOT_BUILDER: u32 = 0x0100_4000;
pub const ROBOT_BATTLEUNIT: u32 = 0x0100_8000;
pub const ROBOT_HQ: u32 = 0x0101_0000;
pub const ROBOT_HERO: u32 = 0x0102_0000;

/// The `PBM_*` codes two functions name: 9 is what function 35 skips a target already raised
/// for, and 12 what function 47 does.
pub const PBM_BUILDING_CAPTURE: u32 = 9;
pub const PBM_BASE_DEFENCE: u32 = 12;

/// The handler every clan's takt runs (docs/34, "When the Mission handler runs"). Function 33
/// can name another, `Problems<n>`.
pub const HANDLER_PROBLEMS: &str = "Problems0";
/// The suffixes a raise appends to its code's variable name to find the handler pair
/// (`ai.dll:0x1003d770`, `0x1003d778`).
pub const START: &str = "_Start";
pub const CONTINUE: &str = "_Continue";

/// `varset.var`'s `ERROR`, which a function answers when it has none.
pub const ERROR: u32 = 0xffff_ffff;

/// What the SuperAI writes into the script's `fDifficulty` as it is built
/// (`ai.dll:0x10005d00`, *read*): six floats indexed by the game level, `Iron_3D.ini`'s
/// `[CS] GAME_LEVEL`. The handler stores the level at the SuperAI's `+0x384`, looks
/// `fDifficulty` up in the variable table by name and sets it through the float setter
/// (`0x10013650`). `varset.var` declares the variable 0.5, which is this table's medium.
pub const DIFFICULTY: [f32; 6] = [0.0, 0.5, 1.0, 0.0, 0.0, 0.0];

/// `fDifficulty` for game level `level`.
pub fn difficulty(level: usize) -> f32 {
    DIFFICULTY.get(level).copied().unwrap_or(0.0)
}

/// The size classes that may capture: a unit's record `+0x30` is 1 or 2
/// (`ai.dll:0x100301a9`, docs/31, "The wingman menu from first person").
pub const CAPTURER_SIZES: std::ops::RangeInclusive<u8> = 1..=2;

/// Function 11's second bar (`ai.dll:0x1000c733`-`0x1000c810`): for each of seven building
/// types, how many of it filed on the clan's list answer 1 whatever its `dMax*` says. The
/// count only ever rises (`0x1000194f`-`0x100019a9`): the constructor's seven zeros
/// (`0x10005814`-`0x10005838`) are the only other writes, so a clan that has once held a mine
/// never asks for another (docs/15, "What the functions do").
pub const FILED_BARS: [(u32, u32); 7] = [
    (0x8000_0004, 1),
    (0x8000_0008, 1),
    (0x8000_0010, 1),
    (0x8000_0040, 1),
    (0x8010_0000, 3),
    (0x8020_0000, 1),
    (0x8000_0400, 1),
];

/// Function 11 (`ai.dll:0x1000c5e7`): whether a type is at its limit. `held` is how many the
/// clan has now (`0x100060e0`), `max` the `dMax*` variable the type is bound by, if any, and
/// `filed` how many of it have ever been filed on the clan's list. A type on neither table,
/// every battle unit among them, answers false.
pub fn at_limit(type_word: u32, held: usize, max: Option<usize>, filed: u32) -> bool {
    let barred = FILED_BARS.iter().any(|&(t, bar)| t == type_word && filed >= bar);
    barred || max.is_some_and(|max| held >= max)
}

/// The one code whose solving problem is ended by losing its units whatever its drain
/// (`ai.dll:0x100056c8`).
pub const PBM_MAKE_RESEARCH: u32 = 10;

/// The orders a unit may be on and still have *nothing doing* (`ai.dll:0x10005253`–
/// `0x1000526a`): `ORDER_ROBOT_STOP`, the construction sphere's 18 and
/// `ORDER_ROBOT_SHUTDOWN`.
pub const NOTHING_DOING_ORDERS: [i32; 3] = [1, 18, 19];

/// The order every robot of an AI clan's problem is given as the problem lets it go
/// (`ai.dll:0x10003e80`): `ORDER_ROBOT_REPARE`, with no target, replacing what it had. A
/// building's logical id is negative, and the routine's sign test passes it over
/// (`0x10003ea3`).
pub const RELEASE_ORDER: i32 = 9;

/// How many rounds of one takt's `_Start` pass the engine will run. The game's loop has no
/// bound of its own (`ai.dll:0x10001e3d`): a round repeats only when every handler it ran
/// started or solved its problem, which takes those problems out of the next.
pub const MAX_START_PASSES: usize = 64;

/// What function 27 files on the running problem, a 16-byte record on the list at the
/// problem's `+0x34` (`ai.dll:0x100094f1`, pushed by `0x10005530`): an `ACTION_*`, the kind
/// of its target and the target. The record's fourth field, a byte the drain sets once the
/// action has come true, is not kept here: with the problem's `+0x48` at 0, which nothing
/// shipped changes, the first action to come true retires the problem and its list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub action: u32,
    pub kind: u32,
    pub target: i32,
}

/// What the clan areal map answers for one logical id, as the drain asks it
/// (`IArealMap` slot 7, then the behaviour's slots 17, 4 and 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    /// Its owner word: a clan's index, 65534 once destroyed.
    pub owner: u32,
    /// The order at the head of its list; `None` for an empty list.
    pub order: Option<i32>,
}

/// A group of units inside a problem, opened by function 25.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    /// The slot of the problem it belongs to.
    pub problem: usize,
    pub units: Vec<i32>,
}

/// One problem, as the constructor lays the 0x64-byte record out (`ai.dll:0x10004e50`).
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    /// The `PBM_*` code, `+0x00`.
    pub code: u32,
    /// The name of the variable the raise named it by, which names its handler pair.
    pub name: String,
    /// `+0x14`, the priority the `_Start` pass ranks by.
    pub weight: f32,
    /// `+0x24` and `+0x28`: the life counter, and what a re-raise reloads it by.
    pub life: i64,
    pub reload: i64,
    /// `+0x2c`, what each takt takes off the counter.
    pub drain: i64,
    /// `+0x04`, `+0x08`, `+0x0c`: `p1`, `p2` and `p3`, which function 29 reads back.
    pub p: [u32; 3],
    /// `+0x18`.
    pub state: u32,
    /// `+0x1c`: the unit array as the record keeps it. An attach writes the entry at the
    /// count and steps the count (`0x10005080`); a unit that leaves by itself — taken by
    /// another problem, reserved, lost to the clan — leaves a hole where it stood
    /// (`0x10004010` writes −1), and only letting them all go empties the array
    /// (`0x100050f0`).
    pub units: Vec<Option<i32>>,
    /// `+0x20`: how many entries of the array the record counts. A hole does not lower it;
    /// a group's release takes one off for each unit it lets go (`0x1000561e`), whichever
    /// entries those were.
    pub held: usize,
    /// `+0x30`: a unit marked critical has left the problem (`0x10004035`).
    pub lost_critical: bool,
    pub actions: Vec<Action>,
}

impl Problem {
    /// Whether the `_Start` pass may pick it: neither solving nor solved (`0x10001bf0`).
    pub fn startable(&self) -> bool {
        self.state != ST_SOLVING && self.state != ST_SOLVED
    }

    /// The units the record counts: the first `held` entries of its array, less the holes.
    pub fn counted(&self) -> impl Iterator<Item = i32> + '_ {
        self.units.iter().take(self.held).flatten().copied()
    }

    /// The handler a `_Start` pass runs for it.
    pub fn start_handler(&self) -> String {
        format!("{}{START}", self.name)
    }

    /// The handler a `_Continue` pass runs for it.
    pub fn continue_handler(&self) -> String {
        format!("{}{CONTINUE}", self.name)
    }
}

/// One object as the planner's picks see it: what the clan areal map caches for a contact
/// (`ArealMap.dll:0x10006e40`), with the two strengths `docs/15-behaviour.md`,
/// "What a strength is", tells apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub id: i32,
    pub clan: i64,
    pub type_word: u32,
    pub at: [f32; 2],
    /// What it is worth to a group: its strength at full life, the live form's property 54.
    pub full: f32,
    /// Its size class, `IGameObject` property `0x201`.
    pub size_class: u8,
    /// Its live top speed, `IControl` property 145.
    pub speed: f32,
    /// The order it is running, for function 50.
    pub order: Option<i32>,
}

impl Candidate {
    /// Whether function 14 picks it as a capturer: a battle unit, by its exact Type, of a size
    /// class of 1 or 2 (`ai.dll:0x10008cfb`, `0x10008db9`, `0x10009050`; `0x100301a9`). A small
    /// builder or transport may capture if it is ordered to, and no script's pick names one.
    pub fn capturer(&self) -> bool {
        self.type_word == ROBOT_BATTLEUNIT && CAPTURER_SIZES.contains(&self.size_class)
    }

    /// Whether it is a battle unit, the class `TAKE_ALL_BATTLE_UNITS` and function 38 count.
    pub fn battle(&self) -> bool {
        self.type_word & ROBOT_BATTLEUNIT == ROBOT_BATTLEUNIT
    }

    /// How long it would take to reach `to` at its live top speed, which is how function 14's
    /// `UNIT_ANY_NEAREST_CAPTURER` ranks the capturers (`ai.dll:0x100091b0`). A unit that
    /// cannot move never arrives.
    pub fn reach(&self, to: [f32; 2]) -> f32 {
        let distance = (self.at[0] - to[0]).hypot(self.at[1] - to[1]);
        if self.speed > 0.0 { distance / self.speed } else { f32::INFINITY }
    }
}

/// The clan's planner: its problem list (`+0xa0`), the groups inside it, the base the place
/// functions measure from (`+0x80`, `+0x84`, `+0x88`), and the two knobs functions 33 and 69
/// set.
#[derive(Clone, Debug, PartialEq)]
pub struct Planner {
    /// The problem list. A retired problem empties its slot rather than shortening the list,
    /// as the game's own retire zeroes the record and frees the slot (`0x10005010`), so an
    /// index a pass is holding stays the problem it named.
    pub problems: Vec<Option<Problem>>,
    /// Groups by id, the same way.
    pub groups: Vec<Option<Group>>,
    /// `dCurrentProblem`, the slot the two passes write before they run a handler.
    pub running: Option<usize>,
    /// Units function 51 reserved: taken by nobody (problem slot `0xfffe`).
    pub reserved: BTreeSet<i32>,
    /// The problem each unit is on: the `+0x14` of its record on the clan's list, which an
    /// attach writes and the unit's leaving clears.
    pub on: BTreeMap<i32, usize>,
    /// The units function 6 marked critical: the `+0xc` of the unit's own record
    /// (`0x10004080`). The mark is the unit's, not the problem's, and stays on it from
    /// problem to problem until function 6 writes it again; a group's fill does not.
    pub critical: BTreeSet<i32>,
    /// The state word a slot's record was left with when its problem was retired (`+0x18`,
    /// which `fn8` writes after the retire has zeroed it): what the `_Start` pass reads back
    /// once a handler has run.
    pub left: Vec<u32>,
    /// The units problems have let go since the caller last took them, in order: an AI
    /// clan's robots are each given [`RELEASE_ORDER`].
    pub released: Vec<i32>,
    /// The base's centre and radius, which function 19 reads and 68 recomputes.
    pub base: [f32; 2],
    pub radius: f32,
    /// The design store's spread, function 69's `+0x10`: how far down its own ranking the
    /// clan's next build may fall.
    pub spread: u32,
    /// The problem handler, which function 33 re-points.
    pub handler: String,
    /// Whether function 43 has loaded `UNITS\\UNITS\\AI\\` into the design store. The store
    /// itself lives where the assemblies do; this is the clan's own switch, and a clan whose
    /// script never calls 43 builds nothing by name.
    pub store_loaded: bool,
    seed: u32,
}

impl Planner {
    pub fn new(base: [f32; 2], seed: u32) -> Self {
        Self {
            problems: Vec::new(),
            groups: Vec::new(),
            running: None,
            reserved: BTreeSet::new(),
            on: BTreeMap::new(),
            critical: BTreeSet::new(),
            left: Vec::new(),
            released: Vec::new(),
            base,
            radius: 0.0,
            spread: 0,
            handler: HANDLER_PROBLEMS.to_owned(),
            store_loaded: false,
            seed: seed | 1,
        }
    }

    /// Function 70's random number below `n`, and the draw the design pick makes over its
    /// spread.
    ///
    /// STAND-IN: docs/15-behaviour.md#function-69-sets-how-sloppy-the-ais-design-pick-is--read-and-measured
    /// -- the game draws on the CRT's `rand()` plus `timeGetTime()`, and its `rand()` is one
    /// stream for the whole module, never seeded (`ai.dll:0x100519d8` holds 1 and only
    /// `rand` itself touches it), which the takt's jitter and function 70 draw on too. The
    /// millisecond clock is what makes the draw free, and the engine has none: it keeps one
    /// stream a clan, the generator the clan takt already uses. The distribution is the
    /// game's, one index in `spread + 1` each.
    pub fn random(&mut self, n: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        if n == 0 { 0 } else { (self.seed >> 16) % n }
    }

    /// Every problem standing, with its slot.
    pub fn standing(&self) -> impl Iterator<Item = (usize, &Problem)> {
        self.problems.iter().enumerate().filter_map(|(i, p)| p.as_ref().map(|p| (i, p)))
    }

    pub fn problem(&self, slot: usize) -> Option<&Problem> {
        self.problems.get(slot).and_then(Option::as_ref)
    }

    /// Function 2 (`ai.dll:0x10009610`, the raise `0x100059f0`): raise the problem `name`
    /// names, unless one with the same code, `p1` and `p2` already stands (`0x10004c50`,
    /// which looks at nothing else — not the state). The caller has already checked that the
    /// script defines both handlers; a raise whose pair is missing is abandoned and makes no
    /// record (`0x10005aa1`).
    ///
    /// A raise that meets a standing problem compares weights (`0x10005bbd`–`0x10005bc9`).
    /// **At a weight below the standing one's it does nothing at all.** Otherwise the
    /// standing problem takes the new weight and the new `p3` (`0x10005bdf`, `0x10005bf7`)
    /// and its life counter its reload again (`0x10005070`). So a want at 0.8 takes over the
    /// robot a want at 0.52 asked for, with its own `SELECT_*`, and from then on the lighter
    /// want's raises no longer keep the problem alive.
    ///
    /// A new record goes into the first free slot, one whose code, `p1` and `p2` are all 0
    /// (`0x100073e0`, matched against a blank record), else onto the end.
    ///
    /// Returns the problem's slot.
    pub fn raise(&mut self, name: &str, code: u32, weight: f32, life: i64, drain: i64, p: [u32; 3]) -> usize {
        let standing = self
            .standing()
            .find(|(_, q)| q.code == code && q.p[0] == p[0] && q.p[1] == p[1])
            .map(|(slot, _)| slot);
        if let Some(slot) = standing {
            if let Some(q) = self.problems[slot].as_mut()
                && q.weight <= weight
            {
                q.weight = weight;
                q.p[2] = p[2];
                q.life += q.reload;
            }
            return slot;
        }
        let problem = Problem {
            code,
            name: name.to_owned(),
            weight,
            life,
            reload: life,
            drain,
            p,
            state: ST_NONE,
            units: Vec::new(),
            held: 0,
            lost_critical: false,
            actions: Vec::new(),
        };
        let slot = match self.problems.iter().position(Option::is_none) {
            Some(slot) => {
                self.problems[slot] = Some(problem);
                slot
            }
            None => {
                self.problems.push(Some(problem));
                self.problems.len() - 1
            }
        };
        if self.left.len() < self.problems.len() {
            self.left.resize(self.problems.len(), ST_NONE);
        }
        self.left[slot] = ST_NONE;
        slot
    }

    /// The first walk of a clan's takt (`ai.dll:0x10005910`), which is where every problem
    /// that ends without its handler's word ends. `look` answers a logical id as the clan
    /// areal map does, `None` when no object has it.
    ///
    /// **First the units** (`0x10003e40` for each on the clan's list): one whose id no object
    /// answers, or whose owner is no longer `clan`, leaves its problem (`0x10003fc0`).
    ///
    /// **Then each problem in list order** (`0x10004ee0`), five tests in this order, any of
    /// which retires it:
    ///
    /// 1. solving, with units, and every one of them gone (`0x100056c0`) — only for a problem
    ///    with a drain, or a `PBM_MAKE_RESEARCH`;
    /// 2. with a drain above 0: solving and holding no unit, or still unstarted and holding
    ///    one (`0x10004efa`–`0x10004f15`, on the count at `+0x20`);
    /// 3. the life counter, less the drain, at 0 or below (`0x10004f1a`);
    /// 4. one of its actions come true (`0x10004f32`–`0x10004fbd`, the evaluator
    ///    `0x10005180`), whatever its state;
    /// 5. solving, a critical unit lost and none of those left critical (`0x10005650`); and
    ///    test 1 again.
    ///
    /// Tests 1, 2 and 5 retire through `0x10005040` and 3 and 4 through `0x10005010`, which
    /// differ in one word nothing reads afterwards, the third parameter.
    pub fn drain(&mut self, clan: i64, look: impl Fn(i32) -> Option<Seen>) {
        let listed: Vec<i32> = self.on.keys().chain(&self.reserved).copied().collect();
        for id in listed {
            if look(id).is_none_or(|s| i64::from(s.owner) != clan) {
                self.lose(id);
            }
        }
        for slot in 0..self.problems.len() {
            if self.problems[slot].is_some() && self.ended(slot, clan, &look) {
                self.retire(slot);
            }
        }
    }

    /// Whether the drain's step retires the problem in `slot` ([`Self::drain`]).
    fn ended(&mut self, slot: usize, clan: i64, look: &impl Fn(i32) -> Option<Seen>) -> bool {
        // Every unit it held has gone: the array holds nothing but holes.
        let emptied = |p: &Problem| {
            p.state == ST_SOLVING
                && (p.drain != 0 || p.code == PBM_MAKE_RESEARCH)
                && p.held > 0
                && p.counted().next().is_none()
        };
        let Some(p) = self.problems[slot].as_mut() else { return false };
        if emptied(p) {
            return true;
        }
        if p.drain > 0 && ((p.state == ST_SOLVING && p.held == 0) || (p.state == ST_NONE && p.held > 0)) {
            return true;
        }
        p.life -= p.drain;
        if p.life <= 0 {
            return true;
        }
        let actions = p.actions.clone();
        if actions.iter().any(|a| self.came_true(a, clan, look)) {
            return true;
        }
        let critical = &self.critical;
        let Some(p) = self.problems[slot].as_mut() else { return false };
        if p.state != ST_SOLVING {
            return false;
        }
        if p.lost_critical {
            if p.counted().any(|id| critical.contains(&id)) {
                // One of its critical units is still with it: the flag is cleared.
                p.lost_critical = false;
            } else if p.held > 0 {
                return true;
            }
        }
        emptied(p)
    }

    /// Whether an action has come true, one arm of the evaluator's five each
    /// (`ai.dll:0x10005180`, the table at `0x10005510`). An action whose target is not of the
    /// kind its arm wants never comes true.
    ///
    /// - `ACTION_DESTROY`: no object answers the id. A destroyed building's shell answers
    ///   for good, so this is a unit deleted, or one that never was.
    /// - `ACTION_NOTHING_DOING`: the object **is there**, and its order list is empty or
    ///   headed by one of [`NOTHING_DOING_ORDERS`].
    /// - `ACTION_CAPTURE_BUILDING`: the object is there and its owner is the clan.
    /// - `ACTION_GROUP_DESTROY`: no unit of the group answers its id; an empty group passes.
    /// - `ACTION_GROUP_NOTHING_DOING`: every unit of the group that answers has an empty
    ///   order list — no order is excused here; an empty group passes.
    pub fn came_true(&self, action: &Action, clan: i64, look: &impl Fn(i32) -> Option<Seen>) -> bool {
        let single = action.kind == EXP_TARGET_BY_LOGIC_ID;
        let group = || {
            usize::try_from(action.target)
                .ok()
                .filter(|_| action.kind == EXP_TARGET_BY_GROUP_ID)
                .and_then(|g| self.group(g))
        };
        match action.action {
            ACTION_DESTROY => single && look(action.target).is_none(),
            ACTION_NOTHING_DOING => {
                single
                    && look(action.target)
                        .is_some_and(|s| s.order.is_none_or(|o| NOTHING_DOING_ORDERS.contains(&o)))
            }
            ACTION_CAPTURE_BUILDING => {
                single && look(action.target).is_some_and(|s| i64::from(s.owner) == clan)
            }
            ACTION_GROUP_DESTROY => group().is_some_and(|g| g.units.iter().all(|&id| look(id).is_none())),
            ACTION_GROUP_NOTHING_DOING => {
                group().is_some_and(|g| g.units.iter().all(|&id| look(id).is_none_or(|s| s.order.is_none())))
            }
            _ => false,
        }
    }

    /// A unit lost to the clan (`0x10003fc0`): off its problem, off its reservation, and its
    /// record's marks with it.
    pub fn lose(&mut self, id: i32) {
        self.detach(id);
        self.reserved.remove(&id);
        self.critical.remove(&id);
    }

    /// Retire the problem in `slot` (`0x10005010`, `0x10005040`): its units are let go, its
    /// actions and groups cleared, and its code, parameters and state zeroed, which frees
    /// the slot. The weight and the life counter are left as they were.
    pub fn retire(&mut self, slot: usize) {
        if self.problems.get(slot).is_none_or(Option::is_none) {
            return;
        }
        self.release(slot);
        self.problems[slot] = None;
        self.left[slot] = ST_NONE;
        if self.running == Some(slot) {
            self.running = None;
        }
    }

    /// Let go every unit the problem in `slot` counts, and drop its groups (`0x100050f0`).
    /// Each unit's record is taken off the problem and the unit handed to
    /// [`Self::released`]: `0x10003e80` gives it [`RELEASE_ORDER`] when the clan is an AI's
    /// (the word slot 5 started the SuperAI with is 2) and the unit is no building.
    fn release(&mut self, slot: usize) {
        let Some(p) = self.problems.get_mut(slot).and_then(Option::as_mut) else { return };
        let gone: Vec<i32> = p.counted().collect();
        p.units.clear();
        p.held = 0;
        for id in gone {
            self.on.remove(&id);
            self.reserved.remove(&id);
            self.released.push(id);
        }
        for g in &mut self.groups {
            if g.as_ref().is_some_and(|g| g.problem == slot) {
                *g = None;
            }
        }
    }

    /// The `_Continue` pass (`0x10001e50`): every `ST_SOLVING` problem, in list order.
    pub fn solving(&self) -> Vec<usize> {
        self.standing().filter(|(_, p)| p.state == ST_SOLVING).map(|(i, _)| i).collect()
    }

    /// One round of the `_Start` pass (`0x10001bf0`): the problems at the largest weight
    /// among those neither solving nor solved, in list order. The largest is found from 0
    /// with a strict comparison (`0x10001c82`), and a round whose largest is still 0 runs
    /// nothing (`0x10001d6f`): **a problem of weight 0 or less is never started.**
    ///
    /// The caller runs each one's handler and then asks [`Self::state_word`]: the game sets a
    /// flag for every handler that left its problem neither solved nor solving
    /// (`0x10001e22`) and runs another round only while the flag is clear (`0x10001e3d`).
    /// So one takt starts problems from the heaviest down **until one will not start**, and
    /// what stood below that one waits for the next takt.
    pub fn start_batch(&self) -> Vec<usize> {
        let open = || self.standing().filter(|(_, p)| p.startable());
        let top = open().map(|(_, p)| p.weight).fold(0.0_f32, f32::max);
        if top == 0.0 {
            return Vec::new();
        }
        open().filter(|(_, p)| p.weight == top).map(|(i, _)| i).collect()
    }

    /// The state word of the record in `slot` (`+0x18`): the problem's while one stands
    /// there, and what the retire's caller left behind once it has gone — `ST_SOLVED` or
    /// `ST_UNSOLVED` after function 8, 0 after the drain.
    pub fn state_word(&self, slot: usize) -> u32 {
        self.problem(slot).map_or_else(|| self.left.get(slot).copied().unwrap_or(ST_NONE), |p| p.state)
    }

    /// The running problem.
    pub fn current(&self) -> Option<&Problem> {
        self.running.and_then(|i| self.problem(i))
    }

    pub fn current_mut(&mut self) -> Option<&mut Problem> {
        self.running.and_then(|i| self.problems.get_mut(i)).and_then(Option::as_mut)
    }

    /// Function 29: the running problem's `p1`, `p2` or `p3`; `ERROR` where it has none.
    pub fn parameter(&self, which: usize) -> u32 {
        self.current().and_then(|p| p.p.get(which).copied()).unwrap_or(ERROR)
    }

    /// Function 8 (`ai.dll:0x10009b0d`, the setter `0x10005710`): set the running problem's
    /// state. **`ST_SOLVED` and `ST_UNSOLVED` both retire it** — the first through
    /// `0x10005010`, the second through `0x10005040` — and only then write the state into
    /// the freed record, where the `_Start` pass reads it back. Any other value is written
    /// as it stands. A solved record has no code left for the raise's duplicate test to
    /// match, so the next want raises its problem afresh.
    pub fn set_state(&mut self, state: u32) {
        let Some(slot) = self.running else { return };
        if state == ST_SOLVED || state == ST_UNSOLVED {
            if self.problem(slot).is_some() {
                self.retire(slot);
                self.left[slot] = state;
            }
            return;
        }
        if let Some(p) = self.current_mut() {
            p.state = state;
        }
    }

    /// Attach a unit to the running problem (`0x10005080`): it leaves the problem it was on
    /// and takes the entry at the record's count. Function 6 passes its second operand as
    /// `critical` and so writes the unit's mark (`0x10004080`); a group's fill passes none
    /// and leaves the mark as it stood. A reservation is the unit's problem word too, and
    /// the attach overwrites it.
    pub fn attach(&mut self, id: i32, critical: Option<bool>) {
        let Some(slot) = self.running.filter(|&s| self.problem(s).is_some()) else { return };
        self.detach(id);
        self.reserved.remove(&id);
        if let Some(p) = self.problems[slot].as_mut() {
            let at = p.held;
            match p.units.get_mut(at) {
                Some(entry) => *entry = Some(id),
                None => p.units.push(Some(id)),
            }
            p.held += 1;
        }
        self.on.insert(id, slot);
        match critical {
            Some(true) => {
                self.critical.insert(id);
            }
            Some(false) => {
                self.critical.remove(&id);
            }
            None => {}
        }
    }

    /// Take `id` off the problem it is on (`0x10004010`): its entry among those the record
    /// counts becomes a hole, and a unit marked critical sets the problem's flag as it
    /// goes. The count is not lowered, and a group that lists the unit goes on listing it.
    pub fn detach(&mut self, id: i32) {
        let Some(slot) = self.on.remove(&id) else { return };
        let critical = self.critical.contains(&id);
        if let Some(p) = self.problems.get_mut(slot).and_then(Option::as_mut) {
            p.lost_critical |= critical;
            let held = p.held;
            if let Some(entry) = p.units.iter_mut().take(held).find(|e| **e == Some(id)) {
                *entry = None;
            }
        }
    }

    /// Function 7: let go the running problem's units, and drop its groups.
    pub fn release_units(&mut self) {
        if let Some(slot) = self.running {
            self.release(slot);
        }
    }

    /// Function 27: file an action on the running problem.
    pub fn expect(&mut self, action: Action) {
        if let Some(p) = self.current_mut() {
            p.actions.push(action);
        }
    }

    /// The weight of the problem unit `id` is on; `None` when it is free. A reserved unit
    /// answers a weight no problem can reach, so nobody takes it.
    pub fn holder(&self, id: i32) -> Option<f32> {
        if self.reserved.contains(&id) {
            return Some(f32::INFINITY);
        }
        self.on.get(&id).and_then(|&slot| self.problem(slot)).map(|p| p.weight)
    }

    /// Whether the running problem may take unit `id`: it is free, or the problem holding it
    /// is lighter (`ai.dll:0x100089a5`).
    pub fn may_take(&self, id: i32) -> bool {
        let Some(held) = self.holder(id) else { return true };
        self.current().is_some_and(|p| held < p.weight)
    }

    /// Whether unit `id` is free of every problem.
    pub fn free(&self, id: i32) -> bool {
        self.holder(id).is_none()
    }

    /// Function 51 (`ai.dll:0x1000e00c`): reserve a unit from every problem, or free it. It
    /// writes the unit's problem word and nothing else (`0x1000e0d9`, `0x1000e0be`): the
    /// problem the unit was on keeps its entry and goes on counting it.
    pub fn reserve(&mut self, id: i32, on: bool) {
        self.on.remove(&id);
        if on {
            self.reserved.insert(id);
        } else {
            self.reserved.remove(&id);
        }
    }

    /// Whether a problem of code `code` names `target` as its first parameter: what functions
    /// 35 and 47 pass over.
    pub fn raised_for(&self, code: u32, target: u32) -> bool {
        self.standing().any(|(_, p)| p.code == code && p.p[0] == target)
    }

    /// Functions 13 and 14 (`ai.dll:0x100091b0`): pick one of the clan's units by `mode`,
    /// `candidates` being the clan's own. `target` is where a `UNIT_ANY_NEAREST_CAPTURER`
    /// measures to, and `type_word` the type the two `_UNIT` modes ask for. An `ANY` mode
    /// takes a unit that is free or on a lighter problem; a `FREE` mode only a free one.
    pub fn pick_unit(
        &self,
        mode: u32,
        type_word: u32,
        target: Option<[f32; 2]>,
        candidates: &[Candidate],
    ) -> Option<i32> {
        let free_only = matches!(mode, UNIT_FREE_UNIT | UNIT_FREE_CAPTURER);
        let available = |id: i32| if free_only { self.free(id) } else { self.may_take(id) };
        let wanted = |c: &Candidate| match mode {
            UNIT_FREE_UNIT | UNIT_ANY_UNIT => c.type_word == type_word,
            _ => c.capturer(),
        };
        let open = candidates.iter().filter(|c| wanted(c) && available(c.id));
        match mode {
            UNIT_ANY_NEAREST_CAPTURER => {
                let to = target?;
                open.min_by(|a, b| a.reach(to).total_cmp(&b.reach(to))).map(|c| c.id)
            }
            _ => open.map(|c| c.id).next(),
        }
    }

    /// Function 50: a unit of the running problem running order `code`; `None` for no such.
    pub fn unit_on_order(&self, code: i32, candidates: &[Candidate]) -> Option<i32> {
        let problem = self.current()?;
        problem.counted().find(|&id| candidates.iter().any(|c| c.id == id && c.order == Some(code)))
    }

    /// Function 45 (`ai.dll:0x10005790`): the summed strength of the units the running
    /// problem counts.
    pub fn units_strength(&self, candidates: &[Candidate]) -> f32 {
        let Some(problem) = self.current() else { return 0.0 };
        problem.counted().filter_map(|id| candidates.iter().find(|c| c.id == id)).map(|c| c.full).sum()
    }

    /// Function 25: open a group in the running problem and fill it by `take`; `None`, and no
    /// group, where it gathers nothing.
    ///
    /// `candidates` are the clan's own units in the order the fill wants them —
    /// `TAKE_BY_HITS` hands them nearest the target first. `amount` goes in as the strength
    /// to reach and comes back as what was gathered, which is how a script reads whether the
    /// group fell short: `if dTemp3 < dT3` after the call.
    pub fn make_group(&mut self, take: u32, candidates: &[Candidate], amount: &mut f32) -> Option<usize> {
        let problem = self.running?;
        let mut units: Vec<i32> = Vec::new();
        match take {
            TAKE_ALL_FREE => units.extend(candidates.iter().filter(|c| self.free(c.id)).map(|c| c.id)),
            // Every battle unit that is free or on a problem of another code (`0x1000a4b0`).
            TAKE_ALL_BATTLE_UNITS => {
                let code = self.problem(problem).map_or(0, |p| p.code);
                let other_code = |id: i32| {
                    self.on.get(&id).and_then(|&slot| self.problem(slot)).is_some_and(|p| p.code != code)
                };
                units.extend(
                    candidates
                        .iter()
                        .filter(|c| c.battle() && !self.reserved.contains(&c.id))
                        .filter(|c| self.free(c.id) || other_code(c.id))
                        .map(|c| c.id),
                );
            }
            // Battle units nearest the target, free ones first and then those on lighter
            // problems, until their summed strength reaches the amount.
            TAKE_BY_HITS => {
                let (mut got, wanted) = (0.0, *amount);
                for pass in [true, false] {
                    for c in candidates.iter().filter(|c| c.battle()) {
                        if got >= wanted {
                            break;
                        }
                        let open = if pass { self.free(c.id) } else { self.may_take(c.id) };
                        if open && !units.contains(&c.id) {
                            units.push(c.id);
                            got += c.full;
                        }
                    }
                }
                *amount = got;
            }
            _ => {}
        }
        if units.is_empty() {
            return None;
        }
        // The fill attaches each unit and leaves its critical mark as it stood.
        for &id in &units {
            self.attach(id, None);
        }
        let group = Group { problem, units };
        Some(match self.groups.iter().position(Option::is_none) {
            Some(i) => {
                self.groups[i] = Some(group);
                i
            }
            None => {
                self.groups.push(Some(group));
                self.groups.len() - 1
            }
        })
    }

    /// Function 24 (`ai.dll:0x100055b0`): release a group. Each unit it lists leaves the
    /// problem it is on and takes one off the group's problem's count, and the group stands
    /// on, empty (`0x10004520`) — which is how both group actions read it as come true. No
    /// order goes with this release.
    pub fn release_group(&mut self, id: usize) {
        let Some(group) = self.groups.get_mut(id).and_then(Option::as_mut) else { return };
        let (problem, units) = (group.problem, std::mem::take(&mut group.units));
        for &unit in &units {
            self.detach(unit);
        }
        if let Some(p) = self.problems.get_mut(problem).and_then(Option::as_mut) {
            p.held = p.held.saturating_sub(units.len());
        }
    }

    pub fn group(&self, id: usize) -> Option<&Group> {
        self.groups.get(id).and_then(Option::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: i32, full: f32) -> Candidate {
        Candidate {
            id,
            clan: 1,
            type_word: ROBOT_BATTLEUNIT,
            at: [0.0, 0.0],
            full,
            size_class: 2,
            speed: 10.0,
            order: None,
        }
    }

    fn planner() -> Planner {
        Planner::new([0.0; 2], 1)
    }

    /// What the areal map answers in these tests: the ids in `here`, each `clan`'s, on the
    /// order given.
    fn world(clan: u32, here: &[(i32, Option<i32>)]) -> impl Fn(i32) -> Option<Seen> + '_ {
        move |id| here.iter().find(|&&(i, _)| i == id).map(|&(_, order)| Seen { owner: clan, order })
    }

    #[test]
    fn a_raise_at_no_less_a_weight_takes_the_standing_problem_over_and_a_lighter_one_does_nothing() {
        let combat = [ROBOT_BATTLEUNIT, 0, SELECT_BEST_COMBAT];
        let mut p = planner();
        let i = p.raise("PBM_ROBOT_NEEDED", 6, 0.52, 25, 24, combat);
        assert_eq!((i, p.standing().count()), (0, 1));
        // The same code, p1 and p2 from a heavier want: its weight, its p3, and the reload.
        let j = p.raise("PBM_ROBOT_NEEDED", 6, 0.801, 25, 24, [ROBOT_BATTLEUNIT, 0, SELECT_FASTEST]);
        assert_eq!((j, p.standing().count()), (0, 1));
        let q = p.problem(0).unwrap();
        assert_eq!((q.life, q.weight, q.p[2]), (50, 0.801, SELECT_FASTEST));
        // The lighter want raising again: nothing, not even the reload.
        p.raise("PBM_ROBOT_NEEDED", 6, 0.52, 25, 24, combat);
        let q = p.problem(0).unwrap();
        assert_eq!((q.life, q.weight, q.p[2]), (50, 0.801, SELECT_FASTEST), "0.52 is below 0.801");
        // At the standing weight itself the raise is taken: the script's own repeat.
        p.raise("PBM_ROBOT_NEEDED", 6, 0.801, 25, 24, combat);
        let q = p.problem(0).unwrap();
        assert_eq!((q.life, q.p[2]), (75, SELECT_BEST_COMBAT));
        // A different p1 is a second problem.
        p.raise("PBM_ROBOT_NEEDED", 6, 0.5, 25, 24, [ROBOT_TRANSPORT, 0, SELECT_FASTEST]);
        assert_eq!(p.standing().count(), 2);
    }

    #[test]
    fn the_drain_retires_a_problem_after_the_takts_its_two_numbers_buy() {
        let takts = |life: i64, drain: i64| {
            let mut p = planner();
            p.raise("PBM_X", 1, 0.5, life, drain, [0; 3]);
            let mut n = 0;
            while p.standing().count() > 0 && n < 100 {
                p.drain(1, |_| None);
                n += 1;
            }
            n
        };
        assert_eq!(takts(25, 24), 2, "the commonest shape stands two takts");
        assert_eq!(takts(5, 2), 3);
        assert_eq!(takts(15, 1), 15);
        assert_eq!(takts(1, 0), 100, "a drain of 0 never expires");
    }

    #[test]
    fn the_start_pass_offers_the_heaviest_unstarted_and_never_a_weight_of_nothing() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 1, 0, [1, 0, 0]);
        p.raise("B", 2, 0.8, 1, 0, [2, 0, 0]);
        p.raise("C", 3, 0.5, 1, 0, [3, 0, 0]);
        p.raise("D", 4, 0.0, 1, 0, [4, 0, 0]);
        assert_eq!(p.start_batch(), vec![1], "0.8 alone");
        // Unstarted, it is offered again: nothing but its state takes it out of the round.
        assert_eq!(p.start_batch(), vec![1]);
        p.running = Some(1);
        p.set_state(ST_SOLVING);
        assert_eq!(p.start_batch(), vec![0, 2], "then both at 0.5, in list order");
        assert_eq!(p.solving(), vec![1]);
        for slot in [0, 2] {
            p.running = Some(slot);
            p.set_state(ST_SOLVED);
        }
        assert!(p.start_batch().is_empty(), "the one left weighs 0, which is never started");
        assert_eq!(p.standing().map(|(i, _)| i).collect::<Vec<_>>(), vec![1, 3]);
    }

    #[test]
    fn solved_and_unsolved_both_retire_the_problem_and_leave_their_word_in_its_slot() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 5, 2, [7, 0, 0]);
        p.running = Some(0);
        p.attach(7, Some(true));
        assert_eq!(p.holder(7), Some(0.5));
        p.set_state(ST_SOLVED);
        assert!(p.standing().count() == 0 && p.free(7) && p.running.is_none());
        assert_eq!(p.state_word(0), ST_SOLVED, "what the _Start pass reads back");
        assert_eq!(p.released, vec![7], "and the unit is let go, to be given its order");
        // Nothing of the solved record is left for the duplicate test to match.
        let slot = p.raise("A", 1, 0.5, 5, 2, [7, 0, 0]);
        assert_eq!((slot, p.state_word(slot), p.problem(slot).unwrap().life), (0, ST_NONE, 5));
        // Unsolved retires it as well: the slot is free, and the word reads unsolved.
        p.running = Some(slot);
        p.attach(7, Some(false));
        p.set_state(ST_UNSOLVED);
        assert!(p.standing().count() == 0 && p.free(7));
        assert_eq!(p.state_word(slot), ST_UNSOLVED);
    }

    #[test]
    fn with_a_drain_a_solving_problem_holding_nothing_ends_and_so_does_an_unstarted_one_holding_a_unit() {
        let here = [(7, Some(12))];
        // Solving, no unit: gone at the next takt, long before its life is.
        let mut p = planner();
        p.raise("A", 1, 0.5, 30, 1, [0; 3]);
        p.running = Some(0);
        p.set_state(ST_SOLVING);
        p.drain(1, world(1, &here));
        assert_eq!(p.standing().count(), 0);
        // Solving and holding one: it stands.
        let mut p = planner();
        p.raise("A", 1, 0.5, 30, 1, [0; 3]);
        p.running = Some(0);
        p.attach(7, Some(false));
        p.set_state(ST_SOLVING);
        p.drain(1, world(1, &here));
        assert_eq!(p.problem(0).map(|q| q.life), Some(29));
        // Unstarted and holding one: its handler returned without a word, and it ends.
        let mut p = planner();
        p.raise("A", 1, 0.5, 30, 1, [0; 3]);
        p.running = Some(0);
        p.attach(7, Some(false));
        p.drain(1, world(1, &here));
        assert!(p.standing().count() == 0 && p.free(7));
        // Without a drain neither test is made.
        for attach in [false, true] {
            let mut p = planner();
            p.raise("A", 1, 0.5, 1, 0, [0; 3]);
            p.running = Some(0);
            if attach {
                p.attach(7, Some(false));
            } else {
                p.set_state(ST_SOLVING);
            }
            p.drain(1, world(1, &here));
            assert_eq!(p.standing().count(), 1, "attach {attach}");
        }
    }

    #[test]
    fn a_solving_problem_ends_when_its_units_are_all_gone_or_its_critical_one_is() {
        // Its one unit is lost to the clan: the array is all holes.
        let mut p = planner();
        p.raise("A", 1, 0.5, 30, 1, [0; 3]);
        p.running = Some(0);
        p.attach(7, Some(false));
        p.set_state(ST_SOLVING);
        p.drain(1, |_| None);
        assert_eq!(p.standing().count(), 0);
        // The same with no drain stands, unless it is a research.
        for (code, stands) in [(1, 1), (PBM_MAKE_RESEARCH, 0)] {
            let mut p = planner();
            p.raise("A", code, 0.5, 1, 0, [0; 3]);
            p.running = Some(0);
            p.attach(7, Some(false));
            p.set_state(ST_SOLVING);
            p.drain(1, |_| None);
            assert_eq!(p.standing().count(), stands, "code {code}");
        }
        // A critical unit taken by a heavier problem: the lighter one ends, though it still
        // holds another and has no drain.
        let here = [(7, None), (8, None)];
        let mut p = planner();
        p.raise("LIGHT", 1, 0.3, 1, 0, [1, 0, 0]);
        p.raise("HEAVY", 2, 0.9, 1, 0, [2, 0, 0]);
        p.running = Some(0);
        p.attach(7, Some(true));
        p.attach(8, Some(false));
        p.set_state(ST_SOLVING);
        p.running = Some(1);
        p.attach(7, None);
        assert!(p.problem(0).unwrap().lost_critical && p.critical.contains(&7), "the mark is the unit's");
        p.set_state(ST_SOLVING);
        p.drain(1, world(1, &here));
        assert_eq!(p.standing().map(|(i, _)| i).collect::<Vec<_>>(), vec![1]);
        // With a second critical unit still on it, the flag is cleared and it stands.
        let mut p = planner();
        p.raise("LIGHT", 1, 0.3, 1, 0, [1, 0, 0]);
        p.raise("HEAVY", 2, 0.9, 1, 0, [2, 0, 0]);
        p.running = Some(0);
        p.attach(7, Some(true));
        p.attach(8, Some(true));
        p.set_state(ST_SOLVING);
        p.running = Some(1);
        p.attach(7, None);
        p.drain(1, world(1, &here));
        assert_eq!(p.problem(0).map(|q| q.lost_critical), Some(false));
    }

    #[test]
    fn an_action_come_true_retires_its_problem_whatever_its_state() {
        let file = |action: u32, kind: u32, target: i32, solving: bool| {
            let mut p = planner();
            p.raise("A", 1, 0.5, 1, 0, [0; 3]);
            p.running = Some(0);
            if solving {
                p.set_state(ST_SOLVING);
            }
            p.expect(Action { action, kind, target });
            p
        };
        let ends = |mut p: Planner, clan: u32, here: &[(i32, Option<i32>)]| {
            p.drain(1, world(clan, here));
            p.standing().count() == 0
        };
        let id = EXP_TARGET_BY_LOGIC_ID;
        // Destroy: nothing answers the id.
        assert!(ends(file(ACTION_DESTROY, id, 7, true), 1, &[]));
        assert!(!ends(file(ACTION_DESTROY, id, 7, true), 1, &[(7, None)]));
        assert!(ends(file(ACTION_DESTROY, id, 7, false), 1, &[]), "an unstarted problem's too");
        // Nothing doing: the object is there, on no order or on one of the three excused.
        assert!(ends(file(ACTION_NOTHING_DOING, id, 7, true), 1, &[(7, None)]));
        for order in NOTHING_DOING_ORDERS {
            assert!(ends(file(ACTION_NOTHING_DOING, id, 7, true), 1, &[(7, Some(order))]), "{order}");
        }
        assert!(!ends(file(ACTION_NOTHING_DOING, id, 7, true), 1, &[(7, Some(12))]), "a factory building");
        assert!(!ends(file(ACTION_NOTHING_DOING, id, 7, true), 1, &[]), "an object that is gone is not idle");
        // Capture: the object is there and the clan's.
        assert!(ends(file(ACTION_CAPTURE_BUILDING, id, 7, true), 1, &[(7, None)]));
        assert!(!ends(file(ACTION_CAPTURE_BUILDING, id, 7, true), 2, &[(7, None)]));
        assert!(!ends(file(ACTION_CAPTURE_BUILDING, id, 7, true), 1, &[]));
        // A target of the wrong kind never comes true.
        assert!(!ends(file(ACTION_DESTROY, EXP_TARGET_BY_GROUP_ID, 7, true), 1, &[]));
    }

    #[test]
    fn a_groups_action_asks_every_unit_it_lists_and_an_empty_group_passes() {
        let units: Vec<Candidate> = (1..=2).map(|i| candidate(i, 1.0)).collect();
        let file = |action: u32| {
            let mut p = planner();
            p.raise("A", 1, 0.5, 1, 0, [0; 3]);
            p.running = Some(0);
            let mut amount = 0.0;
            let g = p.make_group(TAKE_ALL_FREE, &units, &mut amount).expect("a group");
            p.set_state(ST_SOLVING);
            p.expect(Action { action, kind: EXP_TARGET_BY_GROUP_ID, target: g as i32 });
            (p, g)
        };
        let ends = |mut p: Planner, here: &[(i32, Option<i32>)]| {
            p.drain(1, world(1, here));
            p.standing().count() == 0
        };
        assert!(!ends(file(ACTION_GROUP_DESTROY).0, &[(1, None)]), "one of the two still answers");
        assert!(ends(file(ACTION_GROUP_DESTROY).0, &[]), "neither does");
        assert!(ends(file(ACTION_GROUP_NOTHING_DOING).0, &[(1, None), (2, None)]));
        assert!(!ends(file(ACTION_GROUP_NOTHING_DOING).0, &[(1, None), (2, Some(1))]), "no order is excused");
        // A released group stands on, empty, and both actions read that as come true.
        for action in [ACTION_GROUP_DESTROY, ACTION_GROUP_NOTHING_DOING] {
            let (mut p, g) = file(action);
            p.release_group(g);
            assert_eq!(p.group(g).map(|g| g.units.len()), Some(0));
            assert_eq!(p.problem(0).map(|q| q.held), Some(0), "one off the count for each unit");
            assert!(p.released.is_empty(), "and no order goes with it");
            let here = [(1, Some(3)), (2, Some(3))];
            let ended = p.came_true(&p.problem(0).unwrap().actions[0], 1, &world(1, &here));
            assert!(ended, "{action}");
        }
    }

    #[test]
    fn a_retired_problem_leaves_its_slot_so_an_index_a_pass_holds_stays_its_problem() {
        let mut p = planner();
        for i in 0..3 {
            p.raise(&format!("P{i}"), i + 1, 0.5, 1, 0, [i, 0, 0]);
        }
        p.running = Some(0);
        p.set_state(ST_SOLVED);
        assert_eq!(p.problem(2).map(|q| q.code), Some(3), "problem 2 is still problem 2");
        // And the free slot is filled before the list grows.
        let slot = p.raise("P9", 9, 0.5, 1, 0, [9, 0, 0]);
        assert_eq!((slot, p.problems.len()), (0, 3));
    }

    #[test]
    fn a_heavier_problem_takes_a_unit_off_a_lighter_one_and_never_off_a_reservation() {
        let mut p = planner();
        p.raise("LIGHT", 1, 0.3, 1, 0, [1, 0, 0]);
        p.raise("HEAVY", 2, 0.9, 1, 0, [2, 0, 0]);
        p.running = Some(0);
        p.attach(4, Some(false));
        p.running = Some(1);
        assert!(p.may_take(4), "0.3 is below 0.9");
        p.attach(4, Some(false));
        // It leaves a hole in the lighter problem's array, and that record's count stands.
        let light = p.problem(0).unwrap();
        assert_eq!((light.counted().count(), light.held, light.units.clone()), (0, 1, vec![None]));
        assert_eq!(p.problem(1).unwrap().counted().collect::<Vec<_>>(), vec![4]);
        p.running = Some(0);
        assert!(!p.may_take(4), "and the lighter one cannot take it back");
        // A reservation is written on the unit alone: the problem goes on counting it.
        p.reserve(4, true);
        p.running = Some(1);
        assert!(!p.may_take(4) && p.problem(1).unwrap().counted().count() == 1);
        p.reserve(4, false);
        assert!(p.may_take(4) && p.free(4));
    }

    #[test]
    fn take_by_hits_gathers_toward_the_amount_and_answers_what_it_gathered() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 1, 0, [0; 3]);
        p.running = Some(0);
        let units: Vec<Candidate> = (1..=3).map(|i| candidate(i, 2.0)).collect();
        let mut amount = 5.0;
        let g = p.make_group(TAKE_BY_HITS, &units, &mut amount).expect("a group");
        assert_eq!(p.group(g).unwrap().units, vec![1, 2, 3]);
        assert_eq!(amount, 6.0, "three of 2 against an amount of 5");
        p.release_group(g);
        let mut amount = 9.0;
        let g = p.make_group(TAKE_BY_HITS, &units, &mut amount).expect("a group");
        assert_eq!(amount, 6.0, "and against 9 it falls short, which is what the script tests");
        // Every unit is on the problem now, and releasing the group frees them.
        assert_eq!(p.problem(0).unwrap().counted().count(), 3);
        p.release_group(g);
        assert!(units.iter().all(|c| p.free(c.id)));
        // Nothing to gather is `ERROR` and no group.
        let mut amount = 1.0;
        assert_eq!(p.make_group(TAKE_BY_HITS, &[], &mut amount), None);
    }

    #[test]
    fn take_all_battle_units_passes_over_a_problem_of_its_own_code() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 1, 0, [1, 0, 0]);
        p.raise("A2", 1, 0.5, 1, 0, [2, 0, 0]);
        p.raise("B", 2, 0.5, 1, 0, [3, 0, 0]);
        let units: Vec<Candidate> = (1..=3).map(|i| candidate(i, 1.0)).collect();
        p.running = Some(0);
        p.attach(1, Some(false));
        p.running = Some(1);
        p.attach(2, Some(false));
        p.running = Some(2);
        let mut amount = 0.0;
        let g = p.make_group(TAKE_ALL_BATTLE_UNITS, &units, &mut amount).expect("a group");
        // Units 1 and 2 are both on code 1 and the running problem is code 2, so both are of
        // another code and both come, as does the free unit 3.
        assert_eq!(p.group(g).unwrap().units, vec![1, 2, 3]);
    }

    #[test]
    fn the_nearest_capturer_is_the_one_that_reaches_the_target_soonest() {
        let p = planner();
        let near = Candidate { at: [100.0, 0.0], speed: 5.0, ..candidate(1, 1.0) };
        let far = Candidate { at: [300.0, 0.0], speed: 30.0, ..candidate(2, 1.0) };
        let big = Candidate { at: [1.0, 0.0], size_class: 4, ..candidate(3, 1.0) };
        let units = [near, far, big];
        let to = Some([0.0, 0.0]);
        assert_eq!(p.pick_unit(UNIT_ANY_NEAREST_CAPTURER, 0, to, &units), Some(2), "20 s against 10");
        assert_eq!(p.pick_unit(UNIT_ANY_CAPTURER, 0, None, &units), Some(1));
        assert_eq!(p.pick_unit(UNIT_ANY_UNIT, ROBOT_BATTLEUNIT, None, &units), Some(1));
        assert_eq!(p.pick_unit(UNIT_ANY_UNIT, 0x8000_0010, None, &units), None, "no plant here");
    }

    /// A small builder may capture if it is ordered to, but function 14 asks for the exact
    /// Type of a battle unit (`ai.dll:0x10009050`).
    #[test]
    fn a_small_builder_is_no_capturer_to_function_14() {
        let p = planner();
        let builder = Candidate { type_word: ROBOT_BUILDER, ..candidate(1, 1.0) };
        let warbot = Candidate { at: [500.0, 0.0], ..candidate(2, 1.0) };
        let units = [builder, warbot];
        assert_eq!(p.pick_unit(UNIT_ANY_CAPTURER, 0, None, &units), Some(2));
        let to = Some([0.0, 0.0]);
        assert_eq!(p.pick_unit(UNIT_ANY_NEAREST_CAPTURER, 0, to, &units), Some(2));
    }

    /// Function 11's two bars (`ai.dll:0x1000c5e7`): the `dMax*` against what the clan has now,
    /// and the filed count, which nothing lowers, against each type's own number. A type on
    /// neither table never reaches a limit.
    #[test]
    fn a_limit_is_the_variable_or_what_was_ever_filed() {
        const MINE: u32 = 0x8000_0004;
        const TOWER: u32 = 0x8010_0000;
        const INSTITUTE: u32 = 0x8000_0400;
        // None yet, and the variable allows one: not at its limit.
        assert!(!at_limit(MINE, 0, Some(1), 0));
        // One filed and lost: there are none now and the clan will not ask again.
        assert!(at_limit(MINE, 0, Some(1), 1));
        // A larger `dMaxMine` does not help: the second bar is a plain 1.
        assert!(at_limit(MINE, 1, Some(5), 1));
        // A medium tower's bar is three, which its default `dMaxTower` matches.
        assert!(!at_limit(TOWER, 0, Some(3), 2));
        assert!(at_limit(TOWER, 0, Some(3), 3));
        // No variable bounds a research centre, and one filed is the end of them.
        assert!(!at_limit(INSTITUTE, 0, None, 0));
        assert!(at_limit(INSTITUTE, 0, None, 1));
        // `dMaxBuilder` alone bounds a builder, as what the clan has now.
        assert!(at_limit(ROBOT_BUILDER, 0, Some(0), 7));
        assert!(!at_limit(ROBOT_BUILDER, 0, Some(1), 7));
        // And every robot the scripts build answers 0.
        assert!(!at_limit(ROBOT_BATTLEUNIT, 50, None, 50));
    }
}
