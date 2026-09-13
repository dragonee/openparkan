//! The ground under a machine: `docs/24-motion.md`, "Finding the ground".
//!
//! The faces a unit stands on are level 0 of the map's `Land.msh`, less the water
//! surface, which is a class of its own; under a lake the ground is the bed. A
//! face is ground only while its normal z is above cos 80°.

use glam::Vec3;
use parkan_formats::landmesh::LandMesh;

/// cos 80°: a steeper face is never ground (`Control.dll:0x1001a6fd`).
pub const WALKABLE_NORMAL_Z: f32 = 0.173648;
/// The body sphere's radius is held to this unless it is at least `LARGE_BODY`
/// (`0x1001a48e`).
pub const BODY_RADIUS_HOLD: f32 = 7.5;
pub const LARGE_BODY: f32 = 20.0;
/// Side of a lookup cell, in world units. Not the game's: an index for speed.
const CELL: f32 = 16.0;

/// The body sphere's radius as the ground contact uses it.
pub fn contact_radius(sphere_radius: f32) -> f32 {
    if sphere_radius < LARGE_BODY { sphere_radius.min(BODY_RADIUS_HOLD) } else { sphere_radius }
}

/// A face found under a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub face: usize,
    /// The face plane at the query's xy.
    pub point: Vec3,
    /// The face's own normal, as the file stores it.
    pub normal: Vec3,
}

impl Hit {
    pub fn walkable(&self) -> bool {
        self.normal.z > WALKABLE_NORMAL_Z
    }
}

pub struct Ground {
    pub land: LandMesh,
    lo: [f32; 2],
    size: [usize; 2],
    cells: Vec<Vec<u32>>,
}

impl Ground {
    pub fn new(land: LandMesh) -> Self {
        let (lo3, hi3) = land.bounds();
        let lo = [lo3[0], lo3[1]];
        let size = [0, 1].map(|a| (((hi3[a] - lo3[a]) / CELL).floor() as usize + 1).max(1));
        let mut cells = vec![Vec::new(); size[0] * size[1]];
        for f in land.lod_faces(0) {
            let face = &land.faces[f];
            if face.is_water() {
                continue;
            }
            let ps = face.vertices.map(|v| land.positions[usize::from(v)]);
            let cell = |p: f32, a: usize| (((p - lo[a]) / CELL).floor().max(0.0) as usize).min(size[a] - 1);
            let (x0, x1) = (
                cell(ps.iter().map(|p| p[0]).fold(f32::MAX, f32::min), 0),
                cell(ps.iter().map(|p| p[0]).fold(f32::MIN, f32::max), 0),
            );
            let (y0, y1) = (
                cell(ps.iter().map(|p| p[1]).fold(f32::MAX, f32::min), 1),
                cell(ps.iter().map(|p| p[1]).fold(f32::MIN, f32::max), 1),
            );
            for y in y0..=y1 {
                for x in x0..=x1 {
                    cells[y * size[0] + x].push(f as u32);
                }
            }
        }
        Self { land, lo, size, cells }
    }

    /// The map's extent in x and y.
    pub fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let (lo, hi) = self.land.bounds();
        ([lo[0], lo[1]], [hi[0], hi[1]])
    }

    /// A segment through the ground (`Terrain.dll:0x100205c0`): the cells along its xy
    /// in order from `p0`, each cell's faces one-sided through the face normal, and
    /// the nearest strike in the first cell that has one.
    ///
    /// STAND-IN: docs/26-damage.md#the-hit-test--read-and-measured -- the landscape's
    /// own cell size is not this index's; the water surface, mask bit 8 by the look of
    /// it, is passed through.
    pub fn segment(&self, p0: Vec3, p1: Vec3) -> Option<crate::hit::Strike> {
        let cell_of = |p: Vec3| (((p.x - self.lo[0]) / CELL).floor(), ((p.y - self.lo[1]) / CELL).floor());
        let (mut cx, mut cy) = cell_of(p0);
        let (ex, ey) = cell_of(p1);
        let d = p1 - p0;
        let step = |v: f32| if v > 0.0 { 1.0 } else { -1.0 };
        let (sx, sy) = (step(d.x), step(d.y));
        let boundary = |c: f32, s: f32, lo: f32| lo + (c + if s > 0.0 { 1.0 } else { 0.0 }) * CELL;
        let t_at = |b: f32, from: f32, v: f32| if v == 0.0 { f32::INFINITY } else { (b - from) / v };
        let mut tx = t_at(boundary(cx, sx, self.lo[0]), p0.x, d.x);
        let mut ty = t_at(boundary(cy, sy, self.lo[1]), p0.y, d.y);
        let (dtx, dty) = (CELL / d.x.abs(), CELL / d.y.abs());
        let cells = (ex - cx).abs() + (ey - cy).abs() + 1.0;
        for _ in 0..cells as usize {
            if cx >= 0.0 && cy >= 0.0 && (cx as usize) < self.size[0] && (cy as usize) < self.size[1] {
                let mut best: Option<crate::hit::Strike> = None;
                for &f in &self.cells[cy as usize * self.size[0] + cx as usize] {
                    let face = &self.land.faces[f as usize];
                    let [a, b, c] =
                        face.vertices.map(|v| Vec3::from_array(self.land.positions[usize::from(v)]));
                    let Some(q) = crate::hit::plane_crossing(p0, p1, Vec3::from_array(face.normal), a) else {
                        continue;
                    };
                    let d2 = (q - p0).length_squared();
                    if crate::hit::inside(q, a, b, c) && best.is_none_or(|s| d2 < s.d2) {
                        best = Some(crate::hit::Strike { point: q, d2, node: None, triangle: f as usize });
                    }
                }
                if best.is_some() {
                    return best;
                }
            }
            if tx < ty {
                cx += sx;
                tx += dtx;
            } else {
                cy += sy;
                ty += dty;
            }
        }
        None
    }

    /// The highest ground face at `(x, y)` whose plane there is not above `top`.
    pub fn below(&self, x: f32, y: f32, top: f32) -> Option<Hit> {
        let cx = ((x - self.lo[0]) / CELL).floor();
        let cy = ((y - self.lo[1]) / CELL).floor();
        if cx < 0.0 || cy < 0.0 || cx as usize >= self.size[0] || cy as usize >= self.size[1] {
            return None;
        }
        let mut best: Option<Hit> = None;
        for &f in &self.cells[cy as usize * self.size[0] + cx as usize] {
            let face = &self.land.faces[f as usize];
            let [a, b, c] = face.vertices.map(|v| Vec3::from_array(self.land.positions[usize::from(v)]));
            let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
            if den.abs() < 1e-9 {
                continue;
            }
            let l1 = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / den;
            let l2 = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / den;
            let l3 = 1.0 - l1 - l2;
            if l1 < -1e-5 || l2 < -1e-5 || l3 < -1e-5 {
                continue;
            }
            let z = l1 * a.z + l2 * b.z + l3 * c.z;
            if z <= top && best.is_none_or(|h| z > h.point.z) {
                best = Some(Hit {
                    face: f as usize,
                    point: Vec3::new(x, y, z),
                    normal: Vec3::from_array(face.normal),
                });
            }
        }
        best
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use parkan_formats::landmesh::{Cell, Face, NO_TEXTURE, SURFACE_WATER_BIT};

    fn face(vertices: [u16; 3], normal: [f32; 3], surface: u16) -> Face {
        Face {
            vertices,
            adjacency: [0xFFFF; 3],
            flags: 0,
            surface,
            tex1: 0,
            tex2: NO_TEXTURE,
            normal,
            edge_twins: 0,
        }
    }

    /// A 40 by 40 floor at z 0 split in two, a wall rising from x 40 to 41 up to
    /// z 40, and a water sheet at z 5 over the floor's first half.
    pub(crate) fn floor() -> Ground {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [40.0, 0.0, 0.0],
            [40.0, 40.0, 0.0],
            [0.0, 40.0, 0.0],
            [41.0, 0.0, 40.0],
            [41.0, 40.0, 40.0],
            [0.0, 0.0, 5.0],
            [40.0, 0.0, 5.0],
            [40.0, 40.0, 5.0],
        ];
        let steep = [-0.9993, 0.0, 0.0366];
        let faces = vec![
            face([0, 1, 2], [0.0, 0.0, 1.0], 0),
            face([0, 2, 3], [0.0, 0.0, 1.0], 0),
            face([1, 4, 5], steep, 0),
            face([1, 5, 2], steep, 0),
            face([6, 7, 8], [0.0, 0.0, 1.0], SURFACE_WATER_BIT),
        ];
        let land = LandMesh {
            normals: vec![[0.0, 0.0, 1.0]; positions.len()],
            uv1: vec![[0.0; 2]; positions.len()],
            uv2: vec![[0.0; 2]; positions.len()],
            blend: vec![1.0; positions.len()],
            positions,
            cells: vec![Cell { first: 0, count: 5 }],
            faces,
            layer1: Vec::new(),
            layer2: Vec::new(),
        };
        Ground::new(land)
    }

    #[test]
    fn the_ground_under_a_lake_is_its_bed_not_the_water() {
        let g = floor();
        let hit = g.below(30.0, 5.0, 100.0).unwrap();
        assert_eq!(hit.point.z, 0.0);
        assert!(hit.walkable());
    }

    #[test]
    fn a_face_steeper_than_eighty_degrees_is_found_but_not_walkable() {
        let g = floor();
        let wall = g.below(40.5, 20.0, 100.0).unwrap();
        assert!((wall.point.z - 20.0).abs() < 1e-3);
        assert!(!wall.walkable());
        assert_eq!(g.below(40.5, 20.0, 10.0), None);
        assert_eq!(g.below(-1.0, 20.0, 10.0), None);
    }

    #[test]
    fn a_shot_meets_the_floor_through_the_water_and_not_from_below() {
        let g = floor();
        let s = g.segment(Vec3::new(2.0, 30.0, 20.0), Vec3::new(38.0, 1.0, -20.0)).unwrap();
        assert!(s.point.z.abs() < 1e-3 && s.node.is_none(), "{:?}", s.point);
        assert_eq!(g.segment(Vec3::new(20.0, 20.0, -5.0), Vec3::new(21.0, 20.0, 5.0)), None);
        let wall = g.segment(Vec3::new(45.0, 20.0, 20.0), Vec3::new(35.0, 20.0, 20.0));
        assert!(wall.is_none(), "the wall faces -x: a shot from +x is behind it");
    }

    #[test]
    fn a_small_body_is_held_to_seven_and_a_half() {
        assert_eq!(contact_radius(3.0), 3.0);
        assert_eq!(contact_radius(12.0), 7.5);
        assert_eq!(contact_radius(25.0), 25.0);
    }
}
