//! Command mode's camera: the free camera a bunker's pod gives the player over the base
//! (`iron3d.dll:0x10036d60`–`0x10037dd0`). Keys move it at a speed that ramps up and dies
//! away, the cursor at an edge of the screen turns and tilts it, and it is held 36 to 236
//! over what is below and within 200 of its bunker on each axis. See
//! `docs/40-command-mode.md`, "The camera".

use glam::Vec3;

use crate::hero::Eye;

/// The view the level makes both cameras with (`0x100364a0`): the field of view across the
/// frame, and the near plane.
pub const FIELD: f32 = 1.04;
pub const NEAR: f32 = 3.0;
/// The tilt the camera is made with: 0 looks straight down, π/2 level.
pub const TILT_START: f32 = 1.0;
/// The keys' speed on the ground plane, m/s; up and down move at half (`0x1003714b`).
pub const SPEED: f32 = 125.0;
/// How fast an edge turns and tilts the camera, rad/s, and while zoomed (`0x100373d4`).
pub const TURN_RATE: f32 = 1.5;
pub const ZOOMED_TURN_RATE: f32 = 0.2;
/// A key or edge reaches its full rate after this long, and a rate let go is gone once an
/// update comes this long after the last.
pub const RAMP_S: f64 = 0.5;
/// The cursor within this of an edge of the 640 × 480 layout turns the camera (`0x10037c9b`).
pub const EDGE: f32 = 6.0;
/// The camera's height is held this far over the highest surface below it (`0x100e5cf4`).
pub const ABOVE_LOW: f32 = 36.0;
pub const ABOVE_HIGH: f32 = 236.0;
/// x and y are each held within this of the bunker's position (`0x100e5cf0`).
pub const HOLD: f32 = 200.0;
/// The zoomed field of view, and how far the field steps toward it or back each update.
pub const ZOOMED_FIELD: f32 = 0.2;
pub const ZOOM_STEP: f32 = 0.1;

/// The six keys `CMD_JAMES_HQ_MOVE_*` hold (`+0x8c`–`+0x91`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Left,
    Right,
    Forward,
    Backward,
    Up,
    Down,
}

/// Where the cursor is against the screen's four edges (`+0x94`–`+0x97`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Edges {
    /// The edges the cursor at `top_left` on the layout pinned to its top left, and at
    /// `bottom_right` pinned to its bottom right, is against (`0x10037c5f`).
    pub fn of(top_left: [f32; 2], bottom_right: [f32; 2]) -> Edges {
        Edges {
            left: top_left[0] <= EDGE,
            right: bottom_right[0] >= crate::hud::LAYOUT[0] - EDGE,
            top: top_left[1] <= EDGE,
            bottom: bottom_right[1] >= crate::hud::LAYOUT[1] - EDGE,
        }
    }
}

/// One of the camera's five rates: sideways, forward, up, yaw and tilt.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Rate {
    value: f32,
    /// Which way its keys or edge push it now: −1, 0 or +1.
    push: i8,
    /// When `push` last changed, s.
    since: f64,
}

impl Rate {
    fn push(&mut self, push: i8, now: f64) {
        if push != self.push {
            self.push = push;
            self.since = now;
        }
    }

    /// Held, `full × min(2t, 1)` with t the seconds since the push began; let go, the value
    /// is multiplied by (0.5 − dt), and is 0 once dt reaches 0.5 (`0x10037130`).
    fn update(&mut self, full: f32, now: f64, dt: f64) {
        if self.push != 0 {
            let t = (now - self.since).max(0.0);
            self.value = f32::from(self.push) * full * (t / RAMP_S).min(1.0) as f32;
        } else if self.value != 0.0 {
            self.value = if dt >= RAMP_S { 0.0 } else { self.value * (RAMP_S - dt) as f32 };
        }
    }
}

/// The command camera (`+0x100` of the level).
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    pub position: Vec3,
    /// The yaw: 0 looks along +x, π/2 along +y.
    pub yaw: f32,
    /// 0 looks straight down, π/2 level (`+0x4c`).
    pub tilt: f32,
    /// The field of view now (`+0x0c`), and whether it is zoomed (`+0x34`).
    pub field: f32,
    pub zoomed: bool,
    /// The bunker it is held around (`+0x48`), by its position.
    pub held: Option<Vec3>,
    keys: [bool; 6],
    edges: Edges,
    rates: [Rate; 5],
    /// When the camera last moved, s.
    moved: Option<f64>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec3::ZERO,
            yaw: std::f32::consts::FRAC_PI_2,
            tilt: TILT_START,
            field: FIELD,
            zoomed: false,
            held: None,
            keys: [false; 6],
            edges: Edges::default(),
            rates: [Rate::default(); 5],
            moved: None,
        }
    }
}

const SIDEWAYS: usize = 0;
const FORWARD: usize = 1;
const UP: usize = 2;
const YAW: usize = 3;
const TILT: usize = 4;

impl Camera {
    /// Mode 0 → 4 (`0x10063ca0`): held around the bunker at `at` with its velocities and up
    /// and down keys cleared (`0x10037da0`), and placed there facing north (`0x10036ae0`).
    /// The tilt and the zoom are kept from the last visit.
    pub fn enter(&mut self, at: Vec3) {
        self.hold(at);
        self.position = at;
        self.yaw = std::f32::consts::FRAC_PI_2;
    }

    /// Held around the bunker at `at` again, where the camera stands (`0x10037da0`), as
    /// telepresence's way back does (mode 2 → 4).
    pub fn hold(&mut self, at: Vec3) {
        self.held = Some(at);
        self.clear_motion();
    }

    /// Let go of the bunker (`0x10037dd0`).
    pub fn leave(&mut self) {
        self.held = None;
        self.clear_motion();
    }

    fn clear_motion(&mut self) {
        for r in &mut self.rates[..=UP] {
            r.value = 0.0;
        }
        self.keys[Move::Up as usize] = false;
        self.keys[Move::Down as usize] = false;
        self.moved = None;
    }

    /// Every key and edge let go.
    pub fn release(&mut self) {
        self.keys = [false; 6];
        self.edges = Edges::default();
    }

    /// A `CMD_JAMES_HQ_MOVE_*` key goes down or up (`0x10071cd0`, `0x10072740`).
    pub fn key(&mut self, key: Move, down: bool) {
        self.keys[key as usize] = down;
    }

    /// `CMD_JAMES_ZOOM_MODE` (`0x1007244a`): only while the field stands at one end.
    pub fn toggle_zoom(&mut self) {
        if self.field <= ZOOMED_FIELD || self.field >= FIELD {
            self.zoomed = !self.zoomed;
        }
    }

    /// One update at `now` seconds (`0x10037a50`): the rates from the edges, the zoom, the
    /// edges from the cursor, the velocities from the keys, then the move, the turn and the
    /// clamps. `top` is the highest landscape or building surface at a place, and `side`
    /// the map's side.
    pub fn update(&mut self, now: f64, edges: Edges, side: f32, top: impl Fn(f32, f32) -> Option<f32>) {
        let dt = self.moved.map_or(0.0, |m| (now - m).max(0.0));
        self.moved = Some(now);
        let turn = if self.zoomed { ZOOMED_TURN_RATE } else { TURN_RATE };
        let pair = |plus: bool, minus: bool| -> i8 {
            if plus {
                1
            } else if minus {
                -1
            } else {
                0
            }
        };
        // The left edge turns the yaw up, the top tilts toward level.
        self.rates[YAW].push(pair(self.edges.left, self.edges.right), now);
        self.rates[TILT].push(pair(self.edges.top, self.edges.bottom), now);
        self.rates[YAW].update(turn, now, dt);
        self.rates[TILT].update(turn, now, dt);
        let goal = if self.zoomed { ZOOMED_FIELD } else { FIELD };
        if self.field < goal {
            self.field = (self.field + ZOOM_STEP).min(goal);
        } else if self.field > goal {
            self.field = (self.field - ZOOM_STEP).max(goal);
        }
        self.edges = edges;
        let k = |m: Move| self.keys[m as usize];
        // Left wins over right, forward over backward and up over down.
        let right = -pair(k(Move::Left), k(Move::Right));
        let (forward, up) = (pair(k(Move::Forward), k(Move::Backward)), pair(k(Move::Up), k(Move::Down)));
        self.rates[SIDEWAYS].push(right, now);
        self.rates[FORWARD].push(forward, now);
        self.rates[UP].push(up, now);
        self.rates[SIDEWAYS].update(SPEED, now, dt);
        self.rates[FORWARD].update(SPEED, now, dt);
        self.rates[UP].update(SPEED / 2.0, now, dt);

        // The move (`0x100375b0`).
        let dt = dt as f32;
        let (s, c) = self.yaw.sin_cos();
        let (side_v, forward_v) = (self.rates[SIDEWAYS].value, self.rates[FORWARD].value);
        let x = self.position.x + dt * (side_v * s + forward_v * c);
        let y = self.position.y + dt * (forward_v * s - side_v * c);
        // A new x or y is taken only above tan(field/2) × z and below the map's side.
        let margin = (self.field / 2.0).tan() * self.position.z;
        if x > margin && x < side {
            self.position.x = x;
        }
        if y > margin && y < side {
            self.position.y = y;
        }
        self.position.z += dt * self.rates[UP].value;
        self.yaw += dt * self.rates[YAW].value;
        let tilt = self.tilt + dt * self.rates[TILT].value;
        if tilt > 0.0 && tilt < std::f32::consts::FRAC_PI_2 {
            self.tilt = tilt;
        }
        if let Some(h) = top(self.position.x, self.position.y) {
            self.position.z = self.position.z.clamp(h + ABOVE_LOW, h + ABOVE_HIGH);
        }
        if let Some(at) = self.held {
            for (p, centre) in [(&mut self.position.x, at.x), (&mut self.position.y, at.y)] {
                if (*p - centre).round() > HOLD {
                    *p = centre + HOLD;
                } else if (*p - centre).round() < -HOLD {
                    *p = centre - HOLD;
                }
            }
        }
    }

    /// The frame R_z(yaw) · R_y(π/2 − tilt): the look is its first column and up its third
    /// (`0x10037943`).
    pub fn eye(&self) -> Eye {
        let pitch = std::f32::consts::FRAC_PI_2 - self.tilt;
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        Eye {
            position: self.position,
            forward: Vec3::new(cy * cp, sy * cp, -sp),
            up: Vec3::new(cy * sp, sy * sp, cp),
            fov_x: self.field,
            near: NEAR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIDE: f32 = 2048.0;

    fn flat(_: f32, _: f32) -> Option<f32> {
        Some(0.0)
    }

    fn camera_at(at: Vec3) -> Camera {
        let mut c = Camera::default();
        c.enter(at);
        c
    }

    /// Run `seconds` of updates 1/60 s apart from `start`.
    fn run(c: &mut Camera, start: f64, seconds: f64, edges: Edges) -> f64 {
        let mut t = start;
        let end = start + seconds;
        while t < end - 1e-9 {
            t += 1.0 / 60.0;
            c.update(t, edges, SIDE, flat);
        }
        t
    }

    #[test]
    fn entering_faces_north_and_looks_32_degrees_down() {
        let c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        let eye = c.eye();
        assert!(eye.forward.x.abs() < 1e-6 && eye.forward.y > 0.0);
        let below = (-eye.forward.z).asin().to_degrees();
        assert!((below - 32.7).abs() < 0.1, "{below}");
        assert!((eye.forward.dot(eye.up)).abs() < 1e-6 && eye.up.z > 0.0);
    }

    #[test]
    fn forward_ramps_up_over_half_a_second_then_holds_125() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        c.update(0.0, Edges::default(), SIDE, flat);
        c.key(Move::Forward, true);
        let t = run(&mut c, 0.0, 1.0, Edges::default());
        // 0.25 × 125 over the ramp and 0.5 s at full speed: 93.75, less the first step's lag.
        let north = c.position.y - 800.0;
        assert!((north - 93.75).abs() < 2.5, "{north}");
        assert!((c.position.x - 1000.0).abs() < 1e-3);
        c.key(Move::Forward, false);
        run(&mut c, t, 0.2, Edges::default());
        let coasted = c.position.y - 800.0 - north;
        // × 0.48 an update at 60 a second: a few metres more, then still.
        assert!(coasted > 0.0 && coasted < 5.0, "{coasted}");
    }

    #[test]
    fn right_is_east_facing_north() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        c.update(0.0, Edges::default(), SIDE, flat);
        c.key(Move::Right, true);
        run(&mut c, 0.0, 1.0, Edges::default());
        assert!(c.position.x > 1080.0 && (c.position.y - 800.0).abs() < 1e-3, "{}", c.position);
    }

    #[test]
    fn it_stays_within_200_of_the_bunker_on_each_axis() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        c.update(0.0, Edges::default(), SIDE, flat);
        c.key(Move::Forward, true);
        c.key(Move::Left, true);
        run(&mut c, 0.0, 5.0, Edges::default());
        assert_eq!((c.position.x, c.position.y), (800.0, 1000.0));
    }

    #[test]
    fn its_height_is_held_36_to_236_over_the_ground() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 10.0));
        c.update(0.0, Edges::default(), SIDE, |_, _| Some(50.0));
        assert_eq!(c.position.z, 86.0);
        c.key(Move::Up, true);
        for i in 1..=300 {
            c.update(f64::from(i) / 60.0, Edges::default(), SIDE, |_, _| Some(50.0));
        }
        assert_eq!(c.position.z, 286.0);
    }

    #[test]
    fn the_bottom_edge_tilts_down_but_never_straight_down() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        c.update(0.0, Edges::default(), SIDE, flat);
        let bottom = Edges { bottom: true, ..Edges::default() };
        // The edge flags a rate reads are the last update's.
        run(&mut c, 0.0, 1.0 / 60.0, bottom);
        run(&mut c, 1.0 / 60.0, 0.92, bottom);
        assert!(c.tilt < 0.06 && c.tilt > 0.0, "{}", c.tilt);
        run(&mut c, 1.0, 3.0, bottom);
        assert!(c.tilt > 0.0);
    }

    #[test]
    fn the_left_edge_turns_it_left() {
        let mut c = camera_at(Vec3::new(1000.0, 800.0, 100.0));
        c.update(0.0, Edges::default(), SIDE, flat);
        let left = Edges { left: true, ..Edges::default() };
        run(&mut c, 0.0, 1.0, left);
        assert!(c.yaw > std::f32::consts::FRAC_PI_2 + 0.5, "{}", c.yaw);
    }

    #[test]
    fn edges_are_within_6_of_the_layout() {
        assert_eq!(Edges::of([3.0, 200.0], [300.0, 400.0]), Edges { left: true, ..Edges::default() });
        assert_eq!(
            Edges::of([300.0, 300.0], [636.0, 475.0]),
            Edges { right: true, bottom: true, ..Edges::default() }
        );
    }
}
