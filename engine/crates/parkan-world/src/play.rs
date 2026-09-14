//! A mission played: the hero, the ground it walks on, the battle around it, the
//! effects it plays, the player's target and the mission's progression.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use glam::{Mat4, Vec3};
use parkan_formats::control::{
    ACT_EFFECT_NODE, ACT_EFFECT_POINTS, ACT_EFFECT_TIME_POINT, ACT_START_EFFECT, COND_BED, CONDITIONS,
    ENTRY_LOAD, run_group,
};
use parkan_formats::controls::{
    CMD_ENTER_STATE, CMD_JAMES_AIM_TARGET, CMD_JAMES_SELECT_ENEMY, CMD_JAMES_SELECT_FRIEND,
    CMD_JAMES_SELECT_TARGET, CMD_JAMES_WINGMAN_MENU,
};
use parkan_formats::exp::Explosion;
use parkan_formats::landmesh::FLAGS_LIQUID_BED_BIT;
use parkan_formats::materials::Library;
use parkan_formats::mesh::{NO_SLOT, SLOTS_PER_VARIANT};
use parkan_formats::mission::{Clan, KIND_BUILDING, KIND_ROCK, KIND_UNIT, KIND_VEGETATION, Mission, Value};
use parkan_formats::pose::Pose;
use parkan_formats::{gamedir, landmesh};
use parkan_sim::behaviour::{
    FIRE_BAR_FLYER, FIRE_BAR_WALKER, Seen, Senses, Walk, distance_score, fire_wait_ms,
};
use parkan_sim::combat::{Event, Part, Round};
use parkan_sim::damage::FLIGHT_MS;
use parkan_sim::damage::{Life, share_loss, touching};
use parkan_sim::effects::{Cue, Frame, Sprite};
use parkan_sim::ground::Ground;
use parkan_sim::guns::SINGLE_FIGHT;
use parkan_sim::hit::ROUND_SKIPS_FACE;
use parkan_sim::machine::Walker;
use parkan_sim::motion::GRAVITY;
use parkan_sim::orders::{self, ACKNOWLEDGEMENTS, Digit, Picked, Selector, VoicePick};
use parkan_sim::progression::Notice;
use parkan_sim::solid::{self, NO_CONTACT, Solid};
use parkan_sim::targeting::{Contact, TargetList};
use parkan_sim::wizard::{GROUND_POINT, straight_walk, walk_speed};

use crate::assembly::Assembly;
use crate::battle::Battle;
use crate::fx::{Fx, Owner};
use crate::hero::Hero;
use crate::models::Objects;
use crate::progress::{
    Progression, STRING_VACANT_VEHICLE, Say, TARGET_SELECTED, VOICE_ENEMY_DETECTED, VOICE_UNIT_DETECTED,
};
use crate::robot::Robot;
use crate::textures::TextureStore;
use crate::{settings, terrain};

/// `Type` words the target list leaves out: a hero of the player's clan, a bridge and a
/// ruin (`iron3d.dll:0x10091c80`).
pub const ROBOT_HERO: u32 = 0x0102_0000;
pub const BUILDING_BRIDGE: u32 = 0x8000_1000;
pub const BUILDING_RUINE: u32 = 0x8000_2000;
/// Enter takes a unit whose `Type` has no bit outside these (`iron3d.dll:0x10071fad`)
/// within 20 across the ground (`0x10071fe7`).
pub const CAPTURABLE_TYPES: u32 = 0x0103_e000;
pub const CAPTURE_REACH: f32 = 20.0;
/// A clan type: 0 nature, 3 neutral (docs/27).
pub const CLAN_NATURE: u32 = 0;
pub const CLAN_NEUTRAL: u32 = 3;
/// A relation word toward a clan the list counts as hostile, neutral and allied.
pub const RELATION_HOSTILE: u32 = 0;
pub const RELATION_NEUTRAL: u32 = 1;
pub const RELATION_ALLIED: u32 = 2;
/// The colours a mark takes (`iron3d.dll:0x10065440`), r, g, b.
pub const MARK_OWN: [u8; 3] = [128, 128, 255];
pub const MARK_NATURE: [u8; 3] = [255, 255, 0];
pub const MARK_NEUTRAL_CLAN: [u8; 3] = [160, 160, 160];
pub const MARK_NEUTRAL: [u8; 3] = [255, 0, 255];
pub const MARK_ALLIED: [u8; 3] = [0, 255, 255];
pub const MARK_HOSTILE: [u8; 3] = [255, 0, 0];
pub const MARK_OTHER: [u8; 3] = [255, 255, 0];
/// `FlyNearLandHeight`, bound by name (docs/24): how high a flyer's points keep.
pub const FLY_NEAR_LAND: f32 = 15.0;
/// STAND-IN: docs/26-damage.md#the-difficulty-ratio--read-and-measured -- which difficulty
/// profile a wingman's behaviour holds is not read: `Speed_MaximumFactor` 1, as four of the
/// five profiles set it.
pub const SPEED_MAXIMUM_FACTOR: f32 = 1.0;
/// A turret channel this close to its target counts as settled.
pub const AIM_SETTLED: f32 = 0.005;

/// What a target is beside what a round strikes: its clan, its `Type` word, its logical id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    pub clan: Option<i64>,
    pub type_word: u32,
    pub logical_id: i32,
    /// Placed as a unit, a building, or scenery.
    pub kind: u32,
    /// A neutral unit has made itself the hero's target once (`+0x134`).
    pub announced: bool,
    /// What it is and carries, which the HUD shows.
    pub designation: crate::robot::Designation,
}

/// Where the player looks from: for the right button's pick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub eye: Vec3,
    pub look: Vec3,
    pub view_proj: Mat4,
    /// Shift held, which the wingman selector reads.
    pub shift: bool,
}

/// What the wingman panel shows: each wingman's number, name and whether it is chosen;
/// whether the player is picking; and the order menu's rows with whether each is enabled.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub wingmen: Vec<(usize, String, bool)>,
    pub picking: bool,
    pub rows: Vec<(String, bool)>,
}

/// Where a sphere is on screen under `view_proj`, in NDC, when it lies inside the six
/// planes of the view.
pub fn on_screen(view_proj: Mat4, centre: Vec3, radius: f32) -> Option<[f32; 2]> {
    let [r0, r1, r2, r3] = [0, 1, 2, 3].map(|i| view_proj.row(i));
    for plane in [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2] {
        let n = plane.truncate();
        let length = n.length();
        if length > 1e-6 && (n.dot(centre) + plane.w) / length < -radius {
            return None;
        }
    }
    let clip = view_proj * centre.extend(1.0);
    (clip.w > 1e-6).then(|| [clip.x / clip.w, clip.y / clip.w])
}

/// A turret load-group effect: its name, the three control points it sits on, its id,
/// and the node whose channel value drives it (action 14).
#[derive(Clone, Debug, PartialEq)]
pub struct TurretEffect {
    pub name: String,
    pub points: [usize; 3],
    pub id: i32,
    pub driven_by: Option<usize>,
}

/// A chassis load-group effect on a node (action 3): its name, the node, its id.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeEffect {
    pub name: String,
    pub node: usize,
    pub id: i32,
}

pub struct Play {
    pub hero: Hero,
    pub ground: Ground,
    pub battle: Battle,
    pub assembly: Assembly,
    pub fx: Fx,
    pub materials: Library,
    pub turret_effects: Vec<TurretEffect>,
    pub chassis_effects: Vec<NodeEffect>,
    /// Mission objects that have died, not yet taken out of the drawing.
    pub killed: Vec<usize>,
    /// Sounds started and not yet played.
    pub cues: Vec<Cue>,
    /// Each target's clan, type and logical id.
    pub units: Vec<Unit>,
    /// What the hero is and carries, which its own panel shows.
    pub hero_designation: crate::robot::Designation,
    /// The driven unit's auto-driver level, 0–2, which `CMD_JAMES_AUTO_DRIVER` steps (docs/31).
    pub auto_driver: u8,
    pub clans: Vec<Clan>,
    /// The player's clan, and the hero's logical id.
    pub player_clan: i64,
    pub hero_id: i32,
    /// The driven unit's target list (docs/25).
    pub targets: TargetList,
    /// The mission's progression, once loaded (docs/34).
    pub progression: Option<Progression>,
    /// Every unit but the hero that is a robot, with the target it is in the battle.
    pub robots: Vec<(usize, Robot)>,
    /// Each target's name, for the wingman panel.
    pub names: Vec<String>,
    pub selector: Selector,
    voice_pick: VoicePick,
    /// Whether a captured bot is given Standby ([`Play::enter`]); off, as the game's.
    pub capture_standby: bool,
    /// Knocked-off parts in flight.
    pub flights: Vec<Flight>,
    /// Dead units and when each is deleted; and each target deleted.
    pub deaths: Vec<(usize, f64)>,
    pub deleted: Vec<bool>,
    /// What the game says, not yet shown or played.
    pub says: Vec<Say>,
}

/// A round about to leave a barrel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    pub kind: usize,
    pub owner: Option<usize>,
    pub muzzle: Vec3,
    pub direction: Vec3,
    pub velocity: Vec3,
    pub target: Option<usize>,
}

/// The rounds `shots` send from `robot`'s barrels, `owner` being its target in the battle.
/// `0x1002a34c`: a gun on a turret aims a mode-0 round at what the sight meets, and with no
/// hit leaves it its barrel's direction. The player's guns and its wingmen's carry no level
/// ratio: property 180 goes to hostile units only.
fn launches(
    robot: &Robot,
    owner: Option<usize>,
    shots: &[(usize, parkan_sim::guns::Shot)],
    battle: &Battle,
    ground: &Ground,
) -> Vec<Launch> {
    shots
        .iter()
        .filter_map(|&(g, shot)| {
            let kind = (*robot.rounds.get(g)?)?;
            let (muzzle, barrel) = robot.gun_muzzle(g, shot.barrel)?;
            let aim = robot.sight().and_then(|(o, s)| battle.combat.aim_point(ground, owner, o, s));
            Some(Launch {
                kind,
                owner,
                muzzle,
                direction: aim.map_or(barrel, |p| p - muzzle),
                velocity: robot.world_velocity(),
                target: robot.guns.get(g)?.target,
            })
        })
        .collect()
}

/// How a knocked-off part leaves, stand-ins for the unread flight model (see
/// [`Play::knock_off`]): its speed away from the unit and up, m/s, and its turn, rad/s.
pub const FLIGHT_SPEED_OUT: f32 = 2.0;
pub const FLIGHT_SPEED_UP: f32 = 0.0;
pub const FLIGHT_SPIN: f32 = 3.0;

/// A knocked-off part in flight: the node knocked off, and it and the nodes it carries at
/// their poses when it left; the point it turns about, how fast it left, and the level axis
/// it turns about.
#[derive(Clone, Debug, PartialEq)]
pub struct Flight {
    pub target: usize,
    pub part: usize,
    pub node: usize,
    pub start_ms: f64,
    pub centre: Vec3,
    pub velocity: Vec3,
    pub axis: Vec3,
    /// Its sphere's radius, and whether it has met the ground.
    pub radius: f32,
    pub landed: bool,
    pub base: Vec<(usize, Pose)>,
}

/// A round's own frame: y along its flight, z up, x to its side.
fn round_axes(r: &Round) -> [Vec3; 3] {
    let y = r.forward;
    let x = y.cross(Vec3::Z).normalize_or(Vec3::X);
    [x, y, x.cross(y)]
}

/// The wear entry a segment from `p0` to `p1` strikes on `part`: the struck triangle's
/// batch, in its node's level-0 slot, names it by its material byte
/// (`docs/11-effects.md`, "What an explosion plays").
///
/// STAND-IN: docs/11-effects.md#what-an-explosion-plays--read-and-measured -- the node's
/// wear base the material byte is ORed with is not read: the byte alone indexes the wear,
/// as the drawing does.
pub fn struck_wear<'a>(part: &Part, wear: &'a [String], p0: Vec3, p1: Vec3) -> Option<&'a str> {
    let strike = part.segment(p0, p1, ROUND_SKIPS_FACE)?;
    let slot = part.mesh.slots.get(usize::from(part.slot(strike.node?)?))?;
    let first = usize::from(slot.first_batch);
    let batches = part.mesh.batches.get(first..first + usize::from(slot.batch_count))?;
    let batch = batches.iter().find(|b| {
        let (from, count) = b.triangles();
        (from..from + count).contains(&strike.triangle)
    })?;
    wear.get(usize::from(batch.material & 0xFF)).map(String::as_str)
}

impl Play {
    /// The mission's hero armed, its map's ground, every other object a target, and the
    /// effects they can play loaded.
    pub fn load(game: &Path, mission: &Mission) -> Result<Option<Play>> {
        let mut assembly = Assembly::new(game)?;
        let Some(mut hero) = Hero::load(&mut assembly, mission)? else { return Ok(None) };
        let dir = terrain::map_dir(game, &mission.map_path)?;
        let land = landmesh::load(&gamedir::resolve(&dir, "Land.msh").context("the map has no Land.msh")?)?;
        let mut battle =
            Battle::load(&mut assembly, mission, Some(hero.object), settings::level_ratio(game))?;
        hero.arm(&mut battle, &mut assembly);
        let mut robots = Vec::new();
        for t in 0..battle.objects.len() {
            let o = battle.objects[t];
            if mission.objects[o].kind == KIND_UNIT
                && let Some(mut robot) = Robot::load(&mut assembly, mission, o)?
            {
                robot.arm(&mut battle, &mut assembly);
                robots.push((t, robot));
            }
        }
        let materials = Library::open(&gamedir::resolve(game, "Material.lib").context("no Material.lib")?)?;

        let mut fx = Fx::open(game)?;
        let load = hero.turret_controller.group(ENTRY_LOAD);
        let mut turret_effects: Vec<TurretEffect> = load
            .iter()
            .filter(|r| r.action() == ACT_EFFECT_POINTS && !r.resource.member.is_empty())
            .map(|r| TurretEffect {
                name: r.resource.member.clone(),
                points: [4, 5, 6].map(|k| usize::try_from(r.values[k]).unwrap_or(0)),
                id: r.values[7],
                driven_by: None,
            })
            .collect();
        for r in load.iter().filter(|r| r.action() == ACT_EFFECT_TIME_POINT) {
            if let Some(e) = turret_effects.iter_mut().find(|e| e.id == r.values[4]) {
                e.driven_by = usize::try_from(r.values[5]).ok();
            }
        }
        for e in &turret_effects {
            fx.template(&e.name);
        }
        // The chassis's load group puts effects on its nodes: the hero's steps on its feet
        // (docs/13, "A footstep, end to end"). The chassis is the part whose first node is 0.
        let chassis_effects: Vec<NodeEffect> = hero
            .walker
            .controller
            .group(ENTRY_LOAD)
            .iter()
            .filter(|r| r.action() == ACT_EFFECT_NODE && !r.resource.member.is_empty())
            .filter_map(|r| {
                Some(NodeEffect {
                    name: r.resource.member.clone(),
                    node: usize::try_from(r.values[4]).ok()?,
                    id: r.values[7],
                })
            })
            .collect();
        for k in &battle.kinds {
            for (name, _) in &k.effects {
                fx.template(name);
            }
        }
        for kind in &battle.combat.kinds {
            for e in kind.hit.iter().chain(kind.range_end.iter()) {
                fx.preload_explosion(e);
            }
        }
        for e in battle.explosions.iter().flatten().flatten().flatten() {
            fx.preload_explosion(e);
        }
        let number = |v: Value| match v {
            Value::Int(i) => i64::from(i),
            Value::Float(f) => f as i64,
        };
        let profiles = gamedir::resolve(game, parkan_formats::profiles::ARCHIVE)
            .and_then(|p| parkan_formats::nres::Archive::open(&p).ok());
        let hero_designation = {
            let object = &mission.objects[hero.object];
            crate::robot::designation(&mut assembly, profiles.as_ref(), object.kind, &object.path)
        };
        let mut units = Vec::new();
        for &o in &battle.objects {
            let object = &mission.objects[o];
            let designation = if object.kind == KIND_UNIT {
                crate::robot::designation(&mut assembly, profiles.as_ref(), object.kind, &object.path)
            } else {
                Default::default()
            };
            units.push(Unit {
                clan: object.clan_id(),
                type_word: object.property("Type").map_or(0, |p| number(p.value)) as u32,
                logical_id: object.logical_id,
                kind: object.kind,
                announced: false,
                designation,
            });
        }
        // Every target's faces, for the ground a building gives and the collision pass.
        let mut ground = Ground::new(land);
        let materials_for = |t: usize, part: usize, material: u16| {
            let name = battle.wears.get(t)?.get(part)?.get(usize::from(material & 0xFF))?;
            materials.get(name).map(|m| (m.surface, m.damage_rate))
        };
        ground.solids = battle
            .combat
            .targets
            .iter()
            .enumerate()
            .map(|(t, target)| {
                let building = mission.objects[battle.objects[t]].kind == KIND_BUILDING;
                Solid::from_parts(&target.parts, target.centre, target.radius, building, |part, material| {
                    materials_for(t, part, material)
                })
            })
            .collect();
        let player_clan = mission.objects[hero.object].clan_id().unwrap_or(0);
        let hero_id = mission.objects[hero.object].logical_id;
        let names: Vec<String> = battle
            .objects
            .iter()
            .map(|&o| {
                let object = &mission.objects[o];
                let stem = object.path.rsplit(['\\', '/']).next().unwrap_or(&object.path);
                if object.name.is_empty() { stem.to_owned() } else { object.name.clone() }
            })
            .collect();
        let target_count = battle.combat.targets.len();
        let mut play = Play {
            hero,
            ground,
            battle,
            assembly,
            fx,
            materials,
            turret_effects,
            chassis_effects,
            killed: Vec::new(),
            cues: Vec::new(),
            units,
            hero_designation,
            auto_driver: 0,
            clans: mission.clans.clone(),
            player_clan,
            hero_id,
            targets: TargetList::default(),
            progression: None,
            says: Vec::new(),
            names,
            selector: Selector::default(),
            voice_pick: VoicePick::default(),
            capture_standby: false,
            flights: Vec::new(),
            deaths: Vec::new(),
            deleted: vec![false; target_count],
            robots,
        };
        for i in 0..play.turret_effects.len() {
            let e = play.turret_effects[i].clone();
            let frame = play.turret_frame(&e);
            play.fx.start(Owner::Turret(e.id), &e.name, frame, 1.0, 0.0, None);
        }
        for e in play.chassis_effects.clone() {
            let (at, y) = play.hero.chassis_point(e.node);
            play.fx.start(Owner::Chassis(e.id), &e.name, Frame::along(at, y, 1.0), 1.0, 0.0, None);
        }
        Ok(Some(play))
    }

    /// Load the mission's progression from `mission_dir`: its player clan's script, its
    /// messages and objectives.
    pub fn load_progression(&mut self, game: &Path, mission_dir: &Path, mission: &Mission) -> Result<()> {
        self.progression = Some(Progression::load(game, mission_dir, mission, self.hero.object)?);
        Ok(())
    }

    /// Stand the hero `distance` from target `target` on level ground, facing it: the first of
    /// sixteen places about it, from `around` radians, whose ground is no more than 9 below the
    /// target's. False where there is none.
    pub fn stand_facing(&mut self, target: usize, distance: f32, around: f32) -> bool {
        let Some(at_target) = self.battle.combat.targets.get(target).map(|t| t.position) else {
            return false;
        };
        let Some(at) = (0..16)
            .map(|k| around + k as f32 * std::f32::consts::TAU / 16.0)
            .map(|a| at_target + Vec3::new(a.cos(), a.sin(), 0.0) * distance)
            .find(|p| self.ground.below(p.x, p.y, 1000.0).is_some_and(|h| h.point.z > at_target.z - 9.0))
        else {
            return false;
        };
        let facing = (at_target - at).with_z(0.0).normalize();
        let w = &mut self.hero.walker;
        w.body.position = Vec3::new(at.x, at.y, at_target.z + 20.0);
        w.body.yaw = (-facing.x).atan2(facing.y);
        w.follow_ground(&self.ground);
        w.from = (w.body.position, w.body.yaw);
        w.from_heading = w.body.yaw;
        true
    }

    /// Whether clan `other` is hostile to the player's: another clan, not nature's, toward
    /// which the player's clan's relation word is 0 (`iron3d.dll:0x10039440`).
    pub fn hostile(&self, other: Option<i64>) -> bool {
        let Some(other) = other.filter(|&c| c != self.player_clan) else { return false };
        let (Some(them), Some(us)) = (self.clan(other), self.clan(self.player_clan)) else { return false };
        them.kind != CLAN_NATURE
            && us
                .relations
                .iter()
                .find(|(name, _)| *name == them.name)
                .is_some_and(|&(_, w)| w == RELATION_HOSTILE)
    }

    /// The colour the game marks an object of clan `clan` in, seen by the player's clan
    /// (`iron3d.dll:0x10065440`, docs/25, "How the game colours what it marks"): the
    /// player's own light blue, a nature clan yellow, a neutral clan grey whatever its
    /// words, then by the marked clan's word towards the player's: 1 magenta, 2 cyan, 0
    /// red, any other yellow.
    pub fn mark_colour(&self, clan: Option<i64>) -> [u8; 3] {
        let Some(clan) = clan else { return MARK_OTHER };
        if clan == self.player_clan {
            return MARK_OWN;
        }
        let Some(them) = self.clan(clan) else { return MARK_OTHER };
        let us = self.clan(self.player_clan).map(|c| c.name.as_str());
        match them.kind {
            CLAN_NATURE => MARK_NATURE,
            CLAN_NEUTRAL => MARK_NEUTRAL_CLAN,
            _ => match them.relations.iter().find(|(name, _)| Some(name.as_str()) == us).map(|&(_, w)| w) {
                Some(RELATION_NEUTRAL) => MARK_NEUTRAL,
                Some(RELATION_ALLIED) => MARK_ALLIED,
                Some(RELATION_HOSTILE) => MARK_HOSTILE,
                _ => MARK_OTHER,
            },
        }
    }

    fn clan(&self, clan: i64) -> Option<&Clan> {
        usize::try_from(clan).ok().and_then(|c| self.clans.get(c))
    }

    /// What the target list knows of every target.
    ///
    /// STAND-IN: docs/25-sensors.md#the-players-target--read-and-measured -- whether scenery
    /// is among the radar's contacts is not read: a target must have a unit record, and
    /// trees and rocks have none, so they are never listed.
    pub fn contacts(&self) -> Vec<Contact> {
        self.battle
            .combat
            .targets
            .iter()
            .zip(&self.units)
            .map(|(t, u)| {
                let friend = u.clan == Some(self.player_clan);
                Contact {
                    position: t.position,
                    centre: t.centre,
                    radius: t.radius,
                    alive: t.alive && !matches!(u.kind, KIND_VEGETATION | KIND_ROCK),
                    building: u.kind == KIND_BUILDING,
                    hostile: self.hostile(u.clan),
                    friend,
                    unlisted: (friend && u.type_word == ROBOT_HERO)
                        || u.type_word == BUILDING_BRIDGE
                        || u.type_word == BUILDING_RUINE,
                }
            })
            .collect()
    }

    /// A new target: `TARGET_SELECTED`, and the guided guns take it (`0x10090a70`).
    fn target_changed(&mut self) {
        self.hero.relink(self.targets.current);
        if let Some(s) = self.progression.as_ref().and_then(|p| p.sound(TARGET_SELECTED)) {
            self.says.push(Say::Sound(s));
        }
    }

    fn say_sound(&mut self, name: &str, voice: bool) {
        if let Some(s) = self.progression.as_ref().and_then(|p| p.sound(name)) {
            self.says.push(if voice { Say::Voice(s) } else { Say::Sound(s) });
        }
    }

    /// The target list's takt on the hero, and the neutral units that make themselves its
    /// target (`iron3d.dll:0x100757ad`).
    fn update_targets(&mut self) {
        let world = self.contacts();
        let unit = self.hero.walker.body.position;
        let range = self.hero.radar.range;
        let now = self.hero.time_ms;
        let contacts = self.hero.radar.scan(now, unit, &world).to_vec();
        let changes = self.targets.takt(unit, range, &contacts, &world);
        if changes.target {
            self.target_changed();
        }
        if changes.enemy_detected {
            self.say_sound(VOICE_ENEMY_DETECTED, true);
        }
        for (i, seen) in world.iter().enumerate() {
            let u = self.units[i];
            let neutral = u.clan.and_then(|c| self.clan(c)).is_some_and(|c| c.kind == CLAN_NEUTRAL);
            if u.announced || !neutral || u.kind != KIND_UNIT || !seen.alive {
                continue;
            }
            if seen.position.truncate().distance(unit.truncate()) <= range {
                self.units[i].announced = true;
                if self.targets.set(Some(i)) {
                    self.target_changed();
                }
                if let Some(text) =
                    self.progression.as_ref().and_then(|p| p.strings.get(&STRING_VACANT_VEHICLE))
                {
                    self.says.push(Say::Text(crate::progress::Sender::System, text.clone()));
                }
                self.say_sound(VOICE_UNIT_DETECTED, true);
            }
        }
        self.hero.target_point = self.targets.current.and_then(|t| world.get(t)).map(|c| c.position);
    }

    /// One of `iron3d.dll`'s commands (`0x10071cd0`), the player looking from `view`.
    /// Returns whether it was one this answers.
    ///
    /// STAND-IN: docs/25-sensors.md#the-players-target--read-and-measured -- the unit
    /// record's `+0x98` and `+0x94`, where the right button's ray starts and the margin
    /// its pick keeps from the unit, are not read: both are 0.
    pub fn command(&mut self, command: &str, view: &View) -> bool {
        let world = self.contacts();
        let unit = self.hero.walker.body.position;
        let changed = match command {
            CMD_JAMES_SELECT_TARGET => self.targets.select_next(),
            CMD_JAMES_SELECT_ENEMY => self.targets.select_nearest(unit, &world, |c| c.hostile),
            CMD_JAMES_SELECT_FRIEND => self.targets.select_nearest(unit, &world, |c| c.friend),
            CMD_JAMES_AIM_TARGET => {
                let range = self.hero.radar.range;
                self.targets.aim(unit, view.eye, view.look, 0.0, 0.0, range, &world, |c, r| {
                    on_screen(view.view_proj, c, r)
                })
            }
            CMD_ENTER_STATE => {
                self.enter();
                false
            }
            // `0x10075fc0`: the level steps 0, 1, 2 and round.
            //
            // STAND-IN: docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured
            // -- the level decides a bot's overrides when the player takes it, and the player
            // never takes one here: it steps and does nothing else.
            parkan_formats::controls::CMD_JAMES_AUTO_DRIVER => {
                self.auto_driver = (self.auto_driver + 1) % 3;
                false
            }
            CMD_JAMES_WINGMAN_MENU => {
                let wingmen = self.wingmen().len();
                self.selector.tilde(view.shift, wingmen);
                false
            }
            _ => return false,
        };
        if changed {
            self.target_changed();
        }
        true
    }

    /// The wingmen (`iron3d.dll:0x10091f20`): the robots of the player's clan on the hero's
    /// radar, in the radar's order, as indices into `robots`.
    pub fn wingmen(&self) -> Vec<usize> {
        self.targets
            .listed
            .iter()
            .filter(|&&t| self.units.get(t).is_some_and(|u| u.clan == Some(self.player_clan)))
            .filter_map(|&t| self.robots.iter().position(|(rt, _)| *rt == t))
            .filter(|&r| self.battle.combat.targets.get(self.robots[r].0).is_some_and(|t| t.alive))
            .collect()
    }

    /// The hero's current target as the wingman menu's rows test it.
    fn picked(&self) -> Option<Picked> {
        let t = self.targets.current?;
        let u = self.units.get(t)?;
        Some(Picked {
            logic_id: u.logical_id,
            building: u.kind == KIND_BUILDING,
            type_word: u.type_word,
            own_clan: u.clan == Some(self.player_clan),
        })
    }

    /// A digit key while the selector is open (`iron3d.dll:0x100710fa`); whether it was taken.
    /// Ordering, digit n gives row n to the chosen wingmen, closes the menu and the last one
    /// acknowledges (`0x1006df80`, `0x10079230`, `0x1008e840`).
    ///
    /// STAND-IN: docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured --
    /// whether a digit the selector takes also reaches the input table that toggles the
    /// hero's guns is not read: it does not.
    pub fn wingman_digit(&mut self, n: usize) -> bool {
        let wingmen = self.wingmen();
        let chosen: Vec<usize> =
            self.selector.chosen.iter().filter_map(|&i| wingmen.get(i).copied()).collect();
        match self.selector.digit(n, wingmen.len()) {
            Digit::Passed => false,
            Digit::Taken => true,
            Digit::Row(row) => {
                let capturers = chosen.iter().all(|&r| matches!(self.robots[r].1.size_class, 1 | 2));
                let target = self.picked();
                if !chosen.is_empty()
                    && orders::enabled(row, target, capturers)
                    && let Some(order) = orders::order_for(row, self.hero_id, target)
                {
                    for &r in &chosen {
                        self.robots[r].1.order = Some(order);
                        self.robots[r].1.behaviour.order(&order);
                    }
                    self.selector.close();
                    let class = chosen.last().map_or(0, |&r| self.robots[r].1.size_class);
                    let name = format!(
                        "{}{}",
                        ACKNOWLEDGEMENTS[self.voice_pick.pick()],
                        orders::voice_suffix(class)
                    );
                    self.say_sound(&name, true);
                }
                true
            }
        }
    }

    /// What the wingman panel shows now, while the selector is open (`0x100432f0`,
    /// `0x1007aaa0`).
    pub fn panel(&self) -> Option<Panel> {
        if self.selector.state == orders::State::Off {
            return None;
        }
        let wingmen = self.wingmen();
        let lines = wingmen
            .iter()
            .enumerate()
            .take(16)
            .map(|(i, &r)| (i + 1, self.names[self.robots[r].0].clone(), self.selector.chosen.contains(&i)))
            .collect();
        let rows = if self.selector.state == orders::State::Ordering {
            let chosen: Vec<usize> =
                self.selector.chosen.iter().filter_map(|&i| wingmen.get(i).copied()).collect();
            let capturers = chosen.iter().all(|&r| matches!(self.robots[r].1.size_class, 1 | 2));
            let strings = self.progression.as_ref().map(|p| &p.strings);
            orders::ROWS
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let text = strings.and_then(|s| s.get(&row.string)).cloned().unwrap_or_default();
                    (text, orders::enabled(i, self.picked(), capturers))
                })
                .collect()
        } else {
            Vec::new()
        };
        Some(Panel { wingmen: lines, picking: self.selector.state == orders::State::Picking, rows })
    }

    /// `CMD_ENTER_STATE` (`iron3d.dll:0x10071f08`, docs/27): the hero's target, a unit
    /// within 20 across the ground, is captured into the player's clan when its clan is
    /// neutral.
    ///
    /// STAND-IN: docs/27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured
    /// -- the hero boarding what it captured is read, but the engine drives only the hero:
    /// the captured unit stays where it stands, and Enter on a unit of the player's own
    /// clan does nothing.
    pub fn enter(&mut self) -> bool {
        let Some(t) = self.targets.current else { return false };
        let u = self.units[t];
        let position = self.battle.combat.targets[t].position;
        let neutral = u.clan.and_then(|c| self.clan(c)).is_some_and(|c| c.kind == CLAN_NEUTRAL);
        let near = position.truncate().distance(self.hero.walker.body.position.truncate()) <= CAPTURE_REACH;
        if u.kind != KIND_UNIT || u.type_word & !CAPTURABLE_TYPES != 0 || !near || !neutral {
            return false;
        }
        self.units[t].clan = Some(self.player_clan);
        if let Some(p) = self.progression.as_mut() {
            p.progress.captured(u.logical_id, self.player_clan);
        }
        // DEPARTURE: docs/27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured
        // -- the game's capture changes only the unit's clan, SuperAI and areal map and gives
        // it no order, so it engages a hostile within 500 on its own; with
        // `capture_standby` it is given Standby, and holds until the player orders it.
        if self.capture_standby
            && let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t)
        {
            let standby =
                orders::Order { code: orders::STAYGROUND, parameter: 0, target: orders::Target::NotDefined };
            robot.order = Some(standby);
            robot.behaviour.order(&standby);
        }
        true
    }

    /// Give the rounds models and pooled instances among `objects`, and resolve the
    /// effects' looks in `store`.
    pub fn draw_rounds(&mut self, store: &mut TextureStore, objects: &mut Objects) -> Result<()> {
        self.fx.resolve_looks(store)?;
        self.battle.draw_rounds(&mut self.assembly, store, objects)
    }

    fn turret_frame(&self, e: &TurretEffect) -> Frame {
        let at = |i: usize| self.hero.point(i).unwrap_or((Vec3::ZERO, Vec3::Y));
        Frame::from_points([at(e.points[0]), at(e.points[1]), at(e.points[2])])
    }

    /// The ground's surface id under a strike's face (`Control.dll:0x100114fd`).
    fn surface(&self, face: usize) -> Option<u8> {
        let land = &self.ground.land;
        let name = land.layer1.get(usize::from(land.faces.get(face)?.tex1))?;
        self.materials.get(name).map(|m| m.surface)
    }

    /// The class of the material `round` struck on part `part` of target `target`: an
    /// agent with no outer object answers for its own material, so a strike on a unit
    /// plays the slot for the struck batch's material class (`docs/11-effects.md`, "What
    /// an explosion plays"). Scenery is built by the same agent loader with no outer
    /// object (`docs/22-settings.md`), and answers the same way.
    ///
    /// STAND-IN: docs/11-effects.md#what-an-explosion-plays--read-and-measured -- what a
    /// building, whose `CBuilding` aggregates its agent, answers for its material is not
    /// read: a strike on a building plays slot 0.
    fn struck_class(&self, target: usize, part: usize, round: &Round, point: Vec3) -> Option<u8> {
        if self.battle.placed_kinds.get(target) == Some(&KIND_BUILDING) {
            return None;
        }
        let p = self.battle.combat.targets.get(target)?.parts.get(part)?;
        let wear = self.battle.wears.get(target)?.get(part)?;
        let name = struck_wear(p, wear, round.previous, point + round.forward * 0.01)?;
        self.materials.get(name).map(|m| m.surface)
    }

    /// One tick: the hero, then every round that left one of its barrels, then the
    /// battle's frame, then the effects.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2]) -> Vec<Event> {
        self.update_targets();
        self.tick_robots(dt_ms);
        let shots = if self.hero.dead() {
            self.hero.time_ms += dt_ms;
            Vec::new()
        } else {
            let from = self.hero.collision_centre();
            let shots = self.hero.tick(dt_ms, mouse, &self.ground);
            self.collide(from);
            self.footsteps();
            shots
        };
        let now = self.hero.time_ms;
        let mut events = self.ground_damage(now);
        let launches = launches(&self.hero.robot, None, &shots, &self.battle, &self.ground);
        self.launch(launches, now);
        events.extend(self.battle.combat.tick((dt_ms / 1000.0) as f32, &self.ground));
        events.extend(self.battle.combat.takt_lives(now));
        for e in &events {
            self.effects_for(e, now);
            match *e {
                Event::KnockedOff { target, part, node } => self.knock_off(target, part, node, now),
                Event::Staged { target, .. } | Event::Hidden { target, .. } => self.rebuild_solid(target),
                Event::Killed { target } => {
                    self.deaths
                        .push((target, now + self.battle.death_ms.get(target).copied().unwrap_or(0.0)));
                }
                _ => {}
            }
        }
        self.fly(now);
        // `World3D.dll!KillGameObject` once a dead unit's time is up (`Control.dll:0x1000c977`).
        let (due, waiting): (Vec<_>, Vec<_>) = self.deaths.iter().partition(|(_, at)| now >= *at);
        self.deaths = waiting;
        for (target, _) in due {
            self.deleted[target] = true;
            self.flights.retain(|f| f.target != target);
            self.killed.push(self.battle.objects[target]);
        }
        // Turret effects follow their points and the channels that drive them.
        for i in 0..self.turret_effects.len() {
            let e = self.turret_effects[i].clone();
            let frame = self.turret_frame(&e);
            let value = e.driven_by.and_then(|node| {
                let c = self.hero.rig.channels.iter().position(|c| c.node == node as i32)?;
                self.hero.rig.values.get(c).copied()
            });
            for instance in self.fx.owned(Owner::Turret(e.id)) {
                instance.frame = frame;
                instance.value = value.unwrap_or(0.0);
            }
        }
        // Flight effects follow their rounds, and go with them. Time modes 5–15 read the
        // round's speed over its top speed: the plasma bolt's and the missile's trails.
        let rounds: Vec<Round> = self.battle.combat.rounds.clone();
        for r in &rounds {
            let kind = &self.battle.kinds[r.kind];
            let frames: Vec<Frame> = kind.effects.iter().map(|(_, p)| self.round_frame(r, *p)).collect();
            let top = self.battle.combat.kinds[r.kind].top_speed;
            let speed = if top > 0.0 { r.velocity.length() / top } else { 0.0 };
            for (instance, frame) in self.fx.owned(Owner::Round(r.id)).zip(frames) {
                instance.frame = frame;
                instance.speed = speed;
            }
        }
        self.fx.retain(|o, _| match o {
            Owner::Round(id) => rounds.iter().any(|r| r.id == *id),
            _ => true,
        });
        let cues = self.fx.cues(now);
        self.cues.extend(cues);
        self.fx.tick(now);
        for e in &events {
            if let Event::Killed { target } = e
                && let Some(p) = self.progression.as_mut()
            {
                p.progress.destroyed(self.units[*target].logical_id);
            }
        }
        // A won or lost mission plays on under its panel (docs/34, "After the outcome").
        self.progress();
        events
    }

    /// A destroyed part knocked off (docs/26): it and the nodes it carries fly from where
    /// they stand.
    ///
    /// STAND-IN: docs/26-damage.md#not-established -- the flight model (`0x100131da`) is read
    /// to move the part, lower its velocity's z each tick and hand the mesh its matrix, and a
    /// world query to end the flight early, but none of it is transcribed: the part drops
    /// off at 2 m/s away from the unit's centre, falls under gravity turning at 3 rad/s
    /// about a level axis across its path, and its flight ends when its sphere meets the
    /// ground. The player remembers a destroyed part falling for about half a second.
    fn knock_off(&mut self, target: usize, part: usize, node: usize, now: f64) {
        let Some(t) = self.battle.combat.targets.get(target) else { return };
        let Some(p) = t.parts.get(part) else { return };
        let Some(life) = p.life.as_ref() else { return };
        let mut carried = vec![node];
        let mut k = 0;
        while k < carried.len() {
            let n = carried[k];
            carried.extend((0..life.parents.len()).filter(|&c| life.parents[c] == Some(n)));
            k += 1;
        }
        let at = p.nodes[node].translation.map(|v| v as f32);
        let centre =
            p.mesh.nodes.get(node).and_then(|n| p.mesh.slots.get(usize::from(n.slot_index[0]))).map_or(
                Vec3::from_array(at),
                |s| {
                    let c = p.nodes[node]
                        .apply([s.sphere[0], s.sphere[1], s.sphere[2]].map(|v| f64::from(v * p.scale)));
                    Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)
                },
            );
        let away = (centre - t.centre).with_z(0.0).normalize_or(Vec3::X);
        self.flights.push(Flight {
            target,
            part,
            node,
            start_ms: now,
            centre,
            velocity: away * FLIGHT_SPEED_OUT + Vec3::Z * FLIGHT_SPEED_UP,
            axis: Vec3::new(-away.y, away.x, 0.0),
            radius: p
                .mesh
                .nodes
                .get(node)
                .and_then(|n| p.mesh.slots.get(usize::from(n.slot_index[0])))
                .map_or(0.0, |s| s.sphere[3] * p.scale),
            landed: false,
            base: carried.into_iter().map(|n| (n, p.nodes[n])).collect(),
        });
    }

    /// Every flying part where its flight has it at `now`.
    fn fly(&mut self, now: f64) {
        self.flights.retain(|f| now - f.start_ms <= FLIGHT_MS + 1000.0 && !f.landed);
        for f in &mut self.flights {
            let t = ((now - f.start_ms).min(FLIGHT_MS) / 1000.0) as f32;
            let mut at = f.centre + f.velocity * t - Vec3::Z * (GRAVITY / 2.0 * t * t);
            let floor = self.ground.below(at.x, at.y, at.z + 1000.0).map(|h| h.point.z);
            if let Some(z) = floor.filter(|&z| at.z - f.radius <= z) {
                at.z = at.z.max(z);
                f.landed = true;
            }
            let half = f64::from(FLIGHT_SPIN * t) / 2.0;
            let (sin, cos) = (half.sin(), half.cos());
            let rotation =
                [cos, f64::from(f.axis.x) * sin, f64::from(f.axis.y) * sin, f64::from(f.axis.z) * sin];
            let turned = parkan_formats::pose::rotate(rotation, f.centre.to_array().map(f64::from));
            let moved = at.to_array().map(f64::from);
            let flight = Pose { translation: [0, 1, 2].map(|i| moved[i] - turned[i]), rotation };
            let Some(part) =
                self.battle.combat.targets.get_mut(f.target).and_then(|t| t.parts.get_mut(f.part))
            else {
                continue;
            };
            for (n, base) in &f.base {
                if let Some(pose) = part.nodes.get_mut(*n) {
                    *pose = flight.compose(base);
                }
            }
            if f.landed
                && let Some(life) = part.life.as_mut()
            {
                life.end_flight(f.node, now);
            }
        }
    }

    /// A placed object's faces again, once its nodes' blocks or visibility changed; a robot's
    /// are rebuilt every tick.
    fn rebuild_solid(&mut self, t: usize) {
        if self.robots.iter().any(|(rt, _)| *rt == t) {
            return;
        }
        let (Some(target), Some(wears)) = (self.battle.combat.targets.get(t), self.battle.wears.get(t))
        else {
            return;
        };
        let building = self.battle.placed_kinds.get(t) == Some(&KIND_BUILDING);
        let materials = &self.materials;
        let solid =
            Solid::from_parts(&target.parts, target.centre, target.radius, building, |part, material| {
                let name = wears.get(part)?.get(usize::from(material & 0xFF))?;
                materials.get(name).map(|m| (m.surface, m.damage_rate))
            });
        if let Some(s) = self.ground.solids.get_mut(t) {
            *s = Solid { present: s.present, ..solid };
        }
    }

    /// The ground's rate a machine's ground contact reads (docs/24, "Holding the body on
    /// the ground", step 5): a face's material's, when the body sphere touches the ground
    /// point; a liquid bed's, when the water over it lies less than r below the sphere's
    /// centre. `None` when nothing is read, and the machine keeps the rate it had.
    fn ground_rate(&self, walker: &Walker) -> Option<f32> {
        let hit = walker.ground?;
        let centre = walker.sphere_centre();
        let r = walker.radius;
        match (hit.face, hit.solid) {
            (Some(f), _) => {
                let land = &self.ground.land;
                let face = land.faces.get(f)?;
                let read = if face.flags & FLAGS_LIQUID_BED_BIT != 0 {
                    self.ground.water(centre.x, centre.y, centre.z).is_some_and(|z| centre.z - z < r)
                } else {
                    touching(hit.point, centre, r)
                };
                let name = land.layer1.get(usize::from(face.tex1))?;
                read.then(|| self.materials.get(name).map(|m| m.damage_rate)).flatten()
            }
            (None, Some((s, f))) => touching(hit.point, centre, r)
                .then(|| self.ground.solids.get(s)?.faces.get(f).map(|face| face.damage_rate))
                .flatten(),
            _ => None,
        }
    }

    /// Every unit's life update that is due (docs/24, "Water and lava beds kill"): the rate
    /// its ground contact read × the seconds since its last, shared over its nodes. A hero
    /// that dies stops, and the mission fails.
    fn ground_damage(&mut self, now: f64) -> Vec<Event> {
        let mut events = Vec::new();
        for r in 0..self.robots.len() {
            let t = self.robots[r].0;
            if !self.battle.combat.targets.get(t).is_some_and(|target| target.alive) {
                continue;
            }
            if let Some(rate) = self.ground_rate(&self.robots[r].1.walker) {
                self.robots[r].1.ground_damage.read(rate);
            }
            let robot = &mut self.robots[r].1;
            if let Some(loss) = robot.ground_damage.update(robot.time_ms)
                && loss > 0.0
            {
                events.extend(self.battle.combat.ground_loss(t, loss));
            }
        }
        if self.hero.dead() {
            return events;
        }
        if let Some(rate) = self.ground_rate(&self.hero.walker) {
            self.hero.ground_damage.read(rate);
        }
        let Some(loss) = self.hero.ground_damage.update(now).filter(|&l| l > 0.0) else { return events };
        let (parts, mut lives): (Vec<usize>, Vec<&mut Life>) =
            self.hero.lives.iter_mut().enumerate().filter_map(|(p, l)| Some((p, l.as_mut()?))).unzip();
        let gone = share_loss(&mut lives, loss);
        let place = self.hero.placement();
        for (p, destroyed) in parts.into_iter().zip(gone) {
            let mesh = self.hero.parts[p].mesh.clone();
            for n in destroyed {
                let Some(Some(exp)) = self.hero.blasts.get(p).and_then(|b| b.get(n)).cloned() else {
                    continue;
                };
                let Some(slot) = mesh.mesh.slots.get(usize::from(mesh.mesh.nodes[n].slot_index[0])) else {
                    continue;
                };
                let node = place.compose(&self.hero.part_pose(p, n));
                self.node_blast(&exp, &node, slot.sphere, 1.0, now);
            }
        }
        // `iron3d.dll:0x10075619`: the loss of the player's clan's hero fails the mission.
        if self.hero.dead()
            && let Some(p) = self.progression.as_mut()
        {
            p.progress.outcome = Some(false);
            let says = p.say(&Notice::MissionFailed);
            self.says.extend(says);
        }
        events
    }
    /// The condition bytes the ground gives (`Control.dll:0x10002790`, `0x1001ab2d`): byte i
    /// set for the surface id i under the body, then byte 7 the face's liquid-bed flag.
    fn ground_conditions(&self) -> [bool; CONDITIONS] {
        let mut out = [false; CONDITIONS];
        if let Some(hit) = self.hero.walker.ground {
            let surface = match (hit.face, hit.solid) {
                (Some(f), _) => self.surface(f),
                (None, Some((s, f))) => self.ground.solids[s].faces[f].surface,
                _ => None,
            };
            if let Some(s) = surface.map(usize::from).filter(|&s| s <= 10) {
                out[s] = true;
            }
            out[COND_BED] = hit
                .face
                .and_then(|f| self.ground.land.faces.get(f))
                .is_some_and(|f| f.flags & FLAGS_LIQUID_BED_BIT != 0);
        }
        out
    }

    /// What every robot's behaviour sees: each live unit and building, and the hero, with
    /// its target in the battle.
    fn seen(&self) -> Vec<(Option<usize>, Seen)> {
        let mut seen: Vec<(Option<usize>, Seen)> = self
            .battle
            .combat
            .targets
            .iter()
            .zip(&self.units)
            .enumerate()
            .filter(|(_, (t, u))| t.alive && matches!(u.kind, KIND_UNIT | KIND_BUILDING))
            .map(|(i, (t, u))| {
                let s = Seen {
                    id: u.logical_id,
                    position: t.position,
                    radius: t.radius,
                    type_word: u.type_word,
                    building: u.kind == KIND_BUILDING,
                    own: u.clan == Some(self.player_clan),
                    hostile: self.hostile(u.clan),
                };
                (Some(i), s)
            })
            .collect();
        if !self.hero.dead() {
            let hero = Seen {
                id: self.hero_id,
                position: self.hero.walker.body.position,
                radius: self.hero.collision.1,
                type_word: ROBOT_HERO,
                building: false,
                own: true,
                hostile: false,
            };
            seen.push((None, hero));
        }
        seen
    }

    /// Every other robot's tick: a robot of the player's clan runs its behaviour, which
    /// moves it through its Wizard and aims and fires its guns; then its machine and its
    /// turret, and its target in the battle and its faces follow where it stands.
    ///
    /// STAND-IN: docs/31-packages.md#between-orders--read -- the behaviour runs only on the
    /// player's clan's robots, the wingmen; every other unit stands where it was placed.
    fn tick_robots(&mut self, dt_ms: f64) {
        let seen = self.seen();
        let mut fired = Vec::new();
        for r in 0..self.robots.len() {
            let t = self.robots[r].0;
            if !self.battle.combat.targets.get(t).is_some_and(|target| target.alive) {
                continue;
            }
            if self.units[t].clan == Some(self.player_clan) {
                self.behave(r, dt_ms, &seen);
            }
            let (t, robot) = &mut self.robots[r];
            let target = &mut self.battle.combat.targets[*t];
            robot.advance(dt_ms, &self.ground);
            let shots = robot.takt(dt_ms);
            robot.turn_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            if !shots.is_empty() {
                fired.push((r, shots));
            }
            let place = robot.placement();
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for (p, part) in target.parts.iter_mut().enumerate() {
                for n in 0..part.nodes.len() {
                    part.nodes[n] = place.compose(&robot.part_pose(p, n));
                    let Some(slot) = part.mesh.slots.get(usize::from(part.mesh.nodes[n].slot_index[0]))
                    else {
                        continue;
                    };
                    let [cx, cy, cz, r] = slot.sphere;
                    let c = part.nodes[n].apply([cx, cy, cz].map(f64::from));
                    let c = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                    lo = lo.min(c - Vec3::splat(r));
                    hi = hi.max(c + Vec3::splat(r));
                }
            }
            if lo.x <= hi.x {
                target.centre = (lo + hi) / 2.0;
            }
            target.position = robot.walker.body.position;
            if let Some(solid) = self.ground.solids.get_mut(*t) {
                *solid = Solid::from_parts(&target.parts, target.centre, target.radius, false, |_, _| None);
            }
        }
        for (r, shots) in fired {
            let (t, robot) = &self.robots[r];
            let launched = launches(robot, Some(*t), &shots, &self.battle, &self.ground);
            let now = robot.time_ms;
            self.launch(launched, now);
        }
    }

    /// One robot's behaviour takt (docs/31): its task's walk handed to the walker, which
    /// cuts it into points the Wizard follows; the fire control's target handed to its
    /// guns and traced by its turret; and each gun let fire once its AI timer runs out and
    /// its score clears the bar, or freely during a search or an attack (docs/29, "How the
    /// AI fires").
    fn behave(&mut self, r: usize, dt_ms: f64, seen: &[(Option<usize>, Seen)]) {
        let t = self.robots[r].0;
        let id = self.units[t].logical_id;
        let others: Vec<Seen> = seen.iter().filter(|(_, s)| s.id != id).map(|(_, s)| *s).collect();
        let bounds = self.ground.bounds();
        let (_, robot) = &mut self.robots[r];
        let now = robot.time_ms;
        let at = robot.walker.body.position;
        let senses = Senses {
            now_ms: now,
            position: at,
            seen: &others,
            bounds,
            has_weapon: !robot.guns.is_empty(),
            walker_idle: robot.wizard.idle(now),
            neutral: false,
        };
        let takt = robot.behaviour.takt(&senses);
        match takt.walk {
            Walk::Keep => {}
            Walk::Clear => robot.wizard.clear(),
            Walk::To(goal, share) => {
                let top = robot.walker.limits.top_speed[1].abs();
                let low = robot.walker.controller.triples[1][1].abs();
                let speed = walk_speed(share * top, top, low, SPEED_MAXIMUM_FACTOR);
                let floor = self.ground.below(goal.x, goal.y, 10_000.0).map_or(goal.z, |h| h.point.z);
                // STAND-IN: docs/24-motion.md#not-established -- the height a flyer's points
                // are given, and who reads `Movement_FlyHeight`, are not read: a flyer's
                // points keep at least `FlyNearLandHeight` above the ground under them.
                let (goal, flags) = if robot.flyer {
                    (goal.with_z(goal.z.max(floor + FLY_NEAR_LAND)), 0)
                } else {
                    (goal.with_z(floor), GROUND_POINT)
                };
                let (points, stop) = straight_walk(at, goal, speed, now, flags);
                robot.wizard.clear();
                robot.wizard.give(points);
                robot.wizard.stop_at(stop);
            }
        }
        robot.walker.drive = Some(robot.wizard.takt(now, at, dt_ms));

        // The fire control's target reaches every gun: an AI turret traces a point, so its
        // unguided guns take it too (docs/29).
        let target = takt.target.and_then(|id| seen.iter().find(|(_, s)| s.id == id)).and_then(|(i, _)| *i);
        if target != robot.fire_target {
            robot.fire_target = target;
            for g in &mut robot.guns {
                g.relink(target);
            }
        }
        let Some(victim) = target.and_then(|i| self.battle.combat.targets.get(i)) else {
            robot.target_point = None;
            return;
        };
        let (point, reach) = (victim.centre, victim.radius);
        robot.target_point = Some(point);
        robot.aim_at(point);
        let settled = [robot.rig.yaw, robot.rig.pitch].iter().enumerate().all(|(axis, c)| {
            c.is_none_or(|c| (robot.rig.values[c] - robot.rig.target(axis)).abs() < AIM_SETTLED)
        });
        let distance = at.distance(point);
        let bar = if takt.fire_freely {
            0.0
        } else if robot.flyer {
            FIRE_BAR_FLYER
        } else {
            FIRE_BAR_WALKER
        };
        for g in 0..robot.guns.len() {
            if now < robot.next_shot_ms[g] {
                continue;
            }
            let gun = &robot.guns[g];
            let score = if takt.fire_freely {
                0.5
            } else {
                // STAND-IN: docs/29-weapons.md#how-the-ai-fires--read -- the turret's aim stage
                // and the gun's report codes are not transcribed: θ is 0 once both channels
                // have reached their targets and π before, and a guided gun's θ is the share
                // of its lock left × π.
                let turret =
                    if settled { 1.0 } else { 1.0 - std::f32::consts::PI * distance / reach.max(1.0) };
                let lock = if gun.gate.guided() && gun.gate.lock_s > 0.0 {
                    (gun.lock.max(0.0) / gun.gate.lock_s).min(1.0) * std::f32::consts::PI
                } else {
                    0.0
                };
                let own = 1.0 - lock * distance / reach.max(1.0);
                if turret <= 0.0 || own <= 0.0 {
                    continue;
                }
                distance_score(distance, point.z - at.z, gun.round_speed) * turret * own
            };
            if score < bar {
                continue;
            }
            let Some((muzzle, _)) = robot.gun_muzzle(g, gun.current) else { continue };
            let clear = match self.battle.combat.first_hit(&self.ground, Some(t), muzzle, point, 0.0) {
                None => true,
                Some((_, struck, _)) => struck == target,
            };
            if !clear {
                continue;
            }
            let magazine = gun.magazine;
            robot.guns[g].state = SINGLE_FIGHT;
            let wait = fire_wait_ms(magazine, robot.behaviour.random());
            robot.next_shot_ms[g] = now + wait;
        }
    }

    /// Rounds leaving their barrels: each round, and its load group's flight effects.
    fn launch(&mut self, launched: Vec<Launch>, now: f64) {
        for l in launched {
            if let Some(id) =
                self.battle.combat.fire(l.kind, l.owner, l.muzzle, l.direction, l.velocity, 1.0, l.target)
            {
                // The round's load group creates its flight effects at spawn.
                let round = *self.battle.combat.rounds.last().expect("just fired");
                for (name, points) in self.battle.kinds[l.kind].effects.clone() {
                    let frame = self.round_frame(&round, points);
                    self.fx.start(Owner::Round(id), &name, frame, 1.0, now, None);
                }
            }
        }
    }

    /// The collision pass for the hero (docs/24, "Collision between objects"), after its
    /// move and ground contact: against every placed object whose sphere its swept sphere
    /// meets, the hero the mover and taking the whole push.
    ///
    /// STAND-IN: docs/24-motion.md#collision-between-objects--read -- the pass is read for
    /// every pair with a contact record; the engine moves only the hero, so the hero is
    /// always the mover and nothing else is pushed, and a machine standing on a building is
    /// taken to have left the pass for the building's own, so the deck it stands on does
    /// not push it.
    fn collide(&mut self, from: Vec3) {
        let to = self.hero.collision_centre();
        let radius = self.hero.collision.1;
        let standing_on = self.hero.walker.ground.and_then(|h| h.solid).map(|(s, _)| s);
        for (i, target) in self.battle.combat.targets.iter().enumerate() {
            if let Some(s) = self.ground.solids.get_mut(i) {
                s.present = target.alive;
            }
        }
        let mut total = Vec3::ZERO;
        for (i, obstacle) in self.ground.solids.iter().enumerate() {
            if !obstacle.present || Some(i) == standing_on {
                continue;
            }
            let end = to + total;
            let swept = (from, end);
            if parkan_sim::hit::swept_spheres(
                swept,
                radius,
                (obstacle.centre, obstacle.centre),
                obstacle.radius,
            )
            .is_none()
            {
                continue;
            }
            total += solid::push(from, end, radius, obstacle);
        }
        if total.length_squared() >= NO_CONTACT {
            self.hero.walker.take_push(total);
        }
    }

    /// The chassis's node effects follow their nodes, and each foot that landed runs its
    /// contact's group over the ground's condition bytes: action 10 restarts its step
    /// effect in the record's time mode (docs/13, "A footstep, end to end").
    fn footsteps(&mut self) {
        let now = self.hero.time_ms;
        for e in self.chassis_effects.clone() {
            let (at, y) = self.hero.chassis_point(e.node);
            for instance in self.fx.owned(Owner::Chassis(e.id)) {
                instance.frame = Frame::along(at, y, 1.0);
            }
        }
        let landed = std::mem::take(&mut self.hero.walker.landed);
        if landed.is_empty() {
            return;
        }
        let conditions = self.ground_conditions();
        let controller = &self.hero.walker.controller;
        let Some(state) = controller.states.get(self.hero.walker.machine.current) else { return };
        let mut starts = Vec::new();
        for c in landed {
            let Some(contact) = state.contacts.get(c).filter(|c| c.group >= 0) else { continue };
            let records = controller.group_at(contact.group);
            for r in run_group(&records, &conditions) {
                if r.action() == ACT_START_EFFECT {
                    starts.push((r.values[4], u32::try_from(r.values[5]).ok()));
                }
            }
        }
        for (id, mode) in starts {
            self.fx.restart(Owner::Chassis(id), now, mode);
        }
    }

    /// The progression's takts and handler, and what they say.
    fn progress(&mut self) {
        if self.progression.is_none() {
            return;
        }
        let mut at: HashMap<i32, Vec3> = self
            .units
            .iter()
            .zip(&self.battle.combat.targets)
            .map(|(u, t)| (u.logical_id, t.position))
            .collect();
        at.insert(self.hero_id, self.hero.walker.body.position);
        let now = self.hero.time_ms;
        let Some(p) = self.progression.as_mut() else { return };
        let notices = p.tick(now, |id| at.get(&id).copied());
        for n in &notices {
            let says = p.say(n);
            self.says.extend(says);
        }
    }

    /// A round's control points in the world, as action 4's frame.
    fn round_frame(&self, r: &Round, points: [usize; 3]) -> Frame {
        let axes = round_axes(r);
        let world = |v: [f32; 3]| axes[0] * v[0] + axes[1] * v[1] + axes[2] * v[2];
        let at = |i: usize| {
            self.battle.kinds[r.kind]
                .points
                .get(i)
                .map_or((r.position, Vec3::Y), |p| (r.position + world(p.position), world(p.direction)))
        };
        Frame::from_points([at(points[0]), at(points[1]), at(points[2])])
    }

    /// What an event plays: a round's explosion where it struck or ran out, and a
    /// destroyed node's own.
    fn effects_for(&mut self, e: &Event, now: f64) {
        match e {
            Event::Struck { round, target, part, point, .. } => {
                let kind = &self.battle.combat.kinds[round.kind];
                let Some(exp) = kind.hit.clone() else { return };
                let (surface, axis) = match target {
                    None => {
                        let face = self.ground.segment(round.previous, *point + round.forward * 0.01);
                        let surface = face.and_then(|s| self.surface(s.triangle));
                        let normal = face
                            .map_or(Vec3::Z, |s| Vec3::from_array(self.ground.land.faces[s.triangle].normal));
                        (surface, normal)
                    }
                    Some(t) => (self.struck_class(*t, *part, round, *point), -round.forward),
                };
                // Placement 7 turns the effect to the struck face; a round's `.exp` radius is its size.
                self.fx.explode(&exp, surface, Frame::along(*point, axis, 1.0), exp.radius, now);
            }
            Event::Exploded { kind, point, forward, at_range: true } => {
                // Placement 0 on a round: along its second axis, the way it flies (docs/29).
                if let Some(exp) = self.battle.combat.kinds[*kind].range_end.clone() {
                    self.fx.explode(&exp, None, Frame::along(*point, *forward, 1.0), exp.radius, now);
                }
            }
            // A node's stage rising plays its `.exp` at its sphere's centre (docs/26).
            Event::Staged { target, part, node } => {
                let Some(Some(exp)) = self
                    .battle
                    .explosions
                    .get(*target)
                    .and_then(|p| p.get(*part))
                    .and_then(|p| p.get(*node))
                    .cloned()
                else {
                    return;
                };
                let p = &self.battle.combat.targets[*target].parts[*part];
                let block = p.life.as_ref().and_then(|l| l.nodes.get(*node)).map_or(0, |l| l.block());
                let slots = p.mesh.nodes[*node].slot_index;
                let slot = [slots[block * SLOTS_PER_VARIANT], slots[0]].into_iter().find(|&s| s != NO_SLOT);
                let Some(slot) = slot.and_then(|s| p.mesh.slots.get(usize::from(s))) else { return };
                let (pose, sphere, scale) = (p.nodes[*node], slot.sphere, p.scale);
                self.node_blast(&exp, &pose, sphere, scale, now);
            }
            _ => {}
        }
    }

    /// A destroyed node's explosion, on the node posed at `node` with its slot's `sphere`.
    /// Placement 0 is the node's second axis; a node's size is the radius × its sphere's.
    fn node_blast(&mut self, exp: &Explosion, node: &Pose, sphere: [f32; 4], scale: f32, now: f64) {
        let [cx, cy, cz, r] = sphere;
        let c = node.apply([cx, cy, cz].map(|v| f64::from(v * scale)));
        let centre = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
        let y = parkan_formats::pose::rotate(node.rotation, [0.0, 1.0, 0.0]);
        let axis = Vec3::new(y[0] as f32, y[1] as f32, y[2] as f32);
        self.fx.explode(exp, None, Frame::along(centre, axis, 1.0), exp.radius * r * scale, now);
    }

    /// Every effect sprite at the current time, as seen from `eye`: an instance that tests
    /// its point casts a ray to it from the eye, and nothing struck is in view
    /// (`docs/11-effects.md`, "Bit 8 and the tested point").
    ///
    /// STAND-IN: docs/11-effects.md#bit-8-and-the-tested-point--read-and-measured -- how
    /// often an instance tests its point, and what the ray through the world meets, are
    /// not read: every frame, against what a round meets (the ground less its water
    /// surface, and every live object's level-0 mesh).
    pub fn sprites(&self, eye: Vec3) -> Vec<(usize, Sprite)> {
        self.fx.sprites(self.hero.time_ms, |point| {
            self.battle.combat.first_hit(&self.ground, None, eye, point, 0.0).is_none()
        })
    }
}

/// Whether node `node` of a part with this life still has life; a part that takes no
/// damage always does.
pub fn node_alive(life: Option<&parkan_sim::damage::Life>, node: usize) -> bool {
    life.and_then(|l| l.nodes.get(node)).is_none_or(|l| !l.destroyed)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use parkan_formats::mesh::{Batch, Mesh, NO_SLOT, Node, Slot};
    use parkan_formats::pose::IDENTITY;

    use super::*;

    /// One node facing -y at y = 5: a left triangle in batch 0 and a right one in batch 1,
    /// whose material byte is 2 with the high byte set.
    fn panel() -> Part {
        let batch = |material: u16, first: u16| Batch {
            material,
            flag: 0,
            first_index: first * 3,
            index_count: 3,
            first_vertex: 0,
            vertex_count: 4,
        };
        let mesh = Mesh {
            name: "panel".into(),
            positions: vec![[-2.0, 5.0, -1.0], [0.0, 5.0, -1.0], [0.0, 5.0, 1.0], [2.0, 5.0, -1.0]],
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [1, 3, 2]],
            nodes: vec![Node {
                name: "panel".into(),
                flags: 0,
                parent: 0xFFFF,
                anim_start: 0xFFFF,
                fallback_key: 0,
                slot_index: std::array::from_fn(|k| if k == 0 { 0 } else { NO_SLOT }),
            }],
            slots: vec![Slot {
                first_triangle: 0,
                triangle_count: 2,
                first_batch: 0,
                batch_count: 2,
                aabb_min: [0.0; 3],
                aabb_max: [0.0; 3],
                sphere: [0.0; 4],
                area: 0.0,
                volume: 0.0,
            }],
            batches: vec![batch(0, 0), batch(0xFF02, 1)],
            face_flags: vec![0, 0],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
        };
        Part { mesh: Rc::new(mesh), nodes: vec![IDENTITY], scale: 1.0, life: None }
    }

    #[test]
    fn a_strike_names_the_wear_entry_of_the_batch_it_struck() {
        let wear: Vec<String> = ["B_SKIN", "B_GLASS", "R_H_01"].map(String::from).into();
        let part = panel();
        let at = |x: f32| struck_wear(&part, &wear, Vec3::new(x, 0.0, -0.5), Vec3::new(x, 10.0, -0.5));
        assert_eq!(at(-1.0), Some("B_SKIN"));
        assert_eq!(at(1.0), Some("R_H_01"), "the material byte's low byte indexes the wear");
        assert_eq!(at(5.0), None, "a miss");
    }
}
