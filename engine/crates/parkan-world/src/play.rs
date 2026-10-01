//! A mission played: the hero, the ground it walks on, the battle around it, the
//! effects it plays, the player's target and the mission's progression.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use glam::{Mat4, Vec2, Vec3};
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
use parkan_formats::{arealmap, gamedir, landmesh};
use parkan_sim::behaviour::{
    Condition, SCORE_RAMP, SERVICE_GUNS, SERVICE_MAGAZINE, Search, Seen, Senses, Takt, Task, Usable, Walk,
    distance_score, fire_bar, fire_wait_ms,
};
use parkan_sim::combat::{Event, Part, Round, RoundEnd, Target};
use parkan_sim::damage::FLIGHT_MS;
use parkan_sim::damage::{Life, share_loss, touching};
use parkan_sim::effects::{Cue, Frame, Sprite};
use parkan_sim::ground::Ground;
use parkan_sim::guns::SINGLE_FIGHT;
use parkan_sim::hit::ROUND_SKIPS_FACE;
use parkan_sim::machine::Walker;
use parkan_sim::motion::GRAVITY;
use parkan_sim::orders::{self, ACKNOWLEDGEMENTS, Digit, Picked, Selector, VoicePick};
use parkan_sim::path::Graph;
use parkan_sim::progression::Notice;
use parkan_sim::solid::{self, NO_CONTACT, Solid};
use parkan_sim::targeting::{Contact, TargetList};
use parkan_sim::wizard::{GROUND_POINT, flight, flight_leg, path_walk, straight_walk, walk_speed};

use crate::assembly::Assembly;
use crate::battle::{Battle, EffectCommand};
use crate::building_fx::BuildingEffects;
use crate::buildings::{Building, Child, Fired, Standing};
use crate::console::{self, Command};
use crate::factory::{Factory, Project, VOICE_UNIT_READY};
use crate::fx::{Fx, Owner};
use crate::hero::{Hero, Reach};
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
/// How far over the highest surface under the fallen hero the level's second camera stands
/// (`0x100e5c8c`, docs/40, "View state 4").
pub const FALLEN_ABOVE: f32 = 16.0;
/// The campaign at list position 0, which the campaign screen starts in launch mode 4.
pub const TRAINING_CAMPAIGN: &str = "CAMPAIGN.00";
pub use parkan_formats::mission::{
    CLAN_NATURE, CLAN_NEUTRAL, RELATION_ALLIED, RELATION_HOSTILE, RELATION_NEUTRAL,
};
/// The colours a mark takes (`iron3d.dll:0x10065440`), r, g, b.
pub const MARK_OWN: [u8; 3] = [128, 128, 255];
pub const MARK_NATURE: [u8; 3] = [255, 255, 0];
pub const MARK_NEUTRAL_CLAN: [u8; 3] = [160, 160, 160];
pub const MARK_NEUTRAL: [u8; 3] = [255, 0, 255];
pub const MARK_ALLIED: [u8; 3] = [0, 255, 255];
pub const MARK_HOSTILE: [u8; 3] = [255, 0, 0];
pub const MARK_OTHER: [u8; 3] = [255, 255, 0];
/// A shield's effect instances, and the time mode a flash plays in (`0x10025ca0`).
pub const SHIELD_FLASHES: usize = 3;
pub const SHIELD_FLASH_MODE: u32 = parkan_formats::fxid::TIME_ONCE;
/// How high a flyer's walk point stands over what lies under it (`Behavior.dll:0x10040f20`, the
/// float at `0x10059974`): a compiled 15, not the profile's `FlyNearLandHeight`, whose one
/// reader nothing calls (docs/24, "A flyer's walk points").
pub const FLIGHT_CLEARANCE: f32 = 15.0;
/// What the ground under a walk point is taken to be over a building, a tree or a stone: its top
/// face and this much more (`Behavior.dll:0x100148a5`, the float at `0x100595bc`).
pub const OVER_OBSTACLE: f32 = 100.0;
/// An animal's walk point stands this much higher again, and up to the spread more at random,
/// drawn for each point (`0x10040f52`-`0x10040f7c`: 30 and `rand()` ÷ 32767 × 50).
pub const ANIMAL_FLIGHT: f32 = 30.0;
pub const ANIMAL_FLIGHT_SPREAD: f32 = 50.0;
/// `Speed_MaximumFactor` as every behaviour holds it: the compiled default of the difficulty
/// block (`Behavior.dll:0x10019b90`), since the one load that would replace it, the profile
/// loader's kind 5 (`0x1000a2f4`), is handed a `diff_*` name by nothing in the install
/// (docs/26, "The difficulty ratio").
pub const SPEED_MAXIMUM_FACTOR: f32 = 1.0;
/// A turret channel this close to its target counts as settled.
pub const AIM_SETTLED: f32 = 0.005;
/// A plant's `Type`: its pod opens the factory screen for the player (docs/27, "Capture").
pub const BUILDING_PLANT: u32 = 0x8000_0010;
/// A generator's `Type` (docs/27, "Capture").
pub const BUILDING_GENERATOR: u32 = 0x8000_0002;
/// The three bunkers' `Type`s, whose pods open command mode (`iron3d.dll:0x10062779`).
pub const BUILDING_BUNKERS: [u32; 3] = [0x8001_0000, 0x8002_0000, 0x8004_0000];
/// The medium and large towers' `Type`s, whose pods open their manual control, mode 6
/// (`iron3d.dll:0x10062bc0`, docs/27, "Capture").
pub const TOWERS: [u32; 2] = [0x8010_0000, 0x8020_0000];
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
/// How long a pressed *Explode!* waits before the unit's takt kills it (`[0x100e64ec]`, 0.6 s).
pub const EXPLODE_DELAY_MS: f64 = 600.0;
/// God mode's multipliers ([`Play::god_mode`]): the hero's speed, its hit points and its
/// rounds' damage.
pub const GOD_SPEED: f32 = 2.5;
pub const GOD_LIFE: f32 = 10.0;
pub const GOD_DAMAGE: f32 = 10.0;

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
    /// Mode 6 with the tower or bunker that is target `t`: its manual control, the player at its
    /// guns as at a boarded bot's (docs/27, "What the modes show").
    Manual(usize),
    /// Mode 7: the game menu over the world, which stands still under it (docs/39, "The game
    /// menu").
    GameMenu,
}

impl Mode {
    /// A mode whose screens show the cursor: command mode, a building's screen and the game
    /// menu (docs/40, docs/36, docs/39).
    pub fn shows_cursor(self) -> bool {
        matches!(self, Mode::HqCommand(_) | Mode::Command(_) | Mode::Factory(_) | Mode::GameMenu)
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
/// Where the level's probe for the highest surface at an x, y starts looking down from: above
/// any map's ground.
pub const PROBE_TOP: f32 = 10_000.0;
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
    /// The clan it was placed or made in, whose count its name took (`0x10075eb2`). The
    /// record's bind names it once; a capture changes `clan` and not the name: *seen*, a
    /// captured MFW-1 keeps its name (docs/35, "Name and status").
    pub named_clan: Option<i64>,
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
    /// The map's areals cut for the walker's search, where the map has a `Land.map`.
    pub graph: Option<Graph>,
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
    pub clans: Vec<Clan>,
    /// Each clan's relation word towards each clan, as the loader files them and the clan
    /// brains' takt leaves them: `attitudes` owns them and this is refreshed from it.
    pub relations: Vec<Vec<u32>>,
    /// The attitude behind each of those words, and the takt that re-reads one from the other
    /// (docs/25, "Clan relations").
    pub attitudes: parkan_sim::relations::Relations,
    /// Each shielded target's next effect instance, of three, and where each flash stands off
    /// its bubble's centre, which it follows.
    pub shield_flashes: HashMap<usize, usize>,
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
    /// Whether a building's guns hold their fire on a target below the lowest its turret's
    /// sight looks; off, as the game's.
    pub building_fire_floor: bool,
    /// God mode, a cheat the game does not have ([`Play::god_mode`]).
    pub god: bool,
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
    /// What a kill outside the frame's own steps set off, the console's `death`: handled with
    /// the next frame's events.
    pub pending_events: Vec<Event>,
    /// The units whose *Explode!* is pending, each with when it was pressed: the unit
    /// record's `+0x135` and `+0x12c` (docs/41, "Explode!").
    pub exploding: Vec<(usize, f64)>,
    pub deleted: Vec<bool>,
    /// What the game says, not yet shown or played.
    pub says: Vec<Say>,
    /// While a briefing plays, every object but the hero is paused (property `0x20a`) and
    /// the clan scripts wait (docs/21-briefing.md, "The world meanwhile").
    pub paused: bool,
    /// Once the hero is lost, the level's second camera the world is drawn from (view
    /// state 4, [`Play::fallen_eye`]).
    pub fallen: Option<crate::robot::Eye>,
    /// A mission of the training campaign, `CAMPAIGN.00`: the launch mode 4 the campaign
    /// screen writes for the campaign at list position 0, the game's `+0xe6` (docs/34, "The
    /// parameter block's modes").
    pub training: bool,
    /// The buildings with doors or a control pod.
    pub buildings: Vec<Building>,
    /// The interface's mode stack, its front last.
    pub modes: Vec<Mode>,
    /// The buildings the player has selected, by target.
    pub selected: Vec<usize>,
    /// The plants, and what each builds.
    pub factories: Vec<Factory>,
    /// Each clan's AI design store, loaded the first time its planner orders a build
    /// (docs/15, "Function 69 sets how sloppy the AI's design pick is").
    pub stores: HashMap<i64, crate::factory::Store>,
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
    /// Each clan's mind entries a build's start reserved (`Behavior.dll:0x1002a348`, written
    /// 0), which the clan's next takt frees again (`ai.dll:0x10006580`, see
    /// [`Play::free_minds`]).
    pub reserved: HashMap<i64, usize>,
    /// The walker's search's random source (docs/24, "The global path").
    pub walk_seed: u32,
    /// The random source an animal's flight heights draw from (docs/24, "A flyer's walk
    /// points"), kept apart from the search's so an animal's walk does not move anyone's route.
    pub flight_seed: u32,
    /// Each clan's current pasture, which its migrating animals ask for (docs/31, "Migrate:
    /// an animal's pasture").
    pub grazing: std::cell::RefCell<Grazing>,
}

/// Each clan's current pasture, as the system areal map keeps it for the clan's migrating
/// animals (slot 47, `ArealMap.dll:0x10022230`), and the random source its pick draws on.
#[derive(Clone, Debug)]
pub struct Grazing {
    pub clans: Vec<parkan_sim::behaviour::ClanPasture>,
    seed: u32,
}

impl Default for Grazing {
    fn default() -> Self {
        Self { clans: Vec::new(), seed: 0x2545_f491 }
    }
}

impl Grazing {
    /// The pasture clan `clan`, whose zones are `zones`, grazes when one of its animals asks at
    /// `now_ms`.
    pub fn ask(
        &mut self,
        clan: usize,
        zones: &[parkan_sim::behaviour::Pasture],
        now_ms: f64,
    ) -> Option<parkan_sim::behaviour::Pasture> {
        if self.clans.len() <= clan {
            self.clans.resize(clan + 1, Default::default());
        }
        let seed = &mut self.seed;
        let random = || {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 17;
            *seed ^= *seed << 5;
            (*seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let i = self.clans[clan].ask(zones.len(), now_ms, random)?;
        zones.get(i).copied()
    }
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

/// What a behaviour sees of an object: its target in the battle (the hero's number after the
/// targets), what it is, and its clan.
type Sighting = (Option<usize>, Seen, Option<i64>);

/// The fire control's target, which is target `t`'s `robot`'s takt handed, reaches every gun:
/// an AI turret traces a point, so its unguided guns take it too (docs/29). Each gun fires
/// once its AI timer runs out and its score passes the bar its unit's Type `type_word` and
/// live speed set, or freely during a search or an attack (docs/29, "How the AI fires"). With
/// `floor`, a building's guns hold their fire on a target below the lowest its turret's sight
/// can look.
#[allow(clippy::too_many_arguments)]
fn aim_and_fire(
    robot: &mut Robot,
    t: usize,
    type_word: u32,
    takt: &Takt,
    seen: &[Sighting],
    battle: &Battle,
    ground: &Ground,
    animal: bool,
    floor: bool,
) {
    let now = robot.time_ms;
    let at = robot.walker.body.position;
    let found = takt.target.and_then(|id| seen.iter().find(|(_, s, _)| s.id == id));
    let target = found.and_then(|(i, _, _)| *i);
    let building = found.is_some_and(|(_, s, _)| s.building);
    if target != robot.fire_target {
        robot.fire_target = target;
        for g in &mut robot.guns {
            g.relink(target);
        }
    }
    let Some(victim) = target.and_then(|i| battle.combat.target(i)) else {
        robot.target_point = None;
        robot.rig.traced = None;
        return;
    };
    let (point, reach) = (victim.centre, victim.radius);
    robot.target_point = Some(point);
    robot.aim_at(point);
    // DEPARTURE: docs/29-weapons.md#how-the-ai-fires--read -- no floor on a building's fire is
    // read. A building's turret keeps tracing a target that stands lower than its pitch channel
    // lets its sight look (−15° on the Small Bunker's), but its guns hold their fire, so a unit
    // that comes within about 40 m of a bunker, or stands inside it, is not shot at by it.
    if floor && let (Some(lowest), Some((origin, _))) = (robot.lowest_sight(), robot.sight()) {
        let to = point - origin;
        if to.z.atan2(to.truncate().length()) < lowest {
            return;
        }
    }
    let settled = [robot.rig.yaw, robot.rig.pitch].iter().enumerate().all(|(axis, c)| {
        c.is_none_or(|c| (robot.rig.values[c] - robot.rig.target(axis)).abs() < AIM_SETTLED)
    });
    let distance = at.distance(point);
    // The bar by what the unit is and how fast it may go now (`0x10024e7a`–`0x10024ec5`): the
    // live forward top speed is the limited one, which damage lowers.
    let bar = if takt.fire_freely {
        0.0
    } else {
        fire_bar(robot.flyer, robot.walker.limits.top_speed[1], type_word)
    };
    for g in 0..robot.guns.len() {
        if now < robot.next_shot_ms[g] {
            continue;
        }
        let gun = &robot.guns[g];
        // Heavy rounds are held back: a gun whose round does more than 10,000 fires only at a
        // building, an id of class 3 (`Behavior.dll:0x10024d02`–`0x10024d3e`), in free fire too.
        if gun.round_damage > parkan_sim::guns::HEAVY_ROUND && !building {
            continue;
        }
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
            let ramp = if animal { 0.0 } else { SCORE_RAMP };
            distance_score(distance, point.z - at.z, gun.round_speed, gun.round_flags, ramp) * turret * own
        };
        // A score must pass the bar, not reach it (`0x10024f13`–`0x10024f1e`).
        if score <= bar {
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

/// A target's nodes where `robot` stands this tick, and its centre amid their level-0
/// spheres.
fn pose_target(robot: &Robot, target: &mut Target) {
    let place = robot.placement();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (p, part) in target.parts.iter_mut().enumerate() {
        for n in 0..part.nodes.len() {
            part.nodes[n] = place.compose(&robot.part_pose(p, n));
            let Some(slot) = part.mesh.slots.get(usize::from(part.mesh.nodes[n].slot_index[0])) else {
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
    let aim = place.apply(robot.bound.0.to_array().map(f64::from));
    target.aim = Vec3::new(aim[0] as f32, aim[1] as f32, aim[2] as f32);
    target.agent_sphere = (robot.collision_centre(), robot.collision.1);
}

/// The hero as the battle strikes it: each of its parts, posed by [`pose_target`], its sphere
/// over their level-0 spheres as a placed unit's is. Its lives stay the hero's and are lent to
/// the target for the battle's frame ([`Play::lend_hero_lives`]).
fn hero_target(hero: &Hero, shield: Option<parkan_sim::shield::Shield>) -> Target {
    let parts: Vec<Part> = hero
        .parts
        .iter()
        .map(|part| Part {
            mesh: Rc::new(part.mesh.mesh.clone()),
            nodes: vec![parkan_formats::pose::IDENTITY; part.mesh.mesh.nodes.len()],
            scale: 1.0,
            life: None,
            portals: Rc::default(),
            host: usize::try_from(part.host).ok().zip(usize::try_from(part.node).ok()),
        })
        .collect();
    let mut target = Target {
        parts,
        centre: Vec3::ZERO,
        radius: 0.0,
        alive: true,
        position: hero.walker.body.position,
        aim: hero.walker.body.position,
        shield,
        agent_sphere: (hero.walker.body.position, 0.0),
    };
    pose_target(&hero.robot, &mut target);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for part in &target.parts {
        for (n, node) in part.nodes.iter().enumerate() {
            let Some(slot) = part.mesh.slots.get(usize::from(part.mesh.nodes[n].slot_index[0])) else {
                continue;
            };
            let [cx, cy, cz, r] = slot.sphere;
            let c = node.apply([cx, cy, cz].map(f64::from));
            let c = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
            lo = lo.min(c - Vec3::splat(r));
            hi = hi.max(c + Vec3::splat(r));
        }
    }
    if lo.x <= hi.x {
        target.radius = (hi - lo).length() / 2.0;
    }
    target
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
            // STAND-IN: docs/34-progression.md#the-medusas--read-and-measured -- how an animal's
            // gun aims is not read: its turret's pitch channel has no point, so it has no sight,
            // and an AI unit with none fires straight at the point its fire control traces.
            let traced = owner.is_some().then_some(robot.target_point).flatten();
            let falls = robot.guns.get(g).is_some_and(|gun| gun.falls);
            // The sight's convergence is for a mode-0 round: a lobbed one leaves along its
            // barrel, which its mount has raised (`0x1002a34c`).
            let aim = match robot.sight() {
                _ if falls => None,
                Some((o, s)) => battle.combat.aim_point(ground, owner, o, s),
                None => traced,
            };
            let lobbed = if owner.is_some() { robot.lobbed_launch(g, muzzle) } else { None };
            Some(Launch {
                kind,
                owner,
                muzzle,
                direction: lobbed.unwrap_or_else(|| aim.map_or(barrel, |p| p - muzzle)),
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
/// The **wear base** the material byte is ORed with (`AniMesh.dll:0x100135c8`) is the
/// mesh's wear list in the material manager, shifted into the high 16 bits
/// (`0x1000a726`), and the manager splits a material id back into that list and an index
/// into it (`World3D.dll:0x10003697`). So the byte alone indexes the model's own wear,
/// which is what this does (docs/11, "A node's wear base").
pub fn struck_wear<'a>(part: &Part, wear: &'a [String], p0: Vec3, p1: Vec3) -> Option<&'a str> {
    let strike = part.segment(p0, p1, ROUND_SKIPS_FACE)?;
    let slot = part.mesh.slots.get(usize::from(part.slot(strike.node?)?))?;
    let first = usize::from(slot.first_batch);
    let batches = part.mesh.batches.get(first..first + usize::from(slot.batch_count))?;
    let batch = batches.iter().find(|b| {
        let (from, count) = b.triangles();
        (from..from + count).contains(&strike.triangle.unwrap_or(usize::MAX))
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
/// **The push is shared by mass squared** (`Control.dll:0x1001e0db`): with a contact record on
/// both sides, the mover takes P x m_obstacle^2 / (m_obstacle^2 + m_mover^2) of what the pair
/// makes, so the lighter side gives way. A building, a tree and a stone have no record, carry
/// no mass here, and the mover takes the whole push against them (`0x1001e05f`).
///
/// The game works one push out per pair and moves both sides by opposite shares of it; the
/// engine runs each unit as the mover in its own turn, so each side takes its own share of
/// the push the other's faces make.
///
/// A mover whose collision flags carry 8 is pushed by a building's floors too (`keeps_floors`,
/// [`parkan_sim::machine::Machine::keeps_floors`]).
fn collision_push(
    solids: &[Solid],
    from: Vec3,
    to: Vec3,
    radius: f32,
    mover: Option<usize>,
    mover_mass: f32,
    keeps_floors: bool,
) -> Vec3 {
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
        total += solid::push(from, end, radius, obstacle, keeps_floors) * share(obstacle.mass, mover_mass);
    }
    total
}

/// What of a pair's push the mover takes: the obstacle's mass squared over the two squared,
/// and the whole of it where either side has no mass, which is where either has no contact
/// record (`Control.dll:0x1001e05f`, `0x1001e0db`).
fn share(obstacle_mass: f32, mover_mass: f32) -> f32 {
    if obstacle_mass <= 0.0 || mover_mass <= 0.0 {
        return 1.0;
    }
    let (a, b) = (obstacle_mass * obstacle_mass, mover_mass * mover_mass);
    a / (a + b)
}

/// The ground each tree and stone stands on, as the areal map cuts it out of the walkable
/// areals (`ArealMap.dll:0x1000f660`): the four lower corners of its mesh's box, in the box's
/// order, placed and scaled as the object is.
fn scenery_footprints(mission: &Mission, battle: &Battle) -> Vec<[Vec2; 4]> {
    (0..battle.objects.len())
        .filter(|&t| matches!(battle.placed_kinds.get(t), Some(&(KIND_VEGETATION | KIND_ROCK))))
        .filter_map(|t| {
            let object = &mission.objects[battle.objects[t]];
            let corners = battle.meshes.get(t)?.first()?.mesh.corners?;
            let scale = object.placed_scale();
            let (sin, cos) = object.rotation.sin_cos();
            let at = Vec2::new(object.position[0], object.position[1]);
            Some(std::array::from_fn(|k| {
                let (x, y) = (corners[k][0] * scale, corners[k][1] * scale);
                at + Vec2::new(cos * x - sin * y, sin * x + cos * y)
            }))
        })
        .collect()
}

impl Play {
    /// The mission's hero armed, its map's ground, every other object a target, and the
    /// effects they can play loaded.
    pub fn load(game: &Path, mission: &Mission) -> Result<Option<Play>> {
        let mut assembly = Assembly::new(game)?;
        let Some(mut hero) = Hero::load(&mut assembly, mission)? else { return Ok(None) };
        let dir = terrain::map_dir(game, &mission.map_path)?;
        let land = landmesh::load(&gamedir::resolve(&dir, "Land.msh").context("the map has no Land.msh")?)?;
        let mut graph = match gamedir::resolve(&dir, "Land.map").map(|p| arealmap::load(&p)) {
            Some(Ok(map)) => Some(Graph::new(map)),
            Some(Err(e)) => return Err(e.into()),
            None => None,
        };
        let ratio = settings::level_ratio(game);
        let mut battle = Battle::load(&mut assembly, mission, Some(hero.object), ratio)?;
        if let Some(graph) = &mut graph {
            graph.carve(&scenery_footprints(mission, &battle));
        }
        hero.arm(&mut battle, &mut assembly);
        // The player's own hero is never given the level ratio (docs/26).
        let hero_shield =
            crate::shields::load(&mut assembly, KIND_UNIT, &mission.objects[hero.object].path, 1.0);
        battle.combat.hero = Some(hero_target(&hero, hero_shield));
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
                named_clan: object.clan_id(),
                type_word: object.property("Type").map_or(0, |p| number(p.value)) as u32,
                logical_id: object.logical_id,
                kind: object.kind,
                announced: false,
                designation,
            });
        }
        // Every target's faces, for the ground a building gives and the collision pass.
        let mut ground = Ground::new(land);
        ground.cuts = crate::basement::footings(&mut assembly, mission, &ground.land)
            .into_iter()
            .map(crate::basement::cut)
            .collect();
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
        // The hero's own faces carry its mass, so a unit that meets it shares the push by the
        // squares of the two (docs/24, "Collision between objects"). Every other unit's is set
        // as its solid is refreshed each frame.
        if let Some(t) = battle.objects.iter().position(|&o| o == hero.object)
            && let Some(s) = ground.solids.get_mut(t)
        {
            s.mass = hero.heft.mass(|p, n| node_share(hero.lives.get(p).and_then(Option::as_ref), n));
        }
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
            graph,
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
            clans: mission.clans.clone(),
            relations: mission::relation_words(&mission.clans),
            attitudes: parkan_sim::relations::Relations::new(&mission::relation_words(&mission.clans)),
            shield_flashes: HashMap::new(),
            player_clan,
            hero_id,
            targets: TargetList::default(),
            progression: None,
            says: Vec::new(),
            paused: false,
            fallen: None,
            training: false,
            names,
            selector: Selector::default(),
            voice_pick: VoicePick::default(),
            capture_standby: false,
            building_fire_floor: false,
            god: false,
            flights: Vec::new(),
            spent: Vec::new(),
            anchors: HashMap::new(),
            deaths: Vec::new(),
            pending_events: Vec::new(),
            exploding: Vec::new(),
            deleted: vec![false; target_count],
            robots,
            emplacements,
            buildings,
            modes: vec![Mode::OnFoot],
            selected: Vec::new(),
            factories,
            stores: HashMap::new(),
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
            reserved: HashMap::new(),
            walk_seed: 0x2545_f491,
            flight_seed: 0x9e37_79b9,
            grazing: std::cell::RefCell::default(),
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
        play.pose_hero();
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
                    self.fx.start_ambient(Owner::Building(b.target, e.id), &e.name, frame, 1.0, 0.0, None);
                }
            }
            for &(id, mode) in &b.starts {
                self.fx.restart(Owner::Building(b.target, id), 0.0, Some(mode));
            }
        }
    }

    /// The material track target `t`'s meshes draw on, which picks a cell of an insignia
    /// sheet (`B_LBL_01`, docs/07, "Who picks an object mesh's material track").
    ///
    /// STAND-IN: docs/07-objects.md#who-picks-an-object-meshs-material-track--read -- what writes
    /// the control system's `+0x554` (slot 16) is not found. Part 6.5 of the let's play shows C03
    /// M02's Enemy 1 Medium Mine wearing track 1 and the player's Small Bunker track 0, so a
    /// building draws on its owner clan's index, the sign a single-player game gives clan *i*.
    /// A unit keeps track 0, its own not seen, and a capture changes nothing, which is not seen
    /// either.
    pub fn insignia(&self, t: usize) -> usize {
        let clan = self.units.get(t).filter(|u| u.kind == KIND_BUILDING).and_then(|u| u.clan);
        clan.and_then(|c| usize::try_from(c).ok()).unwrap_or(0)
    }

    /// Whether target `t` is drawn node by node -- each node at its own pose and its own damage
    /// stage -- rather than as one model built whole at its placement.
    ///
    /// A target that takes damage is (docs/26, "What a damaged node, a destroyed part and a
    /// dead unit draw"), and so is a building whose own items play its nodes. The second is
    /// what the energy bridge needs: it takes no damage, and drawn whole its three turning
    /// arms stand still while the rays hung on them sweep round without them. *Measured*: of
    /// the buildings the 29 missions place, the ones this brings in and life does not are the
    /// two bridge halves, the generators, the small Main Teleport and a ruin.
    pub fn drawn_by_node(&self, t: usize) -> bool {
        self.battle.combat.targets.get(t).is_some_and(|x| x.parts.iter().any(|p| p.life.is_some()))
            || self.buildings.iter().any(|b| b.target == t && b.plays_nodes())
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

    /// Each fitted gun's effects (docs/29, "What a shot plays"): its load group's, created and
    /// started the first time the gun is seen, follow their control points, and a time-mode-4
    /// one reads the value of the barrel channel that plays its node, so a cannon's flash and
    /// its report run through each stroke; as a barrel starts its stroke the gun's shot group
    /// runs. A deleted unit's go with it (`World3D.dll!KillGameObject`).
    fn follow_gun_effects(&mut self, now: f64) {
        let Play { robots, emplacements, fx, deleted, battle, .. } = self;
        for (t, robot) in robots.iter_mut().chain(emplacements.iter_mut()) {
            let t = *t;
            if deleted.get(t).copied().unwrap_or(false) || battle.combat.targets.get(t).is_none() {
                if robot.gun_parts.iter().flatten().any(|f| f.running) {
                    fx.retain(|o, _| !matches!(o, Owner::Gun(x, _, _) | Owner::Shot(x, _) if *x == t));
                    robot.gun_parts.iter_mut().flatten().for_each(|f| f.running = false);
                }
                continue;
            }
            for g in 0..robot.guns.len() {
                let strokes = std::mem::take(&mut robot.guns[g].stroked).len();
                let Some(Some(fitted)) = robot.gun_parts.get(g) else { continue };
                // An effect whose points the gun's part does not carry is not made.
                let frame = |e: &crate::robot::GunEffect| {
                    let [a, b, c] = e.points.map(|i| robot.fitted_point(g, i));
                    Some(Frame::from_points([a?, b?, c?]))
                };
                if !fitted.running {
                    for e in &fitted.effects {
                        if let Some(frame) = frame(e) {
                            fx.start(Owner::Gun(t, g, e.id), &e.name, frame, 1.0, now, None);
                        }
                    }
                    for &(id, mode) in &fitted.starts {
                        fx.restart(Owner::Gun(t, g, id), now, mode);
                    }
                }
                for e in &fitted.effects {
                    let value = robot.guns[g]
                        .barrels
                        .iter()
                        .find(|b| fitted.channels.get(b.channel).is_some_and(|c| c.node == e.node))
                        .map_or(0.0, |b| b.value(robot.time_ms));
                    let Some(frame) = frame(e) else { continue };
                    for instance in fx.owned(Owner::Gun(t, g, e.id)) {
                        instance.frame = frame;
                        instance.value = value;
                    }
                }
                for _ in 0..strokes {
                    for act in &fitted.shot {
                        match act {
                            crate::robot::ShotAct::Create(e) => {
                                if let Some(frame) = frame(e) {
                                    fx.start(Owner::Shot(t, g), &e.name, frame, 1.0, now, None);
                                }
                            }
                            &crate::robot::ShotAct::Start((id, mode)) => {
                                fx.restart(Owner::Gun(t, g, id), now, mode)
                            }
                        }
                    }
                }
                if let Some(Some(fitted)) = robot.gun_parts.get_mut(g) {
                    fitted.running = true;
                }
            }
        }
        for g in &mut self.hero.guns {
            g.stroked.clear();
        }
    }

    /// Load the mission's progression from `mission_dir`: its player clan's script, its
    /// messages and objectives, and the designs its `mission.cfg` prebuilds.
    pub fn load_progression(&mut self, game: &Path, mission_dir: &Path, mission: &Mission) -> Result<()> {
        self.training = mission_dir
            .parent()
            .and_then(|c| c.file_name())
            .is_some_and(|c| c.to_string_lossy().eq_ignore_ascii_case(TRAINING_CAMPAIGN));
        self.progression = Some(Progression::load(game, mission_dir, mission, self.hero.object)?);
        crate::factory::prebuild(self, game, mission_dir).context("the prebuilt designs")?;
        // What the clans' `Init` handlers ordered: Mission 03's enemy patrol shut down.
        self.deliver_orders();
        self.run_console();
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
        // The battle strikes the hero where it now stands.
        self.pose_hero();
    }

    /// Whether clan `other` is hostile to the player's: another clan, not nature's, toward
    /// which the player's clan's relation word is 0 (`iron3d.dll:0x10039440`).
    pub fn hostile(&self, other: Option<i64>) -> bool {
        self.hostile_to(Some(self.player_clan), other)
    }

    /// Clan `us`'s relation word towards clan `other`, as the loader filed it.
    pub fn word(&self, us: i64, other: i64) -> Option<u32> {
        let (us, other) = (usize::try_from(us).ok()?, usize::try_from(other).ok()?);
        self.relations.get(us)?.get(other).copied()
    }

    /// Whether clan `other` is hostile to clan `us`, by `us`'s relation word toward it.
    pub fn hostile_to(&self, us: Option<i64>, other: Option<i64>) -> bool {
        let Some(us) = us else { return false };
        let Some(other) = other.filter(|&c| c != us) else { return false };
        let Some(them) = self.clan(other) else { return false };
        them.kind != CLAN_NATURE && self.word(us, other) == Some(RELATION_HOSTILE)
    }

    /// Whether a unit of clan `us` takes clan `other`'s objects as hostile, the behaviour's
    /// own test (`Behavior.dll:0x1000d460`, docs/31, "Where a search looks"): never its own
    /// clan; nothing for a neutral clan's unit and everything for a nature clan's; otherwise
    /// the relation word.
    pub fn behaviour_hostile(&self, us: Option<i64>, other: Option<i64>) -> bool {
        let Some(us) = us else { return false };
        let Some(other) = other.filter(|&c| c != us) else { return false };
        match self.clan(us).map(|c| c.kind) {
            Some(CLAN_NEUTRAL) | None => false,
            Some(CLAN_NATURE) => true,
            _ => self.word(us, other) == Some(RELATION_HOSTILE),
        }
    }

    /// Whether clan `other` is an ally of clan `us`, by `us`'s relation word toward it, which a
    /// dock asks before it charges an occupant (`Behavior.dll:0x10019318`).
    pub fn allied_to(&self, us: Option<i64>, other: Option<i64>) -> bool {
        let Some(us) = us else { return false };
        let Some(other) = other.filter(|&c| c != us) else { return false };
        self.word(us, other) == Some(RELATION_ALLIED)
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
        match them.kind {
            CLAN_NATURE => MARK_NATURE,
            CLAN_NEUTRAL => MARK_NEUTRAL_CLAN,
            _ => match self.word(clan, self.player_clan) {
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
        let current = self.targets.current;
        self.gunner_mut().relink(current);
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
        let radar = match driven
            .and_then(|t| self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(rt, _)| *rt == t))
        {
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
        // The gate measures to the target's node sphere's centre (`Control.dll:0x1002a8c0`).
        let point = self
            .targets
            .current
            .filter(|&t| world.get(t).is_some())
            .and_then(|t| self.battle.combat.target(t))
            .map(|x| x.aim);
        self.gunner_mut().target_point = point;
    }

    /// One of `iron3d.dll`'s commands (`0x10071cd0`), the player looking from `view`.
    /// Returns whether it was one this answers.
    ///
    /// STAND-IN: docs/25-sensors.md#the-players-target--read-and-measured -- the unit
    /// record's `+0x98` and `+0x94`, where the right button's ray starts and the margin
    /// its pick keeps from the unit, are not read: both are 0.
    pub fn command(&mut self, command: &str, view: &View) -> bool {
        let world = self.contacts();
        // Every pick is on the unit the player drives, its place and its sensor range
        // (docs/25, "Each takt, on the unit the player drives"), so a bot taken by telepresence
        // or boarded picks with its own list, not with the hero's where it stands.
        let unit = self.driven().walker.body.position;
        let changed = match command {
            CMD_JAMES_SELECT_TARGET => self.targets.select_next(),
            CMD_JAMES_SELECT_ENEMY => self.targets.select_nearest(unit, &world, |c| c.hostile),
            CMD_JAMES_SELECT_FRIEND => self.targets.select_nearest(unit, &world, |c| c.friend),
            CMD_JAMES_AIM_TARGET => {
                let range = self.driven().radar.range;
                self.targets.aim(unit, view.eye, view.look, 0.0, 0.0, range, &world, |c, r| {
                    on_screen(view.view_proj, c, r)
                })
            }
            CMD_ENTER_STATE => {
                self.enter_or_board();
                false
            }
            // `0x10075fc0`, only with mode 1 or 2 at the front (`0x1007263f`-`0x10072648`): the
            // driven unit's record steps its level 0, 1, 2 and round, and the unit is taken again
            // at the new level (`0x10074ff0` with 1). On foot nothing steps.
            parkan_formats::controls::CMD_JAMES_AUTO_DRIVER => {
                if matches!(self.mode(), Mode::Driving(_))
                    && let Some(t) = self.driving.as_ref().map(|d| d.target)
                    && let Some(r) = self.robots.iter().position(|(rt, _)| *rt == t)
                {
                    let robot = &mut self.robots[r].1;
                    robot.auto_driver = (robot.auto_driver + 1) % 3;
                    if robot.auto_driver != 0 {
                        robot.walker.body.command = [0.0; 3];
                    }
                    self.take_as_level(r);
                }
                false
            }
            // In view states 1 and 6, the driven unit's camera and a tower's in its manual
            // control: the outer camera's view lets Z be (`0x10072428`, docs/30, "The zoom").
            CMD_JAMES_ZOOM_MODE => {
                if !self.outer.on()
                    && matches!(self.mode(), Mode::OnFoot | Mode::Driving(_) | Mode::Manual(_))
                {
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

    /// The wingmen (`iron3d.dll:0x10091f20`, docs/31, "Who can be a wingman"): the robots of
    /// the player's clan on the driven unit's radar, in the radar's order, as indices into
    /// `robots`.
    pub fn wingmen(&self) -> Vec<usize> {
        self.targets
            .listed
            .iter()
            .filter(|&&t| self.units.get(t).is_some_and(|u| u.clan == Some(self.player_clan)))
            .filter_map(|&t| self.robots.iter().position(|(rt, _)| *rt == t))
            .filter(|&r| self.battle.combat.targets.get(self.robots[r].0).is_some_and(|t| t.alive))
            .collect()
    }

    /// The wingmen chosen in the selector, by target. Its choice is by place in the wingman
    /// list (`+0xc`, docs/31, "The selector's three states"), which the list turns into units.
    pub fn chosen_wingmen(&self) -> Vec<usize> {
        let wingmen = self.wingmen();
        self.selector
            .chosen
            .iter()
            .filter_map(|&i| wingmen.get(i))
            .filter_map(|&r| self.robots.get(r).map(|(t, _)| *t))
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
                    && let Some(order) = orders::order_for(row, self.driven_id(), target)
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
    /// **A building answers the same way too.** Its agent forwards every query but `0x15`
    /// to the `CBuilding` that aggregates it (`AniMesh.dll:0x10001320`), and `CBuilding`
    /// answers only ids 0, 6, `0x11`, `0x17` and `0x18` itself and hands the rest —
    /// `0xd`, the material manager, among them — straight back to the agent's own table
    /// (`Terrain.dll:0x10057c20`, its default case at `0x10057cf6`;
    /// `AniMesh.dll:0x100012d0`).
    fn struck_class(&self, target: usize, part: usize, round: &Round, point: Vec3) -> Option<u8> {
        self.class_struck_by(target, part, round.previous, point + round.forward * 0.01)
    }

    /// [`Play::struck_class`] for a plain segment: the material class a ray from `p0` to
    /// `p1` strikes on part `part` of `target`, whatever kind of thing that target is.
    pub fn class_struck_by(&self, target: usize, part: usize, p0: Vec3, p1: Vec3) -> Option<u8> {
        let p = self.battle.combat.target(target)?.parts.get(part)?;
        let wear = match self.battle.wears.get(target) {
            Some(wears) => wears.get(part)?,
            None => &self.hero.parts.get(part)?.mesh.wear.materials,
        };
        let name = struck_wear(p, wear, p0, p1)?;
        self.materials.get(name).map(|m| m.surface)
    }

    /// One tick: the hero, then every round that left one of its barrels, then the
    /// battle's frame, then the effects.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2]) -> Vec<Event> {
        // The game menu pauses the game (`0x100656c0` → `0x1005f620` with 1: the game's `+0xe8`
        // and `World3D`'s game time stopped): the game frame is skipped whole (`0x1005ea7f`).
        if self.mode() == Mode::GameMenu {
            return Vec::new();
        }
        self.sync_sensitivity();
        self.update_targets();
        self.refresh_present();
        self.tick_robots(dt_ms, mouse);
        // The game frame tests the bot the hero boarded every frame, and once the component
        // test refuses it -- gone, or its turret's body shot to nothing though the bot flies on
        // -- rolls the stack back to mode 0 (`0x1005eab3`–`0x1005eacf`, docs/39, "When the
        // driven bot is lost"), through any command view or telepresence standing on it: the
        // hero is put out beside the bot, untested.
        if let Some(t) = self.aboard()
            && !self.component_sound(t)
        {
            self.roll_back_to_foot();
        }
        // STAND-IN: docs/40-command-mode.md#not-established -- nothing read pops mode 3 when an
        // HQ the hero did not board, one reached from a bunker's view, is lost: the view rolls
        // back to the view below it.
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
            // STAND-IN: docs/27-ownership.md#what-the-modes-show--read -- nothing read pops mode
            // 6 when its tower is destroyed; the view goes back to the hero.
            if d.telepresence || matches!(self.mode(), Mode::Manual(_)) {
                self.roll_back();
            } else {
                self.leave();
            }
        }
        let shots = if self.hero.dead() || self.hero_away() {
            self.hero.time_ms += dt_ms;
            Vec::new()
        } else {
            // While the player drives a unit or a building's guns, the mouse is theirs and the
            // hero stands with its keys let go (`enter_manual`, the takeover).
            let mouse = if self.driving.is_some() { [0.0; 2] } else { mouse };
            let from = self.hero.collision_centre();
            let shots = self.hero.tick(dt_ms, mouse, &self.ground);
            let lives = std::mem::take(&mut self.hero.lives);
            self.hero.relimit(|p, n| node_share(lives.get(p).and_then(Option::as_ref), n));
            self.hero.lives = lives;
            self.collide(from);
            self.footsteps();
            shots
        };
        self.pose_hero();
        let now = self.hero.time_ms;
        if !self.paused {
            self.takt_relations(now);
            self.tick_buildings(now);
            self.tick_places(now);
            self.tick_economy(now, (dt_ms / 1000.0) as f32);
            self.tick_factories(dt_ms);
            self.tick_research(dt_ms);
            self.check_research();
        }
        let mut events = std::mem::take(&mut self.pending_events);
        events.extend(self.ground_damage(now));
        events.extend(self.tick_explosions(now));
        if !self.paused {
            events.extend(self.tick_construction(now));
        }
        let launches = launches(&self.hero.robot, None, &shots, &self.battle, &self.ground);
        self.launch(launches, now);
        self.tick_power();
        self.power_shields();
        self.lend_hero_lives(true);
        events.extend(self.battle.combat.tick((dt_ms / 1000.0) as f32, &self.ground));
        events.extend(self.battle.combat.takt_lives(now));
        self.lend_hero_lives(false);
        let hero_index = self.battle.combat.hero_index();
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
                Event::Staged { target, .. }
                | Event::Hidden { target, .. }
                | Event::Restored { target, .. } => self.rebuild_solid(target),
                Event::Killed { target } if target == hero_index => self.hero_lost(),
                Event::Hurt { target, owner } => self.hurt(target, owner),
                Event::ShieldHit { target, point } => self.shield_flash(target, point, now),
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
            // A tree's or a stone's load group goes with it, as a building's does
            // (`remove_building`).
            if matches!(self.units.get(target).map(|u| u.kind), Some(KIND_VEGETATION | KIND_ROCK)) {
                self.building_effects.retain(|(b, _)| b.target != target);
                self.fx.retain(|o, _| !matches!(o, Owner::Building(t, _) if *t == target));
            }
            self.killed.push(self.battle.objects[target]);
            // No object answers a deleted unit's id any more: function 52 gives `ERROR`.
            if let (Some(unit), Some(p)) = (self.units.get(target), self.progression.as_mut()) {
                p.progress.deleted(unit.logical_id);
            }
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
        self.follow_gun_effects(now);
        self.follow_shield_flashes();
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
                && let Some(unit) = self.units.get(*target)
                && let Some(p) = self.progression.as_mut()
            {
                p.progress.destroyed(unit.logical_id);
            }
        }
        // A won or lost mission plays on under its panel (docs/34, "After the outcome"). No
        // `Mission` handler or clan takt runs in the briefing's state 5.
        if !self.paused {
            self.progress();
            self.deliver_orders();
            self.run_console();
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
                materials.get(name).map(|m| (m.surface, m.damage_rate))
            });
        let mut solid = solid;
        if let Some(b) = self.buildings.iter().find(|b| b.target == t) {
            for node in solid.nodes.iter_mut().filter(|n| n.part == b.part) {
                node.open = b.open_door_node(node.node);
            }
        }
        if let Some(s) = self.ground.solids.get_mut(t) {
            *s = Solid { present: s.present, mass: s.mass, ..solid };
        }
    }

    /// Target `t`'s machine: a robot's, or the one a building that carries guns keeps for its
    /// turret ([`Play::emplacements`]).
    pub fn machine(&self, t: usize) -> Option<&Robot> {
        self.robots.iter().chain(&self.emplacements).find(|(rt, _)| *rt == t).map(|(_, r)| r)
    }

    /// Target `t`'s machine, to change.
    pub fn machine_mut(&mut self, t: usize) -> Option<&mut Robot> {
        self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(rt, _)| *rt == t).map(|(_, r)| r)
    }

    /// The mode at the front of the interface's stack.
    pub fn mode(&self) -> Mode {
        self.modes.last().copied().unwrap_or(Mode::OnFoot)
    }

    /// The mode the world is viewed in: the front, or under the game menu the mode below it,
    /// whose view state the menu keeps (`0x100645e0`–`0x10064680` set none but 2 for a
    /// command view, which it was already).
    pub fn view_mode(&self) -> Mode {
        self.modes.iter().rev().copied().find(|m| *m != Mode::GameMenu).unwrap_or(Mode::OnFoot)
    }

    /// `CMD_GAME_MENU` (748, F3, `0x10072359`), the commander column's Game menu button
    /// (`0x10084674`) and Esc on foot: while the mission is played, and not while a building is
    /// being placed (cursor state 8), mode 7 is pushed, or with the menu up the stack is rolled
    /// back, closing it. Into 7 from on foot or a driven unit (`0x100645e0`, `0x10064620`) the
    /// unit's manual controller is switched off and the keyboard cleared; from a command view
    /// (`0x10064650`) only the menu shows. Showing it pauses the game.
    pub fn game_menu(&mut self) -> bool {
        if self.progression.as_ref().is_some_and(|p| p.progress.outcome.is_some())
            || self.commander.ghost.is_some()
        {
            return false;
        }
        if self.mode() == Mode::GameMenu {
            return self.roll_back();
        }
        if matches!(self.mode(), Mode::OnFoot | Mode::Driving(_) | Mode::Manual(_)) {
            self.release_keys();
        }
        self.modes.push(Mode::GameMenu);
        true
    }

    /// Roll the stack back one mode (`0x10062ff0`): a building's screen gives the hero back
    /// to the player. The bottom mode stays.
    pub fn roll_back(&mut self) -> bool {
        if self.modes.len() <= 1 {
            return false;
        }
        match self.mode() {
            // Mode 7 → below (`0x100646b0`, `0x10064700`, `0x10064730`, `0x10064770`): the menu
            // hidden and the game going on; the driven unit's controller back, and the keyboard
            // cleared but into a command view, which clears the left button's press instead.
            Mode::GameMenu => {
                self.modes.pop();
                self.release_keys();
                return true;
            }
            Mode::Driving(_) if self.driving.as_ref().is_some_and(|d| d.telepresence) => {
                return self.end_telepresence();
            }
            Mode::Driving(_) => return self.leave(),
            // Mode 6 → 0 (`0x100640ab`): the tower's guns back to its AI (slot 9, mask `0x20`,
            // 1, and message (6, 7, 0)) and the hero the player's again, where it stood.
            Mode::Manual(_) => {
                self.let_go();
                self.modes.pop();
                self.hero.release_keys();
                return true;
            }
            // Mode 3 → 1 (`0x10063ad0`): the HQ selected and taken back at auto-driver level 0
            // with the camera let go; or 3 → 4 and 3 → 3, back to the command view below.
            Mode::HqCommand(t) => {
                self.modes.pop();
                self.command.leave();
                self.command.release();
                match self.mode() {
                    Mode::Driving(below) if below == t => {
                        self.select_unit_alone(t);
                        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
                            robot.auto_driver = 0;
                        }
                        self.take(t, false);
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

    /// Whether the hero is out of the world: aboard a bot. Only boarding detaches its object
    /// (mode 0 → 1, `0x100637ed`, docs/39, "Boarding"). A unit driven from a command view (mode
    /// 2) or a building's guns (mode 6, docs/27, "What the modes show") leave it standing in
    /// the room where it was, where it can still be struck.
    pub fn hero_away(&self) -> bool {
        self.aboard().is_some()
    }

    /// Where the hero's own behaviour reports it to the routes (docs/39, "What becomes of the
    /// hero"). On foot, or driving a unit from a bunker's view, it is where the hero stands.
    /// Aboard, it is the place the game frame gives the hero's object every frame
    /// (`iron3d.dll:0x1005ead6`–`0x1005eb39`): the boarded bot's world matrix with its y lowered
    /// by the bot record's `+0x94`, its node sphere's radius, whatever command view or
    /// telepresence stands on the bot.
    pub fn hero_place(&self) -> Vec3 {
        match self.aboard().and_then(|t| self.robots.iter().find(|(rt, _)| *rt == t)) {
            Some((_, bot)) => bot.walker.body.position - Vec3::new(0.0, bot.bound.1, 0.0),
            None => self.hero.walker.body.position,
        }
    }

    /// Whether unit `t` passes the component test (`iron3d.dll:0x10076d30`, docs/39,
    /// "Boarding"): it is still there, and its first class-1 component's node, a fitted
    /// turret's body, has life left.
    fn component_sound(&self, t: usize) -> bool {
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        self.battle.combat.targets.get(t).is_some_and(|x| x.alive && turret_alive(robot, x))
    }

    /// The view's own unit (`+0xaec`): the one the player drives, else the one the hero rides
    /// in, whose place and radar the target list takes.
    pub fn driven_target(&self) -> Option<usize> {
        self.driving.as_ref().map(|d| d.target).or_else(|| self.aboard())
    }

    /// The logical id of the view's own unit, its record's `+0x34`: the bot the player drives,
    /// else the hero. It is the id Follow me names (docs/31, "The orders").
    pub fn driven_id(&self) -> i32 {
        self.driven_target().and_then(|t| self.units.get(t)).map_or(self.hero_id, |u| u.logical_id)
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
    /// its command cleared. Its record keeps its level; the Wizard's words go to the AI, the
    /// sensors' among them (`0x10075131`).
    fn let_go(&mut self) {
        if let Some(mut d) = self.driving.take()
            && let Some(robot) = self.machine_mut(d.target)
        {
            let reach = Reach::of(robot.auto_driver, robot.sensors_taken);
            crate::hero::drive_input(robot, &mut d.pilot, true, reach);
            robot.walker.body.command = [0.0; 3];
            robot.sensors_taken = false;
            let_go(robot);
            // Back to its fire control, which picks its own target; the hero's guns take the
            // player's again.
            robot.relink(None);
            robot.target_point = None;
            let current = self.targets.current;
            self.hero.relink(current);
        }
    }

    /// Unit `t` taken by the player (`0x10074ff0` with 1) at the auto-driver level its record
    /// holds ([`Robot::auto_driver`], `0x10075027`): driven by its own input table's pilot, as
    /// far as the level reaches ([`Reach`]), with every held key let go and its walk cleared;
    /// its turret lock set at level 0 alone. Its repair system keeps the state it had, which
    /// for a unit its AI ran is what its repair decision last sent. Pushes no mode.
    fn take(&mut self, t: usize, telepresence: bool) -> bool {
        let Some(r) = self.robots.iter().position(|(rt, _)| *rt == t) else { return false };
        let chassis = self.robots[r].1.parts[self.robots[r].1.chassis_part].record.clone();
        let Ok(mut pilot) = crate::hero::Hero::pilot_for(&mut self.assembly, &chassis) else { return false };
        pilot.switches.repair = self.robots[r].1.behaviour.repair;
        self.hero.release_keys();
        self.robots[r].1.wizard.clear();
        self.take_as_level(r);
        self.driving = Some(Driving { target: t, pilot, fire_held: false, telepresence });
        true
    }

    /// Robot `r` taken at the level its record reads (`0x10074ff0` with 1), as a take and each
    /// step of the level do: its turret lock set at level 0 alone (property 179, `0x100750fd`),
    /// the AI's drive dropped where the player moves it, and its guided guns given the player's
    /// target -- or at level 2, where the fight module aims, their own. A take at level 0 or 1
    /// writes the sensors' words the player's; one at 2 leaves them (`0x100750a2`-`0x100750b9`).
    fn take_as_level(&mut self, r: usize) {
        let current = self.targets.current;
        let robot = &mut self.robots[r].1;
        let level = robot.auto_driver.min(2);
        robot.auto_driver = level;
        if level < 2 {
            robot.sensors_taken = true;
        }
        let reach = Reach::of(level, robot.sensors_taken);
        if !reach.ai_moves() {
            robot.walker.drive = None;
        }
        take_over(robot, reach == Reach::Whole);
        robot.fire_target = None;
        robot.relink(if reach.ai_fights() { None } else { current });
    }

    /// The auto-driver level of the unit the player drives, its record's `+0x9c`, which the
    /// indicators show; the hero's is 0, as its bind and the briefing's end leave it
    /// (`0x10074dd2`, `0x1005e7f8`), and nothing steps it on foot.
    pub fn auto_driver(&self) -> u8 {
        self.driving.as_ref().and_then(|d| self.machine(d.target)).map_or(0, |r| r.auto_driver)
    }

    /// What the player's input reaches of the unit it drives ([`Reach`]): the whole hero on
    /// foot.
    pub fn reach(&self) -> Reach {
        self.driving
            .as_ref()
            .and_then(|d| self.machine(d.target))
            .map_or(Reach::Whole, |r| Reach::of(r.auto_driver, r.sensors_taken))
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

    /// Whether building `t` can be taken into its manual control (`0x10033e40`, docs/27, "What
    /// `0x10033e40` refuses on a tower"): it still stands, and its first class-1 item, its
    /// turret, has life left. The commander panel's *Manual* button is grey and inert while it
    /// cannot (docs/41, "The building pages, 5 to 8").
    pub fn manual_open(&self, t: usize) -> bool {
        let Some(robot) = self.machine(t) else { return false };
        self.battle.combat.targets.get(t).is_some_and(|x| x.alive && turret_alive(robot, x))
    }

    /// Mode 6 with the tower or bunker that is target `t` (`iron3d.dll:0x10063fd0` from mode 0,
    /// and the handlers from the command views, 3 and 4; docs/27, "What the modes show"): its
    /// manual control, which a tower's pod opens and the commander panel's *Manual* button on a
    /// bunker's or a tower's row (`0x10086256`, docs/41). It is refused while the building's
    /// first class-1 item, its turret, has no life left (`0x10033e40`), or once it is
    /// destroyed. The outer camera goes off and the hero is handed back where it stands, its
    /// keys let go; the building's guns go to the player: its Wizard's word for the turret and
    /// guns the player's (slot 9, mask `0x20`, 3) and the take's message (6, 7, 1), so it is
    /// driven as a boarded bot is at level 0 (docs/40, "What a bunker's guns do in command
    /// mode"), from the input table the reader falls back to ([`crate::hero::DEFAULT_TABLE`]).
    /// The player's target goes to its guns. Mode 6 goes on top of the mode it came from, so Esc
    /// rolls back to the command view it was taken from. A building already on the stack is
    /// rolled back to.
    ///
    /// STAND-IN: docs/27-ownership.md#what-the-modes-show--read -- mode 6 is entered from
    /// telepresence (2) and the game menu (7) as well; the engine opens it from mode 0 and the
    /// two command views alone, which are where its pod and the panel's button are reached.
    pub fn enter_manual(&mut self, t: usize) -> bool {
        let type_word = self.units.get(t).map_or(0, |u| u.type_word);
        if !TOWERS.contains(&type_word) && !crate::economy::BUNKERS.contains(&type_word) {
            return false;
        }
        if let Some(at) = self.modes.iter().position(|m| *m == Mode::Manual(t)) {
            self.modes.truncate(at + 1);
            return true;
        }
        if !self.manual_open(t) {
            return false;
        }
        let Some(robot) = self.machine(t) else { return false };
        if !matches!(self.mode(), Mode::OnFoot | Mode::Command(_) | Mode::HqCommand(_)) {
            return false;
        }
        let chassis = robot.parts[robot.chassis_part].record.clone();
        let Ok(pilot) = crate::hero::Hero::pilot_for(&mut self.assembly, &chassis) else { return false };
        self.outer.off();
        self.hero.release_keys();
        let current = self.targets.current;
        let Some(robot) = self.machine_mut(t) else { return false };
        robot.wizard.clear();
        robot.auto_driver = 0;
        robot.sensors_taken = true;
        take_over(robot, true);
        robot.fire_target = None;
        robot.relink(current);
        self.driving = Some(Driving { target: t, pilot, fire_held: false, telepresence: false });
        self.modes.push(Mode::Manual(t));
        true
    }

    /// A game command's key going down or up in command mode (`0x10071cd0`, `0x10072740`):
    /// the camera's moves and zoom. True when the command is command mode's.
    pub fn command_key(&mut self, command: &str, down: bool) -> bool {
        use crate::command::Move;
        use parkan_formats::controls::*;
        // The cases test the view state, which the game menu leaves at 2 over a command view
        // (`0x10064650`): its keys still set and clear the camera's flags there.
        if !self.view_mode().commands() {
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
        self.driven_target().and_then(|d| self.machine(d)).unwrap_or(&self.hero.robot)
    }

    /// The unit whose guns take the player's target: the target list is the driven unit's, and
    /// `iron3d.dll:0x10091a80` hands its target to that unit's turret — the bot the player
    /// drives, else the hero.
    fn gunner_mut(&mut self) -> &mut Robot {
        let driven = self.driving.as_ref().map(|d| d.target);
        let found = driven
            .and_then(|d| self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(t, _)| *t == d));
        match found {
            Some((_, r)) => r,
            None => &mut self.hero.robot,
        }
    }

    /// The view's own unit, to change.
    fn driven_mut(&mut self) -> &mut Robot {
        let driven = self.driven_target();
        let found = driven
            .and_then(|d| self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(t, _)| *t == d));
        match found {
            Some((_, r)) => r,
            None => &mut self.hero.robot,
        }
    }

    /// The eye the world is drawn from: command mode's camera, the outer camera, or the driven
    /// unit's.
    pub fn eye(&self) -> crate::robot::Eye {
        // View state 4, once the hero is lost, replaces whatever view was up (`0x100a4e50`).
        if let Some(fallen) = self.fallen {
            return fallen;
        }
        let own = self.own_eye();
        if !self.outer_shows() {
            return own;
        }
        // The outer camera's line (mask `0x41a`, `0x208`) meets what a round's (`0x41e`, `0x208`)
        // does but class 2, a `WPNS` agent, which no shipped object is (docs/30, "What the outer
        // camera's line meets"): the ground, and every live target but the unit looked at.
        let unit = self.outer.unit.flatten();
        let meets = |from, to| {
            self.battle
                .combat
                .first_hit(&self.ground, unit, from, to, 0.0)
                .map(|(s, _, _)| (s.point, s.normal))
        };
        self.outer.place(&own, self.outer_bound(unit), meets)
    }

    /// The driven unit's own eye, or command mode's camera: what the right button picks along.
    pub fn own_eye(&self) -> crate::robot::Eye {
        if self.view_mode().commands() {
            return self.command.eye();
        }
        self.driven().eye().unwrap_or_else(|| self.hero.eye())
    }

    /// Whether the outer camera makes the view: turned on, in the mode it was turned on in.
    pub fn outer_shows(&self) -> bool {
        self.outer.on() && self.view_mode() == self.outer_mode
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
    /// clan steps, and of every building of it with a camera, a tower's gun (`0x1007db30` →
    /// `0x10033417`); the outer camera moves, or is turned off when its mode is left or its unit
    /// lost, and the mouse filter takes the zoomed multiplier while the view is zoomed
    /// (`0x100a4fc0`).
    ///
    /// STAND-IN: docs/30-turrets.md#not-established -- how often the game frame runs, which
    /// paces the zoom's steps and the outer camera's ease: once a 60 Hz tick.
    fn tick_views(&mut self) {
        let widest = self.hero.rig.camera_values[2];
        self.hero.zoom.step(widest);
        let player = Some(self.player_clan);
        for (t, robot) in self.robots.iter_mut().chain(self.emplacements.iter_mut()) {
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
        let reach = self.reach();
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) =
                    self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(t, _)| *t == d.target)
                {
                    crate::hero::drive_key(robot, &mut d.pilot, scan, pressed, reach);
                }
            }
            None => self.hero.key(scan, pressed),
        }
    }

    /// Every key and button held on the unit the player drives comes up, as the game lets
    /// them go when its window is left (`stdSetApplicationState`, docs/14, "Leaving the
    /// window lets every key up").
    pub fn release_keys(&mut self) {
        let reach = self.reach();
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) =
                    self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(t, _)| *t == d.target)
                {
                    crate::hero::drive_input(robot, &mut d.pilot, true, reach);
                }
            }
            None => self.hero.release_keys(),
        }
    }

    /// The input update of the unit the player drives.
    pub fn update_input(&mut self) {
        let reach = self.reach();
        match self.driving.as_mut() {
            Some(d) => {
                if let Some((_, robot)) =
                    self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(t, _)| *t == d.target)
                {
                    crate::hero::drive_input(robot, &mut d.pilot, false, reach);
                }
            }
            None => self.hero.update_input(),
        }
    }

    /// Whether Enter boards target `t` (`iron3d.dll:0x10071ff8`, `0x10076d30`, docs/39,
    /// "Boarding"): a unit of the player's clan, of size class 4, whose turret still has life
    /// (the node its first class-1 component names, a fitted turret's body), less than 20 away
    /// across the ground, while the player is on foot.
    pub fn boardable(&self, t: usize) -> bool {
        let Some(u) = self.units.get(t) else { return false };
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        let Some(target) = self.battle.combat.targets.get(t) else { return false };
        let turret_alive = turret_alive(robot, target);
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
    /// `VOICE_SELECTED_B`, the player takes it at the auto-driver level its record holds -- 0
    /// unless a unit page's drive button or the Y key left it another (mode 0 → 1,
    /// `0x100637c0`, writes none) -- with every held key let go, and a flyer taken over asks
    /// for the mission's message 100.
    pub fn board(&mut self, t: usize) -> bool {
        if !self.boardable(t) || !self.take(t, false) {
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
    /// At level 1 the unit's AI walks it and the player has its turret and guns; at level 2
    /// the AI has it whole and the player rides along ([`Reach`], docs/40, "Telepresence").
    /// The button writes the level into the unit's record before the push (`0x1008495a`), and
    /// the record keeps it once the unit is let go.
    pub fn telepresence(&mut self, t: usize, level: u8) -> bool {
        if !self.mode().commands() || !self.can_take(t) {
            return false;
        }
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
            robot.auto_driver = level.min(2);
        }
        if !self.take(t, true) {
            return false;
        }
        self.select_unit_alone(t);
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
            && turret_alive(robot, target)
            && robot.order.is_none_or(|o| o.code != ORDER_UPGRADE)
    }

    /// *Explode!* on unit `t` (`0x10075fa0`): unless one is pending, the press is stamped, and
    /// 0.6 s later the unit's takt kills it ([`Play::tick_explosions`]).
    pub fn explode(&mut self, t: usize) -> bool {
        if self.explode_pending(t) {
            return false;
        }
        self.exploding.push((t, self.hero.time_ms));
        true
    }

    /// Whether unit `t`'s *Explode!* is pending: its button is `_on` and its icon grey
    /// meanwhile (`0x10085890`).
    pub fn explode_pending(&self, t: usize) -> bool {
        self.exploding.iter().any(|&(e, _)| e == t)
    }

    /// Each pending *Explode!* more than 0.6 s old, at the top of its unit record's takt
    /// (`0x100756bf`–`0x100756f6`): the mark cleared and the unit killed through its life
    /// system's slot 7 ([`parkan_sim::combat::Combat::life_kill`]), which refuses an invulnerable
    /// one ([`Play::invulnerable`]).
    ///
    /// STAND-IN: docs/41-commander.md#explode--read -- whether `getTimer`, which times the
    /// 0.6 s, runs on a clock or on `timeGetTime` is not read: game time.
    fn tick_explosions(&mut self, now: f64) -> Vec<Event> {
        let (due, waiting): (Vec<_>, Vec<_>) =
            self.exploding.iter().partition(|&&(_, at)| now - at > EXPLODE_DELAY_MS);
        self.exploding = waiting;
        let mut events = Vec::new();
        for (t, _) in due {
            if !self.invulnerable(t) {
                events.extend(self.battle.combat.life_kill(t));
            }
        }
        events
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
    /// there, heading as [`leave_yaw`] reads. With none, "Risk area! Landing impossible." and the
    /// player stays aboard.
    pub fn leave(&mut self) -> bool {
        let Some(d) = self.driving.as_ref() else { return false };
        let t = d.target;
        let Some((_, robot)) = self.robots.iter().find(|(rt, _)| *rt == t) else { return false };
        let at = robot.walker.body.position;
        let r = robot.bound.1 + self.hero.bound.1;
        let flyer = robot.flyer;
        let place = if self.component_sound(t) {
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
            // A bot the component test refuses -- gone, or its turret's body at no life -- puts
            // the hero at (x − 1, y − 1), untested (`0x100634ad`).
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
        let yaw = leave_yaw(place, at);
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
        let Play { buildings, emplacements, battle, .. } = self;
        for b in buildings.iter_mut() {
            let standing: Vec<Standing> =
                children.iter().filter(|(s, _)| *s == b.target).map(|(_, c)| *c).collect();
            let Some(target) = battle.combat.targets.get_mut(b.target) else { continue };
            let Some(part) = target.parts.get(b.part) else { continue };
            let phases: Vec<_> = b.doors.iter().map(|d| d.phase).collect();
            let (changed, turned, fire) = b.tick(now, part, &standing);
            if changed || turned {
                // An emplacement's turret poses itself and everything hanging on it each
                // tick, aimed, and runs before this ([`tick_emplacements`]), so a building
                // leaves those where they are rather than put them back on their socket.
                let robot = emplacements.iter().find(|(t, _)| *t == b.target).map(|(_, r)| r);
                b.pose(&mut target.parts, |p| robot.is_some_and(|r| carried_by_turret(r, p)));
            }
            // A running item's own channels only pose the part. STAND-IN:
            // docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured
            // -- a mine's rotors, the Main Teleport's rings and the energy bridge's hub turn
            // for ever, so rebuilding the collision solid for them would rebuild it every
            // tick, on every one of them, for good. Whether the game's own collision mesh
            // follows an animated node is not read; a door's, which it plainly does, still
            // does here.
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
            // A plant stops what it was making for its old owner, who gets the mind its start
            // reserved back; the new owner's panel opens on its recent projects (*seen*, docs/36,
            // "For an engine"; how the capture ends order 12 is not read).
            if let Some(f) = self.factories.iter_mut().find(|f| f.target == t)
                && f.abort()
                && let Some(old) = owner
            {
                self.release_mind(old);
            }
            if let Some(p) = self.progression.as_mut() {
                let id = self.units[t].logical_id;
                p.progress.captured(id, taker);
                // The old clan's SuperAI runs `Fort_Captured` for what it has just lost
                // (docs/27, "Teleport out", for the dispatcher): `c2m3e` answers by raising
                // `PBM_BUILDING_INF_CAPTURE` to take it back.
                if let Some(old) = owner {
                    let notices = p.fort_captured(old, id);
                    let says: Vec<Say> = notices.iter().flat_map(|n| p.say(n)).collect();
                    self.says.extend(says);
                }
            }
            let old = owner.and_then(|c| self.clan(c));
            let word = owner.and_then(|c| self.word(c, taker));
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
        } else if TOWERS.contains(&self.units[t].type_word) {
            self.enter_manual(t);
        }
    }

    /// Clan `clan`'s minds not held: its mission's count less every live robot of the clan
    /// (the hero among them) and every reservation a build's start made that the clan's takt
    /// has not yet swept (docs/23, "The bot limit is the clan's mind count"). Every robot a
    /// mission places takes one as it is made (`iron3d.dll:0x100774d1`,
    /// `ArealMap.dll:0x100152ba`), the hero too, and a unit the hero's Enter took holds none:
    /// `Capture` on a unit takes no mind (`Behavior.dll:0x10009051`), so Mission 04's HQ is
    /// the one of its three that holds none. A build holds its reservation only until the
    /// clan's next takt, which frees every entry naming no object (`ai.dll:0x10006580`), so
    /// Mission 03's figure reads 3 while the first build collects its power and 4 again
    /// from 221.5 s.
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
        let reserved = self.reserved.get(&clan).copied().unwrap_or(0);
        minds.saturating_sub(robots + hero + reserved)
    }

    /// Start factory `f`'s shown project, in batch or not, as its panel's buttons do: nothing
    /// without a free mind, and a start reserves one.
    pub fn start_factory(&mut self, f: usize, batch: bool) -> bool {
        let Some(t) = self.factories.get(f).map(|f| f.target) else { return false };
        let clan = self.units.get(t).and_then(|u| u.clan).unwrap_or(self.player_clan);
        let free = self.free_minds(clan);
        let started = self.factories[f].start(batch, free);
        if started {
            self.reserve_mind(clan);
        }
        started
    }

    /// A build of clan `clan` started: it takes a free entry and marks it reserved
    /// (`Behavior.dll:0x1002a348`, and `0x1002a0bb` for a free bot).
    fn reserve_mind(&mut self, clan: i64) {
        *self.reserved.entry(clan).or_default() += 1;
    }

    /// A build of clan `clan` completed (`0x1002a7e5`) or was aborted (`0x10029983`): the
    /// clan's first reserved entry is freed, whichever build reserved it, if the takt has not
    /// freed it already.
    fn release_mind(&mut self, clan: i64) {
        if let Some(n) = self.reserved.get_mut(&clan) {
            *n = n.saturating_sub(1);
        }
    }

    /// A click on the factory screen of the plant that is target `target` (docs/36, "What the
    /// controls do").
    pub fn factory_click(&mut self, target: usize, click: crate::cockpit::factory::Click) {
        use crate::cockpit::factory::Click;
        let clan = self.units.get(target).and_then(|u| u.clan).unwrap_or(self.player_clan);
        let Some(i) = self.factories.iter().position(|f| f.target == target) else { return };
        match click {
            Click::Exit => {
                self.roll_back();
            }
            Click::Build | Click::Batch => {
                let batch = click == Click::Batch;
                if self.factories[i].idle() {
                    self.start_factory(i, batch);
                } else if self.factories[i].stop(batch) {
                    self.release_mind(clan);
                }
            }
            Click::Recent(r) => self.factories[i].selected = Some(r),
            Click::Active => self.factories[i].selected = None,
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
            // The completion frees the clan's first reserved entry and the bot then claims a
            // free one; with none free it is not made at all ("No free Mind...
            // CreateObjectFromScheme failed", `Behavior.dll:0x1001d4a6`), and the build is
            // over all the same.
            self.release_mind(clan);
            let made = if self.free_minds(clan) > 0 { self.spawn(&project, clan, at, 0.0) } else { None };
            if let Some(unit) = made {
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
            // A batch starts the design it has just made again, whatever the panel shows.
            let restarted = factory.batch && {
                factory.batch = false;
                factory.start_project(project, true, free)
            };
            factory.settle();
            if restarted {
                self.reserve_mind(clan);
            }
        }
    }

    /// The factory screen of the plant that is target `target` opens, from its pod or on the
    /// commander's page 5: it shows the unit in production, when there is one.
    pub fn show_production(&mut self, target: usize) {
        if let Some(f) = self.factories.iter_mut().find(|f| f.target == target) {
            f.show_production();
        }
    }

    /// Make a unit of `project`'s design for clan `clan` at `at`, turned to `yaw`
    /// (`CreateObjectFromScheme`, `Behavior.dll:0x1001d180`): a new target, robot, clan
    /// unit and name, joining its clan's list. Its target, or none when its design does not
    /// load as a robot.
    pub fn spawn(&mut self, project: &Project, clan: i64, at: Vec3, yaw: f32) -> Option<usize> {
        self.make_unit(&project.path, Some(&project.name), project.type_word, clan, at, yaw, None)
    }

    /// Make the unit at `path` of `type_word` for clan `clan` at `at`, turned to `yaw`: a new
    /// target, robot, clan unit and name. `name` is a design's, numbered by its clan as a built
    /// bot is; without one the unit takes its file's name, as a placed unit with none does.
    /// `logical_id` is the id its clan's list has it under already; without one it takes the
    /// next and joins the list.
    #[allow(clippy::too_many_arguments)]
    fn make_unit(
        &mut self,
        path: &str,
        name: Option<&str>,
        type_word: u32,
        clan: i64,
        at: Vec3,
        yaw: f32,
        logical_id: Option<i32>,
    ) -> Option<usize> {
        let filed = logical_id.is_some();
        let logical_id = logical_id.unwrap_or_else(|| {
            crate::progress::next_ids(self.units.iter().map(|u| u.logical_id).chain([self.hero_id])).0
        });
        let property = |name: &str, value: Value| mission::Property {
            name: name.to_owned(),
            kind: 0,
            value,
            minimum: value,
            maximum: value,
        };
        let placed = mission::Object {
            kind: KIND_UNIT,
            path: path.to_owned(),
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
                property("Type", Value::Int(type_word as i32)),
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
            named_clan: Some(clan),
            type_word,
            logical_id,
            kind: KIND_UNIT,
            announced: false,
            designation,
        });
        self.commander.paths.push(placed.path.clone());
        // A built bot is numbered by its clan: one more than the units the clan had named,
        // the hero among them (`0x10075d50`, docs/38, "The name"); the count here holds the
        // new unit already.
        let named = self.units.iter().filter(|u| u.named_clan == Some(clan) && u.kind == KIND_UNIT).count()
            + usize::from(clan == self.player_clan);
        let name = match name {
            Some(design) => design.replacen("-X ", &format!("-{named} "), 1),
            None => path.rsplit(['\\', '/']).next().unwrap_or(path).to_owned(),
        };
        self.names.push(name);
        self.deleted.push(false);
        let target = &self.battle.combat.targets[t];
        let solid = Solid::from_parts(&target.parts, target.centre, target.radius, false, |_, _| None);
        self.ground.solids.push(solid);
        self.robots.push((t, robot));
        if !filed && let Some(p) = self.progression.as_mut() {
            p.progress.join(logical_id, clan, type_word, at, self.hero.time_ms);
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
        for (p, destroyed) in parts.into_iter().zip(gone) {
            for n in destroyed {
                self.hero_node_blast(p, n, now);
            }
        }
        if self.hero.dead() {
            self.hero_lost();
        }
        events
    }

    /// The loss of the player's clan's hero fails the mission (`iron3d.dll:0x10075619`), and
    /// the world is drawn from then on from the level's second camera placed over where the
    /// hero stood (`0x10075612` → `0x100a4e50`, view state 4): [`Play::fallen_eye`].
    fn hero_lost(&mut self) {
        self.fallen = Some(self.fallen_eye());
        if let Some(p) = self.progression.as_mut() {
            p.progress.outcome = Some(false);
            let says = p.say(&Notice::MissionFailed);
            self.says.extend(says);
        }
    }

    /// The level's second camera (`+0x68`) as the hero's loss places it (`0x100a4e50`): the
    /// hero's world matrix handed to its view whole, but for its height, which is 16 over the
    /// highest surface at the hero's x, y the level's query finds with mask `0x41a` -- the
    /// landscape, a building, scenery or a unit (`0x100a14d0` with 1, 1, 1; `0x100e5c8c`). The
    /// view looks along the matrix's first column (docs/40, "The frame"), the hero's own x
    /// axis: level, to its right-hand side. It is made with the command camera's field, 1.04,
    /// and near plane, 3 (`0x100a29c8`). Its update runs only in a network game or with the
    /// level's `+0xaf5` set (`0x10037a70`), so in single play it holds still.
    pub fn fallen_eye(&self) -> crate::robot::Eye {
        let (at, yaw) = self.hero.walker.drawn(self.hero.time_ms);
        let top = at.with_z(at.z + 10_000.0);
        let bottom = at.with_z(at.z - 10_000.0);
        let under = self
            .battle
            .combat
            .first_hit(&self.ground, None, top, bottom, 0.0)
            .map(|(s, _, _)| s.point.z)
            .or_else(|| self.ground.below(at.x, at.y, 10_000.0).map(|h| h.point.z))
            .unwrap_or(at.z);
        crate::robot::Eye {
            position: Vec3::new(at.x, at.y, under + FALLEN_ABOVE),
            forward: Vec3::new(yaw.cos(), yaw.sin(), 0.0),
            up: Vec3::Z,
            fov_x: crate::command::FIELD,
            near: crate::command::NEAR,
        }
    }

    /// A node of the hero's destroyed: its explosion at its sphere's centre, as a placed
    /// unit's node plays its own (docs/26).
    fn hero_node_blast(&mut self, p: usize, n: usize, now: f64) {
        let Some(Some(exp)) = self.hero.blasts.get(p).and_then(|b| b.get(n)).cloned() else { return };
        let mesh = self.hero.parts[p].mesh.clone();
        let Some(slot) = mesh.mesh.slots.get(usize::from(mesh.mesh.nodes[n].slot_index[0])) else { return };
        let node = self.hero.placement().compose(&self.hero.part_pose(p, n));
        self.node_blast(&exp, &node, slot.sphere, 1.0, now);
    }

    /// The hero's target follows the hero: its nodes where it stands, struck while it is alive
    /// and in the world.
    fn pose_hero(&mut self) {
        let alive = !self.hero.dead() && !self.hero_away();
        let Some(target) = self.battle.combat.hero.as_mut() else { return };
        pose_target(&self.hero.robot, target);
        target.alive = alive;
    }

    /// Each building's shield's power level for the frame: the share its batteries serve it at
    /// (docs/23). A unit's is set on its own power tick ([`Play::tick_power`]).
    ///
    /// STAND-IN: docs/26-damage.md#power--read -- a building's shield and deflector draws are not
    /// taken from its batteries, which serve the efficiency alone.
    fn power_shields(&mut self) {
        for t in 0..self.battle.combat.targets.len() {
            let Some(level) = self.economy.site(t).map(|site| site.level) else { continue };
            if let Some(shield) = self.battle.combat.targets[t].shield.as_mut() {
                shield.level = level;
            }
        }
    }

    /// Every unit's power tick that is due (`Control.dll:0x1002d340`, docs/23, "Bots spend power
    /// through the same code, priced by part"): the hero's, with its own switches, and each
    /// robot's, with the switches of the player driving it.
    ///
    /// STAND-IN: docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured
    /// -- the AI's camouflage switch (the device manager's sibling, `Behavior.dll:0x10019a10`) is
    /// not modelled: a unit the player does not drive keeps its camouflage off.
    fn tick_power(&mut self) {
        let jitter = |p: &mut Play| p.economy.power_jitter();
        // A paused world, a dead hero and a hero out of the world keep their tick's clock moving
        // and spend nothing, so no tick afterwards pays for the gap.
        let hero_runs = !self.paused && !self.hero.dead() && !self.hero_away();
        let j = jitter(self);
        if let Some(mut power) = self.hero.robot.power.take() {
            if let Some(dt) = power.due(self.hero.robot.time_ms, j)
                && hero_runs
            {
                let share = self.hero.robot.engine_share();
                let mut lives: Vec<Option<&mut Life>> =
                    self.hero.lives.iter_mut().map(Option::as_mut).collect();
                let shield = self.battle.combat.hero.as_mut().and_then(|h| h.shield.as_mut());
                let switches = self.hero.pilot.switches;
                power.tick(dt, &mut lives, shield, &mut self.hero.robot.guns, share, switches);
            }
            self.hero.robot.power = Some(power);
        }
        for r in 0..self.robots.len() {
            let j = jitter(self);
            let Play { robots, battle, driving, paused, .. } = self;
            let (t, robot) = &mut robots[r];
            let Some(mut power) = robot.power.take() else { continue };
            if let Some(dt) = power.due(robot.time_ms, j)
                && !*paused
                && let Some(target) = battle.combat.targets.get_mut(*t).filter(|x| x.alive)
            {
                let share = robot.engine_share();
                // A bot the player drives spends on the player's switches; one left to itself
                // spends on what its own repair decision last sent (docs/26, "What the AI does
                // with the switch").
                let switches = driving.as_ref().filter(|d| d.target == *t).map_or(
                    parkan_sim::input::Switches { repair: robot.behaviour.repair, ..Default::default() },
                    |d| d.pilot.switches,
                );
                let Target { parts, shield, .. } = target;
                let mut lives: Vec<Option<&mut Life>> = parts.iter_mut().map(|p| p.life.as_mut()).collect();
                power.tick(dt, &mut lives, shield.as_mut(), &mut robot.guns, share, switches);
            }
            robot.power = Some(power);
        }
    }

    /// What unit `t`'s own systems report to its behaviour (docs/27, "What sends a bot to a
    /// dock"): its life fraction over its whole control system (property `0x31`), its
    /// batteries' fill, the rounds its guns have left over their magazines, and whether more
    /// than [`SERVICE_GUNS`] of them are under [`SERVICE_MAGAZINE`] of theirs. A unit with no
    /// battery reports a full one, and a gun of unlimited rounds counts as full. A building
    /// reports its own batteries, and its guns if it carries any.
    pub(crate) fn condition(&self, t: usize) -> Condition {
        let (life, full) = self.battle.combat.targets.get(t).map_or((0.0, 0.0), |x| {
            x.parts
                .iter()
                .filter_map(|p| p.life.as_ref())
                .fold((0.0, 0.0), |(l, f), life| (l + life.total(), f + life.full()))
        });
        let life = if full > 0.0 { life / full } else { 1.0 };
        // A building's charge is its batteries' (docs/23), whether or not it carries guns.
        let charge = self.battery(Some(t)).unwrap_or(1.0);
        let Some(robot) = self.machine(t) else {
            return Condition { life, charge, ..Condition::default() };
        };
        let counted: Vec<&parkan_sim::guns::Gun> = robot.guns.iter().filter(|g| g.magazine > 0).collect();
        let dry = counted.iter().filter(|g| (g.rounds as f32) < SERVICE_MAGAZINE * g.magazine as f32).count();
        let (rounds, magazines) =
            counted.iter().fold((0.0, 0.0), |(r, m), g| (r + g.rounds as f32, m + g.magazine as f32));
        Condition {
            life,
            charge,
            ammo: if magazines > 0.0 { rounds / magazines } else { 1.0 },
            guns_dry: !counted.is_empty() && dry as f32 > SERVICE_GUNS * counted.len() as f32,
        }
    }

    /// The batteries' fill of unit `unit`, or of the hero for `None`: a unit's power, else a
    /// building's batteries; `None` for what holds none.
    pub fn battery(&self, unit: Option<usize>) -> Option<f32> {
        let Some(t) = unit else { return self.hero.robot.power.as_ref().map(crate::power::Power::fill) };
        self.robots
            .iter()
            .find(|(rt, _)| *rt == t)
            .and_then(|(_, r)| r.power.as_ref().map(crate::power::Power::fill))
            .or_else(|| self.economy.site(t).map(|site| site.battery.charge))
    }

    /// A hit on target `t` from the round's `owner` (message `0x19`): a robot's behaviour asks
    /// for an attack on the unit that fired, and, unless the victim is a hero, its clan's
    /// SuperAI takes 0.004 off its attitude toward the firer's clan (`0x1000658c`, docs/25,
    /// "Clan relations"). A building does nothing (`0x100064b0`). A unit the player drives is
    /// hurt as any other: the handler asks nothing of the Wizard's words or of the flags a take
    /// sets, only the network bit, property `0x208` and the building bit
    /// (`0x100064f2`-`0x1000651f`), so at levels 1 and 2, where its takt runs, it turns on the
    /// firer (docs/40, "Telepresence").
    ///
    /// The hit is gated as it arrives (`0x100179c0`). A unit whose takt runs asks at its next
    /// one, a frame on; the unit the player drives at level 0, whose takt does not run, is
    /// gated here and now, and the attack the gate lets through waits on its stack for the
    /// letting-go (docs/40, "What the AI does at levels 1 and 2").
    ///
    /// STAND-IN: docs/31-packages.md#a-hit-pulls-a-unit-in--read -- the call for help to the
    /// clan's warriors within 400 is not modelled.
    pub fn hurt(&mut self, t: usize, owner: Option<usize>) {
        let firer = match owner {
            None => self.hero_id,
            Some(o) => match self.units.get(o) {
                Some(u) => u.logical_id,
                None => return,
            },
        };
        let Some(r) = self.robots.iter().position(|(rt, _)| *rt == t) else { return };
        let driven = self.driving.as_ref().is_some_and(|d| d.target == t);
        if driven && !self.reach().ai_moves() {
            let others = self.seen_by(t, &self.seen(), &[]);
            let condition = self.condition(t);
            let bounds = self.ground.bounds();
            let graph = &self.graph;
            let usable = |x: f32, y: f32| graph.as_ref().is_none_or(|g| g.usable(x, y));
            let animal = self.units[t].type_word & CLASS_ANIMAL != 0;
            let robot = &mut self.robots[r].1;
            let now = robot.time_ms;
            let senses = Senses {
                now_ms: now,
                position: robot.walker.body.position,
                seen: &others,
                places: &[],
                docks: &[],
                condition,
                size_class: robot.size_class,
                flyer: robot.flyer,
                bounds,
                usable: Usable(&usable),
                has_weapon: !robot.guns.is_empty(),
                walker_idle: robot.wizard.idle(now),
                neutral: false,
                building: false,
                animal,
                pastures: parkan_sim::behaviour::Pastures::NONE,
            };
            robot.behaviour.hurt_now(firer, &senses);
        } else {
            self.robots[r].1.behaviour.hurt(firer);
        }
        self.resent(t, owner);
    }

    /// The attitude a hit costs: the victim's clan toward the firer's. The slot 67 handler
    /// skips a building and a hero (`Type` `0x1020000`, `Behavior.dll:0x1000656c`), and it is
    /// the firer's object's owner word that names the clan (`0x10006587`), so a round whose
    /// firer is gone takes nothing.
    fn resent(&mut self, victim: usize, owner: Option<usize>) {
        let them = match owner {
            None => Some(self.player_clan),
            Some(o) => self.units.get(o).and_then(|u| u.clan),
        };
        let Some(us) = self.units.get(victim).filter(|u| u.type_word != parkan_formats::research::TYPE_HERO)
        else {
            return;
        };
        let (Some(us), Some(them)) = (us.clan, them) else { return };
        if let (Ok(us), Ok(them)) = (usize::try_from(us), usize::try_from(them)) {
            self.attitudes.hurt(us, them);
        }
    }

    /// Every clan brain whose 7-8 s takt is due (`ai.dll:0x100017f0` -> `0x10005f30`): it
    /// applies what the hits added and reads each relation word off its attitude again.
    fn takt_relations(&mut self, now_ms: f64) {
        if !self.attitudes.takt(now_ms) {
            return;
        }
        self.relations = self.attitudes.words();
        if let Some(p) = self.progression.as_mut() {
            p.relations = self.relations.clone();
        }
    }

    /// The clan of animal `t` and its zones, the pastures the areal map hands the clan's
    /// migrating animals (`IMission` slot 8 → areal map slot 35, docs/31).
    fn zones(&self, t: usize) -> (Option<usize>, Vec<parkan_sim::behaviour::Pasture>) {
        let Some(c) = self.units.get(t).and_then(|u| u.clan) else { return (None, Vec::new()) };
        let zones = self.clan(c).map_or_else(Vec::new, |clan| {
            clan.zones
                .iter()
                .map(|z| parkan_sim::behaviour::Pasture {
                    centre: Vec3::from_array(z.position),
                    inner: z.inner,
                    outer: z.outer,
                })
                .collect()
        });
        (usize::try_from(c).ok(), zones)
    }

    /// A hit met target `t`'s shield (`0x1002c83e`, played at `0x10025ca0`): the generator's
    /// effect at the bubble's centre, its first axis toward the hit, sized by the bubble's
    /// radius, in time mode 1, on the next of its three instances, the fourth restarting the
    /// first.
    ///
    /// STAND-IN: docs/26-damage.md#what-a-shield-hit-draws--read-and-measured -- the flash is
    /// placed on node 0 and so turns with the unit; here it keeps its direction and follows
    /// the bubble's centre.
    fn shield_flash(&mut self, t: usize, point: Vec3, now: f64) {
        let Some(target) = self.battle.combat.target(t) else { return };
        let Some(shield) = target.shield.as_ref().filter(|s| !s.effect.is_empty()) else { return };
        let (name, centre, radius) = (shield.effect.clone(), target.centre, target.radius);
        let slot = self.shield_flashes.entry(t).or_insert(0);
        let owner = Owner::Shield(t, *slot);
        *slot = (*slot + 1) % SHIELD_FLASHES;
        self.fx.retain(|o, _| *o != owner);
        let frame = Frame::along(centre, point - centre, radius);
        self.fx.start(owner, &name, frame, 1.0, now, Some(SHIELD_FLASH_MODE));
    }

    /// Every shield flash stands at its bubble's centre as the unit moves.
    fn follow_shield_flashes(&mut self) {
        let centres: Vec<(usize, Vec3)> = self
            .shield_flashes
            .keys()
            .filter_map(|&t| Some((t, self.battle.combat.target(t)?.centre)))
            .collect();
        for (t, centre) in centres {
            for k in 0..SHIELD_FLASHES {
                for instance in self.fx.owned(Owner::Shield(t, k)) {
                    instance.frame.origin = centre;
                }
            }
        }
    }

    /// The hero's lives handed to its target for the battle's frame (`to_battle`), or taken
    /// back after it.
    fn lend_hero_lives(&mut self, to_battle: bool) {
        let Some(target) = self.battle.combat.hero.as_mut() else { return };
        for (part, life) in target.parts.iter_mut().zip(self.hero.lives.iter_mut()) {
            if to_battle {
                part.life = life.take();
            } else {
                *life = part.life.take();
            }
        }
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
            let index = self.battle.combat.hero_index();
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
            seen.push((Some(index), hero, Some(self.player_clan)));
        }
        seen
    }

    /// What target `t` sees of `seen`: everything but itself, each of its own clan or
    /// hostile to it by the behaviour's test (`0x1000d460`), and marked with whether its own
    /// radar list holds it, `sensed` naming the ids that list carries. The radar module drops
    /// a contact of a nature or neutral clan from both its lists (`0x10023240`), so neither is
    /// hostile.
    fn seen_by(&self, t: usize, seen: &[Sighting], sensed: &[i32]) -> Vec<Seen> {
        let (id, clan) = (self.units[t].logical_id, self.units[t].clan);
        seen.iter()
            .filter(|(_, s, _)| s.id != id)
            .map(|&(_, s, c)| Seen {
                own: clan.is_some() && c == clan,
                hostile: self.behaviour_hostile(clan, c)
                    && c.and_then(|c| self.clan(c))
                        .is_some_and(|c| !matches!(c.kind, CLAN_NATURE | CLAN_NEUTRAL)),
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
            // A unit the player drives keeps what its auto-driver level leaves the AI: its walk
            // at levels 1 and 2, and its aim and fire at 2.
            let driven = self.driving.as_ref().is_some_and(|d| d.target == t);
            let reach = if driven { self.reach() } else { Reach::Nothing };
            if !self.paused && reach.ai_moves() && self.thinks(self.units[t].clan) {
                let sent = self.robots[r].1.behaviour.repair;
                self.behave(r, dt_ms, &seen, &world, reach.ai_fights());
                // The repair decision runs in the unit's takt, which flag `0x10` lets run at
                // levels 1 and 2 as for a unit nobody drives, and its device manager sends
                // each class-15 component the state when it changes (`+0xaf`,
                // `Behavior.dll:0x10019a80`), through no word of the Wizard's: the behaviour
                // takes no component's bits (`0x100067c1` returns for any but -1). So the
                // switch the player drives with turns under it too (docs/26, "What the AI does
                // with the switch").
                let decided = self.robots[r].1.behaviour.repair;
                if driven
                    && decided != sent
                    && let Some(d) = self.driving.as_mut()
                {
                    d.pilot.switches.repair = decided;
                }
            }
            let (t, robot) = &mut self.robots[r];
            let target = &mut self.battle.combat.targets[*t];
            let from = robot.collision_centre();
            robot
                .check_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            let shots = match self.driving.as_mut().filter(|d| d.target == *t) {
                Some(d) => crate::hero::drive(
                    robot,
                    &mut d.pilot,
                    &mut d.fire_held,
                    dt_ms,
                    mouse,
                    &self.ground,
                    reach,
                ),
                None => {
                    robot.advance(dt_ms, &self.ground);
                    robot.takt(dt_ms)
                }
            };
            // The collision pass, after the move and the ground contact: a unit is a mover
            // like the hero, so a building's walls hold it in until a door opens for it.
            let mass = robot
                .heft
                .mass(|p, n| node_share(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            // STAND-IN: docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured
            // -- a flyer's collision flags are read to carry 8, so a building's floors push it
            // too, and taken whole since its states lack bit 4. With that, Mission 02's flyer,
            // made at the Large Factory's creation vertex 5 m over the hall floor, is pushed 33 m
            // up in its first frame, through the hall's roof; what keeps a flyer's sphere off a
            // floor it is made on is not read -- its walk points' heights are (docs/24, "A
            // flyer's walk points"), and they do not. No robot keeps the floors.
            let keeps_floors = robot.walker.keeps_floors() && !robot.flyer;
            let push = collision_push(
                &self.ground.solids,
                from,
                robot.collision_centre(),
                robot.collision.1,
                Some(*t),
                mass,
                keeps_floors,
            );
            if push.length_squared() >= NO_CONTACT {
                robot.walker.take_push(push);
            }
            robot.turn_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            robot.relimit(|p, n| node_share(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            if !shots.is_empty() {
                fired.push((r, shots));
            }
            pose_target(robot, target);
            if let Some(solid) = self.ground.solids.get_mut(*t) {
                *solid = Solid {
                    mass,
                    ..Solid::from_parts(&target.parts, target.centre, target.radius, false, |_, _| None)
                };
            }
        }
        for (r, shots) in fired {
            let (t, robot) = &self.robots[r];
            let launched = launches(robot, Some(*t), &shots, &self.battle, &self.ground);
            let now = robot.time_ms;
            self.launch(launched, now);
        }
        self.tick_emplacements(dt_ms, &seen, &world, mouse);
    }

    /// Every building that carries guns on a turret: its game time moves on; while its clan
    /// thinks, its behaviour's fire control picks its target, its turret traces it and its
    /// guns fire as an AI unit's do -- or, in its manual control, the player's mouse turns its
    /// turret and the player's keys fire its guns; then its turret's and guns' takt, and the
    /// turret and what hangs on it are posed where they aim. A building does not move: no
    /// machine steps, and its faces stay as placed.
    fn tick_emplacements(&mut self, dt_ms: f64, seen: &[Sighting], world: &[Contact], mouse: [f32; 2]) {
        let mut fired = Vec::new();
        for e in 0..self.emplacements.len() {
            let t = self.emplacements[e].0;
            if !self.battle.combat.targets.get(t).is_some_and(|target| target.alive) {
                continue;
            }
            self.emplacements[e].1.time_ms += dt_ms;
            if let Some(d) = self.driving.as_mut().filter(|d| d.target == t) {
                crate::hero::drive_guns(&mut self.emplacements[e].1, &mut d.pilot, &mut d.fire_held, mouse);
            } else if !self.paused && self.thinks(self.units[t].clan) {
                let sensed = self.radar_ids(e, true, world);
                let others = self.seen_by(t, seen, &sensed);
                let type_word = self.units[t].type_word;
                let Play { emplacements, battle, ground, building_fire_floor, graph, .. } = self;
                let robot = &mut emplacements[e].1;
                let usable = |x: f32, y: f32| graph.as_ref().is_none_or(|g| g.usable(x, y));
                let senses = Senses {
                    now_ms: robot.time_ms,
                    position: robot.walker.body.position,
                    seen: &others,
                    places: &[],
                    docks: &[],
                    condition: Condition::default(),
                    size_class: robot.size_class,
                    flyer: robot.flyer,
                    bounds: ground.bounds(),
                    usable: Usable(&usable),
                    has_weapon: !robot.guns.is_empty(),
                    walker_idle: true,
                    neutral: false,
                    building: true,
                    animal: false,
                    pastures: parkan_sim::behaviour::Pastures::NONE,
                };
                let takt = robot.behaviour.takt(&senses);
                aim_and_fire(robot, t, type_word, &takt, seen, battle, ground, false, *building_fire_floor);
            }
            let (t, robot) = &mut self.emplacements[e];
            let target = &self.battle.combat.targets[*t];
            robot
                .check_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
            // A tower's mast is a chassis item like a warbot's rotors, so its turret, guns and
            // radar ride it: `chassis_pose` takes a node's frame from the items that drive it.
            robot.turn_devices(|p, n| node_alive(target.parts.get(p).and_then(|part| part.life.as_ref()), n));
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
            // A build order goes to the factory building, not to a machine's task stack.
            if o.order.code == orders::CONSTRUCT {
                if let orders::Target::Select(mode) = o.order.target {
                    self.ai_build(t, o.order.parameter as u32, mode);
                }
                continue;
            }
            let robot = self.robots.iter_mut().chain(self.emplacements.iter_mut()).find(|(rt, _)| *rt == t);
            if let Some((_, robot)) = robot
                && robot.behaviour.insert_order(&o.order, o.insert)
            {
                robot.order = Some(o.order);
            }
        }
    }

    /// Carry out what function 57 ran (docs/15, "What the console's `create`, `bcreate` and
    /// `death` do"), each object made under the id its clan's list has it under already. One
    /// the engine cannot make is taken off the list again.
    fn run_console(&mut self) {
        let Some(p) = self.progression.as_mut() else { return };
        for call in std::mem::take(&mut p.console) {
            let path = |file: &str| format!("{}{file}", console::UNITS_AUTO);
            let made = match call.command {
                Command::Create { x, y, heading, clan, ref file } => {
                    let (x, y) = (x as f32, y as f32);
                    // The level's probe for the landscape and the buildings (`0x100a14d0`, mask
                    // `0xa`), 0 where it finds nothing, and 2 over that.
                    let ground = self.ground.below(x, y, PROBE_TOP).map_or(0.0, |h| h.point.z);
                    let at = Vec3::new(x, y, ground + console::CREATE_LIFT);
                    let clan = i64::from(clan);
                    self.make_unit(&path(file), None, call.type_word, clan, at, heading as f32, Some(call.id))
                }
                Command::BCreate { x, y, z, clan, ref file } => {
                    let at = Vec3::new(x as f32, y as f32, z as f32);
                    self.stand_building(i64::from(clan), call.type_word, &path(file), at, call.id)
                }
                Command::Death { x, y, radius } => {
                    let events = self.death(x as f32, y as f32, radius as f32);
                    self.pending_events.extend(events);
                    continue;
                }
            };
            if made.is_none()
                && let Some(p) = self.progression.as_mut()
            {
                p.progress.destroyed(call.id);
                p.progress.deleted(call.id);
            }
        }
    }

    /// A clan's `ORDER_BUILDING_CONSTRUCT` at the factory that is target `t` (docs/36,
    /// "Production"): its own design store picks a design of the robot type the order's
    /// parameter names, ranked by the `SELECT_*` its target carries and drawn over function
    /// 69's spread, and the factory starts it. The store is loaded on the first such order,
    /// priced against that clan's own research tree.
    fn ai_build(&mut self, t: usize, type_word: u32, mode: u32) -> bool {
        let Some(f) = self.factories.iter().position(|f| f.target == t) else { return false };
        let Some(clan) = self.units[t].clan else { return false };
        if !self.factories[f].idle() {
            return false;
        }
        if !self.stores.contains_key(&clan) {
            let grade = usize::from(self.factories[f].size);
            let Some(catalogue) = self.research.catalogue(clan) else { return false };
            let game = self.assembly.game.clone();
            let store =
                crate::factory::Store::load(&game, &mut self.assembly, catalogue, grade).unwrap_or_default();
            self.stores.insert(clan, store);
        }
        let draw = self.progression.as_mut().map_or(0, |p| p.draw(clan));
        let Some(project) = self.stores.get(&clan).and_then(|s| s.pick(type_word, mode, draw)).cloned()
        else {
            return false;
        };
        let free = self.free_minds(clan);
        let started = self.factories[f].start_project(project, false, free);
        if started {
            self.reserve_mind(clan);
        }
        started
    }

    /// One robot's behaviour takt (docs/31): its task's walk handed to the walker, which
    /// cuts it into points the Wizard follows; the fire control's target handed to its
    /// guns and traced by its turret; and each gun let fire once its AI timer runs out and
    /// its score clears the bar, or freely during a search or an attack (docs/29, "How the
    /// AI fires"). Without `fights` -- behaviour flags `0x20` and `0x40` off, a unit the player
    /// drives at auto-driver level 1 -- the fight module does not run and only the walk does.
    fn behave(&mut self, r: usize, dt_ms: f64, seen: &[Sighting], world: &[Contact], fights: bool) {
        let t = self.robots[r].0;
        self.escape_off_building(r);
        let sensed = self.radar_ids(r, false, world);
        let others = self.seen_by(t, seen, &sensed);
        let type_word = self.units[t].type_word;
        let animal = type_word & CLASS_ANIMAL != 0;
        let (clan, zones) = if animal { self.zones(t) } else { (None, Vec::new()) };
        let bounds = self.ground.bounds();
        let capturing = matches!(
            self.robots[r].1.behaviour.task(),
            Task::Search { search: Search::Capture(_) | Search::Building(_), .. }
        );
        let places = if capturing { self.capture_places() } else { Vec::new() };
        // The docks are only wanted by a refit, and by a unit that needs service and so may
        // send itself to one (docs/27, "What sends a bot to a dock").
        let condition = self.condition(t);
        let refitting = matches!(self.robots[r].1.behaviour.task(), Task::Reload { .. });
        let docks = if refitting || condition.needs_service(false) { self.docks_for(t) } else { Vec::new() };
        let graph = &self.graph;
        let usable = |x: f32, y: f32| graph.as_ref().is_none_or(|g| g.usable(x, y));
        let grazing = &self.grazing;
        let (_, robot) = &mut self.robots[r];
        let now = robot.time_ms;
        let ask = || clan.and_then(|c| grazing.borrow_mut().ask(c, &zones, now));
        let at = robot.walker.body.position;
        let senses = Senses {
            now_ms: now,
            position: at,
            seen: &others,
            places: &places,
            docks: &docks,
            condition,
            size_class: robot.size_class,
            flyer: robot.flyer,
            bounds,
            usable: Usable(&usable),
            has_weapon: !robot.guns.is_empty(),
            walker_idle: robot.wizard.idle(now),
            neutral: false,
            building: false,
            animal,
            pastures: parkan_sim::behaviour::Pastures(&ask),
        };
        let takt = robot.behaviour.takt(&senses);
        let (flyer, top, low) = (
            robot.flyer,
            robot.walker.limits.top_speed[1].abs(),
            robot.walker.controller.triples[1][1].abs(),
        );
        // A walk's legs: those it takes as a walker's are cut, and a flyer's in the open, taken
        // one point at a time.
        let legs = match takt.walk {
            Walk::Keep => None,
            Walk::Clear => Some((Vec::new(), Vec::new(), 0.0, false)),
            Walk::To(goal, share) | Walk::Level(goal, share) if flyer => {
                // A flyer's walk out of a building keeps the hall way's vertices at their own
                // heights; its leg on from there is cut every 20 m across the ground, and each
                // point is given its height over what lies under it (docs/24, "A flyer's walk
                // points") -- unless the place keeps its own, follow's, when the cut points
                // stay on the straight line to it (`0x1003a6bc`-`0x1003a726`).
                let level = matches!(takt.walk, Walk::Level(..));
                let mut hall = self.legs_to(t, at, goal);
                let open: Vec<Vec3> = match hall.pop() {
                    Some(end) => {
                        let start = hall.last().copied().unwrap_or(at);
                        let cut = flight_leg(start, end);
                        // STAND-IN: docs/24-motion.md#a-flyers-walk-points--read-and-measured -- the
                        // walker stands an animal's points 30 to 80 m higher still
                        // (`0x10040f52`-`0x10040f7c`, [`Play::flight_height`]), which carries
                        // Mission 02's grazing medusas 5.6 m up in two minutes, and a fought one
                        // 30 to 50 m up to graze on in the air, the Wizard's descent fitting no
                        // moving state; the player remembers the game's staying where they hover
                        // until provoked, never flying (2026-09-22), and what holds them is not
                        // read. An animal's points keep the walk's own height, at least 15 over
                        // the ground under them, as every flyer's did before the read.
                        cut.into_iter()
                            .map(|p| {
                                let z = if level {
                                    p.z
                                } else if animal {
                                    let floor =
                                        self.ground.below(p.x, p.y, 10_000.0).map_or(p.z, |h| h.point.z);
                                    p.z.max(floor + FLIGHT_CLEARANCE)
                                } else {
                                    self.flight_height(p.x, p.y, animal)
                                };
                                p.with_z(z)
                            })
                            .collect()
                    }
                    None => Vec::new(),
                };
                Some((hall, open, share, false))
            }
            Walk::To(goal, share) | Walk::Level(goal, share) => {
                let floor = self.ground.below(goal.x, goal.y, 10_000.0).map_or(goal.z, |h| h.point.z);
                Some((self.legs_to(t, at, goal.with_z(floor)), Vec::new(), share, false))
            }
            Walk::Inside(id, pod, share) => Some((self.legs_inside(t, at, id, pod), Vec::new(), share, true)),
        };
        let (_, robot) = &mut self.robots[r];
        match legs {
            None => {}
            Some((hall, open, _, _)) if hall.is_empty() && open.is_empty() => robot.wizard.clear(),
            Some((legs, open, share, inside)) => {
                let speed = walk_speed(share * top, top, low, SPEED_MAXIMUM_FACTOR);
                // A walker's points run along the ground; a flyer's follow each point's height,
                // down to the hall way's vertices inside a building.
                let flags = if flyer { 0 } else { GROUND_POINT };
                let (points, stop) = if flyer && !inside {
                    flight(at, &legs, &open, speed, now)
                } else if legs.len() == 1 && !inside {
                    straight_walk(at, legs[0], speed, now, flags)
                } else {
                    path_walk(at, &legs, speed, now, flags, inside)
                };
                robot.wizard.clear();
                robot.wizard.give(points);
                robot.wizard.stop_at(stop);
            }
        }
        let forward = robot.walker.body.forward();
        robot.walker.drive = Some(robot.wizard.takt(now, at, forward, dt_ms));
        if fights {
            aim_and_fire(robot, t, type_word, &takt, seen, &self.battle, &self.ground, animal, false);
        }
    }

    /// The height a flyer's walk point at `(x, y)` is given (`Behavior.dll:0x10040f20`): the
    /// ground under it as the behaviour sees it ([`Play::flight_ground`]) and 15, and for an
    /// animal 30 more and up to 50 more again, drawn afresh for the point.
    pub fn flight_height(&mut self, x: f32, y: f32, animal: bool) -> f32 {
        let mut z = self.flight_ground(x, y) + FLIGHT_CLEARANCE;
        if animal {
            z += ANIMAL_FLIGHT + ANIMAL_FLIGHT_SPREAD * self.walk_draw();
        }
        z
    }

    /// The ground under `(x, y)` as the behaviour's ground routine answers it
    /// (`Behavior.dll:0x100146b0`): straight down from above the world, the top face of a
    /// building, a tree or a stone there and 100 more (classes 3 and 10, `0x10014723`, and
    /// `0x100148a5`); failing that the landscape's, water's sheet and all, at the point, then
    /// 0.1 off in x, then in x and y (`0x10014779`-`0x100147f6`); failing that 0.
    pub fn flight_ground(&self, x: f32, y: f32) -> f32 {
        let units = &self.units;
        let obstacle = |i: usize| {
            units.get(i).is_some_and(|u| matches!(u.kind, KIND_BUILDING | KIND_VEGETATION | KIND_ROCK))
        };
        if let Some(top) = self.ground.solid_top(x, y, obstacle) {
            return top + OVER_OBSTACLE;
        }
        [(0.0, 0.0), (0.1, 0.0), (0.1, 0.1)]
            .into_iter()
            .find_map(|(dx, dy)| self.ground.landscape_top(x + dx, y + dy))
            .unwrap_or(0.0)
    }

    /// A draw from 0 to 1 for an animal's flight height, the engine's stand-in for the
    /// `rand()` the walker draws from.
    fn walk_draw(&mut self) -> f32 {
        let mut x = self.flight_seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.flight_seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
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

    /// God mode, the engine's own cheat: the hero walks [`GOD_SPEED`] times as fast, has
    /// [`GOD_LIFE`] times the hit points on every node, and its rounds do [`GOD_DAMAGE`] times
    /// the damage.
    pub fn god_mode(&mut self) {
        if self.god {
            return;
        }
        self.god = true;
        self.hero.walker.stride_scale = GOD_SPEED;
        for node in self.hero.lives.iter_mut().flatten().flat_map(|l| l.nodes.iter_mut()) {
            node.life *= GOD_LIFE;
            node.max *= GOD_LIFE;
        }
    }

    /// Rounds leaving their barrels: each round, and its load group's flight effects.
    fn launch(&mut self, launched: Vec<Launch>, now: f64) {
        for l in launched {
            // The hero's own guns launch with no owner.
            let ratio = if self.god && l.owner.is_none() { GOD_DAMAGE } else { 1.0 };
            if let Some(id) =
                self.battle.combat.fire(l.kind, l.owner, l.muzzle, l.direction, l.velocity, ratio, l.target)
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

    /// A placed object's faces are in the world while its target is alive (docs/26), and a
    /// building made in play's once its controller has placed it.
    fn refresh_present(&mut self) {
        for (i, target) in self.battle.combat.targets.iter().enumerate() {
            let placed = !self.construction.unplaced.contains_key(&i);
            if let Some(s) = self.ground.solids.get_mut(i) {
                s.present = target.alive && placed;
            }
        }
    }

    /// The collision pass for the hero (docs/24, "Collision between objects"), after its
    /// move and ground contact.
    fn collide(&mut self, from: Vec3) {
        let lives = &self.hero.lives;
        let mass = self.hero.heft.mass(|p, n| node_share(lives.get(p).and_then(Option::as_ref), n));
        let push = collision_push(
            &self.ground.solids,
            from,
            self.hero.collision_centre(),
            self.hero.collision.1,
            None,
            mass,
            self.hero.walker.keeps_floors(),
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

    /// What the clan areal map caches for every object it holds, refreshed as the game
    /// refreshes a contact record (`ArealMap.dll:0x10006e40`, docs/15, "What a strength is"):
    /// the strength on the life its nodes have left and the one on the life they could have,
    /// its size class, its live top speed and the order it is running.
    ///
    /// STAND-IN: docs/15-behaviour.md#what-a-strength-is--read-and-measured -- the gun total
    /// is `MBehaviour`'s own, `sum(a ÷ b × rounds)` over each gun's rounds left and two
    /// authored figures at the row's `+0x0c` and `+0x28` that are **not read**. The engine
    /// divides the rounds left by the gun's interval in seconds, the two figures the game's
    /// own refresh fills the row with. That the pair is a *rate* is what the scale demands,
    /// not a guess about which fields: unarmed, every object in the install prices between
    /// 0.06 and 0.53, and the scripts keep a strength in a `DWORD` — so every comparison one
    /// reaches (`fn38(clan) > 0` gating the whole capture plan, `dTemp3 < dPlaceProtectHits`
    /// over the authored 10 to 500, the `TAKE_BY_HITS` amounts of 25) would read 0 and the
    /// clan would never plan at all. With the rate in, an armed warbot prices in the tens,
    /// which is the range those numbers are written for
    /// (docs/15, "What a `TAKE_BY_HITS` amount is worth").
    fn refresh_contacts(&mut self) {
        let mut contacts: Vec<(i32, parkan_sim::progression::Contact)> = Vec::new();
        for (t, unit) in self.units.iter().enumerate() {
            if unit.logical_id < 0 {
                continue;
            }
            let (left, full) = self.battle.combat.targets.get(t).map_or((0.0, 0.0), |x| {
                x.parts
                    .iter()
                    .filter_map(|p| p.life.as_ref())
                    .fold((0.0, 0.0), |(l, f), life| (l + life.total(), f + life.full()))
            });
            let robot = self.robots.iter().chain(&self.emplacements).find(|(rt, _)| *rt == t);
            let factory = self.factories.iter().find(|f| f.target == t);
            let guns: f32 = robot.map_or(0.0, |(_, r)| {
                r.guns
                    .iter()
                    .filter(|g| !g.broken && g.interval_ms > 0.0)
                    // An unlimited magazine (−1) never runs down; it counts one round's rate.
                    .map(|g| {
                        let held = if g.magazine < 0 { 1.0 } else { g.rounds.max(0) as f32 };
                        held * 1000.0 / g.interval_ms
                    })
                    .sum()
            });
            contacts.push((
                unit.logical_id,
                parkan_sim::progression::Contact {
                    strength: parkan_sim::progression::strength(guns, left),
                    full: parkan_sim::progression::strength(guns, full),
                    size_class: robot.map_or(0, |(_, r)| r.size_class),
                    speed: robot.map_or(0.0, |(_, r)| r.walker.limits.top_speed[1]),
                    order: robot
                        .and_then(|(_, r)| r.order.map(|o| o.code))
                        .or_else(|| factory.filter(|f| !f.idle()).map(|_| orders::CONSTRUCT)),
                },
            ));
        }
        let clans: Vec<i64> = (0..self.clans.len() as i64).collect();
        let free: Vec<(usize, usize)> = clans
            .iter()
            .map(|&c| (self.free_minds(c), self.reserved.get(&c).copied().unwrap_or(0)))
            .collect();
        let spare: Vec<f32> =
            clans.iter().map(|c| self.economy.power.get(c).map_or(0.0, |&(out, lack)| out - lack)).collect();
        let Some(p) = self.progression.as_mut() else { return };
        for (id, contact) in contacts {
            p.progress.refresh(id, contact);
        }
        for ((clan, (free, reserved)), spare) in clans.into_iter().zip(free).zip(spare) {
            p.set_free_minds(clan, free, reserved);
            p.set_power(clan, spare);
        }
    }

    /// The progression's takts and handler, and what they say.
    fn progress(&mut self) {
        if self.progression.is_none() {
            return;
        }
        self.refresh_contacts();
        let mut at: HashMap<i32, Vec3> = self
            .units
            .iter()
            .zip(&self.battle.combat.targets)
            .map(|(u, t)| (u.logical_id, t.position))
            .collect();
        // Aboard, the hero is reported from where the game frame puts its object: the boarded
        // bot's place less its node sphere's radius in y (docs/39, "What becomes of the hero").
        at.insert(self.hero_id, self.hero_place());
        let now = self.hero.time_ms;
        let ids = crate::progress::next_ids(self.units.iter().map(|u| u.logical_id).chain([self.hero_id]));
        let Some(p) = self.progression.as_mut() else { return };
        p.next_ids = ids;
        let notices = p.tick(now, |id| at.get(&id).copied());
        // A clan's takt freed its reservations (`ai.dll:0x10006580`).
        for clan in std::mem::take(&mut p.swept) {
            self.reserved.remove(&clan);
        }
        let Some(p) = self.progression.as_ref() else { return };
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
                        // A footing's faces are in no mesh, so they name no surface and the
                        // explosion stands on end, as it does on level ground.
                        let hit = face.and_then(|s| s.triangle);
                        let surface = hit.and_then(|t| self.surface(t));
                        let normal =
                            hit.map_or(Vec3::Z, |t| Vec3::from_array(self.ground.land.faces[t].normal));
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
            Event::Staged { target, part, node } if *target == self.battle.combat.hero_index() => {
                self.hero_node_blast(*part, *node, now);
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
    /// The game tests **once per manager tick** -- the interval its next-test time is
    /// built from, `Effect.dll:0x10026a7c`, is 0 and nothing in the module writes it --
    /// which is what a test per frame here comes to; and the ray is the **sight ray's**
    /// query, which excludes no face class, so a lake surface hides what is behind it
    /// (docs/11, "How often the point is tested, and what the ray meets").
    pub fn sprites(&self, eye: Vec3) -> Vec<(usize, Sprite)> {
        self.fx.sprites(self.hero.time_ms, |point| self.battle.combat.clear_line(&self.ground, eye, point))
    }
}

/// Whether node `node` of a part with this life still has life; a part that takes no
/// damage always does.
pub fn node_alive(life: Option<&parkan_sim::damage::Life>, node: usize) -> bool {
    life.and_then(|l| l.nodes.get(node)).is_none_or(|l| !l.destroyed)
}

/// The yaw the hero leaves a bot at `bot` with, put out at `place` (`iron3d.dll:0x10063699`,
/// docs/39, "Leaving"). With F the unit vector from the place to the bot across the ground, the
/// game writes the rows (−F × z, place x), (−F, place y), (z, place z) and hands the matrix to the
/// hero's control system as it is (`Control.dll:0x10004690`) and to its object as its world
/// matrix (`IGameObject` slot 7, kind 2). A unit's forward axis is that matrix's y column, as
/// the own panel's camera reads it (`0x1004186f`), which is (F.x, −F.y): the hero faces the bot
/// from the places due +x and −x of it, faces away from those due +y and −y, and stands
/// side-on at the four between.
pub fn leave_yaw(place: Vec3, bot: Vec3) -> f32 {
    let f = (bot - place).with_z(0.0).normalize_or(Vec3::Y);
    // The engine's yaw turns +y to (−sin, cos): facing (F.x, −F.y).
    (-f.x).atan2(-f.y)
}

/// The component test's turret half (`iron3d.dll:0x10076d30`): `robot`, which is `target` in
/// the battle, has a class-1 component and the node it names has life left — its property
/// `0x52`, that node's life over its maximum, above 0 (docs/39, "Boarding").
fn turret_alive(robot: &Robot, target: &Target) -> bool {
    robot.turret_life().is_some_and(|(p, n)| node_alive(target.parts.get(p).and_then(|x| x.life.as_ref()), n))
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
    robot.rig.traced = None;
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
            flags: 0,
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
            face_two_sided: Vec::new(),
            face_flags: vec![0, 0],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
            corners: None,
        };
        Part {
            mesh: Rc::new(mesh),
            nodes: vec![IDENTITY],
            scale: 1.0,
            life: None,
            portals: Rc::default(),
            host: None,
        }
    }

    /// `iron3d.dll:0x10063699`: the hero put out of a bot faces (F.x, −F.y), F the unit vector
    /// from its place to the bot: towards the bot from the places due ±x of it, away from it at
    /// those due ±y, and side-on between.
    #[test]
    fn the_hero_put_out_of_a_bot_faces_the_mirror_of_the_bot_about_x() {
        let bot = Vec3::new(100.0, 200.0, 160.0);
        let facing = |i: usize| {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            let place = bot + Vec3::new(a.cos(), a.sin(), 0.0) * 13.43;
            let body = parkan_sim::motion::Body::new(place, leave_yaw(place, bot));
            let toward = (bot - place).with_z(0.0).normalize();
            (body.forward(), toward)
        };
        for i in [0, 4] {
            let (forward, toward) = facing(i);
            assert!(forward.distance(toward) < 1e-5, "place {i}: {forward} faces the bot {toward}");
        }
        for i in [2, 6] {
            let (forward, toward) = facing(i);
            assert!(forward.distance(-toward) < 1e-5, "place {i}: {forward} faces away from {toward}");
        }
        for i in [1, 3, 5, 7] {
            let (forward, toward) = facing(i);
            assert!(forward.dot(toward).abs() < 1e-5, "place {i}: {forward} side-on to {toward}");
            assert!((forward - Vec3::new(toward.x, -toward.y, 0.0)).length() < 1e-5);
        }
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

    /// `Control.dll:0x1001e05f`, `0x1001e0db`: a pair with a contact record on both sides
    /// shares its push by the squares of the two masses, and one with a record on one side
    /// takes it whole. Mission 01's hero weighs 3,300 kg, its enemy `tut1_e1` 3,239 and the
    /// neutral flyer `tut1_mf1` 25,699 (*measured*, docs/24).
    #[test]
    fn a_pair_of_units_shares_its_push_by_mass_squared_and_a_tree_gives_it_whole() {
        assert_eq!(share(0.0, 3300.0), 1.0, "a tree, a stone or a building carries no mass");
        assert_eq!(share(3300.0, 0.0), 1.0, "and neither does what has no contact record");
        let even = share(3239.0, 3300.0);
        assert!((even - 0.4907).abs() < 5e-4, "{even}");
        let shoved = share(25699.0, 3300.0);
        assert!((shoved - 0.9835).abs() < 5e-4, "the hero gives way to the flyer: {shoved}");
        let held = share(3300.0, 25699.0);
        assert!((held - 0.0165).abs() < 5e-4, "and the flyer barely moves: {held}");
        assert!((shoved + held - 1.0).abs() < 1e-6, "the two shares are one push");
    }
}
