//! A campaign mission's briefing as the player sees it: the flythrough's camera, the voices it
//! starts, and its screen, a fade under two black bars, the mission's title and the subtitle.
//! See `docs/21-briefing.md`, "How the briefing is shown".

use std::path::Path;

use anyhow::Result;
use glam::Vec3;
use parkan_formats::cfg;
use parkan_formats::gamedir;
use parkan_formats::resources::TextResources;
use parkan_sim::briefing::{self, Flythrough};

use crate::hud::{Pin, Space};
use crate::resources::{Sound, Sounds};
use crate::robot::Eye;
use crate::text::{GameFont, TextRun};

/// The file the mission's title is the first line of (`iron3d.dll:0x1005dfb1`), read into 128
/// bytes.
pub const DESCRIPTION: &str = "descr";
const TITLE_BYTES: usize = 127;
/// The bars' inner edges, the title's pen and the subtitle's first line on the 640 × 480
/// screen (`0x100315b0`).
pub const TOP_BAR: f32 = 75.0;
pub const BOTTOM_BAR: f32 = 405.0;
pub const TITLE_AT: [f32; 2] = [20.0, 20.0];
pub const SUBTITLE_AT: [f32; 2] = [10.0, 410.0];
/// The subtitle's wrap, a share of the screen's width (`0x10093140`).
pub const WRAP_SHARE: f32 = 0.98;
/// The title's and the subtitle's colour.
pub const GREY: u32 = 0xff80_8080;

/// A filled rectangle in normalised device coordinates (y up), and its colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub colour: [f32; 4],
}

/// The briefing screen this frame: the fade and the bars, the title to draw in `MENU_FONT`,
/// the subtitle's lines in `GAME_FONT`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub fills: Vec<Fill>,
    pub title: Vec<TextRun>,
    pub lines: Vec<TextRun>,
}

pub struct Briefing {
    pub flythrough: Flythrough,
    /// The first line of the mission's `descr`.
    pub title: String,
    /// `Iron_3D.ini`'s `[CS] SUBTITLES`.
    pub subtitles: bool,
    texts: Option<TextResources>,
    sounds: Option<Sounds>,
}

/// The first line of a mission's `descr`, as `fgets` reads it into 128 bytes.
pub fn title(mission_dir: &Path) -> String {
    let Some(bytes) = gamedir::resolve(mission_dir, DESCRIPTION).and_then(|p| std::fs::read(p).ok()) else {
        return String::new();
    };
    let line = bytes.split(|&b| b == b'\n').next().unwrap_or(&[]);
    let line = &line[..line.len().min(TITLE_BYTES)];
    parkan_formats::cursor::latin1(line).trim_end_matches(['\r', '\n']).to_owned()
}

impl Briefing {
    /// The briefing `mission.cfg`'s `briefing` object names, or none: a mission without one,
    /// or whose file does not load, starts at once (`0x10031130`).
    pub fn open(game: &Path, mission_dir: &Path) -> Result<Option<Briefing>> {
        let Some(cfg_path) = gamedir::resolve(mission_dir, "mission.cfg") else { return Ok(None) };
        let blocks = crate::resources::read_cfg(&cfg_path)?;
        let Some(file) = blocks.get("briefing").and_then(|b| b.get("filename")) else { return Ok(None) };
        let Some(path) = gamedir::resolve(mission_dir, file) else { return Ok(None) };
        let Ok(stops) = crate::resources::read_cfg(&path).map(|b| cfg::waypoints(&b)) else {
            return Ok(None);
        };
        if stops.is_empty() {
            return Ok(None);
        }
        // STAND-IN: docs/21-briefing.md#when-it-runs--read -- the default the game reads
        // `SUBTITLES` with is not established; the install's ini sets 1, and an absent key
        // shows them.
        let subtitles =
            crate::settings::value(game, "CS", "SUBTITLES").is_none_or(|v| v.parse::<i64>().ok() != Some(0));
        Ok(Some(Briefing {
            flythrough: Flythrough::new(stops),
            title: title(mission_dir),
            subtitles,
            texts: crate::resources::text_resources(game).ok(),
            sounds: Sounds::open(game, mission_dir).ok(),
        }))
    }

    /// One frame at `seconds` on the briefing's clock: the first starts the flythrough.
    /// Returns the voices the stops arrived at start, to play at once (`0x10030836`).
    pub fn frame(&mut self, seconds: f64) -> Vec<Sound> {
        self.flythrough.start(seconds);
        self.flythrough.update(seconds);
        let voices = std::mem::take(&mut self.flythrough.voices);
        voices.iter().filter_map(|v| self.sounds.as_ref()?.get(v)).collect()
    }

    pub fn finished(&self) -> bool {
        self.flythrough.finished
    }

    /// The camera: at the eye, looking at the look-at point with world z up, never rolling
    /// (`0x10030220`), across a horizontal 1.04 rad.
    pub fn eye(&self) -> Eye {
        let f = &self.flythrough;
        let position = f.eye.as_vec3();
        let forward = (f.look - f.eye).as_vec3().normalize_or(Vec3::Y);
        let right = Vec3::Z.cross(forward).normalize_or(Vec3::X);
        Eye {
            position,
            forward,
            up: forward.cross(right),
            fov_x: f.fov_x() as f32,
            near: briefing::NEAR as f32,
        }
    }

    /// The subtitle's text now: the current stop's `TextResID` through `TextRes`.
    pub fn subtitle(&self) -> &str {
        let id = self.flythrough.text_id();
        if id.is_empty() {
            return "";
        }
        self.texts.as_ref().and_then(|t| t.get(id)).unwrap_or("")
    }

    /// The screen on `space` (`0x100315b0`). The bars run across the whole screen, the title
    /// hangs from its top left and the subtitle from its bottom left.
    pub fn screen(&self, space: Space, game: &GameFont) -> Screen {
        let ndc = |at: [f32; 2], pin: Pin| space.ndc(at, pin);
        let fill = |top: f32, bottom: f32, pin: Pin, colour: [f32; 4]| {
            let (y0, y1) = (ndc([0.0, top], pin)[1], ndc([0.0, bottom], pin)[1]);
            Fill { min: [-1.0, y1.min(y0)], max: [1.0, y1.max(y0)], colour }
        };
        let fade = self.flythrough.fade.clamp(0.0, 1.0) as f32;
        let fills = vec![
            // The fade, over the whole display, then the bars over it.
            Fill {
                min: [-1.0, -1.0],
                max: [1.0, 1.0],
                colour: [0.0, 0.0, 0.0, (fade * 255.0).round() / 255.0],
            },
            fill(0.0, TOP_BAR, Pin::TOP, [0.0, 0.0, 0.0, 1.0]),
            fill(BOTTOM_BAR, 480.0, Pin::BOTTOM, [0.0, 0.0, 0.0, 1.0]),
        ];
        let colour = crate::cockpit::argb(GREY);
        let scale = space.scale();
        let title =
            vec![TextRun { colour, scale, ..TextRun::new(&self.title, ndc(TITLE_AT, Pin::TOP_LEFT)) }];
        let mut lines = Vec::new();
        let text = self.subtitle();
        if self.subtitles && !text.is_empty() {
            // Line i at 410 + i × round((GAME_FONT's height + 1) ÷ the vertical scale), the
            // height taken on the 640 × 480 font the layout is drawn in.
            let step = (game.line_height + 1.0).round();
            let width = WRAP_SHARE * space.width / space.scales()[0];
            let wrapped = game.lines(&TextRun { wrap: Some(width), ..TextRun::new(text, [0.0; 2]) });
            for (i, line) in wrapped.iter().enumerate() {
                let at = [SUBTITLE_AT[0], SUBTITLE_AT[1] + i as f32 * step];
                lines.push(TextRun { colour, scale, ..TextRun::new(line, ndc(at, Pin::BOTTOM_LEFT)) });
            }
        }
        Screen { fills, title, lines }
    }
}
