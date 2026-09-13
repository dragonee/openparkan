//! `sky.ske`, a mission's atmosphere: day cycles of keyframes. See
//! `docs/10-sky.md` and `openparkan/sky.py`.
//!
//! The layout is the deserialiser's (`Terrain.dll:0x100672d0`): a file header, then
//! per section a header and its keyframes, each a version, a time and the event
//! opcode ahead of its slots; the file closes on the time the clock starts at and
//! two ints.

use crate::cursor::{FormatError, latin1, u32_at};

pub const MAGIC: u32 = 0xFFFF_FFFF;
pub const VERSION: u32 = 5;
pub const FILE_HEADER_SIZE: usize = 12;
/// A time: six `u32` and eight bytes; the section, hour and minute are read.
pub const TIME_SIZE: usize = 32;
pub const SECTION_VERSION: u32 = 1;
/// A version, a keyframe count, a time nothing asks for and the day's length.
pub const SECTION_HEADER_SIZE: usize = 4 + 4 + TIME_SIZE + TIME_SIZE;
/// The bytes before the first keyframe: the file header and section 0's header.
pub const HEADER_SIZE: usize = FILE_HEADER_SIZE + SECTION_HEADER_SIZE;
/// The time the clock starts at, an int nothing asks for, and the sky's sixth parameter.
pub const TRAILER_SIZE: usize = TIME_SIZE + 4 + 4;
pub const KEYFRAME_VERSION: u32 = 3;
pub const SLOT_COUNT: usize = 22;
pub const NAME_SLOTS: usize = 6;
/// Section 0's day length, hours then minutes, inside its header.
pub const DAY_LENGTH_AT: usize = FILE_HEADER_SIZE + 8 + TIME_SIZE + 12;
pub const CLOCK_DAY: f64 = 86400.0;
/// A version-1 keyframe stores 20 slots; 20 and 21 are slot 19's colour at this scale.
pub const V1_SCALE: f32 = 0.3;

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

/// A 32-byte time as stored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockTime(pub [u32; 8]);

impl ClockTime {
    pub fn of(hour: u32, minute: u32) -> Self {
        Self([0, 0, 0, hour, minute, 0, 0, 0])
    }

    pub fn section(&self) -> u32 {
        self.0[0]
    }

    pub fn hour(&self) -> u32 {
        self.0[3]
    }

    pub fn minute(&self) -> u32 {
        self.0[4]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub hour: u32,
    pub minute: u32,
    pub section: usize,
    /// 22 four-byte slots as stored: mostly BGRA colours.
    pub slots: [[u8; 4]; SLOT_COUNT],
    /// The first of the six name strings: `sun`, `moon` or empty.
    pub name: String,
    /// The six name strings as stored.
    pub names: Vec<String>,
    /// Rain's background sound or lightning's effect, as stored.
    pub effects: Vec<String>,
    pub intensity: [f32; 4],
    /// The event opcode `GetEvents` acts on when the clock passes here.
    pub opcode: u32,
    pub version: u32,
    /// The keyframe's own time as stored.
    pub time: ClockTime,
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

/// One day cycle's header.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub index: usize,
    pub version: u32,
    pub count: u32,
    /// 23:59 in every shipped section, and never asked for.
    pub end: ClockTime,
    /// How long this day lasts in real time.
    pub day: ClockTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Atmosphere {
    pub sections: u32,
    pub header: Vec<u8>,
    pub section_headers: Vec<Section>,
    pub keyframes: Vec<Keyframe>,
    /// Where the clock starts as the mission loads.
    pub start: ClockTime,
    pub trailer_word: u32,
    pub sky_flag: u32,
}

impl Atmosphere {
    /// How long section 0's day lasts in real seconds.
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

    fn time(&mut self) -> Result<ClockTime, FormatError> {
        let mut words = [0u32; 8];
        for w in &mut words {
            *w = self.u32()?;
        }
        Ok(ClockTime(words))
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
        if self.pos + 4 + TIME_SIZE + 4 > self.b.len() {
            return Err(self.err("keyframe runs past the end"));
        }
        let version = u32_at(self.b, self.pos).expect("inside");
        if !(1..=KEYFRAME_VERSION).contains(&version) {
            return Err(self.err(&format!("keyframe version {version}")));
        }
        self.pos += 4;
        let time = self.time()?;
        let opcode = self.u32()?;

        let stored = if version >= 2 { SLOT_COUNT } else { SLOT_COUNT - 2 };
        if self.pos + stored * 4 > self.b.len() {
            return Err(self.err("keyframe slots run past the end"));
        }
        let at = self.pos;
        let mut slots = [[0u8; 4]; SLOT_COUNT];
        for (i, slot) in slots.iter_mut().take(stored).enumerate() {
            *slot = self.b[at + i * 4..at + i * 4 + 4].try_into().expect("4 bytes");
        }
        self.pos += stored * 4;
        if version == 1 {
            let [b, g, r, _] = slots[19];
            let scale = |c: u8| (f32::from(c) * V1_SCALE).round() as u8;
            let made = [scale(b), scale(g), scale(r), 0];
            slots[20] = made;
            slots[21] = made;
        }

        let names: Vec<String> = (0..NAME_SLOTS).map(|_| self.string()).collect::<Result<_, _>>()?;
        let mut intensity = [0.0_f32; 4];
        for v in &mut intensity {
            *v = f32::from_bits(self.u32()?);
        }
        let mut effects = Vec::new();
        if version == 3 {
            let count = self.u32()?;
            if count > 64 {
                return Err(self.err(&format!("implausible effect count {count}")));
            }
            effects = (0..count).map(|_| self.string()).collect::<Result<_, _>>()?;
        }
        Ok(Keyframe {
            hour: time.hour(),
            minute: time.minute(),
            section,
            slots,
            name: names[0].clone(),
            names,
            effects,
            intensity,
            opcode,
            version,
            time,
        })
    }
}

pub fn parse(b: &[u8], source: &str) -> Result<Atmosphere, FormatError> {
    if b.len() < FILE_HEADER_SIZE {
        return Err(FormatError::invalid(source, "too short to be an atmosphere file"));
    }
    let word = |at| u32_at(b, at).expect("inside");
    if word(0) != MAGIC || word(4) != VERSION {
        return Err(FormatError::invalid(source, format!("magic {:#x}, version {}", word(0), word(4))));
    }
    let count = word(8);
    if count > 16 {
        return Err(FormatError::invalid(source, format!("implausible section count {count}")));
    }
    let mut r = Reader { b, pos: FILE_HEADER_SIZE, source };
    let mut section_headers = Vec::new();
    let mut keyframes = Vec::new();
    for index in 0..count as usize {
        let version = r.u32()?;
        let frames = r.u32()?;
        if version != SECTION_VERSION {
            return Err(r.err(&format!("section {index} version {version}")));
        }
        let end = r.time()?;
        let day = r.time()?;
        section_headers.push(Section { index, version, count: frames, end, day });
        for _ in 0..frames {
            keyframes.push(r.keyframe(index)?);
        }
    }
    if r.pos + TRAILER_SIZE != b.len() {
        let left = b.len().saturating_sub(r.pos);
        return Err(FormatError::invalid(
            source,
            format!("{left} bytes after the keyframes, expected {TRAILER_SIZE}"),
        ));
    }
    let start = r.time()?;
    let trailer_word = r.u32()?;
    let sky_flag = r.u32()?;
    Ok(Atmosphere {
        sections: count,
        header: b[..HEADER_SIZE.min(b.len())].to_vec(),
        section_headers,
        keyframes,
        start,
        trailer_word,
        sky_flag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(hour: u32, minute: u32) -> Vec<u8> {
        ClockTime::of(hour, minute).0.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn u32s(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// One section of one keyframe at 12:30 named `sun`, a 15-minute day, starting at 01:30.
    fn file() -> Vec<u8> {
        let mut b = u32s(&[MAGIC, VERSION, 1, SECTION_VERSION, 1]);
        b.extend(time(23, 59));
        b.extend(time(0, 15));
        b.extend(u32s(&[KEYFRAME_VERSION]));
        b.extend(time(12, 30));
        b.extend(u32s(&[0]));
        b.extend((0..88).map(|i| i as u8));
        b.extend(u32s(&[3]));
        b.extend(b"sun");
        b.extend(u32s(&[0; 5]));
        b.extend([2.2_f32, 2.0, 5.0, 0.0].iter().flat_map(|f| f.to_le_bytes()));
        b.extend(u32s(&[0]));
        b.extend(time(1, 30));
        b.extend(u32s(&[0, 0]));
        b
    }

    #[test]
    fn a_keyframe_carries_its_own_time_and_opcode_ahead_of_its_slots() {
        let a = parse(&file(), "sky.ske").unwrap();
        let k = &a.keyframes[0];
        assert_eq!((k.hour, k.minute, k.opcode, k.name.as_str()), (12, 30, 0, "sun"));
        assert_eq!(k.slots[1], [4, 5, 6, 7]);
        assert_eq!((a.day_seconds(), a.start.hour(), a.start.minute()), (900.0, 1, 30));
        assert_eq!(a.section_headers[0].end, ClockTime::of(23, 59));
    }

    #[test]
    fn bytes_past_the_trailer_are_refused() {
        let mut b = file();
        b.extend([0; 4]);
        assert!(parse(&b, "sky.ske").is_err());
    }
}
