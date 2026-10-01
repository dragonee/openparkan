//! A map's ground, ready to draw: level-0 geometry grouped by the pair of
//! materials a face wears, and the textures those materials name.
//!
//! See `docs/03-terrain.md`: a face's two bytes both index `Land1.wea`, whose names are
//! materials; each material's **microtexture** is track 1 of the material at the same index
//! of `Land2.wea`, drawn as a second texture stage on stream 18's coordinates and doubled
//! (render phase 9); the face's second material is drawn over its first on stream 14's alpha;
//! one level of detail is drawn, never both.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::gamedir;
use parkan_formats::landmesh::{self, FLAGS_LIQUID_BED_BIT, LandMesh, NO_TEXTURE};

use crate::textures::{Look, TextureStore};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// The material's own texture's coordinates, stream 5.
    pub uv1: [f32; 2],
    /// The microtexture's, stream 18.
    pub uv2: [f32; 2],
    /// The alpha of the face's second material over its first, stream 14.
    pub blend: f32,
}

/// One material as the ground wears it: its texture, lit by its own colours.
pub type Layer = Look;

/// The landscape face flag the microtexture is drawn under (`Terrain.dll:0x1004456a`): every
/// shipped face carries it in its constant `0x600`, and a basement face's `0x300` does not.
pub const FACE_MICROTEXTURED: u16 = 0x400;

/// Faces that wear the same layer pair, as a range of `Terrain::indices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    /// The face's material, `Land1.wea` at its first byte.
    pub layer1: Layer,
    /// That material's microtexture: the texture of track 1 of `Land2.wea`'s material at the
    /// same index, or of its track 0 where it has one track (the manager holds a track outside
    /// a material's count to 0, `World3D.dll:0x1000322f`). `None` where none is drawn: on
    /// water, and on a face without [`FACE_MICROTEXTURED`].
    pub micro1: Option<usize>,
    /// The face's second material, `Land1.wea` at its second byte, drawn over the first on
    /// the vertices' [`Vertex::blend`].
    pub layer2: Option<Layer>,
    /// The second material's microtexture.
    pub micro2: Option<usize>,
    pub water: bool,
    /// A liquid's bed (face flag `0x2000`), which is not drawn while the camera is above the
    /// liquid (`Terrain.dll:0x10043c43`).
    pub bed: bool,
    /// A building's footing, stitched in where the landscape was cut away
    /// ([`crate::basement`]): the cut does not apply to it, since it is what fills the cut.
    pub basement: bool,
}

/// The box every water face lies in, at the water level (`Terrain.dll:0x10017e6e`): the
/// reflection texture covers it (docs/03-terrain.md, "The water box").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterBox {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub level: f32,
}

pub struct Terrain {
    pub land: LandMesh,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub groups: Vec<Group>,
    /// The water's box, on a map with water.
    pub water: Option<WaterBox>,
    /// The outer rings of the placed buildings, across the ground, where the landscape is cut
    /// away (docs/03, "Placing a building cuts the landscape"). The band between the rings is
    /// drawn as the basement groups instead.
    pub cuts: Vec<Vec<[f32; 2]>>,
}

impl Terrain {
    /// Let the placed buildings into the ground: cut the landscape away inside each outer
    /// contour and stitch the footing's own faces in between the contours (docs/03,
    /// "Placing a building cuts the landscape").
    ///
    /// A footing's faces are grouped by the pair they wear, exactly as the landscape's own
    /// faces are, and drawn after them.
    pub fn place_buildings(
        &mut self,
        footings: &[crate::basement::Footing],
        store: &mut TextureStore,
    ) -> Result<()> {
        self.cuts = footings.iter().map(|f| f.outline.clone()).collect();
        let mut buckets: BTreeMap<(u8, u8), Vec<&crate::basement::Facet>> = BTreeMap::new();
        for facet in footings.iter().flat_map(|f| f.faces.iter().chain(f.apron.iter())) {
            buckets.entry((facet.tex1, facet.tex2)).or_default().push(facet);
        }
        for ((tex1, tex2), facets) in buckets {
            let start = self.indices.len() as u32;
            for facet in &facets {
                for corner in facet.corners {
                    self.indices.push(self.vertices.len() as u32);
                    self.vertices.push(corner);
                }
            }
            // A basement face's flags are 0x300, without the microtexture's 0x400.
            let (layer1, _) = surface(&self.land, store, tex1, false)?;
            let layer2 = match tex2 {
                NO_TEXTURE => None,
                _ => Some(surface(&self.land, store, tex2, false)?.0),
            };
            self.groups.push(Group {
                start,
                count: facets.len() as u32 * 3,
                layer1,
                micro1: None,
                layer2,
                micro2: None,
                water: false,
                bed: false,
                basement: true,
            });
        }
        Ok(())
    }
}

/// The material at `index` of `Land1.wea` and, where `micro`, its microtexture: the texture of
/// track 1 of `Land2.wea`'s material at the same index (`Terrain.dll:0x1002b4a7`, the handle
/// `index + (wear << 16)` built at `0x100445c0` with the second wear's index the landscape
/// keeps from loading `Land2.wea`, `0x10017215` → `0x10018cef`).
fn surface(
    land: &LandMesh,
    store: &mut TextureStore,
    index: u8,
    micro: bool,
) -> Result<(Layer, Option<usize>)> {
    let name = land.layer1.get(usize::from(index)).cloned().unwrap_or_default();
    let look = store.look(&name)?;
    let micro = match land.layer2.get(usize::from(index)) {
        Some(twin) if micro => store.look_on_track(twin, 1)?.still.texture,
        _ => None,
    };
    Ok((look, micro))
}

/// The box `land`'s water faces' vertices widen from empty (`0x1001d5d0`), its corners handed
/// out at its top (`0x10022630`).
pub fn water_box(land: &LandMesh) -> Option<WaterBox> {
    let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
    for face in land.faces.iter().filter(|f| f.is_water()) {
        for &v in &face.vertices {
            let p = land.positions[usize::from(v)];
            for a in 0..3 {
                min[a] = min[a].min(p[a]);
                max[a] = max[a].max(p[a]);
            }
        }
    }
    (min[0] < max[0] && min[1] < max[1]).then_some(WaterBox {
        min: [min[0], min[1]],
        max: [max[0], max[1]],
        level: max[2],
    })
}

/// The map directory a mission's `map_path` names: `DATA\MAPS\Tut_1\land`.
pub fn map_dir(game: &Path, map_path: &str) -> Result<std::path::PathBuf> {
    let dir = map_path.rsplit_once(['\\', '/']).map_or(map_path, |(d, _)| d);
    gamedir::resolve(game, dir).with_context(|| format!("no map directory {dir}"))
}

/// Build the ground of the map in `map_dir`, resolving its layers through `store`.
pub fn build(map_dir: &Path, store: &mut TextureStore) -> Result<Terrain> {
    let msh = gamedir::resolve(map_dir, "Land.msh").context("the map has no Land.msh")?;
    let land = landmesh::load(&msh)?;

    let vertices = (0..land.positions.len())
        .map(|i| Vertex {
            position: land.positions[i],
            normal: land.normals[i],
            uv1: land.uv1[i],
            uv2: land.uv2[i],
            blend: land.blend[i],
        })
        .collect();

    let mut buckets: BTreeMap<(u8, u8, bool, bool, bool), Vec<usize>> = BTreeMap::new();
    for fi in land.lod_faces(0) {
        let f = &land.faces[fi];
        // The microtexture is the second stage of every face that carries the flag and is not
        // water (`Terrain.dll:0x1002b49d`, `0x1004456a`).
        //
        // STAND-IN: docs/03-terrain.md#not-established -- a water face drawn without its
        // reflection takes the microtexture in passes of its own, within 260 units of the eye
        // over the field of view and at an alpha of at most 0.3 (`0x1002b512` on); here it
        // takes none.
        let micro = f.flags & FACE_MICROTEXTURED != 0 && !f.is_water();
        buckets
            .entry((f.tex1, f.tex2, f.is_water(), f.flags & FLAGS_LIQUID_BED_BIT != 0, micro))
            .or_default()
            .push(fi);
    }
    let mut indices = Vec::new();
    let mut groups = Vec::new();
    for ((tex1, tex2, water, bed, micro), faces) in buckets {
        let start = indices.len() as u32;
        for fi in &faces {
            indices.extend(land.faces[*fi].vertices.map(u32::from));
        }
        let (layer1, micro1) = surface(&land, store, tex1, micro)?;
        let (layer2, micro2) = match tex2 {
            NO_TEXTURE => (None, None),
            _ => {
                let (look, twin) = surface(&land, store, tex2, micro)?;
                (Some(look), twin)
            }
        };
        groups.push(Group {
            start,
            count: faces.len() as u32 * 3,
            layer1,
            micro1,
            layer2,
            micro2,
            water,
            bed,
            basement: false,
        });
    }
    let water = water_box(&land);
    Ok(Terrain { land, vertices, indices, groups, water, cuts: Vec::new() })
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
