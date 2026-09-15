//! `FXID` effects: a 60-byte header, then typed emitter blocks. See
//! `docs/11-effects.md` and `openparkan/effects.py`.

use crate::cursor::{FormatError, u32_at};
use crate::objects::{ResourceRef, fixed};

pub const FXID_TAG: &str = "FXID";
pub const HEADER_SIZE: usize = 60;

/// Header flags (`docs/11-effects.md`, "How an effect runs").
pub const FX_JITTER: u32 = 0x1;
pub const FX_DELETE_AT_END: u32 = 0x2;
pub const FX_RANDOM_OFFSET: u32 = 0x8;
pub const FX_KEEP_WHEN_HIDDEN: u32 = 0x10;
pub const FX_PING_PONG: u32 = 0x20;
pub const FX_START_OFF: u32 = 0x40;
pub const FX_TIMES_LINEAR: u32 = 0x200;
/// Drawn only by a draw call that passes its pass argument: lights, sounds, breath and
/// beacons.
pub const FX_PASS_ONLY: u32 = 0x800;

/// Time modes.
pub const TIME_MANUAL: u32 = 0;
pub const TIME_ONCE: u32 = 1;
pub const TIME_LOOP: u32 = 2;
pub const TIME_REVERSE: u32 = 3;
pub const TIME_POINT: u32 = 4;
pub const TIME_SPEED: u32 = 5;
pub const TIME_SPIN: u32 = 9;
pub const TIME_MOTION: u32 = 15;

pub const EMITTER_LIGHT: u8 = 1;
pub const EMITTER_SOUND: u8 = 2;
pub const EMITTER_FLAG: u32 = 0x100;

/// Emitter type -> the block's length, as `Effect.dll`'s factory advances.
pub fn emitter_size(kind: u8) -> Option<usize> {
    Some(match kind {
        1 => 224,
        2 => 148,
        3 => 200,
        4 => 204,
        5 => 112,
        6 => 4,
        7 => 208,
        8 => 248,
        9 | 10 => 208,
        _ => return None,
    })
}

/// Emitter type -> where its `(archive, member)` pair starts.
pub fn resource_at(kind: u8) -> Option<usize> {
    Some(match kind {
        2 => 84,
        3 | 4 | 9 => 136,
        5 => 48,
        7 | 10 => 144,
        8 => 184,
        _ => return None,
    })
}

/// Emitter type -> where its `(low, high)` window of effect time sits.
pub fn window_at(kind: u8) -> Option<usize> {
    Some(match kind {
        1 | 2 => 8,
        3 | 4 | 9 => 32,
        5 => 12,
        7 | 10 => 20,
        8 => 16,
        _ => return None,
    })
}

/// Emitter type -> the offsets its own class loads as a float.
pub fn read_offsets(kind: u8) -> &'static [usize] {
    const SPRITE: &[usize] = &[8, 12, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 100, 104, 108, 112, 116, 120];
    const PARTICLES: &[usize] = &[
        12, 16, 20, 24, 28, 32, 44, 48, 52, 56, 60, 64, 68, 72, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112,
        116, 120, 124, 128, 132, 136, 140,
    ];
    match kind {
        1 => &[8, 12, 28, 32, 36, 52, 56, 60, 80, 84, 88, 92, 112, 116, 120],
        2 => &[8, 12, 28, 32, 36, 52, 56, 60, 64, 68, 72, 76],
        3 | 4 | 9 => SPRITE,
        5 => &[4, 8, 12, 16, 24, 28, 32, 36, 40, 44],
        7 | 10 => PARTICLES,
        8 => &[
            8, 12, 16, 20, 24, 28, 32, 52, 56, 60, 88, 92, 96, 100, 104, 108, 112, 116, 120, 136, 140, 144,
            148, 152, 156, 160, 164, 168,
        ],
        _ => &[],
    }
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    b.get(at..at + 4).map_or(0.0, |s| f32::from_le_bytes(s.try_into().expect("4 bytes")))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub count: i32,
    pub mode: u32,
    /// Seconds from start to end.
    pub duration: f32,
    pub jitter: f32,
    pub flags: u32,
    pub gate: u32,
    pub offset: [f32; 3],
    pub point: [f32; 3],
    /// What a requested size is multiplied by.
    pub scale: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    pub kind: u8,
    /// The whole type word, flag bits included.
    pub word: u32,
    pub resource: ResourceRef,
    pub body: Vec<u8>,
}

impl Emitter {
    /// The float at `at` in the block.
    pub fn f(&self, at: usize) -> f32 {
        f32_at(&self.body, at)
    }

    pub fn triple(&self, at: usize) -> [f32; 3] {
        [self.f(at), self.f(at + 4), self.f(at + 8)]
    }

    /// The `(low, high)` span of effect time this emitter is active in.
    pub fn window(&self) -> Option<(f32, f32)> {
        let at = window_at(self.kind)?;
        (at + 8 <= self.body.len()).then(|| (self.f(at), self.f(at + 4)))
    }

    /// The floats this emitter's class reads, by offset.
    pub fn live_floats(&self) -> Vec<(usize, f32)> {
        read_offsets(self.kind)
            .iter()
            .filter(|&&at| at + 4 <= self.body.len())
            .map(|&at| (at, self.f(at)))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub name: String,
    pub header: Header,
    pub emitters: Vec<Emitter>,
}

pub fn parse(b: &[u8], name: &str) -> Result<Effect, FormatError> {
    let bad = |m: String| FormatError::invalid(name, m);
    if b.len() < HEADER_SIZE {
        return Err(bad(format!("{} bytes, short of the header", b.len())));
    }
    let count = u32_at(b, 0).expect("inside") as i32;
    if count < 0 {
        return Err(bad(format!("negative emitter count {count}")));
    }
    let triple = |at: usize| [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)];
    let header = Header {
        count,
        mode: u32_at(b, 4).expect("inside"),
        duration: f32_at(b, 8),
        jitter: f32_at(b, 12),
        flags: u32_at(b, 16).expect("inside"),
        gate: u32_at(b, 20).expect("inside"),
        offset: triple(24),
        point: triple(36),
        scale: triple(48),
    };
    let mut emitters = Vec::with_capacity(count as usize);
    let mut pos = HEADER_SIZE;
    for i in 0..count {
        let word = u32_at(b, pos).ok_or_else(|| bad(format!("emitter {i} starts past the end")))?;
        let kind = (word & 0xFF) as u8;
        let size = emitter_size(kind)
            .filter(|s| pos + s <= b.len())
            .ok_or_else(|| bad(format!("emitter {i} has type {word:#x} at offset {pos}")))?;
        let resource = resource_at(kind).map_or_else(ResourceRef::default, |at| ResourceRef {
            library: fixed(&b[pos + at..pos + at + 32]),
            member: fixed(&b[pos + at + 32..pos + at + 64]),
        });
        emitters.push(Emitter { kind, word, resource, body: b[pos..pos + size].to_vec() });
        pos += size;
    }
    Ok(Effect { name: name.to_owned(), header, emitters })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effect_is_a_header_and_its_typed_blocks() {
        let mut b = vec![0u8; HEADER_SIZE];
        b[0..4].copy_from_slice(&2u32.to_le_bytes());
        b[4..8].copy_from_slice(&TIME_ONCE.to_le_bytes());
        b[8..12].copy_from_slice(&1.5_f32.to_le_bytes());
        let mut sprite = vec![0u8; 200];
        sprite[0..4].copy_from_slice(&(3u32 | EMITTER_FLAG).to_le_bytes());
        sprite[32..36].copy_from_slice(&0.01_f32.to_le_bytes());
        sprite[36..40].copy_from_slice(&0.5_f32.to_le_bytes());
        sprite[136 + 32..136 + 32 + 5].copy_from_slice(b"GLOW1");
        let mut sound = vec![0u8; 148];
        sound[0] = EMITTER_SOUND;
        b.extend(sprite);
        b.extend(sound);
        let e = parse(&b, "t").unwrap();
        assert_eq!((e.header.mode, e.header.duration), (TIME_ONCE, 1.5));
        assert_eq!(e.emitters[0].window(), Some((0.01, 0.5)));
        assert_eq!(e.emitters[0].resource.member, "GLOW1");
        assert_eq!(e.emitters[1].kind, EMITTER_SOUND);
        assert!(parse(&b[..b.len() - 1], "t").is_err());
    }
}
