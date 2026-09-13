//! Drawing the placed objects built by `parkan_world::models`.
//!
//! One pipeline per blend mode the materials name, as `Ngi32.dll`'s mode table
//! gives them (`docs/07-objects.md`): 0 draws opaque; 1 `SRCALPHA/ZERO`; 2
//! `SRCALPHA/ONE`, additive; 3 `ZERO/SRCCOLOR`; 4 `SRCALPHA/INVSRCALPHA`; 5
//! `DESTCOLOR/SRCCOLOR`. Every mode but 0 also alpha-tests.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use parkan_world::models::Objects;
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LookUniform {
    diffuse: [f32; 4],
    emissive: [f32; 4],
    fog: [f32; 4],
}

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
    bind_group: wgpu::BindGroup,
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

pub struct ModelRenderer {
    pipelines: Vec<(u8, wgpu::RenderPipeline)>,
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
                            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                        })],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH_FORMAT,
                        // STAND-IN: docs/07-objects.md -- whether a blended phase
                        // writes depth is not read; blended draws come last and do not.
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
                    .map(|v| GpuVertex { position: v.position, normal: v.normal, uv: v.uv })
                    .collect();
                let groups = m
                    .groups
                    .iter()
                    .map(|g| {
                        let [dr, dg, db] = g.look.diffuse;
                        let [er, eg, eb] = g.look.emissive;
                        let uniform = LookUniform {
                            diffuse: [dr, dg, db, 1.0],
                            emissive: [er, eg, eb, 1.0],
                            fog: crate::frame::fog_override(g.look.blend_mode),
                        };
                        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&g.look.material),
                            contents: bytemuck::bytes_of(&uniform),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });
                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some(&g.look.material),
                            layout: &look_layout,
                            entries: &[
                                wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::TextureView(
                                        textures.view(g.look.texture),
                                    ),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Sampler(&textures.sampler),
                                },
                            ],
                        });
                        DrawGroup { start: g.start, count: g.count, mode: g.look.blend_mode, bind_group }
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
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("instance"),
                    contents: bytemuck::bytes_of(&matrix.to_cols_array()),
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

        Self { pipelines, frame, frame_bind_group, models, instances }
    }

    /// Move instance `index` to `matrix`, or hide it.
    pub fn set_instance(&mut self, queue: &wgpu::Queue, index: usize, matrix: Mat4, visible: bool) {
        if let Some(i) = self.instances.get_mut(index) {
            i.visible = visible;
            if visible {
                queue.write_buffer(&i.buffer, 0, bytemuck::bytes_of(&matrix.to_cols_array()));
            }
        }
    }

    pub fn prepare(&self, queue: &wgpu::Queue, view_proj: Mat4, lighting: &crate::frame::Lighting) {
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&FrameUniform::new(view_proj, lighting)));
    }

    /// Opaque groups of every instance first, then each blended mode in turn.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        for (mode, pipeline) in &self.pipelines {
            pass.set_pipeline(pipeline);
            for instance in self.instances.iter().filter(|i| i.visible) {
                let model = &self.models[instance.model];
                let mut bound = false;
                for g in model.groups.iter().filter(|g| g.mode == *mode) {
                    if !bound {
                        pass.set_bind_group(1, &instance.bind_group, &[]);
                        pass.set_vertex_buffer(0, model.vertices.slice(..));
                        pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
                        bound = true;
                    }
                    pass.set_bind_group(2, &g.bind_group, &[]);
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
