//! NRes, the container every archive the game ships is. See `docs/01-nres.md`.

use std::path::Path;

use crate::cursor::{FormatError, latin1, u32_at};

pub const MAGIC: &[u8; 4] = b"NRes";
pub const HEADER_SIZE: usize = 16;
pub const ENTRY_SIZE: usize = 64;

/// One member of an archive: its directory record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// A FourCC tag, or a little-endian stream id in per-map files.
    pub type_raw: [u8; 4],
    pub name: String,
    pub offset: u32,
    pub size: u32,
    /// A member ordinal the engine cross-references; not the directory order.
    pub index: u32,
    /// Records in the payload where it is an array; a flags byte in `Material.lib`.
    pub element_count: u32,
    /// A second count; the record version in `Material.lib`.
    pub link_count: u32,
}

impl Entry {
    /// The FourCC as text, or `#12` for a numeric type id.
    pub fn tag(&self) -> String {
        if self.type_raw.iter().all(|b| (32..127).contains(b)) {
            latin1(&self.type_raw).trim().to_owned()
        } else {
            format!("#{}", self.type_id())
        }
    }

    pub fn type_id(&self) -> u32 {
        u32::from_le_bytes(self.type_raw)
    }
}

/// An archive held in memory.
#[derive(Clone)]
pub struct Archive {
    pub data: Vec<u8>,
    pub source: String,
    pub version: u32,
    pub entries: Vec<Entry>,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        Self::parse(std::fs::read(path)?, path.display().to_string())
    }

    pub fn parse(data: Vec<u8>, source: String) -> Result<Self, FormatError> {
        if data.get(..4) != Some(MAGIC.as_slice()) {
            return Err(FormatError::invalid(&source, "no NRes magic"));
        }
        let header = |at| u32_at(&data, at).ok_or_else(|| FormatError::invalid(&source, "short header"));
        let version = header(4)?;
        let count = header(8)? as usize;
        let declared = header(12)? as usize;
        if declared != data.len() {
            return Err(FormatError::invalid(
                &source,
                format!("header declares {declared} bytes, file is {}", data.len()),
            ));
        }
        let directory = data
            .len()
            .checked_sub(count * ENTRY_SIZE)
            .filter(|&d| d >= HEADER_SIZE)
            .ok_or_else(|| FormatError::invalid(&source, format!("{count} entries do not fit")))?;
        let entries = (0..count)
            .map(|i| {
                let rec = &data[directory + i * ENTRY_SIZE..directory + (i + 1) * ENTRY_SIZE];
                let word = |at| u32_at(rec, at).expect("inside a 64-byte record");
                let name = &rec[20..52];
                let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
                Entry {
                    type_raw: rec[0..4].try_into().expect("4 bytes"),
                    name: latin1(&name[..end]),
                    element_count: word(4),
                    link_count: word(8),
                    size: word(12),
                    offset: word(56),
                    index: word(60),
                }
            })
            .collect();
        Ok(Self { data, source, version, entries })
    }

    pub fn read(&self, entry: &Entry) -> Result<&[u8], FormatError> {
        let start = entry.offset as usize;
        self.data.get(start..start + entry.size as usize).ok_or_else(|| {
            FormatError::invalid(&self.source, format!("{} lies outside the file", entry.name))
        })
    }

    /// Look a member up by name, ignoring case as the game does.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn read_name(&self, name: &str) -> Result<&[u8], FormatError> {
        let entry = self
            .find(name)
            .ok_or_else(|| FormatError::invalid(&self.source, format!("no member named {name}")))?;
        self.read(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An archive of `(tag, name, payload)` members, laid out as the game does.
    pub fn build(members: &[([u8; 4], &str, &[u8])]) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        let mut records = Vec::new();
        for (i, (tag, name, payload)) in members.iter().enumerate() {
            let offset = out.len() as u32;
            out.extend_from_slice(payload);
            while !out.len().is_multiple_of(8) {
                out.push(0);
            }
            let mut rec = [0u8; ENTRY_SIZE];
            rec[0..4].copy_from_slice(tag);
            rec[4..8].copy_from_slice(&(i as u32 * 3).to_le_bytes());
            rec[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            rec[16..20].copy_from_slice(&1u32.to_le_bytes());
            rec[20..20 + name.len()].copy_from_slice(name.as_bytes());
            rec[56..60].copy_from_slice(&offset.to_le_bytes());
            rec[60..64].copy_from_slice(&(members.len() as u32 - i as u32).to_le_bytes());
            records.extend_from_slice(&rec);
        }
        out.extend_from_slice(&records);
        let total = out.len() as u32;
        out[0..4].copy_from_slice(MAGIC);
        out[4..8].copy_from_slice(&0x100u32.to_le_bytes());
        out[8..12].copy_from_slice(&(members.len() as u32).to_le_bytes());
        out[12..16].copy_from_slice(&total.to_le_bytes());
        out
    }

    #[test]
    fn the_directory_at_the_end_names_every_member() {
        let data = build(&[(*b"Texm", "grass.0", b"abc"), ([12, 0, 0, 0], "Land", b"0123456789")]);
        let archive = Archive::parse(data, "t".into()).unwrap();
        assert_eq!(archive.version, 0x100);
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.entries[0].tag(), "Texm");
        assert_eq!(archive.entries[1].tag(), "#12");
        assert_eq!(archive.entries[1].element_count, 3);
        assert_eq!(archive.entries[1].index, 1);
        assert_eq!(archive.read_name("LAND").unwrap(), b"0123456789");
        assert_eq!(archive.entries[1].offset % 8, 0);
    }

    #[test]
    fn a_wrong_size_or_magic_is_refused() {
        let mut data = build(&[(*b"Texm", "a", b"x")]);
        data.push(0);
        assert!(Archive::parse(data, "t".into()).is_err());
        assert!(Archive::parse(b"NReZ".to_vec(), "t".into()).is_err());
    }
}
