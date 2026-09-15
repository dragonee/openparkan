//! The player's hands: an input table's rows turned into what they move.
//! `docs/14-controls.md`, "From a row to a command", and `docs/24-motion.md`,
//! "From input to motion".
//!
//! Keys and mouse axes arrive as the table's scan names (`SCAN_W`,
//! `SCAN_MOUSE_X`); the caller maps its platform's keys onto them.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use parkan_formats::controls::{
    self, Action, CICLS_CAMERA, CICLS_DETECTSHIELD, CICLS_REPAIRSYS, CICLS_TURRET, CIS_INV, CIS_OFF, CIS_ON,
    CIS_SWITCH_INV, MCMD_ANGLE_X, MCMD_ANGLE_Y, MCMD_ANGLE_Z, MCMD_FORWARD, MCMD_LEFT, MCMD_RIGHT,
    MCMD_WALK_B, MCMD_WALK_F, UNKNOWN_CLASS,
};

use crate::motion::Body;

pub const MOUSE_X: &str = "SCAN_MOUSE_X";
pub const MOUSE_Y: &str = "SCAN_MOUSE_Y";
pub const SHIFT: &str = "SCAN_LSHIFT";
/// The mouse filter (`World3D.dll:0x1000f24b`): this much of the new counts, the
/// rest of the last value, and Y a further 1.2.
pub const FILTER_NEW: f32 = 0.95;
pub const FILTER_OLD: f32 = 0.05;
pub const Y_GAIN: f32 = 1.2;
/// A filtered count adds this much times the row's magnitude (`0x10010a50`).
pub const COUNT_SCALE: f32 = 0.006;
/// `Iron_3D.ini`'s `MOUSE_SENS` is a percentage.
pub const SENSITIVITY_SCALE: f32 = 0.01;
/// The strafe angle, or half of it while already walking (`0x1079523c`, `0x10795240`).
pub const STRAFE: f32 = FRAC_PI_2;
pub const STRAFE_WALKING: f32 = FRAC_PI_4;

/// The mouse's signs. The game's are +1 on both (`MOUSE_REV_Y` clear), and its attitude
/// integrator turns the hull by −(z − 0.5) × 2π (`Control.dll:0x10014858`); here the
/// integrator does not negate, so mouse X's sign carries that negation instead. On
/// screen the two agree, as docs/30-turrets.md derives: the mouse moved right turns the
/// hull right, and moved down lowers the sight.
///
/// The negation is the hull's alone: a turret's stored yaw takes the game's sign, which its
/// channel's mounting and invert flag turn to the right on screen, and the turret lock's
/// integrator reads it as the game's does (docs/30, "The hull follows the turret").
pub const INVERT: [f32; 2] = [-1.0, 1.0];

/// The mouse filter's memory.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseFilter {
    previous: [f32; 2],
}

impl MouseFilter {
    /// `m = counts × sensitivity × 0.95 + 0.05 × m_previous`, Y also × 1.2; no
    /// movement at all clears the memory.
    pub fn filter(&mut self, counts: [f32; 2], sensitivity: f32) -> [f32; 2] {
        if counts == [0.0, 0.0] {
            self.previous = [0.0; 2];
            return [0.0; 2];
        }
        let m = [
            counts[0] * sensitivity * FILTER_NEW + FILTER_OLD * self.previous[0],
            counts[1] * sensitivity * Y_GAIN * FILTER_NEW + FILTER_OLD * self.previous[1],
        ];
        self.previous = m;
        m
    }
}

/// What one filtered axis adds through a row: `clamp(m × 0.006, −1, 1) × invert × magnitude`.
pub fn axis_delta(m: f32, invert: f32, magnitude: f32) -> f32 {
    (m * COUNT_SCALE).clamp(-1.0, 1.0) * invert * magnitude
}

/// The triples a row can edit.
pub struct Hands<'a> {
    pub body: &'a mut Body,
    /// The turret's stored target (`+0x9c`).
    pub turret: &'a mut [f32; 3],
    /// The camera's free look (`+0x94`).
    pub camera: &'a mut [f32; 3],
}

/// The rows of one table, and what is held.
#[derive(Clone, Debug, PartialEq)]
pub struct Pilot {
    pub rows: Vec<Action>,
    pub sensitivity: f32,
    pub invert: [f32; 2],
    filter: MouseFilter,
    shift: bool,
    /// What the walking handler last set the command's y to.
    walk: f32,
    /// The strafe keys held: left, right.
    strafing: [bool; 2],
    /// The ramp rows active, each with the game time it became so (row `+0x80`,
    /// `+0x84`).
    active: Vec<(Action, f64)>,
    /// The game clock as the last input update saw it.
    clock_ms: f64,
    /// The fire button is held: `MCMD_STATE` to the selected guns.
    pub fire: bool,
    /// `MCMD_SELECT` rows not yet taken: a gun's number, or −1 for all of them.
    pub selects: Vec<i32>,
    /// The switches the table's state rows turn, which the HUD's indicators show.
    pub switches: Switches,
    /// The keys down, in the order they went down.
    held: Vec<String>,
}

/// A unit's switched systems, as the input table's `MCMD_STATE` rows leave them.
///
/// STAND-IN: docs/35-hud.md#the-indicators--read-and-seen -- the repair system, the detection
/// shield's camouflage and the camera's infrared are not simulated: a row turns only the
/// switch, which does nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Switches {
    /// The repair system's `0x20`: `CIS_SWITCHON` sets it, `CIS_SWITCHOFF` clears it and
    /// `CIS_SWITCH_INV` flips it (`Control.dll:0x10022c90`, docs/26).
    pub repair: bool,
    /// The detection shield's camouflage: `0x1000` on, `0x2000` off, `0x4000` flips it
    /// (`0x10026570`, docs/25).
    pub camouflage: bool,
    /// The camera's infrared: the same three bits (`0x10023a00`).
    pub infrared: bool,
}

impl Switches {
    /// A state row's bits to the switch of its class.
    pub fn state(&mut self, class: i32, bits: i32) {
        let turn = |on: &mut bool| match bits {
            CIS_ON => *on = true,
            CIS_OFF => *on = false,
            CIS_INV => *on = !*on,
            _ => {}
        };
        match class {
            CICLS_REPAIRSYS => match bits {
                CIS_SWITCH_INV => self.repair = !self.repair,
                0 => self.repair = false,
                _ => self.repair = true,
            },
            CICLS_DETECTSHIELD => turn(&mut self.camouflage),
            CICLS_CAMERA => turn(&mut self.infrared),
            _ => {}
        }
    }
}

/// `value` moved toward `target` by at most `reach`, never past it.
fn approach(value: f32, target: f32, reach: f32) -> f32 {
    let gap = target - value;
    if gap.abs() <= reach { target } else { value + reach.copysign(gap) }
}

impl Pilot {
    /// `mouse_sens` is `Iron_3D.ini`'s `MOUSE_SENS`.
    pub fn new(rows: Vec<Action>, mouse_sens: f32) -> Self {
        Self {
            rows,
            sensitivity: mouse_sens * SENSITIVITY_SCALE,
            invert: INVERT,
            filter: MouseFilter::default(),
            shift: false,
            walk: 0.0,
            strafing: [false; 2],
            active: Vec::new(),
            clock_ms: 0.0,
            fire: false,
            selects: Vec::new(),
            switches: Switches::default(),
            held: Vec::new(),
        }
    }

    /// Every key still down comes up, as `stdClearKeyboard` leaves the keyboard when the
    /// player's view changes hands (docs/39-boarding.md, "Boarding").
    pub fn release_all(&mut self, hands: &mut Hands) {
        for key in std::mem::take(&mut self.held).into_iter().rev() {
            self.key(&key, false, hands);
        }
    }

    /// The rows a chord selects: the Shift rows while Shift is held.
    ///
    /// STAND-IN: docs/14-controls.md#the-table -- how a chord with no row of its own
    /// is resolved is not read; it falls back to the plain rows.
    fn matching(&self, key: &str, pressed: bool) -> Vec<Action> {
        let find = |modifier: &str| -> Vec<Action> {
            self.rows
                .iter()
                .filter(|r| r.key == key && r.pressed == pressed && r.modifier == modifier)
                .cloned()
                .collect()
        };
        if self.shift && key != SHIFT {
            let shifted = find(SHIFT);
            if !shifted.is_empty() {
                return shifted;
            }
        }
        find(controls::NO_MODIFIER)
    }

    /// A key or button going down or up.
    ///
    /// A row with a ramp time is not run by the event: the event makes the key's
    /// matching row active, stamped with the game clock, and clears its other row
    /// (`World3D.dll:0x1000f5a4`), and the input update runs it.
    ///
    /// STAND-IN: docs/14-controls.md#a-row-that-stays-down--read -- every active row is
    /// read to run again on each input update, but what the walk, strafe and weapon
    /// handlers do when run again while their key is held is not (a select row run again
    /// would toggle its gun each update); a row without a ramp time runs once, as its key
    /// goes down or comes up.
    pub fn key(&mut self, key: &str, pressed: bool, hands: &mut Hands) {
        let rows = self.matching(key, pressed);
        self.held.retain(|k| k != key);
        if pressed {
            self.held.push(key.to_owned());
        }
        if key == SHIFT {
            self.shift = pressed;
        }
        self.active.retain(|(row, _)| row.key != key);
        for row in &rows {
            if row.ramp_time != 0 {
                self.active.push((row.clone(), self.clock_ms));
            } else {
                self.apply(row, None, hands);
            }
        }
    }

    /// The input update (`World3D.dll:0x1000f477`): every active ramp row is run again,
    /// moving its value toward the row's magnitude by ramp × min(1, held ms ÷ ramp
    /// time), never past it (`0x10010a50`). A press row stays active while its key is
    /// down; a release row clears itself once it arrives.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how often `World3D.dll`'s input
    /// update runs is not read, and it paces the ramp; the caller runs it once a
    /// rendered frame, and once a tick where nothing is rendered.
    pub fn update(&mut self, now_ms: f64, hands: &mut Hands) {
        self.clock_ms = now_ms;
        for (row, since) in std::mem::take(&mut self.active) {
            let held = (now_ms - since).max(0.0) as f32;
            let reach = row.ramp * (held / row.ramp_time as f32).min(1.0);
            let arrived = match row.code() {
                MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD => {
                    self.walk = approach(hands.body.command[1], row.value, reach);
                    hands.body.command[1] = self.walk;
                    self.walk == row.value
                }
                _ => true,
            };
            if row.pressed || !arrived {
                self.active.push((row, since));
            }
        }
    }

    /// One input tick's mouse movement, in counts.
    pub fn mouse(&mut self, counts: [f32; 2], hands: &mut Hands) {
        let m = self.filter.filter(counts, self.sensitivity);
        for (axis, key) in [(0, MOUSE_X), (1, MOUSE_Y)] {
            if m[axis] == 0.0 {
                continue;
            }
            for row in &self.matching(key, true) {
                let invert = if axis == 0 && row.class_id() == CICLS_TURRET {
                    -self.invert[axis]
                } else {
                    self.invert[axis]
                };
                let amount = axis_delta(m[axis], invert, row.value);
                self.apply(row, Some(amount), hands);
            }
        }
    }

    /// `0x1000fb40`: a row's call. An axis row adds `amount`; any other row sets.
    fn apply(&mut self, row: &Action, amount: Option<f32>, hands: &mut Hands) {
        match row.code() {
            MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD => {
                self.walk = row.value;
                hands.body.command[1] = row.value;
            }
            code @ (MCMD_LEFT | MCMD_RIGHT) => self.strafe(code == MCMD_LEFT, row.pressed, hands.body),
            // `World3D.dll:0x1001059b`: the command's z, which a flyer climbs and sinks by
            // (docs/39-boarding.md, "Driving").
            controls::MCMD_UP | controls::MCMD_DOWN => hands.body.command[2] = row.value,
            // `World3D.dll:0x1000fe77` → `IControl` slot 4: the spin's z, a fraction of the live
            // turn rate, and no normalised turn pending (`Control.dll:0x10004491`).
            controls::MCMD_ROTATE_Z => {
                hands.body.spin_set[2] = row.value;
                hands.body.turn_pending = false;
            }
            // `0x1000fcbf`: the key going down switches the turret lock, property 179.
            controls::MCMD_LOCK if row.class_id() == CICLS_TURRET && row.pressed => {
                hands.body.turret_lock = !hands.body.turret_lock;
            }
            code @ (MCMD_ANGLE_X | MCMD_ANGLE_Y | MCMD_ANGLE_Z) => {
                let component = match code {
                    MCMD_ANGLE_X => 0,
                    MCMD_ANGLE_Y => 1,
                    _ => 2,
                };
                let class = row.class_id();
                let triple = match class {
                    // `SetNormAngle` (`0x10004500`) sets the pending flag.
                    UNKNOWN_CLASS => {
                        hands.body.turn_pending = true;
                        &mut hands.body.pending
                    }
                    CICLS_TURRET => &mut *hands.turret,
                    CICLS_CAMERA => &mut *hands.camera,
                    _ => return,
                };
                let v = match amount {
                    Some(add) => triple[component] + add,
                    None => row.value,
                };
                let v = if row.wraps() { v - v.floor() } else { v.clamp(0.0, 1.0) };
                // `0x1002ec23`: a change to the turret's yaw is added into the lead, wrapping.
                if class == CICLS_TURRET && component == 0 {
                    let lead = hands.body.lead + (v - triple[0]);
                    hands.body.lead = lead - lead.floor();
                }
                triple[component] = v;
            }
            controls::MCMD_STATE if row.class_id() == controls::CICLS_MULTIGUN => {
                self.fire = row.pressed && row.bits() != 0;
            }
            controls::MCMD_SELECT if row.class_id() == controls::CICLS_MULTIGUN => {
                self.selects.push(row.index);
            }
            controls::MCMD_STATE if row.pressed => self.switches.state(row.class_id(), row.bits()),
            _ => {}
        }
    }

    /// `MCMD_LEFT` or `MCMD_RIGHT` going down or up: `SetTangAccel` and
    /// `SetStrafeAngle` (docs/24-motion.md, "From input to motion"). Going down sets the
    /// command's y to ±1 with the sign of the current direction, and the angle to +π/2
    /// left or −π/2 right, ±π/4 while already walking; the angle is set as the key goes
    /// down and not worked out again. Coming up returns the angle to 0, and the command
    /// to 0 if nothing else is held.
    fn strafe(&mut self, left: bool, pressed: bool, body: &mut Body) {
        let side = usize::from(!left);
        self.strafing[side] = pressed;
        if pressed {
            body.command[1] = if body.command[1] < 0.0 { -1.0 } else { 1.0 };
            let angle = if self.walk != 0.0 { STRAFE_WALKING } else { STRAFE };
            body.strafe = if left { angle } else { -angle };
        } else {
            body.strafe = 0.0;
            if self.walk == 0.0 && !self.strafing[1 - side] {
                body.command[1] = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_rows_turn_repair_camouflage_and_infrared_as_their_classes_take_the_bits() {
        let mut s = Switches::default();
        s.state(CICLS_REPAIRSYS, CIS_SWITCH_INV);
        s.state(CICLS_DETECTSHIELD, CIS_INV);
        s.state(CICLS_CAMERA, CIS_ON);
        assert_eq!(s, Switches { repair: true, camouflage: true, infrared: true });
        s.state(CICLS_REPAIRSYS, CIS_SWITCH_INV);
        s.state(CICLS_DETECTSHIELD, CIS_INV);
        s.state(CICLS_CAMERA, CIS_OFF);
        assert_eq!(s, Switches::default());
    }
    use glam::Vec3;

    fn hero_table() -> Vec<Action> {
        let text = b"KEY   SCAN_NULL SCAN_A 1 CICLS_UNKNOWN MCMD_LEFT  1.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_A 0 CICLS_UNKNOWN MCMD_LEFT  0.0 1 0 0.0 0
KEY   SCAN_NULL SCAN_D 1 CICLS_UNKNOWN MCMD_RIGHT  1.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_D 0 CICLS_UNKNOWN MCMD_RIGHT  0.0 1 0 0.0 0
KEY   SCAN_NULL SCAN_G_PLUS 1 CICLS_UNKNOWN MCMD_FORWARD  1.0 0 0 0.05 1000
KEY   SCAN_NULL SCAN_G_SUB 1 CICLS_UNKNOWN MCMD_FORWARD -1.0 0 0 0.05 1000
KEY   SCAN_NULL SCAN_G_SLASH 1 CICLS_UNKNOWN MCMD_FORWARD  0.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_W 1 CICLS_UNKNOWN MCMD_WALK_F  1.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_W 0 CICLS_UNKNOWN MCMD_WALK_F  0.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_S 1 CICLS_UNKNOWN MCMD_WALK_B -1.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_S 0 CICLS_UNKNOWN MCMD_WALK_B  0.0 0 0 0.0 0
MOUSE SCAN_NULL SCAN_MOUSE_X 1 CICLS_UNKNOWN MCMD_ANGLE_Z 0.15 0 MAN_WRAP 0.0 0
MOUSE SCAN_NULL SCAN_MOUSE_Y 1 CICLS_TURRET MCMD_ANGLE_Y 0.25 1 MAN_NOTWRAP 0.0 0
MOUSE SCAN_LSHIFT SCAN_MOUSE_X 1 CICLS_CAMERA MCMD_ANGLE_X 0.1 1 MAN_WRAP 0.0 0
KEY   SCAN_NULL SCAN_LSHIFT 0 CICLS_CAMERA MCMD_ANGLE_X 0.5 1 MAN_NOTWRAP 0.0 0
MOUSE SCAN_NULL SCAN_LMOUSE 1 CICLS_MULTIGUN MCMD_STATE 0.0 -1 CIS_CONTINUEFIGHT 0.0 0
MOUSE SCAN_NULL SCAN_LMOUSE 0 CICLS_MULTIGUN MCMD_STATE 0.0 -1 CIS_SWITCHOFF 0.0 0
KEY   SCAN_NULL SCAN_W_3 1 CICLS_MULTIGUN MCMD_SELECT 0.0 3 0 0.0 0
";
        controls::parse(text, "hero.tbl").unwrap()
    }

    struct Rig {
        body: Body,
        turret: [f32; 3],
        camera: [f32; 3],
    }

    impl Rig {
        fn hands(&mut self) -> Hands<'_> {
            Hands { body: &mut self.body, turret: &mut self.turret, camera: &mut self.camera }
        }
    }

    fn rig() -> Rig {
        Rig { body: Body::new(Vec3::ZERO, 0.0), turret: [0.5, 0.7273, 0.5], camera: [0.5; 3] }
    }

    #[test]
    fn the_filter_keeps_a_twentieth_and_forgets_when_still() {
        let mut f = MouseFilter::default();
        assert_eq!(f.filter([10.0, 10.0], 1.0), [9.5, 11.4]);
        let m = f.filter([10.0, 0.0], 1.0);
        assert!((m[0] - (9.5 + 0.475)).abs() < 1e-5 && (m[1] - 0.57).abs() < 1e-5);
        assert_eq!(f.filter([0.0, 0.0], 1.0), [0.0, 0.0]);
        assert_eq!(f.filter([0.0, 10.0], 1.0)[1], 11.4);
    }

    #[test]
    fn walking_and_strafing_set_the_command_and_the_angle_as_each_key_goes_down() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0);
        pilot.key("SCAN_A", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (1.0, FRAC_PI_4), "a quarter while walking");
        pilot.key("SCAN_W", false, &mut r.hands());
        assert_eq!(
            (r.body.command[1], r.body.strafe),
            (0.0, FRAC_PI_4),
            "W's release sends 0; A's angle stays"
        );
        pilot.key("SCAN_A", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));

        pilot.key("SCAN_D", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (1.0, -FRAC_PI_2), "a half from standing");
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.strafe, -FRAC_PI_2, "not worked out again from what is held");
        pilot.key("SCAN_W", false, &mut r.hands());
        pilot.key("SCAN_D", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));

        pilot.key("SCAN_S", true, &mut r.hands());
        assert_eq!(r.body.command[1], -1.0);
        pilot.key("SCAN_A", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (-1.0, FRAC_PI_4), "backing up: not mirrored");
        pilot.key("SCAN_A", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (-1.0, 0.0), "S is still held");
    }

    #[test]
    fn a_cruise_row_held_steps_the_command_on_each_update_by_a_step_that_grows_over_a_second() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_G_PLUS", true, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0, "the event only makes the row active");
        pilot.update(0.0, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0, "held no time at all");
        pilot.update(500.0, &mut r.hands());
        assert!((r.body.command[1] - 0.025).abs() < 1e-6, "{}", r.body.command[1]);
        pilot.update(2000.0, &mut r.hands());
        assert!((r.body.command[1] - 0.075).abs() < 1e-6, "{}", r.body.command[1]);
        for k in 0..30 {
            pilot.update(2050.0 + 50.0 * f64::from(k), &mut r.hands());
        }
        assert_eq!(r.body.command[1], 1.0, "never past the magnitude");
        pilot.key("SCAN_G_PLUS", false, &mut r.hands());
        pilot.key("SCAN_G_SUB", true, &mut r.hands());
        // Down at 3500, the last update's clock: 0.005 at 100 ms held, 0.05 at 1100.
        pilot.update(3600.0, &mut r.hands());
        pilot.update(4600.0, &mut r.hands());
        assert!((r.body.command[1] - 0.945).abs() < 1e-6, "{}", r.body.command[1]);
        pilot.key("SCAN_G_SUB", false, &mut r.hands());
        pilot.update(9000.0, &mut r.hands());
        assert!((r.body.command[1] - 0.945).abs() < 1e-6, "letting go leaves it where it got to");
        pilot.key("SCAN_G_SLASH", true, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0);
    }

    #[test]
    fn the_mouse_turns_the_hull_tilts_the_turret_and_with_shift_moves_the_camera() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.mouse([10.0, 10.0], &mut r.hands());
        // 10 counts: 9.5 × 0.006 × 0.15 on the hull, 11.4 × 0.006 × 0.25 on the turret.
        assert!((r.body.pending[2] - (0.5 - 0.00855)).abs() < 1e-6, "{}", r.body.pending[2]);
        assert!((r.turret[1] - (0.7273 + 0.0171)).abs() < 1e-5, "{}", r.turret[1]);
        pilot.key(SHIFT, true, &mut r.hands());
        pilot.mouse([0.0, 0.0], &mut r.hands());
        pilot.mouse([10.0, 0.0], &mut r.hands());
        assert!((r.camera[0] - (0.5 - 0.0057)).abs() < 1e-5, "{}", r.camera[0]);
        pilot.key(SHIFT, false, &mut r.hands());
        assert_eq!(r.camera[0], 0.5, "releasing Shift centres the camera");
        pilot.key("SCAN_LMOUSE", true, &mut r.hands());
        assert!(pilot.fire);
        pilot.key("SCAN_LMOUSE", false, &mut r.hands());
        assert!(!pilot.fire);
        pilot.key("SCAN_W_3", true, &mut r.hands());
        assert_eq!(pilot.selects, vec![3]);
    }

    fn machine_table() -> Vec<Action> {
        let text = b"KEY   SCAN_NULL SCAN_COMMA 1 CICLS_UNKNOWN MCMD_ROTATE_Z  0.7 0 0 0.0 0
KEY   SCAN_NULL SCAN_COMMA 0 CICLS_UNKNOWN MCMD_ROTATE_Z  0.0 0 0 0.0 0
MOUSE SCAN_NULL SCAN_MOUSE_X 1 CICLS_TURRET MCMD_ANGLE_X 0.15 1 MAN_WRAP 0.0 0
MOUSE SCAN_NULL SCAN_MOUSE_Y 1 CICLS_TURRET MCMD_ANGLE_Y 0.25 1 MAN_NOTWRAP 0.0 0
KEY   SCAN_NULL SCAN_G_5 1 CICLS_TURRET MCMD_LOCK 0.5 1 MAN_NOTWRAP 0.0 0
KEY   SCAN_NULL SCAN_R 1 CICLS_UNKNOWN MCMD_UP  1.0 0 0 0.0 0
";
        controls::parse(text, "m2.tbl").unwrap()
    }

    #[test]
    fn a_machines_mouse_x_turns_its_turret_the_games_way_and_into_the_lead_and_y_never_climbs() {
        let mut pilot = Pilot::new(machine_table(), 100.0);
        let mut r = rig();
        pilot.mouse([10.0, 10.0], &mut r.hands());
        // 10 counts right: 9.5 × 0.006 × 0.15 added to the turret's stored yaw, the game's sign,
        // and the same change into the lead; mouse Y tilts the turret and nothing else.
        assert!((r.turret[0] - (0.5 + 0.00855)).abs() < 1e-6, "{}", r.turret[0]);
        assert!((r.body.lead - (0.5 + 0.00855)).abs() < 1e-6, "{}", r.body.lead);
        assert!((r.turret[1] - (0.7273 + 0.0171)).abs() < 1e-5, "{}", r.turret[1]);
        assert_eq!(r.body.command, [0.0; 3], "the pitch asks for no climb");
        assert!(!r.body.turn_pending, "no normalised turn: the turret leads");
        // Across the wrap the lead wraps with it.
        r.turret[0] = 0.999;
        r.body.lead = 0.999;
        pilot.mouse([0.0, 0.0], &mut r.hands());
        pilot.mouse([10.0, 0.0], &mut r.hands());
        assert!((r.body.lead - (0.999 + 0.00855 - 1.0)).abs() < 1e-5, "{}", r.body.lead);
        // Keypad 5 switches the lock; the spin key sets a fraction and clears a pending turn.
        pilot.key("SCAN_G_5", true, &mut r.hands());
        assert!(r.body.turret_lock);
        pilot.key("SCAN_G_5", true, &mut r.hands());
        assert!(!r.body.turret_lock);
        r.body.turn_pending = true;
        pilot.key("SCAN_COMMA", true, &mut r.hands());
        assert_eq!((r.body.spin_set[2], r.body.turn_pending), (0.7, false));
        pilot.key("SCAN_COMMA", false, &mut r.hands());
        assert_eq!(r.body.spin_set[2], 0.0);
        pilot.key("SCAN_R", true, &mut r.hands());
        assert_eq!(r.body.command[2], 1.0, "only R climbs");
    }
}
