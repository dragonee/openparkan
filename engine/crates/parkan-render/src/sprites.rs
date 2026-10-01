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
    tint: [f32; 4],
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
    /// Each corner's (u, v) across the look's cell, where the quad is a piece of a larger
    /// shape; `None` spans the cell.
    pub uv: Option<[[f32; 2]; 4]>,
    /// How far its sprite stands from the eye: the key the effects' layer files it by, one
    /// for every piece of a larger shape.
    pub depth: f32,
    /// The group-1 layer it is filed in: [`EFFECTS_LAYER`] for an effect's sprite,
    /// [`LIGHTS_LAYER`] for a light drawn on a surface, which goes first.
    pub layer: u8,
    /// Its colour against its look's: rgb times the look's ambient, or with w 1 the self-light
    /// of a device material, the scene colour plus rgb (docs/07, "How a material reaches the
    /// device"), or with w 2 rgb alone, a vertex colour of the quad's own that no fog reaches:
    /// the rain's and the snow's (docs/10, "The weather"); w 3 is w 0 without the fog.
    pub tint: [f32; 4],
}

/// The layers of group 1 the drawn quads are filed in (docs/10, "The dome"): the weather's
/// goes last, over the effects, and a layer of that kind draws its items in the order they
/// were filed.
pub const LIGHTS_LAYER: u8 = 3;
pub const EFFECTS_LAYER: u8 = 6;
pub const WEATHER_LAYER: u8 = 8;
/// A sprite's tint: its look's colour as it is.
pub const PLAIN_TINT: [f32; 4] = [1.0, 1.0, 1.0, 0.0];
/// The same, its fog factor held at 1: an effect whose header carries `0x2000` (docs/11, "How
/// an effect sprite is coloured").
pub const UNFOGGED_TINT: [f32; 4] = [1.0, 1.0, 1.0, 3.0];

/// The order an effect's quads are drawn in: far to near across every look and blend mode.
///
/// Every effect sprite is filed in group 1, layer 6 (`Terrain.dll:0x10042202`–`0x1004221d`),
/// a type-3 `CCamDistSortLayerVB` (docs/10, "The dome"), whose add slot (`0x1003e090`) keeps
/// its list sorted by the item's distance from the camera (item slot 5), nearer items further
/// down, and whose render (`0x1003e1d0`) walks it from the top. So a far sprite is drawn
/// before a near one whatever either blends with; an item whose fade is 0 is not filed. The
/// sort is stable, so the pieces of one shape keep their own order.
///
/// The layers draw in their order (`0x1003e1d0`'s caller walks them one by one), so a light's
/// item, filed in layer 3, goes before every sprite.
pub fn draw_order(quads: &[Quad]) -> Vec<&Quad> {
    let mut sorted: Vec<&Quad> = quads.iter().filter(|q| q.alpha != 0.0).collect();
    sorted.sort_by(|a, b| a.layer.cmp(&b.layer).then(b.depth.total_cmp(&a.depth)));
    sorted
}

/// The corners of a sprite drawn with its own matrix, and their (u, v): the shade's unit quad
/// (`Terrain.dll:0x10027a30`–`0x10027b88`), (−½, ½), (−½, −½), (½, −½), (½, ½) in the sprite's
/// xy plane with (0, 0.99), (0, 0), (0.99, 0), (0.99, 0.99), carried out by `matrix`, where the
/// sprite's own x, y and z end up about `centre`. So u runs with the sprite's x and v with its
/// y, and a mode-1 streak, whose x is its direction, takes u along its length.
pub fn turned(centre: Vec3, matrix: [Vec3; 3]) -> ([Vec3; 4], [[f32; 2]; 4]) {
    let at = |x: f32, y: f32| centre + matrix[0] * x + matrix[1] * y;
    (
        [at(-0.5, 0.5), at(-0.5, -0.5), at(0.5, -0.5), at(0.5, 0.5)],
        [[0.0, 0.99], [0.0, 0.0], [0.99, 0.0], [0.99, 0.99]],
    )
}

/// The pieces a type-9 sprite's hemisphere is drawn as, each a quad with its corners' (u, v),
/// carried into the world by the sprite's `matrix` about `centre`.
///
/// The shade builds the mesh once, in the sprite's own space (`Terrain.dll:0x10027a20`): a
/// vertex at polar angle θ from the pole and φ around is (sin θ sin φ, sin θ cos φ, cos θ),
/// θ running 0 to **π/2** in `rings` steps and φ the whole turn in `segments` -- a half sphere
/// of radius 1, its pole at (0, 0, 1) and its rim the unit circle of the xy plane, and nothing
/// below it. Each segment is a triangle from the pole to the first ring (`0x100273b0`) and a
/// quad for each ring after it (`0x100276a0`); a triangle is given here as a quad with its
/// last corner repeated.
///
/// `projected` lays the texture over the dome as seen down its pole, (u, v) = ((x + 1) / 2,
/// (y + 1) / 2) (`0x100275ff`, `0x10027959`); otherwise every piece takes the whole texture --
/// a quad's own (u, v) on a quad, (0.5, 0.99), (0, 0), (0.99, 0) on a triangle
/// (`0x10028bbe`, `0x10028ce2`, the streams set at `0x10027af6`, `0x10027d4d`).
pub fn dome(
    centre: Vec3,
    matrix: [Vec3; 3],
    segments: u8,
    rings: u8,
    projected: bool,
) -> Vec<([Vec3; 4], [[f32; 2]; 4])> {
    let (segments, rings) = (usize::from(segments.max(3)), usize::from(rings.max(1)));
    let own = |ring: usize, segment: usize| {
        let polar = ring as f32 / rings as f32 * std::f32::consts::FRAC_PI_2;
        let around = segment as f32 / segments as f32 * std::f32::consts::TAU;
        Vec3::new(polar.sin() * around.sin(), polar.sin() * around.cos(), polar.cos())
    };
    let world = |v: Vec3| centre + matrix[0] * v.x + matrix[1] * v.y + matrix[2] * v.z;
    let over = |v: Vec3| [(v.x + 1.0) * 0.5, (v.y + 1.0) * 0.5];
    let mut out = Vec::with_capacity(segments * rings);
    for segment in 0..segments {
        let cap = [own(0, segment), own(1, segment + 1), own(1, segment)];
        let uv = if projected { cap.map(over) } else { [[0.5, 0.99], [0.0, 0.0], [0.99, 0.0]] };
        let cap = cap.map(world);
        out.push(([cap[0], cap[1], cap[2], cap[2]], [uv[0], uv[1], uv[2], uv[2]]));
        for ring in 1..rings {
            let piece = [
                own(ring, segment + 1),
                own(ring + 1, segment + 1),
                own(ring + 1, segment),
                own(ring, segment),
            ];
            let uv = if projected {
                piece.map(over)
            } else {
                [[0.0, 0.99], [0.0, 0.0], [0.99, 0.0], [0.99, 0.99]]
            };
            out.push((piece.map(world), uv));
        }
    }
    out
}

/// The corners of a sprite seen from `eye`: `width` across the eye and `height` up it,
/// or, when `along` is not zero, `along` long and `width` wide, turned about its
/// length toward the eye. Drawn in order, the texture's u runs across a stretched quad and
/// v along it; [`lengthwise`] turns them round.
pub fn billboard(centre: Vec3, along: Vec3, width: f32, height: f32, eye: Vec3) -> [Vec3; 4] {
    let view = (centre - eye).normalize_or(Vec3::Y);
    let (half_long, side) = if along.length_squared() > 1e-12 {
        let side = along.cross(view).normalize_or(Vec3::X) * (width / 2.0);
        (along / 2.0, side)
    } else {
        let right = view.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(view);
        (up * (height / 2.0), right * (width / 2.0))
    };
    [
        centre - half_long - side,
        centre - half_long + side,
        centre + half_long + side,
        centre + half_long - side,
    ]
}

/// The corners of a sprite drawn **through its frame**: the camera-facing unit square built
/// in the frame's own space and carried back out through it, so the frame's axis lengths are
/// the quad's.
///
/// The frame a sprite hangs on is a matrix -- three control points' direction vectors as its
/// rows and their centroid as its fourth (`Control.dll:0x10002d8d`) -- and the draw combines
/// it, scaled per axis by the emitter's size channel (`Effect.dll:0x1000d0c0`), with the basis
/// mode 0 builds from the eye (`0x100093f4`). The eye reaches the draw already in the frame's
/// own space, as the sprite's position does, so the square faces the camera there and comes
/// out stretched by the frame. A burst's and a stream's particles are drawn so, where their
/// frame is three control points'; a type-3, 4 or 9 sprite brings its own matrix, worked by
/// its mode ([`turned`], docs/11, "A sprite's mode").
///
/// Up is the frame's third axis, as it is the model's z in the draw (`0x1000948d` builds the
/// side vector as (−d.y, d.x, 0) of the local view).
pub fn framed(centre: Vec3, axes: [Vec3; 3], eye: Vec3) -> [Vec3; 4] {
    let basis = glam::Mat3::from_cols(axes[0], axes[1], axes[2]);
    let inverse = basis.inverse();
    if !inverse.is_finite() {
        return billboard(centre, Vec3::ZERO, axes[1].length(), axes[2].length(), eye);
    }
    let view = (inverse * (centre - eye)).normalize_or(Vec3::Y);
    let side = view.cross(Vec3::Z).normalize_or(Vec3::X);
    let (half_long, half_side) = (basis * (side.cross(view) / 2.0), basis * (side / 2.0));
    [
        centre - half_long - half_side,
        centre - half_long + half_side,
        centre + half_long + half_side,
        centre + half_long - half_side,
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
                            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32, 3 => Float32x4],
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

    /// Upload this frame's quads, far to near ([`draw_order`]), a run of one look drawn at once.
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
        // The frame's fog colour is the stored one; this shader encodes what it is handed.
        let [r, g, b] = crate::frame::linear(lighting.fog_colour);
        camera[20..24].copy_from_slice(&[r, g, b, 1.0]);
        camera[24..28].copy_from_slice(&[lighting.fog_start, lighting.fog_end, 0.0, 0.0]);
        let [r, g, b] = lighting.scene_colour;
        camera[28..32].copy_from_slice(&[r, g, b, 1.0]);
        queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera));
        let sorted: Vec<&Quad> =
            draw_order(quads).into_iter().filter(|q| q.look < self.looks.len()).collect();
        let mut vertices = Vec::with_capacity(sorted.len() * 6);
        self.draws.clear();
        for q in sorted {
            let [u0, v0, du, dv] = self.looks[q.look].2;
            let uv =
                q.uv.unwrap_or([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
                    .map(|[u, v]| [u0 + u * du, v0 + v * dv]);
            let v = |i: usize| GpuVertex {
                position: q.corners[i].to_array(),
                uv: uv[i],
                alpha: q.alpha,
                tint: q.tint,
            };
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

    /// A near additive plume is drawn after a far alpha-blended smoke, so it lies over it,
    /// and the pieces of a dome keep their order.
    #[test]
    fn effect_quads_are_drawn_far_to_near_whatever_they_blend_with() {
        let quad = |look: usize, depth: f32, alpha: f32| Quad {
            look,
            corners: [Vec3::ZERO; 4],
            alpha,
            overlay: false,
            uv: None,
            depth,
            layer: EFFECTS_LAYER,
            tint: PLAIN_TINT,
        };
        let quads = [
            quad(0, 50.0, 1.0),  // the lode's plume, additive, near
            quad(1, 300.0, 1.0), // a chimney's smoke, see-through, far
            quad(2, 120.0, 0.5), // a dome's first piece
            quad(2, 120.0, 0.5), // and its second
            quad(1, 10.0, 0.0),  // faded out: not filed
        ];
        let order: Vec<(usize, f32)> = draw_order(&quads).iter().map(|q| (q.look, q.depth)).collect();
        assert_eq!(order, [(1, 300.0), (2, 120.0), (2, 120.0), (0, 50.0)]);
        let pieces: Vec<*const Quad> =
            draw_order(&quads).into_iter().filter(|q| q.look == 2).map(|q| q as *const Quad).collect();
        assert_eq!(pieces, [&quads[2] as *const Quad, &quads[3] as *const Quad]);
    }

    /// The shade's hemisphere (`Terrain.dll:0x10027a20`): radius 1, its pole on the sprite's
    /// z and its rim the unit circle of its xy plane, a triangle and `rings − 1` quads to a
    /// segment.
    #[test]
    fn a_dome_is_a_unit_half_sphere_with_its_pole_on_the_sprites_z() {
        let own = [Vec3::X, Vec3::Y, Vec3::Z];
        let pieces = dome(Vec3::ZERO, own, 8, 3, true);
        assert_eq!(pieces.len(), 8 * 3);
        let corners: Vec<Vec3> = pieces.iter().flat_map(|(c, _)| *c).collect();
        assert!(corners.iter().all(|c| (c.length() - 1.0).abs() < 1e-5), "every vertex a unit out");
        assert!(corners.iter().all(|c| c.z > -1e-6), "nothing below the rim: a half, not a whole");
        assert!(corners.iter().any(|c| (*c - Vec3::Z).length() < 1e-6), "the pole");
        assert!(corners.iter().any(|c| c.z.abs() < 1e-6), "the rim");
        // Each segment opens with its triangle from the pole, its last corner repeated.
        let (cap, uv) = pieces[0];
        assert!((cap[0] - Vec3::Z).length() < 1e-6 && cap[2] == cap[3]);
        // Laid over the dome as seen down the pole: the pole at the texture's middle, the rim
        // on its inscribed circle.
        assert!((uv[0][0] - 0.5).abs() < 1e-6 && (uv[0][1] - 0.5).abs() < 1e-6);
        for (c, uv) in &pieces {
            for (v, t) in c.iter().zip(uv) {
                assert!((t[0] - (v.x + 1.0) * 0.5).abs() < 1e-6 && (t[1] - (v.y + 1.0) * 0.5).abs() < 1e-6);
            }
        }
        // Otherwise every facet takes the whole texture.
        let tiled = dome(Vec3::ZERO, own, 16, 6, false);
        assert_eq!(tiled.len(), 16 * 6);
        assert_eq!(tiled[0].1[..3], [[0.5, 0.99], [0.0, 0.0], [0.99, 0.0]]);
        assert_eq!(tiled[1].1, [[0.0, 0.99], [0.0, 0.0], [0.99, 0.0], [0.99, 0.99]]);
        // The sprite's matrix carries it: a pole 2 down and a rim 3 by 4 about a centre.
        let centre = Vec3::new(5.0, 6.0, 7.0);
        let hung = dome(centre, [Vec3::X * 3.0, Vec3::Y * 4.0, Vec3::NEG_Z * 2.0], 8, 3, true);
        let lowest = hung.iter().flat_map(|(c, _)| *c).map(|c| c.z).fold(f32::MAX, f32::min);
        let highest = hung.iter().flat_map(|(c, _)| *c).map(|c| c.z).fold(f32::MIN, f32::max);
        assert!((lowest - 5.0).abs() < 1e-5 && (highest - 7.0).abs() < 1e-5, "{lowest}..{highest}");
        let widest = hung.iter().flat_map(|(c, _)| *c).map(|c| (c.x - 5.0).abs()).fold(0.0, f32::max);
        assert!((widest - 3.0).abs() < 1e-5);
    }

    /// A sprite's quad is the unit square of its own xy plane, u with x and v with y.
    #[test]
    fn a_sprites_quad_is_the_unit_square_of_its_own_xy_plane() {
        let (c, uv) = turned(Vec3::new(1.0, 1.0, 1.0), [Vec3::Y * 8.0, Vec3::Z * 2.0, Vec3::X]);
        assert_eq!(c[0], Vec3::new(1.0, -3.0, 2.0));
        assert_eq!(c[2], Vec3::new(1.0, 5.0, 0.0));
        // u runs along the sprite's x, here world y, and v along its y, here world z.
        assert_eq!((uv[1], uv[2], uv[0]), ([0.0, 0.0], [0.99, 0.0], [0.0, 0.99]));
        assert!(
            (c[2] - c[1] - Vec3::Y * 8.0).length() < 1e-6 && (c[0] - c[1] - Vec3::Z * 2.0).length() < 1e-6
        );
    }

    #[test]
    fn a_square_faces_the_eye_and_a_streak_turns_about_its_length() {
        let c = billboard(Vec3::ZERO, Vec3::ZERO, 2.0, 2.0, Vec3::new(0.0, -10.0, 0.0));
        for p in c {
            assert!(p.y.abs() < 1e-6 && (p.x.abs() - 1.0).abs() < 1e-6 && (p.z.abs() - 1.0).abs() < 1e-6);
        }
        let s = billboard(Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), 1.0, 1.0, Vec3::new(0.0, 0.0, 10.0));
        assert!((s[2] - s[1]).length() - 4.0 < 1e-5);
        assert!(s.iter().all(|p| p.z.abs() < 1e-6), "a streak seen from above lies flat");
        // Lengthwise, u (corner 0 to 1) runs along the streak's 4 and v (1 to 2) across its 1.
        let l = lengthwise(s);
        assert!(((l[1] - l[0]).length() - 4.0).abs() < 1e-5 && ((l[2] - l[1]).length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_quad_in_a_frame_is_as_long_as_the_axis_it_is_seen_across_and_as_short_as_the_one_seen_along() {
        // `fr_e_brige`'s ray: 150 m along the span (model +y here), 1.932 across, 0.414 high.
        let axes = [Vec3::new(0.0, 150.0, 0.0), Vec3::new(1.932, 0.0, 0.0), Vec3::new(0.0, 0.0, -0.414)];
        let side = framed(Vec3::ZERO, axes, Vec3::new(60.0, 0.0, 0.0));
        let (across, up) = ((side[1] - side[0]).length(), (side[2] - side[1]).length());
        assert!((across - 150.0).abs() < 1e-3, "seen across the span it runs its whole length: {across}");
        assert!((up - 0.414).abs() < 1e-3, "and stands its height: {up}");
        let along = framed(Vec3::ZERO, axes, Vec3::new(0.0, -300.0, 0.0));
        let (a, b) = ((along[1] - along[0]).length(), (along[2] - along[1]).length());
        assert!((a - 1.932).abs() < 1e-3 && (b - 0.414).abs() < 1e-3, "down the span: {a} by {b}");
        // Three directions in a plane give no basis to turn through, and the quad falls back
        // to one across the eye.
        let flat = [Vec3::X, Vec3::X * 2.0, Vec3::X * 3.0];
        assert_eq!(framed(Vec3::ZERO, flat, Vec3::new(0.0, -10.0, 0.0)).len(), 4);
    }
}
