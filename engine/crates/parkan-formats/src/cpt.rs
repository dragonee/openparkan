//! `CTPT` control points: named points on a model. See `docs/07-objects.md`,
//! "CTPT — control points", and `openparkan/mesh.py`.
//!
//! Two parallel arrays: a count, `count` × nine float32, then `count` × a 32-byte
//! name.

use crate::cursor::{FormatError, latin1, u32_at};

pub const CTPT_TAG: &str = "CTPT";
pub const NUMERIC: usize = 36;
pub const NAME: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct ControlPoint {
    pub name: String,
    /// Exactly zero on most points; where not, slots 1 and 2 hold int32 node numbers.
    pub a: [f32; 3],
    /// In the frame of the node it sits on.
    pub position: [f32; 3],
    /// A direction whose length carries a magnitude.
    pub direction: [f32; 3],
}

impl ControlPoint {
    /// The first triple's second and third slots read as the int32 they are: the
    /// node of the same-stem mesh the point sits on, and a second node.
    pub fn nodes(&self) -> (i32, i32) {
        (self.a[1].to_bits() as i32, self.a[2].to_bits() as i32)
    }
}

pub fn parse(b: &[u8], source: &str) -> Result<Vec<ControlPoint>, FormatError> {
    let count = u32_at(b, 0).ok_or_else(|| FormatError::invalid(source, "no count"))? as usize;
    let expected = 4 + count * (NUMERIC + NAME);
    if expected != b.len() {
        return Err(FormatError::invalid(
            source,
            format!("{count} control points implies {expected} bytes, have {}", b.len()),
        ));
    }
    let names_at = 4 + count * NUMERIC;
    Ok((0..count)
        .map(|i| {
            let at = 4 + i * NUMERIC;
            let f = |k: usize| f32::from_le_bytes(b[at + 4 * k..at + 4 * k + 4].try_into().expect("4 bytes"));
            let raw = &b[names_at + i * NAME..names_at + (i + 1) * NAME];
            let end = raw.iter().position(|&c| c == 0).unwrap_or(NAME);
            ControlPoint {
                name: latin1(&raw[..end]),
                a: [f(0), f(1), f(2)],
                position: [f(3), f(4), f(5)],
                direction: [f(6), f(7), f(8)],
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_carries_its_node_in_the_first_triple() {
        let mut b = 1u32.to_le_bytes().to_vec();
        for v in [0.0_f32, f32::from_bits(35), f32::from_bits(35), 0.0, 0.0, 0.876, 0.0, 1.0, 0.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&[b"CameraCenter".as_slice(), &[0u8; 20]].concat());
        let p = parse(&b, "t").unwrap();
        assert_eq!(p[0].name, "CameraCenter");
        assert_eq!(p[0].nodes(), (35, 35));
        assert_eq!(p[0].direction, [0.0, 1.0, 0.0]);
        assert!(parse(&b[..b.len() - 1], "t").is_err());
    }
}
