//! `sky.ske`, a mission's atmosphere: day cycles of keyframes. See
//! `docs/10-sky.md` and `openparkan/sky.py`.

use crate::cursor::{FormatError, latin1, u32_at};

pub const MAGIC: u32 = 0xFFFF_FFFF;
pub const VERSION: u32 = 5;
pub const HEADER_SIZE: usize = 124;
pub const SECTION_HEADER_SIZE: usize = 72;
pub const SLOT_COUNT: usize = 22;
pub const NAME_SLOTS: usize = 6;
pub const DAY_LENGTH_AT: usize = 64;
pub const CLOCK_DAY: f64 = 86400.0;
/// A trailer whose kind word is this has one more word before the hour.
pub const KIND_WITH_PADDING: u32 = 3;

/// The slots the sky reads (`docs/10-sky.md`, "How a keyframe reaches the sky").
pub const HORIZON_SLOTS: [usize; 4] = [2, 3, 1, 4];
pub const RING3_SLOTS: [usize; 4] = [7, 10, 8, 9];
pub const RING2_SLOTS: [usize; 4] = [11, 14, 12, 13];
pub const APEX_SLOT: usize = 15;
pub const FOG_START_SLOT: usize = 5;
pub const FOG_END_SLOT: usize = 6;
pub const FOG_SCALE: f32 = 700.0;
pub const SUN_LIGHT_SLOT: usize = 19;
pub const SCENE_COLOUR_SLOT: usize = 20;

#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub hour: u32,
    pub minute: u32,
    pub section: usize,
    /// 22 four-byte slots as stored: mostly BGRA colours.
    pub slots: [[u8; 4]; SLOT_COUNT],
    pub name: String,
    pub sounds: Vec<String>,
    pub intensity: [f32; 4],
    pub trailer: [u32; 10],
}

impl Keyframe {
    /// A slot as `(r, g, b, a)`; stored BGRA.
    pub fn colour(&self, slot: usize) -> [u8; 4] {
        let [b, g, r, a] = self.slots[slot];
        [r, g, b, a]
    }

    pub fn number(&self, slot: usize) -> f32 {
        f32::from_le_bytes(self.slots[slot])
    }

    pub fn minutes(&self) -> u32 {
        self.hour * 60 + self.minute
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Atmosphere {
    pub sections: u32,
    pub header: Vec<u8>,
    pub keyframes: Vec<Keyframe>,
}

impl Atmosphere {
    /// How long a day lasts in real seconds (header +64: hours, minutes).
    pub fn day_seconds(&self) -> f64 {
        let hours = u32_at(&self.header, DAY_LENGTH_AT).unwrap_or(0);
        let minutes = u32_at(&self.header, DAY_LENGTH_AT + 4).unwrap_or(0);
        f64::from(hours * 3600 + minutes * 60)
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
    source: &'a str,
}

impl Reader<'_> {
    fn err(&self, what: &str) -> FormatError {
        FormatError::invalid(self.source, format!("{what} at {}", self.pos))
    }

    fn u32(&mut self) -> Result<u32, FormatError> {
        let v = u32_at(self.b, self.pos).ok_or_else(|| self.err("read past the end"))?;
        self.pos += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, FormatError> {
        let n = self.u32()? as usize;
        if n > 4096 || self.pos + n > self.b.len() {
            return Err(self.err(&format!("implausible string length {n}")));
        }
        let s = latin1(&self.b[self.pos..self.pos + n]);
        self.pos += n;
        Ok(s)
    }

    fn keyframe(&mut self, section: usize) -> Result<Keyframe, FormatError> {
        if self.pos + 88 > self.b.len() {
            return Err(self.err("keyframe runs past the end"));
        }
        let at = self.pos;
        let slots = std::array::from_fn(|i| self.b[at + i * 4..at + i * 4 + 4].try_into().expect("4 bytes"));
        self.pos += 88;
        let names: Vec<String> = (0..NAME_SLOTS).map(|_| self.string()).collect::<Result<_, _>>()?;
        let mut intensity = [0.0_f32; 4];
        for v in &mut intensity {
            *v = f32::from_bits(self.u32()?);
        }
        let count = self.u32()?;
        if count > 64 {
            return Err(self.err(&format!("implausible sound count {count}")));
        }
        let sounds: Vec<String> = (0..count).map(|_| self.string()).collect::<Result<Vec<_>, _>>()?;
        let mut trailer = [0u32; 10];
        for v in &mut trailer {
            *v = self.u32()?;
        }
        let t = if trailer[0] == KIND_WITH_PADDING { 4 } else { 3 };
        Ok(Keyframe {
            hour: trailer[t],
            minute: trailer[t + 1],
            section,
            slots,
            name: names.into_iter().find(|n| !n.is_empty()).unwrap_or_default(),
            sounds: sounds.into_iter().filter(|s| !s.is_empty()).collect(),
            intensity,
            trailer,
        })
    }
}

pub fn parse(b: &[u8], source: &str) -> Result<Atmosphere, FormatError> {
    if b.len() < HEADER_SIZE {
        return Err(FormatError::invalid(source, "too short to be an atmosphere file"));
    }
    let word = |at| u32_at(b, at).expect("inside");
    if word(0) != MAGIC || word(4) != VERSION {
        return Err(FormatError::invalid(source, format!("magic {:#x}, version {}", word(0), word(4))));
    }
    let mut r = Reader { b, pos: HEADER_SIZE, source };
    let mut keyframes = Vec::new();
    for _ in 0..word(16) {
        keyframes.push(r.keyframe(0)?);
    }
    let mut section = 1;
    while r.pos < b.len() {
        r.pos += SECTION_HEADER_SIZE;
        while r.pos < b.len() {
            keyframes.push(r.keyframe(section)?);
        }
        section += 1;
    }
    if r.pos != b.len() {
        return Err(FormatError::invalid(source, format!("parsed {} bytes of {}", r.pos, b.len())));
    }
    Ok(Atmosphere { sections: word(8), header: b[..HEADER_SIZE].to_vec(), keyframes })
}
