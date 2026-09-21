//! The clan's planner: the problem list a SuperAI keeps, the groups inside a problem, and
//! the four passes one clan takt makes over it. See `docs/15-behaviour.md`, "The planner:
//! when a `_Start` runs, and when a `_Continue`", and "What the functions do".
//!
//! Nothing here runs a script. The planner owns the records the script's calls read and
//! write — a problem's weight, its three parameters, its units and its groups — and says
//! which handler the takt runs next; the caller runs it through [`crate::script`].

use std::collections::BTreeSet;

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

/// How many rounds of one takt's `_Start` pass are run before it gives up.
pub const MAX_START_PASSES: usize = 64;

/// One unit on a problem: its logical id, and whether the attach marked it critical
/// (function 6's second operand).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attached {
    pub id: i32,
    pub critical: bool,
}

/// What function 27 files on the running problem: an `ACTION_*` and what it watches.
///
/// STAND-IN: docs/15-behaviour.md#what-the-functions-do -- what the engine below does with
/// an action record is not read, so nothing consumes one: they are kept as the problem's own
/// state and go with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    pub action: u32,
    pub kind: u32,
    pub target: i32,
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
    pub units: Vec<Attached>,
    pub actions: Vec<Action>,
}

impl Problem {
    /// Whether the `_Start` pass may pick it: neither solving nor solved (`0x10001bf0`).
    pub fn startable(&self) -> bool {
        self.state != ST_SOLVING && self.state != ST_SOLVED
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
    /// Whether it may capture: a size class of 1 or 2 (`ai.dll:0x100301a9`).
    pub fn capturer(&self) -> bool {
        CAPTURER_SIZES.contains(&self.size_class)
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
    /// -- the game draws on the CRT's `rand()` plus `timeGetTime()`; the engine keeps one
    /// stream a clan, the generator the clan takt already uses.
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

    /// Function 2 (`ai.dll:0x10009610`): raise the problem `name` names, unless one with the
    /// same code, `p1` and `p2` already stands — which refreshes that one's life counter by
    /// its reload instead (`0x10004c50`, `0x10005070`). The caller has already checked that
    /// the script defines both handlers; a raise whose pair is missing is abandoned and makes
    /// no record (`0x10005aa1`).
    ///
    /// Returns the problem's slot.
    pub fn raise(&mut self, name: &str, code: u32, weight: f32, life: i64, drain: i64, p: [u32; 3]) -> usize {
        let standing = self
            .standing()
            .find(|(_, q)| q.code == code && q.p[0] == p[0] && q.p[1] == p[1])
            .map(|(slot, _)| slot);
        if let Some(slot) = standing {
            if let Some(q) = self.problems[slot].as_mut() {
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
            actions: Vec::new(),
        };
        match self.problems.iter().position(Option::is_none) {
            Some(slot) => {
                self.problems[slot] = Some(problem);
                slot
            }
            None => {
                self.problems.push(Some(problem));
                self.problems.len() - 1
            }
        }
    }

    /// The drain, the first of the takt's four passes (`0x10005910`): every problem loses its
    /// drain from its life counter, and at 0 or below it is retired — its units released and
    /// its slot freed (`0x10005010`).
    pub fn drain(&mut self) {
        for slot in 0..self.problems.len() {
            let Some(p) = self.problems[slot].as_mut() else { continue };
            p.life -= p.drain;
            if p.life <= 0 {
                self.retire(slot);
            }
        }
    }

    /// Retire the problem in `slot`: its groups and units go, and the record with them.
    pub fn retire(&mut self, slot: usize) {
        if self.problems.get(slot).is_none_or(Option::is_none) {
            return;
        }
        self.problems[slot] = None;
        for g in &mut self.groups {
            if g.as_ref().is_some_and(|g| g.problem == slot) {
                *g = None;
            }
        }
        if self.running == Some(slot) {
            self.running = None;
        }
    }

    /// The `_Continue` pass (`0x10001e50`): every `ST_SOLVING` problem, in list order.
    pub fn solving(&self) -> Vec<usize> {
        self.standing().filter(|(_, p)| p.state == ST_SOLVING).map(|(i, _)| i).collect()
    }

    /// One round of the `_Start` pass (`0x10001bf0`): the problems at the largest weight
    /// among those neither solving nor solved. The game repeats the pass while a handler
    /// leaves its problem unstarted, so one takt drains the list from the heaviest down.
    ///
    /// STAND-IN: docs/15-behaviour.md#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured
    /// -- what the repeat excludes is not read. A handler that returns without setting a
    /// state — `PBM_ROBOT_NEEDED_Start` does, when the clan has no factory — would have the
    /// pass pick it again for ever, so a problem already run this takt is not offered again,
    /// which is what "drains the list from the heaviest down" describes.
    pub fn start_batch(&self, done: &BTreeSet<usize>) -> Vec<usize> {
        let open = || {
            self.standing().filter(|(i, p)| !done.contains(i) && p.startable()).map(|(i, p)| (i, p.weight))
        };
        let top = open().map(|(_, w)| w).fold(f32::NEG_INFINITY, f32::max);
        open().filter(|&(_, w)| w == top).map(|(i, _)| i).collect()
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

    /// Function 8: set the running problem's state. `ST_SOLVED` and `ST_UNSOLVED` release its
    /// units first.
    ///
    /// STAND-IN: docs/23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured
    /// -- what `ST_SOLVED` leaves behind is not read. All nine `ORDER_BUILDING_CONSTRUCT`
    /// sites mark their problem solved when the factory refused the build, and the next want
    /// has to raise it afresh — which the raise's duplicate test would block for as long as
    /// the record stood. So a solved problem is retired here, as the drain retires one, and
    /// an unsolved one keeps its slot for the next `_Start` pass.
    pub fn set_state(&mut self, state: u32) {
        let Some(slot) = self.running else { return };
        if state == ST_SOLVED || state == ST_UNSOLVED {
            self.release_units();
        }
        if state == ST_SOLVED {
            self.retire(slot);
            return;
        }
        if let Some(p) = self.current_mut() {
            p.state = state;
        }
    }

    /// Function 6: attach a unit to the running problem, critical or not. A unit function 51
    /// reserved is on no problem and stays that way.
    pub fn attach(&mut self, id: i32, critical: bool) {
        if self.reserved.contains(&id) || self.running.is_none() {
            return;
        }
        // A unit belongs to one problem: it leaves the one it was on.
        self.detach(id);
        if let Some(p) = self.current_mut() {
            p.units.push(Attached { id, critical });
        }
    }

    /// Take `id` off whatever problem or group holds it.
    pub fn detach(&mut self, id: i32) {
        for p in self.problems.iter_mut().flatten() {
            p.units.retain(|u| u.id != id);
        }
        for g in self.groups.iter_mut().flatten() {
            g.units.retain(|&u| u != id);
        }
    }

    /// Function 7: release the running problem's units, and the groups they were in.
    pub fn release_units(&mut self) {
        let Some(slot) = self.running else { return };
        if let Some(p) = self.problems[slot].as_mut() {
            p.units.clear();
        }
        for g in &mut self.groups {
            if g.as_ref().is_some_and(|g| g.problem == slot) {
                *g = None;
            }
        }
    }

    /// Function 27: file an action on the running problem.
    pub fn expect(&mut self, action: Action) {
        if let Some(p) = self.current_mut() {
            p.actions.push(action);
        }
    }

    /// The weight of the problem holding unit `id`; `None` when it is free. A reserved unit
    /// answers a weight no problem can reach, so nobody takes it.
    pub fn holder(&self, id: i32) -> Option<f32> {
        if self.reserved.contains(&id) {
            return Some(f32::INFINITY);
        }
        self.standing().find(|(_, p)| p.units.iter().any(|u| u.id == id)).map(|(_, p)| p.weight)
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

    /// Function 51: reserve a unit from every problem, or free it.
    pub fn reserve(&mut self, id: i32, on: bool) {
        if on {
            self.detach(id);
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
        problem
            .units
            .iter()
            .find(|u| candidates.iter().any(|c| c.id == u.id && c.order == Some(code)))
            .map(|u| u.id)
    }

    /// Function 45: the running problem's units' summed strength.
    pub fn units_strength(&self, candidates: &[Candidate]) -> f32 {
        let Some(problem) = self.current() else { return 0.0 };
        problem.units.iter().filter_map(|u| candidates.iter().find(|c| c.id == u.id)).map(|c| c.full).sum()
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
                    self.standing()
                        .find(|(_, p)| p.units.iter().any(|u| u.id == id))
                        .is_some_and(|(_, p)| p.code != code)
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
        for &id in &units {
            self.attach(id, false);
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

    /// Function 24: release a group.
    pub fn release_group(&mut self, id: usize) {
        if let Some(slot) = self.groups.get_mut(id)
            && let Some(group) = slot.take()
        {
            for unit in group.units {
                self.detach(unit);
            }
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

    #[test]
    fn a_raise_of_the_same_code_and_two_parameters_reloads_the_one_standing() {
        let mut p = planner();
        let i = p.raise("PBM_ROBOT_NEEDED", 6, 0.5, 25, 24, [ROBOT_BATTLEUNIT, 0, SELECT_BEST_COMBAT]);
        assert_eq!((i, p.standing().count()), (0, 1));
        // The same code, p1 and p2: the record's life takes its reload again.
        let j = p.raise("PBM_ROBOT_NEEDED", 6, 0.9, 25, 24, [ROBOT_BATTLEUNIT, 0, SELECT_FASTEST]);
        assert_eq!((j, p.standing().count()), (0, 1));
        let q = p.problem(0).unwrap();
        assert_eq!((q.life, q.weight), (50, 0.5), "reloaded, not re-weighted");
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
                p.drain();
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
    fn the_start_pass_takes_the_heaviest_first_and_then_what_is_left() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 1, 0, [1, 0, 0]);
        p.raise("B", 2, 0.8, 1, 0, [2, 0, 0]);
        p.raise("C", 3, 0.5, 1, 0, [3, 0, 0]);
        let mut done = BTreeSet::new();
        let first = p.start_batch(&done);
        assert_eq!(first, vec![1], "0.8 alone");
        done.extend(first);
        assert_eq!(p.start_batch(&done), vec![0, 2], "then both at 0.5, in list order");
        done.extend([0, 2]);
        assert!(p.start_batch(&done).is_empty());
        // A solving or solved problem is not offered at all.
        let mut q = planner();
        q.raise("A", 1, 0.5, 1, 0, [0; 3]);
        q.running = Some(0);
        q.set_state(ST_SOLVING);
        assert!(q.start_batch(&BTreeSet::new()).is_empty());
        assert_eq!(q.solving(), vec![0]);
    }

    #[test]
    fn solving_a_problem_releases_its_units_and_retires_it() {
        let mut p = planner();
        p.raise("A", 1, 0.5, 5, 2, [0; 3]);
        p.running = Some(0);
        p.attach(7, true);
        assert_eq!(p.holder(7), Some(0.5));
        p.set_state(ST_SOLVED);
        assert!(p.standing().count() == 0 && p.free(7) && p.running.is_none());
        // Unsolved keeps the record for the next pass, without its units.
        let slot = p.raise("A", 1, 0.5, 5, 2, [0; 3]);
        p.running = Some(slot);
        p.attach(7, false);
        p.set_state(ST_UNSOLVED);
        assert_eq!(p.standing().count(), 1);
        assert!(p.free(7) && p.problem(slot).unwrap().startable());
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
        p.attach(4, false);
        p.running = Some(1);
        assert!(p.may_take(4), "0.3 is below 0.9");
        p.attach(4, false);
        assert_eq!(p.problem(0).unwrap().units.len(), 0);
        assert_eq!(p.problem(1).unwrap().units.len(), 1);
        p.running = Some(0);
        assert!(!p.may_take(4), "and the lighter one cannot take it back");
        p.reserve(4, true);
        p.running = Some(1);
        assert!(!p.may_take(4) && p.problem(1).unwrap().units.is_empty());
        p.reserve(4, false);
        assert!(p.may_take(4));
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
        assert_eq!(p.problem(0).unwrap().units.len(), 3);
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
        p.attach(1, false);
        p.running = Some(1);
        p.attach(2, false);
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
}
