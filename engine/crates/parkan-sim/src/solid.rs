//! A placed object's level-0 faces in the world: what a building's deck holds a machine
//! up on, and what a machine collides with. See `docs/24-motion.md`, "Finding the
//! ground" and "Collision between objects".
//!
//! Placed objects do not move once built, so their faces are carried into the world
//! once, with the triangle's flags, its batch's flags and its material's surface id.

use glam::Vec3;

use crate::combat::Part;
use crate::ground::WALKABLE_NORMAL_Z;
use crate::hit::inside;

/// How far off a triangle, in x and y, a point may still find it (`AniMesh.dll:0x1000ce90`).
pub const WALK_MARGIN: f32 = 0.5;
/// A triangle flagged 4 passes a collision's faces: the big trees' leaves
/// (`Control.dll:0x1001db5f`–`0x1001dbb8`).
pub const COLLISION_SKIPS_FACE: u16 = 0x4;
/// A triangle flagged 2 is a floor, which the push-out passes for a mover without flag 8.
pub const FLOOR_FACE: u16 = 0x2;
/// A triangle flagged `0x20` is see-through -- a building's console glass and the energy
/// bridge's additive `B_A_BRIGE` ([07](../../../docs/07-objects.md#the-flags-word)) -- and
/// lets a mover through as one flagged 4 does. See [`passes`].
pub const SEE_THROUGH_FACE: u16 = 0x20;
/// A push is held to this many radii (`AniMesh.dll:0x1000df50`, `0x10020970`).
pub const PUSH_RADII: f32 = 4.0;
/// A push below this squared length is no contact (`Control.dll:0x1001e05f`).
pub const NO_CONTACT: f32 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolidFace {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    /// The world normal, from the file's.
    pub normal: Vec3,
    pub triangle_flags: u16,
    /// The ground surface id of the face's material, if it has one.
    pub surface: Option<u8>,
    /// The damage a second the material deals what touches it (`+0x1a4`).
    pub damage_rate: f32,
}

/// One node's level-0 slot: its sphere in the world, and its faces.
#[derive(Clone, Debug, PartialEq)]
pub struct SolidNode {
    pub centre: Vec3,
    pub radius: f32,
    pub faces: std::ops::Range<usize>,
    /// The part and the mesh node it is.
    pub part: usize,
    pub node: usize,
    /// An open door's node, whose faces let a mover through (`IBuilding` slot 17,
    /// `AniMesh.dll:0x1000dd13`).
    pub open: bool,
}

/// A placed object's faces.
#[derive(Clone, Debug, PartialEq)]
pub struct Solid {
    pub centre: Vec3,
    pub radius: f32,
    /// A building's faces are ground; a unit's and scenery's are not (docs/24).
    pub ground: bool,
    /// Still in the world.
    pub present: bool,
    /// What it weighs, kg: the mass a pair shares its push by the square of (docs/24,
    /// "Collision between objects"). 0 on anything with no contact record -- a building, a
    /// tree, a stone -- which takes none of the push.
    pub mass: f32,
    pub faces: Vec<SolidFace>,
    pub nodes: Vec<SolidNode>,
}

fn vec(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

impl Solid {
    /// The faces of a target's posed parts, with `surface(part, material)` naming a batch's
    /// material's surface id, damage rate and whether its faces let a mover through: each node's level-0 slot of the variant its
    /// stage draws, and nothing of a hidden node, which the walk-face and push visitors pass
    /// over (`AniMesh.dll:0x1000ce90`, `0x1000dfe0`).
    pub fn from_parts(
        parts: &[Part],
        centre: Vec3,
        radius: f32,
        ground: bool,
        surface: impl Fn(usize, u16) -> Option<(u8, f32, bool)>,
    ) -> Self {
        let mut faces = Vec::new();
        let mut nodes = Vec::new();
        for (p, part) in parts.iter().enumerate() {
            let mesh = &part.mesh;
            for i in 0..mesh.nodes.len() {
                let (Some(pose), Some(slot)) =
                    (part.nodes.get(i), part.slot(i).and_then(|s| mesh.slots.get(usize::from(s))))
                else {
                    continue;
                };
                let place = |v: [f32; 3]| vec(pose.apply(v.map(|x| f64::from(x * part.scale))));
                let turn = |v: [f32; 3]| vec(parkan_formats::pose::rotate(pose.rotation, v.map(f64::from)));
                let first = usize::from(slot.first_triangle).min(mesh.triangles.len());
                let last = (first + usize::from(slot.triangle_count)).min(mesh.triangles.len());
                let start = faces.len();
                for t in first..last {
                    let [a, b, c] = mesh.triangles[t].map(|v| place(mesh.positions[usize::from(v)]));
                    let normal = mesh.face_normals.get(t).map_or_else(
                        || (b - a).cross(c - a).normalize_or_zero(),
                        |n| turn(*n).normalize_or_zero(),
                    );
                    let batch = mesh.batches.iter().find(|b| {
                        let (from, count) = b.triangles();
                        (from..from + count).contains(&t)
                    });
                    let material = batch.and_then(|b| surface(p, b.material));
                    let passes = material.is_some_and(|m| m.2);
                    let flags = mesh.face_flags.get(t).copied().unwrap_or(0);
                    faces.push(SolidFace {
                        a,
                        b,
                        c,
                        normal,
                        triangle_flags: if passes { flags | COLLISION_SKIPS_FACE } else { flags },
                        surface: material.map(|m| m.0),
                        damage_rate: material.map_or(0.0, |m| m.1),
                    });
                }
                let [cx, cy, cz, r] = slot.sphere;
                nodes.push(SolidNode {
                    centre: place([cx, cy, cz]),
                    radius: r * part.scale,
                    faces: start..faces.len(),
                    part: p,
                    node: i,
                    open: false,
                });
            }
        }
        Self { centre, radius, ground, present: true, mass: 0.0, faces, nodes }
    }

    /// Interface `0x25` slot 2 (`AniMesh.dll:0x1000ccb0`, `0x1000cfa0`): the walkable face
    /// under or over `p` nearest by height. A node is visited when `p` lies, in x and y,
    /// within its sphere's radius plus 0.5. A face steeper than 80° is refused. Inside
    /// the triangle in x and y the height is its plane's; outside, against the first edge
    /// `p` lies beyond, the nearer end's height past an end or the height along the edge,
    /// refused farther than 0.5 off (`0x1000d220`, `0x1000d145`). `up` keeps faces at or
    /// above `p`, otherwise at or below (`0x1000d161`–`0x1000d18f`). Returns the face and
    /// its height.
    pub fn walk_face(&self, p: Vec3, up: bool) -> Option<(usize, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for node in &self.nodes {
            if node.centre.truncate().distance(p.truncate()) > node.radius + WALK_MARGIN {
                continue;
            }
            for f in node.faces.clone() {
                let face = &self.faces[f];
                if face.normal.z < WALKABLE_NORMAL_Z {
                    continue;
                }
                let Some(z) = height_at(face, p) else { continue };
                if (up && z < p.z) || (!up && z > p.z) {
                    continue;
                }
                if best.is_none_or(|(_, bz)| (z - p.z).powi(2) < (bz - p.z).powi(2)) {
                    best = Some((f, z));
                }
            }
        }
        best
    }
}

/// The height a face gives at `p`'s x and y: its plane's inside the triangle, and
/// against the first edge `p` lies beyond outside it, within the 0.5 margin.
fn height_at(face: &SolidFace, p: Vec3) -> Option<f32> {
    let n = face.normal;
    let corners = [face.a, face.b, face.c];
    let flat = |v: Vec3| v.truncate();
    let q = flat(p);
    // Edges by the triangle's own winding seen from above, whichever way it runs.
    let turn = (flat(face.b) - flat(face.a)).perp_dot(flat(face.c) - flat(face.a)).signum();
    let side = |u: glam::Vec2, v: glam::Vec2| turn * (v - u).perp_dot(q - u);
    for e in 0..3 {
        let (u, v) = (corners[e], corners[(e + 1) % 3]);
        if side(flat(u), flat(v)) >= 0.0 {
            continue;
        }
        // Beyond this edge: measured against it.
        let along = flat(v) - flat(u);
        let length2 = along.length_squared();
        let s = if length2 > 0.0 { (q - flat(u)).dot(along) / length2 } else { 0.0 };
        let (at, z) = if s <= 0.0 {
            (flat(u), u.z)
        } else if s >= 1.0 {
            (flat(v), v.z)
        } else {
            (flat(u) + along * s, u.z + (v.z - u.z) * s)
        };
        return (at.distance_squared(q) <= WALK_MARGIN * WALK_MARGIN).then_some(z);
    }
    (n.z.abs() > 1e-6).then(|| face.a.z - (n.x * (p.x - face.a.x) + n.y * (p.y - face.a.y)) / n.z)
}

/// How a sphere at `centre` of `radius` touches a face (`AniMesh.dll:0x1000e900`): the
/// distance and the unit direction from the touching point to the centre, or none.
///
/// The face is passed over when its plane lies behind the centre or more than the radius in
/// front of it. The centre's projection onto the plane is the point while it lies inside the
/// edges, taken in the order a→b, b→c, c→a. Past the first edge it lies beyond, that edge
/// alone decides (`0x1000eb70`): the projection must lie within √(r² − h²) of the edge's
/// line, h the height over the plane, and the point is the projection's foot on the edge,
/// or the edge's start or end past them, which must then lie within the radius
/// (`0x1000ebb2`, `0x1000ec51`, `0x1000eccf`). A corner nearer through another edge is not
/// looked for.
fn touch(face: &SolidFace, centre: Vec3, radius: f32) -> Option<(f32, Vec3)> {
    let n = face.normal;
    let height = n.dot(centre - face.a);
    if !(0.0..=radius).contains(&height) {
        return None;
    }
    let q = centre - n * height;
    let beyond = [(face.a, face.b), (face.b, face.c), (face.c, face.a)].into_iter().find_map(|(from, to)| {
        let length = (to - from).length();
        let e = (to - from).normalize_or_zero();
        let side = e.cross(n).dot(q - from);
        (side > 0.0).then_some((from, to, e, length, side))
    });
    let (at, distance) = match beyond {
        None => (q, height),
        Some((from, to, e, length, side)) => {
            if radius * radius - height * height < side * side {
                return None;
            }
            let along = (q - from).dot(e);
            if along < 0.0 || along > length {
                let corner = if along < 0.0 { from } else { to };
                let d = centre.distance(corner);
                if d > radius {
                    return None;
                }
                (corner, d)
            } else {
                (from + e * along, (side * side + height * height).sqrt())
            }
        }
    };
    (distance > 1e-6).then(|| (distance, (centre - at).normalize_or_zero()))
}

/// A one-sided crossing (`Ngi32.dll`'s `g_FastProc` slot `+0xb4`, `0x1001fa00`): the segment
/// from `p0` to `p1` meets the plane of `face` running against its normal, `p0` in front of it
/// or on it and `p1` strictly behind. The point where it meets it.
fn crossing(face: &SolidFace, p0: Vec3, p1: Vec3) -> Option<Vec3> {
    let along = (p1 - p0).dot(face.normal);
    if along >= -f32::MIN_POSITIVE {
        return None;
    }
    let front = face.normal.dot(p0 - face.a);
    if front < 0.0 || -along <= front {
        return None;
    }
    Some(p0 + (p1 - p0) * (front / -along))
}

/// Whether `q`, on the plane of `face`, lies in its triangle as the hidden-face test takes it
/// (`AniMesh.dll:0x1000d8d3`–`0x1000dab8`): on the plane dropping the axis the normal is
/// largest along (x before y, and either before z on a tie), each edge in the order c→a,
/// a→b, b→c gives the cross product's component times the normal's. The first edge within
/// 1e-5 of 0 lets the point in at once; one below 0 keeps it out; three above let it in.
fn hides(face: &SolidFace, q: Vec3) -> bool {
    let n = face.normal;
    let axis = if n.y.abs() > n.x.abs() { 1 } else { 0 };
    let axis = if n.z.abs() > n[axis].abs() { 2 } else { axis };
    for (prev, cur) in [(face.c, face.a), (face.a, face.b), (face.b, face.c)] {
        let v = (cur - prev).cross(q - prev)[axis] * n[axis];
        if v.abs() <= HIDDEN_EDGE {
            return true;
        }
        if v < 0.0 {
            return false;
        }
    }
    true
}

/// How near 0 an edge's value lets a point into a hiding face (`AniMesh.dll:0x1002097c`).
pub const HIDDEN_EDGE: f32 = 1e-5;

/// Whether a face lets a mover through.
///
/// STAND-IN: docs/24-motion.md#collision-between-objects--read -- the query also passes
/// batches flagged 8, and 0x200 unless the mover's flags carry 4; a mesh's batch record
/// carries no such word, so they are a flag set on the loaded batch that is not traced,
/// and no batch passes, as a round's query takes it.
///
/// The push-out's own filter drops the floors, triangles flagged 2, unless the mover's
/// collision flags carry 8 (`Control.dll:0x1001db2b`, `0x1001dbce`, docs/24, "The way to the
/// pod").
///
/// STAND-IN: docs/24-motion.md#not-established -- who sets a collision object's flags is
/// not read: no mover carries 8, so every floor lets a mover by, as a recording shows the
/// hero walking the Large Factory's ramps and stairs.
///
/// STAND-IN: docs/24-motion.md#standing-on-a-bridge--read-and-measured -- a triangle flagged
/// `0x20` passes here too, which the read of the collision's filters does not give (their
/// triangle mask is 4, `0x1001dbad`, `0x1001dbce`): the round query's is `0x24` and takes both.
/// The shipped data asks for it. **All four** bridges in `fortif.rlb` are placed as two halves
/// π apart whose decks meet, so the end cap each half carries at the join stands in the way of
/// anything crossing; three of them flag that cap **4** and the fourth, `fr_e_brige`, flags it
/// **`0x20`**, the bit its energy material carries throughout. Without this the hero crosses
/// C00 Mission 01's `m_bridge` and stops dead in the middle of C02 Mission 04's.
fn passes(face: &SolidFace, _obstacle: &Solid) -> bool {
    face.triangle_flags & (COLLISION_SKIPS_FACE | FLOOR_FACE | SEE_THROUGH_FACE) != 0
}

/// Whether a straight move from `start` to `end` runs into a shut face of `obstacle` — a wall
/// between the two. It is the push's own first step (`0x1001dd42`): a face the segment meets
/// against its normal, the doors that stand open and the faces a mover passes left out. A walk
/// planned over the areal map knows nothing of a building's walls (docs/24, "What the links
/// cost"), so a walk into one asks this before it sets off.
pub fn blocked(start: Vec3, end: Vec3, obstacle: &Solid) -> bool {
    obstacle
        .nodes
        .iter()
        .filter(|n| !n.open)
        .flat_map(|n| n.faces.clone())
        .map(|f| &obstacle.faces[f])
        .filter(|f| !passes(f, obstacle))
        .any(|face| {
            crate::hit::plane_crossing(start, end, face.normal, face.a)
                .is_some_and(|q| inside(q, face.a, face.b, face.c, face.normal))
        })
}

/// A mover's sphere, from `start` to `end`, against an obstacle's faces
/// (`Control.dll:0x1001daf0`): the push its move takes.
///
/// 1. **Faces stop it.** A face its centre's segment runs into against the normal puts
///    its end back at its start (`0x1001dd42`).
/// 2. **The shape pushes it out** (`AniMesh.dll:0x1000d410`): every face within the
///    sphere at its end, nearest first, less those whose centroid another face hides
///    from the centre, adds to the push what its depth still needs, square to the push
///    so far; the push is held to 4r.
///
/// A face touches the sphere as [`touch`] reads `0x1000e900`, and hides another as the test
/// at `0x1000d7a5` reads it ([`crossing`], [`hides`]).
///
/// STAND-IN: docs/24-motion.md#collision-between-objects--read -- the small-face stop needs
/// the mover's class 3 or more, and the hero's is taken as its size class, 2, so it never
/// applies.
pub fn push(start: Vec3, end: Vec3, radius: f32, obstacle: &Solid) -> Vec3 {
    let mut end = end;
    let mut total = Vec3::ZERO;
    let move_ = end - start;
    if move_.length_squared() > 0.0 {
        let shut = obstacle.nodes.iter().filter(|n| !n.open).flat_map(|n| n.faces.clone());
        for face in shut.map(|f| &obstacle.faces[f]).filter(|f| !passes(f, obstacle)) {
            if move_.dot(face.normal) < 0.0
                && let Some(q) = crate::hit::plane_crossing(start, end, face.normal, face.a)
                && inside(q, face.a, face.b, face.c, face.normal)
            {
                total = start - end;
                end = start;
                break;
            }
        }
    }

    // Gather every face the sphere at its end touches, floors and faces flagged 4 among them,
    // nearest first (`0x1000dfe0`, `0x1000e900`).
    let mut near: Vec<(f32, Vec3, usize)> = Vec::new();
    for node in obstacle.nodes.iter().filter(|n| !n.open) {
        if node.centre.distance(end) > node.radius + radius {
            continue;
        }
        for f in node.faces.clone() {
            if let Some((distance, direction)) = touch(&obstacle.faces[f], end, radius) {
                near.push((distance, direction, f));
            }
        }
    }
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Drop the hidden faces (`0x1000d7a5`–`0x1000dac0`), from the farthest to the nearest: a
    // face goes when the segment from the centre to its centroid crosses another face still
    // gathered, from its front into its back, inside its triangle. A face dropped hides
    // nothing after.
    //
    // STAND-IN: docs/24-motion.md#collision-between-objects--read -- a hider whose batch word
    // (record `+0x40`) carries 2 is crossed either way (`0x1000d86c`); where the batch word
    // comes from is not traced and the engine carries none, so every hider is one-sided.
    let mut i = near.len();
    while i > 0 {
        i -= 1;
        let face = &obstacle.faces[near[i].2];
        let centroid = (face.a + face.b + face.c) / 3.0;
        let hidden = near.iter().enumerate().rev().any(|(j, &(_, _, g))| {
            let other = &obstacle.faces[g];
            j != i && crossing(other, end, centroid).is_some_and(|q| hides(other, q))
        });
        if hidden {
            near.remove(i);
        }
    }
    // Then filter (`0x1000db93`).
    let kept: Vec<(f32, Vec3)> = near
        .iter()
        .filter(|&&(_, _, f)| !passes(&obstacle.faces[f], obstacle))
        .map(|&(d, n, _)| (d, n))
        .collect();

    // Accumulate nearest first (`0x1000dd7d`–`0x1000def8`).
    let mut p = Vec3::ZERO;
    for (distance, d) in kept {
        let depth = radius - distance;
        let s = p.dot(d);
        if s >= depth {
            continue;
        }
        let pp = p.length_squared();
        let square = if pp <= 1e-4 { d } else { d - p * (d.dot(p) / pp) };
        let dd = square.dot(d);
        p += if dd.abs() <= 0.002 {
            square.normalize_or_zero() * (depth - s)
        } else {
            square * ((depth - s) / dd)
        };
    }
    let most = PUSH_RADII * radius;
    if p.length() > most {
        p = p.normalize_or_zero() * most;
    }
    total + p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(z: f32, normal: Vec3) -> Solid {
        let (a, b, c, d) = (
            Vec3::new(0.0, 0.0, z),
            Vec3::new(10.0, 0.0, z),
            Vec3::new(10.0, 10.0, z),
            Vec3::new(0.0, 10.0, z),
        );
        let face =
            |a, b, c| SolidFace { a, b, c, normal, triangle_flags: 0, surface: Some(5), damage_rate: 0.0 };
        Solid {
            centre: Vec3::new(5.0, 5.0, z),
            radius: 7.1,
            ground: true,
            present: true,
            mass: 0.0,
            faces: vec![face(a, b, c), face(a, c, d)],
            nodes: vec![SolidNode {
                centre: Vec3::new(5.0, 5.0, z),
                radius: 7.1,
                faces: 0..2,
                part: 0,
                node: 0,
                open: false,
            }],
        }
    }

    #[test]
    fn a_deck_answers_the_walk_face_query_above_or_below_and_half_a_metre_past_its_edge() {
        let deck = quad(3.0, Vec3::Z);
        assert_eq!(deck.walk_face(Vec3::new(5.0, 5.0, 1.0), true).map(|(_, z)| z), Some(3.0));
        assert_eq!(deck.walk_face(Vec3::new(5.0, 5.0, 1.0), false), None, "nothing below");
        assert_eq!(deck.walk_face(Vec3::new(5.0, 5.0, 4.0), false).map(|(_, z)| z), Some(3.0));
        assert!(deck.walk_face(Vec3::new(10.4, 5.0, 4.0), false).is_some(), "0.4 past the edge");
        assert!(deck.walk_face(Vec3::new(10.6, 5.0, 4.0), false).is_none(), "0.6 past it");
        let wall = quad(3.0, Vec3::X);
        assert!(wall.walk_face(Vec3::new(5.0, 5.0, 4.0), false).is_none(), "steeper than 80 degrees");
    }

    #[test]
    fn a_sphere_is_pushed_out_of_a_wall_and_stopped_by_one_it_moves_into() {
        // A wall at x = 0 facing -x, from y -10 to 10 and z -10 to 10.
        let (a, b, c, d) = (
            Vec3::new(0.0, -10.0, -10.0),
            Vec3::new(0.0, 10.0, -10.0),
            Vec3::new(0.0, 10.0, 10.0),
            Vec3::new(0.0, -10.0, 10.0),
        );
        let face = |a, b, c| SolidFace {
            a,
            b,
            c,
            normal: -Vec3::X,
            triangle_flags: 0,
            surface: None,
            damage_rate: 0.0,
        };
        let wall = Solid {
            centre: Vec3::ZERO,
            radius: 14.2,
            ground: false,
            present: true,
            mass: 0.0,
            faces: vec![face(a, c, b), face(a, d, c)],
            nodes: vec![SolidNode {
                centre: Vec3::ZERO,
                radius: 14.2,
                faces: 0..2,
                part: 0,
                node: 0,
                open: false,
            }],
        };
        // Standing 1.5 in front of it with a radius of 2: pushed back by 0.5.
        let p = push(Vec3::new(-1.5, 0.0, 0.0), Vec3::new(-1.5, 0.0, 0.0), 2.0, &wall);
        assert!((p - Vec3::new(-0.5, 0.0, 0.0)).length() < 1e-4, "{p}");
        // Walking through it: the end goes back to the start.
        let p = push(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 2.0, &wall);
        assert!((p.x - -4.0).abs() < 1e-4, "{p}");
        // Leaves pass.
        let mut leaves = wall.clone();
        leaves.faces.iter_mut().for_each(|f| f.triangle_flags = COLLISION_SKIPS_FACE);
        assert_eq!(push(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 2.0, &leaves), Vec3::ZERO);
        // And so does a see-through face: the end cap the energy bridge carries where its two
        // halves meet is flagged 0x20 where the other three bridges flag theirs 4.
        let mut glass = wall.clone();
        glass.faces.iter_mut().for_each(|f| f.triangle_flags = SEE_THROUGH_FACE);
        assert_eq!(push(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 2.0, &glass), Vec3::ZERO);
        assert!(!blocked(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), &glass));
        assert!(blocked(Vec3::new(-3.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), &wall), "a wall still stops");
    }

    fn face(a: Vec3, b: Vec3, c: Vec3, triangle_flags: u16) -> SolidFace {
        let normal = (b - a).cross(c - a).normalize();
        SolidFace { a, b, c, normal, triangle_flags, surface: None, damage_rate: 0.0 }
    }

    #[test]
    fn past_its_first_edge_a_triangle_is_touched_along_that_edge_alone() {
        // A floor triangle facing up; the centre 1 above and beyond both of its corner's edges.
        let t = face(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), Vec3::new(0.0, 4.0, 0.0), 0);
        let inside = touch(&t, Vec3::new(1.0, 1.0, 1.0), 2.0).unwrap();
        assert!((inside.0 - 1.0).abs() < 1e-6 && inside.1 == Vec3::Z);
        // Beyond a→b only: the foot on it.
        let (d, dir) = touch(&t, Vec3::new(2.0, -1.0, 1.0), 2.0).unwrap();
        assert!(
            (d - 2f32.sqrt()).abs() < 1e-5 && (dir - Vec3::new(0.0, -1.0, 1.0).normalize()).length() < 1e-5
        );
        // Beyond a→b and c→a near the corner: a→b, the first, decides, and its start is the point.
        let (d, dir) = touch(&t, Vec3::new(-0.5, -0.5, 1.0), 2.0).unwrap();
        assert!((d - 1.5f32.sqrt()).abs() < 1e-5, "{d}");
        assert!((dir - Vec3::new(-0.5, -0.5, 1.0).normalize()).length() < 1e-5);
        // Beyond b→c alone, but 1.6 off its line with 1 of height: √(4 − 1) = 1.73 lets it in.
        assert!(touch(&t, Vec3::new(3.13, 3.13, 1.0), 2.0).is_some());
        assert!(touch(&t, Vec3::new(3.3, 3.3, 1.0), 2.0).is_none());
        // Behind the plane, or farther than the radius in front: nothing.
        assert!(touch(&t, Vec3::new(1.0, 1.0, -0.1), 2.0).is_none());
        assert!(touch(&t, Vec3::new(1.0, 1.0, 2.1), 2.0).is_none());
    }

    #[test]
    fn a_face_hides_another_only_from_its_front_through_its_triangle_or_an_edges_line() {
        // A wall facing -x at x = 0, and behind it a second at x = 1; the centre at x = -1.
        let wall = |x: f32, flags| {
            face(Vec3::new(x, -5.0, -5.0), Vec3::new(x, -5.0, 5.0), Vec3::new(x, 5.0, 5.0), flags)
        };
        let near = wall(0.0, 0);
        assert!(near.normal.x < 0.0);
        let centre = Vec3::new(-1.0, 0.0, 0.0);
        let behind = Vec3::new(1.0, 1.0, 1.0);
        let q = crossing(&near, centre, behind).expect("from its front into its back");
        assert!(q.x.abs() < 1e-6 && hides(&near, q));
        assert!(crossing(&near, behind, centre).is_none(), "not from its back");
        assert!(crossing(&near, centre, Vec3::new(-0.5, 0.0, 0.0)).is_none(), "nor short of it");
        // A point on an edge's line, even past the triangle, lets it in at once.
        assert!(hides(&near, Vec3::new(0.0, 7.0, 7.0)));
        assert!(!hides(&near, Vec3::new(0.0, 6.0, 0.0)));
    }
}
