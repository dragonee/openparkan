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

/// Whether every part of the building at `path` is offered by `catalogue`, the clan's tree as
/// it stands (docs/41, "Which rows it offers", condition 5).
pub fn researched_building(game: &Path, catalogue: Option<&crate::designs::Catalogue>, path: &str) -> bool {
    let Some(catalogue) = catalogue else { return false };
    parkan_formats::gamedir::resolve(game, path)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| parkan_formats::objects::parse_unit(&b, path).ok())
        .is_some_and(|u| u.components.iter().all(|c| catalogue.offered(&c.reference.member)))
}

/// Whether every part of each build Type's scheme's first building is offered by `catalogue`
/// (docs/41, "Which rows it offers", condition 5).
pub fn researched(game: &Path, catalogue: Option<&crate::designs::Catalogue>) -> [bool; 7] {
    let mut out = [false; 7];
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
        out[i] = researched_building(game, catalogue, first);
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

    /// The player's clan's buildings whose Type lies within `mask` (`0x1007df50`), less any
    /// still building itself (property `0x20c`, [`Play::building_itself`]).
    pub fn own_buildings_within(&self, mask: u32) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&t| self.own_alive(t, KIND_BUILDING) && hq::within(self.units[t].type_word, mask))
            .filter(|&t| !self.building_itself(t))
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

    /// Nothing selected (`0x1007d270`).
    pub fn clear_selection(&mut self) {
        self.commander.units.clear();
        self.selected.clear();
    }

    /// The selected units still alive and the player's.
    pub fn selected_units(&self) -> Vec<usize> {
        self.commander.units.iter().copied().filter(|&t| self.own_alive(t, KIND_UNIT)).collect()
    }

    /// The record `+0x30` of unit `t`: its size class, which the record's bind asks the
    /// object for (message `0x201`) and the chassis's name gives (docs/38, "The catalogue").
    pub fn record_class(&self, t: usize) -> u8 {
        self.units.get(t).map_or(0, |u| u.designation.size_class)
    }

    /// What the order menu's row test reads, the menu opening now (`0x1007a8e0`), which marks
    /// the build Types the clan owns.
    pub fn hq_situation(&mut self) -> hq::Situation {
        if self.commander.researched.is_none() {
            let catalogue = self.catalogue();
            self.commander.researched = Some(researched(&self.assembly.game.clone(), catalogue.as_ref()));
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
        let upgradable = hq::BUILD_TYPES.map(|kind| self.upgrade_target(kind).is_some());
        hq::Situation {
            upgradable,
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

    /// The player's clan's buildings of `type_word` an upgrade would take, in the level's order
    /// (`0x10072a80`), each as `0x10034230` accepts it: not of a Type it refuses
    /// ([`hq::NEVER_UPGRADED`]), not building itself (property `0x20c`) and alive; its level --
    /// its place in its scheme's ladder -- with another entry above it; and that entry's parts
    /// all researched in the clan's tree (`0x1008b130`), as a Build row asks of the first.
    pub fn upgradable(&mut self, type_word: u32) -> Vec<usize> {
        if hq::NEVER_UPGRADED.contains(&type_word) {
            return Vec::new();
        }
        let owned: Vec<usize> = self
            .own_buildings_within(type_word)
            .into_iter()
            .filter(|&b| self.units[b].type_word == type_word)
            .collect();
        if owned.is_empty() {
            return owned;
        }
        let game = self.assembly.game.clone();
        let catalogue = self.catalogue();
        owned
            .into_iter()
            .filter(|&b| {
                self.upgrade_model(b)
                    .is_some_and(|next| researched_building(&game, catalogue.as_ref(), &next))
            })
            .collect()
    }

    /// The building an Upgrade row is offered for: the first [`Play::upgradable`] one (the row
    /// test stops at it, `0x1007bd72`).
    pub fn upgrade_target(&mut self, type_word: u32) -> Option<usize> {
        self.upgradable(type_word).first().copied()
    }

    /// The building a click on an Upgrade row sends unit `t` to (`0x10078f60`): of the
    /// [`Play::upgradable`] ones, the nearest to it across the ground; the first of equals.
    pub fn upgrade_target_for(&mut self, type_word: u32, t: usize) -> Option<usize> {
        let at = self.battle.combat.targets.get(t)?.position.truncate();
        let mut best: Option<(usize, f32)> = None;
        for b in self.upgradable(type_word) {
            let d = self.battle.combat.targets[b].position.truncate().distance(at);
            if best.is_none_or(|(_, n)| d < n) {
                best = Some((b, d));
            }
        }
        best.map(|(b, _)| b)
    }

    /// The `.dat` building `b` would become: the entry after its own in its Type's scheme
    /// (docs/32, "What gets built": a scheme's list is its upgrade ladder). `None` where it
    /// stands at the top, or is of no scheme.
    pub fn upgrade_model(&mut self, b: usize) -> Option<String> {
        let type_word = self.units.get(b)?.type_word;
        let path = self.commander.paths.get(b)?.to_ascii_lowercase();
        let scheme = self.schemes().iter().find(|s| s.type_word() == Some(type_word))?;
        let level = scheme.members.iter().position(|m| m.to_ascii_lowercase() == path)?;
        scheme.members.get(level + 1).cloned()
    }

    /// Row `command` clicked (`0x1007b740`): an order goes to every selected robot at once,
    /// replacing its queue (`0x10079230`). An Upgrade row names a building for each robot
    /// (`0x10078f60`), the nearest it would take, and gives a robot with none no order: the
    /// order carries the building's logic id and the Type as its parameter. Returns what the
    /// row does, so the caller can open the picks the others start.
    pub fn hq_command(&mut self, command: u8) -> Option<hq::Act> {
        let act = hq::act(command)?;
        for t in self.selected_units() {
            let order = match act {
                hq::Act::Order(order) => order,
                hq::Act::Upgrade(kind) => {
                    let Some(b) = self.upgrade_target_for(kind, t) else { continue };
                    parkan_sim::orders::Order {
                        code: parkan_sim::orders::UPGRADE,
                        parameter: kind as i32,
                        target: parkan_sim::orders::Target::LogicId(self.units[b].logical_id),
                    }
                }
                _ => break,
            };
            if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
                robot.order = Some(order);
                robot.behaviour.order(&order);
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
        let Some(catalogue) = self.catalogue() else { return };
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

    /// A building's name: [`building_name_id`] for its Type and its size class, the size
    /// from its root record's fourth letter ([`building_size`]).
    pub fn building_name(&self, t: usize, strings: &std::collections::BTreeMap<u32, String>) -> String {
        let root = self.assembly.records(self.commander.paths.get(t).map_or("", String::as_str));
        let size = root.first().map_or(0, |r| building_size(r));
        strings.get(&building_name_id(self.units[t].type_word, size)).cloned().unwrap_or_default()
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
                self.fx.start_ambient(owner, LODE_PLUME, frame, 1.0, now_ms, None);
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

/// A building's size class from its root record's member name: the fourth letter, `l` 2,
/// `m` 3, `b` 4, `e` 5, any other 0 (`Behavior.dll:0x1000cee0`, docs/38, "The grade is the
/// factory's size").
pub fn building_size(root: &str) -> u32 {
    match root.as_bytes().get(3).map(u8::to_ascii_lowercase) {
        Some(b'l') => 2,
        Some(b'm') => 3,
        Some(b'b') => 4,
        Some(b'e') => 5,
        _ => 0,
    }
}

/// The string naming a building of `type_word` and size class `size` (`iron3d.dll:0x100338d0`,
/// which the building record's slot 1, `0x10033720`, hands to the behaviour as its name;
/// docs/35, "Name and status"). A size the Type has no string for, or a Type not listed, is
/// 6205 *"Unknown"*.
pub fn building_name_id(type_word: u32, size: u32) -> u32 {
    const UNKNOWN: u32 = 6205;
    // `sizes` strings from `first` up, for size classes 2 onward.
    let sized =
        |first: u32, sizes: u32| if (2..2 + sizes).contains(&size) { first + size - 2 } else { UNKNOWN };
    match type_word {
        0x8000_0004 => sized(6031, 3),
        0x8000_0008 => sized(6036, 3),
        0x8000_0002 => 6041,
        0x8000_0400 => sized(6046, 4),
        0x8000_0010 => sized(6051, 3),
        0x8000_0040 => 6056,
        0x8000_2000 => sized(6061, 4),
        0x8000_1000 => sized(6066, 4),
        0x8000_0200 => sized(6071, 2),
        0x8010_0000 => 6077,
        0x8020_0000 => 6083,
        0x8001_0000 => 6086,
        0x8002_0000 => 6092,
        0x8004_0000 => 6098,
        _ => UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_building_is_named_by_its_type_and_its_roots_size_letter() {
        assert_eq!(
            ["fr_l_mine", "fr_M_mine", "fr_b_mine", "fr_e_inst", "fr_x_gener", "fr"].map(building_size),
            [2, 3, 4, 5, 0, 0]
        );
        // A mine, a warehouse and a factory come in three sizes, an institute in four, the
        // generator and the outpost in one whatever the letter.
        assert_eq!([2, 3, 4, 5].map(|s| building_name_id(0x8000_0004, s)), [6031, 6032, 6033, 6205]);
        assert_eq!([2, 5].map(|s| building_name_id(0x8000_0400, s)), [6046, 6049]);
        assert_eq!([0, 4].map(|s| building_name_id(0x8000_0002, s)), [6041, 6041]);
        // The main teleport in two sizes, the ruin and the bridge in four each.
        assert_eq!([2, 3, 4].map(|s| building_name_id(0x8000_0200, s)), [6071, 6072, 6205]);
        assert_eq!([2, 5].map(|s| building_name_id(0x8000_2000, s)), [6061, 6064]);
        assert_eq!([2, 5].map(|s| building_name_id(0x8000_1000, s)), [6066, 6069]);
        // Bunkers and towers by Type alone, and anything else Unknown.
        assert_eq!(
            [0x8001_0000, 0x8002_0000, 0x8004_0000, 0x8010_0000, 0x8020_0000, 0x8000_0001]
                .map(|t| building_name_id(t, 4)),
            [6086, 6092, 6098, 6077, 6083, 6205]
        );
    }
}
