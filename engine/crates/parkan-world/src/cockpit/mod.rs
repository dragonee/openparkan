//! The cockpit HUD as `iron3d.dll` draws it on its 640 × 480 layout: the weapons list at the
//! top right, the message box at the top, and the target and own-unit panels in the bottom
//! corners. Each frame gives the HUD's art as painter batches, its `GAME_FONT` text, and the
//! views of units its panels hold. See `docs/35-hud.md`.

pub mod factory;
pub mod map;
pub mod messages;
pub mod objectives;
pub mod panels;
pub mod radar;
pub mod weapons;

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
            map: map::SatelliteMap::new(
                crate::settings::value(game, "CS", "MAP_ALPHA").and_then(|v| v.parse().ok()),
            ),
        })
    }

    pub fn string(&self, id: u32) -> &str {
        self.strings.get(&id).map_or("", String::as_str)
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
            factory::draw(self, &mut ink, play, t, now_ms);
            ink.painter.pin = Pin::TOP_RIGHT;
            map::draw(self, &mut ink, play, now_ms);
            return Drawn {
                batches: ink.painter.batches,
                text: ink.text,
                menu_text: ink.menu_runs,
                ..Drawn::default()
            };
        }
        let mut voices = Vec::new();
        let water_level = self.water_level;
        let sounds = radar::draw(self, &mut ink, play, water_level);
        let views = panels::draw(self, &mut ink, play, view_proj, now_ms);
        ink.painter.pin = Pin::TOP_RIGHT;
        if !self.map.open {
            voices.extend(weapons::draw(self, &mut ink, play));
        }
        radar::reticle(self, &mut ink);
        map::draw(self, &mut ink, play, now_ms);
        ink.painter.pin = Pin::TOP;
        messages::draw(self, &mut ink, now_ms);
        voices.extend(self.panels.voices(play, now_ms));
        Drawn {
            batches: ink.painter.batches,
            text: ink.text,
            menu_text: ink.menu_runs,
            views,
            voices,
            sounds,
        }
    }
}
