//! Rounds in flight and what they strike: `docs/29-weapons.md`, "The round's start"
//! and "How a round ends", and `docs/26-damage.md`.
//!
//! A frame moves every round and spends its range, then runs the collision pass
//! once, then fires the range groups, then delivers the contacts
//! (`World3D.dll:0x10006bf0`).

use std::rc::Rc;

use glam::Vec3;
use parkan_formats::exp::{Explosion, HIT_AREA, HIT_DIRECT};
use parkan_formats::mesh::Mesh;
use parkan_formats::pose::Pose;

use crate::damage::{Life, blast, round_hit};
use crate::ground::Ground;
use crate::hit::{Strike, map_edge, segment_mesh, swept_spheres};

/// A mode-0 round's sideways speed, in its own frame, bleeds off at this many m/s a
/// millisecond (`Control.dll:0x1000ceec`).
pub const SIDEWAYS_BLEED: f32 = 0.003;
/// The sight ray runs from 5 m out (`0x1002a768`) and its aim point is pushed out
/// to at least 100 m (`0x1002a7be`).
pub const SIGHT_FROM: f32 = 5.0;
pub const SIGHT_TO: f32 = 1_000_000.0;
pub const NEAREST_AIM: f32 = 100.0;

/// What every round of one record shares.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundKind {
    pub name: String,
    /// The controller's top speed along y.
    pub top_speed: f32,
    /// File +108.
    pub range: f32,
    /// STAND-IN: docs/26-damage.md#the-hit-test--read-and-measured -- that the collision
    /// radius is the mesh's stream-2 sphere is a guess.
    pub radius: f32,
    /// Node 0's hit points: a round dies from full health.
    pub hit_points: f32,
    /// Node 0's `.exp`, which goes off where the round is killed.
    pub hit: Option<Explosion>,
    /// The `.exp` block entry 4's action 27 names, at the end of the range.
    pub range_end: Option<Explosion>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Round {
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
    expired: bool,
}

/// One mesh of a target, posed.
#[derive(Clone, Debug)]
pub struct Part {
    pub mesh: Rc<Mesh>,
    /// Each node's world pose.
    pub nodes: Vec<Pose>,
    /// Its nodes' hit points; `None` where it takes no damage.
    pub life: Option<Life>,
}

/// Something a round can strike.
#[derive(Clone, Debug)]
pub struct Target {
    pub parts: Vec<Part>,
    /// The bounding sphere the pass and the blasts test first.
    pub centre: Vec3,
    pub radius: f32,
    pub alive: bool,
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
    Exploded { kind: usize, point: Vec3, at_range: bool },
    /// Damage landed on a node.
    Damaged { target: usize, part: usize, node: usize, damage: f32, destroyed: Vec<usize> },
    /// A target died.
    Killed { target: usize },
    /// A round left the map, with no explosion.
    Gone { round: Round },
}

#[derive(Clone, Debug, Default)]
pub struct Combat {
    pub kinds: Vec<RoundKind>,
    pub rounds: Vec<Round>,
    pub targets: Vec<Target>,
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
    /// speed plus the shooter's world velocity, in free flight.
    pub fn fire(
        &mut self,
        kind: usize,
        owner: Option<usize>,
        muzzle: Vec3,
        direction: Vec3,
        shooter_velocity: Vec3,
        ratio: f32,
    ) {
        let Some(k) = self.kinds.get(kind) else { return };
        let forward = direction.normalize_or(Vec3::Y);
        self.rounds.push(Round {
            kind,
            owner,
            previous: muzzle,
            position: muzzle,
            velocity: forward * k.top_speed + shooter_velocity,
            forward,
            remaining: k.range,
            ratio,
            expired: false,
        });
    }

    /// The nearest thing a segment meets: the ground, or a live target other than
    /// `skip`. Returns the strike and the target struck.
    pub fn first_hit(
        &self,
        ground: &Ground,
        skip: Option<usize>,
        p0: Vec3,
        p1: Vec3,
        radius: f32,
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
                if let Some(s) = segment_mesh(&part.mesh, &part.nodes, p0, p1)
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
        let (strike, _, _) = self.first_hit(ground, owner, p0, p1, 0.0)?;
        let mut point = strike.point;
        if (point - origin).length() < NEAREST_AIM {
            point = origin + s * NEAREST_AIM;
        }
        Some(point)
    }

    /// One frame of `dt` seconds.
    pub fn tick(&mut self, dt: f32, ground: &Ground) -> Vec<Event> {
        let mut events = Vec::new();
        // Message 1: every round moves and spends its range (`0x1000cbb0`).
        for r in &mut self.rounds {
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
                events.push(Event::Exploded { kind: r.kind, point: r.position, at_range: true });
                if let Some(e) = &k.range_end {
                    self.explode(e, &k, &r, r.position, None, &mut events);
                }
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
                    events.push(Event::Exploded { kind: r.kind, point: strike.point, at_range: false });
                    if let Some(e) = &k.hit {
                        let direct = target.zip(strike.node).map(|(t, n)| (t, part, n));
                        self.explode(e, &k, &r, strike.point, direct, &mut events);
                    }
                }
                (_, Some((point, _))) => {
                    let mut gone = r;
                    gone.position = point;
                    events.push(Event::Gone { round: gone });
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
                            .filter_map(|(n, node)| {
                                let slot = part.mesh.slots.get(usize::from(node.slot_index[0]))?;
                                let [cx, cy, cz, r] = slot.sphere;
                                let centre = vec(part.nodes.get(n)?.apply(arr(Vec3::new(cx, cy, cz))));
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

    fn damage(&mut self, t: usize, p: usize, n: usize, damage: f32, events: &mut Vec<Event>) {
        let target = &mut self.targets[t];
        let Some(life) = target.parts.get_mut(p).and_then(|part| part.life.as_mut()) else { return };
        let was_dead = life.dead;
        let destroyed = life.hit(n, damage);
        let now_dead = life.dead;
        events.push(Event::Damaged { target: t, part: p, node: n, damage, destroyed });
        if now_dead && !was_dead && p == 0 {
            target.alive = false;
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
            parts: vec![Part { mesh: Rc::new(mesh), nodes: vec![pose], life: Some(life) }],
            centre: at + Vec3::Z,
            radius: 1.5,
            alive: true,
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
        }
    }

    #[test]
    fn two_laser_hits_kill_a_five_hundred_point_post_and_nothing_tunnels() {
        let g = floor();
        let mut c = Combat {
            kinds: vec![laser()],
            rounds: Vec::new(),
            targets: vec![post(Vec3::new(20.0, 30.0, 0.0), 500.0)],
        };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        for shot in 0..2 {
            c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0);
            let events = c.tick(1.0 / 60.0, &g);
            assert!(c.rounds.is_empty(), "the round ended in its first frame");
            let damage: Vec<f32> = events
                .iter()
                .filter_map(|e| if let Event::Damaged { damage, .. } = e { Some(*damage) } else { None })
                .collect();
            assert_eq!(damage, vec![250.0]);
            assert_eq!(events.iter().any(|e| matches!(e, Event::Killed { target: 0 })), shot == 1);
        }
        let life = c.targets[0].parts[0].life.as_ref().unwrap();
        assert!(life.dead && !c.targets[0].alive);
        // Dead, it no longer stops a round: this one runs to the map's edge.
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0);
        let events = c.tick(1.0 / 60.0, &g);
        assert!(events.iter().any(|e| matches!(e, Event::Gone { .. })), "{events:?}");
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
        };
        let mut c = Combat {
            kinds: vec![missile],
            rounds: Vec::new(),
            targets: vec![post(Vec3::new(20.0, 16.0, 0.0), 5000.0), post(Vec3::new(20.0, 12.0, 0.0), 5000.0)],
        };
        // Fired by the nearer post, from inside it: it passes its owner and flies on.
        c.fire(0, Some(1), Vec3::new(20.0, 11.0, 1.0), Vec3::Y, Vec3::ZERO, 1.0);
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
    fn a_rounds_sideways_speed_bleeds_off_and_its_forward_speed_stays() {
        let g = floor();
        let mut c = Combat { kinds: vec![laser()], ..Default::default() };
        c.kinds[0].top_speed = 10.0;
        c.fire(0, None, Vec3::new(10.0, 10.0, 30.0), Vec3::Y, Vec3::new(2.0, 0.0, 0.0), 1.0);
        c.tick(0.5, &g);
        let v = c.rounds[0].velocity;
        assert!((v.x - 0.5).abs() < 1e-4 && (v.y - 10.0).abs() < 1e-4, "{v}");
    }
}
