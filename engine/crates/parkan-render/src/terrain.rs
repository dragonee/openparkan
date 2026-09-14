//! Drawing the ground built by `parkan_world::terrain`, and the water reflecting the world
//! above it (docs/03-terrain.md, "Water reflects").

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use parkan_world::terrain::{Layer, Terrain, WaterBox};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;
use crate::frame::{FrameUniform, linear};
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
    water: bool,
    bed: bool,
}

/// The largest side the reflection texture takes: the largest power of two not above the
/// screen's smaller side, held to this (`Terrain.dll:0x100423a0`).
pub const REFLECTION_MOST: u32 = 256;
/// The buffering camera's near plane (`0x10081a76`).
pub const REFLECTION_NEAR: f32 = 5.0;
/// Nothing lower than this below the water reaches the reflection (clip plane 0,
/// `0x10084426`).
pub const CLIP_BELOW_WATER: f32 = 0.5;
/// `CShade`'s water settings 31–35 at their defaults (`0x1005fa80`): the bump matrix's scale,
/// the bump map's largest value, how often it tiles across the box and the ms it drifts a
/// tile in.
pub const EMBM_COEFF: f32 = 0.01;
pub const EMBM_MAX_VAL: f32 = 64.0;
pub const EMBM_BUMP_TILE: f32 = 100.0;
pub const EMBM_BUMP_MOVE: f64 = 10_000.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterUniform {
    box_: [f32; 4],
    bump: [f32; 4],
}

/// The reflection's side on a `width` × `height` screen.
pub fn reflection_size(width: u32, height: u32) -> u32 {
    let side = width.min(height).max(1);
    (1u32 << (31 - side.leading_zeros())).min(REFLECTION_MOST)
}

/// The reflection camera's clip matrix for an eye above `water` (`0x10083e20`,
/// `REFLECTION_SHIFTED`): the eye mirrored in the water plane, looking straight up, its window
/// the water box. A point is seen where the line from the mirrored eye through it crosses the
/// water, so a point of the water plane at (x, y) lands on the texture's (1 − (y − y₀)/(y₁ − y₀),
/// 1 − (x − x₀)/(x₁ − x₀)), where the water face looks it up. Depth is reversed with no far
/// plane, as the scene's.
pub fn reflection_view_proj(eye: Vec3, water: &WaterBox) -> Option<Mat4> {
    let d = eye.z - water.level;
    let (width, height) = (water.max[0] - water.min[0], water.max[1] - water.min[1]);
    if d <= 0.0 || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let mirrored = 2.0 * water.level - eye.z;
    // clip x = w(1 − 2(ey − y₀)/Y) − 2d(y − ey)/Y and clip y = w(2(ex − x₀)/X − 1) + 2d(x − ex)/X,
    // with w the height over the mirrored eye.
    let a = 1.0 - 2.0 * (eye.y - water.min[1]) / height;
    let b = 2.0 * (eye.x - water.min[0]) / width - 1.0;
    let row_x = Vec4::new(0.0, -2.0 * d / height, a, -a * mirrored + 2.0 * d * eye.y / height);
    let row_y = Vec4::new(2.0 * d / width, 0.0, b, -b * mirrored - 2.0 * d * eye.x / width);
    let row_z = Vec4::new(0.0, 0.0, 0.0, REFLECTION_NEAR);
    let row_w = Vec4::new(0.0, 0.0, 1.0, -mirrored);
    Some(Mat4::from_cols_array_2d(&[
        [row_x.x, row_y.x, row_z.x, row_w.x],
        [row_x.y, row_y.y, row_z.y, row_w.y],
        [row_x.z, row_y.z, row_z.z, row_w.z],
        [row_x.w, row_y.w, row_z.w, row_w.w],
    ]))
}

/// The reflection's texture and depth at their side, and the water's bindings to it.
struct Reflection {
    side: u32,
    colour: wgpu::TextureView,
    depth: wgpu::TextureView,
    water: wgpu::BindGroup,
}

pub struct TerrainRenderer {
    pipeline: wgpu::RenderPipeline,
    frame: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    groups: Vec<DrawGroup>,
    format: wgpu::TextureFormat,
    water_box: Option<WaterBox>,
    /// The mirrored frame draws the faces that face the mirrored eye.
    reflection_pipeline: wgpu::RenderPipeline,
    reflection_frame: wgpu::Buffer,
    reflection_frame_bind_group: wgpu::BindGroup,
    water_pipeline: wgpu::RenderPipeline,
    water_layout: wgpu::BindGroupLayout,
    water_uniform: wgpu::Buffer,
    water_sampler: wgpu::Sampler,
    reflection: Option<Reflection>,
    /// Whether this frame's reflection was drawn, so the water shows it.
    reflecting: std::cell::Cell<bool>,
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
        let water_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water"),
            entries: &[
                uniform(0, wgpu::ShaderStages::FRAGMENT),
                texture(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let water_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water"),
            bind_group_layouts: &[Some(&frame_layout), Some(&group_layout), Some(&water_layout)],
            immediate_size: 0,
        });
        let make = |label: &str, layout: &wgpu::PipelineLayout, fragment: &str, cull: wgpu::Face| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
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
                    cull_mode: Some(cull),
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
                // STAND-IN: docs/03-terrain.md#not-established -- whether the water surface is
                // blended over the frame beneath it is not read; no layer's blend mode is read, and
                // the ground and the water draw opaque.
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make("terrain", &pipeline_layout, "fs_main", wgpu::Face::Back);
        // STAND-IN: docs/03-terrain.md#not-established -- whether a cull mode changes for the
        // mirrored reflection frame is not found; the faces that face the mirrored eye draw,
        // as a mirror shows them.
        let reflection_pipeline = make("terrain reflection", &pipeline_layout, "fs_main", wgpu::Face::Front);
        let water_pipeline = make("water", &water_pipeline_layout, "fs_water", wgpu::Face::Back);

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
        let reflection_frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reflection frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let reflection_frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reflection frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: reflection_frame.as_entire_binding() }],
        });
        let water_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("water"),
            size: std::mem::size_of::<WaterUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Phase 10's two stages both filter by point (`Ngi32.dll` record 16).
        let water_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("reflection"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
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

        let view_of = |layer: Option<&Layer>| textures.view(layer.and_then(|l| l.still.texture));
        let groups = terrain
            .groups
            .iter()
            .map(|g| {
                // A texture tinted by a display-space colour: both decoded, then multiplied.
                let tint = |l: Option<&Layer>, on: bool| {
                    let [r, gr, b] = linear(l.map_or([1.0; 3], |l| l.still.diffuse));
                    [r, gr, b, f32::from(u8::from(on))]
                };
                let uniform = LayersUniform {
                    tint1: tint(Some(&g.layer1), true),
                    tint2: tint(
                        g.layer2.as_ref(),
                        g.layer2.as_ref().is_some_and(|l| l.still.texture.is_some()),
                    ),
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
                DrawGroup { start: g.start, count: g.count, bind_group, water: g.water, bed: g.bed }
            })
            .collect();
        Self {
            pipeline,
            frame,
            frame_bind_group,
            vertices,
            indices,
            groups,
            format,
            water_box: terrain.water,
            reflection_pipeline,
            reflection_frame,
            reflection_frame_bind_group,
            water_pipeline,
            water_layout,
            water_uniform,
            water_sampler,
            reflection: None,
            reflecting: std::cell::Cell::new(false),
        }
    }

    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, lighting: &crate::frame::Lighting) {
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&FrameUniform::new(view_proj, lighting)));
    }

    /// This frame's reflection, for a `screen` of that size seen from `lighting`'s eye: its
    /// texture made at its side, its frame and the water's uniforms written. Returns the
    /// reflection camera's matrix and its lighting, clipped below the water, or none on a map
    /// without water or with the eye not above it.
    pub fn prepare_reflection(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lighting: &crate::frame::Lighting,
        (width, height): (u32, u32),
    ) -> Option<(Mat4, FrameUniform)> {
        self.reflecting.set(false);
        let water = self.water_box?;
        let view_proj = reflection_view_proj(lighting.eye, &water)?;
        let side = reflection_size(width, height);
        if self.reflection.as_ref().is_none_or(|r| r.side != side) {
            self.reflection = Some(self.make_reflection(device, side));
        }
        let eye = Vec3::new(lighting.eye.x, lighting.eye.y, 2.0 * water.level - lighting.eye.z);
        let uniform = FrameUniform::new(view_proj, &crate::frame::Lighting { eye, ..*lighting })
            .clipped_below(water.level - CLIP_BELOW_WATER);
        queue.write_buffer(&self.reflection_frame, 0, bytemuck::bytes_of(&uniform));
        let drift = (lighting.clock_ms.rem_euclid(EMBM_BUMP_MOVE) / EMBM_BUMP_MOVE) as f32;
        let w = WaterUniform {
            box_: [water.min[0], water.min[1], water.max[0], water.max[1]],
            bump: [drift, EMBM_BUMP_TILE, EMBM_COEFF, EMBM_MAX_VAL],
        };
        queue.write_buffer(&self.water_uniform, 0, bytemuck::bytes_of(&w));
        self.reflecting.set(true);
        Some((view_proj, uniform))
    }

    fn make_reflection(&self, device: &wgpu::Device, side: u32) -> Reflection {
        let size = wgpu::Extent3d { width: side, height: side, depth_or_array_layers: 1 };
        let texture = |label, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let colour = texture(
            "reflection",
            self.format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth = texture("reflection depth", DEPTH_FORMAT, wgpu::TextureUsages::RENDER_ATTACHMENT);
        let water = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water"),
            layout: &self.water_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.water_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&colour) },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.water_sampler),
                },
            ],
        });
        Reflection { side, colour, depth, water }
    }

    /// The reflection's colour and depth targets, once made.
    pub fn reflection_targets(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView)> {
        self.reflection.as_ref().map(|r| (&r.colour, &r.depth))
    }

    /// The ground as the camera above the water sees it: a liquid's bed is not drawn
    /// (`0x10043c43`), and the water shows this frame's reflection where there is one.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let reflection = self.reflection.as_ref().filter(|_| self.reflecting.get());
        for g in self.groups.iter().filter(|g| !g.bed) {
            match reflection {
                Some(r) if g.water => {
                    pass.set_pipeline(&self.water_pipeline);
                    pass.set_bind_group(2, &r.water, &[]);
                }
                _ => pass.set_pipeline(&self.pipeline),
            }
            pass.set_bind_group(1, &g.bind_group, &[]);
            pass.draw_indexed(g.start..g.start + g.count, 0, 0..1);
        }
    }

    /// The ground into the reflection: every face but the water's and the beds', through the
    /// reflection camera.
    pub fn draw_reflection(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.reflection_pipeline);
        pass.set_bind_group(0, &self.reflection_frame_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        for g in self.groups.iter().filter(|g| !g.bed && !g.water) {
            pass.set_bind_group(1, &g.bind_group, &[]);
            pass.draw_indexed(g.start..g.start + g.count, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TUT_1: WaterBox = WaterBox { min: [385.5, 255.8], max: [1480.5, 1531.7], level: -1.7255 };

    #[test]
    fn the_reflection_is_256_at_any_real_screen_and_a_power_of_two_below() {
        assert_eq!(
            [reflection_size(640, 480), reflection_size(1920, 1080), reflection_size(300, 200)],
            [256, 256, 128]
        );
    }

    #[test]
    fn a_point_of_the_water_lands_where_the_water_looks_it_up_and_a_point_above_where_its_line_crosses() {
        let eye = Vec3::new(700.0, 600.0, 30.0);
        let m = reflection_view_proj(eye, &TUT_1).unwrap();
        let uv = |p: Vec3| {
            let c = m * p.extend(1.0);
            let ndc = c.truncate() / c.w;
            ((ndc.x + 1.0) / 2.0, (1.0 - ndc.y) / 2.0)
        };
        let on_water = Vec3::new(900.0, 1000.0, TUT_1.level);
        let (u, v) = uv(on_water);
        let want = (1.0 - (1000.0 - 255.8) / (1531.7 - 255.8), 1.0 - (900.0 - 385.5) / (1480.5 - 385.5));
        assert!((u - want.0).abs() < 1e-4 && (v - want.1).abs() < 1e-4, "{u} {v} against {want:?}");
        // A treetop 20 above the water is seen through the water where the mirrored eye's
        // line to it crosses the plane.
        let top = Vec3::new(900.0, 1000.0, TUT_1.level + 20.0);
        let mirrored = Vec3::new(eye.x, eye.y, 2.0 * TUT_1.level - eye.z);
        let s = (TUT_1.level - mirrored.z) / (top.z - mirrored.z);
        let crossing = mirrored + (top - mirrored) * s;
        let (u2, v2) = uv(top);
        let (cu, cv) = uv(crossing.with_z(TUT_1.level));
        assert!((u2 - cu).abs() < 1e-4 && (v2 - cv).abs() < 1e-4);
        let depth = |p: Vec3| {
            let c = m * p.extend(1.0);
            c.z / c.w
        };
        assert!(depth(top) > depth(top + Vec3::Z * 50.0), "nearer is greater");
        assert!(
            reflection_view_proj(Vec3::new(700.0, 600.0, -5.0), &TUT_1).is_none(),
            "no reflection from under it"
        );
    }
}
