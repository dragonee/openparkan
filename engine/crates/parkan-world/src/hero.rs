//! The player's hero: a [`Robot`] its pilot drives from the input table, and the eye in
//! it. The chassis's record names the input table; see `docs/14-controls.md`,
//! `docs/24-motion.md` and `docs/30-turrets.md`, "Aiming and the camera".

use std::ops::{Deref, DerefMut};
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::exp::Explosion;
use parkan_formats::mission::Mission;
use parkan_formats::{controls, gamedir};
use parkan_sim::damage::Life;
use parkan_sim::ground::Ground;
use parkan_sim::guns::{CONTINUE_FIGHT, STATE_OFF, Shot};
use parkan_sim::input::{Hands, Pilot};
use parkan_sim::turret::{ARM_UNFOLD, ITEM_CLOSING, ITEM_OPENING};

use crate::assembly::Assembly;
use crate::battle::part_damage;
pub use crate::robot::{Eye, NO_RADAR_PERIOD_MS, NO_RADAR_RANGE, Robot};

/// What `Iron_3D.ini` sets the mouse to when it says nothing.
pub const DEFAULT_MOUSE_SENS: f32 = 100.0;

/// Whether a mission object is the player's hero.
pub fn is_hero(path: &str) -> bool {
    path.to_ascii_uppercase().contains("\\HERO\\")
}

/// `MOUSE_SENS` from `Iron_3D.ini`.
pub fn mouse_sensitivity(game: &Path) -> f32 {
    crate::settings::value(game, "CS", "MOUSE_SENS")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MOUSE_SENS)
}

pub struct Hero {
    pub robot: Robot,
    pub pilot: Pilot,
    fire_held: bool,
    /// Each of the robot's parts' node life, and what its nodes play when destroyed. The
    /// player's own hero is never given the difficulty ratio (docs/26).
    pub lives: Vec<Option<Life>>,
    pub blasts: Vec<Vec<Option<Explosion>>>,
}

impl Deref for Hero {
    type Target = Robot;
    fn deref(&self) -> &Robot {
        &self.robot
    }
}

impl DerefMut for Hero {
    fn deref_mut(&mut self) -> &mut Robot {
        &mut self.robot
    }
}

impl Hero {
    /// The hero of `mission`, if it has one whose chassis, turret and camera resolve.
    pub fn load(assembly: &mut Assembly, mission: &Mission) -> Result<Option<Hero>> {
        let Some(object) = mission.objects.iter().position(|o| is_hero(&o.path)) else { return Ok(None) };
        let robot = Robot::load(assembly, mission, object)?.context("the hero has no chassis and turret")?;
        robot.camera.as_ref().context("the hero's turret has no camera")?;
        let placed = &mission.objects[object];
        let parts = assembly.parts(placed.kind, &placed.path);
        let chassis = parts.iter().find(|p| p.host == -1).context("the hero has no chassis")?;
        let pilot = Hero::pilot_for(assembly, &chassis.record.clone())?;
        // The robot's parts, those whose meshes load, in its order.
        let (mut lives, mut blasts) = (Vec::new(), Vec::new());
        let armour = crate::shields::armour(assembly, placed.kind, &placed.path);
        for part in &parts {
            let Some(mesh) = assembly.mesh(&part.reference) else { continue };
            let (mut life, blast) = part_damage(assembly, part, &mesh.mesh, 1.0, false);
            if let Some(life) = life.as_mut() {
                life.armour = armour;
            }
            lives.push(life);
            blasts.push(blast);
        }
        Ok(Some(Hero { robot, pilot, fire_held: false, lives, blasts }))
    }

    fn hands(&mut self) -> (&mut Pilot, Hands<'_>) {
        let r = &mut self.robot;
        let hands = Hands { body: &mut r.walker.body, turret: &mut r.rig.aim, camera: &mut r.rig.look };
        (&mut self.pilot, hands)
    }

    /// A pilot for a unit's own input table: its chassis record's `.tbl`
    /// (docs/39-boarding.md, "Driving": `m2.tbl` for a flyer, `m1.tbl` for the rest).
    pub fn pilot_for(assembly: &mut Assembly, chassis_record: &str) -> Result<Pilot> {
        let table = assembly
            .library
            .get(chassis_record)
            .and_then(|r| r.slots.iter().find(|s| s.suffix() == "tbl"))
            .map(|s| s.member.clone())
            .context("the chassis names no input table")?;
        let rows = controls::load(&gamedir::resolve(&assembly.game, &table).context("no input table")?)?;
        Ok(Pilot::new(rows, mouse_sensitivity(&assembly.game)))
    }

    /// A key or button, by its scan name.
    pub fn key(&mut self, scan: &str, pressed: bool) {
        let (pilot, mut hands) = self.hands();
        pilot.key(scan, pressed, &mut hands);
    }

    /// Every key still down comes up.
    pub fn release_keys(&mut self) {
        let (pilot, mut hands) = self.hands();
        pilot.release_all(&mut hands);
    }

    /// The input update at the current game time: the rows held down run again.
    pub fn update_input(&mut self) {
        let now = self.robot.time_ms;
        let (pilot, mut hands) = self.hands();
        pilot.update(now, &mut hands);
    }

    /// One tick of game time: the mouse counts since the last, then the machine, the
    /// guns the number keys select and the button fires, then the turret and the guns.
    /// Returns the rounds that left, by gun.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2], ground: &Ground) -> Vec<(usize, Shot)> {
        let lives = &self.lives;
        self.robot.check_devices(|p, n| crate::play::node_alive(lives.get(p).and_then(Option::as_ref), n));
        let shots = drive(&mut self.robot, &mut self.pilot, &mut self.fire_held, dt_ms, mouse, ground);
        let lives = &self.lives;
        self.robot.turn_devices(|p, n| crate::play::node_alive(lives.get(p).and_then(Option::as_ref), n));
        shots
    }

    /// Whether the hero is dead: its chassis's node 0, or a vital node, destroyed.
    pub fn dead(&self) -> bool {
        self.lives.get(self.robot.chassis_part).and_then(Option::as_ref).is_some_and(|l| l.dead)
    }

    /// The first-person eye (see [`Robot::eye`]); the hero always has a camera.
    pub fn eye(&self) -> Eye {
        self.robot.eye().expect("the hero's turret has a camera")
    }
}

/// One tick of a unit driven from `pilot`: the mouse counts since the last, then the
/// machine, the guns the number keys select and the button fires, then the turret and the
/// guns. Returns the rounds that left, by gun.
pub fn drive(
    robot: &mut Robot,
    pilot: &mut Pilot,
    fire_held: &mut bool,
    dt_ms: f64,
    mouse: [f32; 2],
    ground: &Ground,
) -> Vec<(usize, Shot)> {
    {
        let hands = &mut Hands {
            body: &mut robot.walker.body,
            turret: &mut robot.rig.aim,
            camera: &mut robot.rig.look,
        };
        pilot.mouse(mouse, hands);
    }
    robot.advance(dt_ms, ground);

    // `World3D.dll:0x100109f8`: a gun's number toggles it and sends its arm state 1
    // or 2; -1 selects and resets every gun and sends every arm `0x21`.
    let r = robot;
    for n in std::mem::take(&mut pilot.selects) {
        if n < 0 {
            for g in &mut r.guns {
                g.selected = true;
                g.reset();
            }
            for a in &mut r.rig.arms {
                a.send(ARM_UNFOLD);
            }
        } else if let Some(i) = usize::try_from(n - 1).ok()
            && let Some(g) = r.guns.get_mut(i)
        {
            g.toggle();
            if let Some(a) = r.rig.arms.get_mut(i) {
                a.send(if g.selected { ITEM_OPENING } else { ITEM_CLOSING });
            }
        }
    }
    // `MCMD_STATE` index -1 reaches the selected guns as the button goes down or up.
    if pilot.fire != *fire_held {
        *fire_held = pilot.fire;
        let state = if *fire_held { CONTINUE_FIGHT } else { STATE_OFF };
        for g in r.guns.iter_mut().filter(|g| g.selected) {
            g.state = state;
        }
    }
    r.takt(dt_ms)
}

/// A key to a unit driven from `pilot`.
pub fn drive_key(robot: &mut Robot, pilot: &mut Pilot, scan: &str, pressed: bool) {
    let hands =
        &mut Hands { body: &mut robot.walker.body, turret: &mut robot.rig.aim, camera: &mut robot.rig.look };
    pilot.key(scan, pressed, hands);
}

/// The input update of a unit driven from `pilot`, or every key coming up.
pub fn drive_input(robot: &mut Robot, pilot: &mut Pilot, release: bool) {
    let now = robot.time_ms;
    let hands =
        &mut Hands { body: &mut robot.walker.body, turret: &mut robot.rig.aim, camera: &mut robot.rig.look };
    if release {
        pilot.release_all(hands);
    } else {
        pilot.update(now, hands);
    }
}
