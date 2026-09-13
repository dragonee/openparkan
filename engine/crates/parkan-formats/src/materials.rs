//! `Material.lib`: the `MAT0` records a mesh's wear and a terrain layer name.
//! See `docs/07-objects.md` and `openparkan/materials.py`.

use std::collections::HashMap;
use std::path::Path;

use crate::cursor::{FormatError, latin1};
use crate::nres::Archive;

pub const TAG: &str = "MAT0";
pub const HEADER_SIZE: usize = 14;
pub const ENTRY_STRIDE: usize = 34;
pub const TRACK_HEADER: usize = 6;
pub const KEY_STRIDE: usize = 6;
pub const WHOLE_TEXTURE: i8 = -1;
pub const UNSET: u8 = 0xFF;

/// The directory flags byte's blend index goes through this table to a mode.
pub const BLEND_TRANSLATE: [u8; 5] = [0, 4, 2, 3, 5];

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub texture: String,
    pub cell: i8,
    pub ambient: [u8; 3],
    pub diffuse: [u8; 3],
    pub specular: [u8; 3],
    pub emissive: [u8; 3],
    /// Ambient, diffuse, specular and emissive alpha, each 0..100 per cent.
    pub alphas: [u8; 4],
    pub power: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub entry: u16,
    pub time: u16,
    pub unread: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub word: u32,
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    /// The archive directory's flags byte: how the material draws.
    pub blend: u32,
    pub entries: Vec<Entry>,
    pub tracks: Vec<Track>,
    /// The ground's surface id, 0..10 or `UNSET`.
    pub surface: u8,
    pub speed_factor: f32,
    pub damage_rate: f32,
}

impl Material {
    /// The engine's alpha-blend mode id, or `None` if the index is unused.
    pub fn blend_mode(&self) -> Option<u8> {
        BLEND_TRANSLATE.get(((self.blend >> 2) & 0xF) as usize).copied()
    }

    /// Track 0's texture per key: the animation, or the one still frame.
    pub fn frames(&self) -> Vec<&str> {
        let named: Vec<&str> = self
            .tracks
            .first()
            .map(|t| {
                t.keys
                    .iter()
                    .filter_map(|k| self.entries.get(usize::from(k.entry)))
                    .map(|e| e.texture.as_str())
                    .filter(|n| !n.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        if named.is_empty() {
            self.entries.iter().map(|e| e.texture.as_str()).filter(|n| !n.is_empty()).take(1).collect()
        } else {
            named
        }
    }

    /// The entry track 0's first key names: what the engine draws a still material with.
    pub fn first_entry(&self) -> Option<&Entry> {
        let at = self.tracks.first().and_then(|t| t.keys.first()).map_or(0, |k| usize::from(k.entry));
        self.entries.get(at).or(self.entries.first())
    }
}

fn colour(b: &[u8], at: usize) -> [u8; 3] {
    [b[at], b[at + 1], b[at + 2]]
}

/// Parse one record. `blend` is its directory entry's first count field.
pub fn parse(name: &str, data: &[u8], blend: u32) -> Result<Material, FormatError> {
    let short = || FormatError::invalid(name, "short material record");
    let head = data.get(..HEADER_SIZE).ok_or_else(short)?;
    let count = usize::from(u16::from_le_bytes([head[0], head[1]]));
    let track_count = usize::from(u16::from_le_bytes([head[2], head[3]]));
    let surface = head[4];
    let speed_factor = f32::from_le_bytes(head[6..10].try_into().expect("4 bytes"));
    let damage_rate = f32::from_le_bytes(head[10..14].try_into().expect("4 bytes"));
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let at = HEADER_SIZE + i * ENTRY_STRIDE;
        let Some(b) = data.get(at..at + ENTRY_STRIDE) else { break };
        let name = &b[18..];
        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        entries.push(Entry {
            texture: latin1(&name[..end]),
            cell: b[17] as i8,
            ambient: colour(b, 0),
            diffuse: colour(b, 4),
            specular: colour(b, 8),
            emissive: colour(b, 12),
            alphas: [b[3], b[7], b[11], b[15]],
            power: b[16],
        });
    }
    let mut at = HEADER_SIZE + count * ENTRY_STRIDE;
    let mut tracks = Vec::with_capacity(track_count);
    for _ in 0..track_count {
        let Some(h) = data.get(at..at + TRACK_HEADER) else { break };
        let word = u32::from_le_bytes(h[..4].try_into().expect("4 bytes"));
        let keys = usize::from(u16::from_le_bytes([h[4], h[5]]));
        at += TRACK_HEADER;
        let mut track = Track { word, keys: Vec::with_capacity(keys) };
        for _ in 0..keys {
            let Some(k) = data.get(at..at + KEY_STRIDE) else { break };
            let w = |o: usize| u16::from_le_bytes([k[o], k[o + 1]]);
            track.keys.push(Key { entry: w(0), time: w(2), unread: w(4) });
            at += KEY_STRIDE;
        }
        tracks.push(track);
    }
    Ok(Material { name: name.to_owned(), blend, entries, tracks, surface, speed_factor, damage_rate })
}

/// `Material.lib`, keyed by upper-case name.
pub struct Library {
    pub materials: Vec<Material>,
    by_name: HashMap<String, usize>,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        let archive = Archive::open(path)?;
        let mut materials = Vec::new();
        for entry in archive.entries.iter().filter(|e| e.tag() == TAG) {
            materials.push(parse(&entry.name, archive.read(entry)?, entry.element_count)?);
        }
        let by_name = materials.iter().enumerate().map(|(i, m)| (m.name.to_ascii_uppercase(), i)).collect();
        Ok(Self { materials, by_name })
    }

    /// A material by name, ignoring case and any `.0` suffix.
    pub fn get(&self, name: &str) -> Option<&Material> {
        let key = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        self.by_name.get(&key).map(|&i| &self.materials[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_reads_its_entries_tracks_and_ground_fields() {
        let mut data = Vec::new();
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[1, UNSET]);
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&10000.0f32.to_le_bytes());
        for (tex, diffuse) in [("L20.0", 255u8), ("L20M.0", 0)] {
            let mut e = [0u8; ENTRY_STRIDE];
            e[4..7].copy_from_slice(&[diffuse; 3]);
            e[17] = WHOLE_TEXTURE as u8;
            e[18..18 + tex.len()].copy_from_slice(tex.as_bytes());
            data.extend_from_slice(&e);
        }
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[1, 0, 0, 0, 0, 0]);
        let m = parse("WATER_BOT", &data, 8).unwrap();
        assert_eq!(m.entries[1].texture, "L20M.0");
        assert_eq!(m.frames(), vec!["L20M.0"]);
        assert_eq!(m.first_entry().unwrap().diffuse, [0, 0, 0]);
        assert_eq!((m.surface, m.damage_rate), (1, 10000.0));
        assert_eq!(m.blend_mode(), Some(2));
    }
}
