//! A clan's ore and power: what each unit and building holds, every building's batteries and
//! efficiency, a mine's digging, the distribution step that shares ore and power out, and the
//! Ore and Energy rows the commander reads. See `docs/23-economy.md`, "Mission 03's economy,
//! tick by tick".

use std::collections::{BTreeSet, HashMap};

use parkan_formats::mission::{KIND_BUILDING, Mission, Value};
use parkan_formats::{control, gamedir, nres, profiles};
use parkan_sim::economy::{
    Battery, Dug, EFFICIENCY_POWER, LODE_REACH, MINE_MAX_ORE, Mine, ModuleRand, ORE_ROW_SCALE, POWER_TICK_MS,
    STEP_MS, STORAGE_MAX_ORE, ShiftJitter, WANT_FLOOR, share,
};

use crate::assembly::Assembly;
use crate::play::Play;

/// The mission properties a unit's ore is placed with.
pub const CURRENT_ORE: &str = "CurrentOre";
pub const MAXIMUM_ORE: &str = "MaximumOre";
/// The building Types the economy tells apart: a generator and the three bunkers give power;
/// a mine and a storage hold ore others draw (`Behavior.dll:0x10019e80`).
pub const GENERATOR: u32 = 0x8000_0002;
pub const MINE: u32 = 0x8000_0004;
pub const STORAGE: u32 = 0x8000_0008;
pub const BUNKERS: [u32; 3] = [0x8001_0000, 0x8002_0000, 0x8004_0000];
/// The two unit Types a mine's loading place and a storage's unloading place move ore with by
/// themselves (`Behavior.dll:0x100194db`).
pub const TRANSPORT: u32 = 0x0100_2000;
pub const BUILDER: u32 = 0x0100_4000;
/// The power tick's shift register's seed.
///
/// STAND-IN: docs/23-economy.md#how-often-and-where-it-settles--read-with-a-derived-settle-point
/// -- `Control.dll` seeds its two words from an `Ngi32.dll` import by ordinal as it loads
/// (`0x1000dc34`), which is not followed: a fixed dword. And `Behavior.dll`'s `rand()` is one
/// stream for the whole module, which 65 other call sites draw on between two steps; the
/// engine's step draws on it alone.
pub const JITTER_SEED: u32 = 0x2545_f491;
/// The efficiency component, `CICLS_` 26 (docs/23, "Efficiency is a building's size").
pub const EFFICIENCY_TYPE: i32 = 26;

/// The behaviour profile a building's Type picks (`Behavior.dll:0x10008a80`).
pub fn profile_of(type_word: u32) -> Option<&'static str> {
    Some(match type_word {
        GENERATOR => "prof_generator.var",
        MINE => "prof_mine.var",
        STORAGE => "prof_storage.var",
        0x8000_0010 => "prof_plant.var",
        0x8000_0040 => "prof_angar.var",
        0x8000_0080 => "prof_mast.var",
        0x8000_0100 => "prof_teleport.var",
        0x8000_0200 => "prof_mtp.var",
        0x8000_0400 => "prof_institute.var",
        0x8010_0000 | 0x8020_0000 => "prof_tower.var",
        t if BUNKERS.contains(&t) => "prof_bunker.var",
        _ => return None,
    })
}

/// One building in the distribution: its profile's figures, its efficiency and batteries,
/// what it draws now, and its mine.
#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub target: usize,
    pub type_word: u32,
    /// `Transfer_Power_Out`, `Use_Power`, `Use_Ore`, `Transfer_Ore_OffBoard` and
    /// `Transfer_Ore_OnBoard`.
    pub power_out: f32,
    pub use_power: f32,
    pub use_ore: f32,
    pub off_board: f32,
    pub on_board: f32,
    /// The class-26 value, and the level the batteries serve it at: KPD is their product.
    pub efficiency: f32,
    pub level: f32,
    pub battery: Battery,
    /// What a task at work set with `SetPowerUsage`.
    pub usage: f32,
    pub next_power_ms: f64,
    pub last_power_ms: f64,
    /// Its mine's order, once running, and whether it digs yet.
    pub mine: Option<Mine>,
    pub digging: bool,
}

impl Site {
    /// The building's efficiency now: KPD.
    pub fn kpd(&self) -> f32 {
        self.efficiency * self.level
    }
}

/// What the economy keeps.
#[derive(Clone, Debug, Default)]
pub struct Economy {
    /// Each unit's and building's ore held and its most, by target.
    pub ore: HashMap<usize, (f32, f32)>,
    pub sites: Vec<Site>,
    /// Each clan's next distribution step and its last, ms.
    pub steps: HashMap<i64, (f64, f64)>,
    /// Each clan's power out a second, and its batteries' lack a second, at its last step.
    pub power: HashMap<i64, (f32, f32)>,
    /// Each transport's round, by target.
    pub rounds: HashMap<usize, crate::transport::Round>,
    /// A transport's and a builder's `Transfer_Ore_OnBoard` and `Transfer_Ore_OffBoard`, by
    /// Type, from their profiles once read.
    pub unit_rates: HashMap<u32, (f32, f32)>,
    /// `Behavior.dll`'s `rand()`, which the distribution step's timer draws on, and
    /// `Control.dll`'s shift register, which every controller's power tick draws on.
    pub step_rand: ModuleRand,
    pub jitter: ShiftJitter,
}

fn float(v: Value) -> f32 {
    match v {
        Value::Int(i) => i as f32,
        Value::Float(f) => f,
    }
}

impl Economy {
    /// Every placed object's `CurrentOre` of its `MaximumOre`, `objects` being the mission
    /// object each target is.
    pub fn new(mission: &Mission, objects: &[usize]) -> Economy {
        let ore = objects
            .iter()
            .enumerate()
            .filter_map(|(t, &o)| {
                let object = mission.objects.get(o)?;
                let most = object.property(MAXIMUM_ORE).map_or(0.0, |p| float(p.value));
                let held = object.property(CURRENT_ORE).map_or(0.0, |p| float(p.value));
                (most > 0.0 || held != 0.0).then_some((t, (held, most)))
            })
            .collect();
        Economy { ore, jitter: ShiftJitter::seeded(JITTER_SEED), ..Economy::default() }
    }

    /// The ore target `t` holds.
    pub fn held(&self, t: usize) -> f32 {
        self.ore.get(&t).map_or(0.0, |o| o.0)
    }

    /// The most target `t` holds.
    pub fn most(&self, t: usize) -> f32 {
        self.ore.get(&t).map_or(0.0, |o| o.1)
    }

    /// Take `amount` off what target `t` holds, going below zero as the property setter lets
    /// it (`Behavior.dll:0x100092c0`, docs/32, "Building a building").
    pub fn take_ore(&mut self, t: usize, amount: f32) {
        self.ore.entry(t).or_insert((0.0, 0.0)).0 -= amount;
    }

    /// Put `amount` into what target `t` holds.
    pub fn add_ore(&mut self, t: usize, amount: f32) {
        self.ore.entry(t).or_insert((0.0, 0.0)).0 += amount;
    }

    /// The site of building `t`.
    pub fn site(&self, t: usize) -> Option<&Site> {
        self.sites.iter().find(|s| s.target == t)
    }

    pub fn site_mut(&mut self, t: usize) -> Option<&mut Site> {
        self.sites.iter_mut().find(|s| s.target == t)
    }

    /// The next power tick's offset from 250 ms, from `Control.dll`'s shift register, which
    /// every controller's tick steps, a building's and a unit's alike (`0x1000c756`).
    pub(crate) fn power_jitter(&mut self) -> f64 {
        self.jitter.next_ms()
    }

    /// The distribution step's wait: 192 ms and `Behavior.dll`'s `rand()`'s share of 64 more
    /// (`0x10019e1b`, `0x1004c569`).
    pub(crate) fn step_wait(&mut self) -> f64 {
        STEP_MS + self.step_rand.timer_share_ms(1)
    }

    /// Move up to `amount` of ore from `from` to `to` (`Behavior.dll:0x100155f0`): no more than
    /// `from` holds, and, where `to` has a most, no more than its room. Returns what moved.
    pub fn transfer(&mut self, from: usize, to: usize, amount: f32) -> f32 {
        let held = self.held(from);
        let (to_held, to_most) = (self.held(to), self.most(to));
        let mut moved = amount.min(held);
        if to_most > 0.0 {
            moved = moved.min(to_most - to_held);
        }
        if moved <= 0.0 {
            return 0.0;
        }
        self.take_ore(from, moved);
        self.add_ore(to, moved);
        moved
    }

    /// A transport's or a builder's `Transfer_Ore_OnBoard` and `Transfer_Ore_OffBoard`, by its
    /// Type's profile (`Behavior.dll:0x10008a80`): `prof_trn` and `prof_bld`, 100 and 100 each.
    pub fn unit_rates(&mut self, assembly: &Assembly, type_word: u32) -> (f32, f32) {
        if let Some(&r) = self.unit_rates.get(&type_word) {
            return r;
        }
        let member = match type_word {
            TRANSPORT => "prof_trn.var",
            BUILDER => "prof_bld.var",
            _ => return (0.0, 0.0),
        };
        let vars = gamedir::resolve(&assembly.game, profiles::ARCHIVE)
            .and_then(|p| nres::Archive::open(&p).ok())
            .and_then(|a| a.read_name(member).ok().and_then(|d| profiles::parse(d, member).ok()));
        let figure = |name: &str| -> f32 {
            vars.as_ref().and_then(|v| v.iter().find(|v| v.name == name)).map_or(0.0, |v| v.value as f32)
        };
        let r = (figure("Transfer_Ore_OnBoard"), figure("Transfer_Ore_OffBoard"));
        self.unit_rates.insert(type_word, r);
        r
    }

    /// Building `t` of `type_word` at `path` joins the distribution at `now`: its profile, its
    /// class-26 efficiency and its class-19 batteries, a mine's and a storage's most.
    ///
    /// The batteries are the root controller's slots with the parts fitted into them: an
    /// internal part re-parses the slot its attach field names (docs/28, "A fitted part takes
    /// over its slot"), so a building's `i_pws_f_*` parts, 8 held and 500 a second each, stand
    /// in its root's 5 to 10 held and 13 to 25 a second (docs/23, "A power shortage lowers
    /// efficiency"). A building's parts go through a robot's part loop whichever way it is made,
    /// the mission's through `ArealMap.dll`'s copy and a builder's through `Behavior.dll`'s, and
    /// its object hands the part message on to the agent it wraps (docs/23, "A building's
    /// batteries are the parts fitted into its slots").
    pub fn join_building(&mut self, assembly: &mut Assembly, t: usize, type_word: u32, path: &str, now: f64) {
        if self.site(t).is_some() {
            return;
        }
        let vars = profile_of(type_word).and_then(|member| {
            gamedir::resolve(&assembly.game, profiles::ARCHIVE)
                .and_then(|p| nres::Archive::open(&p).ok())
                .and_then(|a| a.read_name(member).ok().and_then(|d| profiles::parse(d, member).ok()))
        });
        let figure = |name: &str| -> f32 {
            vars.as_ref().and_then(|v| v.iter().find(|v| v.name == name)).map_or(0.0, |v| v.value as f32)
        };
        let (power_out, use_power, use_ore, off_board, on_board) = (
            figure("Transfer_Power_Out"),
            figure("Use_Power"),
            figure("Use_Ore"),
            figure("Transfer_Ore_OffBoard"),
            figure("Transfer_Ore_OnBoard"),
        );
        let (mut efficiency, mut capacity, mut output) = (None, 0.0_f32, 0.0_f32);
        let controller = |assembly: &mut Assembly, record: &str| {
            let slot = assembly.library.record_slot(assembly.library.get(record), "ctl", 0)?;
            assembly
                .archive(&slot.library)
                .and_then(|a| a.read_name(&slot.member).ok())
                .and_then(|d| control::parse(d, &slot.member).ok())
        };
        if let Some(root) = assembly.records(path).into_iter().next()
            && let Some(c) = controller(assembly, &root)
        {
            let mut components = c.components;
            // Each part fitted into the root's controller takes over its slot with its first
            // record (docs/28): the class-19 slots take the `i_pws_f_*` parts.
            for (host, slot, record) in assembly.fitted(path) {
                if host != 0 {
                    continue;
                }
                let Some(part) = controller(assembly, &record).and_then(|p| p.components.into_iter().next())
                else {
                    continue;
                };
                if let Some(k) = components.get_mut(slot) {
                    *k = part;
                }
            }
            for k in &components {
                match k.type_id {
                    EFFICIENCY_TYPE => {
                        efficiency.get_or_insert(k.values[0]);
                    }
                    control::BATTERY_TYPE => {
                        // A battery with a negative capacity makes the whole building read full
                        // (`Control.dll:0x1002b42b`).
                        if k.values[control::BATTERY_CAPACITY] < 0.0 || capacity < 0.0 {
                            capacity = -1.0;
                        } else {
                            capacity += k.values[control::BATTERY_CAPACITY];
                        }
                        output += k.power;
                    }
                    _ => {}
                }
            }
        }
        let most = match type_word {
            MINE => MINE_MAX_ORE,
            STORAGE => STORAGE_MAX_ORE,
            _ => self.most(t),
        };
        let held = self.held(t);
        self.ore.insert(t, (held, most));
        let jitter = self.power_jitter();
        self.sites.push(Site {
            target: t,
            type_word,
            power_out,
            use_power,
            use_ore,
            off_board,
            on_board,
            efficiency: efficiency.unwrap_or(1.0),
            level: 1.0,
            battery: Battery::new(capacity, output),
            usage: 0.0,
            next_power_ms: now + POWER_TICK_MS + jitter,
            last_power_ms: now,
            mine: None,
            digging: false,
        });
    }
}

impl Play {
    /// Every building standing joins the distribution (at the mission's start, and a building
    /// made since). A mine is given order 10 as it joins (`iron3d.dll:0x10032d30`); one placed
    /// by the mission digs at once, a built one once its construction sphere ends.
    pub fn join_sites(&mut self) {
        let now = self.hero.time_ms;
        for t in 0..self.units.len() {
            if self.units[t].kind != KIND_BUILDING || self.economy.site(t).is_some() {
                continue;
            }
            let path = self.commander.paths.get(t).cloned().unwrap_or_default();
            let type_word = self.units[t].type_word;
            self.economy.join_building(&mut self.assembly, t, type_word, &path, now);
            if type_word == MINE {
                self.start_mine(t, !self.building_itself(t));
            }
            let site = self.economy.site(t).cloned();
            if let (Some(site), Some(f)) = (site, self.factories.iter_mut().find(|f| f.target == t)) {
                f.efficiency = site.efficiency;
                f.use_power = site.use_power;
            }
        }
    }

    /// Mine `t`'s order 10 (`M_Task_Mine`, `Behavior.dll:0x1002cd10`): the amounts of the lodes
    /// within 250 across the ground, each marked found; refused with none. It draws `Use_Power`
    /// from its start, and digs once `digging`.
    ///
    /// STAND-IN: docs/23-economy.md#a-mine-digs-to-500-and-then-a-draw-does-not-empty-it--read
    /// -- a built mine's order 10 is derived to wait behind its construction sphere while its
    /// draw is seen from the moment it appears: it draws from its making and digs from the
    /// sphere's end.
    pub fn start_mine(&mut self, t: usize, digging: bool) {
        let Some(at) = self.battle.combat.targets.get(t).map(|x| x.position) else { return };
        let mut to_mine = 0.0;
        for (i, lode) in self.construction.lodes.iter().enumerate() {
            if lode.position.truncate().distance(at.truncate()) <= LODE_REACH {
                to_mine += lode.amount;
                if let Some(found) = self.commander.lodes.get_mut(i) {
                    found.found = true;
                }
            }
        }
        let Some(site) = self.economy.site_mut(t) else { return };
        site.mine = Mine::new(to_mine);
        if site.mine.is_some() {
            site.usage = site.use_power;
            site.digging = digging;
        }
    }

    /// The economy's tick at `now`, `dt` seconds after the last: every building's power tick
    /// that is due, the mines' takt, and every clan's distribution step that is due.
    pub fn tick_economy(&mut self, now: f64, dt: f32) {
        self.join_sites();
        // A building's power tick (`Control.dll:0x1002d340`): its efficiency component draws
        // 0.01 and its usage a second from its batteries, and KPD's level is what they serve.
        for i in 0..self.economy.sites.len() {
            if now < self.economy.sites[i].next_power_ms {
                continue;
            }
            let jitter = self.economy.power_jitter();
            let site = &mut self.economy.sites[i];
            let step = ((now - site.last_power_ms) / 1000.0) as f32;
            site.last_power_ms = now;
            site.next_power_ms = now + POWER_TICK_MS + jitter;
            site.level = site.battery.spend(EFFICIENCY_POWER + site.usage, step).0;
        }
        for f in &mut self.factories {
            if let Some(site) = self.economy.sites.iter().find(|s| s.target == f.target) {
                f.level = site.level;
            }
        }
        self.tick_mines(dt);
        self.tick_transports(dt);
        let clans: BTreeSet<i64> =
            self.economy.sites.iter().filter_map(|s| self.units.get(s.target).and_then(|u| u.clan)).collect();
        for clan in clans {
            let (next, last) = *self.economy.steps.entry(clan).or_insert((now, now));
            if now < next {
                continue;
            }
            let wait = self.economy.step_wait();
            self.economy.steps.insert(clan, (now + wait, now));
            self.distribute(clan, ((now - last) / 1000.0) as f32);
        }
    }

    /// Every mine's takt of `dt` seconds (`0x1002cf60`): while its order digs, its running total
    /// written over what it holds; with all ore mined the order ends and its draw stops.
    fn tick_mines(&mut self, dt: f32) {
        for i in 0..self.economy.sites.len() {
            let site = &self.economy.sites[i];
            let t = site.target;
            if !site.digging || !self.battle.combat.targets.get(t).is_some_and(|x| x.alive) {
                continue;
            }
            // A building with no `Use_Power` digs at a KPD of 1.
            let kpd = if site.use_power > 0.0 { site.kpd() } else { 1.0 };
            let most = self.economy.most(t);
            let site = &mut self.economy.sites[i];
            let Some(mine) = site.mine.as_mut() else { continue };
            match mine.takt(dt, kpd, most) {
                Dug::Digging(held) => self.economy.ore.entry(t).or_insert((0.0, most)).0 = held,
                // "All Ore mined...": the last dig is banked before the task ends.
                Dug::AllMined(held) => {
                    self.economy.ore.entry(t).or_insert((0.0, most)).0 = held;
                    site.mine = None;
                    site.digging = false;
                    site.usage = 0.0;
                }
                Dug::Spent => {
                    site.mine = None;
                    site.digging = false;
                    site.usage = 0.0;
                }
            }
        }
    }

    /// Clan `clan`'s distribution step over `dt` seconds (`Behavior.dll:0x10019e80`): its
    /// generators' power tops every other building's batteries up by the same share of what
    /// they lack, and its mines' and storages' ore goes to every other building by the same
    /// share of what it asks, each holder giving of its offer, `min(held, dt × KPD ×
    /// off-board)`, in proportion (`0x1001a8c6`).
    fn distribute(&mut self, clan: i64, dt: f32) {
        let own: Vec<usize> = (0..self.economy.sites.len())
            .filter(|&i| {
                let t = self.economy.sites[i].target;
                self.units.get(t).and_then(|u| u.clan) == Some(clan)
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
            })
            .collect();
        let gives = |s: &Site| s.type_word == GENERATOR || BUNKERS.contains(&s.type_word);
        let holds = |s: &Site| s.type_word == MINE || s.type_word == STORAGE;
        // Power: a generator is set straight to full and gives; the rest ask for their lack.
        let (mut available, mut lack) = (0.0, 0.0);
        for &i in &own {
            let s = &self.economy.sites[i];
            if gives(s) {
                available += s.power_out * dt;
            } else {
                lack += s.battery.lack();
            }
        }
        let power_share = share(available, lack);
        for &i in &own {
            if !gives(&self.economy.sites[i]) {
                self.economy.sites[i].battery.top_up(power_share);
            }
        }
        if dt > 0.0 {
            self.economy.power.insert(clan, (available / dt, lack / dt));
        }
        // Ore.
        let mut offers = Vec::new();
        let mut wants = Vec::new();
        for &i in &own {
            let s = &self.economy.sites[i];
            let (held, most) = self.economy.ore.get(&s.target).copied().unwrap_or((0.0, 0.0));
            if holds(s) {
                let offer = held.max(0.0).min(dt * s.kpd() * s.off_board);
                if offer > 0.0 {
                    offers.push((s.target, offer));
                }
            } else if most - held >= WANT_FLOOR {
                wants.push((s.target, most - held));
            }
        }
        let offered: f32 = offers.iter().map(|o| o.1).sum();
        let asked: f32 = wants.iter().map(|w| w.1).sum();
        let given = share(offered, asked);
        let received: f32 = wants.iter().map(|w| w.1 * given).sum();
        let take = if offered > 0.0 { (received / offered).min(1.0) } else { 0.0 };
        for (t, want) in wants {
            self.economy.add_ore(t, want * given);
        }
        for (t, offer) in offers {
            self.economy.take_ore(t, offer * take);
        }
    }

    /// The Ore and Energy rows' targets for clan `clan`, whole percentages
    /// (`iron3d.dll:0x1006d8f0`): the ore its mines and storages hold over 4,500, and its power
    /// out less its batteries' lack over every clan's power out.
    pub fn resource_rows(&self, clan: i64) -> [i32; 2] {
        let ore: f32 = self
            .economy
            .sites
            .iter()
            .filter(|s| s.type_word == MINE || s.type_word == STORAGE)
            .filter(|s| self.units.get(s.target).and_then(|u| u.clan) == Some(clan))
            .filter(|s| self.battle.combat.targets.get(s.target).is_some_and(|x| x.alive))
            .map(|s| self.economy.held(s.target).max(0.0))
            .sum();
        let all: f32 = self.economy.power.values().map(|p| p.0).sum();
        let (out, lack) = self.economy.power.get(&clan).copied().unwrap_or((0.0, 0.0));
        let energy = if all > 0.0 { ((out - lack) / all).clamp(0.0, 1.0) } else { 0.0 };
        let percent = |v: f32| (v.clamp(0.0, 1.0) * 100.0).round() as i32;
        [percent(ore / ORE_ROW_SCALE), percent(energy)]
    }
}
