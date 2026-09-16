//! Placed objects, ready to draw: each distinct object built once from its
//! parts, and an instance per placement.
//!
//! A model is rigid in M2: every node sits at its rest pose, frame 0 of an
//! animated node's run. Level 0 of variant 0 is drawn, collision hulls never.
//! A unit's own view draws each node's fifth slot instead, one model a node, so the
//! node can follow its pose. See `docs/07-objects.md`.

use std::collections::HashMap;

use anyhow::Result;
use glam::{Mat4, Vec3, Vec4};
use parkan_formats::mesh::{Mesh, NO_SLOT, Node, SLOTS_PER_VARIANT};
use parkan_formats::mission::Mission;
use parkan_formats::pose::{Pose, rotate};

use parkan_formats::wea::Wear;

use crate::assembly::{Assembly, LoadedMesh};
use crate::textures::{Look, TextureStore};

/// The slot of a variant's block a unit's own view draws in place of a level of detail
/// (`AniMesh.dll:0x10014be5`).
pub const VIEW_LEVEL: usize = 4;

/// A batch's material word's high byte that marks it unlit: any other value indexes its
/// wear's `LIGHTMAPS` list (docs/07, "The batch material's high byte marks the lit batches").
pub const NO_LIGHTMAP: u16 = 0xFF;

/// The see-through materials a building's doorways and portals wear (docs/24, "Portal quads
/// stand between the rooms").
pub const PORTAL_MATERIALS: [&str; 3] = ["DEFAULT", "PORTAL_001", "PORTAL_004"];

/// Whether a face of material `name` is a portal: a doorway or room opening, which is not
/// drawn and which a mover and a round pass through.
///
/// STAND-IN: docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured --
/// where a gathered face's batch word, whose 8 and 0x200 the collision query passes, comes
/// from is not traced; a recording shows the hero walking through the Large Factory's black
/// `DEFAULT` doorway and its `PORTAL_001` quads, so those materials' faces pass.
pub fn doorway(name: &str) -> bool {
    PORTAL_MATERIALS.iter().any(|m| name.eq_ignore_ascii_case(m))
}

/// Which of `mesh`'s triangles wear a [`doorway`] material, by the batch that covers each:
/// empty where none does, so a mesh without portals costs nothing.
pub fn portal_triangles(mesh: &Mesh, wear: &Wear) -> Vec<bool> {
    let portal = |material: u16| wear.materials.get(usize::from(material)).is_some_and(|name| doorway(name));
    if !mesh.batches.iter().any(|b| portal(b.material)) {
        return Vec::new();
    }
    let mut out = vec![false; mesh.triangles.len()];
    for batch in mesh.batches.iter().filter(|b| portal(b.material)) {
        let (first, count) = batch.triangles();
        for flag in out.iter_mut().skip(first).take(count) {
            *flag = true;
        }
    }
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Stream 18's lightmap coordinates, over 1024; 0 where the mesh has none.
    pub lightmap: [f32; 2],
}

/// Triangles of one model drawn with one material, as a range of `Model::indices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    pub look: Look,
    /// A lit batch's lightmap page, by index into the store's textures.
    pub lightmap: Option<usize>,
}

/// What a model's batches resolve through: a material's look, and a lightmap page.
pub trait Skins {
    fn look(&mut self, material: &str) -> Result<Look>;
    fn lightmap(&mut self, page: &str) -> Result<Option<usize>>;
}

impl Skins for TextureStore {
    fn look(&mut self, material: &str) -> Result<Look> {
        TextureStore::look(self, material)
    }

    fn lightmap(&mut self, page: &str) -> Result<Option<usize>> {
        TextureStore::lightmap(self, page)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub groups: Vec<Group>,
}

impl Model {
    /// The lowest z of the model's own frame, where it meets the ground.
    pub fn lowest(&self) -> f32 {
        self.vertices.iter().map(|v| v.position[2]).fold(f32::MAX, f32::min)
    }

    /// Append a slot's batches, each in its wear's material, placing every vertex it
    /// reaches with `place`. A batch whose material word's high byte is not `0xFF` takes the
    /// wear's lightmap it indexes, when the mesh carries stream 18 (docs/07, "How a
    /// lightmapped batch is drawn").
    fn push_slot(
        &mut self,
        mesh: &Mesh,
        wear: &Wear,
        slot: u16,
        skins: &mut impl Skins,
        place: impl Fn(usize) -> Vertex,
    ) -> Result<()> {
        let Some(slot) = mesh.slots.get(usize::from(slot)) else { return Ok(()) };
        let batches =
            usize::from(slot.first_batch)..usize::from(slot.first_batch) + usize::from(slot.batch_count);
        for batch in batches.filter_map(|b| mesh.batches.get(b)) {
            let name = wear.materials.get(usize::from(batch.material)).cloned().unwrap_or_default();
            // A portal quad is not drawn: it is the opening between a building's cells, and
            // `CBuilding` draws the cell beyond it instead (docs/24, "Portal quads stand
            // between the rooms").
            if doorway(&name) {
                continue;
            }
            let look = skins.look(&name)?;
            let page = (batch.flag != NO_LIGHTMAP && !mesh.lightmap_uv.is_empty())
                .then(|| wear.lightmaps.get(usize::from(batch.flag)))
                .flatten();
            let lightmap = match page {
                Some(page) => skins.lightmap(page)?,
                None => None,
            };
            let start = self.indices.len() as u32;
            let mut remap: HashMap<u16, u32> = HashMap::new();
            let (first, count) = batch.triangles();
            for tri in mesh.triangles.iter().skip(first).take(count) {
                for &v in tri {
                    let index = *remap.entry(v).or_insert_with(|| {
                        self.vertices.push(place(usize::from(v)));
                        self.vertices.len() as u32 - 1
                    });
                    self.indices.push(index);
                }
            }
            let count = self.indices.len() as u32 - start;
            if count > 0 {
                self.groups.push(Group { start, count, look, lightmap });
            }
        }
        Ok(())
    }
}

/// One placement of a model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub model: usize,
    pub position: [f32; 3],
    /// Radians about z.
    pub rotation: f32,
    /// Uniform; the record's for scenery, 1 for units and buildings.
    pub scale: f32,
    /// Not drawn: a pooled instance waiting for use, or something destroyed.
    pub hidden: bool,
}

pub struct Objects {
    pub models: Vec<Model>,
    pub instances: Vec<Instance>,
    /// The mission object index each instance places.
    pub placed: Vec<usize>,
}

/// The model an object placed with `kind` and `path` is drawn with, or `None` when
/// nothing resolves. Scenery kinds take `path` as an `objects.rlb` record name.
pub fn build_model(
    assembly: &mut Assembly,
    store: &mut TextureStore,
    kind: u32,
    path: &str,
) -> Result<Option<Model>> {
    let parts = assembly.parts(kind, path);
    let mut model = Model { name: path.to_owned(), ..Default::default() };
    for part in &parts {
        let Some(loaded) = assembly.mesh(&part.reference) else { continue };
        let m = &loaded.mesh;
        let posed = m.posed_positions();
        let owner = m.node_of_vertex();
        let rotations: Vec<[f64; 4]> = (0..m.nodes.len()).map(|i| m.world_pose(i).rotation).collect();
        for node in m.nodes.iter().filter(|n| !n.is_collision()) {
            let Some(slot) = node.slot_for_lod(0, 0) else { continue };
            model.push_slot(m, &loaded.wear, slot, store, |vi| {
                let local = posed.get(vi).copied().unwrap_or_default();
                let normal = m.normals.get(vi).copied().unwrap_or([0.0, 0.0, 1.0]);
                let normal = match owner.get(vi) {
                    Some(&o) if o >= 0 => rotate(rotations[o as usize], normal),
                    _ => normal,
                };
                let uv = m.uv.get(vi).copied().unwrap_or_default();
                Vertex {
                    position: part.pose.apply(local).map(|c| c as f32),
                    normal: rotate(part.pose.rotation, normal).map(|c| c as f32),
                    uv: uv.map(|c| c as f32),
                    lightmap: lightmap_uv(m, vi),
                }
            })?;
        }
    }
    Ok((!model.indices.is_empty()).then_some(model))
}

/// The slot a unit's own view draws for `node`: `variant × 5 + 4`, with no fallback
/// (`AniMesh.dll:0x100124d0`), so a node without one is not drawn in that view.
pub fn view_slot(node: &Node, variant: usize) -> Option<u16> {
    node.slot_index.get(variant * SLOTS_PER_VARIANT + VIEW_LEVEL).copied().filter(|&s| s != NO_SLOT)
}

/// What a unit's own view draws of one node (`docs/07-objects.md`, "The fifth slot is
/// what the unit's own view draws"): its fifth slot, in the node's own frame, or `None`
/// where it has none. Cockpits, flagged `0x20`, are drawn here and nowhere else.
pub fn build_view_node(
    loaded: &LoadedMesh,
    node: usize,
    variant: usize,
    skins: &mut impl Skins,
) -> Result<Option<Model>> {
    let m = &loaded.mesh;
    let Some(slot) = m.nodes.get(node).and_then(|n| view_slot(n, variant)) else { return Ok(None) };
    let mut model = Model { name: format!("{} view {node}", m.name), ..Default::default() };
    model.push_slot(m, &loaded.wear, slot, skins, |vi| node_vertex(m, vi))?;
    Ok((!model.indices.is_empty()).then_some(model))
}

/// One node's level-0 slot of `variant`, in the node's own frame, for drawing an object
/// node by node as its pose and its damage stage move; `None` where the node draws nothing
/// there.
pub fn build_node(
    loaded: &LoadedMesh,
    node: usize,
    variant: usize,
    skins: &mut impl Skins,
) -> Result<Option<Model>> {
    let m = &loaded.mesh;
    let Some(n) = m.nodes.get(node).filter(|n| !n.is_collision()) else { return Ok(None) };
    let Some(slot) = n.slot_for_lod(0, variant) else { return Ok(None) };
    let mut model = Model { name: format!("{} node {node} variant {variant}", m.name), ..Default::default() };
    model.push_slot(m, &loaded.wear, slot, skins, |vi| node_vertex(m, vi))?;
    Ok((!model.indices.is_empty()).then_some(model))
}

/// Vertex `vi` in its node's own frame.
fn node_vertex(m: &Mesh, vi: usize) -> Vertex {
    Vertex {
        position: m.positions.get(vi).copied().unwrap_or_default(),
        normal: m.normals.get(vi).copied().unwrap_or([0.0, 0.0, 1.0]).map(|c| c as f32),
        uv: m.uv.get(vi).copied().unwrap_or_default().map(|c| c as f32),
        lightmap: lightmap_uv(m, vi),
    }
}

/// Vertex `vi`'s stream-18 coordinates, or 0 where the mesh has none.
fn lightmap_uv(m: &Mesh, vi: usize) -> [f32; 2] {
    m.lightmap_uv.get(vi).copied().unwrap_or_default().map(|c| c as f32)
}

/// A pose as the matrix that places a point in its frame.
pub fn pose_matrix(pose: &Pose) -> Mat4 {
    let axis = |v: [f64; 3]| {
        let [x, y, z] = rotate(pose.rotation, v).map(|c| c as f32);
        Vec4::new(x, y, z, 0.0)
    };
    let [tx, ty, tz] = pose.translation.map(|c| c as f32);
    Mat4::from_cols(
        axis([1.0, 0.0, 0.0]),
        axis([0.0, 1.0, 0.0]),
        axis([0.0, 0.0, 1.0]),
        Vec3::new(tx, ty, tz).extend(1.0),
    )
}

/// Every object of `mission` that resolves to geometry.
pub fn build(assembly: &mut Assembly, store: &mut TextureStore, mission: &Mission) -> Result<Objects> {
    let mut models = Vec::new();
    let mut by_path: HashMap<(u32, String), Option<usize>> = HashMap::new();
    let mut instances = Vec::new();
    let mut placed = Vec::new();
    for (i, object) in mission.objects.iter().enumerate() {
        let key = (object.kind, object.path.to_ascii_lowercase());
        let model = match by_path.get(&key) {
            Some(&found) => found,
            None => {
                let built = build_model(assembly, store, object.kind, &object.path)?.map(|m| {
                    models.push(m);
                    models.len() - 1
                });
                by_path.insert(key, built);
                built
            }
        };
        if let Some(model) = model {
            instances.push(Instance {
                model,
                position: object.position,
                rotation: object.rotation,
                scale: object.placed_scale(),
                hidden: false,
            });
            placed.push(i);
        }
    }
    Ok(Objects { models, instances, placed })
}

#[cfg(test)]
mod tests {
    use parkan_formats::mesh::{Batch, Slot};
    use parkan_formats::pose::IDENTITY;
    use parkan_formats::wea::Wear;

    use super::*;
    use crate::textures::Phase;

    fn slot(first_triangle: u16, first_batch: u16) -> Slot {
        Slot {
            first_triangle,
            triangle_count: 1,
            first_batch,
            batch_count: 1,
            aabb_min: [0.0; 3],
            aabb_max: [0.0; 3],
            sphere: [0.0; 4],
            area: 0.0,
            volume: 0.0,
        }
    }

    /// Two nodes: a hull with only a level 0, and a cockpit with only a fifth slot, its
    /// node posed a metre up.
    fn cockpit_mesh() -> LoadedMesh {
        let mut hull = [NO_SLOT; 15];
        hull[0] = 0;
        let mut cockpit = [NO_SLOT; 15];
        cockpit[VIEW_LEVEL] = 1;
        let node = |name: &str, flags, slot_index| Node {
            name: name.to_owned(),
            flags,
            parent: 0xFFFF,
            anim_start: 0xFFFF,
            fallback_key: 0,
            slot_index,
        };
        let batch = |first_index, material| Batch {
            material,
            flag: 0xFF,
            first_index,
            index_count: 3,
            first_vertex: 0,
            vertex_count: 3,
        };
        let key = parkan_formats::mesh::Key {
            translation: [0.0, 0.0, 1.0],
            time: 0.0,
            rotation: [1.0, 0.0, 0.0, 0.0],
        };
        let mesh = Mesh {
            name: "cockpit".to_owned(),
            positions: vec![
                [0.0; 3],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [2.0, 2.0, 0.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 6],
            uv: vec![[0.0; 2]; 6],
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [3, 4, 5]],
            nodes: vec![node("B_Up_m1o1", 0x200, hull), node("CP_m1o1", 0x20, cockpit)],
            slots: vec![slot(0, 0), slot(1, 1)],
            batches: vec![batch(0, 0), batch(3, 1)],
            face_flags: Vec::new(),
            face_normals: Vec::new(),
            keys: vec![key],
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
            corners: None,
        };
        LoadedMesh {
            mesh,
            wear: Wear { materials: vec!["HULL".to_owned(), "GLASS".to_owned()], ..Default::default() },
        }
    }

    /// Every material plain, and every lightmap page texture 7.
    struct Fake;

    impl Skins for Fake {
        fn look(&mut self, name: &str) -> Result<Look> {
            Ok(Look { material: name.to_owned(), blend_mode: 0, still: Phase::PLAIN, animation: None })
        }

        fn lightmap(&mut self, _page: &str) -> Result<Option<usize>> {
            Ok(Some(7))
        }
    }

    #[test]
    fn the_units_own_view_draws_the_fifth_slot_and_nothing_of_a_node_without_one() {
        let loaded = cockpit_mesh();
        assert_eq!(view_slot(&loaded.mesh.nodes[0], 0), None, "level 0 does not stand in");
        assert_eq!(view_slot(&loaded.mesh.nodes[1], 0), Some(1));
        assert_eq!(view_slot(&loaded.mesh.nodes[1], 1), None, "the variant's own block");
        assert!(build_view_node(&loaded, 0, 0, &mut Fake).unwrap().is_none());
        let cockpit = build_view_node(&loaded, 1, 0, &mut Fake).unwrap().expect("the cockpit");
        assert_eq!(cockpit.groups.len(), 1);
        assert_eq!(cockpit.groups[0].look.material, "GLASS");
        // In the node's own frame: its pose is left to the instance.
        let xs: Vec<f32> = cockpit.vertices.iter().map(|v| v.position[0]).collect();
        assert_eq!(xs, vec![2.0, 0.0, 2.0]);
        assert!(cockpit.vertices.iter().all(|v| v.position[2] == 0.0));
    }

    #[test]
    fn a_pose_matrix_places_a_point_as_the_pose_does() {
        let pose =
            Pose { translation: [1.0, 2.0, 3.0], rotation: [0.5_f64.sqrt(), 0.0, 0.0, 0.5_f64.sqrt()] };
        let p = [0.3, -1.0, 2.0];
        let want = pose.apply(p).map(|c| c as f32);
        let got = pose_matrix(&pose).transform_point3(Vec3::from_array(p.map(|c| c as f32)));
        assert!((got - Vec3::from_array(want)).length() < 1e-5, "{got} against {want:?}");
        assert_eq!(pose_matrix(&IDENTITY), Mat4::IDENTITY);
    }
}
