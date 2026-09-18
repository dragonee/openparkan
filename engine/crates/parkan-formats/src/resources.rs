//! Resource descriptors, how a name in a `.cfg` finds its bytes, and the PE string
//! tables the game's text lives in.
//!
//! Ports `openparkan.resources`. See `docs/20-resources.md`.

use std::collections::BTreeMap;

use crate::FormatError;
use crate::cfg::Blocks;
use crate::cursor::u32_at;

/// What a descriptor object says it is.
pub const RESOURCE: &str = "resource";
/// The keys that describe the library rather than bind a name.
pub const META: [&str; 4] = ["desc", "library", "libtype", "type"];
/// Win32 `RT_STRING`: a DLL-backed descriptor states it as its own `type`.
pub const RT_STRING: u32 = 6;
/// Strings per Win32 string block; block *n* holds ids `(n − 1) × 16 + i`.
pub const BLOCK: u32 = 16;
/// A sound descriptor's `type`: every one-shot, `sounds.lib` and `voices.lib`.
pub const SOUNDS: i64 = 4;
/// The music descriptor's `type`: only ever `ambient_music_loop`.
pub const MUSIC: i64 = 5;

/// One `desc = "resource"` object: a library and the names it binds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub role: String,
    pub library: String,
    pub libtype: String,
    pub type_id: Option<i64>,
    /// Every other key, in file order: a name against a member or an index.
    pub bindings: Vec<(String, String)>,
}

/// A number the way the reference reader recognises one: optional leading minus
/// signs, then ASCII digits.
fn integer(text: &str) -> Option<i64> {
    let digits = text.trim_start_matches('-');
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

impl Descriptor {
    /// The member or index bound to `name`: an exact key first, then ignoring case. A key
    /// written twice is already down to its later value, which is what the game binds
    /// (`cfg::parse`).
    pub fn get(&self, name: &str) -> Option<&str> {
        let exact = self.bindings.iter().find(|(k, _)| k == name);
        let lower = name.to_lowercase();
        exact
            .or_else(|| self.bindings.iter().find(|(k, _)| k.to_lowercase() == lower))
            .map(|(_, v)| v.as_str())
    }

    /// Whether the bindings are indices rather than member names.
    pub fn numbered(&self) -> bool {
        !self.bindings.is_empty()
            && self.bindings.iter().all(|(_, v)| {
                let d = v.trim_start_matches('-');
                !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
            })
    }
}

/// Every resource descriptor in a parsed `.cfg`, in file order.
pub fn descriptors(blocks: &Blocks) -> Vec<Descriptor> {
    blocks
        .0
        .iter()
        .filter(|b| b.get("desc") == Some(RESOURCE))
        .map(|b| Descriptor {
            role: b.name.clone(),
            library: b.get("library").unwrap_or("").to_owned(),
            libtype: b.get("libtype").unwrap_or("").to_owned(),
            type_id: b.get("type").and_then(integer),
            bindings: b.properties.iter().filter(|(k, _)| !META.contains(&k.as_str())).cloned().collect(),
        })
        .collect()
}

/// The first descriptor of `found`, in order, that binds `name`, with what it binds.
pub fn bound<'a>(found: &'a [Descriptor], name: &str) -> Option<(&'a Descriptor, &'a str)> {
    found.iter().find_map(|d| d.get(name).map(|v| (d, v)))
}

/// A little-endian word at `at`, or an error naming the image.
fn word(data: &[u8], at: usize, source: &str) -> Result<u32, FormatError> {
    u32_at(data, at).ok_or(FormatError::Truncated { source_name: source.to_owned(), offset: at })
}

fn half(data: &[u8], at: usize, source: &str) -> Result<u16, FormatError> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(FormatError::Truncated { source_name: source.to_owned(), offset: at })
}

/// A section: its virtual address, its size (the larger of virtual and raw), and where
/// its bytes start in the file.
type Section = (u32, u32, u32);

/// A PE image's sections, and the RVA of its resource directory.
fn sections(data: &[u8], source: &str) -> Result<(Vec<Section>, u32), FormatError> {
    if data.get(..2) != Some(b"MZ".as_slice()) {
        return Err(FormatError::invalid(source, "not a PE image: no MZ header"));
    }
    let lfanew = word(data, 0x3c, source)? as usize;
    if data.get(lfanew..lfanew + 4) != Some(b"PE\0\0".as_slice()) {
        return Err(FormatError::invalid(source, "not a PE image: no PE signature"));
    }
    let count = half(data, lfanew + 6, source)?;
    let optional_size = usize::from(half(data, lfanew + 20, source)?);
    let optional = lfanew + 24;
    let magic = half(data, optional, source)?;
    // The data directories follow the optional header's fixed part: 96 bytes for PE32,
    // 112 for PE32+, the count word last. The resource directory is the third.
    let directories = optional + if magic == 0x10b { 92 } else { 108 };
    let resource_rva = word(data, directories + 4 + 2 * 8, source)?;
    let table = (0..usize::from(count))
        .map(|i| {
            let at = optional + optional_size + i * 40;
            let (virtual_size, virtual_address) = (word(data, at + 8, source)?, word(data, at + 12, source)?);
            let (raw_size, raw_offset) = (word(data, at + 16, source)?, word(data, at + 20, source)?);
            Ok((virtual_address, virtual_size.max(raw_size), raw_offset))
        })
        .collect::<Result<Vec<_>, FormatError>>()?;
    Ok((table, resource_rva))
}

fn offset(sections: &[Section], rva: u32, source: &str) -> Result<usize, FormatError> {
    sections
        .iter()
        .find(|&&(va, size, _)| va <= rva && u64::from(rva) < u64::from(va) + u64::from(size))
        .map(|&(va, _, raw)| raw as usize + (rva - va) as usize)
        .ok_or_else(|| FormatError::invalid(source, format!("RVA {rva:#x} is in no section")))
}

/// One resource directory's `(id, offset)` entries, high bits as stored.
fn entries(data: &[u8], base: usize, at: usize, source: &str) -> Result<Vec<(u32, u32)>, FormatError> {
    let named = usize::from(half(data, base + at + 12, source)?);
    let numbered = usize::from(half(data, base + at + 14, source)?);
    (0..named + numbered)
        .map(|i| {
            Ok((word(data, base + at + 16 + i * 8, source)?, word(data, base + at + 20 + i * 8, source)?))
        })
        .collect()
}

/// Every `RT_STRING` in a PE image by id, under `language` or any. Unused slots, which
/// a block writes as zero lengths, are left out; a later block's string replaces an
/// earlier one of the same id.
pub fn strings(
    data: &[u8],
    language: Option<u32>,
    source: &str,
) -> Result<BTreeMap<u32, String>, FormatError> {
    let (sections, rva) = sections(data, source)?;
    let mut out = BTreeMap::new();
    if rva == 0 {
        return Ok(out);
    }
    let base = offset(&sections, rva, source)?;
    const SUBDIRECTORY: u32 = 0x8000_0000;
    for (type_id, entry) in entries(data, base, 0, source)? {
        if type_id != RT_STRING || entry & SUBDIRECTORY == 0 {
            continue;
        }
        for (block, sub) in entries(data, base, (entry & !SUBDIRECTORY) as usize, source)? {
            if sub & SUBDIRECTORY == 0 {
                continue;
            }
            for (lang, leaf) in entries(data, base, (sub & !SUBDIRECTORY) as usize, source)? {
                if language.is_some_and(|l| l != lang) {
                    continue;
                }
                let data_rva = word(data, base + leaf as usize, source)?;
                let size = word(data, base + leaf as usize + 4, source)? as usize;
                let mut at = offset(&sections, data_rva, source)?;
                let end = at + size;
                for i in 0..BLOCK {
                    if at + 2 > end {
                        break;
                    }
                    let count = usize::from(half(data, at, source)?);
                    at += 2;
                    if count > 0 {
                        let units = data
                            .get(at..at + count * 2)
                            .ok_or(FormatError::Truncated { source_name: source.to_owned(), offset: at })?
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|&c| u16::from_le_bytes(c))
                            .collect::<Vec<u16>>();
                        let text = String::from_utf16(&units).map_err(|_| {
                            FormatError::invalid(source, format!("string {} is not UTF-16", i))
                        })?;
                        out.insert(block.wrapping_sub(1).wrapping_mul(BLOCK).wrapping_add(i), text);
                    }
                    at += count * 2;
                }
            }
        }
    }
    Ok(out)
}

/// `TextRes.cfg`'s names joined with the string table of the DLL it names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextResources {
    pub names: BTreeMap<String, i64>,
    pub table: BTreeMap<u32, String>,
}

impl TextResources {
    /// The join of a descriptor's numbered bindings and a string table.
    pub fn new(descriptor: &Descriptor, table: BTreeMap<u32, String>) -> Self {
        let names = descriptor.bindings.iter().filter_map(|(k, v)| Some((k.clone(), integer(v)?))).collect();
        Self { names, table }
    }

    /// The text a resource name stands for, by its exact name; `None` if unbound.
    pub fn get(&self, name: &str) -> Option<&str> {
        let id = u32::try_from(*self.names.get(name)?).ok()?;
        self.table.get(&id).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg;

    const CFG: &str = "object\ttext_resources\r\n desc\t\t= \"resource\"\r\n \
        library\t= \"data\\TextRes.dll\"\r\n libtype\t= \"multi\"\r\n type\t\t= 6\r\n \
        T01_T01\t= 8\r\n T01_T02\t= 9\r\nend\r\n\r\nobject briefing_sounds\r\n desc = \"resource\"\r\n \
        library = \"voices.lib\"\r\n libtype = \"multi\"\r\n type = 4\r\n T01_T01 = t01_t01.wav\r\nend\r\n\
        object primary_objectives\r\n objective1 = \"1. Stay alive\"\r\nend\r\n";

    #[test]
    fn only_resource_objects_are_descriptors_and_meta_keys_are_not_bindings() {
        let found = descriptors(&cfg::parse(CFG.as_bytes()));
        let roles: Vec<&str> = found.iter().map(|d| d.role.as_str()).collect();
        assert_eq!(roles, ["text_resources", "briefing_sounds"]);
        let (text, sounds) = (&found[0], &found[1]);
        assert_eq!(text.bindings, vec![("T01_T01".into(), "8".into()), ("T01_T02".into(), "9".into())]);
        assert_eq!(
            (text.type_id, text.libtype.as_str(), text.library.as_str()),
            (Some(6), "multi", "data\\TextRes.dll")
        );
        assert!(text.numbered() && !sounds.numbered());
        assert_eq!(sounds.get("t01_t01"), Some("t01_t01.wav"));
        assert_eq!(sounds.get("nothing"), None);
        assert_eq!(
            bound(&found, "T01_T01").map(|(d, v)| (d.role.as_str(), v)),
            Some(("text_resources", "8"))
        );
    }

    #[test]
    fn a_key_written_twice_keeps_the_last_line_as_the_loaders_map_does() {
        const TWICE: &str = "object\tambient_music_variation\r\n desc = \"resource\"\r\n \
            library = \"sounds.lib\"\r\n libtype = \"multi\"\r\n type = 4\r\n \
            DAY_VARIATION1 = \"atm_bees.wav\"\r\n DAY_VARIATION1 = \"atm_bird2.wav\"\r\n \
            DAY_VARIATION2 = \"atm_bird3.wav\"\r\nend\r\n";
        let found = descriptors(&cfg::parse(TWICE.as_bytes()));
        let d = &found[0];
        // Three lines bind two names, and the repeat is already down to the later value.
        assert_eq!(d.bindings.len(), 2);
        assert_eq!(d.get("DAY_VARIATION1"), Some("atm_bird2.wav"));
        assert_eq!(d.get("day_variation1"), Some("atm_bird2.wav"), "ignoring case too");
        assert_eq!(d.get("DAY_VARIATION2"), Some("atm_bird3.wav"));
    }

    /// A PE32 image with one section holding a resource directory: `RT_STRING`, one
    /// block per `(block, [(slot, text)])`, all under language 1033.
    fn image(blocks: &[(u32, &[(usize, &str)])]) -> Vec<u8> {
        let (lfanew, optional_size, section_rva, raw) = (0x40usize, 224usize, 0x1000u32, 0x200usize);
        let mut out = vec![0u8; raw];
        out[..2].copy_from_slice(b"MZ");
        out[0x3c..0x40].copy_from_slice(&(lfanew as u32).to_le_bytes());
        out[lfanew..lfanew + 4].copy_from_slice(b"PE\0\0");
        out[lfanew + 6..lfanew + 8].copy_from_slice(&1u16.to_le_bytes());
        out[lfanew + 20..lfanew + 22].copy_from_slice(&(optional_size as u16).to_le_bytes());
        let optional = lfanew + 24;
        out[optional..optional + 2].copy_from_slice(&0x10bu16.to_le_bytes());
        out[optional + 96 + 16..optional + 100 + 16].copy_from_slice(&section_rva.to_le_bytes());

        // The resource section, laid out as root → type → blocks → languages → leaves → data.
        let mut res = Vec::<u8>::new();
        let dir = |res: &mut Vec<u8>, items: &[(u32, u32)]| {
            let at = res.len();
            res.extend_from_slice(&[0u8; 12]);
            res.extend_from_slice(&0u16.to_le_bytes());
            res.extend_from_slice(&(items.len() as u16).to_le_bytes());
            for (id, off) in items {
                res.extend_from_slice(&id.to_le_bytes());
                res.extend_from_slice(&off.to_le_bytes());
            }
            at
        };
        let n = blocks.len() as u32;
        let root = 16 + 8;
        let type_dir = root + 16 + 8 * n;
        let block_dirs: Vec<u32> = (0..n).map(|i| type_dir + (16 + 8) * i).collect();
        let leaves: Vec<u32> = (0..n).map(|i| type_dir + (16 + 8) * n + 16 * i).collect();
        let mut data_at = type_dir + (16 + 8) * n + 16 * n;
        dir(&mut res, &[(RT_STRING, 0x8000_0000 | root)]);
        dir(
            &mut res,
            &blocks.iter().zip(&block_dirs).map(|((b, _), d)| (*b, 0x8000_0000 | d)).collect::<Vec<_>>(),
        );
        for leaf in &leaves {
            dir(&mut res, &[(1033, *leaf)]);
        }
        let mut payloads = Vec::new();
        for (_, slots) in blocks {
            let mut p = Vec::new();
            for slot in 0..BLOCK as usize {
                match slots.iter().find(|(s, _)| *s == slot) {
                    Some((_, text)) => {
                        let units: Vec<u16> = text.encode_utf16().collect();
                        p.extend_from_slice(&(units.len() as u16).to_le_bytes());
                        units.iter().for_each(|u| p.extend_from_slice(&u.to_le_bytes()));
                    }
                    None => p.extend_from_slice(&0u16.to_le_bytes()),
                }
            }
            payloads.push(p);
        }
        for p in &payloads {
            res.extend_from_slice(&(section_rva + data_at).to_le_bytes());
            res.extend_from_slice(&(p.len() as u32).to_le_bytes());
            res.extend_from_slice(&[0u8; 8]);
            data_at += p.len() as u32;
        }
        payloads.iter().for_each(|p| res.extend_from_slice(p));

        let section = optional + optional_size;
        out[section + 8..section + 12].copy_from_slice(&(res.len() as u32).to_le_bytes());
        out[section + 12..section + 16].copy_from_slice(&section_rva.to_le_bytes());
        out[section + 16..section + 20].copy_from_slice(&(res.len() as u32).to_le_bytes());
        out[section + 20..section + 24].copy_from_slice(&(raw as u32).to_le_bytes());
        out.extend_from_slice(&res);
        out
    }

    #[test]
    fn a_string_table_reads_by_block_and_slot_and_skips_empty_slots() {
        let pe = image(&[
            (1, &[(8, "Tara, The Home Base."), (9, "Аskold")]),
            (316, &[(0, "Objective is completed")]),
        ]);
        let table = strings(&pe, None, "test").unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table[&8], "Tara, The Home Base.");
        assert_eq!(table[&9], "Аskold");
        assert_eq!(table[&5040], "Objective is completed");
        assert!(strings(&pe, Some(1049), "test").unwrap().is_empty());

        let text = &descriptors(&cfg::parse(CFG.as_bytes()))[0];
        let joined = TextResources::new(text, table);
        assert_eq!(joined.get("T01_T02"), Some("Аskold"));
        assert_eq!(joined.get("t01_t02"), None, "names are exact");
    }

    #[test]
    fn a_file_that_is_not_a_pe_says_so() {
        assert!(strings(b"NRes", None, "x").is_err());
    }
}
