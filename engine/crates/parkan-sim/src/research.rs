//! Research as `Behavior.dll` runs it: a clan's tree state, what completing an item opens,
//! and a research centre's task, order 14, with its three budgets. See
//! `docs/16-research.md`, "Researching" and "Completing".

use parkan_formats::research::{AVAILABLE, IN_TREE, RESEARCHED, Tree};

/// `ORDER_ROBOT_RESEARCH`: a technology on a research centre's queue (docs/31).
pub const ORDER_RESEARCH: i32 = 14;
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

    /// One takt of `dt` seconds (`0x1002f730`) against clan tree `state`, with the centre's
    /// `grants` and `rates`, and the ore it holds `held`, from which it takes.
    ///
    /// STAND-IN: docs/16-research.md#not-established -- whether property `0x2000100`'s get is
    /// the ore the centre holds or what the distribution delivered is not read: the take comes
    /// out of what the economy says it holds, and the request is what it asks the next step for,
    /// as a factory's build does.
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
        }
        let Some(mut budget) = self.current else { return Takt::Over };
        // Already researched, by another centre say: dropped (`"allready researched"`).
        if state.researched(budget.item) {
            self.current = None;
            return Takt::Working { request: self.request };
        }
        let old = budget.fraction();
        let take = (rates.kpd * rates.use_ore * dt).min(held.max(0.0));
        *held -= take;
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
        // One second with 100 ore held: time 1 of 2, power 21 of 12, ore 7 of 35.
        let mut held = 100.0;
        let takt = task.takt(&mut s, &mut grants, rates, 1.0, &mut held);
        assert!((held - 93.0).abs() < 1e-4, "{held}");
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
        let mut plenty = 1000.0;
        let mut last = Takt::Over;
        for _ in 0..4 {
            last = task.takt(&mut s, &mut grants, rates, 1.0, &mut plenty);
        }
        assert_eq!(last, Takt::Researched(vec![1, 2, 3]));
    }

    #[test]
    fn a_cancelled_research_keeps_its_progress_and_a_new_task_needs_only_the_rest() {
        let mut s = chain();
        let mut grants = Grants::default();
        let rates = Rates { kpd: 1.0, use_ore: 100.0, use_power: 100.0 };
        let mut held = 1000.0;
        Task::new(1).takt(&mut s, &mut grants, rates, 1.0, &mut held);
        assert!((s.progress[1] - 0.5).abs() < 1e-6, "time 1 of 2");
        let mut again = Task::new(1);
        let first = again.takt(&mut s, &mut grants, rates, 0.5, &mut held);
        assert!(matches!(first, Takt::Working { .. }));
        assert!((s.progress[1] - 0.75).abs() < 1e-6, "{}", s.progress[1]);
        assert!(matches!(again.takt(&mut s, &mut grants, rates, 0.5, &mut held), Takt::Researched(_)));
    }
}
