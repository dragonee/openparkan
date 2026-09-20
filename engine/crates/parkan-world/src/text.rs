//! The game font from `gamefont.rlb`, and strings laid out in it for the screen.
//! See `docs/12-rsli.md`, "How a glyph is drawn".
//!
//! A glyph record gives its left and right edge and its top as texture coordinates,
//! and its advance; the font's header gives the glyph height, the extra pen step and
//! the height in texture coordinates. `Ngi32.dll`'s text-out (`0x10010e40`) indexes
//! the 256 records by the string byte itself, lays the glyph in a cell `advance + 1`
//! wide and `height + 1` tall, and moves the pen `advance + spacing`. Where lines
//! break is the caller's, and is laid out here.

use std::path::Path;

use anyhow::{Context, Result};
use parkan_formats::font::{self, Glyph};
use parkan_formats::{gamedir, rsli};

/// The font archive, at the install's root.
pub const FONT_ARCHIVE: &str = "gamefont.rlb";
/// Where the interface names its fonts.
pub const UI_RESOURCES: &str = "ui/menu_resources.cfg";

/// Which single-byte code page an atlas is laid out in, and so which byte of it a
/// character draws with.
///
/// `Ngi32.dll`'s text-out indexes the records by the string byte itself, and passes it
/// through a 256-entry `int16` table at `0x10036e50` first only while the font's flag at
/// `+0x104c` is 1 (`0x100110e9`). The constructor sets that flag (`0x10010c43`) and the
/// interface's font loader clears it on every font it makes (`services.dll:0x10007cc2`,
/// slot `+0x20`), so a `ui/font.lib` font is indexed by the raw Windows-1251 byte and
/// the table converts Windows-1251 to CP866 for the fonts laid out that way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    /// `ui/font.lib`'s nine fonts: 0xC0-0xFF is А-Я а-я, as the game's own strings are.
    #[default]
    Windows1251,
    /// `gamefont.rlb`'s and `sys.lib`'s `ARIALTEX.TFT`: А-Я at 0x80, а-п at 0xA0,
    /// р-я at 0xE0, Ё and ё at 0xF0 and 0xF1.
    Cp866,
}

/// The font and its atlas, decoded through the palette.
#[derive(Clone, Debug, PartialEq)]
pub struct GameFont {
    pub glyphs: Vec<Glyph>,
    /// RGBA8 rows, top first.
    pub atlas: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// How tall a glyph cell is, in atlas pixels: the font header's height plus one.
    pub line_height: f32,
    /// The glyph's height in texture coordinates: the header's own word, which is
    /// `line_height / height` on every shipped font.
    pub v_span: f32,
    /// What the pen adds after every glyph's advance: the header's word, 1 everywhere.
    pub spacing: f32,
    pub encoding: Encoding,
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

/// Windows-1251's upper half, 0x80 to 0xFF. The game's own strings are in it: of the
/// 24740 strings in the install's 112 text resources none carries a byte above 0x7f,
/// and the one non-ASCII character left in `DATA/TextRes.dll`'s 173 is `U+00C0`, the
/// byte Windows-1251 gives А.
const WINDOWS_1251_HIGH: [char; 128] = [
    'Ђ', 'Ѓ', '‚', 'ѓ', '„', '…', '†', '‡', '€', '‰', 'Љ', '‹', 'Њ', 'Ќ', 'Ћ', 'Џ', //
    'ђ', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '•', '–', '—', '\u{fffd}', '™', 'љ', '›', 'њ', 'ќ',
    'ћ', 'џ', //
    '\u{00a0}', 'Ў', 'ў', 'Ј', '¤', 'Ґ', '¦', '§', 'Ё', '©', 'Є', '«', '¬', '\u{00ad}', '®', 'Ї', //
    '°', '±', 'І', 'і', 'ґ', 'µ', '¶', '·', 'ё', '№', 'є', '»', 'ј', 'Ѕ', 'ѕ', 'ї', //
    'А', 'Б', 'В', 'Г', 'Д', 'Е', 'Ж', 'З', 'И', 'Й', 'К', 'Л', 'М', 'Н', 'О', 'П', //
    'Р', 'С', 'Т', 'У', 'Ф', 'Х', 'Ц', 'Ч', 'Ш', 'Щ', 'Ъ', 'Ы', 'Ь', 'Э', 'Ю', 'Я', //
    'а', 'б', 'в', 'г', 'д', 'е', 'ж', 'з', 'и', 'й', 'к', 'л', 'м', 'н', 'о', 'п', //
    'р', 'с', 'т', 'у', 'ф', 'х', 'ц', 'ч', 'ш', 'щ', 'ъ', 'ы', 'ь', 'э', 'ю', 'я',
];

/// The glyph index a character draws with in an atlas laid out in `encoding`.
///
/// Nothing is ever replaced by `?`: every byte indexes one of the 256 records, and a
/// byte the font has no glyph for lands on a placeholder that draws nothing and steps
/// the pen half a line. A character the code page has no byte for takes index 0, whose
/// record is a placeholder in all eleven shipped fonts.
pub fn glyph_index(encoding: Encoding, c: char) -> u8 {
    if c.is_ascii() {
        return c as u8;
    }
    match encoding {
        Encoding::Windows1251 => {
            WINDOWS_1251_HIGH.iter().position(|&h| h == c).map_or(0, |i| (0x80 + i) as u8)
        }
        Encoding::Cp866 => match c {
            'А'..='Я' => (0x80 + (c as u32 - 'А' as u32)) as u8,
            'а'..='п' => (0xA0 + (c as u32 - 'а' as u32)) as u8,
            'р'..='я' => (0xE0 + (c as u32 - 'р' as u32)) as u8,
            'Ё' => 0xF0,
            'ё' => 0xF1,
            _ => 0,
        },
    }
}

impl GameFont {
    /// `ARIALTEX.TFT` and `PAL.PAL` from the install's `gamefont.rlb`.
    pub fn open(game: &Path) -> Result<GameFont> {
        let path = gamedir::resolve(game, FONT_ARCHIVE).with_context(|| format!("no {FONT_ARCHIVE}"))?;
        let archive = rsli::Archive::open(&path)?;
        let tft = font::parse_font(&archive.read_name("ARIALTEX.TFT")?, "ARIALTEX.TFT")?;
        let palette = font::parse_palette(&archive.read_name("PAL.PAL")?, "PAL.PAL")?;
        let atlas = tft.decode_atlas(&palette, "ARIALTEX.TFT")?;
        Ok(GameFont::new(&tft, atlas.levels[0].clone(), atlas.width, atlas.height, Encoding::Cp866))
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
        Ok(GameFont::new(&tft, atlas.levels[0].clone(), atlas.width, atlas.height, Encoding::Windows1251))
    }

    /// A font from its records, its header and its decoded atlas.
    ///
    /// A glyph is drawn in a cell one pixel taller than the header's height
    /// (`Ngi32.dll:0x10010ef0`, `0x100110b5`), which on every shipped font is also the
    /// atlas's row pitch and the header's own `v_span` in texels.
    pub fn new(tft: &font::Font, atlas: Vec<u8>, width: u32, height: u32, encoding: Encoding) -> GameFont {
        GameFont {
            glyphs: tft.glyphs.clone(),
            atlas,
            width,
            height,
            line_height: (tft.height + 1) as f32,
            v_span: tft.v_span,
            spacing: tft.spacing as f32,
            encoding,
        }
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(usize::from(glyph_index(self.encoding, c)))
    }

    /// How far a glyph moves the pen: its advance plus the font's spacing
    /// (`Ngi32.dll:0x10011118` and `0x1001113f`; the width routine at `0x10010dff`
    /// sums the same).
    fn step(&self, g: &Glyph) -> f32 {
        g.advance as f32 + self.spacing
    }

    /// How far a string moves the pen, in font pixels.
    pub fn advance(&self, text: &str) -> f32 {
        text.chars().filter_map(|c| self.glyph(c)).map(|g| self.step(g)).sum()
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
    ///
    /// STAND-IN: docs/12-rsli.md#not-resolved -- `Ngi32.dll`'s text-out draws one line
    /// at the y its caller names, and which of the interface's text controls stacks
    /// lines by what is not read. Lines are set a glyph cell apart, which is what the
    /// atlas's rows are on all eleven shipped fonts.
    pub fn layout(&self, run: &TextRun) -> Vec<Placed> {
        let s = run.scale;
        let w = self.width as f32;
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
                    // The cell is the glyph's own texels, one to the pixel: `advance + 1`
                    // wide (`Ngi32.dll:0x10011187` adds 1.0) and `height + 1` tall.
                    let span = (g.u1 - g.u0) * w;
                    out.push(Placed {
                        at: [pen, top],
                        size: [span * s, self.line_height * s],
                        uv: [g.u0, g.v0, g.u1, g.v0 + self.v_span],
                    });
                }
                pen += self.step(g) * s;
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

    /// A font whose header is a 15-pixel cell, a height of 15, a v span of 16/64 and a
    /// spacing of 1; `A` and `B` are drawn 8 pixels wide with an advance of 7, on a
    /// 64 × 64 atlas with rows at 0 and 16 pixels; everything else is a placeholder.
    fn font() -> GameFont {
        let mut glyphs = vec![Glyph { u0: 0.0, u1: 0.0001, v0: 0.0, advance: 8 }; 256];
        glyphs[usize::from(b'A')] = Glyph { u0: 0.0, u1: 0.125, v0: 0.0, advance: 7 };
        glyphs[usize::from(b'B')] = Glyph { u0: 0.125, u1: 0.25, v0: 0.25, advance: 7 };
        let tft =
            font::Font { cell_width: 15, height: 15, spacing: 1, v_span: 0.25, glyphs, atlas: Vec::new() };
        GameFont::new(&tft, vec![0; 64 * 64 * 4], 64, 64, Encoding::Windows1251)
    }

    #[test]
    fn glyphs_step_their_advance_and_the_fonts_spacing_and_placeholders_draw_nothing() {
        let f = font();
        assert_eq!(f.line_height, 16.0, "the header's height and one more");
        let placed = f.layout(&TextRun::new("A B", [0.0, 0.0]));
        assert_eq!(placed.len(), 2, "the space draws nothing");
        assert_eq!(placed[0].at, [0.0, 0.0]);
        assert_eq!(placed[1].at, [17.0, 0.0], "7 + 1 for A, 8 + 1 for the space");
        assert_eq!(placed[1].size, [8.0, 16.0]);
        assert_eq!(placed[1].uv, [0.125, 0.25, 0.25, 0.5]);
        assert_eq!(f.width(&TextRun::new("A B", [0.0, 0.0])), 25.0);
    }

    #[test]
    fn lines_wrap_between_words_and_centre_on_the_anchor() {
        let f = font();
        let run = TextRun {
            wrap: Some(34.0),
            align: Align::Centre,
            scale: 2.0,
            ..TextRun::new("AB AB AB", [0.0; 2])
        };
        // At scale 2 the limit is 17 font pixels: "AB" is 16 and fits, "AB AB" is 41 and does not.
        assert_eq!(f.lines(&run), vec!["AB", "AB", "AB"]);
        let placed = f.layout(&run);
        assert_eq!(placed.len(), 6);
        assert_eq!(placed[0].at, [-16.0, 0.0], "a 32-pixel line centred");
        assert_eq!(placed[2].at[1], 32.0, "the second line one line height down, scaled");
        assert_eq!(f.lines(&TextRun::new("A\nB", [0.0; 2])), vec!["A", "B"]);
    }

    #[test]
    fn a_character_takes_its_byte_in_the_atlas_own_code_page() {
        use Encoding::{Cp866, Windows1251};
        assert_eq!(glyph_index(Windows1251, 'A'), b'A');
        assert_eq!(glyph_index(Cp866, 'A'), b'A');
        // The nine `ui/font.lib` fonts draw 0xC0-0xFF; the two `ARIALTEX.TFT` draw CP866's places.
        assert_eq!((glyph_index(Windows1251, 'А'), glyph_index(Cp866, 'А')), (0xC0, 0x80));
        assert_eq!((glyph_index(Windows1251, 'я'), glyph_index(Cp866, 'я')), (0xFF, 0xEF));
        assert_eq!((glyph_index(Windows1251, 'п'), glyph_index(Cp866, 'п')), (0xEF, 0xAF));
        assert_eq!((glyph_index(Windows1251, 'Ё'), glyph_index(Cp866, 'Ё')), (0xA8, 0xF0));
        assert_eq!((glyph_index(Windows1251, 'ё'), glyph_index(Cp866, 'ё')), (0xB8, 0xF1));
        assert_eq!(glyph_index(Windows1251, '€'), 0x88, "Windows-1251 has it");
        // Nothing draws a `?` for a character the page has no byte for: index 0 is a placeholder.
        assert_eq!((glyph_index(Windows1251, '漢'), glyph_index(Cp866, '€')), (0, 0));
    }
}
