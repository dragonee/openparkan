//! `.ndp` damage tables: one record per node of a model. See
//! `docs/07-objects.md` ("`.ndp` is a damage table") and `openparkan/objects.py`.

use crate::cursor::{FormatError, u32_at};
use crate::objects::{ResourceRef, fixed};

pub const NDP_TAG: &str = "NDPR";
pub const STRIDE: usize = 76;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeDamage {
    pub flags: i32,
    /// The node's hit points, before the volume scale and the level ratio.
    pub durability: f32,
    /// The node's density: times its level-0 slot's volume, its mass.
    pub density: f32,
    /// The explosion the node plays.
    pub explosion: ResourceRef,
}

pub fn parse(b: &[u8], source: &str) -> Result<Vec<NodeDamage>, FormatError> {
    let count = u32_at(b, 0).ok_or_else(|| FormatError::invalid(source, "no record count"))? as i32;
    if count < 0 || b.len() != 4 + count as usize * STRIDE {
        return Err(FormatError::invalid(
            source,
            format!("{} bytes is not {count} records of {STRIDE}", b.len()),
        ));
    }
    let f = |at: usize| f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
    Ok((0..count as usize)
        .map(|i| {
            let o = 4 + i * STRIDE;
            NodeDamage {
                flags: u32_at(b, o).expect("inside") as i32,
                durability: f(o + 4),
                density: f(o + 8),
                explosion: ResourceRef {
                    library: fixed(&b[o + 12..o + 44]),
                    member: fixed(&b[o + 44..o + STRIDE]),
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_is_flags_hit_points_density_and_an_explosion() {
        let mut b = 1u32.to_le_bytes().to_vec();
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&500.0_f32.to_le_bytes());
        b.extend_from_slice(&1000.0_f32.to_le_bytes());
        b.extend_from_slice(&[b"weapon.rlb".as_slice(), &[0; 22]].concat());
        b.extend_from_slice(&[b"bb_h_01.exp".as_slice(), &[0; 21]].concat());
        let t = parse(&b, "t").unwrap();
        assert_eq!(t[0].durability, 500.0);
        assert_eq!(t[0].explosion.member, "bb_h_01.exp");
        assert!(parse(&b[..b.len() - 1], "t").is_err());
    }
}
