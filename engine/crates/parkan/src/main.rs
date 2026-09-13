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
}

fn args() -> Result<Args> {
    let mut out =
        Args { game: None, mission: gamedir::MISSION_01.to_owned(), screenshot: None, size: (1280, 720) };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--game" => out.game = Some(PathBuf::from(value()?)),
            "--mission" => out.mission = value()?,
            "--screenshot" => out.screenshot = Some(PathBuf::from(value()?)),
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

fn screenshot(loaded: &scene::Loaded, out: &Path, (width, height): (u32, u32)) -> Result<()> {
    let gpu = pollster::block_on(Gpu::headless())?;
    let camera = start_camera(loaded);
    let pixels = parkan_render::capture(
        &gpu,
        &loaded.scene,
        (width, height),
        camera.view_proj(width as f32 / height as f32),
    )?;
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
        renderer.set_scene(&gpu.device, &self.loaded.scene);
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
        return screenshot(&loaded, out, args.size);
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = start_camera(&loaded);
    let mut app =
        App { loaded, camera, running: None, held: HashSet::new(), looking: false, last: Instant::now() };
    event_loop.run_app(&mut app)?;
    Ok(())
}
