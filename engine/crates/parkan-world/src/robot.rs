//! A robot: its chassis walking, its turret aiming, its guns and its senses. The hero is
//! one, driven by the player's input; a friendly warbot is one driven by its orders.
//!
//! The chassis's controller drives a [`Walker`], and the turret's controller and control
//! points give its channels, its guns and, where it has one, its camera. See
//! `docs/24-motion.md`, `docs/29-weapons.md` and `docs/30-turrets.md`.

use std::rc::Rc;

use anyhow::{Context, Result};
use glam::{Quat, Vec3};
use parkan_formats::control::{
    self, CAMERA_TYPE, CHANNEL_UNDRIVEN, Channel, Component, Controller, EFFICIENCY_TYPE, ENGINE_TYPE,
    GUN_TYPE, MAST_TYPE, RADAR_PERIOD, RADAR_RANGE, RADAR_TYPE, SIMPLE_TYPE, TRIPLE_TOP_SPEED, TURRET_TYPE,
};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mesh::NO_SLOT;
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, multiply, rotate};
use parkan_sim::behaviour::Behaviour;
use parkan_sim::damage::GroundDamage;
use parkan_sim::device::{Item, Motion};
use parkan_sim::ground::Ground;
use parkan_sim::guns::{Gun, Shot, Sight, TargetGate};
use parkan_sim::machine::Walker;
use parkan_sim::motion::Limits;
use parkan_sim::targeting::Radar;
use parkan_sim::turret::{ARM_FOLD, ARM_UNFOLD, Rig, view};
use parkan_sim::wizard::{Wizard, yaw_along};

use crate::assembly::{Assembly, LoadedMesh, Part};
use crate::battle::{Battle, STARTS_SELECTED};

/// A unit without a radar senses 1 m (`Control.dll:0x1000e7fc`); every shipped radar
/// holds a scan 750 ms.
pub const NO_RADAR_RANGE: f32 = 1.0;
pub const NO_RADAR_PERIOD_MS: f32 = 750.0;
/// Machine steps one tick runs one at a time before it hands the rest to the machine.
const MAX_STEPS: usize = 2000;

/// Where the camera stands and looks, in game space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eye {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    /// The camera's values 2 and 0: the horizontal field of view, and the near plane.
    pub fov_x: f32,
    pub near: f32,
}

/// One external part of a robot: its mesh, the part it hangs from (or -1) and the node of
/// that part it hangs on.
#[derive(Clone)]
pub struct RobotPart {
    /// The `objects.rlb` record the part is built from.
    pub record: String,
    pub mesh: Rc<LoadedMesh>,
    pub host: i32,
    pub node: i32,
}

/// A gun fitted as a part of its own (`e_gun_*` on a turret socket, docs/29): the part,
/// and its controller's channels and control points, which its barrels name.
#[derive(Clone, Debug)]
pub struct GunPart {
    pub part: usize,
    pub channels: Vec<Channel>,
    pub points: Vec<ControlPoint>,
}

/// The profile whose chassis flies: `CanFly` without `WalkChassis` is on it alone of the
/// six (docs/24, "How the AI drives a machine").
pub const FLYING_PROFILE: &str = "chas_fly.var";

/// `.ndp` flags marking a node the left and the right running gear (docs/24, "Running gear").
pub const GEAR_LEFT: i32 = 0x20;
pub const GEAR_RIGHT: i32 = 0x40;
/// The ground's speed factor G: 1.0 on every shipped surface (docs/24, *measured*).
pub const GROUND_FACTOR: f32 = 1.0;

/// One node the machine's weight counts: which part and node, its density × level-0 volume,
/// its area, its `.ndp` flags, and whether it came with the root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeftNode {
    pub part: usize,
    pub node: usize,
    pub weight: f32,
    pub area: f32,
    pub flags: i32,
    pub root: bool,
}

/// What sets a machine's live limits (`Control.dll:0x1000fca0`, docs/24, "What sets the live
/// limits" and "Load"): its engines, its running gear and what it weighs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heft {
    /// The root's authored payload (file +124), kg.
    pub payload: f32,
    /// Every merged node: the root's all, another part's all but its node 0 (docs/28).
    pub nodes: Vec<HeftNode>,
    /// Armour's weight over area: the fitted armour's value 0.
    pub per_area: f32,
    /// Every device's mass (record +0x1c), the fitted parts' in their slots.
    pub devices: f32,
    /// Each engine's value 0 and the root node it sits on.
    pub engines: Vec<(f32, usize)>,
}

impl Heft {
    /// What the unit at `path` weighs, with `parts` its external parts in the robot's order and
    /// `root` the chassis among them.
    pub fn weigh(assembly: &mut Assembly, path: &str, parts: &[RobotPart], root: usize) -> Heft {
        let Some(unit) = assembly.unit(path) else { return Heft::default() };
        let mut weigher = crate::designs::Weigher::default();
        let Some(assembled) = weigher.assemble(assembly, &unit.components) else { return Heft::default() };
        let mut nodes = Vec::new();
        for (p, part) in parts.iter().enumerate() {
            let masses = weigher.node_masses(assembly, &part.record, p == root);
            for (n, m) in masses.iter().enumerate().filter(|(n, _)| p == root || *n != 0) {
                nodes.push(HeftNode {
                    part: p,
                    node: n,
                    weight: m.density * m.volume,
                    area: m.area,
                    flags: m.flags,
                    root: p == root,
                });
            }
        }
        Heft {
            payload: assembled.payload,
            nodes,
            per_area: assembled.of_type(crate::designs::ARMOUR_TYPE).last().map_or(0.0, |d| d.values[0]),
            devices: assembled.devices.iter().map(|d| d.mass).sum(),
            engines: assembled
                .of_type(ENGINE_TYPE)
                .map(|d| (d.values[0], usize::try_from(d.node).unwrap_or(0)))
                .collect(),
        }
    }

    /// E and r, with `life(part, node)` a node's life over its maximum and whether it is gone.
    ///
    /// E is Σ engines' value 0 × their node's condition, times the mean of the two sides'
    /// running gear, each side its nodes' mean life, 1 with none (`0x10012a40`). r is the spare
    /// payload over the payload: payload + the root's body − everything's weight, never below 0
    /// (`0x1000fc51`), 0 with no payload.
    ///
    /// STAND-IN: docs/24-motion.md#what-sets-the-live-limits--read -- a node that "reaches its
    /// last damage stage" (`0x10011920`) is taken as one destroyed, and what leaves the totals
    /// with it is its own weight and armour; the devices on it stay.
    pub fn factors(&self, root: usize, life: impl Fn(usize, usize) -> (f32, bool)) -> (f32, f32) {
        let side = |mask: i32| {
            let lives: Vec<f32> =
                self.nodes.iter().filter(|n| n.flags & mask != 0).map(|n| life(n.part, n.node).0).collect();
            if lives.is_empty() { 1.0 } else { lives.iter().sum::<f32>() / lives.len() as f32 }
        };
        let drive: f32 = self.engines.iter().map(|&(value, node)| value * life(root, node).0).sum();
        let e = drive * (side(GEAR_LEFT) + side(GEAR_RIGHT)) / 2.0;
        let (total, body) = self.weight(&life);
        let r = if self.payload != 0.0 { (self.payload + body - total).max(0.0) / self.payload } else { 0.0 };
        (e, r)
    }

    /// What the machine weighs, kg, and what of that is the root's own body: every live node's
    /// density x volume, armour's weight over its area, and every device's mass
    /// (`Control.dll:0x1000fac0`).
    fn weight(&self, life: impl Fn(usize, usize) -> (f32, bool)) -> (f32, f32) {
        let (mut total, mut body) = (self.devices, 0.0);
        for n in self.nodes.iter().filter(|n| !life(n.part, n.node).1) {
            total += n.weight + self.per_area * n.area;
            if n.root {
                body += n.weight;
            }
        }
        (total, body)
    }

    /// The machine's mass, kg: control `+0x538`, which the weigh sums and property `0x7c`
    /// hands out (`Control.dll:0x1000e0ac`). A colliding pair shares its push by the squares
    /// of the two (docs/24, "Collision between objects").
    pub fn mass(&self, life: impl Fn(usize, usize) -> (f32, bool)) -> f32 {
        self.weight(life).0
    }
}

pub struct Robot {
    /// The mission object the robot is.
    pub object: usize,
    /// Every external part with a mesh, as the assembly lists them; the chassis and the
    /// turret among them.
    pub parts: Vec<RobotPart>,
    pub chassis_part: usize,
    pub turret_part: usize,
    /// The size class its chassis's name gives (docs/31): t 1, l and h 2, m 3, b 4, else 0.
    pub size_class: u8,
    /// The order it was last given, which its behaviour carries out.
    pub order: Option<parkan_sim::orders::Order>,
    pub walker: Walker,
    pub rig: Rig,
    pub chassis: Rc<LoadedMesh>,
    pub turret: Rc<LoadedMesh>,
    /// The chassis node the turret hangs from.
    pub socket: usize,
    /// The camera's position and direction points, where the turret has a camera.
    pub camera: Option<(ControlPoint, ControlPoint)>,
    /// The turret's controller and control points.
    pub turret_controller: Controller,
    pub points: Vec<ControlPoint>,
    /// The turret's guns, then the guns fitted as parts; the round kind each fires in its
    /// `Battle`; and for a fitted gun, its part.
    pub guns: Vec<Gun>,
    pub rounds: Vec<Option<usize>>,
    pub gun_parts: Vec<Option<GunPart>>,
    /// Each gun's node, as a part and a node of it: whose life decides whether it fires.
    pub gun_nodes: Vec<Option<(usize, usize)>>,
    /// Whether its chassis flies: its movement points keep every axis.
    pub flyer: bool,
    /// The points it follows, and what it does with its orders, when the AI drives it.
    pub wizard: Wizard,
    pub behaviour: Behaviour,
    /// When each gun may next fire on the AI's timer, and the target its guns were given.
    pub next_shot_ms: Vec<f64>,
    pub fire_target: Option<usize>,
    /// The unit's one radar: its fitted radar part's, else its turret's (docs/25).
    pub radar: Radar,
    /// The radar slot's node, as a part and a node of it, which a fitted radar keeps (docs/28,
    /// "A fitted part takes over its slot").
    pub radar_node: Option<(usize, usize)>,
    /// A unit's batteries and what draws on them; `None` on a building, and on a unit with no
    /// battery.
    pub power: Option<crate::power::Power>,
    /// The collision sphere in the unit's frame: its centre and radius.
    pub collision: (Vec3, f32),
    /// The node sphere in the unit's frame ([`node_sphere`]): the centre the ground contact
    /// holds the body about, and the radius getting out of a bot reaches by.
    pub bound: (Vec3, f32),
    /// Where the guns' target stands, for their gates; the caller sets it each tick.
    pub target_point: Option<Vec3>,
    /// Whether the turret, and the eye in it, are held steady against the body's gait
    /// yaw ([`Robot::chassis_pose`]). Off, the view swings as the game's does.
    pub steady: bool,
    /// The machine's velocity over its last step, from the two poses either side.
    velocity: Vec3,
    /// Game time, ms.
    pub time_ms: f64,
    /// What the ground under it deals it, and when its life next updates.
    pub ground_damage: GroundDamage,
    /// The parts that move by themselves: the chassis controller's generic devices and
    /// radars, which pose chassis nodes, and the turret's, which drive its channels
    /// (docs/28-chassis.md, "What a device's value turns").
    pub chassis_devices: Vec<Item>,
    pub turret_devices: Vec<Item>,
    /// What sets its live limits.
    pub heft: Heft,
    /// Its own camera's zoom (docs/30, "The zoom").
    pub zoom: crate::camera::Zoom,
}

/// A controller's items that drive channels and that nothing else steers: its generic
/// devices and its radars, and — on a building that carries guns — the efficiency and mast
/// classes a warbot never holds ([`crate::buildings::running_items`], which steps the same
/// records for a building with no guns at all).
pub fn devices(controller: &Controller) -> Vec<Item> {
    controller
        .components
        .iter()
        .enumerate()
        .filter(|(_, k)| {
            matches!(k.type_id, SIMPLE_TYPE | RADAR_TYPE | EFFICIENCY_TYPE | MAST_TYPE)
                && !k.entries.is_empty()
        })
        .map(|(i, k)| crate::buildings::started(Item::new(i, k, &controller.channels)))
        .collect()
}

/// Spheres joined as `AniMesh.dll:0x10009510` joins them: the centres weighted by the radii,
/// and a radius reaching the farthest sphere.
fn join_spheres(spheres: &[(Vec3, f32)]) -> (Vec3, f32) {
    let weight: f32 = spheres.iter().map(|s| s.1).sum();
    let centre =
        if weight > 0.0 { spheres.iter().map(|s| s.0 * s.1).sum::<Vec3>() / weight } else { Vec3::ZERO };
    (centre, spheres.iter().map(|s| s.0.distance(centre) + s.1).fold(0.0, f32::max))
}

/// The agent's node sphere, which interface `0x20` slot 3 hands out by default
/// (`AniMesh.dll:0x1000f5c8`, worked out at `0x10009e0a`; docs/24, "Finding the ground"): over
/// the merged model's exterior nodes, each node's level-0 slot box through the node's matrix
/// as a sphere about the box's diagonal, joined as the parts' spheres are. The chassis's node
/// 0 answers as the whole object and adds nothing; a node with no level-0 slot is a point at
/// its origin.
///
/// STAND-IN: docs/24-motion.md#finding-the-ground--read -- not read: when `0x10009510` works
/// the agent's and node spheres out again, and at which pose. Both are worked out once, at rest.
pub fn node_sphere(assembly: &mut Assembly, parts: &[Part]) -> (Vec3, f32) {
    let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
    let mut spheres = Vec::new();
    for part in parts {
        let Some(mesh) = assembly.mesh(&part.reference) else { continue };
        for (n, node) in mesh.mesh.nodes.iter().enumerate().skip(1) {
            if node.is_interior() {
                continue;
            }
            let at = part.pose.compose(&mesh.mesh.world_pose(n));
            let slot = node.slot_index[0];
            let sphere = match mesh.mesh.slots.get(usize::from(slot)).filter(|_| slot != NO_SLOT) {
                None => (f(at.translation), 0.0),
                Some(s) => {
                    let (lo, hi) = (Vec3::from_array(s.aabb_min), Vec3::from_array(s.aabb_max));
                    (f(at.apply(((lo + hi) * 0.5).to_array().map(f64::from))), (hi - lo).length() * 0.5)
                }
            };
            spheres.push(sphere);
        }
    }
    join_spheres(&spheres)
}

fn read_member(assembly: &mut Assembly, library: &str, member: &str) -> Result<Vec<u8>> {
    let archive = assembly.archive(library).with_context(|| format!("no archive {library}"))?;
    Ok(archive.read_name(member)?.to_vec())
}

pub(crate) fn controller(assembly: &mut Assembly, record: &str) -> Result<Option<Controller>> {
    // A building's FORT record names its controller through a mesh-less slot, as its mesh.
    let Some(slot) = assembly.library.record_slot(assembly.library.get(record), "ctl", 0) else {
        return Ok(None);
    };
    let data = read_member(assembly, &slot.library, &slot.member)?;
    Ok(Some(control::parse(&data, &slot.member)?))
}

/// A chassis record's size class: `R_H_02`'s letter after `R_` (`Behavior.dll:0x1000cee0`), t
/// 1, l or h 2, m 3, b 4, and 0 for anything else.
pub fn chassis_size(record: &str) -> u8 {
    match record.as_bytes().get(2).map(u8::to_ascii_lowercase) {
        Some(b't') => 1,
        Some(b'l' | b'h') => 2,
        Some(b'm') => 3,
        Some(b'b') => 4,
        _ => 0,
    }
}

/// What a unit is and carries, as the HUD shows it (docs/35-hud.md, "The target panel and the
/// player's own unit").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Designation {
    /// 1–4, or 0 unknown.
    pub size_class: u8,
    /// Its chassis profile's `ChassisType`: 1 flying, 2 walking, 3 wheeled, 4 tracked; 0
    /// unknown.
    pub chassis_type: u8,
    /// A battery with a capacity.
    pub battery: bool,
    /// Both a fight shield and a deflector, without which no sector is drawn.
    pub shielded: bool,
    pub repair: bool,
    pub detection_shield: bool,
}

/// A placed object's designation: its size class and chassis type from its chassis part, the
/// one hanging from nothing, and its equipment from every part's controller. `profiles` is
/// `behpsp.res`.
pub fn designation(
    assembly: &mut Assembly,
    profiles: Option<&parkan_formats::nres::Archive>,
    kind: u32,
    path: &str,
) -> Designation {
    let mut out = Designation::default();
    let mut deflector = false;
    for record in assembly.records(path) {
        let Some(c) = controller(assembly, &record).ok().flatten() else { continue };
        for k in &c.components {
            match k.type_id {
                control::BATTERY_TYPE if k.values[control::BATTERY_CAPACITY] > 0.0 => out.battery = true,
                control::FIGHT_SHIELD_TYPE => out.shielded = true,
                control::REPAIR_TYPE => out.repair = true,
                control::DETECT_SHIELD_TYPE => out.detection_shield = true,
                control::DEFLECTOR_TYPE => deflector = true,
                _ => {}
            }
        }
    }
    out.shielded &= deflector;
    let parts = assembly.parts(kind, path);
    let Some(chassis) = parts.iter().find(|p| p.host == -1) else { return out };
    let chassis_type = assembly
        .library
        .get(&chassis.record)
        .and_then(|r| r.slot_with_suffix("var"))
        .and_then(|slot| profiles?.read_name(&slot.member).ok())
        .and_then(|data| parkan_formats::profiles::parse(data, "var").ok())
        .and_then(|vars| vars.into_iter().find(|v| v.name == parkan_formats::profiles::CHASSIS_TYPE))
        .map_or(0, |v| v.value.clamp(0.0, 255.0) as u8);
    Designation { size_class: chassis_size(&chassis.record), chassis_type, ..out }
}

/// `pose` with its turn about z, measured from `rest`, taken out: the swing that is left
/// of rest⁻¹ × pose once its twist about z is removed, applied to `rest`.
fn without_yaw(pose: &Pose, rest: &Pose) -> Pose {
    let [w, x, y, z] = rest.rotation;
    let relative = multiply([w, -x, -y, -z], pose.rotation);
    let length = relative[0].hypot(relative[3]);
    if length < 1e-9 {
        return *pose;
    }
    let untwist = [relative[0] / length, 0.0, 0.0, -relative[3] / length];
    Pose { rotation: multiply(rest.rotation, multiply(relative, untwist)), ..*pose }
}

impl Robot {
    /// Mission object `object` as a robot, if its chassis has a controller and one of the
    /// parts on it is a turret: a part whose controller has a camera (the hero's) or a
    /// turret component.
    pub fn load(assembly: &mut Assembly, mission: &Mission, object: usize) -> Result<Option<Robot>> {
        let Some(placed) = mission.objects.get(object) else { return Ok(None) };
        Self::load_placed(assembly, placed, object)
    }

    /// Placed object `placed`, numbered `object`, as a robot.
    pub fn load_placed(
        assembly: &mut Assembly,
        placed: &parkan_formats::mission::Object,
        object: usize,
    ) -> Result<Option<Robot>> {
        let parts: Vec<Part> = assembly.parts(placed.kind, &placed.path);
        let Some(chassis_part) = parts.iter().find(|p| p.host == -1).cloned() else { return Ok(None) };
        let Some(chassis) = assembly.mesh(&chassis_part.reference) else { return Ok(None) };
        let chassis_ctl = match controller(assembly, &chassis_part.record)? {
            Some(c) => c,
            // A building's frame is no machine: it stands on one still state, and its own
            // controller's doors and pod are the building's (docs/24, "Walking into a
            // building"). What it carries on a turret aims and fires as a unit's does.
            None if placed.kind == parkan_formats::mission::KIND_BUILDING => Controller {
                states: vec![control::State::default()],
                costs: vec![0.0],
                ..Controller::default()
            },
            None => return Ok(None),
        };

        let mut turret = None;
        for part in parts.iter().filter(|p| p.host == 0) {
            if let Some(c) = controller(assembly, &part.record)? {
                let camera = c.components.iter().any(|k| k.type_id == CAMERA_TYPE);
                let aims = c.components.iter().any(|k| k.type_id == TURRET_TYPE);
                if camera || (aims && turret.is_none()) {
                    turret = Some((part.clone(), c));
                    if camera {
                        break;
                    }
                }
            }
        }
        // An animal carries no turret part: its one controller holds the turret and the gun,
        // on its own mesh (docs/34, "The medusas").
        //
        // STAND-IN: docs/34-progression.md#the-medusas--read-and-measured -- how a turret component on the chassis's
        // own controller poses its nodes against the chassis's frames is not read: the turret
        // channels pose the mesh as a turret part's would, about the chassis's root.
        let turret = turret.or_else(|| {
            chassis_ctl
                .components
                .iter()
                .any(|k| k.type_id == TURRET_TYPE)
                .then(|| (chassis_part.clone(), chassis_ctl.clone()))
        });
        let Some((turret_part, turret_ctl)) = turret else { return Ok(None) };
        let mut robot_parts = Vec::new();
        let (mut chassis_index, mut turret_index) = (0, 0);
        for part in &parts {
            let Some(mesh) = assembly.mesh(&part.reference) else { continue };
            if part.host == -1 && part.record == chassis_part.record {
                chassis_index = robot_parts.len();
            }
            if part.record == turret_part.record
                && part.host == turret_part.host
                && part.node == turret_part.node
            {
                turret_index = robot_parts.len();
            }
            robot_parts.push(RobotPart {
                record: part.record.clone(),
                mesh,
                host: part.host,
                node: part.node,
            });
        }
        let turret_mesh = assembly.mesh(&turret_part.reference).context("the turret mesh does not load")?;
        let points = match assembly
            .library
            .get(&turret_part.record)
            .and_then(|r| r.slot_with_suffix("cpt"))
            .cloned()
        {
            Some(slot) => cpt::parse(&read_member(assembly, &slot.library, &slot.member)?, "cpt")?,
            None => Vec::new(),
        };
        let rig = Rig::new(&turret_ctl);
        let point = |i: i32| usize::try_from(i).ok().and_then(|i| points.get(i)).cloned();
        let camera =
            rig.camera.map(|c| rig.channels[c]).and_then(|c| Some((point(c.origin)?, point(c.point)?)));

        // The chassis's control points place its contacts: the feet (docs/24).
        let feet_slot =
            assembly.library.get(&chassis_part.record).and_then(|r| r.slot_with_suffix("cpt")).cloned();
        let feet = match feet_slot {
            Some(slot) => cpt::parse(&read_member(assembly, &slot.library, &slot.member)?, &slot.member)?,
            None => Vec::new(),
        };
        let position = Vec3::from_array(placed.position);
        let chassis_devices = devices(&chassis_ctl);
        let turret_devices = devices(&turret_ctl);
        let mut walker = Walker::new(chassis_ctl, &chassis.mesh, &feet, position, placed.rotation);
        let heft = Heft::weigh(assembly, &placed.path, &robot_parts, chassis_index);
        if placed.kind != parkan_formats::mission::KIND_BUILDING {
            let (e, r) = heft.factors(chassis_index, |_, _| (1.0, false));
            walker.limits = Limits::live(&walker.controller, e, r, GROUND_FACTOR);
        }

        // Only one radar counts: a fitted radar part takes over the turret's radar slot
        // (docs/25-sensors.md, "A scan is a sphere, a falloff and three tests").
        let mut radar = None;
        for record in assembly.records(&placed.path) {
            let Some(c) = controller(assembly, &record).ok().flatten() else { continue };
            if let Some(r) = c.components.iter().find(|k| k.type_id == RADAR_TYPE)
                && (radar.is_none() || !record.eq_ignore_ascii_case(&turret_part.record))
            {
                radar = Some(Radar::new(r.values[RADAR_RANGE], r.values[RADAR_PERIOD]));
            }
        }
        let radar = radar.unwrap_or_else(|| Radar::new(NO_RADAR_RANGE, NO_RADAR_PERIOD_MS));
        let slot = |c: &Controller| c.components.iter().find(|k| k.type_id == RADAR_TYPE).map(|k| k.node);
        let radar_node = match slot(&turret_ctl) {
            Some(n) => usize::try_from(n).ok().map(|n| (turret_index, n)),
            None => {
                slot(&walker.controller).and_then(|n| usize::try_from(n).ok()).map(|n| (chassis_index, n))
            }
        };

        // The agent's sphere from its parts' header spheres (`AniMesh.dll:0x10009510`,
        // docs/26): every part's, the chassis's, the turret's and each gun's, their centres
        // weighted by their radii, and a radius reaching the farthest part's sphere. A part's is
        // carried by its mount at rest. The ground contact holds the body by this sphere's
        // radius (`Control.dll:0x1001a487`) about the node sphere's centre (`0x1001a518`,
        // docs/24, "Finding the ground").
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let spheres: Vec<(Vec3, f32)> = parts
            .iter()
            .filter_map(|p| {
                let (c, r) = assembly.mesh(&p.reference)?.mesh.sphere?;
                Some((f(p.pose.apply(c.map(f64::from))), r))
            })
            .collect();
        let collision = join_spheres(&spheres);
        let bound = node_sphere(assembly, &parts);
        if !spheres.is_empty() {
            walker.set_body_sphere(bound.0, collision.1, bound.1);
        }
        let power = (placed.kind == parkan_formats::mission::KIND_UNIT)
            .then(|| crate::power::Power::load(assembly, placed.kind, &placed.path))
            .flatten();
        let size_class = chassis_size(&chassis_part.record);
        let flyer = assembly
            .library
            .get(&chassis_part.record)
            .and_then(|r| r.slot_with_suffix("var"))
            .is_some_and(|v| v.member.eq_ignore_ascii_case(FLYING_PROFILE));
        Ok(Some(Robot {
            object,
            flyer,
            wizard: Wizard::default(),
            behaviour: Behaviour::new((object as u32).wrapping_mul(2_654_435_761)),
            next_shot_ms: Vec::new(),
            fire_target: None,
            gun_parts: Vec::new(),
            gun_nodes: Vec::new(),
            size_class,
            order: None,
            parts: robot_parts,
            chassis_part: chassis_index,
            turret_part: turret_index,
            walker,
            rig,
            chassis,
            turret: turret_mesh,
            socket: usize::try_from(turret_part.node).unwrap_or(0),
            camera,
            turret_controller: turret_ctl,
            points,
            guns: Vec::new(),
            rounds: Vec::new(),
            radar,
            radar_node,
            power,
            collision,
            bound,
            target_point: None,
            steady: false,
            velocity: Vec3::ZERO,
            time_ms: 0.0,
            // Units apart update their lives apart.
            ground_damage: GroundDamage::new(0.0, (object as u16).wrapping_mul(40_503)),
            chassis_devices,
            turret_devices,
            heft,
            zoom: crate::camera::Zoom::default(),
        }))
    }

    /// Where the boarding test reads the turret's life (see [`turret_life_node`]): a part and
    /// a node of it, `None` for a unit with no class-1 component, which the test refuses.
    pub fn turret_life(&self) -> Option<(usize, usize)> {
        let socket = self
            .parts
            .get(self.turret_part)
            .and_then(|p| Some((usize::try_from(p.host).ok()?, usize::try_from(p.node).ok()?)));
        turret_life_node(
            &self.walker.controller,
            self.chassis_part,
            &self.turret_controller,
            self.turret_part,
            socket,
        )
    }

    /// The part whose research code names gun `i` in the weapons list (docs/35, "The weapons
    /// list"): the device's own part, the id it keeps at `+8` (`Control.dll:0x1002d7a5`) — a gun
    /// fitted as a part of its own, or else the part whose controller carries the gun, the
    /// turret's or an animal's chassis's. A clip fitted to a gun keeps the gun's id, since its
    /// re-parse (`0x1002d890`) leaves `+8` alone.
    pub fn gun_part(&self, i: usize) -> usize {
        match self.gun_parts.get(i) {
            Some(Some(g)) => g.part,
            _ => self.turret_part,
        }
    }

    /// The live limits again (`0x1000fca0`), from each node's life: `life(part, node)` is its
    /// life over its maximum and whether it is gone. The game recomputes them the tick after a
    /// node is damaged and when one reaches or leaves its last stage; the same figures come of
    /// running it every tick.
    pub fn relimit(&mut self, life: impl Fn(usize, usize) -> (f32, bool)) {
        if self.heft.payload == 0.0 && self.heft.engines.is_empty() {
            return;
        }
        let (e, r) = self.heft.factors(self.chassis_part, life);
        self.walker.limits = Limits::live(&self.walker.controller, e, r, GROUND_FACTOR);
    }

    /// Fit the turret's guns: each class-2 component, the round its record names
    /// loaded into `battle`. A gun whose round's frame +116 carries 4 starts selected;
    /// on the hero the rest start deselected (`World3D.dll:0x1000ed20`). The n-th gun's
    /// arm, the n-th class-24 component, is sent `0x21` if its gun starts selected and
    /// `0x22` if not (`0x1000ef4e`, `0x1000ef9e`): the cannon's and the laser's unfold
    /// from folded.
    pub fn arm(&mut self, battle: &mut Battle, assembly: &mut Assembly) {
        self.guns.clear();
        self.rounds.clear();
        self.gun_parts.clear();
        self.gun_nodes.clear();
        let components = self.turret_controller.components.clone();
        for (i, c) in components.iter().enumerate().filter(|(_, c)| c.type_id == GUN_TYPE) {
            let mut gun = Gun::new(i, c, &self.turret_controller.channels);
            let kind = self.fit(&mut gun, c, battle, assembly);
            gun.selected = kind.is_some_and(|k| battle.kinds[k].frame_flags & STARTS_SELECTED != 0);
            if let Some(arm) = self.rig.arms.get_mut(self.guns.len()) {
                arm.send(if gun.selected { ARM_UNFOLD } else { ARM_FOLD });
            }
            self.guns.push(gun);
            self.rounds.push(kind);
            self.gun_parts.push(None);
            self.gun_nodes.push(usize::try_from(c.node).ok().map(|n| (self.turret_part, n)));
        }
        // The guns fitted as parts of their own: each part's controller's class-2 component.
        for p in 0..self.parts.len() {
            if p == self.chassis_part || p == self.turret_part {
                continue;
            }
            let record = self.parts[p].record.clone();
            let Some(ctl) = controller(assembly, &record).ok().flatten() else { continue };
            let points = assembly
                .library
                .get(&record)
                .and_then(|r| r.slot_with_suffix("cpt"))
                .cloned()
                .and_then(|slot| read_member(assembly, &slot.library, &slot.member).ok().map(|b| (b, slot)))
                .and_then(|(b, slot)| cpt::parse(&b, &slot.member).ok())
                .unwrap_or_default();
            for (i, c) in ctl.components.iter().enumerate().filter(|(_, c)| c.type_id == GUN_TYPE) {
                let mut gun = Gun::new(i, c, &ctl.channels);
                let kind = self.fit(&mut gun, c, battle, assembly);
                // A gun fitted as a part is not given a mount here, so it keeps the 1 its
                // constructor sets (`Control.dll:0x100295b3`), which is what a gun with no
                // follower keeps for good; its barrels' recoil is not played. In the game a
                // fitted part's own follower joins the turret's list (`0x10009120`), so all
                // but 19 of the 1034 shipped guns are mounted and lifted, and none of the 19
                // lobs (docs/29, "A gun with no follower is ready from birth"). The lobbed
                // lift a mount would give is applied below.
                gun.ready = true;
                gun.selected = true;
                self.guns.push(gun);
                self.rounds.push(kind);
                self.gun_parts.push(Some(GunPart {
                    part: p,
                    channels: ctl.channels.clone(),
                    points: points.clone(),
                }));
                self.gun_nodes.push(usize::try_from(c.node).ok().map(|n| (p, n)));
            }
        }
        self.next_shot_ms = vec![0.0; self.guns.len()];
    }

    /// A gun's round loaded into `battle`, and what the gun keeps of it: its top speed,
    /// whether it falls (`0x100297ef`), and values 8-10 from it, its range and a guided
    /// round's cone and lock.
    fn fit(
        &self,
        gun: &mut Gun,
        c: &Component,
        battle: &mut Battle,
        assembly: &mut Assembly,
    ) -> Option<usize> {
        let kind = battle.round_kind(assembly, &c.resource.member);
        gun.round_speed = kind.map_or(0.0, |k| battle.combat.kinds[k].top_speed);
        let frame = controller(assembly, &c.resource.member).ok().flatten();
        gun.falls = frame.as_ref().is_some_and(|r| r.mode != 0);
        gun.round_flags = frame.as_ref().map_or(0, |r| r.flags);
        if let Some(k) = kind.map(|k| &battle.combat.kinds[k]) {
            gun.link(TargetGate::new(k.range, k.seeker.map(|s| (s.cone, s.reach, s.lock_ms))));
        }
        kind
    }

    /// Hand the turret's guns `target` (`iron3d.dll:0x10091a80`): the turret is in
    /// `CIS_MANUALCONTROL`, so only its guided guns take it (`Control.dll:0x10028164`), and
    /// every gun's lock starts again.
    pub fn relink(&mut self, target: Option<usize>) {
        for g in &mut self.guns {
            g.relink(if g.gate.guided() { target } else { None });
        }
    }

    /// The first half of a tick: game time moves on, the machine's steps due by then run,
    /// and the turret takes the step's strafe offset.
    pub fn advance(&mut self, dt_ms: f64, ground: &Ground) {
        self.time_ms += dt_ms;
        self.walk(ground);
        self.rig.strafe = self.walker.strafe_offset(self.time_ms);
        // `Control.dll:0x10005b13`: while the turret lock holds, the control takt aims the
        // turret from the lead, so it keeps its heading as the hull comes round under it.
        let body = &self.walker.body;
        if body.turret_lock && !body.turn_pending {
            self.rig.aim[0] = parkan_sim::motion::led_aim(body, self.walker.phase(self.time_ms));
        }
    }

    /// The second half of a tick: the turret's takt (its channels, the arms, and each
    /// mount's gun's ready byte), then the guns. Returns the rounds that left, by gun.
    pub fn takt(&mut self, dt_ms: f64) -> Vec<(usize, Shot)> {
        if self.guns.iter().any(|g| g.falls)
            && let Some(up) = self.rig.yaw.and_then(|c| usize::try_from(self.rig.channels[c].origin).ok())
            && let Some((_, direction)) = self.point(up)
        {
            self.rig.center_up = direction.z;
        }
        self.rig.update((dt_ms / 1000.0) as f32, &mut self.guns);
        // What each gun's gate sees: the unit, its barrel point's direction, its target.
        let sights: Vec<Sight> = (0..self.guns.len())
            .map(|i| Sight {
                unit: self.walker.body.position,
                barrel: self.gun_muzzle(i, self.guns[i].current).map_or(Vec3::Y, |m| m.1),
                target: self.guns[i].target.and(self.target_point),
            })
            .collect();
        let mut shots = Vec::new();
        let traced = self.rig.traced;
        for (i, (g, sight)) in self.guns.iter_mut().zip(sights).enumerate() {
            g.sight = sight;
            if self.gun_parts.get(i).is_some_and(Option::is_some) {
                // A part's gun brings its own follower, so the turret's takt runs the same
                // mount step over it: ready while the point its turret traces lies within the
                // round's lower arc, and its round leaves on that arc
                // (docs/29, "A gun with no follower is ready from birth").
                g.ready = !g.falls
                    || traced.is_none_or(|to| {
                        parkan_sim::turret::lobbed_launch(g.round_speed, parkan_sim::turret::GRAVITY, to)
                            .is_some()
                    });
            }
            if self.power.is_none() {
                g.recharge();
            }
            shots.extend(g.tick(self.time_ms).into_iter().map(|s| (i, s)));
            if self.gun_parts.get(i).is_some_and(Option::is_some) {
                continue;
            }
            for b in &g.barrels {
                if let Some(v) = self.rig.values.get_mut(b.channel) {
                    *v = b.value(self.time_ms);
                }
            }
        }
        shots
    }

    /// The engines' share of their power figure now (`Control.dll:0x100265c0`, docs/24, "The
    /// engine factor is the state's"): the largest component of the machine's velocity over the
    /// largest of its top speed, times the current state's factor, which is 0 on every state of
    /// the hero's chassis.
    pub fn engine_share(&self) -> f32 {
        let w = &self.walker;
        let largest = |v: [f32; 3]| v.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        let top = largest(w.controller.triples[TRIPLE_TOP_SPEED]);
        let factor = w.controller.states.get(w.machine.current).map_or(0.0, |s| s.engine);
        if top > 0.0 { largest(w.body.velocity) / top * factor } else { 0.0 }
    }

    /// The guns and the radar as their nodes stand, `alive` saying whether node `node` of part
    /// `part` still has life: a device whose node is destroyed does nothing (slot 2,
    /// `Control.dll:0x10021820`). A gun starts no stroke (`0x10029cc3`) and the radar answers
    /// no scan (`0x10024390`, docs/25).
    pub fn check_devices(&mut self, alive: impl Fn(usize, usize) -> bool) {
        for (g, node) in self.guns.iter_mut().zip(&self.gun_nodes) {
            g.broken = node.is_some_and(|(p, n)| !alive(p, n));
        }
        self.radar.broken = self.radar_node.is_some_and(|(p, n)| !alive(p, n));
    }

    /// The parts that move by themselves, at the current game time: each item's steps due,
    /// its channels played, and the turret's written into its channel values. `alive`
    /// says whether node `node` of part `part` still has life (slot 2, `0x10021820`).
    ///
    /// The machine's lean is 0: the body does not lean (see [`parkan_sim::motion::Body`]).
    pub fn turn_devices(&mut self, alive: impl Fn(usize, usize) -> bool) {
        let body = &self.walker.body;
        let motion = Motion {
            spin: body.spin,
            lean: [0.0; 3],
            velocity: body.velocity,
            top: self.walker.controller.triples[TRIPLE_TOP_SPEED],
        };
        let node = |c: &Controller, d: &Item| usize::try_from(c.components[d.component].node).ok();
        for d in &mut self.chassis_devices {
            let live = node(&self.walker.controller, d).is_none_or(|n| alive(self.chassis_part, n));
            d.tick(self.time_ms, &motion, live);
        }
        for d in &mut self.turret_devices {
            let live = node(&self.turret_controller, d).is_none_or(|n| alive(self.turret_part, n));
            d.tick(self.time_ms, &motion, live);
            for (&e, &v) in d.entries.iter().zip(&d.now) {
                if let Some(value) = self.rig.values.get_mut(e) {
                    *value = v;
                }
            }
        }
    }

    /// The frame a chassis node plays where one of its devices drives it: the first driven
    /// channel on that node that has frames.
    pub fn device_frame(&self, node: usize) -> Option<f32> {
        self.chassis_devices
            .iter()
            .flat_map(|d| d.channels.iter().zip(&d.now))
            .find(|(c, _)| c.node == node as i32 && c.flags & CHANNEL_UNDRIVEN == 0 && c.first >= 0.0)
            .map(|(c, &v)| c.frame(v))
    }

    /// Where the material of node `node` of part `part` stands, 0..1, when a device plays it
    /// instead of posing the node: the first `CHANNEL_MATERIAL` channel on that node.
    ///
    /// A tracked chassis's belt is the only thing that carries it. The channel spans no
    /// frames and its device is the skid-steering one, so the value runs with the track's
    /// own speed and holds where it is while the unit stands (docs/28-chassis.md, "The belt
    /// is a material a channel plays").
    pub fn material_phase(&self, part: usize, node: usize) -> Option<f32> {
        let devices = if part == self.chassis_part {
            &self.chassis_devices
        } else if part == self.turret_part {
            &self.turret_devices
        } else {
            return None;
        };
        devices
            .iter()
            .flat_map(|d| d.channels.iter().zip(&d.now))
            .find(|(c, _)| c.node == node as i32 && c.flags & control::CHANNEL_MATERIAL != 0)
            .map(|(_, &v)| v)
    }

    /// Run the machine's steps due by now one at a time, handing the camera each step's
    /// jolt: (previous − current velocity) ÷ the step in seconds, the velocity from the
    /// poses either side of the step (`Control.dll:0x1000c6e7`, docs/30, "The camera shake").
    fn walk(&mut self, ground: &Ground) {
        let w = &mut self.walker;
        for _ in 0..MAX_STEPS {
            if w.controller.states.is_empty() || self.time_ms < w.machine.clock_ms {
                break;
            }
            let due = w.machine.clock_ms;
            w.advance(due, ground);
            let step = (w.machine.step_ms / 1000.0) as f32;
            if step > 0.0 {
                let velocity = (w.body.position - w.from.0) / step;
                self.rig.shake.jolt((self.velocity - velocity) / step, w.machine.step_start_ms / 1000.0);
                self.velocity = velocity;
            }
        }
        w.advance(self.time_ms, ground);
    }

    /// The robot's world velocity.
    pub fn world_velocity(&self) -> Vec3 {
        self.walker.body.to_world(Vec3::from_array(self.walker.body.velocity))
    }

    /// A turret control point in the world: its position and direction.
    pub fn point(&self, index: usize) -> Option<(Vec3, Vec3)> {
        let p = self.points.get(index)?;
        let (position, _) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(self.walker.drawn(self.time_ms).1);
        let pose = self.turret_node(&self.mount(), usize::try_from(p.nodes().0).ok()?);
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let at = f(pose.apply(p.position.map(f64::from)));
        let dir = f(rotate(pose.rotation, p.direction.map(f64::from)));
        Some((position + heading * at, heading * dir))
    }

    /// The lowest the turret's sight can look, radians above the horizontal (negative below):
    /// its pitch channel at either end of its span. `None` without a sight or a pitch channel.
    pub fn lowest_sight(&mut self) -> Option<f32> {
        let c = self.rig.pitch?;
        let now = self.rig.values[c];
        let mut lowest = None::<f32>;
        for end in [0.0, 1.0] {
            self.rig.values[c] = end;
            if let Some((_, d)) = self.sight() {
                let rise = d.z.atan2(d.truncate().length());
                lowest = Some(lowest.map_or(rise, |l| l.min(rise)));
            }
        }
        self.rig.values[c] = now;
        lowest
    }

    /// The sight a turret hands its guns (`0x10028130`): the yaw channel's second point
    /// and the pitch channel's point, `TurretCenter` and `TargetDirect`.
    pub fn sight(&self) -> Option<(Vec3, Vec3)> {
        let ch = |c: Option<usize>| c.and_then(|c| self.rig.channels.get(c));
        let origin = self.point(usize::try_from(ch(self.rig.yaw)?.origin).ok()?)?.0;
        let direction = self.point(usize::try_from(ch(self.rig.pitch)?.point).ok()?)?.1;
        Some((origin, direction))
    }

    /// A barrel's muzzle (`0x1002a302`): its channel's control point, in the world.
    pub fn muzzle(&self, channel: usize) -> Option<(Vec3, Vec3)> {
        self.point(usize::try_from(self.rig.channels.get(channel)?.point).ok()?)
    }

    /// The direction a lobbed round of gun `gun`, fitted as a part, leaves its muzzle at to land
    /// on the point its turret traces: the lower arc from the muzzle at its round's speed. `None`
    /// for any other gun, for no traced point, or out of the round's reach.
    pub fn lobbed_launch(&self, gun: usize, muzzle: Vec3) -> Option<Vec3> {
        let g = self.guns.get(gun).filter(|g| g.falls)?;
        self.gun_parts.get(gun)?.as_ref()?;
        let target = self.target_point?;
        self.rig.traced?;
        parkan_sim::turret::lobbed_launch(g.round_speed, parkan_sim::turret::GRAVITY, target - muzzle)
    }

    /// Barrel `barrel` of gun `gun`'s muzzle in the world: a turret gun's through the
    /// turret's channels, a fitted gun's through its own part's control point.
    pub fn gun_muzzle(&self, gun: usize, barrel: usize) -> Option<(Vec3, Vec3)> {
        let barrel = self.guns.get(gun)?.barrels.get(barrel)?;
        let Some(Some(fitted)) = self.gun_parts.get(gun) else { return self.muzzle(barrel.channel) };
        let point = fitted.points.get(usize::try_from(fitted.channels.get(barrel.channel)?.point).ok()?)?;
        let (position, yaw) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(yaw);
        let pose = self.part_pose(fitted.part, usize::try_from(point.nodes().0).unwrap_or(0));
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let at = f(pose.apply(point.position.map(f64::from)));
        let dir = f(rotate(pose.rotation, point.direction.map(f64::from)));
        Some((position + heading * at, heading * dir))
    }

    /// The turret put in `CIS_POINTTRACE` on `point` (`0x10024c51`): its yaw and pitch
    /// targets set so the sight looks at it.
    ///
    /// STAND-IN: docs/29-weapons.md#how-the-ai-fires--read -- how the turret turns a traced
    /// point into its targets (`0x10028bb0`) is not read: each channel is moved on from its
    /// value by the angle the sight is off, at the rate a small nudge of the channel turns
    /// the sight, the aim being linear in the value (docs/30).
    pub fn aim_at(&mut self, point: Vec3) {
        let Some((origin, direction)) = self.sight() else { return };
        self.rig.traced = Some(point - origin);
        let want = point - origin;
        let yaw = |d: Vec3| yaw_along(d).unwrap_or(0.0);
        let rise = |d: Vec3| d.z.atan2(d.truncate().length());
        for (axis, channel) in [(0, self.rig.yaw), (1, self.rig.pitch)] {
            let Some(c) = channel else { continue };
            let angle = |d: Vec3| if axis == 0 { yaw(d) } else { rise(d) };
            let wraps = self.rig.channels[c].flags & control::CHANNEL_WRAP != 0;
            let now = self.rig.values[c];
            let nudge = if now > 0.99 { -0.01 } else { 0.01 };
            self.rig.values[c] = now + nudge;
            let nudged = self.sight().map_or(direction, |s| s.1);
            self.rig.values[c] = now;
            let rate = parkan_sim::machine::wrap_angle(angle(nudged) - angle(direction)) / nudge;
            if rate.abs() < 1e-3 {
                continue;
            }
            let v = now + parkan_sim::machine::wrap_angle(angle(want) - angle(direction)) / rate;
            let v = if wraps { v.rem_euclid(1.0) } else { v.clamp(0.0, 1.0) };
            self.rig.aim[axis] = if self.rig.upright { 1.0 - v } else { v };
        }
    }

    /// A chassis node's pose in the unit's frame, the chassis playing its frames as the
    /// pose walk does ([`parkan_formats::mesh::Mesh::walk_pose`]), and then — on a node a
    /// `CONTACT_PLACE` contact carries — the tracked chassis's twelve belts, and a walker's
    /// feet in the states whose pose stands them up — turned to lie along the ground under
    /// it and put back where it stood ([`parkan_sim::machine::Walker::placed`],
    /// docs/28-chassis.md, "The belt lies along the ground"; docs/24-motion.md, "A walker's
    /// feet lie flat where the animation lays them"). The turn goes on the world side, so it
    /// does not reach the node's own children; every one of the twelve belts is a leaf.
    ///
    /// DEPARTURE: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- the game's
    /// body node yaws with the gait (±10° once a run cycle on the hero), and the turret,
    /// the eye, the sight and the barrels swing with it. With [`Robot::steady`] node 0 keeps
    /// only the part of its turn that is not about its up axis, so all of them hold the
    /// cycle's mean heading, which is the heading the body moves along.
    pub fn chassis_pose(&self, node: usize) -> Pose {
        let f = self.walker.frames(self.time_ms);
        let chassis = &self.chassis.mesh;
        let (a, b, w) = (f64::from(f.a), f64::from(f.b), f64::from(f.weight));
        let pose = chassis.world_pose_by(node, |n| {
            if let Some(frame) = self.device_frame(n) {
                return chassis.pose_at(n, f64::from(frame));
            }
            let pose = chassis.walk_pose(n, a, b, w);
            if n == 0 && self.steady { without_yaw(&pose, &chassis.local_pose(0)) } else { pose }
        });
        match self.walker.placed.iter().find(|&&(n, _)| n == node) {
            Some(&(_, turn)) => {
                let t = turn.to_array();
                let turn = [f64::from(t[3]), f64::from(t[0]), f64::from(t[1]), f64::from(t[2])];
                Pose { translation: pose.translation, rotation: multiply(turn, pose.rotation) }
            }
            None => pose,
        }
    }

    /// The collision sphere's centre in the world.
    pub fn collision_centre(&self) -> Vec3 {
        self.walker.body.position + self.walker.body.to_world(self.collision.0)
    }

    /// A chassis node in the world: where it is and its second axis.
    pub fn chassis_point(&self, node: usize) -> (Vec3, Vec3) {
        let (position, yaw) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(yaw);
        let pose = self.chassis_pose(node);
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let at = f(pose.apply([0.0; 3]));
        let y = f(rotate(pose.rotation, [0.0, 1.0, 0.0]));
        (position + heading * at, heading * y)
    }

    /// The turret's pose in the unit's frame, with the chassis playing its frames.
    pub fn mount(&self) -> Pose {
        self.chassis_pose(self.socket).compose(&self.turret.mesh.root_pose().invert())
    }

    /// Node `node` of part `part` posed in the unit's frame: the chassis playing its frames,
    /// the turret its channels, and any other part hanging at rest on its host's node as
    /// the assembly mounts it.
    pub fn part_pose(&self, part: usize, node: usize) -> Pose {
        if part == self.chassis_part {
            return self.chassis_pose(node);
        }
        if part == self.turret_part {
            return self.turret_node(&self.mount(), node);
        }
        let Some(p) = self.parts.get(part) else { return parkan_formats::pose::IDENTITY };
        let own = p.mesh.mesh.world_pose(node);
        match (usize::try_from(p.host), usize::try_from(p.node)) {
            (Ok(host), Ok(socket)) if host < part => {
                self.part_pose(host, socket).compose(&p.mesh.mesh.root_pose().invert()).compose(&own)
            }
            _ => own,
        }
    }

    /// Where the robot is drawn now: its position and heading as a placement.
    pub fn placement(&self) -> Pose {
        let (position, yaw) = self.walker.drawn(self.time_ms);
        let half = f64::from(yaw) / 2.0;
        Pose { translation: position.to_array().map(f64::from), rotation: [half.cos(), 0.0, 0.0, half.sin()] }
    }

    /// A turret node's pose in the unit's frame.
    pub fn turret_node(&self, mount: &Pose, node: usize) -> Pose {
        let mesh = &self.turret.mesh;
        let local = |n: usize| match self.rig.frame_of(n) {
            Some(frame) => mesh.pose_at(n, f64::from(frame)),
            None => mesh.local_pose(n),
        };
        mount.compose(&mesh.world_pose_by(node, local))
    }

    /// The first-person eye (`Control.dll:0x100234c0`): `CameraCenter`'s position plus
    /// the shake, `TargetDirect`'s vector for the look and `CameraCenter`'s own vector
    /// for the up, turned by free look ([`view`]), with the field its zoom has reached.
    pub fn eye(&self) -> Option<Eye> {
        let (eye_point, look_point) = self.camera.as_ref()?;
        let (position, _) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(self.walker.drawn(self.time_ms).1);
        let mount = self.mount();
        let node = |p: &ControlPoint| usize::try_from(p.nodes().0).unwrap_or(0);
        let at = self.turret_node(&mount, node(eye_point));
        let look = self.turret_node(&mount, node(look_point));
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let eye = f(at.apply(eye_point.position.map(f64::from)));
        let forward = f(rotate(look.rotation, look_point.direction.map(f64::from)));
        let up = f(rotate(at.rotation, eye_point.direction.map(f64::from)));
        let (forward, up) = view(heading * forward, heading * up, self.rig.look);
        let values = self.rig.camera_values;
        Some(Eye {
            position: position + heading * eye + self.rig.shake.eye(self.time_ms / 1000.0),
            forward,
            up,
            fov_x: self.zoom.field(values[2]),
            near: values[0],
        })
    }
}

/// Where a unit's first class-1 component reads its life (`iron3d.dll:0x10076d30`, docs/39,
/// "Boarding"): the part and the node it names, given the chassis's controller and the turret
/// part's, and the socket the turret part hangs on. Property `0x52` of that component is its
/// node's life over its maximum: interface `0x202` slot 3 answers it with `IDeviceManager`'s
/// value `0x400` (`Control.dll:0x1002e68c`), which asks `ILifeSystem` slot 3 for id 1 of the
/// node the device keeps at `+4` (`0x1002bc3c`). Parts load chassis first, so a chassis with a
/// turret component of its own answers before a fitted turret; and a fitted part's node 0 is
/// the socket it hangs on (`0x10009081`), so a component naming it reads its host's node. All
/// 58 turret controllers in `turrets.rlb` name node 1, the turret's body.
pub fn turret_life_node(
    chassis: &Controller,
    chassis_part: usize,
    turret: &Controller,
    turret_part: usize,
    socket: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let first = |c: &Controller| {
        c.components.iter().find(|k| k.type_id == TURRET_TYPE).and_then(|k| usize::try_from(k.node).ok())
    };
    if let Some(n) = first(chassis) {
        return Some((chassis_part, n));
    }
    match first(turret)? {
        0 if turret_part != chassis_part => socket,
        n => Some((turret_part, n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_turret(node: i32) -> Controller {
        let turret = Component {
            type_id: TURRET_TYPE,
            resource: Default::default(),
            index: None,
            entries: Vec::new(),
            label: String::new(),
            values: [0.0; 16],
            power: 0.0,
            node,
            mass: 0.0,
            flags: 0,
            group: -1,
            weights: [0.0; 2],
        };
        Controller { components: vec![turret], ..Controller::default() }
    }

    #[test]
    fn the_boarding_test_reads_the_life_of_the_node_the_first_turret_component_names() {
        let bare = Controller::default();
        // A fitted turret names its own node 1, its body, as every shipped one does.
        assert_eq!(turret_life_node(&bare, 0, &with_turret(1), 2, Some((0, 5))), Some((2, 1)));
        // Its node 0 would be the socket on the host it hangs from.
        assert_eq!(turret_life_node(&bare, 0, &with_turret(0), 2, Some((0, 5))), Some((0, 5)));
        // A chassis's own turret component loads first: an animal's, or `r_l_06`'s.
        assert_eq!(turret_life_node(&with_turret(0), 0, &with_turret(0), 0, None), Some((0, 0)));
        assert_eq!(turret_life_node(&with_turret(1), 0, &with_turret(1), 2, Some((0, 5))), Some((0, 1)));
        // No class-1 component: refused.
        assert_eq!(turret_life_node(&bare, 0, &bare, 2, Some((0, 5))), None);
    }

    fn node(part: usize, node: usize, weight: f32, area: f32, flags: i32) -> HeftNode {
        HeftNode { part, node, weight, area, flags, root: part == 0 }
    }

    /// A chassis of 1,000 kg body over nodes 0–2, the gear on 1 (left) and 2 (right), a turret
    /// part whose node 1 weighs 500, armour of 2 kg a unit of area, 1,500 kg of devices, a
    /// payload of 4,000 and an engine of drive 0.8 on node 0.
    fn heft() -> Heft {
        Heft {
            payload: 4000.0,
            nodes: vec![
                node(0, 0, 600.0, 100.0, 0),
                node(0, 1, 200.0, 50.0, GEAR_LEFT),
                node(0, 2, 200.0, 50.0, GEAR_RIGHT),
                node(1, 1, 500.0, 50.0, 0),
            ],
            per_area: 2.0,
            devices: 1500.0,
            engines: vec![(0.8, 0)],
        }
    }

    #[test]
    fn the_live_factors_are_the_engines_condition_the_gear_and_the_spare_payload() {
        let h = heft();
        let (e, r) = h.factors(0, |_, _| (1.0, false));
        // Total 1,500 + 1,500 of nodes + 2 × 250 of armour = 3,500; spare 4,000 + 1,000 − 3,500.
        assert!((e - 0.8).abs() < 1e-6, "{e}");
        assert!((r - 0.375).abs() < 1e-6, "{r}");
        // The engine's node at half life halves E; the left gear at 0 halves it again.
        let (e, _) = h.factors(0, |p, n| (if (p, n) == (0, 0) { 0.5 } else { 1.0 }, false));
        assert!((e - 0.4).abs() < 1e-6, "{e}");
        let (e, _) = h.factors(0, |p, n| (if (p, n) == (0, 1) { 0.0 } else { 1.0 }, false));
        assert!((e - 0.4).abs() < 1e-6, "{e}");
        // The turret's node gone takes its 500 kg and its 100 of armour out: spare 2,100.
        let (_, r) = h.factors(0, |p, n| (1.0, (p, n) == (1, 1)));
        assert!((r - 0.525).abs() < 1e-6, "{r}");
        // A root node gone leaves the spare as it was, but for its armour.
        let (_, r) = h.factors(0, |p, n| (1.0, (p, n) == (0, 0)));
        assert!((r - (1700.0 / 4000.0)).abs() < 1e-6, "{r}");
        // Overloaded, the spare is 0; with no payload, r is 0.
        let heavy = Heft { devices: 9000.0, ..heft() };
        assert_eq!(heavy.factors(0, |_, _| (1.0, false)).1, 0.0);
        assert_eq!(Heft { payload: 0.0, ..heft() }.factors(0, |_, _| (1.0, false)).1, 0.0);
        // No gear nodes count as both sides whole.
        let bare = Heft { nodes: vec![node(0, 0, 600.0, 0.0, 0)], ..heft() };
        assert!((bare.factors(0, |_, _| (1.0, false)).0 - 0.8).abs() < 1e-6);
    }

    fn about(axis: [f64; 3], angle: f64) -> [f64; 4] {
        let s = (angle / 2.0).sin();
        [(angle / 2.0).cos(), axis[0] * s, axis[1] * s, axis[2] * s]
    }

    fn close(a: [f64; 4], b: [f64; 4]) -> bool {
        let d = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>().abs();
        (d - 1.0).abs() < 1e-9
    }

    #[test]
    fn without_yaw_takes_out_the_turn_about_up_and_keeps_the_rest() {
        let rest = Pose { translation: [0.0; 3], rotation: about([1.0, 0.0, 0.0], 0.3) };
        let tilt = about([0.0, 1.0, 0.0], 0.05);
        let turned = Pose {
            translation: [1.0, 2.0, 3.0],
            rotation: multiply(rest.rotation, multiply(tilt, about([0.0, 0.0, 1.0], 0.17))),
        };
        let steady = without_yaw(&turned, &rest);
        assert_eq!(steady.translation, [1.0, 2.0, 3.0]);
        assert!(close(steady.rotation, multiply(rest.rotation, tilt)), "{:?}", steady.rotation);
        let only_yaw = Pose { rotation: multiply(rest.rotation, about([0.0, 0.0, 1.0], -0.17)), ..rest };
        assert!(close(without_yaw(&only_yaw, &rest).rotation, rest.rotation));
    }
}
