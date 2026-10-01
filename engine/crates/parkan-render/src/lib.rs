//! The wgpu renderer. It draws what it is handed and never opens a file.
//!
//! Everything is in game space: Z up, +X east, +Y north, metres. The camera
//! matrix uses reverse Z — depth clears to 0 and nearer is greater — so the
//! far distances of a map cost no precision up close.

use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use parkan_world::hud::Layer;
use wgpu::util::DeviceExt;

pub mod body;
pub mod dome;
pub mod flare;
pub mod frame;
pub mod hud;
pub mod models;
pub mod shade;
pub mod sprites;
pub mod terrain;
pub mod text;
pub mod textures;
pub mod ui;

pub use models::ModelRenderer;
pub use terrain::TerrainRenderer;
pub use textures::GpuTextures;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The colour format of an offscreen capture.
pub const CAPTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const CLEAR: wgpu::Color = wgpu::Color { r: 0.05, g: 0.06, b: 0.08, a: 1.0 };
/// The text slot the tooltip's text draws in: its box, [`Layer::Tip`], goes between the slots
/// below it and this one.
pub const TIP_TEXT_SLOT: usize = 4;

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

/// A second look at some placed objects, drawn after the scene and the HUD art under it:
/// `instances`, shown or hidden, through their own camera and light, into `viewport`
/// (x, y, width, height in screen pixels, y down) over depth of their own; each flat in its
/// paint where one is given.
#[derive(Clone, Debug)]
pub struct ModelView {
    pub viewport: [f32; 4],
    pub view_proj: Mat4,
    pub lighting: frame::Lighting,
    pub instances: Vec<usize>,
    pub paints: Option<Vec<[f32; 3]>>,
    /// Drawn from the previews' own objects ([`Renderer::set_previews`]) rather than the
    /// world's.
    pub previews: bool,
    /// Drawn over the world but under the HUD, as a building being placed is.
    pub under_hud: bool,
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
    /// The format everything is drawn in: the target's, read without sRGB decoding. The game's
    /// device writes and blends the values its surface holds, the scene's as much as the
    /// HUD's, so the frame is written with the stored values and no encoding between
    /// (docs/10-sky.md, "The frame holds what the files hold").
    display: wgpu::TextureFormat,
    solid: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    triangles: Option<(wgpu::Buffer, u32)>,
    grid: Option<(wgpu::Buffer, u32)>,
    depth: Option<(wgpu::TextureView, u32, u32)>,
    terrain: Option<TerrainRenderer>,
    objects: Option<ModelRenderer>,
    bank: Option<GpuTextures>,
    /// Models shown only in views, apart from the world: the designer's previews.
    previews: Option<ModelRenderer>,
    preview_bank: Option<GpuTextures>,
    lighting: frame::Lighting,
    dome: Option<(dome::DomeRenderer, Vec<[f32; 4]>)>,
    /// The sun's and the moon's sprites, drawn over the dome.
    bodies: Option<body::BodyRenderer>,
    body_sprites: Vec<body::Sprite>,
    /// The lens flare, a 2D overlay over the whole scene.
    flare: Option<flare::FlareRenderer>,
    this_frames_flare: Option<flare::Flare>,
    hud: Option<hud::HudRenderer>,
    text: Option<text::TextRenderer>,
    /// Text in other fonts, drawn after `text` in slot order.
    more_text: Vec<Option<text::TextRenderer>>,
    sprites: Option<sprites::SpriteRenderer>,
    ui: Option<ui::UiRenderer>,
    views: Vec<(ModelView, models::ViewFrame)>,
    /// The objects' uniforms through the water's reflection camera.
    reflection_frame: Option<models::ViewFrame>,
}

impl Renderer {
    /// A renderer for a target of `format`, which it draws into through a view of the same
    /// format without the sRGB suffix ([`Renderer::display_format`]).
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let display = format.remove_srgb_suffix();
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
                    targets: &[Some(display.into())],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            display,
            solid: pipeline(wgpu::PrimitiveTopology::TriangleList, "solid"),
            lines: pipeline(wgpu::PrimitiveTopology::LineList, "lines"),
            camera,
            bind_group,
            triangles: None,
            grid: None,
            depth: None,
            terrain: None,
            objects: None,
            bank: None,
            previews: None,
            preview_bank: None,
            lighting: frame::Lighting::default(),
            dome: None,
            bodies: None,
            body_sprites: Vec::new(),
            flare: None,
            this_frames_flare: None,
            hud: None,
            text: None,
            more_text: Vec::new(),
            sprites: None,
            ui: None,
            views: Vec::new(),
            reflection_frame: None,
        }
    }

    /// The format the frame is drawn in: the target's, read without sRGB decoding.
    pub fn display_format(&self) -> wgpu::TextureFormat {
        self.display
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
        self.terrain = terrain.map(|t| TerrainRenderer::new(device, queue, self.display, t, &bank));
        self.objects = objects.map(|o| ModelRenderer::new(device, self.display, o, &bank));
        self.reflection_frame = self.objects.as_ref().map(|o| o.view_frame(device));
        self.bank = Some(bank);
    }

    /// The sun's and the moon's sprites, from the textures `set_world` uploaded.
    pub fn set_body_sprites(&mut self, device: &wgpu::Device, sprites: Vec<body::Sprite>) {
        if self.bodies.is_none()
            && let Some(bank) = &self.bank
        {
            // A body draws in the dome's own pass.
            self.bodies = Some(body::BodyRenderer::new(device, self.display, bank));
        }
        self.body_sprites = sprites;
    }

    /// The looks effect sprites draw with, by index, from the textures `set_world` uploaded.
    pub fn set_sprite_looks(&mut self, device: &wgpu::Device, looks: &[sprites::SpriteLook]) {
        if let Some(bank) = &self.bank {
            self.sprites = Some(sprites::SpriteRenderer::new(device, self.display, looks, bank));
        }
    }

    /// The light and fog to draw with from now on; the clear colour follows the fog.
    pub fn set_lighting(&mut self, lighting: frame::Lighting) {
        self.lighting = lighting;
    }

    /// The effects' point lights the scene is lit with from now on, beside the lighting's own:
    /// of `lights`, those whose reach comes nearest the eye.
    pub fn set_point_lights(&mut self, lights: &[frame::PointLight], eye: Vec3) {
        self.lighting.points = frame::Points::nearest(lights, eye);
    }

    /// The sky dome's shape, drawn from now on. `set_world` must have run: the dome's
    /// nebula and clouds draw with textures from its bank.
    pub fn set_dome(&mut self, device: &wgpu::Device, positions: &[glam::Vec3], indices: &[u32]) {
        if let Some(bank) = &self.bank {
            let dome = dome::DomeRenderer::new(device, self.display, positions, indices, bank);
            self.dome = Some((dome, Vec::new()));
        }
    }

    /// The dome's vertex colours for this frame, each with its keyframe's own alpha.
    pub fn set_dome_colours(&mut self, colours: Vec<[f32; 4]>) {
        if let Some((_, c)) = self.dome.as_mut() {
            *c = colours;
        }
    }

    /// Which textures the sky's nebula and clouds draw with, and the clouds' tint.
    pub fn set_sky_layers(&mut self, layers: dome::Layers) {
        if let Some((dome, _)) = self.dome.as_mut() {
            dome.set_layers(layers);
        }
    }

    /// This frame's lens flare, or `None` where no body is up or its gate has shut.
    pub fn set_flare(&mut self, device: &wgpu::Device, f: Option<flare::Flare>) {
        if f.is_some()
            && self.flare.is_none()
            && let Some(bank) = &self.bank
        {
            // The display format: the flare is a 2D overlay, which the game's device
            // blends in the surface's own space as the effects are.
            self.flare = Some(flare::FlareRenderer::new(device, self.display, bank));
        }
        self.this_frames_flare = f;
    }

    /// This frame's HUD rectangles.
    pub fn set_hud(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, rects: &[hud::Rect]) {
        let display = self.display;
        let hud = self.hud.get_or_insert_with(|| hud::HudRenderer::new(device, display));
        hud.prepare(device, queue, rects);
    }

    /// Draw the HUD's art from `pages` from now on; until then [`Renderer::set_ui`] draws
    /// nothing.
    pub fn set_ui_pages(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pages: &[parkan_world::hud::Page],
    ) {
        self.ui = Some(ui::UiRenderer::new(device, queue, self.display, pages));
    }

    /// This frame's HUD art on a `width` × `height` screen, drawn before the HUD rectangles.
    pub fn set_ui(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        batches: &[parkan_world::hud::Batch],
    ) {
        if let Some(ui) = self.ui.as_mut() {
            ui.prepare(device, queue, size, batches);
        }
    }

    /// Draw text in `font` from now on; until then [`Renderer::set_text`] draws nothing.
    pub fn set_font(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: parkan_world::text::GameFont,
    ) {
        self.text = Some(text::TextRenderer::new(device, queue, self.display, font));
    }

    /// This frame's text, drawn after the HUD.
    pub fn set_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[parkan_world::text::TextRun],
    ) {
        if let Some(t) = self.text.as_mut() {
            t.prepare(device, queue, runs);
        }
    }

    /// Draw slot `slot`'s text in `font` from now on; a slot's text draws after the main
    /// text and every lower slot's.
    pub fn set_font_slot(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        font: parkan_world::text::GameFont,
    ) {
        if self.more_text.len() <= slot {
            self.more_text.resize_with(slot + 1, || None);
        }
        self.more_text[slot] = Some(text::TextRenderer::new(device, queue, self.display, font));
    }

    /// This frame's text in slot `slot`'s font.
    pub fn set_text_slot(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        runs: &[parkan_world::text::TextRun],
    ) {
        if let Some(Some(t)) = self.more_text.get_mut(slot) {
            t.prepare(device, queue, runs);
        }
    }

    /// Slot `slot`'s font, once [`Renderer::set_font_slot`] has given one.
    pub fn font_slot(&self, slot: usize) -> Option<&parkan_world::text::GameFont> {
        self.more_text.get(slot).and_then(Option::as_ref).map(text::TextRenderer::font)
    }

    /// The font text is drawn in, once [`Renderer::set_font`] has given one.
    pub fn font(&self) -> Option<&parkan_world::text::GameFont> {
        self.text.as_ref().map(text::TextRenderer::font)
    }

    /// This frame's effect quads.
    pub fn set_sprites(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        quads: &[sprites::Quad],
    ) {
        if let Some(s) = self.sprites.as_mut() {
            s.prepare(device, queue, view_proj, &self.lighting, quads);
        }
    }

    /// Objects drawn only in views, with textures of their own: the designer's previews.
    pub fn set_previews(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        textures: &[parkan_world::textures::Texture],
        objects: &parkan_world::models::Objects,
    ) {
        let bank = GpuTextures::new(device, queue, textures);
        self.previews = Some(ModelRenderer::new(device, self.display, objects, &bank));
        self.preview_bank = Some(bank);
        // Frames made for the old previews' layout are not kept.
        self.views.retain(|(v, _)| !v.previews);
    }

    /// Move a preview's instance to `matrix`, or hide it.
    pub fn set_preview_instance(&mut self, queue: &wgpu::Queue, index: usize, matrix: Mat4, visible: bool) {
        if let Some(previews) = self.previews.as_mut() {
            previews.set_instance(queue, index, matrix, visible);
        }
    }

    /// This frame's views of placed objects, or of the previews, drawn in order.
    pub fn set_views(&mut self, device: &wgpu::Device, views: Vec<ModelView>) {
        let mut world: Vec<models::ViewFrame> = Vec::new();
        let mut previews: Vec<models::ViewFrame> = Vec::new();
        for (v, f) in self.views.drain(..) {
            if v.previews { previews.push(f) } else { world.push(f) }
        }
        let mut out = Vec::new();
        for view in views {
            let (pool, renderer) =
                if view.previews { (&mut previews, &self.previews) } else { (&mut world, &self.objects) };
            let Some(renderer) = renderer else { continue };
            let frame = pool.pop().unwrap_or_else(|| renderer.view_frame(device));
            out.push((view, frame));
        }
        self.views = out;
    }

    /// Put a placed object's instance at `matrix` for the views without showing or hiding it
    /// in the scene.
    pub fn move_instance(&self, queue: &wgpu::Queue, index: usize, matrix: Mat4) {
        if let Some(objects) = &self.objects {
            objects.move_instance(queue, index, matrix);
        }
    }

    /// Move a placed object's instance to `matrix`, or hide it.
    pub fn set_instance(&mut self, queue: &wgpu::Queue, index: usize, matrix: Mat4, visible: bool) {
        if let Some(objects) = self.objects.as_mut() {
            objects.set_instance(queue, index, matrix, visible);
        }
    }

    /// Whose a placed object's instance is, for the point lights that light their owner alone.
    pub fn set_instance_owner(&self, queue: &wgpu::Queue, index: usize, owner: u32) {
        if let Some(objects) = &self.objects {
            objects.set_instance_owner(queue, index, owner);
        }
    }

    /// Where an instance's played materials stand, 0..1, in place of the world clock: a
    /// tracked chassis's belt, which its device plays (docs/28-chassis.md, "The belt is a
    /// material a channel plays"). `None` puts them back on the clock.
    pub fn set_model_phase(&self, index: usize, phase: Option<f32>) {
        if let Some(objects) = &self.objects {
            objects.set_model_phase(index, phase);
        }
    }

    /// Draw the scene and the HUD into `display`, a view of a `width` × `height` texture in
    /// [`Renderer::display_format`].
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        display: &wgpu::TextureView,
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
        if let Some((dome, colours)) = &self.dome {
            dome.prepare(queue, view_proj, &self.lighting, colours);
        }
        if let Some(bodies) = self.bodies.as_mut() {
            bodies.prepare(queue, view_proj, self.lighting.eye, &self.body_sprites);
        }
        let this_frames_flare = self.this_frames_flare;
        if let Some(f) = self.flare.as_mut() {
            f.prepare(queue, view_proj, this_frames_flare.as_ref(), width as f32 / height.max(1) as f32);
        }
        if let Some(terrain) = &self.terrain {
            terrain.prepare(queue, view_proj, &self.lighting);
        }
        let reflection = self
            .terrain
            .as_mut()
            .and_then(|t| t.prepare_reflection(device, queue, &self.lighting, (width, height)));
        if let Some((reflected, uniform)) = &reflection {
            if let Some((dome, _)) = &self.dome {
                let [x, y, z, _] = uniform.eye;
                dome.prepare_reflection(queue, *reflected, Vec3::new(x, y, z));
            }
            if let (Some(objects), Some(frame)) = (&self.objects, &self.reflection_frame) {
                objects.prepare_view(queue, frame, uniform);
            }
        }
        if let Some(objects) = &self.objects {
            objects.prepare(queue, view_proj, &self.lighting);
        }
        if let Some(previews) = &self.previews {
            previews.prepare(queue, view_proj, &self.lighting);
        }
        for (view, frame) in &self.views {
            let renderer = if view.previews { &self.previews } else { &self.objects };
            let Some(objects) = renderer else { continue };
            let mut uniform = frame::FrameUniform::new(view.view_proj, &view.lighting);
            if let Some(paints) = &view.paints {
                uniform.paint = [1.0, 0.0, 0.0, 0.0];
                for (&i, &[r, g, b]) in view.instances.iter().zip(paints) {
                    objects.paint_instance(queue, i, [r, g, b, 1.0]);
                }
            }
            objects.prepare_view(queue, frame, &uniform);
        }
        for text in self.more_text.iter().flatten() {
            text.resize(queue, (width, height));
        }
        if let Some(text) = &self.text {
            text.resize(queue, (width, height));
        }
        // STAND-IN: docs/10-sky.md#the-skys-first-draw-is-a-screen-wide-quad--read -- what
        // stands below the dome's rim is the sky's own screen-wide quad, drawn first of all
        // in a scene camera's frame; that it carries the scene colour is derived and not
        // checked against a frame, so it is not drawn, and the frame is cleared to the fog
        // colour, where the horizon meets it.
        let [fr, fg, fb] = self.lighting.fog_colour;
        let clear = if self.lighting.fog_end < f32::MAX {
            wgpu::Color { r: f64::from(fr), g: f64::from(fg), b: f64::from(fb), a: 1.0 }
        } else {
            CLEAR
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        // The water's reflection first (`Terrain.dll:0x100844c0`): the dome, the ground and the
        // objects through the mirrored camera into the reflection's texture.
        //
        // STAND-IN: docs/03-terrain.md#not-established -- what the reflection camera's pass
        // flags 0x120 leave out is not read; the effects' sprites are left out.
        if let (Some(terrain), Some(_)) = (&self.terrain, &reflection)
            && let Some((colour, depth)) = terrain.reflection_targets()
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("reflection"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: colour,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
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
            if let Some((dome, _)) = &self.dome {
                dome.draw_reflection(&mut pass);
            }
            terrain.draw_reflection(&mut pass);
            if let (Some(objects), Some(frame)) = (&self.objects, &self.reflection_frame) {
                objects.draw_through(&mut pass, frame);
            }
        }
        {
            let depth = &self.depth.as_ref().expect("made above").0;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: display,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
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
            if let Some((dome, _)) = &self.dome {
                dome.draw(&mut pass);
            }
            // The bodies go over the dome and under the scene: they write no depth, so
            // the ground and the objects drawn next paint over them.
            if let Some(bodies) = &self.bodies {
                bodies.draw(&mut pass);
            }
            // The clouds are the last of the sky's layers, so they pass in front of a
            // body: the cap they ride is 5000 below the camera and that much nearer.
            if let Some((dome, _)) = &self.dome {
                dome.draw_clouds(&mut pass);
            }
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
        let depth = &self.depth.as_ref().expect("made above").0;
        // The effects over the scene, in a pass of their own after it.
        if let Some(sprites) = &self.sprites {
            let mut pass = pass_over(&mut encoder, display, depth, "effects", wgpu::LoadOp::Load);
            sprites.draw(&mut pass);
        }
        // The flare over the whole scene, in the lens rather than the world.
        if let Some(f) = &self.flare {
            let mut pass = pass_over(&mut encoder, display, depth, "flare", wgpu::LoadOp::Load);
            f.draw(&mut pass);
        }
        self.draw_views(&mut encoder, display, depth, (width, height), true);
        if let Some(ui) = &self.ui {
            let mut pass = pass_over(&mut encoder, display, depth, "hud under", wgpu::LoadOp::Load);
            ui.draw(&mut pass, Layer::UnderViews);
        }
        self.draw_views(&mut encoder, display, depth, (width, height), false);
        {
            let mut pass = pass_over(&mut encoder, display, depth, "overlay", wgpu::LoadOp::Load);
            if let Some(ui) = &self.ui {
                ui.draw(&mut pass, Layer::OverViews);
            }
            if let Some(hud) = &self.hud {
                hud.draw(&mut pass);
            }
            if let Some(text) = &self.text {
                text.draw(&mut pass);
            }
            // The tooltip's box goes over every slot's text below the tooltip's own slot, and
            // its text over the box (docs/37, "A tooltip").
            for (slot, text) in self.more_text.iter().enumerate() {
                if slot == TIP_TEXT_SLOT
                    && let Some(ui) = &self.ui
                {
                    ui.draw(&mut pass, Layer::Tip);
                }
                if let Some(text) = text {
                    text.draw(&mut pass);
                }
            }
            if self.more_text.len() <= TIP_TEXT_SLOT
                && let Some(ui) = &self.ui
            {
                ui.draw(&mut pass, Layer::Tip);
            }
            // The mouse cursor last of all, over the text as well (docs/42, "The cursor
            // shows a state").
            if let Some(ui) = &self.ui {
                ui.draw(&mut pass, Layer::OverText);
            }
        }
        queue.submit([encoder.finish()]);
    }
}

/// A pass that draws over what `target` and `depth` hold, clearing or keeping the depth.
impl Renderer {
    /// The model views drawn under the HUD (`under`) or over it, each in its viewport over a
    /// cleared depth.
    fn draw_views(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        (width, height): (u32, u32),
        under: bool,
    ) {
        for (view, frame) in self.views.iter().filter(|(v, _)| v.under_hud == under) {
            let renderer = if view.previews { &self.previews } else { &self.objects };
            let Some(objects) = renderer else { continue };
            let [x, y, w, h] = view.viewport;
            let (x0, y0) = (x.max(0.0), y.max(0.0));
            let (x1, y1) = ((x + w).min(width as f32), (y + h).min(height as f32));
            if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
                continue;
            }
            let mut pass = pass_over(encoder, target, depth, "view", wgpu::LoadOp::Clear(0.0));
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
            pass.set_scissor_rect(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);
            objects.draw_view(&mut pass, frame, &view.instances, view.paints.is_some());
        }
    }
}

fn pass_over<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    depth: &wgpu::TextureView,
    label: &str,
    depth_load: wgpu::LoadOp<f32>,
) -> wgpu::RenderPass<'e> {
    encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: depth_load, store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
        .forget_lifetime()
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
        view_formats: &[CAPTURE_FORMAT.remove_srgb_suffix()],
    });
    let display = texture.create_view(&wgpu::TextureViewDescriptor {
        format: Some(CAPTURE_FORMAT.remove_srgb_suffix()),
        ..Default::default()
    });
    renderer.draw(device, &gpu.queue, &display, (width, height), view_proj);

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
