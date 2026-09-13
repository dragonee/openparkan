//! `objects.rlb` records and `UNITS/**/*.dat` assemblies. See `docs/07-objects.md`.

use std::collections::HashMap;
use std::path::Path;

use crate::cursor::{FormatError, latin1, u32_at};
use crate::nres::Archive;

pub const SLOT_SIZE: usize = 64;
pub const NAME_FIELD: usize = 32;
pub const DAT_MAGIC: u32 = 0xF0F1;
pub const DAT_HEADER: usize = 8;
pub const DAT_COMPONENT: usize = 112;

pub const CLASS_CHASSIS: u32 = 0;
pub const CLASS_TURRET: u32 = 1;
pub const CLASS_GUN: u32 = 4;

pub(crate) fn fixed(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    latin1(&raw[..end])
}

/// An `(archive, member)` pair.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResourceRef {
    pub library: String,
    pub member: String,
}

impl ResourceRef {
    pub fn is_set(&self) -> bool {
        !self.library.is_empty() && !self.member.is_empty()
    }

    /// The member's extension in lower case, or "".
    pub fn suffix(&self) -> String {
        self.member.rsplit_once('.').map(|(_, s)| s.to_ascii_lowercase()).unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub name: String,
    pub tag: String,
    pub slots: Vec<ResourceRef>,
}

impl Record {
    pub fn slot_with_suffix(&self, suffix: &str) -> Option<&ResourceRef> {
        self.slots.iter().find(|s| s.is_set() && s.suffix() == suffix)
    }

    pub fn mesh(&self) -> Option<&ResourceRef> {
        self.slot_with_suffix("msh")
    }
}

/// `objects.rlb`, keyed by lower-case name.
pub struct Library {
    records: HashMap<String, Record>,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self, FormatError> {
        let archive = Archive::open(path)?;
        let mut records = HashMap::new();
        for entry in &archive.entries {
            let data = archive.read(entry)?;
            let slots = data
                .as_chunks::<SLOT_SIZE>()
                .0
                .iter()
                .map(|s| ResourceRef { library: fixed(&s[..NAME_FIELD]), member: fixed(&s[NAME_FIELD..]) })
                .collect();
            records.insert(
                entry.name.to_ascii_lowercase(),
                Record { name: entry.name.clone(), tag: entry.tag(), slots },
            );
        }
        Ok(Self { records })
    }

    pub fn get(&self, name: &str) -> Option<&Record> {
        self.records.get(&name.to_ascii_lowercase())
    }

    /// A record's `.msh`, following a FORT's first mesh-less slot to the
    /// record that carries one, at most three hops.
    pub fn record_mesh(&self, record: Option<&Record>, depth: usize) -> Option<ResourceRef> {
        let record = record.filter(|_| depth <= 3)?;
        if let Some(mesh) = record.mesh() {
            return Some(mesh.clone());
        }
        record
            .slots
            .iter()
            .filter(|s| s.is_set() && s.suffix().is_empty())
            .find_map(|s| self.record_mesh(self.get(&s.member), depth + 1))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub reference: ResourceRef,
    pub label: String,
    pub flags: u32,
    /// The parent mesh node an external part bolts onto; -1 on the chassis.
    pub attach_node: i32,
    pub class_id: u32,
    pub child_count: i32,
}

impl Component {
    /// Chassis, turrets and guns are drawn outside the hull.
    pub fn is_external(&self) -> bool {
        matches!(self.class_id, CLASS_CHASSIS | CLASS_TURRET | CLASS_GUN)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub kind: u32,
    pub components: Vec<Component>,
}

impl Unit {
    /// Each component's parent in the depth-first tree; -1 for the root.
    pub fn parents(&self) -> Result<Vec<i32>, FormatError> {
        let mut parent = vec![-1; self.components.len()];
        let mut stack: Vec<(usize, i32)> = Vec::new();
        for (i, c) in self.components.iter().enumerate() {
            if let Some((owner, owed)) = stack.last_mut() {
                parent[i] = *owner as i32;
                if *owed == 1 {
                    stack.pop();
                } else {
                    *owed -= 1;
                }
            } else if i > 0 {
                return Err(FormatError::invalid("unit", format!("component {i} has no parent")));
            }
            if c.child_count > 0 {
                stack.push((i, c.child_count));
            }
        }
        if stack.is_empty() { Ok(parent) } else { Err(FormatError::invalid("unit", "children still owed")) }
    }
}

pub fn parse_unit(data: &[u8], source: &str) -> Result<Unit, FormatError> {
    if u32_at(data, 0) != Some(DAT_MAGIC) {
        return Err(FormatError::invalid(source, "not a unit definition"));
    }
    let kind = u32_at(data, 4).ok_or_else(|| FormatError::invalid(source, "short unit definition"))?;
    let body = data.len() - DAT_HEADER;
    if !body.is_multiple_of(DAT_COMPONENT) {
        return Err(FormatError::invalid(source, "components do not divide the file"));
    }
    let components = data[DAT_HEADER..]
        .as_chunks::<DAT_COMPONENT>()
        .0
        .iter()
        .map(|c| {
            let w = |at| u32_at(c, at).expect("inside a component");
            Component {
                reference: ResourceRef { library: fixed(&c[..32]), member: fixed(&c[32..64]) },
                flags: w(64),
                attach_node: w(68) as i32,
                label: fixed(&c[72..104]),
                class_id: w(104),
                child_count: w(108) as i32,
            }
        })
        .collect();
    Ok(Unit { kind, components })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_assembly_is_a_depth_first_tree() {
        let comp = |children: i32| Component {
            reference: ResourceRef::default(),
            label: String::new(),
            flags: 1,
            attach_node: -1,
            class_id: 0,
            child_count: children,
        };
        let unit = Unit { kind: 0, components: vec![comp(2), comp(1), comp(0), comp(0)] };
        assert_eq!(unit.parents().unwrap(), vec![-1, 0, 1, 0]);
    }
}
