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

/// Whether a face of material `name` is a portal material: a doorway or room opening. A mover
/// and a round pass a portal by its batch's word ([`passing_triangles`]), and it is drawn by
/// that word too ([`Portal`]).
pub fn doorway(name: &str) -> bool {
    PORTAL_MATERIALS.iter().any(|m| name.eq_ignore_ascii_case(m))
}

/// The batch word's portal bit: the doorway and portal quads between a building's cells, 633
/// of the install's 15153 batches (docs/24, "A building is drawn cell by cell through its
/// portals").
pub const BATCH_PORTAL: u32 = 0x8;
/// A portal whose quad fades in close by, as a sign does: the 66 `PORTAL_001` and `PORTAL_004`
/// batches, and no other (`Terrain.dll:0x1002c4ea`).
pub const BATCH_PORTAL_SIGN: u32 = 0x10;
/// A portal whose quad is never seen and whose room is always drawn: 170 `DEFAULT` batches
/// (`0x1002c5ae`).
pub const BATCH_PORTAL_OPEN: u32 = 0x40;

/// `PortalNearDist` and `PortalFarDist`, the render settings' entries 27 and 28 at their
/// compiled defaults (docs/10, "The render settings"), which `CShade` keeps at `+0x1660` and
/// `+0x1664` (`0x10046d77`).
pub const PORTAL_NEAR_DIST: f32 = 75.0;
pub const PORTAL_FAR_DIST: f32 = 95.0;
/// The field of view, in radians across, at which a portal's distances are the settings' own:
/// they are scaled by 1.3 (`0x1009a908`) and divided by the camera's field (`0x1002c410`).
pub const PORTAL_FIELD: f32 = 1.3;
/// A sign's fade, on the square root of its distance, times the field (`0x1009a8ec`,
/// `0x1009a8f4`).
pub const SIGN_NEAR: f32 = 2.6;
pub const SIGN_FAR: f32 = 7.8;

/// How a portal quad is drawn (`Terrain.dll:0x1002c4d0`, docs/24, "A building is drawn cell by
/// cell through its portals"). A portal whose word carries [`BATCH_PORTAL_OPEN`] is never seen,
/// and is not built at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portal {
    /// A `DEFAULT` doorway: gone within [`PORTAL_NEAR_DIST`], black from [`PORTAL_FAR_DIST`]
    /// on, and beyond it the room it opens onto is not drawn.
    Gate,
    /// A `PORTAL_*` sign: gone within a few units, whole from about 36 on.
    Sign,
}

impl Portal {
    /// The kind a batch's word gives, `None` for a batch that is no portal or is never seen.
    pub fn of(word: u32) -> Option<Self> {
        if word & BATCH_PORTAL == 0 || word & BATCH_PORTAL_OPEN != 0 {
            return None;
        }
        Some(if word & BATCH_PORTAL_SIGN != 0 { Self::Sign } else { Self::Gate })
    }

    /// The alpha its quad draws with, `distance` from the camera to the quad's first corner,
    /// under a field of view of `field` radians across (`0x1002c4d0`): it replaces the
    /// material's ambient alpha, which the device takes as the alpha (docs/07).
    pub fn alpha(self, distance: f32, field: f32) -> f32 {
        let [near, far, root] = self.range();
        let at = if root > 0.5 { distance.max(0.0).sqrt() } else { distance };
        ((at - near / field) / ((far - near) / field)).clamp(0.0, 1.0)
    }

    /// Its fade as the model shader takes it: where it starts and where it is whole under a
    /// field of one radian, and 1 where it runs on the square root of the distance.
    pub fn range(self) -> [f32; 3] {
        match self {
            Self::Gate => [PORTAL_FIELD * PORTAL_NEAR_DIST, PORTAL_FIELD * PORTAL_FAR_DIST, 0.0],
            Self::Sign => [SIGN_NEAR, SIGN_FAR, 1.0],
        }
    }
}

/// A portal quad's batch as drawn: its kind, and its first corner in the model's frame, from
/// which the camera's distance is taken (the primitive's first vertex, `0x1003c340`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalQuad {
    pub kind: Portal,
    pub anchor: [f32; 3],
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

/// Which of `mesh`'s triangles a round passes by its batch's word, as a mover does
/// (`parkan_sim::solid::COLLISION_SKIPS_BATCH`): the batches flagged 8, the query's filter
/// built the same way for a round (`Control.dll:0x1001d9fa`). Empty where none is, so a mesh
/// without portals costs nothing. *Measured*: the 633 such batches are `fortif.rlb`'s doorway
/// and portal quads, so a shot reaches the door behind the black doorway.
pub fn passing_triangles(mesh: &Mesh) -> Vec<bool> {
    let passes = |flags: u32| flags & parkan_sim::solid::COLLISION_SKIPS_BATCH != 0;
    if !mesh.batches.iter().any(|b| passes(b.flags)) {
        return Vec::new();
    }
    let mut out = vec![false; mesh.triangles.len()];
    for batch in mesh.batches.iter().filter(|b| passes(b.flags)) {
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
    /// A portal quad's batch, which fades by its distance from the camera and is filed
    /// translucent, as every batch whose word carries 8 is (`Terrain.dll:0x1004552a`).
    pub portal: Option<PortalQuad>,
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
            // A portal quad fades by its distance from the camera, and one that is never seen
            // is not built (docs/24, "A building is drawn cell by cell through its portals").
            //
            // STAND-IN: docs/24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read
            // -- no cell is culled: beyond far the game leaves a doorway's room undrawn, and here
            // it is drawn behind the black the doorway's fade reaches there. And 57 batches wear
            // `DEFAULT` with no portal bit: three trees, four internal systems, three turrets and
            // some buildings' lower levels of detail. Nothing in the draw read skips them, so the
            // game draws them black; they are left out here, as every portal material was
            // before, until they are looked at.
            let portal = Portal::of(batch.flags);
            if batch.flags & BATCH_PORTAL != 0 && portal.is_none() {
                continue;
            }
            if portal.is_none() && doorway(&name) {
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
                // The distance is taken to the quad's first corner: the first vertex the
                // primitive holds, its first triangle's first (`0x10045149`).
                let portal = portal.map(|kind| PortalQuad {
                    kind,
                    anchor: mesh.triangles.get(first).map_or([0.0; 3], |t| place(usize::from(t[0])).position),
                });
                self.groups.push(Group { start, count, look, lightmap, portal });
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
            flags: 0,
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
            face_two_sided: Vec::new(),
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
    fn a_portals_word_says_how_it_fades() {
        // The install's six portal words (docs/24, "A building is drawn cell by cell").
        assert_eq!(Portal::of(0x108), Some(Portal::Gate));
        assert_eq!(Portal::of(0x4108), Some(Portal::Gate));
        assert_eq!(Portal::of(0x118), Some(Portal::Sign));
        assert_eq!(Portal::of(0x4118), Some(Portal::Sign));
        assert_eq!(Portal::of(0x148), None, "never seen");
        assert_eq!(Portal::of(0x4148), None, "never seen");
        assert_eq!(Portal::of(0x100), None, "no portal");
        // At the game's 1.3 rad a doorway is gone to 75 and black from 95; zoomed in to half
        // the field, both distances double.
        let gate = |d: f32, field: f32| Portal::Gate.alpha(d, field);
        let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(near(gate(74.0, 1.3), 0.0) && near(gate(85.0, 1.3), 0.5) && near(gate(96.0, 1.3), 1.0));
        assert!(near(gate(170.0, 0.65), 0.5));
        // A sign on the square root of its distance: gone within 4, whole from 36.
        let sign = |d: f32| Portal::Sign.alpha(d, 1.3);
        assert!(near(sign(3.9), 0.0) && near(sign(16.0), 0.5) && near(sign(36.1), 1.0));
    }

    #[test]
    fn a_portal_batch_builds_a_fading_group_and_one_never_seen_builds_none() {
        let mut loaded = cockpit_mesh();
        loaded.mesh.batches[0].flags = 0x4108;
        loaded.wear.materials[0] = "DEFAULT".to_owned();
        let model = build_node(&loaded, 0, 0, &mut Fake).unwrap().expect("the doorway");
        assert_eq!(model.groups[0].portal, Some(PortalQuad { kind: Portal::Gate, anchor: [0.0; 3] }));
        // Its first corner is the first triangle's first.
        loaded.mesh.triangles[0] = [2, 1, 0];
        let model = build_node(&loaded, 0, 0, &mut Fake).unwrap().expect("the doorway");
        assert_eq!(model.groups[0].portal.map(|p| p.anchor), Some([0.0, 1.0, 0.0]));
        loaded.mesh.batches[0].flags = 0x148;
        assert!(build_node(&loaded, 0, 0, &mut Fake).unwrap().is_none(), "an open portal is not drawn");
        // A plain batch is no portal.
        loaded.mesh.batches[0].flags = 0x100;
        loaded.wear.materials[0] = "HULL".to_owned();
        let model = build_node(&loaded, 0, 0, &mut Fake).unwrap().expect("the hull");
        assert_eq!(model.groups[0].portal, None);
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
