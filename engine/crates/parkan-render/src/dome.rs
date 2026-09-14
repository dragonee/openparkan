//! Drawing the sky dome: `docs/10-sky.md`, "The dome". It is drawn at the camera's
//! position with no rotation (`Terrain.dll:0x1007a17d`), in vertex colours.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    colour: [f32; 3],
}

/// How the dome meets the depth buffer. The sky's draws at the camera, as the dome is
/// drawn, are depth-tested and write no depth; only the one on a fixed matrix
/// (`0x1007a37a`) turns the test off (render states 7 and 14, `Terrain.dll:0x100302fb`).
/// Drawn first into a cleared buffer, the test passes everywhere.
pub fn depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Greater),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

pub struct DomeRenderer {
    pipeline: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// The reflection camera's, for the water's reflection.
    reflection_camera: wgpu::Buffer,
    reflection_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    positions: Vec<Vec3>,
}

impl DomeRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        positions: &[Vec3],
        indices: &[u32],
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("dome.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dome camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dome camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dome camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
        });
        let reflection_camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dome reflection camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let reflection_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dome reflection camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: reflection_camera.as_entire_binding() }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dome"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dome"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            // STAND-IN: docs/10-sky.md#the-dome -- how the engine keeps a 34 km dome past
            // its far plane and out of a fog that ends by 700 is not read: it is drawn
            // first, under a projection with no far plane, unfogged but for its rim.
            depth_stencil: Some(depth_state()),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dome"),
            size: (positions.len() * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let count = indices.len() as u32;
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dome"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            pipeline,
            camera,
            group,
            reflection_camera,
            reflection_group,
            vertices,
            indices,
            count,
            positions: positions.to_vec(),
        }
    }

    /// This frame's camera and the dome's colours, one a vertex.
    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, eye: Vec3, colours: &[[f32; 3]]) {
        let placed = view_proj * Mat4::from_translation(eye);
        queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&placed.to_cols_array()));
        let vertices: Vec<GpuVertex> = self
            .positions
            .iter()
            .zip(colours.iter().chain(std::iter::repeat(&[0.0; 3])))
            .map(|(p, c)| GpuVertex { position: p.to_array(), colour: *c })
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
    }

    /// The reflection camera, at its own eye, for [`DomeRenderer::draw_reflection`].
    pub fn prepare_reflection(&self, queue: &wgpu::Queue, view_proj: Mat4, eye: Vec3) {
        let placed = view_proj * Mat4::from_translation(eye);
        queue.write_buffer(&self.reflection_camera, 0, bytemuck::bytes_of(&placed.to_cols_array()));
    }

    /// The dome as the reflection camera sees it.
    pub fn draw_reflection(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.reflection_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        // STAND-IN: docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured
        // -- how the sky's textures draw (the nebula, the stars, the clouds, the sun and
        // moon sprites, the lens flare) is not read; none is drawn, only the dome's
        // vertex colours.
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dome_is_depth_tested_without_writing_depth_and_passes_against_a_cleared_buffer() {
        let state = depth_state();
        assert_eq!(state.depth_write_enabled, Some(false));
        assert_eq!(state.depth_compare, Some(wgpu::CompareFunction::Greater), "the scene's own test");
        // Reverse depth clears to 0; the dome's rim, 24 km out at eye height, still
        // lands in front of that under a projection with no far plane.
        let proj = Mat4::perspective_infinite_reverse_rh(1.0, 16.0 / 9.0, 0.1);
        let rim = proj * Mat4::look_to_rh(Vec3::ZERO, Vec3::Y, Vec3::Z);
        let depth = rim.project_point3(Vec3::new(0.0, 24_142.1, 0.0)).z;
        assert!(depth > 0.0 && depth < 1.0, "{depth}");
    }
}
