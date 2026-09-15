//! Drawing effect sprites: quads that face the camera, or stretch along a
//! direction and turn about it to face the camera, textured by their material and
//! blended by its mode.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;
use crate::frame::{Lighting, fog_override};
use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    uv: [f32; 2],
    alpha: f32,
}

/// A quad ready to draw: which look, its four corners, and its alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub look: usize,
    pub corners: [Vec3; 4],
    pub alpha: f32,
    /// Drawn over the scene, with the depth test off (effect draw flag 1,
    /// `docs/11-effects.md`, "Bit 8 and the tested point").
    pub overlay: bool,
}

/// The corners of a sprite seen from `eye`: a square of side `width` facing the eye,
/// or, when `along` is not zero, `along` long and `width` wide, turned about its
/// length toward the eye. Drawn in order, the texture's u runs across a stretched quad and
/// v along it; [`lengthwise`] turns them round.
pub fn billboard(centre: Vec3, along: Vec3, width: f32, eye: Vec3) -> [Vec3; 4] {
    let view = (centre - eye).normalize_or(Vec3::Y);
    let (half_long, side) = if along.length_squared() > 1e-12 {
        let side = along.cross(view).normalize_or(Vec3::X) * (width / 2.0);
        (along / 2.0, side)
    } else {
        let right = view.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(view);
        (up * (width / 2.0), right * (width / 2.0))
    };
    [
        centre - half_long - side,
        centre - half_long + side,
        centre + half_long + side,
        centre + half_long - side,
    ]
}

/// A stretched quad's corners reordered so the texture's u runs along its length and v
/// across it, as a bolt's sprites take theirs (`docs/11-effects.md`, "Bolts, streams and fades").
pub fn lengthwise(corners: [Vec3; 4]) -> [Vec3; 4] {
    [corners[0], corners[3], corners[2], corners[1]]
}

/// A material's look as sprites use it: its texture and blend mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteLook {
    pub texture: Option<usize>,
    pub blend_mode: u8,
    /// The entry's ambient colour, 0..1 in display space as the file gives it.
    pub ambient: [f32; 3],
    /// The texture's cell a quad spans: `(u0, v0, du, dv)`.
    pub cell: [f32; 4],
}

fn blend(mode: u8) -> wgpu::BlendState {
    use wgpu::BlendFactor as F;
    let c = |src, dst| wgpu::BlendComponent {
        src_factor: src,
        dst_factor: dst,
        operation: wgpu::BlendOperation::Add,
    };
    let (src, dst) = match mode {
        2 => (F::SrcAlpha, F::One),
        3 => (F::Zero, F::Src),
        5 => (F::Dst, F::Src),
        // STAND-IN: docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory -- how
        // a sprite whose material says opaque (0) or SRCALPHA/ZERO (1) blends is not read: it
        // is drawn alpha-blended, so its fade shows.
        _ => (F::SrcAlpha, F::OneMinusSrcAlpha),
    };
    wgpu::BlendState { color: c(src, dst), alpha: c(F::One, F::OneMinusSrcAlpha) }
}

pub struct SpriteRenderer {
    camera: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    pipelines: Vec<wgpu::RenderPipeline>,
    looks: Vec<(u8, wgpu::BindGroup, [f32; 4])>,
    vertices: Option<wgpu::Buffer>,
    capacity: usize,
    /// This frame's draws: pipeline, look, first vertex, vertex count.
    draws: Vec<(usize, usize, u32, u32)>,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        looks: &[SpriteLook],
        bank: &GpuTextures,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let skin_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite skin"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite camera"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[Some(&camera_layout), Some(&skin_layout)],
            immediate_size: 0,
        });
        // Every blend mode twice: depth-tested, then over the scene with the depth test off.
        // Neither writes depth; the sprites bit 8 draws carry `ZWRITEENABLE` off too
        // (docs/11-effects.md, "Bit 8 and the tested point").
        let pipelines = (0..6u8)
            .flat_map(|mode| [(mode, false), (mode, true)])
            .map(|(mode, overlay)| {
                let depth_compare = if overlay { wgpu::CompareFunction::Always } else { wgpu::CompareFunction::Greater };
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("sprite"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[Some(wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<GpuVertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32],
                        })],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(depth_compare),
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
                            blend: Some(blend(mode)),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .collect();
        let looks = looks
            .iter()
            .map(|l| {
                let view = l.texture.and_then(|t| bank.views.get(t)).unwrap_or(&bank.white);
                let [r, g, b] = l.ambient;
                let skin: [f32; 8] = {
                    let [fr, fg, fb, fw] = fog_override(l.blend_mode);
                    [fr, fg, fb, fw, r, g, b, 1.0]
                };
                let toward = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("sprite skin"),
                    contents: bytemuck::cast_slice(&skin),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sprite skin"),
                    layout: &skin_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&bank.sampler),
                        },
                        wgpu::BindGroupEntry { binding: 2, resource: toward.as_entire_binding() },
                    ],
                });
                (l.blend_mode.min(5), group, l.cell)
            })
            .collect();
        Self { camera, camera_group, pipelines, looks, vertices: None, capacity: 0, draws: Vec::new() }
    }

    /// Upload this frame's quads, grouped by look.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        lighting: &Lighting,
        quads: &[Quad],
    ) {
        let mut camera = [0.0_f32; 32];
        camera[..16].copy_from_slice(&view_proj.to_cols_array());
        camera[16..20].copy_from_slice(&[lighting.eye.x, lighting.eye.y, lighting.eye.z, 1.0]);
        let [r, g, b] = lighting.fog_colour;
        camera[20..24].copy_from_slice(&[r, g, b, 1.0]);
        camera[24..28].copy_from_slice(&[lighting.fog_start, lighting.fog_end, 0.0, 0.0]);
        let [r, g, b] = lighting.scene_colour;
        camera[28..32].copy_from_slice(&[r, g, b, 1.0]);
        queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera));
        let mut sorted: Vec<&Quad> = quads.iter().filter(|q| q.look < self.looks.len()).collect();
        sorted.sort_by_key(|q| (q.overlay, self.looks[q.look].0, q.look));
        let mut vertices = Vec::with_capacity(sorted.len() * 6);
        self.draws.clear();
        for q in sorted {
            let [u0, v0, du, dv] = self.looks[q.look].2;
            let uv =
                [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]].map(|[u, v]| [u0 + u * du, v0 + v * dv]);
            let v = |i: usize| GpuVertex { position: q.corners[i].to_array(), uv: uv[i], alpha: q.alpha };
            let start = vertices.len() as u32;
            vertices.extend([v(0), v(1), v(2), v(0), v(2), v(3)]);
            let pipeline = usize::from(self.looks[q.look].0) * 2 + usize::from(q.overlay);
            match self.draws.last_mut() {
                Some(d) if d.0 == pipeline && d.1 == q.look => d.3 += 6,
                _ => self.draws.push((pipeline, q.look, start, 6)),
            }
        }
        if vertices.is_empty() {
            return;
        }
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprites"),
                size: (self.capacity * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        if let Some(buffer) = &self.vertices {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&vertices));
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(buffer) = &self.vertices else { return };
        if self.draws.is_empty() {
            return;
        }
        pass.set_bind_group(0, &self.camera_group, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for &(pipeline, look, start, count) in &self.draws {
            pass.set_pipeline(&self.pipelines[pipeline]);
            pass.set_bind_group(1, &self.looks[look].1, &[]);
            pass.draw(start..start + count, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_faces_the_eye_and_a_streak_turns_about_its_length() {
        let c = billboard(Vec3::ZERO, Vec3::ZERO, 2.0, Vec3::new(0.0, -10.0, 0.0));
        for p in c {
            assert!(p.y.abs() < 1e-6 && (p.x.abs() - 1.0).abs() < 1e-6 && (p.z.abs() - 1.0).abs() < 1e-6);
        }
        let s = billboard(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 1.0, Vec3::new(0.0, 0.0, 10.0));
        assert!((s[2] - s[1]).length() - 4.0 < 1e-5);
        assert!(s.iter().all(|p| p.z.abs() < 1e-6), "a streak seen from above lies flat");
        // Lengthwise, u (corner 0 to 1) runs along the streak's 4 and v (1 to 2) across its 1.
        let l = lengthwise(s);
        assert!(((l[1] - l[0]).length() - 4.0).abs() < 1e-5 && ((l[2] - l[1]).length() - 1.0).abs() < 1e-5);
    }
}
