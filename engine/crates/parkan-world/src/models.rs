//! Placed objects, ready to draw: each distinct object built once from its
//! parts, and an instance per placement.
//!
//! A model is rigid in M2: every node sits at its rest pose, frame 0 of an
//! animated node's run. Level 0 of variant 0 is drawn, collision hulls never.
//! See `docs/07-objects.md`.

use std::collections::HashMap;

use anyhow::Result;
use parkan_formats::mission::Mission;
use parkan_formats::pose::rotate;

use crate::assembly::Assembly;
use crate::textures::{Look, TextureStore};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Triangles of one model drawn with one material, as a range of `Model::indices`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub start: u32,
    pub count: u32,
    pub look: Look,
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
}

pub struct Objects {
    pub models: Vec<Model>,
    pub instances: Vec<Instance>,
    /// The mission object index each instance places.
    pub placed: Vec<usize>,
}

fn build_model(
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
            let Some(slot) = node.slot_for_lod(0, 0).and_then(|s| m.slots.get(usize::from(s))) else {
                continue;
            };
            let batches =
                usize::from(slot.first_batch)..usize::from(slot.first_batch) + usize::from(slot.batch_count);
            for batch in batches.filter_map(|b| m.batches.get(b)) {
                let name =
                    loaded.wear.materials.get(usize::from(batch.material)).cloned().unwrap_or_default();
                let look = store.look(&name)?;
                let start = model.indices.len() as u32;
                let mut remap: HashMap<u16, u32> = HashMap::new();
                let (first, count) = batch.triangles();
                for tri in m.triangles.iter().skip(first).take(count) {
                    for &v in tri {
                        let index = *remap.entry(v).or_insert_with(|| {
                            let vi = usize::from(v);
                            let local = posed.get(vi).copied().unwrap_or_default();
                            let normal = m.normals.get(vi).copied().unwrap_or([0.0, 0.0, 1.0]);
                            let normal = match owner.get(vi) {
                                Some(&o) if o >= 0 => rotate(rotations[o as usize], normal),
                                _ => normal,
                            };
                            let uv = m.uv.get(vi).copied().unwrap_or_default();
                            model.vertices.push(Vertex {
                                position: part.pose.apply(local).map(|c| c as f32),
                                normal: rotate(part.pose.rotation, normal).map(|c| c as f32),
                                uv: uv.map(|c| c as f32),
                            });
                            model.vertices.len() as u32 - 1
                        });
                        model.indices.push(index);
                    }
                }
                let count = model.indices.len() as u32 - start;
                if count > 0 {
                    model.groups.push(Group { start, count, look });
                }
            }
        }
    }
    Ok((!model.indices.is_empty()).then_some(model))
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
            });
            placed.push(i);
        }
    }
    Ok(Objects { models, instances, placed })
}
