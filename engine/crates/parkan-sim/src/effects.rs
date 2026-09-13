//! Effects as they run: `docs/11-effects.md`, "How an effect runs", and
//! `engine/design/r5-effects.md`.
//!
//! An instance places a template on a frame and keeps its clock; effect time
//! *t* runs 0 to 1 by the time mode, and each emitter acts only inside its window
//! of *t*. What an emitter's floats mean beyond the window, the phase and the
//! sprite's moving and growing triples is not read: the drawing here is a
//! stand-in, marked where it chooses.

use std::rc::Rc;

use glam::Vec3;
use parkan_formats::fxid::{
    EMITTER_LIGHT, EMITTER_SOUND, Effect, Emitter, FX_DELETE_AT_END, FX_PING_PONG, FX_TIMES_LINEAR,
    TIME_LOOP, TIME_ONCE, TIME_REVERSE,
};

/// Where an effect sits: an origin and three axes whose lengths carry a size, as
/// a control point's vector does (`docs/07-objects.md`, "CTPT").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: Vec3,
    pub axes: [Vec3; 3],
}

impl Frame {
    /// A frame at `origin` whose first axis is `x`, the others square to it, all `size` long.
    pub fn along(origin: Vec3, x: Vec3, size: f32) -> Self {
        let x = x.normalize_or(Vec3::X);
        let helper = if x.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
        let y = helper.cross(x).normalize();
        let z = x.cross(y);
        Self { origin, axes: [x * size, y * size, z * size] }
    }

    /// The frame three control points give (action 4, `0x10002a8d`): their centroid,
    /// and their vectors as the axes.
    pub fn from_points(points: [(Vec3, Vec3); 3]) -> Self {
        let origin = (points[0].0 + points[1].0 + points[2].0) / 3.0;
        Self { origin, axes: [points[0].1, points[1].1, points[2].1] }
    }

    pub fn point(&self, local: Vec3) -> Vec3 {
        self.origin + self.axes[0] * local.x + self.axes[1] * local.y + self.axes[2] * local.z
    }
}

/// A quad to draw: a material, a centre, a length along a direction and a width.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub material: String,
    pub centre: Vec3,
    /// The quad's long side, as a vector; a square faces the camera when this is zero.
    pub along: Vec3,
    pub width: f32,
    pub alpha: f32,
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub effect: Rc<Effect>,
    pub frame: Frame,
    /// The size asked for, times the header's scale.
    pub scale: f32,
    pub start_ms: f64,
    pub end_ms: f64,
    pub mode: u32,
    /// The value mode 0 reads, and modes 4, 16 and 17 the owner's point value.
    pub value: f32,
    /// The owner's speed as a fraction of its top speed, for modes 5–15.
    pub speed: f32,
    seed: u32,
}

fn lerp3(lo: [f32; 3], hi: [f32; 3], s: f32) -> Vec3 {
    Vec3::from_array(lo).lerp(Vec3::from_array(hi), s)
}

/// A repeatable number in 0..1 from three keys.
fn noise(a: u32, b: u32, c: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    x as f32 / u32::MAX as f32
}

impl Instance {
    /// Start `effect` on `frame` (action 10): start now, end after its duration, in
    /// `mode` or its header's own.
    pub fn new(
        effect: Rc<Effect>,
        frame: Frame,
        size: f32,
        now_ms: f64,
        mode: Option<u32>,
        seed: u32,
    ) -> Self {
        let scale = size * effect.header.scale[0];
        let end_ms = now_ms + f64::from(effect.header.duration) * 1000.0;
        let mode = mode.unwrap_or(effect.header.mode);
        Self { effect, frame, scale, start_ms: now_ms, end_ms, mode, value: 0.0, speed: 0.0, seed }
    }

    /// Mode 1's progress, (now − start) ÷ (end − start); a duration of 0 is through at once.
    fn linear(&self, now_ms: f64) -> f32 {
        if self.end_ms <= self.start_ms {
            1.0
        } else {
            ((now_ms - self.start_ms) / (self.end_ms - self.start_ms)) as f32
        }
    }

    /// Effect time *t* (`Effect.dll:0x10005c60`).
    pub fn t(&self, now_ms: f64) -> f32 {
        let linear = self.linear(now_ms);
        let mut t = match self.mode {
            TIME_ONCE => linear,
            TIME_LOOP => linear - linear.floor(),
            TIME_REVERSE => 1.0 - linear,
            // STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the owner values
            // modes 5–15 read are given by the caller as one speed fraction.
            5..=15 => self.speed,
            _ => self.value,
        };
        if self.effect.header.flags & FX_TIMES_LINEAR != 0 {
            t *= linear;
        }
        if self.effect.header.flags & FX_PING_PONG != 0 {
            t = if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) };
        }
        // STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- flag 1's jitter uses
        // the engine's generator, which is not ours; it is left out.
        t.clamp(0.0, 1.0)
    }

    /// Flag 2: an instance deletes itself once *t* ≥ 1 (`0x100062a6`).
    pub fn finished(&self, now_ms: f64) -> bool {
        self.effect.header.flags & FX_DELETE_AT_END != 0 && self.t(now_ms) >= 1.0
    }

    /// What the instance draws at `now_ms`.
    pub fn sprites(&self, now_ms: f64, out: &mut Vec<Sprite>) {
        let t = self.t(now_ms);
        let seconds = ((now_ms - self.start_ms) / 1000.0) as f32;
        for (i, e) in self.effect.emitters.iter().enumerate() {
            if e.kind == EMITTER_LIGHT || e.kind == EMITTER_SOUND || e.resource.member.is_empty() {
                continue;
            }
            let Some((lo, hi)) = e.window() else { continue };
            if t < lo || t > hi {
                continue;
            }
            let p = if hi > lo { (t - lo) / (hi - lo) } else { 1.0 };
            match e.kind {
                3 | 4 | 9 => self.sprite(e, p, out),
                5 => self.bolt(e, seconds, out),
                7 | 10 => self.burst(e, i as u32, p, out),
                _ => {}
            }
        }
    }

    /// STAND-IN: docs/11-effects.md#emitter-types--read-and-measured -- a type 3, 4 or 9
    /// sprite at lerp(+40, +52) in the frame, lerp(+100, +112) in size along its axes,
    /// both by progress through the window; alpha lerp(1, +24) by progress to the power
    /// +28; the phase is left to animated textures, which are not drawn.
    fn sprite(&self, e: &Emitter, p: f32, out: &mut Vec<Sprite>) {
        let local = lerp3(e.triple(40), e.triple(52), p);
        let size = lerp3(e.triple(100), e.triple(112), p);
        let power = e.f(28);
        let fade = if power > 0.0 { p.powf(power) } else { p };
        let alpha = 1.0 + (e.f(24) - 1.0) * fade;
        let centre = self.frame.point(local * self.scale);
        let (x, y) = (self.frame.axes[0], self.frame.axes[1]);
        let long = size.x * self.scale;
        let width = size.y * self.scale * y.length();
        let stretched = (size.x - size.y).abs() > f32::EPSILON;
        out.push(Sprite {
            material: e.resource.member.clone(),
            centre,
            along: if stretched { x * long } else { Vec3::ZERO },
            width: if stretched { width } else { size.x * self.scale * x.length() },
            alpha,
        });
    }

    /// STAND-IN: docs/11-effects.md#not-resolved -- a type-5 bolt is a quad along the
    /// frame's first axis, +24 wide and min(+36, +32 × 1000 × seconds) long, behind its origin.
    fn bolt(&self, e: &Emitter, seconds: f32, out: &mut Vec<Sprite>) {
        let x = self.frame.axes[0];
        let length = e.f(36).min(e.f(32).max(1.0) * 1000.0 * seconds.max(0.001));
        out.push(Sprite {
            material: e.resource.member.clone(),
            centre: self.frame.origin + x.normalize_or(Vec3::NEG_Y) * (length / 2.0) * self.scale,
            along: x.normalize_or(Vec3::NEG_Y) * length * self.scale,
            width: e.f(24) * self.scale,
            alpha: 1.0,
        });
    }

    /// STAND-IN: docs/11-effects.md#emitter-types--read-and-measured -- a type 7 or 10
    /// burst of max(1, +16) particles, each flying from the origin at a velocity
    /// between +44 and +56 per axis (a random share of each), spread by +68/2, for
    /// +28 of the window; its size lerp(+92, +104) by its age, fading out.
    fn burst(&self, e: &Emitter, index: u32, p: f32, out: &mut Vec<Sprite>) {
        let count = e.f(16).clamp(1.0, 64.0) as u32;
        let life = if e.f(28) > 0.0 { e.f(28) } else { 1.0 };
        let age = (p / life).clamp(0.0, 1.0);
        if p > life {
            return;
        }
        for k in 0..count {
            let r = |axis: u32| noise(self.seed, index * 64 + k, axis);
            let (vlo, vhi, spread) = (e.triple(44), e.triple(56), e.triple(68));
            let velocity = Vec3::new(
                vlo[0] + (vhi[0] - vlo[0]) * r(0),
                vlo[1] + (vhi[1] - vlo[1]) * r(1),
                vlo[2] + (vhi[2] - vlo[2]) * r(2),
            );
            let offset = Vec3::new(r(3) - 0.5, r(4) - 0.5, r(5) - 0.5) * Vec3::from_array(spread);
            let local = offset + velocity * age;
            let size = e.f(92) + (e.f(104) - e.f(92)) * age;
            out.push(Sprite {
                material: e.resource.member.clone(),
                centre: self.frame.point(local * self.scale),
                along: Vec3::ZERO,
                width: size * self.scale * self.frame.axes[0].length().max(f32::EPSILON),
                alpha: 1.0 - age,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::fxid::{Header, TIME_POINT};
    use parkan_formats::objects::ResourceRef;

    fn block(kind: u8, size: usize, floats: &[(usize, f32)], material: &str) -> Emitter {
        let mut body = vec![0u8; size];
        body[0] = kind;
        for &(at, v) in floats {
            body[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        Emitter {
            kind,
            word: u32::from(kind),
            resource: ResourceRef { library: "Material.lib".into(), member: material.into() },
            body,
        }
    }

    fn effect(mode: u32, duration: f32, flags: u32, emitters: Vec<Emitter>) -> Rc<Effect> {
        Rc::new(Effect {
            name: "t".into(),
            header: Header {
                count: emitters.len() as i32,
                mode,
                duration,
                jitter: 0.0,
                flags,
                gate: 0,
                offset: [0.0; 3],
                point: [0.0; 3],
                scale: [0.1; 3],
            },
            emitters,
        })
    }

    /// `hero_cannon`'s long flash: 5 to 20 along the barrel over t 0.01–0.5.
    fn flash() -> Emitter {
        block(
            3,
            200,
            &[
                (32, 0.01),
                (36, 0.5),
                (24, 1.0),
                (28, 1.0),
                (40, 2.0),
                (52, 10.0),
                (100, 5.0),
                (104, 3.0),
                (112, 20.0),
                (116, 4.0),
            ],
            "NE_Gun01",
        )
    }

    #[test]
    fn a_muzzle_flash_runs_with_its_point_value_and_only_inside_its_window() {
        let frame = Frame { origin: Vec3::ZERO, axes: [Vec3::Y, Vec3::Z, Vec3::NEG_X] };
        let mut fx = Instance::new(effect(TIME_POINT, 0.0, 0, vec![flash()]), frame, 1.0, 0.0, None, 1);
        let mut out = Vec::new();
        fx.sprites(0.0, &mut out);
        assert!(out.is_empty(), "t 0 is before the window");
        fx.value = 0.5;
        fx.sprites(10.0, &mut out);
        let s = &out[0];
        // At the window's end: centre 10 × 0.1 along the barrel, 20 × 0.1 long, 4 × 0.1 wide.
        assert!((s.centre - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5, "{:?}", s.centre);
        assert!((s.along - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5 && (s.width - 0.4).abs() < 1e-5);
        out.clear();
        fx.value = 0.7;
        fx.sprites(20.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn an_impact_runs_once_and_deletes_itself_at_the_end() {
        let burst = block(
            7,
            208,
            &[(20, 0.0), (24, 0.6), (16, 4.0), (28, 0.5), (56, 3.0), (92, 0.1), (104, 0.3)],
            "splash6",
        );
        let fx = Instance::new(
            effect(TIME_ONCE, 1.5, FX_DELETE_AT_END, vec![burst]),
            Frame::along(Vec3::ZERO, Vec3::Z, 1.0),
            10.0,
            0.0,
            None,
            7,
        );
        let mut out = Vec::new();
        fx.sprites(150.0, &mut out);
        assert_eq!(out.len(), 4, "four particles early in their life");
        assert!(out.iter().all(|s| s.alpha < 1.0 && s.alpha > 0.0));
        assert!(!fx.finished(1000.0) && fx.finished(1500.0));
        out.clear();
        fx.sprites(1000.0, &mut out);
        assert!(out.is_empty(), "past the burst's share of the window");
    }

    #[test]
    fn looping_reversing_and_ping_pong_times() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let at = |mode, flags, ms| {
            Instance::new(effect(mode, 1.0, flags, Vec::new()), frame, 1.0, 0.0, None, 0).t(ms)
        };
        assert_eq!(at(TIME_LOOP, 0, 1250.0), 0.25);
        assert_eq!(at(TIME_REVERSE, 0, 250.0), 0.75);
        assert_eq!(at(TIME_ONCE, FX_PING_PONG, 750.0), 0.5);
        assert_eq!(at(TIME_ONCE, 0, 5000.0), 1.0);
    }
}
