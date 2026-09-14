//! Materials resolved to textures, shared by everything that draws.
//!
//! A mesh batch or terrain layer names a material; each key of the material's track 0
//! names an entry, whose texture, cell and colours the draw takes, and its directory
//! flags byte the blend mode. See `docs/07-objects.md`, "How a material reaches the
//! device".

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::materials::{self, Library};
use parkan_formats::nres::Archive;
use parkan_formats::{gamedir, texm};

#[derive(Clone, Debug)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8, largest first, as the file carries them.
    pub levels: Vec<Vec<u8>>,
    /// Its cells, `(x, y, width, height)` in pixels, from the `Page` table.
    pub pages: Vec<[u16; 4]>,
}

/// The whole texture as a cell: `(u0, v0, du, dv)`.
pub const WHOLE_CELL: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// One entry of a material as the device takes it (`Terrain.dll:0x10030819`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    /// Index into `TextureStore::textures`, or `None` when nothing resolves.
    pub texture: Option<usize>,
    /// The diffuse colour the scene light multiplies, 0..1 in display space as the file
    /// gives it; the renderer decodes it with the rest of the lit colour.
    pub diffuse: [f32; 3],
    /// The entry's ambient colour, the material's self-light: the device's emissive is
    /// the scene colour plus this. The entry's own emissive is never read.
    pub ambient: [f32; 3],
    /// The ambient alpha, × 0.01: the device's diffuse alpha, which scales the texture's.
    pub alpha: f32,
    /// The cell's rectangle, `(u0, v0, du, dv)`: a UV becomes `u0 + u × du`,
    /// `v0 + v × dv`.
    pub cell: [f32; 4],
}

impl Phase {
    /// White, lit, and whole: a material that does not resolve.
    pub const PLAIN: Phase =
        Phase { texture: None, diffuse: [1.0; 3], ambient: [0.0; 3], alpha: 1.0, cell: WHOLE_CELL };
}

/// A track's lerp mask bits (`World3D.dll:0x10003030`): which fields glide between keys.
pub const LERP_AMBIENT: u32 = 1;
pub const LERP_DIFFUSE: u32 = 2;
pub const LERP_AMBIENT_ALPHA: u32 = 0x10;
/// A track's modes (`0x10003668`): loop, ping-pong, once, jump.
pub const TRACK_LOOP: u32 = 0;
pub const TRACK_PING_PONG: u32 = 1;
pub const TRACK_ONCE: u32 = 2;
pub const TRACK_JUMP: u32 = 3;

/// Track 0 of a material with more than one key, as the manager plays it.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// The track word's low three bits, and the rest.
    pub mode: u32,
    pub mask: u32,
    /// Each key's phase and the time, ms, its interval ends.
    pub keys: Vec<(Phase, f32)>,
}

impl Animation {
    /// The phase shown `t_ms` after the material's start (slot 3, `0x100031f0`): the
    /// period is the last key's time; key i is the largest whose interval, from the
    /// previous key's time, holds t, or 0 when none does; the phase is key i's with the
    /// masked colours lerped toward key i + 1's (wrapping) by how far t is through it.
    ///
    /// STAND-IN: docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured --
    /// a material's start stamp (list record `+4`, set by slot 10) is not read: every
    /// material starts at the clock's 0. Mode 3's `rand()` is not the game's: a hash of t.
    pub fn at(&self, t_ms: f64) -> Phase {
        let n = self.keys.len();
        let period = f64::from(self.keys.last().map_or(0.0, |k| k.1));
        if n == 0 || period <= 0.0 {
            return self.keys.first().map_or(Phase::PLAIN, |k| k.0);
        }
        let t = t_ms.max(0.0);
        let t = match self.mode {
            TRACK_PING_PONG => {
                let u = t % (2.0 * period);
                if u > period { 2.0 * period - u } else { u }
            }
            TRACK_ONCE if t >= period => return self.keys[n - 1].0,
            TRACK_ONCE => t,
            TRACK_JUMP => {
                let mut x = (t as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
                x ^= x >> 29;
                (x % period.max(1.0) as u64) as f64
            }
            _ => t % period,
        } as f32;
        let i = (0..n).find(|&i| t < self.keys[i].1 && (i == 0 || self.keys[i - 1].1 <= t)).unwrap_or(0);
        let from = if i == 0 { 0.0 } else { self.keys[i - 1].1 };
        let span = self.keys[i].1 - from;
        let f = if span > 0.0 { ((t - from) / span).clamp(0.0, 1.0) } else { 0.0 };
        let (a, b) = (self.keys[i].0, self.keys[(i + 1) % n].0);
        let lerp3 = |x: [f32; 3], y: [f32; 3]| std::array::from_fn(|c| x[c] + (y[c] - x[c]) * f);
        let mut phase = a;
        if self.mask & LERP_AMBIENT != 0 {
            phase.ambient = lerp3(a.ambient, b.ambient);
        }
        if self.mask & LERP_DIFFUSE != 0 {
            phase.diffuse = lerp3(a.diffuse, b.diffuse);
        }
        if self.mask & LERP_AMBIENT_ALPHA != 0 {
            phase.alpha = a.alpha + (b.alpha - a.alpha) * f;
        }
        phase
    }
}

/// How a material draws.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub material: String,
    /// The engine's blend mode id: 0 opaque, 2 additive, 4 alpha, 3 and 5 modulate.
    pub blend_mode: u8,
    /// Track 0's first key's entry: what the material shows when it is not played.
    pub still: Phase,
    /// Track 0, when it has more than one key.
    pub animation: Option<Animation>,
}

impl Look {
    /// The phase shown at the world clock's `t_ms`.
    pub fn at(&self, t_ms: f64) -> Phase {
        self.animation.as_ref().map_or(self.still, |a| a.at(t_ms))
    }
}

pub struct TextureStore {
    materials: Library,
    archive: Archive,
    /// `lightmap.lib`, the buildings' lightmap pages, when the install has one.
    lightmaps: Option<Archive>,
    pub textures: Vec<Texture>,
    by_name: BTreeMap<String, Option<usize>>,
    lightmap_by_name: BTreeMap<String, Option<usize>>,
    looks: BTreeMap<String, Look>,
}

/// The library a wear's `LIGHTMAPS` pages are loaded from (`World3D.dll:0x10003f24`, docs/07,
/// "How a lightmapped batch is drawn").
pub const LIGHTMAP_LIBRARY: &str = "lightmap.lib";

fn key(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_ascii_uppercase()
}

impl TextureStore {
    pub fn open(game: &Path) -> Result<Self> {
        let materials = Library::open(&gamedir::resolve(game, "Material.lib").context("no Material.lib")?)?;
        let archive = Archive::open(&gamedir::resolve(game, "Textures.lib").context("no Textures.lib")?)?;
        let lightmaps = gamedir::resolve(game, LIGHTMAP_LIBRARY).and_then(|p| Archive::open(&p).ok());
        Ok(Self {
            materials,
            archive,
            lightmaps,
            textures: Vec::new(),
            by_name: BTreeMap::new(),
            lightmap_by_name: BTreeMap::new(),
            looks: BTreeMap::new(),
        })
    }

    /// A lightmap page from `lightmap.lib` by name, decoded once like any texture.
    pub fn lightmap(&mut self, name: &str) -> Result<Option<usize>> {
        let k = key(name);
        if let Some(&found) = self.lightmap_by_name.get(&k) {
            return Ok(found);
        }
        let Some(archive) = self.lightmaps.as_ref() else { return Ok(None) };
        let entry = archive.entries.iter().find(|e| e.tag() == "Texm" && key(&e.name) == k);
        let index = match entry {
            Some(e) => {
                let t = texm::decode(archive.read(e)?, &e.name, None)?;
                self.textures.push(Texture {
                    name: e.name.clone(),
                    width: t.width,
                    height: t.height,
                    levels: t.levels,
                    pages: t.pages,
                });
                Some(self.textures.len() - 1)
            }
            None => None,
        };
        self.lightmap_by_name.insert(k, index);
        Ok(index)
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
                    pages: t.pages,
                });
                Some(self.textures.len() - 1)
            }
            None => None,
        };
        self.by_name.insert(k, index);
        Ok(index)
    }

    /// An entry as the device takes it, its texture `fallback` when it names none.
    fn phase(&mut self, entry: &materials::Entry, fallback: Option<usize>) -> Result<Phase> {
        let texture =
            if entry.texture.is_empty() { fallback } else { self.texture(&entry.texture)?.or(fallback) };
        let unit = |c: [u8; 3]| c.map(|v| f32::from(v) / 255.0);
        // Cell -1 is the whole texture; cell k the page table's record k.
        let cell = usize::try_from(entry.cell)
            .ok()
            .zip(texture)
            .and_then(|(c, t)| {
                let t = &self.textures[t];
                let [x, y, w, h] = t.pages.get(c)?.map(f32::from);
                let (tw, th) = (t.width as f32, t.height as f32);
                Some([x / tw, y / th, w / tw, h / th])
            })
            .unwrap_or(WHOLE_CELL);
        Ok(Phase {
            texture,
            diffuse: unit(entry.diffuse),
            ambient: unit(entry.ambient),
            alpha: f32::from(entry.alphas[0]) * 0.01,
            cell,
        })
    }

    /// How a material draws, resolved once.
    pub fn look(&mut self, material: &str) -> Result<Look> {
        let k = key(material);
        if let Some(found) = self.looks.get(&k) {
            return Ok(found.clone());
        }
        let Some(m) = self.materials.get(material).cloned() else {
            let still = Phase { texture: self.texture(material)?, ..Phase::PLAIN };
            let look = Look { material: material.to_owned(), blend_mode: 0, still, animation: None };
            self.looks.insert(k, look.clone());
            return Ok(look);
        };
        // An entry naming no texture draws the track's first named one, or the material's
        // own name's.
        let named = m.frames().first().map(|s| (*s).to_owned());
        let fallback = self.texture(named.as_deref().unwrap_or(material))?;
        let still = match m.first_entry() {
            Some(e) => self.phase(e, fallback)?,
            None => Phase { texture: fallback, ..Phase::PLAIN },
        };
        let animation = match m.tracks.first() {
            Some(track) if track.keys.len() > 1 => {
                let mut keys = Vec::with_capacity(track.keys.len());
                for key in &track.keys {
                    let phase = match m.entries.get(usize::from(key.entry)) {
                        Some(e) => self.phase(e, fallback)?,
                        None => still,
                    };
                    keys.push((phase, f32::from(key.time)));
                }
                Some(Animation { mode: track.word & 7, mask: track.word >> 3, keys })
            }
            _ => None,
        };
        let look =
            Look { material: material.to_owned(), blend_mode: m.blend_mode().unwrap_or(0), still, animation };
        self.looks.insert(k, look.clone());
        Ok(look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phase(ambient: f32, cell: f32) -> Phase {
        Phase { ambient: [ambient; 3], cell: [cell, 0.0, 0.25, 0.5], ..Phase::PLAIN }
    }

    /// The buoy beam's track: entries 0, 1, 2, 1 ending at 50, 100, 150 and 200 ms.
    fn beam(mode: u32, mask: u32) -> Animation {
        let keys = [(0.0, 50.0), (0.5, 100.0), (1.0, 150.0), (0.5, 200.0)];
        Animation { mode, mask, keys: keys.iter().map(|&(e, t)| (phase(e, e), t)).collect() }
    }

    #[test]
    fn a_key_ends_its_interval_with_the_next_keys_colours_arrived_and_its_own_cell() {
        let a = beam(TRACK_LOOP, LERP_AMBIENT);
        assert_eq!(a.at(0.0).ambient, [0.0; 3]);
        assert_eq!(a.at(25.0).ambient, [0.25; 3], "halfway from entry 0 toward entry 1");
        assert_eq!(a.at(25.0).cell[0], 0.0, "the cell steps with the key");
        assert_eq!(a.at(50.0).ambient, [0.5; 3]);
        assert_eq!(a.at(125.0).cell[0], 1.0);
        assert_eq!(a.at(175.0).ambient, [0.25; 3], "the last key glides back to the first");
        assert_eq!(a.at(200.0), a.at(0.0), "and loops at the last key's time");
        assert_eq!(a.at(1025.0), a.at(25.0));
        assert_eq!(beam(TRACK_LOOP, 0).at(25.0).ambient, [0.0; 3], "an unmasked colour steps");
    }

    #[test]
    fn a_ping_pong_runs_back_and_a_once_holds_its_last_key() {
        let p = beam(TRACK_PING_PONG, LERP_AMBIENT);
        assert_eq!(p.at(225.0), p.at(175.0));
        assert_eq!(p.at(375.0), p.at(25.0));
        let once = beam(TRACK_ONCE, LERP_AMBIENT);
        assert_eq!(once.at(25.0), beam(TRACK_LOOP, LERP_AMBIENT).at(25.0));
        assert_eq!(once.at(5000.0).cell[0], 0.5, "the last key's entry");
        let jump = beam(TRACK_JUMP, 0);
        assert!((0..50).all(|t| jump.at(f64::from(t) * 37.0).cell[0] <= 1.0));
    }
}
