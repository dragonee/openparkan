//! A building's footing: the band of ground between its two `.bas` rings that the engine
//! stitches in when the building is let into the landscape.
//!
//! See `docs/03-terrain.md`, "Placing a building cuts the landscape". The insertion deletes
//! every landscape face inside the outer contour, triangulates what is left of the cut faces
//! into patch faces, and fills the ring between the outer contour and the inner one with
//! **basement** faces. Those are what joins the building's own floor to the ground around
//! it; without them the landscape stops at the contour and the sky shows through.
//!
//! What a basement face wears is `Terrain.dll`'s own (*read*). The basement builder
//! (`0x1000cd40`, reached from the insertion at `0x10011ce1`) writes every face it makes the
//! same way (`0x1000d9c4`): **layer-1 slot 0, no second layer**, flags `0x300`, surface 0 and
//! its own normal from its corners. Each corner takes its layer-1 UV from its own world x and
//! y, times `0.066` (`0x1009a214`, packed at `0x1000d4be`), a blend of 1 (`0x1000d1fd`) and a
//! zero vertex normal (`0x1000d106`, see [`facet`]).
//!
//! Slot 0 is where every map keeps its footing material: `Land1.wea` names it `B_S0` on 32 of
//! the 33 maps (`B_MTP_01` on FINAL), whose texture is `B_FOUND`, a grey foundation slab —
//! and **no shipped landscape face names slot 0**, on any map. The slot is there for the
//! faces the engine makes, which is why a building in the game stands on a band of stone.

use glam::Vec3;
use parkan_formats::basement;
use parkan_formats::landmesh::{LandMesh, NO_TEXTURE, UV_FIXED_POINT_SCALE};
use parkan_formats::mission::{KIND_BUILDING, Mission};

use crate::assembly::Assembly;
use crate::terrain::Vertex;

/// A building's two `.bas` rings, the inner one then the outer.
pub type Plan = (Vec<[f32; 3]>, Vec<[f32; 3]>);

/// The layer-1 slot a footing wears: every map keeps its foundation material there
/// (`Terrain.dll:0x1000d9c4`).
pub const FOUNDATION_LAYER: u8 = 0;

/// How far a footing corner's layer-1 UV travels across a world unit.
///
/// The engine multiplies the corner's world x and y by `0.066` (`0x1009a214`) and packs the
/// result at 1024 to the UV unit (`0x1009a1f4`), while the streams are read here at
/// [`UV_FIXED_POINT_SCALE`] to the unit: the foundation tiles every 3.8 world units.
pub const FOUNDATION_PER_UNIT: f32 = 0.066 * (1024.0 / UV_FIXED_POINT_SCALE);

/// How far apart a contour is sampled onto the landscape, in world units.
///
/// STAND-IN: docs/03-terrain.md#for-an-engine -- the insertion cuts the contour against
/// every landscape face it crosses and takes a corner at each crossing; this walks the
/// contour at a fixed step instead, which follows the ground as closely wherever the
/// landscape's faces are no smaller than the step.
pub const CONTOUR_STEP: f32 = 4.0;

/// How far the apron's rim hangs below the landscape at its own foot.
///
/// The rim is already buried, so this wall is too. It shows only where the straight rim runs
/// over a dip in the ground, and there it puts ground behind the gap instead of sky.
pub const SKIRT: f32 = 1.5;

/// How far past its contour the band's apron runs, and how far under the landscape there.
///
/// The band's own edge meets the landscape **on the contour**, at the height of the ground
/// under it, so the two are flush. The landscape is cut to a mask a texel at a time, though,
/// so its edge steps in and out of the contour by a fraction of a unit and would leave slivers
/// of sky along it. The apron runs on past the contour, under the ground that is left, and
/// covers them; buried, it is never seen. Its rim is sunk below the landscape at both ends of
/// the apron, so it stays under the ground across it however the ground runs.
pub const APRON: f32 = 2.0;
pub const APRON_SINK: f32 = 0.5;

/// One building's footing in the world.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Footing {
    /// The outer `.bas` ring across the ground, walked at [`CONTOUR_STEP`]. The landscape is
    /// cut away inside it.
    pub outline: Vec<[f32; 2]>,
    /// The band between the rings, each face three corners counter-clockwise from above.
    pub faces: Vec<Facet>,
    /// The apron past the contour and the wall under its rim, both buried under the landscape
    /// that is left. Drawn, but no ground: nothing stands outside the contour.
    pub apron: Vec<Facet>,
}

/// One face of a footing: its corners as the ground draws them, and the map's layer-1 slot
/// for its foundation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Facet {
    pub corners: [Vertex; 3],
    pub tex1: u8,
    /// Always `landmesh::NO_TEXTURE`: a footing has no second layer.
    pub tex2: u8,
}

impl Facet {
    /// The face's own normal, from its corners, as `Land.msh` stores a face normal.
    pub fn normal(&self) -> Vec3 {
        let [a, b, c] = self.corners.map(|v| Vec3::from_array(v.position));
        (b - a).cross(c - a).normalize_or_zero()
    }

    /// Its corners in the world.
    pub fn triangle(&self) -> [Vec3; 3] {
        self.corners.map(|v| Vec3::from_array(v.position))
    }
}

/// The height of the highest level-0 face over `(x, y)`.
///
/// The same barycentric query the ground search makes, over the faces of the map's first
/// level of detail. A water surface is passed over, so a contour across a lake meets its bed.
fn under(land: &LandMesh, x: f32, y: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for face in &land.faces[land.lod_faces(0)] {
        if face.is_water() {
            continue;
        }
        let v = face.vertices.map(usize::from);
        let [a, b, c] = v.map(|i| land.positions[i]);
        if x < a[0].min(b[0]).min(c[0])
            || x > a[0].max(b[0]).max(c[0])
            || y < a[1].min(b[1]).min(c[1])
            || y > a[1].max(b[1]).max(c[1])
        {
            continue;
        }
        let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
        if den.abs() < 1e-9 {
            continue;
        }
        let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den;
        let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den;
        let l3 = 1.0 - l1 - l2;
        if l1 < -1e-5 || l2 < -1e-5 || l3 < -1e-5 {
            continue;
        }
        let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
        if best.is_none_or(|b| z > b) {
            best = Some(z);
        }
    }
    best
}

/// A corner of the footing, its foundation laid over the world under it.
fn corner(p: [f32; 3]) -> Vertex {
    Vertex {
        position: p,
        normal: [0.0; 3],
        uv1: [p[0] * FOUNDATION_PER_UNIT, p[1] * FOUNDATION_PER_UNIT],
        uv2: [0.0; 2],
        blend: 1.0,
    }
}

/// The band's edge pushed out by [`APRON`] and sunk under the landscape: the apron's rim,
/// corner for corner with `edge`.
///
/// A corner goes out along the bisector of its two edges' outward normals, and its z is the
/// lower of the ground at the contour and at the rim, less [`APRON_SINK`], so the rim stays
/// under the ground all the way across the apron.
fn rim(land: &LandMesh, edge: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let n = edge.len();
    let normal = |i: usize| {
        let (a, b) = (edge[i], edge[(i + 1) % n]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy).max(1e-6);
        // The ring is wound counter-clockwise, so an edge's outward side is to its right.
        [dy / len, -dx / len]
    };
    (0..n)
        .map(|i| {
            let (p, q) = (normal((i + n - 1) % n), normal(i));
            let out = [p[0] + q[0], p[1] + q[1]];
            let len = out[0].hypot(out[1]).max(1e-6);
            let at = [edge[i][0] + out[0] / len * APRON, edge[i][1] + out[1] / len * APRON];
            let here = edge[i][2];
            let there = under(land, at[0], at[1]).unwrap_or(here);
            [at[0], at[1], here.min(there) - APRON_SINK]
        })
        .collect()
}

/// A footing's face over three corners, lit by its own normal.
///
/// STAND-IN: docs/03-terrain.md#for-an-engine -- the engine writes a **zero** normal on every
/// footing corner (`0x1000d106`). That is not carried over: the ground here takes its light
/// from its corners' normals, and a zero normal would leave the band black, where the game's
/// own footings are lit like the ground around them. Each corner takes the face's normal
/// instead.
fn facet(corners: [Vertex; 3]) -> Facet {
    let mut facet = Facet { corners, tex1: FOUNDATION_LAYER, tex2: NO_TEXTURE };
    let normal = facet.normal().to_array();
    for corner in &mut facet.corners {
        corner.normal = normal;
    }
    facet
}

/// A ring's corners placed by a building's position and its turn about z.
fn place(ring: &[[f32; 3]], at: [f32; 3], yaw: f32) -> Vec<[f32; 3]> {
    let (s, c) = yaw.sin_cos();
    ring.iter().map(|p| [at[0] + p[0] * c - p[1] * s, at[1] + p[0] * s + p[1] * c, at[2] + p[2]]).collect()
}

/// Twice the area a ring encloses, signed: positive where it is wound counter-clockwise.
fn winding(ring: &[[f32; 3]]) -> f32 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum()
}

/// A closed ring walked counter-clockwise at no more than [`CONTOUR_STEP`] between corners,
/// its own corners kept.
fn walked(ring: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut ring = ring.to_vec();
    if winding(&ring) < 0.0 {
        ring.reverse();
    }
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        let steps = ((b[0] - a[0]).hypot(b[1] - a[1]) / CONTOUR_STEP).ceil().max(1.0) as usize;
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            out.push([0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * t));
        }
    }
    out
}

/// The middle of a ring's corners.
fn middle(ring: &[[f32; 3]]) -> [f32; 2] {
    let n = ring.len().max(1) as f32;
    let sum = ring.iter().fold([0.0, 0.0], |m, p| [m[0] + p[0], m[1] + p[1]]);
    [sum[0] / n, sum[1] / n]
}

/// The band between `inner`, at the building's base, and `outer`, the contour dropped onto
/// the landscape. Both are wound counter-clockwise and carry their own z.
///
/// STAND-IN: docs/03-terrain.md#for-an-engine -- how `Terrain.dll` triangulates the band
/// between its two rings is not read. The rings are stitched here: the two are walked from
/// the corners that face the same way, and each face is closed by whichever ring is the
/// further behind in its own walk, which joins rings of different corner counts and leaves
/// no corner out.
fn band(inner: &[[f32; 3]], outer: &[[f32; 3]]) -> Vec<Facet> {
    let (n, m) = (inner.len(), outer.len());
    if n == 0 || m == 0 {
        return Vec::new();
    }
    // The two walks start where the rings face the same way about the outer ring's middle.
    let mid = middle(outer);
    let turn = |p: &[f32; 3]| (p[1] - mid[1]).atan2(p[0] - mid[0]);
    let zero = turn(&inner[0]);
    let start = outer
        .iter()
        .enumerate()
        .min_by(|a, b| {
            let gap =
                |p: &[f32; 3]| (turn(p) - zero).abs().min(std::f32::consts::TAU - (turn(p) - zero).abs());
            gap(a.1).total_cmp(&gap(b.1))
        })
        .map_or(0, |(i, _)| i);
    let built: Vec<Vertex> = inner.iter().chain(outer).map(|p| corner(*p)).collect();
    let at_inner = |k: usize| built[k % n];
    let at_outer = |k: usize| built[n + (start + k) % m];
    let mut faces = Vec::new();
    let (mut i, mut o) = (0usize, 0usize);
    while i < n || o < m {
        let ahead_i = (i + 1) as f32 / n as f32;
        let ahead_o = (o + 1) as f32 / m as f32;
        faces.push(facet(if o >= m || (i < n && ahead_i <= ahead_o) {
            i += 1;
            [at_outer(o), at_inner(i), at_inner(i - 1)]
        } else {
            o += 1;
            [at_outer(o - 1), at_outer(o), at_inner(i)]
        }));
    }
    faces
}

/// The apron: the ring between the contour and its rim, corner for corner, and the wall
/// hanging under the rim. Both are buried; they only ever fill a gap.
fn apron(edge: &[[f32; 3]], rim: &[[f32; 3]]) -> Vec<Facet> {
    let n = edge.len().min(rim.len());
    let mut out = Vec::new();
    let at = |ring: &[[f32; 3]], k: usize| corner(ring[k % n]);
    for k in 0..n {
        let (a, b, c, d) = (at(edge, k), at(edge, k + 1), at(rim, k), at(rim, k + 1));
        // The ring from the contour out to the rim, then the wall dropping under it.
        out.push(facet([a, c, d]));
        out.push(facet([a, d, b]));
        let drop = |v: Vertex| {
            let mut v = v;
            v.position[2] -= SKIRT;
            v
        };
        out.push(facet([c, drop(d), d]));
        out.push(facet([c, drop(c), drop(d)]));
    }
    out
}

/// One building's footing from its two rings, already placed in the world.
///
/// The band runs from the inner ring, at the building's base, out to the outer contour, whose
/// corners drop onto the landscape, so the two meet flush there. Past the contour the apron
/// carries on under the ground.
pub fn footing(land: &LandMesh, inner: &[[f32; 3]], outer: &[[f32; 3]]) -> Footing {
    let inner = walked(inner);
    let outer = walked(outer);
    // Off the mesh a contour corner has no ground to drop onto, and holds the base instead.
    let base = inner.first().map_or(0.0, |p| p[2]);
    let edge: Vec<[f32; 3]> =
        outer.iter().map(|p| [p[0], p[1], under(land, p[0], p[1]).unwrap_or(base)]).collect();
    let rim = rim(land, &edge);
    Footing {
        outline: outer.iter().map(|p| [p[0], p[1]]).collect(),
        faces: band(&inner, &edge),
        apron: apron(&edge, &rim),
    }
}

/// Every building the mission places, as its footing.
pub fn footings(assembly: &mut Assembly, mission: &Mission, land: &LandMesh) -> Vec<Footing> {
    rings(assembly, mission).iter().map(|(i, o)| footing(land, i, o)).collect()
}

/// A footing as the ground search takes it: what it cuts, and the band that is ground there.
pub fn cut(footing: Footing) -> parkan_sim::ground::Cut {
    let faces = footing.faces.iter().map(Facet::triangle).collect();
    parkan_sim::ground::Cut::with_faces(footing.outline, faces)
}

/// Every placed building's `.bas` rings in the world, the inner one then the outer
/// (`IBasement` slots 4 and 3, docs/03).
pub fn rings(assembly: &mut Assembly, mission: &Mission) -> Vec<Plan> {
    mission
        .objects
        .iter()
        .filter(|o| o.kind == KIND_BUILDING)
        .filter_map(|o| {
            let (inner, outer) = plan(assembly, &o.path)?;
            Some((place(&inner, o.position, o.rotation), place(&outer, o.position, o.rotation)))
        })
        .collect()
}

/// A building's two `.bas` rings in its own frame, the inner one then the outer.
pub fn plan(assembly: &mut Assembly, path: &str) -> Option<Plan> {
    let parts = assembly.parts(KIND_BUILDING, path);
    let root = parts.iter().find(|p| p.host == -1)?;
    let slot = assembly.library.record_slot(assembly.library.get(&root.record), "bas", 0)?;
    let data = assembly.archive(&slot.library)?.read_name(&slot.member).ok()?.to_vec();
    let rings = basement::parse(&data, &slot.member).ok()?;
    Some((rings.first()?.points.clone(), rings.get(1)?.points.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::landmesh::{Cell, Face, NO_TEXTURE};

    /// One flat 200 by 200 quad at z 0, wearing layer pair (1, 2).
    fn flat() -> LandMesh {
        let positions =
            vec![[-100.0, -100.0, 0.0], [100.0, -100.0, 0.0], [100.0, 100.0, 0.0], [-100.0, 100.0, 0.0]];
        let face = |vertices: [u16; 3]| Face {
            vertices,
            adjacency: [0xFFFF; 3],
            flags: 0,
            surface: 0,
            tex1: 1,
            tex2: 2,
            normal: [0.0, 0.0, 1.0],
            edge_twins: 0,
        };
        LandMesh {
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uv1: vec![[0.0; 2]; 4],
            uv2: vec![[0.0; 2]; 4],
            blend: vec![1.0; 4],
            positions,
            faces: vec![face([0, 1, 2]), face([0, 2, 3])],
            cells: vec![Cell { first: 0, count: 2 }, Cell { first: 2, count: 0 }],
            grid: [1, 1],
            layer1: vec!["one".into(), "two".into()],
            layer2: vec!["a".into(), "b".into(), "c".into()],
        }
    }

    fn square(r: f32, z: f32) -> Vec<[f32; 3]> {
        vec![[-r, -r, z], [r, -r, z], [r, r, z], [-r, r, z]]
    }

    #[test]
    fn a_footing_joins_the_buildings_base_to_the_ground_around_it() {
        let land = flat();
        let f = footing(&land, &square(10.0, -4.0), &square(20.0, 0.0));
        assert!(!f.faces.is_empty() && !f.outline.is_empty());
        // Every face wears the map's foundation slot alone and faces up, and its corners
        // stand either on the ground at the contour or at the building's base.
        for face in &f.faces {
            assert_eq!((face.tex1, face.tex2), (FOUNDATION_LAYER, NO_TEXTURE));
            assert!(face.normal().z > 0.5, "{:?}", face.normal());
            for c in face.corners {
                assert!(c.position[2] == 0.0 || c.position[2] == -4.0, "{c:?}");
                assert_eq!(c.blend, 1.0);
            }
        }
        // The corners on the ground ring the outline, the rest the building's base.
        let base = f.faces.iter().flat_map(|f| f.corners).filter(|c| c.position[2] == -4.0).count();
        assert!(base > 0);
    }

    #[test]
    fn the_apron_runs_on_past_the_contour_and_under_the_ground_there() {
        let land = flat();
        let f = footing(&land, &square(10.0, -4.0), &square(20.0, 0.0));
        let rim: Vec<Vertex> =
            f.apron.iter().flat_map(|x| x.corners).filter(|c| c.position[2] == -APRON_SINK).collect();
        assert!(!rim.is_empty());
        // Every rim corner lies outside the contour the landscape is cut to, and under the
        // ground it is cut from, so no sliver of the cut's own edge shows sky.
        for c in &rim {
            assert!(!basement::contains(&f.outline, c.position[0], c.position[1]), "{c:?}");
            assert!(c.position[2] < under(&land, c.position[0], c.position[1]).unwrap(), "{c:?}");
        }
        // A corner along an edge goes straight out from it; the square's own corners go out
        // along the bisector, so they reach less far.
        let far = rim.iter().map(|c| c.position[0].abs()).fold(0.0, f32::max);
        assert!((far - (20.0 + APRON)).abs() < 1e-3, "{far}");
    }

    #[test]
    fn the_foundation_is_laid_over_the_world_under_it() {
        let land = flat();
        let f = footing(&land, &square(10.0, 0.0), &square(20.0, 0.0));
        let c = f.faces[0].corners[0];
        assert!((c.uv1[0] - c.position[0] * FOUNDATION_PER_UNIT).abs() < 1e-6);
        assert!((c.uv1[1] - c.position[1] * FOUNDATION_PER_UNIT).abs() < 1e-6);
        assert!((FOUNDATION_PER_UNIT - 0.264).abs() < 1e-6, "3.8 world units a tile");
    }

    #[test]
    fn a_ring_wound_either_way_gives_the_same_band() {
        let land = flat();
        let mut backwards = square(20.0, 0.0);
        backwards.reverse();
        let a = footing(&land, &square(10.0, -4.0), &square(20.0, 0.0));
        let b = footing(&land, &square(10.0, -4.0), &backwards);
        assert_eq!(a.faces.len(), b.faces.len());
        assert!(b.faces.iter().all(|f| f.normal().z > 0.5));
    }

    #[test]
    fn the_apron_runs_past_the_contour_and_hangs_a_wall_under_its_rim() {
        let land = flat();
        let f = footing(&land, &square(10.0, 0.0), &square(20.0, 0.0));
        assert_eq!(f.apron.len(), f.outline.len() * 4);
        let (level, walls): (Vec<&Facet>, Vec<&Facet>) =
            f.apron.iter().partition(|x| x.normal().z.abs() > 0.5);
        // Two faces of each four carry the contour's height out to the rim, sunk under the
        // ground; two hang a wall under the rim.
        assert_eq!((level.len(), walls.len()), (f.outline.len() * 2, f.outline.len() * 2));
        for face in walls {
            let n = face.normal();
            let mid = face.triangle().iter().sum::<Vec3>() / 3.0;
            assert!(n.truncate().dot(mid.truncate()) > 0.0, "facing out: {n:?} at {mid:?}");
            assert!(face.corners.iter().any(|c| c.position[2] == -APRON_SINK - SKIRT));
        }
    }
}
