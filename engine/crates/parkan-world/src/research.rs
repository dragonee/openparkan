//! Research in play: each clan's research tree as it stands, every research centre with its
//! grants, its queue of order 14s and its task, the orders the research panel gives, and the
//! check that reports each research completed. See `docs/16-research.md`, "Researching", and
//! `docs/41-commander.md`, "The research page, 4".

use std::path::Path;

use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::research::{self, Tree};
use parkan_sim::research::{Grants, Rates, State, Takt, Task};

use crate::designs::Catalogue;
use crate::play::{Play, Unit};
use crate::progress::Sender;
use crate::selection::RESEARCH_CENTRE;

/// The mission properties a research centre is granted with (docs/23, "The four grants").
pub const FREE_TECHNO_NUM: &str = "FreeTechnoNum";
pub const FREE_RESEARCH_TIME: &str = "FreeResearchTime";
/// *"Research complete..."*, and the voice with it (`iron3d.dll:0x10089a50`).
pub const STRING_RESEARCH_COMPLETE: u32 = 2010;
pub const VOICE_RSRCH_COMPLETE: &str = "VOICE_RSRCH_COMPLETE";

/// One clan's tree: the archive as read, and its state.
#[derive(Clone, Debug)]
pub struct ClanTree {
    pub tree: Tree,
    pub state: State,
}

/// A research centre: its target, its grants, its queue of technologies
/// (each an order 14, the running one first) and the task running the first.
#[derive(Clone, Debug, PartialEq)]
pub struct Centre {
    pub target: usize,
    pub grants: Grants,
    pub orders: Vec<usize>,
    pub task: Option<Task>,
    /// The clock its task last read, `+0x50`: stamped as it starts and at each takt.
    pub started_ms: f64,
}

/// What research keeps.
#[derive(Clone, Debug, Default)]
pub struct Research {
    /// Each clan's tree, by clan index; none where the clan names no tree that reads.
    pub trees: Vec<Option<ClanTree>>,
    /// `[CS] FULL_RESEARCH_TREE`: every part is offered.
    pub full: bool,
    pub centres: Vec<Centre>,
    /// How many researches the check has reported, which the panel refreshes on.
    pub reported: u64,
    /// Bumped whenever a tree changes.
    pub version: u64,
}

fn number(v: Value) -> f32 {
    match v {
        Value::Int(i) => i as f32,
        Value::Float(f) => f,
    }
}

impl Research {
    /// Every clan's tree from its `behaviour`, and every placed research centre with the grants
    /// its mission object carries; `objects` is the mission object each target is.
    pub fn load(game: &Path, mission: &Mission, objects: &[usize], units: &[Unit]) -> Research {
        let trees = mission
            .clans
            .iter()
            .map(|c| {
                let file = parkan_formats::gamedir::resolve(game, &c.behaviour)?;
                let tree = research::parse(&std::fs::read(file).ok()?, &c.behaviour).ok()?;
                let state = State::new(&tree);
                Some(ClanTree { tree, state })
            })
            .collect();
        let full = crate::settings::value(game, "CS", "FULL_RESEARCH_TREE")
            .is_some_and(|v| v.parse::<i64>().ok().is_some_and(|n| n != 0));
        let centres = units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.kind == mission::KIND_BUILDING && u.type_word == RESEARCH_CENTRE)
            .map(|(t, _)| {
                let placed = objects.get(t).and_then(|&o| mission.objects.get(o));
                let grant = |name: &str| placed.and_then(|p| p.property(name)).map(|p| number(p.value));
                let mut grants = Grants::default();
                if let Some(n) = grant(FREE_TECHNO_NUM) {
                    grants.free_technologies = n as i32;
                }
                if let Some(s) = grant(FREE_RESEARCH_TIME) {
                    grants.free_time = s;
                }
                Centre { target: t, grants, orders: Vec::new(), task: None, started_ms: 0.0 }
            })
            .collect();
        Research { trees, full, centres, reported: 0, version: 0 }
    }

    /// Clan `clan`'s tree.
    pub fn clan(&self, clan: i64) -> Option<&ClanTree> {
        usize::try_from(clan).ok().and_then(|c| self.trees.get(c)).and_then(Option::as_ref)
    }

    /// Clan `clan`'s tree as the constructor reads it: the archive with its state's bits.
    pub fn catalogue(&self, clan: i64) -> Option<Catalogue> {
        let c = self.clan(clan)?;
        let mut tree = c.tree.clone();
        for (item, &bits) in tree.items.iter_mut().zip(&c.state.bits) {
            item.category = bits;
        }
        Some(Catalogue::new(tree, self.full))
    }
}

impl Play {
    /// The player clan's tree as the constructor reads it, with what it has researched since.
    pub fn catalogue(&self) -> Option<Catalogue> {
        self.research.catalogue(self.player_clan)
    }

    /// The research panel's rows for the player's clan (`0x10089fd0`).
    pub fn research_rows(&self) -> Vec<usize> {
        self.research.clan(self.player_clan).map(|c| c.state.rows()).unwrap_or_default()
    }

    /// Every research centre standing joins the list an order may go to: those the mission
    /// placed, at load, and one built since, which carries `MBehaviour`'s own grants --
    /// no free technology and a two-second research time (docs/23, "The four grants").
    pub fn join_centres(&mut self) {
        for t in 0..self.units.len() {
            if self.units[t].kind != mission::KIND_BUILDING
                || self.units[t].type_word != RESEARCH_CENTRE
                || self.research.centres.iter().any(|c| c.target == t)
            {
                continue;
            }
            let centre = Centre {
                target: t,
                grants: Grants::default(),
                orders: Vec::new(),
                task: None,
                started_ms: 0.0,
            };
            self.research.centres.push(centre);
        }
    }

    /// The player's research centres an order may go to (`0x10087c00`, list `0x1010c36c`):
    /// those of the clan that stand, in target order.
    fn player_centres(&self) -> Vec<usize> {
        (0..self.research.centres.len())
            .filter(|&c| {
                let t = self.research.centres[c].target;
                self.units.get(t).is_some_and(|u| u.clan == Some(self.player_clan))
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
            })
            .collect()
    }

    /// Whether an order 14 for `item` sits at any of the player's research centres.
    pub fn research_queued(&self, item: usize) -> bool {
        self.player_centres().iter().any(|&c| self.research.centres[c].orders.contains(&item))
    }

    /// Order technology `item` researched (`0x100880b0`): unless it is researched or queued, to
    /// the end of the queue of the player's centre that is not destroyed, is not building
    /// itself and holds the fewest orders. False when nothing is ordered.
    pub fn order_research(&mut self, item: usize) -> bool {
        let researched = self.research.clan(self.player_clan).is_none_or(|c| c.state.researched(item));
        if researched || self.research_queued(item) {
            return false;
        }
        let Some(c) = self
            .player_centres()
            .into_iter()
            .filter(|&c| !self.building_itself(self.research.centres[c].target))
            .min_by_key(|&c| self.research.centres[c].orders.len())
        else {
            return false;
        };
        self.research.centres[c].orders.push(item);
        true
    }

    /// Abort every order 14 for `item` at the player's centres (`IAgent` slot 40): a running
    /// task stops, its request and power use cleared; what it collected is not refunded and
    /// the item keeps its progress.
    pub fn cancel_research(&mut self, item: usize) {
        for c in self.player_centres() {
            let Some(at) = self.research.centres[c].orders.iter().position(|&i| i == item) else { continue };
            self.abort_research(c, at);
        }
    }

    /// The batch button's cancel (`0x10087f10`): every queued research at every centre.
    pub fn cancel_all_research(&mut self) {
        for c in self.player_centres() {
            while !self.research.centres[c].orders.is_empty() {
                self.abort_research(c, 0);
            }
        }
    }

    fn abort_research(&mut self, c: usize, at: usize) {
        let centre = &mut self.research.centres[c];
        centre.orders.remove(at);
        if at == 0 && centre.task.take().is_some() {
            let t = centre.target;
            self.stop_research_draw(t);
        }
    }

    /// A task's end and its stop (`0x1002f400`): the ore property to 0 of 0, what it held with
    /// it, and no power use.
    fn stop_research_draw(&mut self, t: usize) {
        if let Some(site) = self.economy.site_mut(t) {
            site.usage = 0.0;
        }
        self.economy.ore.insert(t, (0.0, 0.0));
    }

    /// A clan's function 41 at the research centre that is target `t`: order 16 for the store's
    /// record `record`, replacing what the centre had (`ai.dll:0x100111a1`), with the parts the
    /// design still lacks as the task's list (`Behavior.dll:0x10023790`). With nothing to list
    /// the game's task ends at its first takt, so here there is none.
    pub(crate) fn order_design_research(&mut self, t: usize, record: usize) {
        let Some(clan) = self.units.get(t).and_then(|u| u.clan) else { return };
        let list = self
            .progression
            .as_ref()
            .and_then(|p| p.desk(clan))
            .map_or_else(Vec::new, |d| d.book.technologies(record, &d.state));
        self.join_centres();
        let Some(c) = self.research.centres.iter().position(|c| c.target == t) else { return };
        while !self.research.centres[c].orders.is_empty() {
            self.abort_research(c, 0);
        }
        let Some(&first) = list.first() else { return };
        let now = self.hero.time_ms;
        let centre = &mut self.research.centres[c];
        centre.orders.push(first);
        centre.task = Some(Task::with_list(list));
        centre.started_ms = now;
    }

    /// Every research centre's order for this tick: a first order with no task yet starts one,
    /// which stamps the clock (`0x1002f710`), and on the building's takt (`0x100054fd`) the task
    /// runs (`0x1002f730`) over the time since against its clan's tree, drawing `Use_Power`
    /// while it works and asking the distribution for its ore; a task that is over ends its
    /// order and the next starts.
    pub fn tick_research(&mut self, now: f64) {
        self.join_centres();
        for c in 0..self.research.centres.len() {
            let t = self.research.centres[c].target;
            let alive = self.battle.combat.targets.get(t).is_some_and(|x| x.alive);
            let Some(clan) = self.units.get(t).and_then(|u| u.clan).filter(|_| alive) else { continue };
            let Some(&item) = self.research.centres[c].orders.first() else { continue };
            let centre = &mut self.research.centres[c];
            if centre.task.is_none() {
                centre.task = Some(Task::new(item));
                centre.started_ms = now;
            }
            if self.economy.site(t).is_some_and(|s| !s.takt_due) {
                continue;
            }
            let rates = self.economy.site(t).map_or(Rates { kpd: 1.0, use_ore: 0.0, use_power: 0.0 }, |s| {
                Rates { kpd: s.kpd(), use_ore: s.use_ore, use_power: s.use_power }
            });
            let mut held = self.economy.held(t);
            let Research { trees, centres, .. } = &mut self.research;
            let Some(Some(tree)) = usize::try_from(clan).ok().and_then(|i| trees.get_mut(i)) else {
                continue;
            };
            let centre = &mut centres[c];
            let dt = ((now - centre.started_ms) / 1000.0) as f32;
            centre.started_ms = now;
            let Some(task) = centre.task.as_mut() else { continue };
            let takt = task.takt(&mut tree.state, &mut centre.grants, rates, dt, &mut held);
            let request = task.request;
            match takt {
                Takt::Over => {
                    centre.orders.remove(0);
                    centre.task = None;
                    self.stop_research_draw(t);
                    continue;
                }
                Takt::Researched(items) => {
                    if !items.is_empty() {
                        self.research.version += 1;
                        self.researched_changed();
                    }
                }
                Takt::Working { .. } => {}
            }
            if let Some(site) = self.economy.site_mut(t) {
                site.usage = site.use_power;
            }
            self.economy.ore.insert(t, (held, request));
        }
    }

    /// What the tree feeds forgets it: the build rows' marks and the building costs.
    fn researched_changed(&mut self) {
        self.commander.researched = None;
        self.construction.costs.clear();
    }

    /// Hand each clan's research desk its tree as it now stands, which function 41 walks. A tree
    /// moves only as a research completes.
    pub(crate) fn sync_desks(&mut self) {
        let Some(p) = self.progression.as_mut() else { return };
        if p.desks_version == self.research.version {
            return;
        }
        p.desks_version = self.research.version;
        for clan in 0..self.research.trees.len() as i64 {
            if let Some(tree) = self.research.clan(clan) {
                p.set_tree(clan, &tree.state);
            }
        }
    }

    /// A centre's order, as the clan's takt asks whether it has nothing doing (docs/16, "When the
    /// clan asks again"): its order list is the queue here, headed by a technology's or a
    /// design's research, neither of which reads as idle.
    pub(crate) fn centre_order(&self, t: usize) -> Option<i32> {
        let busy = self.research.centres.iter().any(|c| c.target == t && !c.orders.is_empty());
        busy.then_some(parkan_sim::research::ORDER_RESEARCH_DESIGN)
    }

    /// The game loop's check of the queued research (`iron3d.dll:0x10089a50`): each order 14 at
    /// the player's centres whose technology now reads researched is aborted, and reported
    /// with *"Research complete... (name)"* from System and `VOICE_RSRCH_COMPLETE`.
    pub fn check_research(&mut self) {
        let Some(tree) = self.research.clan(self.player_clan) else { return };
        let mut done = Vec::new();
        for c in self.player_centres() {
            for (at, &item) in self.research.centres[c].orders.iter().enumerate() {
                if tree.state.researched(item) {
                    done.push((c, at, item));
                }
            }
        }
        if done.is_empty() {
            return;
        }
        for &(c, at, _) in done.iter().rev() {
            self.abort_research(c, at);
        }
        for (_, _, item) in done {
            let name = self
                .research
                .clan(self.player_clan)
                .and_then(|t| t.tree.items.get(item))
                .map(|i| i.name.clone())
                .unwrap_or_default();
            if let Some(p) = self.progression.as_ref() {
                let text = p.strings.get(&STRING_RESEARCH_COMPLETE).cloned().unwrap_or_default();
                self.says.push(crate::progress::Say::Text(Sender::System, format!("{text} ({name})")));
                self.says.extend(p.sound(VOICE_RSRCH_COMPLETE).map(crate::progress::Say::Voice));
            }
            self.research.reported += 1;
        }
    }
}
