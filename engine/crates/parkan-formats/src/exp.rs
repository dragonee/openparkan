//! `.exp` explosions: a hit kind, its damage and radius, and the effects it plays.
//! See `docs/26-damage.md` ("A hit, from the round to the node"), `docs/11-effects.md`
//! and `openparkan/effects.py`.

use crate::cursor::{FormatError, u32_at};
use crate::objects::{ResourceRef, fixed};

pub const EXP_TAG: &str = "EXPL";
pub const HEADER: usize = 24;
pub const STRIDE: usize = 64;
pub const SLOTS: usize = 12;
pub const SIZE: usize = HEADER + SLOTS * STRIDE;

/// The hit kinds `Control.dll:0x1000ebc0` tells apart.
pub const HIT_NONE: i32 = 1;
pub const HIT_DIRECT: i32 = 2;
pub const HIT_AREA: i32 = 3;
pub const HIT_SHIELDS: i32 = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct Explosion {
    pub kind: i32,
    pub damage: f32,
    /// Absolute on a round; a multiple of the node's bounding radius otherwise.
    pub radius: f32,
    pub values: [f32; 2],
    pub placement: i32,
    /// Slot 0 the effect; slots 1-11 one per ground surface.
    pub slots: Vec<ResourceRef>,
}

pub fn parse(b: &[u8], source: &str) -> Result<Explosion, FormatError> {
    if b.len() != SIZE {
        return Err(FormatError::invalid(
            source,
            format!("{} bytes, not the {SIZE} of an explosion", b.len()),
        ));
    }
    let f = |at: usize| f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
    Ok(Explosion {
        kind: u32_at(b, 0).expect("inside") as i32,
        damage: f(4),
        radius: f(8),
        values: [f(12), f(16)],
        placement: u32_at(b, 20).expect("inside") as i32,
        slots: (0..SLOTS)
            .map(|i| {
                let o = HEADER + i * STRIDE;
                ResourceRef { library: fixed(&b[o..o + 32]), member: fixed(&b[o + 32..o + 64]) }
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explosion_is_a_kind_a_damage_and_a_radius() {
        let mut b = vec![0u8; SIZE];
        b[0..4].copy_from_slice(&HIT_AREA.to_le_bytes());
        b[4..8].copy_from_slice(&170.0_f32.to_le_bytes());
        b[8..12].copy_from_slice(&7.0_f32.to_le_bytes());
        let e = parse(&b, "t").unwrap();
        assert_eq!((e.kind, e.damage, e.radius), (HIT_AREA, 170.0, 7.0));
        assert_eq!(e.slots.len(), SLOTS);
        assert!(parse(&b[1..], "t").is_err());
    }
}
