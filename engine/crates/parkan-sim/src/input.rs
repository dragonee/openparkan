//! The player's hands: an input table's rows turned into what they move.
//! `docs/14-controls.md`, "From a row to a command", and `docs/24-motion.md`,
//! "From input to motion".
//!
//! Keys and mouse axes arrive as the table's scan names (`SCAN_W`,
//! `SCAN_MOUSE_X`); the caller maps its platform's keys onto them.

use std::f32::consts::FRAC_PI_2;

use parkan_formats::controls::{
    self, Action, CICLS_CAMERA, CICLS_DETECTSHIELD, CICLS_REPAIRSYS, CICLS_TURRET, CIS_INV, CIS_OFF, CIS_ON,
    CIS_SWITCH_INV, MCMD_ANGLE_X, MCMD_ANGLE_Y, MCMD_ANGLE_Z, MCMD_FORWARD, MCMD_LEFT, MCMD_RIGHT,
    MCMD_WALK_B, MCMD_WALK_F, UNKNOWN_CLASS,
};

use crate::motion::Body;

pub const MOUSE_X: &str = "SCAN_MOUSE_X";
pub const MOUSE_Y: &str = "SCAN_MOUSE_Y";
pub const SHIFT: &str = "SCAN_LSHIFT";
/// The walk-straight key, the engine's own: no shipped table holds a row for it (docs/14,
/// "The table"), so the pilot answers it itself. It toggles the walk forward on and off, and
/// any strafe or a walk back turns it off again.
pub const WALK_STRAIGHT: &str = "SCAN_Q";
/// The mouse filter (`World3D.dll:0x1000f24b`): this much of the new counts, the
/// rest of the last value, and Y a further 1.2.
pub const FILTER_NEW: f32 = 0.95;
pub const FILTER_OLD: f32 = 0.05;
pub const Y_GAIN: f32 = 1.2;
/// A filtered count adds this much times the row's magnitude (`0x10010a50`).
pub const COUNT_SCALE: f32 = 0.006;
/// `Iron_3D.ini`'s `MOUSE_SENS` is a percentage.
pub const SENSITIVITY_SCALE: f32 = 0.01;
/// The strafe angle from standing (`World3D.dll:0x10020b60`); walking takes half of it, with
/// the sign of the way the unit walks.
pub const STRAFE: f32 = FRAC_PI_2;
/// A walk row walks when its magnitude is at least this (`0x10020b68`).
pub const WALKING: f32 = 1e-4;
/// A command's y within this of 0 counts as standing to a strafe key (`0x10020b5c`,
/// `0x10020250`).
pub const STILL: f32 = 0.001;

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
    /// The walk keys held: W, S (the input context's `+0x20`, `+0x24`).
    walks: [bool; 2],
    /// Whether the last walk row to run walks (`0x10795244`).
    walking: bool,
    /// The strafe keys held: left, right (`0x1079523c`, `0x10795240`).
    strafing: [bool; 2],
    /// The ramp rows active, each with the game time it became so (row `+0x80`,
    /// `+0x84`).
    active: Vec<(Action, f64)>,
    /// The game clock as the last input update saw it.
    clock_ms: f64,
    /// The whole millisecond the last input update ran at: the update is passed over when the
    /// game clock has not moved since (`World3D.dll:0x1000ec90`).
    last_update_ms: Option<u32>,
    /// The fire button is held: `MCMD_STATE` to the selected guns.
    pub fire: bool,
    /// `MCMD_SELECT` rows not yet taken: a gun's number, or −1 for all of them.
    pub selects: Vec<i32>,
    /// The switches the table's state rows turn, which the HUD's indicators show.
    pub switches: Switches,
    /// The keys down, in the order they went down.
    held: Vec<String>,
    /// Walk straight ([`WALK_STRAIGHT`]): the command is held forward while no walk or strafe
    /// key of its own holds it.
    pub walk_straight: bool,
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
            walks: [false; 2],
            walking: false,
            strafing: [false; 2],
            active: Vec::new(),
            clock_ms: 0.0,
            last_update_ms: None,
            fire: false,
            selects: Vec::new(),
            switches: Switches::default(),
            held: Vec::new(),
            walk_straight: false,
        }
    }

    /// Every key still down comes up, as `stdClearKeyboard` leaves the keyboard when the
    /// player's view changes hands (docs/39-boarding.md, "Boarding").
    pub fn release_all(&mut self, hands: &mut Hands) {
        // Walk straight comes up with them: nothing is holding the unit forward any more.
        if std::mem::take(&mut self.walk_straight) {
            hands.body.command[1] = 0.0;
        }
        for key in std::mem::take(&mut self.held).into_iter().rev() {
            self.key(&key, false, hands);
        }
    }

    /// The rows a chord selects (`World3D.dll:0x1000f5ee`-`0x1000f6d9`,
    /// docs/14-controls.md, "A chord with no row of its own").
    ///
    /// A row whose key and press/release match fires when its modifier is held. A **plain**
    /// row fires unless one of the modifiers that another row pairs with **the same key** is
    /// held: the table's load lists those on the plain row (`0x1000bd31`), and the lookup
    /// walks that list. So there is no best match and no search: a chord with no row of its
    /// own falls through to the plain row, and it is only a key that has a row under some
    /// other modifier that goes silent.
    ///
    /// *Measured*: `SCAN_LSHIFT` is the only modifier the three shipped tables use, on four
    /// rows each, and the only plain rows it blocks are mouse X's and mouse Y's -- two per
    /// table. Shift with any of the keyboard's keys reaches the plain row.
    fn matching(&self, key: &str, pressed: bool) -> Vec<Action> {
        let held = |modifier: &str| match modifier {
            SHIFT => self.shift,
            other => self.held.iter().any(|k| k == other),
        };
        let blocked = self
            .rows
            .iter()
            .any(|r| r.key == key && r.modifier != controls::NO_MODIFIER && held(&r.modifier));
        self.rows
            .iter()
            .filter(|r| r.key == key && r.pressed == pressed)
            .filter(|r| if r.modifier == controls::NO_MODIFIER { !blocked } else { held(&r.modifier) })
            .cloned()
            .collect()
    }

    /// A key or button going down or up.
    ///
    /// A row with a ramp time is not run by the event: the event makes the key's
    /// matching row active, stamped with the game clock, and clears its other row
    /// (`World3D.dll:0x1000f5a4`), and the input update runs it. A row without one runs
    /// once, as its key goes down or comes up: the handler clears it after its run
    /// (`0x100109e7`, docs/14-controls.md, "A row that stays down").
    pub fn key(&mut self, key: &str, pressed: bool, hands: &mut Hands) {
        let rows = self.matching(key, pressed);
        self.held.retain(|k| k != key);
        if pressed {
            self.held.push(key.to_owned());
        }
        if key == SHIFT {
            self.shift = pressed;
        }
        // The walk-straight key going down turns the toggle over; turned off, the unit stops
        // where no walk key of its own is holding it up.
        if key == WALK_STRAIGHT && pressed {
            self.walk_straight = !self.walk_straight;
            if !self.walk_straight && self.walks == [false; 2] {
                hands.body.command[1] = 0.0;
            }
        }
        self.active.retain(|(row, _)| row.key != key);
        for row in &rows {
            if row.ramp_time != 0 {
                self.active.push((row.clone(), self.clock_ms));
            } else {
                self.apply(row, None, hands);
            }
        }
        self.hold_walk_straight(hands.body);
    }

    /// The input update (`World3D.dll:0x1000f477`): every active ramp row is run again,
    /// moving its value toward the row's magnitude by ramp × min(1, held ms ÷ ramp
    /// time), never past it (`0x10010a50`). A press row stays active while its key is
    /// down; a release row clears itself once it arrives.
    ///
    /// **It runs once a game frame** (docs/24, "From input to motion"): the world's frame
    /// hands every object message 1 with the game clock, the machine's Wizard passes it to the
    /// controls manager, and the manager runs the update (`World3D.dll:0x1000ecd5`). The
    /// mission loop calculates and renders once a pass with no cap, so the ramp's step is a
    /// step a rendered frame. The manager passes the update over while the clock stands on the
    /// same whole millisecond as the last (`0x1000ec90`), so it never runs twice in one.
    pub fn update(&mut self, now_ms: f64, hands: &mut Hands) {
        let whole = now_ms.max(0.0) as u32;
        if self.last_update_ms == Some(whole) {
            return;
        }
        self.last_update_ms = Some(whole);
        self.clock_ms = now_ms;
        for (row, since) in std::mem::take(&mut self.active) {
            let held = (now_ms - since).max(0.0) as f32;
            let reach = row.ramp * (held / row.ramp_time as f32).min(1.0);
            let arrived = match row.code() {
                MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD => {
                    let value = approach(hands.body.command[1], row.value, reach);
                    self.walk(&row, value, hands.body);
                    value == row.value
                }
                _ => true,
            };
            if row.pressed || !arrived {
                self.active.push((row, since));
            }
        }
        self.hold_walk_straight(hands.body);
    }

    /// Walk straight holds the command forward while nothing else moves the unit: no walk or
    /// strafe key down, and no walk row ramping, whose own step the toggle must not override.
    /// The value is the table's full walk forward.
    fn hold_walk_straight(&self, body: &mut Body) {
        let ramping =
            self.active.iter().any(|(r, _)| matches!(r.code(), MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD));
        if self.walk_straight && self.walks == [false; 2] && self.strafing == [false; 2] && !ramping {
            let forward =
                self.rows.iter().find(|r| r.code() == MCMD_WALK_F && r.pressed).map_or(1.0, |r| r.value);
            body.command[1] = forward;
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
            MCMD_WALK_F | MCMD_WALK_B | MCMD_FORWARD => self.walk(row, row.value, hands.body),
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

    /// The handler `MCMD_WALK_F`, `MCMD_WALK_B` and `MCMD_FORWARD` share (`World3D.dll:0x100101b2`;
    /// docs/24-motion.md, "From input to motion"). `value` is what the axis function gives
    /// the row: its magnitude, or a ramp's step.
    ///
    /// A walk key coming up while the other is held walks the other way and does nothing
    /// more. Otherwise the row walks when its magnitude is not 0, and with no strafe key held
    /// the command's y takes `value`. Under a strafe key the strafe is worked out again: a
    /// walk takes y to the sign of its magnitude and the angle to ±π/4 by it, and a stop
    /// leaves y be and turns the angle to ±π/2 by y's sign, so the strafe goes on.
    fn walk(&mut self, row: &Action, value: f32, body: &mut Body) {
        let code = row.code();
        // A walk back turns the walk-straight toggle off (the engine's own).
        if code == MCMD_WALK_B && row.pressed {
            self.walk_straight = false;
        }
        if code == MCMD_WALK_F || code == MCMD_WALK_B {
            let key = usize::from(code == MCMD_WALK_B);
            self.walks[key] = row.pressed;
            if !row.pressed && self.walks[1 - key] {
                body.command[1] = if code == MCMD_WALK_F { -1.0 } else { 1.0 };
                return;
            }
        }
        self.walking = row.value.abs() >= WALKING;
        if self.strafing == [false; 2] {
            body.command[1] = value;
            return;
        }
        let angle = if self.walking {
            body.command[1] = sign(row.value);
            body.command[1] * STRAFE * 0.5
        } else {
            sign(body.command[1]) * STRAFE
        };
        body.strafe = if self.strafing[1] { -angle } else { angle };
    }

    /// `MCMD_LEFT` (`0x10010350`) or `MCMD_RIGHT` (`0x10010457`) going down or up:
    /// `SetTangAccel` and `SetStrafeAngle` (docs/24-motion.md, "From input to motion").
    ///
    /// Going down sets the command's y to ±1 by the sign of the y it finds, and the angle to
    /// f × π/2 left or −f × π/2 right: f is 1 standing, and while walking ½ with y's sign, so
    /// backing up mirrors it and S with A walks back and to the left. Coming up turns the
    /// angle to the other strafe key's, or 0, and the command to 0 when nothing else moves.
    fn strafe(&mut self, left: bool, pressed: bool, body: &mut Body) {
        // A strafe turns the walk-straight toggle off (the engine's own).
        if pressed {
            self.walk_straight = false;
        }
        let y = body.command[1];
        let share = match self.walking {
            true if y < -STILL => -0.5,
            true if y > STILL => 0.5,
            _ => 1.0,
        };
        let angle = |left: bool| if left { share * STRAFE } else { -share * STRAFE };
        let side = usize::from(!left);
        let other = self.strafing[1 - side];
        self.strafing[side] = pressed;
        if pressed {
            body.command[1] = sign(y);
            body.strafe = angle(left);
        } else {
            body.strafe = if other { angle(!left) } else { 0.0 };
            if !other && !self.walking {
                body.command[1] = 0.0;
            }
        }
    }
}

/// ±1 by a value's sign, 0 counting as positive, as the handlers' `fcomp` with 0 reads it.
fn sign(v: f32) -> f32 {
    if v >= 0.0 { 1.0 } else { -1.0 }
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

    /// `World3D.dll:0x1000f5ee`-`0x1000f6d9`: a plain row fires unless a modifier that
    /// another row pairs with the **same key** is held. So Shift+W, which has no row,
    /// walks; Shift+mouse X, whose key does have a Shift row, moves the camera and not
    /// the hull. *Measured*: `SCAN_LSHIFT` is the only modifier the three shipped tables
    /// use, and mouse X and mouse Y are the only plain rows it blocks.
    #[test]
    fn a_chord_with_no_row_of_its_own_falls_through_to_the_plain_row() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key(SHIFT, true, &mut r.hands());
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0, "Shift+W has no row: the plain W row walks");

        // Mouse X has both, so the plain row is blocked and only the camera moves.
        let turn = r.body.pending[2];
        pilot.mouse([20.0, 0.0], &mut r.hands());
        assert_eq!(r.body.pending[2], turn, "the hull's row is blocked while Shift is held");
        assert_ne!(r.camera[0], 0.5, "the Shift row turns the camera");

        // Shift up: the same movement turns the hull and leaves the camera where it is.
        pilot.key(SHIFT, false, &mut r.hands());
        let camera = r.camera[0];
        pilot.mouse([20.0, 0.0], &mut r.hands());
        assert_ne!(r.body.pending[2], turn, "the plain row is back");
        assert_eq!(r.camera[0], camera);
    }

    #[test]
    fn walking_and_strafing_set_the_command_and_the_angle_as_each_key_goes_down() {
        use std::f32::consts::FRAC_PI_4;
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0);
        pilot.key("SCAN_A", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (1.0, FRAC_PI_4), "a quarter while walking");
        pilot.key("SCAN_W", false, &mut r.hands());
        assert_eq!(
            (r.body.command[1], r.body.strafe),
            (1.0, FRAC_PI_2),
            "W's release under A leaves y be and turns A's angle to a half: the strafe goes on"
        );
        pilot.key("SCAN_A", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));

        pilot.key("SCAN_D", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (1.0, -FRAC_PI_2), "a half from standing");
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.strafe, -FRAC_PI_4, "a walk key under a strafe works it out again");
        pilot.key("SCAN_W", false, &mut r.hands());
        pilot.key("SCAN_D", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));

        pilot.key("SCAN_S", true, &mut r.hands());
        assert_eq!(r.body.command[1], -1.0);
        pilot.key("SCAN_A", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (-1.0, -FRAC_PI_4), "backing up mirrors it");
        pilot.key("SCAN_A", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (-1.0, 0.0), "S is still held");
        pilot.key("SCAN_D", true, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (-1.0, FRAC_PI_4), "back and to the right");
        pilot.key("SCAN_S", false, &mut r.hands());
        assert_eq!(
            (r.body.command[1], r.body.strafe),
            (-1.0, FRAC_PI_2),
            "S's release under D: backwards on a hull turned left, still to the right"
        );
        pilot.key("SCAN_D", false, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));
    }

    #[test]
    fn the_walk_straight_key_holds_the_walk_forward_and_a_strafe_or_a_walk_back_turns_it_off() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();

        // Q walks forward with no key held, and holds it through the input updates.
        pilot.key(WALK_STRAIGHT, true, &mut r.hands());
        assert!(pilot.walk_straight);
        assert_eq!(r.body.command[1], 1.0, "the table's own full walk forward");
        pilot.update(16.0, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0);

        // W under it walks as it always did, and letting W go leaves the walk running.
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0);
        pilot.key("SCAN_W", false, &mut r.hands());
        assert_eq!((pilot.walk_straight, r.body.command[1]), (true, 1.0), "W's release does not stop it");

        // Q again stops it.
        pilot.key(WALK_STRAIGHT, true, &mut r.hands());
        assert_eq!((pilot.walk_straight, r.body.command[1]), (false, 0.0));
        pilot.update(32.0, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0);

        // A strafe turns it off, and the unit stands once the strafe key comes up.
        pilot.key(WALK_STRAIGHT, true, &mut r.hands());
        pilot.key("SCAN_A", true, &mut r.hands());
        assert!(!pilot.walk_straight, "a strafe turns it off");
        pilot.key("SCAN_A", false, &mut r.hands());
        pilot.update(48.0, &mut r.hands());
        assert_eq!((r.body.command[1], r.body.strafe), (0.0, 0.0));
        for key in ["SCAN_D", "SCAN_S"] {
            pilot.key(WALK_STRAIGHT, true, &mut r.hands());
            assert!(pilot.walk_straight);
            pilot.key(key, true, &mut r.hands());
            assert!(!pilot.walk_straight, "{key} turns it off");
            pilot.key(key, false, &mut r.hands());
            pilot.update(64.0, &mut r.hands());
            assert_eq!(r.body.command[1], 0.0, "and the unit stands after {key}");
        }

        // Leaving the window, which lets every key up, turns it off too.
        pilot.key(WALK_STRAIGHT, true, &mut r.hands());
        pilot.release_all(&mut r.hands());
        assert!(!pilot.walk_straight);
        pilot.update(80.0, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0);
    }

    /// Where a unit goes, in degrees clockwise from its heading: the hull turned by the strafe
    /// angle (+ left), walked along y.
    fn goes(body: &Body) -> f32 {
        let along = body.strafe + if body.command[1] > 0.0 { 0.0 } else { std::f32::consts::PI };
        (-along.to_degrees()).rem_euclid(360.0).round()
    }

    #[test]
    fn a_strafe_backing_up_goes_back_to_its_side_whichever_key_went_down_first() {
        let press = |keys: &[(&str, bool)]| {
            let mut pilot = Pilot::new(hero_table(), 100.0);
            let mut r = rig();
            for &(k, down) in keys {
                pilot.key(k, down, &mut r.hands());
            }
            goes(&r.body)
        };
        assert_eq!(press(&[("SCAN_S", true), ("SCAN_A", true)]), 225.0);
        assert_eq!(press(&[("SCAN_A", true), ("SCAN_S", true)]), 225.0);
        assert_eq!(press(&[("SCAN_S", true), ("SCAN_D", true)]), 135.0);
        assert_eq!(press(&[("SCAN_D", true), ("SCAN_S", true)]), 135.0);
        assert_eq!(press(&[("SCAN_W", true), ("SCAN_A", true)]), 315.0);
        assert_eq!(press(&[("SCAN_A", true)]), 270.0);
        assert_eq!(press(&[("SCAN_S", true), ("SCAN_A", true), ("SCAN_S", false)]), 270.0);
    }

    #[test]
    fn a_walk_key_let_go_under_the_other_walks_the_other_way() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_S", true, &mut r.hands());
        pilot.key("SCAN_W", true, &mut r.hands());
        assert_eq!(r.body.command[1], 1.0, "the last key down walks");
        pilot.key("SCAN_W", false, &mut r.hands());
        assert_eq!(r.body.command[1], -1.0, "S is still held");
        pilot.key("SCAN_S", false, &mut r.hands());
        assert_eq!(r.body.command[1], 0.0);
    }

    /// The manager passes the update over while the game clock stands on the same whole
    /// millisecond (`World3D.dll:0x1000ec90`), so a frame rate above 1000 a second does not
    /// ramp the cruise faster than one step a millisecond.
    #[test]
    fn two_updates_in_one_millisecond_step_the_cruise_once() {
        let mut pilot = Pilot::new(hero_table(), 100.0);
        let mut r = rig();
        pilot.key("SCAN_G_PLUS", true, &mut r.hands());
        pilot.update(0.0, &mut r.hands());
        pilot.update(1000.0, &mut r.hands());
        let once = r.body.command[1];
        assert!((once - 0.05).abs() < 1e-6, "{once}");
        pilot.update(1000.4, &mut r.hands());
        assert_eq!(r.body.command[1], once, "the same whole millisecond runs nothing");
        pilot.update(1001.0, &mut r.hands());
        assert!(r.body.command[1] > once, "the next millisecond runs");
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
