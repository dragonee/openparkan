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
    self, CAMERA_TYPE, Controller, GUN_TYPE, RADAR_PERIOD, RADAR_RANGE, RADAR_TYPE, TURRET_TYPE,
};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, multiply, rotate};
use parkan_sim::damage::GroundDamage;
use parkan_sim::ground::Ground;
use parkan_sim::guns::{Gun, Shot, Sight, TargetGate};
use parkan_sim::machine::Walker;
use parkan_sim::targeting::Radar;
use parkan_sim::turret::{ARM_FOLD, ARM_UNFOLD, Rig, view};

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
    pub mesh: Rc<LoadedMesh>,
    pub host: i32,
    pub node: i32,
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
    /// The turret's guns, and the round kind each fires in its `Battle`.
    pub guns: Vec<Gun>,
    pub rounds: Vec<Option<usize>>,
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
        let parts: Vec<Part> = assembly.parts(placed.kind, &placed.path);
        let Some(chassis_part) = parts.iter().find(|p| p.host == -1).cloned() else { return Ok(None) };
        let Some(chassis) = assembly.mesh(&chassis_part.reference) else { return Ok(None) };
        let Some(chassis_ctl) = controller(assembly, &chassis_part.record)? else { return Ok(None) };

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
            robot_parts.push(RobotPart { mesh, host: part.host, node: part.node });
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
        // `R_H_02`: the letter after `R_` (`Behavior.dll:0x1000cee0`).
        let size_class = match chassis_part.record.as_bytes().get(2).map(u8::to_ascii_lowercase) {
            Some(b't') => 1,
            Some(b'l' | b'h') => 2,
            Some(b'm') => 3,
            Some(b'b') => 4,
            _ => 0,
        };
        Ok(Some(Robot {
            object,
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
        let components = self.turret_controller.components.clone();
        for (i, c) in components.iter().enumerate().filter(|(_, c)| c.type_id == GUN_TYPE) {
            let mut gun = Gun::new(i, c, &self.turret_controller.channels);
            let kind = battle.round_kind(assembly, &c.resource.member);
            gun.selected = kind.is_some_and(|k| battle.kinds[k].frame_flags & STARTS_SELECTED != 0);
            // The gun keeps its round's top speed and whether it falls (`0x100297ef`).
            gun.round_speed = kind.map_or(0.0, |k| battle.combat.kinds[k].top_speed);
            gun.falls = controller(assembly, &c.resource.member).ok().flatten().is_some_and(|r| r.mode != 0);
            // Values 8-10 from the round: its range, and a guided round's cone and lock.
            if let Some(k) = kind.map(|k| &battle.combat.kinds[k]) {
                gun.link(TargetGate::new(k.range, k.seeker.map(|s| (s.cone, s.reach, s.lock_ms))));
            }
            if let Some(arm) = self.rig.arms.get_mut(self.guns.len()) {
                arm.send(if gun.selected { ARM_UNFOLD } else { ARM_FOLD });
            }
            self.guns.push(gun);
            self.rounds.push(kind);
        }
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
        let sights: Vec<Sight> = self
            .guns
            .iter()
            .map(|g| Sight {
                unit: self.walker.body.position,
                barrel: g
                    .barrels
                    .get(g.current)
                    .and_then(|b| self.muzzle(b.channel))
                    .map_or(Vec3::Y, |m| m.1),
                target: g.target.and(self.target_point),
            })
            .collect();
        let mut shots = Vec::new();
        for (i, (g, sight)) in self.guns.iter_mut().zip(sights).enumerate() {
            g.sight = sight;
            g.recharge();
            shots.extend(g.tick(self.time_ms).into_iter().map(|s| (i, s)));
            for b in &g.barrels {
                if let Some(v) = self.rig.values.get_mut(b.channel) {
                    *v = b.value(self.time_ms);
                }
            }
        }
        shots
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
