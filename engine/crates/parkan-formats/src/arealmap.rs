//! A map's `Land.map`, its areal map: the world cut into polygons, the *areals*, each with
//! its neighbour across every edge, and a 128 × 128 grid that lists the areals over each
//! cell. See `docs/08-arealmap.md`.

use std::path::Path;

use crate::FormatError;
use crate::cursor::Cursor;
use crate::nres::Archive;

/// The member type the areal map is (`ArealMap.dll`'s loader looks up chunk type 12).
pub const AREAL_MAP_TYPE: u32 = 12;
/// An areal's header, before its vertices.
pub const HEADER_SIZE: usize = 0x38;
/// The neighbour and twin an edge on the outside of the map carries.
pub const NO_NEIGHBOUR: i32 = -1;
/// Three more pairs follow the polygon's own for each sub-block.
pub const SUB_BLOCK_EDGES: usize = 3;
/// The bits of the third flag word that mark a lake.
pub const LAKE_BITS: u32 = 0xF0;

/// One areal.
#[derive(Clone, Debug, PartialEq)]
pub struct Areal {
    pub centre: [f32; 2],
    pub area: f32,
    /// The polygon, counter-clockwise.
    pub vertices: Vec<[f32; 3]>,
    /// One per vertex, for the edge from it to the next: the areal across, and the index of
    /// the same edge in that areal's list; both [`NO_NEIGHBOUR`] on the outside.
    pub edges: Vec<(i32, i32)>,
    /// The four flag words at `+0x20`.
    pub flags: [u32; 4],
}

impl Areal {
    /// The areal across edge `e`, or none on the outside of the map.
    pub fn neighbour(&self, e: usize) -> Option<usize> {
        self.edges.get(e).and_then(|&(n, _)| usize::try_from(n).ok())
    }

    /// The first flag word is set: a unit that does not fly may be sent to a place on it
    /// (`iron3d.dll:0x10076805`, docs/42, "A valid place").
    pub fn usable(&self) -> bool {
        self.flags[0] != 0
    }

    /// The third flag word has all of [`LAKE_BITS`].
    pub fn lake(&self) -> bool {
        self.flags[2] & LAKE_BITS == LAKE_BITS
    }

    /// Whether `(x, y)` lies inside the polygon, by the crossings of a ray toward +x.
    pub fn holds(&self, x: f32, y: f32) -> bool {
        let n = self.vertices.len();
        let mut inside = false;
        for i in 0..n {
            let (a, b) = (self.vertices[i], self.vertices[(i + 1) % n]);
            if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
                inside = !inside;
            }
        }
        inside
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArealMap {
    pub areals: Vec<Areal>,
    pub cells_across: usize,
    pub cells_down: usize,
    /// The areals over each cell, x the outer index: cell (x, y) is `x * cells_down + y`.
    pub cells: Vec<Vec<u16>>,
}

impl ArealMap {
    /// The map's extent: the lowest and highest vertex in x and y.
    pub fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let mut lo = [f32::MAX; 2];
        let mut hi = [f32::MIN; 2];
        for v in self.areals.iter().flat_map(|a| &a.vertices) {
            for k in 0..2 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        (lo, hi)
    }

    /// The areals the grid lists over `(x, y)`, the grid spanning `bounds`; none off the map.
    pub fn listed_at(&self, bounds: ([f32; 2], [f32; 2]), x: f32, y: f32) -> &[u16] {
        let (lo, hi) = bounds;
        let cell = |v: f32, k: usize, count: usize| {
            let span = hi[k] - lo[k];
            (span > 0.0 && (lo[k]..=hi[k]).contains(&v))
                .then(|| (((v - lo[k]) / span * count as f32) as usize).min(count - 1))
        };
        match (cell(x, 0, self.cells_across), cell(y, 1, self.cells_down)) {
            (Some(cx), Some(cy)) => &self.cells[cx * self.cells_down + cy],
            _ => &[],
        }
    }

    /// The areal whose polygon holds `(x, y)`, among those the grid lists there.
    pub fn areal_at(&self, bounds: ([f32; 2], [f32; 2]), x: f32, y: f32) -> Option<usize> {
        self.listed_at(bounds, x, y)
            .iter()
            .map(|&i| usize::from(i))
            .find(|&i| self.areals.get(i).is_some_and(|a| a.holds(x, y)))
    }
}

fn areal(c: &mut Cursor) -> Result<Areal, FormatError> {
    let start = c.pos;
    let centre = [c.f32()?, c.f32()?];
    c.pos = start + 0x10;
    let area = c.f32()?;
    c.pos = start + 0x20;
    let flags = [c.u32()?, c.u32()?, c.u32()?, c.u32()?];
    let (count, blocks) = (c.u32()? as usize, c.u32()? as usize);
    let vertices = (0..count).map(|_| c.vec3()).collect::<Result<Vec<_>, _>>()?;
    let pairs = (0..count + SUB_BLOCK_EDGES * blocks)
        .map(|_| Ok((c.i32()?, c.i32()?)))
        .collect::<Result<Vec<_>, FormatError>>()?;
    // A sub-block is carried and never used (docs/08): its points are stepped over.
    for _ in 0..blocks {
        let n = c.u32()? as usize;
        c.bytes(n * 12)?;
    }
    Ok(Areal { centre, area, vertices, edges: pairs[..count].to_vec(), flags })
}

/// The areal map of a `Land.map` archive: as many areals as its entry's element count, then
/// the grid, and nothing after.
pub fn parse(blob: Vec<u8>, name: &str) -> Result<ArealMap, FormatError> {
    let archive = Archive::parse(blob, name.to_owned())?;
    let entry = archive
        .entries
        .iter()
        .find(|e| e.type_id() == AREAL_MAP_TYPE)
        .ok_or_else(|| FormatError::invalid(name, "no ArealMap member"))?;
    let data = archive.read(entry)?;
    let mut c = Cursor::new(data, name);
    let areals = (0..entry.element_count).map(|_| areal(&mut c)).collect::<Result<Vec<_>, _>>()?;
    let (across, down) = (c.u32()? as usize, c.u32()? as usize);
    if across == 0 || down == 0 {
        return Err(FormatError::invalid(name, "an empty cell grid"));
    }
    let mut cells = Vec::with_capacity(across * down);
    for _ in 0..across * down {
        let n = u16::from_le_bytes(c.bytes(2)?.try_into().expect("2 bytes"));
        let items = c.bytes(usize::from(n) * 2)?;
        cells.push(items.as_chunks::<2>().0.iter().map(|&b| u16::from_le_bytes(b)).collect());
    }
    if c.remaining() != 0 {
        return Err(FormatError::invalid(name, "bytes after the cell grid"));
    }
    Ok(ArealMap { areals, cells_across: across, cells_down: down, cells })
}

pub fn load(path: &Path) -> Result<ArealMap, FormatError> {
    parse(std::fs::read(path)?, &path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square areal from `x0` to `x0 + 10`, `y` 0 to 10, its right edge shared with `right`.
    fn square(x0: f32, flags: [u32; 4], right: i32, left: i32) -> Vec<u8> {
        let mut out = Vec::new();
        for f in [x0 + 5.0, 5.0, 0.0, 0.0, 100.0, 0.0, 0.0, 1.0] {
            out.extend(f.to_le_bytes());
        }
        for w in flags {
            out.extend(w.to_le_bytes());
        }
        out.extend(4u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        for [x, y] in [[x0, 0.0], [x0 + 10.0, 0.0], [x0 + 10.0, 10.0], [x0, 10.0]] {
            for f in [x, y, 1.0f32] {
                out.extend(f.to_le_bytes());
            }
        }
        for (n, twin) in [(-1i32, -1i32), (right, 3), (-1, -1), (left, 1)] {
            out.extend(n.to_le_bytes());
            out.extend(twin.to_le_bytes());
        }
        out
    }

    fn archive(payload: &[u8], count: u32) -> Vec<u8> {
        let mut blob = vec![0u8; 16];
        blob.extend(payload);
        while !blob.len().is_multiple_of(8) {
            blob.push(0);
        }
        let mut entry = [0u8; 64];
        entry[0..4].copy_from_slice(&AREAL_MAP_TYPE.to_le_bytes());
        entry[4..8].copy_from_slice(&count.to_le_bytes());
        entry[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        entry[20..27].copy_from_slice(b"ArealMa");
        entry[56..60].copy_from_slice(&16u32.to_le_bytes());
        blob.extend(entry);
        let total = blob.len() as u32;
        blob[0..4].copy_from_slice(b"NRes");
        blob[4..8].copy_from_slice(&0x100u32.to_le_bytes());
        blob[8..12].copy_from_slice(&1u32.to_le_bytes());
        blob[12..16].copy_from_slice(&total.to_le_bytes());
        blob
    }

    #[test]
    fn an_areal_map_reads_its_areals_by_the_entrys_count_then_its_grid_x_outer() {
        let mut payload = square(0.0, [1, 0, 4, 0], 1, -1);
        payload.extend(square(10.0, [0, 0, 240, 0], -1, 0));
        payload.extend(2u32.to_le_bytes());
        payload.extend(1u32.to_le_bytes());
        // Cell (0, 0) lists areal 0, cell (1, 0) areal 1.
        for index in [0u16, 1] {
            payload.extend(1u16.to_le_bytes());
            payload.extend(index.to_le_bytes());
        }
        let map = parse(archive(&payload, 2), "Land.map").unwrap();
        assert_eq!(map.areals.len(), 2);
        let (a, b) = (&map.areals[0], &map.areals[1]);
        assert_eq!((a.centre, a.area, a.flags), ([5.0, 5.0], 100.0, [1, 0, 4, 0]));
        assert_eq!((a.neighbour(1), a.neighbour(0), b.neighbour(3)), (Some(1), None, Some(0)));
        assert!(b.lake() && !a.lake());
        assert_eq!((map.cells_across, map.cells_down), (2, 1));
        let bounds = map.bounds();
        assert_eq!(bounds, ([0.0, 0.0], [20.0, 10.0]));
        assert_eq!(map.areal_at(bounds, 15.0, 5.0), Some(1));
        assert_eq!(map.areal_at(bounds, 5.0, 5.0), Some(0));
        assert_eq!(map.areal_at(bounds, 25.0, 5.0), None, "off the map");
        // A byte too many is refused, as the game's loader panics on it.
        let mut long = payload.clone();
        long.push(0);
        assert!(parse(archive(&long, 2), "Land.map").is_err());
    }

    #[test]
    fn a_polygon_holds_what_its_notch_leaves_out() {
        // An L: the square (0..10)² less its top right quarter.
        let l = Areal {
            centre: [0.0; 2],
            area: 75.0,
            vertices: [[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [5.0, 5.0], [5.0, 10.0], [0.0, 10.0]]
                .map(|[x, y]| [x, y, 0.0])
                .to_vec(),
            edges: vec![(NO_NEIGHBOUR, NO_NEIGHBOUR); 6],
            flags: [0; 4],
        };
        assert!(l.holds(2.0, 8.0) && l.holds(8.0, 2.0));
        assert!(!l.holds(8.0, 8.0));
    }
}
