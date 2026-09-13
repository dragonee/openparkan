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
use parkan_formats::control::{self, ARM_TYPE, CAMERA_TYPE, Controller, GUN_TYPE};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, rotate};
use parkan_formats::{controls, gamedir};
use parkan_sim::ground::Ground;
use parkan_sim::guns::{CONTINUE_FIGHT, Gun, STATE_OFF, Shot};
use parkan_sim::input::{Hands, Pilot};
use parkan_sim::machine::Walker;
use parkan_sim::turret::Rig;
use parkan_sim::turret::step_toward;

use crate::assembly::{Assembly, LoadedMesh, Part};
use crate::battle::{Battle, STARTS_SELECTED};

/// What `Iron_3D.ini` sets the mouse to when it says nothing.
pub const DEFAULT_MOUSE_SENS: f32 = 100.0;

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
    /// Each gun's arm: the channels of the class-24 component paired with it.
    pub arms: Vec<Vec<usize>>,
    fire_held: bool,
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

        let position = Vec3::from_array(placed.position);
        let walker = Walker::new(chassis_ctl, &chassis.mesh, position, placed.rotation);
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
            arms: Vec::new(),
            fire_held: false,
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

    /// Fit the turret's guns: each class-2 component, the round its record names
    /// loaded into `battle`. A gun whose round's frame +116 carries 4 starts selected;
    /// on the hero the rest start deselected (`World3D.dll:0x1000ed20`).
    pub fn arm(&mut self, battle: &mut Battle, assembly: &mut Assembly) {
        self.guns.clear();
        self.rounds.clear();
        let components = self.turret_controller.components.clone();
        self.arms = components
            .iter()
            .filter(|c| c.type_id == ARM_TYPE)
            .map(|c| c.entries.iter().filter_map(|&e| usize::try_from(e).ok()).collect())
            .collect();
        for (i, c) in components.iter().enumerate().filter(|(_, c)| c.type_id == GUN_TYPE) {
            let mut gun = Gun::new(i, c, &self.turret_controller.channels);
            let kind = battle.round_kind(assembly, &c.resource.member);
            gun.selected = kind.is_some_and(|k| battle.kinds[k].frame_flags & STARTS_SELECTED != 0);
            // STAND-IN: docs/29-weapons.md#the-button-reaches-the-selected-guns -- selecting
            // sends the gun's arm state 1 and deselecting state 2, which by the frames
            // unfold and fold it; a gun selected at the start begins unfolded.
            if gun.selected
                && let Some(arm) = self.arms.get(self.guns.len())
            {
                for &c in arm {
                    if let Some(v) = self.rig.values.get_mut(c) {
                        *v = 1.0;
                    }
                }
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
        self.walker.advance(self.time_ms, ground);
        self.rig.update((dt_ms / 1000.0) as f32);

        // `World3D.dll:0x100109f8`: a gun's number toggles it; -1 selects and resets all.
        for n in std::mem::take(&mut self.pilot.selects) {
            if n < 0 {
                for g in &mut self.guns {
                    g.selected = true;
                    g.reset();
                }
            } else if let Some(g) = usize::try_from(n - 1).ok().and_then(|i| self.guns.get_mut(i)) {
                g.toggle();
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
        let dt = (dt_ms / 1000.0) as f32;
        for (g, arm) in self.guns.iter().zip(&self.arms) {
            let target = if g.selected { 1.0 } else { 0.0 };
            for &c in arm {
                if let (Some(ch), Some(v)) = (self.rig.channels.get(c), self.rig.values.get(c).copied()) {
                    self.rig.values[c] = step_toward(v, target, ch.rate, dt, false);
                }
            }
        }
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

    /// The hero's world velocity.
    pub fn world_velocity(&self) -> Vec3 {
        self.walker.body.to_world(Vec3::from_array(self.walker.body.velocity))
    }

    /// A turret control point in the world: its position and direction.
    pub fn point(&self, index: usize) -> Option<(Vec3, Vec3)> {
        let p = self.points.get(index)?;
        let (position, _) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(self.walker.drawn_heading(self.time_ms));
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

    /// The turret's pose in the unit's frame, with the chassis playing its frames.
    fn mount(&self) -> Pose {
        let f = self.walker.frames(self.time_ms);
        let chassis = &self.chassis.mesh;
        let (a, b, w) = (f64::from(f.a), f64::from(f.b), f64::from(f.weight));
        let socket = chassis.world_pose_by(self.socket, |n| chassis.blended_pose(n, a, b, w));
        socket.compose(&self.turret.mesh.root_pose().invert())
    }

    /// A turret node's pose in the unit's frame.
    fn turret_node(&self, mount: &Pose, node: usize) -> Pose {
        let mesh = &self.turret.mesh;
        let local = |n: usize| match self.rig.frame_of(n) {
            Some(frame) => mesh.pose_at(n, f64::from(frame)),
            None => mesh.local_pose(n),
        };
        mount.compose(&mesh.world_pose_by(node, local))
    }

    /// The first-person eye (`Control.dll:0x100234c0`).
    ///
    /// STAND-IN: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- which
    /// point gives the position and which the direction is read from the data:
    /// position from `CameraCenter`, look along `TargetDirect`, up the look node's +z.
    pub fn eye(&self) -> Eye {
        let (position, _) = self.walker.drawn(self.time_ms);
        let heading = Quat::from_rotation_z(self.walker.drawn_heading(self.time_ms));
        let mount = self.mount();
        let node = |p: &ControlPoint| usize::try_from(p.nodes().0).unwrap_or(0);
        let at = self.turret_node(&mount, node(&self.eye_point));
        let look = self.turret_node(&mount, node(&self.look_point));
        let f = |v: [f64; 3]| Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        let eye = f(at.apply(self.eye_point.position.map(f64::from)));
        let forward =
            f(rotate(look.rotation, self.look_point.direction.map(f64::from))).normalize_or(Vec3::Y);
        let up = f(rotate(look.rotation, [0.0, 0.0, 1.0])).normalize_or(Vec3::Z);

        // Free look (`0x10023788`): pitch (0.5 − y) × π about the side axis, yaw
        // (0.5 − x) × 2π about the up axis.
        // STAND-IN: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- the
        // senses on screen are not read; yaw is taken clockwise so it follows the hull's.
        let side = forward.cross(up).normalize_or(Vec3::X);
        let [x, y, _] = self.rig.look;
        let turn = Quat::from_axis_angle(up, -(0.5 - x) * std::f32::consts::TAU)
            * Quat::from_axis_angle(side, (0.5 - y) * std::f32::consts::PI);
        let values = self.rig.camera_values;
        Eye {
            position: position + heading * eye,
            forward: heading * (turn * forward),
            up: heading * (turn * up),
            fov_x: values[2],
            near: values[0],
        }
    }
}
