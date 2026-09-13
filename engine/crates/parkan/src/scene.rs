//! A mission as the engine plays it: its map's ground, its placed objects and
//! its hero.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::{gamedir, mission, sky};
use parkan_world::assembly::Assembly;
use parkan_world::hero;
use parkan_world::models::{self, Objects};
use parkan_world::terrain::{self, Terrain};
use parkan_world::textures::TextureStore;

/// A loaded mission and where its hero stands.
pub struct Loaded {
    pub dir: PathBuf,
    pub mission: mission::Mission,
    /// The player's hero: position and heading, radians about z.
    pub hero: Option<(Vec3, f32)>,
}

pub fn mission_dir(game: &Path, relative: &str) -> Result<PathBuf> {
    gamedir::resolve(game, relative)
        .with_context(|| format!("no mission {relative} under {}", game.display()))
}

pub fn load(game: &Path, relative: &str) -> Result<Loaded> {
    let dir = mission_dir(game, relative)?;
    let tma = gamedir::resolve(&dir, "data.tma").context("the mission has no data.tma")?;
    let data = std::fs::read(&tma)?;
    let mission = mission::parse(&data, &tma.display().to_string())?;

    let hero = mission
        .objects
        .iter()
        .find(|o| hero::is_hero(&o.path))
        .map(|o| (Vec3::from_array(o.position), o.rotation));
    Ok(Loaded { dir, mission, hero })
}

/// The world a mission is drawn in: its textures, its map's ground, its objects.
pub struct World {
    pub store: TextureStore,
    pub terrain: Terrain,
    pub objects: Objects,
    pub atmosphere: Option<sky::Atmosphere>,
}

pub fn world(game: &Path, loaded: &Loaded) -> Result<World> {
    let mut store = TextureStore::open(game)?;
    let terrain = terrain::build(&terrain::map_dir(game, &loaded.mission.map_path)?, &mut store)?;
    let mut assembly = Assembly::new(game)?;
    let objects = models::build(&mut assembly, &mut store, &loaded.mission)?;
    let atmosphere = gamedir::resolve(&loaded.dir, "sky.ske")
        .and_then(|p| std::fs::read(&p).ok().map(|b| (b, p)))
        .and_then(|(b, p)| sky::parse(&b, &p.display().to_string()).ok());
    Ok(World { store, terrain, objects, atmosphere })
}

pub use parkan_world::play::Play;

/// STAND-IN: docs/10-sky.md#not-resolved -- the time of day a mission starts at is not
/// read; its atmosphere clock starts at noon.
pub const START_HOUR: f64 = 12.0;

/// The light, fog and dome colours at `seconds` into the mission, seen from `eye`
/// looking along `forward`; `None` without an atmosphere.
pub fn lighting(
    world: &World,
    seconds: f64,
    eye: Vec3,
    forward: Vec3,
) -> Option<(parkan_render::frame::Lighting, Vec<[f32; 3]>)> {
    use parkan_render::frame::linear;
    use parkan_sim::sky as atm;
    let a = world.atmosphere.as_ref()?;
    let clock = a.day_seconds() * START_HOUR / 24.0 + seconds;
    let sky = atm::at(a, 0, clock)?;
    // The heading from +y towards +x (docs/10-sky.md, "Fog").
    let heading = forward.x.atan2(forward.y);
    let fog = sky.fog_colour(heading);
    // STAND-IN: docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured --
    // how CSun lights the scene is not traced: the body that is up shines along its
    // direction in the sky keyframes' slot 19 × light, held to 1 as a D3D light is.
    let body = if atm::body_up(a, 0, "sun", clock) { atm::SUN_DIRECTION } else { atm::MOON_DIRECTION };
    let lighting = parkan_render::frame::Lighting {
        light_direction: -body,
        light_colour: linear(sky.sun_light.map(|c| c.clamp(0.0, 1.0))),
        scene_colour: linear(sky.scene_colour),
        fog_colour: linear(fog),
        fog_start: sky.fog_start,
        fog_end: sky.fog_end,
        eye,
    };
    Some((lighting, sky.dome_colours(fog).into_iter().map(linear).collect()))
}

pub fn play(game: &Path, loaded: &Loaded) -> Result<Option<Play>> {
    Play::load(game, &loaded.mission)
}

/// The HUD: a crosshair at the screen's centre, where the sight looks, and a slot for
/// each gun, lit while selected, with its magazine and capacitor as bars.
///
/// STAND-IN: docs/30-turrets.md#not-established -- how the game's HUD draws the aim
/// point and the guns is not read.
pub fn hud(play: &Play, aspect: f32) -> Vec<parkan_render::hud::Rect> {
    use parkan_render::hud::Rect;
    let mut out = Vec::new();
    let (w, h) = (0.02 / aspect.max(0.1), 0.02);
    let (tw, th) = (0.002 / aspect.max(0.1), 0.002);
    let white = [1.0, 1.0, 1.0, 0.8];
    out.push(Rect { min: [-w, -th], max: [-w / 3.0, th], colour: white });
    out.push(Rect { min: [w / 3.0, -th], max: [w, th], colour: white });
    out.push(Rect { min: [-tw, -h], max: [tw, -h / 3.0], colour: white });
    out.push(Rect { min: [-tw, h / 3.0], max: [tw, h], colour: white });
    for (i, g) in play.hero.guns.iter().enumerate() {
        let x0 = -0.95 + i as f32 * 0.12;
        let (x1, y0, y1) = (x0 + 0.1, -0.95, -0.85);
        let frame = if g.selected { [1.0, 0.8, 0.2, 0.9] } else { [0.4, 0.4, 0.4, 0.6] };
        out.push(Rect { min: [x0, y0], max: [x1, y1], colour: [0.0, 0.0, 0.0, 0.4] });
        out.push(Rect { min: [x0, y1], max: [x1, y1 + 0.008], colour: frame });
        let full = |v: f32| x0 + 0.005 + (x1 - x0 - 0.01) * v.clamp(0.0, 1.0);
        let magazine = if g.magazine < 0 { 1.0 } else { g.rounds as f32 / g.magazine.max(1) as f32 };
        let charge = if g.capacitor > 0.0 { g.charge / g.capacitor } else { 1.0 };
        out.push(Rect {
            min: [x0 + 0.005, y0 + 0.055],
            max: [full(magazine), y0 + 0.08],
            colour: [0.9, 0.9, 0.9, 0.9],
        });
        out.push(Rect {
            min: [x0 + 0.005, y0 + 0.02],
            max: [full(charge), y0 + 0.045],
            colour: [0.3, 0.7, 1.0, 0.9],
        });
    }
    out
}

/// The looks the effects draw with, for the renderer.
pub fn sprite_looks(play: &Play) -> Vec<parkan_render::sprites::SpriteLook> {
    play.fx
        .looks
        .iter()
        .map(|l| parkan_render::sprites::SpriteLook { texture: l.texture, blend_mode: l.blend_mode })
        .collect()
}

/// Bring the drawing up to date with the battle: hide what died, place the rounds, and
/// hand over this frame's effect sprites as seen from `eye`.
pub fn sync(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    play: &mut Play,
    objects: &Objects,
    view_proj: glam::Mat4,
    eye: Vec3,
) {
    let quads: Vec<parkan_render::sprites::Quad> = play
        .sprites()
        .into_iter()
        .map(|(look, s)| parkan_render::sprites::Quad {
            look,
            corners: parkan_render::sprites::billboard(s.centre, s.along, s.width, eye),
            alpha: s.alpha,
        })
        .collect();
    renderer.set_sprites(device, queue, view_proj, &quads);
    for object in std::mem::take(&mut play.killed) {
        if let Some(i) = objects.placed.iter().position(|&p| p == object) {
            // STAND-IN: docs/26-damage.md#hit-points--read-and-measured -- what a dead
            // object leaves (its explosion, wreck or damage stages) is not drawn: it goes.
            renderer.set_instance(queue, i, glam::Mat4::IDENTITY, false);
        }
    }
    for (i, matrix, visible) in play.battle.round_instances() {
        renderer.set_instance(queue, i, matrix, visible);
    }
}

/// Take the hero's own placement out of what is drawn: the eye is inside it.
///
/// STAND-IN: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- whether the
/// game draws the player's own unit in first person is not read; it is not drawn.
pub fn hide(objects: &mut Objects, object: usize) {
    if let Some(i) = objects.placed.iter().position(|&p| p == object) {
        objects.placed.remove(i);
        objects.instances.remove(i);
    }
}

/// A key as the input tables name it.
pub fn scan_name(code: winit::keyboard::KeyCode) -> Option<&'static str> {
    use winit::keyboard::KeyCode as K;
    const LETTERS: [&str; 26] = [
        "SCAN_A", "SCAN_B", "SCAN_C", "SCAN_D", "SCAN_E", "SCAN_F", "SCAN_G", "SCAN_H", "SCAN_I", "SCAN_J",
        "SCAN_K", "SCAN_L", "SCAN_M", "SCAN_N", "SCAN_O", "SCAN_P", "SCAN_Q", "SCAN_R", "SCAN_S", "SCAN_T",
        "SCAN_U", "SCAN_V", "SCAN_W", "SCAN_X", "SCAN_Y", "SCAN_Z",
    ];
    const LETTER_KEYS: [K; 26] = [
        K::KeyA,
        K::KeyB,
        K::KeyC,
        K::KeyD,
        K::KeyE,
        K::KeyF,
        K::KeyG,
        K::KeyH,
        K::KeyI,
        K::KeyJ,
        K::KeyK,
        K::KeyL,
        K::KeyM,
        K::KeyN,
        K::KeyO,
        K::KeyP,
        K::KeyQ,
        K::KeyR,
        K::KeyS,
        K::KeyT,
        K::KeyU,
        K::KeyV,
        K::KeyW,
        K::KeyX,
        K::KeyY,
        K::KeyZ,
    ];
    const DIGITS: [&str; 10] = [
        "SCAN_W_0", "SCAN_W_1", "SCAN_W_2", "SCAN_W_3", "SCAN_W_4", "SCAN_W_5", "SCAN_W_6", "SCAN_W_7",
        "SCAN_W_8", "SCAN_W_9",
    ];
    const DIGIT_KEYS: [K; 10] = [
        K::Digit0,
        K::Digit1,
        K::Digit2,
        K::Digit3,
        K::Digit4,
        K::Digit5,
        K::Digit6,
        K::Digit7,
        K::Digit8,
        K::Digit9,
    ];
    if let Some(i) = LETTER_KEYS.iter().position(|&k| k == code) {
        return Some(LETTERS[i]);
    }
    if let Some(i) = DIGIT_KEYS.iter().position(|&k| k == code) {
        return Some(DIGITS[i]);
    }
    Some(match code {
        K::ShiftLeft => "SCAN_LSHIFT",
        K::ShiftRight => "SCAN_RSHIFT",
        K::NumpadMultiply => "SCAN_G_ASTERISK",
        K::NumpadAdd => "SCAN_G_PLUS",
        K::NumpadSubtract => "SCAN_G_SUB",
        K::NumpadDivide => "SCAN_G_SLASH",
        _ => return None,
    })
}

/// A mouse button as the input tables name it.
pub fn button_name(button: winit::event::MouseButton) -> Option<&'static str> {
    use winit::event::MouseButton as B;
    match button {
        B::Left => Some("SCAN_LMOUSE"),
        B::Right => Some("SCAN_RMOUSE"),
        B::Middle => Some("SCAN_MMOUSE"),
        _ => None,
    }
}
