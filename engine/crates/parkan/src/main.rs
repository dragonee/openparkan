//! `parkan`: Parkan: Iron Strategy, played from the install.
//!
//! Opens a mission in its hero's cockpit, or flies a debug camera over it.
//!
//! ```text
//! parkan [--game DIR] [--mission MISSIONS/…] [--fly]
//!        [--screenshot OUT.png] [--size WxH] [--top-down] [--look X,Y,Z,TX,TY,TZ]
//!        [--headless] [--ticks N] [--hold SCAN_W,SCAN_A] [--press N] [--release N] [--mouse DX,DY] [--trace] [--sway]
//!        [--capture-idle] [--god-mode] [--stretch-hud] [--save-to-game] [--outcome won|lost] [--text "…"] [--face NAME,DISTANCE] [--at X,Y,YAW[,Z]] [--pod NAME] [--drive PATH] [--hq] [--take NAME] [--designer] [--design PART,…]
//!        [--skip-briefing] [--briefing-at SECONDS] [--objectives] [--map]
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
//! `--screenshot` or, with `--headless`, printing where it got to. In the window
//! `--hold` keeps those keys down and `--mouse` adds its counts every tick, and
//! `--trace` prints where the hero is every second, and the window's key, modifier and
//! focus events.
//!
//! The flags that say where the hero starts and what it is driving — `--at`, `--face`,
//! `--pod`, `--drive`, `--take`, `--hq`, `--build`, `--wingmen`, `--capture` — apply in all
//! three, the window included.
//!
//! Cmd frees the cursor and lets every key up, as leaving the window does, so a system
//! shortcut such as Cmd-Shift-4 leaves nothing held.
//!
//! The hero's view holds the heading it moves along; `--sway` lets it swing with the
//! gait, ten degrees each way on a run, as the game's does.
//!
//! `--god-mode` is a cheat of the engine's own: the hero walks 2.5 times as fast, has 10 times
//! the hit points and its rounds do 10 times the damage.
//!
//! A captured bot stands by until it is given an order; `--capture-idle` leaves it with
//! none, as the game's capture does, so it engages a hostile within 500 on its own.
//! The cockpit's HUD keeps the game's 640 × 480 layout round on a wide window, its corners
//! on the window's; `--stretch-hud` stretches it across as the game's does. F2 hides the
//! message box or shows it again. The warbot designer saves a design to the player's own
//! folder (`openparkan/units` in `~/Library/Application Support` on macOS); `--save-to-game`
//! writes it into the game's `units/`, as the game does.
//!
//! `--text` draws a string in the game font near the top of a `--screenshot`,
//! `--outcome won|lost` draws a screenshot's mission as won or lost, and `--face NAME,DISTANCE`
//! stands the hero that far from the mission object whose path ends in NAME, facing it, before
//! `--ticks` play.
//!
//! Once a mission is won or lost its panel takes the HUD's place and play goes on under
//! it: Esc leaves, and after a loss R restarts the mission.
//!
//! A campaign mission opens on its briefing: the camera flies its waypoints over the paused
//! world while its voices speak and its subtitles run, until the path ends or Esc skips it.
//! `--skip-briefing` starts in the cockpit at once, and `--briefing-at SECONDS` draws a
//! `--screenshot` of the briefing that far in.
//!
//! The objectives screen opens as the cockpit first shows and closes 7 s later; F12 opens and
//! closes it, and Esc closes it. M opens the satellite map, and ] and [ make it more or less
//! opaque. A `--screenshot` draws the cockpit with neither, unless `--objectives` or `--map`.

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

#[derive(Clone)]
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
    /// `--trace`: the window prints where the hero is every second of game time, and its key,
    /// modifier and focus events.
    trace: bool,
    /// `--sway`: the view swings with the hero's gait, as the game's does.
    sway: bool,
    /// `--capture-idle`: a captured bot is given no order, as the game's capture gives none.
    capture_idle: bool,
    /// `--god-mode`: the hero is 2.5× as fast, has 10× the hit points and does 10× the damage.
    god_mode: bool,
    /// `--fire-below`: a building's guns fire on a target below the lowest its turret looks, as
    /// the game's do.
    fire_below: bool,
    /// `--stretch-hud`: the HUD's layout stretches to the window, as the game's does.
    stretch_hud: bool,
    /// `--save-to-game`: the warbot designer saves into the game's `units/`, as the game does.
    save_to_game: bool,
    /// `--outcome won|lost`: a screenshot's mission is taken to have that outcome.
    outcome: Option<bool>,
    /// `--text`: a string a screenshot draws in the game font.
    text: Option<String>,
    /// `--face NAME,DISTANCE`: the hero starts that far from that object, facing it.
    face: Option<(String, f32)>,
    /// `--at X,Y,YAW[,Z]`: the hero starts on the ground at X,Y, turned to YAW; with Z, on the
    /// highest floor at or below Z, as in a building's buried rooms.
    at: Option<[f32; 4]>,
    /// `--pod NAME`: the hero starts on the control pod of the building whose path ends in NAME.
    pod: Option<String>,
    /// `--drive PATH`: a unit of that design is made beside the hero, and the hero boards it.
    drive: Option<String>,
    /// `--hq`: the hero takes and boards the mission's first HQ unit with Enter from 12 m, and
    /// Enter again opens its command view, before `--ticks` play.
    hq: bool,
    /// `--take NAME`: from the command view `--hq` or `--pod` opens, the player's unit whose path
    /// ends in NAME is taken over by telepresence, before `--ticks` play.
    take: Option<String>,
    /// `--designer`: a screenshot with the warbot designer open on the first factory, and
    /// `--design PART,…` the parts fitted to it in turn.
    designer: bool,
    design: Vec<String>,
    /// `--skip-briefing`: the mission starts in the cockpit, as Esc in its briefing would.
    skip_briefing: bool,
    /// `--briefing-at SECONDS`: a screenshot of the briefing that far in.
    briefing_at: Option<f64>,
    /// `--objectives`, `--map`: a screenshot with the objectives screen up, or the map open.
    objectives: bool,
    map: bool,
    /// `--page N`: a screenshot in command mode with the commander panel on page N.
    page: Option<u8>,
    /// `--ghost X,Y`: a screenshot in command mode with the first builder placing a mine at X,Y.
    ghost: Option<[f32; 2]>,
    /// `--camera-yaw RAD`: command mode's camera turned to that yaw for a screenshot.
    camera_yaw: Option<f32>,
    /// `--build X,Y`: the first builder of the player's clan is ordered to build a mine at X,Y
    /// before `--ticks` play.
    build: Option<[f32; 2]>,
    /// `--capture`: every unit of the player's clan that may capture is given Search and
    /// capture before `--ticks` play.
    capture: bool,
    /// `--wingmen`: the hero takes every neutral warbot with Enter from 12 m, one after the
    /// other, before `--ticks` play; `--tilde` then presses the tilde once `--ticks` have played.
    wingmen: bool,
    tilde: bool,
    /// `--outer N`: after `--ticks` play, C is pressed N times, half a second apart, so a
    /// screenshot shows the outer camera's Nth place (5 is back in the eye).
    outer: usize,
    /// `--zoom`: after `--ticks` (and `--outer`) play, Z is pressed and a quarter of a second
    /// plays, so a screenshot shows the view zoomed.
    zoom: bool,
    ticks: u32,
    hold: Vec<String>,
    /// `--press N` and `--release N`: the `--hold` keys go down after N of the `--ticks` (0 when
    /// not given) and come up after N.
    press: u32,
    release: Option<u32>,
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
        trace: false,
        sway: false,
        capture_idle: false,
        fire_below: false,
        god_mode: false,
        stretch_hud: false,
        save_to_game: false,
        outcome: None,
        text: None,
        face: None,
        at: None,
        pod: None,
        drive: None,
        hq: false,
        take: None,
        designer: false,
        design: Vec::new(),
        skip_briefing: false,
        briefing_at: None,
        objectives: false,
        map: false,
        page: None,
        ghost: None,
        camera_yaw: None,
        build: None,
        capture: false,
        wingmen: false,
        tilde: false,
        outer: 0,
        zoom: false,
        ticks: 0,
        hold: Vec::new(),
        press: 0,
        release: None,
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
            "--trace" => out.trace = true,
            "--sway" => out.sway = true,
            "--capture-idle" => out.capture_idle = true,
            "--fire-below" => out.fire_below = true,
            "--god-mode" => out.god_mode = true,
            "--stretch-hud" => out.stretch_hud = true,
            "--save-to-game" => out.save_to_game = true,
            "--outcome" => out.outcome = Some(value()? == "won"),
            "--text" => out.text = Some(value()?),
            "--skip-briefing" => out.skip_briefing = true,
            "--objectives" => out.objectives = true,
            "--map" => out.map = true,
            "--page" => out.page = Some(value()?.parse()?),
            "--camera-yaw" => out.camera_yaw = Some(value()?.parse()?),
            "--capture" => out.capture = true,
            "--wingmen" => out.wingmen = true,
            "--tilde" => out.tilde = true,
            "--outer" => out.outer = value()?.parse()?,
            "--zoom" => out.zoom = true,
            "--build" => {
                let v: Vec<f32> = value()?.split(',').map(str::parse).collect::<Result<_, _>>()?;
                out.build = v.get(..2).map(|v| [v[0], v[1]]);
            }
            "--ghost" => {
                let v: Vec<f32> = value()?.split(',').map(str::parse).collect::<Result<_, _>>()?;
                out.ghost = v.get(..2).map(|v| [v[0], v[1]]);
            }
            "--briefing-at" => out.briefing_at = Some(value()?.parse()?),
            "--face" => {
                let v = value()?;
                let (name, distance) = v.split_once(',').context("--face is NAME,DISTANCE")?;
                out.face = Some((name.to_ascii_lowercase(), distance.parse()?));
            }
            "--at" => {
                let mut v: Vec<f32> = value()?.split(',').map(str::parse).collect::<Result<_, _>>()?;
                if v.len() == 3 {
                    v.push(10_000.0);
                }
                out.at = Some(v.try_into().map_err(|_| anyhow::anyhow!("--at takes X,Y,YAW or X,Y,YAW,Z"))?);
            }
            "--pod" => out.pod = Some(value()?.to_ascii_lowercase()),
            "--drive" => out.drive = Some(value()?),
            "--hq" => out.hq = true,
            "--take" => out.take = Some(value()?.to_ascii_lowercase()),
            "--designer" => out.designer = true,
            "--design" => out.design = value()?.split(',').map(str::to_owned).collect(),
            "--ticks" => out.ticks = value()?.parse()?,
            "--hold" => out.hold = value()?.split(',').map(str::to_owned).collect(),
            "--press" => out.press = value()?.parse()?,
            "--release" => out.release = Some(value()?.parse()?),
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

/// The screen the HUD's layout is drawn on, stretched with `--stretch-hud`.
fn hud_space(width: u32, height: u32, args: &Args) -> parkan_world::hud::Space {
    parkan_world::hud::Space {
        stretch: args.stretch_hud,
        ..parkan_world::hud::Space::new(width as f32, height as f32)
    }
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

/// Play the hero for `--ticks`, holding `--hold` and moving `--mouse`, from `--face`.
fn rehearse(play: &mut scene::Play, loaded: &scene::Loaded, args: &Args) {
    if let Some((name, distance)) = &args.face {
        let target = play.battle.objects.iter().position(|&o| {
            loaded
                .mission
                .objects
                .get(o)
                .is_some_and(|m| m.path.to_ascii_lowercase().ends_with(name.as_str()))
        });
        match target {
            Some(t) if play.stand_facing(t, *distance, 0.0) => {}
            _ => eprintln!("--face: no place {distance} from an object {name}"),
        }
    }
    if let Some(name) = &args.pod {
        let target = play.battle.objects.iter().position(|&o| {
            loaded
                .mission
                .objects
                .get(o)
                .is_some_and(|m| m.path.to_ascii_lowercase().ends_with(name.as_str()))
        });
        if !target.is_some_and(|t| play.stand_on_pod(t)) {
            eprintln!("--pod: no building {name} with a pod");
        }
    }
    if let Some([x, y, yaw, top]) = args.at
        && !play.stand_below(x, y, top, yaw)
    {
        eprintln!("--at: no ground at {x}, {y}");
    }
    if let Some(path) = &args.drive {
        let data =
            parkan_formats::gamedir::resolve(&play.assembly.game, path).and_then(|p| std::fs::read(p).ok());
        let type_word = data
            .as_ref()
            .and_then(|d| d.get(4..8))
            .map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4])));
        let project = parkan_world::factory::Project {
            path: path.clone(),
            name: String::new(),
            type_word,
            chassis_size: 4,
            ore: 0.0,
            power: 0.0,
            lines: Vec::new(),
            sphere: None,
        };
        let at = play.hero.walker.body.position + Vec3::new(8.0, 0.0, 1.0);
        match play.spawn(&project, play.player_clan, at, play.hero.walker.body.yaw) {
            Some(t) => {
                play.tick(TICK_MS, [0.0; 2]);
                if !play.board(t) {
                    eprintln!("--drive: cannot board {path}");
                }
            }
            None => eprintln!("--drive: {path} is not a robot"),
        }
    }
    if args.hq && !take_hq(play) {
        eprintln!("--hq: no HQ unit the hero can take and board");
    }
    if let Some(name) = &args.take {
        let unit = play.robots.iter().map(|(t, _)| *t).find(|&t| {
            play.battle
                .objects
                .get(t)
                .and_then(|&o| loaded.mission.objects.get(o))
                .is_some_and(|o| o.path.to_ascii_lowercase().ends_with(name.as_str()))
        });
        if !unit.is_some_and(|t| play.telepresence(t, 0)) {
            eprintln!("--take: no unit ending in {name} can be taken over from a command view");
        }
    }
    if let Some([x, y]) = args.build {
        let builder = play.own_units_within(parkan_world::selection::BUILDERS).first().copied();
        let z = play.ground.below(x, y, 1.0e5).map_or(0.0, |h| h.point.z);
        let mine = parkan_sim::hq::BUILD_TYPES[0];
        if !builder.is_some_and(|b| play.order_build(b, mine, Vec3::new(x, y, z), 0.0)) {
            eprintln!("--build: no builder can build a mine at {x}, {y}");
        }
    }
    if args.wingmen && !take_neutrals(play) {
        eprintln!("--wingmen: no neutral warbot the hero can take");
    }
    if args.capture {
        let capturers: Vec<usize> = play
            .own_units_within(parkan_world::selection::BATTLE_UNITS | parkan_world::selection::BUILDERS)
            .into_iter()
            .filter(|&t| matches!(play.record_class(t), 1 | 2))
            .collect();
        if capturers.is_empty() {
            eprintln!("--capture: no unit of the player's clan may capture");
        }
        play.commander.units = capturers;
        play.hq_command(2);
    }
    let mut kills = Vec::new();
    for tick in 0..args.ticks {
        if tick == args.press {
            for key in &args.hold {
                play.key(key, true);
            }
        }
        if args.release == Some(tick) {
            for key in &args.hold {
                play.key(key, false);
            }
        }
        // Nothing is rendered here, so this run's fixed tick stands in for the frame the
        // game runs the input update on (docs/24); command mode's
        // camera once a tick on the hero's clock.
        play.update_input();
        play.command_frame(play.hero.time_ms / 1000.0, parkan_world::command::Edges::default());
        for e in play.tick(TICK_MS, args.mouse) {
            if let parkan_sim::combat::Event::Killed { target } = e
                && let Some(&object) = play.battle.objects.get(target)
            {
                kills.push(object);
            }
        }
        if args.headless {
            // With no window, what the game says is printed.
            for say in std::mem::take(&mut play.says) {
                if let parkan_world::progress::Say::Text(_, text) = say {
                    println!("  says: {text}");
                }
            }
            if (tick + 1) % 60 == 0 {
                report(play);
            }
        }
    }
    if args.headless && !kills.is_empty() {
        println!("killed mission objects {kills:?}");
    }
    // Keys pressed as the game runs them, each move played out.
    let press = |play: &mut scene::Play, command: &str, ticks: usize| {
        let eye = play.own_eye();
        let view = parkan_world::play::View {
            eye: eye.position,
            look: eye.forward,
            view_proj: glam::Mat4::IDENTITY,
            shift: false,
        };
        play.command(command, &view);
        for _ in 0..ticks {
            play.update_input();
            play.tick(TICK_MS, args.mouse);
        }
    };
    for _ in 0..args.outer {
        press(play, parkan_formats::controls::CMD_JAMES_OUTER_CAMERA, 30);
    }
    if args.zoom {
        press(play, parkan_formats::controls::CMD_JAMES_ZOOM_MODE, 15);
    }
    if args.tilde {
        press(play, parkan_formats::controls::CMD_JAMES_WINGMAN_MENU, 0);
    }
}

/// `--wingmen`: the hero stands 12 m from each neutral warbot in turn, targets it and presses
/// Enter, which captures it (docs/27); whether any was taken.
fn take_neutrals(play: &mut scene::Play) -> bool {
    let neutral: Vec<usize> = play
        .robots
        .iter()
        .map(|(t, _)| *t)
        .filter(|&t| play.units.get(t).is_some_and(|u| u.clan.is_some() && !play.thinks(u.clan)))
        .collect();
    let mut taken = false;
    for bot in neutral {
        if !play.stand_facing(bot, 12.0, 3.5) {
            continue;
        }
        play.tick(TICK_MS, [0.0; 2]);
        for _ in 0..=play.targets.listed.len() {
            if play.targets.current == Some(bot) {
                break;
            }
            play.targets.select_next();
        }
        taken |= play.targets.current == Some(bot) && play.enter();
    }
    taken
}

/// `--hq`: the hero stands 12 m from the first HQ unit, targets it and presses Enter, which
/// takes and boards it, then Enter again for its command view (docs/40, mode 3).
fn take_hq(play: &mut scene::Play) -> bool {
    let Some(hq) = (0..play.units.len()).find(|&t| play.is_hq(t)) else { return false };
    if !play.stand_facing(hq, 12.0, 0.0) {
        return false;
    }
    play.tick(TICK_MS, [0.0; 2]);
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(hq) {
            break;
        }
        play.targets.select_next();
    }
    for _ in 0..2 {
        let eye = play.eye();
        let view = parkan_world::play::View {
            eye: eye.position,
            look: eye.forward,
            view_proj: glam::Mat4::IDENTITY,
            shift: false,
        };
        play.command(parkan_formats::controls::CMD_ENTER_STATE, &view);
    }
    play.mode() == parkan_world::play::Mode::HqCommand(hq)
}

fn report(play: &scene::Play) {
    let h = &play.hero;
    let b = &h.walker.body;
    let eye = h.eye();
    println!(
        "t {:6.2} s  at ({:.2}, {:.2}, {:.2})  speed {:5.2} m/s  heading {:+.3}  state {:3}  look ({:+.3}, {:+.3}, {:+.3})  rounds {}  targets alive {}  target {:?}  objectives {:?}",
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
        play.targets.current.map(|t| play.battle.objects[t]),
        play.progression.as_ref().map(|p| p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>()),
    );
}

fn screenshot(loaded: &scene::Loaded, game: &Path, args: &Args, out: &Path) -> Result<()> {
    let (width, height) = args.size;
    let gpu = pollster::block_on(Gpu::headless())?;
    let mut world = scene::world(game, loaded)?;
    let mut play = if args.fly || args.top_down { None } else { scene::play(game, loaded, args)? };
    let mut view = None;
    let mut briefing = None;
    if let Some(p) = play.as_mut() {
        view = Some(scene::own_view(&mut world.objects, &mut world.store, p)?);
        p.draw_rounds(&mut world.store, &mut world.objects)?;
        if let Some(at) = args.briefing_at {
            // The briefing played to `at` a frame a tick, the world paused under it.
            let mut b = parkan_world::briefing::Briefing::open(game, &loaded.dir)?
                .context("--briefing-at: the mission has no briefing")?;
            p.paused = true;
            let mut t = 0.0;
            while t <= at && !b.finished() {
                b.frame(t);
                p.tick(TICK_MS, [0.0; 2]);
                t += TICK_MS / 1000.0;
            }
            briefing = Some(b);
        } else {
            rehearse(p, loaded, args);
            if let Some(yaw) = args.camera_yaw {
                p.command.yaw = yaw;
            }
        }
        if let (Some(outcome), Some(progression)) = (args.outcome, p.progression.as_mut()) {
            progression.progress.outcome = Some(outcome);
        }
    }
    // What the rehearsal made, and a boarded bot's cockpit, are drawn too.
    if let (Some(p), Some(v)) = (play.as_mut(), view.as_mut())
        && scene::add_targets(&mut world.objects, &mut world.store, v, p)?
    {
        p.draw_rounds(&mut world.store, &mut world.objects)?;
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
    } else if let Some(b) = &briefing {
        camera::first_person(&b.eye(), aspect)
    } else if let Some(p) = &play {
        camera::first_person(&p.eye(), aspect)
    } else {
        start_camera(loaded).view_proj(aspect)
    };
    // Where the fog is measured from and the flare gate looks along: the camera drawn, and
    // the field of view a portal quad's fade is scaled by.
    let (eye, forward, field, seconds) = match (&play, args.look) {
        (_, Some([x, y, z, tx, ty, tz])) => {
            let (eye, target) = (Vec3::new(x, y, z), Vec3::new(tx, ty, tz));
            (
                eye,
                (target - eye).normalize_or(Vec3::Y),
                camera::debug_field(aspect),
                play.as_ref().map_or(0.0, |p| p.hero.time_ms / 1000.0),
            )
        }
        (Some(p), None) => {
            let e = briefing.as_ref().map_or_else(|| p.eye(), |b| b.eye());
            (e.position, e.forward, e.fov_x, p.hero.time_ms / 1000.0)
        }
        (None, None) => {
            let c = start_camera(loaded);
            (c.position, c.forward(), camera::debug_field(aspect), 0.0)
        }
    };
    if let Some((lighting, colours)) = scene::lighting(&world, seconds, eye, forward, field) {
        renderer.set_lighting(lighting);
        renderer.set_dome_colours(colours);
        renderer.set_sky_layers(scene::sky_layers(&world, seconds));
        renderer.set_body_sprites(&gpu.device, scene::body_sprites(&world, seconds));
        renderer.set_flare(&gpu.device, scene::flare(&world, seconds));
    }
    if let Some(p) = play.as_mut() {
        scene::sync(&mut renderer, &gpu.device, &gpu.queue, p, &world.objects, view_proj, eye);
        if let Some(v) = &view {
            let outside = briefing.is_some() || p.mode().shows_cursor() || p.outer_shows();
            scene::place_own_view(&mut renderer, &gpu.queue, v, p, outside);
        }
        scene::panel_fonts(&mut renderer, &gpu.device, &gpu.queue, game);
        let outcome =
            scene::draw_outcome(&mut renderer, &gpu.device, &gpu.queue, p, hud_space(width, height, args));
        if let Some(b) = &briefing {
            scene::draw_briefing(&mut renderer, &gpu.device, &gpu.queue, b, hud_space(width, height, args));
        }
        match scene::hud(&mut renderer, &gpu.device, &gpu.queue, game, &loaded.dir, p, args) {
            Ok(mut hud) => {
                if args.objectives {
                    hud.cockpit.objectives.open_at_start();
                }
                hud.cockpit.map.open = args.map;
                if let Some(page) = args.page {
                    hud.cockpit.update(p, p.hero.time_ms);
                    hud.cockpit.commander.turn(p, page, p.hero.time_ms);
                }
                if let Some([x, y]) = args.ghost
                    && let Some(&builder) = p.own_units_within(parkan_world::selection::BUILDERS).first()
                {
                    p.select_unit_alone(builder);
                    p.open_pick(parkan_sim::hq::Act::Build(parkan_sim::hq::BUILD_TYPES[0]));
                    let z = p.ground.below(x, y, 1.0e5).map_or(0.0, |h| h.point.z);
                    let eye = p.eye().position;
                    let direction = (Vec3::new(x, y, z) - eye).normalize();
                    p.update_ghost(parkan_world::pick::Aim::Ray { eye, direction });
                    hud.cockpit.commander.cursor_state = 8;
                }
                if (args.designer || !args.design.is_empty())
                    && let Some(t) = p.factories.first().map(|f| f.target)
                {
                    if p.mode() == parkan_world::play::Mode::OnFoot {
                        p.modes.push(parkan_world::play::Mode::Factory(t));
                    }
                    let cockpit = &mut hud.cockpit;
                    // A screenshot's saves stay in memory, out of every folder.
                    cockpit.designer.saves = None;
                    match cockpit.designer.open(p, t, &cockpit.strings) {
                        Ok(()) => {
                            for part in &args.design {
                                // `accept`, `save` and `load` click their buttons; `type=TEXT`
                                // types into the name field, `\r` for Enter.
                                let button = match part.as_str() {
                                    "accept" => Some([213.0, 462.0]),
                                    "save" => Some([290.0, 462.0]),
                                    "load" => Some([320.0, 462.0]),
                                    _ => None,
                                };
                                if let Some(at) = button {
                                    cockpit.designer.click(p, at, &cockpit.strings);
                                    continue;
                                }
                                if let Some(text) = part.strip_prefix("type=") {
                                    for c in text.replace("\\r", "\r").chars() {
                                        cockpit.designer.key(c, &hud.font);
                                    }
                                    continue;
                                }
                                let Some(s) = cockpit.designer.session.as_mut() else { break };
                                if !s.fit_part(part, &mut p.assembly, &cockpit.strings) {
                                    eprintln!("--design: {part} does not fit");
                                }
                            }
                        }
                        Err(e) => eprintln!("--designer: {e:#}"),
                    }
                }
                // What the game said during the rehearsal: the newest line is in the box.
                let now = p.hero.time_ms;
                for say in std::mem::take(&mut p.says) {
                    if let parkan_world::progress::Say::Text(sender, text) = say {
                        hud.cockpit.messages.show(sender, text, now);
                    }
                }
                let lighting = scene::lighting(&world, seconds, eye, forward, field).map(|l| l.0);
                if let Some(v) = &view {
                    scene::draw_hud(
                        &mut renderer,
                        &gpu.device,
                        &gpu.queue,
                        &mut hud,
                        p,
                        v,
                        (width, height),
                        view_proj,
                        lighting,
                        !outcome && briefing.is_none(),
                    );
                }
            }
            Err(e) => eprintln!("no cockpit HUD: {e:#}"),
        }
    }
    if let Some(text) = &args.text {
        use parkan_world::text::{Align, GameFont, TextRun};
        renderer.set_font(&gpu.device, &gpu.queue, GameFont::open(game)?);
        let run = TextRun {
            align: Align::Centre,
            wrap: Some(width as f32 * 0.6),
            ..TextRun::new(text, [0.0, 0.8])
        };
        renderer.set_text(&gpu.device, &gpu.queue, &[run]);
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
    /// What the hero's own view draws of it.
    view: Option<scene::OwnView>,
    camera: FlyCamera,
    running: Option<Running>,
    held: HashSet<KeyCode>,
    looking: bool,
    grabbed: bool,
    /// Mouse counts since the last tick.
    counts: [f32; 2],
    /// `--mouse`: counts added every tick.
    mouse: [f32; 2],
    trace: bool,
    last: Instant,
    /// Real time not yet simulated, ms.
    owed: f64,
    audio: Option<audio::Audio>,
    /// The CD's music, as `iron3d.dll`'s CD player drives it (docs/34, "Music").
    cd: Option<parkan_world::music::CdPlayer>,
    /// The mission's ambient variations and when the next one is due (docs/34, "Ambient
    /// sound").
    ambience: Option<parkan_world::resources::Ambience>,
    started: Instant,
    game: PathBuf,
    /// The game's own key chords, and the scan names held down.
    bindings: Vec<parkan_formats::controls::Binding>,
    scans: HashSet<&'static str>,
    /// The cockpit's HUD, once the window has a renderer to load it into.
    hud: Option<scene::Hud>,
    /// What the window was opened with, for a restart.
    args: Args,
    /// The mission's briefing while it plays, and when its first frame was drawn.
    briefing: Option<parkan_world::briefing::Briefing>,
    briefing_clock: Option<Instant>,
    /// Where the cursor is in the window, in pixels, and whether it is over the window.
    cursor: [f32; 2],
    cursor_in: bool,
    /// Cmd is down: the keys pressed now are a system shortcut's.
    shortcut: bool,
    /// In command mode: when the left button went down and where, for a band, and whether the
    /// system's cursor is hidden for the software one.
    left_down: Option<(Instant, [f32; 2])>,
    cursor_hidden: bool,
}

/// The mission's briefing, unless `--skip-briefing` or `--fly`; a play has its world paused
/// while one runs.
fn open_briefing(
    game: &Path,
    loaded: &scene::Loaded,
    args: &Args,
    play: Option<&mut scene::Play>,
) -> Option<parkan_world::briefing::Briefing> {
    let play = play?;
    if args.skip_briefing {
        return None;
    }
    let briefing = parkan_world::briefing::Briefing::open(game, &loaded.dir)
        .map_err(|e| eprintln!("no briefing: {e:#}"))
        .ok()
        .flatten()?;
    play.paused = true;
    Some(briefing)
}

/// A CD player over the install's tracks, `rand()` seeded from the clock as the shell's
/// `srand(timeGetTime())` seeds it (`iron3d.dll:0x1000798f`).
fn cd_player(game: &Path) -> parkan_world::music::CdPlayer {
    let clock =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    parkan_world::music::CdPlayer::open(game, clock as u32)
}

/// A briefing's start stops the CD and plays its `cd_track`, looping (`iron3d.dll:0x10031431`,
/// `0x10031500`).
fn briefing_music(
    cd: &mut Option<parkan_world::music::CdPlayer>,
    audio: &mut Option<audio::Audio>,
    loaded: &scene::Loaded,
    now_ms: f64,
) {
    let Some(cd) = cd.as_mut() else { return };
    let mut commands = vec![cd.stop()];
    commands.extend(parkan_world::music::briefing_track(&loaded.dir).and_then(|t| cd.play(t, true)));
    for command in commands {
        let length = audio.as_mut().and_then(|a| a.cd(&command));
        if matches!(command, parkan_world::music::Cd::Play { .. }) {
            cd.started(now_ms, length.unwrap_or(0.0));
        }
    }
}

/// The mission's theme, looping (docs/34, "Ambient sound").
fn theme(audio: &mut Option<audio::Audio>, game: &Path, loaded: &scene::Loaded) {
    if let Some(a) = audio.as_mut()
        && let Ok(ambient) = parkan_world::resources::ambient(game, &loaded.dir)
        && let Some(theme) = ambient.theme
    {
        a.theme(&theme);
    }
}

/// The mission's ambient variations, due from its first frame on (docs/34, "Ambient
/// sound"). The names are gathered in the mission's set-up, before any briefing
/// (`iron3d.dll:0x1005f650`).
fn ambience(game: &Path, loaded: &scene::Loaded) -> Option<parkan_world::resources::Ambience> {
    let ambient = parkan_world::resources::ambient(game, &loaded.dir).ok()?;
    // The wait is drawn from the same CRT `rand()` the CD's track picker uses, seeded once
    // from `timeGetTime()` (`iron3d.dll:0x1000798f`).
    let clock =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    // STAND-IN: docs/34-progression.md#ambient-sound--read-and-measured -- where the
    // picker's own two words start is not read: 0xace1 and 0x1234, as the acknowledgement
    // voices' generator starts.
    Some(parkan_world::resources::Ambience::new(ambient, clock as u32, (0xace1, 0x1234)))
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
        // The HUD draws into the same frame read without sRGB decoding.
        config.view_formats = vec![config.format.remove_srgb_suffix()];
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
        match parkan_world::text::GameFont::open(&self.game) {
            Ok(font) => renderer.set_font(&gpu.device, &gpu.queue, font),
            Err(e) => eprintln!("no game font: {e:#}"),
        }
        scene::panel_fonts(&mut renderer, &gpu.device, &gpu.queue, &self.game);
        if let Some(p) = self.play.as_ref() {
            match scene::hud(
                &mut renderer,
                &gpu.device,
                &gpu.queue,
                &self.game,
                &self.loaded.dir,
                p,
                &self.args,
            ) {
                Ok(mut hud) => {
                    // A mission started fresh opens its objectives screen (`0x1005e117`).
                    hud.cockpit.objectives.open_at_start();
                    self.hud = Some(hud);
                }
                Err(e) => eprintln!("no cockpit HUD: {e:#}"),
            }
        }
        self.running = Some(Running { window, surface, config, gpu, renderer });
        Ok(())
    }

    /// The mission again from its start, as the executable runs the game again with the same
    /// parameters on exit code 2 (docs/34, "After the outcome").
    fn restart(&mut self) -> Result<()> {
        let mut world = scene::world(&self.game, &self.loaded)?;
        let mut play =
            scene::play(&self.game, &self.loaded, &self.args)?.context("the mission has no hero")?;
        let view = scene::own_view(&mut world.objects, &mut world.store, &play)?;
        play.draw_rounds(&mut world.store, &mut world.objects)?;
        if let Some(r) = self.running.as_mut() {
            let (d, q) = (&r.gpu.device, &r.gpu.queue);
            r.renderer.set_world(d, q, &world.store.textures, Some(&world.terrain), Some(&world.objects));
            r.renderer.set_sprite_looks(d, &scene::sprite_looks(&play));
            self.hud = scene::hud(&mut r.renderer, d, q, &self.game, &self.loaded.dir, &play, &self.args)
                .map_err(|e| eprintln!("no cockpit HUD: {e:#}"))
                .ok();
            if let Some(hud) = self.hud.as_mut() {
                hud.cockpit.objectives.open_at_start();
            }
        }
        self.audio = audio::Audio::open(&self.game);
        self.cd = Some(cd_player(&self.game));
        if let Some(r) = self.running.as_mut() {
            scene::clear_briefing(&mut r.renderer, &r.gpu.device, &r.gpu.queue);
        }
        self.briefing = open_briefing(&self.game, &self.loaded, &self.args, Some(&mut play));
        self.briefing_clock = None;
        self.ambience = ambience(&self.game, &self.loaded);
        if self.briefing.is_none() {
            theme(&mut self.audio, &self.game, &self.loaded);
        } else {
            let now = self.started.elapsed().as_secs_f64() * 1000.0;
            briefing_music(&mut self.cd, &mut self.audio, &self.loaded, now);
        }
        self.world = world;
        self.play = Some(play);
        self.view = Some(view);
        self.scans.clear();
        self.owed = 0.0;
        Ok(())
    }

    /// A key the system repeats while it is held. The game takes a repeat as the key going down
    /// again: nothing on a key-down's way from the window to the command handler asks the key's
    /// previous state (`iron3d.dll:0x100a0e30`, `0x10071c10`, `World3D.dll:0x10011330`), so a
    /// held `,` or `.` goes on turning the building being placed (docs/32, "Turning it"). The
    /// handler would take any command's repeat; here the two turns alone take one.
    fn repeat(&mut self, scan: &'static str) {
        use parkan_formats::controls::{CMD_JAMES_BASE_ROTLEFT, CMD_JAMES_BASE_ROTRIGHT, command_for};
        if self.briefing.is_some() {
            return;
        }
        let Some(play) = self.play.as_mut() else { return };
        if let Some(command) = command_for(&self.bindings, scan, |m| self.scans.contains(m))
            && matches!(command, CMD_JAMES_BASE_ROTLEFT | CMD_JAMES_BASE_ROTRIGHT)
        {
            play.command_key(command, true);
        }
    }

    /// A key or button went down or up: the hero's table takes it, and a press runs the
    /// game's command its chord binds (`ui_other.man`).
    fn scan(&mut self, scan: &'static str, pressed: bool) {
        // STAND-IN: docs/21-briefing.md#not-established -- which keys act while a briefing
        // plays is not read beyond Esc: none reaches the hero or the game's commands.
        if self.briefing.is_some() {
            return;
        }
        let Some(play) = self.play.as_mut() else { return };
        // A building's screen has the hero handed away: its table takes no key
        // (`0x10074ff0` with 0, docs/36), and only the screens' commands act.
        // The player drives the hero on foot, or the bot it boarded.
        let on_foot =
            matches!(play.mode(), parkan_world::play::Mode::OnFoot | parkan_world::play::Mode::Driving(_));
        // While the wingman selector is open a digit is its (`iron3d.dll:0x100710fa`).
        let digit = scan
            .strip_prefix("SCAN_W_")
            .and_then(|d| d.parse::<usize>().ok())
            .filter(|d| (1..=9).contains(d));
        // Command mode's keys act on the way down and up (`0x10071cd0`, `0x10072740`).
        if play.mode().commands()
            && let Some(command) =
                parkan_formats::controls::command_for(&self.bindings, scan, |m| self.scans.contains(m))
            && play.command_key(command, pressed)
        {
            if pressed {
                self.scans.insert(scan);
            } else {
                self.scans.remove(scan);
            }
            return;
        }
        let selecting = play.selector.state != parkan_sim::orders::State::Off;
        match digit {
            Some(n) if selecting => {
                if pressed {
                    play.wingman_digit(n);
                }
            }
            _ if on_foot => play.key(scan, pressed),
            _ => {}
        }
        if !pressed {
            self.scans.remove(scan);
            return;
        }
        self.scans.insert(scan);
        let Some(command) =
            parkan_formats::controls::command_for(&self.bindings, scan, |m| self.scans.contains(m))
        else {
            return;
        };
        let aspect = self
            .running
            .as_ref()
            .map_or(16.0 / 9.0, |r| r.config.width as f32 / r.config.height.max(1) as f32);
        // The right button picks along the unit's own camera, whichever view is drawn.
        let eye = play.own_eye();
        let view = parkan_world::play::View {
            eye: eye.position,
            look: eye.forward,
            view_proj: camera::first_person(&eye, aspect),
            shift: self.scans.contains("SCAN_LSHIFT") || self.scans.contains("SCAN_RSHIFT"),
        };
        let command = command.to_owned();
        {
            use parkan_formats::controls::*;
            if let Some(hud) = self.hud.as_mut() {
                let now = play.hero.time_ms;
                let screens = &mut hud.cockpit;
                match command.as_str() {
                    CMD_PAGER => screens.messages.pager(),
                    CMD_JAMES_MISSION_OBJ => screens.objectives.toggle(),
                    CMD_JAMES_SATELLITE_MAP => screens.map.toggle(),
                    CMD_INC_MAP_ALPHA => screens.map.step_alpha(true, now),
                    CMD_DEC_MAP_ALPHA => screens.map.step_alpha(false, now),
                    // `0x1007255b`: no wingman menu while the objectives are up.
                    CMD_JAMES_WINGMAN_MENU if screens.objectives.up => {}
                    _ if !on_foot => {}
                    _ => {
                        play.command(&command, &view);
                    }
                }
                return;
            }
        }
        play.command(&command, &view);
    }

    /// The briefing is over, run out or skipped (`iron3d.dll:0x1005e7a4`): the objects go on,
    /// the theme starts, and the cockpit takes the view.
    fn end_briefing(&mut self) {
        self.briefing = None;
        self.briefing_clock = None;
        if let Some(play) = self.play.as_mut() {
            play.paused = false;
        }
        if let Some(r) = self.running.as_mut() {
            scene::clear_briefing(&mut r.renderer, &r.gpu.device, &r.gpu.queue);
        }
        theme(&mut self.audio, &self.game, &self.loaded);
    }

    /// Every key and button held comes up: the unit the player drives lets them go, and no
    /// chord or fly key stays down.
    fn release_keys(&mut self) {
        self.scans.clear();
        self.held.clear();
        self.looking = false;
        if let Some(play) = self.play.as_mut() {
            play.release_keys();
        }
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

    /// A mouse button in command mode (docs/42, "The mouse's way in").
    fn command_mouse(&mut self, button: MouseButton, pressed: bool) {
        use parkan_world::cockpit::commander::{Click, MAP_PANEL};
        use parkan_world::hud::Pin;
        use parkan_world::pick::{BAND_LEAST_PIXELS, BandSpace};
        let aim = self.command_aim();
        let Some(r) = self.running.as_ref() else { return };
        let space = hud_space(r.config.width, r.config.height, &self.args);
        let (w, h) = (r.config.width as f32, r.config.height as f32);
        let (left, right) =
            (space.layout(self.cursor, Pin::TOP_LEFT), space.layout(self.cursor, Pin::TOP_RIGHT));
        let cursor = self.cursor;
        let (Some(play), Some(hud)) = (self.play.as_mut(), self.hud.as_mut()) else { return };
        let now = play.hero.time_ms;
        let cockpit = &mut hud.cockpit;
        // The warbot designer, while it is up and the mission is played, takes the click
        // (`0x10055ff0`).
        if cockpit.designer.takes_input(play) {
            if pressed && button == MouseButton::Left {
                let at = parkan_world::cockpit::designer::layout_point(space, cursor);
                cockpit.designer.click(play, at, &cockpit.strings);
            }
            return;
        }
        match (button, pressed) {
            (MouseButton::Left, true) => {
                self.left_down = Some((Instant::now(), cursor));
                if play.commander.ghost.is_some() {
                    play.commit_placement();
                    return;
                }
                let pick = play.pick(aim);
                match cockpit.commander.click(play, &mut cockpit.map, left, right, now) {
                    Click::Factory(t, parkan_world::cockpit::factory::Click::Constructor) => {
                        // The rest of this press is the designer's. Left down, it would grow a
                        // band while the designer is up, and the band let go over the designer's
                        // exit would select across the screen and turn the factory's page to 0.
                        self.left_down = None;
                        if let Err(e) = cockpit.designer.open(play, t, &cockpit.strings) {
                            eprintln!("cannot open the designer: {e:#}");
                        }
                    }
                    Click::Factory(t, click) => play.factory_click(t, click),
                    Click::Pick(act) => {
                        if play.open_pick(act) && !cockpit.map.open {
                            cockpit.map.toggle();
                        }
                    }
                    Click::Taken => {}
                    Click::World => {
                        if let Some((page, always)) = play.click_world(pick)
                            && (always || cockpit.commander.page != 0)
                        {
                            cockpit.commander.turn(play, page, now);
                        }
                    }
                }
            }
            (MouseButton::Left, false) => {
                let Some((_, anchor)) = self.left_down.take() else { return };
                if cockpit.commander.band.take().is_none() {
                    return;
                }
                let (dx, dy) = ((cursor[0] - anchor[0]).abs(), (cursor[1] - anchor[1]).abs());
                if dx < BAND_LEAST_PIXELS || dy < BAND_LEAST_PIXELS {
                    return;
                }
                let [x0, y0, x1, y1] = MAP_PANEL;
                let on_map = |p: [f32; 2]| space.layout(p, Pin::TOP_RIGHT);
                let a = on_map(anchor);
                if cockpit.map.open && (x0..=x1).contains(&a[0]) && (y0..=y1).contains(&a[1]) {
                    let l = play.ground.world_box().1.truncate().max_element();
                    let world = |p: [f32; 2]| {
                        let q = on_map(p);
                        [(q[0] - (x0 + 5.0)) * l / 256.0, ((y1 - 5.0) - q[1]) * l / 256.0]
                    };
                    play.band_select([world(anchor), world(cursor)], BandSpace::Map);
                } else {
                    let eye = play.eye();
                    let view_proj = camera::first_person(&eye, w / h.max(1.0));
                    play.band_select([anchor, cursor], BandSpace::World(view_proj, [w, h]));
                }
                cockpit.commander.page = 0;
            }
            (MouseButton::Right, true) => {
                if cockpit.commander.band.take().is_some() {
                    self.left_down = None;
                    return;
                }
                let undo = play.right_click(cockpit.commander.page, cockpit.map.open);
                if undo.page_zero {
                    cockpit.commander.page = 0;
                }
                if undo.close_map {
                    cockpit.map.toggle();
                }
            }
            _ => {}
        }
    }

    /// Where the cursor points in command mode (docs/42, "The pick under the cursor"): nothing
    /// over the panel, a world place over the open satellite map, else the command camera's
    /// ray.
    fn command_aim(&self) -> parkan_world::pick::Aim {
        use parkan_world::cockpit::commander::MAP_PANEL;
        use parkan_world::hud::Pin;
        use parkan_world::pick::Aim;
        let (Some(play), Some(r), Some(hud)) = (self.play.as_ref(), self.running.as_ref(), self.hud.as_ref())
        else {
            return Aim::Nothing;
        };
        if !self.cursor_in {
            return Aim::Nothing;
        }
        let space = hud_space(r.config.width, r.config.height, &self.args);
        let (left, right) =
            (space.layout(self.cursor, Pin::TOP_LEFT), space.layout(self.cursor, Pin::TOP_RIGHT));
        let cockpit = &hud.cockpit;
        if play.commander.ghost.is_none() && cockpit.commander.hit(play, cockpit.map.open, left, right) {
            return Aim::Nothing;
        }
        if cockpit.map.open && play.commander.ghost.is_none() {
            let [x0, y0, x1, y1] = MAP_PANEL;
            if (x0..=x1).contains(&right[0]) && (y0..=y1).contains(&right[1]) {
                let l = play.ground.world_box().1.truncate().max_element();
                let (u, v) = (right[0] - (x0 + 5.0), (y1 - 5.0) - right[1]);
                return Aim::Map([u * l / 256.0, v * l / 256.0]);
            }
        }
        let (w, h) = (r.config.width as f32, r.config.height as f32);
        let eye = play.eye();
        parkan_world::pick::ray(camera::first_person(&eye, w / h.max(1.0)), eye.position, self.cursor, [w, h])
    }

    /// The commander's cursor this frame: the ghost follows it, and the pick under it picks
    /// the cursor's state (`0x10058710`).
    fn command_cursor(&mut self) {
        // Command mode, a building's screen and the designer over either show the cursor
        // (`0x1008d51c`, `0x1008d4f1`, `0x1008d457`), and the cockpit draws it in software in
        // place of the system's (docs/42, "The cursor shows a state").
        let shown = self.play.as_ref().is_some_and(|p| p.mode().shows_cursor());
        if let Some(r) = self.running.as_ref()
            && shown != self.cursor_hidden
        {
            r.window.set_cursor_visible(!shown);
            self.cursor_hidden = shown;
        }
        // Over the designer the pick answers nothing, and the cursor is `ARROW` (`0x1008dae5`).
        let designer = self.hud.as_ref().is_some_and(|h| h.cockpit.designer.is_open());
        let command = !designer && self.play.as_ref().is_some_and(|p| p.mode().commands());
        if !command {
            return;
        }
        let aim = self.command_aim();
        let (Some(play), Some(hud)) = (self.play.as_mut(), self.hud.as_mut()) else { return };
        play.update_ghost(aim);
        let pick = play.pick(aim);
        // The line under the commander's map names the pick's object when the pick came from
        // the map (the cursor object's `+0x1c`, docs/35, "The commander's variant").
        hud.cockpit.map.pointed = match aim {
            parkan_world::pick::Aim::Map(_) => pick.object,
            _ => None,
        };
        let panel = &mut hud.cockpit.commander;
        // The markers are never drawn over the hero, which the pick can take (docs/42).
        panel.hovered = pick.object.filter(|&t| !play.is_hero(t));
        panel.cursor_state = if play.commander.ghost.is_some() {
            8
        } else if panel.band.is_some() {
            7
        } else {
            parkan_world::pick::cursor_state(pick.kind)
        };
    }

    fn step(&mut self) {
        let elapsed = self.last.elapsed().as_secs_f64() * 1000.0;
        self.last = Instant::now();
        if let Some(play) = self.play.as_mut() {
            self.owed = (self.owed + elapsed).min(250.0);
            while self.owed >= TICK_MS {
                let counts = [self.counts[0] + self.mouse[0], self.counts[1] + self.mouse[1]];
                play.tick(TICK_MS, counts);
                if self.trace && ((play.hero.time_ms / TICK_MS).round() as u64).is_multiple_of(60) {
                    report(play);
                }
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
        // A building's screen shows the cursor, as command mode does (docs/36, "The cursor in
        // mode 5"): the grab is let go, and the cockpit draws the cursor.
        if self.grabbed && self.play.as_ref().is_some_and(|p| p.mode().shows_cursor()) {
            self.grab(false);
        }
        // Command mode's camera runs each frame in real seconds, turned by the cursor at an
        // edge of the screen (docs/40, "The cursor at an edge turns and tilts it").
        if let (Some(play), Some(r)) = (self.play.as_mut(), self.running.as_ref()) {
            let space = hud_space(r.config.width, r.config.height, &self.args);
            let designer = self.hud.as_ref().is_some_and(|h| h.cockpit.designer.is_open());
            let edges = match self.cursor_in && !designer {
                true => parkan_world::command::Edges::of(
                    space.layout(self.cursor, parkan_world::hud::Pin::TOP_LEFT),
                    space.layout(self.cursor, parkan_world::hud::Pin::BOTTOM_RIGHT),
                ),
                false => parkan_world::command::Edges::default(),
            };
            play.command_frame(self.started.elapsed().as_secs_f64(), edges);
        }
        self.command_cursor();
        // The designer goes with the factory screen or command mode it was opened from.
        if let (Some(play), Some(hud)) = (self.play.as_ref(), self.hud.as_mut())
            && !play.mode().shows_cursor()
            && hud.cockpit.designer.is_open()
        {
            hud.cockpit.designer.close();
        }
        // The input update runs once a game frame, and the mission loop renders once a
        // frame with no cap (docs/24, "From input to motion").
        if let Some(play) = self.play.as_mut() {
            play.update_input();
        }
        self.step();
        // The briefing's clock starts on its first drawn frame; its voices play at once.
        if let Some(b) = self.briefing.as_mut() {
            let clock = *self.briefing_clock.get_or_insert_with(Instant::now);
            for voice in b.frame(clock.elapsed().as_secs_f64()) {
                if let Some(a) = self.audio.as_mut() {
                    a.play_now(&voice);
                }
            }
            if b.finished() {
                self.end_briefing();
            }
        }
        // A unit a factory made is drawn from now on, and named on the HUD.
        if let (Some(play), Some(view)) = (self.play.as_mut(), self.view.as_mut()) {
            let names: Vec<String> = play.added.iter().map(|&t| play.names[t].clone()).collect();
            let added = scene::add_targets(&mut self.world.objects, &mut self.world.store, view, play)
                .and_then(|added| {
                    if added {
                        play.draw_rounds(&mut self.world.store, &mut self.world.objects)?;
                    }
                    Ok(added)
                });
            match added {
                Ok(true) => {
                    if let Some(r) = self.running.as_mut() {
                        let (d, q) = (&r.gpu.device, &r.gpu.queue);
                        r.renderer.set_world(
                            d,
                            q,
                            &self.world.store.textures,
                            Some(&self.world.terrain),
                            Some(&self.world.objects),
                        );
                        r.renderer.set_sprite_looks(d, &scene::sprite_looks(play));
                    }
                    if let Some(hud) = self.hud.as_mut() {
                        hud.cockpit.panels.names.extend(names);
                    }
                }
                Ok(false) => {}
                Err(e) => eprintln!("cannot draw a new unit: {e:#}"),
            }
        }
        // The sounds the ticks started play now, whether or not a frame can be drawn.
        if let (Some(play), Some(audio)) = (self.play.as_mut(), self.audio.as_mut()) {
            let eye = play.eye();
            let right = eye.forward.cross(eye.up);
            for cue in std::mem::take(&mut play.cues) {
                audio.play(&cue, eye.position, right);
            }
        }
        // What the game says: its text on screen, its voices queued, its sounds at once.
        if let Some(play) = self.play.as_mut() {
            let now = play.hero.time_ms;
            for say in std::mem::take(&mut play.says) {
                match say {
                    parkan_world::progress::Say::Text(sender, text) => {
                        println!("{text}");
                        if let Some(hud) = self.hud.as_mut() {
                            hud.cockpit.messages.show(sender, text, now);
                        }
                    }
                    parkan_world::progress::Say::Voice(s) => {
                        if let Some(a) = self.audio.as_mut() {
                            a.queue(&s);
                        }
                    }
                    parkan_world::progress::Say::Sound(s) => {
                        if let Some(a) = self.audio.as_mut() {
                            a.play_now(&s);
                        }
                    }
                }
            }
        }
        if let Some(a) = self.audio.as_mut() {
            a.update();
        }
        // The CD moves on to another track 2 s after one ends, but not in a briefing.
        if let Some(cd) = self.cd.as_mut() {
            let now = self.started.elapsed().as_secs_f64() * 1000.0;
            if let Some(command) = cd.frame(now, self.briefing.is_some()) {
                let length = self.audio.as_mut().and_then(|a| a.cd(&command));
                if matches!(command, parkan_world::music::Cd::Play { .. }) {
                    cd.started(now, length.unwrap_or(0.0));
                }
            }
        }
        // An ambient variation falls due every 10 to 19 s, briefing or not: the frame's
        // test stands between the pause and the state word alone (`0x1005eb49`).
        if let Some(amb) = self.ambience.as_mut() {
            let seconds = self.play.as_ref().map_or(0.0, |p| p.hero.time_ms / 1000.0);
            // STAND-IN: docs/34-progression.md#ambient-sound--read-and-measured -- which
            // flag of the level's `+0xae4` object the picker reads for night is not
            // followed: night is when no celestial body is up.
            let night = self
                .world
                .atmosphere
                .as_ref()
                .is_some_and(|a| parkan_sim::sky::bodies_up(a, seconds).is_empty());
            let now = seconds * 1000.0;
            if let Some(sound) = amb.frame(now, night).cloned()
                && let Some(a) = self.audio.as_mut()
            {
                a.play_now(&sound);
            }
        }
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
        let display = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(r.renderer.display_format()),
            ..Default::default()
        });
        let aspect = r.config.width as f32 / r.config.height.max(1) as f32;
        let (eye, forward, field, seconds) = match &self.play {
            Some(p) => {
                let e = self.briefing.as_ref().map_or_else(|| p.eye(), |b| b.eye());
                (e.position, e.forward, e.fov_x, p.hero.time_ms / 1000.0)
            }
            None => (
                self.camera.position,
                self.camera.forward(),
                camera::debug_field(aspect),
                self.started.elapsed().as_secs_f64(),
            ),
        };
        if let Some((lighting, colours)) = scene::lighting(&self.world, seconds, eye, forward, field) {
            r.renderer.set_lighting(lighting);
            r.renderer.set_dome_colours(colours);
            r.renderer.set_sky_layers(scene::sky_layers(&self.world, seconds));
            r.renderer.set_body_sprites(&r.gpu.device, scene::body_sprites(&self.world, seconds));
            r.renderer.set_flare(&r.gpu.device, scene::flare(&self.world, seconds));
        }
        let lighting = scene::lighting(&self.world, seconds, eye, forward, field).map(|l| l.0);
        let view_proj = match self.play.as_mut() {
            Some(play) => {
                let briefing = self.briefing.as_ref();
                let eye = briefing.map_or_else(|| play.eye(), |b| b.eye());
                let view_proj = camera::first_person(&eye, aspect);
                let screen = hud_space(r.config.width, r.config.height, &self.args);
                let outcome = scene::draw_outcome(&mut r.renderer, &r.gpu.device, &r.gpu.queue, play, screen);
                if let Some(b) = briefing {
                    scene::draw_briefing(&mut r.renderer, &r.gpu.device, &r.gpu.queue, b, screen);
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
                if let Some(v) = &self.view {
                    let outside = briefing.is_some() || play.mode().shows_cursor() || play.outer_shows();
                    scene::place_own_view(&mut r.renderer, &r.gpu.queue, v, play, outside);
                    if let Some(hud) = self.hud.as_mut() {
                        let (voices, sounds) = scene::draw_hud(
                            &mut r.renderer,
                            &r.gpu.device,
                            &r.gpu.queue,
                            hud,
                            play,
                            v,
                            (r.config.width, r.config.height),
                            view_proj,
                            lighting,
                            !outcome && briefing.is_none(),
                        );
                        if let (Some(audio), Some(progression)) =
                            (self.audio.as_mut(), play.progression.as_ref())
                        {
                            for sound in voices.into_iter().filter_map(|n| progression.sound(n)) {
                                audio.queue(&sound);
                            }
                            for sound in sounds.into_iter().filter_map(|n| progression.sound(n)) {
                                audio.play_now(&sound);
                            }
                        }
                    }
                }
                view_proj
            }
            None => self.camera.view_proj(aspect),
        };
        r.renderer.draw(
            &r.gpu.device,
            &r.gpu.queue,
            &view,
            &display,
            (r.config.width, r.config.height),
            view_proj,
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
            // `WM_ACTIVATEAPP` (`iron3d.dll:0x100a0e20`): leaving the window lets every key and
            // button up (docs/14, "Leaving the window lets every key up").
            WindowEvent::Focused(focused) => {
                if self.trace {
                    eprintln!("window: focused {focused}");
                }
                if !focused {
                    self.grab(false);
                    self.release_keys();
                }
                self.shortcut = false;
            }
            // winit brings its modifiers up to date from every key, click and mouse movement, so
            // a Shift let go while the system had the keyboard comes up at the next of them.
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                if self.trace {
                    eprintln!("window: modifiers {state:?}");
                }
                if !state.shift_key() {
                    for scan in ["SCAN_LSHIFT", "SCAN_RSHIFT"] {
                        if self.scans.contains(scan) {
                            self.scan(scan, false);
                        }
                    }
                }
                if !state.super_key() {
                    self.shortcut = false;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                if self.trace {
                    eprintln!(
                        "window: {code:?} {:?}{}",
                        event.state,
                        if event.repeat { " repeat" } else { "" }
                    );
                }
                // Cmd starts a system shortcut, and on macOS the system takes the keyboard and
                // the mouse without the window losing focus: under Cmd-Shift-4's screenshot the
                // keys' ups never arrive and a grabbed cursor cannot move. On the game's own
                // system the shortcut leaves the window, which lets every key up; Cmd going down
                // does that here, and frees the cursor. The keys pressed with it are the
                // shortcut's.
                if matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight) {
                    let pressed = event.state == ElementState::Pressed;
                    if pressed && !self.shortcut {
                        self.grab(false);
                        self.release_keys();
                    }
                    self.shortcut = pressed;
                    return;
                }
                if self.shortcut && event.state == ElementState::Pressed {
                    return;
                }
                let outcome =
                    self.play.as_ref().and_then(|p| p.progression.as_ref()).and_then(|p| p.progress.outcome);
                // `0x10070e75`: Esc in a briefing skips the rest of it.
                if code == KeyCode::Escape && event.state == ElementState::Pressed && self.briefing.is_some()
                {
                    if let Some(b) = self.briefing.as_mut() {
                        b.flythrough.skip();
                    }
                    return;
                }
                // While save's name field is up the designer takes the characters (`0x10055fa0`):
                // the printable ones, backspace and Enter.
                if event.state == ElementState::Pressed
                    && let Some(hud) = self.hud.as_mut()
                    && hud.cockpit.designer.session.as_ref().is_some_and(|s| s.field.is_some())
                    && code != KeyCode::Escape
                {
                    let typed: Vec<char> = match code {
                        KeyCode::Backspace => vec!['\u{8}'],
                        KeyCode::Enter | KeyCode::NumpadEnter => vec!['\r'],
                        _ => event.text.as_ref().map(|t| t.chars().collect()).unwrap_or_default(),
                    };
                    for c in typed {
                        hud.cockpit.designer.key(c, &hud.font);
                    }
                    return;
                }
                // Esc goes to the warbot designer first while the mission is played
                // (`0x10055e80`): its load list or name field, else the designer itself.
                if code == KeyCode::Escape
                    && event.state == ElementState::Pressed
                    && let (Some(hud), Some(play)) = (self.hud.as_mut(), self.play.as_ref())
                    && hud.cockpit.designer.takes_input(play)
                {
                    hud.cockpit.designer.escape();
                    return;
                }
                // Esc's key-down meets the game's listener before its binding (`0x100a0eb8`,
                // `0x10070db0`): an objectives screen, a placement, a message box, and in the
                // commander's view an open map and a page other than 0 are put away first, one
                // a press, and only then does 735 roll the mode back.
                if code == KeyCode::Escape
                    && event.state == ElementState::Pressed
                    && outcome.is_none()
                    && let (Some(play), Some(hud)) = (self.play.as_mut(), self.hud.as_mut())
                {
                    use parkan_world::cockpit::escape::{Layer, Up, peel};
                    let cockpit = &mut hud.cockpit;
                    let now = play.hero.time_ms;
                    let up =
                        Up {
                            objectives: cockpit.objectives.up,
                            placing: play.commander.ghost.is_some()
                                || play.commander.pending.as_ref().is_some_and(|p| {
                                    matches!(p.kind, parkan_world::pick::PendingKind::Build(_))
                                }),
                            message_box: cockpit.messages.on_screen(now),
                            commander_view: play.mode().commands(),
                            map: cockpit.map.open,
                            page: cockpit.commander.page != 0,
                        };
                    if let Some(layer) = peel(up) {
                        match layer {
                            Layer::Objectives => cockpit.objectives.close(),
                            Layer::Placement => play.cancel_placement(),
                            Layer::MessageBox => cockpit.messages.pager(),
                            Layer::Map => cockpit.map.toggle(),
                            Layer::Page => cockpit.commander.page = 0,
                        }
                        return;
                    }
                }
                // `CMD_ROLLBACK_STATE` (735, Esc): a building's screen gives the hero back (docs/36).
                if code == KeyCode::Escape
                    && event.state == ElementState::Pressed
                    && outcome.is_none()
                    && let Some(play) = self.play.as_mut()
                    && play.mode() != parkan_world::play::Mode::OnFoot
                {
                    play.roll_back();
                    return;
                }
                if code == KeyCode::Escape && event.state == ElementState::Pressed {
                    // `iron3d.dll:0x10070e2c`: once the outcome is recorded Esc leaves the mission.
                    //
                    // STAND-IN: docs/34-progression.md#after-the-outcome--read-and-measured -- the
                    // shell's menus are not built: leaving closes the window, and a win is not
                    // written to `MISSIONS/dispatcher.ini`.
                    if self.grabbed && outcome.is_none() {
                        self.grab(false)
                    } else {
                        event_loop.exit()
                    }
                    return;
                }
                // `0x100711f0`: after a loss R restarts the mission. L would open the
                // load-game screen, which the engine does not have.
                if code == KeyCode::KeyR && event.state == ElementState::Pressed && outcome == Some(false) {
                    if let Err(e) = self.restart() {
                        eprintln!("cannot restart the mission: {e:#}");
                    }
                    return;
                }
                let pressed = event.state == ElementState::Pressed;
                if self.play.is_some() {
                    match scene::scan_name(code) {
                        Some(scan) if !event.repeat => self.scan(scan, pressed),
                        Some(scan) if pressed => self.repeat(scan),
                        _ => {}
                    }
                } else if pressed {
                    self.held.insert(code);
                } else {
                    self.held.remove(&code);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = [position.x as f32, position.y as f32];
                self.cursor_in = true;
                if let (Some(r), Some(hud)) = (self.running.as_ref(), self.hud.as_mut()) {
                    let space = hud_space(r.config.width, r.config.height, &self.args);
                    let at = space.layout(self.cursor, parkan_world::hud::Pin::TOP_LEFT);
                    let panel = &mut hud.cockpit.commander;
                    panel.cursor = Some(at);
                    // A drag held 0.35 s becomes a band, outside a route and a placement
                    // (`0x10071410`).
                    let route = self.play.as_ref().is_some_and(|p| {
                        p.commander.ghost.is_some()
                            || p.commander.pick_mode == parkan_world::pick::PickMode::Route
                    });
                    if let Some((since, anchor)) = self.left_down
                        && !route
                        && (panel.band.is_some()
                            || since.elapsed().as_secs_f64() > parkan_world::pick::BAND_HOLD_S)
                    {
                        panel.band = Some([space.layout(anchor, parkan_world::hud::Pin::TOP_LEFT), at]);
                    }
                }
            }
            // The game's cursor cannot leave its full screen; a window's can, and away from
            // the window it is against no edge.
            WindowEvent::CursorLeft { .. } => {
                self.cursor_in = false;
                if let Some(hud) = self.hud.as_mut() {
                    hud.cockpit.commander.cursor = None;
                }
            }
            WindowEvent::CursorEntered { .. } => self.cursor_in = true,
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if self.briefing.is_some() {
                    return;
                }
                if let Some(play) = self.play.as_mut()
                    && let parkan_world::play::Mode::Factory(t) = play.mode()
                {
                    if pressed
                        && button == MouseButton::Left
                        && let Some(r) = self.running.as_ref()
                    {
                        let space = hud_space(r.config.width, r.config.height, &self.args);
                        let at = space.layout(self.cursor, parkan_world::hud::Pin::TOP_LEFT);
                        use parkan_world::cockpit::{designer, factory};
                        match self.hud.as_mut().map(|h| &mut h.cockpit) {
                            // A research centre's screen is the research panel (`0x10089ce0`).
                            Some(cockpit)
                                if play.units.get(t).is_some_and(|u| {
                                    u.type_word == parkan_world::selection::RESEARCH_CENTRE
                                }) =>
                            {
                                let click = cockpit.commander.research.click(play, at);
                                if click == parkan_world::cockpit::research::Click::Exit {
                                    play.roll_back();
                                }
                            }
                            // The designer, while it is up, takes the click (`0x10055ff0`).
                            Some(cockpit) if cockpit.designer.is_open() => {
                                let at = designer::layout_point(space, self.cursor);
                                cockpit.designer.click(play, at, &cockpit.strings);
                            }
                            cockpit => {
                                if let Some(click) = factory::click(play, t, at) {
                                    match (click, cockpit) {
                                        (factory::Click::Constructor, Some(cockpit)) => {
                                            if let Err(e) = cockpit.designer.open(play, t, &cockpit.strings) {
                                                eprintln!("cannot open the designer: {e:#}");
                                            }
                                        }
                                        _ => play.factory_click(t, click),
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
                // Command mode keeps the cursor free. A press goes to the ghost while one is up,
                // else to the commander panel first (`0x1008d690`), then the world; a band is let
                // go on the way up; the right button undoes what is open (docs/42).
                if self.play.as_ref().is_some_and(|p| p.mode().commands()) {
                    self.command_mouse(button, pressed);
                    return;
                }
                if self.play.is_some() && pressed && !self.grabbed {
                    self.grab(true);
                    return;
                }
                if self.play.is_some() {
                    if let Some(scan) = scene::button_name(button) {
                        self.scan(scan, pressed);
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
            if self.grabbed && self.briefing.is_none() {
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
        let mut play = scene::play(&game, &loaded, &args)?.context("the mission has no hero to play")?;
        rehearse(&mut play, &loaded, &args);
        report(&play);
        return Ok(());
    }
    if let Some(out) = &args.screenshot {
        return screenshot(&loaded, &game, &args, out);
    }
    let mut world = scene::world(&game, &loaded)?;
    let mut play = if args.fly { None } else { scene::play(&game, &loaded, &args)? };
    let mut view = None;
    if let Some(p) = play.as_mut() {
        // Where the hero starts and what it is driving are set in the window too: `--at`,
        // `--face`, `--pod`, `--drive`, `--take` and the rest had reached only `--screenshot`
        // and `--headless`, so opening a window with one of them left the hero at the
        // mission's own start. `--ticks` is 0 unless it is asked for, so nothing is played
        // out here.
        rehearse(p, &loaded, &args);
        view = Some(scene::own_view(&mut world.objects, &mut world.store, p)?);
        p.draw_rounds(&mut world.store, &mut world.objects)?;
        // `--hold` presses keys in the window too, for trying things without hands.
        for key in &args.hold {
            p.key(key, true);
        }
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let camera = start_camera(&loaded);
    let mut audio = if play.is_some() { audio::Audio::open(&game) } else { None };
    let mut cd = play.is_some().then(|| cd_player(&game));
    let started = Instant::now();
    // The theme waits for the briefing's end (`0x10030f10`).
    let briefing = open_briefing(&game, &loaded, &args, play.as_mut());
    let ambience = ambience(&game, &loaded);
    if briefing.is_none() {
        theme(&mut audio, &game, &loaded);
    } else {
        briefing_music(&mut cd, &mut audio, &loaded, 0.0);
    }
    let mut app = App {
        loaded,
        world,
        play,
        view,
        camera,
        running: None,
        held: HashSet::new(),
        looking: false,
        grabbed: false,
        cursor: [0.0; 2],
        cursor_in: false,
        shortcut: false,
        left_down: None,
        cursor_hidden: false,
        counts: [0.0; 2],
        mouse: args.mouse,
        trace: args.trace,
        last: Instant::now(),
        owed: 0.0,
        audio,
        cd,
        ambience,
        started,
        game: game.clone(),
        bindings: scene::bindings(&game),
        scans: HashSet::new(),
        hud: None,
        args: args.clone(),
        briefing,
        briefing_clock: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}
