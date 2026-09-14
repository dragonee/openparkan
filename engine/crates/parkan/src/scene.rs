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

/// The lights, fog and dome colours at `seconds` into the mission, seen from `eye`
/// looking along `forward`; `None` without an atmosphere.
///
/// The clock starts at the file's closing time and plays the sections in turn; the sun
/// object's two lights shine while a body is up (docs/10-sky.md).
pub fn lighting(
    world: &World,
    seconds: f64,
    eye: Vec3,
    forward: Vec3,
) -> Option<(parkan_render::frame::Lighting, Vec<[f32; 3]>)> {
    use parkan_render::frame::{Light, linear};
    use parkan_sim::sky as atm;
    let a = world.atmosphere.as_ref()?;
    let now = atm::position(a, seconds);
    let sky = atm::at(a, now)?;
    // STAND-IN: docs/10-sky.md#not-resolved -- which camera axis the fog's heading angle
    // measures is not read; the view direction's heading, 0 along +y turning towards +x.
    let heading = forward.x.atan2(forward.y);
    let fog = sky.fog_colour(heading);
    // No shipped section has the sun and the moon up at once; where two bodies are up,
    // the first started lights the scene.
    let lights = match atm::bodies_up(a, seconds).first() {
        Some(&body) => {
            let [main, second] = atm::sun_lights(&sky, body, forward);
            // STAND-IN: docs/10-sky.md#not-resolved -- where the sun object's two lights
            // point is not read (CSun sets only their colours); both shine from the
            // body's fixed place.
            let direction = -body.direction();
            [Light { direction, colour: main }, Light { direction, colour: second }]
        }
        None => [Light::OFF; 2],
    };
    let lighting = parkan_render::frame::Lighting {
        lights,
        scene_colour: sky.scene_colour,
        fog_colour: linear(fog),
        fog_start: sky.fog_start,
        fog_end: sky.fog_end,
        eye,
        clock_ms: seconds * 1000.0,
    };
    Some((lighting, sky.dome_colours(fog).into_iter().map(linear).collect()))
}

/// The mission's play, with the hero's view held steady against its gait unless `--sway`,
/// a captured bot standing by unless `--capture-idle`, the target bracketed unless
/// `--no-bracket`, and its progression when its
/// script and messages load.
pub fn play(game: &Path, loaded: &Loaded, args: &crate::Args) -> Result<Option<Play>> {
    let mut play = Play::load(game, &loaded.mission)?;
    if let Some(p) = play.as_mut() {
        p.hero.steady = !args.sway;
        p.capture_standby = !args.capture_idle;
        p.bracket = !args.no_bracket;
        if let Err(e) = p.load_progression(game, &loaded.dir, &loaded.mission) {
            eprintln!("no mission progression: {e:#}");
        }
    }
    Ok(play)
}

/// The game's own key chords (`ui_other.man`), or none when the file does not load.
pub fn bindings(game: &Path) -> Vec<parkan_formats::controls::Binding> {
    use parkan_formats::controls;
    gamedir::resolve(game, controls::GAME_BINDINGS)
        .and_then(|p| std::fs::read(&p).ok().map(|b| (b, p)))
        .and_then(|(b, p)| controls::bindings(&b, &p.display().to_string()).ok())
        .unwrap_or_default()
}

/// The wingman panel as text: each wingman's number and name, a chosen one bright and
/// the rest grey, dimmed while picking; and the order menu's rows, numbered, disabled
/// ones grey (docs/31, "The wingman menu from first person").
///
/// STAND-IN: docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured --
/// the two labels a wingman line draws beside its number, and where the panel stands on
/// the screen, are not read: the unit's name; the wingmen down the left 19 apart from
/// (20, 100) and the menu's rows 19 apart from (220, 250), on a 640 by 480 screen.
pub fn panel_runs(panel: &parkan_world::play::Panel) -> Vec<parkan_world::text::TextRun> {
    use parkan_world::text::TextRun;
    let at = |x: f32, y: f32| [x / 320.0 - 1.0, 1.0 - y / 240.0];
    let mut out = Vec::new();
    for (i, (number, name, chosen)) in panel.wingmen.iter().enumerate() {
        let colour = match (chosen, panel.picking) {
            (true, _) => [1.0, 0.9, 0.3, 1.0],
            (false, true) => [0.5, 0.5, 0.5, 0.5],
            (false, false) => [0.6, 0.6, 0.6, 1.0],
        };
        out.push(TextRun {
            colour,
            ..TextRun::new(format!("{number} {name}"), at(20.0, 100.0 + 19.0 * i as f32))
        });
    }
    for (i, (text, enabled)) in panel.rows.iter().enumerate() {
        let colour = if *enabled { [1.0, 1.0, 1.0, 1.0] } else { [0.45, 0.45, 0.45, 1.0] };
        out.push(TextRun {
            colour,
            ..TextRun::new(format!("{} {text}", i + 1), at(220.0, 250.0 + 19.0 * i as f32))
        });
    }
    out
}

/// The outcome panel's font slots: its title's `MENU_FONT` and its lines' `GAME_FONT`.
pub const PANEL_TITLE_SLOT: usize = 0;
pub const PANEL_LINES_SLOT: usize = 1;

/// The outcome panel on a `width` × `height` screen (`iron3d.dll:0x1009f8b0`, docs/34, "After
/// the outcome"), in place of the HUD: a black box at 60% and, centred on x 320 of a 640 ×
/// 480 layout, the title in `menu` at y 50, green won and red lost, and the lines in `game`,
/// grey, from y 75 one font height + 2 apart. The box runs from (x₀, 35) to (640 − x₀, the
/// pen after the last line + 15), x₀ being the leftmost text's x less 30. Returns the box,
/// the title's run and the lines' runs.
///
/// STAND-IN: docs/34-progression.md#after-the-outcome--read-and-measured -- the two display
/// scale queries a font's height and a text's width pass through are not read: the layout
/// and its fonts scale by the screen's height over 480, the 640-wide layout centred across
/// the screen. On the recording of Mission 01's win the box runs 225 to 415 by 35 to about 98.
pub fn outcome_panel(
    panel: &parkan_world::progress::OutcomePanel,
    menu: &parkan_world::text::GameFont,
    game: &parkan_world::text::GameFont,
    (width, height): (f32, f32),
) -> (Vec<parkan_render::hud::Rect>, Vec<parkan_world::text::TextRun>, Vec<parkan_world::text::TextRun>) {
    use parkan_world::text::{Align, TextRun};
    let s = height / 480.0;
    let at = |x: f32, y: f32| [(x - 320.0) * s / (width / 2.0), 1.0 - y * s / (height / 2.0)];
    let rgb = |r: u8, g: u8, b: u8| [r, g, b].map(|c| f32::from(c) / 255.0);
    let [tr, tg, tb] = if panel.won { rgb(0x64, 0xff, 0x64) } else { rgb(0xff, 0x64, 0x64) };
    let [lr, lg, lb] = rgb(0xf0, 0xf0, 0xf0);
    let run = |text: &str, y: f32, colour: [f32; 4]| TextRun {
        colour,
        scale: s,
        align: Align::Centre,
        ..TextRun::new(text, at(320.0, y))
    };
    let title = run(&panel.title, 50.0, [tr, tg, tb, 1.0]);
    let mut left = 320.0 - menu.advance(&panel.title) / 2.0;
    let mut y = 75.0;
    let mut lines = Vec::new();
    for line in panel.lines.iter().filter(|l| !l.is_empty()) {
        lines.push(run(line, y, [lr, lg, lb, 1.0]));
        left = left.min(320.0 - game.advance(line) / 2.0);
        y += game.line_height + 2.0;
    }
    let x0 = (left - 30.0).max(0.0);
    let [x_min, y_max] = at(x0, 35.0);
    let [x_max, y_min] = at(640.0 - x0, y + 15.0);
    let bx =
        parkan_render::hud::Rect { min: [x_min, y_min], max: [x_max, y_max], colour: [0.0, 0.0, 0.0, 0.6] };
    (vec![bx], vec![title], lines)
}

/// Load the outcome panel's two fonts into their slots, or say why not.
pub fn panel_fonts(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    game: &Path,
) {
    use parkan_world::text::GameFont;
    for (slot, name) in [(PANEL_TITLE_SLOT, "MENU_FONT"), (PANEL_LINES_SLOT, "GAME_FONT")] {
        match GameFont::ui(game, name) {
            Ok(font) => renderer.set_font_slot(device, queue, slot, font),
            Err(e) => eprintln!("no {name}: {e:#}"),
        }
    }
}

/// Draw the outcome panel in the HUD's place once `play`'s outcome is recorded, or clear its
/// text; whether it is drawn.
pub fn draw_outcome(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    play: &Play,
    screen: (f32, f32),
) -> bool {
    let panel = play.progression.as_ref().and_then(|p| p.panel());
    let laid = match (&panel, renderer.font_slot(PANEL_TITLE_SLOT), renderer.font_slot(PANEL_LINES_SLOT)) {
        (Some(panel), Some(menu), Some(game)) => Some(outcome_panel(panel, menu, game, screen)),
        _ => None,
    };
    let Some((rects, title, lines)) = laid else {
        renderer.set_text_slot(device, queue, PANEL_TITLE_SLOT, &[]);
        renderer.set_text_slot(device, queue, PANEL_LINES_SLOT, &[]);
        return false;
    };
    renderer.set_hud(device, queue, &rects);
    renderer.set_text_slot(device, queue, PANEL_TITLE_SLOT, &title);
    renderer.set_text_slot(device, queue, PANEL_LINES_SLOT, &lines);
    true
}

/// The lines of the message history on screen, each with when it goes.
///
/// STAND-IN: docs/34-progression.md#not-established -- where the game draws a message's
/// text and for how long is not read: each line shows for 8 s, the newest four stacked
/// up from just above the guns, wrapped to 70% of the screen.
#[derive(Default)]
pub struct Subtitles {
    lines: std::collections::VecDeque<(String, f64)>,
    /// The font the lines wrap by.
    font: Option<parkan_world::text::GameFont>,
}

impl Subtitles {
    pub const SECONDS: f64 = 8.0;
    pub const SHOWN: usize = 4;
    /// Where the lowest line ends, in NDC, and the gap between lines, in font pixels.
    pub const BOTTOM: f32 = -0.78;
    pub const GAP: f32 = 6.0;

    pub fn new(font: Option<parkan_world::text::GameFont>) -> Self {
        Self { lines: Default::default(), font }
    }

    pub fn push(&mut self, text: String, now_s: f64) {
        self.lines.push_back((text, now_s + Self::SECONDS));
    }

    /// The text to draw at `now_s` on a screen `width` × `height` pixels.
    pub fn runs(&mut self, now_s: f64, width: f32, height: f32) -> Vec<parkan_world::text::TextRun> {
        use parkan_world::text::{Align, TextRun};
        self.lines.retain(|(_, until)| *until > now_s);
        let Some(font) = self.font.as_ref() else { return Vec::new() };
        let pixel = 2.0 / height.max(1.0);
        let mut bottom = Self::BOTTOM;
        let mut out = Vec::new();
        for (text, _) in self.lines.iter().rev().take(Self::SHOWN) {
            let run = TextRun {
                align: Align::Centre,
                wrap: Some(width * 0.7),
                colour: [1.0, 0.95, 0.7, 1.0],
                ..TextRun::new(text.as_str(), [0.0, 0.0])
            };
            let rows = font.lines(&run).len() as f32;
            let top = bottom + rows * font.line_height * run.scale * pixel;
            out.push(TextRun { anchor: [0.0, top], ..run });
            bottom = top + Self::GAP * pixel;
        }
        out
    }
}

/// The HUD: a crosshair at the screen's centre, where the sight looks, and a slot for
/// each gun, lit while selected, with its magazine and capacitor as bars.
///
/// STAND-IN: docs/30-turrets.md#not-established -- how the game's HUD draws the aim
/// point and the guns is not read.
///
/// DEPARTURE: docs/25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured --
/// in the cockpit the game brackets nothing in the world: it frames the target in its
/// target panel and outlines its radar mark, neither of which is drawn yet. With
/// [`Play::bracket`] four corners stand around the target's bounding sphere on screen,
/// in the colour the game marks its clan in.
pub fn hud(play: &Play, aspect: f32, view_proj: glam::Mat4) -> Vec<parkan_render::hud::Rect> {
    use parkan_render::hud::Rect;
    let mut out = Vec::new();
    if play.bracket
        && let Some(t) = play.targets.current
        && let Some(c) = play.contacts().get(t).copied()
        && let Some([x, y]) = parkan_world::play::on_screen(view_proj, c.centre, c.radius)
    {
        let edge = view_proj * (c.centre + Vec3::Z * c.radius).extend(1.0);
        let half_h = if edge.w > 1e-6 { (edge.y / edge.w - y).abs().clamp(0.02, 0.8) } else { 0.02 };
        let half_w = half_h / aspect.max(0.1);
        let [r, g, b] = play.mark_colour(play.units[t].clan).map(|v| f32::from(v) / 255.0);
        let colour = [r, g, b, 0.9];
        let (lw, lh) = (half_w * 0.35, half_h * 0.35);
        let (tw, th) = (0.003 / aspect.max(0.1), 0.003);
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (cx, cy) = (x + sx * half_w, y + sy * half_h);
            let horizontal = [cx.min(cx - sx * lw), cx.max(cx - sx * lw)];
            let vertical = [cy.min(cy - sy * lh), cy.max(cy - sy * lh)];
            out.push(Rect { min: [horizontal[0], cy - th], max: [horizontal[1], cy + th], colour });
            out.push(Rect { min: [cx - tw, vertical[0]], max: [cx + tw, vertical[1]], colour });
        }
    }
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
        .sprites(eye)
        .into_iter()
        .map(|(look, s)| parkan_render::sprites::Quad {
            look,
            corners: parkan_render::sprites::billboard(s.centre, s.along, s.width, eye),
            alpha: s.alpha,
            overlay: s.overlay,
        })
        .collect();
    renderer.set_sprites(device, queue, view_proj, &quads);
    // A dead unit is deleted its controller's +92 ms after it dies (docs/26); a building is
    // never killed. An object drawn node by node has hidden its nodes before.
    for object in std::mem::take(&mut play.killed) {
        if let Some(i) = objects.placed.iter().position(|&p| p == object) {
            renderer.set_instance(queue, i, glam::Mat4::IDENTITY, false);
        }
    }
    for (i, matrix, visible) in play.battle.round_instances() {
        renderer.set_instance(queue, i, matrix, visible);
    }
}

/// Which of the hero's meshes a node of its own view belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mount {
    Chassis,
    Turret,
}

/// The player's unit as its own view draws it: an instance for each node with a fifth
/// slot, following that node's pose.
pub struct OwnView {
    nodes: Vec<(usize, Mount, usize)>,
    /// Every target with node life drawn node by node: an instance, the target, its part,
    /// the node and the variant that instance draws.
    targets: Vec<(usize, usize, usize, usize, usize)>,
}

/// Draw the hero from now on as its own view does (docs/07-objects.md, "The fifth slot is
/// what the unit's own view draws"): every node's slot `variant × 5 + 4`, the cockpit and
/// whatever of the hull and guns has one, and nothing of a node without. The hero's
/// placement, its level 0, leaves `objects`. The chassis and the turret are the meshes
/// the hero poses; Mission 01's hero has no other part.
///
/// STAND-IN: docs/07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws -- what the
/// draw layers 10 and 9 a fifth slot is filed under do is not read; it draws with the
/// scene, depth-tested, lit and fogged like any model.
pub fn own_view(objects: &mut Objects, store: &mut TextureStore, play: &Play) -> Result<OwnView> {
    // Every whole model drawn node by node leaves first: removing one shifts the instances
    // after it, so no node's instance may be counted before the last removal.
    let noded: Vec<usize> = (0..play.battle.combat.targets.len())
        .filter(|&t| play.battle.combat.targets[t].parts.iter().any(|p| p.life.is_some()))
        .collect();
    let leaving: Vec<usize> =
        std::iter::once(play.hero.object).chain(noded.iter().map(|&t| play.battle.objects[t])).collect();
    for object in leaving {
        if let Some(i) = objects.placed.iter().position(|&p| p == object) {
            objects.placed.remove(i);
            objects.instances.remove(i);
        }
    }
    let mut nodes = Vec::new();
    for (mount, loaded) in [(Mount::Chassis, &play.hero.chassis), (Mount::Turret, &play.hero.turret)] {
        for node in 0..loaded.mesh.nodes.len() {
            let Some(model) = models::build_view_node(loaded, node, 0, |name| store.look(name))? else {
                continue;
            };
            objects.models.push(model);
            objects.instances.push(models::Instance {
                model: objects.models.len() - 1,
                position: [0.0; 3],
                rotation: 0.0,
                scale: 1.0,
                hidden: true,
            });
            objects.placed.push(usize::MAX);
            nodes.push((objects.instances.len() - 1, mount, node));
        }
    }
    // A unit or building that takes damage is drawn from its nodes, each at its own pose,
    // in place of the model built whole at rest: every level-0 slot of each variant its
    // stages draw, one shown at a time (docs/26, "What a damaged node, a destroyed part and
    // a dead unit draw"). A robot's nodes move with it, and a knocked-off part flies.
    let mut targets = Vec::new();
    for t in noded {
        let target = &play.battle.combat.targets[t];
        for (p, part) in target.parts.iter().enumerate() {
            let Some(loaded) = play.battle.meshes.get(t).and_then(|m| m.get(p)) else { continue };
            for node in 0..part.mesh.nodes.len() {
                let stages = part.life.as_ref().and_then(|l| l.nodes.get(node)).map_or(1, |l| l.stages);
                for variant in 0..usize::from(stages) {
                    let Some(model) = models::build_node(loaded, node, variant, |name| store.look(name))?
                    else {
                        continue;
                    };
                    objects.models.push(model);
                    objects.instances.push(models::Instance {
                        model: objects.models.len() - 1,
                        position: [0.0; 3],
                        rotation: 0.0,
                        scale: 1.0,
                        hidden: true,
                    });
                    objects.placed.push(usize::MAX);
                    targets.push((objects.instances.len() - 1, t, p, node, variant));
                }
            }
        }
    }
    Ok(OwnView { nodes, targets })
}

/// Put each node of the hero's own view where the hero's pose has it this frame: the
/// chassis playing its frames, the turret its channels, as the eye is placed.
pub fn place_own_view(
    renderer: &mut parkan_render::Renderer,
    queue: &wgpu::Queue,
    view: &OwnView,
    play: &Play,
) {
    use glam::{Mat4, Quat};
    let hero = &play.hero;
    let t = hero.time_ms;
    let (position, yaw) = hero.walker.drawn(t);
    let unit = Mat4::from_translation(position) * Mat4::from_quat(Quat::from_rotation_z(yaw));
    let mount = hero.mount();
    for &(instance, part, node) in &view.nodes {
        let pose = match part {
            Mount::Chassis => hero.chassis_pose(node),
            Mount::Turret => hero.turret_node(&mount, node),
        };
        renderer.set_instance(queue, instance, unit * models::pose_matrix(&pose), true);
    }
    for &(instance, t, p, node, variant) in &view.targets {
        let Some(part) = play.battle.combat.targets.get(t).and_then(|target| target.parts.get(p)) else {
            continue;
        };
        let shown = match part.life.as_ref().and_then(|l| l.nodes.get(node)) {
            Some(life) => !life.hidden() && life.block() == variant,
            None => variant == 0,
        };
        let visible = shown && !play.deleted.get(t).copied().unwrap_or(false);
        let matrix = models::pose_matrix(&part.nodes[node]) * glam::Mat4::from_scale(Vec3::splat(part.scale));
        renderer.set_instance(queue, instance, matrix, visible);
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
        K::Tab => "SCAN_TAB",
        K::Backquote => "SCAN_TILDA",
        K::Enter => "SCAN_W_ENTER",
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
