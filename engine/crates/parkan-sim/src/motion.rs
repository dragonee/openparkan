//! How a machine's speed and heading change: `docs/24-motion.md`.
//!
//! Velocity is in the machine's own frame, y forward, and a turn waits in a
//! pending triple the attitude integrator pays out. Both integrators run once a
//! state step, with dt the step ([`crate::machine`]).

use std::f32::consts::TAU;

use glam::Vec3;
use parkan_formats::control::{Controller, ENGINE_TYPE, TRIPLE_ACCELERATION, TRIPLE_TOP_SPEED, TRIPLE_TURN};

/// The live acceleration is twice the authored one (`Control.dll:0x1000fe26`).
pub const LIVE_ACCELERATION: f32 = 2.0;
/// Only a controller of this mode brakes on slopes (`0x100157ac`).
pub const SLOPE_MODE: i32 = 2;
/// The slope brake pulls the velocity at this multiple of the acceleration.
pub const SLOPE_BRAKE: f32 = 1.5;
/// A pending triple at rest: nothing left to turn.
pub const NO_TURN: f32 = 0.5;

/// The live block's speeds and rates (`0x1000fca0`), per axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub acceleration: [f32; 3],
    pub top_speed: [f32; 3],
    pub turn: [f32; 3],
}

impl Limits {
    /// With `e` the engines' drive times the running gear, `r` the spare payload
    /// over the payload (1 empty) and `g` the ground's speed factor:
    /// top speed `top × g × e × (1 + r) ÷ 2` capped at the authored top, turn
    /// `turn × e × (1.5 r + 0.5)`, acceleration twice the authored.
    pub fn live(controller: &Controller, e: f32, r: f32, g: f32) -> Self {
        let factor = g * e * (1.0 + r) / 2.0;
        Self {
            acceleration: controller.triples[TRIPLE_ACCELERATION].map(|a| a * LIVE_ACCELERATION),
            top_speed: controller.triples[TRIPLE_TOP_SPEED]
                .map(|top| if (top * factor).abs() > top.abs() { top } else { top * factor }),
            turn: controller.triples[TRIPLE_TURN].map(|turn| turn * e * (1.5 * r + 0.5)),
        }
    }
}

/// E before damage: the sum of the engines' value 0 (id 0x100, `0x1002c06d`), each
/// at full condition, with both running gear sides whole.
pub fn engine_drive(controller: &Controller) -> f32 {
    controller.components.iter().filter(|c| c.type_id == ENGINE_TYPE).map(|c| c.values[0]).sum()
}

/// `0x100153c0`: each axis moves toward `command × top speed` by at most
/// `acceleration × dt`, stops at the target, and is held to the top speed.
pub fn integrate_velocity(velocity: &mut [f32; 3], command: [f32; 3], limits: &Limits, dt: f32) {
    for (a, v) in velocity.iter_mut().enumerate() {
        let target = command[a] * limits.top_speed[a];
        let top = limits.top_speed[a].abs();
        *v = approach(*v, target, limits.acceleration[a] * dt).clamp(-top, top);
    }
}

/// The mode-2 brake's fraction for ground whose tilt has cosine `c`:
/// `min(1, 2 (c − cos cone) ÷ (1 − cos cone))`, 0 past the cone.
pub fn slope_factor(c: f32, cone: f32) -> f32 {
    let edge = cone.cos();
    if c <= edge { 0.0 } else { (2.0 * (c - edge) / (1.0 - edge)).min(1.0) }
}

/// `value` moved toward `target` by at most `reach`, stopping there.
fn approach(value: f32, target: f32, reach: f32) -> f32 {
    let gap = target - value;
    if gap.abs() <= reach { target } else { value + reach.copysign(gap) }
}

/// Pull the velocity toward `factor` of itself at 1.5 × the acceleration.
pub fn brake(velocity: &mut [f32; 3], factor: f32, limits: &Limits, dt: f32) {
    for (a, v) in velocity.iter_mut().enumerate() {
        *v = approach(*v, *v * factor, SLOPE_BRAKE * limits.acceleration[a] * dt);
    }
}

/// `0x1001480f`: pay out a pending turn. Each axis reads `(v − 0.5) × 2π`; all three
/// are scaled by one factor so the largest fits `turn × dt`, and the step is taken
/// back out of the triple, which wraps (`0x100149d2`). Returns the radians turned.
pub fn integrate_turn(pending: &mut [f32; 3], limits: &Limits, dt: f32) -> [f32; 3] {
    let wanted = pending.map(|v| (v - NO_TURN) * TAU);
    let mut k = 1.0_f32;
    for (w, turn) in wanted.iter().zip(limits.turn) {
        let most = turn.abs() * dt;
        if w.abs() > most {
            k = k.min(most / w.abs());
        }
    }
    let step = wanted.map(|w| w * k);
    for (p, s) in pending.iter_mut().zip(step) {
        let v = *p - s / TAU;
        *p = v - v.floor();
    }
    step
}

/// A machine's place and motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    /// World position of the object's origin.
    pub position: Vec3,
    /// Radians about z, the mission placement's sense: facing `(−sin, cos)`.
    pub yaw: f32,
    /// In the machine's frame, y forward (`+0x1c8`).
    pub velocity: [f32; 3],
    /// Angular velocity, z yaw (`+0x1d4`).
    pub spin: [f32; 3],
    /// What the input or the AI asks for, −1 to 1 per axis (body `+0x08`).
    pub command: [f32; 3],
    /// The turn not yet made (`+0x1e0`), 0.5 at rest.
    pub pending: [f32; 3],
}

impl Body {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self { position, yaw, velocity: [0.0; 3], spin: [0.0; 3], command: [0.0; 3], pending: [NO_TURN; 3] }
    }

    /// A vector in the machine's frame, turned into the world.
    pub fn to_world(&self, v: Vec3) -> Vec3 {
        let (s, c) = self.yaw.sin_cos();
        Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
    }

    pub fn forward(&self) -> Vec3 {
        self.to_world(Vec3::Y)
    }

    /// The largest component of the velocity: what a state step calls speed.
    pub fn speed(&self) -> f32 {
        self.velocity.iter().fold(0.0_f32, |m, v| m.max(v.abs()))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use parkan_formats::control::Component;
    use parkan_formats::objects::ResourceRef;

    /// The hero chassis `r_h_02`'s triples, engine and mode.
    pub fn hero() -> Controller {
        let mut values = [0.0; 16];
        values[0] = 1.0;
        let engine = Component {
            type_id: ENGINE_TYPE,
            resource: ResourceRef::default(),
            index: None,
            entries: Vec::new(),
            label: String::new(),
            values,
            power: 0.1,
            node: 0,
            mass: 0.0,
            flags: 0,
            group: -1,
        };
        let mut c =
            Controller { mode: SLOPE_MODE, cone: 0.6, components: vec![engine], ..Default::default() };
        c.triples[TRIPLE_ACCELERATION] = [90.0, 70.0, 90.0];
        c.triples[TRIPLE_TOP_SPEED] = [1.0, 14.0, 1.0];
        c.triples[TRIPLE_TURN] = [1.57, 1.57, 25.12];
        c
    }

    #[test]
    fn the_hero_empty_on_flat_ground_runs_at_its_authored_top_and_turns_at_twice() {
        let c = hero();
        let live = Limits::live(&c, engine_drive(&c), 1.0, 1.0);
        assert_eq!(live.top_speed, [1.0, 14.0, 1.0]);
        assert_eq!(live.acceleration, [180.0, 140.0, 180.0]);
        // Not capped: an empty machine turns twice as fast as the file says.
        assert_eq!(live.turn, c.triples[TRIPLE_TURN].map(|t| t * 2.0));
        let loaded = Limits::live(&c, 1.0, 0.0, 1.0);
        assert_eq!(loaded.top_speed[1], 7.0);
        assert_eq!(loaded.turn[2], 25.12 * 0.5);
    }

    #[test]
    fn holding_forward_reaches_exactly_the_top_speed_and_stays() {
        let c = hero();
        let live = Limits::live(&c, 1.0, 1.0, 1.0);
        let mut v = [0.0; 3];
        integrate_velocity(&mut v, [0.0, 1.0, 0.0], &live, 0.05);
        assert_eq!(v[1], 7.0);
        for _ in 0..3 {
            integrate_velocity(&mut v, [0.0, 1.0, 0.0], &live, 0.05);
        }
        assert_eq!(v, [0.0, 14.0, 0.0]);
        integrate_velocity(&mut v, [0.0, -1.0, 0.0], &live, 0.1);
        assert_eq!(v[1], 0.0);
    }

    #[test]
    fn the_slope_brake_leaves_a_fraction_and_nothing_past_the_cone() {
        let f = slope_factor(0.5_f32.cos(), 0.6);
        assert!((f * 14.0 - 8.375).abs() < 0.01, "{}", f * 14.0);
        assert_eq!(slope_factor(0.7_f32.cos(), 0.6), 0.0);
        assert_eq!(slope_factor(1.0, 0.6), 1.0);
    }

    #[test]
    fn a_pending_turn_is_paid_out_at_the_turn_rate_with_one_scale_for_all_axes() {
        let live = Limits { acceleration: [0.0; 3], top_speed: [0.0; 3], turn: [1.0, 1.0, 25.12] };
        let mut pending = [0.5, 0.5 + 0.1 / TAU, 0.75];
        let step = integrate_turn(&mut pending, &live, 0.05);
        // z alone would fit 1.256 of its pi/2, but y's 0.1 against its 0.05 scales all by 0.5.
        assert!((step[1] - 0.05).abs() < 1e-5 && (step[2] - std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        assert!((pending[2] - 0.625).abs() < 1e-5);
        let mut rest = [0.5; 3];
        assert_eq!(integrate_turn(&mut rest, &live, 0.05), [0.0; 3]);
        assert_eq!(rest, [0.5; 3]);
    }

    #[test]
    fn a_placement_heading_faces_minus_sine_and_cosine() {
        let b = Body::new(Vec3::ZERO, -1.639);
        let f = b.forward();
        assert!((f.x - 0.9977).abs() < 1e-3 && (f.y + 0.068).abs() < 1e-3);
    }
}
