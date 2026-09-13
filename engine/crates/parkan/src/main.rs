//! `parkan`: Parkan: Iron Strategy, played from the install.
//!
//! M0 opens a mission and flies a debug camera over its placed objects.
//!
//! ```text
//! parkan [--game DIR] [--mission MISSIONS/…] [--screenshot OUT.png] [--size WxH]
//! ```
//!
//! In the window: W/A/S/D and Q/E fly, the right mouse button held turns,
//! Shift is faster, Escape quits.

mod camera;
mod scene;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use camera::FlyCamera;
use glam::Vec3;
use parkan_formats::gamedir;
use parkan_render::{Gpu, Renderer};
use parkan_world::terrain::Terrain;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

struct Args {
    game: Option<PathBuf>,
    mission: String,
    screenshot: Option<PathBuf>,
    size: (u32, u32),
    top_down: bool,
    /// `--look X,Y,Z,TX,TY,TZ`: a screenshot camera at X,Y,Z looking at TX,TY,TZ.
    look: Option<[f32; 6]>,
}

fn args() -> Result<Args> {
    let mut out = Args {
        game: None,
        mission: gamedir::MISSION_01.to_owned(),
        screenshot: None,
        size: (1280, 720),
        top_down: false,
        look: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--game" => out.game = Some(PathBuf::from(value()?)),
            "--mission" => out.mission = value()?,
            "--screenshot" => out.screenshot = Some(PathBuf::from(value()?)),
            "--top-down" => out.top_down = true,
            "--look" => {
                let v: Vec<f32> = value()?.split(',').map(str::parse).collect::<Result<_, _>>()?;
                out.look = Some(v.try_into().map_err(|_| anyhow::anyhow!("--look takes six numbers"))?);
            }
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').context("--size is WIDTHxHEIGHT")?;
                out.size = (w.parse()?, h.parse()?);
            }
            other => bail!("unknown argument {other}"),
        }
    }
    Ok(out)
}

fn start_camera(loaded: &scene::Loaded) -> FlyCamera {
    match loaded.hero {
        Some((position, heading)) => FlyCamera::behind(position, heading),
        None => FlyCamera { position: Vec3::new(0.0, 0.0, 50.0), yaw: 0.0, pitch: -0.3 },
    }
}

/// The whole map from straight above, north up, for comparing with
/// `openparkan heightmap`.
fn top_down(terrain: &Terrain, aspect: f32) -> glam::Mat4 {
    let (lo, hi) = terrain.land.bounds();
    let centre = Vec3::new((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, hi[2] + 100.0);
    let half = ((hi[0] - lo[0]).max(hi[1] - lo[1]) / 2.0).max(1.0);
    let (hw, hh) = if aspect >= 1.0 { (half * aspect, half) } else { (half, half / aspect) };
    let depth = hi[2] - lo[2] + 200.0;
    // Near and far swapped: depth is reversed, so nearer is greater.
    let proj = glam::Mat4::orthographic_rh(-hw, hw, -hh, hh, depth, 0.0);
    proj * glam::Mat4::look_to_rh(centre, -Vec3::Z, Vec3::Y)
}

fn screenshot(loaded: &scene::Loaded, game: &Path, args: &Args, out: &Path) -> Result<()> {
    let (width, height) = args.size;
    let gpu = pollster::block_on(Gpu::headless())?;
    let world = scene::world(game, loaded)?;
    let mut renderer = Renderer::new(&gpu.device, parkan_render::CAPTURE_FORMAT);
    renderer.set_world(
        &gpu.device,
        &gpu.queue,
        &world.store.textures,
        Some(&world.terrain),
        Some(&world.objects),
    );
    let aspect = width as f32 / height as f32;
    let view_proj = if args.top_down {
        top_down(&world.terrain, aspect)
    } else if let Some([x, y, z, tx, ty, tz]) = args.look {
        let (eye, target) = (Vec3::new(x, y, z), Vec3::new(tx, ty, tz));
        glam::Mat4::perspective_infinite_reverse_rh(
            camera::DEBUG_FOV_Y_DEGREES.to_radians(),
            aspect,
            camera::NEAR,
        ) * glam::Mat4::look_at_rh(eye, target, Vec3::Z)
    } else {
        start_camera(loaded).view_proj(aspect)
    };
    let pixels = parkan_render::capture(&gpu, &mut renderer, (width, height), view_proj)?;
    let file = std::io::BufWriter::new(std::fs::File::create(out)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&pixels)?;
    println!("wrote {}", out.display());
    Ok(())
}

struct Running {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    renderer: Renderer,
}

struct App {
    loaded: scene::Loaded,
    world: scene::World,
    camera: FlyCamera,
    running: Option<Running>,
    held: HashSet<KeyCode>,
    looking: bool,
    last: Instant,
}

impl App {
    fn start(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let title = format!("parkan — {}", self.loaded.mission.map_name());
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title(title))?);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let gpu = pollster::block_on(Gpu::new(instance, Some(&surface)))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .context("the surface does not suit the adapter")?;
        let caps = surface.get_capabilities(&gpu.adapter);
        if let Some(srgb) = caps.formats.iter().copied().find(wgpu::TextureFormat::is_srgb) {
            config.format = srgb;
        }
        surface.configure(&gpu.device, &config);
        let mut renderer = Renderer::new(&gpu.device, config.format);
        let w = &self.world;
        renderer.set_world(&gpu.device, &gpu.queue, &w.store.textures, Some(&w.terrain), Some(&w.objects));
        self.running = Some(Running { window, surface, config, gpu, renderer });
        Ok(())
    }

    fn step(&mut self) {
        let dt = self.last.elapsed().as_secs_f32().min(0.1);
        self.last = Instant::now();
        let fast = self.held.contains(&KeyCode::ShiftLeft) || self.held.contains(&KeyCode::ShiftRight);
        let speed = if fast { 120.0 } else { 30.0 };
        let axis = |plus, minus| {
            f32::from(u8::from(self.held.contains(&plus))) - f32::from(u8::from(self.held.contains(&minus)))
        };
        let flat_forward = self.camera.forward().with_z(0.0).normalize_or_zero();
        let motion = flat_forward * axis(KeyCode::KeyW, KeyCode::KeyS)
            + self.camera.right() * axis(KeyCode::KeyD, KeyCode::KeyA)
            + Vec3::Z * axis(KeyCode::KeyE, KeyCode::KeyQ);
        self.camera.position += motion * speed * dt;
    }

    fn redraw(&mut self) {
        self.step();
        let Some(r) = self.running.as_mut() else { return };
        let frame = match r.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            _ => {
                r.surface.configure(&r.gpu.device, &r.config);
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let aspect = r.config.width as f32 / r.config.height.max(1) as f32;
        r.renderer.draw(
            &r.gpu.device,
            &r.gpu.queue,
            &view,
            (r.config.width, r.config.height),
            self.camera.view_proj(aspect),
        );
        r.gpu.queue.present(frame);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_none()
            && let Err(e) = self.start(event_loop)
        {
            eprintln!("cannot open the window: {e:#}");
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.running.as_mut() {
                    r.config.width = size.width.max(1);
                    r.config.height = size.height.max(1);
                    r.surface.configure(&r.gpu.device, &r.config);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape {
                        event_loop.exit();
                    }
                    match event.state {
                        ElementState::Pressed => self.held.insert(code),
                        ElementState::Released => self.held.remove(&code),
                    };
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Right, .. } => {
                self.looking = state == ElementState::Pressed;
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && self.looking
        {
            self.camera.turn(dx as f32 * 0.004, dy as f32 * 0.004);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(r) = self.running.as_ref() {
            r.window.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    let args = args()?;
    let game = gamedir::find(args.game.as_deref())
        .context("no Parkan install found: pass --game DIR or set PARKAN_DIR")?;
    let loaded = scene::load(&game, &args.mission)?;
    println!(
        "{}: map {}, {} objects, {} clans{}",
        args.mission,
        loaded.mission.map_name(),
        loaded.mission.objects.len(),
        loaded.mission.clans.len(),
        loaded.hero.map_or(String::new(), |(p, r)| format!(
            ", hero at ({:.1}, {:.1}, {:.1}) facing {r:.3}",
            p.x, p.y, p.z
        )),
    );
    if let Some(out) = &args.screenshot {
        return screenshot(&loaded, &game, &args, out);
    }
    let world = scene::world(&game, &loaded)?;
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = start_camera(&loaded);
    let mut app = App {
        loaded,
        world,
        camera,
        running: None,
        held: HashSet::new(),
        looking: false,
        last: Instant::now(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
