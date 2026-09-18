//! The ground under a machine: `docs/24-motion.md`, "Finding the ground".
//!
//! The faces a unit stands on are level 0 of the map's `Land.msh`, less the water
//! surface, which is a class of its own; under a lake the ground is the bed. The
//! search takes a face only while its normal z is above cos 80°.

use glam::Vec3;
use parkan_formats::landmesh::LandMesh;

use crate::solid::Solid;

/// cos 80°: a steeper face is never ground (`Control.dll:0x1001a6fd`).
pub const WALKABLE_NORMAL_Z: f32 = 0.173648;
/// The body sphere's radius is held to this unless it is at least `LARGE_BODY`
/// (`0x1001a48e`).
pub const BODY_RADIUS_HOLD: f32 = 7.5;
pub const LARGE_BODY: f32 = 20.0;

/// The body sphere's radius as the ground contact uses it.
pub fn contact_radius(sphere_radius: f32) -> f32 {
    if sphere_radius < LARGE_BODY { sphere_radius.min(BODY_RADIUS_HOLD) } else { sphere_radius }
}

/// A face found under a point: a landscape face, or a face of a building's solid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The landscape face.
    pub face: Option<usize>,
    /// The solid and its face.
    pub solid: Option<(usize, usize)>,
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

/// What the map's water level is taken as with no water (`Terrain.dll:0x10017d60`).
pub const NO_WATER_LEVEL: f32 = -1.0;

/// Whichever of two strikes is the nearer.
fn nearer(a: Option<crate::hit::Strike>, b: Option<crate::hit::Strike>) -> Option<crate::hit::Strike> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if a.d2 <= b.d2 { a } else { b }),
        (a, b) => a.or(b),
    }
}

/// A building's outer ring in the world, across the ground: the landscape is cut from under
/// it, and the building's footing takes its place between the ring and the building's own
/// floor (docs/03, "Placing a building cuts the landscape").
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub points: Vec<[f32; 2]>,
    /// The footing's faces, which are ground where the landscape was cut away.
    pub faces: Vec<[Vec3; 3]>,
    pub lo: [f32; 2],
    pub hi: [f32; 2],
}

impl Cut {
    pub fn new(points: Vec<[f32; 2]>) -> Self {
        Self::with_faces(points, Vec::new())
    }

    pub fn with_faces(points: Vec<[f32; 2]>, faces: Vec<[Vec3; 3]>) -> Self {
        let lo = points.iter().fold([f32::MAX; 2], |m, p| [m[0].min(p[0]), m[1].min(p[1])]);
        let hi = points.iter().fold([f32::MIN; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
        Self { points, faces, lo, hi }
    }

    pub fn holds(&self, x: f32, y: f32) -> bool {
        (self.lo[0]..=self.hi[0]).contains(&x)
            && (self.lo[1]..=self.hi[1]).contains(&y)
            && parkan_formats::basement::contains(&self.points, x, y)
    }

    /// The footing's faces whose triangle holds `(x, y)`, each with its height there and its
    /// own normal.
    fn holding(&self, x: f32, y: f32) -> impl Iterator<Item = (f32, Vec3)> + '_ {
        self.faces.iter().filter_map(move |t| {
            let [a, b, c] = *t;
            let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
            if den.abs() < 1e-9 {
                return None;
            }
            let l1 = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / den;
            let l2 = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / den;
            let l3 = 1.0 - l1 - l2;
            (l1 >= -1e-5 && l2 >= -1e-5 && l3 >= -1e-5)
                .then(|| (l1 * a.z + l2 * b.z + l3 * c.z, (b - a).cross(c - a).normalize_or_zero()))
        })
    }
}

pub struct Ground {
    pub land: LandMesh,
    /// Placed objects' faces, by the caller's numbering; a building's are ground.
    pub solids: Vec<Solid>,
    /// Where placed buildings have cut the landscape away, each with the footing that stands
    /// there instead (docs/03, "Placing a building cuts the landscape").
    pub cuts: Vec<Cut>,
    lo: [f32; 2],
    size: [usize; 2],
    /// The landscape's own cell on each axis, `LandMesh::cell_size`.
    cell: [f32; 2],
    cells: Vec<Vec<u32>>,
    /// The water surface's faces, indexed apart: the liquid surface the bed's test asks for.
    water: Vec<Vec<u32>>,
    /// The mesh's bounding box: its stream-2 header's corners are exactly this.
    world: ([f32; 3], [f32; 3]),
}

impl Ground {
    /// The index over the map's own grid: the landscape's cells, the size and the origin
    /// the file gives (`LandMesh::grid`, `LandMesh::cell_size`). A map with no square
    /// stream falls back to one cell.
    pub fn new(land: LandMesh) -> Self {
        let (lo3, hi3) = land.bounds();
        let lo = [lo3[0], lo3[1]];
        let size = [0, 1].map(|a| land.grid[a].max(1));
        let own = land.cell_size();
        let cell_size = [0, 1].map(|a| if own[a] > 0.0 { own[a] } else { (hi3[a] - lo3[a]).max(1.0) });
        let mut cells = vec![Vec::new(); size[0] * size[1]];
        let mut water = vec![Vec::new(); size[0] * size[1]];
        for f in land.lod_faces(0) {
            let face = &land.faces[f];
            let index = if face.is_water() { &mut water } else { &mut cells };
            let ps = face.vertices.map(|v| land.positions[usize::from(v)]);
            let cell =
                |p: f32, a: usize| (((p - lo[a]) / cell_size[a]).floor().max(0.0) as usize).min(size[a] - 1);
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
                    index[y * size[0] + x].push(f as u32);
                }
            }
        }
        Self {
            land,
            solids: Vec::new(),
            cuts: Vec::new(),
            lo,
            size,
            cell: cell_size,
            cells,
            water,
            world: (lo3, hi3),
        }
    }

    /// The height the map gives its water (`Terrain.dll:0x10019180`, `ITerrain` slot 11, docs/35-hud.md,
    /// "The altitude"): the z of the first vertex of the first face whose surface is water, or
    /// −1 on a map with none.
    pub fn water_level(&self) -> f32 {
        self.land
            .faces
            .iter()
            .find(|f| f.is_water())
            .and_then(|f| self.land.positions.get(usize::from(f.vertices[0])))
            .map_or(NO_WATER_LEVEL, |p| p[2])
    }

    /// The map's extent in x and y.
    pub fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let (lo, hi) = self.world;
        ([lo[0], lo[1]], [hi[0], hi[1]])
    }

    /// The map's box, the one the rounds are clipped to: the ground mesh's bounds.
    pub fn world_box(&self) -> (Vec3, Vec3) {
        (Vec3::from_array(self.world.0), Vec3::from_array(self.world.1))
    }

    /// A segment through the ground (`Terrain.dll:0x100205c0`): the cells along its xy
    /// in order from `p0`, each cell's faces one-sided through the face normal, and
    /// the nearest strike in the first cell that has one. The cells are the landscape's
    /// own -- the map's extent over the grid its square stream carries, 49.90 to 311.28
    /// world units across the 33 shipped maps, and not a constant.
    ///
    /// **A round's ground test passes through the water surface** (*read*): its query
    /// excludes world face flags `0x208` (`Control.dll:0x1001da1e`), of which `0x200` is
    /// the landscape's `0x20000`, the surface word's water bit, on exactly the 3630 water
    /// faces of the 33 maps. So a shot into a lake splashes on the bed, not the sheet.
    pub fn segment(&self, p0: Vec3, p1: Vec3) -> Option<crate::hit::Strike> {
        self.segment_over(p0, p1, false)
    }

    /// [`Ground::segment`] with the water surface among the faces: the sight ray's query
    /// excludes no face at all -- no world flags, no class (`Control.dll:0x1002a68e`,
    /// `0x1001bca0`) -- so a lake stops the sight on its sheet where it lets a round by.
    pub fn segment_including_water(&self, p0: Vec3, p1: Vec3) -> Option<crate::hit::Strike> {
        self.segment_over(p0, p1, true)
    }

    fn segment_over(&self, p0: Vec3, p1: Vec3, with_water: bool) -> Option<crate::hit::Strike> {
        let (wx, wy) = (self.cell[0], self.cell[1]);
        let cell_of = |p: Vec3| (((p.x - self.lo[0]) / wx).floor(), ((p.y - self.lo[1]) / wy).floor());
        let (mut cx, mut cy) = cell_of(p0);
        let (ex, ey) = cell_of(p1);
        let d = p1 - p0;
        let step = |v: f32| if v > 0.0 { 1.0 } else { -1.0 };
        let (sx, sy) = (step(d.x), step(d.y));
        let boundary = |c: f32, s: f32, lo: f32, w: f32| lo + (c + if s > 0.0 { 1.0 } else { 0.0 }) * w;
        let t_at = |b: f32, from: f32, v: f32| if v == 0.0 { f32::INFINITY } else { (b - from) / v };
        let mut tx = t_at(boundary(cx, sx, self.lo[0], wx), p0.x, d.x);
        let mut ty = t_at(boundary(cy, sy, self.lo[1], wy), p0.y, d.y);
        let (dtx, dty) = (wx / d.x.abs(), wy / d.y.abs());
        let cells = (ex - cx).abs() + (ey - cy).abs() + 1.0;
        for _ in 0..cells as usize {
            if cx >= 0.0 && cy >= 0.0 && (cx as usize) < self.size[0] && (cy as usize) < self.size[1] {
                let at = cy as usize * self.size[0] + cx as usize;
                let mut best: Option<crate::hit::Strike> = None;
                let sheet: &[u32] = if with_water { &self.water[at] } else { &[] };
                for &f in self.cells[at].iter().chain(sheet) {
                    let face = &self.land.faces[f as usize];
                    let [a, b, c] =
                        face.vertices.map(|v| Vec3::from_array(self.land.positions[usize::from(v)]));
                    let Some(q) = crate::hit::plane_crossing(p0, p1, Vec3::from_array(face.normal), a) else {
                        continue;
                    };
                    let d2 = (q - p0).length_squared();
                    if crate::hit::inside(q, a, b, c, Vec3::from_array(face.normal))
                        && best.is_none_or(|s| d2 < s.d2)
                        && !self.cut(q.x, q.y)
                    {
                        best = Some(crate::hit::Strike {
                            point: q,
                            d2,
                            node: None,
                            triangle: Some(f as usize),
                            normal: Vec3::from_array(face.normal),
                        });
                    }
                }
                if best.is_some() {
                    return nearer(best, self.footing_strike(p0, p1));
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
        self.footing_strike(p0, p1)
    }

    /// The nearest footing face the segment crosses: a round that reaches the band stops on
    /// it, as it would on the landscape the band replaced.
    fn footing_strike(&self, p0: Vec3, p1: Vec3) -> Option<crate::hit::Strike> {
        let mut best: Option<crate::hit::Strike> = None;
        for [a, b, c] in self.cuts.iter().flat_map(|cut| cut.faces.iter().copied()) {
            let normal = (b - a).cross(c - a).normalize_or_zero();
            let Some(q) = crate::hit::plane_crossing(p0, p1, normal, a) else { continue };
            let d2 = (q - p0).length_squared();
            if crate::hit::inside(q, a, b, c, normal) && best.is_none_or(|s| d2 < s.d2) {
                best =
                    Some(crate::hit::Strike { point: q, d2, node: None, triangle: None, normal });
            }
        }
        best
    }

    /// The ground faces whose triangle holds `(x, y)`, in the file's order, which is the
    /// landscape's cell order, each with its barycentric height there.
    /// Whether a building has cut the landscape away at (x, y).
    pub fn cut(&self, x: f32, y: f32) -> bool {
        self.cuts.iter().any(|c| c.holds(x, y))
    }

    fn holding(&self, x: f32, y: f32) -> impl Iterator<Item = (usize, f32)> + '_ {
        let cut = self.cut(x, y);
        self.holding_in(&self.cells, x, y).filter(move |_| !cut)
    }

    /// The footings' faces over `(x, y)`: where a building has cut the landscape away, its
    /// basement band is the ground out to the outer contour.
    fn footing_faces(&self, x: f32, y: f32) -> impl Iterator<Item = Hit> + '_ {
        self.cuts
            .iter()
            .filter(move |c| c.holds(x, y))
            .flat_map(move |c| c.holding(x, y))
            .map(move |(z, normal)| Hit { face: None, solid: None, point: Vec3::new(x, y, z), normal })
    }

    fn holding_in<'a>(
        &'a self,
        index: &'a [Vec<u32>],
        x: f32,
        y: f32,
    ) -> impl Iterator<Item = (usize, f32)> + 'a {
        let cx = ((x - self.lo[0]) / self.cell[0]).floor();
        let cy = ((y - self.lo[1]) / self.cell[1]).floor();
        let inside = cx >= 0.0 && cy >= 0.0 && (cx as usize) < self.size[0] && (cy as usize) < self.size[1];
        let cell: &[u32] = if inside { &index[cy as usize * self.size[0] + cx as usize] } else { &[] };
        cell.iter().filter_map(move |&f| {
            let face = &self.land.faces[f as usize];
            let [a, b, c] = face.vertices.map(|v| Vec3::from_array(self.land.positions[usize::from(v)]));
            let den = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
            if den.abs() < 1e-9 {
                return None;
            }
            let l1 = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / den;
            let l2 = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / den;
            let l3 = 1.0 - l1 - l2;
            if l1 < -1e-5 || l2 < -1e-5 || l3 < -1e-5 {
                return None;
            }
            Some((f as usize, l1 * a.z + l2 * b.z + l3 * c.z))
        })
    }

    /// The liquid surface's height over or under `(x, y)` nearest `z` (`IWorld` slot 8,
    /// `Terrain.dll:0x10025ba0`): the same vertical query in both directions, for the
    /// faces whose `Land.msh` surface bitfield has bit `0x02` (world flag `0x200`).
    pub fn water(&self, x: f32, y: f32, z: f32) -> Option<f32> {
        self.holding_in(&self.water, x, y)
            .map(|(_, h)| h)
            .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
    }

    /// The highest ground face at `(x, y)` whose plane there is not above `top`, a
    /// building's deck included.
    pub fn below(&self, x: f32, y: f32, top: f32) -> Option<Hit> {
        let mut best: Option<Hit> = self
            .solid_faces(Vec3::new(x, y, top), false)
            .chain(self.footing_faces(x, y).filter(|h| h.point.z <= top))
            .max_by(|a, b| a.point.z.total_cmp(&b.point.z));
        for (f, z) in self.holding(x, y) {
            if z <= top && best.is_none_or(|h| z > h.point.z) {
                best = Some(Hit {
                    face: Some(f),
                    solid: None,
                    point: Vec3::new(x, y, z),
                    normal: Vec3::from_array(self.land.faces[f].normal),
                });
            }
        }
        best
    }

    /// The walk-face query (`IWorld` slot 10, `Terrain.dll:0x10026b20`): the landscape's
    /// first face whose triangle holds `p`'s xy and whose plane lies at or above `p`
    /// (register 6) or at or below it (register 10), dropped vertically onto the plane
    /// through the face's first vertex (`Control.dll:0x1001bfc0`), and every building's
    /// answer; the smallest gap wins.
    pub fn query(&self, p: Vec3, up: bool) -> Option<Hit> {
        self.holding(p.x, p.y)
            .find_map(|(f, _)| {
                let face = &self.land.faces[f];
                let normal = Vec3::from_array(face.normal);
                if normal.z.abs() < 1e-6 {
                    return None;
                }
                let v = Vec3::from_array(self.land.positions[usize::from(face.vertices[0])]);
                let z = v.z - (normal.x * (p.x - v.x) + normal.y * (p.y - v.y)) / normal.z;
                let right_way = if up { z >= p.z } else { z <= p.z };
                right_way.then_some(Hit { face: Some(f), solid: None, point: Vec3::new(p.x, p.y, z), normal })
            })
            .into_iter()
            .chain(self.solid_faces(p, up))
            .chain(
                self.footing_faces(p.x, p.y).filter(|h| if up { h.point.z >= p.z } else { h.point.z <= p.z }),
            )
            .min_by(|a, b| (a.point.z - p.z).abs().total_cmp(&(b.point.z - p.z).abs()))
    }

    /// The faces the buildings answer the walk-face query with: each present ground
    /// solid's nearest (docs/24, "Buildings are ground").
    fn solid_faces(&self, p: Vec3, up: bool) -> impl Iterator<Item = Hit> + '_ {
        self.solids.iter().enumerate().filter(|(_, s)| s.ground && s.present).filter_map(move |(i, s)| {
            if s.centre.truncate().distance(p.truncate()) > s.radius + crate::solid::WALK_MARGIN {
                return None;
            }
            let (f, z) = s.walk_face(p, up)?;
            Some(Hit {
                face: None,
                solid: Some((i, f)),
                point: Vec3::new(p.x, p.y, z),
                normal: s.faces[f].normal,
            })
        })
    }

    /// The ground contact's search from `p` (`Control.dll:0x1001a6cc`, `0x1001a77d`):
    /// the face above if it is walkable and less than `r2` above, else the face below if
    /// it is walkable, else the face above whatever it is (`0x1001a7e1`).
    ///
    /// STAND-IN: docs/24-motion.md#finding-the-ground--read -- the walk from the face
    /// held last tick (`FindWorldFace`) is read but not modelled: the face is searched
    /// fresh each step.
    pub fn search(&self, p: Vec3, r2: f32) -> Option<Hit> {
        let up = self.query(p, true);
        if let Some(h) = up
            && h.walkable()
            && h.point.z - p.z < r2
        {
            return up;
        }
        match self.query(p, false) {
            Some(h) if h.walkable() => Some(h),
            _ => up,
        }
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
        mesh(positions, faces)
    }

    fn mesh(positions: Vec<[f32; 3]>, faces: Vec<Face>) -> Ground {
        let land = LandMesh {
            normals: vec![[0.0, 0.0, 1.0]; positions.len()],
            uv1: vec![[0.0; 2]; positions.len()],
            uv2: vec![[0.0; 2]; positions.len()],
            blend: vec![1.0; positions.len()],
            positions,
            cells: vec![Cell { first: 0, count: faces.len() as u16 }],
            faces,
            grid: [4, 4],
            layer1: Vec::new(),
            layer2: Vec::new(),
        };
        Ground::new(land)
    }

    /// Ground from quads, each four corners counter-clockwise from above, split in two
    /// triangles with their normals from the geometry.
    pub(crate) fn quads(quads: &[[[f32; 3]; 4]]) -> Ground {
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for q in quads {
            let at = positions.len() as u16;
            positions.extend_from_slice(q);
            for corners in [[0u16, 1, 2], [0, 2, 3]] {
                let [a, b, c] = corners.map(|i| Vec3::from_array(q[usize::from(i)]));
                let n = (b - a).cross(c - a).normalize();
                faces.push(face(corners.map(|i| at + i), n.to_array(), 0));
            }
        }
        mesh(positions, faces)
    }

    #[test]
    fn the_search_takes_a_sunken_face_above_within_r2_and_otherwise_the_face_below() {
        // A floor at 0 under a shelf at 1 over x 0 to 20.
        let g = quads(&[
            [[0.0, 0.0, 0.0], [40.0, 0.0, 0.0], [40.0, 40.0, 0.0], [0.0, 40.0, 0.0]],
            [[0.0, 0.0, 1.0], [20.0, 0.0, 1.0], [20.0, 40.0, 1.0], [0.0, 40.0, 1.0]],
        ]);
        let p = Vec3::new(10.0, 10.0, 0.5);
        assert_eq!(g.query(p, true).unwrap().point, Vec3::new(10.0, 10.0, 1.0));
        assert_eq!(g.query(p, false).unwrap().point, Vec3::new(10.0, 10.0, 0.0));
        assert_eq!(g.search(p, 1.0).unwrap().point.z, 1.0, "sunk 0.5 into the shelf, within r2");
        assert_eq!(g.search(p, 0.4).unwrap().point.z, 0.0, "the shelf is too far above");
        assert_eq!(g.search(Vec3::new(30.0, 10.0, 0.5), 1.0).unwrap().point.z, 0.0);
        assert_eq!(g.search(Vec3::new(50.0, 10.0, 0.5), 1.0), None, "off the mesh");
    }

    #[test]
    fn with_nothing_walkable_the_search_keeps_the_face_above_whatever_it_is() {
        // A face at 85 degrees, rising 11.4 over x 0 to 1.
        let g = quads(&[[[0.0, 0.0, 0.0], [1.0, 0.0, 11.43], [1.0, 10.0, 11.43], [0.0, 10.0, 0.0]]]);
        let hit = g.search(Vec3::new(0.5, 5.0, 0.0), 1.0).unwrap();
        assert!(!hit.walkable());
        assert!((hit.point.z - 5.715).abs() < 1e-2, "{}", hit.point.z);
        assert_eq!(g.search(Vec3::new(0.5, 5.0, 10.0), 1.0), None, "below, and not walkable");
    }

    #[test]
    fn the_world_box_is_the_ground_meshs_bounds() {
        let (lo, hi) = floor().world_box();
        assert_eq!((lo, hi), (Vec3::ZERO, Vec3::new(41.0, 40.0, 40.0)));
        assert_eq!(floor().bounds(), ([0.0, 0.0], [41.0, 40.0]));
    }

    #[test]
    fn the_ground_under_a_lake_is_its_bed_not_the_water() {
        let g = floor();
        let hit = g.below(30.0, 5.0, 100.0).unwrap();
        assert_eq!(hit.point.z, 0.0);
        assert!(hit.walkable());
        assert_eq!((g.water(30.0, 5.0, 0.0), g.water(30.0, 5.0, 9.0)), (Some(5.0), Some(5.0)));
        assert_eq!(g.water(5.0, 30.0, 0.0), None, "the sheet covers only the first half");
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

    /// The landscape's cell is the map's extent over the grid its square stream carries,
    /// not a constant (`Terrain.dll:0x100178e6`, `0x10017c3f`). The test floor is 41 by 40
    /// over a 4 by 4 grid.
    #[test]
    fn the_index_takes_the_maps_own_grid_and_cell() {
        let g = floor();
        assert_eq!(g.land.grid, [4, 4]);
        let [wx, wy] = g.land.cell_size();
        assert!((wx - 41.0 / 4.0).abs() < 1e-4 && (wy - 40.0 / 4.0).abs() < 1e-4, "{wx} {wy}");
        assert_eq!(g.size, [4, 4]);
        assert_eq!(g.cell, [wx, wy]);
        // The walk crosses the cells and still finds the far floor.
        let s = g.segment(Vec3::new(1.0, 1.0, 20.0), Vec3::new(39.0, 39.0, -20.0)).unwrap();
        assert!(s.point.z.abs() < 1e-3, "{:?}", s.point);
    }

    /// The sight ray's query excludes no face (`Control.dll:0x1002a68e`), where a round's
    /// excludes world flags `0x208`, the water surface among them (`0x1001da1e`). So a
    /// lake stops the sight on its sheet and lets a round through to the bed.
    #[test]
    fn the_sight_stops_on_the_water_sheet_a_round_passes_through() {
        let g = floor();
        let (from, to) = (Vec3::new(30.0, 5.0, 20.0), Vec3::new(30.0, 5.0, -20.0));
        assert_eq!(g.segment(from, to).unwrap().point.z, 0.0, "a round meets the bed");
        assert_eq!(g.segment_including_water(from, to).unwrap().point.z, 5.0, "the sight, the sheet");
        // Off the sheet the two agree.
        let (dry, down) = (Vec3::new(5.0, 30.0, 20.0), Vec3::new(5.0, 30.0, -20.0));
        assert_eq!(
            g.segment(dry, down).unwrap().point.z,
            g.segment_including_water(dry, down).unwrap().point.z
        );
    }

    #[test]
    fn a_small_body_is_held_to_seven_and_a_half() {
        assert_eq!(contact_radius(3.0), 3.0);
        assert_eq!(contact_radius(12.0), 7.5);
        assert_eq!(contact_radius(25.0), 25.0);
    }
}
