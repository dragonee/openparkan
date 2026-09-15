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

use glam::Vec3;
use parkan_formats::control::{
    ANY_REQUEST, CONTACT_PLANTED, CONTACT_SUPPORT, Controller, PLANTED_WITHIN, STATE_FIXED,
    STATE_GROUND_CONTACTS, STATE_JITTER, State,
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
    pub request: i32,
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
    /// `frames`, on the node its first triple's third slot names (docs/13). The pose walk
    /// leaves out node 0's travel ([`Mesh::walk_pose`]): the body's own move carries it.
    pub fn place(&self, point: i32, frames: Frames) -> Option<Vec3> {
        let p = self.points.get(usize::try_from(point).ok()?)?;
        let node = usize::try_from(p.nodes().1).ok().filter(|&n| n < self.mesh.nodes.len())?;
        let (a, b, w) = (f64::from(frames.a), f64::from(frames.b), f64::from(frames.weight));
        let pose = self.mesh.world_pose_by(node, |n| self.mesh.walk_pose(n, a, b, w));
        let at = pose.apply(p.position.map(f64::from));
        Some(Vec3::new(at[0] as f32, at[1] as f32, at[2] as f32))
    }

    /// Whether `state`'s last pose plants control point `point` (`0x1001a2d5`–`0x1001a328`):
    /// the mesh posed at pair B's last frame with all of the weight on B, the root's own
    /// height kept, puts the point within 0.1 of its height at rest.
    ///
    /// STAND-IN: docs/13-control.md#a-footstep-end-to-end--read-and-measured -- which pose
    /// the live contact record's height is taken from at load is not read: the rest pose.
    pub fn planted(&self, point: i32, state: &State) -> bool {
        let Some(p) = self.points.get(usize::try_from(point).unwrap_or(usize::MAX)) else { return false };
        let Some(node) = usize::try_from(p.nodes().1).ok().filter(|&n| n < self.mesh.nodes.len()) else {
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
    /// How far the origin stands above the model's lowest point.
    pub base: f32,
    /// The face the ground search last found under the body's centre.
    pub ground: Option<Hit>,
    /// The contact points' model, where the controller has contacts and the object
    /// control points.
    pub feet: Option<Feet>,
    /// Per state, per contact: whether the state plants it (`0x1000`, set at load).
    pub planting: Vec<Vec<bool>>,
    /// Each contact's live planted byte (`+0x5a`), and the contacts that have landed
    /// since the caller last took them.
    pub planted: Vec<bool>,
    pub landed: Vec<usize>,
    /// What the Wizard writes, when the AI drives: the velocity is taken as the machine's
    /// own, and the hull turns toward the heading.
    pub drive: Option<Drive>,
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
                // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- the
                // state a machine starts in is not read; state 0.
                current: 0,
                queue: VecDeque::new(),
                clock_ms: 0.0,
                step_start_ms: 0.0,
                step_ms: 0.0,
                q: 1.0,
                q_prev: 1.0,
                // STAND-IN: docs/32-builder.md#the-construction-sphere--read-and-measured --
                // the code a controller holds before any is sent is not read; none, so a
                // state waiting for a code of its own does not apply until one is sent.
                request: ANY_REQUEST,
                seed: 0x2545_F491,
            },
            from: (position, yaw),
            from_heading: yaw,
            centre: Vec3::ZERO,
            radius,
            sphere_radius: radius,
            base,
            ground: None,
            feet: None,
            planting: Vec::new(),
            planted: Vec::new(),
            landed: Vec::new(),
            drive: None,
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
        // The use count +0x94 is not read; unlimited.
        if c.states[current].applies(v, s, code)
            && let Some((path, _)) = c.path_by(&self.costs, current, current)
        {
            self.machine.queue = path.into();
            return;
        }
        let best = (0..c.states.len())
            .filter(|&j| j != current && c.states[j].anchor() && c.states[j].applies(v, s, code))
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
        // STAND-IN: docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured
        // -- how a walker climbs a building's stairs is not read; a recording shows the hero
        // climbing the Large Factory's, whose faces stand at up to 67°, so on a building's
        // faces the slope brake is left out.
        let on_building = self.ground.is_some_and(|h| h.solid.is_some());
        if self.controller.mode == SLOPE_MODE && !on_building {
            let along = self.body.to_world(Vec3::from_array(self.body.velocity));
            let normal = self.body.ground_normal;
            // STAND-IN: docs/24-motion.md#ground-and-slope--read -- the brake is read to
            // act one way across the slope; taken as uphill, against the face normal.
            if along.x * normal.x + along.y * normal.y < 0.0 {
                let factor = motion::slope_factor(normal.z, self.controller.cone);
                motion::brake(&mut self.body.velocity, factor, &self.limits, step);
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
        self.body.position += moved;
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
    /// not read: when the ground contact runs and its dt, the frames the contact points
    /// are placed at, the second sphere's radius r₂, and what a sphere with no face under
    /// it does. It runs after every state step with dt the step, the contacts on the
    /// step's last frames, r₂ = r, and a sphere with no face is not lifted.
    fn hold(&mut self, ground: &Ground, state: &State, dt: f32) {
        let r = self.radius;
        let centre = self.body.position + self.body.to_world(self.centre);
        let hit = ground.search(centre, r);
        self.ground = hit;
        let last = Frames { a: state.pair_a[1], b: state.pair_b[1], weight: self.machine.q };
        let contacts: Vec<(Vec3, Option<Hit>)> = match &self.feet {
            Some(feet) if state.mode & STATE_GROUND_CONTACTS != 0 => state
                .contacts
                .iter()
                .filter(|c| c.flags & CONTACT_SUPPORT != 0)
                .filter_map(|c| feet.place(c.point, last))
                .map(|at| {
                    let p = self.body.position + self.body.to_world(at);
                    (p, ground.search(p, r))
                })
                .collect(),
            _ => Vec::new(),
        };
        let lift = if contacts.is_empty() {
            hit.filter(|h| centre.z - h.point.z < r).map(|h| h.point.z - centre.z + r)
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
            let normals: Vec<Vec3> =
                hit.iter().chain(contacts.iter().filter_map(|(_, h)| h.as_ref())).map(|h| h.normal).collect();
            if !normals.is_empty() {
                self.body.ground_normal = normals.iter().sum::<Vec3>() / normals.len() as f32;
            }
        }
    }

    /// `0x1001e650`, once a tick: the map edge's push on the body sphere, taken as a
    /// machine takes a push.
    pub fn keep_inside(&mut self, ground: &Ground) {
        let (lo, hi) = ground.world_box();
        let centre = self.body.position + self.body.to_world(self.centre);
        self.take_push(motion::edge_push(centre, self.sphere_radius, lo, hi));
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
        self.body.position += if whole { push } else { motion::horizontal(push) };
    }

    /// The body sphere as the ground contact takes it: the centre in the model's frame
    /// (`0x1001a518`, the agent's node sphere's), and the radius (`0x1001a487`, the agent's
    /// sphere's), held to 7.5 under 20 for the contact.
    pub fn set_body_sphere(&mut self, centre: Vec3, radius: f32) {
        self.centre = centre;
        self.sphere_radius = radius;
        self.radius = contact_radius(radius);
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
    pub fn drawn(&self, t_ms: f64) -> (Vec3, f32) {
        let s = self.phase(t_ms);
        let position = self.from.0.lerp(self.body.position, s);
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
