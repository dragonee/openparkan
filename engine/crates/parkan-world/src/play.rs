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
    CMD_ENTER_STATE, CMD_JAMES_AIM_TARGET, CMD_JAMES_OUTER_CAMERA, CMD_JAMES_SELECT_ENEMY,
    CMD_JAMES_SELECT_FRIEND, CMD_JAMES_SELECT_TARGET, CMD_JAMES_WINGMAN_MENU, CMD_JAMES_ZOOM_MODE,
};
use parkan_formats::exp::Explosion;
use parkan_formats::landmesh::FLAGS_LIQUID_BED_BIT;
use parkan_formats::materials::Library;
use parkan_formats::mesh::{NO_SLOT, SLOTS_PER_VARIANT};
use parkan_formats::mission::{
    self, Clan, KIND_BUILDING, KIND_ROCK, KIND_UNIT, KIND_VEGETATION, Mission, Value,
};
use parkan_formats::pose::Pose;
use parkan_formats::{gamedir, landmesh};
use parkan_sim::behaviour::{
    FIRE_BAR_FLYER, FIRE_BAR_WALKER, Search, Seen, Senses, Takt, Task, Walk, distance_score, fire_wait_ms,
};
use parkan_sim::combat::{Event, Part, Round, RoundEnd};
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
use parkan_sim::wizard::{GROUND_POINT, path_walk, straight_walk, walk_speed};

use crate::assembly::Assembly;
use crate::battle::{Battle, EffectCommand};
use crate::building_fx::BuildingEffects;
use crate::buildings::{Building, Child, Fired, Standing};
use crate::factory::{Factory, Project, VOICE_UNIT_READY};
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
/// A plant's `Type`: its pod opens the factory screen for the player (docs/27, "Capture").
pub const BUILDING_PLANT: u32 = 0x8000_0010;
/// A generator's `Type` (docs/27, "Capture").
pub const BUILDING_GENERATOR: u32 = 0x8000_0002;
/// The three bunkers' `Type`s, whose pods open command mode (`iron3d.dll:0x10062779`).
pub const BUILDING_BUNKERS: [u32; 3] = [0x8001_0000, 0x8002_0000, 0x8004_0000];
/// The System line an ownership change shows (`iron3d.dll:0x100a48a0`).
pub const STRING_BUILDING_CAPTURED: u32 = 5039;
/// What the player hears when a building changes hands: taken from a neutral or an ally,
/// taken from an enemy, and lost (`iron3d.dll:0x100a48a0`, docs/27).
pub const VOICE_NBUILD_CAPTURE: &str = "VOICE_NBUILD_CAPTURE";
pub const VOICE_EBUILD_CAPTURE: &str = "VOICE_EBUILD_CAPTURE";
pub const VOICE_BUILD_CAPTURE: &str = "VOICE_BUILD_CAPTURE";
pub const VOICE_SELECTED: &str = "VOICE_SELECTED";

/// A produced unit's object number: past every mission's objects, so no mission object is
/// taken for it.
pub const SPAWNED_OBJECTS: usize = 1 << 20;
/// `CLASS_ROBOT`: a Type word with this bit is a robot, which holds one of its clan's minds
/// (docs/23, "The bot limit is the clan's mind count").
pub const CLASS_ROBOT: u32 = 0x0100_0000;
/// `CLASS_ANIMAL`: an animal, which takes up no fight unless it migrates (docs/31, "Between
/// orders").
pub const CLASS_ANIMAL: u32 = 0x2000_0000;

/// A view mode on the interface's stack (docs/39-boarding.md, "The game view keeps a stack
/// of modes"): the hero on foot, or a building's screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Mode 0.
    OnFoot,
    /// Mode 1 with the robot that is target `t`: the player drives it (docs/39); mode 2,
    /// telepresence, when the [`Driving`] says so (docs/40).
    Driving(usize),
    /// Mode 3 with the HQ unit that is target `t`: command mode, the camera riding with the
    /// unit (docs/40, "An HQ's command mode: mode 3").
    HqCommand(usize),
    /// Mode 4 with the bunker that is target `t`: command mode, the camera over the base
    /// (docs/40).
    Command(usize),
    /// Mode 5 with the building that is target `t`: its screen, a factory's (docs/36) or a
    /// research centre's, the commander panel turned to page 4 (docs/41).
    Factory(usize),
}

impl Mode {
    /// A mode whose screens show the cursor, the world going on behind them: command mode
    /// and a building's screen (docs/40, docs/36).
    pub fn shows_cursor(self) -> bool {
        matches!(self, Mode::HqCommand(_) | Mode::Command(_) | Mode::Factory(_))
    }

    /// A command view, an HQ's (mode 3) or a bunker's (mode 4): they share their screens,
    /// their panel and their input (`0x1008d51c`, docs/40).
    pub fn commands(self) -> bool {
        matches!(self, Mode::HqCommand(_) | Mode::Command(_))
    }
}

/// The bot the player has boarded, and the pilot its own input table drives it with.
pub struct Driving {
    pub target: usize,
    pub pilot: parkan_sim::input::Pilot,
    fire_held: bool,
    /// Taken from command mode (mode 2, telepresence): the hero stays where it stood, and
    /// leaving goes back to the command view.
    pub telepresence: bool,
}

/// `ORDER_ROBOT_UPGRADE`, which keeps a unit from being taken over (docs/40).
pub const ORDER_UPGRADE: i32 = 24;
/// A unit the hero boards: size class 4, a `b` chassis (docs/39, "Boarding").
pub const BOARDABLE_SIZE: u8 = 4;
/// Leaving tries eight places about the bot, π/4 apart from +x (`iron3d.dll:0x100633ce`); a
/// flyer must be less than this above the place's ground (`0x1006347c`); the hero is dropped
/// this far above the highest surface there (`0x100634e4`).
pub const LEAVE_PLACES: usize = 8;
pub const LEAVE_FLYER_HEIGHT: f32 = 10.0;
pub const LEAVE_DROP: f32 = 8.0;
/// "Risk area! Landing impossible." (`0x10063542`), and its voice.
pub const STRING_RISK_AREA: u32 = 6211;
pub const VOICE_RISK_AREA: &str = "VOICE_RISK_AREA";
pub const VOICE_SELECTED_B: &str = "VOICE_SELECTED_B";
/// The mission message `iron3d.dll` asks for when the player takes over a flyer
/// (`0x100638a7`, docs/34 "Mission 02").
pub const MESSAGE_FLYER_TAKEN: i64 = 100;

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

/// What the wingman panel shows: each wingman's number, its target index and whether it is
/// chosen; whether the player is picking; and, while ordering, the order menu's rows, each
/// its string and whether it is enabled.
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub wingmen: Vec<(usize, usize, bool)>,
    pub picking: bool,
    pub rows: Vec<(u32, bool)>,
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
    /// Every building that carries guns on a turret, as a robot that does not move, with its
    /// target in the battle (docs/31, "Which objects run a behaviour").
    pub emplacements: Vec<(usize, Robot)>,
    /// Each target's name, for the wingman panel.
    pub names: Vec<String>,
    pub selector: Selector,
    voice_pick: VoicePick,
    /// Whether a captured bot is given Standby ([`Play::enter`]); off, as the game's.
    pub capture_standby: bool,
    /// Knocked-off parts in flight.
    pub flights: Vec<Flight>,
    /// Rounds whose flight is over, where each stopped, and when each goes: a round stays its
    /// controller's `+92` ms, and its effects with it (docs/29, "A beam outlives its round").
    pub spent: Vec<(Round, f64)>,
    /// Each round's target point: who fired it (none for the hero) and the muzzle in that
    /// shooter's node-0 frame, which the round's bolt starts from.
    anchors: HashMap<u64, (Option<usize>, [f64; 3])>,
    /// Dead units and when each is deleted; and each target deleted.
    pub deaths: Vec<(usize, f64)>,
    pub deleted: Vec<bool>,
    /// What the game says, not yet shown or played.
    pub says: Vec<Say>,
    /// While a briefing plays, every object but the hero is paused (property `0x20a`) and
    /// the clan scripts wait (docs/21-briefing.md, "The world meanwhile").
    pub paused: bool,
    /// The buildings with doors or a control pod.
    pub buildings: Vec<Building>,
    /// The interface's mode stack, its front last.
    pub modes: Vec<Mode>,
    /// The buildings the player has selected, by target.
    pub selected: Vec<usize>,
    /// The plants, and what each builds.
    pub factories: Vec<Factory>,
    /// The difficulty's level ratio a unit's hit points take (docs/26).
    pub ratio: f32,
    /// How many units the play has made.
    pub spawned: usize,
    /// Units made since the drawing last caught up, by target.
    pub added: Vec<usize>,
    /// Every building's load-group effects, and the value each door-driven one last showed.
    pub building_effects: Vec<(BuildingEffects, Vec<f32>)>,
    /// The bot the player drives, while the hero is aboard it.
    pub driving: Option<Driving>,
    /// Command mode's camera, kept between visits (docs/40).
    pub command: crate::command::Camera,
    /// The outer camera, and the mode it was turned on in: every change of mode from 0, 1 or 2
    /// turns it off (docs/30, "The outer camera").
    pub outer: crate::camera::Outer,
    outer_mode: Mode,
    /// `Iron_3D.ini`'s `MOUSE_SENS` × 0.01, the mouse filter's multiplier while nothing is
    /// zoomed.
    pub mouse_sensitivity: f32,
    /// The commander's selection, lodes and build marks (docs/41).
    pub commander: crate::selection::Commander,
    /// The schemes, sites and construction spheres (docs/32).
    pub construction: crate::construction::Construction,
    /// Ore and power (docs/23).
    pub economy: crate::economy::Economy,
    /// Each clan's research tree, and the research centres' queues (docs/16).
    pub research: crate::research::Research,
    /// The main teleports' places (docs/27, "The main teleport").
    pub places: Vec<crate::places::Places>,
    /// The units the hero's Enter took, which hold no mind (see [`Play::free_minds`]).
    pub mindless: Vec<usize>,
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

/// What a behaviour sees of an object: its target in the battle (none for the hero), what
/// it is, and its clan.
type Sighting = (Option<usize>, Seen, Option<i64>);

/// The fire control's target, which is target `t`'s `robot`'s takt handed, reaches every gun:
/// an AI turret traces a point, so its unguided guns take it too (docs/29). Each gun fires
/// once its AI timer runs out and its score clears the bar, or freely during a search or an
/// attack (docs/29, "How the AI fires").
fn aim_and_fire(
    robot: &mut Robot,
    t: usize,
    takt: &Takt,
    seen: &[Sighting],
    battle: &Battle,
    ground: &Ground,
) {
    let now = robot.time_ms;
    let at = robot.walker.body.position;
    let target = takt.target.and_then(|id| seen.iter().find(|(_, s, _)| s.id == id)).and_then(|(i, _, _)| *i);
    if target != robot.fire_target {
        robot.fire_target = target;
        for g in &mut robot.guns {
            g.relink(target);
        }
    }
    let Some(victim) = target.and_then(|i| battle.combat.targets.get(i)) else {
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
    // STAND-IN: docs/29-weapons.md#how-the-ai-fires--read -- which of the fight module's bars
    // a building's guns clear is not read: a building flies not, and takes the walker's.
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
            let turret = if settled { 1.0 } else { 1.0 - std::f32::consts::PI * distance / reach.max(1.0) };
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
        let clear = match battle.combat.first_hit(ground, Some(t), muzzle, point, 0.0) {
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

/// Whether part `p` of `robot` is its turret or hangs from it, however far down.
fn carried_by_turret(robot: &Robot, p: usize) -> bool {
    let mut part = p;
    for _ in 0..robot.parts.len() {
        if part == robot.turret_part {
            return true;
        }
        match robot.parts.get(part).and_then(|x| usize::try_from(x.host).ok()) {
            Some(host) if host != part => part = host,
            _ => return false,
        }
    }
    false
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

/// The push a unit takes this frame (docs/24, "Collision between objects"): its sphere, swept
/// from `from` to `to`, against the level-0 faces of every placed object whose sphere it
/// meets, its own solid `mover` excepted. Each obstacle sees the end the ones before it have
/// already moved, as the pass reads B's end with its push so far.
///
/// A building's faces push every unit on it or near it, the one it stands on included,
/// through the building's own pass (docs/24, "Walking into a building"), and an open door's
/// faces let it by.
///
/// STAND-IN: docs/24-motion.md#not-established -- the masses property `0x7c` gives a unit and
/// a static object are not read, so a pair's push cannot be shared by mass squared: every
/// unit is run as the mover against every obstacle and takes the whole push. Against a
/// building, a tree or a stone that is what the pass gives anyway, the obstacle having no
/// contact record; between two units it moves both sides where the game moves the lighter
/// one further.
fn collision_push(solids: &[Solid], from: Vec3, to: Vec3, radius: f32, mover: Option<usize>) -> Vec3 {
    let mut total = Vec3::ZERO;
    for (i, obstacle) in solids.iter().enumerate() {
        if !obstacle.present || mover == Some(i) {
            continue;
        }
        let end = to + total;
        if parkan_sim::hit::swept_spheres(
            (from, end),
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
    total
}

impl Play {
    /// The mission's hero armed, its map's ground, every other object a target, and the
    /// effects they can play loaded.
    pub fn load(game: &Path, mission: &Mission) -> Result<Option<Play>> {
        let mut assembly = Assembly::new(game)?;
        let Some(mut hero) = Hero::load(&mut assembly, mission)? else { return Ok(None) };
        let dir = terrain::map_dir(game, &mission.map_path)?;
        let land = landmesh::load(&gamedir::resolve(&dir, "Land.msh").context("the map has no Land.msh")?)?;
        let ratio = settings::level_ratio(game);
        let mut battle = Battle::load(&mut assembly, mission, Some(hero.object), ratio)?;
        hero.arm(&mut battle, &mut assembly);
        let mut robots = Vec::new();
        let mut emplacements = Vec::new();
        for t in 0..battle.objects.len() {
            let o = battle.objects[t];
            if mission.objects[o].kind == KIND_UNIT
                && let Some(mut robot) = Robot::load(&mut assembly, mission, o)?
            {
                robot.arm(&mut battle, &mut assembly);
                robots.push((t, robot));
            } else if mission.objects[o].kind == KIND_BUILDING
                && let Some(mut robot) = Robot::load(&mut assembly, mission, o)?
            {
                robot.arm(&mut battle, &mut assembly);
                if !robot.guns.is_empty() {
                    emplacements.push((t, robot));
                }
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
            for e in &k.effects {
                fx.template(&e.name);
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
        ground.cuts = terrain::building_cuts(&mut assembly, mission)
            .into_iter()
            .map(parkan_sim::ground::Cut::new)
            .collect();
        let materials_for = |t: usize, part: usize, material: u16| {
            let name = battle.wears.get(t)?.get(part)?.get(usize::from(material & 0xFF))?;
            materials.get(name).map(|m| (m.surface, m.damage_rate, crate::models::doorway(name)))
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
        let mut buildings: Vec<Building> = (0..target_count)
            .filter_map(|t| Building::load(&mut assembly, mission, battle.objects[t], t))
            .collect();
        for b in &mut buildings {
            if let Some(part) = battle.combat.targets.get(b.target).and_then(|t| t.parts.get(b.part)) {
                b.place_zone(part);
            }
        }
        let factories: Vec<Factory> = (0..target_count)
            .filter_map(|t| Factory::load(&mut assembly, mission, battle.objects[t], t))
            .collect();
        let building_effects: Vec<(BuildingEffects, Vec<f32>)> = (0..target_count)
            .filter_map(|t| BuildingEffects::load(&mut assembly, mission, battle.objects[t], t))
            .map(|b| {
                let n = b.effects.len();
                (b, vec![0.0; n])
            })
            .collect();
        let battle_objects = battle.objects.clone();
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
            paused: false,
            names,
            selector: Selector::default(),
            voice_pick: VoicePick::default(),
            capture_standby: false,
            flights: Vec::new(),
            spent: Vec::new(),
            anchors: HashMap::new(),
            deaths: Vec::new(),
            deleted: vec![false; target_count],
            robots,
            emplacements,
            buildings,
            modes: vec![Mode::OnFoot],
            selected: Vec::new(),
            factories,
            ratio,
            spawned: 0,
            added: Vec::new(),
            building_effects,
            driving: None,
            command: crate::command::Camera::default(),
            outer: crate::camera::Outer::default(),
            outer_mode: Mode::OnFoot,
            mouse_sensitivity: crate::hero::mouse_sensitivity(game) * parkan_sim::input::SENSITIVITY_SCALE,
            commander: crate::selection::Commander::new(mission, &battle_objects),
            construction: crate::construction::Construction::new(mission, &battle_objects),
            economy: crate::economy::Economy::new(mission, &battle_objects),
            research: crate::research::Research::default(),
            places: Vec::new(),
            mindless: Vec::new(),
        };
        play.research = crate::research::Research::load(game, mission, &battle_objects, &play.units);
        play.load_places(mission);
        for i in 0..play.turret_effects.len() {
            let e = play.turret_effects[i].clone();
            let frame = play.turret_frame(&e);
            play.fx.start(Owner::Turret(e.id), &e.name, frame, 1.0, 0.0, None);
            // STAND-IN: docs/11-effects.md#how-a-sound-is-heard--read-and-measured -- the
            // hero's breath is measured silent in Mission 01's recording though its effect is
            // read to run, and what silences it is not established: the hero's own turret
            // effects flagged 0x800 (its breath; its helm light is a light) play no sound.
            for instance in play.fx.owned(Owner::Turret(e.id)) {
                instance.silent = instance.effect.header.flags & parkan_formats::fxid::FX_PASS_ONLY != 0;
            }
        }
        for e in play.chassis_effects.clone() {
            let (at, y) = play.hero.chassis_point(e.node);
            play.fx.start(Owner::Chassis(e.id), &e.name, Frame::along(at, y, 1.0), 1.0, 0.0, None);
        }
        play.start_building_effects();
        play.join_sites();
        Ok(Some(play))
    }

    /// Every building's load group as it is placed (docs/13, "A building's load group"): its
    /// action-3 and action-4 effects on its nodes and control points, each in its own time
    /// mode, then action 10's starts by id. Action 5's, the construction sphere, is left to
    /// construction.
    fn start_building_effects(&mut self) {
        for (b, _) in &self.building_effects {
            let Some(part) = self.battle.combat.targets.get(b.target).and_then(|t| t.parts.get(b.part))
            else {
                continue;
            };
            for e in &b.effects {
                if let Some(frame) = b.frame(part, e.on) {
                    self.fx.start(Owner::Building(b.target, e.id), &e.name, frame, 1.0, 0.0, None);
                }
            }
            for &(id, mode) in &b.starts {
                self.fx.restart(Owner::Building(b.target, id), 0.0, Some(mode));
            }
        }
    }

    /// A building's load-group effects follow the nodes they hang on, and an effect on a door's
    /// node reads the door's channel value, as time modes 4, 16 and 17 read a node's animation
    /// value (docs/11, "Effect time t").
    fn follow_building_effects(&mut self) {
        for (b, last) in &mut self.building_effects {
            let Some(part) = self.battle.combat.targets.get(b.target).and_then(|t| t.parts.get(b.part))
            else {
                continue;
            };
            let doors = self.buildings.iter().find(|d| d.target == b.target);
            for (k, e) in b.effects.iter().enumerate() {
                let Some(frame) = b.frame(part, e.on) else { continue };
                let value = match e.on {
                    crate::building_fx::On::Node(node) => doors.and_then(|d| {
                        d.doors
                            .iter()
                            .flat_map(|door| door.item.channels.iter().zip(&door.item.now))
                            .find(|(c, _)| c.node == node as i32)
                            .map(|(_, &v)| v.clamp(0.0, 1.0))
                    }),
                    crate::building_fx::On::Points(_) => None,
                };
                for instance in self.fx.owned(Owner::Building(b.target, e.id)) {
                    instance.frame = frame;
                    if let Some(v) = value {
                        last[k] = crate::building_fx::monotone(instance.mode, last[k], v);
                        instance.value = last[k];
                    }
                }
            }
        }
    }

    /// Load the mission's progression from `mission_dir`: its player clan's script, its
    /// messages and objectives, and the designs its `mission.cfg` prebuilds.
    pub fn load_progression(&mut self, game: &Path, mission_dir: &Path, mission: &Mission) -> Result<()> {
        self.progression = Some(Progression::load(game, mission_dir, mission, self.hero.object)?);
        crate::factory::prebuild(self, game, mission_dir).context("the prebuilt designs")?;
        // What the clans' `Init` handlers ordered: Mission 03's enemy patrol shut down.
        self.deliver_orders();
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
        self.place_hero(Vec3::new(at.x, at.y, at_target.z + 20.0), (-facing.x).atan2(facing.y));
        true
    }

    /// Stand the hero on the highest ground or building floor at (x, y), turned to `yaw`.
    /// False where there is none.
    pub fn stand_at(&mut self, x: f32, y: f32, yaw: f32) -> bool {
        self.stand_below(x, y, 10_000.0, yaw)
    }

    /// Stand the hero on the highest ground or building floor at (x, y) at or below `top`,
    /// turned to `yaw`: a buried room under the landscape, as a main teleport's chamber.
    pub fn stand_below(&mut self, x: f32, y: f32, top: f32, yaw: f32) -> bool {
        let Some(hit) = self.ground.below(x, y, top) else { return false };
        self.place_hero(Vec3::new(x, y, hit.point.z + 2.0), yaw);
        true
    }

    /// Stand the hero on the control pod of the building that is target `t`, from above its
    /// centre. False when it has none.
    pub fn stand_on_pod(&mut self, t: usize) -> bool {
        let Some(b) = self.buildings.iter().find(|b| b.target == t) else { return false };
        let Some(part) = self.battle.combat.targets.get(t).and_then(|x| x.parts.get(b.part)) else {
            return false;
        };
        let Some(centre) = b.pod_centre(part) else { return false };
        let heights = b.zone_heights(part);
        let yaw = self.hero.walker.body.yaw;
        self.place_hero(centre + Vec3::Z * 4.0, yaw);
        // A pod room whose floor stands higher over the node's centre, as the main teleport's
        // does, is stood on from under the top of the zone's box.
        if let Some((low, high)) = heights
            && !(low..high).contains(&self.hero.collision_centre().z)
            && let Some(hit) = self.ground.below(centre.x, centre.y, high - 0.5)
        {
            self.place_hero(Vec3::new(centre.x, centre.y, hit.point.z + 2.0), yaw);
        }
        true
    }

    fn place_hero(&mut self, at: Vec3, yaw: f32) {
        let w = &mut self.hero.walker;
        w.body.position = at;
        w.body.yaw = yaw;
        w.follow_ground(&self.ground);
        w.from = (w.body.position, w.body.yaw);
        w.from_heading = w.body.yaw;
    }

    /// Whether clan `other` is hostile to the player's: another clan, not nature's, toward
    /// which the player's clan's relation word is 0 (`iron3d.dll:0x10039440`).
    pub fn hostile(&self, other: Option<i64>) -> bool {
        self.hostile_to(Some(self.player_clan), other)
    }

    /// Whether clan `other` is hostile to clan `us`, by `us`'s relation word toward it.
    pub fn hostile_to(&self, us: Option<i64>, other: Option<i64>) -> bool {
        let Some(us) = us else { return false };
        let Some(other) = other.filter(|&c| c != us) else { return false };
        let (Some(them), Some(us)) = (self.clan(other), self.clan(us)) else { return false };
        them.kind != CLAN_NATURE
            && us
                .relations
                .iter()
                .find(|(name, _)| *name == them.name)
                .is_some_and(|&(_, w)| w == RELATION_HOSTILE)
    }

    /// Whether clan `other` is an ally of clan `us`, by `us`'s relation word toward it, which a
    /// dock asks before it charges an occupant (`Behavior.dll:0x10019318`).
    pub fn allied_to(&self, us: Option<i64>, other: Option<i64>) -> bool {
        let Some(us) = us else { return false };
        let Some(other) = other.filter(|&c| c != us) else { return false };
        let (Some(them), Some(us)) = (self.clan(other), self.clan(us)) else { return false };
        us.relations.iter().find(|(name, _)| *name == them.name).is_some_and(|&(_, w)| w == RELATION_ALLIED)
    }

    /// Whether clan `clan`'s objects run their behaviour: every clan's but a neutral one's,
    /// which runs no radar, takt or fire control (`Behavior.dll:0x10005070`, docs/31, "Which
    /// objects run a behaviour").
    pub fn thinks(&self, clan: Option<i64>) -> bool {
        clan.and_then(|c| self.clan(c)).is_some_and(|c| c.kind != CLAN_NEUTRAL)
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
        let mut world = self.contacts();
        let driven = self.driven_target();
        if let Some(c) = driven.and_then(|t| world.get_mut(t)) {
            // The driven bot is not its own contact.
            c.alive = false;
        }
        let unit = self.driven().walker.body.position;
        let range = self.driven().radar.range;
        let now = self.hero.time_ms;
        let radar = match driven.and_then(|t| self.robots.iter_mut().find(|(rt, _)| *rt == t)) {
            Some((_, robot)) => &mut robot.radar,
            None => &mut self.hero.radar,
        };
        let contacts = radar.scan(now, unit, &world).to_vec();
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
                self.enter_or_board();
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
            // In view state 1 only: the outer camera's view lets Z be (`0x10072428`).
            CMD_JAMES_ZOOM_MODE => {
                if !self.outer.on() && matches!(self.mode(), Mode::OnFoot | Mode::Driving(_)) {
                    let robot = self.driven_mut();
                    let widest = robot.rig.camera_values[2];
                    robot.zoom.toggle(widest);
                }
                false
            }
            CMD_JAMES_OUTER_CAMERA => {
                self.press_outer();
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

    /// An ordered unit of size class `class` acknowledges, never with the voice it gave last
    /// (`0x1008e840`, docs/31, "The wingman menu from first person").
    pub fn acknowledge(&mut self, class: u8) {
        let name = format!("{}{}", ACKNOWLEDGEMENTS[self.voice_pick.pick()], orders::voice_suffix(class));
        self.say_sound(&name, true);
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
                    self.acknowledge(class);
                }
                true
            }
        }
    }

    /// What the wingman panel shows now (`0x100432f0`, `0x1007aaa0`): a line for every
    /// wingman, with the selector off too, and none without one.
    pub fn panel(&self) -> Option<Panel> {
        let wingmen = self.wingmen();
        if wingmen.is_empty() {
            return None;
        }
        let lines = wingmen
            .iter()
            .enumerate()
            .take(16)
            .map(|(i, &r)| (i + 1, self.robots[r].0, self.selector.chosen.contains(&i)))
            .collect();
        let rows = if self.selector.state == orders::State::Ordering {
            let chosen: Vec<usize> =
                self.selector.chosen.iter().filter_map(|&i| wingmen.get(i).copied()).collect();
            let capturers = chosen.iter().all(|&r| matches!(self.robots[r].1.size_class, 1 | 2));
            orders::ROWS
                .iter()
                .enumerate()
                .map(|(i, row)| (row.string, orders::enabled(i, self.picked(), capturers)))
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
        self.mindless.push(t);
        if let Some(p) = self.progression.as_mut() {
            p.progress.captured(u.logical_id, self.player_clan);
        }
        // A unit the hero does not then board answers with its acknowledgement (`0x10072054`).
        if !self.boardable(t)
            && let Some(class) = self.robots.iter().find(|(rt, _)| *rt == t).map(|(_, r)| r.size_class)
        {
            self.acknowledge(class);
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
        self.sync_sensitivity();
        self.update_targets();
        self.refresh_present();
        self.tick_robots(dt_ms, mouse);
        // STAND-IN: docs/40-command-mode.md#not-established -- nothing read pops mode 3 when its HQ is
        // lost: the view rolls back to the HQ's cockpit, which the lost bot then puts the hero
        // out of, as below.
        if let Mode::HqCommand(h) = self.mode()
            && !self.battle.combat.targets.get(h).is_some_and(|x| x.alive)
        {
            self.roll_back();
        }
        // A driven bot that is lost rolls the stack back at once, in mode 1 or 2 (the unit
        // record's removal, `0x100751a0`, table `0x1007563c`): the hero is put out of a
        // boarded bot, and telepresence goes back to its command view.
        if let Some(d) = self.driving.as_ref()
            && !self.battle.combat.targets.get(d.target).is_some_and(|x| x.alive)
        {
            if d.telepresence {
                self.roll_back();
            } else {
                self.leave();
            }
        }
        let shots = if self.hero.dead() || self.hero_away() {
            self.hero.time_ms += dt_ms;
            Vec::new()
        } else {
            let from = self.hero.collision_centre();
            let shots = self.hero.tick(dt_ms, mouse, &self.ground);
            let lives = std::mem::take(&mut self.hero.lives);
            self.hero.relimit(|p, n| node_share(lives.get(p).and_then(Option::as_ref), n));
            self.hero.lives = lives;
            self.collide(from);
            self.footsteps();
            shots
        };
        let now = self.hero.time_ms;
        if !self.paused {
            self.tick_buildings(now);
            self.tick_places(now);
            self.tick_economy(now, (dt_ms / 1000.0) as f32);
            self.tick_factories(dt_ms);
            self.tick_research(dt_ms);
            self.check_research();
        }
        let mut events = self.ground_damage(now);
        if !self.paused {
            events.extend(self.tick_construction(now));
        }
        let launches = launches(&self.hero.robot, None, &shots, &self.battle, &self.ground);
        self.launch(launches, now);
        events.extend(self.battle.combat.tick((dt_ms / 1000.0) as f32, &self.ground));
        events.extend(self.battle.combat.takt_lives(now));
        for e in &events {
            self.effects_for(e, now);
            match *e {
                // A hit naming a building's node opens its door there (docs/24, "A shot opens a
                // door").
                Event::Struck { round, target: Some(t), part, node: Some(n), point } => {
                    let reach = self.battle.combat.kinds.get(round.kind).and_then(|k| k.hit.as_ref());
                    let reach = reach.filter(|x| x.kind == parkan_formats::exp::HIT_AREA).map(|x| x.radius);
                    let targets = &self.battle.combat.targets;
                    crate::buildings::open_shot_doors(
                        &mut self.buildings,
                        targets,
                        (t, part, n),
                        point,
                        reach,
                    );
                }
                Event::KnockedOff { target, part, node } => self.knock_off(target, part, node, now),
                Event::Staged { target, .. } | Event::Hidden { target, .. } => self.rebuild_solid(target),
                Event::Killed { target } => {
                    self.deaths
                        .push((target, now + self.battle.death_ms.get(target).copied().unwrap_or(0.0)));
                }
                Event::Ended { round, end } => self.spend(round, end, now),
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
        self.follow_building_effects();
        self.tick_views();
        // Flight effects follow their rounds. A round whose flight is over stays where it
        // stopped until its `+92` is up, and its effects go with it. Time modes 5–15 read the
        // round's speed over its top speed: the plasma bolt's and the missile's trails. Every
        // manager tick hands the effects the muzzle, carried with the shooter's node 0
        // (`Effect.dll:0x10003e16`), and a bolt starts there.
        let anchors = &mut self.anchors;
        self.spent.retain(|(r, gone)| {
            let stays = now < *gone;
            if !stays {
                anchors.remove(&r.id);
            }
            stays
        });
        let flying = self.battle.combat.rounds.iter().map(|r| {
            let top = self.battle.combat.kinds[r.kind].top_speed;
            (*r, if top > 0.0 { r.velocity.length() / top } else { 0.0 })
        });
        let rounds: Vec<(Round, f32)> = flying.chain(self.spent.iter().map(|(r, _)| (*r, 0.0))).collect();
        for (r, speed) in &rounds {
            let placed: Vec<(i32, Frame)> = self.battle.kinds[r.kind]
                .effects
                .iter()
                .map(|e| (e.id, self.round_frame(r, e.points)))
                .collect();
            let start = self.anchors.get(&r.id).and_then(|&(owner, local)| {
                let at = self.node_zero(owner)?.apply(local);
                Some(Vec3::new(at[0] as f32, at[1] as f32, at[2] as f32))
            });
            for (id, frame) in placed {
                for instance in self.fx.owned(Owner::Round(r.id, id)) {
                    instance.frame = frame;
                    instance.speed = *speed;
                    if let Some(at) = start.filter(|_| instance.takes_target_point()) {
                        instance.start_point = at;
                    }
                }
            }
        }
        self.fx.retain(|o, _| match o {
            Owner::Round(id, _) => rounds.iter().any(|(r, _)| r.id == *id),
            _ => true,
        });
        self.lode_plumes(now);
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
        // A won or lost mission plays on under its panel (docs/34, "After the outcome"). No
        // `Mission` handler or clan takt runs in the briefing's state 5.
        if !self.paused {
            self.progress();
            self.deliver_orders();
        }
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
    pub(crate) fn rebuild_solid(&mut self, t: usize) {
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
                materials.get(name).map(|m| (m.surface, m.damage_rate, crate::models::doorway(name)))
            });
        let mut solid = solid;
        if let Some(b) = self.buildings.iter().find(|b| b.target == t) {
            for node in solid.nodes.iter_mut().filter(|n| n.part == b.part) {
                node.open = b.open_door_node(node.node);
            }
        }
        if let Some(s) = self.ground.solids.get_mut(t) {
            *s = Solid { present: s.present, ..solid };
        }
    }

    /// The mode at the front of the interface's stack.
    pub fn mode(&self) -> Mode {
        self.modes.last().copied().unwrap_or(Mode::OnFoot)
    }

    /// Roll the stack back one mode (`0x10062ff0`): a building's screen gives the hero back
    /// to the player. The bottom mode stays.
    pub fn roll_back(&mut self) -> bool {
        if self.modes.len() <= 1 {
            return false;
        }
        match self.mode() {
            Mode::Driving(_) if self.driving.as_ref().is_some_and(|d| d.telepresence) => {
                return self.end_telepresence();
            }
            Mode::Driving(_) => return self.leave(),
            // Mode 3 → 1 (`0x10063ad0`): the HQ selected and taken back at auto-driver level 0
            // with the camera let go; or 3 → 4 and 3 → 3, back to the command view below.
            Mode::HqCommand(t) => {
                self.modes.pop();
                self.command.leave();
                self.command.release();
                match self.mode() {
                    Mode::Driving(below) if below == t => {
                        self.select_unit_alone(t);
                        self.auto_driver = 0;
                        self.take(t, false, true);
                    }
                    Mode::Command(b) => {
                        self.select_building(b);
                        let at = self.battle.combat.targets.get(b).map_or(Vec3::ZERO, |x| x.position);
                        self.command.enter(at);
                    }
                    Mode::HqCommand(h) => {
                        self.select_unit_alone(h);
                        self.ride(h);
                    }
                    _ => {}
                }
                return true;
            }
            // Mode 4 → 0 (`0x10063d60`): the hero is taken back where it stands, the selection
            // cleared, and the camera let go of its bunker; or 4 → 3, back to an HQ's view
            // (`0x100647e0`).
            Mode::Command(_) => {
                self.selected.clear();
                self.command.leave();
                self.command.release();
                self.hero.release_keys();
                self.modes.pop();
                if let Mode::HqCommand(h) = self.mode() {
                    self.select_unit_alone(h);
                    self.ride(h);
                }
                return true;
            }
            _ => {}
        }
        self.modes.pop();
        true
    }

    /// The hero button (`0x10062ce0` with 0): the stack rolled back to mode 0 a mode at a
    /// time, so from an HQ's view through its cockpit to the hero put down beside it. A step
    /// that is refused, as leaving a bot over a risk area is, stops it there.
    pub fn roll_back_to_foot(&mut self) {
        while self.modes.len() > 1 {
            let depth = self.modes.len();
            if !self.roll_back() || self.modes.len() >= depth {
                break;
            }
        }
    }

    /// Whether target `t` is an HQ unit: a robot whose turret carries the HQ flag (`IsHQ`,
    /// `0x10076f50`, docs/30).
    pub fn is_hq(&self, t: usize) -> bool {
        self.robots.iter().any(|(rt, r)| *rt == t && r.rig.hq)
    }

    /// The unit the hero rides in: the bot boarded from on foot, mode 1 at the bottom of the
    /// stack, whatever command view or telepresence stands on it.
    pub fn aboard(&self) -> Option<usize> {
        match self.modes[..] {
            [Mode::OnFoot, Mode::Driving(t), ..] => Some(t),
            _ => None,
        }
    }

    /// Whether the hero is out of the world: aboard a bot, or driving one from afar.
    pub fn hero_away(&self) -> bool {
        self.driving.is_some() || self.aboard().is_some()
    }

    /// The view's own unit (`+0xaec`): the one the player drives, else the one the hero rides
    /// in, whose place and radar the target list takes.
    pub fn driven_target(&self) -> Option<usize> {
        self.driving.as_ref().map(|d| d.target).or_else(|| self.aboard())
    }

    /// Mode 3 with HQ unit `t` pushed: from its cockpit (1 → 3, `0x10063a20`), from
    /// telepresence aboard it (2 → 3), from a bunker's view (4 → 3, `0x100647e0`) or from
    /// another HQ's (3 → 3, `0x10064900`). The HQ is let go to its AI with its order and
    /// selected, and the camera is placed on it facing north and rides with it. A unit that is
    /// not an HQ is refused (`0x10062c2e`); an HQ already on the stack is rolled back to.
    ///
    /// STAND-IN: docs/40-command-mode.md#an-hqs-command-mode-mode-3--read-and-seen -- how the
    /// stack reads after Enter in telepresence aboard an HQ is not followed: the telepresence
    /// is ended and mode 3 takes its place, over the command view it came from.
    pub fn enter_hq_command(&mut self, t: usize) -> bool {
        if !self.is_hq(t) || !self.battle.combat.targets.get(t).is_some_and(|x| x.alive) {
            return false;
        }
        if let Some(at) = self.modes.iter().position(|m| *m == Mode::HqCommand(t)) {
            self.modes.truncate(at + 1);
            return true;
        }
        match self.mode() {
            Mode::Driving(d) if d == t => {
                if self.driving.as_ref().is_some_and(|x| x.telepresence) {
                    self.modes.pop();
                }
                self.let_go();
            }
            Mode::Command(_) | Mode::HqCommand(_) => {}
            _ => return false,
        }
        self.select_unit_alone(t);
        self.ride(t);
        self.modes.push(Mode::HqCommand(t));
        true
    }

    /// The command camera placed on unit `t` and riding with it from distance 0.
    fn ride(&mut self, t: usize) {
        let (Some(target), Some(reach)) = (self.battle.combat.targets.get(t), self.ride_reach(t)) else {
            return;
        };
        self.command.ride(crate::command::Follow { at: target.position, reach });
    }

    /// How far back the command camera settles from HQ unit `t`: 8 × its record's `+0x98`.
    ///
    /// STAND-IN: docs/40-command-mode.md#not-established -- which bound `+0x98` is was not
    /// traced: the chassis mesh's authored sphere's radius, which on Mission 04's HQ gives the
    /// 61 m the recording favours, else the unit's whole bound.
    pub fn ride_reach(&self, t: usize) -> Option<f32> {
        let whole = self.battle.combat.targets.get(t)?.radius;
        let chassis = self.robots.iter().find(|(rt, _)| *rt == t).and_then(|(_, r)| {
            r.parts.get(r.chassis_part).and_then(|p| p.mesh.mesh.sphere).map(|(_, radius)| radius)
        });
        Some(crate::command::REACH_TIMES * chassis.unwrap_or(whole))
    }

    /// The driven unit let go (`0x10074ff0` with 0), back to its AI: its held keys dropped and
    /// its command cleared.
    fn let_go(&mut self) {
        if let Some(mut d) = self.driving.take()
            && let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == d.target)
        {
            crate::hero::drive_input(robot, &mut d.pilot, true);
            robot.walker.body.command = [0.0; 3];
            let_go(robot);
        }
    }

    /// Unit `t` taken by the player (`0x10074ff0` with 1): driven by its own input table's
    /// pilot with every held key let go and its walk cleared, its turret lock set when `lock`
    /// (auto-driver level 0). Pushes no mode.
    fn take(&mut self, t: usize, telepresence: bool, lock: bool) -> bool {
        let Some(r) = self.robots.iter().position(|(rt, _)| *rt == t) else { return false };
        let chassis = self.robots[r].1.parts[self.robots[r].1.chassis_part].record.clone();
        let Ok(pilot) = crate::hero::Hero::pilot_for(&mut self.assembly, &chassis) else { return false };
        self.hero.release_keys();
        let robot = &mut self.robots[r].1;
        robot.wizard.clear();
        robot.walker.drive = None;
        take_over(robot, lock);
        self.driving = Some(Driving { target: t, pilot, fire_held: false, telepresence });
        true
    }

    /// Mode 0 → 4 with the bunker that is target `t` (`0x10063ca0`): the bunker selected, the
    /// hero let go where it stands, the keys cleared, and the camera held around the bunker
    /// and placed over it facing north. A bunker already on the stack is rolled back to
    /// rather than pushed again (`0x10062a40`).
    pub fn enter_command(&mut self, t: usize) {
        if let Some(at) = self.modes.iter().position(|m| *m == Mode::Command(t)) {
            self.modes.truncate(at + 1);
            return;
        }
        self.hero.release_keys();
        self.selected.clear();
        self.selected.push(t);
        let at = self.battle.combat.targets.get(t).map_or(Vec3::ZERO, |x| x.position);
        self.command.enter(at);
        self.modes.push(Mode::Command(t));
    }

    /// A game command's key going down or up in command mode (`0x10071cd0`, `0x10072740`):
    /// the camera's moves and zoom. True when the command is command mode's.
    pub fn command_key(&mut self, command: &str, down: bool) -> bool {
        use crate::command::Move;
        use parkan_formats::controls::*;
        if !self.mode().commands() {
            return false;
        }
        let key = match command {
            CMD_JAMES_HQ_MOVE_LEFT => Move::Left,
            CMD_JAMES_HQ_MOVE_RIGHT => Move::Right,
            CMD_JAMES_HQ_MOVE_FORWARD => Move::Forward,
            CMD_JAMES_HQ_MOVE_BACKWARD => Move::Backward,
            CMD_JAMES_HQ_MOVE_UP => Move::Up,
            CMD_JAMES_HQ_MOVE_DOWN => Move::Down,
            CMD_JAMES_ZOOM_MODE => {
                if down {
                    self.command.toggle_zoom();
                }
                return true;
            }
            CMD_JAMES_BASE_ROTLEFT | CMD_JAMES_BASE_ROTRIGHT => {
                if down {
                    self.turn_ghost(command == CMD_JAMES_BASE_ROTLEFT);
                }
                return true;
            }
            _ => return false,
        };
        self.command.key(key, down);
        true
    }

    /// Command mode's camera this frame, at `now` real seconds with the cursor against
    /// `edges` (docs/40, "The camera"): held over the highest landscape or building surface,
    /// and in an HQ's view riding with the HQ where it now stands.
    pub fn command_frame(&mut self, now: f64, edges: crate::command::Edges) {
        match self.mode() {
            Mode::HqCommand(t) => {
                if let Some(at) = self.battle.combat.targets.get(t).map(|x| x.position) {
                    self.command.follow_to(at);
                }
            }
            Mode::Command(_) => {}
            _ => return,
        }
        let (_, hi) = self.ground.bounds();
        let ground = &self.ground;
        self.command
            .update(now, edges, hi[0].min(hi[1]), |x, y| ground.below(x, y, 1.0e5).map(|h| h.point.z));
    }

    /// The view's own unit: the bot the player drives or the hero rides in, or the hero.
    pub fn driven(&self) -> &Robot {
        self.driven_target()
            .and_then(|d| self.robots.iter().find(|(t, _)| *t == d))
            .map_or(&self.hero.robot, |(_, r)| r)
    }

    /// The view's own unit, to change.
    fn driven_mut(&mut self) -> &mut Robot {
        match self.driven_target().and_then(|d| self.robots.iter().position(|(t, _)| *t == d)) {
            Some(r) => &mut self.robots[r].1,
            None => &mut self.hero.robot,
        }
    }

    /// The eye the world is drawn from: command mode's camera, the outer camera, or the driven
    /// unit's.
    pub fn eye(&self) -> crate::robot::Eye {
        let own = self.own_eye();
        if !self.outer_shows() {
            return own;
        }
        // STAND-IN: docs/30-turrets.md#not-established -- which classes and faces the outer
        // camera's line meets (mask `0x41a`, `0x208`) is not followed: the ground, and every live
        // target but the unit looked at, passing what a round passes.
        let unit = self.outer.unit.flatten();
        let meets = |from, to| {
            self.battle.combat.first_hit(&self.ground, unit, from, to, 0.0).map(|(s, _, _)| s.point)
        };
        self.outer.place(&own, self.outer_bound(unit), meets)
    }

    /// The driven unit's own eye, or command mode's camera: what the right button picks along.
    pub fn own_eye(&self) -> crate::robot::Eye {
        if self.mode().commands() {
            return self.command.eye();
        }
        self.driven().eye().unwrap_or_else(|| self.hero.eye())
    }

    /// Whether the outer camera makes the view: turned on, in the mode it was turned on in.
    pub fn outer_shows(&self) -> bool {
        self.outer.on() && self.mode() == self.outer_mode
    }

    /// `CMD_JAMES_OUTER_CAMERA` (`0x10072244`): in modes 0, 1 and 2, on the driven unit
    /// (`0x10038b30`).
    fn press_outer(&mut self) {
        if !matches!(self.mode(), Mode::OnFoot | Mode::Driving(_)) {
            return;
        }
        let unit = self.driven_target();
        let flyer = self.driven().flyer;
        self.outer_mode = self.mode();
        self.outer.press(unit, flyer, self.hero.time_ms);
    }

    /// The bound the outer camera stands off by, the unit record's `+0x98` (`0x1007e5d6`).
    ///
    /// STAND-IN: docs/40-command-mode.md#not-established -- which bound the record's `+0x98`
    /// is: the half-diagonal of the chassis mesh's authored box, as the multi-part branch of
    /// `AniMesh.dll:0x10009d0f` works a radius out of a box (1.47 m on Mission 01's hero, which
    /// its recording's outer views favour), else the unit's collision radius.
    pub fn outer_bound(&self, unit: Option<usize>) -> f32 {
        let robot = unit
            .and_then(|t| self.robots.iter().find(|(rt, _)| *rt == t))
            .map_or(&self.hero.robot, |(_, r)| r);
        robot.chassis.mesh.corners.map_or(robot.collision.1, |corners| {
            let (lo, hi) =
                corners.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), c| {
                    (lo.min(Vec3::from_array(*c)), hi.max(Vec3::from_array(*c)))
                });
            (hi - lo).length() / 2.0
        })
    }

    /// A game frame's views (`0x1007d6e0`, `0x10038720`): the zoom of every unit of the player's
    /// clan steps, the outer camera moves, or is turned off when its mode is left or its unit
    /// lost, and the mouse filter takes the zoomed multiplier while the view is zoomed
    /// (`0x100a4fc0`).
    ///
    /// STAND-IN: docs/30-turrets.md#not-established -- how often the game frame runs, which
    /// paces the zoom's steps and the outer camera's ease: once a 60 Hz tick.
    fn tick_views(&mut self) {
        let widest = self.hero.rig.camera_values[2];
        self.hero.zoom.step(widest);
        let player = Some(self.player_clan);
        for (t, robot) in &mut self.robots {
            if self.units.get(*t).is_some_and(|u| u.clan == player) {
                let widest = robot.rig.camera_values[2];
                robot.zoom.step(widest);
            }
        }
        if self.outer.on() {
            let lost = self
                .outer
                .unit
                .flatten()
                .is_some_and(|t| !self.battle.combat.targets.get(t).is_some_and(|x| x.alive));
            if self.mode() != self.outer_mode || lost {
                self.outer.off();
            }
            self.outer.update(self.hero.time_ms);
        }
    }

    /// The mouse filter's multiplier, as `0x100a4fc0` hands it on at each change of view: 0.5
    /// while the driven unit's own view is zoomed, else `MOUSE_SENS` × 0.01.
    fn sync_sensitivity(&mut self) {
        let zoomed = !self.outer_shows() && self.driven().zoom.on;
        let s = if zoomed { crate::camera::ZOOMED_SENSITIVITY } else { self.mouse_sensitivity };
        match self.driving.as_mut() {
            Some(d) => d.pilot.sensitivity = s,
            None => self.hero.pilot.sensitivity = s,
        }
    }

    /// A key or button to the unit the player drives.
    pub fn key(&mut self, scan: &str, pressed: bool) {
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == d.target) {
                    crate::hero::drive_key(robot, &mut d.pilot, scan, pressed);
                }
            }
            None => self.hero.key(scan, pressed),
        }
    }

    /// Every key and button held on the unit the player drives comes up, as the game lets
    /// them go when its window is left (`stdSetApplicationState`, docs/14, "Leaving the
    /// window lets every key up").
    pub fn release_keys(&mut self) {
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == d.target) {
                    crate::hero::drive_input(robot, &mut d.pilot, true);
                }
            }
            None => self.hero.release_keys(),
        }
    }

    /// The input update of the unit the player drives.
    pub fn update_input(&mut self) {
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == d.target) {
                    crate::hero::drive_input(robot, &mut d.pilot, false);
                }
            }
            None => self.hero.update_input(),
        }
    }

    /// Whether Enter boards target `t` (`iron3d.dll:0x10071ff8`, `0x10076d30`, docs/39,
    /// "Boarding"): a unit of the player's clan, of size class 4, whose turret still has life,
    /// less than 20 away across the ground, while the player is on foot.
    ///
    /// STAND-IN: docs/39-boarding.md#boarding--read -- which of the turret's nodes property
    /// `0x52` reads the life of is not traced: the turret part's node 0.
    pub fn boardable(&self, t: usize) -> bool {
        let Some(u) = self.units.get(t) else { return false };
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        let Some(target) = self.battle.combat.targets.get(t) else { return false };
        let turret_alive = node_alive(target.parts.get(robot.turret_part).and_then(|p| p.life.as_ref()), 0);
        let near = robot.walker.body.position.truncate().distance(self.hero.walker.body.position.truncate())
            < CAPTURE_REACH;
        self.mode() == Mode::OnFoot
            && u.kind == KIND_UNIT
            && u.clan == Some(self.player_clan)
            && robot.size_class == BOARDABLE_SIZE
            && target.alive
            && turret_alive
            && near
    }

    /// Board target `t` (`0x100720e8`): the hero leaves the world, the bot is selected with
    /// `VOICE_SELECTED_B`, the player takes it at auto-driver level 0 with every held key let
    /// go, and a flyer taken over asks for the mission's message 100.
    pub fn board(&mut self, t: usize) -> bool {
        if !self.boardable(t) || !self.take(t, false, true) {
            return false;
        }
        let flyer = self.robots.iter().any(|(rt, r)| *rt == t && r.flyer);
        self.modes.push(Mode::Driving(t));
        self.say_sound(VOICE_SELECTED_B, true);
        if flyer && let Some(p) = self.progression.as_mut() {
            let notices = p.progress.call(parkan_sim::progression::MESSAGE_INFO, MESSAGE_FLYER_TAKEN);
            for n in &notices {
                let says = p.say(n);
                self.says.extend(says);
            }
        }
        true
    }

    /// Telepresence (mode 4 → 2, `0x10063e90`): from command mode the player takes unit `t` at
    /// auto-driver level `level`: the selection is the unit alone, the unit taken with every
    /// held key let go, the camera let go of its bunker, and the unit's view drawn. A unit that
    /// is not the player's, cannot be boarded, or is upgrading is refused (`0x10076d30`).
    ///
    /// STAND-IN: docs/40-command-mode.md#telepresence-mode-2--read -- the auto-driver levels'
    /// overrides are not modelled: the player drives the unit whole at every level.
    pub fn telepresence(&mut self, t: usize, level: u8) -> bool {
        if !self.mode().commands() || !self.can_take(t) || !self.take(t, true, level == 0) {
            return false;
        }
        self.select_unit_alone(t);
        self.auto_driver = level.min(2);
        self.command.leave();
        self.modes.push(Mode::Driving(t));
        true
    }

    /// Whether unit `t` can be taken over from command mode (`0x10076d30`, docs/39): the
    /// player's live unit with a turret whose node lives, not upgrading. Unlike boarding, any
    /// size will do.
    pub fn can_take(&self, t: usize) -> bool {
        let Some(u) = self.units.get(t) else { return false };
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        let Some(target) = self.battle.combat.targets.get(t) else { return false };
        u.kind == KIND_UNIT
            && u.clan == Some(self.player_clan)
            && target.alive
            && node_alive(target.parts.get(robot.turret_part).and_then(|p| p.life.as_ref()), 0)
            && robot.order.is_none_or(|o| o.code != ORDER_UPGRADE)
    }

    /// Mode 2 → 4 (`0x10063f30`): the unit let go, the selection cleared, and the camera held
    /// around its bunker again where the player left it; or 2 → 3 (`0x10063bf0`), the camera
    /// placed on the HQ again and pulled back out from it.
    fn end_telepresence(&mut self) -> bool {
        self.let_go();
        if matches!(self.mode(), Mode::Driving(_)) {
            self.modes.pop();
        }
        self.clear_selection();
        match self.mode() {
            Mode::Command(b) => {
                let at = self.battle.combat.targets.get(b).map_or(Vec3::ZERO, |x| x.position);
                self.command.hold(at);
            }
            Mode::HqCommand(h) => self.ride(h),
            _ => {}
        }
        true
    }

    /// Leave the boarded bot (`0x10063350`, `0x100638c0`, docs/39, "Leaving"): the first of
    /// eight places about it, both node spheres' radii out, with landscape under it that is not
    /// water, and for a flyer less than 10 below it, takes the hero 8 above the highest surface
    /// there, facing the bot. With none, "Risk area! Landing impossible." and the player stays
    /// aboard.
    pub fn leave(&mut self) -> bool {
        let Some(d) = self.driving.as_ref() else { return false };
        let t = d.target;
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        let at = robot.walker.body.position;
        let r = robot.bound.1 + self.hero.bound.1;
        let flyer = robot.flyer;
        let alive = self.battle.combat.targets.get(t).is_some_and(|x| x.alive);
        let place = if alive {
            (0..LEAVE_PLACES).find_map(|i| {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let p = at + Vec3::new(a.cos(), a.sin(), 0.0) * r;
                let face = self.ground.query(Vec3::new(p.x, p.y, at.z), false)?;
                let land = self.ground.land.faces.get(face.face?)?;
                let wet = land.is_water() || land.flags & FLAGS_LIQUID_BED_BIT != 0;
                if wet || (flyer && at.z - face.point.z >= LEAVE_FLYER_HEIGHT) {
                    return None;
                }
                Some(p)
            })
        } else {
            // A bot that is broken or gone puts the hero at (x − 1, y − 1), untested (`0x100634ad`).
            Some(at - Vec3::new(1.0, 1.0, 0.0))
        };
        let Some(place) = place else {
            if let Some(text) =
                self.progression.as_ref().and_then(|p| p.strings.get(&STRING_RISK_AREA)).cloned()
            {
                self.says.push(Say::Text(crate::progress::Sender::System, text));
            }
            self.say_sound(VOICE_RISK_AREA, true);
            return false;
        };
        let top = self.ground.below(place.x, place.y, 10_000.0).map_or(place.z, |h| h.point.z);
        let toward = (at - place).with_z(0.0).normalize_or(Vec3::Y);
        // STAND-IN: docs/39-boarding.md#not-established -- the heading is read as (F.x, −F.y)
        // under an assumed matrix layout; the hero is turned to face the bot.
        let yaw = (-toward.x).atan2(toward.y);
        self.let_go();
        if let Some(at) = self.modes.iter().rposition(|m| *m == Mode::Driving(t)) {
            self.modes.truncate(at);
        }
        self.hero.walker.body.velocity = [0.0; 3];
        self.place_hero(Vec3::new(place.x, place.y, top + LEAVE_DROP), yaw);
        self.hero.release_keys();
        true
    }

    /// Enter (`CMD_ENTER_STATE`, `0x10071f08`). On foot, a neutral unit is captured and then,
    /// as one of the player's own is, boarded if the hero can board it (`0x100720e8`). Aboard
    /// an HQ, or in telepresence aboard one, it opens the HQ's command view (`0x100720f2`,
    /// `0x10071f43`); aboard any other bot it does nothing.
    fn enter_or_board(&mut self) -> bool {
        match self.mode() {
            Mode::OnFoot => {}
            Mode::Driving(t) => return self.enter_hq_command(t),
            _ => return false,
        }
        let captured = self.enter();
        let Some(t) = self.targets.current else { return captured };
        self.board(t) || captured
    }

    /// Every building's doors and pod for this tick, with the units standing on it (a unit
    /// whose ground is the building's face is its child, docs/24): a door or pod that moved
    /// poses its nodes and rebuilds its faces, and a pod that fires runs the capture.
    fn tick_buildings(&mut self, now: f64) {
        let mut children: Vec<(usize, Standing)> = Vec::new();
        if !self.hero.dead()
            && self.driving.is_none()
            && let Some((s, _)) = self.hero.walker.ground.and_then(|h| h.solid)
        {
            children.push((
                s,
                Standing {
                    child: Child::Hero,
                    position: self.hero.walker.body.position,
                    centre: self.hero.collision_centre(),
                    radius: self.hero.collision.1,
                },
            ));
        }
        for (t, robot) in &self.robots {
            if let Some((s, _)) = robot.walker.ground.and_then(|h| h.solid)
                && self.battle.combat.targets.get(*t).is_some_and(|x| x.alive)
            {
                children.push((
                    s,
                    Standing {
                        child: Child::Robot(*t),
                        position: robot.walker.body.position,
                        centre: robot.collision_centre(),
                        radius: robot.collision.1,
                    },
                ));
            }
        }
        let mut moved = Vec::new();
        let mut fired = Vec::new();
        for b in &mut self.buildings {
            let standing: Vec<Standing> =
                children.iter().filter(|(s, _)| *s == b.target).map(|(_, c)| *c).collect();
            let Some(part) =
                self.battle.combat.targets.get_mut(b.target).and_then(|t| t.parts.get_mut(b.part))
            else {
                continue;
            };
            let phases: Vec<_> = b.doors.iter().map(|d| d.phase).collect();
            let (changed, fire) = b.tick(now, part, &standing);
            if changed {
                b.pose(part);
            }
            if changed || b.doors.iter().map(|d| d.phase).ne(phases) {
                moved.push(b.target);
            }
            fired.extend(fire);
        }
        for t in moved {
            self.rebuild_solid(t);
        }
        for f in fired {
            self.pod_fired(f);
        }
    }

    /// A pod's firing (`iron3d.dll:0x10061050`): a building of another clan changes owner,
    /// with string 5039 and its voice (`0x100a48a0`); then, for the player's own unit, the
    /// building opens (`0x10062630`): a plant's screen, or any other building selected.
    fn pod_fired(&mut self, fired: Fired) {
        let t = fired.target;
        let taker = match fired.child {
            Child::Hero => Some(self.player_clan),
            Child::Robot(r) => self.units.get(r).and_then(|u| u.clan),
        };
        let Some(taker) = taker else { return };
        let owner = self.units[t].clan;
        if owner != Some(taker) {
            self.units[t].clan = Some(taker);
            if let Some(p) = self.progression.as_mut() {
                p.progress.captured(self.units[t].logical_id, taker);
            }
            let old = owner.and_then(|c| self.clan(c));
            let taker_name = self.clan(taker).map(|c| c.name.clone());
            let word = old.and_then(|c| {
                c.relations.iter().find(|(name, _)| Some(name) == taker_name.as_ref()).map(|&(_, w)| w)
            });
            let said =
                crate::capture::announcement(self.player_clan, taker, owner, old.map(|c| c.kind), word);
            if let Some(p) = self.progression.as_ref() {
                if said.text
                    && let Some(text) = p.strings.get(&STRING_BUILDING_CAPTURED)
                {
                    self.says.push(Say::Text(crate::progress::Sender::System, text.clone()));
                }
                self.says.extend(said.voice.and_then(|v| p.sound(v)).map(Say::Voice));
            }
        }
        // The opening acts only for the player's own unit (`0x10062630`). Its switch on the Type
        // less `0x80000002` stops at `0x3e`: a main teleport opens nothing and selects nothing
        // (`0x100626f2`, docs/27, "Taking it").
        if fired.child != Child::Hero
            || taker != self.player_clan
            || self.units[t].type_word == parkan_sim::behaviour::MAIN_TELEPORT
        {
            return;
        }
        if !self.selected.contains(&t) {
            self.selected.push(t);
            if let Some(v) = self.progression.as_ref().and_then(|p| p.sound(VOICE_SELECTED)) {
                self.says.push(Say::Voice(v));
            }
        }
        // A plant's pod opens its screen, and a research centre's the research page
        // (`0x10062756`).
        if self.units[t].type_word == BUILDING_PLANT
            || self.units[t].type_word == crate::selection::RESEARCH_CENTRE
        {
            self.hero.release_keys();
            self.modes.push(Mode::Factory(t));
        } else if BUILDING_BUNKERS.contains(&self.units[t].type_word) {
            self.enter_command(t);
        }
    }

    /// Clan `clan`'s minds not held: its mission's count less every live robot of the clan
    /// (the hero among them) and every build its factories are running (docs/23, "The bot
    /// limit is the clan's mind count").
    ///
    /// STAND-IN: docs/34-progression.md#mission-04-teleport-end-to-end--derived --
    /// which of Mission 04's hero, helicopter and HQ holds no mind is not established: the
    /// recording's factory shows one free of three once all three are the player's, while
    /// Mission 02's shows the hero holding one. A unit the hero's Enter took holds none.
    pub fn free_minds(&self, clan: i64) -> usize {
        let minds =
            usize::try_from(clan).ok().and_then(|c| self.clans.get(c)).map_or(0, |c| c.minds as usize);
        let robots = self
            .units
            .iter()
            .zip(&self.battle.combat.targets)
            .enumerate()
            .filter(|(t, (u, target))| {
                u.clan == Some(clan)
                    && u.type_word & CLASS_ROBOT != 0
                    && target.alive
                    && !self.mindless.contains(t)
            })
            .count();
        let hero = usize::from(clan == self.player_clan && !self.hero.dead());
        let building = if clan == self.player_clan {
            self.factories
                .iter()
                .filter(|f| self.units[f.target].clan == Some(clan) && f.build.is_some())
                .count()
        } else {
            0
        };
        minds.saturating_sub(robots + hero + building)
    }

    /// A click on the factory screen of the plant that is target `target` (docs/36, "What the
    /// controls do").
    pub fn factory_click(&mut self, target: usize, click: crate::cockpit::factory::Click) {
        use crate::cockpit::factory::Click;
        let free = self.free_minds(self.player_clan);
        let Some(f) = self.factories.iter_mut().find(|f| f.target == target) else { return };
        match click {
            Click::Exit => {
                self.roll_back();
            }
            Click::Build | Click::Batch => {
                let batch = click == Click::Batch;
                if f.idle() {
                    f.start(batch, free);
                } else {
                    f.stop(batch);
                }
            }
            Click::Recent(i) => f.selected = Some(i),
            Click::Active => f.selected = None,
            // The designer opens from the screen (docs/37), which the cockpit keeps.
            Click::Constructor => {}
        }
    }

    /// Every factory's build for this tick: a bot that completes appears at its creation
    /// vertex, of the factory's clan, and is given the escape (docs/36, "Production").
    fn tick_factories(&mut self, dt_ms: f64) {
        for f in 0..self.factories.len() {
            // The build takes what the distribution brought, and asks the next step for its
            // request (docs/23, "Ore reaches the factory"); it draws `Use_Power` while its
            // power is still to collect.
            let t = self.factories[f].target;
            let mut held = self.economy.held(t);
            let project = self.factories[f].takt((dt_ms / 1000.0) as f32, &mut held);
            let request = self.factories[f].build.as_ref().map_or(0.0, |b| b.request);
            self.economy.ore.insert(t, (held, request));
            let drawing = self.factories[f].drawing_power();
            if let Some(site) = self.economy.site_mut(t) {
                site.usage = if drawing { site.use_power } else { 0.0 };
            }
            let Some(project) = project else { continue };
            let factory = &self.factories[f];
            let t = factory.target;
            let clan = self.units[t].clan.unwrap_or(self.player_clan);
            let place = factory.creation.as_ref().and_then(|v| {
                let part = self.battle.combat.targets.get(t)?.parts.first()?;
                crate::factory::vertex_world(v, part)
            });
            let Some(at) = place.or_else(|| self.battle.combat.targets.get(t).map(|x| x.position)) else {
                continue;
            };
            if let Some(unit) = self.spawn(&project, clan, at, 0.0) {
                // The bot is made at the factory's creation vertex, inside it (docs/36, "The
                // bot appears"), so it walks out along the building's own paths, as the
                // escape's 20-second check routes out a unit still on a building ("LEAVE IS
                // TOO !!!", docs/31, "The escape"). Without it the escape's straight line
                // would run into a wall.
                if place.is_some() {
                    self.construction.ways.entered.insert(unit, t);
                }
                if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == unit) {
                    let leave = orders::Order {
                        code: orders::LEAVE,
                        parameter: 0,
                        target: orders::Target::NotDefined,
                    };
                    robot.order = Some(leave);
                    robot.behaviour.order(&leave);
                }
                if clan == self.player_clan
                    && let Some(v) = self.progression.as_ref().and_then(|p| p.sound(VOICE_UNIT_READY))
                {
                    self.says.push(Say::Voice(v));
                }
            }
            let free = self.free_minds(clan);
            let factory = &mut self.factories[f];
            if factory.batch {
                factory.batch = false;
                factory.start(true, free);
            }
        }
    }

    /// Make a unit of `project`'s design for clan `clan` at `at`, turned to `yaw`
    /// (`CreateObjectFromScheme`, `Behavior.dll:0x1001d180`): a new target, robot, clan
    /// unit and name, joining its clan's list. Its target, or none when its design does not
    /// load as a robot.
    pub fn spawn(&mut self, project: &Project, clan: i64, at: Vec3, yaw: f32) -> Option<usize> {
        let logical_id =
            self.units.iter().map(|u| u.logical_id).chain([self.hero_id]).max().unwrap_or(0).max(0) + 1;
        let property = |name: &str, value: Value| mission::Property {
            name: name.to_owned(),
            kind: 0,
            value,
            minimum: value,
            maximum: value,
        };
        let placed = mission::Object {
            kind: KIND_UNIT,
            path: project.path.clone(),
            unknown_q: 0,
            logical_id,
            position: at.to_array(),
            pad: [0; 2],
            rotation: yaw,
            scale: [1.0; 3],
            name: String::new(),
            tail: (0, 0, 0, 0),
            properties: vec![
                property("ClanID", Value::Int(clan as i32)),
                property("Type", Value::Int(project.type_word as i32)),
            ],
        };
        let index = SPAWNED_OBJECTS + self.spawned;
        let mut robot = Robot::load_placed(&mut self.assembly, &placed, index).ok().flatten()?;
        let t = self.battle.add(
            &mut self.assembly,
            &self.clans,
            &placed,
            index,
            Some(self.player_clan),
            self.ratio,
        )?;
        robot.arm(&mut self.battle, &mut self.assembly);
        for k in &self.battle.kinds {
            for e in &k.effects {
                self.fx.template(&e.name);
            }
        }
        for kind in &self.battle.combat.kinds {
            for e in kind.hit.iter().chain(kind.range_end.iter()) {
                self.fx.preload_explosion(e);
            }
        }
        let profiles = gamedir::resolve(&self.assembly.game, parkan_formats::profiles::ARCHIVE)
            .and_then(|p| parkan_formats::nres::Archive::open(&p).ok());
        let designation =
            crate::robot::designation(&mut self.assembly, profiles.as_ref(), KIND_UNIT, &placed.path);
        self.units.push(Unit {
            clan: Some(clan),
            type_word: project.type_word,
            logical_id,
            kind: KIND_UNIT,
            announced: false,
            designation,
        });
        self.commander.paths.push(placed.path.clone());
        // A built bot is numbered by its clan: one more than the units the clan had named,
        // the hero among them (`0x10075d50`, docs/38, "The name"); the count here holds the
        // new unit already.
        let named = self.units.iter().filter(|u| u.clan == Some(clan) && u.kind == KIND_UNIT).count()
            + usize::from(clan == self.player_clan);
        let name = project.name.replacen("-X ", &format!("-{named} "), 1);
        self.names.push(name);
        self.deleted.push(false);
        let target = &self.battle.combat.targets[t];
        let solid = Solid::from_parts(&target.parts, target.centre, target.radius, false, |_, _| None);
        self.ground.solids.push(solid);
        self.robots.push((t, robot));
        if let Some(p) = self.progression.as_mut() {
            p.progress.join(logical_id, clan, project.type_word, at, self.hero.time_ms);
        }
        self.spawned += 1;
        self.added.push(t);
        Some(t)
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
    /// its target in the battle and its clan. `own` and `hostile` are the player's clan's;
    /// [`Play::seen_by`] turns them to another's.
    fn seen(&self) -> Vec<Sighting> {
        let mut seen: Vec<Sighting> = self
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
                    sensed: false,
                };
                (Some(i), s, u.clan)
            })
            .collect();
        if !self.hero.dead() && !self.hero_away() {
            let hero = Seen {
                id: self.hero_id,
                position: self.hero.walker.body.position,
                radius: self.hero.collision.1,
                type_word: ROBOT_HERO,
                building: false,
                own: true,
                hostile: false,
                sensed: false,
            };
            seen.push((None, hero, Some(self.player_clan)));
        }
        seen
    }

    /// What target `t` sees of `seen`: everything but itself, each of its own clan or
    /// hostile to it by its own clan's relations (`0x1000d460`), and marked with whether its
    /// own radar list holds it, `sensed` naming the ids that list carries.
    fn seen_by(&self, t: usize, seen: &[Sighting], sensed: &[i32]) -> Vec<Seen> {
        let (id, clan) = (self.units[t].logical_id, self.units[t].clan);
        seen.iter()
            .filter(|(_, s, _)| s.id != id)
            .map(|&(_, s, c)| Seen {
                own: clan.is_some() && c == clan,
                hostile: self.hostile_to(clan, c),
                sensed: sensed.contains(&s.id),
                ..s
            })
            .collect()
    }

    /// Every object a radar may find, by the same numbering `contacts` uses with the hero
    /// after the targets, so a kept answer stays good as units die.
    fn radar_world(&self) -> Vec<Contact> {
        let mut world = self.contacts();
        let hero = &self.hero;
        world.push(Contact {
            position: hero.walker.body.position,
            centre: hero.collision_centre(),
            radius: hero.collision.1,
            alive: !hero.dead() && !self.hero_away(),
            building: false,
            hostile: false,
            friend: true,
            unlisted: true,
        });
        world
    }

    /// The logical id of a `radar_world` entry.
    fn radar_id(&self, i: usize) -> Option<i32> {
        match self.units.get(i) {
            Some(u) => Some(u.logical_id),
            None => (i == self.units.len()).then_some(self.hero_id),
        }
    }

    /// The ids on unit `r`'s radar list now (docs/25, "What the AI does with it"): the
    /// behaviour's radar module re-reads its machine's radar list, whose scan holds for the
    /// radar's period. A unit with no radar senses 1 m and so lists nothing, and picks no
    /// target of its own.
    fn radar_ids(&mut self, robot: usize, emplacement: bool, world: &[Contact]) -> Vec<i32> {
        let list = if emplacement { &mut self.emplacements } else { &mut self.robots };
        let Some((_, unit)) = list.get_mut(robot) else { return Vec::new() };
        let (now, at) = (unit.time_ms, unit.walker.body.position);
        let contacts = unit.radar.scan(now, at, world).to_vec();
        contacts.iter().filter_map(|&i| self.radar_id(i)).collect()
    }

    /// Every other robot's tick: a robot of a clan that thinks, the enemy's too, runs its
    /// behaviour, which moves it through its Wizard and aims and fires its guns; then its
    /// machine and its turret, and its target in the battle and its faces follow where it
    /// stands. Then every building that carries guns.
    fn tick_robots(&mut self, dt_ms: f64, mouse: [f32; 2]) {
        let seen = self.seen();
        let world = self.radar_world();
        let mut fired = Vec::new();
        for r in 0..self.robots.len() {
            let t = self.robots[r].0;
            if !self.battle.combat.targets.get(t).is_some_and(|target| target.alive) {
                continue;
            }
            let driven = self.driving.as_ref().is_some_and(|d| d.target == t);
            if !self.paused && !driven && self.thinks(self.units[t].clan) {
                self.behave(r, dt_ms, &seen, &world);
            }
            let (t, robot) = &mut self.robots[r];
            let target = &mut self.battle.combat.targets[*t];
            let from = robot.collision_centre();
            let shots = match self.driving.as_mut().filter(|d| d.target == *t) {
                Some(d) => {
                    crate::hero::drive(robot, &mut d.pilot, &mut d.fire_held, dt_ms, mouse, &self.ground)
                }
                None => {
                    robot.advance(dt_ms, &self.ground);
                    robot.takt(dt_ms)
                }
            };
            // The collision pass, after the move and the ground contact: a unit is a mover
            // like the hero, so a building's walls hold it in until a door opens for it.
            let push = collision_push(
                &self.ground.solids,
                from,
                robot.collision_centre(),
                robot.collision.1,
                Some(*t),
            );
            if push.length_squared() >= NO_CONTACT {
                robot.walker.take_push(push);
            }
            robot.turn_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            robot.relimit(|p, n| node_share(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
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
        self.tick_emplacements(dt_ms, &seen, &world);
    }

    /// Every building that carries guns on a turret: its game time moves on; while its clan
    /// thinks, its behaviour's fire control picks its target, its turret traces it and its
    /// guns fire as an AI unit's do; then its turret's and guns' takt, and the turret and
    /// what hangs on it are posed where they aim. A building does not move: no machine
    /// steps, and its faces stay as placed.
    fn tick_emplacements(&mut self, dt_ms: f64, seen: &[Sighting], world: &[Contact]) {
        let mut fired = Vec::new();
        for e in 0..self.emplacements.len() {
            let t = self.emplacements[e].0;
            if !self.battle.combat.targets.get(t).is_some_and(|target| target.alive) {
                continue;
            }
            self.emplacements[e].1.time_ms += dt_ms;
            if !self.paused && self.thinks(self.units[t].clan) {
                let sensed = self.radar_ids(e, true, world);
                let others = self.seen_by(t, seen, &sensed);
                let Play { emplacements, battle, ground, .. } = self;
                let robot = &mut emplacements[e].1;
                let senses = Senses {
                    now_ms: robot.time_ms,
                    position: robot.walker.body.position,
                    seen: &others,
                    places: &[],
                    size_class: robot.size_class,
                    flyer: robot.flyer,
                    bounds: ground.bounds(),
                    has_weapon: !robot.guns.is_empty(),
                    walker_idle: true,
                    neutral: false,
                    building: true,
                    animal: false,
                };
                let takt = robot.behaviour.takt(&senses);
                aim_and_fire(robot, t, &takt, seen, battle, ground);
            }
            let (t, robot) = &mut self.emplacements[e];
            let shots = robot.takt(dt_ms);
            if !shots.is_empty() {
                fired.push((e, shots));
            }
            let place = robot.placement();
            let target = &mut self.battle.combat.targets[*t];
            for (p, part) in target.parts.iter_mut().enumerate() {
                if carried_by_turret(robot, p) {
                    for n in 0..part.nodes.len() {
                        part.nodes[n] = place.compose(&robot.part_pose(p, n));
                    }
                }
            }
        }
        for (e, shots) in fired {
            let (t, robot) = &self.emplacements[e];
            let launched = launches(robot, Some(*t), &shots, &self.battle, &self.ground);
            let now = robot.time_ms;
            self.launch(launched, now);
        }
    }

    /// Hand every order the scripts gave to its unit (function 15): a robot, or a building
    /// that carries guns, puts it in its list as its insert mode says (docs/34, "Mission
    /// 03").
    fn deliver_orders(&mut self) {
        let Some(p) = self.progression.as_mut() else { return };
        for o in std::mem::take(&mut p.orders) {
            let Some(t) = self.units.iter().position(|u| u.logical_id == o.id) else { continue };
            let robot = self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(rt, _)| *rt == t);
            if let Some((_, robot)) = robot
                && robot.behaviour.insert_order(&o.order, o.insert)
            {
                robot.order = Some(o.order);
            }
        }
    }

    /// One robot's behaviour takt (docs/31): its task's walk handed to the walker, which
    /// cuts it into points the Wizard follows; the fire control's target handed to its
    /// guns and traced by its turret; and each gun let fire once its AI timer runs out and
    /// its score clears the bar, or freely during a search or an attack (docs/29, "How the
    /// AI fires").
    fn behave(&mut self, r: usize, dt_ms: f64, seen: &[Sighting], world: &[Contact]) {
        let t = self.robots[r].0;
        self.escape_off_building(r);
        let sensed = self.radar_ids(r, false, world);
        let others = self.seen_by(t, seen, &sensed);
        let animal = self.units[t].type_word & CLASS_ANIMAL != 0;
        let bounds = self.ground.bounds();
        let capturing = matches!(
            self.robots[r].1.behaviour.task(),
            Task::Search { search: Search::Capture(_) | Search::Building(_), .. }
        );
        let places = if capturing { self.capture_places() } else { Vec::new() };
        let (_, robot) = &mut self.robots[r];
        let now = robot.time_ms;
        let at = robot.walker.body.position;
        let senses = Senses {
            now_ms: now,
            position: at,
            seen: &others,
            places: &places,
            size_class: robot.size_class,
            flyer: robot.flyer,
            bounds,
            has_weapon: !robot.guns.is_empty(),
            walker_idle: robot.wizard.idle(now),
            neutral: false,
            building: false,
            animal,
        };
        let takt = robot.behaviour.takt(&senses);
        let (flyer, top, low) = (
            robot.flyer,
            robot.walker.limits.top_speed[1].abs(),
            robot.walker.controller.triples[1][1].abs(),
        );
        let legs = match takt.walk {
            Walk::Keep => None,
            Walk::Clear => Some((Vec::new(), 0.0, false)),
            Walk::To(goal, share) => {
                let floor = self.ground.below(goal.x, goal.y, 10_000.0).map_or(goal.z, |h| h.point.z);
                // STAND-IN: docs/24-motion.md#not-established -- the height a flyer's points
                // are given, and who reads `Movement_FlyHeight`, are not read: a flyer's
                // points keep at least `FlyNearLandHeight` above the ground under them.
                let goal =
                    if flyer { goal.with_z(goal.z.max(floor + FLY_NEAR_LAND)) } else { goal.with_z(floor) };
                Some((self.legs_to(t, at, goal), share, false))
            }
            Walk::Inside(id, pod, share) => Some((self.legs_inside(t, at, id, pod), share, true)),
        };
        let (_, robot) = &mut self.robots[r];
        match legs {
            None => {}
            Some((legs, _, _)) if legs.is_empty() => robot.wizard.clear(),
            Some((legs, share, inside)) => {
                let speed = walk_speed(share * top, top, low, SPEED_MAXIMUM_FACTOR);
                // A walker's points run along the ground; a flyer's follow each point's height,
                // down to the hall way's vertices inside a building.
                let flags = if flyer { 0 } else { GROUND_POINT };
                let (points, stop) = if legs.len() == 1 && !inside {
                    straight_walk(at, legs[0], speed, now, flags)
                } else {
                    path_walk(at, &legs, speed, now, flags, inside)
                };
                robot.wizard.clear();
                robot.wizard.give(points);
                robot.wizard.stop_at(stop);
            }
        }
        robot.walker.drive = Some(robot.wizard.takt(now, at, dt_ms));
        aim_and_fire(robot, t, &takt, seen, &self.battle, &self.ground);
    }

    /// The unit takt's escape (`Behavior.dll:0x10005408`, docs/31, "The escape"): a unit with
    /// no order but its stop, standing on a live building that is not a ruin, is given the
    /// escape, replacing.
    ///
    /// STAND-IN: docs/31-packages.md#the-escape--read -- the node test (a unit on a damaged
    /// node of the building is left be) is not modelled: every node counts as whole.
    fn escape_off_building(&mut self, r: usize) {
        let (_, robot) = &self.robots[r];
        let Some((b, _)) = robot.walker.ground.and_then(|h| h.solid) else { return };
        if robot.behaviour.task() != Task::Stop
            || self.units.get(b).is_none_or(|u| u.kind != KIND_BUILDING || u.type_word == BUILDING_RUINE)
            || !self.battle.combat.targets.get(b).is_some_and(|x| x.alive)
        {
            return;
        }
        let leave = orders::Order { code: orders::LEAVE, parameter: 0, target: orders::Target::NotDefined };
        let robot = &mut self.robots[r].1;
        robot.order = Some(leave);
        robot.behaviour.order(&leave);
    }

    /// Rounds leaving their barrels: each round, and its load group's flight effects.
    fn launch(&mut self, launched: Vec<Launch>, now: f64) {
        for l in launched {
            if let Some(id) =
                self.battle.combat.fire(l.kind, l.owner, l.muzzle, l.direction, l.velocity, 1.0, l.target)
            {
                // The round's load group creates its flight effects at spawn, and the gun hands its
                // effect manager the muzzle on the shooter's node 0 (`Control.dll:0x1002a56d`).
                let round = *self.battle.combat.rounds.last().expect("just fired");
                for e in self.battle.kinds[l.kind].effects.clone() {
                    let frame = self.round_frame(&round, e.points);
                    self.fx.start(Owner::Round(id, e.id), &e.name, frame, 1.0, now, None);
                }
                if let Some(node) = self.node_zero(l.owner) {
                    let local = node.invert().apply(l.muzzle.to_array().map(f64::from));
                    self.anchors.insert(id, (l.owner, local));
                }
            }
        }
    }

    /// A placed object's faces are in the world while its target is alive (docs/26).
    fn refresh_present(&mut self) {
        for (i, target) in self.battle.combat.targets.iter().enumerate() {
            if let Some(s) = self.ground.solids.get_mut(i) {
                s.present = target.alive;
            }
        }
    }

    /// The collision pass for the hero (docs/24, "Collision between objects"), after its
    /// move and ground contact.
    fn collide(&mut self, from: Vec3) {
        let push = collision_push(
            &self.ground.solids,
            from,
            self.hero.collision_centre(),
            self.hero.collision.1,
            None,
        );
        if push.length_squared() >= NO_CONTACT {
            self.hero.walker.take_push(push);
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
        // A recording shows the hero's route reported where the bot it rides goes (docs/34,
        // "Mission 02").
        at.insert(self.hero_id, self.driven().walker.body.position);
        let now = self.hero.time_ms;
        let Some(p) = self.progression.as_mut() else { return };
        let notices = p.tick(now, |id| at.get(&id).copied());
        for n in &notices {
            let says = p.say(n);
            self.says.extend(says);
        }
    }

    /// A round's flight is over (docs/29, "A beam outlives its round"): its hit, edge or range
    /// group runs on its effects, and it stays where it stopped for its controller's `+92` ms.
    fn spend(&mut self, round: Round, end: RoundEnd, now: f64) {
        let Some(kind) = self.battle.kinds.get(round.kind) else { return };
        let gone = now + kind.death_ms;
        for command in kind.ends[end as usize].clone() {
            match command {
                EffectCommand::Start(id, mode) => {
                    self.fx.restart(Owner::Round(round.id, id), now, Some(mode))
                }
                EffectCommand::Switch(id, on) => self.fx.switch(Owner::Round(round.id, id), on),
                EffectCommand::Delete(id) => self.fx.remove(Owner::Round(round.id, id)),
            }
        }
        self.spent.push((round, gone));
    }

    /// Node 0 of the unit that fired a round, in the world: the hero's for `None`, else the
    /// robot's whose target `owner` is.
    ///
    /// STAND-IN: docs/29-weapons.md#not-established -- which matrix `AniMesh` slot `0x10` hands
    /// the effect manager for its argument 2 is not read: node 0's world pose, the unit's
    /// placement and its chassis's node 0 as drawn.
    fn node_zero(&self, owner: Option<usize>) -> Option<Pose> {
        let robot = match owner {
            None => &self.hero.robot,
            Some(t) => &self.robots.iter().find(|(r, _)| *r == t)?.1,
        };
        Some(robot.placement().compose(&robot.chassis_pose(0)))
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

/// A node's condition, its life over its maximum (docs/23, "What a value id is"), and whether
/// it is destroyed; a part that takes no damage is whole.
pub fn node_share(life: Option<&parkan_sim::damage::Life>, node: usize) -> (f32, bool) {
    match life.and_then(|l| l.nodes.get(node)) {
        Some(l) if l.max > 0.0 => ((l.life / l.max).clamp(0.0, 1.0), l.destroyed),
        Some(l) => (if l.destroyed { 0.0 } else { 1.0 }, l.destroyed),
        None => (1.0, false),
    }
}

/// A unit handed to the player (`iron3d.dll:0x10074ff0`): its turret lock, property 179, is
/// set at auto-driver level 0 (`0x100750fd`) and clear otherwise (docs/30, "The hull follows
/// the turret").
///
/// STAND-IN: docs/30-turrets.md#the-hull-follows-the-turret--read-and-measured -- the game's
/// lead takes every change to the turret's yaw target from the unit's start; whether the AI's
/// aiming reaches it through the same setter is not read. Here the lead starts from the
/// turret's target at the takeover, as if it had.
fn take_over(robot: &mut Robot, lock: bool) {
    let body = &mut robot.walker.body;
    body.turret_lock = lock;
    body.lead = robot.rig.aim[0];
    body.lead_step = 0.0;
}

/// A unit let go (control message 7 with 0, `Control.dll:0x10031a38`): the turret lock is off.
///
/// STAND-IN: docs/30-turrets.md#the-hull-follows-the-turret--read-and-measured -- the AI's
/// Wizard writes the spin once it drives again; a unit it does not drive keeps no spin.
fn let_go(robot: &mut Robot) {
    let body = &mut robot.walker.body;
    body.turret_lock = false;
    body.spin_set = [0.0; 3];
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
            corners: None,
        };
        Part { mesh: Rc::new(mesh), nodes: vec![IDENTITY], scale: 1.0, life: None, portals: Rc::default() }
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
