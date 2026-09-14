//! A building's `.bas`, its ground plan: rings of corners in the model's frame, the traced
//! inner ring carrying where each corner was taken off the mesh, the outer clearance ring
//! none. See `docs/07-objects.md`, "`.bas` is a building's ground plan", and
//! `docs/03-terrain.md`, "Placing a building cuts the landscape".

use crate::FormatError;

/// The word every ring starts with.
pub const RING_MARKER: i32 = 1;

/// One ring: its corners without the closing repeat, and each traced corner's (triangle,
/// corner) into the building's own mesh, empty on the outer ring.
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    pub points: Vec<[f32; 3]>,
    pub traced: Vec<(i32, i32)>,
}

fn i32_at(data: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// Parse a `.bas`. Every shipped record holds two rings: the traced inner one, then the
/// outer one.
pub fn parse(data: &[u8], name: &str) -> Result<Vec<Ring>, FormatError> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        if pos + 8 > data.len() {
            return Err(FormatError::invalid(name, "no room for a ring header"));
        }
        let (marker, count) = (i32_at(data, pos), i32_at(data, pos + 4));
        if marker != RING_MARKER || !(3..=256).contains(&count) {
            return Err(FormatError::invalid(name, "bad ring header"));
        }
        let count = count as usize;
        let end = pos + 8 + (count + 1) * 12;
        if end > data.len() {
            return Err(FormatError::invalid(name, "a ring runs past the end"));
        }
        let points: Vec<[f32; 3]> = (0..=count)
            .map(|i| {
                let at = pos + 8 + i * 12;
                [f32_at(data, at), f32_at(data, at + 4), f32_at(data, at + 8)]
            })
            .collect();
        if points[0] != points[count] {
            return Err(FormatError::invalid(name, "a ring does not close"));
        }
        pos = end;
        let mut traced = Vec::new();
        if pos < data.len() {
            if pos + 8 * count > data.len() {
                return Err(FormatError::invalid(name, "no room for the traced corners"));
            }
            traced = (0..count)
                .map(|i| (i32_at(data, pos + 4 * i), i32_at(data, pos + 4 * (count + i))))
                .collect();
            pos += 8 * count;
        }
        out.push(Ring { points: points[..count].to_vec(), traced });
    }
    Ok(out)
}

/// Whether (x, y) lies inside a ring's outline, by the crossing test.
pub fn contains(points: &[[f32; 2]], x: f32, y: f32) -> bool {
    let mut inside = false;
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if (a[1] > y) != (b[1] > y) && x < a[0] + (y - a[1]) / (b[1] - a[1]) * (b[0] - a[0]) {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(points: &[[f32; 3]], traced: bool) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(RING_MARKER.to_le_bytes());
        out.extend((points.len() as i32).to_le_bytes());
        for p in points.iter().chain(std::iter::once(&points[0])) {
            for v in p {
                out.extend(v.to_le_bytes());
            }
        }
        if traced {
            for i in 0..points.len() as i32 {
                out.extend(i.to_le_bytes());
            }
            for i in 0..points.len() as i32 {
                out.extend((i % 3).to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn a_ground_plan_reads_its_traced_inner_ring_and_its_outer_ring() {
        let square = [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0]];
        let wide = square.map(|p| [p[0] * 2.0, p[1] * 2.0, 0.0]);
        let mut data = ring(&square, true);
        data.extend(ring(&wide, false));
        let rings = parse(&data, "t.bas").unwrap();
        assert_eq!(rings.len(), 2);
        assert_eq!((rings[0].points.len(), rings[0].traced[1]), (4, (1, 1)));
        assert!(rings[1].traced.is_empty());
        let flat: Vec<[f32; 2]> = rings[0].points.iter().map(|p| [p[0], p[1]]).collect();
        assert!(contains(&flat, 0.0, 0.5) && !contains(&flat, 1.5, 0.0));
    }
}
