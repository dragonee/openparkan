//! `gamefont.rlb`'s two members: the `Tfnt` font `ARIALTEX.TFT` and its palette
//! `PAL.PAL`. See `docs/12-rsli.md`, "What is inside".
//!
//! A `Tfnt` is a 20-byte header, 256 glyph records of 16 bytes (`u0`, `u1`, `v0` as
//! texture coordinates and an `int32` advance), then a `Texm` in pixel format 2 whose
//! indices the palette's 256 BGRA entries colour.
//!
//! The four header words after the magic are the font's own metrics, and the engine
//! keeps them at `font + 0x1038`, `+0x103c`, `+0x1040` and `+0x1048`
//! (`Ngi32.dll:0x1000fbfd`): a fixed-pitch cell width, the glyph height, the extra
//! pen step per character, and the glyph's height in texture coordinates.

use crate::cursor::{Cursor, FormatError};
use crate::texm::{self, Texture};

pub const MAGIC: &[u8; 4] = b"Tfnt";
pub const HEADER_SIZE: usize = 20;
pub const GLYPH_COUNT: usize = 256;
pub const GLYPH_STRIDE: usize = 16;
/// Where the atlas begins: the header and the 256 records.
pub const ATLAS_AT: usize = HEADER_SIZE + GLYPH_COUNT * GLYPH_STRIDE;

/// `PAL.PAL`: a 1024-byte palette, the tag, then a 256 × 256 blend table.
pub const PALETTE_SIZE: usize = 1024;
pub const PALETTE_TAG: &[u8; 4] = b"Ipol";
pub const TABLE_SIDE: usize = 256;
pub const PAL_SIZE: usize = PALETTE_SIZE + 4 + TABLE_SIDE * TABLE_SIDE;

/// A record that draws nothing spans a fifth of a pixel; the narrowest glyph two.
pub const MIN_SPAN: f32 = 1.0 / 256.0;

/// One code point's place in the atlas, and how far the pen moves after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub u0: f32,
    pub u1: f32,
    pub v0: f32,
    /// Pixels.
    pub advance: i32,
}

impl Glyph {
    pub fn drawn(&self) -> bool {
        self.u1 - self.u0 > MIN_SPAN
    }
}

/// `ARIALTEX.TFT`.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// Header word 0: the cell width a fixed-pitch run gives every glyph
    /// (`Ngi32.dll:0x10010e73`), and what the font's `GetWidth` returns.
    pub cell_width: i32,
    /// Header word 1: how tall a glyph is, one less than the cell it is drawn in
    /// (`0x10010ef0`); what the font's `GetHeight` returns.
    pub height: i32,
    /// Header word 3: the pen moves a glyph's advance plus this after every
    /// character (`0x10011118`, `0x10010dff`). It is 1 on all eleven shipped fonts.
    pub spacing: i32,
    /// Header word 2: the glyph's height in texture coordinates (`0x10011287`). It is
    /// `(height + 1) / atlas height` on all eleven, so the atlas is drawn one texel to
    /// the pixel.
    pub v_span: f32,
    pub glyphs: Vec<Glyph>,
    /// The embedded `Texm` blob.
    pub atlas: Vec<u8>,
}

impl Font {
    /// The atlas decoded through `palette`: RGBA at level 0.
    pub fn decode_atlas(&self, palette: &Palette, source: &str) -> Result<Texture, FormatError> {
        texm::decode(&self.atlas, source, Some(&palette.raw))
    }
}

/// `PAL.PAL`.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// 256 BGRA entries, the fourth byte always zero.
    pub raw: Vec<u8>,
    /// `blend[a * 256 + b]` is the index of the mixture of `a` and `b`.
    pub blend: Vec<u8>,
}

impl Palette {
    /// `(r, g, b)` for index `i`.
    pub fn colour(&self, i: u8) -> [u8; 3] {
        let j = usize::from(i) * 4;
        [self.raw[j + 2], self.raw[j + 1], self.raw[j]]
    }
}

pub fn parse_font(data: &[u8], source: &str) -> Result<Font, FormatError> {
    if data.len() < ATLAS_AT + 4 || data[..4] != MAGIC[..] {
        return Err(FormatError::invalid(source, "not a Tfnt"));
    }
    let mut c = Cursor::new(data, source);
    c.bytes(4)?;
    let (cell_width, height) = (c.i32()?, c.i32()?);
    let (v_span, spacing) = (c.f32()?, c.i32()?);
    let glyphs = (0..GLYPH_COUNT)
        .map(|_| Ok(Glyph { u0: c.f32()?, u1: c.f32()?, v0: c.f32()?, advance: c.i32()? }))
        .collect::<Result<Vec<_>, FormatError>>()?;
    if &data[ATLAS_AT..ATLAS_AT + 4] != b"Texm" {
        return Err(FormatError::invalid(source, format!("no Texm at {ATLAS_AT}")));
    }
    Ok(Font { cell_width, height, spacing, v_span, glyphs, atlas: data[ATLAS_AT..].to_vec() })
}

pub fn parse_palette(data: &[u8], source: &str) -> Result<Palette, FormatError> {
    if data.len() < PAL_SIZE {
        return Err(FormatError::invalid(source, format!("{} bytes, want {PAL_SIZE}", data.len())));
    }
    if data[PALETTE_SIZE..PALETTE_SIZE + 4] != PALETTE_TAG[..] {
        return Err(FormatError::invalid(source, "no Ipol tag"));
    }
    Ok(Palette { raw: data[..PALETTE_SIZE].to_vec(), blend: data[PALETTE_SIZE + 4..].to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tfnt() -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&15i32.to_le_bytes());
        out.extend_from_slice(&1i32.to_le_bytes());
        out.extend_from_slice(&1.0f32.to_le_bytes());
        out.extend_from_slice(&1i32.to_le_bytes());
        for i in 0..GLYPH_COUNT {
            let (u0, u1, advance) =
                if i == usize::from(b'A') { (0.0f32, 0.0625f32, 7i32) } else { (0.0, 0.0001, 8) };
            for f in [u0, u1, 0.5] {
                out.extend_from_slice(&f.to_le_bytes());
            }
            out.extend_from_slice(&advance.to_le_bytes());
        }
        // A 2 × 2 format-2 Texm with the alpha-surface bit set: indices 0, 73, 73, 0.
        for w in [0x6d78_6554u32, 2, 2, 1, 0, texm::ALPHA_SURFACE, 0, 2] {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out.extend_from_slice(&[0, 73, 73, 0]);
        out
    }

    #[test]
    fn glyph_records_precede_an_atlas_the_palette_colours_and_keys_on_index_0() {
        let font = parse_font(&tfnt(), "t").unwrap();
        assert_eq!((font.cell_width, font.height, font.v_span, font.spacing), (15, 1, 1.0, 1));
        assert!(font.glyphs[usize::from(b'A')].drawn());
        assert!(!font.glyphs[usize::from(b' ')].drawn());

        let mut pal = vec![0u8; PAL_SIZE];
        pal[73 * 4..73 * 4 + 3].copy_from_slice(&[255, 255, 255]);
        pal[PALETTE_SIZE..PALETTE_SIZE + 4].copy_from_slice(PALETTE_TAG);
        let palette = parse_palette(&pal, "p").unwrap();
        assert_eq!(palette.colour(73), [255, 255, 255]);
        let atlas = font.decode_atlas(&palette, "t").unwrap();
        assert_eq!(atlas.levels[0][4..8], [255, 255, 255, 255]);
        // Index 0 is the colour key: the palette's alpha is cleared on an alpha surface.
        assert_eq!(atlas.levels[0][0..4], [0, 0, 0, 0]);
        assert!(parse_palette(&pal[..100], "p").is_err());
    }
}
