//! Drawing the ground built by `parkan_world::terrain`.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use parkan_world::terrain::{Layer, Terrain};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;
use crate::frame::FrameUniform;
use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv1: [f32; 2],
    uv2: [f32; 2],
    blend: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LayersUniform {
    tint1: [f32; 4],
    tint2: [f32; 4],
}

struct DrawGroup {
    start: u32,
    count: u32,
    bind_group: wgpu::BindGroup,
}

pub struct TerrainRenderer {
    pipeline: wgpu::RenderPipeline,
    frame: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    groups: Vec<DrawGroup>,
}

impl TerrainRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        terrain: &Terrain,
        textures: &GpuTextures,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("terrain.wgsl"));
        let uniform = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain frame"),
            entries: &[uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terrain layers"),
            entries: &[
                uniform(0, wgpu::ShaderStages::FRAGMENT),
                texture(1),
                texture(2),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terrain"),
            bind_group_layouts: &[Some(&frame_layout), Some(&group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terrain"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2, 4 => Float32
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
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

        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terrain frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terrain frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame.as_entire_binding() }],
        });

        let gpu_vertices: Vec<GpuVertex> = terrain
            .vertices
            .iter()
            .map(|v| GpuVertex {
                position: v.position,
                normal: v.normal,
                uv1: v.uv1,
                uv2: v.uv2,
                blend: v.blend,
            })
            .collect();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain vertices"),
            contents: bytemuck::cast_slice(&gpu_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("terrain indices"),
            contents: bytemuck::cast_slice(&terrain.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let view_of = |layer: Option<&Layer>| textures.view(layer.and_then(|l| l.texture));
        let groups = terrain
            .groups
            .iter()
            .map(|g| {
                let tint = |l: Option<&Layer>, on: bool| {
                    let [r, gr, b] = l.map_or([1.0; 3], |l| l.diffuse);
                    [r, gr, b, f32::from(u8::from(on))]
                };
                let uniform = LayersUniform {
                    tint1: tint(Some(&g.layer1), true),
                    tint2: tint(g.layer2.as_ref(), g.layer2.as_ref().is_some_and(|l| l.texture.is_some())),
                };
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain layers"),
                    contents: bytemuck::bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&g.layer1.material),
                    layout: &group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(view_of(Some(&g.layer1))),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(view_of(g.layer2.as_ref())),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&textures.sampler),
                        },
                    ],
                });
                DrawGroup { start: g.start, count: g.count, bind_group }
            })
            .collect();
        Self { pipeline, frame, frame_bind_group, vertices, indices, groups }
    }

    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, lighting: &crate::frame::Lighting) {
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&FrameUniform::new(view_proj, lighting)));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for g in &self.groups {
            pass.set_bind_group(1, &g.bind_group, &[]);
            pass.draw_indexed(g.start..g.start + g.count, 0, 0..1);
        }
    }
}
