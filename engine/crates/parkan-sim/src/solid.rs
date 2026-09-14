//! A placed object's level-0 faces in the world: what a building's deck holds a machine
//! up on, and what a machine collides with. See `docs/24-motion.md`, "Finding the
//! ground" and "Collision between objects".
//!
//! Placed objects do not move once built, so their faces are carried into the world
//! once, with the triangle's flags, its batch's flags and its material's surface id.

use glam::Vec3;
use parkan_formats::mesh::NO_SLOT;

use crate::combat::Part;
use crate::ground::WALKABLE_NORMAL_Z;
use crate::hit::inside;

/// How far off a triangle, in x and y, a point may still find it (`AniMesh.dll:0x1000ce90`).
pub const WALK_MARGIN: f32 = 0.5;
/// A triangle flagged 4 passes a collision's faces: the big trees' leaves
/// (`Control.dll:0x1001db5f`–`0x1001dbb8`).
pub const COLLISION_SKIPS_FACE: u16 = 0x4;
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
    pub faces: Vec<SolidFace>,
    pub nodes: Vec<SolidNode>,
}

fn vec(v: [f64; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

impl Solid {
    /// The faces of a target's posed parts, with `surface(part, material)` naming a batch's
    /// material's surface id and damage rate.
    pub fn from_parts(
        parts: &[Part],
        centre: Vec3,
        radius: f32,
        ground: bool,
        surface: impl Fn(usize, u16) -> Option<(u8, f32)>,
    ) -> Self {
        let mut faces = Vec::new();
        let mut nodes = Vec::new();
        for (p, part) in parts.iter().enumerate() {
            let mesh = &part.mesh;
            for (i, node) in mesh.nodes.iter().enumerate() {
                let (Some(pose), Some(slot)) =
                    (part.nodes.get(i), mesh.slots.get(usize::from(node.slot_index[0])))
                else {
                    continue;
                };
                if node.slot_index[0] == NO_SLOT {
                    continue;
                }
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
                    faces.push(SolidFace {
                        a,
                        b,
                        c,
                        normal,
                        triangle_flags: mesh.face_flags.get(t).copied().unwrap_or(0),
                        surface: material.map(|m| m.0),
                        damage_rate: material.map_or(0.0, |m| m.1),
                    });
                }
                let [cx, cy, cz, r] = slot.sphere;
                nodes.push(SolidNode {
                    centre: place([cx, cy, cz]),
                    radius: r * part.scale,
                    faces: start..faces.len(),
                });
            }
        }
        Self { centre, radius, ground, present: true, faces, nodes }
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

/// The point of a triangle nearest `p`.
fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

/// Whether a face lets a mover through.
///
/// STAND-IN: docs/24-motion.md#collision-between-objects--read -- the query also passes
/// batches flagged 8, and 0x200 unless the mover's flags carry 4; a mesh's batch record
/// carries no such word, so they are a flag set on the loaded batch that is not traced,
/// and no batch passes, as a round's query takes it.
///
/// STAND-IN: docs/24-motion.md#standing-on-a-bridge--read-and-measured -- how a machine
/// gets onto a building's ramp while the ramp's faces push its sphere back is not read:
/// a building's walkable faces, which the ground contact stands machines on, do not push.
fn passes(face: &SolidFace, obstacle: &Solid) -> bool {
    face.triangle_flags & COLLISION_SKIPS_FACE != 0 || (obstacle.ground && face.normal.z >= WALKABLE_NORMAL_Z)
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
/// `0x1000e900` accepts a face whose plane has the centre in front within the radius,
/// and measures to the centre's projection inside the triangle or to the nearest edge
/// outside it.
///
/// STAND-IN: docs/24-motion.md#collision-between-objects--read -- past the first edge's
/// test `0x1000e900` is not transcribed: the triangle's nearest point, within the radius,
/// the direction running from it to the centre. The small-face stop needs the mover's
/// class 3 or more, and the hero's is taken as its size class, 2, so it never applies.
pub fn push(start: Vec3, end: Vec3, radius: f32, obstacle: &Solid) -> Vec3 {
    let mut end = end;
    let mut total = Vec3::ZERO;
    let move_ = end - start;
    if move_.length_squared() > 0.0 {
        for face in obstacle.faces.iter().filter(|f| !passes(f, obstacle)) {
            if move_.dot(face.normal) < 0.0
                && let Some(q) = crate::hit::plane_crossing(start, end, face.normal, face.a)
                && inside(q, face.a, face.b, face.c)
            {
                total = start - end;
                end = start;
                break;
            }
        }
    }

    // Gather the faces within the sphere at its end.
    let mut near: Vec<(f32, Vec3, usize)> = Vec::new();
    for node in &obstacle.nodes {
        if node.centre.distance(end) > node.radius + radius {
            continue;
        }
        for f in node.faces.clone() {
            let face = &obstacle.faces[f];
            if passes(face, obstacle) {
                continue;
            }
            // `0x1000e900`: the centre in front of the face's plane and within the radius
            // of it, then the triangle's nearest point, inside it or on an edge or corner.
            let plane = face.normal.dot(end - face.a);
            if !(0.0..=radius).contains(&plane) {
                continue;
            }
            let on = closest_on_triangle(end, face.a, face.b, face.c);
            let distance = on.distance(end);
            if distance < radius && distance > 1e-6 {
                near.push((distance, (end - on) / distance, f));
            }
        }
    }
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    // A face whose centroid another gathered face hides from the centre goes
    // (`0x1000d7a5`–`0x1000dac0`).
    let hidden = |f: usize| {
        let face = &obstacle.faces[f];
        let centroid = (face.a + face.b + face.c) / 3.0;
        near.iter().any(|&(_, _, g)| {
            let other = &obstacle.faces[g];
            g != f
                && crate::hit::plane_crossing(end, centroid, other.normal, other.a)
                    .or_else(|| crate::hit::plane_crossing(centroid, end, other.normal, other.a))
                    .is_some_and(|q| inside(q, other.a, other.b, other.c))
        })
    };
    let kept: Vec<(f32, Vec3)> =
        near.iter().filter(|&&(_, _, f)| !hidden(f)).map(|&(d, n, _)| (d, n)).collect();

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
            faces: vec![face(a, b, c), face(a, c, d)],
            nodes: vec![SolidNode { centre: Vec3::new(5.0, 5.0, z), radius: 7.1, faces: 0..2 }],
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
            faces: vec![face(a, c, b), face(a, d, c)],
            nodes: vec![SolidNode { centre: Vec3::ZERO, radius: 14.2, faces: 0..2 }],
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
    }
}
