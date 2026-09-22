//! The constrained Delaunay triangulation a building's insertion builds its faces from.
//!
//! `Terrain.dll` lets a building into the landscape through a quad-edge subdivision it
//! triangulates in the Delaunay way: a site goes in and every edge around it that fails the
//! in-circle test is swapped (`0x10003660`, against `0x10002ae0`), and an edge laid in as a
//! constraint is flagged in its record's `+0x60` and never swapped again (`0x10004eb0`).
//! Each side of a constraint carries a label, which floods across the unconstrained edges
//! (`0x1000b710`), and the faces are handed out by label (docs/03, "Placing a building cuts
//! the landscape"). So the basement band, the faces between the outer contour and the inner
//! one, is the constrained Delaunay triangulation of that ring, its two contours its only
//! constraints.
//!
//! This is written from that description, not from the routine: the sites go in one by one
//! (Bowyer and Watson), each ring edge is then recovered by swapping the edges that cross it,
//! and every edge that is not a ring edge is swapped until it passes the in-circle test.

/// The constrained Delaunay triangulation of the ring between `outer` and `inner`, both closed
/// rings without their closing repeat, the inner lying inside the outer. Each triangle is three
/// indices into `outer` followed by `inner` (so `outer.len() + k` is `inner[k]`), wound
/// counter-clockwise across the ground.
///
/// Either ring may be wound either way. An empty `inner` triangulates the outer ring whole.
pub fn annulus(outer: &[[f32; 2]], inner: &[[f32; 2]]) -> Vec<[usize; 3]> {
    let n = outer.len();
    if n < 3 {
        return Vec::new();
    }
    let all: Vec<[f64; 2]> = outer.iter().chain(inner).map(|p| [f64::from(p[0]), f64::from(p[1])]).collect();
    // Worked in a frame where the rings span about a unit, so one tolerance serves every
    // building from a bunker to a factory.
    let (lo, hi) = all.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
        ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
    });
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-9) / 2.0;
    let mut points: Vec<[f64; 2]> =
        all.iter().map(|p| [(p[0] - mid[0]) / span, (p[1] - mid[1]) / span]).collect();
    let sites = points.len();
    // A triangle holding every site with room to spare, taken away at the end.
    points.extend([[-20.0, -20.0], [20.0, -20.0], [0.0, 20.0]]);
    let mut mesh = Mesh { points, triangles: vec![[sites, sites + 1, sites + 2]] };
    for site in 0..sites {
        mesh.insert(site);
    }
    let ring = |start: usize, len: usize| (0..len).map(move |k| (start + k, start + (k + 1) % len));
    let constraints: Vec<(usize, usize)> = ring(0, n)
        .chain(if inner.len() >= 3 { Some(ring(n, inner.len())) } else { None }.into_iter().flatten())
        .collect();
    for &(a, b) in &constraints {
        mesh.recover(a, b);
    }
    mesh.legalise(&constraints);
    let outer_ring: Vec<[f64; 2]> = mesh.points[..n].to_vec();
    let inner_ring: Vec<[f64; 2]> = mesh.points[n..sites].to_vec();
    mesh.triangles
        .iter()
        .filter(|t| t.iter().all(|&v| v < sites))
        .filter(|t| orient(mesh.points[t[0]], mesh.points[t[1]], mesh.points[t[2]]) > EPSILON)
        .filter(|t| {
            let c = centroid(&mesh.points, **t);
            inside(&outer_ring, c) && (inner_ring.len() < 3 || !inside(&inner_ring, c))
        })
        .copied()
        .collect()
}

/// Below this the normalised frame calls an area or a circle test zero.
const EPSILON: f64 = 1e-12;

/// A bound on the swaps any one step may take, so a degenerate ring cannot hang the load.
const MAX_SWAPS: usize = 100_000;

struct Mesh {
    points: Vec<[f64; 2]>,
    /// Counter-clockwise.
    triangles: Vec<[usize; 3]>,
}

/// Twice the signed area of `abc`: positive when it turns counter-clockwise.
fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Positive when `d` lies inside the circle through the counter-clockwise `abc`.
fn in_circle(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> f64 {
    let row = |p: [f64; 2]| {
        let (x, y) = (p[0] - d[0], p[1] - d[1]);
        [x, y, x * x + y * y]
    };
    let (a, b, c) = (row(a), row(b), row(c));
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

/// Whether segments `ab` and `cd` cross at a point inside both.
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let (d1, d2) = (orient(a, b, c), orient(a, b, d));
    let (d3, d4) = (orient(c, d, a), orient(c, d, b));
    ((d1 > EPSILON && d2 < -EPSILON) || (d1 < -EPSILON && d2 > EPSILON))
        && ((d3 > EPSILON && d4 < -EPSILON) || (d3 < -EPSILON && d4 > EPSILON))
}

fn centroid(points: &[[f64; 2]], t: [usize; 3]) -> [f64; 2] {
    let [a, b, c] = t.map(|v| points[v]);
    [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0]
}

/// Whether `p` lies inside the closed ring, by the crossing test.
fn inside(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut hit = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
            hit = !hit;
        }
    }
    hit
}

impl Mesh {
    fn at(&self, v: usize) -> [f64; 2] {
        self.points[v]
    }

    /// Put site `p` in: take away every triangle whose circle holds it, and fan the hole from it.
    fn insert(&mut self, p: usize) {
        let at = self.at(p);
        let mut bad: Vec<bool> = self
            .triangles
            .iter()
            .map(|t| in_circle(self.at(t[0]), self.at(t[1]), self.at(t[2]), at) > EPSILON)
            .collect();
        // The triangle the site stands in is always taken, whatever the circle test says of a
        // site on its edge.
        if let Some(k) = self.triangles.iter().position(|t| {
            let [a, b, c] = t.map(|v| self.at(v));
            orient(a, b, at) >= -EPSILON && orient(b, c, at) >= -EPSILON && orient(c, a, at) >= -EPSILON
        }) {
            bad[k] = true;
        }
        let mut rim = Vec::new();
        for (t, _) in self.triangles.iter().zip(&bad).filter(|(_, b)| **b) {
            for e in 0..3 {
                let (a, b) = (t[e], t[(e + 1) % 3]);
                let shared = self
                    .triangles
                    .iter()
                    .zip(&bad)
                    .any(|(u, ub)| *ub && (0..3).any(|f| u[f] == b && u[(f + 1) % 3] == a));
                if !shared {
                    rim.push((a, b));
                }
            }
        }
        let mut kept: Vec<[usize; 3]> =
            self.triangles.iter().zip(&bad).filter(|(_, b)| !**b).map(|(t, _)| *t).collect();
        kept.extend(rim.into_iter().map(|(a, b)| [a, b, p]));
        self.triangles = kept;
    }

    /// The triangle holding directed edge `a → b`, and its third corner.
    fn beside(&self, a: usize, b: usize) -> Option<(usize, usize)> {
        self.triangles.iter().enumerate().find_map(|(k, t)| {
            (0..3).find(|&e| t[e] == a && t[(e + 1) % 3] == b).map(|e| (k, t[(e + 2) % 3]))
        })
    }

    fn has_edge(&self, a: usize, b: usize) -> bool {
        self.beside(a, b).is_some() || self.beside(b, a).is_some()
    }

    /// Swap the diagonal `a–b` of the quad its two triangles make, when that quad is convex;
    /// the new diagonal, or `None`.
    fn swap(&mut self, a: usize, b: usize) -> Option<(usize, usize)> {
        let (t1, c) = self.beside(a, b)?;
        let (t2, d) = self.beside(b, a)?;
        let (pa, pb, pc, pd) = (self.at(a), self.at(b), self.at(c), self.at(d));
        if !cross(pa, pb, pc, pd) {
            return None;
        }
        self.triangles[t1] = [c, a, d];
        self.triangles[t2] = [d, b, c];
        Some((c, d))
    }

    /// Make `a–b` an edge, swapping away every edge that crosses it.
    fn recover(&mut self, a: usize, b: usize) {
        if a == b || self.has_edge(a, b) {
            return;
        }
        let (pa, pb) = (self.at(a), self.at(b));
        let mut crossing: std::collections::VecDeque<(usize, usize)> = self
            .edges()
            .into_iter()
            .filter(|&(u, v)| u != a && u != b && v != a && v != b && cross(pa, pb, self.at(u), self.at(v)))
            .collect();
        let mut swaps = 0;
        while let Some((u, v)) = crossing.pop_front() {
            swaps += 1;
            if swaps > MAX_SWAPS {
                return;
            }
            match self.swap(u, v) {
                Some((c, d)) => {
                    let new = (c.min(d), c.max(d));
                    if c != a && c != b && d != a && d != b && cross(pa, pb, self.at(c), self.at(d)) {
                        crossing.push_back(new);
                    }
                }
                None => crossing.push_back((u, v)),
            }
        }
    }

    /// Every undirected edge once, `(low, high)`.
    fn edges(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = self
            .triangles
            .iter()
            .flat_map(|t| (0..3).map(move |e| (t[e].min(t[(e + 1) % 3]), t[e].max(t[(e + 1) % 3]))))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Swap every edge but the constraints until each passes the in-circle test.
    fn legalise(&mut self, constraints: &[(usize, usize)]) {
        let fixed =
            |u: usize, v: usize| constraints.iter().any(|&(a, b)| (a == u && b == v) || (a == v && b == u));
        for _ in 0..MAX_SWAPS {
            let mut swapped = false;
            for (u, v) in self.edges() {
                if fixed(u, v) {
                    continue;
                }
                let (Some((_, c)), Some((_, d))) = (self.beside(u, v), self.beside(v, u)) else { continue };
                if in_circle(self.at(u), self.at(v), self.at(c), self.at(d)) > EPSILON
                    && self.swap(u, v).is_some()
                {
                    swapped = true;
                }
            }
            if !swapped {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(points: &[[f32; 2]], t: [usize; 3]) -> f64 {
        let [a, b, c] = t.map(|v| points[v].map(f64::from));
        orient(a, b, c) / 2.0
    }

    fn ring_area(ring: &[[f32; 2]]) -> f64 {
        (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1])
            })
            .sum::<f64>()
            .abs()
            / 2.0
    }

    fn square(r: f32) -> Vec<[f32; 2]> {
        vec![[-r, -r], [r, -r], [r, r], [-r, r]]
    }

    /// The triangles tile the ring exactly: every one turns counter-clockwise, and their areas
    /// sum to the outer ring's less the inner's, so none overlaps and none is missing.
    fn tiles(outer: &[[f32; 2]], inner: &[[f32; 2]]) -> Vec<[usize; 3]> {
        let tris = annulus(outer, inner);
        let points: Vec<[f32; 2]> = outer.iter().chain(inner).copied().collect();
        assert!(tris.iter().all(|&t| area(&points, t) > 0.0), "{tris:?}");
        let sum: f64 = tris.iter().map(|&t| area(&points, t)).sum();
        let want = ring_area(outer) - if inner.len() >= 3 { ring_area(inner) } else { 0.0 };
        assert!((sum - want).abs() < want * 1e-6, "{sum} against {want}");
        tris
    }

    #[test]
    fn a_square_ring_is_eight_triangles() {
        let tris = tiles(&square(20.0), &square(10.0));
        assert_eq!(tris.len(), 8);
    }

    #[test]
    fn either_winding_gives_the_same_ring() {
        let mut backwards = square(20.0);
        backwards.reverse();
        assert_eq!(tiles(&backwards, &square(10.0)).len(), 8);
    }

    #[test]
    fn a_concave_outer_ring_keeps_to_the_ring() {
        // An L-shaped outer ring round a square: a stitch across the notch would cover ground
        // outside the contour.
        let outer = vec![[-20.0, -20.0], [20.0, -20.0], [20.0, 0.0], [0.0, 0.0], [0.0, 20.0], [-20.0, 20.0]];
        let inner = vec![[-15.0, -15.0], [-5.0, -15.0], [-5.0, -5.0], [-15.0, -5.0]];
        tiles(&outer, &inner);
    }

    #[test]
    fn corners_along_a_straight_edge_are_kept() {
        // The outer contour cut where the landscape's edges cross it: many corners in a line.
        let mut outer = Vec::new();
        for k in 0..10 {
            outer.push([-20.0 + 4.0 * k as f32, -20.0]);
        }
        outer.extend([[20.0, -20.0], [20.0, 20.0], [-20.0, 20.0]]);
        let tris = tiles(&outer, &square(5.0));
        // Every corner of either ring is a corner of some triangle.
        let used: std::collections::BTreeSet<usize> = tris.iter().flatten().copied().collect();
        assert_eq!(used.len(), outer.len() + 4);
    }

    #[test]
    fn a_ring_edge_is_never_crossed() {
        // A long thin inner ring that a Delaunay triangulation of the corners alone would cut
        // across: the constraints hold.
        let outer = square(30.0);
        let inner = vec![[-25.0, -1.0], [25.0, -1.0], [25.0, 1.0], [-25.0, 1.0]];
        let tris = tiles(&outer, &inner);
        let points: Vec<[f32; 2]> = outer.iter().chain(&inner).copied().collect();
        for t in &tris {
            let c =
                t.iter().fold([0.0f32; 2], |c, &v| [c[0] + points[v][0] / 3.0, c[1] + points[v][1] / 3.0]);
            assert!(c[1].abs() > 1.0 || c[0].abs() > 25.0, "a face over the inner ring: {t:?}");
        }
    }

    #[test]
    fn the_ring_is_delaunay_where_nothing_constrains_it() {
        // No unconstrained edge fails the in-circle test.
        let outer = vec![[-30.0, -10.0], [0.0, -25.0], [30.0, -10.0], [25.0, 20.0], [-25.0, 22.0]];
        let inner = vec![[-8.0, -4.0], [7.0, -5.0], [9.0, 6.0], [-6.0, 5.0]];
        let tris = tiles(&outer, &inner);
        let points: Vec<[f64; 2]> = outer.iter().chain(&inner).map(|p| p.map(f64::from)).collect();
        let n = outer.len();
        let ring_edge = |a: usize, b: usize| {
            let on = |s: usize, len: usize, u: usize, v: usize| {
                u >= s
                    && v >= s
                    && u < s + len
                    && v < s + len
                    && ((u - s + 1) % len == v - s || (v - s + 1) % len == u - s)
            };
            on(0, n, a, b) || on(n, inner.len(), a, b)
        };
        for t in &tris {
            for u in &tris {
                for e in 0..3 {
                    let (a, b) = (t[e], t[(e + 1) % 3]);
                    if ring_edge(a, b) {
                        continue;
                    }
                    if let Some(f) = (0..3).find(|&f| u[f] == b && u[(f + 1) % 3] == a) {
                        let d = u[(f + 2) % 3];
                        let s = in_circle(points[t[0]], points[t[1]], points[t[2]], points[d]);
                        assert!(s <= 1e-6, "{t:?} and {u:?}: {s}");
                    }
                }
            }
        }
    }
}
