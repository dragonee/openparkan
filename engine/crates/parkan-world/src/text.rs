//! The game font from `gamefont.rlb`, and strings laid out in it for the screen.
//! See `docs/12-rsli.md`, "What is inside".
//!
//! A glyph record gives its left and right edge and its top as texture coordinates,
//! and how far the pen moves; the atlas is white glyphs on black, which the text
//! pass tints. Everything past that, which string byte a character is, how tall a
//! glyph is drawn and where lines break, is laid out here.

use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::font::{self, Glyph};
use parkan_formats::{gamedir, rsli};

/// The font archive, at the install's root.
pub const FONT_ARCHIVE: &str = "gamefont.rlb";
/// Where the interface names its fonts.
pub const UI_RESOURCES: &str = "ui/menu_resources.cfg";

/// The font and its atlas, decoded through the palette.
#[derive(Clone, Debug, PartialEq)]
pub struct GameFont {
    pub glyphs: Vec<Glyph>,
    /// RGBA8 rows, top first.
    pub atlas: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// How tall a glyph cell is, in atlas pixels.
    pub line_height: f32,
}

/// Where a run's lines sit against its anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Centre,
    Right,
}

/// A string to draw: its anchor in normalised device coordinates (y up), where the
/// first line's top sits; a colour; a scale on the font's own pixels; and, if given,
/// the width in screen pixels its lines wrap to.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub anchor: [f32; 2],
    pub colour: [f32; 4],
    pub scale: f32,
    pub align: Align,
    pub wrap: Option<f32>,
}

impl TextRun {
    pub fn new(text: impl Into<String>, anchor: [f32; 2]) -> Self {
        Self { text: text.into(), anchor, colour: [1.0; 4], scale: 1.0, align: Align::Left, wrap: None }
    }
}

/// One glyph placed: its top-left and size in screen pixels from the run's anchor
/// (y down), and its atlas rectangle as texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub at: [f32; 2],
    pub size: [f32; 2],
    pub uv: [f32; 4],
}

/// The glyph index a character draws with.
///
/// STAND-IN: docs/12-rsli.md#what-is-inside -- how the game turns its strings into
/// glyph indices is not read. ASCII is taken as its own index, and Cyrillic by the
/// DOS code page 866, whose places are exactly where the font draws its Cyrillic
/// (А–Я at 0x80, а–п at 0xA0, р–я at 0xE0); anything else draws as `?`.
pub fn glyph_index(c: char) -> u8 {
    match c {
        '\0'..='\x7f' => c as u8,
        'А'..='Я' => (0x80 + (c as u32 - 'А' as u32)) as u8,
        'а'..='п' => (0xA0 + (c as u32 - 'а' as u32)) as u8,
        'р'..='я' => (0xE0 + (c as u32 - 'р' as u32)) as u8,
        'Ё' => 0xF0,
        'ё' => 0xF1,
        _ => b'?',
    }
}

/// How far a glyph moves the pen, in font pixels: its advance.
///
/// STAND-IN: docs/12-rsli.md#what-is-inside -- how the game spaces its glyphs is not
/// read; *measured* on a recording of Mission 01's win for the interface's `mf_640.tft`:
/// each glyph of "MISSION COMPLETE !" starts its record's advance after the one before,
/// the space's 6 included (M to I 10.6 for 10, I to S 3.4 for 3, N to C 14.7 for 8 + 6).
/// The game font is taken to space the same way.
fn step(g: &Glyph) -> f32 {
    g.advance as f32
}

impl GameFont {
    /// `ARIALTEX.TFT` and `PAL.PAL` from the install's `gamefont.rlb`.
    pub fn open(game: &Path) -> Result<GameFont> {
        let path = gamedir::resolve(game, FONT_ARCHIVE).with_context(|| format!("no {FONT_ARCHIVE}"))?;
        let archive = rsli::Archive::open(&path)?;
        let tft = font::parse_font(&archive.read_name("ARIALTEX.TFT")?, "ARIALTEX.TFT")?;
        let palette = font::parse_palette(&archive.read_name("PAL.PAL")?, "PAL.PAL")?;
        let atlas = tft.decode_atlas(&palette, "ARIALTEX.TFT")?;
        Ok(GameFont::new(tft.glyphs.clone(), &tft.rows(), atlas.levels[0].clone(), atlas.width, atlas.height))
    }

    /// The font `name` (`MENU_FONT`, `GAME_FONT`, …) names for a 640 × 480 screen: through
    /// `ui/menu_resources.cfg`'s font substitute to its `fonts` resource index, an entry of
    /// `ui/font.lib`, a `Tfnt` whose atlas carries its own pixels (docs/34, "After the
    /// outcome").
    pub fn ui(game: &Path, name: &str) -> Result<GameFont> {
        let cfg = gamedir::resolve(game, UI_RESOURCES).with_context(|| format!("no {UI_RESOURCES}"))?;
        let blocks = crate::resources::read_cfg(&cfg)?;
        let unquote = |v: &str| v.trim().trim_matches('"').to_owned();
        let substitute = blocks
            .0
            .iter()
            .find(|b| {
                b.get("desc").map(unquote).as_deref() == Some("font-substitute")
                    && b.get("dimension_x").map(unquote).as_deref() == Some("640")
                    && b.get("dimension_y").map(unquote).as_deref() == Some("480")
            })
            .and_then(|b| b.get(name).map(unquote))
            .with_context(|| format!("no 640x480 substitute for {name}"))?;
        let fonts = blocks.get("fonts").context("no fonts resource")?;
        let index: usize =
            fonts.get(&substitute).map(unquote).with_context(|| format!("no font {substitute}"))?.parse()?;
        let library = fonts.get("library").map(unquote).context("the fonts name no library")?;
        let path = crate::resources::locate(game, &library).with_context(|| format!("no {library}"))?;
        let archive = parkan_formats::nres::Archive::open(&path)?;
        let entry = archive.entries.get(index).with_context(|| format!("no font {index} in {library}"))?;
        let tft = font::parse_font(archive.read(entry)?, &entry.name)?;
        let atlas = parkan_formats::texm::decode(&tft.atlas, &entry.name, None)?;
        Ok(GameFont::new(tft.glyphs.clone(), &tft.rows(), atlas.levels[0].clone(), atlas.width, atlas.height))
    }

    /// A font from its glyphs, its drawn rows' tops and its atlas.
    ///
    /// STAND-IN: docs/12-rsli.md#what-is-inside -- a record gives no bottom edge, and
    /// how tall the game draws a glyph is not read. A glyph is drawn as tall as the
    /// atlas's row pitch (18 pixels on the shipped font), and a line is as tall.
    pub fn new(glyphs: Vec<Glyph>, rows: &[f32], atlas: Vec<u8>, width: u32, height: u32) -> GameFont {
        let pitch = rows.windows(2).map(|w| w[1] - w[0]).fold(f32::MAX, f32::min);
        let line_height = if pitch < f32::MAX { pitch * height as f32 } else { height as f32 };
        GameFont { glyphs, atlas, width, height, line_height }
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(usize::from(glyph_index(c)))
    }

    /// How far a string moves the pen, in font pixels.
    pub fn advance(&self, text: &str) -> f32 {
        text.chars().filter_map(|c| self.glyph(c)).map(step).sum()
    }

    /// The lines `run` breaks into: at each newline, and between words where a line
    /// would pass the wrap width. A word wider than the width stands alone.
    pub fn lines(&self, run: &TextRun) -> Vec<String> {
        let limit = run.wrap.map(|w| w / run.scale.max(f32::EPSILON));
        let mut out = Vec::new();
        for paragraph in run.text.split('\n') {
            let Some(limit) = limit else {
                out.push(paragraph.to_owned());
                continue;
            };
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
                if !line.is_empty() && self.advance(&candidate) > limit {
                    out.push(std::mem::replace(&mut line, word.to_owned()));
                } else {
                    line = candidate;
                }
            }
            out.push(line);
        }
        out
    }

    /// Every drawn glyph of `run`, placed in whole screen pixels at its scale.
    pub fn layout(&self, run: &TextRun) -> Vec<Placed> {
        let s = run.scale;
        let (w, h) = (self.width as f32, self.height as f32);
        let mut out = Vec::new();
        for (row, line) in self.lines(run).iter().enumerate() {
            let width = self.advance(line) * s;
            let mut pen = match run.align {
                Align::Left => 0.0,
                Align::Centre => (-width / 2.0).round(),
                Align::Right => -width,
            };
            let top = (row as f32 * self.line_height * s).round();
            for c in line.chars() {
                let Some(g) = self.glyph(c) else { continue };
                if g.drawn() {
                    let span = (g.u1 - g.u0) * w;
                    out.push(Placed {
                        at: [pen, top],
                        size: [span * s, self.line_height * s],
                        uv: [g.u0, g.v0, g.u1, g.v0 + self.line_height / h],
                    });
                }
                pen += step(g) * s;
            }
        }
        out
    }

    /// The width in screen pixels of `run`'s widest line.
    pub fn width(&self, run: &TextRun) -> f32 {
        self.lines(run).iter().map(|l| self.advance(l) * run.scale).fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font where `A` and `B` are drawn 8 pixels wide with an advance of 7, on a
    /// 64 × 64 atlas with rows at 0 and 16 pixels; everything else is a placeholder.
    fn font() -> GameFont {
        let mut glyphs = vec![Glyph { u0: 0.0, u1: 0.0001, v0: 0.0, advance: 8 }; 256];
        glyphs[usize::from(b'A')] = Glyph { u0: 0.0, u1: 0.125, v0: 0.0, advance: 7 };
        glyphs[usize::from(b'B')] = Glyph { u0: 0.125, u1: 0.25, v0: 0.25, advance: 7 };
        GameFont::new(glyphs, &[0.0, 0.25], vec![0; 64 * 64 * 4], 64, 64)
    }

    #[test]
    fn glyphs_advance_by_their_records_and_placeholders_draw_nothing() {
        let f = font();
        assert_eq!(f.line_height, 16.0);
        let placed = f.layout(&TextRun::new("A B", [0.0, 0.0]));
        assert_eq!(placed.len(), 2, "the space draws nothing");
        assert_eq!(placed[0].at, [0.0, 0.0]);
        assert_eq!(placed[1].at, [15.0, 0.0], "7 for A, 8 for the space: each its advance");
        assert_eq!(placed[1].size, [8.0, 16.0]);
        assert_eq!(placed[1].uv, [0.125, 0.25, 0.25, 0.5]);
        assert_eq!(f.width(&TextRun::new("A B", [0.0, 0.0])), 22.0);
    }

    #[test]
    fn lines_wrap_between_words_and_centre_on_the_anchor() {
        let f = font();
        let run = TextRun {
            wrap: Some(30.0),
            align: Align::Centre,
            scale: 2.0,
            ..TextRun::new("AB AB AB", [0.0; 2])
        };
        // At scale 2 the limit is 15 font pixels: "AB" is 14 and fits, "AB AB" is 36 and does not.
        assert_eq!(f.lines(&run), vec!["AB", "AB", "AB"]);
        let placed = f.layout(&run);
        assert_eq!(placed.len(), 6);
        assert_eq!(placed[0].at, [-14.0, 0.0], "a 28-pixel line centred");
        assert_eq!(placed[2].at[1], 32.0, "the second line one line height down, scaled");
        assert_eq!(f.lines(&TextRun::new("A\nB", [0.0; 2])), vec!["A", "B"]);
    }

    #[test]
    fn cyrillic_takes_the_places_the_font_draws_it_at() {
        assert_eq!(glyph_index('A'), b'A');
        assert_eq!(glyph_index('А'), 0x80);
        assert_eq!(glyph_index('я'), 0xEF);
        assert_eq!(glyph_index('п'), 0xAF);
        assert_eq!(glyph_index('€'), b'?');
    }
}
