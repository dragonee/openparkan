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

#[derive(Clone)]
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
            // A unit is built from its `.dat` with the placement's matrix alone, so its
            // volume scale is 1 (docs/04-missions.md, "The scale").
            let (mut life, blast) = part_damage(assembly, part, &mesh.mesh, 1.0, 1.0, false);
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
    /// (docs/39-boarding.md, "Driving": `m2.tbl` for a flyer, `m1.tbl` for the rest), or for a
    /// chassis that names none, a building's, [`DEFAULT_TABLE`].
    pub fn pilot_for(assembly: &mut Assembly, chassis_record: &str) -> Result<Pilot> {
        let table = assembly
            .library
            .get(chassis_record)
            .and_then(|r| r.slots.iter().find(|s| s.suffix() == "tbl"))
            .map_or_else(|| DEFAULT_TABLE.to_owned(), |s| s.member.clone());
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
        let shots =
            drive(&mut self.robot, &mut self.pilot, &mut self.fire_held, dt_ms, mouse, ground, Reach::Whole);
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

/// The input table the table reader defaults to (`World3D.dll:0x1000b3e0`, docs/14, "A row
/// that stays down"). No building's chassis names a `.tbl`, so it is what a tower is driven by
/// in its manual control (*derived*).
pub const DEFAULT_TABLE: &str = "M1.TBL";

/// What of a taken unit the player's input reaches, by its auto-driver level (record `+0x9c`):
/// `iron3d.dll:0x10074ff0` writes the unit's `Wizard.dll` group words by it, and the Wizard
/// hands each word's side its bits (`Wizard.dll:0x10003890`; docs/40, "Telepresence").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// Level 0: the whole unit -- its own rows, turret, guns, shields and sensors.
    Whole,
    /// Level 1: all but the unit's own rows, whose word goes to the AI and with it the
    /// behaviour's movement flag `0x10`. The turret, guns, camera and the rest stay the player's.
    Weapons,
    /// Level 2 with the two words its take leaves alone still the player's: the camera, radar
    /// and seeker (group 2, `+0x210`) and group 0 with the repair system (`+0x214`), which a take
    /// at level 1 wrote 3 into and the key's step to 2 kept (docs/40, "Telepresence"). The rest
    /// is [`Reach::Nothing`]'s.
    Sensors,
    /// Level 2: nothing. The AI has the movement, the turret and guns (group 4) with its fight
    /// module (flags `0x20`, `0x40`), the shields and armour (group 5), and by message 7 with 0
    /// every group left to follow the mode -- the sensors and group 0 too, since a letting-go
    /// wrote them 1.
    Nothing,
}

impl Reach {
    /// The reach of auto-driver level `level`, which the game steps 0, 1, 2 and round, over a
    /// unit whose sensor words are the player's when `sensors` holds ([`Robot::sensors_taken`]).
    pub fn of(level: u8, sensors: bool) -> Reach {
        match level {
            0 => Reach::Whole,
            1 => Reach::Weapons,
            _ if sensors => Reach::Sensors,
            _ => Reach::Nothing,
        }
    }

    /// Whether the unit's behaviour walks it: flag `0x10` on, at levels 1 and 2.
    pub fn ai_moves(self) -> bool {
        self != Reach::Whole
    }

    /// Whether its fight module aims and fires: flags `0x20` and `0x40` on, at level 2.
    pub fn ai_fights(self) -> bool {
        matches!(self, Reach::Sensors | Reach::Nothing)
    }

    /// Whether the player's rows reach the camera, radar and seeker (group 2) and group 0, the
    /// repair system's: all but a level 2 over words a letting-go gave the AI.
    pub fn sensors(self) -> bool {
        self != Reach::Nothing
    }

    /// Whether they reach the shields and armour (group 5), the detection shield's camouflage
    /// among them: levels 0 and 1.
    pub fn shields(self) -> bool {
        matches!(self, Reach::Whole | Reach::Weapons)
    }
}

/// `f` given the unit's hands, what `reach` keeps from the player swapped for scratch copies:
/// the pilot's held keys and mouse stay its own and move nothing they do not reach. The
/// switches its state rows turn are handed back where their group is not the player's:
/// the repair system's (group 0) and the camera's infrared (group 2) with the sensors, the
/// detection shield's camouflage (group 5) with the shields.
fn reached<R>(
    robot: &mut Robot,
    pilot: &mut Pilot,
    reach: Reach,
    f: impl FnOnce(&mut Pilot, &mut Hands) -> R,
) -> R {
    let mut body = robot.walker.body;
    let mut turret = robot.rig.aim;
    let mut camera = robot.rig.look;
    let hands = &mut Hands {
        body: if reach == Reach::Whole { &mut robot.walker.body } else { &mut body },
        turret: if reach.ai_fights() { &mut turret } else { &mut robot.rig.aim },
        camera: if reach.sensors() { &mut robot.rig.look } else { &mut camera },
    };
    let before = pilot.switches;
    let out = f(pilot, hands);
    if !reach.sensors() {
        pilot.switches.repair = before.repair;
        pilot.switches.infrared = before.infrared;
    }
    if !reach.shields() {
        pilot.switches.camouflage = before.camouflage;
    }
    out
}

/// One tick of a unit driven from `pilot`: the mouse counts since the last, then the
/// machine, the guns the number keys select and the button fires, then the turret and the
/// guns. Returns the rounds that left, by gun. A unit the player reaches nothing of runs as
/// the AI leaves it, and what the player asked of its guns is dropped.
pub fn drive(
    robot: &mut Robot,
    pilot: &mut Pilot,
    fire_held: &mut bool,
    dt_ms: f64,
    mouse: [f32; 2],
    ground: &Ground,
    reach: Reach,
) -> Vec<(usize, Shot)> {
    reached(robot, pilot, reach, |pilot, hands| pilot.mouse(mouse, hands));
    robot.advance(dt_ms, ground);
    if reach.ai_fights() {
        pilot.selects.clear();
        return robot.takt(dt_ms);
    }
    press_guns(robot, pilot, fire_held);
    robot.takt(dt_ms)
}

/// The guns of a building the player drives (mode 6, docs/27, "What the modes show"): the mouse
/// counts since the last tick to its turret, and the guns the number keys select and the button
/// fires. A building does not move, and its turret's and guns' takt runs with its own
/// (`Play::tick_emplacements`).
pub fn drive_guns(robot: &mut Robot, pilot: &mut Pilot, fire_held: &mut bool, mouse: [f32; 2]) {
    reached(robot, pilot, Reach::Whole, |pilot, hands| pilot.mouse(mouse, hands));
    press_guns(robot, pilot, fire_held);
}

/// The player's number keys and button on a unit's guns.
fn press_guns(r: &mut Robot, pilot: &mut Pilot, fire_held: &mut bool) {
    // `World3D.dll:0x100109f8`: a gun's number toggles it and sends its arm state 1
    // or 2; -1 selects and resets every gun and sends every arm `0x21`.
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
}

/// A key to a unit driven from `pilot`, as far as `reach` goes.
pub fn drive_key(robot: &mut Robot, pilot: &mut Pilot, scan: &str, pressed: bool, reach: Reach) {
    reached(robot, pilot, reach, |pilot, hands| pilot.key(scan, pressed, hands));
}

/// The input update of a unit driven from `pilot` as far as `reach` goes, or every key
/// coming up.
pub fn drive_input(robot: &mut Robot, pilot: &mut Pilot, release: bool, reach: Reach) {
    let now = robot.time_ms;
    reached(robot, pilot, reach, |pilot, hands| {
        if release {
            pilot.release_all(hands);
        } else {
            pilot.update(now, hands);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::Reach;

    #[test]
    fn level_1_gives_the_ai_the_walk_and_level_2_the_fight_as_well() {
        // `iron3d.dll:0x10074ff0`: level 1 hands the Wizard's unit word to the AI, level 2 the
        // guns' and shields' words too, and message 7 with 0.
        let of = |l, s| {
            let r = Reach::of(l, s);
            (r, r.ai_moves(), r.ai_fights())
        };
        for sensors in [false, true] {
            assert_eq!(of(0, sensors), (Reach::Whole, false, false));
            assert_eq!(of(1, sensors), (Reach::Weapons, true, false));
        }
        assert_eq!(of(2, false), (Reach::Nothing, true, true));
        assert_eq!(of(2, true), (Reach::Sensors, true, true));
    }

    #[test]
    fn a_level_2_the_key_reached_leaves_the_player_the_camera_and_the_repair_switch_alone() {
        // The take at 2 writes neither `+0x210` (group 2: camera, radar, seeker) nor `+0x214`
        // (group 0: the repair system) (`0x10075072`-`0x10075099`), so after the take at 1 wrote
        // them 3 they stay the player's; a letting-go wrote them 1, the AI's (`0x10075131`).
        // The shields' `+0x208` goes to the AI at 2 either way (`0x10075082`).
        let groups = |r: Reach| (r.sensors(), r.shields());
        assert_eq!(groups(Reach::of(0, false)), (true, true));
        assert_eq!(groups(Reach::of(1, false)), (true, true));
        assert_eq!(groups(Reach::of(2, true)), (true, false));
        assert_eq!(groups(Reach::of(2, false)), (false, false));
    }
}
