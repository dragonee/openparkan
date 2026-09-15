//! What the commander works with in command mode: the player's units and buildings by kind,
//! the selection, the order rows a selection is offered and the orders they give. The
//! panel that shows them is `cockpit::commander`. See `docs/41-commander.md` and
//! `docs/31-packages.md`, "The commander's menus".

use std::path::Path;

use glam::Vec3;
use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT, Mission};
use parkan_sim::hq;

use crate::play::{Mode, Play, VOICE_SELECTED};

/// The unit pages' Types: battle units (warriors and HQs), transports and builders.
pub const BATTLE_UNITS: u32 = 0x0101_8000;
pub const TRANSPORTS: u32 = 0x0100_2000;
pub const BUILDERS: u32 = 0x0100_4000;
/// The building pages' Types: research centres and factories exactly, towers, bunkers, and
/// the other buildings.
pub const RESEARCH_CENTRE: u32 = 0x8000_0400;
pub const FACTORY: u32 = 0x8000_0010;
pub const TOWERS: u32 = 0x8030_0000;
pub const BUNKERS: u32 = 0x8007_0000;
pub const OTHER_BUILDINGS: u32 = 0x8000_044e;
/// A lode's plume, and how near a building hides it.
pub const LODE_PLUME: &str = "env_mineral";
pub const LODE_PLUME_REACH: f32 = 80.0;

/// A mineral lode (`data.tma`'s trailer record, docs/31, "Mineral lodes").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lode {
    pub position: Vec3,
    /// Already found: the minerals search skips it.
    pub found: bool,
}

/// The commander's state in the play: the unit selection (a building's is `Play::selected`),
/// the lodes, and what the Build rows know.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Commander {
    /// The selected units, by target, in the order they were selected.
    pub units: Vec<usize>,
    pub lodes: Vec<Lode>,
    /// Per build Type (`hq::BUILD_TYPES`), whether every part of its scheme's first building
    /// is researched in the player's tree; worked out when first asked.
    pub researched: Option<[bool; 7]>,
    /// The Types the order menu has marked owned, which it never unmarks (`0x1007a9a6`).
    pub owned_marks: [bool; 7],
    /// Each target's `.dat`, as placed.
    pub paths: Vec<String>,
    /// The pick mode, and the pick an order row left open (docs/42).
    pub pick_mode: crate::pick::PickMode,
    pub pending: Option<crate::pick::Pending>,
    /// The building a Build row places, and whether its ray has missed once.
    pub ghost: Option<crate::pick::Ghost>,
    pub missed: bool,
    /// Each unit's box lines once rated, by target.
    pub lines: std::collections::HashMap<usize, Option<[String; 5]>>,
}

impl Commander {
    /// The mission's lodes, and the `.dat` of each target, `objects` being the mission object
    /// each target is.
    pub fn new(mission: &Mission, objects: &[usize]) -> Commander {
        let paths = objects
            .iter()
            .map(|&o| mission.objects.get(o).map_or_else(String::new, |m| m.path.clone()))
            .collect();
        let lodes = mission
            .viewpoints
            .iter()
            .map(|v| Lode { position: Vec3::from_array(v.position), found: v.unknown[0] != 0 })
            .collect();
        Commander { lodes, paths, ..Commander::default() }
    }
}

/// Whether every part of each build Type's scheme's first building is offered by the tree at
/// `tree` (docs/41, "Which rows it offers", condition 5).
pub fn researched(game: &Path, tree: &str) -> [bool; 7] {
    let mut out = [false; 7];
    let Ok(catalogue) = crate::designs::Catalogue::open(game, tree) else { return out };
    let schemes = parkan_formats::gamedir::resolve(game, parkan_formats::controls::BUILD_SCHEMES)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| {
            parkan_formats::controls::build_schemes(&b, parkan_formats::controls::BUILD_SCHEMES).ok()
        })
        .unwrap_or_default();
    for (i, kind) in hq::BUILD_TYPES.iter().enumerate() {
        let Some(first) =
            schemes.iter().find(|s| s.type_word() == Some(*kind)).and_then(|s| s.members.first())
        else {
            continue;
        };
        let unit = parkan_formats::gamedir::resolve(game, first)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| parkan_formats::objects::parse_unit(&b, first).ok());
        out[i] = unit.is_some_and(|u| u.components.iter().all(|c| catalogue.offered(&c.reference.member)));
    }
    out
}

impl Play {
    fn own_alive(&self, t: usize, kind: u32) -> bool {
        self.units.get(t).is_some_and(|u| u.kind == kind && u.clan == Some(self.player_clan))
            && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
            && !self.deleted.get(t).copied().unwrap_or(false)
    }

    /// The player's clan's live units whose Type lies within `mask`, in the level's order
    /// (`0x1007d7f0`); never the hero.
    pub fn own_units_within(&self, mask: u32) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&t| self.own_alive(t, KIND_UNIT))
            .filter(|&t| {
                self.units[t].logical_id != self.hero_id && hq::within(self.units[t].type_word, mask)
            })
            .collect()
    }

    /// The player's clan's buildings whose Type lies within `mask` (`0x1007df50`).
    ///
    /// STAND-IN: docs/41-commander.md#what-enables-a-button-and-what-lights-it -- a building
    /// building itself (property `0x20c`, the construction sphere) is not modelled: every
    /// building is complete.
    pub fn own_buildings_within(&self, mask: u32) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&t| self.own_alive(t, KIND_BUILDING) && hq::within(self.units[t].type_word, mask))
            .collect()
    }

    /// Select unit `t` alone (`0x1007d0a0` with 1): every other unit and the building let go,
    /// with `VOICE_SELECTED` when it was not selected.
    pub fn select_unit_alone(&mut self, t: usize) {
        let was = self.commander.units.contains(&t);
        self.commander.units = vec![t];
        self.selected.clear();
        if !was {
            self.voice(VOICE_SELECTED);
        }
    }

    /// `VOICE_SELECTED`, said once.
    pub fn ui_voice_selected(&mut self) {
        self.voice(VOICE_SELECTED);
    }

    fn voice(&mut self, name: &str) {
        if let Some(v) = self.progression.as_ref().and_then(|p| p.sound(name)) {
            self.says.push(crate::progress::Say::Voice(v));
        }
    }

    /// Select building `t` (`0x1007db80`), which lets the units go.
    pub fn select_building(&mut self, t: usize) {
        let was = self.selected.contains(&t);
        self.commander.units.clear();
        self.selected = vec![t];
        if !was {
            self.voice(VOICE_SELECTED);
        }
    }

    /// Whether building `t` is building itself: its construction sphere running (property
    /// `0x20c`, docs/32, "The construction sphere").
    ///
    /// STAND-IN: docs/41-commander.md#what-enables-a-button-and-what-lights-it -- the
    /// construction sphere is not modelled: no building builds itself.
    pub fn building_itself(&self, _t: usize) -> bool {
        false
    }

    /// The first `.dat` of the scheme that builds `type_word` (`BuildDat.lst`, docs/32).
    pub fn first_building(&self, type_word: u32) -> Option<String> {
        let game = &self.assembly.game;
        let file = parkan_formats::gamedir::resolve(game, parkan_formats::controls::BUILD_SCHEMES)?;
        let schemes = parkan_formats::controls::build_schemes(
            &std::fs::read(file).ok()?,
            parkan_formats::controls::BUILD_SCHEMES,
        )
        .ok()?;
        schemes.into_iter().find(|s| s.type_word() == Some(type_word))?.members.into_iter().next()
    }

    /// Order `builder` to build a building of `type_word` at `at`, turned `yaw`
    /// (`ORDER_ROBOT_BUILD`, target `0x206` the model's matrix, replacing; docs/32).
    ///
    /// STAND-IN: docs/32-builder.md#building-a-building-tick-by-tick--read-and-seen -- the build task is
    /// not modelled yet: the order is given and the builder, finding no task for it, stops.
    pub fn order_build(&mut self, builder: usize, type_word: u32, at: Vec3, _yaw: f32) -> bool {
        let order = parkan_sim::orders::Order {
            code: hq::BUILD,
            parameter: type_word as i32,
            target: parkan_sim::orders::Target::Place(at.to_array()),
        };
        let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == builder) else { return false };
        robot.order = Some(order);
        robot.behaviour.order(&order);
        let class = robot.size_class;
        self.acknowledge(class);
        true
    }

    /// Nothing selected (`0x1007d270`).
    pub fn clear_selection(&mut self) {
        self.commander.units.clear();
        self.selected.clear();
    }

    /// The selected units still alive and the player's.
    pub fn selected_units(&self) -> Vec<usize> {
        self.commander.units.iter().copied().filter(|&t| self.own_alive(t, KIND_UNIT)).collect()
    }

    /// The record `+0x30` of unit `t`.
    ///
    /// STAND-IN: docs/31-packages.md#not-established -- what the record's `+0x30` is is not
    /// read: the size class of the chassis's name.
    pub fn record_class(&self, t: usize) -> u8 {
        self.units.get(t).map_or(0, |u| u.designation.size_class)
    }

    /// What the order menu's row test reads, the menu opening now (`0x1007a8e0`), which marks
    /// the build Types the clan owns.
    pub fn hq_situation(&mut self) -> hq::Situation {
        if self.commander.researched.is_none() {
            let tree = usize::try_from(self.player_clan)
                .ok()
                .and_then(|c| self.clans.get(c))
                .map(|c| c.behaviour.clone())
                .unwrap_or_default();
            self.commander.researched = Some(researched(&self.assembly.game.clone(), &tree));
        }
        let researched = self.commander.researched.unwrap_or_default();
        let selected = self.selected_units();
        let first = selected.first().copied();
        let mut buildable = [false; 7];
        for (i, kind) in hq::BUILD_TYPES.iter().enumerate() {
            let owned = self.own_buildings_within(*kind).iter().any(|&b| self.units[b].type_word == *kind);
            if owned {
                self.commander.owned_marks[i] = true;
            }
            let heading_off = self.robots.iter().any(|(t, r)| {
                self.units[*t].clan == Some(self.player_clan)
                    && r.order.is_some_and(|o| o.code == hq::BUILD && o.parameter as u32 == *kind)
            });
            buildable[i] = researched[i] && !self.commander.owned_marks[i] && !owned && !heading_off;
        }
        hq::Situation {
            first_class: first.map_or(0, |t| self.record_class(t)),
            lode_unfound: self.commander.lodes.iter().any(|l| !l.found),
            // STAND-IN: docs/32-builder.md#building-a-building--read -- a builder's beam's
            // life is not told apart: a live builder can build.
            can_build: first.is_some_and(|t| hq::within(self.units[t].type_word, BUILDERS)),
            buildable,
        }
    }

    /// The order rows offered to the selection now.
    pub fn hq_rows(&mut self) -> Vec<u8> {
        let situation = self.hq_situation();
        let types: Vec<u32> = self.selected_units().iter().map(|&t| self.units[t].type_word).collect();
        hq::offered(&types, &situation)
    }

    /// Row `command` clicked (`0x1007b740`): an order goes to every selected robot at once,
    /// replacing its queue (`0x10079230`). Returns what the row does, so the caller can open
    /// the picks the others start.
    pub fn hq_command(&mut self, command: u8) -> Option<hq::Act> {
        let act = hq::act(command)?;
        if let hq::Act::Order(order) = act {
            for t in self.selected_units() {
                if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
                    robot.order = Some(order);
                    robot.behaviour.order(&order);
                }
            }
        }
        Some(act)
    }

    /// A sound of `ui/game_resources.cfg`'s, played now.
    pub fn ui_sound(&mut self, name: &str) {
        if let Some(s) = self.progression.as_ref().and_then(|p| p.sound(name)) {
            self.says.push(crate::progress::Say::Sound(s));
        }
    }

    /// Unit `t`'s box lines, once rated by [`Play::rate_units`].
    pub fn unit_lines(&self, t: usize) -> Option<&[String; 5]> {
        self.commander.lines.get(&t).and_then(Option::as_ref)
    }

    /// Rate the units of `targets` not yet rated from their designs, as the unit box does
    /// (`0x1006fc00`, docs/38, "The unit box").
    pub fn rate_units(&mut self, targets: &[usize]) {
        let missing: Vec<usize> =
            targets.iter().copied().filter(|t| !self.commander.lines.contains_key(t)).collect();
        if missing.is_empty() {
            return;
        }
        let game = self.assembly.game.clone();
        let tree = usize::try_from(self.player_clan)
            .ok()
            .and_then(|c| self.clans.get(c))
            .map(|c| c.behaviour.clone())
            .unwrap_or_default();
        let Ok(catalogue) = crate::designs::Catalogue::open(&game, &tree) else { return };
        let mut designer = crate::designs::Designer::new(&game, catalogue, 4);
        for t in missing {
            let path = self.commander.paths.get(t).cloned().unwrap_or_default();
            let lines = self
                .assembly
                .unit(&path)
                .and_then(|u| crate::designs::Node::from_unit(&u))
                .and_then(|node| designer.rate(&mut self.assembly, &node))
                .map(|r| r.lines(designer.offence_range, designer.defence_range));
            self.commander.lines.insert(t, lines);
        }
    }

    /// A building's name.
    ///
    /// STAND-IN: docs/35-hud.md#name-and-status--read-and-seen -- the routine that names a
    /// building is not read: `iron3d.dll`'s strings 6031–6098 by its Type, a bunker's size by
    /// its Type and any other's by the size letter of its root record (`fr_l_` small, `fr_m_`
    /// medium, `fr_b_` large), as the recording names the Small Generator, Small Warehouse,
    /// Large Factory and Small Bunker.
    pub fn building_name(&self, t: usize, strings: &std::collections::BTreeMap<u32, String>) -> String {
        let unit = &self.units[t];
        let size = self
            .assembly
            .records(self.commander.paths.get(t).map_or("", String::as_str))
            .first()
            .and_then(|r| r.to_ascii_lowercase().split('_').nth(1).map(str::to_owned))
            .map_or(0, |letter| match letter.as_str() {
                "m" => 1,
                "b" => 2,
                _ => 0,
            });
        let id = match unit.type_word {
            0x8000_0004 => 6031 + size,
            0x8000_0008 => 6036 + size,
            0x8000_0002 => 6041,
            0x8000_0400 => 6046 + size,
            0x8000_0010 => 6051 + size,
            0x8000_0040 => 6056,
            0x8010_0000 => 6077,
            0x8020_0000 => 6083,
            0x8001_0000 => 6086,
            0x8002_0000 => 6092,
            0x8004_0000 => 6098,
            _ => return String::new(),
        };
        strings.get(&id).cloned().unwrap_or_default()
    }

    /// Each lode's plume, `effects.rlb`'s `env_mineral` on the ground under it, shows while no
    /// building of any clan stands strictly within 80 of it (`iron3d.dll:0x10081c10`,
    /// `0x10081cd1`; docs/32, "The plume").
    pub fn lode_plumes(&mut self, now_ms: f64) {
        for i in 0..self.commander.lodes.len() {
            let lode = self.commander.lodes[i].position;
            let covered = (0..self.units.len()).any(|t| {
                self.units[t].kind == KIND_BUILDING
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
                    && self.battle.combat.targets[t].position.truncate().distance(lode.truncate())
                        < LODE_PLUME_REACH
            });
            let owner = crate::fx::Owner::Lode(i);
            let shown = self.fx.owned(owner).next().is_some();
            if covered && shown {
                self.fx.retain(|o, _| *o != owner);
            } else if !covered && !shown {
                let z = self.ground.below(lode.x, lode.y, 1.0e5).map_or(lode.z, |h| h.point.z);
                let frame = parkan_sim::effects::Frame::along(Vec3::new(lode.x, lode.y, z), Vec3::X, 1.0);
                self.fx.start(owner, LODE_PLUME, frame, 1.0, now_ms, None);
            }
        }
    }

    /// The building the commander is over, while in command mode.
    pub fn command_bunker(&self) -> Option<usize> {
        match self.mode() {
            Mode::Command(t) => Some(t),
            _ => None,
        }
    }
}
