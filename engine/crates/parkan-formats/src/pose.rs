//! A node's pose: a translation and a unit quaternion `(w, x, y, z)`.
//!
//! Kept in `f64` so composing a part tree matches the Python reference to the
//! golden tolerance; a renderer narrows the result.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
}

pub const IDENTITY: Pose = Pose { translation: [0.0; 3], rotation: [1.0, 0.0, 0.0, 0.0] };

pub fn multiply(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [w1, x1, y1, z1] = a;
    let [w2, x2, y2, z2] = b;
    [
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
    ]
}

pub fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let [w, x, y, z] = q;
    let [vx, vy, vz] = v;
    let tx = 2.0 * (y * vz - z * vy);
    let ty = 2.0 * (z * vx - x * vz);
    let tz = 2.0 * (x * vy - y * vx);
    [vx + w * tx + (y * tz - z * ty), vy + w * ty + (z * tx - x * tz), vz + w * tz + (x * ty - y * tx)]
}

/// Two unit rotations interpolated the short way round.
pub fn slerp(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    let mut dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mut b = b;
    if dot < 0.0 {
        b = b.map(|v| -v);
        dot = -dot;
    }
    let out: [f64; 4] = if dot > 0.9995 {
        std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))
    } else {
        let theta = dot.acos();
        let (sa, sb, s) = (((1.0 - t) * theta).sin(), (t * theta).sin(), theta.sin());
        std::array::from_fn(|i| (sa * a[i] + sb * b[i]) / s)
    };
    let norm = out.iter().map(|v| v * v).sum::<f64>().sqrt();
    let norm = if norm == 0.0 { 1.0 } else { norm };
    out.map(|v| v / norm)
}

impl Pose {
    /// A lerp of the translations and a slerp of the rotations.
    pub fn blend(&self, other: &Pose, t: f64) -> Pose {
        Pose {
            translation: std::array::from_fn(|i| {
                self.translation[i] + t * (other.translation[i] - self.translation[i])
            }),
            rotation: slerp(self.rotation, other.rotation, t),
        }
    }

    /// Place `child` in this pose's frame.
    pub fn compose(&self, child: &Pose) -> Pose {
        let r = rotate(self.rotation, child.translation);
        let t = self.translation;
        Pose {
            translation: [t[0] + r[0], t[1] + r[1], t[2] + r[2]],
            rotation: multiply(self.rotation, child.rotation),
        }
    }

    /// The pose that undoes this one.
    pub fn invert(&self) -> Pose {
        let [w, x, y, z] = self.rotation;
        let inverse = [w, -x, -y, -z];
        let [tx, ty, tz] = self.translation;
        Pose { translation: rotate(inverse, [-tx, -ty, -tz]), rotation: inverse }
    }

    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let r = rotate(self.rotation, p);
        [r[0] + self.translation[0], r[1] + self.translation[1], r[2] + self.translation[2]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pose_composed_with_its_inverse_is_identity() {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let p = Pose { translation: [1.0, 2.0, 3.0], rotation: [h, 0.0, 0.0, h] };
        let back = p.compose(&p.invert());
        for (a, b) in back.translation.iter().zip(IDENTITY.translation) {
            assert!((a - b).abs() < 1e-12);
        }
        let q = p.apply([1.0, 0.0, 0.0]);
        assert!((q[0] - 1.0).abs() < 1e-12 && (q[1] - 3.0).abs() < 1e-12);
    }
}
