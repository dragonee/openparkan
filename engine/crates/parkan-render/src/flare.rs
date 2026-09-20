//! The lens flare: `docs/10-sky.md`, "The lens flare".
//!
//! `CSun::RenderFlare` strings twelve sprites along the line from the body's place on
//! screen through the centre of the screen, taking each one's place, size, colour and
//! texture from four parallel tables ([`parkan_sim::sky::FLARE_ELEMENTS`]). It is a 2D
//! overlay drawn after the scene -- it is in the lens, not the world -- so it takes no
//! depth test, and both its materials are `SUN.0` blended additively: *measured* over the
//! 29 shipped missions, `sky.wea` slot 5 is cell 1 and slot 6 cell 3 on every one, and
//! both carry blend mode 2, `SRCALPHA`/`ONE`.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use parkan_sim::sky::{FLARE_CUT, FLARE_ELEMENTS, FLARE_SCALE, flare_view_gate};

use crate::body::{additive_blend, skin_group, skin_layout};
use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    /// Straight to clip space: the flare is drawn in the lens.
    position: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
}

/// One of the two `sky.wea` flare slots, resolved against the uploaded textures.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Look {
    pub texture: Option<usize>,
    /// The material's cell of it, `(u0, v0, du, dv)`.
    pub cell: [f32; 4],
}

/// The flare to draw this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flare {
    /// The unit direction from the eye to the body whose flare this is.
    pub toward: Vec3,
    /// The **second** gate: how high the body stands, which rises and falls as it crosses.
    /// The first gate is the view's, and the view is the camera this frame is drawn with,
    /// so it is worked out here rather than handed in.
    pub height_gate: f32,
    /// Slots 5 and 6, which the table's texture column picks between.
    pub looks: [Look; 2],
}

/// The twelve ghosts' quads in clip space, given the body's place and the viewport.
///
/// Position 1 is the body and 0 the middle of the screen, so the chain starts just past the
/// body and runs out the far side. A ghost's half-size is `0.25 × (viewport width / 2) ×
/// size` in pixels, which makes the largest of them an eighth of the screen across; here it
/// is carried in clip space, where the width is 2.
pub fn ghosts(at: Vec2, intensity: f32, aspect: f32) -> Vec<([Vec2; 4], [f32; 4], usize)> {
    FLARE_ELEMENTS
        .iter()
        .map(|g| {
            let centre = at * g.at;
            let half = FLARE_SCALE * g.size;
            let (x, y) = (Vec2::X * half, Vec2::Y * half * aspect);
            let [a, r, gr, b] = g.colour;
            let tint = [
                f32::from(r) / 255.0,
                f32::from(gr) / 255.0,
                f32::from(b) / 255.0,
                f32::from(a) / 255.0 * intensity,
            ];
            ([centre - x + y, centre + x + y, centre + x - y, centre - x - y], tint, g.texture)
        })
        .collect()
}

/// Where a body stands in normalised device coordinates, or `None` behind the camera,
/// where the projection wraps round and there is no flare to draw.
pub fn on_screen(view_proj: Mat4, eye: Vec3, toward: Vec3) -> Option<Vec2> {
    let clip = view_proj * (eye + toward.normalize_or_zero() * crate::body::BODY_DISTANCE).extend(1.0);
    (clip.w > 0.0).then(|| Vec2::new(clip.x / clip.w, clip.y / clip.w))
}

/// The eye and the view axis this frame is drawn from, read back out of its camera.
///
/// The flare is gated on the **view**, and a screenshot may be taken from somewhere other
/// than the hero's eye, so taking the camera's own word for it is what keeps a picture
/// honest about what the game would show.
pub fn camera_axis(view_proj: Mat4) -> (Vec3, Vec3) {
    let inverse = view_proj.inverse();
    // Reverse depth: NDC z runs 1 at the near plane towards 0 at infinity.
    let near = inverse.project_point3(Vec3::new(0.0, 0.0, 1.0));
    let along = inverse.project_point3(Vec3::new(0.0, 0.0, 0.5));
    (near, (along - near).normalize_or_zero())
}

pub struct FlareRenderer {
    pipeline: wgpu::RenderPipeline,
    skins: Vec<wgpu::BindGroup>,
    white: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// This frame's quads: skin, first vertex.
    draws: Vec<(Option<usize>, u32)>,
}

impl FlareRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, bank: &GpuTextures) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("flare.wgsl"));
        let layout = skin_layout(device, "flare skin");
        let skin = |view, label| skin_group(device, &layout, bank, view, label);
        let skins = bank.views.iter().map(|v| skin(v, "flare skin")).collect();
        let white = skin(&bank.white, "flare white");
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("flare"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("flare"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            // No depth test at all: the flare is in the lens, over everything the scene
            // drew, so it neither tests nor writes -- the pass it joins carries a depth
            // attachment for the effects drawn beside it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DEPTH_FORMAT,
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
                    blend: Some(additive_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flare"),
            size: (FLARE_ELEMENTS.len() * 6 * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, skins, white, vertices, draws: Vec::new() }
    }

    /// This frame's chain, or none where no body is up, the first gate has shut, or the
    /// body is behind the camera.
    pub fn prepare(&mut self, queue: &wgpu::Queue, view_proj: Mat4, flare: Option<&Flare>, aspect: f32) {
        self.draws.clear();
        let Some(flare) = flare else { return };
        let (eye, forward) = camera_axis(view_proj);
        // The first gate: out once the body is more than 15° off the view axis, linear in
        // the cosine to full on it, and the ramp then squared. Below 0.1 there is no flare
        // at all.
        let gate = flare_view_gate(forward, flare.toward);
        if gate < FLARE_CUT {
            return;
        }
        let Some(at) = on_screen(view_proj, eye, flare.toward) else { return };
        let mut vertices: Vec<GpuVertex> = Vec::with_capacity(FLARE_ELEMENTS.len() * 6);
        for (quad, tint, slot) in ghosts(at, gate * gate * flare.height_gate, aspect) {
            let look = flare.looks[slot.min(flare.looks.len() - 1)];
            let [u0, v0, du, dv] = look.cell;
            let uv = [[u0, v0], [u0 + du, v0], [u0 + du, v0 + dv], [u0, v0 + dv]];
            self.draws.push((look.texture, vertices.len() as u32));
            let [tl, tr, br, bl] = quad;
            for (p, uv) in [(tl, uv[0]), (tr, uv[1]), (br, uv[2]), (tl, uv[0]), (br, uv[2]), (bl, uv[3])] {
                vertices.push(GpuVertex { position: p.to_array(), uv, tint });
            }
        }
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.draws.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for &(texture, first) in &self.draws {
            pass.set_bind_group(0, texture.and_then(|i| self.skins.get(i)).unwrap_or(&self.white), &[]);
            pass.draw(first..first + 6, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chain_runs_from_the_body_through_the_centre_and_out_the_far_side() {
        let body = Vec2::new(0.5, 0.3);
        let chain = ghosts(body, 1.0, 1.0);
        assert_eq!(chain.len(), 12);
        let centre = |i: usize| chain[i].0.iter().copied().sum::<Vec2>() / 4.0;
        // The first ghost sits past the body at 1.2, the fifth on the centre of the
        // screen at 0, and the last beyond it at -1.1.
        assert!((centre(0) - body * 1.2).length() < 1e-5, "{}", centre(0));
        assert!(centre(4).length() < 1e-5, "the 0.0 element is the middle: {}", centre(4));
        assert!(centre(11).dot(body) < 0.0, "the tail is the other side of the centre");
    }

    #[test]
    fn only_the_alpha_takes_the_intensity() {
        let full = ghosts(Vec2::new(0.4, 0.0), 1.0, 1.0);
        let half = ghosts(Vec2::new(0.4, 0.0), 0.5, 1.0);
        for (a, b) in full.iter().zip(&half) {
            assert_eq!(a.1[..3], b.1[..3], "the colour bytes are left alone");
            assert!((a.1[3] * 0.5 - b.1[3]).abs() < 1e-6, "the alpha is scaled");
        }
        // The table's own alphas are 0xFF or 0x96, never zero.
        assert!(full.iter().all(|g| g.1[3] > 0.5), "{:?}", full.iter().map(|g| g.1[3]).collect::<Vec<_>>());
    }

    #[test]
    fn a_ghost_is_square_on_screen_and_the_largest_an_eighth_across() {
        let chain = ghosts(Vec2::new(0.2, 0.1), 1.0, 16.0 / 9.0);
        let width = |i: usize| (chain[i].0[1].x - chain[i].0[0].x) / 2.0;
        // Clip space is 2 wide, so an eighth of the screen is 0.25 in x.
        let largest = (0..12).map(width).fold(0.0_f32, f32::max);
        assert!((largest - 0.25).abs() < 1e-6, "{largest}");
        // The aspect keeps it square: in y it is that much times the aspect.
        let tall = (chain[9].0[0].y - chain[9].0[3].y) / 2.0;
        assert!((tall / largest - 16.0 / 9.0).abs() < 1e-5, "{tall}");
    }

    #[test]
    fn a_body_behind_the_camera_has_no_flare() {
        let proj = Mat4::perspective_infinite_reverse_rh(1.0, 16.0 / 9.0, 0.1);
        let view = proj * Mat4::look_to_rh(Vec3::ZERO, Vec3::Y, Vec3::Z);
        assert!(on_screen(view, Vec3::ZERO, Vec3::Y).is_some(), "ahead");
        assert!(on_screen(view, Vec3::ZERO, -Vec3::Y).is_none(), "behind, where the projection wraps");
    }

    #[test]
    fn the_cameras_own_axis_comes_back_out_of_its_matrix() {
        let proj = Mat4::perspective_infinite_reverse_rh(1.0, 16.0 / 9.0, 0.1);
        let eye = Vec3::new(120.0, -40.0, 15.0);
        let look = Vec3::new(0.3, 0.9, 0.2).normalize();
        let (read_eye, read_forward) = camera_axis(proj * Mat4::look_to_rh(eye, look, Vec3::Z));
        assert!(read_forward.dot(look) > 0.999_9, "{read_forward} against {look}");
        // The eye comes back as the near plane's centre, a tenth of a unit along.
        assert!((read_eye - eye).length() < 0.2, "{read_eye}");
    }
}
