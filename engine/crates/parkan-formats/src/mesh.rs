//! Object geometry: a `MESH` member is an NRes archive of streams.
//! See `docs/07-objects.md` and `openparkan/mesh.py`.

use crate::cursor::{FormatError, latin1};
use crate::nres::Archive;
use crate::pose::{IDENTITY, Pose};

pub const STREAM_NODE: u32 = 1;
pub const STREAM_SLOT: u32 = 2;
pub const STREAM_POSITION: u32 = 3;
pub const STREAM_NORMAL: u32 = 4;
pub const STREAM_UV: u32 = 5;
pub const STREAM_TRIANGLE: u32 = 6;
pub const STREAM_FACE: u32 = 7;
pub const FACE_STRIDE: usize = 16;
pub const STREAM_POSE_KEY: u32 = 8;
pub const STREAM_NAME: u32 = 9;
pub const STREAM_BATCH: u32 = 13;
pub const STREAM_LIGHTMAP_UV: u32 = 18;
pub const STREAM_FRAME_MAP: u32 = 19;

pub const NODE_SIZE: usize = 38;
pub const SLOT_HEADER_SIZE: usize = 0x8C;
pub const SLOT_SIZE: usize = 68;
pub const BATCH_SIZE: usize = 20;
pub const POSE_KEY_SIZE: usize = 24;
pub const SLOTS_PER_VARIANT: usize = 5;
pub const LOD_COUNT: usize = 4;
pub const NO_SLOT: u16 = 0xFFFF;
pub const NO_PARENT: u16 = 0xFFFF;
pub const NO_ANIMATION: u16 = 0xFFFF;
pub const NODE_INTERIOR: u16 = 0x0001;
/// A collision hull: tested against, never drawn.
pub const NODE_COLLISION: u16 = 0x0020;
/// The high byte of a batch's material word when the batch takes the lightmap.
pub const BATCH_LIT: u16 = 0x00;
pub const UV_SCALE: f64 = 1024.0;
pub const LIGHTMAP_UV_SCALE: f64 = 1024.0;
pub const QUATERNION_SCALE: f64 = 32767.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub name: String,
    pub flags: u16,
    pub parent: u16,
    pub anim_start: u16,
    pub fallback_key: u16,
    /// `[variant * 5 + lod]`, `NO_SLOT` where there is nothing.
    pub slot_index: [u16; 15],
}

impl Node {
    pub fn is_animated(&self) -> bool {
        self.anim_start != NO_ANIMATION
    }

    pub fn is_collision(&self) -> bool {
        self.flags & NODE_COLLISION != 0
    }

    pub fn is_interior(&self) -> bool {
        self.flags & NODE_INTERIOR != 0
    }

    /// The one slot drawn at `lod`; level 0 falls back to the variant's first slot.
    pub fn slot_for_lod(&self, lod: usize, variant: usize) -> Option<u16> {
        let block = &self.slot_index[variant * SLOTS_PER_VARIANT..(variant + 1) * SLOTS_PER_VARIANT];
        match block.get(lod) {
            Some(&s) if s != NO_SLOT => Some(s),
            _ if lod == 0 => block.iter().copied().find(|&s| s != NO_SLOT),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub first_triangle: u16,
    pub triangle_count: u16,
    pub first_batch: u16,
    pub batch_count: u16,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub sphere: [f32; 4],
    pub area: f32,
    pub volume: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    /// Index into the wear's materials.
    pub material: u16,
    pub flag: u16,
    pub first_index: u16,
    pub index_count: u16,
    pub first_vertex: u16,
    pub vertex_count: u16,
}

impl Batch {
    pub fn is_lit(&self) -> bool {
        self.flag == BATCH_LIT
    }

    /// `(first triangle, triangle count)`.
    pub fn triangles(&self) -> (usize, usize) {
        (usize::from(self.first_index) / 3, usize::from(self.index_count) / 3)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub translation: [f32; 3],
    pub time: f32,
    /// Conjugated from the file: the game builds its matrices left-handed.
    pub rotation: [f64; 4],
}

impl Key {
    pub fn pose(&self) -> Pose {
        Pose { translation: self.translation.map(f64::from), rotation: self.rotation }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub uv: Vec<[f64; 2]>,
    pub lightmap_uv: Vec<[f64; 2]>,
    /// Absolute vertex indices: each batch's first vertex already added.
    pub triangles: Vec<[u16; 3]>,
    pub nodes: Vec<Node>,
    pub slots: Vec<Slot>,
    pub batches: Vec<Batch>,
    /// Stream 7, one record per triangle: its flags word, and its normal (int16 ÷ 32767).
    pub face_flags: Vec<u16>,
    pub face_normals: Vec<[f32; 3]>,
    pub keys: Vec<Key>,
    pub frame_map: Vec<u16>,
    pub frame_count: u32,
    /// The authored bounding sphere: centre and radius.
    pub sphere: Option<([f32; 3], f32)>,
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn vec3(b: &[u8], at: usize) -> [f32; 3] {
    [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)]
}

impl Mesh {
    /// The key that puts a node at rest: frame 0 of an animated node's run.
    pub fn rest_key(&self, node: usize) -> Option<usize> {
        let n = &self.nodes[node];
        if n.is_animated() && self.frame_count > 0 && usize::from(n.anim_start) < self.frame_map.len() {
            return Some(usize::from(self.frame_map[usize::from(n.anim_start)]));
        }
        (usize::from(n.fallback_key) < self.keys.len()).then_some(usize::from(n.fallback_key))
    }

    pub fn local_pose(&self, node: usize) -> Pose {
        self.rest_key(node).and_then(|k| self.keys.get(k)).map_or(IDENTITY, Key::pose)
    }

    pub fn world_pose(&self, node: usize) -> Pose {
        self.world_pose_by(node, |n| self.local_pose(n))
    }

    /// A node's pose in model space, with every node of the chain posed by `local`.
    pub fn world_pose_by(&self, node: usize, local: impl Fn(usize) -> Pose) -> Pose {
        let mut pose = local(node);
        let mut seen = vec![node];
        let mut parent = self.nodes[node].parent;
        while parent != NO_PARENT
            && usize::from(parent) < self.nodes.len()
            && !seen.contains(&usize::from(parent))
        {
            let p = usize::from(parent);
            seen.push(p);
            pose = local(p).compose(&pose);
            parent = self.nodes[p].parent;
        }
        pose
    }

    /// A node's pose at a fractional frame, as `AniMesh.dll:0x10012880` finds it.
    ///
    /// The key is the run's entry at `floor(frame)`. Past the run, on a node that is
    /// not animated, or where the entry is at or past the node's fallback key, the
    /// fallback key is used. At a key's time, or the next key's, that key is taken
    /// whole; otherwise the two are blended by time.
    pub fn pose_at(&self, node: usize, frame: f64) -> Pose {
        let n = &self.nodes[node];
        let mut index = usize::from(n.fallback_key);
        let k = frame.floor();
        if n.is_animated() && k >= 0.0 && k < f64::from(self.frame_count) {
            let at = usize::from(n.anim_start) + k as usize;
            if let Some(&entry) = self.frame_map.get(at)
                && entry < n.fallback_key
            {
                index = usize::from(entry);
            }
        }
        let Some(key) = self.keys.get(index) else { return IDENTITY };
        let time = f64::from(key.time);
        let Some(next) = self.keys.get(index + 1).filter(|_| frame != time) else { return key.pose() };
        let next_time = f64::from(next.time);
        if frame == next_time {
            return next.pose();
        }
        if next_time == time {
            return key.pose();
        }
        key.pose().blend(&next.pose(), (frame - time) / (next_time - time))
    }

    /// Two frames and a weight, as a controller hands them (`0x10012560`): frame A
    /// alone at weight 0 or when B is negative, frame B alone at weight 1 or when A
    /// is negative, a blend between otherwise.
    pub fn blended_pose(&self, node: usize, frame_a: f64, frame_b: f64, weight: f64) -> Pose {
        let use_a = weight < 1.0 && frame_a >= 0.0;
        let use_b = weight > 0.0 && frame_b >= 0.0;
        match (use_a, use_b) {
            (true, true) => self.pose_at(node, frame_a).blend(&self.pose_at(node, frame_b), weight),
            (false, true) => self.pose_at(node, frame_b),
            _ => self.pose_at(node, frame_a.max(0.0)),
        }
    }

    /// [`Mesh::blended_pose`] as the pose walk uses it (`AniMesh.dll:0x10008d88`): the
    /// root node keeps its rotation but not its translation, so a body turns in the
    /// picture and its stride is carried by the object's own move instead.
    pub fn walk_pose(&self, node: usize, frame_a: f64, frame_b: f64, weight: f64) -> Pose {
        let pose = self.blended_pose(node, frame_a, frame_b, weight);
        if node == 0 { Pose { translation: [0.0; 3], ..pose } } else { pose }
    }

    /// The pose a host's socket replaces when this mesh is mounted.
    pub fn root_pose(&self) -> Pose {
        self.nodes.iter().position(|n| n.parent == NO_PARENT).map_or(IDENTITY, |i| self.local_pose(i))
    }

    fn slot_triangles(&self, slot: u16) -> &[[u16; 3]] {
        let Some(s) = self.slots.get(usize::from(slot)) else { return &[] };
        let start = usize::from(s.first_triangle).min(self.triangles.len());
        let stop = (usize::from(s.first_triangle) + usize::from(s.triangle_count)).min(self.triangles.len());
        &self.triangles[start..stop]
    }

    /// Which node poses each vertex, or -1.
    pub fn node_of_vertex(&self) -> Vec<i32> {
        let mut out = vec![-1; self.positions.len()];
        for (i, node) in self.nodes.iter().enumerate() {
            for &si in node.slot_index.iter().filter(|&&s| s != NO_SLOT) {
                for tri in self.slot_triangles(si) {
                    for &v in tri {
                        if let Some(slot) = out.get_mut(usize::from(v)) {
                            *slot = i as i32;
                        }
                    }
                }
            }
        }
        out
    }

    /// The lowest z of what is drawn at rest: level 0 of variant 0, hulls left out.
    pub fn lowest(&self) -> Option<f64> {
        let posed = self.posed_positions();
        self.nodes
            .iter()
            .filter(|n| !n.is_collision())
            .filter_map(|n| n.slot_for_lod(0, 0))
            .flat_map(|s| self.slot_triangles(s).iter().flatten())
            .filter_map(|&v| posed.get(usize::from(v)).map(|p| p[2]))
            .reduce(f64::min)
    }

    /// Vertex positions with each node's world pose applied, in model space.
    pub fn posed_positions(&self) -> Vec<[f64; 3]> {
        let mut out: Vec<[f64; 3]> = self.positions.iter().map(|p| p.map(f64::from)).collect();
        for (i, node) in self.nodes.iter().enumerate() {
            let pose = self.world_pose(i);
            if pose == IDENTITY {
                continue;
            }
            for &si in node.slot_index.iter().filter(|&&s| s != NO_SLOT) {
                for tri in self.slot_triangles(si) {
                    for &v in tri {
                        let v = usize::from(v);
                        if v < out.len() {
                            out[v] = pose.apply(self.positions[v].map(f64::from));
                        }
                    }
                }
            }
        }
        out
    }
}

/// Parse a `MESH` payload.
pub fn parse(blob: &[u8], name: &str) -> Result<Mesh, FormatError> {
    let inner = Archive::parse(blob.to_vec(), name.to_owned())?;
    let entry = |id: u32| inner.entries.iter().rev().find(|e| e.type_id() == id);
    let stream = |id: u32| -> Result<&[u8], FormatError> { entry(id).map_or(Ok(&[][..]), |e| inner.read(e)) };

    let raw_pos = stream(STREAM_POSITION)?;
    let nv = raw_pos.len() / 12;
    let positions = (0..nv).map(|i| vec3(raw_pos, i * 12)).collect();
    let raw_n = stream(STREAM_NORMAL)?;
    let normals = (0..nv.min(raw_n.len() / 4))
        .map(|i| std::array::from_fn(|k| f64::from(raw_n[i * 4 + k] as i8) / 127.0))
        .collect();
    let uv = |raw: &[u8], scale: f64| -> Vec<[f64; 2]> {
        (0..nv)
            .map(|i| [f64::from(u16_at(raw, i * 4)) / scale, f64::from(u16_at(raw, i * 4 + 2)) / scale])
            .collect()
    };
    let raw_uv = stream(STREAM_UV)?;
    if raw_uv.len() < nv * 4 {
        return Err(FormatError::invalid(name, "short UV stream"));
    }
    let uvs = uv(raw_uv, UV_SCALE);
    let raw_lm = stream(STREAM_LIGHTMAP_UV)?;
    let lightmap_uv =
        if nv > 0 && raw_lm.len() >= nv * 4 { uv(raw_lm, LIGHTMAP_UV_SCALE) } else { Vec::new() };

    let raw_tri = stream(STREAM_TRIANGLE)?;
    let raw_triangles: Vec<[u16; 3]> = (0..raw_tri.len() / 6)
        .map(|i| [u16_at(raw_tri, i * 6), u16_at(raw_tri, i * 6 + 2), u16_at(raw_tri, i * 6 + 4)])
        .collect();

    let raw_faces = stream(STREAM_FACE)?;
    let face_count = raw_faces.len() / FACE_STRIDE;
    let face_flags = (0..face_count).map(|i| u16_at(raw_faces, i * FACE_STRIDE)).collect();
    let face_normals = (0..face_count)
        .map(|i| {
            std::array::from_fn(|k| {
                f32::from(u16_at(raw_faces, i * FACE_STRIDE + 8 + 2 * k) as i16) / 32767.0
            })
        })
        .collect();

    let names = stream(STREAM_NAME)?;
    let name_at = |i: usize| {
        names.get(i * 32..(i + 1) * 32).map(|raw| {
            let end = raw.iter().position(|&b| b == 0).unwrap_or(32);
            latin1(&raw[..end])
        })
    };
    let headers = stream(STREAM_NODE)?;
    let n_nodes = entry(STREAM_NODE).map_or(0, |e| e.element_count as usize);
    let nodes = if headers.len() == n_nodes * NODE_SIZE {
        (0..n_nodes)
            .map(|i| {
                let w = |k: usize| u16_at(headers, i * NODE_SIZE + 2 * k);
                Node {
                    name: name_at(i).unwrap_or_default(),
                    flags: w(0),
                    parent: w(1),
                    anim_start: w(2),
                    fallback_key: w(3),
                    slot_index: std::array::from_fn(|k| w(4 + k)),
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let raw_slots = stream(STREAM_SLOT)?;
    let n_slots = entry(STREAM_SLOT).map_or(0, |e| e.element_count as usize);
    let slots = if raw_slots.len() == SLOT_HEADER_SIZE + n_slots * SLOT_SIZE {
        (0..n_slots)
            .map(|i| {
                let o = SLOT_HEADER_SIZE + i * SLOT_SIZE;
                Slot {
                    first_triangle: u16_at(raw_slots, o),
                    triangle_count: u16_at(raw_slots, o + 2),
                    first_batch: u16_at(raw_slots, o + 4),
                    batch_count: u16_at(raw_slots, o + 6),
                    aabb_min: vec3(raw_slots, o + 8),
                    aabb_max: vec3(raw_slots, o + 20),
                    sphere: std::array::from_fn(|k| f32_at(raw_slots, o + 0x20 + 4 * k)),
                    area: f32_at(raw_slots, o + 0x30),
                    volume: f32_at(raw_slots, o + 0x34),
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let sphere =
        (raw_slots.len() >= SLOT_HEADER_SIZE).then(|| (vec3(raw_slots, 24 * 4), f32_at(raw_slots, 27 * 4)));

    let raw_keys = stream(STREAM_POSE_KEY)?;
    let n_keys = entry(STREAM_POSE_KEY).map_or(0, |e| e.element_count as usize);
    let keys = (0..n_keys)
        .take_while(|i| (i + 1) * POSE_KEY_SIZE <= raw_keys.len())
        .map(|i| {
            let o = i * POSE_KEY_SIZE;
            let q = [0, 1, 2, 3].map(|k| f64::from(u16_at(raw_keys, o + 16 + 2 * k) as i16));
            let length = q.iter().map(|v| (v / QUATERNION_SCALE).powi(2)).sum::<f64>().sqrt();
            let rotation = if length < 1e-6 {
                [1.0, 0.0, 0.0, 0.0]
            } else {
                let s = QUATERNION_SCALE * length;
                [q[0] / s, -q[1] / s, -q[2] / s, -q[3] / s]
            };
            Key { translation: vec3(raw_keys, o), time: f32_at(raw_keys, o + 12), rotation }
        })
        .collect();

    let raw_frames = stream(STREAM_FRAME_MAP)?;
    let frame_map = (0..raw_frames.len() / 2).map(|i| u16_at(raw_frames, i * 2)).collect();
    let frame_count = entry(STREAM_FRAME_MAP).map_or(0, |e| e.link_count);

    let raw_batch = stream(STREAM_BATCH)?;
    let n_batches = entry(STREAM_BATCH).map_or(0, |e| e.element_count as usize);
    let batches: Vec<Batch> = (0..n_batches)
        .take_while(|i| (i + 1) * BATCH_SIZE <= raw_batch.len())
        .map(|i| {
            let w = |k: usize| u16_at(raw_batch, i * BATCH_SIZE + 2 * k);
            Batch {
                material: w(2) & 0xFF,
                flag: w(2) >> 8,
                index_count: w(4),
                first_index: w(5),
                vertex_count: w(7),
                first_vertex: w(8),
            }
        })
        .collect();

    let mut triangles = raw_triangles.clone();
    for b in &batches {
        let (first, count) = b.triangles();
        for t in first..(first + count).min(triangles.len()) {
            triangles[t] = raw_triangles[t].map(|v| v.wrapping_add(b.first_vertex));
        }
    }

    Ok(Mesh {
        name: name.to_owned(),
        positions,
        normals,
        uv: uvs,
        lightmap_uv,
        triangles,
        nodes,
        slots,
        batches,
        face_flags,
        face_normals,
        keys,
        frame_map,
        frame_count,
        sphere,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(slots: &[u16]) -> Node {
        let mut slot_index = [NO_SLOT; 15];
        slot_index[..slots.len()].copy_from_slice(slots);
        Node {
            name: String::new(),
            flags: 0,
            parent: NO_PARENT,
            anim_start: NO_ANIMATION,
            fallback_key: 0,
            slot_index,
        }
    }

    #[test]
    fn level_zero_falls_back_to_the_first_slot_a_variant_has() {
        assert_eq!(node(&[3, 4]).slot_for_lod(0, 0), Some(3));
        assert_eq!(node(&[NO_SLOT, NO_SLOT, NO_SLOT, NO_SLOT, 9]).slot_for_lod(0, 0), Some(9));
        assert_eq!(node(&[NO_SLOT, NO_SLOT, NO_SLOT, NO_SLOT, 9]).slot_for_lod(1, 0), None);
    }
}
