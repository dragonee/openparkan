//! The player's hands: an input table's rows turned into what they move.
//! `docs/14-controls.md`, "From a row to a command", and `docs/24-motion.md`,
//! "From input to motion".
//!
//! Keys and mouse axes arrive as the table's scan names (`SCAN_W`,
//! `SCAN_MOUSE_X`); the caller maps its platform's keys onto them.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use parkan_formats::controls::{
    self, Action, CICLS_CAMERA, CICLS_TURRET, MCMD_ANGLE_X, MCMD_ANGLE_Y, MCMD_ANGLE_Z, MCMD_FORWARD,
    MCMD_LEFT, MCMD_RIGHT, MCMD_WALK_B, MCMD_WALK_F, UNKNOWN_CLASS,
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

/// STAND-IN: docs/14-controls.md#from-a-row-to-a-command--read-and-measured -- the
/// signs `SetInverseMotion` starts with are not read; chosen so the mouse moved right
/// turns right and moved down looks down.
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
    /// The walk command held: 1, −1 or 0.
    walk: f32,
    /// 1 while strafing left, −1 right.
    strafe: f32,
    /// The fire button is held: `MCMD_STATE` to the selected guns.
    pub fire: bool,
    /// `MCMD_SELECT` rows not yet taken: a gun's number, or −1 for all of them.
    pub selects: Vec<i32>,
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
            strafe: 0.0,
            fire: false,
            selects: Vec::new(),
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
    pub fn key(&mut self, key: &str, pressed: bool, hands: &mut Hands) {
        let rows = self.matching(key, pressed);
        if key == SHIFT {
            self.shift = pressed;
        }
        for row in &rows {
            self.apply(row, None, hands);
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
                let amount = axis_delta(m[axis], self.invert[axis], row.value);
                self.apply(row, Some(amount), hands);
            }
        }
    }

    /// `0x1000fb40`: a row's call. An axis row adds `amount`; any other row sets.
    fn apply(&mut self, row: &Action, amount: Option<f32>, hands: &mut Hands) {
        match row.code() {
            MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD if row.ramp == 0.0 => {
                self.walk = row.value;
                self.drive(hands.body);
            }
            MCMD_LEFT => {
                self.strafe = row.value;
                self.drive(hands.body);
            }
            MCMD_RIGHT => {
                self.strafe = -row.value;
                self.drive(hands.body);
            }
            code @ (MCMD_ANGLE_X | MCMD_ANGLE_Y | MCMD_ANGLE_Z) => {
                let component = match code {
                    MCMD_ANGLE_X => 0,
                    MCMD_ANGLE_Y => 1,
                    _ => 2,
                };
                let triple = match row.class_id() {
                    UNKNOWN_CLASS => &mut hands.body.pending,
                    CICLS_TURRET => &mut *hands.turret,
                    CICLS_CAMERA => &mut *hands.camera,
                    _ => return,
                };
                let v = match amount {
                    Some(add) => triple[component] + add,
                    None => row.value,
                };
                triple[component] = if row.wraps() { v - v.floor() } else { v.clamp(0.0, 1.0) };
            }
            controls::MCMD_STATE if row.class_id() == controls::CICLS_MULTIGUN => {
                self.fire = row.pressed && row.bits() != 0;
            }
            controls::MCMD_SELECT if row.class_id() == controls::CICLS_MULTIGUN => {
                self.selects.push(row.index);
            }
            // STAND-IN: docs/24-motion.md#from-input-to-motion--read-and-measured -- what
            // the keypad cruise's ramp does is not read; ramp rows do nothing.
            _ => {}
        }
    }

    /// `SetTangAccel` and `SetStrafeAngle` from what is held: walking sets the
    /// command's y; strafing sets it to ±1 in the current direction and turns by ±π/2,
    /// ±π/4 while walking.
    fn drive(&self, body: &mut Body) {
        let direction = if self.walk < 0.0 { -1.0 } else { 1.0 };
        body.command[1] = if self.walk != 0.0 {
            self.walk
        } else if self.strafe != 0.0 {
            direction
        } else {
            0.0
        };
        // STAND-IN: docs/24-motion.md#from-input-to-motion--read-and-measured -- the
        // angle is set as each key goes down; here it follows whatever is held, and
        // backing up mirrors it so strafing left still goes left.
        let angle = if self.walk != 0.0 { STRAFE_WALKING } else { STRAFE };
        body.strafe = self.strafe * direction * angle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn hero_table() -> Vec<Action> {
        let text = b"KEY   SCAN_NULL SCAN_A 1 CICLS_UNKNOWN MCMD_LEFT  1.0 0 0 0.0 0
KEY   SCAN_NULL SCAN_A 0 CICLS_UNKNOWN MCMD_LEFT  0.0 1 0 0.0 0
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
    fn walking_and_strafing_set_the_command_and_the_angle() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0);
        pilot.key("SCAN_A", true, &mut r.hands());
        assert_eq!(r.body.strafe, FRAC_PI_4);
        pilot.key("SCAN_W", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (1.0, FRAC_PI_2));
        pilot.key("SCAN_A", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));
        pilot.key("SCAN_S", true, &mut r.hands());
        assert_eq!(r.body.command[1], -1.0);
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
}
