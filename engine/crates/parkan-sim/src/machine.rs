//! A controller's animation states, played on the machine's own clock:
//! `docs/24-motion.md`, "Playing a state".
//!
//! Each state step moves the clock on by the step's length, runs the attitude
//! and velocity integrators once with that length as dt, and moves the body by
//! its velocity or by the animation's root stride. So a walk plays one stride of
//! animation per stride of ground, and position advances in steps, not ticks.
//! After the move the ground contact holds the body on the ground, and each tick
//! the map edge keeps it inside the world's box.

use std::collections::VecDeque;

use glam::{Quat, Vec3};
use parkan_formats::control::{
    CONTACT_PLACE, CONTACT_PLACE_BY_POSE, CONTACT_PLANTED, CONTACT_SUPPORT, Controller, FIRST_REQUEST,
    PLACE_AXIS_WITHIN, PLANTED_WITHIN, STATE_FIXED, STATE_GROUND_CONTACTS, STATE_JITTER, State,
    UNLIMITED_USES,
};
use parkan_formats::cpt::ControlPoint;
use parkan_formats::mesh::Mesh;

use crate::ground::{Ground, Hit, contact_radius};
use crate::motion::{self, Body, GRAVITY, Limits, SLOPE_MODE};
use crate::wizard::Drive;

/// A step is held to 0.01–5 s (`0x100057d6`).
pub const STEP_MIN: f32 = 0.01;
pub const STEP_MAX: f32 = 5.0;
/// A velocity-driven state's fixed step is cut so speed × step ≤ 5 (`0x1000550e`).
pub const VELOCITY_STEP_REACH: f32 = 5.0;
/// A jittering state's step moves by up to ±12.5%: it gains step × `JITTER` × (r − 0.5)
/// with r the generator's word over 65536 (`0x100057de`-`0x1000584b`).
pub const JITTER: f32 = 0.25;
/// The jitter's draw is a 16-bit word over 65536 (`0x1003b374`).
const RANDOM_SCALE: f32 = 1.0 / 65536.0;
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

/// What a contact's [`CONTACT_PLACE_BY_POSE`] asks of the pass that measures the strides
/// (`0x10019df0`, `0x1001a331`): it sets [`CONTACT_PLACE`] on the contact where the state's
/// last pose stands its axis up, and clears it where it does not.
///
/// The machine does this each time it takes a state, on the copy of that state's record it
/// keeps at `+0x100` (`0x10007b99`, `0x10031982`); since the answer depends on nothing but
/// the state, the engine works it out once per state instead and writes it into the
/// controller's own records, which is what the rest of the machine reads.
///
/// *Measured*: 2410 of the install's 2634 contacts ask for it — every walking chassis's
/// feet — and 2217 of them stand up (docs/24-motion.md, "A walker's feet lie flat where
/// the animation lays them").
fn place_by_pose(controller: &mut Controller, feet: &Feet) {
    let decided: Vec<Vec<bool>> = controller
        .states
        .iter()
        .map(|s| s.contacts.iter().map(|c| feet.stands_up(c.point, s)).collect())
        .collect();
    for (state, up) in controller.states.iter_mut().zip(&decided) {
        for (contact, &up) in state.contacts.iter_mut().zip(up) {
            if contact.flags & CONTACT_PLACE_BY_POSE == 0 {
                continue;
            }
            if up {
                contact.flags |= CONTACT_PLACE;
            } else {
                contact.flags &= !CONTACT_PLACE;
            }
        }
    }
}

/// The weight toward pair B of a state that is neither velocity-driven nor fixed in
/// length: q = p + (1 − p) × blend, p = (speed − lo) ÷ D held to 0–1 (`0x100057a3`).
pub fn blend_weight(state: &State, speed: f32) -> f32 {
    let on = |a: usize| state.flags & (1 << a) != 0;
    let (lo_box, hi_box) = state.velocity;
    let largest = |v: [f32; 3]| (0..3).filter(|&a| on(a)).fold(0.0_f32, |m, a| m.max(v[a].abs()));
    // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- lo and D are read
    // over all three velocity axes, D as the largest ||max| − |min||; here both are taken
    // over the switched-on axes and D as the largest span (max − min). Every shipped
    // state that blends switches on all three axes, and where the two D differ its blend
    // base is 1, so the weight is the same.
    let lo = largest(lo_box).min(largest(hi_box));
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
    /// The request code the controller holds (IControl slot 19,
    /// `docs/32-builder.md`); a state with a code of its own applies only while it is this.
    /// The constructor's is [`FIRST_REQUEST`], 0 (`Control.dll:0x10006ecf`).
    pub request: i32,
    /// The jitter generator's two words, s0 in the low half and s1 in the high
    /// (`Control.dll:0x10042230`; [`Walker::random`]).
    seed: u32,
}

/// What the mesh plays at an instant: frames on pairs A and B and the weight toward B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frames {
    pub a: f32,
    pub b: f32,
    pub weight: f32,
}

/// The model a machine's contact points are placed on, and its control points.
#[derive(Clone, Debug, PartialEq)]
pub struct Feet {
    pub mesh: Mesh,
    pub points: Vec<ControlPoint>,
}

impl Feet {
    /// Where control point `point` stands in the model's frame with the mesh posed at
    /// `frames`, on the node its first triple's **second** slot names — the one the point
    /// sits on (docs/07, "a wheel's contact point rides on the body and dies with the
    /// wheel"). The third slot names the node the contact dies with, which the state's
    /// damage conditions read (docs/13), not a frame to place the point in. The pose walk
    /// leaves out node 0's travel ([`Mesh::walk_pose`]): the body's own move carries it.
    pub fn place(&self, point: i32, frames: Frames) -> Option<Vec3> {
        let p = self.points.get(usize::try_from(point).ok()?)?;
        let node = usize::try_from(p.nodes().0).ok().filter(|&n| n < self.mesh.nodes.len())?;
        let (a, b, w) = (f64::from(frames.a), f64::from(frames.b), f64::from(frames.weight));
        let pose = self.mesh.world_pose_by(node, |n| self.mesh.walk_pose(n, a, b, w));
        let at = pose.apply(p.position.map(f64::from));
        Some(Vec3::new(at[0] as f32, at[1] as f32, at[2] as f32))
    }

    /// The contact's own axis in the model's frame: the control point's vector turned by
    /// the pose of the node it sits on, the same pose [`Feet::place`] puts the point in.
    ///
    /// *Measured*: on all twelve contacts that ask for it the point sits on node 0 and its
    /// vector comes out along the model's up — `(0, −1, 0)` on the S-42t, whose root turns
    /// −90° about x, and `(0, 0, 1)` on the M-42t and the L-42t. So it is the direction the
    /// belt's ground normal is compared against.
    pub fn axis(&self, point: i32, frames: Frames) -> Option<Vec3> {
        let p = self.points.get(usize::try_from(point).ok()?)?;
        let node = usize::try_from(p.nodes().0).ok().filter(|&n| n < self.mesh.nodes.len())?;
        let (a, b, w) = (f64::from(frames.a), f64::from(frames.b), f64::from(frames.weight));
        let pose = self.mesh.world_pose_by(node, |n| self.mesh.walk_pose(n, a, b, w));
        let v = parkan_formats::pose::rotate(pose.rotation, p.direction.map(f64::from));
        let v = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
        (v.length_squared() > 1e-12).then(|| v.normalize())
    }

    /// The node a contact lives and dies with, which is also the node it lays along the
    /// ground when its flags carry [`CONTACT_PLACE`] (the control point's third slot).
    pub fn carrier(&self, point: i32) -> Option<usize> {
        let p = self.points.get(usize::try_from(point).ok()?)?;
        usize::try_from(p.nodes().1).ok().filter(|&n| n < self.mesh.nodes.len())
    }

    /// Whether `state`'s last pose stands control point `point`'s own axis up
    /// (`0x1001a331`–`0x1001a364`): the axis at pair B's last frame with all of the weight
    /// on B, its z above 0 and within [`PLACE_AXIS_WITHIN`] of 1. That is what a contact's
    /// [`CONTACT_PLACE_BY_POSE`] asks, and the answer is [`CONTACT_PLACE`] for that state.
    ///
    /// The machine compares the posed vector as it comes, unnormalised, where [`Feet::axis`]
    /// hands it back normalised; every shipped control point's vector is unit length, so on
    /// all 2410 contacts that ask the two answers agree.
    pub fn stands_up(&self, point: i32, state: &State) -> bool {
        let b = state.pair_b[1];
        self.axis(point, Frames { a: b, b, weight: 1.0 })
            .is_some_and(|v| v.z > 0.0 && 1.0 - v.z < PLACE_AXIS_WITHIN)
    }

    /// Whether `state`'s last pose plants control point `point` (`0x1001a2d5`–`0x1001a328`):
    /// the mesh posed at pair B's last frame with all of the weight on B, the root's own
    /// height kept, puts the point within 0.1 of its height at rest.
    ///
    /// STAND-IN: docs/13-control.md#a-footstep-end-to-end--read-and-measured -- which pose
    /// the live contact record's height is taken from at load is not read: the rest pose.
    pub fn planted(&self, point: i32, state: &State) -> bool {
        let Some(p) = self.points.get(usize::try_from(point).unwrap_or(usize::MAX)) else { return false };
        let Some(node) = usize::try_from(p.nodes().0).ok().filter(|&n| n < self.mesh.nodes.len()) else {
            return false;
        };
        let b = f64::from(state.pair_b[1]);
        let height = |pose: parkan_formats::pose::Pose| pose.apply(p.position.map(f64::from))[2] as f32;
        let last = height(self.mesh.world_pose_by(node, |n| self.mesh.blended_pose(n, b, b, 1.0)));
        let rest = height(self.mesh.world_pose(node));
        (last - rest).abs() <= PLANTED_WITHIN
    }
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
    /// The body sphere's centre in the model's frame, its radius as the ground contact
    /// holds it, and its radius whole, which the map edge insets the box by.
    pub centre: Vec3,
    pub radius: f32,
    pub sphere_radius: f32,
    /// r₂: the **node** sphere's own radius, which bounds the ground search's up pass
    /// (`0x1001a70a`). It is held to 7.5 only when it is under 20 and the object's flags
    /// carry `0x1000000` (`0x1001a51b`-`0x1001a58a`); which objects carry that flag is not
    /// read, and the hold is left out here. *Measured*: r₂ differs from r on all 148 units
    /// the campaign places, r₂/r running 0.42 to 2.34.
    pub node_radius: f32,
    /// How far the origin stands above the model's lowest point.
    pub base: f32,
    /// The face the ground search last found under the body's centre.
    pub ground: Option<Hit>,
    /// Whether the slope brake may act (body `+0x1aa`): the body starts with it set
    /// (`Control.dll:0x100143c6`), and the ground contact sets it from each face it holds, clear
    /// on a face whose class carries 2 (`0x1001a9b4`-`0x1001a9be`) -- a building's walk-through
    /// floor, triangle flag 2 -- and set on any other.
    pub slope_brakes: bool,
    /// The contact points' model, where the controller has contacts and the object
    /// control points.
    pub feet: Option<Feet>,
    /// Per state, per contact: whether the state plants it (`0x1000`, set at load).
    pub planting: Vec<Vec<bool>>,
    /// Each contact's live planted byte (`+0x5a`), and the contacts that have landed
    /// since the caller last took them.
    pub planted: Vec<bool>,
    pub landed: Vec<usize>,
    /// What the ground contact leaves on a [`CONTACT_PLACE`] contact: the node it carries
    /// and the turn that lays that node along the ground under it, in the machine's own
    /// frame. The pose walk turns the node's world pose by it and leaves it where it was.
    pub placed: Vec<(usize, Quat)>,
    /// What the Wizard writes, when the AI drives: the velocity is taken as the machine's
    /// own, and the hull turns toward the heading.
    pub drive: Option<Drive>,
    /// What each step's move is multiplied by: 1, but for a god mode's hero. The states still
    /// play at the machine's own speed, so the one that applies is the one that would.
    pub stride_scale: f32,
    /// How far pushes have dropped the body since the ground contact last held it: the parts of
    /// pushes that point down, taken whole while the machine stands on a building
    /// ([`Walker::take_push`]). The body is drawn that much higher ([`Walker::drawn`]).
    pub dropped: f32,
}

impl Walker {
    /// A machine from its controller, its mesh and the object's control points.
    pub fn new(
        controller: Controller,
        mesh: &Mesh,
        points: &[ControlPoint],
        position: Vec3,
        yaw: f32,
    ) -> Self {
        let strides = strides(&controller, mesh);
        let (centre, sphere) = mesh.sphere.map_or((Vec3::ZERO, 1.0), |(c, r)| (Vec3::from_array(c), r));
        let base = -mesh.lowest().unwrap_or(0.0) as f32;
        let contacts = controller.states.iter().any(|s| !s.contacts.is_empty());
        let mut walker = Self::with_strides(controller, strides, contact_radius(sphere), base, position, yaw);
        walker.centre = centre;
        walker.sphere_radius = sphere;
        // A bare mesh has no merged model to work a node sphere out of; its own header
        // sphere stands in for r₂ until [`Walker::set_body_sphere`] is given one.
        walker.node_radius = sphere;
        if contacts && !points.is_empty() {
            let feet = Feet { mesh: mesh.clone(), points: points.to_vec() };
            walker.planting = walker
                .controller
                .states
                .iter()
                .map(|s| {
                    s.contacts
                        .iter()
                        .map(|c| c.flags & CONTACT_PLANTED != 0 || feet.planted(c.point, s))
                        .collect()
                })
                .collect();
            place_by_pose(&mut walker.controller, &feet);
            walker.feet = Some(feet);
        }
        walker
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
        // The controller alone knows no fitted engine and no load: its own engine slot and an
        // empty machine, r = 1. A robot puts its weighed limits in their place
        // (`parkan_world::robot::Heft`).
        let limits = Limits::live(&controller, motion::engine_drive(&controller), 1.0, 1.0);
        let body = Body::new(position, yaw);
        Self {
            costs: controller.live_costs(),
            controller,
            strides,
            limits,
            body,
            machine: Machine {
                // The constructor puts the state index at 0 (`Control.dll:0x10006d19`) and
                // zeroes the current-state copy but for its flags, `0x10001`
                // (`0x10006d43`-`0x10006d49`). A zeroed record's use count is 0, so that
                // copy does not apply and the first plan runs from index 0 to the cheapest
                // anchor that does (docs/24, "Playing a state").
                current: 0,
                queue: VecDeque::new(),
                clock_ms: 0.0,
                step_start_ms: 0.0,
                step_ms: 0.0,
                q: 1.0,
                q_prev: 1.0,
                // The constructor's code is 0 (`Control.dll:0x10006ecf`), which is the code
                // a finished building's own state waits for (docs/32, "stop the ray").
                request: FIRST_REQUEST,
                // The game seeds its generator once, at load, from `ngiGetClocks`
                // (`0x10006330`); a fixed seed keeps the engine's runs repeatable.
                seed: 0x2545_F491,
            },
            from: (position, yaw),
            from_heading: yaw,
            centre: Vec3::ZERO,
            radius,
            sphere_radius: radius,
            node_radius: radius,
            base,
            ground: None,
            slope_brakes: true,
            feet: None,
            planting: Vec::new(),
            planted: Vec::new(),
            landed: Vec::new(),
            placed: Vec::new(),
            drive: None,
            stride_scale: 1.0,
            dropped: 0.0,
        }
    }

    /// Run every state step due by `t_ms`, then one pass of the map edge.
    pub fn advance(&mut self, t_ms: f64, ground: &Ground) {
        if self.controller.states.is_empty() {
            return;
        }
        let mut steps = 0;
        while t_ms >= self.machine.clock_ms && steps < MAX_STEPS {
            // 1. Only an anchor plans. 2. The next state comes off the queue; with
            // nothing queued nothing is taken, and the current state plays again.
            if self.controller.states[self.machine.current].anchor() {
                self.plan();
            }
            if let Some(next) = self.machine.queue.pop_front() {
                self.machine.current = next;
            }
            self.step(ground);
            steps += 1;
        }
        if steps == MAX_STEPS {
            self.machine.clock_ms = t_ms;
        }
        self.keep_inside(ground);
    }

    /// `0x100051c0`: an anchor that still applies queues the cheapest cycle back to
    /// itself; otherwise the path to the cheapest other anchor that applies.
    fn plan(&mut self) {
        let (v, s, code) = (self.body.velocity, self.body.spin, self.machine.request);
        let c = &self.controller;
        let current = self.machine.current;
        // STAND-IN: docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured
        // -- a state's contacts are read to need their nodes intact (0x100) or destroyed
        // (0x200), but node life is not modelled here: those conditions are taken as met.
        if c.states[current].applies(v, s, code)
            && let Some((path, _)) = c.path_by(&self.costs, current, current)
        {
            self.machine.queue = path.into();
            return;
        }
        let best = (0..c.states.len())
            .filter(|&j| j != current && c.states[j].anchor() && c.states[j].applies(v, s, code))
            .filter_map(|j| c.path_by(&self.costs, current, j).map(|(path, cost)| (j, path, cost)))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let Some((to, path, _)) = best {
            // `0x100052fd`-`0x1000530f`: the destination anchor spends one use, unless its
            // count is unlimited or already 0.
            let uses = &mut self.controller.states[to].uses;
            if *uses != UNLIMITED_USES && *uses != 0 {
                *uses -= 1;
            }
            self.machine.queue = path.into();
        }
    }

    /// The generator the jitter draws from (`Control.dll:0x100057de`-`0x1000582f`), a pair
    /// of 16-bit words held side by side in one dword: s0 takes (s0 << 1) xor s1, then s1
    /// takes (s1 >> 1) xor the new s0, and the new s1 over 65536 is the draw. The game
    /// seeds it once at load from `ngiGetClocks` (`0x10006330`, a static initialiser in the
    /// module's table at `0x1003e160`); all-zero is a fixed point, so an unseeded one would
    /// hand every jittering step the same −12.5%.
    fn random(&mut self) -> f32 {
        let (mut s0, mut s1) = (self.machine.seed as u16, (self.machine.seed >> 16) as u16);
        s0 = (s0 << 1) ^ s1;
        s1 = (s1 >> 1) ^ s0;
        self.machine.seed = u32::from(s0) | (u32::from(s1) << 16);
        f32::from(s1) * RANDOM_SCALE
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

        // STAND-IN: docs/24-motion.md#not-established -- how the velocity integrator's pull
        // toward the command combines with a velocity the Wizard writes, and whether its
        // spin is a rate or a fraction, are not read: a driven machine takes the written
        // velocity as its own and turns toward the heading at up to its live yaw rate.
        let turned = match self.drive.and_then(|d| d.heading) {
            Some(heading) => {
                let most = self.limits.turn[2].abs() * step;
                [0.0, 0.0, wrap_angle(heading - self.body.yaw).clamp(-most, most)]
            }
            None if self.drive.is_some() => [0.0; 3],
            // `0x100147ff`: a normalised turn pending is paid out; without one the spin turns
            // the hull, and the turret lock sets it (docs/30, "The hull follows the turret").
            None if self.body.turn_pending => {
                motion::integrate_turn(&mut self.body.pending, &self.limits, step)
            }
            None => motion::integrate_spin(&mut self.body, &self.limits, step),
        };
        // `0x10014cf0`: the change in strafe angle is added to the step's turn after the
        // turn-rate clamp, so the hull swings by the whole change in one step.
        let change = self.body.strafe - self.body.strafe_previous;
        self.body.strafe_previous = self.body.strafe;
        self.body.strafe_change = change;
        self.body.yaw = wrap_angle(self.body.yaw + turned[2] + change);
        self.body.spin = turned.map(|t| t / step);
        match self.drive {
            Some(d) => self.body.velocity = d.in_frame(self.body.yaw, self.limits.top_speed),
            None => {
                motion::integrate_velocity(&mut self.body.velocity, self.body.command, &self.limits, step)
            }
        }
        // The brake reads the ground normal the lift last averaged, whoever's faces the contacts
        // stood on (`0x100156c6`, body `+0x194`), and acts only while the body's gate is set
        // (`0x10015690`): not on a building's walk-through floors, its stairs and ramps among
        // them, which carry triangle flag 2 (docs/24, "Ground and slope").
        if self.controller.mode == SLOPE_MODE && self.slope_brakes {
            let along = self.body.to_world(Vec3::from_array(self.body.velocity));
            let normal = self.body.ground_normal;
            // `0x10015738`-`0x10015799`: the brake acts while (velocity x up) . (up x normal)
            // is not negative, which is the horizontal velocity having no component along the
            // normal's own horizontal part -- moving uphill, or exactly across the slope
            // (docs/24, "Ground and slope").
            if along.x * normal.x + along.y * normal.y <= 0.0 {
                let factor = motion::slope_factor(normal.z, self.controller.cone);
                // STAND-IN: docs/24-motion.md#ground-and-slope--read -- the brake is read to be
                // passed over while a velocity has been written since the command last was
                // (`0x1001566b`, body `+0x1a8`: `SetTangSpeed` sets it at `0x100044f1`, the
                // command's setter clears it at `0x1000442b`), so a machine the Wizard drives is
                // never braked. Here a driven machine still is, the ground's fraction taken off
                // the written velocity whole: without it an AI unit walks up a 40 degree slope
                // at 8 m/s, and a small warbot sent into Mission 03's Small Bunker never reaches
                // its dock, since the local path that would keep it off such faces is a stand-in
                // of its own (docs/24, "Not established").
                if self.drive.is_some() {
                    self.body.velocity = self.body.velocity.map(|v| v * factor);
                } else {
                    motion::brake(&mut self.body.velocity, factor, &self.limits, step);
                }
            }
        }

        // A velocity-driven state moves by the velocity clamped into its own box
        // (`0x10001160`, `0x1001592b`); the integrated velocity itself is kept.
        let moved = if state.by_velocity() {
            self.body.to_world(Vec3::from_array(state.clamp_velocity(self.body.velocity))) * step
        } else {
            let q = self.machine.q;
            self.body.to_world(stride.a * (1.0 - q) + stride.b * q)
        };
        self.body.position += moved * self.stride_scale;
        self.hold(ground, &state, step);
        self.land(self.machine.current, &state);
    }

    /// The ground contact's pass over the contacts (`0x1001b081`–`0x1001b0be`): a contact
    /// the state plants that was not planted lands, and runs its group; one the state
    /// does not plant is no longer planted. There is no ground distance in the test.
    ///
    /// STAND-IN: docs/13-control.md#section-1s-conditions-are-contacts--read-and-measured
    /// -- node life is not modelled on the walker: every contact is intact.
    fn land(&mut self, current: usize, state: &State) {
        let Some(plants) = self.planting.get(current) else { return };
        self.planted.resize(state.contacts.len(), false);
        for (i, &plants) in plants.iter().enumerate().take(state.contacts.len()) {
            if plants && !self.planted[i] {
                self.planted[i] = true;
                self.landed.push(i);
            } else if !plants {
                self.planted[i] = false;
            }
        }
    }

    /// The ground contact (`0x1001a450`) and the lift it moves the body by
    /// (`0x10015d60`): docs/24-motion.md, "Holding the body on the ground".
    ///
    /// The body sphere's centre is searched for the ground. In a state with bit `0x4` and
    /// contact points the lift is the largest gap from a flag-1 contact up to the ground
    /// under it, and the body falls under gravity until the fall would not stay above
    /// that lift; in any other state the sphere is only lifted out of the ground, by
    /// (ground − centre) + r while the ground lies less than r below. When the lift is
    /// taken the fall speed returns to 0 and the sphere's and contacts' normals, averaged,
    /// become the ground normal. A point with no face under it has itself as its ground.
    ///
    /// STAND-IN: docs/24-motion.md#holding-the-body-on-the-ground--read-and-measured --
    /// the pass runs once a frame on message `0x1c`, with the frame's own milliseconds as
    /// its dt (`0x1001b41b` reads machine `+0xe8`), and reads its contact points at the
    /// pose the machine tick has just put the mesh in for that frame
    /// (`0x1000c737`). Here it still runs after every state step, with the step as dt and
    /// the contacts on the step's last frames.
    fn hold(&mut self, ground: &Ground, state: &State, dt: f32) {
        self.dropped = 0.0;
        let r = self.radius;
        let centre = self.body.position + self.body.to_world(self.centre);
        // The up pass takes a face only where it stands less than r₂ -- the node sphere's
        // own radius, not r -- above the centre (`0x1001a70a`).
        let hit = ground.search(centre, self.node_radius);
        self.ground = hit;
        if let Some(h) = &hit {
            self.slope_brakes = !ground.class_carries_2(h);
        }
        let last = Frames { a: state.pair_a[1], b: state.pair_b[1], weight: self.machine.q };
        let searched: Vec<(u32, i32, Vec3, Option<Hit>)> = match &self.feet {
            Some(feet) if state.mode & STATE_GROUND_CONTACTS != 0 => state
                .contacts
                .iter()
                .filter(|c| c.flags & (CONTACT_SUPPORT | CONTACT_PLACE) != 0)
                .filter_map(|c| feet.place(c.point, last).map(|at| (c.flags, c.point, at)))
                .map(|(flags, point, at)| {
                    let p = self.body.position + self.body.to_world(at);
                    // STAND-IN: docs/24-motion.md#not-established -- what a contact point's
                    // own up pass tests against is not read: it compares with a triple the
                    // pass builds from control `+0x2ec`, `+0x2fc` and `+0x30c`
                    // (`0x1001aba7`, `0x1001ae12`). Here it is the body sphere's r2, the one
                    // bound the pass *is* read to use. It had been the agent sphere's r, and
                    // r2 is the smaller on 122 of the 148 unit models the campaign places:
                    // on C02 Mission 03's Large Factory, a metre past the door, a wheel kept
                    // finding the floor it had just left 2.9 m above itself, and the lift,
                    // which is the largest rise over the contacts, hoisted the machine back
                    // up the ramp into the structure over it -- where the collision pass put
                    // the whole move back, every tick, for good.
                    (flags, point, p, ground.search(p, self.node_radius))
                })
                .collect(),
            _ => Vec::new(),
        };
        self.lay_belts(&searched, last);
        let contacts: Vec<(Vec3, Option<Hit>)> = searched
            .iter()
            .filter(|(flags, ..)| flags & CONTACT_SUPPORT != 0)
            .map(|&(_, _, p, h)| (p, h))
            .collect();
        let lift = if state.mode & STATE_GROUND_CONTACTS == 0 {
            // Bit `0x4` clear (`0x1001b3c3`): the sphere is pushed out of the ground and
            // never pulled down. With no face under it the ground point is the centre
            // itself (`0x1001a86e`), so the gap is 0 and the lift is the whole r -- such a
            // sphere climbs by its own radius every frame.
            let gap = hit.map_or(0.0, |h| h.point.z - centre.z);
            Some(if gap > -r { gap + r } else { 0.0 })
        } else if contacts.is_empty() {
            // Bit `0x4` with no flag-1 contact: the lift vector the contact loop fills is
            // left at zero (`0x1001ab5d`), and the fall is not tried either (`0x10015d7b`).
            Some(0.0)
        } else {
            let lift =
                contacts.iter().map(|(p, h)| h.map_or(0.0, |h| h.point.z - p.z)).fold(f32::MIN, f32::max);
            let fall = motion::fall(self.body.fall_speed, dt);
            if fall > lift {
                self.body.position.z += fall;
                self.body.fall_speed -= GRAVITY * dt;
                None
            } else {
                Some(lift)
            }
        };
        if let Some(lift) = lift {
            self.body.position.z += lift;
            self.body.fall_speed = 0.0;
            // The sphere's normal and every flag-1 contact's, over their count plus one
            // (`0x1001b0ea`). A point with no face under it keeps the default normal, world
            // up (`Control.dll:0x1001bfa0`), and counts like any other.
            let up = |h: &Option<Hit>| h.map_or(Vec3::Z, |h| h.normal);
            let normals: Vec<Vec3> =
                std::iter::once(up(&hit)).chain(contacts.iter().map(|(_, h)| up(h))).collect();
            self.body.ground_normal = normals.iter().sum::<Vec3>() / normals.len() as f32;
        }
    }

    /// A [`CONTACT_PLACE`] contact lays its carrier node along the ground beneath it
    /// (`Control.dll:0x1001affd`): a tracked chassis's belt, which carries the flag as
    /// authored (docs/28-chassis.md, "The belt lies along the ground"), and a walker's foot
    /// in the states where [`place_by_pose`] has given it the flag (docs/24-motion.md, "A
    /// walker's feet lie flat where the animation lays them").
    ///
    /// The ground contact hands `IAnimation` slot 31 (`0x10005c90`) the contact's own axis
    /// and the ground normal it has just found; the slot builds the rotation that takes the
    /// first onto the second and leaves it on the node the contact carries, which the
    /// ground contact has marked with node mask `0x10` (`0x1001a3af`). The pose walk then
    /// turns that node's world pose by it and writes the translation back, so the node
    /// **tilts where it stands** and the hull above it does not move.
    ///
    /// STAND-IN: docs/28-chassis.md#the-belt-lies-along-the-ground--read-and-measured --
    /// the slot keeps the previous turn beside the current one and the walk slerps the two
    /// by the mesh's pose-blend weight, and a contact whose node is dead is handed nulls,
    /// which leaves identity and lets the node relax back to level. Node life is not
    /// modelled on the walker (see [`Walker::land`]); the current turn is kept and used
    /// whole. On the twelve belts that costs nothing — each belongs to a velocity-driven
    /// state, whose weight never leaves 1 — but a walker's foot is laid in states that
    /// blend, so its tilt arrives a step sooner than the game's.
    fn lay_belts(&mut self, searched: &[(u32, i32, Vec3, Option<Hit>)], frames: Frames) {
        let Some(feet) = self.feet.as_ref() else {
            self.placed.clear();
            return;
        };
        let level = Quat::from_rotation_z(-self.body.yaw);
        self.placed = searched
            .iter()
            .filter(|(flags, ..)| flags & CONTACT_PLACE != 0)
            .filter_map(|&(_, point, _, hit)| {
                let node = feet.carrier(point)?;
                let axis = feet.axis(point, frames)?;
                // A point with no face under it is left level, as its own ground.
                let normal = level * hit.map_or(Vec3::Z, |h| h.normal);
                let normal = normal.try_normalize().unwrap_or(Vec3::Z);
                Some((node, Quat::from_rotation_arc(axis, normal)))
            })
            .collect();
    }

    /// `0x1001e650`, once a tick: the map edge's push on the body sphere, taken as a
    /// machine takes a push.
    pub fn keep_inside(&mut self, ground: &Ground) {
        let (lo, hi) = ground.world_box();
        let centre = self.body.position + self.body.to_world(self.centre);
        self.take_push(motion::edge_push(centre, self.sphere_radius, lo, hi));
    }

    /// Whether the machine's collision flags carry 8, so a building's floors (triangle flag 2)
    /// push it too. The control sets them through the collision object's slot 5
    /// (`Control.dll:0x1001f670`, called at `0x10007bd0` as message 4 initialises the machine)
    /// from its current state's word: 8 unless the word carries 4. *Measured*: of the install's
    /// 206 controllers with states, none mixes states with and without bit 4 -- 99 carry it in
    /// every state, every walker and every wheeled and tracked chassis among them, and 107 in
    /// none: the nine flying chassis, two animals, the 30 buildings and the 66 rounds.
    pub fn keeps_floors(&self) -> bool {
        !self.controller.states.get(self.machine.current).is_some_and(|s| s.mode & STATE_GROUND_CONTACTS != 0)
    }

    /// Message `0x1b` (`0x1000c990`): a push moves the position, the velocity kept; in a
    /// state with bit `0x4` it is made horizontal, its length kept up to ×4.
    pub fn take_push(&mut self, push: Vec3) {
        let flat = self
            .controller
            .states
            .get(self.machine.current)
            .is_some_and(|s| s.mode & STATE_GROUND_CONTACTS != 0);
        // A machine whose parent is a building takes a push that points down whole
        // (`Control.dll:0x1000c9eb`, docs/24, "Collision between objects").
        let on_building = self.ground.is_some_and(|h| h.solid.is_some());
        let whole = !flat || (on_building && push.z <= 0.0);
        if flat && whole {
            self.dropped += push.z;
        }
        self.body.position += if whole { push } else { motion::horizontal(push) };
    }

    /// The body sphere as the ground contact takes it: the centre in the model's frame
    /// (`0x1001a518`, the agent's node sphere's), and the radius (`0x1001a487`, the agent's
    /// sphere's), held to 7.5 under 20 for the contact.
    pub fn set_body_sphere(&mut self, centre: Vec3, radius: f32, node_radius: f32) {
        self.centre = centre;
        self.sphere_radius = radius;
        self.radius = contact_radius(radius);
        self.node_radius = node_radius;
    }

    /// The body sphere's centre in the world.
    pub fn sphere_centre(&self) -> Vec3 {
        self.body.position + self.body.to_world(self.centre)
    }

    /// Stand the body on the ground at once, as a mission places a unit: its lowest
    /// point on the highest face below it, or up to the contact radius above it
    /// (docs/07-objects.md, "The ground datum"). For setting a scene up; the ground
    /// contact holds it from the next step.
    pub fn follow_ground(&mut self, ground: &Ground) {
        let p = self.body.position;
        if let Some(hit) = ground.below(p.x, p.y, p.z - self.base + self.radius) {
            self.body.position.z = hit.point.z + self.base;
            self.body.fall_speed = 0.0;
            self.ground = Some(hit);
        }
    }

    /// Where the body is drawn at `t_ms`: between the step's start and end by the phase
    /// (`0x10015a50`).
    ///
    /// STAND-IN: docs/24-motion.md#collision-between-objects--read -- the body is drawn where
    /// the ground contact last held it, before the down pushes taken on a building since have
    /// dropped it ([`Walker::dropped`]). Read, the frame runs the move, the collision pass, the ground
    /// contact and then the push (message `0x1b`, `Control.dll:0x1000c9eb` taking a down push
    /// whole on a building), and where the frame is drawn among them is not read. Drawn after
    /// the push, C02 M03's medium walker stands with its hull at the Small Warehouse's floor:
    /// the ceiling, 6.3-7.5 m up, presses its 5.87 m agent sphere 2.85 m down every tick and
    /// the contact lifts it back. The recording shows it upright on its legs in every frame
    /// ("Let's Play - Parkan: Iron Strategy, Part 4", 1:43 and 15:13-15:15 at 60 fps), which
    /// the push drawn before it lands gives, and which moves nothing the simulation holds:
    /// units still go down a ramp into a building by that push.
    pub fn drawn(&self, t_ms: f64) -> (Vec3, f32) {
        let s = self.phase(t_ms);
        let position = self.from.0.lerp(self.body.position, s) - Vec3::Z * self.dropped;
        let yaw = self.from.1 + wrap_angle(self.body.yaw - self.from.1) * s;
        (position, yaw)
    }

    /// The unit's heading drawn at `t_ms`: the drawn hull's yaw plus the turret's strafe
    /// offset then, which is the heading at the step's start plus the phase's share of
    /// the hull's own turn.
    pub fn drawn_heading(&self, t_ms: f64) -> f32 {
        self.from_heading + wrap_angle(self.body.heading() - self.from_heading) * self.phase(t_ms)
    }

    /// The strafe offset the turret is handed at `t_ms`, in radians (`0x10005ab8`).
    pub fn strafe_offset(&self, t_ms: f64) -> f32 {
        self.body.strafe_offset(self.phase(t_ms))
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
    use crate::ground::tests::quads;
    use parkan_formats::control::{Contact, NO_EDGE, STATE_ANCHOR, STATE_BY_VELOCITY, TRIPLE_ACCELERATION};
    use parkan_formats::mesh::{Key, NO_PARENT, NO_SLOT, Node};
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    const ALL_AXES: u32 = 0b111;
    const DT: f64 = 1000.0 / 60.0;

    /// A 1000 m level field at 0, and a post at 1000..1001 standing 100 high, so the
    /// world's box has a top well above the field.
    fn field() -> Ground {
        quads(&[
            [[0.0, 0.0, 0.0], [1000.0, 0.0, 0.0], [1000.0, 1000.0, 0.0], [0.0, 1000.0, 0.0]],
            [[1000.0, 0.0, 100.0], [1001.0, 0.0, 100.0], [1001.0, 1.0, 100.0], [1000.0, 1.0, 100.0]],
        ])
    }

    fn stand() -> State {
        State {
            flags: ALL_AXES,
            mode: STATE_ANCHOR,
            velocity: ([-0.5, -2.0, -0.5], [0.5, 2.0, 0.5]),
            pair_a: [2.0, 2.0],
            pair_b: [2.0, 2.0],
            blend: 1.0,
            length: 50.0,
            ..Default::default()
        }
    }

    /// Standing (anchor, |vy| ≤ 2, 50 ms) and walking (anchor, velocity-driven,
    /// vy 2 to 14.5, 0.5 of stride a step), each cycling on itself; the sphere, radius
    /// 2 about the origin, is only lifted out of the ground.
    fn walker(position: Vec3, yaw: f32) -> Walker {
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
        c.states = vec![stand(), walk];
        c.costs = vec![0.0, 1.0, 1.0, 0.0];
        assert!(c.costs.iter().all(|&x| x < NO_EDGE));
        let strides = vec![Stride::default(), Stride { a: Vec3::ZERO, b: Vec3::new(0.0, 0.5, 0.0) }];
        Walker::with_strides(c, strides, 2.0, 0.0, position, yaw)
    }

    #[test]
    fn the_planner_spends_a_states_use_count_and_stops_choosing_it_at_zero() {
        // `0x100052fd`-`0x1000530f`: the destination anchor pays one use, and at 0 the
        // state no longer applies (`0x10001132`). Every finite count in the install is 30,
        // 20 or 10, and every one of them is on an anchor.
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.controller.states[1].uses = 2;
        // Already walking, so the walk anchor applies and standing does not.
        w.body.velocity = [0.0, 10.0, 0.0];
        w.advance(0.0, &g);
        assert_eq!((w.machine.current, w.controller.states[1].uses), (1, 1));
        // Back to standing, then forward again: the walk's second and last use.
        w.machine.current = 0;
        w.advance(w.machine.clock_ms, &g);
        assert_eq!((w.machine.current, w.controller.states[1].uses), (1, 0));
        w.machine.current = 0;
        w.advance(w.machine.clock_ms, &g);
        assert_eq!(w.machine.current, 0, "the walk is spent, so it is no longer chosen");
        w.controller.states[1].uses = UNLIMITED_USES;
        w.body.velocity = [0.0, 10.0, 0.0];
        w.advance(w.machine.clock_ms, &g);
        assert_eq!(w.machine.current, 1, "an unlimited count is never spent");
        assert_eq!(w.controller.states[1].uses, UNLIMITED_USES);
    }

    #[test]
    fn the_jitter_draws_from_the_games_own_pair_of_words() {
        // `0x100057de`-`0x1000582f`: s0 = (s0 << 1) ^ s1, then s1 = (s1 >> 1) ^ s0, and
        // the draw is the new s1 over 65536. Worked by hand from seed 1 | (2 << 16):
        // s0 = 2 ^ 2 = 0, s1 = 1 ^ 0 = 1, so the first draw is 1/65536.
        let mut w = walker(Vec3::ZERO, 0.0);
        w.machine.seed = 1 | (2 << 16);
        assert_eq!(w.random(), 1.0 / 65536.0);
        assert_eq!(w.machine.seed, 1 << 16);
        // All-zero is a fixed point, which is why the game seeds from the clock: an
        // unseeded generator would hand every jittering step the same -12.5%.
        w.machine.seed = 0;
        assert_eq!((w.random(), w.machine.seed), (0.0, 0));
        // Whatever it draws, a jittering step stays within 12.5%.
        w.machine.seed = 0x2545_F491;
        for _ in 0..1000 {
            let r = w.random();
            assert!((0.0..1.0).contains(&r) && (JITTER * (r - 0.5)).abs() <= 0.125);
        }
    }

    /// A 30° ramp rising along +y, 200 long, as a building's face with the given triangle
    /// flags standing over a field.
    fn ramp(triangle_flags: u16) -> Ground {
        let mut g = field();
        let rise = 30f32.to_radians().tan();
        let (a, b, c, d) = (
            Vec3::new(400.0, 400.0, 0.0),
            Vec3::new(600.0, 400.0, 0.0),
            Vec3::new(600.0, 600.0, 200.0 * rise),
            Vec3::new(400.0, 600.0, 200.0 * rise),
        );
        let normal = (b - a).cross(c - a).normalize();
        let face = |a, b, c| crate::solid::SolidFace {
            a,
            b,
            c,
            normal,
            triangle_flags,
            batch_flags: 0,
            surface: None,
            damage_rate: 0.0,
        };
        g.solids.push(crate::solid::Solid {
            centre: Vec3::new(500.0, 500.0, 100.0 * rise),
            radius: 160.0,
            ground: true,
            present: true,
            mass: 0.0,
            faces: vec![face(a, b, c), face(a, c, d)],
            nodes: vec![crate::solid::SolidNode {
                centre: Vec3::new(500.0, 500.0, 100.0 * rise),
                radius: 160.0,
                faces: 0..2,
                part: 0,
                node: 0,
                open: false,
            }],
        });
        g
    }

    #[test]
    fn the_slope_brake_holds_a_climber_on_a_buildings_face_unless_it_is_a_walk_through_floor() {
        // `Control.dll:0x10015690`: the brake acts only while the body's `+0x1aa` is set, and
        // the ground contact sets it from the class of the face it holds, clear on class 2
        // (`0x1001a9b4`-`0x1001a9be`) -- a building's floor, triangle flag 2. A 30° climb is
        // past the hero's 0.6 rad cone's full speed (2 (cos 30° − cos 0.6) ÷ (1 − cos 0.6),
        // 0.47), so a face without the flag slows it and a floor does not.
        let climbed = |flags: u16| {
            let g = ramp(flags);
            let mut w = walker(Vec3::new(500.0, 420.0, 20.0), 0.0);
            w.follow_ground(&g);
            let start = w.body.position.y;
            w.body.command = [0.0, 1.0, 0.0];
            w.advance(3000.0, &g);
            (w.body.position.y - start, w.slope_brakes)
        };
        let (floor, floor_brakes) = climbed(2);
        let (face, face_brakes) = climbed(0);
        assert!(!floor_brakes && face_brakes);
        assert!(floor > 35.0, "a walk-through floor lets it climb at speed: {floor} m in 3 s");
        assert!(face < 0.6 * floor, "any other face brakes it: {face} m against {floor}");
    }

    #[test]
    fn holding_forward_walks_at_the_top_speed_one_stride_a_step() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.body.command = [0.0, 1.0, 0.0];
        w.advance(1000.0, &g);
        assert_eq!(w.machine.current, 1);
        assert_eq!(w.body.velocity, [0.0, 14.0, 0.0]);
        assert!((w.machine.step_ms - 500.0 / 14.0).abs() < 1e-3, "{}", w.machine.step_ms);
        assert_eq!(w.body.position.z, 2.0, "the sphere rests on the ground");
        assert!(w.body.position.y > 500.0 + 10.0, "{}", w.body.position.y);
        w.body.command = [0.0; 3];
        w.advance(3000.0, &g);
        assert_eq!(w.machine.current, 0);
        assert_eq!(w.body.velocity, [0.0; 3]);
    }

    #[test]
    fn a_velocity_driven_step_moves_by_the_velocity_clamped_into_its_box() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.controller.states[1].velocity.1[1] = 10.0;
        w.machine.current = 1;
        w.body.velocity = [0.0, 14.0, 0.0];
        w.body.command = [0.0, 1.0, 0.0];
        w.advance(0.0, &g);
        let step = (w.machine.step_ms / 1000.0) as f32;
        assert!((w.body.position.y - 500.0 - 10.0 * step).abs() < 1e-4, "{}", w.body.position.y);
        assert_eq!(w.body.velocity, [0.0, 14.0, 0.0], "the integrated velocity is kept");
    }

    #[test]
    fn only_an_anchor_plans_and_with_nothing_queued_the_state_plays_again() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.controller.states[1].mode = STATE_BY_VELOCITY;
        w.machine.current = 1;
        w.advance(500.0, &g);
        assert_eq!(w.machine.current, 1, "a state between anchors does not plan");
        assert!(w.machine.queue.is_empty());
        w.machine.current = 0;
        w.advance(1000.0, &g);
        assert_eq!(w.machine.current, 0, "standing still applies: its cycle");
    }

    #[test]
    fn a_state_waiting_for_a_request_code_applies_once_the_controller_holds_it() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        // Standing needs 5 m/s, which it never has; the other anchor waits for code 6.
        w.controller.states[0].velocity = ([-0.5, 5.0, -0.5], [0.5, 6.0, 0.5]);
        w.controller.states[1] = State { request: 6, ..stand() };
        w.advance(500.0, &g);
        assert_eq!(w.machine.current, 0);
        w.machine.request = 6;
        w.advance(1000.0, &g);
        assert_eq!(w.machine.current, 1);
    }

    #[test]
    fn a_pending_turn_turns_the_body_during_steps() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.body.pending[2] = 0.75;
        w.body.turn_pending = true;
        w.advance(500.0, &g);
        assert!((w.body.yaw - FRAC_PI_2).abs() < 1e-4, "{}", w.body.yaw);
        assert!((w.body.pending[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn the_hull_takes_the_whole_strafe_change_after_the_turn_clamp_and_the_turret_eases_it_back() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
        w.limits.turn = [0.1; 3];
        w.body.pending[2] = 0.75;
        w.body.turn_pending = true;
        w.body.strafe = FRAC_PI_2;
        w.advance(0.0, &g);
        // One 50 ms step: the pending quarter turn pays out 0.005, the strafe all of it.
        assert!((w.body.yaw - (FRAC_PI_2 + 0.005)).abs() < 1e-5, "{}", w.body.yaw);
        assert_eq!((w.body.strafe_previous, w.body.strafe_change), (FRAC_PI_2, FRAC_PI_2));
        assert!((w.body.heading() - 0.005).abs() < 1e-5);
        let start = w.machine.step_start_ms;
        assert_eq!(w.strafe_offset(start), 0.0, "the turret still faces ahead as the step begins");
        assert!((w.strafe_offset(start + 25.0) + FRAC_PI_4).abs() < 1e-5);
        assert_eq!(w.strafe_offset(start + 50.0), -FRAC_PI_2);
        // The hull drawn at any phase plus the offset is the heading eased by the turn alone.
        let (_, yaw) = w.drawn(start + 25.0);
        assert!((yaw + w.strafe_offset(start + 25.0) - w.drawn_heading(start + 25.0)).abs() < 1e-5);
        // The next step has no change: the offset is the whole angle, held.
        w.advance(50.0, &g);
        assert_eq!(w.body.strafe_change, 0.0);
        assert_eq!(w.strafe_offset(w.machine.step_start_ms), -FRAC_PI_2);
    }

    #[test]
    fn a_body_without_contacts_is_lifted_out_of_the_ground_and_never_falls() {
        let g = field();
        let mut sunk = walker(Vec3::new(500.0, 500.0, 0.5), 0.0);
        sunk.advance(0.0, &g);
        assert_eq!(sunk.body.position.z, 2.0, "lifted by (ground - centre) + r");
        let mut high = walker(Vec3::new(500.0, 500.0, 5.0), 0.0);
        high.advance(3000.0, &g);
        assert_eq!(high.body.position.z, 5.0);
    }

    /// A body, node 0, whose frame 1 lunges it 5 forward and 0.2 up, and a leg hung from
    /// it that frame 1 lifts 0.3, with a control point 1 below each.
    fn legs() -> (Mesh, Vec<ControlPoint>) {
        let node = |parent: u16, anim_start: u16| Node {
            name: String::new(),
            flags: 0,
            parent,
            anim_start,
            fallback_key: 4,
            slot_index: [NO_SLOT; 15],
        };
        let key =
            |time: f32, translation: [f32; 3]| Key { translation, time, rotation: [1.0, 0.0, 0.0, 0.0] };
        let mesh = Mesh {
            name: "legs".into(),
            positions: Vec::new(),
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: Vec::new(),
            nodes: vec![node(NO_PARENT, 0), node(0, 2)],
            slots: Vec::new(),
            batches: Vec::new(),
            face_two_sided: Vec::new(),
            face_flags: Vec::new(),
            face_normals: Vec::new(),
            keys: vec![
                key(0.0, [0.0; 3]),
                key(1.0, [0.0, 5.0, 0.2]),
                key(0.0, [0.0; 3]),
                key(1.0, [0.0, 0.0, 0.3]),
                key(0.0, [0.0; 3]),
            ],
            frame_map: vec![0, 1, 2, 3],
            frame_count: 2,
            sphere: None,
            corners: None,
        };
        let point = |node: u32| ControlPoint {
            name: String::new(),
            a: [0.0, f32::from_bits(node), f32::from_bits(node)],
            position: [0.0, 0.0, -1.0],
            direction: [0.0, 0.0, 1.0],
        };
        (mesh, vec![point(0), point(1)])
    }

    /// Standing on two contacts in 50 ms steps, the mesh posed at frame 1, which lifts
    /// the second leg.
    fn on_legs(z: f32, flags: [u32; 2]) -> Walker {
        let (mesh, points) = legs();
        let mut c = crate::motion::tests::hero();
        c.counts[1] = 2;
        let contact = |point: i32, flags: u32| Contact { point, flags, group: -1 };
        c.states = vec![State {
            mode: STATE_ANCHOR | STATE_GROUND_CONTACTS,
            pair_a: [1.0, 1.0],
            pair_b: [1.0, 1.0],
            contacts: vec![contact(0, flags[0]), contact(1, flags[1])],
            ..stand()
        }];
        c.costs = vec![0.0];
        Walker::new(c, &mesh, &points, Vec3::new(500.0, 500.0, z), 0.0)
    }

    #[test]
    fn a_body_on_contacts_falls_under_gravity_until_one_lands() {
        let g = field();
        let mut w = on_legs(3.0, [CONTACT_SUPPORT; 2]);
        w.advance(0.0, &g);
        assert!((w.body.position.z - (3.0 - 0.0125)).abs() < 1e-6, "{}", w.body.position.z);
        assert!((w.body.fall_speed + 0.5).abs() < 1e-6);
        w.advance(50.0, &g);
        assert!((w.body.position.z - (3.0 - 0.0125 - 0.0375)).abs() < 1e-5);
        w.advance(2000.0, &g);
        // The body's point lands 1 below the origin, its lunge left out; the lifted leg
        // is 0.3 short.
        assert!((w.body.position.z - 1.0).abs() < 1e-5, "{}", w.body.position.z);
        let feet = w.feet.as_ref().unwrap();
        let last = Frames { a: 1.0, b: 1.0, weight: 1.0 };
        assert_eq!(feet.place(0, last), Some(Vec3::new(0.0, 0.0, -1.0)));
        assert_eq!(feet.place(1, last), Some(Vec3::new(0.0, 0.0, -0.7)));
        assert_eq!(w.body.fall_speed, 0.0);
        assert_eq!(w.body.ground_normal, Vec3::Z);
        w.advance(4000.0, &g);
        assert!((w.body.position.z - 1.0).abs() < 1e-5, "it stays");
    }

    #[test]
    fn only_a_flag_one_contact_holds_the_body_up_and_a_sunken_one_lifts_it() {
        let g = field();
        let mut w = on_legs(1.0, [0, CONTACT_SUPPORT]);
        w.advance(2000.0, &g);
        assert!((w.body.position.z - 0.7).abs() < 1e-5, "the lifted leg lands: {}", w.body.position.z);
        let mut sunk = on_legs(0.2, [CONTACT_SUPPORT; 2]);
        sunk.advance(0.0, &g);
        assert!((sunk.body.position.z - 1.0).abs() < 1e-5, "{}", sunk.body.position.z);
    }

    /// A contact that sits on the body and carries the second node, as every tracked
    /// chassis's `weel_*` does: the point's own axis is the model's up.
    fn on_a_belt(flags: u32, yaw: f32) -> Walker {
        let (mesh, _) = legs();
        let mut c = crate::motion::tests::hero();
        c.counts[1] = 1;
        c.states = vec![State {
            mode: STATE_ANCHOR | STATE_GROUND_CONTACTS,
            pair_a: [1.0, 1.0],
            pair_b: [1.0, 1.0],
            contacts: vec![Contact { point: 0, flags, group: -1 }],
            ..stand()
        }];
        c.costs = vec![0.0];
        let point = ControlPoint {
            name: String::new(),
            a: [0.0, f32::from_bits(0), f32::from_bits(1)],
            position: [0.0, 0.0, -1.0],
            direction: [0.0, 0.0, 1.0],
        };
        Walker::new(c, &mesh, &[point], Vec3::new(500.0, 500.0, 3.0), yaw)
    }

    #[test]
    fn a_flag_two_contact_lays_its_carrier_node_along_the_ground_and_a_flag_one_one_does_not() {
        // A 1000 m field rising 100 in x: 5.71 degrees, normal (-0.0995, 0, 0.995).
        let g =
            quads(&[[[0.0, 0.0, 0.0], [1000.0, 0.0, 100.0], [1000.0, 1000.0, 100.0], [0.0, 1000.0, 0.0]]]);
        let slope = Vec3::new(-100.0, 0.0, 1000.0).normalize();

        let mut plain = on_a_belt(CONTACT_SUPPORT, 0.0);
        plain.advance(2000.0, &g);
        assert!(plain.placed.is_empty(), "flag 1 alone leaves the node level");

        let mut belt = on_a_belt(CONTACT_SUPPORT | CONTACT_PLACE, 0.0);
        belt.advance(2000.0, &g);
        assert_eq!(belt.placed.len(), 1);
        let (node, turn) = belt.placed[0];
        assert_eq!(node, 1, "the node the point carries, not the one it sits on");
        assert!((turn * Vec3::Z - slope).length() < 1e-5, "{:?}", turn * Vec3::Z);

        // The turn is kept in the machine's own frame, so a bot facing another way tilts
        // the same amount about its own axis.
        let mut turned = on_a_belt(CONTACT_SUPPORT | CONTACT_PLACE, FRAC_PI_2);
        turned.advance(2000.0, &g);
        let (_, about) = turned.placed[0];
        let want = Quat::from_rotation_z(-FRAC_PI_2) * slope;
        assert!((about * Vec3::Z - want).length() < 1e-5, "{:?}", about * Vec3::Z);
        let angle = |q: Quat| (q * Vec3::Z).dot(Vec3::Z).acos().to_degrees();
        assert!((angle(turn) - angle(about)).abs() < 1e-4);
        assert!((angle(turn) - 5.71).abs() < 0.01, "{}", angle(turn));
    }

    /// One node whose animation rolls it about x: frame 0 upright, frame 1 by 10 degrees,
    /// frame 2 by 30. A control point on it carries its own axis, the model's up.
    fn a_rolling_foot() -> (Mesh, Vec<ControlPoint>) {
        let roll = |degrees: f32| {
            let half = (degrees.to_radians() / 2.0) as f64;
            Key { translation: [0.0; 3], time: 0.0, rotation: [half.cos(), half.sin(), 0.0, 0.0] }
        };
        let keys: Vec<Key> = [0.0, 10.0, 30.0, 0.0]
            .iter()
            .enumerate()
            .map(|(i, &d)| Key { time: i.min(2) as f32, ..roll(d) })
            .collect();
        let mesh = Mesh {
            name: "foot".into(),
            positions: Vec::new(),
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: Vec::new(),
            nodes: vec![Node {
                name: String::new(),
                flags: 0,
                parent: NO_PARENT,
                anim_start: 0,
                fallback_key: 3,
                slot_index: [NO_SLOT; 15],
            }],
            slots: Vec::new(),
            batches: Vec::new(),
            face_two_sided: Vec::new(),
            face_flags: Vec::new(),
            face_normals: Vec::new(),
            keys,
            frame_map: vec![0, 1, 2],
            frame_count: 3,
            sphere: None,
            corners: None,
        };
        let point = ControlPoint {
            name: String::new(),
            a: [0.0, f32::from_bits(0), f32::from_bits(0)],
            position: [0.0, 0.0, -1.0],
            direction: [0.0, 0.0, 1.0],
        };
        (mesh, vec![point])
    }

    /// A machine whose three states end on frames 0, 1 and 2 of [`a_rolling_foot`], each
    /// with one contact carrying `flags`.
    fn on_a_rolling_foot(flags: u32) -> Walker {
        let (mesh, points) = a_rolling_foot();
        let mut c = crate::motion::tests::hero();
        c.counts[1] = 1;
        c.states = (0..3)
            .map(|frame| State {
                mode: STATE_ANCHOR | STATE_GROUND_CONTACTS,
                pair_a: [frame as f32, frame as f32],
                pair_b: [frame as f32, frame as f32],
                contacts: vec![Contact { point: 0, flags, group: -1 }],
                ..stand()
            })
            .collect();
        c.costs = vec![0.0; 9];
        Walker::new(c, &mesh, &points, Vec3::new(500.0, 500.0, 3.0), 0.0)
    }

    /// `Control.dll:0x1001a331`: a contact with `CONTACT_PLACE_BY_POSE` places in the
    /// states whose last pose stands its axis within 0.05 of the model's up, and not in
    /// the ones that lean it away. That is how a walker's feet come to lie flat on the
    /// ground: 2410 of the install's 2634 contacts ask for it and none of them is
    /// authored with `CONTACT_PLACE`.
    #[test]
    fn a_contact_places_only_in_the_states_whose_last_pose_stands_its_axis_up() {
        let asked = on_a_rolling_foot(CONTACT_SUPPORT | CONTACT_PLACE_BY_POSE);
        let places: Vec<bool> =
            asked.controller.states.iter().map(|s| s.contacts[0].flags & CONTACT_PLACE != 0).collect();
        // cos 10 degrees is 0.985, within 0.05 of 1; cos 30 is 0.866 and is not.
        assert_eq!(places, [true, true, false]);

        // The control: without the flag the pose decides nothing, either way.
        let plain = on_a_rolling_foot(CONTACT_SUPPORT);
        assert!(plain.controller.states.iter().all(|s| s.contacts[0].flags & CONTACT_PLACE == 0));
        let authored = on_a_rolling_foot(CONTACT_SUPPORT | CONTACT_PLACE);
        assert!(authored.controller.states.iter().all(|s| s.contacts[0].flags & CONTACT_PLACE != 0));
    }

    /// And the flag the pose sets reaches the ground contact: on a slope the foot's node
    /// is laid along it in the states that place and left alone in the one that does not.
    #[test]
    fn a_foot_the_pose_places_is_laid_along_the_ground_and_one_it_does_not_is_left_alone() {
        let g =
            quads(&[[[0.0, 0.0, 0.0], [1000.0, 0.0, 100.0], [1000.0, 1000.0, 100.0], [0.0, 1000.0, 0.0]]]);
        let slope = Vec3::new(-100.0, 0.0, 1000.0).normalize();
        let mut w = on_a_rolling_foot(CONTACT_SUPPORT | CONTACT_PLACE_BY_POSE);
        w.machine.current = 0;
        w.advance(0.0, &g);
        assert_eq!(w.placed.len(), 1, "state 0's pose stands the axis up");
        assert!((w.placed[0].1 * Vec3::Z - slope).length() < 1e-5);
        w.machine.current = 2;
        w.advance(w.machine.clock_ms + 1.0, &g);
        assert!(w.placed.is_empty(), "state 2 leans the foot 30 degrees: it is not placed");
    }

    #[test]
    fn a_belt_on_level_ground_and_one_over_nothing_are_not_turned_at_all() {
        let mut level = on_a_belt(CONTACT_SUPPORT | CONTACT_PLACE, 0.4);
        level.advance(2000.0, &field());
        assert!((level.placed[0].1.angle_between(Quat::IDENTITY)).abs() < 1e-6);
        // Off the mesh there is no face: the node stays as it stood.
        let off = quads(&[[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0], [0.0, 10.0, 0.0]]]);
        let mut away = on_a_belt(CONTACT_SUPPORT | CONTACT_PLACE, 0.0);
        away.advance(2000.0, &off);
        assert!((away.placed[0].1.angle_between(Quat::IDENTITY)).abs() < 1e-6);
    }

    #[test]
    fn the_map_edge_keeps_the_body_inside_and_a_body_on_contacts_takes_its_push_flat() {
        let g = field();
        // The box runs to x 1001, so the side inset by 2 is at 999: 4 back and the band's 3.
        let mut w = walker(Vec3::new(1003.0, 500.0, 2.0), 0.0);
        w.keep_inside(&g);
        assert!((w.body.position.x - 996.0).abs() < 1e-4, "{}", w.body.position.x);
        // 150 is 50 above the box's top: 30 back down and the band's 3.
        let mut flyer = walker(Vec3::new(500.0, 500.0, 150.0), 0.0);
        flyer.keep_inside(&g);
        assert!((flyer.body.position.z - 117.0).abs() < 1e-4, "{}", flyer.body.position.z);
        let mut legs = on_legs(150.0, [CONTACT_SUPPORT; 2]);
        legs.keep_inside(&g);
        assert_eq!(legs.body.position, Vec3::new(500.0, 500.0, 150.0), "a push straight down is dropped");
    }

    #[test]
    fn walking_into_the_map_edge_stops_inside_its_band() {
        let g = field();
        let facing_east = -FRAC_PI_2;
        let mut w = walker(Vec3::new(850.0, 500.0, 2.0), facing_east);
        w.body.command = [0.0, 1.0, 0.0];
        let mut t = 0.0;
        while t < 20_000.0 {
            t += DT;
            w.advance(t, &g);
        }
        // 60 passes a second: the band's push matches 14 m/s some 6 m into it.
        let x = w.body.position.x;
        assert!(x > 999.0 - 80.0 && x < 999.0 - 70.0, "{x}");
    }

    #[test]
    fn between_steps_the_body_and_the_blend_are_interpolated() {
        let g = field();
        let mut w = walker(Vec3::new(500.0, 500.0, 2.0), 0.0);
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
