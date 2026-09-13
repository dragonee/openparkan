//! Drawing the ground built by `parkan_world::terrain`.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use parkan_world::terrain::{Layer, Terrain};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;

/// The sun the ground is lit by until M5 reads the sky.
///
/// STAND-IN: docs/10-sky.md#not-resolved -- what the sun does with its values
/// is not read, and no sky keyframe is interpolated yet.
pub const LIGHT_DIRECTION: Vec3 = Vec3::new(-0.35, -0.45, -0.82);
pub const LIGHT_COLOUR: [f32; 3] = [0.85, 0.85, 0.8];
/// STAND-IN: docs/10-sky.md -- Mission 01's noon scene colour, 40/255 grey,
/// held fixed until M5 interpolates the sky.
pub const SCENE_COLOUR: [f32; 3] = [0.16, 0.16, 0.16];

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
struct FrameUniform {
    view_proj: [f32; 16],
    light_direction: [f32; 4],
    light_colour: [f32; 4],
    scene_colour: [f32; 4],
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

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    t: &parkan_world::terrain::Texture,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&t.name),
        size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        mip_level_count: t.levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, pixels) in t.levels.iter().enumerate() {
        let (w, h) = ((t.width >> level).max(1), (t.height >> level).max(1));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }
    texture.create_view(&Default::default())
}

fn white(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    upload_texture(
        device,
        queue,
        &parkan_world::terrain::Texture {
            name: "white".into(),
            width: 1,
            height: 1,
            levels: vec![vec![255; 4]],
        },
    )
}

impl TerrainRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        terrain: &Terrain,
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ground"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let views: Vec<wgpu::TextureView> =
            terrain.textures.iter().map(|t| upload_texture(device, queue, t)).collect();
        let blank = white(device, queue);
        let view_of = |layer: Option<&Layer>| layer.and_then(|l| l.texture).map_or(&blank, |i| &views[i]);
        let groups = terrain
            .groups
            .iter()
            .map(|g| {
                let tint = |l: Option<&Layer>, on: bool| {
                    let [r, gr, b] = l.map_or([1.0; 3], |l| l.tint);
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
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });
                DrawGroup { start: g.start, count: g.count, bind_group }
            })
            .collect();
        Self { pipeline, frame, frame_bind_group, vertices, indices, groups }
    }

    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        let d = LIGHT_DIRECTION.normalize();
        let [r, g, b] = LIGHT_COLOUR;
        let [sr, sg, sb] = SCENE_COLOUR;
        let uniform = FrameUniform {
            view_proj: view_proj.to_cols_array(),
            light_direction: [d.x, d.y, d.z, 0.0],
            light_colour: [r, g, b, 1.0],
            scene_colour: [sr, sg, sb, 1.0],
        };
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&uniform));
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
