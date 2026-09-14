//! Drawing the placed objects built by `parkan_world::models`.
//!
//! One pipeline per blend mode the materials name, as `Ngi32.dll`'s mode table
//! gives them (`docs/07-objects.md`): 0 draws opaque; 1 `SRCALPHA/ZERO`; 2
//! `SRCALPHA/ONE`, additive; 3 `ZERO/SRCCOLOR`; 4 `SRCALPHA/INVSRCALPHA`; 5
//! `DESTCOLOR/SRCCOLOR`. Every mode but 0 also alpha-tests.

use std::cell::Cell;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use parkan_world::models::Objects;
use parkan_world::textures::{Animation, Phase};
use wgpu::util::DeviceExt;

use crate::DEPTH_FORMAT;
use crate::frame::FrameUniform;
use crate::textures::GpuTextures;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    lightmap: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LookUniform {
    diffuse: [f32; 4],
    emissive: [f32; 4],
    fog: [f32; 4],
    cell: [f32; 4],
    /// x 1: a lit batch, which its lightmap shades in place of the scene's lights.
    lit: [f32; 4],
}

impl LookUniform {
    /// A phase as the device material takes it (docs/07, "How a material reaches the
    /// device"): diffuse with the ambient alpha, the ambient colour as the emissive the
    /// scene colour is added to, and the cell's rectangle. A lit batch keeps its diffuse,
    /// which the shader moves into the emissive (docs/07, "How a lightmapped batch is
    /// drawn").
    fn new(phase: &Phase, blend_mode: u8, lit: bool) -> Self {
        let [dr, dg, db] = phase.diffuse;
        let [ar, ag, ab] = phase.ambient;
        Self {
            diffuse: [dr, dg, db, phase.alpha],
            emissive: [ar, ag, ab, 1.0],
            fog: crate::frame::fog_override(blend_mode),
            cell: phase.cell,
            lit: [f32::from(u8::from(lit)), 0.0, 0.0, 0.0],
        }
    }
}

/// An instance's uniform: its matrix, and the colour a view paints it in.
const INSTANCE_FLOATS: usize = 20;

/// The blend modes a pipeline exists for, opaque first.
pub const BLEND_MODES: [u8; 6] = [0, 1, 2, 3, 4, 5];

fn blend_state(mode: u8) -> Option<wgpu::BlendState> {
    use wgpu::BlendFactor as F;
    let colour = |src, dst| wgpu::BlendComponent {
        src_factor: src,
        dst_factor: dst,
        operation: wgpu::BlendOperation::Add,
    };
    let (src, dst) = match mode {
        1 => (F::SrcAlpha, F::Zero),
        2 => (F::SrcAlpha, F::One),
        3 => (F::Zero, F::Src),
        4 => (F::SrcAlpha, F::OneMinusSrcAlpha),
        5 => (F::Dst, F::Src),
        _ => return None,
    };
    Some(wgpu::BlendState { color: colour(src, dst), alpha: colour(F::One, F::OneMinusSrcAlpha) })
}

struct DrawGroup {
    start: u32,
    count: u32,
    mode: u8,
    lit: bool,
    look: wgpu::Buffer,
    /// A bind group for each texture the material's phases draw, and the one drawn now.
    bind_groups: Vec<(Option<usize>, wgpu::BindGroup)>,
    current: Cell<usize>,
    animation: Option<Animation>,
}

struct GpuModel {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    groups: Vec<DrawGroup>,
}

struct GpuInstance {
    model: usize,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    visible: bool,
}

/// The frame uniforms of a second look at some instances, as [`ModelRenderer::view_frame`]
/// makes them.
pub struct ViewFrame {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

pub struct ModelRenderer {
    pipelines: Vec<(u8, wgpu::RenderPipeline)>,
    frame_layout: wgpu::BindGroupLayout,
    frame: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    models: Vec<GpuModel>,
    instances: Vec<GpuInstance>,
}

/// An object's placement: `T(position) · Rz(rotation) · S(scale)`.
pub fn placement(position: [f32; 3], rotation: f32, scale: f32) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::splat(scale),
        Quat::from_rotation_z(rotation),
        Vec3::from(position),
    )
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl ModelRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        objects: &Objects,
        textures: &GpuTextures,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("model.wgsl"));
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model frame"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let instance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model instance"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX)],
        });
        let look_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("model look"),
            entries: &[
                uniform_entry(0, wgpu::ShaderStages::FRAGMENT),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model"),
            bind_group_layouts: &[Some(&frame_layout), Some(&instance_layout), Some(&look_layout)],
            immediate_size: 0,
        });
        let pipelines = BLEND_MODES
            .iter()
            .map(|&mode| {
                let blend = blend_state(mode);
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("model"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[Some(wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<GpuVertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![
                                0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x2
                            ],
                        })],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        // `Ngi32.dll:0x10007662`: a mesh batch's draw flags carry 4, culling
                        // off, so every batch is two-sided.
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        // STAND-IN: docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory
                        // -- whether a blended material writes depth, and the alpha test's
                        // reference, are not read; blended draws come last and write no
                        // depth, and nothing is discarded.
                        depth_write_enabled: Some(blend.is_none()),
                        depth_compare: Some(wgpu::CompareFunction::Greater),
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
                            blend,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                });
                (mode, pipeline)
            })
            .collect();

        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model frame"),
            layout: &frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: frame.as_entire_binding() }],
        });

        let models = objects
            .models
            .iter()
            .map(|m| {
                let vertices: Vec<GpuVertex> = m
                    .vertices
                    .iter()
                    .map(|v| GpuVertex {
                        position: v.position,
                        normal: v.normal,
                        uv: v.uv,
                        lightmap: v.lightmap,
                    })
                    .collect();
                let groups = m
                    .groups
                    .iter()
                    .map(|g| {
                        // A lit batch draws opaque, with no alpha test (docs/07, "How a
                        // lightmapped batch is drawn").
                        let lit = g.lightmap.is_some();
                        let mode = if lit { 0 } else { g.look.blend_mode };
                        // Display space: the shader forms the lit colour from them, then decodes it.
                        let uniform = LookUniform::new(&g.look.still, mode, lit);
                        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&g.look.material),
                            contents: bytemuck::bytes_of(&uniform),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });
                        let mut bind_groups: Vec<(Option<usize>, wgpu::BindGroup)> = Vec::new();
                        let phases = std::iter::once(&g.look.still)
                            .chain(g.look.animation.iter().flat_map(|a| a.keys.iter().map(|k| &k.0)));
                        for phase in phases {
                            if bind_groups.iter().any(|(t, _)| *t == phase.texture) {
                                continue;
                            }
                            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                label: Some(&g.look.material),
                                layout: &look_layout,
                                entries: &[
                                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::TextureView(
                                            textures.view(phase.texture),
                                        ),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 2,
                                        resource: wgpu::BindingResource::Sampler(&textures.sampler),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 3,
                                        resource: wgpu::BindingResource::TextureView(
                                            textures.view(g.lightmap),
                                        ),
                                    },
                                ],
                            });
                            bind_groups.push((phase.texture, bind_group));
                        }
                        DrawGroup {
                            start: g.start,
                            count: g.count,
                            mode,
                            lit,
                            look: buffer,
                            bind_groups,
                            current: Cell::new(0),
                            animation: g.look.animation.clone(),
                        }
                    })
                    .collect();
                GpuModel {
                    vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&m.name),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&m.name),
                        contents: bytemuck::cast_slice(&m.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    groups,
                }
            })
            .collect();

        let instances = objects
            .instances
            .iter()
            .map(|i| {
                let matrix = placement(i.position, i.rotation, i.scale);
                let mut contents = [0.0f32; INSTANCE_FLOATS];
                contents[..16].copy_from_slice(&matrix.to_cols_array());
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("instance"),
                    contents: bytemuck::cast_slice(&contents),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("instance"),
                    layout: &instance_layout,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
                });
                GpuInstance { model: i.model, buffer, bind_group, visible: !i.hidden }
            })
            .collect();

        Self { pipelines, frame_layout, frame, frame_bind_group, models, instances }
    }

    /// Move instance `index` to `matrix`, or hide it from the scene; a view still draws a
    /// hidden instance where it was last put.
    pub fn set_instance(&mut self, queue: &wgpu::Queue, index: usize, matrix: Mat4, visible: bool) {
        if let Some(i) = self.instances.get_mut(index) {
            i.visible = visible;
            if visible {
                queue.write_buffer(&i.buffer, 0, bytemuck::bytes_of(&matrix.to_cols_array()));
            }
        }
    }

    /// Put instance `index` at `matrix` without showing or hiding it.
    pub fn move_instance(&self, queue: &wgpu::Queue, index: usize, matrix: Mat4) {
        if let Some(i) = self.instances.get(index) {
            queue.write_buffer(&i.buffer, 0, bytemuck::bytes_of(&matrix.to_cols_array()));
        }
    }

    /// The colour instance `index` is painted in where a view paints.
    pub fn paint_instance(&self, queue: &wgpu::Queue, index: usize, colour: [f32; 4]) {
        if let Some(i) = self.instances.get(index) {
            queue.write_buffer(&i.buffer, 64, bytemuck::cast_slice(&colour));
        }
    }

    /// Uniforms for a view of its own.
    pub fn view_frame(&self, device: &wgpu::Device) -> ViewFrame {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("model view frame"),
            size: std::mem::size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("model view frame"),
            layout: &self.frame_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        ViewFrame { buffer, bind_group }
    }

    /// A view's uniforms for this frame.
    pub fn prepare_view(&self, queue: &wgpu::Queue, frame: &ViewFrame, uniform: &FrameUniform) {
        queue.write_buffer(&frame.buffer, 0, bytemuck::bytes_of(uniform));
    }

    /// The frame's uniforms, and every played material's phase at the lighting's clock:
    /// its colours and cell, and the texture its key names.
    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, lighting: &crate::frame::Lighting) {
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&FrameUniform::new(view_proj, lighting)));
        for g in self.models.iter().flat_map(|m| &m.groups) {
            let Some(animation) = &g.animation else { continue };
            let phase = animation.at(lighting.clock_ms);
            queue.write_buffer(&g.look, 0, bytemuck::bytes_of(&LookUniform::new(&phase, g.mode, g.lit)));
            g.current.set(g.bind_groups.iter().position(|(t, _)| *t == phase.texture).unwrap_or(0));
        }
    }

    /// Opaque groups of every instance first, then each blended mode in turn.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        self.draw_instances(pass, self.instances.iter().filter(|i| i.visible));
    }

    /// Every shown instance through a view's uniforms, as the water's reflection draws them.
    pub fn draw_through(&self, pass: &mut wgpu::RenderPass<'_>, frame: &ViewFrame) {
        pass.set_bind_group(0, &frame.bind_group, &[]);
        self.draw_instances(pass, self.instances.iter().filter(|i| i.visible));
    }

    /// Only `indices`, shown or hidden, through a view's uniforms; a painted view draws every
    /// group opaque, as the game's panel view sets blend mode 0 (docs/35-hud.md).
    pub fn draw_view(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ViewFrame,
        indices: &[usize],
        painted: bool,
    ) {
        pass.set_bind_group(0, &frame.bind_group, &[]);
        let instances = indices.iter().filter_map(|&i| self.instances.get(i));
        if !painted {
            self.draw_instances(pass, instances);
            return;
        }
        let Some((_, opaque)) = self.pipelines.first() else { return };
        pass.set_pipeline(opaque);
        for instance in instances {
            let model = &self.models[instance.model];
            pass.set_bind_group(1, &instance.bind_group, &[]);
            pass.set_vertex_buffer(0, model.vertices.slice(..));
            pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
            for g in &model.groups {
                pass.set_bind_group(2, &g.bind_groups[g.current.get()].1, &[]);
                pass.draw_indexed(g.start..g.start + g.count, 0, 0..1);
            }
        }
    }

    fn draw_instances<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'_>,
        instances: impl Iterator<Item = &'a GpuInstance> + Clone,
    ) {
        for (mode, pipeline) in &self.pipelines {
            pass.set_pipeline(pipeline);
            for instance in instances.clone() {
                let model = &self.models[instance.model];
                let mut bound = false;
                for g in model.groups.iter().filter(|g| g.mode == *mode) {
                    if !bound {
                        pass.set_bind_group(1, &instance.bind_group, &[]);
                        pass.set_vertex_buffer(0, model.vertices.slice(..));
                        pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
                        bound = true;
                    }
                    pass.set_bind_group(2, &g.bind_groups[g.current.get()].1, &[]);
                    pass.draw_indexed(g.start..g.start + g.count, 0, 0..1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placement_turns_about_z_then_moves() {
        let m = placement([10.0, 20.0, 5.0], std::f32::consts::FRAC_PI_2, 2.0);
        let p = m.transform_point3(Vec3::new(0.0, 1.0, 0.0));
        assert!((p - Vec3::new(8.0, 20.0, 5.0)).length() < 1e-5, "{p}");
    }
}
