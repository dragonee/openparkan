//! A controller's animation states, played on the machine's own clock:
//! `docs/24-motion.md`, "Playing a state".
//!
//! Each state step moves the clock on by the step's length, runs the attitude
//! and velocity integrators once with that length as dt, and moves the body by
//! its velocity or by the animation's root stride. So a walk plays one stride of
//! animation per stride of ground, and position advances in steps, not ticks.

use std::collections::VecDeque;

use glam::Vec3;
use parkan_formats::control::{Controller, STATE_FIXED, STATE_JITTER, State};
use parkan_formats::mesh::Mesh;

use crate::ground::{Ground, Hit, contact_radius};
use crate::motion::{self, Body, Limits, SLOPE_MODE};

/// A step is held to 0.01–5 s (`0x100057d6`).
pub const STEP_MIN: f32 = 0.01;
pub const STEP_MAX: f32 = 5.0;
/// A velocity-driven state's fixed step is cut so speed × step ≤ 5 (`0x1000550e`).
pub const VELOCITY_STEP_REACH: f32 = 5.0;
/// A jittering state's step moves by up to ±12.5%.
pub const JITTER: f32 = 0.25;
/// The blend weight eases from the last step's q over the first quarter.
pub const EASE: f32 = 4.0;
/// Steps one `advance` may run before it gives the clock up to the caller's time.
const MAX_STEPS: usize = 2000;

/// How far node 0, the body, travels over a state's two frame pairs, in the model's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stride {
    pub a: Vec3,
    pub b: Vec3,
}

/// Every state's stride, measured through the mesh (`0x10019df0`).
pub fn strides(controller: &Controller, mesh: &Mesh) -> Vec<Stride> {
    let travel = |pair: [f32; 2]| {
        if mesh.nodes.is_empty() || pair[0] < 0.0 || pair[1] < 0.0 {
            return Vec3::ZERO;
        }
        let from = mesh.pose_at(0, f64::from(pair[0])).translation;
        let to = mesh.pose_at(0, f64::from(pair[1])).translation;
        Vec3::new((to[0] - from[0]) as f32, (to[1] - from[1]) as f32, (to[2] - from[2]) as f32)
    };
    controller.states.iter().map(|s| Stride { a: travel(s.pair_a), b: travel(s.pair_b) }).collect()
}

/// The weight toward pair B of a state that is neither velocity-driven nor fixed in
/// length: q = p + (1 − p) × blend, p = (speed − lo) ÷ D held to 0–1 (`0x100057a3`).
pub fn blend_weight(state: &State, speed: f32) -> f32 {
    let on = |a: usize| state.flags & (1 << a) != 0;
    let (lo_box, hi_box) = state.velocity;
    let largest = |v: [f32; 3]| (0..3).filter(|&a| on(a)).fold(0.0_f32, |m, a| m.max(v[a].abs()));
    let lo = largest(lo_box).min(largest(hi_box));
    // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- D, a box
    // extent, taken as the largest span of the switched-on axes.
    let d = (0..3).filter(|&a| on(a)).fold(0.0_f32, |m, a| m.max(hi_box[a] - lo_box[a]));
    let p = if d > 0.0 { ((speed - lo) / d).clamp(0.0, 1.0) } else { 1.0 };
    p + (1.0 - p) * state.blend
}

/// The clock, the queue and the step being played.
#[derive(Clone, Debug, PartialEq)]
pub struct Machine {
    pub current: usize,
    pub queue: VecDeque<usize>,
    /// `+0xdc`: when the next step is due, in ms of game time.
    pub clock_ms: f64,
    pub step_start_ms: f64,
    pub step_ms: f64,
    /// This step's weight toward pair B, and the last step's.
    pub q: f32,
    pub q_prev: f32,
    seed: u32,
}

/// What the mesh plays at an instant: frames on pairs A and B and the weight toward B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frames {
    pub a: f32,
    pub b: f32,
    pub weight: f32,
}

/// A machine on the ground: its controller, its body and its state clock.
pub struct Walker {
    pub controller: Controller,
    /// The transition costs as the loader scales them (`0x10001790`).
    pub costs: Vec<f32>,
    pub strides: Vec<Stride>,
    pub limits: Limits,
    pub body: Body,
    pub machine: Machine,
    /// The body's position and yaw when the current step began, for drawing.
    pub from: (Vec3, f32),
    /// The unit's heading when the current step began.
    pub from_heading: f32,
    /// The body sphere's radius as the ground contact holds it.
    pub radius: f32,
    /// How far the origin stands above the model's lowest point.
    pub base: f32,
    /// The face the body last stood on.
    pub ground: Option<Hit>,
}

impl Walker {
    pub fn new(controller: Controller, mesh: &Mesh, position: Vec3, yaw: f32) -> Self {
        let strides = strides(&controller, mesh);
        let radius = contact_radius(mesh.sphere.map_or(1.0, |(_, r)| r));
        let base = -mesh.lowest().unwrap_or(0.0) as f32;
        Self::with_strides(controller, strides, radius, base, position, yaw)
    }

    pub fn with_strides(
        controller: Controller,
        strides: Vec<Stride>,
        radius: f32,
        base: f32,
        position: Vec3,
        yaw: f32,
    ) -> Self {
        // G is 1.0 on every shipped material (docs/24, measured).
        // STAND-IN: docs/24-motion.md#load--read-and-measured -- the spare payload
        // needs the node range the payload counts as the chassis; taken as empty, r = 1.
        let limits = Limits::live(&controller, motion::engine_drive(&controller), 1.0, 1.0);
        let body = Body::new(position, yaw);
        Self {
            costs: controller.live_costs(),
            controller,
            strides,
            limits,
            body,
            machine: Machine {
                // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- the
                // state a machine starts in is not read; state 0.
                current: 0,
                queue: VecDeque::new(),
                clock_ms: 0.0,
                step_start_ms: 0.0,
                step_ms: 0.0,
                q: 1.0,
                q_prev: 1.0,
                seed: 0x2545_F491,
            },
            from: (position, yaw),
            from_heading: yaw,
            radius,
            base,
            ground: None,
        }
    }

    /// Run every state step due by `t_ms`.
    pub fn advance(&mut self, t_ms: f64, ground: &Ground) {
        if self.controller.states.is_empty() {
            return;
        }
        let mut steps = 0;
        while t_ms >= self.machine.clock_ms && steps < MAX_STEPS {
            let current = &self.controller.states[self.machine.current];
            if current.anchor() || self.machine.queue.is_empty() {
                self.plan();
            }
            // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- with
            // nothing queued the current state plays again.
            if let Some(next) = self.machine.queue.pop_front() {
                self.machine.current = next;
            }
            self.step(ground);
            steps += 1;
        }
        if steps == MAX_STEPS {
            self.machine.clock_ms = t_ms;
        }
    }

    /// `0x100051c0`: an anchor that still applies queues the cheapest cycle back to
    /// itself; otherwise the path to the cheapest other anchor that applies.
    fn plan(&mut self) {
        let (v, s) = (self.body.velocity, self.body.spin);
        let c = &self.controller;
        let current = self.machine.current;
        // STAND-IN: docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured
        // -- a state's 16-byte conditions are taken as met, and its use count as
        // unlimited.
        if c.states[current].applies(v, s)
            && let Some((path, _)) = c.path_by(&self.costs, current, current)
        {
            self.machine.queue = path.into();
            return;
        }
        let best = (0..c.states.len())
            .filter(|&j| j != current && c.states[j].anchor() && c.states[j].applies(v, s))
            .filter_map(|j| c.path_by(&self.costs, current, j))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((path, _)) = best {
            self.machine.queue = path.into();
        }
    }

    fn random(&mut self) -> f32 {
        // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- the
        // game's random source is not read; xorshift.
        let mut x = self.machine.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.machine.seed = x;
        x as f32 / u32::MAX as f32
    }

    /// `0x10005370`: one state step.
    fn step(&mut self, ground: &Ground) {
        let state = self.controller.states[self.machine.current].clone();
        let stride = self.strides.get(self.machine.current).copied().unwrap_or_default();
        self.machine.q_prev = self.machine.q;
        self.machine.q = 1.0;
        self.from = (self.body.position, self.body.yaw);
        self.from_heading = self.body.heading();
        self.machine.step_start_ms = self.machine.clock_ms;

        // In ms and f64, so a 50 ms step is 50 ms on the clock.
        let speed = f64::from(self.body.speed());
        let mut step_ms = f64::from(state.length);
        if state.mode & STATE_FIXED == 0 {
            let reach = f64::from(VELOCITY_STEP_REACH) * 1000.0;
            if state.by_velocity() && step_ms > 0.0 && speed * step_ms > reach {
                step_ms = reach / speed;
            }
            if step_ms == 0.0 && speed > 1e-9 {
                if !state.by_velocity() {
                    self.machine.q = blend_weight(&state, speed as f32);
                }
                let q = self.machine.q;
                let travel = (1.0 - q) * stride.a.length() + q * stride.b.length();
                step_ms = f64::from(travel) * 1000.0 / speed;
            }
        }
        if state.mode & STATE_JITTER != 0 {
            step_ms += step_ms * f64::from(JITTER * (self.random() - 0.5));
        }
        let step_ms = step_ms.clamp(f64::from(STEP_MIN) * 1000.0, f64::from(STEP_MAX) * 1000.0);
        self.machine.step_ms = step_ms;
        self.machine.clock_ms += step_ms;
        if state.mode & STATE_FIXED != 0 {
            return;
        }
        let step = (step_ms / 1000.0) as f32;

        // STAND-IN: docs/24-motion.md#from-input-to-motion--read-and-measured -- the
        // body turns by the change in strafe angle (`0x10014cf0`); at what rate is not
        // read, so it is turned at once.
        self.body.yaw = wrap_angle(self.body.yaw + self.body.strafe - self.body.strafe_turned);
        self.body.strafe_turned = self.body.strafe;
        let turned = motion::integrate_turn(&mut self.body.pending, &self.limits, step);
        // STAND-IN: docs/24-motion.md#from-input-to-motion--read-and-measured -- triple 6
        // is (0, 0, 6.28) on the hero; only the turn about z is applied.
        self.body.yaw = wrap_angle(self.body.yaw + turned[2]);
        self.body.spin = turned.map(|t| t / step);
        motion::integrate_velocity(&mut self.body.velocity, self.body.command, &self.limits, step);
        if self.controller.mode == SLOPE_MODE
            && let Some(hit) = self.ground
        {
            let along = self.body.to_world(Vec3::from_array(self.body.velocity));
            // STAND-IN: docs/24-motion.md#ground-and-slope--read -- the brake is read to
            // act one way across the slope; taken as uphill, against the face normal.
            if along.x * hit.normal.x + along.y * hit.normal.y < 0.0 {
                let factor = motion::slope_factor(hit.normal.z, self.controller.cone);
                motion::brake(&mut self.body.velocity, factor, &self.limits, step);
            }
        }

        let moved = if state.by_velocity() {
            self.body.to_world(Vec3::from_array(self.body.velocity)) * step
        } else {
            let q = self.machine.q;
            self.body.to_world(stride.a * (1.0 - q) + stride.b * q)
        };
        self.body.position += moved;
        self.follow_ground(ground);
    }

    /// Put the body on the ground under it.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how the body is put back on the
    /// ground point, the map edge and collision are not read. The model's lowest
    /// point is set on the highest walkable face within the contact radius above it,
    /// as a mission places a unit (docs/07-objects.md, "The ground datum"); with no
    /// face there, or only a wall, the step's travel is undone and the body stops.
    pub fn follow_ground(&mut self, ground: &Ground) {
        let p = self.body.position;
        match ground.below(p.x, p.y, p.z - self.base + self.radius) {
            Some(hit) if hit.walkable() => {
                self.body.position.z = hit.point.z + self.base;
                self.ground = Some(hit);
            }
            _ => {
                self.body.position = self.from.0;
                self.body.velocity = [0.0; 3];
            }
        }
    }

    /// Where the body is drawn at `t_ms`: between the step's start and end by the phase
    /// (`0x10015a50`).
    pub fn drawn(&self, t_ms: f64) -> (Vec3, f32) {
        let s = self.phase(t_ms);
        let position = self.from.0.lerp(self.body.position, s);
        let yaw = self.from.1 + wrap_angle(self.body.yaw - self.from.1) * s;
        (position, yaw)
    }

    /// The unit's heading drawn at `t_ms`.
    pub fn drawn_heading(&self, t_ms: f64) -> f32 {
        self.from_heading + wrap_angle(self.body.heading() - self.from_heading) * self.phase(t_ms)
    }

    /// The step's phase at `t_ms`, 0 to 1.
    pub fn phase(&self, t_ms: f64) -> f32 {
        let m = &self.machine;
        if m.step_ms <= 0.0 { 1.0 } else { ((t_ms - m.step_start_ms) / m.step_ms).clamp(0.0, 1.0) as f32 }
    }

    /// What the mesh plays at `t_ms` (`0x100059a0`).
    pub fn frames(&self, t_ms: f64) -> Frames {
        let s = self.phase(t_ms);
        let state = &self.controller.states[self.machine.current];
        let u = (EASE * s).min(1.0);
        Frames {
            a: state.pair_a[0] + s * (state.pair_a[1] - state.pair_a[0]),
            b: state.pair_b[0] + s * (state.pair_b[1] - state.pair_b[0]),
            weight: (1.0 - u) * self.machine.q_prev + u * self.machine.q,
        }
    }
}

/// An angle brought into −π..π.
pub fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    a - tau * ((a + std::f32::consts::PI) / tau).floor()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::tests::floor;
    use parkan_formats::control::{NO_EDGE, STATE_ANCHOR, STATE_BY_VELOCITY, TRIPLE_ACCELERATION};

    const ALL_AXES: u32 = 0b111;

    /// Standing (anchor, |vy| ≤ 2, 50 ms) and walking (anchor, velocity-driven,
    /// vy 2 to 14.5, 0.5 of stride a step), each cycling on itself.
    fn walker(position: Vec3, yaw: f32) -> Walker {
        let stand = State {
            flags: ALL_AXES,
            mode: STATE_ANCHOR,
            velocity: ([-0.5, -2.0, -0.5], [0.5, 2.0, 0.5]),
            pair_a: [2.0, 2.0],
            pair_b: [2.0, 2.0],
            blend: 1.0,
            length: 50.0,
            ..Default::default()
        };
        let walk = State {
            flags: ALL_AXES,
            mode: STATE_ANCHOR | STATE_BY_VELOCITY,
            velocity: ([-0.5, 2.0, -0.5], [0.5, 14.5, 0.5]),
            pair_a: [2.0, 2.0],
            pair_b: [5.0, 6.0],
            blend: 1.0,
            ..Default::default()
        };
        let mut c = crate::motion::tests::hero();
        c.states = vec![stand, walk];
        c.costs = vec![0.0, 1.0, 1.0, 0.0];
        assert!(c.costs.iter().all(|&x| x < NO_EDGE));
        let strides = vec![Stride::default(), Stride { a: Vec3::ZERO, b: Vec3::new(0.0, 0.5, 0.0) }];
        Walker::with_strides(c, strides, 2.0, 0.0, position, yaw)
    }

    #[test]
    fn holding_forward_walks_at_the_top_speed_one_stride_a_step() {
        let g = floor();
        let mut w = walker(Vec3::new(5.0, 5.0, 3.0), 0.0);
        w.body.command = [0.0, 1.0, 0.0];
        w.advance(1000.0, &g);
        assert_eq!(w.machine.current, 1);
        assert_eq!(w.body.velocity, [0.0, 14.0, 0.0]);
        assert!((w.machine.step_ms - 500.0 / 14.0).abs() < 1e-3, "{}", w.machine.step_ms);
        assert_eq!(w.body.position.z, 0.0);
        assert!(w.body.position.y > 5.0 + 10.0, "{}", w.body.position.y);
        w.body.command = [0.0; 3];
        w.advance(3000.0, &g);
        assert_eq!(w.machine.current, 0);
        assert_eq!(w.body.velocity, [0.0; 3]);
    }

    #[test]
    fn a_wall_stops_the_body_before_it() {
        let g = floor();
        let facing_east = -std::f32::consts::FRAC_PI_2;
        let mut w = walker(Vec3::new(30.0, 20.0, 0.0), facing_east);
        w.body.command = [0.0, 1.0, 0.0];
        w.advance(3000.0, &g);
        assert!(w.body.position.x <= 40.0 && w.body.position.x > 36.0, "{}", w.body.position.x);
    }

    #[test]
    fn a_pending_turn_turns_the_body_during_steps() {
        let g = floor();
        let mut w = walker(Vec3::new(20.0, 20.0, 0.0), 0.0);
        w.body.pending[2] = 0.75;
        w.advance(500.0, &g);
        assert!((w.body.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "{}", w.body.yaw);
        assert!((w.body.pending[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn between_steps_the_body_and_the_blend_are_interpolated() {
        let g = floor();
        let mut w = walker(Vec3::new(5.0, 5.0, 0.0), 0.0);
        w.body.command = [0.0, 1.0, 0.0];
        w.advance(1000.0, &g);
        let mid = w.machine.step_start_ms + w.machine.step_ms / 2.0;
        let (p, _) = w.drawn(mid);
        assert!((p - (w.from.0 + w.body.position) / 2.0).length() < 1e-4);
        let f = w.frames(mid);
        assert_eq!((f.a, f.b, f.weight), (2.0, 5.5, 1.0));
        assert_eq!(w.limits.acceleration[1], 2.0 * w.controller.triples[TRIPLE_ACCELERATION][1]);
    }

    #[test]
    fn a_blended_state_weighs_toward_b_as_speed_passes_its_box() {
        let s = State {
            flags: ALL_AXES,
            velocity: ([-0.5, 6.0, -0.5], [0.5, 14.0, 0.5]),
            blend: 0.6,
            ..Default::default()
        };
        assert_eq!(blend_weight(&s, 6.0), 0.6);
        assert_eq!(blend_weight(&s, 14.0), 1.0);
        assert!((blend_weight(&s, 10.0) - 0.8).abs() < 1e-6);
    }
}
