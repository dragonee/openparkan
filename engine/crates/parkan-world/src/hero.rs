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
use parkan_formats::control::{self, CAMERA_TYPE, Controller};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, rotate};
use parkan_formats::{controls, gamedir};
use parkan_sim::ground::Ground;
use parkan_sim::input::{Hands, Pilot};
use parkan_sim::machine::Walker;
use parkan_sim::turret::Rig;

use crate::assembly::{Assembly, LoadedMesh, Part};

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
    gamedir::resolve(game, "Iron_3D.ini")
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| {
            String::from_utf8_lossy(&b).lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                key.trim().eq_ignore_ascii_case("MOUSE_SENS").then(|| value.trim().parse().ok())?
            })
        })
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

    /// One tick of game time: the mouse counts since the last, then the machine and
    /// the turret.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2], ground: &Ground) {
        let (pilot, mut hands) = self.hands();
        pilot.mouse(mouse, &mut hands);
        self.time_ms += dt_ms;
        self.walker.advance(self.time_ms, ground);
        self.rig.update((dt_ms / 1000.0) as f32);
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
