//! A map's ground, ready to draw: level-0 geometry grouped by the pair of
//! materials a face wears, and the textures those materials name.
//!
//! See `docs/03-terrain.md`: layer names are materials, not textures; the
//! ground is `mix(layer2, layer1, blend)` in one pass; one level of detail is
//! drawn, never both.

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
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    /// The weight of layer 1.
    pub blend: f32,
}

/// One material as the ground wears it: its texture, tinted by its diffuse colour.
pub type Layer = Look;

/// Faces that wear the same layer pair, as a range of `Terrain::indices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    pub layer1: Layer,
    pub layer2: Option<Layer>,
    pub water: bool,
    /// A liquid's bed (face flag `0x2000`), which is not drawn while the camera is above the
    /// liquid (`Terrain.dll:0x10043c43`).
    pub bed: bool,
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
    /// The inner rings of the placed buildings, across the ground, where the landscape is cut
    /// away (docs/03, "Placing a building cuts the landscape").
    pub cuts: Vec<Vec<[f32; 2]>>,
}

/// Every placed building's inner `.bas` ring, placed by its position and angle, across the
/// ground (`IBasement` slot 4, docs/03, "Placing a building cuts the landscape").
pub fn building_cuts(
    assembly: &mut crate::assembly::Assembly,
    mission: &parkan_formats::mission::Mission,
) -> Vec<Vec<[f32; 2]>> {
    let mut out = Vec::new();
    for o in mission.objects.iter().filter(|o| o.kind == parkan_formats::mission::KIND_BUILDING) {
        let parts = assembly.parts(o.kind, &o.path);
        let Some(root) = parts.iter().find(|p| p.host == -1) else { continue };
        let Some(slot) = assembly.library.record_slot(assembly.library.get(&root.record), "bas", 0) else {
            continue;
        };
        let Some(rings) = assembly
            .archive(&slot.library)
            .and_then(|a| a.read_name(&slot.member).ok())
            .and_then(|data| parkan_formats::basement::parse(data, &slot.member).ok())
        else {
            continue;
        };
        let Some(inner) = rings.first() else { continue };
        let (s, c) = o.rotation.sin_cos();
        out.push(
            inner
                .points
                .iter()
                .map(|p| [o.position[0] + p[0] * c - p[1] * s, o.position[1] + p[0] * s + p[1] * c])
                .collect(),
        );
    }
    out
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

    let mut buckets: BTreeMap<(u8, u8, bool, bool), Vec<usize>> = BTreeMap::new();
    for fi in land.lod_faces(0) {
        let f = &land.faces[fi];
        buckets
            .entry((f.tex1, f.tex2, f.is_water(), f.flags & FLAGS_LIQUID_BED_BIT != 0))
            .or_default()
            .push(fi);
    }
    let mut indices = Vec::new();
    let mut groups = Vec::new();
    for ((tex1, tex2, water, bed), faces) in buckets {
        let start = indices.len() as u32;
        for fi in &faces {
            indices.extend(land.faces[*fi].vertices.map(u32::from));
        }
        let name1 = land.layer1.get(usize::from(tex1)).cloned().unwrap_or_default();
        let layer1 = store.look(&name1)?;
        let layer2 = match land.layer2.get(usize::from(tex2)) {
            Some(name) if tex2 != NO_TEXTURE => Some(store.look(name)?),
            _ => None,
        };
        groups.push(Group { start, count: faces.len() as u32 * 3, layer1, layer2, water, bed });
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
