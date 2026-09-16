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
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
    /// Unique among the rounds a `Combat` has fired.
    pub id: u64,
    pub kind: usize,
    /// The target id of the robot that fired it, which it never strikes.
    pub owner: Option<usize>,
    pub previous: Vec3,
    pub position: Vec3,
    /// World velocity.
    pub velocity: Vec3,
    /// The round's own y, which it was created facing.
    pub forward: Vec3,
    pub remaining: f32,
    pub ratio: f32,
    /// The target its gun handed it (`0x1002a514`), which its seeker follows.
    pub target: Option<usize>,
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
    /// The portal quads, by triangle: a segment passes them whatever their flags, as a
    /// mover does (docs/24, "Portal quads stand between the rooms"). Empty on a mesh that
    /// wears none, which is every mesh but a building's.
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

/// Something a round can strike.
#[derive(Clone, Debug)]
pub struct Target {
    pub parts: Vec<Part>,
    /// The bounding sphere the pass and the blasts test first.
    pub centre: Vec3,
    pub radius: f32,
    pub alive: bool,
    /// The object's placement: where a gun's gate and a seeker find it.
    pub position: Vec3,
}

impl Target {
    pub fn dead(&self) -> bool {
        self.parts.first().and_then(|p| p.life.as_ref()).is_some_and(|l| l.dead)
    }
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
    /// A target died.
    Killed { target: usize },
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
    pub fired: u64,
}

/// A unit dead (`0x10011098`): out of the fight, and every part goes with node 0, which
/// takes the unit's whole life so the walk takes every node below it (`0x10011105`).
///
/// STAND-IN: docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured
/// -- a unit's life system holds the nodes of all its models under one root; the engine
/// keeps a life per part, so a dead unit's other parts' roots are destroyed with it, and a
/// part's root has no parent to be knocked off from.
fn kill(target: &mut Target) {
    target.alive = false;
    for part in target.parts.iter_mut().skip(1) {
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
            target,
            expired: false,
        });
        Some(self.fired)
    }

    /// The nearest thing a round's segment meets: the ground, or a live target other
    /// than `skip`, passing the triangles a round passes. Returns the strike and the
    /// target struck.
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
        let mut best: Option<(Strike, Option<usize>, usize)> = ground.segment(p0, p1).map(|s| (s, None, 0));
        for (id, target) in self.targets.iter().enumerate() {
            if !target.alive || Some(id) == skip {
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

    /// The sight's aim point (`0x1002a610`): the first thing the ray from `origin` along
    /// `direction` meets, pushed out to at least 100 m; `None` when it meets nothing.
    /// The ray asks for every class and passes no triangle (`0x1002adc0`), so it can stop
    /// on leaves a round flies through.
    ///
    /// STAND-IN: docs/29-weapons.md#not-established -- whether the landscape is one of
    /// the objects the ray walks is not traced; the ground is met, less its water
    /// surface, as a round meets it.
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
        let (strike, _, _) = self.nearest(ground, owner, p0, p1, 0.0, SIGHT_SKIPS_FACE)?;
        let mut point = strike.point;
        if (point - origin).length() < NEAREST_AIM {
            point = origin + s * NEAREST_AIM;
        }
        Some(point)
    }

    /// One frame of `dt` seconds.
    pub fn tick(&mut self, dt: f32, ground: &Ground) -> Vec<Event> {
        let mut events = Vec::new();
        // Message 1: every round moves and spends its range (`0x1000cbb0`), a guided one
        // turning toward its target first (`0x1000ccc5`).
        for r in &mut self.rounds {
            let k = &self.kinds[r.kind];
            if let Some(seeker) = k.seeker
                && let Some(target) = r.target.and_then(|t| self.targets.get(t)).filter(|t| t.alive)
            {
                steer(r, seeker, k.turn_rate, target.position, dt);
            }
            let side = r.forward.cross(Vec3::Z).normalize_or(Vec3::X);
            let up = side.cross(r.forward).normalize_or(Vec3::Z);
            let bleed = SIDEWAYS_BLEED * dt * 1000.0;
            let toward_zero = |v: f32| if v.abs() <= bleed { 0.0 } else { v - bleed.copysign(v) };
            let (along, x, z) = (r.velocity.dot(r.forward), r.velocity.dot(side), r.velocity.dot(up));
            r.velocity = r.forward * along + side * toward_zero(x) + up * toward_zero(z);
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
                    self.explode(e, &k, &r, r.position, None, &mut events);
                }
                events.push(Event::Ended { round: r, end: RoundEnd::Range });
                continue;
            }
            let hit = self.first_hit(ground, r.owner, r.previous, r.position, k.radius);
            let edge = map_edge(box_lo, box_hi, r.previous, r.position);
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
                        let direct = target.zip(strike.node).map(|(t, n)| (t, part, n));
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

    /// A round's `.exp` going off (`0x1000ebc0`): kind 2 on the node struck, kind 3 a
    /// blast over every node in reach, each for `ratio × (hit points + damage)`.
    fn explode(
        &mut self,
        e: &Explosion,
        kind: &RoundKind,
        round: &Round,
        point: Vec3,
        direct: Option<(usize, usize, usize)>,
        events: &mut Vec<Event>,
    ) {
        let damage = round_hit(round.ratio, kind.hit_points, e.damage);
        match e.kind {
            HIT_DIRECT => {
                if let Some((t, p, n)) = direct {
                    self.damage(t, p, n, damage, events);
                }
            }
            HIT_AREA => {
                for t in 0..self.targets.len() {
                    let target = &self.targets[t];
                    if !target.alive || (target.centre - point).length() >= target.radius + e.radius {
                        continue;
                    }
                    for p in 0..target.parts.len() {
                        let part = &self.targets[t].parts[p];
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
            // STAND-IN: docs/26-damage.md#shields-a-generator-a-deflector-six-sectors--read-and-measured
            // -- shields are not modelled: no bubble stops a round, a blast skips its
            // shield step, and kind 4, shields only, does nothing. Mission 01's `tut1_e1`,
            // `tut1_mf1` and `helic` carry fight shields and deflectors.
            HIT_SHIELDS => {}
            _ => {}
        }
    }

    /// What a unit's ground deals it in a life update (docs/24, "Water and lava beds kill"):
    /// `loss` shared over the nodes of every part that takes damage, as a hit's events.
    pub fn ground_loss(&mut self, t: usize, loss: f32) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(target) = self.targets.get_mut(t).filter(|t| t.alive) else { return events };
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

    /// Every target's lives after the tick's hits, at `now_ms` ([`Life::takt`]): the stages
    /// that rose, the nodes hidden and the parts knocked off, as events.
    pub fn takt_lives(&mut self, now_ms: f64) -> Vec<Event> {
        let mut events = Vec::new();
        for (t, target) in self.targets.iter_mut().enumerate() {
            // A death by any other way than a round, the ground's loss say.
            if target.alive && target.dead() {
                kill(target);
                events.push(Event::Killed { target: t });
            }
            for (p, part) in target.parts.iter_mut().enumerate() {
                let Some(life) = part.life.as_mut() else { continue };
                for change in life.takt(now_ms) {
                    events.push(match change {
                        Change::Staged(node) => Event::Staged { target: t, part: p, node },
                        Change::Hidden(node) => Event::Hidden { target: t, part: p, node },
                        Change::KnockedOff(node) => Event::KnockedOff { target: t, part: p, node },
                    });
                }
            }
        }
        events
    }

    fn damage(&mut self, t: usize, p: usize, n: usize, damage: f32, events: &mut Vec<Event>) {
        let target = &mut self.targets[t];
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
            }],
            centre: at + Vec3::Z,
            radius: 1.5,
            alive: true,
            position: at,
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

    #[test]
    fn two_laser_hits_kill_a_five_hundred_point_post_and_nothing_tunnels() {
        let g = floor();
        let mut c = Combat {
            kinds: vec![laser()],
            rounds: Vec::new(),
            fired: 0,
            targets: vec![post(Vec3::new(20.0, 30.0, 0.0), 500.0)],
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
        let (seeker, turn, at) = (c.kinds[0].seeker.unwrap(), c.kinds[0].turn_rate, c.targets[0].position);
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
            targets: vec![post(Vec3::new(20.0, 16.0, 0.0), 5000.0), post(Vec3::new(20.0, 12.0, 0.0), 5000.0)],
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
        // The hit blast reaches both posts' spheres: 370 wholly on the first.
        let on = |t: usize| {
            events.iter().filter_map(move |e| match e {
                Event::Damaged { target, damage, .. } if *target == t => Some(*damage),
                _ => None,
            })
        };
        assert_eq!(on(0).collect::<Vec<_>>(), vec![370.0]);
        assert_eq!(on(1).count(), 1);
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
