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
/// The most of the live turn rate a hull turns toward its turret at, the bounds of the spin
/// the turret lock sets (`Control.dll:0x1003bdd0`, `0x1003bdd4`).
pub const LEAD_SPIN: f32 = 0.7;
/// The world's gravity, `CWorld` `+0xc` (`Terrain.dll:0x10024c1a`); nothing changes it.
pub const GRAVITY: f32 = 10.0;
/// The map edge (`Control.dll:0x1001e7ba`): the band inside an inset side that pushes
/// back, and how hard; the height above the box's top that is let in, and how hard.
pub const EDGE_BAND: f32 = 80.0;
pub const EDGE_BAND_PUSH: f32 = 3.0 * 0.0125;
pub const EDGE_ABOVE: f32 = 20.0;
pub const EDGE_ABOVE_PUSH: f32 = 3.0 * 0.05;
/// A push made horizontal keeps its length, its x and y scaled up at most this much
/// (`Control.dll:0x1000ca44`).
pub const HORIZONTAL_SCALE_MOST: f32 = 4.0;

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

/// A step with no normalised turn pending (`0x10014a54`–`0x10014bff`, docs/30-turrets.md, "The
/// hull follows the turret"). The pending triple goes back to rest. With the turret lock on,
/// the spin's z becomes clamp(−(lead − 0.5) × 2π ÷ (live turn z × dt), ±0.7), replacing what
/// the spin setter wrote. The hull turns by spin × live turn × dt, and with the lock the lead
/// pays that turn out, wrapping, and keeps it as the step. Returns the radians turned.
///
/// The body has a yaw alone (see [`Body`]), so only the turn about z is taken; every state
/// the righting bits `0x40`/`0x80` mark zeroes the spin about x and y (`0x10014af9`). The
/// running gear is whole, so no veer is added.
pub fn integrate_spin(body: &mut Body, limits: &Limits, dt: f32) -> [f32; 3] {
    body.pending = [NO_TURN; 3];
    let most = limits.turn[2].abs() * dt;
    if body.turret_lock {
        let wanted = -(body.lead - NO_TURN) * TAU;
        body.spin_set[2] = if most > 0.0 { (wanted / most).clamp(-LEAD_SPIN, LEAD_SPIN) } else { 0.0 };
    }
    // `0x10014c02`: each axis's turn is held to the live rate × dt.
    let turn = (body.spin_set[2] * most).clamp(-most, most);
    if body.turret_lock {
        body.lead_step = turn / TAU;
        let v = body.lead + body.lead_step;
        body.lead = v - v.floor();
    }
    [0.0, 0.0, turn]
}

/// The turret's yaw target the control takt writes while the lock holds (`0x10005b13`): the
/// lead less the part of the step's turn not yet taken, `lead − (1 − s) × step`, with `s` the
/// step's phase. So the turret holds its heading while the hull comes round under it.
pub fn led_aim(body: &Body, s: f32) -> f32 {
    let v = body.lead - (1.0 - s) * body.lead_step;
    v - v.floor()
}

/// The fall a body tries this step (`Control.dll:0x10015d91`): `(v − g dt ÷ 2) dt`,
/// with `v` its fall speed.
pub fn fall(fall_speed: f32, dt: f32) -> f32 {
    (fall_speed - GRAVITY * dt / 2.0) * dt
}

/// `0x1001e650`: the push that keeps a sphere at `centre` inside the box `lo`..`hi`.
///
/// Hard: past an x or y side inset by `radius`, back to it; more than 20 above the top,
/// back to that; nothing from below. Soft: within 80 of an inset side,
/// 3 × 0.0125 × min(depth, 80), the depth into that band; above the top,
/// 3 × 0.05 × min(height, 20).
pub fn edge_push(centre: Vec3, radius: f32, lo: Vec3, hi: Vec3) -> Vec3 {
    let mut push = Vec3::ZERO;
    for a in 0..2 {
        let (low, high) = (lo[a] + radius, hi[a] - radius);
        if centre[a] < low {
            push[a] += low - centre[a];
        } else if centre[a] > high {
            push[a] += high - centre[a];
        }
        let (in_low, in_high) = (centre[a] - low, high - centre[a]);
        if in_low < EDGE_BAND {
            push[a] += EDGE_BAND_PUSH * (EDGE_BAND - in_low).min(EDGE_BAND);
        }
        if in_high < EDGE_BAND {
            push[a] -= EDGE_BAND_PUSH * (EDGE_BAND - in_high).min(EDGE_BAND);
        }
    }
    let above = centre.z - hi.z;
    if above > EDGE_ABOVE {
        push.z -= above - EDGE_ABOVE;
    }
    if above > 0.0 {
        push.z -= EDGE_ABOVE_PUSH * above.min(EDGE_ABOVE);
    }
    push
}

/// A push as a machine in a state with contact points takes it (`0x1000ca44`): z
/// dropped, x and y scaled up to keep the length, at most ×4.
pub fn horizontal(push: Vec3) -> Vec3 {
    let flat = push.with_z(0.0);
    let across = flat.length();
    if across <= 0.0 {
        return Vec3::ZERO;
    }
    flat * (push.length() / across).min(HORIZONTAL_SCALE_MOST)
}

/// A machine's place and motion.
///
/// STAND-IN: docs/24-motion.md#the-hull-leans-and-rights-itself--read-and-measured --
/// the lean (state `+0x08`, triple 6) and the righting (bits `0x30`/`0xC0`, triple 5)
/// are read but not modelled, and the vector bits `0x30` right the hull toward is not
/// read: the body has a yaw alone, takes only the turn about z, and neither leans nor
/// rights. Every hero state leans on no axis and rights toward world up.
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
    /// Whether a normalised turn is pending (machine `+0x35d`): `SetNormAngle` sets it and
    /// the spin setter clears it. With it clear the pending triple is not paid out.
    pub turn_pending: bool,
    /// The spin the spin setter wrote (`IControl` slot 4, `+0x1d4`): a fraction of the live
    /// turn rate on each axis. `,` and `.` send ±0.7 about z.
    pub spin_set: [f32; 3],
    /// The turret lock (machine `+0x65c`, property 179): while it is on, and no normalised
    /// turn is pending, the hull comes round under the turret.
    pub turret_lock: bool,
    /// The turret's lead over the hull (body `+0x38`), in turns, 0.5 when it faces the hull's
    /// way: every change to the turret's yaw target is added in, and the hull's turns toward
    /// it are paid out of it.
    pub lead: f32,
    /// The hull's last step toward the turret, in turns (body `+0x3c`).
    pub lead_step: f32,
    /// The strafe angle asked for (`+0x1f4`).
    pub strafe: f32,
    /// The strafe angle the attitude integrator last took up (body `+0x17c`), and the
    /// change it added to the hull's turn that step (`+0x188`).
    pub strafe_previous: f32,
    pub strafe_change: f32,
    /// The fall speed, 0 or less (body `+0xac`).
    pub fall_speed: f32,
    /// The averaged ground normal of the last landing (body `+0x194`), which the slope
    /// brake reads.
    pub ground_normal: Vec3,
}

impl Body {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            position,
            yaw,
            velocity: [0.0; 3],
            spin: [0.0; 3],
            command: [0.0; 3],
            pending: [NO_TURN; 3],
            turn_pending: false,
            spin_set: [0.0; 3],
            turret_lock: false,
            lead: NO_TURN,
            lead_step: 0.0,
            strafe: 0.0,
            strafe_previous: 0.0,
            strafe_change: 0.0,
            fall_speed: 0.0,
            ground_normal: Vec3::Z,
        }
    }

    /// A vector in the machine's frame, turned into the world.
    pub fn to_world(&self, v: Vec3) -> Vec3 {
        let (s, c) = self.yaw.sin_cos();
        Vec3::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z)
    }

    /// Where the unit looks once a step is done: the hull's yaw plus the turret's strafe
    /// offset at the step's end, −(the strafe angle taken up) (`Control.dll:0x10005ab8`).
    pub fn heading(&self) -> f32 {
        self.yaw - self.strafe_previous
    }

    /// The strafe offset the control takt hands the turret at phase `s` of the step
    /// (`0x10005ab8`): `(1 − s) × change − angle`, which is −(previous + s × change), in
    /// radians. The turret keeps it ÷ its yaw span, negated on a hung mounting, and its
    /// yaw channel's first entry adds that before it wraps and inverts (docs/30).
    pub fn strafe_offset(&self, s: f32) -> f32 {
        (1.0 - s) * self.strafe_change - self.strafe_previous
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
            weights: [0.0; 2],
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
    fn a_locked_hull_comes_round_under_its_turret_at_most_at_0_7_of_the_turn_rate() {
        let live = Limits { acceleration: [0.0; 3], top_speed: [0.0; 3], turn: [1.0, 1.0, 4.0] };
        let mut body = Body::new(Vec3::ZERO, 0.0);
        body.turret_lock = true;
        // The turret turned a quarter to the right: 0.25 turns past the hull.
        body.lead = 0.75;
        let dt = 0.05;
        let turned = integrate_spin(&mut body, &live, dt);
        // −π/2 wanted against 0.2 a step: the spin is held to −0.7, a right turn of 0.14 rad.
        assert!((body.spin_set[2] + LEAD_SPIN).abs() < 1e-6, "{}", body.spin_set[2]);
        assert!((turned[2] + 0.7 * 4.0 * dt).abs() < 1e-6, "{turned:?}");
        assert!((body.lead - (0.75 - 0.14 / TAU)).abs() < 1e-6, "the lead pays it out: {}", body.lead);
        // The heading the turret looks along, the hull's yaw less the lead, has not moved.
        assert!((turned[2] - (body.lead - NO_TURN) * TAU + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        // Half-way through the step the turret's target has taken back half the step.
        assert!((led_aim(&body, 0.5) - (body.lead + 0.07 / TAU)).abs() < 1e-6);
        assert!((led_aim(&body, 1.0) - body.lead).abs() < 1e-6);
        // Steps close the gap and then hold: π/2 at 2.8 rad/s takes 12 steps.
        let mut yaw = turned[2];
        let mut steps = 1;
        while (body.lead - NO_TURN).abs() > 1e-6 {
            yaw += integrate_spin(&mut body, &live, dt)[2];
            steps += 1;
            assert!(steps < 50);
        }
        assert_eq!(steps, 12);
        assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4, "{yaw}");
        assert_eq!(integrate_spin(&mut body, &live, dt), [0.0; 3], "and it rests there");
        // The world heading the turret looks along, hull plus lead, never moved.
        assert!((yaw - (body.lead - NO_TURN) * TAU + std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn without_the_lock_the_spin_setters_fraction_turns_the_hull_and_the_pending_triple_rests() {
        let live = Limits { acceleration: [0.0; 3], top_speed: [0.0; 3], turn: [1.0, 1.0, 4.0] };
        let mut body = Body::new(Vec3::ZERO, 0.0);
        body.pending = [0.5, 0.5, 0.75];
        body.lead = 0.75;
        body.spin_set[2] = 0.7;
        let turned = integrate_spin(&mut body, &live, 0.1);
        assert!((turned[2] - 0.28).abs() < 1e-6, "0.7 x 4 rad/s x 0.1 s: {turned:?}");
        assert_eq!(body.pending, [NO_TURN; 3]);
        assert_eq!(body.lead, 0.75, "the lead is not paid out without the lock");
    }

    #[test]
    fn a_fall_is_the_speed_less_half_a_steps_gravity_times_the_step() {
        assert!((fall(0.0, 0.05) + 0.0125).abs() < 1e-7);
        assert!((fall(-0.5, 0.05) + 0.0375).abs() < 1e-7);
    }

    #[test]
    fn the_map_edge_pushes_back_hard_past_an_inset_side_and_softly_within_eighty() {
        let (lo, hi) = (Vec3::new(0.0, 0.0, -10.0), Vec3::new(1000.0, 1000.0, 50.0));
        let deep = Vec3::new(500.0, 500.0, 0.0);
        assert_eq!(edge_push(deep, 2.0, lo, hi), Vec3::ZERO);
        // 40 inside the inset side at x 998: 40 into the band, 1.5 back.
        let p = edge_push(Vec3::new(958.0, 500.0, 0.0), 2.0, lo, hi);
        assert!((p.x + 1.5).abs() < 1e-5 && p.y == 0.0 && p.z == 0.0, "{p}");
        // 5 past it: back to the side, and the whole band's 3.
        let p = edge_push(Vec3::new(1003.0, 500.0, 0.0), 2.0, lo, hi);
        assert!((p.x + 8.0).abs() < 1e-4, "{p}");
        let p = edge_push(Vec3::new(1.0, -4.0, 0.0), 2.0, lo, hi);
        assert!((p.x - 4.0).abs() < 1e-4 && (p.y - 9.0).abs() < 1e-4, "{p}");
        // Above the top: softly up to 20, and back to 20 past it; nothing from below.
        assert!((edge_push(Vec3::new(500.0, 500.0, 60.0), 2.0, lo, hi).z + 1.5).abs() < 1e-5);
        assert!((edge_push(Vec3::new(500.0, 500.0, 75.0), 2.0, lo, hi).z + 8.0).abs() < 1e-4);
        assert_eq!(edge_push(Vec3::new(500.0, 500.0, -500.0), 2.0, lo, hi), Vec3::ZERO);
    }

    #[test]
    fn a_push_made_horizontal_keeps_its_length_up_to_four_times_its_breadth() {
        assert_eq!(horizontal(Vec3::new(3.0, 0.0, 4.0)), Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(horizontal(Vec3::new(1.0, 0.0, 10.0)), Vec3::new(4.0, 0.0, 0.0));
        assert_eq!(horizontal(Vec3::new(0.0, 0.0, -3.0)), Vec3::ZERO);
    }

    #[test]
    fn the_turret_is_handed_minus_the_previous_angle_plus_the_change_eased_across_the_step() {
        let mut b = Body::new(Vec3::ZERO, 0.0);
        b.strafe_previous = std::f32::consts::FRAC_PI_2;
        b.strafe_change = std::f32::consts::FRAC_PI_2;
        b.yaw = std::f32::consts::FRAC_PI_2;
        assert_eq!(b.strafe_offset(0.0), 0.0);
        assert!((b.strafe_offset(0.5) + std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        assert_eq!(b.strafe_offset(1.0), -std::f32::consts::FRAC_PI_2);
        assert_eq!(b.heading(), b.yaw + b.strafe_offset(1.0));
    }

    #[test]
    fn a_placement_heading_faces_minus_sine_and_cosine() {
        let b = Body::new(Vec3::ZERO, -1.639);
        let f = b.forward();
        assert!((f.x - 0.9977).abs() < 1e-3 && (f.y + 0.068).abs() < 1e-3);
    }
}
