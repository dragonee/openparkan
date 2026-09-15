//! Transport minerals (order 6, `M_Task_Transport`): a transport's round between its clan's
//! nearest mine with a free loading place and its nearest storage, loading and unloading at
//! 100 a second. The task walks where the round sends it; the play picks the places and moves
//! the ore. See `docs/32-builder.md`, "Transporting ore", and `docs/23-economy.md`, "A
//! transport's round".

use std::collections::HashMap;

use glam::Vec3;
use parkan_formats::hallway::{self, PLACE_LOADING, PLACE_UNLOADING};
use parkan_sim::behaviour::Task;
use parkan_sim::economy::{TRANSPORT_MAX_ORE, TRANSPORT_ORE_PER_SECOND};

use crate::economy::{MINE, STORAGE};
use crate::play::Play;

/// A storage with less than this free as the tick begins turns its transport aside, and one
/// waiting sets off again once it has more than this (`0x100322fd`, `0x10032681`).
pub const STORAGE_FULL: f32 = 0.1;
pub const STORAGE_ROOM: f32 = 0.5;

/// Where a transport's round stands (the task's states, `0x10032000`–`0x10032681`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leg {
    /// 1: to the mine's loading place.
    ToMine,
    /// 2: loading.
    Loading,
    /// 3: to the storage's unloading place.
    ToStorage,
    /// 4: unloading.
    Unloading,
    /// 5: beside a full storage, waiting for room.
    Waiting,
}

/// One transport's round: its leg, its mine and its storage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
    pub leg: Leg,
    pub mine: usize,
    pub storage: usize,
}

impl Play {
    /// A building's place with any bit of `flag` in its hall way, in the world.
    fn place_of(&mut self, t: usize, flag: u32) -> Option<Vec3> {
        let path = self.commander.paths.get(t)?.clone();
        let parts = self.assembly.parts(parkan_formats::mission::KIND_BUILDING, &path);
        let root = parts.iter().find(|p| p.host == -1)?.clone();
        let blob =
            self.assembly.archive(&root.reference.library)?.read_name(&root.reference.member).ok()?.to_vec();
        let vertex = *hallway::parse(&blob, &root.reference.member).ok()?.first(flag)?;
        let part = self.battle.combat.targets.get(t)?.parts.first()?;
        crate::factory::vertex_world(&vertex, part)
    }

    /// The live buildings of `clan` of exactly `type_word`, nearest `at` first, across the
    /// ground.
    fn nearest_buildings(&self, clan: Option<i64>, type_word: u32, at: Vec3) -> Vec<usize> {
        let mut found: Vec<usize> = (0..self.units.len())
            .filter(|&t| self.units[t].clan == clan && self.units[t].type_word == type_word)
            .filter(|&t| {
                self.battle.combat.targets.get(t).is_some_and(|x| x.alive) && !self.building_itself(t)
            })
            .collect();
        let d = |t: &usize| self.battle.combat.targets[*t].position.truncate().distance(at.truncate());
        found.sort_by(|a, b| d(a).total_cmp(&d(b)));
        found
    }

    /// The go command (`0x100327b0`, the route `0x10032870`): the nearest mine of the
    /// transport's clan whose loading place no other transport holds, and the nearest storage;
    /// with ore aboard, to the storage, else to the mine. `None` with either missing: the task
    /// cannot run.
    fn plan_round(&self, t: usize, rounds: &HashMap<usize, Round>) -> Option<Round> {
        let clan = self.units.get(t)?.clan;
        let at = self.battle.combat.targets.get(t)?.position;
        let mine = self.nearest_buildings(clan, MINE, at).into_iter().find(|&m| {
            !rounds.iter().any(|(&other, r)| other != t && r.mine == m && r.leg == Leg::Loading)
        })?;
        let storage = *self.nearest_buildings(clan, STORAGE, at).first()?;
        let leg = if self.economy.held(t) > 0.0 { Leg::ToStorage } else { Leg::ToMine };
        Some(Round { leg, mine, storage })
    }

    /// Every transport on its round this tick of `dt` seconds.
    ///
    /// STAND-IN: docs/32-builder.md#transporting-ore--read-and-measured -- a transport turned
    /// aside by a full storage tries up to 100 random points within 30 of the unloading place
    /// and walks to one at a quarter of its speed; it waits where it stands.
    pub fn tick_transports(&mut self, dt: f32) {
        let mut rounds = std::mem::take(&mut self.economy.rounds);
        let transports: Vec<usize> = self
            .robots
            .iter()
            .filter(|(t, r)| {
                self.battle.combat.targets.get(*t).is_some_and(|x| x.alive)
                    && matches!(r.behaviour.task(), Task::Transport { .. })
            })
            .map(|(t, _)| *t)
            .collect();
        rounds.retain(|t, _| transports.contains(t));
        for t in transports {
            let round = match rounds.get(&t).copied() {
                Some(r)
                    if self.battle.combat.targets.get(r.mine).is_some_and(|x| x.alive)
                        && self.battle.combat.targets.get(r.storage).is_some_and(|x| x.alive) =>
                {
                    r
                }
                // Either one gone: plan again, or end the task ("Task Ended").
                _ => match self.plan_round(t, &rounds) {
                    Some(r) => {
                        self.send(t, r);
                        r
                    }
                    None => {
                        self.end_transport(t);
                        rounds.remove(&t);
                        continue;
                    }
                },
            };
            let arrived = matches!(self.task_of(t), Some(Task::Transport { arrived: true, .. }));
            let most = match self.economy.most(t) {
                m if m > 0.0 => m,
                _ => TRANSPORT_MAX_ORE,
            };
            let held = self.economy.held(t);
            let next = match round.leg {
                Leg::ToMine if arrived => Leg::Loading,
                Leg::ToStorage if arrived => {
                    let (s_held, s_most) =
                        self.economy.ore.get(&round.storage).copied().unwrap_or((0.0, 0.0));
                    if s_most - s_held < STORAGE_FULL { Leg::Waiting } else { Leg::Unloading }
                }
                // Loading (`0x10032000`): 100 a second, no more than the mine holds or there is
                // room for; whichever runs out ends it.
                Leg::Loading => {
                    let mine_held = self.economy.held(round.mine).max(0.0);
                    let load = (TRANSPORT_ORE_PER_SECOND * dt).min(mine_held).min((most - held).max(0.0));
                    self.economy.take_ore(round.mine, load);
                    self.economy.add_ore(t, load);
                    if most - (held + load) <= 0.0 || mine_held - load <= 0.0 {
                        Leg::ToStorage
                    } else {
                        Leg::Loading
                    }
                }
                // Unloading: 100 a second while the storage has room; empty or full, back to the
                // mine.
                Leg::Unloading => {
                    let (s_held, s_most) =
                        self.economy.ore.get(&round.storage).copied().unwrap_or((0.0, 0.0));
                    let unload =
                        (TRANSPORT_ORE_PER_SECOND * dt).min(held.max(0.0)).min((s_most - s_held).max(0.0));
                    self.economy.take_ore(t, unload);
                    self.economy.add_ore(round.storage, unload);
                    if held - unload <= 0.0 || s_most - (s_held + unload) <= 0.0 {
                        Leg::ToMine
                    } else {
                        Leg::Unloading
                    }
                }
                Leg::Waiting => {
                    let (s_held, s_most) =
                        self.economy.ore.get(&round.storage).copied().unwrap_or((0.0, 0.0));
                    let clan = self.units[t].clan;
                    let theirs =
                        self.units[round.storage].clan != clan || self.units[round.mine].clan != clan;
                    if !theirs && s_most - s_held > STORAGE_ROOM { Leg::ToMine } else { Leg::Waiting }
                }
                leg => leg,
            };
            let mut round = Round { leg: next, ..round };
            if next != rounds.get(&t).map_or(next, |r| r.leg) {
                match next {
                    // The go command picks the mine and the storage afresh.
                    Leg::ToMine => {
                        round =
                            self.plan_round(t, &rounds).map_or(round, |r| Round { leg: Leg::ToMine, ..r });
                        self.send(t, round);
                    }
                    Leg::ToStorage => self.send(t, round),
                    Leg::Loading | Leg::Unloading | Leg::Waiting => self.hold_transport(t),
                }
            }
            rounds.insert(t, round);
        }
        self.economy.rounds = rounds;
    }

    fn task_of(&self, t: usize) -> Option<Task> {
        self.robots.iter().find(|(rt, _)| *rt == t).map(|(_, r)| r.behaviour.task())
    }

    /// Hand transport `t`'s task the place its round's leg walks to.
    fn send(&mut self, t: usize, round: Round) {
        let place = match round.leg {
            Leg::ToMine => self.place_of(round.mine, PLACE_LOADING).or_else(|| self.position_of(round.mine)),
            Leg::ToStorage => {
                self.place_of(round.storage, PLACE_UNLOADING).or_else(|| self.position_of(round.storage))
            }
            _ => None,
        };
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t)
            && let Some(task) = robot.behaviour.tasks.last_mut()
            && matches!(task, Task::Transport { .. })
        {
            *task = Task::Transport { goal: place, going: false, arrived: false };
        }
    }

    /// Transport `t` stands at its place while it loads, unloads or waits.
    fn hold_transport(&mut self, t: usize) {
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t)
            && let Some(task @ Task::Transport { .. }) = robot.behaviour.tasks.last_mut()
        {
            *task = Task::Transport { goal: None, going: false, arrived: false };
        }
    }

    fn position_of(&self, t: usize) -> Option<Vec3> {
        self.battle.combat.targets.get(t).map(|x| x.position)
    }

    fn end_transport(&mut self, t: usize) {
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
            robot.behaviour.tasks = vec![Task::Stop];
            robot.order = None;
        }
    }
}
