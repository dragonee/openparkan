//! `parkan`: Parkan: Iron Strategy, played from the install.
//!
//! Opens a mission in its hero's cockpit, or flies a debug camera over it.
//!
//! ```text
//! parkan [--game DIR] [--mission MISSIONS/…] [--fly]
//!        [--screenshot OUT.png] [--size WxH] [--top-down] [--look X,Y,Z,TX,TY,TZ]
//!        [--headless] [--ticks N] [--hold SCAN_W,SCAN_A] [--mouse DX,DY]
//! ```
//!
//! In the cockpit the hero's own input table drives it: W/S walk, A/D strafe,
//! the mouse turns the hull and tilts the turret, Shift and the mouse look
//! around. A click grabs the mouse; Escape lets it go, and quits when it is free.
//! With `--fly`: W/A/S/D and Q/E fly, the right mouse button held turns, Shift is
//! faster.
//!
//! `--ticks`, `--hold` and `--mouse` play the hero for that many 60 Hz ticks
//! holding those keys and moving the mouse by that many counts a tick, before a
//! `--screenshot` or, with `--headless`, printing where it got to.

mod audio;
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
use winit::window::{CursorGrabMode, Window, WindowId};

/// The simulation runs at a fixed 60 ticks a second.
const TICK_MS: f64 = 1000.0 / 60.0;

struct Args {
    game: Option<PathBuf>,
    mission: String,
    fly: bool,
    screenshot: Option<PathBuf>,
    size: (u32, u32),
    top_down: bool,
    /// `--look X,Y,Z,TX,TY,TZ`: a screenshot camera at X,Y,Z looking at TX,TY,TZ.
    look: Option<[f32; 6]>,
    headless: bool,
    ticks: u32,
    hold: Vec<String>,
    mouse: [f32; 2],
}

fn args() -> Result<Args> {
    let mut out = Args {
        game: None,
        mission: gamedir::MISSION_01.to_owned(),
        fly: false,
        screenshot: None,
        size: (1280, 720),
        top_down: false,
        look: None,
        headless: false,
        ticks: 0,
        hold: Vec::new(),
        mouse: [0.0; 2],
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--game" => out.game = Some(PathBuf::from(value()?)),
            "--mission" => out.mission = value()?,
            "--fly" => out.fly = true,
            "--screenshot" => out.screenshot = Some(PathBuf::from(value()?)),
            "--top-down" => out.top_down = true,
            "--headless" => out.headless = true,
            "--ticks" => out.ticks = value()?.parse()?,
            "--hold" => out.hold = value()?.split(',').map(str::to_owned).collect(),
            "--mouse" => {
                let v: Vec<f32> = value()?.split(',').map(str::parse).collect::<Result<_, _>>()?;
                out.mouse = v.try_into().map_err(|_| anyhow::anyhow!("--mouse takes two numbers"))?;
            }
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

/// Play the hero for `--ticks`, holding `--hold` and moving `--mouse`.
fn rehearse(play: &mut scene::Play, args: &Args) {
    for key in &args.hold {
        play.hero.key(key, true);
    }
    let mut kills = Vec::new();
    for tick in 0..args.ticks {
        for e in play.tick(TICK_MS, args.mouse) {
            if let parkan_sim::combat::Event::Killed { target } = e {
                kills.push(play.battle.objects[target]);
            }
        }
        if args.headless && (tick + 1) % 60 == 0 {
            report(play);
        }
    }
    if args.headless && !kills.is_empty() {
        println!("killed mission objects {kills:?}");
    }
}

fn report(play: &scene::Play) {
    let h = &play.hero;
    let b = &h.walker.body;
    let eye = h.eye();
    println!(
        "t {:6.2} s  at ({:.2}, {:.2}, {:.2})  speed {:5.2} m/s  heading {:+.3}  state {:3}  look ({:+.3}, {:+.3}, {:+.3})  rounds {}  targets alive {}",
        h.time_ms / 1000.0,
        b.position.x,
        b.position.y,
        b.position.z,
        Vec3::from_array(b.velocity).length(),
        b.heading(),
        h.walker.machine.current,
        eye.forward.x,
        eye.forward.y,
        eye.forward.z,
        play.battle.combat.rounds.len(),
        play.battle
            .combat
            .targets
            .iter()
            .filter(|t| t.alive && t.parts.iter().any(|p| p.life.is_some()))
            .count(),
    );
}

fn screenshot(loaded: &scene::Loaded, game: &Path, args: &Args, out: &Path) -> Result<()> {
    let (width, height) = args.size;
    let gpu = pollster::block_on(Gpu::headless())?;
    let mut world = scene::world(game, loaded)?;
    let mut play =
        if args.fly || args.top_down || args.look.is_some() { None } else { scene::play(game, loaded)? };
    if let Some(p) = play.as_mut() {
        scene::hide(&mut world.objects, p.hero.object);
        p.draw_rounds(&mut world.store, &mut world.objects)?;
        rehearse(p, args);
    }
    let mut renderer = Renderer::new(&gpu.device, parkan_render::CAPTURE_FORMAT);
    renderer.set_world(
        &gpu.device,
        &gpu.queue,
        &world.store.textures,
        Some(&world.terrain),
        Some(&world.objects),
    );
    if let Some(p) = play.as_ref() {
        renderer.set_sprite_looks(&gpu.device, &scene::sprite_looks(p));
    }
    if world.atmosphere.is_some() {
        renderer.set_dome(&gpu.device, &parkan_sim::sky::dome(), &parkan_sim::sky::dome_indices());
    }
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
    } else if let Some(p) = &play {
        camera::first_person(&p.hero.eye(), aspect)
    } else {
        start_camera(loaded).view_proj(aspect)
    };
    let (eye, forward, seconds) = match &play {
        Some(p) => {
            let e = p.hero.eye();
            (e.position, e.forward, p.hero.time_ms / 1000.0)
        }
        None => {
            let c = start_camera(loaded);
            (c.position, c.forward(), 0.0)
        }
    };
    if let Some((lighting, colours)) = scene::lighting(&world, seconds, eye, forward) {
        renderer.set_lighting(lighting);
        renderer.set_dome_colours(colours);
    }
    if let Some(p) = play.as_mut() {
        scene::sync(&mut renderer, &gpu.device, &gpu.queue, p, &world.objects, view_proj, eye);
        renderer.set_hud(&gpu.device, &gpu.queue, &scene::hud(p, aspect));
    }
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
    /// The hero's cockpit, or `None` to fly.
    play: Option<scene::Play>,
    camera: FlyCamera,
    running: Option<Running>,
    held: HashSet<KeyCode>,
    looking: bool,
    grabbed: bool,
    /// Mouse counts since the last tick.
    counts: [f32; 2],
    last: Instant,
    /// Real time not yet simulated, ms.
    owed: f64,
    audio: Option<audio::Audio>,
    started: Instant,
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
        if let Some(p) = self.play.as_ref() {
            renderer.set_sprite_looks(&gpu.device, &scene::sprite_looks(p));
        }
        if w.atmosphere.is_some() {
            renderer.set_dome(&gpu.device, &parkan_sim::sky::dome(), &parkan_sim::sky::dome_indices());
        }
        self.running = Some(Running { window, surface, config, gpu, renderer });
        Ok(())
    }

    fn grab(&mut self, on: bool) {
        let Some(r) = self.running.as_ref() else { return };
        let mode = if on { CursorGrabMode::Locked } else { CursorGrabMode::None };
        let ok = r.window.set_cursor_grab(mode).or_else(|_| {
            r.window.set_cursor_grab(if on { CursorGrabMode::Confined } else { CursorGrabMode::None })
        });
        r.window.set_cursor_visible(!on);
        self.grabbed = on && ok.is_ok();
    }

    fn step(&mut self) {
        let elapsed = self.last.elapsed().as_secs_f64() * 1000.0;
        self.last = Instant::now();
        if let Some(play) = self.play.as_mut() {
            self.owed = (self.owed + elapsed).min(250.0);
            while self.owed >= TICK_MS {
                play.tick(TICK_MS, self.counts);
                self.counts = [0.0; 2];
                self.owed -= TICK_MS;
            }
            return;
        }
        let dt = (elapsed as f32 / 1000.0).min(0.1);
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
        let (eye, forward, seconds) = match &self.play {
            Some(p) => {
                let e = p.hero.eye();
                (e.position, e.forward, p.hero.time_ms / 1000.0)
            }
            None => (self.camera.position, self.camera.forward(), self.started.elapsed().as_secs_f64()),
        };
        if let Some((lighting, colours)) = scene::lighting(&self.world, seconds, eye, forward) {
            r.renderer.set_lighting(lighting);
            r.renderer.set_dome_colours(colours);
        }
        let view_proj = match self.play.as_mut() {
            Some(play) => {
                let eye = play.hero.eye();
                let view_proj = camera::first_person(&eye, aspect);
                r.renderer.set_hud(&r.gpu.device, &r.gpu.queue, &scene::hud(play, aspect));
                if let Some(audio) = self.audio.as_mut() {
                    let right = eye.forward.cross(eye.up);
                    for cue in std::mem::take(&mut play.cues) {
                        audio.play(&cue, eye.position, right);
                    }
                }
                scene::sync(
                    &mut r.renderer,
                    &r.gpu.device,
                    &r.gpu.queue,
                    play,
                    &self.world.objects,
                    view_proj,
                    eye.position,
                );
                view_proj
            }
            None => self.camera.view_proj(aspect),
        };
        r.renderer.draw(&r.gpu.device, &r.gpu.queue, &view, (r.config.width, r.config.height), view_proj);
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
            WindowEvent::Focused(false) => self.grab(false),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                if code == KeyCode::Escape && event.state == ElementState::Pressed {
                    if self.grabbed {
                        self.grab(false)
                    } else {
                        event_loop.exit()
                    }
                    return;
                }
                let pressed = event.state == ElementState::Pressed;
                if let Some(play) = self.play.as_mut() {
                    if let Some(scan) = scene::scan_name(code)
                        && !event.repeat
                    {
                        play.hero.key(scan, pressed);
                    }
                } else if pressed {
                    self.held.insert(code);
                } else {
                    self.held.remove(&code);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if self.play.is_some() && pressed && !self.grabbed {
                    self.grab(true);
                    return;
                }
                if let Some(play) = self.play.as_mut() {
                    if let Some(scan) = scene::button_name(button) {
                        play.hero.key(scan, pressed);
                    }
                } else if button == MouseButton::Right {
                    self.looking = pressed;
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let DeviceEvent::MouseMotion { delta: (dx, dy) } = event else { return };
        if self.play.is_some() {
            if self.grabbed {
                self.counts[0] += dx as f32;
                self.counts[1] += dy as f32;
            }
        } else if self.looking {
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
    if args.headless {
        let mut play = scene::play(&game, &loaded)?.context("the mission has no hero to play")?;
        rehearse(&mut play, &args);
        report(&play);
        return Ok(());
    }
    if let Some(out) = &args.screenshot {
        return screenshot(&loaded, &game, &args, out);
    }
    let mut world = scene::world(&game, &loaded)?;
    let mut play = if args.fly { None } else { scene::play(&game, &loaded)? };
    if let Some(p) = play.as_mut() {
        scene::hide(&mut world.objects, p.hero.object);
        p.draw_rounds(&mut world.store, &mut world.objects)?;
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = start_camera(&loaded);
    let audio = if play.is_some() { audio::Audio::open(&game) } else { None };
    let mut app = App {
        loaded,
        world,
        play,
        camera,
        running: None,
        held: HashSet::new(),
        looking: false,
        grabbed: false,
        counts: [0.0; 2],
        last: Instant::now(),
        owed: 0.0,
        audio,
        started: Instant::now(),
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
