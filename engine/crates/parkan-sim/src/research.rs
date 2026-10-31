//! Research as `Behavior.dll` runs it: a clan's tree state, what completing an item opens,
//! and a research centre's task, order 14, with its three budgets. See
//! `docs/16-research.md`, "Researching" and "Completing".

use parkan_formats::research::{AVAILABLE, IN_TREE, RESEARCHED, Tree};

/// `ORDER_ROBOT_RESEARCH`: a technology on a research centre's queue (docs/31).
pub const ORDER_RESEARCH: i32 = 14;
/// Order 16, which `varset.var` does not declare: research what a design still lacks, by its
/// `.dat`'s name, from a clan's function 41 (`ai.dll:0x10011147`, docs/16, "What an AI clan's
/// tree gates").
pub const ORDER_RESEARCH_DESIGN: i32 = 16;
/// A research completes once its stored progress is within this of 1 (`0x1002fe16`).
pub const DONE: f32 = 0.001;
/// `FreeResearchTime` before a mission sets it (`Behavior.dll:0x10003a3c`, docs/23).
pub const FREE_RESEARCH_TIME: f32 = 2.0;
/// The research panel lists at most 50 items (`iron3d.dll:0x10089fd0`).
pub const PANEL_ROWS: usize = 50;
/// While more ore than this is missing, the centre asks for half of it (`0x1002fbdf`).
pub const REQUEST_HALVED_ABOVE: f32 = 5.0;

/// One clan's tree as it stands: each item's three bits (`TRF1`, copied as state) and its
/// stored progress (`+0x38`), with the edges and research costs they are worked from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub bits: Vec<u8>,
    pub progress: Vec<f32>,
    requires: Vec<Vec<usize>>,
    unlocks: Vec<Vec<usize>>,
    /// Research energy and ore.
    costs: Vec<[f32; 2]>,
}

fn edges(list: &[i32], count: usize) -> Vec<usize> {
    list.iter().filter_map(|&i| usize::try_from(i).ok()).filter(|&i| i < count).collect()
}

impl State {
    /// The tree's starting state: its `TRF1` bits, no progress.
    pub fn new(tree: &Tree) -> State {
        let n = tree.items.len();
        State::from_parts(
            tree.items.iter().map(|i| i.category).collect(),
            tree.items.iter().map(|i| [i.values[0], i.values[1]]).collect(),
            tree.items.iter().map(|i| edges(&i.requires, n)).collect(),
            tree.items.iter().map(|i| edges(&i.unlocks, n)).collect(),
        )
    }

    /// A state from its columns: bits, research costs, prerequisites and unlocks per item.
    pub fn from_parts(
        bits: Vec<u8>,
        costs: Vec<[f32; 2]>,
        requires: Vec<Vec<usize>>,
        unlocks: Vec<Vec<usize>>,
    ) -> State {
        let progress = vec![0.0; bits.len()];
        State { bits, progress, requires, unlocks, costs }
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    fn has(&self, i: usize, bit: u8) -> bool {
        self.bits.get(i).is_some_and(|b| b & bit != 0)
    }

    pub fn in_tree(&self, i: usize) -> bool {
        self.has(i, IN_TREE)
    }

    pub fn researched(&self, i: usize) -> bool {
        self.has(i, RESEARCHED)
    }

    pub fn available(&self, i: usize) -> bool {
        self.has(i, AVAILABLE)
    }

    /// Research energy and ore.
    pub fn cost(&self, i: usize) -> [f32; 2] {
        self.costs.get(i).copied().unwrap_or_default()
    }

    /// A research cost other than 0 (`0x10089fd0`).
    pub fn priced(&self, i: usize) -> bool {
        self.cost(i).iter().any(|&c| c != 0.0)
    }

    pub fn unlocks(&self, i: usize) -> &[usize] {
        self.unlocks.get(i).map_or(&[], Vec::as_slice)
    }

    /// `IResearch` slot 7 (`MisLoad.dll:0x10002c10`): an item in the tree becomes available
    /// and researched, and every item in the tree not yet available whose prerequisites all
    /// carry in-tree and researched becomes available. False for an item out of the tree.
    pub fn complete(&mut self, i: usize) -> bool {
        if !self.in_tree(i) {
            return false;
        }
        self.bits[i] |= AVAILABLE | RESEARCHED;
        for j in 0..self.bits.len() {
            if self.in_tree(j)
                && !self.available(j)
                && self.requires[j].iter().all(|&r| self.in_tree(r) && self.researched(r))
            {
                self.bits[j] |= AVAILABLE;
            }
        }
        true
    }

    /// A research centre's completion (`Behavior.dll:0x10023aa0`): slot 7, then every unlock
    /// now in the tree, available, not researched and free to research is completed the same
    /// way, recursively. Returns every item it researched, `i` first.
    pub fn research(&mut self, i: usize) -> Vec<usize> {
        let mut done = Vec::new();
        self.research_into(i, &mut done);
        done
    }

    fn research_into(&mut self, i: usize, done: &mut Vec<usize>) {
        if !self.complete(i) {
            return;
        }
        done.push(i);
        for u in self.unlocks(i).to_vec() {
            if self.in_tree(u) && self.available(u) && !self.researched(u) && !self.priced(u) {
                self.research_into(u, done);
            }
        }
    }

    /// The research panel's rows (`iron3d.dll:0x10089fd0`): items in the tree, available, not
    /// researched and priced, in tree order, at most 50.
    pub fn rows(&self) -> Vec<usize> {
        (0..self.bits.len())
            .filter(|&i| self.in_tree(i) && self.available(i) && !self.researched(i) && self.priced(i))
            .take(PANEL_ROWS)
            .collect()
    }
}

/// A research centre's two grants (`MBehaviour+0x9f4`, `+0x9f8`, docs/23).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grants {
    pub free_technologies: i32,
    pub free_time: f32,
}

impl Default for Grants {
    fn default() -> Self {
        Grants { free_technologies: 0, free_time: FREE_RESEARCH_TIME }
    }
}

/// The technology a task has in hand: its costs and what is collected, each as time, ore,
/// power.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Budget {
    pub item: usize,
    pub cost: [f32; 3],
    pub got: [f32; 3],
}

impl Budget {
    /// The smallest of the three fractions, a zero cost's counting as 1.
    pub fn fraction(&self) -> f32 {
        let share = |k: usize| if self.cost[k] > 0.0 { self.got[k] / self.cost[k] } else { 1.0 };
        share(0).min(share(1)).min(share(2))
    }
}

/// What a centre's efficiency and profile give a takt: KPD, `Use_Ore` and `Use_Power`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rates {
    pub kpd: f32,
    pub use_ore: f32,
    pub use_power: f32,
}

/// What a takt leaves the centre doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Takt {
    /// No technology qualifies: the task is over, its ore request and power use cleared
    /// (`0x1002fa37`).
    Over,
    /// At work: the ore it asks the distribution for, and its power use `Use_Power`.
    Working { request: f32 },
    /// Its technology completed, researching these; the task runs on to the next takt.
    Researched(Vec<usize>),
}

/// `M_Task_Research` (`Behavior.dll`, table `0x10059f58`): the technologies it was given
/// (`+0x58`) and the one in hand (`+0x6c`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Task {
    pub list: Vec<usize>,
    pub current: Option<Budget>,
    /// The request its last takt set.
    pub request: f32,
}

impl Task {
    /// `SetTarget` with target `0x204` (`0x1002f470`): the id appended to the list.
    pub fn new(item: usize) -> Task {
        Task { list: vec![item], current: None, request: 0.0 }
    }

    /// The technologies an order 16 names, in the order it lists them (`SetTarget` with target
    /// `0x205`, `Behavior.dll:0x10023790`).
    pub fn with_list(list: Vec<usize>) -> Task {
        Task { list, current: None, request: 0.0 }
    }

    /// One takt of `dt` seconds (`0x1002f730`) against clan tree `state`, with the centre's
    /// `grants` and `rates`, and `held` the centre's ore property.
    ///
    /// What the property holds is what the distribution has brought it since its last takt, for
    /// every takt writes 0 over it with the request as its most (`0x1002fc27`): the take reads
    /// it and the rest is gone. Starting a technology (`0x1002f98d`) and ending the task
    /// (`0x1002fa6d`) zero it too.
    pub fn takt(
        &mut self,
        state: &mut State,
        grants: &mut Grants,
        rates: Rates,
        dt: f32,
        held: &mut f32,
    ) -> Takt {
        if self.current.is_none() {
            // The first id not researched and available becomes current (`0x1002f8db`).
            let Some(item) = self.list.iter().copied().find(|&i| !state.researched(i) && state.available(i))
            else {
                self.request = 0.0;
                *held = 0.0;
                return Takt::Over;
            };
            let [power, ore] = if grants.free_technologies > 0 {
                grants.free_technologies -= 1;
                [0.0, 0.0]
            } else {
                state.cost(item)
            };
            self.current = Some(Budget { item, cost: [grants.free_time, ore, power], got: [0.0; 3] });
            self.request = ore;
            *held = 0.0;
        }
        let Some(mut budget) = self.current else { return Takt::Over };
        // Already researched, by another centre say: dropped (`"allready researched"`).
        if state.researched(budget.item) {
            self.current = None;
            return Takt::Working { request: self.request };
        }
        let old = budget.fraction();
        let take = (rates.kpd * rates.use_ore * dt).min(held.max(0.0));
        *held = 0.0;
        budget.got[1] += take;
        let missing = budget.cost[1] - budget.got[1];
        self.request = if missing <= 0.0 {
            0.0
        } else if missing > REQUEST_HALVED_ABOVE {
            missing * 0.5
        } else {
            missing
        };
        budget.got[2] += rates.kpd * rates.use_power * dt;
        budget.got[0] += dt;
        let new = budget.fraction();
        let item = budget.item;
        if let Some(p) = state.progress.get_mut(item) {
            *p = (*p + new - old).min(1.0);
        }
        self.current = Some(budget);
        if state.progress.get(item).is_some_and(|&p| p + DONE >= 1.0) {
            let researched = state.research(item);
            if let Some(p) = state.progress.get_mut(item) {
                *p = 1.0;
            }
            self.current = None;
            return Takt::Researched(researched);
        }
        Takt::Working { request: self.request }
    }
}

/// Whether every part of a design passes `test`; a part the tree does not list passes nothing
/// (`IResearch` slot 2's −1, `ai.dll:0x1001161e`, `0x10011568`).
fn every(items: &[Option<usize>], test: impl Fn(usize) -> bool) -> bool {
    items.iter().all(|i| i.is_some_and(&test))
}

/// One record of a clan's design store as its research reads it: the scheme's file, the item
/// each of its parts is researched by (`None`, a part the tree does not list), and the byte at
/// `+0x104`.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub file: String,
    pub items: Vec<Option<usize>>,
    pub may_build: bool,
}

/// A clan's design store's research (`ai.dll:0x10010f90`, `0x10011100`, docs/15, "The byte at
/// `+0x104` is the clan's research"): the records, the index at `+0xc`, which is the design it
/// researches next, and the byte at `+0x25`, which says nothing is left to research.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Book {
    pub records: Vec<Record>,
    pub cursor: usize,
    pub everything: bool,
}

/// What no design costs (`0x10010fca`, `0x4b18967f`): the walk's first best price.
const NO_PRICE: f32 = 9_999_999.0;

impl Book {
    /// The walk (`0x10010f90`). A record whose byte is set is passed over: once allowed, always
    /// allowed. For any other, a part out of the tree or not yet available leaves its byte
    /// clear and is done with; the byte becomes whether every part is researched; and of the
    /// designs left open the cheapest, first of an equal pair, goes to `+0xc`
    /// (`0x100110a0`-`0x100110c3`). With none open `+0xc` keeps what it had.
    pub fn refresh(&mut self, state: &State) {
        self.everything = true;
        let mut best = NO_PRICE;
        for (i, r) in self.records.iter_mut().enumerate() {
            if r.may_build {
                continue;
            }
            self.everything = false;
            if !every(&r.items, |p| state.available(p) && state.in_tree(p)) {
                continue;
            }
            r.may_build = every(&r.items, |p| state.researched(p) && state.in_tree(p));
            let price = Self::price(&r.items, state);
            if price < best && !r.may_build {
                best = price;
                self.cursor = i;
            }
        }
    }

    /// `ArealMap.dll:CalcFullResearchCost` (`0x10016280`) as the walk reads it, its first
    /// answer: the research ore of each part not researched, a part counted once. Every part
    /// of a design the walk prices is available, so no parent enters it (`0x10016644`).
    fn price(items: &[Option<usize>], state: &State) -> f32 {
        Self::missing(items, state).into_iter().map(|i| state.cost(i)[1]).sum()
    }

    fn missing(items: &[Option<usize>], state: &State) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        for i in items.iter().flatten().copied() {
            if !state.researched(i) && !out.contains(&i) {
                out.push(i);
            }
        }
        out
    }

    /// The record function 41 orders researched (`0x10011100`), the one at `+0xc`, unless its
    /// byte is set: then there is no order.
    pub fn next(&self) -> Option<usize> {
        self.records.get(self.cursor).filter(|r| !r.may_build).map(|_| self.cursor)
    }

    /// What an order 16 for record `record` lists (`Behavior.dll:0x10023790`): each part not
    /// researched, in the scheme's order, once. It would add a part's unresearched parents
    /// (`0x100235e0`), and a design the walk chooses has none.
    pub fn technologies(&self, record: usize, state: &State) -> Vec<usize> {
        self.records.get(record).map_or_else(Vec::new, |r| Self::missing(&r.items, state))
    }

    /// The byte of the record whose file is `file`.
    pub fn may_build(&self, file: &str) -> bool {
        self.records.iter().any(|r| r.may_build && r.file.eq_ignore_ascii_case(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Five items: 0 granted, 1 open at 12 energy and 35 ore needing 0, 2 free needing 1,
    /// 3 free needing 2, 4 out of the tree needing 1.
    fn chain() -> State {
        State::from_parts(
            vec![7, 5, 4, 4, 0],
            vec![[0.0; 2], [12.0, 35.0], [0.0; 2], [0.0; 2], [5.0, 5.0]],
            vec![vec![], vec![0], vec![1], vec![2], vec![1]],
            vec![vec![1], vec![2, 4], vec![3], vec![], vec![]],
        )
    }

    #[test]
    fn completing_an_item_opens_its_unlocks_and_researches_the_free_ones_at_once() {
        let mut s = chain();
        assert_eq!(s.rows(), vec![1]);
        assert_eq!(s.research(1), vec![1, 2, 3], "the free chain follows");
        assert!(s.researched(3) && s.available(3));
        assert!(!s.available(4) && !s.researched(4), "an item out of the tree never opens");
        assert!(s.rows().is_empty());
        assert!(!s.complete(4), "slot 7 needs the in-tree bit");
    }

    #[test]
    fn a_free_technology_takes_only_the_centres_free_time() {
        let mut s = chain();
        let mut grants = Grants { free_technologies: 5, free_time: 5.0 };
        let rates = Rates { kpd: 7.0, use_ore: 1.0, use_power: 3.0 };
        let mut task = Task::new(1);
        let (mut t, mut held) = (0.0_f32, 0.0_f32);
        let dt = 1.0 / 60.0;
        let researched = loop {
            t += dt;
            match task.takt(&mut s, &mut grants, rates, dt, &mut held) {
                Takt::Researched(r) => break r,
                Takt::Working { request } => assert_eq!(request, 0.0),
                Takt::Over => panic!("over at {t}"),
            }
        };
        assert!((t - 5.0).abs() < 2.0 * dt, "done at {t}");
        assert_eq!(researched, vec![1, 2, 3]);
        assert_eq!(grants.free_technologies, 4);
        assert_eq!(s.progress[1], 1.0);
        assert_eq!(task.takt(&mut s, &mut grants, rates, dt, &mut held), Takt::Over, "the next takt ends it");
    }

    #[test]
    fn a_paid_research_progresses_by_its_slowest_budget_and_takes_ore_at_kpd_times_use() {
        let mut s = chain();
        let mut grants = Grants::default();
        let rates = Rates { kpd: 7.0, use_ore: 1.0, use_power: 3.0 };
        let mut task = Task::new(1);
        // The first takt starts the technology, which zeroes the property: nothing to take.
        let mut held = 100.0;
        let takt = task.takt(&mut s, &mut grants, rates, 0.0, &mut held);
        assert_eq!(held, 0.0, "`0x1002f98d`");
        assert_eq!(takt, Takt::Working { request: 17.5 }, "35 missing, halved");
        // One second with 100 ore arrived: time 1 of 2, power 21 of 12, ore 7 of 35.
        let mut held = 100.0;
        let takt = task.takt(&mut s, &mut grants, rates, 1.0, &mut held);
        assert_eq!(held, 0.0, "the takt writes 0 over what it did not take, `0x1002fc27`");
        assert_eq!(takt, Takt::Working { request: 14.0 }, "28 missing, halved");
        assert!((s.progress[1] - 0.2).abs() < 1e-6, "the ore's 7/35: {}", s.progress[1]);
        // With no ore the time and power run on and progress holds.
        let mut none = 0.0;
        task.takt(&mut s, &mut grants, rates, 1.0, &mut none);
        assert!((s.progress[1] - 0.2).abs() < 1e-6);
        // Held to what arrives: 3 ore now, then the last 25 over four seconds.
        let mut three = 3.0;
        task.takt(&mut s, &mut grants, rates, 1.0, &mut three);
        assert_eq!(three, 0.0);
        assert!((s.progress[1] - 10.0 / 35.0).abs() < 1e-5);
        let mut last = Takt::Over;
        for _ in 0..4 {
            let mut plenty = 1000.0;
            last = task.takt(&mut s, &mut grants, rates, 1.0, &mut plenty);
        }
        assert_eq!(last, Takt::Researched(vec![1, 2, 3]));
    }

    /// A step brings a small centre 0.224 ore and the takt after it may take 0.1: the 0.124 is
    /// burnt, the takts before the next step find nothing, and the clan pays for 0.224 a step
    /// what research collects at 0.1 (docs/16, "The ore a centre takes").
    #[test]
    fn a_step_more_than_a_takt_may_take_is_burnt_and_the_next_takts_find_nothing() {
        let mut s = chain();
        let mut grants = Grants::default();
        let rates = Rates { kpd: 1.0, use_ore: 1.0, use_power: 3.0 };
        let mut task = Task::new(1);
        let mut held = 0.0;
        task.takt(&mut s, &mut grants, rates, 0.1, &mut held);
        let (mut collected, mut paid) = (0.0, 0.0);
        for _ in 0..10 {
            // A distribution step's delivery, and the two takts that fall before the next.
            held += 0.224;
            paid += 0.224;
            task.takt(&mut s, &mut grants, rates, 0.1, &mut held);
            assert_eq!(held, 0.0);
            task.takt(&mut s, &mut grants, rates, 0.1, &mut held);
            collected += 0.1;
        }
        let got = task.current.unwrap().got[1];
        assert!((got - collected).abs() < 1e-4, "{got}, of {paid} paid");
    }

    /// A book over [`chain`]'s tree: `granted` all researched, `open` needing the priced item 1
    /// and `waits` the free item 2 behind it, `out` item 4, out of the tree, and `unknown` a
    /// part the tree does not list.
    fn book() -> Book {
        let record = |file: &str, items: &[Option<usize>]| Record {
            file: file.into(),
            items: items.to_vec(),
            may_build: false,
        };
        Book {
            records: vec![
                record("out", &[Some(0), Some(4)]),
                record("unknown", &[Some(0), None]),
                record("waits", &[Some(0), Some(2)]),
                record("open", &[Some(0), Some(1), Some(1)]),
                record("granted", &[Some(0)]),
            ],
            cursor: 0,
            everything: false,
        }
    }

    #[test]
    fn a_refresh_marks_what_is_researched_and_points_at_the_cheapest_open_design_and_never_unmarks() {
        let mut s = chain();
        let mut b = book();
        b.refresh(&s);
        assert!(b.may_build("GRANTED") && !b.may_build("open"), "the file's case is not the key");
        assert_eq!(b.next(), Some(3), "the open one; out of the tree and waiting are never chosen");
        assert!(!b.everything);
        assert_eq!(b.technologies(3, &s), [1], "each part once, and only what is not researched");
        assert_eq!(Book::price(&b.records[3].items, &s), 35.0, "the research ore, once");

        s.research(1);
        b.refresh(&s);
        assert!(b.may_build("open") && b.may_build("waits"), "the free item came with it");
        // Nothing is open now: `+0xc` keeps the design it had, whose byte is set, so no order.
        assert_eq!(b.cursor, 3);
        assert_eq!(b.next(), None);
        assert!(!b.everything, "`out` and `unknown` will never be researched");

        // A byte once set stands, whatever the tree says of it.
        s.bits[1] = 4;
        b.refresh(&s);
        assert!(b.may_build("open"));
    }

    #[test]
    fn a_book_with_nothing_left_says_so_only_at_the_refresh_after_every_byte_is_set() {
        let s = chain();
        let granted = Record { file: "granted".into(), items: vec![Some(0)], may_build: false };
        let mut b = Book { records: vec![granted], cursor: 0, everything: false };
        b.refresh(&s);
        assert!(!b.everything, "it was clear as the walk came to it");
        b.refresh(&s);
        assert!(b.everything);
    }

    #[test]
    fn a_cancelled_research_keeps_its_progress_and_a_new_task_needs_only_the_rest() {
        let mut s = chain();
        let mut grants = Grants::default();
        let rates = Rates { kpd: 1.0, use_ore: 100.0, use_power: 100.0 };
        // A takt to start it, and each after with the ore a step brought.
        let mut takt = |task: &mut Task, dt: f32| {
            let mut held = 1000.0;
            task.takt(&mut s, &mut grants, rates, dt, &mut held)
        };
        let mut first = Task::new(1);
        takt(&mut first, 0.0);
        takt(&mut first, 1.0);
        let mut again = Task::new(1);
        takt(&mut again, 0.0);
        let working = takt(&mut again, 0.5);
        assert!(matches!(working, Takt::Working { .. }));
        let researched = takt(&mut again, 0.5);
        assert!(matches!(researched, Takt::Researched(_)), "time 1 of 2, then the half left");
    }
}
