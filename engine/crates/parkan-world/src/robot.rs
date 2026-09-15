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
    self, CAMERA_TYPE, CHANNEL_UNDRIVEN, Channel, Component, Controller, GUN_TYPE, RADAR_PERIOD, RADAR_RANGE,
    RADAR_TYPE, SIMPLE_TYPE, TRIPLE_TOP_SPEED, TURRET_TYPE,
};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, multiply, rotate};
use parkan_sim::behaviour::Behaviour;
use parkan_sim::damage::GroundDamage;
use parkan_sim::device::{Item, Motion};
use parkan_sim::ground::Ground;
use parkan_sim::guns::{Gun, Shot, Sight, TargetGate};
use parkan_sim::machine::Walker;
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
    /// The collision sphere in the unit's frame: its centre and radius.
    pub collision: (Vec3, f32),
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
}

/// A controller's items that drive channels: its generic devices and its radars.
pub fn devices(controller: &Controller) -> Vec<Item> {
    controller
        .components
        .iter()
        .enumerate()
        .filter(|(_, k)| matches!(k.type_id, SIMPLE_TYPE | RADAR_TYPE) && !k.entries.is_empty())
        .map(|(i, k)| Item::new(i, k, &controller.channels))
        .collect()
}

fn read_member(assembly: &mut Assembly, library: &str, member: &str) -> Result<Vec<u8>> {
    let archive = assembly.archive(library).with_context(|| format!("no archive {library}"))?;
    Ok(archive.read_name(member)?.to_vec())
}

fn controller(assembly: &mut Assembly, record: &str) -> Result<Option<Controller>> {
    let Some(slot) = assembly.library.get(record).and_then(|r| r.slot_with_suffix("ctl")).cloned() else {
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
        let walker = Walker::new(chassis_ctl, &chassis.mesh, &feet, position, placed.rotation);

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

        // The agent's sphere from its parts' header spheres (`AniMesh.dll:0x10009510`,
        // docs/26): their centres weighted by their radii, and a radius reaching the
        // farthest part's sphere. The turret's is carried by its mount at rest.
        let rest_mount = chassis
            .mesh
            .world_pose(usize::try_from(turret_part.node).unwrap_or(0))
            .compose(&turret_mesh.mesh.root_pose().invert());
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let spheres: Vec<(Vec3, f32)> = [
            chassis.mesh.sphere.map(|(c, r)| (Vec3::from_array(c), r)),
            turret_mesh.mesh.sphere.map(|(c, r)| (f(rest_mount.apply(c.map(f64::from))), r)),
        ]
        .into_iter()
        .flatten()
        .collect();
        let weight: f32 = spheres.iter().map(|s| s.1).sum();
        let centre =
            if weight > 0.0 { spheres.iter().map(|s| s.0 * s.1).sum::<Vec3>() / weight } else { Vec3::ZERO };
        let collision = (centre, spheres.iter().map(|s| s.0.distance(centre) + s.1).fold(0.0, f32::max));
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
            collision,
            target_point: None,
            steady: false,
            velocity: Vec3::ZERO,
            time_ms: 0.0,
            // Units apart update their lives apart.
            ground_damage: GroundDamage::new(0.0, (object as u16).wrapping_mul(40_503)),
            chassis_devices,
            turret_devices,
        }))
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
                // STAND-IN: docs/29-weapons.md#a-gun-is-ready-once-its-arm-is-out--read-and-measured
                // -- a gun fitted as a part has no mount on the turret's list: its ready byte
                // is taken as set, and its barrels' recoil is not played.
                gun.ready = true;
                gun.selected = true;
                self.guns.push(gun);
                self.rounds.push(kind);
                self.gun_parts.push(Some(GunPart {
                    part: p,
                    channels: ctl.channels.clone(),
                    points: points.clone(),
                }));
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
        gun.falls = controller(assembly, &c.resource.member).ok().flatten().is_some_and(|r| r.mode != 0);
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
        for (i, (g, sight)) in self.guns.iter_mut().zip(sights).enumerate() {
            g.sight = sight;
            if self.gun_parts.get(i).is_some_and(Option::is_some) {
                g.ready = true;
            }
            g.recharge();
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
    /// pose walk does ([`parkan_formats::mesh::Mesh::walk_pose`]).
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
        chassis.world_pose_by(node, |n| {
            if let Some(frame) = self.device_frame(n) {
                return chassis.pose_at(n, f64::from(frame));
            }
            let pose = chassis.walk_pose(n, a, b, w);
            if n == 0 && self.steady { without_yaw(&pose, &chassis.local_pose(0)) } else { pose }
        })
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
    /// for the up, turned by free look ([`view`]).
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
            fov_x: values[2],
            near: values[0],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
