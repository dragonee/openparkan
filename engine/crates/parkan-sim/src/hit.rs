//! The hit test: `docs/26-damage.md`, "The hit test", and `engine/design/r1-hit.md`.
//!
//! Every test is a segment or a swept sphere over the frame, so nothing tunnels:
//! a round's segment against the ground, against the map box, and against the
//! level-0 triangles of every node of an object whose swept sphere it meets.

use glam::Vec3;
use parkan_formats::mesh::{Mesh, NO_SLOT};
use parkan_formats::pose::Pose;

/// A round passes through triangles flagged 4 or 32 (`Control.dll:0x1001d9fa`).
pub const ROUND_SKIPS_FACE: u16 = 0x24;
/// The sight ray passes through no triangle (`Control.dll:0x1002adc0`).
pub const SIGHT_SKIPS_FACE: u16 = 0;

/// Where a segment strikes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub point: Vec3,
    /// Squared distance from the segment's start.
    pub d2: f32,
    /// The mesh node struck, or `None` on the ground.
    pub node: Option<usize>,
    /// The triangle, or the ground face; `None` on a building's footing, whose faces belong
    /// to no mesh (`parkan_world::basement`).
    pub triangle: Option<usize>,
    /// The struck face's normal, in world space: the vector at `+8` of what `IWorld` slot 6,
    /// `CWorld::GetWorldFace`, answers (`Terrain.dll:0x10024d70` → the face record's `+0x1c`,
    /// which `0x1001a6ec` dots with the face's first vertex for the plane's *d*). The outer
    /// camera stands 0.75 along it off whatever it meets (docs/30, "What the outer camera's
    /// line meets").
    pub normal: Vec3,
}

/// `NGI32.dll:0x10024410` (`g_FastProc` slot `0xb4`), one-sided and with no epsilon:
/// with `v = p1 − p0`, the segment crosses when `n·v < 0`, `n·p0 + d ≥ 0` and
/// `n·p1 + d < 0`. A segment that reaches a face from behind meets nothing.
pub fn plane_crossing(p0: Vec3, p1: Vec3, normal: Vec3, on_plane: Vec3) -> Option<Vec3> {
    let v = p1 - p0;
    let d = -normal.dot(on_plane);
    let nv = normal.dot(v);
    let s0 = normal.dot(p0) + d;
    let s1 = normal.dot(p1) + d;
    (nv < 0.0 && s0 >= 0.0 && s1 < 0.0).then(|| p0 + v * (s0 / -nv))
}

/// [`plane_crossing`], and where `two_sided` the same test again with the segment's ends
/// swapped (`AniMesh.dll:0x1001110c`–`0x10011163`): a batch flagged
/// [`parkan_formats::mesh::BATCH_TWO_SIDED`] is struck from either side.
pub fn plane_crossing_sided(
    p0: Vec3,
    p1: Vec3,
    normal: Vec3,
    on_plane: Vec3,
    two_sided: bool,
) -> Option<Vec3> {
    plane_crossing(p0, p1, normal, on_plane)
        .or_else(|| two_sided.then(|| plane_crossing(p1, p0, normal, on_plane)).flatten())
}

/// Whether `p`, on the triangle's plane, lies inside it (`NGI32.dll:0x10001e60`,
/// `mrnPointInPoly`): the cross product of each edge with `p` less that edge's first
/// vertex, signed by the face's **own** normal, is required non-negative on all three,
/// against exactly zero and with no epsilon, so a point on an edge is inside.
///
/// The game does the same arithmetic in a two-dimensional projection: it drops the axis
/// its two comparisons of `|n.x|`, `|n.y|`, `|n.z|` leave, and multiplies each 2D cross
/// product by the normal's component along that axis. For a point on the plane that is
/// the full dot product up to a positive factor, so the answer is the one here.
pub fn inside(p: Vec3, a: Vec3, b: Vec3, c: Vec3, normal: Vec3) -> bool {
    (b - a).cross(p - a).dot(normal) >= 0.0
        && (c - b).cross(p - b).dot(normal) >= 0.0
        && (a - c).cross(p - c).dot(normal) >= 0.0
}

fn vec(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

fn arr(v: Vec3) -> [f64; 3] {
    [f64::from(v.x), f64::from(v.y), f64::from(v.z)]
}

/// A segment through a posed mesh (`AniMesh.dll:0x10013ef0`): every node once, its
/// level-0 slot of variant 0 in the node's frame, `world[i]` placing node `i`. The
/// nearest strike by squared distance from `p0` wins.
///
/// `scale` is the object's uniform scale: the poses carry it in their translations,
/// and the node frames are scaled by it, as `SetScale` recomposes them (docs/04).
/// A round's query passes through the triangles flagged [`ROUND_SKIPS_FACE`].
pub fn segment_mesh(mesh: &Mesh, world: &[Pose], scale: f32, p0: Vec3, p1: Vec3) -> Option<Strike> {
    segment_mesh_passing(mesh, world, scale, p0, p1, ROUND_SKIPS_FACE)
}

/// [`segment_mesh`], passing through the triangles whose flags meet `passes`.
pub fn segment_mesh_passing(
    mesh: &Mesh,
    world: &[Pose],
    scale: f32,
    p0: Vec3,
    p1: Vec3,
    passes: u16,
) -> Option<Strike> {
    segment_mesh_slots(mesh, world, scale, p0, p1, passes, |i| mesh.nodes.get(i).map(|n| n.slot_index[0]))
}

/// [`segment_mesh_passing`] through the slot `slot_of` names for each node, or none: the
/// level-0 slot of the variant its stage draws, and nothing of a hidden node
/// (`AniMesh.dll:0x10010c33` → `0x100124d0`, `0x100106d0`).
pub fn segment_mesh_slots(
    mesh: &Mesh,
    world: &[Pose],
    scale: f32,
    p0: Vec3,
    p1: Vec3,
    passes: u16,
    slot_of: impl Fn(usize) -> Option<u16>,
) -> Option<Strike> {
    segment_mesh_skipping(mesh, world, scale, p0, p1, passes, slot_of, |_| false)
}

/// [`segment_mesh_slots`], passing every triangle `skip` names whatever its flags.
///
/// STAND-IN: docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured --
/// the mover's face query drops a batch by a word that is not traced, and the stand-in for
/// it is the portal materials; a round's query builds its filter the same way
/// (`Control.dll:0x1001d9fa`), so it passes the same faces. Without it a shot at a building's
/// door strikes the black doorway quad a step in front of it and no door opens.
#[allow(clippy::too_many_arguments)]
pub fn segment_mesh_skipping(
    mesh: &Mesh,
    world: &[Pose],
    scale: f32,
    p0: Vec3,
    p1: Vec3,
    passes: u16,
    slot_of: impl Fn(usize) -> Option<u16>,
    skip: impl Fn(usize) -> bool,
) -> Option<Strike> {
    let mut best = (p1 - p0).length_squared();
    let mut out = None;
    for i in 0..mesh.nodes.len() {
        let Some(slot) = slot_of(i).filter(|&s| s != NO_SLOT) else { continue };
        let (Some(pose), Some(s)) = (world.get(i), mesh.slots.get(usize::from(slot))) else { continue };
        let back = pose.invert();
        let (q0, q1) = (vec(back.apply(arr(p0))) / scale, vec(back.apply(arr(p1))) / scale);
        let first = usize::from(s.first_triangle);
        let last = (first + usize::from(s.triangle_count)).min(mesh.triangles.len());
        for t in first..last {
            if mesh.face_flags.get(t).is_some_and(|f| f & passes != 0) || skip(t) {
                continue;
            }
            let [a, b, c] = mesh.triangles[t].map(|v| Vec3::from_array(mesh.positions[usize::from(v)]));
            let normal =
                mesh.face_normals.get(t).map_or_else(|| (b - a).cross(c - a), |n| Vec3::from_array(*n));
            let two_sided = mesh.face_two_sided.get(t).copied().unwrap_or(false);
            let Some(q) = plane_crossing_sided(q0, q1, normal, a, two_sided) else { continue };
            if !inside(q, a, b, c, normal) {
                continue;
            }
            let d2 = (q - q0).length_squared() * scale * scale;
            if d2 <= best {
                best = d2;
                out = Some(Strike {
                    point: vec(pose.apply(arr(q * scale))),
                    d2,
                    node: Some(i),
                    triangle: Some(t),
                    // The face's normal is the node's; the world wants it turned by the pose.
                    normal: vec(parkan_formats::pose::rotate(
                        pose.rotation,
                        arr(normal.normalize_or_zero()),
                    )),
                });
            }
        }
    }
    out
}

/// `Control.dll:0x1001e9f0`: when, within the frame, two moving spheres first touch.
pub fn swept_spheres(a: (Vec3, Vec3), ra: f32, b: (Vec3, Vec3), rb: f32) -> Option<f32> {
    let r = ra + rb;
    let d = b.0 - a.0;
    if d.length_squared() < r * r {
        return Some(0.0);
    }
    let v = (b.1 - b.0) - (a.1 - a.0);
    let vv = v.length_squared();
    if vv < 1e-6 {
        return None;
    }
    let bb = -d.dot(v);
    if bb < 0.0 {
        return None;
    }
    let disc = bb * bb - (d.length_squared() - r * r) * vv;
    if disc < 0.0 {
        return None;
    }
    let t = (bb - disc.sqrt()) / vv;
    (t <= 1.0).then_some(t)
}

/// `Control.dll:0x1001e1e0`: where a segment that starts inside the map box leaves it.
/// The box's top is doubled by the caller.
pub fn map_edge(lo: Vec3, hi: Vec3, p0: Vec3, p1: Vec3) -> Option<(Vec3, f32)> {
    let within = |p: Vec3| p.cmpge(lo).all() && p.cmple(hi).all();
    if !within(p0) || within(p1) {
        return None;
    }
    let v = p1 - p0;
    let mut best: Option<(Vec3, f32)> = None;
    for axis in 0..3 {
        if v[axis] == 0.0 {
            continue;
        }
        for plane in [lo[axis], hi[axis]] {
            let t = (plane - p0[axis]) / v[axis];
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let p = p0 + v * t;
            let d2 = (p - p0).length_squared();
            if best.is_none_or(|(_, b)| d2 < b) {
                best = Some((p, d2));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::mesh::{Node, Slot};
    use parkan_formats::pose::IDENTITY;

    #[test]
    fn a_plane_is_struck_only_from_its_front() {
        let up = Vec3::Z;
        let hit = plane_crossing(Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -3.0), up, Vec3::ZERO);
        assert_eq!(hit, Some(Vec3::ZERO));
        assert_eq!(plane_crossing(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 3.0), up, Vec3::ZERO), None);
        assert_eq!(plane_crossing(Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 1.0), up, Vec3::ZERO), None);
    }

    #[test]
    fn spheres_moving_through_each_other_touch_within_the_frame() {
        let still = (Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0));
        let fast = (Vec3::ZERO, Vec3::new(10_000.0, 0.0, 0.0));
        let t = swept_spheres(still, 1.0, fast, 0.1).unwrap();
        assert!((t - 8.9 / 10_000.0).abs() < 1e-5, "{t}");
        let past = (Vec3::new(0.0, 5.0, 0.0), Vec3::new(10_000.0, 5.0, 0.0));
        assert_eq!(swept_spheres(still, 1.0, past, 0.1), None);
    }

    #[test]
    fn a_segment_leaving_the_map_box_is_clipped_at_its_face() {
        let (lo, hi) = (Vec3::ZERO, Vec3::splat(100.0));
        let (p, d2) = map_edge(lo, hi, Vec3::new(50.0, 50.0, 50.0), Vec3::new(150.0, 50.0, 50.0)).unwrap();
        assert_eq!((p, d2), (Vec3::new(100.0, 50.0, 50.0), 2500.0));
        assert_eq!(map_edge(lo, hi, Vec3::splat(50.0), Vec3::splat(60.0)), None);
    }

    /// One node, one triangle facing -y at y = 5, and a second node the same, moved.
    fn wall() -> (Mesh, Vec<Pose>) {
        let node = |slot: u16| Node {
            name: String::new(),
            flags: 0,
            parent: 0xFFFF,
            anim_start: 0xFFFF,
            fallback_key: 0,
            slot_index: std::array::from_fn(|k| if k == 0 { slot } else { NO_SLOT }),
        };
        let slot = |first: u16| Slot {
            first_triangle: first,
            triangle_count: 1,
            first_batch: 0,
            batch_count: 0,
            aabb_min: [0.0; 3],
            aabb_max: [0.0; 3],
            sphere: [0.0; 4],
            area: 0.0,
            volume: 0.0,
        };
        let mesh = Mesh {
            name: "wall".into(),
            positions: vec![[-1.0, 5.0, -1.0], [1.0, 5.0, -1.0], [0.0, 5.0, 1.0]],
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [0, 1, 2]],
            nodes: vec![node(0), node(1)],
            slots: vec![slot(0), slot(1)],
            batches: Vec::new(),
            face_two_sided: Vec::new(),
            face_flags: vec![0, ROUND_SKIPS_FACE],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
            corners: None,
        };
        let moved = Pose { translation: [0.0, -2.0, 0.0], ..IDENTITY };
        (mesh, vec![IDENTITY, moved])
    }

    #[test]
    fn a_scaled_object_is_struck_on_its_scaled_geometry() {
        let (mesh, _) = wall();
        let strike = segment_mesh(&mesh, &[IDENTITY], 2.0, Vec3::ZERO, Vec3::new(0.0, 20.0, 0.0)).unwrap();
        assert_eq!(strike.point, Vec3::new(0.0, 10.0, 0.0));
        assert_eq!(strike.d2, 100.0);
    }

    #[test]
    fn a_segment_strikes_the_nearest_node_and_passes_flagged_triangles() {
        let (mut mesh, world) = wall();
        let strike = segment_mesh(&mesh, &world, 1.0, Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)).unwrap();
        // The second node's triangle, nearer at y = 3, is flagged 0x24: the first is struck.
        assert_eq!((strike.node, strike.point), (Some(0), Vec3::new(0.0, 5.0, 0.0)));
        mesh.face_flags[1] = 0;
        let strike = segment_mesh(&mesh, &world, 1.0, Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)).unwrap();
        assert_eq!((strike.node, strike.point), (Some(1), Vec3::new(0.0, 3.0, 0.0)));
        assert_eq!(
            segment_mesh(&mesh, &world, 1.0, Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO),
            None,
            "from behind"
        );
        assert_eq!(
            segment_mesh(&mesh, &world, 1.0, Vec3::new(3.0, 0.0, 0.0), Vec3::new(3.0, 10.0, 0.0)),
            None
        );
    }

    /// `AniMesh.dll:0x1001110c`: a batch flagged `BATCH_TWO_SIDED` runs the plane test a
    /// second time with the segment reversed, so a round reaches its triangles from behind.
    /// 1477 of the 15153 shipped batches carry the bit, trees and buildings mostly.
    #[test]
    fn a_two_sided_batch_is_struck_from_behind_and_a_plain_one_is_not() {
        let (mut mesh, world) = wall();
        mesh.face_flags[1] = 0;
        let (behind, front) = (Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        assert_eq!(segment_mesh(&mesh, &world, 1.0, behind, front), None, "one-sided");
        mesh.face_two_sided = vec![true, true];
        let strike = segment_mesh(&mesh, &world, 1.0, behind, front).unwrap();
        // Its own node's triangle at y = 5 is the nearer one from up the segment.
        assert_eq!((strike.node, strike.point), (Some(0), Vec3::new(0.0, 5.0, 0.0)));
        // And a face it already met head-on is still met the same way.
        let head_on = segment_mesh(&mesh, &world, 1.0, front, behind).unwrap();
        assert_eq!((head_on.node, head_on.point), (Some(1), Vec3::new(0.0, 3.0, 0.0)));
    }

    /// `mrnPointInPoly` signs its edge tests by the **stored** normal, the one the plane
    /// test uses, not by the cross product of the triangle: the eight mesh faces whose
    /// stream-7 normal disagrees with their winding are tested the file's way.
    #[test]
    fn the_edge_test_is_signed_by_the_faces_own_normal() {
        let (a, b, c) = (Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0));
        let p = Vec3::new(0.5, 0.5, 0.0);
        let wound = (b - a).cross(c - a);
        assert!(inside(p, a, b, c, wound));
        assert!(!inside(p, a, b, c, -wound), "the same point, the normal reversed");
        assert!(inside(a, a, b, c, wound), "a corner is inside: the test is against zero");
        assert!(!inside(Vec3::new(2.0, 2.0, 0.0), a, b, c, wound));
    }

    #[test]
    fn the_sights_segment_stops_on_the_triangles_a_round_passes() {
        let (mesh, world) = wall();
        let (p0, p1) = (Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0));
        assert_eq!(segment_mesh(&mesh, &world, 1.0, p0, p1).unwrap().node, Some(0));
        let sight = segment_mesh_passing(&mesh, &world, 1.0, p0, p1, SIGHT_SKIPS_FACE).unwrap();
        assert_eq!((sight.node, sight.point), (Some(1), Vec3::new(0.0, 3.0, 0.0)));
    }
}
