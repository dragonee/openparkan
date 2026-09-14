//! RsLi, the other archive: `gamefont.rlb` and `sprites.lib`. See `docs/12-rsli.md`.
//!
//! A 32-byte header, then an entry table whose every byte is XORed with a
//! keystream from two bytes of state, then the members. The shipped data stores
//! members two ways: raw Deflate (`sprites.lib`) and an LZSS whose match offset is
//! an absolute index into a ring that starts at 0xFEE full of spaces (`gamefont.rlb`).

use std::path::Path;

use crate::cursor::{FormatError, latin1, u32_at};

pub const MAGIC: &[u8; 2] = b"NL";
pub const VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 32;
pub const ENTRY_SIZE: usize = 32;
/// Set in the header when the entry table is already in order.
pub const PRESORTED: u16 = 0xABBA;

/// Storage methods: the engine defines seven, the shipped data uses these.
pub const STORE_RAW: i16 = 0x000;
pub const STORE_LZSS: i16 = 0x040;
pub const STORE_DEFLATE: i16 = 0x100;

/// The LZSS ring: its size, the shortest match, where writing starts and what fills it.
pub const LZSS_WINDOW: usize = 4096;
pub const LZSS_MIN_MATCH: usize = 3;
pub const LZSS_START: usize = 0xFEE;
pub const LZSS_FILL: u8 = 0x20;

/// One member's record, decrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    /// The storage method.
    pub flags: i16,
    /// Where this entry sat before the table was sorted.
    pub order: i16,
    pub size: u32,
    pub offset: u32,
    pub packed: u32,
}

/// An archive held in memory, its entry table decrypted.
pub struct Archive {
    pub data: Vec<u8>,
    pub source: String,
    pub presorted: bool,
    /// The unpacked sizes summed.
    pub total: u32,
    /// The entry table's cipher key.
    pub seed: u32,
    pub entries: Vec<Entry>,
}

/// Whether `data` begins with the RsLi header.
pub fn is_rsli(data: &[u8]) -> bool {
    data.get(..4) == Some([MAGIC[0], MAGIC[1], 0, VERSION].as_slice())
}

/// Undo the entry table's stream cipher; it is its own inverse.
pub fn decrypt_table(cipher: &[u8], seed: u32) -> Vec<u8> {
    let mut a = (seed & 0xFF) as u8;
    let mut d = ((seed >> 8) & 0xFF) as u8;
    cipher
        .iter()
        .map(|&byte| {
            a = (a << 1) ^ d;
            d >>= 1;
            d ^= a;
            byte ^ a
        })
        .collect()
}

/// Unpack a `STORE_LZSS` member to at most `size` bytes: a flag byte, then eight
/// items least significant bit first, a set bit a literal and a clear bit a two-byte
/// match whose low byte and high nibble index the ring and whose low nibble + 3 is
/// its length.
pub fn unpack_lzss(data: &[u8], size: usize) -> Vec<u8> {
    let mut ring = [LZSS_FILL; LZSS_WINDOW];
    let mut pos = LZSS_START;
    let mut out = Vec::with_capacity(size);
    let mut src = 0;
    let (mut flags, mut left) = (0u8, 0u8);
    let put = |byte: u8, ring: &mut [u8; LZSS_WINDOW], out: &mut Vec<u8>, pos: &mut usize| {
        ring[*pos] = byte;
        *pos = (*pos + 1) % LZSS_WINDOW;
        out.push(byte);
    };
    while out.len() < size {
        if left == 0 {
            let Some(&f) = data.get(src) else { break };
            flags = f;
            src += 1;
            left = 8;
        }
        let literal = flags & 1 != 0;
        flags >>= 1;
        left -= 1;
        if literal {
            let Some(&byte) = data.get(src) else { break };
            src += 1;
            put(byte, &mut ring, &mut out, &mut pos);
            continue;
        }
        let (Some(&low), Some(&high)) = (data.get(src), data.get(src + 1)) else { break };
        src += 2;
        let run = usize::from(high & 0x0F) + LZSS_MIN_MATCH;
        let mut at = usize::from(low) | (usize::from(high >> 4) << 8);
        for _ in 0..run {
            if out.len() >= size {
                break;
            }
            let byte = ring[at];
            at = (at + 1) % LZSS_WINDOW;
            put(byte, &mut ring, &mut out, &mut pos);
        }
    }
    out
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        Self::parse(std::fs::read(path)?, path.display().to_string())
    }

    pub fn parse(data: Vec<u8>, source: String) -> Result<Self, FormatError> {
        if data.len() < HEADER_SIZE || data[..2] != MAGIC[..] {
            return Err(FormatError::invalid(&source, "not an RsLi archive"));
        }
        if data[3] != VERSION {
            return Err(FormatError::invalid(&source, format!("version {}, expected {VERSION}", data[3])));
        }
        let i16_at = |at: usize| i16::from_le_bytes([data[at], data[at + 1]]);
        let (count, again) = (i16_at(4), i16_at(6));
        if count < 0 || count != again {
            return Err(FormatError::invalid(&source, format!("entry count {count} / {again}")));
        }
        let count = count as usize;
        let presorted = u16::from_le_bytes([data[0x0E], data[0x0F]]) == PRESORTED;
        let total = u32_at(&data, 0x10).expect("inside the header");
        let seed = u32_at(&data, 0x14).expect("inside the header");
        let end = HEADER_SIZE + count * ENTRY_SIZE;
        let cipher = data
            .get(HEADER_SIZE..end)
            .ok_or_else(|| FormatError::invalid(&source, format!("{count} entries do not fit")))?;
        let table = decrypt_table(cipher, seed);
        let entries = table
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .map(|rec| {
                let name = &rec[..12];
                let stop = name.iter().position(|&b| b == 0).unwrap_or(name.len());
                let word = |at| u32_at(rec, at).expect("inside a 32-byte record");
                Entry {
                    name: latin1(&name[..stop]),
                    flags: i16::from_le_bytes([rec[0x10], rec[0x11]]),
                    order: i16::from_le_bytes([rec[0x12], rec[0x13]]),
                    size: word(0x14),
                    offset: word(0x18),
                    packed: word(0x1C),
                }
            })
            .collect();
        Ok(Self { data, source, presorted, total, seed, entries })
    }

    /// Unpack one member. A packed run may end past the file, as `INTERF8.TEX`'s does
    /// by a byte: the read stops at the end.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>, FormatError> {
        let start = (entry.offset as usize).min(self.data.len());
        let stop = (start + entry.packed as usize).min(self.data.len());
        let raw = &self.data[start..stop];
        if raw.is_empty() {
            return Err(FormatError::invalid(&self.source, format!("{} is outside the file", entry.name)));
        }
        let size = entry.size as usize;
        let out = match entry.flags {
            STORE_RAW => raw[..size.min(raw.len())].to_vec(),
            STORE_LZSS => unpack_lzss(raw, size),
            STORE_DEFLATE => miniz_oxide::inflate::decompress_to_vec(raw).map_err(|e| {
                FormatError::invalid(&self.source, format!("{} does not inflate: {e:?}", entry.name))
            })?,
            other => {
                return Err(FormatError::invalid(
                    &self.source,
                    format!("{} uses storage {other:#x}, which is not implemented", entry.name),
                ));
            }
        };
        if out.len() != size {
            return Err(FormatError::invalid(
                &self.source,
                format!("{} unpacked to {} bytes, not the {size} it declares", entry.name, out.len()),
            ));
        }
        Ok(out)
    }

    /// Look a member up by name, ignoring case.
    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn read_name(&self, name: &str) -> Result<Vec<u8>, FormatError> {
        let entry = self
            .find(name)
            .ok_or_else(|| FormatError::invalid(&self.source, format!("no member named {name}")))?;
        self.read(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keystream a table of `n` bytes is XORed with, straight from the doc's loop.
    fn keystream(seed: u32, n: usize) -> Vec<u8> {
        decrypt_table(&vec![0; n], seed)
    }

    #[test]
    fn the_cipher_runs_across_records_and_undoes_itself() {
        let seed = 0x1234_5678;
        let ks = keystream(seed, 4);
        // a = 0x78, d = 0x56: a = (0xF0 ^ 0x56) = 0xA6, d = 0x2B ^ 0xA6 = 0x8D; then
        // a = (0x4C ^ 0x8D) = 0xC1, d = 0x46 ^ 0xC1 = 0x87.
        assert_eq!(&ks[..2], &[0xA6, 0xC1]);
        let plain: Vec<u8> = (0..64).collect();
        let cipher = decrypt_table(&plain, seed);
        assert_ne!(cipher, plain);
        assert_eq!(decrypt_table(&cipher, seed), plain);
    }

    #[test]
    fn lzss_matches_index_a_ring_that_starts_full_of_spaces() {
        // Flags 0b0000_0011: two literals, then a match. The match reads 4 bytes from
        // ring index 0xFEE, where the literals were written.
        let data = [0b0000_0011, b'a', b'b', 0xEE, 0xF1];
        assert_eq!(unpack_lzss(&data, 6), b"ababab");
        // A match at index 0 before anything is written there reads the spaces.
        assert_eq!(unpack_lzss(&[0x00, 0x00, 0x00], 3), b"   ");
        // Output stops at the declared size, mid-match.
        assert_eq!(unpack_lzss(&data, 4), b"abab");
    }

    fn build(members: &[(&str, i16, &[u8], u32)]) -> Vec<u8> {
        let seed: u32 = 0x0000_9A3C;
        let mut table = Vec::new();
        let mut payload = Vec::new();
        let base = HEADER_SIZE + members.len() * ENTRY_SIZE;
        for (i, (name, flags, bytes, size)) in members.iter().enumerate() {
            let mut rec = [0u8; ENTRY_SIZE];
            rec[..name.len()].copy_from_slice(name.as_bytes());
            rec[0x10..0x12].copy_from_slice(&flags.to_le_bytes());
            rec[0x12..0x14].copy_from_slice(&(i as i16).to_le_bytes());
            rec[0x14..0x18].copy_from_slice(&size.to_le_bytes());
            rec[0x18..0x1C].copy_from_slice(&((base + payload.len()) as u32).to_le_bytes());
            rec[0x1C..0x20].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
            table.extend_from_slice(&rec);
            payload.extend_from_slice(bytes);
        }
        let mut out = vec![0u8; HEADER_SIZE];
        out[..4].copy_from_slice(&[b'N', b'L', 0, VERSION]);
        out[4..6].copy_from_slice(&(members.len() as i16).to_le_bytes());
        out[6..8].copy_from_slice(&(members.len() as i16).to_le_bytes());
        out[0x14..0x18].copy_from_slice(&seed.to_le_bytes());
        out.extend_from_slice(&decrypt_table(&table, seed));
        out.extend_from_slice(&payload);
        out
    }

    #[test]
    fn members_decrypt_and_unpack_by_their_storage() {
        let lzss = [0b0000_0011, b'a', b'b', 0xEE, 0xF1];
        let deflated = [0x4b, 0x4c, 0x4a, 0x06, 0x00]; // raw Deflate of "abc"
        let data = build(&[
            ("PAL.PAL", STORE_LZSS, &lzss, 6),
            ("RAW.BIN", STORE_RAW, b"xyz", 3),
            ("SPRITE.TEX", STORE_DEFLATE, &deflated, 3),
        ]);
        assert!(is_rsli(&data));
        let archive = Archive::parse(data, "t".into()).unwrap();
        assert_eq!(archive.entries[0].name, "PAL.PAL");
        assert_eq!(archive.entries[2].order, 2);
        assert_eq!(archive.read_name("pal.pal").unwrap(), b"ababab");
        assert_eq!(archive.read_name("RAW.BIN").unwrap(), b"xyz");
        assert_eq!(archive.read_name("SPRITE.TEX").unwrap(), b"abc");
    }

    #[test]
    fn a_wrong_header_or_size_is_refused() {
        assert!(Archive::parse(b"NRes0000000000000000000000000000".to_vec(), "t".into()).is_err());
        let data = build(&[("A", STORE_RAW, b"xyz", 4)]);
        let archive = Archive::parse(data, "t".into()).unwrap();
        assert!(archive.read_name("A").is_err(), "3 bytes where 4 are declared");
    }
}
