//! The cockpit HUD as `iron3d.dll` draws it on its 640 × 480 layout: the weapons list at the
//! top right, the message box at the top, and the target and own-unit panels in the bottom
//! corners. Each frame gives the HUD's art as painter batches, its `GAME_FONT` text, and the
//! views of units its panels hold. See `docs/35-hud.md`.

pub mod commander;
pub mod designer;
pub mod factory;
pub mod map;
pub mod markers;
pub mod messages;
pub mod objectives;
pub mod panels;
pub mod radar;
pub mod research;
pub mod weapons;
pub mod wingmen;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use glam::Mat4;

use crate::hud::{Batch, Pages, Painter, Pin, Skin, Space};
use crate::play::Play;
use crate::text::{GameFont, TextRun};

/// A colour as the game gives one, `0xAARRGGBB`, as RGBA.
pub fn argb(c: u32) -> [f32; 4] {
    let byte = |s: u32| ((c >> s) & 0xFF) as f32 / 255.0;
    [byte(16), byte(8), byte(0), byte(24)]
}

pub const WHITE: u32 = 0xffff_ffff;

/// What a widget draws into: the painter, and the text it lays out in `GAME_FONT` and in
/// `MENU_FONT`.
pub struct Ink<'a> {
    pub painter: Painter,
    pub text: Vec<TextRun>,
    pub font: &'a GameFont,
    pub menu_runs: Vec<TextRun>,
    pub menu: &'a GameFont,
}

impl Ink<'_> {
    /// `text` with its top left at the layout's `at`, under the painter's pin.
    pub fn text(&mut self, text: &str, at: [f32; 2], colour: u32) {
        let space = self.painter.space;
        self.text.push(TextRun {
            colour: argb(colour),
            scale: space.scale(),
            ..TextRun::new(text, space.ndc(at, self.painter.pin))
        });
    }

    /// `text` in `MENU_FONT` with its top left at the layout's `at`, under the painter's pin.
    pub fn menu_text(&mut self, text: &str, at: [f32; 2], colour: u32) {
        let space = self.painter.space;
        self.menu_runs.push(TextRun {
            colour: argb(colour),
            scale: space.scale(),
            ..TextRun::new(text, space.ndc(at, self.painter.pin))
        });
    }

    /// `text` centred across `width` from `x`, as the game centres a string in a box:
    /// x = int((width − its width) × 0.5 + x).
    pub fn centred(&mut self, text: &str, x: f32, width: f32, y: f32, colour: u32) {
        let left = ((width - self.font.advance(text)) * 0.5 + x).trunc();
        self.text(text, [left, y], colour);
    }

    /// `GAME_FONT`'s line step on the layout: its height and 2, rounded (`0x1007fe80`).
    pub fn line_step(&self) -> f32 {
        (self.font.line_height + 2.0).round()
    }
}

/// What the cockpit draws this frame.
#[derive(Clone, Debug, Default)]
pub struct Drawn {
    pub batches: Vec<Batch>,
    /// In `GAME_FONT`, and in `MENU_FONT`.
    pub text: Vec<TextRun>,
    pub menu_text: Vec<TextRun>,
    pub views: Vec<panels::UnitView>,
    /// The designer's and the factory screen's model views.
    pub previews: Vec<designer::Preview>,
    /// The names of `ui/game_resources.cfg`'s voices to queue, and of its sounds to play now.
    pub voices: Vec<&'static str>,
    pub sounds: Vec<&'static str>,
}

/// The cockpit's art, words and what it keeps from frame to frame.
pub struct Cockpit {
    pub skin: Skin,
    pub strings: BTreeMap<u32, String>,
    pub messages: messages::MessageBox,
    pub weapons: weapons::Latches,
    pub panels: panels::Panels,
    /// The `textures` resource's pages by name.
    pub pages: BTreeMap<String, u16>,
    /// The map's water level, which the altitude is measured from.
    pub water_level: f32,
    /// When the radar's sweep ring last started.
    pub ring_since_ms: f64,
    pub objectives: objectives::Screen,
    pub map: map::SatelliteMap,
    pub factory: factory::Screen,
    pub designer: designer::Screen,
    /// The commander panel of command mode.
    pub commander: commander::Panel,
    /// Each part's code in the player clan's research tree, by lower-case record name.
    pub gun_codes: BTreeMap<String, String>,
}

/// Each part's code in the player clan's research tree, by lower-case part name: `IResearch`
/// slot 2 finds the first `TRFB` entry whose part name matches, case folded
/// (`MisLoad.dll:0x10002a40`), and slot 13 gives its item's short code (`0x10002e70`). No part
/// name is listed twice in any of the 29 shipped trees.
pub fn gun_codes(game: &Path, play: &Play) -> BTreeMap<String, String> {
    let tree =
        usize::try_from(play.player_clan).ok().and_then(|c| play.clans.get(c)).map(|c| c.behaviour.clone());
    let Some(tree) = tree.filter(|t| !t.is_empty()) else { return BTreeMap::new() };
    let Some(data) = parkan_formats::gamedir::resolve(game, &tree).and_then(|p| std::fs::read(p).ok()) else {
        return BTreeMap::new();
    };
    let Ok(tree) = parkan_formats::research::parse(&data, &tree) else { return BTreeMap::new() };
    let mut codes = BTreeMap::new();
    for (part, &item) in tree.part_ids.iter().zip(&tree.part_items) {
        if let Some(it) = tree.items.get(item) {
            codes.entry(part.to_ascii_lowercase()).or_insert_with(|| it.code.clone());
        }
    }
    codes
}

impl Cockpit {
    /// The skin from `pages`, the game's strings, and each unit of `play` named.
    pub fn open(game: &Path, pages: &Pages, play: &Play) -> Result<Cockpit> {
        let strings = crate::resources::game_strings(game).unwrap_or_default();
        let panels = panels::Panels::new(play, &strings);
        Ok(Cockpit {
            skin: Skin::open(game, pages)?,
            strings,
            messages: messages::MessageBox::default(),
            weapons: weapons::Latches::default(),
            panels,
            pages: pages.names.iter().map(|(k, &v)| (k.clone(), v as u16)).collect(),
            water_level: play.ground.water_level(),
            ring_since_ms: 0.0,
            objectives: objectives::Screen::default(),
            factory: factory::Screen::default(),
            designer: designer::Screen::default(),
            commander: commander::Panel::default(),
            gun_codes: gun_codes(game, play),
            map: map::SatelliteMap::new(
                crate::settings::value(game, "CS", "MAP_ALPHA").and_then(|v| v.parse().ok()),
            ),
        })
    }

    pub fn string(&self, id: u32) -> &str {
        self.strings.get(&id).map_or("", String::as_str)
    }

    /// What the screens keep current every frame before they draw (`0x1008d5f0`): in command
    /// mode the column's update and the units the unit box shows rated.
    pub fn update(&mut self, play: &mut Play, now_ms: f64) {
        let command = play.mode().commands();
        // Entering command mode turns the panel to page 0 (`0x10063ca0`).
        if command && !self.commander.entered {
            self.commander.page = 0;
        }
        self.commander.entered = command;
        if command {
            self.commander.update(play, now_ms);
            let shown = play.selected_units();
            play.rate_units(&shown);
        }
        // The research panel, on page 4 or on a research centre's screen.
        let research = match play.mode() {
            crate::play::Mode::Factory(t) => {
                play.units.get(t).is_some_and(|u| u.type_word == crate::selection::RESEARCH_CENTRE)
            }
            _ => command && self.commander.page == 4,
        };
        self.commander.research.update(play, research);
    }

    /// The HUD for `play` on `space` at its hero's clock, `view_proj` its main camera, laid
    /// out in `font` (`GAME_FONT`) and `menu` (`MENU_FONT`). While the objectives screen is up
    /// it alone is drawn (`0x1008d3ac`); otherwise the widgets, the satellite map over them,
    /// and the message box.
    pub fn draw(
        &mut self,
        play: &Play,
        space: Space,
        font: &GameFont,
        menu: &GameFont,
        view_proj: Mat4,
    ) -> Drawn {
        let now_ms = play.hero.time_ms;
        let mut ink =
            Ink { painter: Painter::new(space), text: Vec::new(), font, menu_runs: Vec::new(), menu };
        if objectives::draw(self, &mut ink, play, now_ms) {
            return Drawn { batches: ink.painter.batches, menu_text: ink.menu_runs, ..Drawn::default() };
        }
        // A building's screen in place of the HUD, with the satellite map and the message box
        // (`0x1008d444`).
        if let crate::play::Mode::Factory(t) = play.mode() {
            // The warbot designer, while it is up, is drawn and nothing else (`0x1008d444`).
            let previews = if self.designer.is_open() {
                designer::draw(self, &mut ink, play, now_ms)
            } else if play.units.get(t).is_some_and(|u| u.type_word == crate::selection::RESEARCH_CENTRE) {
                research::screen(self, &mut ink, play, now_ms)
            } else {
                let previews = factory::draw(self, &mut ink, play, t, now_ms);
                ink.painter.pin = Pin::TOP_RIGHT;
                map::draw(self, &mut ink, play, now_ms);
                previews
            };
            // Mode 5 shows the cursor (`0x1008d4f1`, over the designer `0x1008d457`), and in
            // view state 1 it is always `ARROW` (docs/36, "The cursor in mode 5").
            commander::cursor(self, &mut ink, crate::pick::ARROW, now_ms);
            return Drawn {
                batches: ink.painter.batches,
                text: ink.text,
                menu_text: ink.menu_runs,
                previews,
                ..Drawn::default()
            };
        }
        if play.mode().commands() {
            if self.designer.is_open() {
                let previews = designer::draw(self, &mut ink, play, now_ms);
                // The pick answers kind 0 while the designer is up: `ARROW` (`0x1008dae5`).
                commander::cursor(self, &mut ink, crate::pick::ARROW, now_ms);
                return Drawn {
                    batches: ink.painter.batches,
                    text: ink.text,
                    menu_text: ink.menu_runs,
                    previews,
                    ..Drawn::default()
                };
            }
            let previews = commander::draw(self, &mut ink, play, now_ms, view_proj);
            return Drawn {
                batches: ink.painter.batches,
                text: ink.text,
                menu_text: ink.menu_runs,
                previews,
                ..Drawn::default()
            };
        }
        let mut voices = Vec::new();
        let water_level = self.water_level;
        let mut sounds = radar::draw(self, &mut ink, play, water_level);
        let views = panels::draw(self, &mut ink, play, view_proj, now_ms);
        sounds.extend(weapons::locks(self, &mut ink, play, view_proj));
        ink.painter.pin = Pin::TOP_RIGHT;
        if !self.map.open {
            voices.extend(weapons::draw(self, &mut ink, play));
        }
        radar::reticle(self, &mut ink);
        wingmen::draw(self, &mut ink, play);
        map::draw(self, &mut ink, play, now_ms);
        ink.painter.pin = Pin::TOP;
        messages::draw(self, &mut ink, now_ms);
        voices.extend(self.panels.voices(play, now_ms));
        Drawn {
            batches: ink.painter.batches,
            text: ink.text,
            menu_text: ink.menu_runs,
            views,
            previews: Vec::new(),
            voices,
            sounds,
        }
    }
}
