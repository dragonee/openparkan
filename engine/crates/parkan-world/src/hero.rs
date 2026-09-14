//! The player's hero: its chassis walking, its turret aiming, and the eye in it.
//!
//! The chassis's controller drives a [`Walker`], its record names the input table,
//! and the turret's controller and control points give the pitch channel and the
//! camera. See `docs/24-motion.md`, `docs/14-controls.md` and `docs/30-turrets.md`,
//! "Aiming and the camera".

use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use glam::{Quat, Vec3};
use parkan_formats::control::{self, CAMERA_TYPE, Controller, GUN_TYPE};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, multiply, rotate};
use parkan_formats::{controls, gamedir};
use parkan_sim::ground::Ground;
use parkan_sim::guns::{CONTINUE_FIGHT, Gun, STATE_OFF, Shot};
use parkan_sim::input::{Hands, Pilot};
use parkan_sim::machine::Walker;
use parkan_sim::turret::{ARM_FOLD, ARM_UNFOLD, ITEM_CLOSING, ITEM_OPENING, Rig, view};

use crate::assembly::{Assembly, LoadedMesh, Part};
use crate::battle::{Battle, STARTS_SELECTED};

/// What `Iron_3D.ini` sets the mouse to when it says nothing.
pub const DEFAULT_MOUSE_SENS: f32 = 100.0;
/// Machine steps one tick runs one at a time before it hands the rest to the machine.
const MAX_STEPS: usize = 2000;

/// Whether a mission object is the player's hero.
pub fn is_hero(path: &str) -> bool {
    path.to_ascii_uppercase().contains("\\HERO\\")
}

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

pub struct Hero {
    /// The mission object the hero is.
    pub object: usize,
    pub walker: Walker,
    pub pilot: Pilot,
    pub rig: Rig,
    pub chassis: Rc<LoadedMesh>,
    pub turret: Rc<LoadedMesh>,
    /// The chassis node the turret hangs from.
    pub socket: usize,
    /// The camera's position and direction points.
    pub eye_point: ControlPoint,
    pub look_point: ControlPoint,
    /// The turret's controller and control points.
    pub turret_controller: Controller,
    pub points: Vec<ControlPoint>,
    /// The turret's guns, and the round kind each fires in its `Battle`.
    pub guns: Vec<Gun>,
    pub rounds: Vec<Option<usize>>,
    fire_held: bool,
    /// Whether the turret, and the eye in it, are held steady against the body's gait
    /// yaw ([`Hero::chassis_pose`]). Off, the view swings as the game's does.
    pub steady: bool,
    /// The machine's velocity over its last step, from the two poses either side.
    velocity: Vec3,
    /// Game time, ms.
    pub time_ms: f64,
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

/// `MOUSE_SENS` from `Iron_3D.ini`.
pub fn mouse_sensitivity(game: &Path) -> f32 {
    crate::settings::value(game, "CS", "MOUSE_SENS")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MOUSE_SENS)
}

impl Hero {
    /// The hero of `mission`, if it has one whose chassis and turret resolve.
    pub fn load(assembly: &mut Assembly, mission: &Mission) -> Result<Option<Hero>> {
        let Some((object, placed)) = mission.objects.iter().enumerate().find(|(_, o)| is_hero(&o.path))
        else {
            return Ok(None);
        };
        let parts: Vec<Part> = assembly.parts(placed.kind, &placed.path);
        let chassis_part = parts.iter().find(|p| p.host == -1).context("the hero has no chassis")?.clone();
        let chassis = assembly.mesh(&chassis_part.reference).context("the chassis mesh does not load")?;
        let chassis_ctl =
            controller(assembly, &chassis_part.record)?.context("the chassis has no controller")?;

        let mut turret = None;
        for part in parts.iter().filter(|p| p.host == 0) {
            if let Some(c) = controller(assembly, &part.record)?
                && c.components.iter().any(|k| k.type_id == CAMERA_TYPE)
            {
                turret = Some((part.clone(), c));
                break;
            }
        }
        let (turret_part, turret_ctl) = turret.context("the hero has no turret with a camera")?;
        let turret_mesh = assembly.mesh(&turret_part.reference).context("the turret mesh does not load")?;
        let points_slot = assembly
            .library
            .get(&turret_part.record)
            .and_then(|r| r.slot_with_suffix("cpt"))
            .cloned()
            .context("the turret has no control points")?;
        let points = cpt::parse(&read_member(assembly, &points_slot.library, &points_slot.member)?, "cpt")?;
        let rig = Rig::new(&turret_ctl);
        let camera = rig.camera.map(|c| rig.channels[c]).context("the camera has no channel")?;
        let point = |i: i32| usize::try_from(i).ok().and_then(|i| points.get(i)).cloned();
        let eye_point = point(camera.origin).context("the camera's CameraCenter does not resolve")?;
        let look_point = point(camera.point).context("the camera's TargetDirect does not resolve")?;

        let table = assembly
            .library
            .get(&chassis_part.record)
            .and_then(|r| r.slots.iter().find(|s| s.suffix() == "tbl"))
            .map(|s| s.member.clone())
            .context("the chassis names no input table")?;
        let rows = controls::load(&gamedir::resolve(&assembly.game, &table).context("no input table")?)?;
        let pilot = Pilot::new(rows, mouse_sensitivity(&assembly.game));

        // The chassis's control points place its contacts: the feet (docs/24).
        let feet_slot =
            assembly.library.get(&chassis_part.record).and_then(|r| r.slot_with_suffix("cpt")).cloned();
        let feet = match feet_slot {
            Some(slot) => cpt::parse(&read_member(assembly, &slot.library, &slot.member)?, &slot.member)?,
            None => Vec::new(),
        };
        let position = Vec3::from_array(placed.position);
        let walker = Walker::new(chassis_ctl, &chassis.mesh, &feet, position, placed.rotation);
        Ok(Some(Hero {
            object,
            walker,
            pilot,
            rig,
            chassis,
            turret: turret_mesh,
            socket: usize::try_from(turret_part.node).unwrap_or(0),
            eye_point,
            look_point,
            turret_controller: turret_ctl,
            points,
            guns: Vec::new(),
            rounds: Vec::new(),
            fire_held: false,
            steady: false,
            velocity: Vec3::ZERO,
            time_ms: 0.0,
        }))
    }

    fn hands(&mut self) -> (&mut Pilot, Hands<'_>) {
        let hands =
            Hands { body: &mut self.walker.body, turret: &mut self.rig.aim, camera: &mut self.rig.look };
        (&mut self.pilot, hands)
    }

    /// A key or button, by its scan name.
    pub fn key(&mut self, scan: &str, pressed: bool) {
        let (pilot, mut hands) = self.hands();
        pilot.key(scan, pressed, &mut hands);
    }

    /// The input update at the current game time: the rows held down run again.
    pub fn update_input(&mut self) {
        let now = self.time_ms;
        let (pilot, mut hands) = self.hands();
        pilot.update(now, &mut hands);
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
            if let Some(arm) = self.rig.arms.get_mut(self.guns.len()) {
                arm.send(if gun.selected { ARM_UNFOLD } else { ARM_FOLD });
            }
            self.guns.push(gun);
            self.rounds.push(kind);
        }
    }

    /// One tick of game time: the mouse counts since the last, then the machine, the
    /// turret and the guns. Returns the rounds that left, by gun.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2], ground: &Ground) -> Vec<(usize, Shot)> {
        let (pilot, mut hands) = self.hands();
        pilot.mouse(mouse, &mut hands);
        self.time_ms += dt_ms;
        self.walk(ground);
        self.rig.strafe = self.walker.strafe_offset(self.time_ms);

        // `World3D.dll:0x100109f8`: a gun's number toggles it and sends its arm state 1
        // or 2; -1 selects and resets every gun and sends every arm `0x21`.
        for n in std::mem::take(&mut self.pilot.selects) {
            if n < 0 {
                for g in &mut self.guns {
                    g.selected = true;
                    g.reset();
                }
                for a in &mut self.rig.arms {
                    a.send(ARM_UNFOLD);
                }
            } else if let Some(i) = usize::try_from(n - 1).ok()
                && let Some(g) = self.guns.get_mut(i)
            {
                g.toggle();
                if let Some(a) = self.rig.arms.get_mut(i) {
                    a.send(if g.selected { ITEM_OPENING } else { ITEM_CLOSING });
                }
            }
        }
        // `MCMD_STATE` index -1 reaches the selected guns as the button goes down or up.
        if self.pilot.fire != self.fire_held {
            self.fire_held = self.pilot.fire;
            let state = if self.fire_held { CONTINUE_FIGHT } else { STATE_OFF };
            for g in self.guns.iter_mut().filter(|g| g.selected) {
                g.state = state;
            }
        }
        if self.guns.iter().any(|g| g.falls)
            && let Some(up) = self.rig.yaw.and_then(|c| usize::try_from(self.rig.channels[c].origin).ok())
            && let Some((_, direction)) = self.point(up)
        {
            self.rig.center_up = direction.z;
        }
        // The turret's takt: its channels, the arms, and each mount's gun's ready byte.
        self.rig.update((dt_ms / 1000.0) as f32, &mut self.guns);
        let mut shots = Vec::new();
        for (i, g) in self.guns.iter_mut().enumerate() {
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

    /// The hero's world velocity.
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
    /// the eye, the sight and the barrels swing with it. With [`Hero::steady`] node 0 keeps
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

    /// The turret's pose in the unit's frame, with the chassis playing its frames.
    pub fn mount(&self) -> Pose {
        self.chassis_pose(self.socket).compose(&self.turret.mesh.root_pose().invert())
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
    pub fn eye(&self) -> Eye {
        let (position, _) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(self.walker.drawn(self.time_ms).1);
        let mount = self.mount();
        let node = |p: &ControlPoint| usize::try_from(p.nodes().0).unwrap_or(0);
        let at = self.turret_node(&mount, node(&self.eye_point));
        let look = self.turret_node(&mount, node(&self.look_point));
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let eye = f(at.apply(self.eye_point.position.map(f64::from)));
        let forward = f(rotate(look.rotation, self.look_point.direction.map(f64::from)));
        let up = f(rotate(at.rotation, self.eye_point.direction.map(f64::from)));
        let (forward, up) = view(heading * forward, heading * up, self.rig.look);
        let values = self.rig.camera_values;
        Eye {
            position: position + heading * eye + self.rig.shake.eye(self.time_ms / 1000.0),
            forward,
            up,
            fov_x: values[2],
            near: values[0],
        }
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
