//! `Land.msh`, a map's terrain. See `docs/03-terrain.md`.

use std::path::Path;

use crate::cursor::FormatError;
use crate::nres::Archive;
use crate::wea;

pub const STREAM_SQUARES: u32 = 1;
pub const STREAM_BOUNDS: u32 = 2;
pub const STREAM_POSITION: u32 = 3;
pub const STREAM_NORMAL: u32 = 4;
pub const STREAM_UV1: u32 = 5;
pub const STREAM_DRAW_ORDER: u32 = 11;
pub const STREAM_BLEND: u32 = 14;
pub const STREAM_UV2: u32 = 18;
pub const STREAM_FACE: u32 = 21;
pub const FACE_STRIDE: usize = 28;
pub const NO_TEXTURE: u8 = 0xFF;
/// The engine's UV unit: a stream's `uint16` pair is that many to one turn of the texture
/// (`Terrain.dll:0x10035070` keeps `1.0 / 1024.0` at `0x100a5c6c`, and every strided expansion
/// scales a `uint16` UV by it, `0x10038876`; the landscape's own face query the same,
/// `0x100227cf` into `0x100a5a04`). Stream 18's gradient is 51.2 a world unit on all 33 maps,
/// which is the unread setting `MicroTexScale`'s 0.05 at this unit (docs/03, "The UV unit").
pub const UV_FIXED_POINT_SCALE: f32 = 1024.0;
pub const SURFACE_WATER_BIT: u16 = 0x02;
pub const FLAGS_LIQUID_BED_BIT: u16 = 0x2000;
/// The two levels of detail a map is stored at.
pub const LOD_COUNT: usize = 2;
const BOX_HEADER: usize = 96;
const CELL_START: usize = 44;
const CELL_STRIDE: usize = 68;
const NORMAL_SCALE: f32 = 32767.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub vertices: [u16; 3],
    pub adjacency: [u16; 3],
    pub flags: u16,
    pub surface: u16,
    /// The face's material, an index into `Land1.wea`; its microtexture is the material at the
    /// same index of `Land2.wea`.
    pub tex1: u8,
    /// A second material drawn over the first, an index into the same two tables;
    /// `NO_TEXTURE` when the face has none.
    pub tex2: u8,
    pub normal: [f32; 3],
    pub edge_twins: u16,
}

impl Face {
    pub fn is_water(&self) -> bool {
        self.surface & SURFACE_WATER_BIT != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub first: u16,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LandMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Stream 5, the first texture stage's coordinates: the material's own texture, across
    /// 170 to 256 world units a turn.
    pub uv1: Vec<[f32; 2]>,
    /// Stream 18, the second stage's: the microtexture, a turn every 20 world units.
    pub uv2: Vec<[f32; 2]>,
    /// Stream 14, the vertex alpha of a face's **second** material, which is drawn over its
    /// first (`Terrain.dll:0x1002c000`, alpha mode 2 and `SRCALPHA`/`INVSRCALPHA`): the
    /// ground is `mix(first, second, blend)`. 1.0 on every vertex no such face touches.
    pub blend: Vec<f32>,
    pub faces: Vec<Face>,
    pub cells: Vec<Cell>,
    /// The landscape's own grid, cells across and down: the square stream's second count
    /// field is the first, and its element count divided by that is the second
    /// (`Terrain.dll:0x100178e6`–`0x1001794f`). 16 x 16 on 28 of the 33 shipped maps and
    /// 8 x 8 on the other five.
    pub grid: [usize; 2],
    /// `Land1.wea`: the materials a face's two bytes index, its first and its second alike.
    pub layer1: Vec<String>,
    /// `Land2.wea`, index for index with `Land1.wea`: the material whose track 1 is the
    /// **microtexture** of the one at the same index (`Terrain.dll:0x10017215` loads it as the
    /// landscape manager's second wear, and the cell draw asks for it under that wear's index,
    /// `0x100445c0`). Not a second layer's table: a face's second byte indexes `Land1.wea`.
    pub layer2: Vec<String>,
}

impl LandMesh {
    /// The first face of level 1; level 0 is `[0, split)`.
    pub fn lod_split(&self) -> usize {
        let per = self.cells.len() / LOD_COUNT;
        if per > 0 && self.cells.len() > per { usize::from(self.cells[per].first) } else { self.faces.len() }
    }

    pub fn lod_faces(&self, level: usize) -> std::ops::Range<usize> {
        let split = self.lod_split();
        if level == 0 { 0..split } else { split..self.faces.len() }
    }

    /// The landscape's own cell, in world units on each axis: the map's extent over its
    /// [`LandMesh::grid`]. `CLandscape` keeps its reciprocal and finds the cell under a
    /// point by `floor((x − x0) ÷ cell)` (`Terrain.dll:0x1001775c`, `0x10017c3f`), the box
    /// corner 0 of the stream-2 header being the origin. It is a **per-map** figure, from
    /// 49.90 on map 41 to 311.28 on `SC_3`, and never a constant.
    pub fn cell_size(&self) -> [f32; 2] {
        let (lo, hi) = self.bounds();
        [0, 1].map(|a| if self.grid[a] > 0 { (hi[a] - lo[a]) / self.grid[a] as f32 } else { 0.0 })
    }

    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &self.positions {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        (lo, hi)
    }

    /// The z of the map's one water plane, or `None` if it has none or several.
    pub fn water_level(&self) -> Option<f32> {
        let mut level = None;
        for face in self.faces.iter().filter(|f| f.is_water()) {
            for &v in &face.vertices {
                let z = self.positions[usize::from(v)][2];
                match level {
                    None => level = Some(z),
                    Some(l) if l != z => return None,
                    _ => {}
                }
            }
        }
        level
    }

    /// Terrain elevation at a world xy over level 0, the higher surface where
    /// water covers ground; `None` outside the mesh.
    pub fn height_at(&self, x: f32, y: f32) -> Option<f32> {
        self.height_at_f64(f64::from(x), f64::from(y)).map(|z| z as f32)
    }

    /// `height_at` in double precision throughout.
    pub fn height_at_f64(&self, x: f64, y: f64) -> Option<f64> {
        let mut best: Option<f64> = None;
        for face in &self.faces[self.lod_faces(0)] {
            let [a, b, c] = face.vertices.map(|v| self.positions[usize::from(v)].map(f64::from));
            let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if den.abs() < 1e-12 {
                continue;
            }
            let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den;
            let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den;
            let l3 = 1.0 - l1 - l2;
            if l1 < -1e-6 || l2 < -1e-6 || l3 < -1e-6 {
                continue;
            }
            let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
            best = Some(best.map_or(z, |b| b.max(z)));
        }
        best
    }
}

fn stream(archive: &Archive, id: u32) -> Result<&[u8], FormatError> {
    let mut found = archive.entries.iter().filter(|e| e.type_id() == id);
    match (found.next(), found.next()) {
        (Some(entry), None) => archive.read(entry),
        _ => Err(FormatError::invalid(&archive.source, format!("expected one stream of type {id}"))),
    }
}

fn f32s<const N: usize>(raw: &[u8], i: usize) -> [f32; N] {
    std::array::from_fn(|k| f32::from_le_bytes(raw[i + 4 * k..i + 4 * k + 4].try_into().expect("4 bytes")))
}

/// Load a `Land.msh`, and the `Land1.wea` / `Land2.wea` beside it.
pub fn load(path: &Path) -> Result<LandMesh, FormatError> {
    let archive = Archive::open(path)?;
    let positions_raw = stream(&archive, STREAM_POSITION)?;
    let nv = positions_raw.len() / 12;
    let positions = (0..nv).map(|i| f32s(positions_raw, i * 12)).collect();
    let normals_raw = stream(&archive, STREAM_NORMAL)?;
    let normals =
        (0..nv).map(|i| std::array::from_fn(|k| f32::from(normals_raw[i * 4 + k] as i8) / 127.0)).collect();
    let uv = |raw: &[u8]| -> Vec<[f32; 2]> {
        (0..nv)
            .map(|i| {
                let w = |o: usize| f32::from(u16::from_le_bytes([raw[i * 4 + o], raw[i * 4 + o + 1]]));
                [w(0) / UV_FIXED_POINT_SCALE, w(2) / UV_FIXED_POINT_SCALE]
            })
            .collect()
    };
    let uv1 = uv(stream(&archive, STREAM_UV1)?);
    let uv2 = uv(stream(&archive, STREAM_UV2)?);
    let blend_raw = stream(&archive, STREAM_BLEND)?;
    let blend = (0..nv).map(|i| f32s::<1>(blend_raw, i * 4)[0]).collect();
    let face_raw = stream(&archive, STREAM_FACE)?;
    let faces = face_raw
        .as_chunks::<FACE_STRIDE>()
        .0
        .iter()
        .map(|r| {
            let w = |i: usize| u16::from_le_bytes([r[2 * i], r[2 * i + 1]]);
            Face {
                flags: w(0),
                surface: w(1),
                tex1: (w(2) & 0xFF) as u8,
                tex2: (w(2) >> 8) as u8,
                vertices: [w(4), w(5), w(6)],
                adjacency: [w(7), w(8), w(9)],
                normal: [10, 11, 12].map(|i| f32::from(w(i) as i16) / NORMAL_SCALE),
                edge_twins: w(13),
            }
        })
        .collect();
    let bounds = stream(&archive, STREAM_BOUNDS)?;
    let cells = bounds
        .get(BOX_HEADER..)
        .map(|body| {
            let mut out = Vec::new();
            let mut at = CELL_START;
            while at + CELL_STRIDE <= body.len() {
                out.push(Cell {
                    first: u16::from_le_bytes([body[at], body[at + 1]]),
                    count: u16::from_le_bytes([body[at + 2], body[at + 3]]),
                });
                at += CELL_STRIDE;
            }
            out
        })
        .unwrap_or_default();
    let squares = archive.entries.iter().find(|e| e.type_id() == STREAM_SQUARES);
    let grid = squares.map_or([0, 0], |e| {
        let across = e.link_count as usize;
        [across, (e.element_count as usize).checked_div(across).unwrap_or(0)]
    });
    let table = |name: &str| {
        crate::gamedir::resolve(path.parent().unwrap_or(Path::new(".")), name)
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| wea::parse(&b).materials)
            .unwrap_or_default()
    };
    Ok(LandMesh {
        positions,
        normals,
        uv1,
        uv2,
        blend,
        faces,
        cells,
        grid,
        layer1: table("Land1.wea"),
        layer2: table("Land2.wea"),
    })
}
