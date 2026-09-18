//! Drawing the HUD's art: the batches a [`parkan_world::hud::Painter`] gives, in order,
//! over everything, from the interface's pages held as one texture array with a white
//! layer after them for flat colour.
//!
//! The pass draws into the target read without sRGB decoding, so art, tint and blend meet in
//! display space: a weapon row's bar on the recording of Mission 01 reads (74, 146, 92), its
//! half-alpha green fill over the 20% dark green bar over the sky, blended in display space.
//!
//! STAND-IN: docs/35-hud.md#how-the-radar-draws--read -- the stage sets 1 and 7 a sprite may
//! pick, and how the 2D layer samples, are not read: pages are sampled nearest.

use bytemuck::{Pod, Zeroable};
use parkan_world::hud::{Batch, Blend, Layer, Page};

use crate::DEPTH_FORMAT;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 2],
    uv: [f32; 2],
    layer: u32,
    colour: [f32; 4],
}

pub struct UiRenderer {
    alpha: wgpu::RenderPipeline,
    add: wgpu::RenderPipeline,
    screen: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// The pages' shared size, and the white layer's index.
    size: u32,
    white: u32,
    vertices: Option<wgpu::Buffer>,
    capacity: usize,
    runs: Vec<(Blend, Layer, std::ops::Range<u32>)>,
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        pages: &[Page],
    ) -> Self {
        let size = pages.iter().map(|p| p.width.max(p.height)).max().unwrap_or(1).max(1);
        let white = pages.len() as u32;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ui pages"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: white + 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let layer = |index: u32, rgba: &[u8], width: u32, height: u32| {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: index },
                    aspect: wgpu::TextureAspect::All,
                },
                rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            );
        };
        for (i, p) in pages.iter().enumerate() {
            if p.rgba.len() == (4 * p.width * p.height) as usize && p.width > 0 && p.height > 0 {
                layer(i as u32, &p.rgba, p.width, p.height);
            }
        }
        layer(white, &vec![255; (4 * size * size) as usize], size, size);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: screen.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("ui.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |blend: wgpu::BlendState, label| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<GpuVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2, 1 => Float32x2, 2 => Uint32, 3 => Float32x4
                        ],
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
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
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        Self {
            alpha: pipeline(wgpu::BlendState::ALPHA_BLENDING, "ui alpha"),
            add: pipeline(add, "ui add"),
            screen,
            bind_group,
            size,
            white,
            vertices: None,
            capacity: 0,
            runs: Vec::new(),
        }
    }

    /// This frame's batches, on a `width` × `height` screen.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        (width, height): (u32, u32),
        batches: &[Batch],
    ) {
        let size = [width as f32, height as f32, self.size as f32, self.size as f32];
        queue.write_buffer(&self.screen, 0, bytemuck::cast_slice(&size));
        self.runs.clear();
        let mut vertices = Vec::new();
        for b in batches {
            let start = vertices.len() as u32;
            vertices.extend(b.vertices.iter().map(|v| GpuVertex {
                position: v.position,
                uv: if v.page.is_some() { v.uv } else { [0.5; 2] },
                layer: v.page.map_or(self.white, u32::from),
                colour: v.colour,
            }));
            if vertices.len() as u32 > start {
                self.runs.push((b.blend, b.layer, start..vertices.len() as u32));
            }
        }
        if vertices.is_empty() {
            return;
        }
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui"),
                size: (self.capacity * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buffer) = &self.vertices {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// The batches of one layer, the layers drawn in their own order.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, layer: Layer) {
        let Some(buffer) = &self.vertices else { return };
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for (blend, _, range) in self.runs.iter().filter(|r| r.1 == layer) {
            pass.set_pipeline(match blend {
                Blend::Alpha => &self.alpha,
                Blend::Add => &self.add,
            });
            pass.draw(range.clone(), 0..1);
        }
    }
}
