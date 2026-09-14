//! What a placed object is made of, and where each visible part sits.
//!
//! The port of `openparkan/assembly.py`, checked against it by the golden
//! cross-check. A child part's root node takes its host socket's pose, so its
//! transform is the socket composed with the inverse of the part's root,
//! composed down the assembly tree. See `docs/07-objects.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use parkan_formats::mesh::{self, Mesh};
use parkan_formats::nres::Archive;
use parkan_formats::objects::{self, ResourceRef};
use parkan_formats::pose::{IDENTITY, Pose};
use parkan_formats::{gamedir, mission, wea};

/// One visible mesh of a placed object.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// The `objects.rlb` record the part is built from.
    pub record: String,
    pub reference: ResourceRef,
    /// Where it sits in the object's own frame.
    pub pose: Pose,
    /// The part it hangs from, or -1.
    pub host: i32,
    /// The node of that part it hangs from, or -1.
    pub node: i32,
}

/// A mesh and its wear, parsed once.
pub struct LoadedMesh {
    pub mesh: Mesh,
    pub wear: wea::Wear,
}

pub struct Assembly {
    pub game: PathBuf,
    pub library: objects::Library,
    archives: HashMap<String, Option<Archive>>,
    meshes: HashMap<(String, String), Option<std::rc::Rc<LoadedMesh>>>,
    /// `.dat` assemblies held in memory under a virtual path, read before the disk.
    units: HashMap<String, Vec<u8>>,
}

/// A unit path as the in-memory table keys it: separators one way, case folded.
fn unit_key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

impl Assembly {
    pub fn new(game: &Path) -> Result<Self> {
        let library =
            objects::Library::open(&gamedir::resolve(game, "objects.rlb").context("no objects.rlb")?)?;
        Ok(Self {
            game: game.to_path_buf(),
            library,
            archives: HashMap::new(),
            meshes: HashMap::new(),
            units: HashMap::new(),
        })
    }

    /// Hold a `.dat` assembly in memory under `path`, as a design the constructor or the
    /// factory writes (`UNITS\temp_unit.dat`, `UNITS\bld_unit_<id>.dat`), so that
    /// [`Assembly::parts`] and [`Assembly::records`] load it without a file on disk.
    pub fn register_unit(&mut self, path: &str, bytes: Vec<u8>) {
        self.units.insert(unit_key(path), bytes);
    }

    /// A unit's `.dat` bytes: the one registered under `path`, else the file.
    fn unit_bytes(&self, path: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        if let Some(bytes) = self.units.get(&unit_key(path)) {
            return Some(std::borrow::Cow::Borrowed(bytes));
        }
        let file = gamedir::resolve(&self.game, path)?;
        std::fs::read(&file).ok().map(std::borrow::Cow::Owned)
    }

    pub fn archive(&mut self, name: &str) -> Option<&Archive> {
        let key = name.to_ascii_lowercase();
        if !self.archives.contains_key(&key) {
            let opened = gamedir::resolve(&self.game, name).and_then(|p| Archive::open(&p).ok());
            self.archives.insert(key.clone(), opened);
        }
        self.archives.get(&key).and_then(Option::as_ref)
    }

    /// A mesh and its wear, or `None` when either archive member is missing.
    pub fn mesh(&mut self, reference: &ResourceRef) -> Option<std::rc::Rc<LoadedMesh>> {
        let key = (reference.library.to_ascii_lowercase(), reference.member.to_ascii_lowercase());
        if let Some(found) = self.meshes.get(&key) {
            return found.clone();
        }
        let loaded = self.archive(&reference.library).and_then(|archive| {
            let stem = reference.member.rsplit_once('.').map_or(reference.member.as_str(), |(s, _)| s);
            let wear = archive.read_name(&format!("{stem}.wea")).map(wea::parse).unwrap_or_default();
            let blob = archive.read_name(&reference.member).ok()?;
            let mesh = mesh::parse(blob, &reference.member).ok()?;
            Some(std::rc::Rc::new(LoadedMesh { mesh, wear }))
        });
        self.meshes.insert(key, loaded.clone());
        loaded
    }

    /// The visible parts of an object placed with this kind and path.
    /// Every component record a unit's `.dat` names, internal parts included, in order.
    pub fn records(&self, path: &str) -> Vec<String> {
        let Some(data) = self.unit_bytes(path) else { return Vec::new() };
        objects::parse_unit(&data, path)
            .map(|unit| unit.components.iter().map(|c| c.reference.member.clone()).collect())
            .unwrap_or_default()
    }

    pub fn parts(&mut self, kind: u32, path: &str) -> Vec<Part> {
        if matches!(kind, mission::KIND_VEGETATION | mission::KIND_ROCK) {
            let reference = self.library.record_mesh(self.library.get(path), 0);
            return reference
                .map(|r| {
                    vec![Part { record: path.to_owned(), reference: r, pose: IDENTITY, host: -1, node: -1 }]
                })
                .unwrap_or_default();
        }
        let Some(data) = self.unit_bytes(path) else { return Vec::new() };
        let Ok(unit) = objects::parse_unit(&data, path) else { return Vec::new() };
        let Ok(parents) = unit.parents() else { return Vec::new() };
        let mut references: Vec<Option<ResourceRef>> = Vec::new();
        let mut poses: Vec<Pose> = Vec::new();
        let mut slot_of: HashMap<usize, usize> = HashMap::new();
        let mut out = Vec::new();
        for (i, component) in unit.components.iter().enumerate() {
            let reference = self.library.record_mesh(self.library.get(&component.reference.member), 0);
            let parent = parents[i];
            let mut pose = IDENTITY;
            if parent >= 0 {
                let p = parent as usize;
                pose = poses[p];
                let host = references[p].clone().and_then(|r| self.mesh(&r));
                let part = reference.clone().and_then(|r| self.mesh(&r));
                if let (Some(host), Some(part)) = (host, part)
                    && component.attach_node >= 0
                    && (component.attach_node as usize) < host.mesh.nodes.len()
                {
                    let socket = host.mesh.world_pose(component.attach_node as usize);
                    let mount = socket.compose(&part.mesh.root_pose().invert());
                    pose = pose.compose(&mount);
                }
            }
            poses.push(pose);
            if let Some(r) = &reference
                && component.is_external()
            {
                slot_of.insert(i, out.len());
                let (host, node) = if parent >= 0 {
                    (slot_of.get(&(parent as usize)).map_or(-1, |&s| s as i32), component.attach_node)
                } else {
                    (-1, -1)
                };
                out.push(Part {
                    record: component.reference.member.clone(),
                    reference: r.clone(),
                    pose,
                    host,
                    node,
                });
            }
            references.push(reference);
        }
        out
    }
}
