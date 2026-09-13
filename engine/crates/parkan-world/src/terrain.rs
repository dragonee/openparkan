//! A map's ground, ready to draw: level-0 geometry grouped by the pair of
//! materials a face wears, and the textures those materials name.
//!
//! See `docs/03-terrain.md`: layer names are materials, not textures; the
//! ground is `mix(layer2, layer1, blend)` in one pass; one level of detail is
//! drawn, never both.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::landmesh::{self, LandMesh, NO_TEXTURE};
use parkan_formats::materials::Library;
use parkan_formats::nres::Archive;
use parkan_formats::{gamedir, texm};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    /// The weight of layer 1.
    pub blend: f32,
}

/// One material as the ground wears it: a texture and the diffuse colour that
/// multiplies it (where water gets its blue).
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub material: String,
    /// Index into `Terrain::textures`, or `None` when nothing resolves.
    pub texture: Option<usize>,
    pub tint: [f32; 3],
}

/// Faces that wear the same layer pair, as a range of `Terrain::indices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    pub layer1: Layer,
    pub layer2: Option<Layer>,
    pub water: bool,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8, largest first, as the file carries them.
    pub levels: Vec<Vec<u8>>,
}

pub struct Terrain {
    pub land: LandMesh,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub groups: Vec<Group>,
    pub textures: Vec<Texture>,
}

/// The map directory a mission's `map_path` names: `DATA\MAPS\Tut_1\land`.
pub fn map_dir(game: &Path, map_path: &str) -> Result<std::path::PathBuf> {
    let dir = map_path.rsplit_once(['\\', '/']).map_or(map_path, |(d, _)| d);
    gamedir::resolve(game, dir).with_context(|| format!("no map directory {dir}"))
}

struct Resolver<'a> {
    materials: &'a Library,
    archive: &'a Archive,
    textures: Vec<Texture>,
    by_name: BTreeMap<String, Option<usize>>,
}

impl Resolver<'_> {
    fn texture(&mut self, name: &str) -> Result<Option<usize>> {
        let key = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
        if let Some(&found) = self.by_name.get(&key) {
            return Ok(found);
        }
        let entry = self.archive.entries.iter().find(|e| {
            e.tag() == "Texm" && e.name.split('.').next().unwrap_or(&e.name).eq_ignore_ascii_case(&key)
        });
        let index = match entry {
            Some(e) => {
                let t = texm::decode(self.archive.read(e)?, &e.name, None)?;
                self.textures.push(Texture {
                    name: e.name.clone(),
                    width: t.width,
                    height: t.height,
                    levels: t.levels,
                });
                Some(self.textures.len() - 1)
            }
            None => None,
        };
        self.by_name.insert(key, index);
        Ok(index)
    }

    /// A terrain layer: the material's track-0 texture and its diffuse tint.
    fn layer(&mut self, material: &str) -> Result<Layer> {
        let (texture_name, tint) = match self.materials.get(material) {
            Some(m) => {
                let tint = m.entries.first().map_or([255; 3], |e| e.diffuse);
                (m.frames().first().map(|s| (*s).to_owned()), tint)
            }
            None => (None, [255; 3]),
        };
        let texture = match texture_name {
            Some(name) => self.texture(&name)?,
            None => self.texture(material)?,
        };
        Ok(Layer { material: material.to_owned(), texture, tint: tint.map(|c| f32::from(c) / 255.0) })
    }
}

/// Build the ground of the map in `map_dir`.
pub fn build(game: &Path, map_dir: &Path) -> Result<Terrain> {
    let msh = gamedir::resolve(map_dir, "Land.msh").context("the map has no Land.msh")?;
    let land = landmesh::load(&msh)?;
    let materials = Library::open(&gamedir::resolve(game, "Material.lib").context("no Material.lib")?)?;
    let archive = Archive::open(&gamedir::resolve(game, "Textures.lib").context("no Textures.lib")?)?;
    let mut resolver =
        Resolver { materials: &materials, archive: &archive, textures: Vec::new(), by_name: BTreeMap::new() };

    let vertices = (0..land.positions.len())
        .map(|i| Vertex {
            position: land.positions[i],
            normal: land.normals[i],
            uv1: land.uv1[i],
            uv2: land.uv2[i],
            blend: land.blend[i],
        })
        .collect();

    let mut buckets: BTreeMap<(u8, u8, bool), Vec<usize>> = BTreeMap::new();
    for fi in land.lod_faces(0) {
        let f = &land.faces[fi];
        buckets.entry((f.tex1, f.tex2, f.is_water())).or_default().push(fi);
    }
    let mut indices = Vec::new();
    let mut groups = Vec::new();
    for ((tex1, tex2, water), faces) in buckets {
        let start = indices.len() as u32;
        for fi in &faces {
            indices.extend(land.faces[*fi].vertices.map(u32::from));
        }
        let name1 = land.layer1.get(usize::from(tex1)).cloned().unwrap_or_default();
        let layer1 = resolver.layer(&name1)?;
        let layer2 = match land.layer2.get(usize::from(tex2)) {
            Some(name) if tex2 != NO_TEXTURE => Some(resolver.layer(name)?),
            _ => None,
        };
        groups.push(Group { start, count: faces.len() as u32 * 3, layer1, layer2, water });
    }
    let textures = resolver.textures;
    Ok(Terrain { land, vertices, indices, groups, textures })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_path_names_its_directory() {
        let dir = std::env::temp_dir().join("parkan-map-dir-test/DATA/MAPS/Tut_1");
        std::fs::create_dir_all(&dir).unwrap();
        let game = dir.parent().unwrap().parent().unwrap().parent().unwrap();
        assert_eq!(map_dir(game, "DATA\\MAPS\\Tut_1\\land").unwrap(), dir);
    }
}
