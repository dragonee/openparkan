//! Drawing text in the game font: each run laid out by
//! [`parkan_world::text::GameFont`], its glyphs drawn as quads over everything,
//! after the HUD.
//!
//! The game draws its font through render phase 8 (`Ngi32.dll`'s phase table at
//! `0x10036a30`, record 13 at `0x10036c6c`, its 25 states at `0x10034a00`): stage 0 is
//! `MODULATE` of the texture by the diffuse colour, its alpha is `SELECTARG1` of the
//! texture, stages 1-7 are disabled, the filters are point and point with no mip, and
//! the alpha test is on with `GREATEREQUAL` and the device's reference of 1. So the
//! run's colour **multiplies** the atlas rather than replacing it, and the key is the
//! atlas's own alpha -- palette index 0, cleared when the font's alpha surface is made
//! (`0x1000f698`). The `Ipol` blend table of `PAL.PAL` is the 8-bit software renderer's
//! and this build has none: `vrtTextOut` is a stub. See `docs/12-rsli.md`, "How a glyph
//! is drawn". Everything is in display space (docs/05).

use bytemuck::{Pod, Zeroable};
use parkan_world::text::{GameFont, TextRun};

use crate::DEPTH_FORMAT;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    anchor: [f32; 2],
    offset: [f32; 2],
    uv: [f32; 2],
    colour: [f32; 4],
}

pub struct TextRenderer {
    font: GameFont,
    pipeline: wgpu::RenderPipeline,
    screen: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: Option<wgpu::Buffer>,
    capacity: usize,
    count: u32,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        font: GameFont,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("font atlas"),
            size: wgpu::Extent3d { width: font.width, height: font.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &font.atlas,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * font.width),
                rows_per_image: Some(font.height),
            },
            wgpu::Extent3d { width: font.width, height: font.height, depth_or_array_layers: 1 },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text"),
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
                        view_dimension: wgpu::TextureViewDimension::D2,
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: screen.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { font, pipeline, screen, bind_group, vertices: None, capacity: 0, count: 0 }
    }

    pub fn font(&self) -> &GameFont {
        &self.font
    }

    /// Lay `runs` out and upload their glyphs.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, runs: &[TextRun]) {
        let vertices: Vec<GpuVertex> = runs
            .iter()
            .flat_map(|run| {
                self.font.layout(run).into_iter().flat_map(move |g| {
                    let [x, y] = g.at;
                    let [w, h] = g.size;
                    let [u0, v0, u1, v1] = g.uv;
                    let v = |dx: f32, dy: f32, u: f32, t: f32| GpuVertex {
                        anchor: run.anchor,
                        offset: [x + dx, y + dy],
                        uv: [u, t],
                        colour: run.colour,
                    };
                    [
                        v(0.0, 0.0, u0, v0),
                        v(w, 0.0, u1, v0),
                        v(w, h, u1, v1),
                        v(0.0, 0.0, u0, v0),
                        v(w, h, u1, v1),
                        v(0.0, h, u0, v1),
                    ]
                })
            })
            .collect();
        self.count = vertices.len() as u32;
        if vertices.is_empty() {
            return;
        }
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("text"),
                size: (self.capacity * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buffer) = &self.vertices {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    /// Hand the pass the screen's size in pixels, which glyphs are placed in.
    pub fn resize(&self, queue: &wgpu::Queue, (width, height): (u32, u32)) {
        let size = [width as f32, height as f32, 0.0, 0.0];
        queue.write_buffer(&self.screen, 0, bytemuck::cast_slice(&size));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(buffer) = &self.vertices else { return };
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}
