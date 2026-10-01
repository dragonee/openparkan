//! Rounds in flight and what they strike: `docs/29-weapons.md`, "The round's start"
//! and "How a round ends", and `docs/26-damage.md`.
//!
//! A frame moves every round and spends its range, then runs the collision pass
//! once, then fires the range groups, then delivers the contacts
//! (`World3D.dll:0x10006bf0`).

use std::rc::Rc;

use glam::Vec3;
use parkan_formats::exp::{Explosion, HIT_AREA, HIT_DIRECT, HIT_SHIELDS};
use parkan_formats::mesh::Mesh;
use parkan_formats::pose::Pose;

use crate::damage::{Change, Life, blast, round_hit, share_loss};
use crate::ground::Ground;
use crate::hit::{ROUND_SKIPS_FACE, SIGHT_SKIPS_FACE, Strike, map_edge, swept_spheres};
use crate::shield::Shield;

/// A mode-0 round's sideways speed, in its own frame, bleeds off at this many m/s a
/// millisecond (`Control.dll:0x1000ceec`).
pub const SIDEWAYS_BLEED: f32 = 0.003;
/// The sight ray runs from 5 m out (`0x1002a768`) and its aim point is pushed out
/// to at least 100 m (`0x1002a7be`).
pub const SIGHT_FROM: f32 = 5.0;
pub const SIGHT_TO: f32 = 1_000_000.0;
pub const NEAREST_AIM: f32 = 100.0;

/// A guided round's class-17 seeker (docs/29-weapons.md, "Guided rounds differ in how
/// hard they steer").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seeker {
    /// Value 0: the cone's half-angle, radians.
    pub cone: f32,
    /// Value 1: how far it follows a target, m.
    pub reach: f32,
    /// Value 2: the lock its gun waits, ms.
    pub lock_ms: f32,
}

/// What every round of one record shares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoundKind {
    pub name: String,
    /// The controller's top speed along y.
    pub top_speed: f32,
    /// File +108.
    pub range: f32,
    /// The collision radius: the mesh's stream-2 header sphere times the object's
    /// largest scale, and a round is never scaled (docs/26, "The hit test").
    pub radius: f32,
    /// Node 0's hit points: a round dies from full health.
    pub hit_points: f32,
    /// Node 0's `.exp`, which goes off where the round is killed.
    pub hit: Option<Explosion>,
    /// The `.exp` block entry 4's action 27 names, at the end of the range.
    pub range_end: Option<Explosion>,
    pub seeker: Option<Seeker>,
    /// The controller's fourth triple: the most it turns about each axis, rad/s.
    pub turn_rate: [f32; 3],
    /// Its controller's mode: 3 falls under the world's gravity (docs/24, "Gravity").
    pub mode: i32,
}

/// The controller mode whose velocity integrator adds gravity (`0x10015879`).
pub const MODE_FALLING: i32 = 3;
/// The world's gravity (`Terrain.dll:0x10024c1a`).
pub const GRAVITY: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
    /// Unique among the rounds a `Combat` has fired.
    pub id: u64,
    pub kind: usize,
    /// The target id of the robot that fired it, which it never strikes; `None` for the
    /// hero's, which never strikes the hero.
    pub owner: Option<usize>,
    pub previous: Vec3,
    pub position: Vec3,
    /// World velocity.
    pub velocity: Vec3,
    /// The round's own y, which it was created facing.
    pub forward: Vec3,
    pub remaining: f32,
    pub ratio: f32,
    /// Its node 0's life, which a shield sector it passes through takes from
    /// (`Control.dll:0x1000d1b4`).
    pub life: f32,
    /// The target its gun handed it (`0x1002a514`), which its seeker follows.
    pub target: Option<usize>,
    /// The node of that target the gun handed it with the id (`0x1002a514`, the gun's pair at
    /// `+0x108`): the player's target is handed with node 0 (`iron3d.dll:0x10091b0e`), an AI
    /// gun's with the part its fight module picked ([`Target::aim_part`]). The seeker steers
    /// at that node's own sphere (`Control.dll:0x100247c0`). `None` steers at [`Target::aim`].
    pub part: Option<(usize, usize)>,
    expired: bool,
}

/// One mesh of a target, posed.
#[derive(Clone, Debug)]
pub struct Part {
    pub mesh: Rc<Mesh>,
    /// Each node's world pose.
    pub nodes: Vec<Pose>,
    /// The object's uniform scale, which `nodes` carry in their translations.
    pub scale: f32,
    /// Its nodes' hit points; `None` where it takes no damage.
    pub life: Option<Life>,
    /// The earlier part and its node this part hangs on. In the game's one merged model a
    /// turret's or gun's own node 0 is not merged: that socket takes its place
    /// (`AniMesh.dll:0x1000a79d`, `Control.dll:0x10008c6a`, docs/28, "The order parts load
    /// in"), so the part's nodes go when the socket does.
    pub host: Option<(usize, usize)>,
    /// The triangles of the batches whose word carries 8, the portal quads: a segment passes
    /// them whatever their flags, as a mover does (docs/24, "The doorways are portal quads").
    /// Empty on a mesh that has none, which is every mesh but a building's.
    pub portals: Rc<Vec<bool>>,
}

impl Part {
    /// The level-0 slot node `node` draws and is struck through: its stage's variant's, or
    /// none once it is hidden (docs/26, "What a damaged node, a destroyed part and a dead
    /// unit draw").
    pub fn slot(&self, node: usize) -> Option<u16> {
        let n = self.mesh.nodes.get(node)?;
        let block = match self.life.as_ref().and_then(|l| l.nodes.get(node)) {
            Some(life) if life.hidden() => return None,
            Some(life) => life.block(),
            None => 0,
        };
        n.slot_index
            .get(block * parkan_formats::mesh::SLOTS_PER_VARIANT)
            .copied()
            .filter(|&s| s != parkan_formats::mesh::NO_SLOT)
    }

    /// A segment through the part as it stands, passing the triangles flagged `passes` and
    /// its portal quads.
    pub fn segment(&self, p0: Vec3, p1: Vec3, passes: u16) -> Option<Strike> {
        crate::hit::segment_mesh_skipping(
            &self.mesh,
            &self.nodes,
            self.scale,
            p0,
            p1,
            passes,
            |i| self.slot(i),
            |t| self.portals.get(t).copied().unwrap_or(false),
        )
    }
}

/// A component of an object's device manager: its class, and the node it sits on as a part
/// and a node of it (the record's `+4`, which the manager's query answers for id `0x200`,
/// docs/13, "The component record").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Device {
    pub class: i32,
    pub node: (usize, usize),
}

/// What a running-gear node weighs when the fight module picks the part to aim at
/// (`Behavior.dll:0x10025830`, the 7.5 at `0x10059978`).
pub const GEAR_WEIGHT: f32 = 7.5;

/// What a device of `class` weighs in that pick (the jump table at `0x100259c8` through the
/// byte map at `0x100259e8`): a deflector 30, a turret 20, a gun 15, an engine or a radar 14, a
/// power store 10, a fight shield 7, a repair system 3, anything else 1.
pub fn device_weight(class: i32) -> f32 {
    match class {
        21 => 30.0,
        1 => 20.0,
        2 => 15.0,
        5 | 8 => 14.0,
        19 => 10.0,
        9 => 7.0,
        15 => 3.0,
        _ => 1.0,
    }
}

/// An AI unit's line of fire is a sphere this thick, swept from end to end past each unit of
/// its own owner (`Behavior.dll:0x10024954`, into `0x10025c60`).
pub const LINE_RADIUS: f32 = 0.5;
/// A line shorter than this is clear though it meets a face flagged [`LINE_PASSES_FACE`]
/// (`0x100247aa`–`0x100247eb`, the 20 at `0x1005960c`).
pub const LINE_NEAR: f32 = 20.0;
pub const LINE_PASSES_FACE: u16 = 0x20;

/// Something a round can strike.
#[derive(Clone, Debug)]
pub struct Target {
    pub parts: Vec<Part>,
    /// The bounding sphere the pass and the blasts test first.
    pub centre: Vec3,
    pub radius: f32,
    pub alive: bool,
    /// The object's placement.
    pub position: Vec3,
    /// Its node sphere's centre in the world: interface `0x20` slot 3 asked by the default
    /// request with no node (docs/24, "Finding the ground"). A gate or a seeker with a target
    /// is always handed a node and asks that node's own sphere ([`Target::slot_sphere`],
    /// docs/29, "The part the AI aims at"); this is what one handed none falls back on.
    pub aim: Vec3,
    /// Its fight shield and deflector, where it has both; the bubble is its bounding sphere.
    pub shield: Option<Shield>,
    /// The agent's own sphere in the world, its centre and radius: its parts' stream-2 header
    /// spheres joined (`AniMesh.dll:0x10009510`), which interface `0x18` slot 9 hands out
    /// (`0x10014580`). The world's object pick asks for it (docs/42, "The object pick").
    pub agent_sphere: (Vec3, f32),
    /// Its device manager's components in its own order: the root's, then each fitted part's,
    /// an internal part standing in the slot it was fitted to.
    pub devices: Vec<Device>,
    /// The nodes whose `.ndp` flags carry `0x70`, a machine's running gear, in node order.
    pub gear: Vec<(usize, usize)>,
}

impl Target {
    pub fn dead(&self) -> bool {
        self.parts.first().and_then(|p| p.life.as_ref()).is_some_and(|l| l.dead)
    }

    /// The object's turn: its first node's, as placed.
    pub fn rotation(&self) -> glam::Quat {
        let Some(pose) = self.parts.first().and_then(|p| p.nodes.first()) else {
            return glam::Quat::IDENTITY;
        };
        let [w, x, y, z] = pose.rotation.map(|v| v as f32);
        glam::Quat::from_xyzw(x, y, z, w).normalize()
    }

    /// The share of its life node `node` of part `part` keeps: a device's condition. A node
    /// that takes no damage is whole.
    pub fn condition(&self, (part, node): (usize, usize)) -> f32 {
        let Some(n) = self.parts.get(part).and_then(|p| p.life.as_ref()).and_then(|l| l.nodes.get(node))
        else {
            return 1.0;
        };
        if n.destroyed || n.max <= 0.0 { if n.destroyed { 0.0 } else { 1.0 } } else { n.life / n.max }
    }

    /// One node's own sphere in the world: what interface `0x20` slot 3 answers a request whose
    /// fourth word is 1 (`AniMesh.dll:0x1000f3c5`) -- the sphere of the node's level-0 slot in
    /// its current variant, through the node's matrix, or the node's origin with no radius
    /// where it has no such slot (`0x1000f40e`). The turret, the gun's gate, the seeker and the
    /// fight module's line all ask it of the node they are handed.
    pub fn slot_sphere(&self, (part, node): (usize, usize)) -> Option<(Vec3, f32)> {
        let p = self.parts.get(part)?;
        let pose = p.nodes.get(node)?;
        let Some(slot) = p.slot(node).and_then(|s| p.mesh.slots.get(usize::from(s))) else {
            return Some((vec(pose.translation), 0.0));
        };
        let [cx, cy, cz, r] = slot.sphere;
        let centre = pose.apply([cx, cy, cz].map(|v| f64::from(v * p.scale)));
        Some((vec(centre), r * p.scale))
    }

    /// The part of it an AI unit aims at (`Behavior.dll:0x10025830`), the node the fight module
    /// hands the turret with the target (`0x10024b1b`, `0x10024f8f`): the heaviest of its
    /// running-gear nodes and its devices that still has life, each weighing its class's
    /// figure × (2 − its node's share of life), so a damaged part outweighs a whole one of
    /// its class. Both lists are walked from the last, and only a greater weight takes the
    /// pick, so among equals the last listed keeps it. `None` where nothing has life, which
    /// the callers take as node 0.
    pub fn aim_part(&self) -> Option<(usize, usize)> {
        self.aim_part_by(|node| self.condition(node))
    }

    /// [`Target::aim_part`] with each node's share of life from `share`: the hero's lives are
    /// its own between the battle's frames.
    pub fn aim_part_by(&self, share: impl Fn((usize, usize)) -> f32) -> Option<(usize, usize)> {
        let mut best: Option<((usize, usize), f32)> = None;
        let gear = self.gear.iter().rev().map(|&node| (node, GEAR_WEIGHT));
        let devices = self.devices.iter().rev().map(|d| (d.node, device_weight(d.class)));
        for (node, weight) in gear.chain(devices) {
            let share = share(node);
            if share <= 0.0 {
                continue;
            }
            let score = (2.0 - share) * weight;
            if score > best.map_or(0.0, |(_, b)| b) {
                best = Some((node, score));
            }
        }
        best.map(|(node, _)| node)
    }

    /// Where a seeker handed `part` steers (`Control.dll:0x100247c0`): at that node's own
    /// sphere's centre while the node has life, then at node 0's, and nowhere once node 0's is
    /// gone.
    pub fn seeker_point(&self, part: (usize, usize)) -> Option<Vec3> {
        let node = if self.condition(part) > 0.0 { part } else { (0, 0) };
        (self.condition(node) > 0.0).then(|| self.slot_sphere(node)).flatten().map(|(c, _)| c)
    }

    /// Its shield's bubble while it is up.
    fn bubble(&self) -> Option<&Shield> {
        self.shield.as_ref().filter(|s| self.alive && s.up())
    }
}

/// What a round's explosion names (`0x10011479`, `0x1000d23f`): a node it struck, a shield
/// sector of the object whose bubble stopped it, or nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Struck {
    Nothing,
    Node(usize, usize, usize),
    Bubble(usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A round struck a target's node, or the ground (`target` `None`).
    Struck { round: Round, target: Option<usize>, part: usize, node: Option<usize>, point: Vec3 },
    /// An explosion went off: a round's on a hit, or its range end's.
    /// `forward` is the round's own y, the axis a placement-0 effect goes off along.
    Exploded { kind: usize, point: Vec3, forward: Vec3, at_range: bool },
    /// Damage landed on a node, and whether it destroyed it.
    Damaged { target: usize, part: usize, node: usize, damage: f32, destroyed: bool },
    /// A node's stage rose: its explosion plays (docs/26, "What a damaged node, a destroyed
    /// part and a dead unit draw").
    Staged { target: usize, part: usize, node: usize },
    /// A node reached its last stage and is gone from the world.
    Hidden { target: usize, part: usize, node: usize },
    /// A destroyed part was knocked off, and flies from now.
    KnockedOff { target: usize, part: usize, node: usize },
    /// A node's stage fell as it gained life back, and it is in the world again if its last
    /// stage had hidden it.
    Restored { target: usize, part: usize, node: usize },
    /// A target died.
    Killed { target: usize },
    /// A hit reached a target, whatever it did to it (`0x1000ebdf`, `0x1000d1ed`): its
    /// behaviour learns who fired, `owner` as a round names it.
    Hurt { target: usize, owner: Option<usize> },
    /// A hit met a shield sector with strength left: the generator's effect plays at the
    /// bubble, turned toward `point` (`Control.dll:0x1002c83e`).
    ShieldHit { target: usize, point: Vec3 },
    /// A round left the map, with no explosion.
    Gone { round: Round },
    /// A round's flight is over, `round.position` where it stopped: `end` names the block
    /// entry whose group runs. None of the three deletes it at once: it stays its controller's
    /// `+92` ms (docs/29-weapons.md, "A beam outlives its round").
    Ended { round: Round, end: RoundEnd },
}

/// How a round's flight ended: the block entry whose group runs (`docs/29-weapons.md`, "How
/// a round ends").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundEnd {
    /// Entry 2, a hit.
    Hit,
    /// Entry 3, the map's edge.
    Edge,
    /// Entry 4, the end of its range.
    Range,
}

#[derive(Clone, Debug, Default)]
pub struct Combat {
    pub kinds: Vec<RoundKind>,
    pub rounds: Vec<Round>,
    pub targets: Vec<Target>,
    /// The player's hero, which is no placed object's target but is struck like one:
    /// numbered after the targets ([`Combat::hero_index`]), as the radar numbers it.
    pub hero: Option<Target>,
    pub fired: u64,
    /// Targets not in the world yet, which nothing strikes or sees: a building its
    /// controller has not placed in the landscape. A building made in play is hidden (node
    /// flag 1) for the same 40 s, and every segment query passes a hidden node over
    /// (`AniMesh.dll:0x10010dc0`, docs/26, "What a hidden node is left out of"); the
    /// command-mode pick, which asks for its sphere alone, still meets it.
    pub absent: std::collections::HashSet<usize>,
    /// Targets whose object is deleted, `World3D.dll!KillGameObject` once a dead unit's time is
    /// up: no object answers their id, so a round one of them fired is worth nothing where it
    /// goes off (`Control.dll:0x10011783`, docs/26, "Whose hit it is, and whom it spares").
    pub gone: std::collections::HashSet<usize>,
}

/// Part `p`'s socket, as the walk from node 0 treats the node standing in for the part's node 0
/// (`0x10011130`): once the socket is destroyed, or its stage rose in this takt (`rose`, by
/// part), the part's node 0 is destroyed, so its own walk takes every node below it. A turret
/// shot off takes its guns and its radar with it.
///
/// STAND-IN: docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured
/// -- in the game the socket stands for the part's node 0 and its knocked-off flight carries
/// the part's nodes; the engine keeps a life per part, so the part's node 0 is destroyed where
/// it stands and its nodes explode and go there, not in the air.
fn follow_socket(target: &mut Target, p: usize, rose: &[Vec<usize>]) {
    let Some((h, socket)) = target.parts[p].host.filter(|&(h, _)| h < p) else { return };
    let host = target.parts[h].life.as_ref().and_then(|l| l.nodes.get(socket));
    if !host.is_some_and(|n| n.destroyed) && !rose[h].contains(&socket) {
        return;
    }
    let Some(life) = target.parts[p].life.as_mut() else { return };
    let Some(root) = life.nodes.first().filter(|n| !n.destroyed) else { return };
    let all = root.life;
    life.lose(0, all.max(f32::MIN_POSITIVE));
}

/// A unit dead (`0x10011098`): out of the fight, and every part goes with node 0, which
/// takes the unit's whole life so the walk takes every node below it (`0x10011105`). It is
/// a vital node's death that makes this do work on the part it happened in: the hero and
/// the two monster chassis die when a leg segment goes, and node 0 is then taken with it
/// (docs/26, "What the loader takes from the mesh").
///
/// STAND-IN: docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured
/// -- a unit's life system holds the nodes of all its models under one root; the engine
/// keeps a life per part, so a dead unit's other parts' roots are destroyed with it, and a
/// part's root has no parent to be knocked off from.
fn kill(target: &mut Target) {
    target.alive = false;
    for part in target.parts.iter_mut() {
        let Some(life) = part.life.as_mut() else { continue };
        for root in 0..life.nodes.len() {
            if life.parents[root].is_none() {
                let all = life.nodes[root].life;
                life.lose(root, all.max(f32::MIN_POSITIVE));
            }
        }
    }
}

/// One tick of a seeker's steering (`Control.dll:0x100247c0`, `0x1000cd0a`). The seeker
/// answers while the target is within its reach and inside its cone. Its heading, in
/// the round's frame, gives two angles, of z against y and of x against y, a quarter
/// turn when the target is abeam. Each is asked for within the tick and held to the
/// turn rate about its axis.
///
/// STAND-IN: docs/29-weapons.md#guided-rounds-differ-in-how-hard-they-steer--read-and-measured
/// -- whether a round's velocity turns with it is not read: it is kept in the round's
/// frame, as a machine's is, so it turns with the round.
fn steer(r: &mut Round, seeker: Seeker, turn: [f32; 3], target: Vec3, dt: f32) {
    let to = target - r.position;
    let distance = to.length();
    if dt <= 0.0
        || distance <= 0.0
        || distance > seeker.reach
        || r.forward.dot(to / distance) < seeker.cone.cos()
    {
        return;
    }
    let side = r.forward.cross(Vec3::Z).normalize_or(Vec3::X);
    let up = side.cross(r.forward).normalize_or(Vec3::Z);
    let (x, y, z) = (to.dot(side), to.dot(r.forward), to.dot(up));
    let angle = |a: f32| if y == 0.0 { std::f32::consts::FRAC_PI_2.copysign(a) } else { (a / y).atan() };
    let pitch = (angle(z) / dt).clamp(-turn[0], turn[0]) * dt;
    let yaw = (-angle(x) / dt).clamp(-turn[2], turn[2]) * dt;
    let q = glam::Quat::from_axis_angle(up, yaw) * glam::Quat::from_axis_angle(side, pitch);
    r.forward = (q * r.forward).normalize_or(r.forward);
    r.velocity = q * r.velocity;
}

/// World xyz to the f64 poses use.
fn arr(v: Vec3) -> [f64; 3] {
    [f64::from(v.x), f64::from(v.y), f64::from(v.z)]
}

fn vec(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

impl Combat {
    /// The hero's number among the targets: the one after the last.
    pub fn hero_index(&self) -> usize {
        self.targets.len()
    }

    /// Target `t`, the hero's number giving the hero.
    pub fn target(&self, t: usize) -> Option<&Target> {
        match t.cmp(&self.targets.len()) {
            std::cmp::Ordering::Less => self.targets.get(t),
            std::cmp::Ordering::Equal => self.hero.as_ref(),
            std::cmp::Ordering::Greater => None,
        }
    }

    pub fn target_mut(&mut self, t: usize) -> Option<&mut Target> {
        match t.cmp(&self.targets.len()) {
            std::cmp::Ordering::Less => self.targets.get_mut(t),
            std::cmp::Ordering::Equal => self.hero.as_mut(),
            std::cmp::Ordering::Greater => None,
        }
    }

    /// Every target by its number, the hero last.
    pub fn every(&self) -> impl Iterator<Item = (usize, &Target)> {
        self.targets.iter().chain(self.hero.as_ref()).enumerate().filter(|(t, _)| !self.absent.contains(t))
    }

    /// A round leaves a muzzle (`0x1002a387`): facing `direction` with z up, at its top
    /// speed plus the shooter's world velocity, in free flight, with its gun's `target`.
    #[allow(clippy::too_many_arguments)]
    pub fn fire(
        &mut self,
        kind: usize,
        owner: Option<usize>,
        muzzle: Vec3,
        direction: Vec3,
        shooter_velocity: Vec3,
        ratio: f32,
        target: Option<usize>,
    ) -> Option<u64> {
        let k = self.kinds.get(kind)?;
        let forward = direction.normalize_or(Vec3::Y);
        self.fired += 1;
        self.rounds.push(Round {
            id: self.fired,
            kind,
            owner,
            previous: muzzle,
            position: muzzle,
            velocity: forward * k.top_speed + shooter_velocity,
            forward,
            remaining: k.range,
            ratio,
            life: ratio * k.hit_points,
            target,
            part: None,
            expired: false,
        });
        Some(self.fired)
    }

    /// The nearest thing a round's segment meets: the ground, or a live target other
    /// than `skip` (the hero, for `None`), passing the triangles a round passes. Returns the
    /// strike and the target struck.
    pub fn first_hit(
        &self,
        ground: &Ground,
        skip: Option<usize>,
        p0: Vec3,
        p1: Vec3,
        radius: f32,
    ) -> Option<(Strike, Option<usize>, usize)> {
        self.nearest(ground, skip, p0, p1, radius, ROUND_SKIPS_FACE)
    }

    /// Whether a clear line runs from `p0` to `p1`: the query an effect's view test makes.
    ///
    /// It goes into `IWorld` slot 7, the **sight ray's** own entry
    /// (`Effect.dll:0x10007f7f`), with a record that excludes no face class at all, so
    /// unlike a round's ground query it stops on a lake's surface; its one excluded world
    /// flag is on 0 of the 275882 shipped faces (docs/11, "How often the point is tested,
    /// and what the ray meets").
    pub fn clear_line(&self, ground: &Ground, p0: Vec3, p1: Vec3) -> bool {
        self.nearest_over(ground, None, p0, p1, 0.0, SIGHT_SKIPS_FACE, true).is_none()
    }

    /// [`Combat::first_hit`], passing the triangles whose flags meet `passes`.
    fn nearest(
        &self,
        ground: &Ground,
        skip: Option<usize>,
        p0: Vec3,
        p1: Vec3,
        radius: f32,
        passes: u16,
    ) -> Option<(Strike, Option<usize>, usize)> {
        self.nearest_over(ground, skip, p0, p1, radius, passes, false)
    }

    /// [`Combat::nearest`], with `water` taking the water surface as ground too.
    #[allow(clippy::too_many_arguments)]
    fn nearest_over(
        &self,
        ground: &Ground,
        skip: Option<usize>,
        p0: Vec3,
        p1: Vec3,
        radius: f32,
        passes: u16,
        water: bool,
    ) -> Option<(Strike, Option<usize>, usize)> {
        let skip = skip.unwrap_or(self.hero_index());
        self.nearest_among(ground, p0, p1, radius, passes, water, |id| id != skip)
    }

    /// [`Combat::nearest_over`] among the live targets `visit` admits.
    #[allow(clippy::too_many_arguments)]
    fn nearest_among(
        &self,
        ground: &Ground,
        p0: Vec3,
        p1: Vec3,
        radius: f32,
        passes: u16,
        water: bool,
        visit: impl Fn(usize) -> bool,
    ) -> Option<(Strike, Option<usize>, usize)> {
        let ray = if water { ground.segment_including_water(p0, p1) } else { ground.segment(p0, p1) };
        let mut best: Option<(Strike, Option<usize>, usize)> = ray.map(|s| (s, None, 0));
        for (id, target) in self.every() {
            if !target.alive || !visit(id) {
                continue;
            }
            let still = (target.centre, target.centre);
            if swept_spheres(still, target.radius, (p0, p1), radius).is_none() {
                continue;
            }
            for (p, part) in target.parts.iter().enumerate() {
                if let Some(s) = part.segment(p0, p1, passes)
                    && best.as_ref().is_none_or(|(b, _, _)| s.d2 < b.d2)
                {
                    best = Some((s, Some(id), p));
                }
            }
        }
        best
    }

    /// Whether an AI unit's line of fire from `p0` to `p1` is clear (`Behavior.dll:0x10024464`–
    /// `0x10024989`), the ends being [`crate::behaviour::fire_line`]'s.
    ///
    /// The line goes into `IWorld` slot 12 (`Terrain.dll:0x10025540`), the segment query that
    /// leaves out the objects whose ids it is handed: the firing unit and its target. Its record
    /// is `[0x40a, 0, 0, 0, 0, 8, 0, 0]` (`0x10024470`–`0x100244bc`): classes 1, 3 and 10 -- the
    /// landscape, buildings and scenery, and no unit -- with no triangle passed and no face
    /// class excluded, so a lake's sheet stops it as it stops the sight. `solid` says which
    /// targets are of those classes. Whatever it meets blocks the line, but for a face flagged
    /// [`LINE_PASSES_FACE`] met by a line under [`LINE_NEAR`] long.
    ///
    /// With nothing met, each unit in `allies` -- the units of the firing unit's owner, itself
    /// left out (`0x10024802`–`0x1002497b`) -- blocks it when a sphere of [`LINE_RADIUS`] swept
    /// along the line touches its agent sphere (`0x10025c60`).
    pub fn fire_line_clear(
        &self,
        ground: &Ground,
        p0: Vec3,
        p1: Vec3,
        solid: impl Fn(usize) -> bool,
        allies: impl IntoIterator<Item = usize>,
    ) -> bool {
        if let Some((strike, struck, part)) =
            self.nearest_among(ground, p0, p1, 0.0, SIGHT_SKIPS_FACE, true, solid)
        {
            let flags = struck
                .and_then(|id| self.target(id))
                .and_then(|t| t.parts.get(part))
                .zip(strike.triangle)
                .and_then(|(p, t)| p.mesh.face_flags.get(t).copied());
            return flags.is_some_and(|f| f & LINE_PASSES_FACE != 0) && p0.distance(p1) < LINE_NEAR;
        }
        // `0x10025d79`: a line with no length to speak of sweeps nothing.
        let sweeps = (p1 - p0).length_squared() >= 0.001;
        !allies.into_iter().filter_map(|id| self.target(id)).any(|ally| {
            let (centre, radius) = ally.agent_sphere;
            let inside = p0.distance_squared(centre) < (LINE_RADIUS + radius).powi(2);
            inside || (sweeps && swept_spheres((p0, p1), LINE_RADIUS, (centre, centre), radius).is_some())
        })
    }

    /// The sight's aim point (`0x1002a610`): the first thing the ray from `origin` along
    /// `direction` meets, pushed out to at least 100 m; `None` when it meets nothing.
    /// The ray asks for every class and passes no triangle (`0x1002adc0`), so it can stop
    /// on leaves a round flies through.
    ///
    /// **The landscape is the first object the ray walks** (*read*): `IWorld` slot 7 takes
    /// the world's root (`Terrain.dll:0x10024feb`) and the root is the landscape --
    /// `CLightning::Init` panics "Root object is not a landscape" unless its class is 1
    /// (`0x10071c41`) -- the walk admits an object whose `1 << class` meets the query's
    /// mask (`0x1002510f`, the table of `1 << N` at `0x1009a5f0`), which `0xfff` does, and
    /// the landscape's interface `0x18` slot 6 is `GetFirstIntersectedFace`
    /// (`0x1001a202`, vtable `0x1009a3f8`). The ray excludes no face, so unlike a round it
    /// stops on a lake's surface.
    pub fn aim_point(
        &self,
        ground: &Ground,
        owner: Option<usize>,
        origin: Vec3,
        direction: Vec3,
    ) -> Option<Vec3> {
        let s = direction.normalize_or(Vec3::Y);
        // The ray to 1,000,000 m is cut at the map box's far corner: nothing lies beyond.
        let (lo, hi) = ground.bounds();
        let reach = Vec3::new(hi[0] - lo[0], hi[1] - lo[1], 0.0).length() + 2.0 * NEAREST_AIM;
        let (p0, p1) = (origin + s * SIGHT_FROM, origin + s * reach.min(SIGHT_TO));
        let (strike, _, _) = self.nearest_over(ground, owner, p0, p1, 0.0, SIGHT_SKIPS_FACE, true)?;
        let mut point = strike.point;
        if (point - origin).length() < NEAREST_AIM {
            point = origin + s * NEAREST_AIM;
        }
        Some(point)
    }

    /// One frame of `dt` seconds.
    pub fn tick(&mut self, dt: f32, ground: &Ground) -> Vec<Event> {
        let mut events = Vec::new();
        // The power tick's shields: each device's condition from its node, then the charge.
        for target in self.targets.iter_mut().chain(self.hero.as_mut()) {
            if !target.alive {
                continue;
            }
            let (Some(sn), Some(dn)) = (
                target.shield.as_ref().map(|s| s.shield_node),
                target.shield.as_ref().map(|s| s.deflector_node),
            ) else {
                continue;
            };
            let (sc, dc) = (sn.map_or(1.0, |n| target.condition(n)), dn.map_or(1.0, |n| target.condition(n)));
            if let Some(shield) = target.shield.as_mut() {
                shield.shield_condition = sc;
                shield.deflector_condition = dc;
                shield.tick(dt);
            }
        }
        // Message 1: every round moves and spends its range (`0x1000cbb0`), a guided one
        // turning toward its target first (`0x1000ccc5`).
        let (targets, hero) = (&self.targets, self.hero.as_ref());
        let target_at = |t: usize| if t == targets.len() { hero } else { targets.get(t) };
        for r in &mut self.rounds {
            let k = &self.kinds[r.kind];
            if let Some(seeker) = k.seeker
                && let Some(target) = r.target.and_then(target_at).filter(|t| t.alive)
            {
                let point = match r.part {
                    Some(part) => target.seeker_point(part),
                    None => Some(target.aim),
                };
                if let Some(point) = point {
                    steer(r, seeker, k.turn_rate, point, dt);
                }
            }
            if k.mode == MODE_FALLING {
                // A lobbed round falls, and turns along its flight.
                r.velocity.z -= GRAVITY * dt;
                r.forward = r.velocity.normalize_or(r.forward);
            } else {
                let side = r.forward.cross(Vec3::Z).normalize_or(Vec3::X);
                let up = side.cross(r.forward).normalize_or(Vec3::Z);
                let bleed = SIDEWAYS_BLEED * dt * 1000.0;
                let toward_zero = |v: f32| if v.abs() <= bleed { 0.0 } else { v - bleed.copysign(v) };
                let (along, x, z) = (r.velocity.dot(r.forward), r.velocity.dot(side), r.velocity.dot(up));
                r.velocity = r.forward * along + side * toward_zero(x) + up * toward_zero(z);
            }
            r.previous = r.position;
            let moved = r.velocity * dt;
            let length = moved.length();
            if length >= r.remaining {
                // Clamped at the range end (`0x1000cfd0`).
                r.position += moved * (r.remaining / length.max(f32::MIN_POSITIVE));
                r.remaining = 0.0;
                r.expired = true;
            } else {
                r.position += moved;
                r.remaining -= length;
            }
        }

        // The collision pass, and the range groups before the contacts are delivered.
        let (lo, hi) = ground.bounds();
        let (zlo, zhi) = ground.land.bounds();
        let box_lo = Vec3::new(lo[0], lo[1], zlo[2]);
        let box_hi = Vec3::new(hi[0], hi[1], zhi[2] * 2.0);
        let rounds = std::mem::take(&mut self.rounds);
        let mut flying = Vec::with_capacity(rounds.len());
        for r in rounds {
            let k = self.kinds[r.kind].clone();
            if r.expired {
                events.push(Event::Exploded {
                    kind: r.kind,
                    point: r.position,
                    forward: r.forward,
                    at_range: true,
                });
                if let Some(e) = &k.range_end {
                    self.explode(e, &k, &r, r.position, Struck::Nothing, &mut events);
                }
                events.push(Event::Ended { round: r, end: RoundEnd::Range });
                continue;
            }
            let hit = self.first_hit(ground, r.owner, r.previous, r.position, k.radius);
            let edge = map_edge(box_lo, box_hi, r.previous, r.position);
            // The bubbles it meets nearer than its face or the edge (`0x1000d0c0`): each sector
            // with strength flashes; a round with more life than the strength passes, emptying
            // the sector, and otherwise it stops on the bubble.
            let nearest = hit.as_ref().map_or(f32::MAX, |h| h.0.d2).min(edge.map_or(f32::MAX, |e| e.1));
            let mut r = r;
            let mut stopped = None;
            for (t, point, _) in self.bubble_contacts(&r, k.radius, nearest) {
                let Some(target) = self.target_mut(t) else { continue };
                let (centre, rotation) = (target.centre, target.rotation());
                let Some(shield) = target.shield.as_mut() else { continue };
                let s = Shield::sector_of(centre, rotation, point);
                let strength = shield.strength(s);
                if strength > 0.0 {
                    events.push(Event::ShieldHit { target: t, point });
                }
                if r.life > strength {
                    shield.empty(s);
                    r.life -= strength;
                    events.push(Event::Hurt { target: t, owner: r.owner });
                } else {
                    stopped = Some((t, s, point));
                    break;
                }
            }
            if let Some((t, s, point)) = stopped {
                let mut struck = r;
                struck.position = point;
                events.push(Event::Struck { round: struck, target: Some(t), part: 0, node: None, point });
                events.push(Event::Exploded { kind: r.kind, point, forward: r.forward, at_range: false });
                if let Some(e) = &k.hit {
                    self.explode(e, &k, &r, point, Struck::Bubble(t, s), &mut events);
                }
                events.push(Event::Ended { round: struck, end: RoundEnd::Hit });
                continue;
            }
            match (hit, edge) {
                (Some((strike, target, part)), e) if e.is_none_or(|(_, d2)| strike.d2 <= d2) => {
                    let mut struck = r;
                    struck.position = strike.point;
                    events.push(Event::Struck {
                        round: struck,
                        target,
                        part,
                        node: strike.node,
                        point: strike.point,
                    });
                    events.push(Event::Exploded {
                        kind: r.kind,
                        point: strike.point,
                        forward: r.forward,
                        at_range: false,
                    });
                    if let Some(e) = &k.hit {
                        let direct = target
                            .zip(strike.node)
                            .map_or(Struck::Nothing, |(t, n)| Struck::Node(t, part, n));
                        self.explode(e, &k, &r, strike.point, direct, &mut events);
                    }
                    events.push(Event::Ended { round: struck, end: RoundEnd::Hit });
                }
                (_, Some((point, _))) => {
                    let mut gone = r;
                    gone.position = point;
                    events.push(Event::Gone { round: gone });
                    events.push(Event::Ended { round: gone, end: RoundEnd::Edge });
                }
                _ => flying.push(r),
            }
        }
        self.rounds = flying;
        events
    }

    /// The bubbles a round meets over its move, nearer than `within` (squared): each live
    /// target's but its owner's, nearest first.
    fn bubble_contacts(&self, r: &Round, radius: f32, within: f32) -> Vec<(usize, Vec3, f32)> {
        let skip = r.owner.unwrap_or(self.hero_index());
        let mut contacts: Vec<(usize, Vec3, f32)> = self
            .every()
            .filter(|&(t, target)| t != skip && target.bubble().is_some())
            .filter_map(|(t, target)| {
                let (point, d2) =
                    crate::shield::contact(r.previous, r.position, radius, target.centre, target.radius)?;
                (d2 < within).then_some((t, point, d2))
            })
            .collect();
        contacts.sort_by(|a, b| a.2.total_cmp(&b.2));
        contacts
    }

    /// A hit's shield step on target `t` (`0x1000ff00`): a hit carrying a sector takes it, and
    /// any other is given one where its sphere of `radius` about `point` crosses the bubble
    /// from outside; a sector with strength flashes, and stops what it can. Returns the damage
    /// left.
    fn shield_step(
        &mut self,
        t: usize,
        point: Vec3,
        radius: f32,
        carried: Option<usize>,
        damage: f32,
        events: &mut Vec<Event>,
    ) -> f32 {
        let Some(target) = self.target_mut(t) else { return damage };
        if target.bubble().is_none() {
            return damage;
        }
        let (centre, rotation, r) = (target.centre, target.rotation(), target.radius);
        let d = (point - centre).length();
        let Some(s) =
            carried.or_else(|| (r < d && d < r + radius).then(|| Shield::sector_of(centre, rotation, point)))
        else {
            return damage;
        };
        let Some(shield) = target.shield.as_mut() else { return damage };
        if carried.is_none() && shield.strength(s) > 0.0 {
            events.push(Event::ShieldHit { target: t, point });
        }
        damage - shield.absorb(s, damage)
    }

    /// A round's `.exp` going off (`0x1000ebc0`): kind 2 on the node struck, kind 3 a
    /// blast over every node in reach, each for `ratio × (hit points + damage)`, and kind 4
    /// on shields alone. A shield first stops what it can: all that is left of a round
    /// stopped by a bubble is spent, and what is left of a blast goes on to the nodes.
    ///
    /// The hit carries the id of the object that fired the round, the whole robot or building
    /// its gun is on (`+0x18`, property `0x7f`, `0x10011766`). **Its nodes take nothing of
    /// it**: the object whose id that is leaves after the shield step and before its first
    /// node (`0x1000ed4e`, and `0x1000ee0c` for the node a direct hit names), so a blast spares
    /// the unit that fired it, every node of it, while its shield pays for it as anyone's does
    /// and it is told of the hit like the rest. No clan is asked: a blast hurts the firer's own
    /// side. And a hit whose firer answers no id any more is worth 0 (`0x10011783`), which the
    /// queue passes over whole (`0x10012f0b`): nothing is hurt and nobody is told.
    fn explode(
        &mut self,
        e: &Explosion,
        kind: &RoundKind,
        round: &Round,
        point: Vec3,
        struck: Struck,
        events: &mut Vec<Event>,
    ) {
        if round.owner.is_some_and(|o| self.gone.contains(&o)) {
            return;
        }
        let firer = round.owner.unwrap_or(self.hero_index());
        let damage = round_hit(round.ratio, kind.hit_points, e.damage);
        let carried = |t: usize| match struck {
            Struck::Bubble(b, s) if b == t => Some(s),
            _ => None,
        };
        let hurt = |t: usize| Event::Hurt { target: t, owner: round.owner };
        match e.kind {
            HIT_DIRECT => match struck {
                Struck::Node(t, p, n) => {
                    events.push(hurt(t));
                    if t != firer {
                        self.damage(t, p, n, damage, events);
                    }
                }
                Struck::Bubble(t, s) => {
                    events.push(hurt(t));
                    self.shield_step(t, point, e.radius, Some(s), damage, events);
                }
                Struck::Nothing => {}
            },
            HIT_SHIELDS => {
                for t in 0..=self.targets.len() {
                    let Some(target) = self.target(t) else { continue };
                    if !target.alive || (target.centre - point).length() >= target.radius + e.radius {
                        continue;
                    }
                    events.push(hurt(t));
                    self.shield_step(t, point, e.radius, carried(t), damage, events);
                }
            }
            HIT_AREA => {
                for t in 0..=self.targets.len() {
                    let Some(target) = self.target(t) else { continue };
                    if !target.alive || (target.centre - point).length() >= target.radius + e.radius {
                        continue;
                    }
                    let parts = target.parts.len();
                    events.push(hurt(t));
                    let damage = self.shield_step(t, point, e.radius, carried(t), damage, events);
                    if damage <= 0.0 || t == firer {
                        continue;
                    }
                    for p in 0..parts {
                        let Some(part) = self.target(t).and_then(|x| x.parts.get(p)) else { continue };
                        let spheres: Vec<(usize, f32)> = part
                            .mesh
                            .nodes
                            .iter()
                            .enumerate()
                            .filter_map(|(n, _)| {
                                let slot = part.mesh.slots.get(usize::from(part.slot(n)?))?;
                                let [cx, cy, cz, r] = slot.sphere;
                                let local = Vec3::new(cx, cy, cz) * part.scale;
                                let centre = vec(part.nodes.get(n)?.apply(arr(local)));
                                let r = r * part.scale;
                                Some((n, blast(damage, e.radius, r, (centre - point).length())))
                            })
                            .filter(|&(_, d)| d > 0.0)
                            .collect();
                        for (n, d) in spheres {
                            self.damage(t, p, n, d, events);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// What a unit's ground deals it in a life update (docs/24, "Water and lava beds kill"):
    /// `loss` shared over the nodes of every part that takes damage, as a hit's events.
    pub fn ground_loss(&mut self, t: usize, loss: f32) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(target) = self.target_mut(t).filter(|t| t.alive) else { return events };
        let (parts, mut lives): (Vec<usize>, Vec<&mut Life>) = target
            .parts
            .iter_mut()
            .enumerate()
            .filter_map(|(p, part)| Some((p, part.life.as_mut()?)))
            .unzip();
        let before: Vec<f32> = lives.iter().map(|l| l.total()).collect();
        let gone = share_loss(&mut lives, loss);
        for (i, destroyed) in gone.into_iter().enumerate() {
            let damage = before[i] - lives[i].total();
            if damage > 0.0 {
                let destroyed = !destroyed.is_empty();
                events.push(Event::Damaged { target: t, part: parts[i], node: 0, damage, destroyed });
            }
        }
        if target.dead() {
            target.alive = false;
            events.push(Event::Killed { target: t });
        }
        events
    }

    /// `ILifeSystem` slot 7, the kill (`Control.dll:0x1000eb70`): node 0 loses the object's
    /// whole maximum, every node's summed, through `0x10010f30` past the armour, so the object
    /// dies and the stages that rise play their explosions at the next takt. The slot does
    /// nothing to an object whose invulnerability byte (`+0x5ac`) is set; the caller asks that.
    pub fn life_kill(&mut self, t: usize) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(target) = self.target_mut(t).filter(|t| t.alive) else { return events };
        let whole: f32 = target.parts.iter().filter_map(|p| p.life.as_ref()).map(Life::full).sum();
        let Some(life) = target.parts.first_mut().and_then(|p| p.life.as_mut()) else { return events };
        let was_dead = life.dead;
        let destroyed = life.lose(0, whole);
        let now_dead = life.dead;
        events.push(Event::Damaged { target: t, part: 0, node: 0, damage: whole, destroyed });
        if now_dead && !was_dead {
            kill(target);
            events.push(Event::Killed { target: t });
        }
        events
    }

    /// Every target's lives after the tick's hits, at `now_ms` ([`Life::takt`]): the stages
    /// that rose, the nodes hidden and the parts knocked off, as events.
    pub fn takt_lives(&mut self, now_ms: f64) -> Vec<Event> {
        let mut events = Vec::new();
        for (t, target) in self.targets.iter_mut().chain(self.hero.as_mut()).enumerate() {
            // A death by any other way than a round, the ground's loss say.
            if target.alive && target.dead() {
                kill(target);
                events.push(Event::Killed { target: t });
            }
            // Parts load after their hosts, so a socket's walk has run before its part's.
            let mut rose: Vec<Vec<usize>> = vec![Vec::new(); target.parts.len()];
            for p in 0..target.parts.len() {
                follow_socket(target, p, &rose);
                let Some(life) = target.parts[p].life.as_mut() else { continue };
                for change in life.takt(now_ms) {
                    events.push(match change {
                        Change::Staged(node) => {
                            rose[p].push(node);
                            Event::Staged { target: t, part: p, node }
                        }
                        Change::Hidden(node) => Event::Hidden { target: t, part: p, node },
                        Change::KnockedOff(node) => Event::KnockedOff { target: t, part: p, node },
                        Change::Restored(node) => Event::Restored { target: t, part: p, node },
                    });
                }
            }
        }
        events
    }

    fn damage(&mut self, t: usize, p: usize, n: usize, damage: f32, events: &mut Vec<Event>) {
        let Some(target) = self.target_mut(t) else { return };
        let Some(life) = target.parts.get_mut(p).and_then(|part| part.life.as_mut()) else { return };
        let was_dead = life.dead;
        let destroyed = life.hit(n, damage);
        let now_dead = life.dead;
        events.push(Event::Damaged { target: t, part: p, node: n, damage, destroyed });
        if now_dead && !was_dead && p == 0 {
            kill(target);
            events.push(Event::Killed { target: t });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::Life;
    use crate::ground::tests::floor;
    use parkan_formats::mesh::{NO_SLOT, Node, Slot};
    use parkan_formats::ndp::NodeDamage;
    use parkan_formats::objects::ResourceRef;
    use parkan_formats::pose::IDENTITY;

    fn explosion(kind: i32, damage: f32, radius: f32) -> Explosion {
        Explosion { kind, damage, radius, values: [1.0; 2], placement: 7, slots: Vec::new() }
    }

    /// A post: one node, a square facing -y at y = 0 from x -1 to 1, z 0 to 2.
    fn post(at: Vec3, hit_points: f32) -> Target {
        let mesh = Mesh {
            name: "post".into(),
            positions: vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 2.0], [-1.0, 0.0, 2.0]],
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            nodes: vec![Node {
                name: "post".into(),
                flags: 0,
                parent: 0xFFFF,
                anim_start: 0xFFFF,
                fallback_key: 0,
                slot_index: std::array::from_fn(|k| if k == 0 { 0 } else { NO_SLOT }),
            }],
            slots: vec![Slot {
                first_triangle: 0,
                triangle_count: 2,
                first_batch: 0,
                batch_count: 0,
                aabb_min: [-1.0, 0.0, 0.0],
                aabb_max: [1.0, 0.0, 2.0],
                sphere: [0.0, 0.0, 1.0, 1.5],
                area: 0.0,
                volume: 0.0,
            }],
            batches: Vec::new(),
            face_two_sided: Vec::new(),
            face_flags: vec![0, 0],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
            corners: None,
        };
        let table = vec![NodeDamage {
            flags: 0,
            durability: hit_points,
            density: 0.0,
            explosion: ResourceRef::default(),
        }];
        let life = Life::new(&table, vec![None], vec![false], 1.0, 1.0);
        let pose = Pose { translation: [f64::from(at.x), f64::from(at.y), f64::from(at.z)], ..IDENTITY };
        Target {
            parts: vec![Part {
                mesh: Rc::new(mesh),
                nodes: vec![pose],
                scale: 1.0,
                life: Some(life),
                portals: Rc::default(),
                host: None,
            }],
            centre: at + Vec3::Z,
            radius: 1.5,
            alive: true,
            position: at,
            aim: at,
            shield: None,
            agent_sphere: (at + Vec3::Z, 1.5),
            devices: Vec::new(),
            gear: Vec::new(),
        }
    }

    fn laser() -> RoundKind {
        RoundKind {
            name: "bl_h_01".into(),
            top_speed: 10_000.0,
            range: 1000.0,
            radius: 0.12,
            hit_points: 249.0,
            hit: Some(explosion(HIT_DIRECT, 1.0, 1.0)),
            range_end: Some(explosion(HIT_DIRECT, 1.0, 1.0)),
            ..RoundKind::default()
        }
    }

    /// A post behind a small warbot's shield: 350 a sector at 0.7.
    fn shielded_post() -> Target {
        let mut post = post(Vec3::new(20.0, 30.0, 0.0), 500.0);
        post.shield = Some(Shield::new(350.0, 0.0, 0.03, [0.7; crate::shield::SECTORS]));
        post
    }

    fn damage_of(events: &[Event]) -> Vec<f32> {
        events
            .iter()
            .filter_map(|e| if let Event::Damaged { damage, .. } = e { Some(*damage) } else { None })
            .collect()
    }

    /// A unit of three parts: a body whose node 1 is a turret's socket, the turret, whose node
    /// 1 carries a gun, and the gun.
    fn turret_unit() -> Target {
        let part = |points: &[f32], parents: Vec<Option<usize>>, host: Option<(usize, usize)>| {
            let mut p = post(Vec3::ZERO, 1.0).parts.remove(0);
            let table: Vec<NodeDamage> = points
                .iter()
                .map(|&d| NodeDamage {
                    flags: 0,
                    durability: d,
                    density: 0.0,
                    explosion: ResourceRef::default(),
                })
                .collect();
            let vital = vec![false; parents.len()];
            p.life = Some(Life::new(&table, parents, vital, 1.0, 1.0));
            p.host = host;
            p
        };
        let mut unit = post(Vec3::ZERO, 1.0);
        unit.parts = vec![
            part(&[500.0, 1.0], vec![None, Some(0)], None),
            part(&[1.0, 270.0, 1.0], vec![None, Some(0), Some(1)], Some((0, 1))),
            part(&[150.0, 150.0], vec![None, Some(0)], Some((1, 2))),
        ];
        unit
    }

    #[test]
    fn a_turret_shot_off_its_body_takes_its_gun_and_a_gun_its_nodes() {
        let mut c = Combat::default();
        c.targets.push(turret_unit());
        let life = |c: &Combat, p: usize| c.targets[0].parts[p].life.clone().unwrap();
        // The turret's main node destroyed: its socket for the gun goes with it in the turret's
        // walk, and the gun's nodes in the gun's, in the same takt.
        c.targets[0].parts[1].life.as_mut().unwrap().hit(1, 270.0);
        c.takt_lives(0.0);
        assert!(life(&c, 1).nodes[2].destroyed);
        assert!(life(&c, 2).nodes.iter().all(|n| n.destroyed && n.hidden()), "{:?}", life(&c, 2).nodes);
        assert!(!life(&c, 0).nodes[1].destroyed && c.targets[0].alive, "the body stands");

        // The body's socket destroyed: the turret goes, and its gun through it.
        let mut c = Combat::default();
        c.targets.push(turret_unit());
        c.targets[0].parts[0].life.as_mut().unwrap().hit(1, 1.0);
        c.takt_lives(0.0);
        assert!(life(&c, 1).nodes.iter().all(|n| n.destroyed));
        assert!(life(&c, 2).nodes.iter().all(|n| n.destroyed));

        // A gun shot off leaves the turret whole.
        let mut c = Combat::default();
        c.targets.push(turret_unit());
        c.targets[0].parts[2].life.as_mut().unwrap().hit(1, 150.0);
        c.takt_lives(0.0);
        assert!(life(&c, 1).nodes.iter().all(|n| !n.destroyed));
    }

    #[test]
    fn the_life_systems_kill_takes_node_0_past_the_armour_and_the_unit_with_it() {
        let mut c = Combat::default();
        c.targets.push(turret_unit());
        // Armour that would leave a hit next to nothing.
        c.targets[0].parts[0].life.as_mut().unwrap().armour = Some((1.0e6, 1.0e6));
        let events = c.life_kill(0);
        assert_eq!(damage_of(&events), vec![1073.0], "every node's maximum, summed");
        assert!(events.contains(&Event::Killed { target: 0 }));
        assert!(!c.targets[0].alive);
        let body = c.targets[0].parts[0].life.clone().unwrap();
        assert!(body.dead && body.nodes[0].destroyed);
        assert!(c.targets[0].parts.iter().all(|p| p.life.as_ref().unwrap().nodes[0].destroyed));
        assert!(c.life_kill(0).is_empty(), "a dead unit is not killed again");
    }

    #[test]
    fn a_bubble_stops_a_round_it_outlives_and_flashes_and_a_stronger_round_empties_the_sector_and_goes_on() {
        let g = floor();
        let bullet = RoundKind { name: "bb_t_01".into(), hit_points: 99.0, ..laser() };
        let mut c =
            Combat { kinds: vec![bullet, laser()], targets: vec![shielded_post()], ..Default::default() };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        let back = crate::shield::BACK;

        // A bullet of 100 against the back sector's 245: stopped on the bubble, its 100 taken
        // from the sector, nothing from the post.
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(damage_of(&events).is_empty(), "{events:?}");
        assert_eq!(events.iter().filter(|e| matches!(e, Event::ShieldHit { target: 0, .. })).count(), 1);
        let ended = events
            .iter()
            .find_map(|e| if let Event::Ended { round, .. } = e { Some(round.position) } else { None });
        assert!((ended.unwrap().y - 28.5).abs() < 1e-3, "on the bubble's surface: {ended:?}");
        let shield = c.targets[0].shield.clone().unwrap();
        assert!((shield.fills[back] * 350.0 - (350.0 - 100.0 / 0.7)).abs() < 0.05, "{:?}", shield.fills);
        assert_eq!(shield.fills[crate::shield::FRONT], 1.0);

        // A laser of 249 life outlives the 145 left: it flashes, empties the sector and strikes.
        c.fire(1, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(events.iter().any(|e| matches!(e, Event::ShieldHit { .. })));
        assert_eq!(damage_of(&events), vec![250.0]);
        assert_eq!(c.targets[0].shield.as_ref().unwrap().fills[back], 0.0);

        // A spent sector still meets a round, but has nothing to flash or to stop.
        c.fire(1, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(!events.iter().any(|e| matches!(e, Event::ShieldHit { .. })));
        assert_eq!(damage_of(&events), vec![250.0]);
    }

    #[test]
    fn a_blast_crossing_the_bubble_is_stopped_first_and_one_inside_it_meets_no_shield() {
        let g = floor();
        // A round whose range ends 5 m short of the post's centre, in a blast of 10.
        let shell = |damage: f32| RoundKind {
            name: "shell".into(),
            top_speed: 1200.0,
            range: 20.0,
            radius: 0.1,
            hit_points: 0.0,
            hit: None,
            range_end: Some(explosion(HIT_AREA, damage, 10.0)),
            ..RoundKind::default()
        };
        let mut c = Combat {
            kinds: vec![shell(200.0), shell(1000.0)],
            targets: vec![shielded_post()],
            ..Default::default()
        };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(events.iter().any(|e| matches!(e, Event::ShieldHit { .. })), "{events:?}");
        assert!(damage_of(&events).is_empty(), "200 stopped whole: {events:?}");

        // 1000 against the 102.9 the back sector has left: the rest reaches the node whole.
        let left = c.targets[0].shield.as_ref().unwrap().strength(crate::shield::BACK);
        c.fire(1, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        let damage = damage_of(&events);
        assert_eq!(damage.len(), 1);
        assert!((damage[0] - (1000.0 - left)).abs() < 0.05, "{damage:?} with {left} left");

        // Fired from inside the bubble, the same blast goes off inside it: no shield.
        let mut c =
            Combat { kinds: vec![shell(200.0)], targets: vec![shielded_post()], ..Default::default() };
        c.kinds[0].range = 0.5;
        c.fire(0, None, Vec3::new(20.0, 29.0, 1.0), Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(!events.iter().any(|e| matches!(e, Event::ShieldHit { .. })));
        assert_eq!(damage_of(&events), vec![200.0]);
    }

    #[test]
    fn two_laser_hits_kill_a_five_hundred_point_post_and_nothing_tunnels() {
        let g = floor();
        let mut c = Combat {
            kinds: vec![laser()],
            rounds: Vec::new(),
            fired: 0,
            hero: None,
            targets: vec![post(Vec3::new(20.0, 30.0, 0.0), 500.0)],
            absent: Default::default(),
            gone: Default::default(),
        };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        for shot in 0..2 {
            c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
            let events = c.tick(1.0 / 60.0, &g);
            assert!(c.rounds.is_empty(), "the round ended in its first frame");
            let damage: Vec<f32> = events
                .iter()
                .filter_map(|e| if let Event::Damaged { damage, .. } = e { Some(*damage) } else { None })
                .collect();
            assert_eq!(damage, vec![250.0]);
            assert_eq!(events.iter().any(|e| matches!(e, Event::Killed { target: 0 })), shot == 1);
            // Its flight ends where it struck the post's face, and the hit group runs.
            let ended: Vec<_> = events
                .iter()
                .filter_map(|e| {
                    if let Event::Ended { round, end } = e { Some((round.position, *end)) } else { None }
                })
                .collect();
            assert_eq!(ended.len(), 1, "{events:?}");
            assert_eq!(ended[0].1, RoundEnd::Hit);
            assert!((ended[0].0.y - 30.0).abs() < 1e-3, "{:?}", ended[0].0);
        }
        let life = c.targets[0].parts[0].life.as_ref().unwrap();
        assert!(life.dead && !c.targets[0].alive);
        // Dead, it no longer stops a round: this one runs to the map's edge.
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(events.iter().any(|e| matches!(e, Event::Gone { .. })), "{events:?}");
        assert!(events.iter().any(|e| matches!(e, Event::Ended { end: RoundEnd::Edge, .. })), "{events:?}");
    }

    #[test]
    fn the_sight_stops_on_a_face_a_round_flies_through() {
        let g = floor();
        let mut c = Combat { kinds: vec![laser()], ..Default::default() };
        // A flagged post 110 m out, like a tree's leaves, and a plain one behind it.
        let mut leaves = post(Vec3::new(20.0, 120.0, 0.0), 500.0);
        let mesh = Rc::make_mut(&mut leaves.parts[0].mesh);
        mesh.face_flags = vec![crate::hit::ROUND_SKIPS_FACE; 2];
        c.targets = vec![leaves, post(Vec3::new(20.0, 140.0, 0.0), 500.0)];
        let origin = Vec3::new(20.0, 10.0, 1.0);
        let aim = c.aim_point(&g, None, origin, Vec3::Y).unwrap();
        assert!((aim - Vec3::new(20.0, 120.0, 1.0)).length() < 1e-3, "{aim}");
        let (_, target, _) = c.first_hit(&g, None, origin, origin + Vec3::Y * 200.0, 0.0).unwrap();
        assert_eq!(target, Some(1), "the round passes the flagged faces");
        // Nearer than 100 m, the aim point is pushed out along the ray.
        c.targets[0] = post(Vec3::new(20.0, 50.0, 0.0), 500.0);
        let aim = c.aim_point(&g, None, origin, Vec3::Y).unwrap();
        assert!((aim - Vec3::new(20.0, 110.0, 1.0)).length() < 1e-3, "{aim}");
    }

    #[test]
    fn a_seeker_turns_its_round_onto_a_target_in_its_cone_at_its_turn_rate() {
        let g = floor();
        let missile = RoundKind {
            name: "bm_h_01".into(),
            top_speed: 70.0,
            range: 350.0,
            radius: 0.4,
            hit_points: 200.0,
            seeker: Some(Seeker { cone: 0.85, reach: 500.0, lock_ms: 4000.0 }),
            turn_rate: [1.4, 1.4, 1.4],
            ..RoundKind::default()
        };
        // A target 30 degrees right of the launch line and one 60 degrees left, beyond the cone.
        let right = post(Vec3::new(10.0 + 150.0 * 0.5, 10.0 + 150.0 * 0.866, 30.0), 5000.0);
        let wide = post(Vec3::new(10.0 - 150.0 * 0.866, 10.0 + 150.0 * 0.5, 30.0), 5000.0);
        let mut c = Combat { kinds: vec![missile], targets: vec![right, wide], ..Default::default() };
        let from = Vec3::new(10.0, 10.0, 30.0);
        c.fire(0, None, from, Vec3::Y, Vec3::ZERO, 1.0, Some(0));
        c.fire(0, None, from, Vec3::Y, Vec3::ZERO, 1.0, Some(1));
        c.fire(0, None, from, Vec3::Y, Vec3::ZERO, 1.0, None);
        let dt = 1.0 / 60.0;
        c.tick(dt, &g);
        let turned = c.rounds[0].forward.angle_between(Vec3::Y);
        assert!((turned - 1.4 * dt).abs() < 1e-3, "one tick turns {turned} rad, at most the rate");
        assert!(c.rounds[0].forward.x > 0.0, "toward the right");
        assert!(
            (c.rounds[0].velocity.normalize() - c.rounds[0].forward).length() < 1e-4,
            "its velocity turns too"
        );
        assert_eq!(c.rounds[1].forward, Vec3::Y, "outside the cone the seeker gives no heading");
        assert_eq!(c.rounds[2].forward, Vec3::Y, "with no target it flies straight");

        // Past the test floor's edge: the steering alone, a second of it.
        let mut r = c.rounds[0];
        let (seeker, turn, at) = (c.kinds[0].seeker.unwrap(), c.kinds[0].turn_rate, c.targets[0].aim);
        for _ in 0..60 {
            steer(&mut r, seeker, turn, at, dt);
            r.position += r.velocity * dt;
        }
        assert!(r.forward.dot((at - r.position).normalize()) > 0.999, "on the target after a second");
    }

    #[test]
    fn a_missile_blasts_at_the_end_of_its_range_and_its_owner_is_never_struck() {
        let g = floor();
        let missile = RoundKind {
            name: "bm_h_01".into(),
            top_speed: 70.0,
            range: 10.0,
            radius: 0.4,
            hit_points: 200.0,
            hit: Some(explosion(HIT_AREA, 170.0, 7.0)),
            range_end: Some(explosion(HIT_AREA, 200.0, 10.0)),
            ..RoundKind::default()
        };
        let mut c = Combat {
            kinds: vec![missile],
            rounds: Vec::new(),
            fired: 0,
            hero: None,
            targets: vec![post(Vec3::new(20.0, 16.0, 0.0), 5000.0), post(Vec3::new(20.0, 12.0, 0.0), 5000.0)],
            absent: Default::default(),
            gone: Default::default(),
        };
        // Fired by the nearer post, from inside it: it passes its owner and flies on.
        c.fire(0, Some(1), Vec3::new(20.0, 11.0, 1.0), Vec3::Y, Vec3::ZERO, 1.0, None);
        let mut events = Vec::new();
        for _ in 0..20 {
            events.extend(c.tick(1.0 / 60.0, &g));
        }
        let struck: Vec<_> = events.iter().filter(|e| matches!(e, Event::Struck { .. })).collect();
        assert_eq!(struck.len(), 1, "{events:?}");
        assert!(matches!(struck[0], Event::Struck { target: Some(0), .. }));
        // The hit blast reaches both posts' spheres: 370 wholly on the first, and nothing on
        // the post that fired it, 4 m off in a blast of 7, which is told of the hit all the same.
        let on = |t: usize| {
            events.iter().filter_map(move |e| match e {
                Event::Damaged { target, damage, .. } if *target == t => Some(*damage),
                _ => None,
            })
        };
        assert_eq!(on(0).collect::<Vec<_>>(), vec![370.0]);
        assert_eq!(on(1).count(), 0, "the firer's nodes take nothing: {events:?}");
        assert!(events.contains(&Event::Hurt { target: 1, owner: Some(1) }));
    }

    /// The firer test is one id against another (`Control.dll:0x1000ed4e`): the object that
    /// fired is spared, and the post beside it, whoever's it is, is not. The shield step comes
    /// before the test, so the firer's own sector pays for its blast.
    #[test]
    fn a_blast_spares_the_nodes_of_the_object_that_fired_it_and_of_no_other_and_its_shield_pays() {
        let g = floor();
        // A round whose range ends 5 m short of the post's centre, in a blast of 10.
        let shell = RoundKind {
            name: "shell".into(),
            top_speed: 1200.0,
            range: 20.0,
            radius: 0.1,
            hit_points: 0.0,
            hit: None,
            range_end: Some(explosion(HIT_AREA, 1000.0, 10.0)),
            ..RoundKind::default()
        };
        let far = post(Vec3::new(200.0, 200.0, 0.0), 500.0);
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        let back = crate::shield::BACK;

        // Fired by the far post: the shielded one's sector stops what it holds and the rest
        // reaches its node.
        let mut c = Combat {
            kinds: vec![shell.clone()],
            targets: vec![shielded_post(), far.clone()],
            ..Default::default()
        };
        let held = c.targets[0].shield.as_ref().unwrap().strength(back);
        c.fire(0, Some(1), muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        let damage = damage_of(&events);
        assert_eq!(damage.len(), 1, "{events:?}");
        assert!((damage[0] - (1000.0 - held)).abs() < 0.05, "{damage:?} past {held}");

        // Fired by the shielded post itself: the sector is spent just the same, and no node
        // of the post is touched.
        let mut c = Combat { kinds: vec![shell], targets: vec![shielded_post(), far], ..Default::default() };
        c.fire(0, Some(0), muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(damage_of(&events).is_empty(), "{events:?}");
        assert!(events.iter().any(|e| matches!(e, Event::ShieldHit { target: 0, .. })));
        assert_eq!(c.targets[0].shield.as_ref().unwrap().fills[back], 0.0, "its own sector paid");
        assert!(events.contains(&Event::Hurt { target: 0, owner: Some(0) }), "and it is told");
        let whole = c.targets[0].parts[0].life.as_ref().unwrap();
        assert_eq!(whole.nodes[0].life, whole.nodes[0].max);
    }

    /// A hit is worth `ratio × .exp damage + lost life` only while the object that fired it
    /// answers its id (`Control.dll:0x10011783`); otherwise 0, and the queue passes a hit of 0
    /// over before it hurts or tells anyone (`0x10012f0b`). The explosion is still seen.
    #[test]
    fn a_round_whose_firer_is_deleted_goes_off_for_nothing() {
        let g = floor();
        let missile = RoundKind {
            name: "bm_h_01".into(),
            top_speed: 70.0,
            range: 100.0,
            radius: 0.4,
            hit_points: 200.0,
            hit: Some(explosion(HIT_AREA, 170.0, 7.0)),
            range_end: Some(explosion(HIT_AREA, 200.0, 10.0)),
            ..RoundKind::default()
        };
        let posts =
            vec![post(Vec3::new(20.0, 16.0, 0.0), 5000.0), post(Vec3::new(200.0, 200.0, 0.0), 5000.0)];
        let run = |gone: bool| {
            let mut c = Combat { kinds: vec![missile.clone()], targets: posts.clone(), ..Default::default() };
            c.fire(0, Some(1), Vec3::new(20.0, 11.0, 1.0), Vec3::Y, Vec3::ZERO, 1.0, None);
            if gone {
                // Dead and, its time up, deleted, with its round in the air.
                c.targets[1].alive = false;
                c.gone.insert(1);
            }
            let mut events = Vec::new();
            for _ in 0..20 {
                events.extend(c.tick(1.0 / 60.0, &g));
            }
            events
        };
        let live = run(false);
        assert_eq!(damage_of(&live), vec![370.0]);
        assert!(live.contains(&Event::Hurt { target: 0, owner: Some(1) }));

        let dud = run(true);
        assert!(dud.iter().any(|e| matches!(e, Event::Struck { target: Some(0), .. })), "{dud:?}");
        assert!(dud.iter().any(|e| matches!(e, Event::Exploded { at_range: false, .. })));
        assert!(damage_of(&dud).is_empty(), "{dud:?}");
        assert!(!dud.iter().any(|e| matches!(e, Event::Hurt { .. })), "nobody is told");
    }

    #[test]
    fn a_round_that_runs_out_ends_where_its_range_does() {
        let g = floor();
        let short = RoundKind { range: 40.0, ..laser() };
        let mut c = Combat { kinds: vec![short], ..Default::default() };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        let events = c.tick(1.0 / 60.0, &g);
        let ended: Vec<_> = events
            .iter()
            .filter_map(
                |e| if let Event::Ended { round, end } = e { Some((round.position, *end)) } else { None },
            )
            .collect();
        assert_eq!(ended.len(), 1, "{events:?}");
        assert_eq!(ended[0].1, RoundEnd::Range);
        assert!((ended[0].0 - (muzzle + Vec3::Y * 40.0)).length() < 1e-3, "{:?}", ended[0].0);
    }

    /// An effect's view test is the sight ray's query, so the water sheet hides a point
    /// under it where a round's query goes through to the bed (docs/11, "How often the
    /// point is tested, and what the ray meets").
    #[test]
    fn an_effects_view_test_is_stopped_by_water_where_a_rounds_query_is_not() {
        let g = floor();
        let c = Combat::default();
        // The floor's first half carries a sheet at z 5; look down at a point under it.
        let eye = Vec3::new(10.0, 10.0, 30.0);
        let under = Vec3::new(10.0, 10.0, 2.0);
        assert!(!c.clear_line(&g, eye, under), "the sheet hides it");
        assert!(c.first_hit(&g, None, eye, under, 0.0).is_none(), "a round's query passes the same sheet");
        // Over the sheet, nothing is in the way either way.
        let over = Vec3::new(10.0, 10.0, 8.0);
        assert!(c.clear_line(&g, eye, over));
        assert!(c.first_hit(&g, None, eye, over, 0.0).is_none());
    }

    /// The fight module's line is the world's query over the landscape, buildings and scenery,
    /// the firing unit and its target left out, and then a sweep past the firer's own units
    /// (docs/29, "The line every gun waits on").
    #[test]
    fn an_ai_units_line_is_blocked_by_ground_buildings_and_its_own_units_and_by_no_other_unit() {
        let g = floor();
        // The floor's dry half, a post standing across the line 18 m along it.
        let c = Combat { targets: vec![post(Vec3::new(10.0, 30.0, 0.0), 500.0)], ..Default::default() };
        let (a, b) = (Vec3::new(10.0, 12.0, 1.0), Vec3::new(10.0, 38.0, 1.0));
        let (nothing, no_one) = (|_: usize| false, [0usize; 0]);
        assert!(!c.fire_line_clear(&g, a, b, |_| true, no_one), "a building or a tree stops it");
        assert!(c.fire_line_clear(&g, a, b, nothing, no_one), "a unit does not: class 4 is not asked");

        // One of the firer's own units, by its agent sphere of 1.5 about (10, 30, 1) and the
        // line's own half metre: on the line, 1.9 m beside it, and 2.1 m beside it.
        assert!(!c.fire_line_clear(&g, a, b, nothing, [0]));
        let beside = |x: f32| (Vec3::new(10.0 + x, 12.0, 1.0), Vec3::new(10.0 + x, 38.0, 1.0));
        let (a1, b1) = beside(1.9);
        assert!(!c.fire_line_clear(&g, a1, b1, nothing, [0]));
        let (a2, b2) = beside(2.1);
        assert!(c.fire_line_clear(&g, a2, b2, nothing, [0]));
        // Past the line's end or behind its start it is not in the way; about the start it is.
        assert!(c.fire_line_clear(&g, a, Vec3::new(10.0, 27.0, 1.0), nothing, [0]), "short of it");
        assert!(c.fire_line_clear(&g, Vec3::new(10.0, 33.0, 1.0), b, nothing, [0]), "behind the start");
        assert!(!c.fire_line_clear(&g, Vec3::new(10.0, 31.0, 1.0), b, nothing, [0]), "the start inside it");

        // The landscape, and a lake's sheet as the sight meets it.
        assert!(!c.fire_line_clear(&g, a, Vec3::new(10.0, 38.0, -1.0), nothing, no_one), "into the floor");
        let (over, under) = (Vec3::new(30.0, 10.0, 8.0), Vec3::new(30.0, 12.0, 2.0));
        assert!(!c.fire_line_clear(&g, over, under, nothing, no_one), "through the water's sheet");
    }

    /// A face flagged `0x20` does not block a line under 20 m long (`0x100247aa`–`0x100247eb`).
    #[test]
    fn a_line_under_twenty_metres_passes_a_face_flagged_0x20_and_a_longer_one_does_not() {
        let g = floor();
        let leaves = |flags: u16| {
            let mut t = post(Vec3::new(10.0, 30.0, 0.0), 500.0);
            let mut mesh = (*t.parts[0].mesh).clone();
            mesh.face_flags = vec![flags; 2];
            t.parts[0].mesh = Rc::new(mesh);
            Combat { targets: vec![t], ..Default::default() }
        };
        let far = (Vec3::new(10.0, 12.0, 1.0), Vec3::new(10.0, 38.0, 1.0));
        let near = (Vec3::new(10.0, 25.0, 1.0), Vec3::new(10.0, 35.0, 1.0));
        let no_one = [0usize; 0];
        let c = leaves(LINE_PASSES_FACE);
        assert!(c.fire_line_clear(&g, near.0, near.1, |_| true, no_one), "10 m through a flagged face");
        assert!(!c.fire_line_clear(&g, far.0, far.1, |_| true, no_one), "26 m through the same face");
        let c = leaves(0x4);
        assert!(
            !c.fire_line_clear(&g, near.0, near.1, |_| true, no_one),
            "another flag blocks, however near"
        );
    }

    /// The part an AI unit aims at (`Behavior.dll:0x10025830`): the heaviest live device or gear
    /// node, a damaged one outweighing a whole one of its class, the last listed among equals.
    #[test]
    fn the_part_aimed_at_is_the_heaviest_with_life_and_a_damaged_one_weighs_more() {
        // A body (node 1 a wheel), a turret on it (node 1 its body, node 2 its deflector) and a
        // gun on the turret.
        let mut unit = turret_unit();
        unit.gear = vec![(0, 1)];
        unit.devices = vec![
            Device { class: 5, node: (0, 0) },
            Device { class: 1, node: (1, 1) },
            Device { class: 21, node: (1, 2) },
            Device { class: 2, node: (2, 1) },
        ];
        let lose = |unit: &mut Target, (p, n): (usize, usize), damage: f32| {
            unit.parts[p].life.as_mut().unwrap().lose(n, damage);
        };
        assert_eq!(unit.aim_part(), Some((1, 2)), "the deflector, 30 against the turret's 20");
        lose(&mut unit, (1, 2), 1.0);
        assert_eq!(unit.aim_part(), Some((1, 1)), "a part with no life left is passed over: the turret");
        // The gun at 60% weighs 15 × 1.4 = 21, over the whole turret's 20.
        lose(&mut unit, (2, 1), 60.0);
        assert_eq!(unit.aim_part(), Some((2, 1)));
        // The turret at half weighs 30.
        lose(&mut unit, (1, 1), 135.0);
        assert_eq!(unit.aim_part(), Some((1, 1)));

        // Among equals the last listed keeps the pick, and gear weighs 7.5 under an engine's 14.
        let mut unit = turret_unit();
        unit.devices = vec![Device { class: 2, node: (2, 0) }, Device { class: 2, node: (2, 1) }];
        assert_eq!(unit.aim_part(), Some((2, 1)));
        unit.devices.reverse();
        assert_eq!(unit.aim_part(), Some((2, 0)));
        unit.devices = vec![Device { class: 5, node: (0, 0) }];
        unit.gear = vec![(0, 1)];
        assert_eq!(unit.aim_part(), Some((0, 0)));
        unit.devices.clear();
        assert_eq!(unit.aim_part(), Some((0, 1)), "the wheel, with no device listed");
        unit.gear.clear();
        assert_eq!(unit.aim_part(), None, "nothing to pick: the callers take node 0");
        assert_eq!(
            [21, 1, 2, 5, 8, 19, 9, 15, 3].map(device_weight),
            [30., 20., 15., 14., 14., 10., 7., 3., 1.]
        );
    }

    /// A node's own sphere is its level-0 slot's through its matrix, and a seeker handed a part
    /// steers at that sphere's centre, not at the object's node sphere.
    #[test]
    fn a_seeker_handed_a_part_steers_at_that_nodes_own_sphere() {
        let g = floor();
        let missile = RoundKind {
            name: "bm_m_04".into(),
            top_speed: 45.0,
            range: 700.0,
            radius: 0.4,
            hit_points: 200.0,
            seeker: Some(Seeker { cone: 0.7, reach: 500.0, lock_ms: 7000.0 }),
            turn_rate: [0.5, 0.5, 0.5],
            ..RoundKind::default()
        };
        // A post dead ahead whose node sphere's centre is put well to the left of it.
        let mut ahead = post(Vec3::new(10.0, 110.0, 29.0), 5000.0);
        assert_eq!(ahead.slot_sphere((0, 0)), Some((Vec3::new(10.0, 110.0, 30.0), 1.5)));
        ahead.aim = Vec3::new(-40.0, 110.0, 30.0);
        let mut c = Combat { kinds: vec![missile], targets: vec![ahead], ..Default::default() };
        let from = Vec3::new(10.0, 10.0, 30.0);
        c.fire(0, None, from, Vec3::Y, Vec3::ZERO, 1.0, Some(0));
        c.fire(0, None, from, Vec3::Y, Vec3::ZERO, 1.0, Some(0));
        c.rounds[1].part = Some((0, 0));
        c.tick(1.0 / 60.0, &g);
        assert!(c.rounds[0].forward.x < -1e-3, "with no part it turns toward the node sphere");
        assert!(c.rounds[1].forward.x.abs() < 1e-5, "with the part it flies on at the node's own");
        // Its part's life gone it has node 0 to fall back on, and with node 0's gone nothing.
        assert_eq!(c.targets[0].seeker_point((0, 0)), Some(Vec3::new(10.0, 110.0, 30.0)));
        c.targets[0].parts[0].life.as_mut().unwrap().lose(0, 5000.0);
        assert_eq!(c.targets[0].seeker_point((0, 0)), None);
    }

    #[test]
    fn a_rounds_sideways_speed_bleeds_off_and_its_forward_speed_stays() {
        let g = floor();
        let mut c = Combat { kinds: vec![laser()], ..Default::default() };
        c.kinds[0].top_speed = 10.0;
        c.fire(0, None, Vec3::new(10.0, 10.0, 30.0), Vec3::Y, Vec3::new(2.0, 0.0, 0.0), 1.0, None);
        c.tick(0.5, &g);
        let v = c.rounds[0].velocity;
        assert!((v.x - 0.5).abs() < 1e-4 && (v.y - 10.0).abs() < 1e-4, "{v}");
    }
}
