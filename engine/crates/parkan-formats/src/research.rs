//! The research tree, `MISSIONS/SCRIPTS/*.trf`: an NRes archive of twelve streams tagged
//! `TRF0`..`TRFB`, one table in columns. Ports `openparkan/research.py`; see
//! `docs/16-research.md` and `docs/19-descriptions.md`.

use crate::cursor::{FormatError, latin1};
use crate::nres::Archive;

/// One `TRF0` record, and one `TRFB` entry.
pub const RECORD: usize = 40;
pub const PART: usize = 4;

/// `TRF1`'s three bits of state (`MisLoad.dll:0x10002aa0`).
pub const AVAILABLE: u8 = 0x1;
pub const RESEARCHED: u8 = 0x2;
pub const IN_TREE: u8 = 0x4;

/// Record `+0x23` and `+0x24`: the part's catalogue kind and sub-kind as numbers.
pub const KIND_BUILDING: u8 = 8;
pub const KIND_UNIT: u8 = 9;
pub const KIND_AMMO: u8 = 10;
pub const KIND_DEVICE: u8 = 11;
pub const KIND_WEAPON: u8 = 12;
pub const SUB_CHASSIS: u8 = 32;
pub const SUB_TURRET: u8 = 33;
pub const SUB_BUNKER: u8 = 17;
pub const SUB_ARMOUR: u8 = 68;
pub const SUB_BRAIN: u8 = 69;

/// A robot's `Type` words (docs/32-builder.md).
pub const TYPE_TRANSPORT: u32 = 0x0100_2000;
pub const TYPE_BUILDER: u32 = 0x0100_4000;
pub const TYPE_WARRIOR: u32 = 0x0100_8000;
pub const TYPE_HQ: u32 = 0x0101_0000;
pub const TYPE_HERO: u32 = 0x0102_0000;
pub const TYPE_ANIMAL: u32 = 0x2000_0000;

/// One researchable item.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub index: usize,
    /// `TRF8` and `TRF7`: the display name and the short code.
    pub name: String,
    pub code: String,
    /// `TRF1`: the state bits.
    pub category: u8,
    /// Research energy and ore, then build energy and ore.
    pub values: [f32; 4],
    pub requires: Vec<i32>,
    pub unlocks: Vec<i32>,
    /// The part ids `TRFB` maps onto the item.
    pub parts: Vec<String>,
    /// Record `+0x20`: its own entry in `TRFB`.
    pub part_index: u16,
    /// Record `+0x22`..`+0x27`: role, kind, sub-kind, branch, size, upgrade level.
    pub tail: [u8; 6],
    pub description: String,
    /// `TRFA`: the stat panel's template rows.
    pub template: String,
}

impl Item {
    pub fn role(&self) -> u8 {
        self.tail[0]
    }

    pub fn kind(&self) -> u8 {
        self.tail[1]
    }

    pub fn sub_kind(&self) -> u8 {
        self.tail[2]
    }

    pub fn size(&self) -> u8 {
        self.tail[4]
    }

    pub fn in_tree(&self) -> bool {
        self.category & IN_TREE != 0
    }

    pub fn researched(&self) -> bool {
        self.category & RESEARCHED != 0
    }

    pub fn available(&self) -> bool {
        self.category & AVAILABLE != 0
    }

    /// What building it costs: (energy, ore).
    pub fn build_cost(&self) -> (f32, f32) {
        (self.values[2], self.values[3])
    }

    /// The `Type` the item gives the assembly it heads, 0 for none (`iron3d.dll:0x1008a590`):
    /// a turret's by its role, a building's by its sub-kind or a bunker's by its size.
    pub fn object_type(&self) -> u32 {
        match (self.kind(), self.sub_kind()) {
            (KIND_UNIT, SUB_TURRET) => match self.role() {
                3 => TYPE_TRANSPORT,
                4 => TYPE_BUILDER,
                5 => TYPE_HQ,
                6 => TYPE_HERO,
                _ => TYPE_WARRIOR,
            },
            (KIND_BUILDING, SUB_BUNKER) => match self.size() {
                1 => 0x8001_0000,
                2 => 0x8002_0000,
                3 => 0x8004_0000,
                _ => 0,
            },
            (KIND_BUILDING, sub) => match sub {
                16 => 0x8000_0040,
                18 => 0x8000_0400,
                19 => 0x8000_0004,
                20 => 0x8000_0010,
                21 => 0x8000_0008,
                24 => 0x8000_0200,
                25 => 0x8000_1000,
                26 => 0x8000_0002,
                29 => 0x8010_0000,
                30 => 0x8020_0000,
                _ => 0,
            },
            _ => 0,
        }
    }
}

/// One `.trf`.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    pub items: Vec<Item>,
    /// Every part id in `TRFB` order.
    pub part_ids: Vec<String>,
    /// Each part id's item, in the same order.
    pub part_items: Vec<usize>,
}

impl Tree {
    /// The item that researches a part id, case-insensitively.
    pub fn item_for(&self, part: &str) -> Option<&Item> {
        let at = self.part_ids.iter().position(|p| p.eq_ignore_ascii_case(part))?;
        self.items.get(self.part_items[at])
    }
}

fn text(blob: &[u8], offset: i32) -> String {
    let Ok(start) = usize::try_from(offset) else { return String::new() };
    let Some(rest) = blob.get(start..) else { return String::new() };
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    latin1(&rest[..end])
}

fn words(blob: &[u8]) -> Vec<i32> {
    blob.as_chunks::<4>().0.iter().map(|w| i32::from_le_bytes(*w)).collect()
}

fn slices(counts: &[i32], flat: &[i32]) -> Vec<Vec<i32>> {
    let mut at = 0usize;
    counts
        .iter()
        .map(|&c| {
            let n = usize::try_from(c).unwrap_or(0);
            let out = flat.get(at..(at + n).min(flat.len())).unwrap_or(&[]).to_vec();
            at += n;
            out
        })
        .collect()
}

/// Read one `.trf`; an error unless its columns agree.
pub fn parse(data: &[u8], source: &str) -> Result<Tree, FormatError> {
    let archive = Archive::parse(data.to_vec(), source.to_owned())?;
    let stream = |tag: &str| archive.entries.iter().find(|e| e.tag() == tag).map(|e| archive.read(e));
    let required =
        |tag: &str| stream(tag).transpose()?.ok_or_else(|| FormatError::invalid(source, format!("no {tag}")));
    let (records, states, codes, names) =
        (required("TRF0")?, required("TRF1")?, required("TRF7")?, required("TRF8")?);
    let count = states.len();
    if records.len() != count * RECORD {
        return Err(FormatError::invalid(
            source,
            format!("TRF0 is {} bytes, not {count} x {RECORD}", records.len()),
        ));
    }
    let optional = |tag: &str| stream(tag).transpose().map(Option::unwrap_or_default);
    let graph = |counts: &str, flat: &str| -> Result<Vec<Vec<i32>>, FormatError> {
        let (c, f) = (optional(counts)?, optional(flat)?);
        if c.is_empty() || f.is_empty() {
            return Ok(vec![Vec::new(); count]);
        }
        let (c, f) = (words(c), words(f));
        if c.iter().map(|&n| i64::from(n)).sum::<i64>() != f.len() as i64 {
            return Err(FormatError::invalid(source, format!("{counts} does not count {flat}")));
        }
        Ok(slices(&c, &f))
    };
    let requires = graph("TRF2", "TRF3")?;
    let unlocks = graph("TRF4", "TRF5")?;
    let (table, part_names) = (optional("TRFB")?, optional("TRF6")?);
    let mut parts = vec![Vec::new(); count];
    let (mut part_ids, mut part_items) = (Vec::new(), Vec::new());
    for entry in table.as_chunks::<PART>().0 {
        let at = u16::from_le_bytes([entry[0], entry[1]]);
        let item = usize::from(u16::from_le_bytes([entry[2], entry[3]]));
        if item >= count || usize::from(at) >= part_names.len() {
            return Err(FormatError::invalid(source, format!("TRFB names item {item} at {at}")));
        }
        let part = text(part_names, i32::from(at));
        parts[item].push(part.clone());
        part_ids.push(part);
        part_items.push(item);
    }
    let (descriptions, templates) = (optional("TRF9")?, optional("TRFA")?);
    let items = (0..count)
        .map(|index| {
            let r = &records[index * RECORD..(index + 1) * RECORD];
            let f = |k: usize| f32::from_le_bytes(r[k * 4..k * 4 + 4].try_into().expect("4 bytes"));
            let i = |k: usize| i32::from_le_bytes(r[16 + k * 4..20 + k * 4].try_into().expect("4 bytes"));
            Item {
                index,
                name: text(names, i(1)),
                code: text(codes, i(0)),
                category: states[index],
                values: [f(0), f(1), f(2), f(3)],
                requires: requires[index].clone(),
                unlocks: unlocks[index].clone(),
                parts: std::mem::take(&mut parts[index]),
                part_index: u16::from_le_bytes([r[32], r[33]]),
                tail: std::array::from_fn(|k| r[34 + k]),
                description: text(descriptions, i(2)),
                template: text(templates, i(3)),
            }
        })
        .collect();
    Ok(Tree { items, part_ids, part_items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nres::{ENTRY_SIZE, HEADER_SIZE, MAGIC};

    /// An archive of `(tag, payload)` members, all named `ResTree`.
    fn build(members: &[([u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        let mut records = Vec::new();
        for (i, (tag, payload)) in members.iter().enumerate() {
            let offset = out.len() as u32;
            out.extend_from_slice(payload);
            let mut rec = [0u8; ENTRY_SIZE];
            rec[0..4].copy_from_slice(tag);
            rec[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            rec[20..27].copy_from_slice(b"ResTree");
            rec[56..60].copy_from_slice(&offset.to_le_bytes());
            rec[60..64].copy_from_slice(&(i as u32).to_le_bytes());
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

    fn record(values: [f32; 4], offsets: [i32; 4], part_index: u16, tail: [u8; 6]) -> Vec<u8> {
        let mut out = Vec::new();
        values.iter().for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
        offsets.iter().for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
        out.extend_from_slice(&part_index.to_le_bytes());
        out.extend_from_slice(&tail);
        out
    }

    #[test]
    fn a_tree_is_columns_with_parts_mapped_to_their_items() {
        let mut trf0 = record([1.0, 2.0, 3.0, 4.0], [0, 0, 0, 0], 1, [255, 9, 32, 255, 3, 0]);
        trf0.extend(record([0.0, 0.0, 5.0, 6.0], [4, 8, 0, 29], 0, [2, 9, 33, 255, 3, 0]));
        let trf1 = [7u8, 4];
        let trf6 = b"e_tur_bb_01\0R_B_02\0";
        let mut trfb = Vec::new();
        for (at, item) in [(12u16, 0u16), (0, 1)] {
            trfb.extend_from_slice(&at.to_le_bytes());
            trfb.extend_from_slice(&item.to_le_bytes());
        }
        let bytes = build(&[
            (*b"TRF0", &trf0),
            (*b"TRF1", &trf1),
            (*b"TRF6", trf6),
            (*b"TRF7", b"L2f\x004L1\0"),
            (*b"TRF8", b"Chassis\0Turret\0"),
            (*b"TRFA", b"@G@Weight @B,weight,G,t,5,1@\0"),
            (*b"TRFB", &trfb),
        ]);
        let tree = parse(&bytes, "t.trf").unwrap();
        assert_eq!(tree.part_ids, vec!["R_B_02", "e_tur_bb_01"]);
        let chassis = tree.item_for("r_b_02").unwrap();
        assert_eq!((chassis.name.as_str(), chassis.code.as_str()), ("Chassis", "L2f"));
        assert!(chassis.in_tree() && chassis.researched() && chassis.available());
        assert_eq!(chassis.build_cost(), (3.0, 4.0));
        let turret = tree.item_for("E_TUR_BB_01").unwrap();
        assert_eq!((turret.name.as_str(), turret.object_type()), ("Turret", TYPE_WARRIOR));
        assert!(turret.in_tree() && !turret.researched());
        assert_eq!(turret.template, "");
        assert_eq!(chassis.template, "@G@Weight @B,weight,G,t,5,1@");
    }
}
