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

pub struct DomeRenderer {
    pipeline: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    group: wgpu::BindGroup,
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
            // its far plane and out of the fog is not read: it is drawn first, behind
            // everything, without depth.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
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
        Self { pipeline, camera, group, vertices, indices, count, positions: positions.to_vec() }
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

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }
}
