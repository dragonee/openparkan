//! Drawing the sky dome and its two textured layers: `docs/10-sky.md`, "The dome" and
//! "The three layers and their texture coordinates". The dome is drawn at the camera's
//! position with no rotation (`Terrain.dll:0x1007a17d`).
//!
//! The sky puts three draws of the same cap into the frame, in this order: the **nebula**
//! on it (material block `+0x384`, from `sky.wea` slot 0), the **gradient** over that in
//! vertex colours alone (`+0x404`, untextured), and the **clouds** last (`+0x304`, slot 2)
//! on the same cap with its origin [`CLOUD_DROP`] below the camera. A fourth layer, the
//! stars, is loaded and laid out and never drawn.
//!
//! The first two are unlit items whose vertices carry a specular of `0xff000000`, so the fog
//! leaves them alone. The clouds are a **lit** item with a fog of their own: the shade lights
//! each vertex of their cap with the keyframe's cloud colour as the material's diffuse, and
//! fogs it between [`CLOUD_FOG_START`] and [`CLOUD_FOG_END`], so the layer shows overhead and
//! is the horizon's own colour from the cap's third ring out (docs/10-sky.md, "The clouds
//! are lit, and fogged on a range of their own").

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use parkan_sim::sky::{
    CLOUD_DROP, CLOUD_FOG_END, CLOUD_FOG_START, CLOUD_UV_SCALE, NEBULA_UV_SCALE, dome_normal, layer_uv,
};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;
use crate::frame::Lighting;
use crate::shade;
use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    /// The layer texture coordinates at scale 1; the shader scales them per layer.
    uv: [f32; 2],
    /// The vertex's diffuse colour.
    colour: [f32; 4],
    /// The clouds' specular colour, and in w what their fog keeps.
    spill: [f32; 4],
}

/// What the shade gives one vertex of the clouds' cap: its diffuse colour with the material's
/// ambient alpha, and its specular colour with what the layer's fog keeps in the alpha.
///
/// The material is the entry `sky.wea`'s slot 2 names with its diffuse overwritten by the
/// keyframe's cloud colour and its ambient zeroed (`Terrain.dll:0x1007a4de`-`0x1007a5b3`); its
/// ambient alpha stays the entry's, 1.0 on all three shipped cloud materials, and the
/// keyframe's own alpha is not read. The lights are the frame's two directional ones: the
/// sky's items are filed after the world's (`0x100847a0`), so the list the shade hands them
/// (`0x1002863d`) is the last gathered one, which carries both.
///
/// STAND-IN: docs/10-sky.md#the-clouds-are-lit-and-fogged-on-a-range-of-their-own--read-and-measured
/// -- that list is the last drawn object's or cell's, its point lights with it; here the
/// sun's two alone.
pub fn cloud_vertex(vertex: Vec3, cloud_colour: [f32; 3], lighting: &Lighting) -> ([f32; 4], [f32; 4]) {
    let normal = dome_normal(vertex);
    let mut lights = [0.0; 3];
    for light in &lighting.lights {
        let cosine = shade::cosine(normal, light.direction.normalize_or_zero());
        for c in 0..3 {
            lights[c] += light.colour[c] * cloud_colour[c] * cosine;
        }
    }
    let shaded = shade::lit(lights, [0.0; 3], [0.0; 3], lighting.scene_colour);
    // The cap's origin is the drop below the eye, so the eye stands that far up its axis.
    let keep = shade::fog((vertex - Vec3::Z * CLOUD_DROP).length(), CLOUD_FOG_START, CLOUD_FOG_END);
    let [r, g, b] = shaded.diffuse;
    let [sr, sg, sb] = shaded.specular;
    ([r, g, b, 1.0], [sr, sg, sb, keep])
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct Uniform {
    view_proj: [f32; 16],
    tint: [f32; 4],
    uv_scale: f32,
    /// [`GRADIENT`], [`TEXTURED`] or [`OPAQUE_GRADIENT`].
    mode: u32,
    _pad: [f32; 2],
}

/// Vertex colours, alpha and all: the dome's gradient over the nebula.
const GRADIENT: u32 = 0;
/// The layer's texture as it is: the nebula.
const TEXTURED: u32 = 1;
/// Vertex colours held opaque, for the water's reflection, which has no nebula under it
/// to show through.
const OPAQUE_GRADIENT: u32 = 2;
/// The clouds: the texture by the vertex's lit colour, fogged toward the frame's fog colour.
const CLOUDS: u32 = 3;

/// How the dome meets the depth buffer. The sky's draws at the camera, as the dome is
/// drawn, are depth-tested and write no depth; only the one on a fixed matrix
/// (`0x1007a37a`) turns the test off (render states 7 and 14, `Terrain.dll:0x100302fb`).
/// Drawn first into a cleared buffer, the test passes everywhere.
pub fn depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Greater),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

/// The sky's textured layers for this frame, from the textures `set_world` uploaded.
///
/// Both materials take the whole texture and blend on their own alpha: *measured* over the
/// 29 shipped missions, every slot 0 is an `ENV_NEBULA*` and every slot 2 an `ENV_CLOUDS*`
/// or the one `TOK51`, and all 58 carry cell −1 and blend mode 4,
/// `SRCALPHA`/`INVSRCALPHA`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layers {
    /// Slot 0's texture, drawn on the dome under the gradient. The four shipped nebulae
    /// are 256-pixel `RGB565` images with no alpha at all, so this layer is the backdrop.
    pub nebula: Option<usize>,
    /// Slot 2's, on the cap lowered below the camera.
    pub clouds: Option<usize>,
    /// The clouds' material's diffuse colour: the keyframe's slot 18, as the file gives it.
    pub cloud_colour: [f32; 3],
}

/// One layer's uniform and its bind group.
struct Pass {
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

pub struct DomeRenderer {
    pipeline: wgpu::RenderPipeline,
    /// The nebula, the gradient, the clouds, and the gradient through the water.
    nebula: Pass,
    gradient: Pass,
    clouds: Pass,
    reflection: Pass,
    skins: Vec<wgpu::BindGroup>,
    white: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// The same cap as the clouds' item shades it.
    cloud_vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    positions: Vec<Vec3>,
    layers: Layers,
}

impl DomeRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        positions: &[Vec3],
        indices: &[u32],
        bank: &GpuTextures,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("dome.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dome camera"),
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
        let skin_layout = crate::body::skin_layout(device, "dome skin");
        let pass = |label: &str| {
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<Uniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }],
            });
            Pass { uniform, group }
        };
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dome"),
            bind_group_layouts: &[Some(&layout), Some(&skin_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dome"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            // The game draws the sky in a pass of its own -- group 1's layer 0, which
            // overrides the camera to near 700, far 50000 and viewport z 1.0 to 1.0
            // (docs/10-sky.md, "The dome") -- so its output lands at the very back of the
            // depth buffer and fills only what the scene left. Drawing it first under a
            // projection with no far plane, depth-tested and writing no depth, comes to
            // the same image.
            depth_stencil: Some(depth_state()),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // The layers' own blend, mode 4 on all 58 shipped sky materials. The
                    // gradient takes it too, which is what lets the nebula through: its
                    // colours carry the keyframe's alpha, 0 at the apex on 355 of the 656.
                    blend: Some(crate::body::material_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (positions.len() * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let (vertices, cloud_vertices) = (buffer("dome"), buffer("dome clouds"));
        let count = indices.len() as u32;
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("dome"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let skin = |view, label| crate::body::skin_group(device, &skin_layout, bank, view, label);
        let skins = bank.stored.iter().map(|v| skin(v, "dome skin")).collect();
        let white = skin(&bank.white, "dome white");
        Self {
            pipeline,
            nebula: pass("dome nebula"),
            gradient: pass("dome gradient"),
            clouds: pass("dome clouds"),
            reflection: pass("dome reflection"),
            skins,
            white,
            vertices,
            cloud_vertices,
            indices,
            count,
            positions: positions.to_vec(),
            layers: Layers::default(),
        }
    }

    /// Which textures the sky's layers draw with from now on.
    pub fn set_layers(&mut self, layers: Layers) {
        self.layers = layers;
    }

    /// This frame's camera and lights, and the dome's colours, one a vertex.
    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, lighting: &Lighting, colours: &[[f32; 4]]) {
        let eye = lighting.eye;
        let placed = view_proj * Mat4::from_translation(eye);
        let write = |pass: &Pass, matrix: Mat4, tint: [f32; 4], uv_scale: f32, mode: u32| {
            let u = Uniform { view_proj: matrix.to_cols_array(), tint, uv_scale, mode, _pad: [0.0; 2] };
            queue.write_buffer(&pass.uniform, 0, bytemuck::bytes_of(&u));
        };
        write(&self.nebula, placed, [1.0; 4], NEBULA_UV_SCALE, TEXTURED);
        write(&self.gradient, placed, [1.0; 4], 1.0, GRADIENT);
        // The clouds ride the same cap with its origin 5000 below the camera (`0x1007a08e`),
        // fogged toward the frame's fog colour.
        let lowered = view_proj * Mat4::from_translation(eye - Vec3::Z * CLOUD_DROP);
        let [fr, fg, fb] = lighting.fog_colour;
        write(&self.clouds, lowered, [fr, fg, fb, 1.0], CLOUD_UV_SCALE, CLOUDS);
        let vertices: Vec<GpuVertex> = self
            .positions
            .iter()
            .zip(colours.iter().chain(std::iter::repeat(&[0.0; 4])))
            .map(|(p, c)| GpuVertex {
                position: p.to_array(),
                uv: layer_uv(*p, 1.0),
                colour: *c,
                spill: [0.0, 0.0, 0.0, 1.0],
            })
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        let clouds: Vec<GpuVertex> = self
            .positions
            .iter()
            .map(|p| {
                let (colour, spill) = cloud_vertex(*p, self.layers.cloud_colour, lighting);
                GpuVertex { position: p.to_array(), uv: layer_uv(*p, 1.0), colour, spill }
            })
            .collect();
        queue.write_buffer(&self.cloud_vertices, 0, bytemuck::cast_slice(&clouds));
    }

    /// The reflection camera, at its own eye, for [`DomeRenderer::draw_reflection`].
    pub fn prepare_reflection(&self, queue: &wgpu::Queue, view_proj: Mat4, eye: Vec3) {
        let u = Uniform {
            view_proj: (view_proj * Mat4::from_translation(eye)).to_cols_array(),
            tint: [1.0; 4],
            uv_scale: 1.0,
            mode: OPAQUE_GRADIENT,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.reflection.uniform, 0, bytemuck::bytes_of(&u));
    }

    fn skin(&self, texture: Option<usize>) -> &wgpu::BindGroup {
        texture.and_then(|i| self.skins.get(i)).unwrap_or(&self.white)
    }

    fn layer(&self, pass: &mut wgpu::RenderPass<'_>, uniform: &Pass, texture: Option<usize>) {
        self.layer_of(pass, uniform, texture, &self.vertices);
    }

    fn layer_of(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        uniform: &Pass,
        texture: Option<usize>,
        vertices: &wgpu::Buffer,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &uniform.group, &[]);
        pass.set_bind_group(1, self.skin(texture), &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.count, 0, 0..1);
    }

    /// The dome as the reflection camera sees it: the gradient alone.
    pub fn draw_reflection(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.layer(pass, &self.reflection, None);
    }

    /// The nebula, then the gradient over it.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.layers.nebula.is_some() {
            self.layer(pass, &self.nebula, self.layers.nebula);
        }
        self.layer(pass, &self.gradient, None);
    }

    /// The clouds, which the sky draws last of its layers and so after the bodies.
    pub fn draw_clouds(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.layers.clouds.is_some() {
            self.layer_of(pass, &self.clouds, self.layers.clouds, &self.cloud_vertices);
        }
    }
}

#[cfg(test)]
mod tests {
    use parkan_sim::sky;

    use super::*;

    #[test]
    fn the_dome_is_depth_tested_without_writing_depth_and_passes_against_a_cleared_buffer() {
        let state = depth_state();
        assert_eq!(state.depth_write_enabled, Some(false));
        assert_eq!(state.depth_compare, Some(wgpu::CompareFunction::Greater), "the scene's own test");
        // Reverse depth clears to 0; the dome's rim, 24 km out at eye height, still
        // lands in front of that under a projection with no far plane.
        let proj = Mat4::perspective_infinite_reverse_rh(1.0, 16.0 / 9.0, 0.1);
        let rim = proj * Mat4::look_to_rh(Vec3::ZERO, Vec3::Y, Vec3::Z);
        let depth = rim.project_point3(Vec3::new(0.0, 24_142.1, 0.0)).z;
        assert!(depth > 0.0 && depth < 1.0, "{depth}");
    }

    /// C02 M04's sky 43.5 s in: the cloud colour (200, 255, 0), the scene colour
    /// (86, 111, 67), the sun's light (163, 255, 67) at 1.30.
    fn last_bastion(toward_sun: Vec3) -> Lighting {
        Lighting {
            lights: [
                crate::frame::Light { direction: -toward_sun, colour: [0.83, 1.30, 0.34] },
                crate::frame::Light { direction: toward_sun, colour: [0.61, 1.0, 0.38] },
            ],
            scene_colour: [0.337, 0.435, 0.263],
            fog_colour: [0.42, 0.675, 0.098],
            ..Lighting::default()
        }
    }

    #[test]
    fn the_clouds_are_lit_by_the_body_above_them_and_fogged_out_by_the_third_ring() {
        let dome = sky::dome();
        let cloud = [200.0 / 255.0, 1.0, 0.0];
        let overhead = last_bastion(Vec3::Z);
        // The apex: the sun's light on the cloud colour, kneed where it passes 1, and the
        // scene colour holding up the blue the cloud colour has none of. No fog: the apex is
        // the fog's start away.
        let (colour, spill) = cloud_vertex(dome[0], cloud, &overhead);
        assert!((colour[0] - 0.83 * 200.0 / 255.0).abs() < 1e-3, "{colour:?}");
        assert_eq!((colour[1], colour[3]), (1.0, 1.0));
        assert!((colour[2] - 0.263).abs() < 1e-6, "the floor");
        assert!((spill[1] - 0.8 * (1.30 / 6.0 + 5.0 / 6.0 - 1.0)).abs() < 1e-4, "{spill:?}");
        assert_eq!(spill[3], 1.0);
        // Ring by ring the layer goes to the fog's colour: 0.77, 0.07, and none from the third.
        let ring = |r: usize| cloud_vertex(dome[1 + (r - 1) * sky::DOME_SEGMENTS], cloud, &overhead).1[3];
        assert!((ring(1) - 0.766).abs() < 2e-3, "{}", ring(1));
        assert!((ring(2) - 0.068).abs() < 2e-3, "{}", ring(2));
        assert_eq!((ring(3), ring(4), ring(5)), (0.0, 0.0, 0.0));
        // A body under the horizon lights nothing of a cap whose normals point up; then it is
        // the second light, which travels the other way, that falls on it.
        let (colour, spill) = cloud_vertex(dome[0], cloud, &last_bastion(Vec3::NEG_Z));
        assert!((colour[0] - 0.61 * 200.0 / 255.0).abs() < 1e-3, "{colour:?}");
        assert_eq!((colour[1], colour[2]), (1.0, 0.263));
        assert_eq!([spill[0], spill[1], spill[2]], [0.0; 3]);
        // And with both lights side-on the scene colour is all there is.
        let (colour, _) = cloud_vertex(dome[0], cloud, &last_bastion(Vec3::X));
        assert_eq!([colour[0], colour[1], colour[2]], [0.337, 0.435, 0.263]);
    }

    #[test]
    fn the_caps_normals_point_out_of_its_sphere() {
        let dome = sky::dome();
        assert_eq!(sky::dome_normal(dome[0]), Vec3::Z);
        let rim = sky::dome_normal(*dome.last().expect("a rim vertex"));
        // 45 degrees off the axis, in the 127ths the stream holds.
        assert!((rim.z - 90.0 / 127.0).abs() < 1e-6 && (rim.length() - 1.0).abs() < 0.01, "{rim:?}");
    }

    #[test]
    fn a_layers_texture_coordinates_are_the_vertex_from_above_in_radii() {
        let dome = sky::dome();
        let apex = layer_uv(dome[0], sky::NEBULA_UV_SCALE);
        assert_eq!(apex, [0.0, 0.0], "the apex is the middle of the sheet");
        // The rim is R sin(π/4) out, so it reaches 0.707 of the scale, never a whole tile
        // for the nebula and just over three for the clouds.
        let rim = *dome.last().expect("a rim vertex");
        let reach = |scale: f32| {
            let [u, v] = layer_uv(rim, scale);
            u.hypot(v)
        };
        assert!((reach(sky::NEBULA_UV_SCALE) - 0.707).abs() < 0.01, "{}", reach(sky::NEBULA_UV_SCALE));
        assert!((reach(sky::CLOUD_UV_SCALE) - 2.121).abs() < 0.01, "{}", reach(sky::CLOUD_UV_SCALE));
    }
}
