//! A building mesh's hall way: stream 17, the path graph through its inside, whose
//! vertices' flag words make some of them places. See `docs/07-objects.md`, "Buildings
//! carry an interior path graph", and `docs/27-ownership.md`, "The places".

use crate::FormatError;
use crate::nres::Archive;

/// The mesh stream the hall way is.
pub const STREAM_HALL_WAY: u32 = 17;
pub const VERTEX_SIZE: usize = 20;
pub const LINK_SIZE: usize = 40;
/// A factory's creation places: the first vertex with `0x800`, else `0x80`
/// (`Behavior.dll:0x100299a0`, docs/36).
pub const PLACE_CREATION: u32 = 0x800;
pub const PLACE_CREATION_OLD: u32 = 0x80;
/// The control pod a capturer walks to.
pub const PLACE_POD: u32 = 0x40;
/// A mine's loading place and a storage's unloading place, where a transport takes and leaves
/// ore (docs/32, "Transporting ore").
pub const PLACE_LOADING: u32 = 0x8;
pub const PLACE_UNLOADING: u32 = 0x10;
/// The size gate a vertex puts on a unit (`Behavior.dll:0x10042d08`, docs/24, "The global
/// path"): [`VERTEX_ANY_SIZE`] lets any unit through, [`VERTEX_BUILDING_SIZE`] one no bigger
/// than the building, and a vertex with neither only a unit of size class 2 or less.
pub const VERTEX_ANY_SIZE: u32 = 0x1000_0000;
pub const VERTEX_BUILDING_SIZE: u32 = 0x2000_0000;
/// The link tail's two words the search's link flags come from: `+0xc` and `+0x1c` of the
/// 56-byte link the loader builds, which are the file's tail words 1 and 5
/// (`ArealMap.dll:0x1000a274`, `0x1000a294`).
pub const LINK_GATE: [usize; 2] = [1, 5];
/// A link only a flyer crosses (`Behavior.dll:0x10036934`), and one nothing crosses
/// (`0x10042c90`).
pub const LINK_FLYER_ONLY: u32 = 0x1_0000;
pub const LINK_SHUT: u32 = 0x2_0000;

/// A vertex: its point in its joint node's frame, its flag word, and the joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub flags: u32,
    pub joint: u32,
}

/// A link between two vertices, and the eight further words the loader copies after them
/// (`ArealMap.dll:0x1000a56f`: a 56-byte record, the file's forty bytes at its front).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link {
    pub start: u32,
    pub end: u32,
    pub tail: [u32; 8],
}

impl Link {
    /// The flags the search gives this link (`ArealMap.dll:0x1000a274`, `0x1000a294`):
    /// [`LINK_SHUT`] where both gate words are zero, [`LINK_FLYER_ONLY`] where the first is
    /// zero and the second is not.
    pub fn flags(&self) -> u32 {
        match (self.tail[LINK_GATE[0]], self.tail[LINK_GATE[1]]) {
            (0, 0) => LINK_SHUT,
            (0, _) => LINK_FLYER_ONLY,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HallWay {
    pub vertices: Vec<Vertex>,
    pub links: Vec<Link>,
}

impl HallWay {
    /// The first vertex with any bit of `mask`.
    pub fn first(&self, mask: u32) -> Option<&Vertex> {
        self.vertices.iter().find(|v| v.flags & mask != 0)
    }
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// The hall way of a `MESH` payload, empty when the mesh has none. Its counts are the
/// stream entry's element count and the field after it.
pub fn parse(blob: &[u8], name: &str) -> Result<HallWay, FormatError> {
    let inner = Archive::parse(blob.to_vec(), name.to_owned())?;
    let Some(entry) = inner.entries.iter().rev().find(|e| e.type_id() == STREAM_HALL_WAY) else {
        return Ok(HallWay::default());
    };
    let data = inner.read(entry)?;
    let (n, l) = (entry.element_count as usize, entry.link_count as usize);
    if data.len() != n * VERTEX_SIZE + l * LINK_SIZE {
        return Err(FormatError::invalid(name, "hall way size does not match its counts"));
    }
    let vertices = (0..n)
        .map(|i| {
            let at = i * VERTEX_SIZE;
            let f = |k: usize| f32::from_le_bytes(u32_at(data, at + 4 * k).to_le_bytes());
            Vertex {
                position: [f(0), f(1), f(2)],
                flags: u32_at(data, at + 12),
                joint: u32_at(data, at + 16),
            }
        })
        .collect();
    let base = n * VERTEX_SIZE;
    let links = (0..l)
        .map(|i| {
            let at = base + i * LINK_SIZE;
            let mut tail = [0u32; 8];
            for (k, slot) in tail.iter_mut().enumerate() {
                *slot = u32_at(data, at + 8 + 4 * k);
            }
            Link { start: u32_at(data, at), end: u32_at(data, at + 4), tail }
        })
        .collect();
    Ok(HallWay { vertices, links })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hall_way_reads_its_vertices_and_links_by_the_entrys_counts() {
        let mut data = Vec::new();
        for (p, flags, joint) in [([1.0f32, 2.0, 3.0], 0x2000_0800u32, 4u32), ([0.0, -1.0, 0.5], 0x40, 25)] {
            for v in p {
                data.extend(v.to_le_bytes());
            }
            data.extend(flags.to_le_bytes());
            data.extend(joint.to_le_bytes());
        }
        data.extend(0u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend([0xffu8; 32]);
        // An NRes of one stream: the header, the payload padded to 8, one 64-byte entry with
        // the element count at +4 and the link count at +8.
        let mut blob = vec![0u8; 16];
        blob.extend(&data);
        while !blob.len().is_multiple_of(8) {
            blob.push(0);
        }
        let mut entry = [0u8; 64];
        entry[0..4].copy_from_slice(&STREAM_HALL_WAY.to_le_bytes());
        entry[4..8].copy_from_slice(&2u32.to_le_bytes());
        entry[8..12].copy_from_slice(&1u32.to_le_bytes());
        entry[12..16].copy_from_slice(&(data.len() as u32).to_le_bytes());
        entry[56..60].copy_from_slice(&16u32.to_le_bytes());
        blob.extend(entry);
        let total = blob.len() as u32;
        blob[0..4].copy_from_slice(b"NRes");
        blob[4..8].copy_from_slice(&0x100u32.to_le_bytes());
        blob[8..12].copy_from_slice(&1u32.to_le_bytes());
        blob[12..16].copy_from_slice(&total.to_le_bytes());
        let h = parse(&blob, "test").unwrap();
        assert_eq!(h.vertices.len(), 2);
        assert_eq!(h.first(PLACE_CREATION).map(|v| (v.position, v.joint)), Some(([1.0, 2.0, 3.0], 4)));
        assert_eq!(h.first(PLACE_POD).map(|v| v.joint), Some(25));
        assert_eq!(h.links, vec![Link { start: 0, end: 1, tail: [0xffff_ffff; 8] }]);
        assert_eq!(h.links[0].flags(), 0, "a tail of -1 throughout gates nothing");
    }

    #[test]
    fn a_links_gate_words_give_it_its_flags() {
        let link = |a: u32, b: u32| {
            let mut tail = [0xffff_ffffu32; 8];
            tail[LINK_GATE[0]] = a;
            tail[LINK_GATE[1]] = b;
            Link { start: 0, end: 1, tail }
        };
        assert_eq!(link(0xffff_ffff, 0xffff_ffff).flags(), 0);
        // The eighteen shipped flyer-only links, on the three mines and the three factories.
        assert_eq!(link(0, 0xffff_ffff).flags(), LINK_FLYER_ONLY);
        assert_eq!(link(1, 1).flags(), 0, "the door links gate nothing");
        assert_eq!(link(0, 0).flags(), LINK_SHUT, "no shipped link is shut");
    }
}
