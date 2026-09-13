//! Materials resolved to textures, shared by everything that draws.
//!
//! A mesh batch or terrain layer names a material; the material's track 0
//! names the texture it draws, its first entry the colours that texture is lit
//! with, and its directory flags byte the blend mode. See `docs/07-objects.md`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::materials::Library;
use parkan_formats::nres::Archive;
use parkan_formats::{gamedir, texm};

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8, largest first, as the file carries them.
    pub levels: Vec<Vec<u8>>,
}

/// How a material draws.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub material: String,
    /// Index into `TextureStore::textures`, or `None` when nothing resolves.
    pub texture: Option<usize>,
    /// The diffuse colour the scene light multiplies, 0..1 in display space as the file
    /// gives it; the renderer decodes it with the rest of the lit colour.
    pub diffuse: [f32; 3],
    /// Likewise the emissive colour, to which the scene colour is added.
    pub emissive: [f32; 3],
    /// The engine's blend mode id: 0 opaque, 2 additive, 4 alpha, 3 and 5 modulate.
    pub blend_mode: u8,
}

pub struct TextureStore {
    materials: Library,
    archive: Archive,
    pub textures: Vec<Texture>,
    by_name: BTreeMap<String, Option<usize>>,
    looks: BTreeMap<String, Look>,
}

fn key(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_ascii_uppercase()
}

impl TextureStore {
    pub fn open(game: &Path) -> Result<Self> {
        let materials = Library::open(&gamedir::resolve(game, "Material.lib").context("no Material.lib")?)?;
        let archive = Archive::open(&gamedir::resolve(game, "Textures.lib").context("no Textures.lib")?)?;
        Ok(Self {
            materials,
            archive,
            textures: Vec::new(),
            by_name: BTreeMap::new(),
            looks: BTreeMap::new(),
        })
    }

    /// A texture from `Textures.lib` by name, decoded once.
    pub fn texture(&mut self, name: &str) -> Result<Option<usize>> {
        let k = key(name);
        if let Some(&found) = self.by_name.get(&k) {
            return Ok(found);
        }
        let entry = self.archive.entries.iter().find(|e| e.tag() == "Texm" && key(&e.name) == k);
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
        self.by_name.insert(k, index);
        Ok(index)
    }

    /// How a material draws, resolved once.
    pub fn look(&mut self, material: &str) -> Result<Look> {
        let k = key(material);
        if let Some(found) = self.looks.get(&k) {
            return Ok(found.clone());
        }
        let (texture_name, diffuse, emissive, blend_mode) = match self.materials.get(material) {
            Some(m) => {
                let entry = m.entries.first();
                (
                    m.frames().first().map(|s| (*s).to_owned()),
                    entry.map_or([255; 3], |e| e.diffuse),
                    entry.map_or([0; 3], |e| e.emissive),
                    m.blend_mode().unwrap_or(0),
                )
            }
            None => (None, [255; 3], [0; 3], 0),
        };
        let texture = self.texture(texture_name.as_deref().unwrap_or(material))?;
        let unit = |c: [u8; 3]| c.map(|v| f32::from(v) / 255.0);
        let look = Look {
            material: material.to_owned(),
            texture,
            diffuse: unit(diffuse),
            emissive: unit(emissive),
            blend_mode,
        };
        self.looks.insert(k, look.clone());
        Ok(look)
    }
}
