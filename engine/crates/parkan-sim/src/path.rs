//! The walker's global path: the areals and hall-way vertices from a unit to its goal, found
//! by A* over the links the areal map builds, and the points the unit walks through them, round
//! the trees and stones cut out of the areals. See `docs/24-motion.md`, "The global path" and
//! "A tree or a stone cuts the areals it stands on".

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use glam::{DVec2, Vec2, Vec3};
use parkan_formats::arealmap::ArealMap;

/// A link from a walkable areal to the next across an edge costs the distance between their
/// centres and this (`ArealMap.dll:0x10023430`).
pub const AREAL_STEP: f32 = 1.0;
/// A hall-way exit and the walkable areal under it are joined at this (`0x1002363f`).
pub const EXIT_STEP: f32 = 1.0;
/// A link along a hall way costs three times its length and five, or ten from or to a vertex
/// with [`VERTEX_FLAT`] (`0x1000a1dd`).
pub const HALL_FACTOR: f32 = 3.0;
pub const HALL_STEP: f32 = 5.0;
pub const HALL_FLAT: f32 = 10.0;
/// Two buildings' vertices with [`VERTEX_JOIN`] closer than this are joined at
/// [`JOIN_STEP`] (`0x1000b663`, `0x1000b6bc`).
pub const JOIN_REACH: f32 = 50.0;
pub const JOIN_STEP: f32 = 0.1;
/// A hall-way vertex's flags: an exit onto the ground, one whose links cost [`HALL_FLAT`], and
/// one that joins another building's.
pub const VERTEX_EXIT: u32 = 0x1;
pub const VERTEX_FLAT: u32 = 0x2;
pub const VERTEX_JOIN: u32 = 0x4;
/// The size gate (`Behavior.dll:0x10042d08`, docs/24, "The global path"): a vertex flagged
/// [`VERTEX_ANY_SIZE`] passes any unit, one flagged [`VERTEX_BUILDING_SIZE`] a unit no bigger
/// than the building's own size class, and one with neither only a unit of [`SMALL_SIZE`] or
/// less. *Measured*: 165 of the 1056 shipped vertices carry the first, 7 the second — all on
/// the three factories — and the other 884 neither, the 21 control pods among them.
pub const VERTEX_ANY_SIZE: u32 = 0x1000_0000;
pub const VERTEX_BUILDING_SIZE: u32 = 0x2000_0000;
pub const SMALL_SIZE: u8 = 2;
/// Each link's cost, added to the cost so far, is scaled by 1 and up to this share at random
/// (graph `+0x38`, `Behavior.dll:0x1003642b`, `0x10042e0b`).
pub const RANDOM_SHARE: f64 = 0.7;
/// The search takes no more nodes than this off its open list (`0x100207df`).
pub const MAX_NODES: usize = 2048;
/// A waypoint on an edge the straight line to the goal misses stands this far along the edge
/// from one of its ends (`Behavior.dll:0x10036cc5`).
pub const EDGE_INSET: f32 = 3.0;
/// On a piece of a broken areal the end is moved a fifth of the edge's length instead
/// (`Behavior.dll:0x100377d6`).
pub const PIECE_EDGE_INSET: f32 = 0.2;
/// A walker on an areal no link leaves (`0x1003e2dc`) tries this many random points in a square
/// about it, of half-width 30, then 33, and so on while under 500, and goes to the first that
/// lies on a walkable areal (`0x1003e2f4`–`0x1003e419`).
pub const ESCAPE_TRIES: usize = 50;
pub const ESCAPE_REACH: f32 = 30.0;
pub const ESCAPE_GROWTH: f32 = 3.0;
pub const ESCAPE_LIMIT: f32 = 500.0;

/// A triangle of an areal: its corners counter-clockwise, and across each edge (corner k to
/// k + 1) the triangle there.
#[derive(Clone, Debug, PartialEq)]
struct Triangle {
    corners: [Vec3; 3],
    /// Each corner's vertex, one for every place areals meet.
    vertices: [usize; 3],
    areal: usize,
    across: [Option<usize>; 3],
}

impl Triangle {
    fn flat(&self, k: usize) -> Vec2 {
        self.corners[k % 3].truncate()
    }

    /// Whether `p` lies inside or on the triangle, within `slack`.
    fn holds(&self, p: Vec2, slack: f32) -> bool {
        (0..3).all(|k| (self.flat(k + 1) - self.flat(k)).perp_dot(p - self.flat(k)) >= -slack)
    }

    /// The nearest point of the triangle to `p`, across the ground.
    fn nearest(&self, p: Vec2) -> Vec2 {
        if self.holds(p, 0.0) {
            return p;
        }
        (0..3)
            .map(|k| {
                let (a, b) = (self.flat(k), self.flat(k + 1));
                let t = ((p - a).dot(b - a) / (b - a).length_squared().max(1e-9)).clamp(0.0, 1.0);
                a + (b - a) * t
            })
            .min_by(|a, b| a.distance_squared(p).total_cmp(&b.distance_squared(p)))
            .expect("three edges")
    }

    fn centroid(&self) -> Vec2 {
        (self.flat(0) + self.flat(1) + self.flat(2)) / 3.0
    }

    /// The height of the triangle's plane at `p`, or its corners' mean where it has none.
    fn height(&self, p: Vec2) -> f32 {
        let [a, b, c] = self.corners;
        let n = (b - a).cross(c - a);
        if n.z.abs() < 1e-6 {
            return (a.z + b.z + c.z) / 3.0;
        }
        a.z - (n.x * (p.x - a.x) + n.y * (p.y - a.y)) / n.z
    }

    /// Edge `k` as a portal walked out through: its left end, then its right.
    fn portal(&self, k: usize) -> (Vec2, Vec2) {
        (self.flat(k + 1), self.flat(k))
    }
}

/// A polygon's triangles, as corner indices counter-clockwise. An ear is clipped where its
/// diagonal is shortest, and a polygon too degenerate to have one left is fanned.
fn triangulate(polygon: &[DVec2]) -> Vec<[usize; 3]> {
    let cross = |a: DVec2, b: DVec2, c: DVec2| (b - a).perp_dot(c - a);
    let mut left: Vec<usize> = (0..polygon.len()).collect();
    let mut out = Vec::with_capacity(polygon.len().saturating_sub(2));
    while left.len() > 3 {
        let m = left.len();
        let corner = |k: usize| (left[(k + m - 1) % m], left[k], left[(k + 1) % m]);
        let ear = (0..m)
            .filter(|&k| {
                let (i, j, l) = corner(k);
                let (a, b, c) = (polygon[i], polygon[j], polygon[l]);
                let scale = (b - a).length() * (c - b).length();
                cross(a, b, c) > 1e-9 * scale
                    && !left.iter().any(|&o| {
                        o != i
                            && o != j
                            && o != l
                            && cross(a, b, polygon[o]) >= 0.0
                            && cross(b, c, polygon[o]) >= 0.0
                            && cross(c, a, polygon[o]) >= 0.0
                    })
            })
            .min_by(|&x, &y| {
                let span = |k: usize| {
                    let (i, _, l) = corner(k);
                    polygon[i].distance_squared(polygon[l])
                };
                span(x).total_cmp(&span(y))
            });
        let Some(k) = ear else {
            out.extend((1..m - 1).map(|k| [left[0], left[k], left[k + 1]]));
            return out;
        };
        let (i, j, l) = corner(k);
        out.push([i, j, l]);
        left.remove(k);
    }
    if left.len() == 3 {
        out.push([left[0], left[1], left[2]]);
    }
    out
}

/// A building's hall way in the world, as the search links it: each vertex's point and flag
/// word, and the links between vertices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Way {
    pub points: Vec<Vec3>,
    pub flags: Vec<u32>,
    pub links: Vec<(usize, usize)>,
    /// The building's own size class, its property `0x201`: what a [`VERTEX_BUILDING_SIZE`]
    /// vertex measures the unit against.
    pub size: u8,
}

/// Why the walker has no path to give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The goal lies on no areal, or on one no walker may be sent to: `MWalker::SetTarget`
    /// refuses it (`Behavior.dll:0x1003bd5d`).
    Goal,
    /// The unit stands on an areal no link leaves.
    Stranded,
    /// The search ran out of links, or of nodes, short of the goal.
    NoWay,
}

/// A piece of an areal: its triangles that still hang together once the scenery's footprints
/// are cut out, as the game's sub-areals do (`MBrokenAreal::Divide`, `ArealMap.dll:0x10010b10`).
#[derive(Clone, Debug, PartialEq)]
struct Piece {
    areal: usize,
    triangles: Vec<usize>,
    /// Where the search measures it from: the areal's record centre while the areal is one
    /// piece, else its triangles' centre.
    centre: Vec2,
    /// The pieces across its areal's edges whose areals are walkable.
    links: Vec<usize>,
}

/// The areal map as the walker's search links it, cut into triangles for a walk through an
/// areal that is not convex, with the scenery's footprints cut out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    map: ArealMap,
    bounds: ([f32; 2], [f32; 2]),
    triangles: Vec<Triangle>,
    /// Each areal's triangles.
    of_areal: Vec<Vec<usize>>,
    /// Whether scenery was cut out of an areal.
    broken: Vec<bool>,
    pieces: Vec<Piece>,
    /// Each triangle's piece.
    piece_of: Vec<usize>,
    /// Each vertex's triangles, and whether it lies on the outside of the map or of a hole.
    fans: Vec<Vec<usize>>,
    outside: Vec<bool>,
}

/// A node of the search: a piece of an areal, or a way's vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Node {
    Piece(usize),
    Vertex(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Open {
    estimate: f64,
    node: Node,
}

impl Eq for Open {}

impl Ord for Open {
    fn cmp(&self, other: &Self) -> Ordering {
        other.estimate.total_cmp(&self.estimate)
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A corner nearer than this to a vertex already there joins it as a footprint is cut in.
const SNAP: f32 = 0.05;

/// Whether `p` lies strictly inside the counter-clockwise convex polygon `polygon`.
fn inside_convex(polygon: &[Vec2], p: Vec2) -> bool {
    let n = polygon.len();
    (0..n).all(|k| (polygon[(k + 1) % n] - polygon[k]).perp_dot(p - polygon[k]) > 0.0)
}

/// What a segment being cut in meets next from the vertex it has reached.
enum Step {
    Vertex(Vec3),
    Split(usize, usize, Vec3),
}

impl Graph {
    pub fn new(map: ArealMap) -> Self {
        let bounds = map.bounds();
        let mut triangles = Vec::new();
        let mut of_areal = Vec::with_capacity(map.areals.len());
        // Where each areal edge landed: (triangle, its edge).
        let mut outer: Vec<Vec<Option<(usize, usize)>>> = Vec::with_capacity(map.areals.len());
        for (a, areal) in map.areals.iter().enumerate() {
            let n = areal.vertices.len();
            let start = triangles.len();
            let polygon: Vec<DVec2> =
                areal.vertices.iter().map(|v| DVec2::new(f64::from(v[0]), f64::from(v[1]))).collect();
            let mut edges = vec![None; n];
            let mut diagonals: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
            for corners in triangulate(&polygon) {
                let t = triangles.len();
                let mut across = [None; 3];
                for k in 0..3 {
                    let (i, j) = (corners[k], corners[(k + 1) % 3]);
                    if j == (i + 1) % n {
                        edges[i] = Some((t, k));
                    } else if let Some((u, e)) = diagonals.remove(&(i.min(j), i.max(j))) {
                        across[k] = Some(u);
                        let tri: &mut Triangle = &mut triangles[u];
                        tri.across[e] = Some(t);
                    } else {
                        diagonals.insert((i.min(j), i.max(j)), (t, k));
                    }
                }
                triangles.push(Triangle {
                    corners: corners.map(|i| Vec3::from(areal.vertices[i])),
                    vertices: [0; 3],
                    areal: a,
                    across,
                });
            }
            of_areal.push((start..triangles.len()).collect());
            outer.push(edges);
        }
        // Across an areal's edge lies the triangle on its twin in the neighbour.
        for (a, areal) in map.areals.iter().enumerate() {
            for (e, &(neighbour, twin)) in areal.edges.iter().enumerate() {
                let (Ok(b), Ok(twin)) = (usize::try_from(neighbour), usize::try_from(twin)) else { continue };
                let there = outer.get(b).and_then(|edges| edges.get(twin).copied().flatten());
                if let (Some((t, k)), Some((u, _))) = (outer[a][e], there) {
                    triangles[t].across[k] = Some(u);
                }
            }
        }
        let broken = vec![false; map.areals.len()];
        let mut graph = Self {
            map,
            bounds,
            triangles,
            of_areal,
            broken,
            pieces: Vec::new(),
            piece_of: Vec::new(),
            fans: Vec::new(),
            outside: Vec::new(),
        };
        graph.finish();
        graph
    }

    /// The vertices, and the pieces each areal's triangles make, worked out afresh.
    fn finish(&mut self) {
        // A vertex is a place, to the centimetre, that corners share.
        let mut places: HashMap<(i64, i64), usize> = HashMap::new();
        let mut fans: Vec<Vec<usize>> = Vec::new();
        for (t, tri) in self.triangles.iter_mut().enumerate() {
            for k in 0..3 {
                let key =
                    ((tri.corners[k].x * 100.0).round() as i64, (tri.corners[k].y * 100.0).round() as i64);
                let v = *places.entry(key).or_insert_with(|| {
                    fans.push(Vec::new());
                    fans.len() - 1
                });
                fans[v].push(t);
                tri.vertices[k] = v;
            }
        }
        let mut outside = vec![false; fans.len()];
        for tri in &self.triangles {
            for k in (0..3).filter(|&k| tri.across[k].is_none()) {
                outside[tri.vertices[k]] = true;
                outside[tri.vertices[(k + 1) % 3]] = true;
            }
        }
        self.fans = fans;
        self.outside = outside;

        let n = self.triangles.len();
        let mut piece_of = vec![usize::MAX; n];
        let mut pieces: Vec<Piece> = Vec::new();
        for first in 0..n {
            if piece_of[first] != usize::MAX {
                continue;
            }
            let (areal, p) = (self.triangles[first].areal, pieces.len());
            let (mut stack, mut members) = (vec![first], Vec::new());
            piece_of[first] = p;
            while let Some(t) = stack.pop() {
                members.push(t);
                for &u in self.triangles[t].across.iter().flatten() {
                    if piece_of[u] == usize::MAX && self.triangles[u].areal == areal {
                        piece_of[u] = p;
                        stack.push(u);
                    }
                }
            }
            pieces.push(Piece { areal, triangles: members, centre: Vec2::ZERO, links: Vec::new() });
        }
        let mut count = vec![0usize; self.map.areals.len()];
        for piece in &pieces {
            count[piece.areal] += 1;
        }
        for piece in &mut pieces {
            piece.centre = if count[piece.areal] == 1 {
                Vec2::from(self.map.areals[piece.areal].centre)
            } else {
                let (mut sum, mut area) = (Vec2::ZERO, 0.0);
                for &t in &piece.triangles {
                    let tri = &self.triangles[t];
                    let a = (tri.flat(1) - tri.flat(0)).perp_dot(tri.flat(2) - tri.flat(0)).abs() / 2.0;
                    sum += tri.centroid() * a;
                    area += a;
                }
                if area > 0.0 { sum / area } else { Vec2::from(self.map.areals[piece.areal].centre) }
            };
        }
        for piece in &mut pieces {
            let mut links = Vec::new();
            for &t in &piece.triangles {
                for &u in self.triangles[t].across.iter().flatten() {
                    let (q, there) = (piece_of[u], self.triangles[u].areal);
                    if there != piece.areal && self.walkable(there) && !links.contains(&q) {
                        links.push(q);
                    }
                }
            }
            piece.links = links;
        }
        self.pieces = pieces;
        self.piece_of = piece_of;
    }

    /// Cut the scenery's `footprints` out of the walkable areals, as the areal map does for an
    /// object of kind 10 (`M_ISystemArealMap::OnAddStatic`, `ArealMap.dll:0x10022580`): each a
    /// rectangle across the ground, corners in order. What lies inside is no longer walkable,
    /// an areal it cuts apart falls into pieces the search takes as nodes, and one it lies
    /// wholly inside keeps a hole a walk goes round.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- the sub-areals' shapes (`0x1000fe00`),
    /// the holes' own list and the inflated contours are not followed: the footprints' edges are
    /// cut into the triangles, what lies inside goes, and each areal's triangles that still hang
    /// together make a piece. A piece is measured from its areal's record centre while the areal
    /// is one piece, and from its triangles' centre once it falls apart, where a sub-areal is
    /// measured from its vertices' average.
    pub fn carve(&mut self, footprints: &[[Vec2; 4]]) {
        if footprints.is_empty() {
            return;
        }
        let (lo, hi) = self.bounds;
        let on_map = |p: Vec2| Vec2::new(p.x.clamp(lo[0], hi[0]), p.y.clamp(lo[1], hi[1]));
        let polygons: Vec<[Vec2; 4]> = footprints
            .iter()
            .map(|f| {
                let mut f = f.map(on_map);
                let area: f32 = (0..4).map(|k| f[k].perp_dot(f[(k + 1) % 4])).sum();
                if area < 0.0 {
                    f.reverse();
                }
                f
            })
            .collect();
        for f in &polygons {
            let corners: Vec<Option<Vec3>> = f.iter().map(|&p| self.insert_point(p)).collect();
            for k in 0..4 {
                if let (Some(a), Some(b)) = (corners[k], corners[(k + 1) % 4]) {
                    self.insert_segment(a, b);
                }
            }
        }
        let gone: Vec<bool> = self
            .triangles
            .iter()
            .map(|t| self.walkable(t.areal) && polygons.iter().any(|f| inside_convex(f, t.centroid())))
            .collect();
        let mut index = vec![usize::MAX; self.triangles.len()];
        let mut kept = Vec::with_capacity(self.triangles.len());
        for (t, tri) in self.triangles.iter().enumerate() {
            if gone[t] {
                self.broken[tri.areal] = true;
            } else {
                index[t] = kept.len();
                kept.push(tri.clone());
            }
        }
        for tri in &mut kept {
            for e in &mut tri.across {
                *e = e.and_then(|u| (index[u] != usize::MAX).then_some(index[u]));
            }
        }
        self.of_areal = vec![Vec::new(); self.map.areals.len()];
        for (t, tri) in kept.iter().enumerate() {
            self.of_areal[tri.areal].push(t);
        }
        self.triangles = kept;
        self.finish();
    }

    /// The triangles of every areal the grid lists within `rings` cells of `p`.
    fn near(&self, p: Vec2, rings: i32) -> Vec<usize> {
        let (lo, hi) = self.bounds;
        let cell = Vec2::new(
            (hi[0] - lo[0]) / self.map.cells_across.max(1) as f32,
            (hi[1] - lo[1]) / self.map.cells_down.max(1) as f32,
        );
        let mut areals: Vec<usize> = Vec::new();
        for dx in -rings..=rings {
            for dy in -rings..=rings {
                let q = p + Vec2::new(dx as f32 * cell.x, dy as f32 * cell.y);
                for &a in self.map.listed_at(self.bounds, q.x, q.y) {
                    if !areals.contains(&usize::from(a)) {
                        areals.push(usize::from(a));
                    }
                }
            }
        }
        areals.iter().flat_map(|&a| self.of_areal.get(a).cloned().unwrap_or_default()).collect()
    }

    /// Point `n` of neighbour `n` of a split at `from` now answers `to`.
    fn repoint(&mut self, n: Option<usize>, from: usize, to: usize) {
        if let Some(n) = n {
            for e in &mut self.triangles[n].across {
                if *e == Some(from) {
                    *e = Some(to);
                }
            }
        }
    }

    /// Triangle `t` cut in two at `x` on its edge `k`: the two halves, each with that edge's
    /// part first, the neighbour across it left for the caller to join.
    fn split_side(&mut self, t: usize, k: usize, x: Vec3) -> (usize, usize) {
        let old = self.triangles[t].clone();
        let c = |i: usize| old.corners[(k + i) % 3];
        let n = |i: usize| old.across[(k + i) % 3];
        let b = self.triangles.len();
        self.triangles[t] = Triangle {
            corners: [c(0), x, c(2)],
            vertices: [0; 3],
            areal: old.areal,
            across: [None, Some(b), n(2)],
        };
        self.triangles.push(Triangle {
            corners: [x, c(1), c(2)],
            vertices: [0; 3],
            areal: old.areal,
            across: [None, n(1), Some(t)],
        });
        self.of_areal[old.areal].push(b);
        self.repoint(n(1), t, b);
        (t, b)
    }

    /// Edge `k` of triangle `t` cut at `x`, and the triangle across it with it.
    fn split_edge(&mut self, t: usize, k: usize, x: Vec3) {
        let u = self.triangles[t].across[k];
        let (ta, tb) = self.split_side(t, k, x);
        let Some(u) = u else { return };
        let Some(k2) = (0..3).find(|&e| self.triangles[u].across[e] == Some(t)) else { return };
        let (ua, ub) = self.split_side(u, k2, x);
        // `t`'s edge runs one way and `u`'s the other: each half meets the other's far half.
        self.triangles[ta].across[0] = Some(ub);
        self.triangles[ub].across[0] = Some(ta);
        self.triangles[tb].across[0] = Some(ua);
        self.triangles[ua].across[0] = Some(tb);
    }

    /// Triangle `t` cut into three about `x` inside it.
    fn split_face(&mut self, t: usize, x: Vec3) {
        let old = self.triangles[t].clone();
        let ([c0, c1, c2], [n0, n1, n2], areal) = (old.corners, old.across, old.areal);
        let (t1, t2) = (self.triangles.len(), self.triangles.len() + 1);
        let tri = |corners, across| Triangle { corners, vertices: [0; 3], areal, across };
        self.triangles[t] = tri([c0, c1, x], [n0, Some(t1), Some(t2)]);
        self.triangles.push(tri([c1, c2, x], [n1, Some(t2), Some(t)]));
        self.triangles.push(tri([c2, c0, x], [n2, Some(t), Some(t1)]));
        self.of_areal[areal].extend([t1, t2]);
        self.repoint(n1, t, t1);
        self.repoint(n2, t, t2);
    }

    /// A vertex at `p`: one already there within [`SNAP`], or `p` cut into the edge or the
    /// triangle it lies on. None off the triangles.
    fn insert_point(&mut self, p: Vec2) -> Option<Vec3> {
        let t = self.near(p, 1).into_iter().find(|&t| self.triangles[t].holds(p, 1e-3))?;
        let tri = self.triangles[t].clone();
        if let Some(k) = (0..3).find(|&k| tri.flat(k).distance(p) < SNAP) {
            return Some(tri.corners[k]);
        }
        for k in 0..3 {
            let (a, b) = (tri.corners[k], tri.corners[(k + 1) % 3]);
            let e = (b - a).truncate();
            let s = ((p - a.truncate()).dot(e) / e.length_squared().max(1e-9)).clamp(0.0, 1.0);
            if (a.truncate() + e * s).distance(p) < SNAP * 0.2 {
                let x = a.lerp(b, s);
                self.split_edge(t, k, x);
                return Some(x);
            }
        }
        let x = p.extend(tri.height(p));
        self.split_face(t, x);
        Some(x)
    }

    /// The segment from vertex `a` to vertex `b` cut into the triangles: each edge it crosses
    /// split where it does, until it runs along edges from one to the other.
    fn insert_segment(&mut self, a: Vec3, b: Vec3) {
        let mut at = a;
        for _ in 0..4096 {
            let (here, there) = (at.truncate(), b.truncate());
            if here.distance(there) < SNAP {
                return;
            }
            let d = there - here;
            let mut step = None;
            for t in self.near(here, 1) {
                let tri = &self.triangles[t];
                let Some(i) = (0..3).find(|&k| tri.flat(k).distance(here) < 1e-4) else { continue };
                let (p, q) = (tri.corners[(i + 1) % 3], tri.corners[(i + 2) % 3]);
                if p.truncate().distance(there) < SNAP || q.truncate().distance(there) < SNAP {
                    return;
                }
                let (pp, qq) = (p.truncate() - here, q.truncate() - here);
                let along = |v: Vec2| v.perp_dot(d).abs() <= 1e-4 * v.length() * d.length() && v.dot(d) > 0.0;
                if along(pp) {
                    step = Some(Step::Vertex(p));
                    break;
                }
                if along(qq) {
                    step = Some(Step::Vertex(q));
                    break;
                }
                if pp.perp_dot(d) > 0.0 && d.perp_dot(qq) > 0.0 {
                    let e = (q - p).truncate();
                    let denominator = d.perp_dot(e);
                    if denominator.abs() < 1e-9 {
                        continue;
                    }
                    let r = ((p.truncate() - here).perp_dot(d) / denominator).clamp(0.0, 1.0);
                    let x = p.lerp(q, r);
                    if x.truncate().distance(here) > d.length() + SNAP {
                        return;
                    }
                    step = Some(if x.truncate().distance(p.truncate()) < SNAP {
                        Step::Vertex(p)
                    } else if x.truncate().distance(q.truncate()) < SNAP {
                        Step::Vertex(q)
                    } else {
                        Step::Split(t, (i + 1) % 3, x)
                    });
                    break;
                }
            }
            match step {
                Some(Step::Vertex(v)) => at = v,
                Some(Step::Split(t, k, x)) => {
                    self.split_edge(t, k, x);
                    at = x;
                }
                None => return,
            }
        }
    }

    pub fn map(&self) -> &ArealMap {
        &self.map
    }

    /// The areal under `(x, y)`.
    pub fn areal_at(&self, x: f32, y: f32) -> Option<usize> {
        self.map.areal_at(self.bounds, x, y)
    }

    /// Whether `(x, y)` lies on a walkable areal: one whose first flag word is set, which the
    /// search links (`ArealMap.dll:0x100232ab`) and a walker may be sent to.
    pub fn usable(&self, x: f32, y: f32) -> bool {
        self.areal_at(x, y).is_some_and(|a| self.walkable(a))
    }

    /// Whether `(x, y)` lies on a walkable areal inside a footprint cut out of it.
    pub fn in_footprint(&self, x: f32, y: f32) -> bool {
        let p = Vec2::new(x, y);
        self.areal_at(x, y).is_some_and(|a| {
            self.walkable(a) && !self.of_areal[a].iter().any(|&t| self.triangles[t].holds(p, 1e-3))
        })
    }

    fn walkable(&self, a: usize) -> bool {
        self.map.areals.get(a).is_some_and(|areal| areal.usable())
    }

    /// The triangle under `p`: one of its areal's, or, on an edge the areals' own test leaves
    /// out or in a footprint, the nearest of those the grid lists there.
    fn triangle_at(&self, p: Vec2) -> Option<usize> {
        if let Some(a) = self.areal_at(p.x, p.y)
            && let Some(t) = self.of_areal[a].iter().copied().find(|&t| self.triangles[t].holds(p, 1e-3))
        {
            return Some(t);
        }
        self.map
            .listed_at(self.bounds, p.x, p.y)
            .iter()
            .flat_map(|&a| self.of_areal.get(usize::from(a)).cloned().unwrap_or_default())
            .min_by(|&x, &y| {
                let d = |t: usize| self.triangles[t].nearest(p).distance_squared(p);
                d(x).total_cmp(&d(y))
            })
    }

    /// The nearest walkable ground to `p` in a footprint, `clearance` on out from the
    /// footprint's edge where that is walkable too, and the triangle it stands in.
    fn off_footprint(&self, p: Vec2, clearance: f32) -> Option<(usize, Vec2)> {
        let d = |t: usize| self.triangles[t].nearest(p).distance_squared(p);
        let t = (1..=12).find_map(|rings| {
            self.near(p, rings)
                .into_iter()
                .filter(|&t| self.walkable(self.triangles[t].areal))
                .min_by(|&x, &y| d(x).total_cmp(&d(y)))
        })?;
        let edge = self.triangles[t].nearest(p);
        let out = edge + (edge - p).normalize_or_zero() * clearance;
        let beyond = self
            .near(out, 1)
            .into_iter()
            .find(|&u| self.walkable(self.triangles[u].areal) && self.triangles[u].holds(out, 1e-3));
        Some(beyond.map_or((t, edge), |u| (u, out)))
    }

    /// The points a walker that does not fly passes from `from` to `goal`, the last `goal`
    /// itself. The search starts from the piece under `from`, or from the nearest vertex of the
    /// way `aboard` names, and ends at the piece under `goal`. `random` gives a number in 0..1
    /// for each link tried; `clearance` is how far a walk through an areal's triangles keeps off
    /// ground that is not walkable.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how a unit's place comes to stand on a
    /// building's map object, and which of its vertices the search starts from, are not read:
    /// a unit on a way's building takes the way's nearest vertex. Nor is what the walker does
    /// with a goal inside a footprint ("Finish is inside ObstacleContour",
    /// `Behavior.dll:0x10039337`), nor how one inside a footprint leaves it ("Leave Obstacle",
    /// `0x1003e81d`): a goal inside one is moved to the nearest ground outside it, `clearance`
    /// on where that is walkable, and a unit inside one sets out from the nearest piece.
    pub fn route(
        &self,
        from: Vec3,
        goal: Vec3,
        ways: &[Way],
        aboard: Option<usize>,
        clearance: f32,
        size: u8,
        random: &mut dyn FnMut() -> f32,
    ) -> Result<Vec<Vec3>, Refusal> {
        let nearest = |w: usize, p: Vec3| {
            let points = &ways.get(w)?.points;
            (0..points.len())
                .min_by(|&a, &b| points[a].distance_squared(p).total_cmp(&points[b].distance_squared(p)))
                .map(|v| Node::Vertex(w, v))
        };
        let areal = match self.areal_at(goal.x, goal.y) {
            Some(a) if self.walkable(a) => a,
            _ => return Err(Refusal::Goal),
        };
        let (goal, end) = match self.of_areal[areal]
            .iter()
            .copied()
            .find(|&t| self.triangles[t].holds(goal.truncate(), 1e-3))
        {
            Some(t) => (goal, Node::Piece(self.piece_of[t])),
            None => {
                let (t, p) = self.off_footprint(goal.truncate(), clearance).ok_or(Refusal::Goal)?;
                (p.extend(self.triangles[t].height(p)), Node::Piece(self.piece_of[t]))
            }
        };
        let start = match aboard.and_then(|w| nearest(w, from)) {
            Some(node) => node,
            None => match self.triangle_at(from.truncate()) {
                Some(t) if self.walkable(self.triangles[t].areal) => Node::Piece(self.piece_of[t]),
                _ if self.in_footprint(from.x, from.y) => {
                    let (t, _) = self.off_footprint(from.truncate(), 0.0).ok_or(Refusal::Stranded)?;
                    Node::Piece(self.piece_of[t])
                }
                _ => return Err(Refusal::Stranded),
            },
        };
        let nodes = self.search(start, end, ways, size, random).ok_or(Refusal::NoWay)?;
        Ok(self.walk(&nodes, from, goal, ways, clearance))
    }

    /// A node's centre, as the search's estimate measures from it: a piece's centre at height 0,
    /// a vertex's point.
    fn node_centre(&self, node: Node, ways: &[Way]) -> Vec3 {
        match node {
            Node::Piece(p) => self.pieces[p].centre.extend(0.0),
            Node::Vertex(w, v) => ways[w].points[v],
        }
    }

    /// The nodes from `start` to `end` (`MWorldGraph`, `Behavior.dll:0x10042c10`): A* ordered
    /// by the cost so far and the distance between centres, each link's new cost the old and
    /// the link's scaled by 1 + random × 0.7. It stops as the goal is reached, and fails once it
    /// has taken 2048 nodes.
    ///
    /// A vertex is reached only where its size gate lets `size` through
    /// (docs/24, "The global path").
    fn search(
        &self,
        start: Node,
        end: Node,
        ways: &[Way],
        size: u8,
        random: &mut dyn FnMut() -> f32,
    ) -> Option<Vec<Node>> {
        if start == end {
            return Some(vec![start]);
        }
        // Each way's exits by the walkable piece under them.
        let mut exits: HashMap<usize, Vec<Node>> = HashMap::new();
        let mut under: HashMap<Node, usize> = HashMap::new();
        for (w, way) in ways.iter().enumerate() {
            for (v, &p) in way.points.iter().enumerate() {
                if way.flags.get(v).is_some_and(|f| f & VERTEX_EXIT != 0)
                    && let Some(a) = self.areal_at(p.x, p.y).filter(|&a| self.walkable(a))
                    && let Some(t) = self.of_areal[a]
                        .iter()
                        .copied()
                        .find(|&t| self.triangles[t].holds(p.truncate(), 1e-3))
                {
                    exits.entry(self.piece_of[t]).or_default().push(Node::Vertex(w, v));
                    under.insert(Node::Vertex(w, v), self.piece_of[t]);
                }
            }
        }
        let goal = self.node_centre(end, ways);
        // The size gate a vertex puts on the unit reaching it (`Behavior.dll:0x10042d08`).
        let fits = |w: usize, v: usize| {
            let way = &ways[w];
            let flags = way.flags.get(v).copied().unwrap_or(0);
            if flags & VERTEX_ANY_SIZE != 0 {
                true
            } else if flags & VERTEX_BUILDING_SIZE != 0 {
                way.size >= size
            } else {
                size <= SMALL_SIZE
            }
        };
        let links = |node: Node| -> Vec<(Node, f32)> {
            let mut out = Vec::new();
            match node {
                Node::Piece(p) => {
                    let piece = &self.pieces[p];
                    for &q in &piece.links {
                        out.push((Node::Piece(q), piece.centre.distance(self.pieces[q].centre) + AREAL_STEP));
                    }
                    out.extend(exits.get(&p).into_iter().flatten().map(|&x| (x, EXIT_STEP)));
                }
                Node::Vertex(w, v) => {
                    let way = &ways[w];
                    let flag = |u: usize| way.flags.get(u).copied().unwrap_or(0);
                    for &(a, b) in &way.links {
                        let other = if a == v {
                            b
                        } else if b == v {
                            a
                        } else {
                            continue;
                        };
                        let (Some(p), Some(q)) = (way.points.get(v), way.points.get(other)) else { continue };
                        let cost = if (flag(v) | flag(other)) & VERTEX_FLAT != 0 {
                            HALL_FLAT
                        } else {
                            HALL_FACTOR * p.distance(*q) + HALL_STEP
                        };
                        out.push((Node::Vertex(w, other), cost));
                    }
                    if let Some(&p) = under.get(&node) {
                        out.push((Node::Piece(p), EXIT_STEP));
                    }
                    if flag(v) & VERTEX_JOIN != 0 {
                        for (x, there) in ways.iter().enumerate().filter(|&(x, _)| x != w) {
                            for (u, q) in there.points.iter().enumerate() {
                                if there.flags.get(u).is_some_and(|f| f & VERTEX_JOIN != 0)
                                    && q.distance(way.points[v]) < JOIN_REACH
                                {
                                    out.push((Node::Vertex(x, u), JOIN_STEP));
                                }
                            }
                        }
                    }
                }
            }
            out
        };

        let mut cost: HashMap<Node, f64> = HashMap::from([(start, 0.0)]);
        let mut before: HashMap<Node, Node> = HashMap::new();
        let mut closed = HashSet::new();
        let mut open = BinaryHeap::from([Open { estimate: 0.0, node: start }]);
        let mut taken = 0;
        while let Some(Open { node, .. }) = open.pop() {
            if !closed.insert(node) {
                continue;
            }
            taken += 1;
            if taken > MAX_NODES {
                return None;
            }
            for (to, step) in links(node) {
                if closed.contains(&to) {
                    continue;
                }
                if let Node::Vertex(w, v) = to
                    && !fits(w, v)
                {
                    continue;
                }
                let scale = 1.0 + f64::from(random()) * RANDOM_SHARE;
                let reached = (cost[&node] + f64::from(step)) * scale;
                if cost.get(&to).is_some_and(|&c| c <= reached) {
                    continue;
                }
                cost.insert(to, reached);
                before.insert(to, node);
                if to == end {
                    let mut nodes = vec![end];
                    while let Some(&n) = before.get(nodes.last().expect("one")) {
                        nodes.push(n);
                    }
                    nodes.reverse();
                    return Some(nodes);
                }
                let estimate = reached + f64::from(self.node_centre(to, ways).distance(goal));
                open.push(Open { estimate, node: to });
            }
        }
        None
    }

    /// The waypoint for a step from piece `a` into piece `b`, from `at` bound for `goal`
    /// (`Behavior.dll:0x10036a80`): on an edge of `a` that `b` lies across, where the straight
    /// line to the goal crosses it, else either end of it moved along it across the ground at
    /// the end's own height, 3 on a whole areal's edge and a fifth of its length on a piece's
    /// (`0x100377d6`); of those, the one that makes the way through it shortest.
    fn edge_point(&self, a: usize, b: usize, at: Vec3, goal: Vec3) -> Option<Vec3> {
        let (from, to) = (at.truncate(), goal.truncate());
        let (areal_a, areal_b) = (self.pieces[a].areal, self.pieces[b].areal);
        let edges: Vec<(Vec3, Vec3, f32)> = if !self.broken[areal_a] && !self.broken[areal_b] {
            let areal = &self.map.areals[areal_a];
            let n = areal.vertices.len();
            (0..n)
                .filter(|&e| areal.neighbour(e) == Some(areal_b))
                .map(|e| (Vec3::from(areal.vertices[e]), Vec3::from(areal.vertices[(e + 1) % n]), EDGE_INSET))
                .collect()
        } else {
            self.pieces[a]
                .triangles
                .iter()
                .flat_map(|&t| {
                    let tri = &self.triangles[t];
                    (0..3)
                        .filter(|&k| tri.across[k].is_some_and(|u| self.piece_of[u] == b))
                        .map(|k| {
                            let (p, q) = (tri.corners[k], tri.corners[(k + 1) % 3]);
                            (p, q, (q - p).truncate().length() * PIECE_EDGE_INSET)
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        edges
            .into_iter()
            .flat_map(|(p, q, inset)| {
                let (span, edge) = (to - from, (q - p).truncate());
                let denominator = span.perp_dot(edge);
                let crossing = (denominator.abs() > 1e-9)
                    .then(|| {
                        let along = (p.truncate() - from).perp_dot(span) / denominator;
                        let ahead = (p.truncate() - from).perp_dot(edge) / denominator;
                        ((0.0..=1.0).contains(&along) && (0.0..=1.0).contains(&ahead))
                            .then(|| p.lerp(q, along))
                    })
                    .flatten();
                let inset = (edge.normalize_or_zero() * inset).extend(0.0);
                match crossing {
                    Some(c) => vec![c],
                    None => vec![p + inset, q - inset],
                }
            })
            .min_by(|x, y| {
                let through = |c: &Vec3| c.truncate().distance(from) + c.truncate().distance(to);
                through(x).total_cmp(&through(y))
            })
    }

    /// The points along `nodes` from `from` to `goal`: each step's waypoint, a vertex's point,
    /// and the goal.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how the walker drops the points a unit has
    /// already passed (`MWalker::ClearMoverReachedPoint`) is not read: a way's first vertex is
    /// left out while the unit stands no farther from the next vertex than it does, so a unit
    /// part way along a way, planned again, goes on rather than back.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how the local path goes round its obstacle
    /// contours is not read. A straight leg across an areal that would leave the walkable ground,
    /// as a leg across one that is not convex or across a footprint cut out of it can, walks
    /// through its piece's triangles instead, pulled straight, `clearance` off each vertex that
    /// touches ground that is not walkable.
    fn walk(&self, nodes: &[Node], from: Vec3, goal: Vec3, ways: &[Way], clearance: f32) -> Vec<Vec3> {
        let allowed = |t: usize| self.walkable(self.triangles[t].areal);
        let exposed = |v: usize| self.outside[v] || self.fans[v].iter().any(|&t| !allowed(t));
        let keep_off = |v: usize| if exposed(v) { clearance } else { 0.0 };
        let leg = |out: &mut Vec<Vec3>, at: Vec3, to: Vec3, areal: Option<usize>| {
            if let Some(a) = areal
                && !self.clear(at.truncate(), to.truncate(), &allowed, &|_| 0.0)
            {
                out.extend(self.through(a, at.truncate(), to.truncate(), &keep_off));
            }
            out.push(to);
        };
        let mut out = Vec::new();
        let mut at = from;
        let mut across = None;
        match nodes.first() {
            Some(&Node::Piece(a)) => across = Some(a),
            // A unit aboard a way steps onto it first.
            Some(&Node::Vertex(w, v)) => {
                at = ways[w].points[v];
                out.push(at);
            }
            None => {}
        }
        for pair in nodes.windows(2) {
            match (pair[0], pair[1]) {
                (Node::Piece(a), Node::Piece(b)) => {
                    let Some(point) = self.edge_point(a, b, at, goal) else { continue };
                    leg(&mut out, at, point, Some(a));
                    at = point;
                    across = Some(b);
                }
                (_, Node::Vertex(w, v)) => {
                    let point = ways[w].points[v];
                    leg(&mut out, at, point, across);
                    at = point;
                    across = None;
                }
                (Node::Vertex(..), Node::Piece(b)) => across = Some(b),
            }
        }
        leg(&mut out, at, goal, across);
        let vertex =
            |p: &Vec3| nodes.iter().any(|n| matches!(*n, Node::Vertex(w, v) if ways[w].points[v] == *p));
        let flat = |a: &Vec3, b: &Vec3| a.truncate().distance(b.truncate());
        while out.len() >= 2
            && vertex(&out[0])
            && vertex(&out[1])
            && flat(&from, &out[1]) <= flat(&out[0], &out[1])
        {
            out.remove(0);
        }
        out
    }

    /// The corners of a walk from `from` to `to` inside piece `piece`, through its triangles by
    /// the shortest run between their centres.
    fn through(&self, piece: usize, from: Vec2, to: Vec2, keep_off: &dyn Fn(usize) -> f32) -> Vec<Vec3> {
        let own = &self.pieces[piece].triangles;
        let nearest = |p: Vec2| {
            own.iter().copied().min_by(|&x, &y| {
                let d = |t: usize| self.triangles[t].nearest(p).distance_squared(p);
                d(x).total_cmp(&d(y))
            })
        };
        let (Some(first), Some(last)) = (nearest(from), nearest(to)) else { return Vec::new() };
        let mut cost: HashMap<usize, f32> = HashMap::from([(first, 0.0)]);
        let mut before: HashMap<usize, usize> = HashMap::new();
        let mut done: HashSet<usize> = HashSet::new();
        while let Some((&t, &c)) =
            cost.iter().filter(|(t, _)| !done.contains(*t)).min_by(|a, b| a.1.total_cmp(b.1))
        {
            if t == last {
                break;
            }
            done.insert(t);
            let tri = &self.triangles[t];
            for &u in tri.across.iter().flatten() {
                if self.piece_of[u] != piece || done.contains(&u) {
                    continue;
                }
                let step = c + tri.centroid().distance(self.triangles[u].centroid());
                if cost.get(&u).is_none_or(|&old| step < old) {
                    cost.insert(u, step);
                    before.insert(u, t);
                }
            }
        }
        let mut run = vec![last];
        while let Some(&t) = before.get(run.last().expect("one")) {
            run.push(t);
        }
        if run.last() != Some(&first) {
            return Vec::new();
        }
        run.reverse();
        self.funnel(&run, from, to, keep_off)
            .into_iter()
            .map(|(t, p)| p.extend(self.triangles[t].height(p)))
            .collect()
    }

    /// Whether the straight line from `a` to `b` crosses only triangles `allowed` lets it,
    /// passing no vertex of them nearer than `keep_off` gives.
    fn clear(
        &self,
        a: Vec2,
        b: Vec2,
        allowed: &dyn Fn(usize) -> bool,
        keep_off: &dyn Fn(usize) -> f32,
    ) -> bool {
        let span = b - a;
        let length = span.length();
        if length < 1e-4 {
            return true;
        }
        let near = |v: Vec2| {
            let t = ((v - a).dot(span) / (length * length)).clamp(0.0, 1.0);
            (a + span * t).distance(v)
        };
        let Some(mut t) = self.triangle_at(a + span / length * 1e-2) else { return false };
        for _ in 0..self.triangles.len() {
            let tri = &self.triangles[t];
            if !allowed(t) || (0..3).any(|k| near(tri.flat(k)) < keep_off(tri.vertices[k]) - 1e-3) {
                return false;
            }
            if tri.holds(b, 1e-3) {
                return true;
            }
            // Out across the edge the line leaves by: b beyond it, and the line meeting it.
            let exit = (0..3).find(|&k| {
                let (p, q) = (tri.flat(k), tri.flat(k + 1));
                let edge = q - p;
                let denominator = span.perp_dot(edge);
                if edge.perp_dot(b - p) >= 0.0 || denominator.abs() < 1e-9 {
                    return false;
                }
                let along = (p - a).perp_dot(span) / denominator;
                let ahead = (p - a).perp_dot(edge) / denominator;
                (-1e-4..=1.0 + 1e-4).contains(&along) && ahead >= -1e-4
            });
            let Some(next) = exit.and_then(|k| tri.across[k]) else { return false };
            t = next;
        }
        false
    }

    /// The corners of the shortest way from `from` to `to` through the corridor of triangles
    /// `run`, each with the triangle it stands in; each portal's end is held `clearance` of its
    /// vertex in, to half the portal's width.
    fn funnel(
        &self,
        run: &[usize],
        from: Vec2,
        to: Vec2,
        clearance: &dyn Fn(usize) -> f32,
    ) -> Vec<(usize, Vec2)> {
        let mut portals = vec![(from, from, run[0])];
        for pair in run.windows(2) {
            let tri = &self.triangles[pair[0]];
            let Some(k) = (0..3).find(|&k| tri.across[k] == Some(pair[1])) else { continue };
            let (left, right) = tri.portal(k);
            let half = left.distance(right) / 2.0;
            let along = (right - left).normalize_or_zero();
            let (inset_left, inset_right) =
                (clearance(tri.vertices[(k + 1) % 3]).min(half), clearance(tri.vertices[k]).min(half));
            portals.push((left + along * inset_left, right - along * inset_right, pair[1]));
        }
        portals.push((to, to, *run.last().expect("a corridor")));

        // A corner is kept once, and the funnel's end is not one.
        let push = |out: &mut Vec<(usize, Vec2)>, t: usize, p: Vec2| {
            if p != to && out.last().is_none_or(|&(_, q)| q != p) {
                out.push((t, p));
            }
        };
        let mut out = Vec::new();
        let (mut apex, mut left, mut right) = (from, from, from);
        let (mut left_at, mut right_at) = (0, 0);
        let mut i = 1;
        while i < portals.len() {
            let (l, r, _) = portals[i];
            // The right side closes in while the new right is not outside the funnel.
            if (right - apex).perp_dot(r - apex) >= 0.0 {
                if apex == right || (left - apex).perp_dot(r - apex) < 0.0 {
                    right = r;
                    right_at = i;
                } else {
                    let at = left_at;
                    apex = left;
                    push(&mut out, portals[at].2, apex);
                    (left, right, left_at, right_at) = (apex, apex, at, at);
                    i = at + 1;
                    continue;
                }
            }
            if (left - apex).perp_dot(l - apex) <= 0.0 {
                if apex == left || (right - apex).perp_dot(l - apex) > 0.0 {
                    left = l;
                    left_at = i;
                } else {
                    let at = right_at;
                    apex = right;
                    push(&mut out, portals[at].2, apex);
                    (left, right, left_at, right_at) = (apex, apex, at, at);
                    i = at + 1;
                    continue;
                }
            }
            i += 1;
        }
        out
    }

    /// Where a walker on an areal no link leaves goes (`Behavior.dll:0x1003e2dc`): the first of
    /// 50 random points in a square about it, of half-width 30 and 3 more each round while under
    /// 500, that lies on a walkable areal; none past that ("Warbot is absolutely in
    /// non-walkable").
    ///
    /// STAND-IN: docs/24-motion.md#the-global-path--read -- the square's doubling for a unit
    /// whose slot 14 answers `0x20000000` is not modelled.
    pub fn escape(&self, from: Vec3, random: &mut dyn FnMut() -> f32) -> Option<Vec3> {
        let mut reach = ESCAPE_REACH;
        loop {
            for _ in 0..ESCAPE_TRIES {
                let (u, v) = (random(), random());
                let p = Vec2::new(from.x + (u - 0.5) * reach * 2.0, from.y + (v - 0.5) * reach * 2.0);
                if self.usable(p.x, p.y) {
                    let z = self.triangle_at(p).map_or(from.z, |t| self.triangles[t].height(p));
                    return Some(p.extend(z));
                }
            }
            reach += ESCAPE_GROWTH;
            if reach >= ESCAPE_LIMIT {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::arealmap::{Areal, NO_NEIGHBOUR};

    /// A map of `across` × `down` squares of side 10, areal `x * down + y` at column x, row y,
    /// walkable unless `blocked` names it; the grid is one cell a square.
    fn squares(across: usize, down: usize, blocked: &[usize]) -> ArealMap {
        let index = |x: i64, y: i64| {
            if x >= 0 && y >= 0 && (x as usize) < across && (y as usize) < down {
                (x as usize * down + y as usize) as i32
            } else {
                NO_NEIGHBOUR
            }
        };
        let mut areals = Vec::new();
        for x in 0..across as i64 {
            for y in 0..down as i64 {
                let (x0, y0) = (x as f32 * 10.0, y as f32 * 10.0);
                // Counter-clockwise from the bottom left; edge k runs from corner k to k + 1.
                let vertices = vec![
                    [x0, y0, 0.0],
                    [x0 + 10.0, y0, 0.0],
                    [x0 + 10.0, y0 + 10.0, 0.0],
                    [x0, y0 + 10.0, 0.0],
                ];
                let near = [index(x, y - 1), index(x + 1, y), index(x, y + 1), index(x - 1, y)];
                let twin = [2, 3, 0, 1];
                let edges = (0..4)
                    .map(|k| if near[k] < 0 { (NO_NEIGHBOUR, NO_NEIGHBOUR) } else { (near[k], twin[k]) })
                    .collect();
                let i = (x as usize) * down + y as usize;
                let flags = [u32::from(!blocked.contains(&i)), 0, 4, 0];
                areals.push(Areal { centre: [x0 + 5.0, y0 + 5.0], area: 100.0, vertices, edges, flags });
            }
        }
        let cells = (0..across * down).map(|i| vec![i as u16]).collect();
        ArealMap { areals, cells_across: across, cells_down: down, cells }
    }

    /// A random source that always gives `u`.
    fn fixed(u: f32) -> impl FnMut() -> f32 {
        move || u
    }

    /// Whether every leg from `from` through `legs` keeps off the squares in `blocked`.
    fn keeps_off(map: &ArealMap, from: Vec3, legs: &[Vec3], blocked: &[usize]) -> bool {
        let mut a = from;
        legs.iter().all(|&b| {
            let clear = (0..=100).all(|s| {
                let p = a.lerp(b, s as f32 / 100.0);
                let (x, y) = ((p.x / 10.0) as usize, (p.y / 10.0) as usize);
                !blocked.contains(&(x * map.cells_down + y))
            });
            a = b;
            clear
        })
    }

    #[test]
    #[ignore = "needs the game install"]
    fn every_maps_areals_are_cut_into_triangles_that_tile_them_and_meet_across_their_edges() {
        use parkan_formats::{arealmap, gamedir};
        let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
        let maps = gamedir::resolve(&game, "DATA/MAPS").unwrap();
        let mut dirs: Vec<_> = std::fs::read_dir(maps).unwrap().map(|e| e.unwrap().path()).collect();
        dirs.sort();
        let mut count = 0;
        for dir in dirs {
            let Some(land) = gamedir::resolve(&dir, "Land.map") else { continue };
            let graph = Graph::new(arealmap::load(&land).unwrap());
            count += 1;
            for (a, areal) in graph.map.areals.iter().enumerate() {
                let n = areal.vertices.len();
                let polygon: f64 = (0..n)
                    .map(|i| {
                        let (p, q) = (areal.vertices[i], areal.vertices[(i + 1) % n]);
                        f64::from(p[0]) * f64::from(q[1]) - f64::from(q[0]) * f64::from(p[1])
                    })
                    .sum::<f64>()
                    / 2.0;
                let cut: f64 = graph.of_areal[a]
                    .iter()
                    .map(|&t| {
                        let [p, q, r] = graph.triangles[t].corners.map(|c| c.truncate().as_dvec2());
                        (q - p).perp_dot(r - p) / 2.0
                    })
                    .sum();
                assert_eq!(graph.of_areal[a].len(), n - 2, "{dir:?} areal {a}");
                assert!(
                    (cut - polygon).abs() <= 1e-3 * polygon.abs().max(1.0),
                    "{dir:?} areal {a}: {cut} of {polygon}"
                );
                // Each edge with an areal across it has a triangle across it.
                let across = areal.edges.iter().filter(|e| e.0 >= 0).count();
                let met = graph.of_areal[a]
                    .iter()
                    .flat_map(|&t| {
                        graph.triangles[t].across.map(|u| u.filter(|&u| graph.triangles[u].areal != a))
                    })
                    .flatten()
                    .count();
                assert_eq!(met, across, "{dir:?} areal {a}");
            }
            for (t, tri) in graph.triangles.iter().enumerate() {
                for u in tri.across.iter().flatten() {
                    assert!(graph.triangles[*u].across.contains(&Some(t)), "{dir:?} triangle {t}");
                }
            }
        }
        assert_eq!(count, 33);
    }

    #[test]
    fn a_polygon_with_a_notch_is_cut_into_its_vertex_count_less_two_triangles() {
        let l: Vec<DVec2> = [[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [5.0, 5.0], [5.0, 10.0], [0.0, 10.0]]
            .map(|[x, y]| DVec2::new(x, y))
            .to_vec();
        let triangles = triangulate(&l);
        assert_eq!(triangles.len(), 4);
        let area: f64 = triangles.iter().map(|&[a, b, c]| (l[b] - l[a]).perp_dot(l[c] - l[a]) / 2.0).sum();
        assert!((area - 75.0).abs() < 1e-9, "they tile it: {area}");
        assert!(triangles.iter().all(|&[a, b, c]| (l[b] - l[a]).perp_dot(l[c] - l[a]) > 0.0));
        // A collinear corner is kept, so each areal edge stays one triangle's edge.
        let flat: Vec<DVec2> = [[0.0, 0.0], [5.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            .map(|[x, y]| DVec2::new(x, y))
            .to_vec();
        let triangles = triangulate(&flat);
        assert_eq!(triangles.len(), 3);
        assert!(triangles.iter().all(|&[a, b, c]| (flat[b] - flat[a]).perp_dot(flat[c] - flat[a]) > 0.0));
    }

    #[test]
    fn a_walk_crosses_each_edge_where_the_line_to_the_goal_does_or_three_from_an_end() {
        let graph = Graph::new(squares(5, 1, &[]));
        let (from, goal) = (Vec3::new(5.0, 2.0, 0.0), Vec3::new(45.0, 8.0, 7.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        // Along a row the line to the goal crosses every edge where it meets it.
        let line = (goal - from).truncate().normalize();
        assert_eq!(legs.len(), 5, "{legs:?}");
        assert_eq!(legs.last(), Some(&goal));
        assert!(
            legs.iter().all(|p| line.perp_dot((*p - from).truncate()).abs() < 1e-3),
            "on the line: {legs:?}"
        );
        // On a grid the search takes a staircase, and an edge the line misses is crossed 3 from
        // the end nearer the way on.
        let graph = Graph::new(squares(5, 5, &[]));
        let (from, goal) = (Vec3::new(5.0, 5.0, 0.0), Vec3::new(45.0, 35.0, 7.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        assert_eq!(legs.last(), Some(&goal));
        let on_edge = |p: &Vec3| p.x % 10.0 == 0.0 || p.y % 10.0 == 0.0;
        assert!(legs[..legs.len() - 1].iter().all(on_edge), "{legs:?}");
        assert!(legs.windows(2).all(|w| w[0].x <= w[1].x), "{legs:?}");
        let inset = |p: &Vec3| {
            [p.x % 10.0, p.y % 10.0].iter().any(|&r| (r - 3.0).abs() < 1e-3 || (r - 7.0).abs() < 1e-3)
        };
        assert!(legs.iter().any(inset), "{legs:?}");
    }

    #[test]
    fn a_walker_is_refused_a_goal_it_may_not_be_sent_to_and_one_it_cannot_reach() {
        let wall = [10, 11, 12, 13, 14];
        let graph = Graph::new(squares(5, 5, &wall));
        let from = Vec3::new(5.0, 25.0, 0.0);
        let route = |goal: Vec3| graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.5));
        assert_eq!(route(Vec3::new(25.0, 25.0, 0.0)), Err(Refusal::Goal), "on the wall");
        assert_eq!(route(Vec3::new(75.0, 25.0, 0.0)), Err(Refusal::Goal), "off the map");
        assert_eq!(route(Vec3::new(45.0, 25.0, 0.0)), Err(Refusal::NoWay), "beyond the wall");
        let on_wall = graph.route(Vec3::new(25.0, 5.0, 0.0), from, &[], None, 2.0, 2, &mut fixed(0.5));
        assert_eq!(on_wall, Err(Refusal::Stranded));
    }

    #[test]
    fn a_walk_goes_round_an_areal_it_may_not_be_sent_to_three_along_the_edges_it_misses() {
        // A wall down the middle column, x 20 to 30, with a gap in its top row.
        let wall = [10, 11, 12, 13];
        let map = squares(5, 5, &wall);
        let graph = Graph::new(map.clone());
        let (from, goal) = (Vec3::new(5.0, 5.0, 0.0), Vec3::new(45.0, 5.0, 0.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).expect("a way round");
        assert_eq!(legs.last(), Some(&goal));
        assert!(keeps_off(&map, from, &legs, &wall), "{legs:?}");
        // Into the gap and out of it over the edges x = 20 and x = 30 of its row, 3 above the
        // wall's ends, since the line to the goal crosses neither.
        let (into, out) = (Vec3::new(20.0, 43.0, 0.0), Vec3::new(30.0, 43.0, 0.0));
        assert!(legs.contains(&into) && legs.contains(&out), "{legs:?}");
    }

    #[test]
    fn a_leg_that_would_cut_across_ground_it_may_not_cross_follows_its_areals_triangles() {
        // An L-shaped walkable areal wrapped round a square that is not: the straight leg from
        // one arm to the other cuts the square.
        let outside = (NO_NEIGHBOUR, NO_NEIGHBOUR);
        let l = Areal {
            centre: [5.0, 5.0],
            area: 300.0,
            vertices: [[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0], [10.0, 20.0], [0.0, 20.0]]
                .map(|[x, y]| [x, y, 0.0])
                .to_vec(),
            edges: vec![outside, outside, (1, 0), (1, 3), outside, outside],
            flags: [1, 0, 4, 0],
        };
        let corner = Areal {
            centre: [15.0, 15.0],
            area: 100.0,
            vertices: [[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0]]
                .map(|[x, y]| [x, y, 0.0])
                .to_vec(),
            edges: vec![(0, 2), outside, outside, (0, 3)],
            flags: [0, 0, 4, 0],
        };
        let map =
            ArealMap { areals: vec![l, corner], cells_across: 1, cells_down: 1, cells: vec![vec![0, 1]] };
        let graph = Graph::new(map);
        let (from, goal) = (Vec3::new(18.0, 2.0, 0.0), Vec3::new(2.0, 18.0, 0.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        assert_eq!(legs.last(), Some(&goal));
        let mut at = from;
        for &p in &legs {
            for s in 0..=100 {
                let q = at.lerp(p, s as f32 / 100.0);
                assert!(!(q.x > 10.0 && q.y > 10.0), "{q} is in the square: {legs:?}");
            }
            at = p;
        }
        // It rounds the square's corner at (10, 10) the clearance off.
        let round = legs.iter().any(|p| (p.distance(Vec3::new(10.0, 10.0, 0.0)) - 2.0).abs() < 1e-3);
        assert!(round, "{legs:?}");
    }

    #[test]
    fn a_hall_way_joins_the_areals_under_its_exits_and_its_halves_join_within_fifty() {
        let wall = [10, 11, 12, 13, 14];
        let map = squares(5, 5, &wall);
        let graph = Graph::new(map.clone());
        // Two halves of a deck over the wall, each an exit on the ground and a joining end.
        let half = |exit: Vec3, end: Vec3| Way {
            points: vec![exit, end],
            flags: vec![VERTEX_ANY_SIZE | VERTEX_EXIT, VERTEX_ANY_SIZE | VERTEX_JOIN],
            links: vec![(0, 1)],
            size: 2,
        };
        let ways = [
            half(Vec3::new(15.0, 25.0, 0.0), Vec3::new(24.0, 25.0, 5.0)),
            half(Vec3::new(35.0, 25.0, 0.0), Vec3::new(26.0, 25.0, 5.0)),
        ];
        let (from, goal) = (Vec3::new(5.0, 5.0, 0.0), Vec3::new(45.0, 5.0, 0.0));
        let legs = graph.route(from, goal, &ways, None, 2.0, 2, &mut fixed(0.0)).expect("over the deck");
        let on = |p: Vec3| legs.iter().position(|&q| q == p).unwrap_or(usize::MAX);
        let order =
            [on(ways[0].points[0]), on(ways[0].points[1]), on(ways[1].points[1]), on(ways[1].points[0])];
        assert!(order.windows(2).all(|w| w[0] < w[1]) && order[3] < legs.len(), "along the deck: {legs:?}");
        assert!(keeps_off(&map, from, &legs[..=order[0]], &wall));
        assert!(keeps_off(&map, ways[1].points[0], &legs[order[3] + 1..], &wall));
        // Halves 50 apart do not join.
        let apart = [ways[0].clone(), half(Vec3::new(35.0, 25.0, 0.0), Vec3::new(74.0, 25.0, 5.0))];
        assert_eq!(graph.route(from, goal, &apart, None, 2.0, 2, &mut fixed(0.0)), Err(Refusal::NoWay));
        // A unit on the deck sets out from its nearest vertex, or from the next once it stands no
        // farther from that; a goal over the deck is refused, the wall being under it.
        let aboard = graph.route(Vec3::new(25.5, 25.0, 5.0), goal, &ways, Some(1), 2.0, 2, &mut fixed(0.0));
        assert_eq!(aboard.map(|l| l[0]), Ok(ways[1].points[1]));
        let past = graph.route(Vec3::new(27.0, 25.0, 5.0), goal, &ways, Some(1), 2.0, 2, &mut fixed(0.0));
        assert_eq!(past.map(|l| l[0]), Ok(ways[1].points[0]), "the joining end is behind it");
        let onto = graph.route(from, Vec3::new(25.0, 25.0, 5.0), &ways, None, 2.0, 2, &mut fixed(0.0));
        assert_eq!(onto, Err(Refusal::Goal));
    }

    #[test]
    fn a_hall_way_vertex_gates_the_unit_by_size() {
        let wall = [10, 11, 12, 13, 14];
        let map = squares(5, 5, &wall);
        let graph = Graph::new(map);
        // A deck over the wall: an exit each side and a middle vertex whose flags decide.
        let deck = |middle: u32, size: u8| Way {
            points: vec![Vec3::new(15.0, 25.0, 0.0), Vec3::new(25.0, 25.0, 5.0), Vec3::new(35.0, 25.0, 0.0)],
            flags: vec![VERTEX_ANY_SIZE | VERTEX_EXIT, middle, VERTEX_ANY_SIZE | VERTEX_EXIT],
            links: vec![(0, 1), (1, 2)],
            size,
        };
        let (from, goal) = (Vec3::new(5.0, 25.0, 0.0), Vec3::new(45.0, 25.0, 0.0));
        let over = |middle: u32, size: u8, unit: u8| {
            let ways = [deck(middle, size)];
            graph.route(from, goal, &ways, None, 2.0, unit, &mut fixed(0.0)).is_ok()
        };
        // A vertex with neither flag passes only size class 2 or less -- the shipped pods' case.
        assert!(over(0, 4, 2));
        assert!(over(0, 4, 1));
        assert!(!over(0, 4, 3));
        assert!(!over(0, 4, 4));
        // 0x10000000, a ground-level place, passes anything.
        assert!(over(VERTEX_ANY_SIZE, 2, 4));
        // 0x20000000 passes a unit no bigger than the building.
        assert!(over(VERTEX_BUILDING_SIZE, 4, 4));
        assert!(over(VERTEX_BUILDING_SIZE, 4, 3));
        assert!(!over(VERTEX_BUILDING_SIZE, 3, 4));
    }

    /// A rectangle's corners from `lo` to `hi`, counter-clockwise.
    fn rectangle(lo: [f32; 2], hi: [f32; 2]) -> [Vec2; 4] {
        [Vec2::new(lo[0], lo[1]), Vec2::new(hi[0], lo[1]), Vec2::new(hi[0], hi[1]), Vec2::new(lo[0], hi[1])]
    }

    /// Whether any leg from `from` through `legs` passes inside `rect` by more than `slack`.
    fn enters(from: Vec3, legs: &[Vec3], rect: [Vec2; 4], slack: f32) -> bool {
        let mut a = from;
        legs.iter().any(|&b| {
            let inside = (0..=200).any(|s| {
                let p = a.lerp(b, s as f32 / 200.0);
                p.x > rect[0].x + slack
                    && p.x < rect[2].x - slack
                    && p.y > rect[0].y + slack
                    && p.y < rect[2].y - slack
            });
            a = b;
            inside
        })
    }

    /// Every triangle turns counter-clockwise and meets the one across each edge on that edge,
    /// the other way round.
    fn sound(graph: &Graph) {
        for (t, tri) in graph.triangles.iter().enumerate() {
            assert!((tri.flat(1) - tri.flat(0)).perp_dot(tri.flat(2) - tri.flat(0)) > 0.0, "{t}: {tri:?}");
            for k in 0..3 {
                let Some(u) = tri.across[k] else { continue };
                let there = &graph.triangles[u];
                let back = (0..3).find(|&e| there.across[e] == Some(t));
                let Some(e) = back else { panic!("{u} does not answer {t}") };
                assert!(there.flat(e).distance(tri.flat(k + 1)) < 1e-3, "{t}/{k} and {u}/{e}");
                assert!(there.flat(e + 1).distance(tri.flat(k)) < 1e-3, "{t}/{k} and {u}/{e}");
            }
        }
    }

    /// The ground each areal's triangles cover.
    fn cover(graph: &Graph, a: usize) -> f32 {
        graph.of_areal[a]
            .iter()
            .map(|&t| {
                let tri = &graph.triangles[t];
                (tri.flat(1) - tri.flat(0)).perp_dot(tri.flat(2) - tri.flat(0)) / 2.0
            })
            .sum()
    }

    #[test]
    fn a_footprint_inside_an_areal_leaves_a_hole_its_walks_go_round() {
        let mut graph = Graph::new(squares(5, 5, &[]));
        let stone = rectangle([22.0, 22.0], [28.0, 28.0]);
        graph.carve(&[stone]);
        sound(&graph);
        assert!((cover(&graph, 12) - 64.0).abs() < 1e-3, "{}", cover(&graph, 12));
        assert!((cover(&graph, 11) - 100.0).abs() < 1e-3);
        // The areal is still one piece, measured from its record centre.
        let pieces: Vec<_> = graph.pieces.iter().filter(|p| p.areal == 12).collect();
        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].centre, Vec2::new(25.0, 25.0));
        assert!(graph.in_footprint(25.0, 25.0) && !graph.in_footprint(21.0, 25.0));
        assert!(graph.usable(25.0, 25.0), "the areal is still walkable");

        let (from, goal) = (Vec3::new(5.0, 25.0, 0.0), Vec3::new(45.0, 25.0, 0.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        assert_eq!(legs.last(), Some(&goal));
        assert!(!enters(from, &legs, stone, 1e-3), "{legs:?}");
        // Round the stone's corners, off them by the clearance or half the way past them.
        let corners =
            [Vec2::new(22.0, 22.0), Vec2::new(28.0, 22.0), Vec2::new(22.0, 28.0), Vec2::new(28.0, 28.0)];
        let off = |p: &Vec3| corners.iter().map(|c| p.truncate().distance(*c)).fold(f32::MAX, f32::min);
        assert!(legs.iter().filter(|p| off(p) < 2.0 + 1e-3).count() == 2, "{legs:?}");
        assert!(legs.iter().all(|p| off(p) > 1.0), "{legs:?}");
    }

    #[test]
    fn a_footprint_across_an_areals_edges_cuts_it_into_pieces_the_search_links_apart() {
        // Down the middle square of a 3 × 3 grid, from inside the square below to inside the one
        // above.
        let mut graph = Graph::new(squares(3, 3, &[]));
        let stone = rectangle([12.0, 5.0], [18.0, 25.0]);
        graph.carve(&[stone]);
        sound(&graph);
        let middle: Vec<_> = graph.pieces.iter().filter(|p| p.areal == 4).collect();
        assert_eq!(middle.len(), 2, "{middle:?}");
        let mut centres: Vec<Vec2> = middle.iter().map(|p| p.centre).collect();
        centres.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert!(centres[0].distance(Vec2::new(11.0, 15.0)) < 1e-3, "{centres:?}");
        assert!(centres[1].distance(Vec2::new(19.0, 15.0)) < 1e-3, "{centres:?}");
        assert_eq!(graph.pieces.iter().filter(|p| p.areal == 3).count(), 1, "a notch only");
        assert!((cover(&graph, 3) - 70.0).abs() < 1e-3 && (cover(&graph, 4) - 40.0).abs() < 1e-3);

        let (from, goal) = (Vec3::new(5.0, 15.0, 0.0), Vec3::new(25.0, 15.0, 0.0));
        let legs = graph.route(from, goal, &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        assert_eq!(legs.last(), Some(&goal));
        assert!(!enters(from, &legs, stone, 1e-3), "{legs:?}");

        // Right across the map it leaves no way past.
        let mut graph = Graph::new(squares(3, 1, &[]));
        graph.carve(&[rectangle([13.0, -5.0], [17.0, 15.0])]);
        sound(&graph);
        let route = graph.route(
            Vec3::new(5.0, 5.0, 0.0),
            Vec3::new(25.0, 5.0, 0.0),
            &[],
            None,
            2.0,
            2,
            &mut fixed(0.0),
        );
        assert_eq!(route, Err(Refusal::NoWay));
    }

    #[test]
    fn a_walker_sent_into_a_footprint_stops_outside_it_and_one_inside_walks_out() {
        let mut graph = Graph::new(squares(5, 5, &[]));
        let stone = rectangle([22.0, 22.0], [28.0, 28.0]);
        // Two footprints that overlap cut as one.
        graph.carve(&[stone, rectangle([26.0, 21.0], [29.0, 24.0])]);
        sound(&graph);
        assert!((cover(&graph, 12) - (100.0 - 36.0 - 9.0 + 4.0)).abs() < 1e-3, "{}", cover(&graph, 12));

        let from = Vec3::new(5.0, 25.0, 0.0);
        let legs = graph.route(from, Vec3::new(23.0, 25.0, 0.0), &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        let last = *legs.last().unwrap();
        assert!(!enters(from, &legs, stone, 1e-3), "{legs:?}");
        assert!(last.truncate().distance(Vec2::new(20.0, 25.0)) < 1e-3, "2 in from the stone's edge: {last}");

        let inside = Vec3::new(24.0, 25.0, 0.0);
        let legs =
            graph.route(inside, Vec3::new(5.0, 25.0, 0.0), &[], None, 2.0, 2, &mut fixed(0.0)).unwrap();
        assert_eq!(legs.last(), Some(&Vec3::new(5.0, 25.0, 0.0)));
    }

    #[test]
    fn a_stranded_walker_makes_for_the_first_walkable_point_of_its_square() {
        let wall = [10, 11, 12, 13, 14];
        let graph = Graph::new(squares(5, 5, &wall));
        let from = Vec3::new(25.0, 25.0, 0.0);
        // Its square's right edge is off the map and its middle on the wall, however far out.
        let mut draws = [1.0, 0.5, 0.5, 0.5].into_iter().cycle();
        assert_eq!(graph.escape(from, &mut || draws.next().unwrap()), None);
        // The first try lands off the map at x −5; the second 15 left, on walkable ground.
        let mut draws = [0.0, 0.5, 0.25, 0.5].into_iter();
        assert_eq!(graph.escape(from, &mut || draws.next().unwrap()), Some(Vec3::new(10.0, 25.0, 0.0)));
    }
}
