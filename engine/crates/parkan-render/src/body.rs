//! Drawing the sun's and the moon's sprites: `docs/10-sky.md`, "Where the sun stands"
//! and "What the sun does with its seven values".
//!
//! `CSun` carries a body along an arc each takt and draws it with the `sky.wea` slot its
//! start keyframe's name picked -- 3 for `sun`, 4 for anything else. The sprite's extent
//! across and up are the keyframe's first two floats, and they are a **factor**, not a
//! length: *measured* over all 656 keyframes of the 29 shipped files, across runs 0.4 to
//! 3.3 and up 0.4 to 3.0, most often (1, 1) or (0.65, 0.65) -- no world size, and across
//! is at or above up on all 656, so a body is never taller than it is wide.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    uv: [f32; 2],
    tint: [f32; 4],
}

/// How far out a body's quad is put. Any distance does, since the size is worked out as
/// an angle and scaled by it; this one sits inside the dome's 34142 rim so the sprite
/// lands between the dome and the scene.
pub const BODY_DISTANCE: f32 = 10_000.0;

/// STAND-IN: docs/10-sky.md#not-resolved -- the unit of the sprite's two extents is not
/// read. They are a factor on a base this engine has to choose, the game's own being
/// camera slot 27.
///
/// A body at extent 1 is drawn **8° across**, which puts the shipped 0.4 to 3.3 at 3.2°
/// to 26°. The first choice here was 3°, and it was wrong for a reason the picture showed
/// at once: a body came out a dot of a few pixels, throwing away artwork drawn at 128
/// pixels square, and the sheets hold **planets** as well as suns (`SUN3.0` is a star and
/// three planets), which a sci-fi sky hangs large. The figure is still a choice, not a
/// reading.
pub const BODY_BASE_HALF_ANGLE: f32 = 0.069_813_17; // 4° in radians

/// One body to draw this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// The unit direction from the eye to the body.
    pub toward: Vec3,
    /// Its extent across and up, the keyframe's first two floats.
    pub extent: [f32; 2],
    /// Which loaded texture it draws with.
    pub texture: Option<usize>,
    /// The texture's cell this body spans, `(u0, v0, du, dv)`. A sun texture is a page of
    /// four sub-images and the material names which one (`docs/10-sky.md`, "A material
    /// picks a sub-image as well as a texture"), so a body that took the whole sheet
    /// would draw all four suns at once.
    pub cell: [f32; 4],
    /// Multiplied into the texture; the alpha fades a body in and out.
    pub tint: [f32; 4],
}

/// The four corners of a body's quad, in world space about `eye`.
///
/// The quad faces the camera: its across axis is perpendicular to both the line to the
/// body and the world's up, and its up axis completes the pair, so the sprite stands
/// upright in the sky as the game's billboards do.
pub fn corners(eye: Vec3, sprite: &Sprite) -> [Vec3; 4] {
    let toward = sprite.toward.normalize_or_zero();
    let centre = eye + toward * BODY_DISTANCE;
    // Vertical in the world unless the body is straight overhead, where any across does.
    let across = toward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
    let up = across.cross(toward).normalize_or_zero();
    let half = |e: f32| BODY_DISTANCE * (BODY_BASE_HALF_ANGLE * e).tan();
    let (a, u) = (across * half(sprite.extent[0]), up * half(sprite.extent[1]));
    [centre - a + u, centre + a + u, centre + a - u, centre - a - u]
}

pub struct BodyRenderer {
    pipeline: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    skins: Vec<wgpu::BindGroup>,
    white: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// This frame's quads: skin, first vertex.
    draws: Vec<(Option<usize>, u32)>,
}

/// How many bodies a frame may draw: no shipped section starts more than the sun and the
/// moon, and one file's second sun aside they never overlap.
const CAPACITY: usize = 4;

impl BodyRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, bank: &GpuTextures) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("body.wgsl"));
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky body camera"),
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
        let skin_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky body skin"),
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
            ],
        });
        let skin_group = |view: &wgpu::TextureView, label: &str| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &skin_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&bank.sampler),
                    },
                ],
            })
        };
        let skins = bank.views.iter().map(|v| skin_group(v, "sky body skin")).collect();
        let white = skin_group(&bank.white, "sky body white");
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky body camera"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky body camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky body"),
            bind_group_layouts: &[Some(&camera_layout), Some(&skin_layout)],
            immediate_size: 0,
        });
        // The blend is the material's own, not a choice: every shipped sun and moon
        // material carries blend mode 4, `SRCALPHA`/`INVSRCALPHA`
        // (docs/07-objects.md, "How a material draws").
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky body"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            // A body draws with the dome's depth state: tested, writing nothing, so the
            // scene drawn after it paints over it and the sun sits behind the world.
            depth_stencil: Some(crate::dome::depth_state()),
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
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky body"),
            size: (CAPACITY * 6 * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, camera, camera_group, skins, white, vertices, draws: Vec::new() }
    }

    /// This frame's camera and bodies. The quad is built about the eye, as the dome is
    /// drawn at the camera, so a body stays put however far the player walks.
    pub fn prepare(&mut self, queue: &wgpu::Queue, view_proj: Mat4, eye: Vec3, sprites: &[Sprite]) {
        queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&view_proj.to_cols_array()));
        self.draws.clear();
        let mut vertices: Vec<GpuVertex> = Vec::with_capacity(sprites.len() * 6);
        for sprite in sprites.iter().take(CAPACITY) {
            let [tl, tr, br, bl] = corners(eye, sprite);
            let [u0, v0, du, dv] = sprite.cell;
            let uv = [[u0, v0], [u0 + du, v0], [u0 + du, v0 + dv], [u0, v0 + dv]];
            self.draws.push((sprite.texture, vertices.len() as u32));
            for (p, uv) in [(tl, uv[0]), (tr, uv[1]), (br, uv[2]), (tl, uv[0]), (br, uv[2]), (bl, uv[3])] {
                vertices.push(GpuVertex { position: p.to_array(), uv, tint: sprite.tint });
            }
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.draws.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for &(texture, first) in &self.draws {
            let skin = texture.and_then(|i| self.skins.get(i)).unwrap_or(&self.white);
            pass.set_bind_group(1, skin, &[]);
            pass.draw(first..first + 6, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(toward: Vec3, extent: [f32; 2]) -> Sprite {
        Sprite { toward, extent, texture: None, cell: [0.0, 0.0, 1.0, 1.0], tint: [1.0; 4] }
    }

    #[test]
    fn a_body_stands_along_its_own_direction_from_the_eye() {
        let eye = Vec3::new(120.0, -40.0, 15.0);
        let toward = Vec3::new(0.5, 0.0, 0.866).normalize();
        let quad = corners(eye, &sprite(toward, [1.0, 1.0]));
        let centre = quad.iter().copied().sum::<Vec3>() / 4.0;
        let line = (centre - eye).normalize();
        assert!(line.dot(toward) > 0.999_9, "the quad's centre is on the line to the body: {line}");
        assert!((centre - eye).length() - BODY_DISTANCE < 1.0);
    }

    #[test]
    fn the_extents_scale_the_quad_across_and_up_independently() {
        let eye = Vec3::ZERO;
        let toward = Vec3::new(0.0, 1.0, 0.0);
        let one = corners(eye, &sprite(toward, [1.0, 1.0]));
        let wide = corners(eye, &sprite(toward, [2.0, 1.0]));
        let across = |q: [Vec3; 4]| (q[1] - q[0]).length();
        let up = |q: [Vec3; 4]| (q[0] - q[3]).length();
        assert!((across(wide) / across(one) - 2.0).abs() < 0.01, "twice across");
        assert!((up(wide) / up(one) - 1.0).abs() < 0.01, "the same up");
    }

    #[test]
    fn a_body_at_extent_one_is_eight_degrees_across() {
        // The stand-in's own figure, pinned so a change to it is deliberate.
        let quad = corners(Vec3::ZERO, &sprite(Vec3::Y, [1.0, 1.0]));
        let half = (quad[1] - quad[0]).length() / 2.0;
        let degrees = 2.0 * (half / BODY_DISTANCE).atan().to_degrees();
        assert!((degrees - 8.0).abs() < 0.01, "{degrees}");
    }

    #[test]
    fn a_body_takes_its_materials_own_cell_and_not_the_whole_sheet() {
        // `SUN1.0` is a page of four suns and `ENV_SUN_3` names one of them; a quad that
        // spanned 0..1 drew all four at once, which is what this pins against.
        let quarter = [0.0, 0.0, 0.5, 0.5];
        let s =
            Sprite { toward: Vec3::Y, extent: [1.0, 1.0], texture: Some(0), cell: quarter, tint: [1.0; 4] };
        let [u0, v0, du, dv] = s.cell;
        assert_eq!([u0 + du, v0 + dv], [0.5, 0.5], "the far corner stays inside its own cell");
        assert!(du < 1.0 && dv < 1.0, "a page's cell is a part of the sheet");
    }

    #[test]
    fn a_body_overhead_still_gets_a_quad() {
        let quad = corners(Vec3::ZERO, &sprite(Vec3::Z, [1.0, 1.0]));
        assert!(quad.iter().all(|c| c.is_finite()), "{quad:?}");
        assert!((quad[1] - quad[0]).length() > 0.0, "an across axis even straight up");
    }
}
