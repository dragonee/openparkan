//! The wgpu renderer. It draws what it is handed and never opens a file.
//!
//! Everything is in game space: Z up, +X east, +Y north, metres. The camera
//! matrix uses reverse Z — depth clears to 0 and nearer is greater — so the
//! far distances of a map cost no precision up close.

use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;

pub mod frame;
pub mod models;
pub mod terrain;
pub mod textures;

pub use models::ModelRenderer;
pub use terrain::TerrainRenderer;
pub use textures::GpuTextures;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The colour format of an offscreen capture.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const CLEAR: wgpu::Color = wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 };

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    colour: [f32; 3],
}

/// A box drawn at a place: the M0 stand-in for an object's mesh.
#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub position: Vec3,
    pub half_size: f32,
    pub colour: [f32; 3],
}

/// What M0 draws: markers and a ground grid at z = 0.
#[derive(Clone, Debug, Default)]
pub struct SceneData {
    pub markers: Vec<Marker>,
    pub grid_min: Vec2,
    pub grid_max: Vec2,
    pub grid_step: f32,
}

/// A device and its queue.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// A device for `surface`, or for offscreen work when there is none.
    pub async fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface,
                apply_limit_buckets: false,
            })
            .await
            .context("no graphics adapter")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("parkan"), ..Default::default() })
            .await
            .context("no graphics device")?;
        Ok(Self { instance, adapter, device, queue })
    }

    pub async fn headless() -> Result<Self> {
        Self::new(wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle()), None).await
    }
}

pub struct Renderer {
    format: wgpu::TextureFormat,
    solid: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    triangles: Option<(wgpu::Buffer, u32)>,
    grid: Option<(wgpu::Buffer, u32)>,
    depth: Option<(wgpu::TextureView, u32, u32)>,
    terrain: Option<TerrainRenderer>,
    objects: Option<ModelRenderer>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("debug"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |topology, label| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                    })],
                },
                primitive: wgpu::PrimitiveState { topology, ..Default::default() },
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
            })
        };
        Self {
            format,
            solid: pipeline(wgpu::PrimitiveTopology::TriangleList, "solid"),
            lines: pipeline(wgpu::PrimitiveTopology::LineList, "lines"),
            camera,
            bind_group,
            triangles: None,
            grid: None,
            depth: None,
            terrain: None,
            objects: None,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn set_scene(&mut self, device: &wgpu::Device, scene: &SceneData) {
        let mut triangles = Vec::new();
        for m in &scene.markers {
            push_box(&mut triangles, m);
        }
        let mut lines = Vec::new();
        push_grid(&mut lines, scene);
        let upload = |data: &[Vertex], label| {
            (!data.is_empty()).then(|| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (buffer, data.len() as u32)
            })
        };
        self.triangles = upload(&triangles, "markers");
        self.grid = upload(&lines, "grid");
    }

    /// Draw this world from now on: its textures, its ground and its placed objects.
    pub fn set_world(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &[parkan_world::textures::Texture],
        terrain: Option<&parkan_world::terrain::Terrain>,
        objects: Option<&parkan_world::models::Objects>,
    ) {
        let bank = GpuTextures::new(device, queue, textures);
        self.terrain = terrain.map(|t| TerrainRenderer::new(device, self.format, t, &bank));
        self.objects = objects.map(|o| ModelRenderer::new(device, self.format, o, &bank));
    }

    /// Draw the scene into `target`, a view of a `width` × `height` texture.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        (width, height): (u32, u32),
        view_proj: Mat4,
    ) {
        if self.depth.as_ref().is_none_or(|&(_, w, h)| (w, h) != (width, height)) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((texture.create_view(&Default::default()), width, height));
        }
        queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&view_proj.to_cols_array()));
        if let Some(terrain) = &self.terrain {
            terrain.prepare(queue, view_proj);
        }
        if let Some(objects) = &self.objects {
            objects.prepare(queue, view_proj);
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let depth = &self.depth.as_ref().expect("made above").0;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(CLEAR), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(terrain) = &self.terrain {
                terrain.draw(&mut pass);
            }
            if let Some(objects) = &self.objects {
                objects.draw(&mut pass);
            }
            pass.set_bind_group(0, &self.bind_group, &[]);
            for (pipeline, geometry) in [(&self.lines, &self.grid), (&self.solid, &self.triangles)] {
                if let Some((buffer, count)) = geometry {
                    pass.set_pipeline(pipeline);
                    pass.set_vertex_buffer(0, buffer.slice(..));
                    pass.draw(0..*count, 0..1);
                }
            }
        }
        queue.submit([encoder.finish()]);
    }
}

/// Render one frame offscreen with `renderer`, made for `CAPTURE_FORMAT`, and
/// return it as RGBA8 rows, top row first.
pub fn capture(
    gpu: &Gpu,
    renderer: &mut Renderer,
    (width, height): (u32, u32),
    view_proj: Mat4,
) -> Result<Vec<u8>> {
    let device = &gpu.device;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("capture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: CAPTURE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.draw(device, &gpu.queue, &texture.create_view(&Default::default()), (width, height), view_proj);

    let row = 4 * width;
    let padded = row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture readback"),
        size: u64::from(padded * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| anyhow!("{e:?}"))?;
    rx.recv()?.map_err(|e| anyhow!("{e:?}"))?;
    let mapped = buffer.get_mapped_range(..).map_err(|e| anyhow!("{e:?}"))?;
    let mut out = Vec::with_capacity((row * height) as usize);
    for y in 0..height {
        let start = (y * padded) as usize;
        out.extend_from_slice(&mapped[start..start + row as usize]);
    }
    Ok(out)
}

fn push_box(out: &mut Vec<Vertex>, m: &Marker) {
    let h = m.half_size;
    let c = |x: f32, y: f32, z: f32| m.position + Vec3::new(x * h, y * h, z * h + h);
    let corners = [
        c(-1., -1., -1.),
        c(1., -1., -1.),
        c(1., 1., -1.),
        c(-1., 1., -1.),
        c(-1., -1., 1.),
        c(1., -1., 1.),
        c(1., 1., 1.),
        c(-1., 1., 1.),
    ];
    // Each face a shade of the colour, so a box reads as a solid.
    let faces: [([usize; 4], f32); 6] = [
        ([4, 5, 6, 7], 1.0),
        ([0, 3, 2, 1], 0.35),
        ([0, 1, 5, 4], 0.7),
        ([2, 3, 7, 6], 0.55),
        ([1, 2, 6, 5], 0.85),
        ([3, 0, 4, 7], 0.45),
    ];
    for (idx, shade) in faces {
        let colour = m.colour.map(|v| v * shade);
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(Vertex { position: corners[idx[i]].to_array(), colour });
        }
    }
}

fn push_grid(out: &mut Vec<Vertex>, scene: &SceneData) {
    if scene.grid_step <= 0.0 {
        return;
    }
    let colour = [0.22, 0.26, 0.3];
    let (lo, hi) = (scene.grid_min, scene.grid_max);
    let mut x = lo.x;
    while x <= hi.x + 1e-3 {
        out.push(Vertex { position: [x, lo.y, 0.0], colour });
        out.push(Vertex { position: [x, hi.y, 0.0], colour });
        x += scene.grid_step;
    }
    let mut y = lo.y;
    while y <= hi.y + 1e-3 {
        out.push(Vertex { position: [lo.x, y, 0.0], colour });
        out.push(Vertex { position: [hi.x, y, 0.0], colour });
        y += scene.grid_step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marker_is_a_closed_box_standing_on_its_position() {
        let mut v = Vec::new();
        push_box(&mut v, &Marker { position: Vec3::new(10.0, 20.0, 5.0), half_size: 2.0, colour: [1.0; 3] });
        assert_eq!(v.len(), 36);
        let zs: Vec<f32> = v.iter().map(|p| p.position[2]).collect();
        assert_eq!(zs.iter().cloned().fold(f32::MAX, f32::min), 5.0);
        assert_eq!(zs.iter().cloned().fold(f32::MIN, f32::max), 9.0);
    }

    #[test]
    fn a_grid_spans_its_bounds() {
        let scene = SceneData {
            grid_min: Vec2::ZERO,
            grid_max: Vec2::splat(100.0),
            grid_step: 50.0,
            ..Default::default()
        };
        let mut v = Vec::new();
        push_grid(&mut v, &scene);
        assert_eq!(v.len(), 12);
    }
}
