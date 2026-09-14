//! `behpsp.res`'s behaviour profiles, the binary `.var` files: a list of named, typed
//! variables with a value, a default and bounds. Ports `openparkan.profiles.parse`.

use crate::cursor::{FormatError, u32_at};

/// The archive the profiles are members of, at the install's root.
pub const ARCHIVE: &str = "behpsp.res";
/// A chassis profile's locomotion: 1 flying, 2 walking, 3 wheeled, 4 tracked.
pub const CHASSIS_TYPE: &str = "ChassisType";

pub const TYPE_BOOL: u32 = 2;
pub const TYPE_FLOAT: u32 = 3;
pub const TYPE_DWORD: u32 = 5;
const VALUE_SIZE: u32 = 4;

/// One variable: its value, default, minimum and maximum, a float's as written and a
/// BOOL's or DWORD's as the whole number it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    pub name: String,
    pub kind: u32,
    pub value: f64,
    pub default: f64,
    pub minimum: f64,
    pub maximum: f64,
}

/// A `.var` file's variables in file order.
pub fn parse(data: &[u8], source: &str) -> Result<Vec<Variable>, FormatError> {
    let word = |at: usize| u32_at(data, at).ok_or_else(|| FormatError::invalid(source, "runs past the end"));
    let count = word(0)?;
    let mut at = 4;
    let mut out = Vec::new();
    for _ in 0..count {
        let (size, kind, length) = (word(at)?, word(at + 4)?, word(at + 12)? as usize);
        if size != VALUE_SIZE || !matches!(kind, TYPE_BOOL | TYPE_FLOAT | TYPE_DWORD) {
            return Err(FormatError::invalid(source, format!("record ({size}, {kind}) at {at}")));
        }
        let name = data
            .get(at + 16..at + 16 + length)
            .ok_or_else(|| FormatError::invalid(source, "a name runs past the end"))?;
        let values = at + 16 + length + 4;
        let pick = |i: usize| {
            word(values + 4 * i)
                .map(|w| if kind == TYPE_FLOAT { f64::from(f32::from_bits(w)) } else { f64::from(w as i32) })
        };
        out.push(Variable {
            name: name.iter().map(|&b| char::from(b)).collect(),
            kind,
            value: pick(0)?,
            default: pick(1)?,
            minimum: pick(2)?,
            maximum: pick(3)?,
        });
        at = values + 16;
    }
    if at != data.len() {
        return Err(FormatError::invalid(
            source,
            format!("{} bytes left over", data.len().saturating_sub(at)),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: u32, name: &str, values: [u32; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        for w in [VALUE_SIZE, kind, 0, name.len() as u32] {
            out.extend(w.to_le_bytes());
        }
        out.extend(name.as_bytes());
        out.extend(0u32.to_le_bytes());
        for w in values {
            out.extend(w.to_le_bytes());
        }
        out
    }

    #[test]
    fn a_profile_is_its_variables_in_order_floats_and_whole_numbers_apart() {
        let mut data = 2u32.to_le_bytes().to_vec();
        data.extend(record(TYPE_DWORD, CHASSIS_TYPE, [2, 0, 0, 5]));
        data.extend(record(TYPE_FLOAT, "Speed", [1.5f32.to_bits(), 1.0f32.to_bits(), 0, 10.0f32.to_bits()]));
        let v = parse(&data, "t").unwrap();
        assert_eq!((v[0].name.as_str(), v[0].value, v[0].maximum), (CHASSIS_TYPE, 2.0, 5.0));
        assert_eq!((v[1].value, v[1].default, v[1].maximum), (1.5, 1.0, 10.0));
        assert!(parse(&data[..data.len() - 1], "t").is_err());
    }
}
