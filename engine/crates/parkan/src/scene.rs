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
    /// The sun's and the moon's own textures, resolved as the world loads so the store
    /// uploads them with the rest; `None` where the mission names no `sky.wea` slot.
    pub body_textures: [Option<usize>; 2],
    /// Each body's cell in that texture, `(u0, v0, du, dv)`.
    pub body_cells: [[f32; 4]; 2],
    /// The nebula's and the clouds' textures, `sky.wea` slots 0 and 2, resolved with the
    /// bodies'. Both materials take the whole texture on all 29 shipped missions.
    pub sky_layers: parkan_render::dome::Layers,
    /// The two lens-flare slots, 5 and 6: `SUN.0` cells 1 and 3 on all 29.
    pub flare_looks: [parkan_render::flare::Look; 2],
}

pub fn world(game: &Path, loaded: &Loaded) -> Result<World> {
    let mut store = TextureStore::open(game)?;
    let mut terrain = terrain::build(&terrain::map_dir(game, &loaded.mission.map_path)?, &mut store)?;
    let mut assembly = Assembly::new(game)?;
    let footings = parkan_world::basement::footings(&mut assembly, &loaded.mission, &terrain.land);
    terrain.place_buildings(&footings, &mut store)?;
    let objects = models::build(&mut assembly, &mut store, &loaded.mission)?;
    // The sibling `sky.wea` names the nine slots' materials, the body sprites' among them
    // (docs/10-sky.md, "The sibling `sky.wea`"); it is a separate file, so `parse` never
    // holds it and a mission without one simply draws no sun.
    let atmosphere = gamedir::resolve(&loaded.dir, "sky.ske")
        .and_then(|p| std::fs::read(&p).ok().map(|b| (b, p)))
        .and_then(|(b, p)| sky::parse(&b, &p.display().to_string()).ok())
        .map(|mut a| {
            a.textures = gamedir::resolve(&loaded.dir, "sky.wea")
                .and_then(|p| std::fs::read(&p).ok())
                .map(|b| parkan_formats::wea::parse(&b).materials)
                .unwrap_or_default();
            a
        });
    // Resolved here rather than at draw time: a texture index means nothing until the
    // store has uploaded it, and the upload takes the store whole.
    let looks = [parkan_sim::sky::Body::Sun, parkan_sim::sky::Body::Moon].map(|body| {
        atmosphere
            .as_ref()
            .and_then(|a| parkan_sim::sky::body_texture(a, body))
            .map(str::to_owned)
            .and_then(|name| store.look(&name).ok())
    });
    let body_textures = looks.each_ref().map(|l| l.as_ref().and_then(|l| l.still.texture));
    let body_cells = looks.each_ref().map(|l| l.as_ref().map_or([0.0, 0.0, 1.0, 1.0], |l| l.still.cell));
    // The rest of the sky's own slots, resolved the same way and at the same moment. The
    // stars, slot 1, are deliberately not among them: the game builds them and draws them
    // nowhere (`docs/10-sky.md`, "The stars are built and never drawn").
    let mut role = |role: &str| {
        atmosphere
            .as_ref()
            .and_then(|a| parkan_sim::sky::role_texture(a, role))
            .map(str::to_owned)
            .and_then(|name| store.look(&name).ok())
    };
    let sky_layers = parkan_render::dome::Layers {
        nebula: role("nebula").and_then(|l| l.still.texture),
        clouds: role("clouds").and_then(|l| l.still.texture),
        cloud_tint: [1.0; 4],
    };
    let flare_looks = ["flare", "flare2"].map(|r| {
        role(r).map_or_else(Default::default, |l| parkan_render::flare::Look {
            texture: l.still.texture,
            cell: l.still.cell,
        })
    });
    Ok(World { store, terrain, objects, atmosphere, body_textures, body_cells, sky_layers, flare_looks })
}

/// The bodies to draw this frame: each one up, along its own arc, with its slot's texture.
///
/// STAND-IN: docs/10-sky.md#not-resolved -- what tints a body's sprite is not read; it is
/// drawn in its texture's own colours.
pub fn body_sprites(world: &World, seconds: f64) -> Vec<parkan_render::body::Sprite> {
    let Some(a) = world.atmosphere.as_ref() else {
        return Vec::new();
    };
    parkan_sim::sky::bodies_aloft(a, seconds)
        .into_iter()
        .map(|(body, progress)| parkan_render::body::Sprite {
            toward: body.direction(progress),
            extent: parkan_sim::sky::at(a, parkan_sim::sky::position(a, seconds))
                .map_or([1.0, 1.0], |s| s.body_extent),
            texture: world.body_textures[body.slot() - parkan_sim::sky::Body::Sun.slot()],
            cell: world.body_cells[body.slot() - parkan_sim::sky::Body::Sun.slot()],
            tint: [1.0; 4],
        })
        .collect()
}

pub use parkan_world::play::Play;

/// The lights, fog and dome colours at `seconds` into the mission, seen from `eye`
/// looking along `forward` with a field of view `field` radians across; `None` without an
/// atmosphere.
///
/// The clock starts at the file's closing time and plays the sections in turn; the sun
/// object's two lights shine while a body is up, the first along the body's travel and the
/// second against it (docs/10-sky.md).
pub fn lighting(
    world: &World,
    seconds: f64,
    eye: Vec3,
    forward: Vec3,
    field: f32,
) -> Option<(parkan_render::frame::Lighting, Vec<[f32; 4]>)> {
    use parkan_render::frame::{Light, linear};
    use parkan_sim::sky as atm;
    let a = world.atmosphere.as_ref()?;
    let now = atm::position(a, seconds);
    let sky = atm::at(a, now)?;
    // The heading is the compass heading of the camera matrix's first column, 0 along +y
    // turning towards +x (Terrain.dll:0x100850f0); that column is the view direction --
    // Ngi32.dll's view builder makes it the view's depth axis (0x10009450).
    let heading = forward.x.atan2(forward.y);
    let fog = sky.fog_colour(heading);
    // No shipped section has the sun and the moon up at once; where two bodies are up,
    // the first started lights the scene.
    let lights = match atm::bodies_aloft(a, seconds).first() {
        Some(&(body, progress)) => {
            // CSun sets both directions every takt (Terrain.dll:0x1007ed40): the first
            // light travels the way the body's light does, the second the opposite way.
            let toward = body.direction(progress);
            let [main, second] = atm::sun_lights(&sky, toward, forward);
            [Light { direction: -toward, colour: main }, Light { direction: toward, colour: second }]
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
        field,
        clock_ms: seconds * 1000.0,
        // The effects' point lights follow, once the battle is synced ([`sync`]).
        points: Default::default(),
    };
    let colours = sky
        .dome_colours(fog)
        .into_iter()
        .map(|[r, g, b, a]| {
            let [r, g, b] = linear([r, g, b]);
            [r, g, b, a]
        })
        .collect();
    Some((lighting, colours))
}

/// The sky's textured layers this frame: the mission's own nebula and clouds, the clouds
/// tinted by slot 18 (`docs/10-sky.md`, "The three layers and their texture coordinates").
/// The stars are not among them -- the game does not draw those.
pub fn sky_layers(world: &World, seconds: f64) -> parkan_render::dome::Layers {
    use parkan_render::frame::linear;
    let mut layers = world.sky_layers;
    layers.cloud_tint = world
        .atmosphere
        .as_ref()
        .and_then(|a| parkan_sim::sky::at(a, parkan_sim::sky::position(a, seconds)))
        .map_or([1.0; 4], |s| {
            let [r, g, b, a] = s.cloud_colour;
            let [r, g, b] = linear([r, g, b]);
            [r, g, b, a]
        });
    layers
}

/// This frame's lens flare, or `None` where no body is up.
///
/// The second of its two gates is here, on how high the body stands (`docs/10-sky.md`,
/// "The lens flare"); the first is on the view, so the renderer takes it from the camera
/// the frame is actually drawn with.
pub fn flare(world: &World, seconds: f64) -> Option<parkan_render::flare::Flare> {
    use parkan_sim::sky as atm;
    let a = world.atmosphere.as_ref()?;
    let &(body, progress) = atm::bodies_aloft(a, seconds).first()?;
    let toward = body.direction(progress);
    let height_gate = atm::flare_height_gate(toward.z);
    Some(parkan_render::flare::Flare { toward, height_gate, looks: world.flare_looks })
}

/// The mission's play, with the hero's view held steady against its gait unless `--sway`,
/// a captured bot standing by unless `--capture-idle`, a building holding its fire below its
/// turret's reach unless `--fire-below`, at `--level`'s game level where one is given, and its
/// progression when its script and messages load.
pub fn play(game: &Path, loaded: &Loaded, args: &crate::Args) -> Result<Option<Play>> {
    let mut play = Play::load_at(game, &loaded.mission, args.level)?;
    if let Some(p) = play.as_mut() {
        p.hero.steady = !args.sway;
        p.capture_standby = !args.capture_idle;
        p.building_fire_floor = !args.fire_below;
        if args.god_mode {
            p.god_mode();
        }
        if let Err(e) = p.load_progression(game, &loaded.dir, &loaded.mission) {
            eprintln!("no mission progression: {e:#}");
        }
        p.load_weather(&loaded.dir);
    }
    Ok(play)
}

/// The game's own key chords (`addition.man`), or none when the file does not load.
pub fn bindings(game: &Path) -> Vec<parkan_formats::controls::Binding> {
    use parkan_formats::controls;
    gamedir::resolve(game, controls::GAME_BINDINGS)
        .and_then(|p| std::fs::read(&p).ok().map(|b| (b, p)))
        .and_then(|(b, p)| controls::bindings(&b, &p.display().to_string()).ok())
        .unwrap_or_default()
}

/// The outcome panel's font slots: its title's `MENU_FONT` and its lines' `GAME_FONT`.
pub const PANEL_TITLE_SLOT: usize = 0;
pub const PANEL_LINES_SLOT: usize = 1;

/// The outcome panel on `space` (`iron3d.dll:0x1009f8b0`, docs/34, "After the outcome"), in
/// place of the HUD: a black box at 60% and, centred on x 320 of the 640 × 480 layout, pinned
/// to the screen's top middle, the title in `menu` at y 50, green won and red lost, and the
/// lines in `game`, grey, from y 75 one font height + 2 apart. The box runs from (x₀, 35) to
/// (640 − x₀, the pen after the last line + 15), x₀ being the leftmost text's x less 30.
/// Returns the box, the title's run and the lines' runs. On the recording of Mission 01's win
/// the box runs 225 to 415 by 35 to about 98.
pub fn outcome_panel(
    panel: &parkan_world::progress::OutcomePanel,
    menu: &parkan_world::text::GameFont,
    game: &parkan_world::text::GameFont,
    space: parkan_world::hud::Space,
) -> (Vec<parkan_render::hud::Rect>, Vec<parkan_world::text::TextRun>, Vec<parkan_world::text::TextRun>) {
    use parkan_world::text::{Align, TextRun};
    let s = space.scale();
    let at = |x: f32, y: f32| space.ndc([x, y], parkan_world::hud::Pin::TOP);
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
    screen: parkan_world::hud::Space,
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

/// Draw the briefing's screen over the flythrough (docs/21-briefing.md, "The screen"): the
/// fade and the bars, the title in `MENU_FONT` and the subtitle in `GAME_FONT`.
pub fn draw_briefing(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    briefing: &parkan_world::briefing::Briefing,
    screen: parkan_world::hud::Space,
) {
    let Some(laid) = renderer.font_slot(PANEL_LINES_SLOT).map(|game| briefing.screen(screen, game)) else {
        return;
    };
    let rects: Vec<parkan_render::hud::Rect> = laid
        .fills
        .iter()
        .map(|f| parkan_render::hud::Rect { min: f.min, max: f.max, colour: f.colour })
        .collect();
    renderer.set_hud(device, queue, &rects);
    renderer.set_text_slot(device, queue, PANEL_TITLE_SLOT, &laid.title);
    renderer.set_text_slot(device, queue, PANEL_LINES_SLOT, &laid.lines);
}

/// Take the briefing's screen away.
pub fn clear_briefing(renderer: &mut parkan_render::Renderer, device: &wgpu::Device, queue: &wgpu::Queue) {
    renderer.set_hud(device, queue, &[]);
    renderer.set_text_slot(device, queue, PANEL_TITLE_SLOT, &[]);
    renderer.set_text_slot(device, queue, PANEL_LINES_SLOT, &[]);
}

/// The font slots the cockpit's text is drawn in: `GAME_FONT`'s, and `MENU_FONT`'s over it.
pub const HUD_TEXT_SLOT: usize = 2;
pub const HUD_MENU_SLOT: usize = 3;
/// The slot the tooltip's text draws in, `TOOL_FONT`, over its box (docs/37, "A tooltip").
pub const HUD_TIP_SLOT: usize = parkan_render::TIP_TEXT_SLOT;

/// The cockpit HUD (docs/35-hud.md): its state, and `GAME_FONT` and `MENU_FONT` to lay its text
/// out in.
pub struct Hud {
    pub cockpit: parkan_world::cockpit::Cockpit,
    pub font: parkan_world::text::GameFont,
    pub menu: parkan_world::text::GameFont,
    /// `--stretch-hud`: the layout stretches to the screen as the game's does.
    pub stretch: bool,
    /// The textures the previews' models draw with, and what the previews show now.
    pub preview_store: Option<TextureStore>,
    pub preview_keys: Vec<parkan_world::cockpit::designer::PreviewKey>,
    /// `TOOL_FONT`, the game's `+0x18`, which a tooltip is drawn in, and the real time in ms
    /// the tooltip's timer reads, as the game's reads `timeGetTime`.
    pub tool: parkan_world::text::GameFont,
    pub clock_ms: f64,
}

/// The cockpit for `play` in `mission_dir`: the interface's pages and the mission's minimap
/// into `renderer`, `GAME_FONT` and `MENU_FONT` into their slots.
pub fn hud(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    game: &Path,
    mission_dir: &Path,
    play: &Play,
    args: &crate::Args,
) -> Result<Hud> {
    use parkan_world::text::GameFont;
    let mut pages = parkan_world::hud::Pages::open(game)?;
    if let Err(e) = pages.add_minimap(game, mission_dir) {
        eprintln!("no minimap: {e:#}");
    }
    renderer.set_ui_pages(device, queue, &pages.pages);
    renderer.set_font_slot(device, queue, HUD_TEXT_SLOT, GameFont::ui(game, "GAME_FONT")?);
    renderer.set_font_slot(device, queue, HUD_MENU_SLOT, GameFont::ui(game, "MENU_FONT")?);
    renderer.set_font_slot(device, queue, HUD_TIP_SLOT, GameFont::ui(game, "TOOL_FONT")?);
    let mut cockpit = parkan_world::cockpit::Cockpit::open(game, &pages, play)?;
    // The designer loads from the game's `units/` (docs/37, "The buttons") and saves to the
    // player's own folder, or with `--save-to-game` to `units/` as the game does.
    use parkan_formats::userdir;
    let units =
        parkan_formats::gamedir::resolve(game, userdir::UNITS).unwrap_or_else(|| game.join(userdir::UNITS));
    cockpit.designer.saves =
        if args.save_to_game { Some(units.clone()) } else { userdir::resolve(userdir::UNITS) };
    cockpit.designer.units = Some(units);
    Ok(Hud {
        cockpit,
        font: GameFont::ui(game, "GAME_FONT")?,
        menu: GameFont::ui(game, "MENU_FONT")?,
        stretch: args.stretch_hud,
        preview_store: None,
        preview_keys: Vec::new(),
        tool: GameFont::ui(game, "TOOL_FONT")?,
        clock_ms: 0.0,
    })
}

/// This frame's cockpit, or none while `shown` is false: its art, its text and the views of
/// the units its panels hold. Returns what it asks to be heard.
#[allow(clippy::too_many_arguments)]
pub fn draw_hud(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    hud: &mut Hud,
    play: &mut Play,
    view: &OwnView,
    (width, height): (u32, u32),
    view_proj: glam::Mat4,
    lighting: Option<parkan_render::frame::Lighting>,
    shown: bool,
) -> (Vec<&'static str>, Vec<&'static str>) {
    if !shown {
        renderer.set_ui(device, queue, (width, height), &[]);
        renderer.set_text_slot(device, queue, HUD_TEXT_SLOT, &[]);
        renderer.set_text_slot(device, queue, HUD_MENU_SLOT, &[]);
        renderer.set_text_slot(device, queue, HUD_TIP_SLOT, &[]);
        renderer.set_views(device, Vec::new());
        return (Vec::new(), Vec::new());
    }
    let space = parkan_world::hud::Space {
        stretch: hud.stretch,
        ..parkan_world::hud::Space::new(width as f32, height as f32)
    };
    hud.cockpit.update(play, play.hero.time_ms);
    let mut drawn = hud.cockpit.draw(play, space, &hud.font, &hud.menu, view_proj);
    let (tip, tip_text) = hud.cockpit.tooltip(space, &hud.tool, hud.clock_ms);
    drawn.batches.extend(tip);
    renderer.set_ui(device, queue, (width, height), &drawn.batches);
    renderer.set_text_slot(device, queue, HUD_TEXT_SLOT, &drawn.text);
    renderer.set_text_slot(device, queue, HUD_MENU_SLOT, &drawn.menu_text);
    renderer.set_text_slot(device, queue, HUD_TIP_SLOT, &tip_text);
    let views = drawn
        .views
        .iter()
        .map(|v| {
            // The view has no fog: it stands a unit's width from what it shows.
            let lighting = parkan_render::frame::Lighting {
                fog_end: f32::MAX,
                fog_start: f32::MAX,
                eye: v.eye,
                ..lighting.unwrap_or_default()
            };
            let (instances, paints) = unit_instances(renderer, queue, view, play, v.unit);
            parkan_render::ModelView {
                viewport: v.viewport,
                view_proj: v.view_proj(),
                lighting,
                instances,
                paints: Some(paints),
                previews: false,
                under_hud: false,
            }
        })
        .collect();
    let mut views: Vec<parkan_render::ModelView> = views;
    views.extend(previews(
        renderer,
        device,
        queue,
        hud,
        play,
        &drawn.previews,
        lighting.map(|l| l.scene_colour),
    ));
    renderer.set_views(device, views);
    (drawn.voices, drawn.sounds)
}

/// The designer's and the factory screen's model views: each shown model built once while
/// it stays on screen, into objects of their own, lit from the two sides the constructor
/// lights them from (docs/37, "The previews").
fn previews(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    hud: &mut Hud,
    play: &mut Play,
    shown: &[parkan_world::cockpit::designer::Preview],
    scene_colour: Option<[f32; 3]>,
) -> Vec<parkan_render::ModelView> {
    let keys: Vec<_> = shown.iter().map(|p| p.key.clone()).collect();
    if keys != hud.preview_keys {
        hud.preview_keys = keys.clone();
        if hud.preview_store.is_none() {
            hud.preview_store =
                TextureStore::open(&play.assembly.game).map_err(|e| eprintln!("no previews: {e:#}")).ok();
        }
        let Some(store) = hud.preview_store.as_mut() else { return Vec::new() };
        let mut objects = Objects { models: Vec::new(), instances: Vec::new(), placed: Vec::new() };
        for key in &keys {
            let model = models::build_model(&mut play.assembly, store, key.kind, &key.path)
                .map_err(|e| eprintln!("no preview of {}: {e:#}", key.path))
                .ok()
                .flatten()
                .unwrap_or_default();
            objects.models.push(model);
            objects.instances.push(models::Instance {
                model: objects.models.len() - 1,
                position: [0.0; 3],
                rotation: 0.0,
                scale: 1.0,
                hidden: false,
            });
            objects.placed.push(usize::MAX);
        }
        renderer.set_previews(device, queue, &store.textures, &objects);
    }
    shown
        .iter()
        .enumerate()
        .map(|(i, p)| {
            renderer.set_preview_instance(queue, i, p.model, true);
            // The two lights turn with the model and are (2, 2, 2) each (docs/37, "The
            // previews").
            let [a, b] = p.lights.map(|d| p.model.transform_vector3(d).normalize_or(glam::Vec3::NEG_Z));
            let colour = [parkan_world::cockpit::designer::PREVIEW_LIGHT_COLOUR; 3];
            let light = |direction| parkan_render::frame::Light { direction, colour };
            parkan_render::ModelView {
                viewport: p.viewport,
                view_proj: p.view_proj,
                lighting: parkan_render::frame::Lighting {
                    lights: [light(a), light(b)],
                    // The scene colour is one for the whole game, the shader singleton's, which
                    // the sky's takt sends every takt whether or not the world is drawn, and
                    // which the model view's draw copies as the world's does (docs/37, "The
                    // previews"): the world's this frame, or the shader's own 0.2 before any.
                    scene_colour: scene_colour.unwrap_or(parkan_render::frame::SHADER_SCENE_COLOUR),
                    fog_start: f32::MAX,
                    fog_end: f32::MAX,
                    eye: glam::Vec3::ZERO,
                    ..Default::default()
                },
                instances: vec![i],
                paints: p.paint.map(|c| vec![c]),
                previews: true,
                // A building being placed shows over the world and under the HUD (docs/32).
                under_hud: p.paint.is_some(),
            }
        })
        .collect()
}

/// The instances that draw `unit` (a battle target, or the hero with none) as its nodes stand
/// now, each with its node's colour by its life (docs/35-hud.md, "The unit in the middle").
fn unit_instances(
    renderer: &parkan_render::Renderer,
    queue: &wgpu::Queue,
    view: &OwnView,
    play: &Play,
    unit: Option<usize>,
) -> (Vec<usize>, Vec<[f32; 3]>) {
    use parkan_world::cockpit::panels::node_colour;
    let colour = |life: Option<&parkan_sim::damage::NodeLife>| {
        node_colour(life.map_or(1.0, |l| if l.max > 0.0 { l.life / l.max } else { 1.0 }))
    };
    let mut instances = Vec::new();
    let mut paints = Vec::new();
    match unit {
        None => {
            let hero = &play.hero;
            let (position, _) = hero.walker.drawn(hero.time_ms);
            let placed = glam::Mat4::from_translation(position)
                * glam::Mat4::from_quat(hero.walker.drawn_turn(hero.time_ms));
            let mount = hero.mount();
            for &(instance, part, node, variant) in &view.outside {
                let index = match part {
                    Mount::Chassis => hero.chassis_part,
                    Mount::Turret => hero.turret_part,
                };
                let life = hero.lives.get(index).and_then(Option::as_ref).and_then(|l| l.nodes.get(node));
                let shown = life.map_or(variant == 0, |l| !l.hidden() && l.block() == variant);
                if !shown {
                    continue;
                }
                let pose = match part {
                    Mount::Chassis => hero.chassis_pose(node),
                    Mount::Turret => hero.turret_node(&mount, node),
                };
                renderer.move_instance(queue, instance, placed * models::pose_matrix(&pose));
                instances.push(instance);
                paints.push(colour(life));
            }
        }
        Some(t) => {
            let Some(target) = play.battle.combat.targets.get(t) else { return (instances, paints) };
            let noded: Vec<_> = view.targets.iter().filter(|e| e.1 == t).collect();
            if noded.is_empty() {
                let object = play.battle.objects[t];
                if let Some(i) = renderer_placed(view, object) {
                    instances.push(i);
                    paints.push(colour(None));
                }
            }
            for &&(instance, _, p, node, variant) in &noded {
                let Some(part) = target.parts.get(p) else { continue };
                let life = part.life.as_ref().and_then(|l| l.nodes.get(node));
                if life.map_or(variant == 0, |l| !l.hidden() && l.block() == variant) {
                    instances.push(instance);
                    paints.push(colour(life));
                }
            }
        }
    }
    (instances, paints)
}

/// The instance a whole placed object draws with, if it has one.
fn renderer_placed(view: &OwnView, object: usize) -> Option<usize> {
    view.placed.iter().position(|&p| p == object)
}

/// The looks the effects draw with, for the renderer.
pub fn sprite_looks(play: &Play) -> Vec<parkan_render::sprites::SpriteLook> {
    play.fx
        .looks
        .iter()
        .map(|l| parkan_render::sprites::SpriteLook {
            texture: l.texture,
            blend_mode: l.blend_mode,
            ambient: l.ambient,
            cell: l.cell,
        })
        .collect()
}

/// The owner an instance carries for the point lights that light their owner alone
/// (`Terrain.dll:0x10047a52`): the hero's, and a battle target's.
const HERO_OWNER: u32 = 1;
fn target_owner(target: usize) -> u32 {
    target as u32 + 2
}

/// This frame's point lights for the renderer: the effects', each with the owner it lights
/// alone or 0 (docs/11, "What a light does to a surface").
fn point_lights(play: &Play) -> Vec<parkan_render::frame::PointLight> {
    use parkan_world::play::Lit;
    play.vertex_lights()
        .into_iter()
        .map(|d| parkan_render::frame::PointLight {
            position: d.light.position,
            range: d.light.range,
            colour: d.light.colour.to_array(),
            attenuation: d.light.attenuation,
            owner: match d.lit {
                Lit::Everything => 0,
                Lit::Hero => HERO_OWNER,
                Lit::Target(t) => target_owner(t),
            },
        })
        .collect()
}

/// Bring the drawing up to date with the battle: hide what died, place the rounds, and
/// hand over this frame's effect sprites and point lights as seen from `eye`.
///
/// `weather` is the camera the rain and the snow are kept about and drawn through, where the
/// frame is drawn through one the game's weather would draw in.
#[allow(clippy::too_many_arguments)]
pub fn sync(
    renderer: &mut parkan_render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    play: &mut Play,
    objects: &Objects,
    view_proj: glam::Mat4,
    eye: Vec3,
    weather: Option<&parkan_sim::weather::View>,
) {
    let quads: Vec<parkan_render::sprites::Quad> = play
        .sprites(eye)
        .into_iter()
        .flat_map(|(look, s)| {
            use parkan_render::sprites::{
                EFFECTS_LAYER, PLAIN_TINT, Quad, UNFOGGED_TINT, billboard, dome, framed, lengthwise, turned,
            };
            let tint = if s.unfogged { UNFOGGED_TINT } else { PLAIN_TINT };
            // A type-3, 4 or 9 sprite is drawn with its own matrix: a quad across its x and y,
            // or a type-9 block's hemisphere with its pole on its z (docs/11, "A sprite's mode").
            if let Some(m) = s.matrix {
                let pieces = match s.dome {
                    Some(d) => dome(s.centre, m, d.segments, d.rings, d.projected),
                    None => vec![turned(s.centre, m)],
                };
                return pieces
                    .into_iter()
                    .map(|(corners, uv)| Quad {
                        look,
                        corners,
                        alpha: s.alpha,
                        overlay: s.overlay,
                        uv: Some(uv),
                        depth: s.centre.distance(eye),
                        layer: EFFECTS_LAYER,
                        tint,
                    })
                    .collect::<Vec<_>>();
            }
            let corners = match s.frame {
                Some(axes) => framed(s.centre, axes, eye),
                None => billboard(s.centre, s.along, s.width, s.height, eye),
            };
            vec![Quad {
                look,
                corners: if s.lengthwise { lengthwise(corners) } else { corners },
                alpha: s.alpha,
                overlay: s.overlay,
                uv: None,
                depth: s.centre.distance(eye),
                layer: EFFECTS_LAYER,
                tint,
            }]
        })
        .collect();
    let mut quads = quads;
    // The effects' point lights on the landscape, each disc a fan from its first corner
    // (`Terrain.dll:0x1002afc3`), filed before the sprites.
    for disc in play.light_discs(eye, view_proj) {
        use parkan_render::sprites::{LIGHTS_LAYER, Quad};
        let [r, g, b] = disc.tint;
        let first = disc.polygon[0];
        for pair in disc.polygon[1..].windows(2) {
            let (b1, c1) = (pair[0], pair[1]);
            quads.push(Quad {
                look: disc.look,
                corners: [first.0, b1.0, c1.0, c1.0],
                alpha: disc.alpha,
                overlay: false,
                uv: Some([first.1, b1.1, c1.1, c1.1]),
                depth: first.0.distance(eye),
                layer: LIGHTS_LAYER,
                tint: [r, g, b, 1.0],
            });
        }
    }
    // The rain and the snow over everything in the world, the depth test off: screen-space
    // quads filed in group 1, layer 8, in the pass the world's draw ends with
    // (`Terrain.dll:0x100754dc`, docs/10, "The weather"). Each corner is the world point drawn
    // that many pixels from its particle.
    if let Some(view) = weather {
        use parkan_render::sprites::{Quad, WEATHER_LAYER};
        for d in play.weather_specks(view) {
            let [r, g, b, alpha] = d.colour;
            quads.push(Quad {
                look: d.look,
                corners: std::array::from_fn(|i| view.nudged(d.speck.anchors[i], d.speck.offsets[i])),
                alpha,
                overlay: true,
                uv: Some(d.speck.uv),
                depth: 0.0,
                layer: WEATHER_LAYER,
                tint: [r, g, b, 2.0],
            });
        }
    }
    renderer.set_sprites(device, queue, view_proj, &quads);
    // The same lights as the shade's lighter takes them, and whose each whole placed object is.
    renderer.set_point_lights(&point_lights(play), eye);
    let target_of: std::collections::HashMap<usize, usize> =
        play.battle.objects.iter().enumerate().map(|(t, &object)| (object, t)).collect();
    for (i, object) in objects.placed.iter().enumerate() {
        if let Some(&t) = target_of.get(object) {
            renderer.set_instance_owner(queue, i, target_owner(t));
        }
    }
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
#[derive(Clone)]
pub struct OwnView {
    nodes: Vec<(usize, Mount, usize)>,
    /// The hero from outside, for its own panel: an instance for each level-0 slot of each
    /// variant its nodes' stages draw, hidden from the scene.
    outside: Vec<(usize, Mount, usize, usize)>,
    /// Which object each instance places, `usize::MAX` for a node's.
    placed: Vec<usize>,
    /// Every target with node life drawn node by node: an instance, the target, its part,
    /// the node and the variant that instance draws.
    targets: Vec<(usize, usize, usize, usize, usize)>,
    /// A boarded bot's own view: an instance for each node's fifth slot, with the target, its
    /// part and the node (docs/39, "The view and the HUD").
    cockpits: Vec<(usize, usize, usize, usize)>,
}

/// Draw the hero from now on as its own view does (docs/07-objects.md, "The fifth slot is
/// what the unit's own view draws"): every node's slot `variant × 5 + 4`, the cockpit and
/// whatever of the hull and guns has one, and nothing of a node without. The hero's
/// placement, its level 0, leaves `objects`. The chassis and the turret are the meshes
/// the hero poses; Mission 01's hero has no other part.
///
/// STAND-IN: docs/07-objects.md#a-layer-is-a-pass-and-the-cockpits-two-are-a-frustum-of-their-own--read
/// -- the fifth slots draw here with the scene, depth-tested, lit and fogged like any
/// model. The game gives layers 9 and 10 a frustum of their own -- near 0.05, far 10,
/// viewport z 0.0 to 0.1, against the world's 0.5, 700 and 0.1 to 0.99 -- and draws them
/// after the scene, so a cockpit close to the eye is never clipped by the world's near
/// plane and never occluded by anything in the world.
pub fn own_view(objects: &mut Objects, store: &mut TextureStore, play: &Play) -> Result<OwnView> {
    // Every whole model drawn node by node leaves first: removing one shifts the instances
    // after it, so no node's instance may be counted before the last removal.
    let noded: Vec<usize> =
        (0..play.battle.combat.targets.len()).filter(|&t| play.drawn_by_node(t)).collect();
    let leaving: Vec<usize> =
        std::iter::once(play.hero.object).chain(noded.iter().map(|&t| play.battle.objects[t])).collect();
    for object in leaving {
        if let Some(i) = objects.placed.iter().position(|&p| p == object) {
            objects.placed.remove(i);
            objects.instances.remove(i);
        }
    }
    // The hero's record writes its clan's sign as every unit's does (docs/07, "Who picks an
    // object mesh's material track"). No hero mesh carries a material with a second track.
    let hero_track = usize::try_from(play.player_clan).unwrap_or(0);
    let mut nodes = Vec::new();
    for (mount, loaded) in [(Mount::Chassis, &play.hero.chassis), (Mount::Turret, &play.hero.turret)] {
        for node in 0..loaded.mesh.nodes.len() {
            let skins = &mut models::OnTrack { skins: &mut *store, track: hero_track };
            let Some(model) = models::build_view_node(loaded, node, 0, skins)? else {
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
    let mut outside = Vec::new();
    for (mount, loaded, part) in [
        (Mount::Chassis, &play.hero.chassis, play.hero.chassis_part),
        (Mount::Turret, &play.hero.turret, play.hero.turret_part),
    ] {
        let life = play.hero.lives.get(part).and_then(Option::as_ref);
        for node in 0..loaded.mesh.nodes.len() {
            let stages = life.and_then(|l| l.nodes.get(node)).map_or(1, |l| l.stages);
            for variant in 0..usize::from(stages) {
                let skins = &mut models::OnTrack { skins: &mut *store, track: hero_track };
                let Some(model) = models::build_node(loaded, node, variant, skins)? else {
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
                outside.push((objects.instances.len() - 1, mount, node, variant));
            }
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
                    let skins = &mut models::OnTrack { skins: &mut *store, track: play.insignia(t) };
                    let Some(model) = models::build_node(loaded, node, variant, skins)? else {
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
    Ok(OwnView { nodes, outside, placed: objects.placed.clone(), targets, cockpits: Vec::new() })
}

/// Units the play has made since the last call, drawn node by node as `own_view` draws a
/// target, and the insignia of what changed owner: whether any was added or wears another
/// emblem, and the world must be uploaded again.
pub fn add_targets(
    objects: &mut Objects,
    store: &mut TextureStore,
    view: &mut OwnView,
    play: &mut Play,
) -> Result<bool> {
    let mut added = std::mem::take(&mut play.added);
    // A boarded bot's cockpit, or a tower's in its manual control, the first time it is driven.
    if let Some(t) = play.driving.as_ref().map(|d| d.target)
        && !view.cockpits.iter().any(|c| c.1 == t)
        && let Some(robot) = play.machine(t)
    {
        for (p, part) in robot.parts.iter().enumerate() {
            for node in 0..part.mesh.mesh.nodes.len() {
                let skins = &mut models::OnTrack { skins: &mut *store, track: play.insignia(t) };
                let Some(model) = models::build_view_node(&part.mesh, node, 0, skins)? else { continue };
                objects.models.push(model);
                objects.instances.push(models::Instance {
                    model: objects.models.len() - 1,
                    position: [0.0; 3],
                    rotation: 0.0,
                    scale: 1.0,
                    hidden: true,
                });
                objects.placed.push(usize::MAX);
                view.placed.push(usize::MAX);
                view.cockpits.push((objects.instances.len() - 1, t, p, node));
            }
        }
        if view.cockpits.iter().any(|c| c.1 == t) {
            added.push(usize::MAX);
        }
    }
    let added: Vec<usize> = added;
    // A captured target's nodes draw on its new clan's track from the next frame (docs/07, "Who
    // picks an object mesh's material track"), its own view's among them.
    let mut reskinned = false;
    for t in std::mem::take(&mut play.reskinned) {
        let track = play.insignia(t);
        let nodes = view.targets.iter().filter(|e| e.1 == t).map(|e| e.0);
        let cockpit = view.cockpits.iter().filter(|c| c.1 == t).map(|c| c.0);
        for instance in nodes.chain(cockpit) {
            let model = objects.instances[instance].model;
            reskinned |= objects.models[model].wear_track(&mut *store, track)?;
        }
    }
    for &t in added.iter().filter(|&&t| t != usize::MAX) {
        let target = &play.battle.combat.targets[t];
        for (p, part) in target.parts.iter().enumerate() {
            let Some(loaded) = play.battle.meshes.get(t).and_then(|m| m.get(p)) else { continue };
            for node in 0..part.mesh.nodes.len() {
                let stages = part.life.as_ref().and_then(|l| l.nodes.get(node)).map_or(1, |l| l.stages);
                for variant in 0..usize::from(stages) {
                    let skins = &mut models::OnTrack { skins: &mut *store, track: play.insignia(t) };
                    let Some(model) = models::build_node(loaded, node, variant, skins)? else {
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
                    view.placed.push(usize::MAX);
                    view.targets.push((objects.instances.len() - 1, t, p, node, variant));
                }
            }
        }
    }
    Ok(!added.is_empty() || reskinned)
}

/// Put each node of the hero's own view where the hero's pose has it this frame: the
/// chassis playing its frames, the turret its channels, as the eye is placed. From
/// `outside`, as a briefing's camera sees it, the hero is drawn whole instead: each node's
/// level-0 slot of the variant its stage draws.
pub fn place_own_view(
    renderer: &mut parkan_render::Renderer,
    queue: &wgpu::Queue,
    view: &OwnView,
    play: &Play,
    outside: bool,
) {
    use glam::Mat4;
    // Aboard a bot the hero is out of the world, and the bot's cockpit is drawn in place of
    // its hull (docs/39, "Boarding"). In an HQ's command view the hero stays aboard, and the
    // HQ, driven by its AI, is drawn whole. Driving from afar or at a building's guns, the hero
    // stands in its room and the view is the driven one's.
    let driven = play.driving.as_ref().map(|d| d.target);
    let away = play.hero_away() || driven.is_some();
    let first_person = !outside && !away;
    let outside = outside && !away;
    let hero = &play.hero;
    let t = hero.time_ms;
    let (position, _) = hero.walker.drawn(t);
    let unit = Mat4::from_translation(position) * Mat4::from_quat(hero.walker.drawn_turn(t));
    let mount = hero.mount();
    for &(instance, part, node) in &view.nodes {
        let pose = match part {
            Mount::Chassis => hero.chassis_pose(node),
            Mount::Turret => hero.turret_node(&mount, node),
        };
        renderer.set_instance(queue, instance, unit * models::pose_matrix(&pose), first_person);
        renderer.set_instance_owner(queue, instance, HERO_OWNER);
    }
    // From the outer camera a boarded bot is drawn whole, as any other unit.
    let driven = driven.filter(|_| !play.outer_shows());
    let robot_of = |t: usize| play.machine(t);
    for &(instance, t, p, node) in &view.cockpits {
        let robot = (Some(t) == driven).then(|| robot_of(t)).flatten();
        let placed = robot.map(|r| models::pose_matrix(&r.placement().compose(&r.part_pose(p, node))));
        renderer.set_model_phase(instance, robot.and_then(|r| r.material_phase(p, node)));
        renderer.set_instance(queue, instance, placed.unwrap_or(Mat4::IDENTITY), placed.is_some());
        renderer.set_instance_owner(queue, instance, target_owner(t));
    }
    for &(instance, part, node, variant) in &view.outside {
        let index = match part {
            Mount::Chassis => hero.chassis_part,
            Mount::Turret => hero.turret_part,
        };
        let life = hero.lives.get(index).and_then(Option::as_ref).and_then(|l| l.nodes.get(node));
        let shown = outside && life.map_or(variant == 0, |l| !l.hidden() && l.block() == variant);
        if !shown {
            renderer.set_instance(queue, instance, Mat4::IDENTITY, false);
            continue;
        }
        let pose = match part {
            Mount::Chassis => hero.chassis_pose(node),
            Mount::Turret => hero.turret_node(&mount, node),
        };
        renderer.set_instance(queue, instance, unit * models::pose_matrix(&pose), true);
        renderer.set_instance_owner(queue, instance, HERO_OWNER);
    }
    for &(instance, t, p, node, variant) in &view.targets {
        let Some(part) = play.battle.combat.targets.get(t).and_then(|target| target.parts.get(p)) else {
            continue;
        };
        let shown = match part.life.as_ref().and_then(|l| l.nodes.get(node)) {
            Some(life) => !life.hidden() && life.block() == variant,
            None => variant == 0,
        };
        let visible =
            shown && !play.deleted.get(t).copied().unwrap_or(false) && play.shown(t) && Some(t) != driven;
        let matrix = models::pose_matrix(&part.nodes[node]) * glam::Mat4::from_scale(Vec3::splat(part.scale));
        // A tracked chassis's belt runs with the track under it, not with the world clock:
        // its device plays the material (docs/28, "The belt is a material a channel plays").
        renderer.set_model_phase(instance, robot_of(t).and_then(|r| r.material_phase(p, node)));
        renderer.set_instance(queue, instance, matrix, visible);
        renderer.set_instance_owner(queue, instance, target_owner(t));
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
        K::Numpad5 => "SCAN_G_5",
        K::F1 => "SCAN_F1",
        K::F2 => "SCAN_F2",
        K::F3 => "SCAN_F3",
        K::F7 => "SCAN_F7",
        K::F8 => "SCAN_F8",
        K::F12 => "SCAN_F12",
        K::BracketLeft => "SCAN_LBRACKET",
        K::BracketRight => "SCAN_RBRACKET",
        K::Comma => "SCAN_COMMA",
        K::Period => "SCAN_DOT",
        // The arrows and PageUp and PageDown by the keypad are the `SCAN_G_` keys.
        K::ArrowUp => "SCAN_G_UP",
        K::ArrowDown => "SCAN_G_DOWN",
        K::ArrowLeft => "SCAN_G_LEFT",
        K::ArrowRight => "SCAN_G_RIGHT",
        K::PageUp => "SCAN_G_PGUP",
        K::PageDown => "SCAN_G_PGDN",
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
