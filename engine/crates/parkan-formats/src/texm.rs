//! `Texm`, the texture format. See `docs/02-texm.md`.

use crate::cursor::{FormatError, u32_at};

pub const MAGIC: &[u8; 4] = b"Texm";
pub const HEADER_SIZE: usize = 32;
pub const PALETTE_SIZE: usize = 256 * 4;
pub const PAGE_MAGIC: &[u8; 4] = b"Page";

pub const FMT_PALETTE8: u32 = 0;
/// Indices into an external palette; only the font atlas uses it.
pub const FMT_INDEX8: u32 = 2;
pub const FMT_RGB565: u32 = 565;
pub const FMT_ARGB4444: u32 = 4444;
pub const FMT_XRGB8888: u32 = 888;
pub const FMT_ARGB8888: u32 = 8888;

/// Header +0x14 bits that would give a texture an alpha surface or a faded
/// palette (`Ngi32.dll:0x1000fdf6`); no shipped texture sets either.
pub const ALPHA_SURFACE: u32 = 0x0100_0000;
pub const FADE_PALETTE: u32 = 0x0200_0000;

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    /// The mip level count the header declares.
    pub mips: u32,
    pub flags: u32,
    pub flags14: u32,
    /// Every mip level the file carries, largest first, RGBA8 rows top first.
    pub levels: Vec<Vec<u8>>,
    /// Sub-images as `(x, y, width, height)`, from the `Page` chunk.
    pub pages: Vec<[u16; 4]>,
}

impl Texture {
    /// Whether the engine uploads it with an alpha channel: only 4444, 8888
    /// and the two header bits do, so a palettised texture is always opaque.
    pub fn has_alpha(&self) -> bool {
        matches!(self.format, FMT_ARGB4444 | FMT_ARGB8888)
            || self.flags14 & (ALPHA_SURFACE | FADE_PALETTE) != 0
    }

    pub fn level_size(&self, level: usize) -> (u32, u32) {
        ((self.width >> level).max(1), (self.height >> level).max(1))
    }
}

fn bytes_per_pixel(format: u32) -> Option<usize> {
    match format {
        FMT_PALETTE8 | FMT_INDEX8 => Some(1),
        FMT_RGB565 | FMT_ARGB4444 => Some(2),
        FMT_XRGB8888 | FMT_ARGB8888 => Some(4),
        _ => None,
    }
}

fn decode_level(format: u32, body: &[u8], palette: &[u8], pixels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels * 4);
    for i in 0..pixels {
        let px = match format {
            FMT_RGB565 => {
                let v = u16::from_le_bytes([body[2 * i], body[2 * i + 1]]);
                let (r, g, b) = ((v >> 11) & 0x1F, (v >> 5) & 0x3F, v & 0x1F);
                [((r << 3) | (r >> 2)) as u8, ((g << 2) | (g >> 4)) as u8, ((b << 3) | (b >> 2)) as u8, 255]
            }
            FMT_ARGB4444 => {
                let v = u16::from_le_bytes([body[2 * i], body[2 * i + 1]]);
                let c = |s: u16| (((v >> s) & 0xF) * 17) as u8;
                [c(8), c(4), c(0), c(12)]
            }
            FMT_XRGB8888 | FMT_ARGB8888 => {
                let p = &body[4 * i..4 * i + 4];
                [p[2], p[1], p[0], if format == FMT_XRGB8888 { 255 } else { p[3] }]
            }
            _ => {
                let j = usize::from(body[i]) * 4;
                [palette[j + 2], palette[j + 1], palette[j], 255]
            }
        };
        out.extend_from_slice(&px);
    }
    out
}

/// Decode a `Texm` blob. `external` is the palette a format-2 texture uses;
/// without one it comes out a grey ramp.
pub fn decode(data: &[u8], source: &str, external: Option<&[u8]>) -> Result<Texture, FormatError> {
    if data.get(..4) != Some(MAGIC.as_slice()) {
        return Err(FormatError::invalid(source, "not a Texm blob"));
    }
    let word = |at| u32_at(data, at).ok_or_else(|| FormatError::invalid(source, "short header"));
    let (width, height, mips, flags, flags14, format) =
        (word(4)?, word(8)?, word(12)?, word(16)?, word(20)?, word(28)?);
    let bpp = bytes_per_pixel(format)
        .ok_or_else(|| FormatError::invalid(source, format!("unknown pixel format {format}")))?;
    let mut at = HEADER_SIZE;
    let grey: Vec<u8> = (0..256).flat_map(|i| [i as u8, i as u8, i as u8, 0]).collect();
    let palette: &[u8] = match format {
        FMT_PALETTE8 => {
            let p = data
                .get(at..at + PALETTE_SIZE)
                .ok_or_else(|| FormatError::invalid(source, "short palette"))?;
            at += PALETTE_SIZE;
            p
        }
        FMT_INDEX8 => external.unwrap_or(&grey),
        _ => &[],
    };
    let mut levels = Vec::new();
    for level in 0..mips.max(1) {
        let (w, h) = ((width >> level).max(1) as usize, (height >> level).max(1) as usize);
        let need = w * h * bpp;
        let Some(body) = data.get(at..at + need) else {
            if level == 0 {
                return Err(FormatError::invalid(source, "truncated pixel data"));
            }
            break;
        };
        levels.push(decode_level(format, body, palette, w * h));
        at += need;
    }
    let mut pages = Vec::new();
    if data.get(at..at + 4) == Some(PAGE_MAGIC.as_slice())
        && let Some(count) = u32_at(data, at + 4)
    {
        let count = count as usize;
        if at + 8 + count * 8 <= data.len() {
            for i in 0..count {
                let r = at + 8 + i * 8;
                let w = |o: usize| u16::from_le_bytes([data[r + o], data[r + o + 1]]);
                pages.push([w(0), w(4), w(2), w(6)]);
            }
        }
    }
    Ok(Texture { width, height, format, mips, flags, flags14, levels, pages })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(w: u32, h: u32, mips: u32, format: u32) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        for v in [w, h, mips, 0, 0, 0, format] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    #[test]
    fn rgb565_expands_to_full_range() {
        let mut data = header(1, 1, 1, FMT_RGB565);
        data.extend_from_slice(&0xFFFFu16.to_le_bytes());
        let t = decode(&data, "t", None).unwrap();
        assert_eq!(t.levels[0], vec![255, 255, 255, 255]);
        assert!(!t.has_alpha());
    }

    #[test]
    fn argb8888_is_stored_bgra_and_keeps_every_mip() {
        let mut data = header(2, 2, 2, FMT_ARGB8888);
        for _ in 0..4 {
            data.extend_from_slice(&[10, 20, 30, 40]);
        }
        data.extend_from_slice(&[1, 2, 3, 4]);
        let t = decode(&data, "t", None).unwrap();
        assert_eq!(&t.levels[0][..4], &[30, 20, 10, 40]);
        assert_eq!(t.levels[1], vec![3, 2, 1, 4]);
        assert!(t.has_alpha());
    }

    #[test]
    fn a_palette_comes_first_and_a_page_table_last() {
        let mut data = header(1, 1, 1, FMT_PALETTE8);
        let mut palette = vec![0u8; PALETTE_SIZE];
        palette[4..8].copy_from_slice(&[9, 8, 7, 0]);
        data.extend_from_slice(&palette);
        data.push(1);
        data.extend_from_slice(PAGE_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        for v in [5u16, 6, 7, 8] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        let t = decode(&data, "t", None).unwrap();
        assert_eq!(t.levels[0], vec![7, 8, 9, 255]);
        assert_eq!(t.pages, vec![[5, 7, 6, 8]]);
    }
}
