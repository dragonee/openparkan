//! The walker's global path: the areals and hall-way vertices from a unit to its goal, found
//! by A* over the links the areal map builds, and the points the unit walks through them. See
//! `docs/24-motion.md`, "The global path".

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

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
/// Each link's cost, added to the cost so far, is scaled by 1 and up to this share at random
/// (graph `+0x38`, `Behavior.dll:0x1003642b`, `0x10042e0b`).
pub const RANDOM_SHARE: f64 = 0.7;
/// The search takes no more nodes than this off its open list (`0x100207df`).
pub const MAX_NODES: usize = 2048;
/// A waypoint on an edge the straight line to the goal misses stands this far along the edge
/// from one of its ends (`Behavior.dll:0x10036cc5`).
pub const EDGE_INSET: f32 = 3.0;
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

/// The areal map as the walker's search links it, and cut into triangles for a walk through
/// an areal that is not convex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graph {
    map: ArealMap,
    bounds: ([f32; 2], [f32; 2]),
    triangles: Vec<Triangle>,
    /// Each areal's triangles, a range of `triangles`.
    of_areal: Vec<std::ops::Range<usize>>,
    /// Each vertex's triangles, and whether it lies on the outside of the map.
    fans: Vec<Vec<usize>>,
    outside: Vec<bool>,
}

/// A node of the search: an areal, or a way's vertex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Node {
    Areal(usize),
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
            of_areal.push(start..triangles.len());
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
        // A vertex is a place, to the centimetre, that corners share.
        let mut places: HashMap<(i64, i64), usize> = HashMap::new();
        let mut fans: Vec<Vec<usize>> = Vec::new();
        for (t, tri) in triangles.iter_mut().enumerate() {
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
        for tri in &triangles {
            for k in (0..3).filter(|&k| tri.across[k].is_none()) {
                outside[tri.vertices[k]] = true;
                outside[tri.vertices[(k + 1) % 3]] = true;
            }
        }
        Self { map, bounds, triangles, of_areal, fans, outside }
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

    fn walkable(&self, a: usize) -> bool {
        self.map.areals.get(a).is_some_and(|areal| areal.usable())
    }

    fn centre(&self, a: usize) -> Vec2 {
        Vec2::from(self.map.areals[a].centre)
    }

    /// The triangle under `p`: one of its areal's, or, on an edge the areals' own test leaves
    /// out, the nearest of those the grid lists there.
    fn triangle_at(&self, p: Vec2) -> Option<usize> {
        if let Some(a) = self.areal_at(p.x, p.y)
            && let Some(t) = self.of_areal[a].clone().find(|&t| self.triangles[t].holds(p, 1e-3))
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

    /// The points a walker that does not fly passes from `from` to `goal`, the last `goal`
    /// itself. The search starts from the areal under `from`, or from the nearest vertex of the
    /// way `aboard` names, and ends at the areal under `goal`. `random` gives a number in 0..1
    /// for each link tried; `clearance` is how far a walk through an areal's triangles keeps off
    /// ground that is not walkable.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- how a unit's place comes to stand on a
    /// building's map object, and which of its vertices the search starts from, are not read:
    /// a unit on a way's building takes the way's nearest vertex.
    pub fn route(
        &self,
        from: Vec3,
        goal: Vec3,
        ways: &[Way],
        aboard: Option<usize>,
        clearance: f32,
        random: &mut dyn FnMut() -> f32,
    ) -> Result<Vec<Vec3>, Refusal> {
        let nearest = |w: usize, p: Vec3| {
            let points = &ways.get(w)?.points;
            (0..points.len())
                .min_by(|&a, &b| points[a].distance_squared(p).total_cmp(&points[b].distance_squared(p)))
                .map(|v| Node::Vertex(w, v))
        };
        let end = match self.areal_at(goal.x, goal.y) {
            Some(a) if self.walkable(a) => Node::Areal(a),
            _ => return Err(Refusal::Goal),
        };
        let start = match aboard.and_then(|w| nearest(w, from)) {
            Some(node) => node,
            None => match self.triangle_at(from.truncate()).map(|t| self.triangles[t].areal) {
                Some(a) if self.walkable(a) => Node::Areal(a),
                _ => return Err(Refusal::Stranded),
            },
        };
        let nodes = self.search(start, end, ways, random).ok_or(Refusal::NoWay)?;
        Ok(self.walk(&nodes, from, goal, ways, clearance))
    }

    /// A node's centre, as the search's estimate measures from it: an areal's record centre at
    /// height 0, a vertex's point.
    fn node_centre(&self, node: Node, ways: &[Way]) -> Vec3 {
        match node {
            Node::Areal(a) => self.centre(a).extend(0.0),
            Node::Vertex(w, v) => ways[w].points[v],
        }
    }

    /// The nodes from `start` to `end` (`MWorldGraph`, `Behavior.dll:0x10042c10`): A* ordered
    /// by the cost so far and the distance between centres, each link's new cost the old and
    /// the link's scaled by 1 + random × 0.7. It stops as the goal is reached, and fails once it
    /// has taken 2048 nodes.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- the size gate a hall-way vertex puts on
    /// a unit (its flags `0x10000000` and `0x20000000` against the unit's `+0x960`, and its
    /// record's `+0x28`) is not modelled: every vertex passes.
    fn search(
        &self,
        start: Node,
        end: Node,
        ways: &[Way],
        random: &mut dyn FnMut() -> f32,
    ) -> Option<Vec<Node>> {
        if start == end {
            return Some(vec![start]);
        }
        // Each way's exits by the walkable areal under them.
        let mut exits: HashMap<usize, Vec<Node>> = HashMap::new();
        let mut under: HashMap<Node, usize> = HashMap::new();
        for (w, way) in ways.iter().enumerate() {
            for (v, &p) in way.points.iter().enumerate() {
                if way.flags.get(v).is_some_and(|f| f & VERTEX_EXIT != 0)
                    && let Some(a) = self.areal_at(p.x, p.y).filter(|&a| self.walkable(a))
                {
                    exits.entry(a).or_default().push(Node::Vertex(w, v));
                    under.insert(Node::Vertex(w, v), a);
                }
            }
        }
        let goal = self.node_centre(end, ways);
        let links = |node: Node| -> Vec<(Node, f32)> {
            let mut out = Vec::new();
            match node {
                Node::Areal(a) => {
                    let areal = &self.map.areals[a];
                    for b in (0..areal.edges.len()).filter_map(|e| areal.neighbour(e)) {
                        if self.walkable(b) {
                            out.push((Node::Areal(b), self.centre(a).distance(self.centre(b)) + AREAL_STEP));
                        }
                    }
                    out.extend(exits.get(&a).into_iter().flatten().map(|&x| (x, EXIT_STEP)));
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
                    if let Some(&a) = under.get(&node) {
                        out.push((Node::Areal(a), EXIT_STEP));
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

    /// The waypoint for a step from areal `a` into `b`, from `at` bound for `goal`
    /// (`Behavior.dll:0x10036a80`): on an edge of `a` that `b` lies across, where the straight
    /// line to the goal crosses it, else either end of it moved 3 along it across the ground
    /// at the end's own height; of those, the one that makes the way through it shortest.
    fn edge_point(&self, a: usize, b: usize, at: Vec3, goal: Vec3) -> Option<Vec3> {
        let areal = &self.map.areals[a];
        let n = areal.vertices.len();
        let (from, to) = (at.truncate(), goal.truncate());
        (0..n)
            .filter(|&e| areal.neighbour(e) == Some(b))
            .flat_map(|e| {
                let (p, q) = (Vec3::from(areal.vertices[e]), Vec3::from(areal.vertices[(e + 1) % n]));
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
                let inset = (edge.normalize_or_zero() * EDGE_INSET).extend(0.0);
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
    /// STAND-IN: docs/24-motion.md#not-established -- the local path and its obstacle contours
    /// are not read. A straight leg across an areal that would leave the walkable areals, as a
    /// leg across one that is not convex can, walks through that areal's triangles instead,
    /// pulled straight, `clearance` off each vertex that touches ground that is not walkable.
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
            Some(&Node::Areal(a)) => across = Some(a),
            // A unit aboard a way steps onto it first.
            Some(&Node::Vertex(w, v)) => {
                at = ways[w].points[v];
                out.push(at);
            }
            None => {}
        }
        for pair in nodes.windows(2) {
            match (pair[0], pair[1]) {
                (Node::Areal(a), Node::Areal(b)) => {
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
                (Node::Vertex(..), Node::Areal(b)) => across = Some(b),
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

    /// The corners of a walk from `from` to `to` inside areal `a`, through its triangles.
    fn through(&self, a: usize, from: Vec2, to: Vec2, keep_off: &dyn Fn(usize) -> f32) -> Vec<Vec3> {
        let own = self.of_areal[a].clone();
        let nearest = |p: Vec2| {
            own.clone().min_by(|&x, &y| {
                let d = |t: usize| self.triangles[t].nearest(p).distance_squared(p);
                d(x).total_cmp(&d(y))
            })
        };
        let (Some(first), Some(last)) = (nearest(from), nearest(to)) else { return Vec::new() };
        // An areal's triangles make a tree: there is one run between two of them.
        let mut before: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::from([first]);
        while let Some(t) = queue.pop_front() {
            if t == last {
                break;
            }
            for &u in self.triangles[t].across.iter().flatten() {
                if own.contains(&u) && u != first && !before.contains_key(&u) {
                    before.insert(u, t);
                    queue.push_back(u);
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
                    .clone()
                    .map(|t| {
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
                    .clone()
                    .flat_map(|t| {
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
        let legs = graph.route(from, goal, &[], None, 2.0, &mut fixed(0.0)).unwrap();
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
        let legs = graph.route(from, goal, &[], None, 2.0, &mut fixed(0.0)).unwrap();
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
        let route = |goal: Vec3| graph.route(from, goal, &[], None, 2.0, &mut fixed(0.5));
        assert_eq!(route(Vec3::new(25.0, 25.0, 0.0)), Err(Refusal::Goal), "on the wall");
        assert_eq!(route(Vec3::new(75.0, 25.0, 0.0)), Err(Refusal::Goal), "off the map");
        assert_eq!(route(Vec3::new(45.0, 25.0, 0.0)), Err(Refusal::NoWay), "beyond the wall");
        let on_wall = graph.route(Vec3::new(25.0, 5.0, 0.0), from, &[], None, 2.0, &mut fixed(0.5));
        assert_eq!(on_wall, Err(Refusal::Stranded));
    }

    #[test]
    fn a_walk_goes_round_an_areal_it_may_not_be_sent_to_three_along_the_edges_it_misses() {
        // A wall down the middle column, x 20 to 30, with a gap in its top row.
        let wall = [10, 11, 12, 13];
        let map = squares(5, 5, &wall);
        let graph = Graph::new(map.clone());
        let (from, goal) = (Vec3::new(5.0, 5.0, 0.0), Vec3::new(45.0, 5.0, 0.0));
        let legs = graph.route(from, goal, &[], None, 2.0, &mut fixed(0.0)).expect("a way round");
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
        let legs = graph.route(from, goal, &[], None, 2.0, &mut fixed(0.0)).unwrap();
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
            flags: vec![0x1000_0001, 0x1000_0004],
            links: vec![(0, 1)],
        };
        let ways = [
            half(Vec3::new(15.0, 25.0, 0.0), Vec3::new(24.0, 25.0, 5.0)),
            half(Vec3::new(35.0, 25.0, 0.0), Vec3::new(26.0, 25.0, 5.0)),
        ];
        let (from, goal) = (Vec3::new(5.0, 5.0, 0.0), Vec3::new(45.0, 5.0, 0.0));
        let legs = graph.route(from, goal, &ways, None, 2.0, &mut fixed(0.0)).expect("over the deck");
        let on = |p: Vec3| legs.iter().position(|&q| q == p).unwrap_or(usize::MAX);
        let order =
            [on(ways[0].points[0]), on(ways[0].points[1]), on(ways[1].points[1]), on(ways[1].points[0])];
        assert!(order.windows(2).all(|w| w[0] < w[1]) && order[3] < legs.len(), "along the deck: {legs:?}");
        assert!(keeps_off(&map, from, &legs[..=order[0]], &wall));
        assert!(keeps_off(&map, ways[1].points[0], &legs[order[3] + 1..], &wall));
        // Halves 50 apart do not join.
        let apart = [ways[0].clone(), half(Vec3::new(35.0, 25.0, 0.0), Vec3::new(74.0, 25.0, 5.0))];
        assert_eq!(graph.route(from, goal, &apart, None, 2.0, &mut fixed(0.0)), Err(Refusal::NoWay));
        // A unit on the deck sets out from its nearest vertex, or from the next once it stands no
        // farther from that; a goal over the deck is refused, the wall being under it.
        let aboard = graph.route(Vec3::new(25.5, 25.0, 5.0), goal, &ways, Some(1), 2.0, &mut fixed(0.0));
        assert_eq!(aboard.map(|l| l[0]), Ok(ways[1].points[1]));
        let past = graph.route(Vec3::new(27.0, 25.0, 5.0), goal, &ways, Some(1), 2.0, &mut fixed(0.0));
        assert_eq!(past.map(|l| l[0]), Ok(ways[1].points[0]), "the joining end is behind it");
        let onto = graph.route(from, Vec3::new(25.0, 25.0, 5.0), &ways, None, 2.0, &mut fixed(0.0));
        assert_eq!(onto, Err(Refusal::Goal));
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
