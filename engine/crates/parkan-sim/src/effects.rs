//! Effects as they run: `docs/11-effects.md`, "How an effect runs" and "Bolts,
//! streams and fades", and `engine/design/r5-effects.md`.
//!
//! An instance places a template on a frame and keeps its clock; effect time
//! *t* runs 0 to 1 by the time mode, and each emitter acts only inside its window
//! of *t*. Read and followed: the window, every fade value, a burst's count, a
//! bolt's sprites along its line, a stream's emission into its ring, and bit 8's
//! draw over the scene. Where a sprite or a particle sits and how big it is are
//! not read: the drawing there is a stand-in, marked where it chooses.

use std::collections::VecDeque;
use std::rc::Rc;

use glam::Vec3;
use parkan_formats::fxid::{
    EMITTER_FLAG, EMITTER_LIGHT, EMITTER_SOUND, Effect, Emitter, FX_DELETE_AT_END, FX_PING_PONG,
    FX_TIMES_LINEAR, TIME_LOOP, TIME_ONCE, TIME_REVERSE,
};

/// Header flag 0x400: draw nothing while the tested point is hidden (`Effect.dll:0x10008016`).
pub const FX_HIDE_OCCLUDED: u32 = 0x400;

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

/// A sound to play: a type-2 emitter's `sounds.lib` member, where, and the distances
/// it is heard fully and last heard at (+64, +68).
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub sound: String,
    pub position: Vec3,
    pub near: f32,
    pub far: f32,
}

/// A quad to draw: a material, a centre, a length along a direction and a width.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub material: String,
    pub centre: Vec3,
    /// The quad's long side, as a vector; a square faces the camera when this is zero.
    pub along: Vec3,
    pub width: f32,
    /// The fade value the emitter hands the renderer; 0 is not drawn.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- whether the fade value scales alpha
    /// or colour is not read; it is the quad's alpha.
    pub alpha: f32,
    /// Drawn with the depth test off: an emitter with bit 8 whose effect's tested point
    /// is in view (`Effect.dll:0x10009930`, `Terrain.dll:0x100282c6`).
    pub overlay: bool,
}

/// A particle a stream left: when, from where, and how long it lives, in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    born: f32,
    frame: Frame,
    life: f32,
}

/// A type-8 emitter's ring of particles, and when it last emitted one.
#[derive(Clone, Debug, PartialEq)]
struct Stream {
    emitter: usize,
    last: Option<f32>,
    ring: VecDeque<Particle>,
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
    /// Where a bolt starts: where the effect was when it started.
    pub start_point: Vec3,
    seed: u32,
    /// *t* when the sounds were last looked at, −1 before.
    heard_t: f32,
    streams: Vec<Stream>,
    /// Seconds since the start, and where the effect was, at the last update.
    updated: Option<(f32, Vec3)>,
}

fn lerp3(lo: [f32; 3], hi: [f32; 3], s: f32) -> Vec3 {
    Vec3::from_array(lo).lerp(Vec3::from_array(hi), s)
}

/// A block's four bytes at `at` read as a `uint32`.
fn word(e: &Emitter, at: usize) -> u32 {
    e.body.get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

/// A fade value (`0x100013c2`, `0x10012322`, `0x10010881`): start + (end − start) × x^power.
pub fn fade(start: f32, end: f32, power: f32, x: f32) -> f32 {
    start + (end - start) * x.max(0.0).powf(power)
}

/// Where `t` is in `e`'s window, 0 to 1, or `None` outside it.
fn progress(e: &Emitter, t: f32) -> Option<f32> {
    let (lo, hi) = e.window()?;
    (lo..=hi).contains(&t).then(|| if hi > lo { (t - lo) / (hi - lo) } else { 1.0 })
}

/// A repeatable number in 0..1 from three keys.
///
/// STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the effect manager's random
/// generator is not read: an integer hash of the instance's seed, the particle and the axis.
fn noise(a: u32, b: u32, c: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    x as f32 / u32::MAX as f32
}

/// The most particles one burst draws: a guard against a malformed block, since the
/// shipped bursts are 3 to 60.
pub const BURST_CAP: u32 = 1024;

/// A stream's shortest interval, so an interval of 0 cannot emit without end.
const SHORTEST_INTERVAL: f32 = 1e-4;

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
        let streams = (effect.emitters.iter().enumerate())
            .filter(|(_, e)| e.kind == 8)
            .map(|(emitter, _)| Stream { emitter, last: None, ring: VecDeque::new() })
            .collect();
        Self {
            effect,
            frame,
            scale,
            start_ms: now_ms,
            end_ms,
            mode,
            value: 0.0,
            speed: 0.0,
            start_point: frame.origin,
            seed,
            heard_t: -1.0,
            streams,
            updated: None,
        }
    }

    /// Mode 1's progress, (now − start) ÷ (end − start); a duration of 0 is through at once.
    fn linear(&self, now_ms: f64) -> f32 {
        if self.end_ms <= self.start_ms {
            1.0
        } else {
            ((now_ms - self.start_ms) / (self.end_ms - self.start_ms)) as f32
        }
    }

    /// The emitters' clock: seconds since the instance started (`0x1000846c`).
    fn seconds(&self, now_ms: f64) -> f32 {
        ((now_ms - self.start_ms) / 1000.0) as f32
    }

    /// Effect time *t* (`Effect.dll:0x10005c60`).
    pub fn t(&self, now_ms: f64) -> f32 {
        let linear = self.linear(now_ms);
        let mut t = match self.mode {
            TIME_ONCE => linear,
            TIME_LOOP => linear - linear.floor(),
            TIME_REVERSE => 1.0 - linear,
            // STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the owner values
            // modes 5–15 read are not modelled one by one (speed per axis in 6–8, spin in
            // 9–12, one minus a point's or property's value in 13 and 14): every one reads
            // the owner's speed over its top speed, which the owner sets.
            5..=15 => self.speed,
            _ => self.value,
        };
        if self.effect.header.flags & FX_TIMES_LINEAR != 0 {
            t *= linear;
        }
        if self.effect.header.flags & FX_PING_PONG != 0 {
            t = if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) };
        }
        // STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- flag 1 moves t by up to
        // ± half of +0xc from the effect manager's generator, which is not read; the jitter
        // is left out (no Mission 01 effect carries the flag).
        t.clamp(0.0, 1.0)
    }

    /// Flag 2: an instance deletes itself once *t* ≥ 1 (`0x100062a6`).
    pub fn finished(&self, now_ms: f64) -> bool {
        self.effect.header.flags & FX_DELETE_AT_END != 0 && self.t(now_ms) >= 1.0
    }

    /// Where the instance tests its view from the camera, if it asks: an emitter with bit
    /// 8 or header flag 0x400 does (`0x10007984`), at header +0x24 carried into the world
    /// through its frame (`0x10007eb5`).
    pub fn test_point(&self) -> Option<Vec3> {
        let asks = self.effect.header.flags & FX_HIDE_OCCLUDED != 0
            || self.effect.emitters.iter().any(|e| e.word & EMITTER_FLAG != 0);
        asks.then(|| self.frame.point(Vec3::from_array(self.effect.header.point) * self.scale))
    }

    /// An update at `now_ms`, once the owner has placed the frame. A stream inside its
    /// window emits a particle each time its last emission plus lerp(+24, +28) by window
    /// progress has come round, spaced along the path the effect moved since the last
    /// update, into its ring of +36 (`0x10011a6c`, `0x10011230`); a particle lives +36
    /// intervals (`0x1001209e`).
    ///
    /// STAND-IN: docs/11-effects.md#bolts-streams-and-fades--read-and-measured -- when a
    /// stream's first particle leaves is not read: on its first update inside the window.
    pub fn update(&mut self, now_ms: f64) {
        let t = self.t(now_ms);
        let seconds = self.seconds(now_ms);
        let origin = self.frame.origin;
        let (then, from) = self.updated.unwrap_or((seconds, origin));
        for stream in &mut self.streams {
            let e = &self.effect.emitters[stream.emitter];
            let Some(p) = progress(e, t) else {
                stream.ring.retain(|q| seconds - q.born < q.life);
                stream.last = None;
                continue;
            };
            let ring = word(e, 36).max(1);
            let interval = (e.f(24) + (e.f(28) - e.f(24)) * p).max(SHORTEST_INTERVAL);
            let mut next = stream.last.map_or(seconds, |last| last + interval);
            let mut emitted = 0;
            while next <= seconds && emitted < ring {
                let share =
                    if seconds > then { ((next - then) / (seconds - then)).clamp(0.0, 1.0) } else { 1.0 };
                let frame = Frame { origin: from.lerp(origin, share), axes: self.frame.axes };
                stream.ring.push_back(Particle { born: next, frame, life: ring as f32 * interval });
                if stream.ring.len() > ring as usize {
                    stream.ring.pop_front();
                }
                stream.last = Some(next);
                next += interval;
                emitted += 1;
            }
            if next <= seconds {
                // A whole ring behind: the rest would be overwritten before they showed.
                stream.last = Some(seconds);
            }
            stream.ring.retain(|q| seconds - q.born < q.life);
        }
        self.updated = Some((seconds, origin));
    }

    /// The sounds whose trigger *t* has crossed since the last call: a type-2 emitter
    /// plays once as *t* passes its +8 (`Effect.dll:0x10012f42`).
    pub fn cues(&mut self, now_ms: f64) -> Vec<Cue> {
        let t = self.t(now_ms);
        let before = self.heard_t;
        self.heard_t = t;
        self.effect
            .emitters
            .iter()
            .filter(|e| e.kind == EMITTER_SOUND && !e.resource.member.is_empty())
            .filter(|e| before < e.f(8) && t >= e.f(8))
            .map(|e| Cue {
                sound: e.resource.member.clone(),
                position: self.frame.origin,
                near: e.f(64),
                far: e.f(68),
            })
            .collect()
    }

    /// What the instance draws at `now_ms`; `in_view` is whether its tested point
    /// (`test_point`) is in view from the camera. While it is hidden flag 0x400 draws
    /// nothing (`0x10008016`); while it is in view an emitter with bit 8 draws over the
    /// scene (`0x10009930`).
    pub fn sprites(&self, now_ms: f64, in_view: bool, out: &mut Vec<Sprite>) {
        if self.effect.header.flags & FX_HIDE_OCCLUDED != 0 && !in_view {
            return;
        }
        let t = self.t(now_ms);
        let seconds = self.seconds(now_ms);
        for (i, e) in self.effect.emitters.iter().enumerate() {
            // STAND-IN: docs/11-effects.md#not-resolved -- type 1 is a light in the owner's
            // light manager; how the shade lights with its range and attenuation is not
            // read, and lights are not drawn.
            if e.kind == EMITTER_LIGHT || e.kind == EMITTER_SOUND || e.resource.member.is_empty() {
                continue;
            }
            let Some(p) = progress(e, t) else { continue };
            let first = out.len();
            match e.kind {
                3 | 4 | 9 => self.sprite(e, p, out),
                5 => self.bolt(e, p, out),
                7 | 10 => self.burst(e, i as u32, p, out),
                8 => self.stream(i, e, seconds, out),
                _ => {}
            }
            let overlay = in_view && e.word & EMITTER_FLAG != 0;
            for s in &mut out[first..] {
                s.overlay = overlay;
            }
            // A fade value of 0 draws nothing (`Terrain.dll:0x1002887e`).
            let mut k = first;
            while k < out.len() {
                if out[k].alpha == 0.0 {
                    out.swap_remove(k);
                } else {
                    k += 1;
                }
            }
        }
    }

    /// A quad at `local` in `frame`, `size` along its axes, both times the instance's
    /// scale: stretched along the frame's first axis when the first two sizes differ,
    /// and otherwise a square facing the camera.
    fn quad(&self, e: &Emitter, frame: &Frame, local: Vec3, size: Vec3, alpha: f32) -> Sprite {
        let (x, y) = (frame.axes[0], frame.axes[1]);
        let stretched = (size.x - size.y).abs() > f32::EPSILON;
        Sprite {
            material: e.resource.member.clone(),
            centre: frame.point(local * self.scale),
            along: if stretched { x * size.x * self.scale } else { Vec3::ZERO },
            width: if stretched {
                size.y * self.scale * y.length()
            } else {
                size.x * self.scale * x.length()
            },
            alpha,
            overlay: false,
        }
    }

    /// A type 3, 4 or 9 sprite, fading +20 + (+24 − +20) × progress^+28 (`0x10010881`).
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- how the per-axis powers at +64 and +124
    /// shape a sprite's moving (+40 → +52) and growing (+100 → +112) triples is not read:
    /// it sits at lerp(+40, +52) in the frame and is lerp(+100, +112) in size along its
    /// axes, both straight by progress through the window; the phase is left to animated
    /// textures, which are not drawn.
    fn sprite(&self, e: &Emitter, p: f32, out: &mut Vec<Sprite>) {
        let local = lerp3(e.triple(40), e.triple(52), p);
        let size = lerp3(e.triple(100), e.triple(112), p);
        out.push(self.quad(e, &self.frame, local, size, fade(e.f(20), e.f(24), e.f(28), p)));
    }

    /// A type-5 bolt: floor(length / +36) sprites, at least 1 and at most +20, along the
    /// line from its start point to where the effect is now (`0x10002c53`), fading
    /// +4 → +8 across the window.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- who sets the manager's target point
    /// that flag 0x1000 hands a bolt for its start is not read, nor what its widths +24
    /// and +28 are: it starts where the effect started, each sprite is +24 wide, and the
    /// fade runs straight across the window.
    fn bolt(&self, e: &Emitter, p: f32, out: &mut Vec<Sprite>) {
        let line = self.frame.origin - self.start_point;
        let step = e.f(36);
        let n = if step > 0.0 { (line.length() / step) as u32 } else { 0 };
        let n = n.min(word(e, 20)).max(1);
        let alpha = e.f(4) + (e.f(8) - e.f(4)) * p;
        for k in 0..n {
            out.push(Sprite {
                material: e.resource.member.clone(),
                centre: self.start_point + line * ((k as f32 + 0.5) / n as f32),
                along: line / n as f32,
                width: e.f(24) * self.scale,
                alpha,
                overlay: false,
            });
        }
    }

    /// A type 7 or 10 burst of +0x24 × +0x28 particles (`0x10001720`), each fading
    /// +8 + (+12 − +8) × age^+16 (`0x100013c2`).
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- where a burst's particles go and how
    /// big they are is not read, nor when each spawns: each flies from the origin at a
    /// velocity between +44 and +56 per axis (a random share of each), spread by +68/2,
    /// its age its progress through the window over +28; its size lerp(+92, +104) by age.
    fn burst(&self, e: &Emitter, index: u32, p: f32, out: &mut Vec<Sprite>) {
        let count = word(e, 36).saturating_mul(word(e, 40)).min(BURST_CAP);
        let life = if e.f(28) > 0.0 { e.f(28) } else { 1.0 };
        let age = (p / life).clamp(0.0, 1.0);
        if p > life {
            return;
        }
        let alpha = fade(e.f(8), e.f(12), e.f(16), age);
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
                alpha,
                overlay: false,
            });
        }
    }

    /// A type-8 stream's particles, each fading +4 + (+8 − +4) × age^+12 (`0x10012322`),
    /// its age running 0 to 1 over its life.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- where a stream's particle goes and how
    /// big it is are not read (the exponent triples +124 and +172 shape them, a guess): it
    /// sits at lerp(+88, +100) in the frame it left from and is lerp(+136, +148) in size,
    /// both by its age; its age is in seconds since it left.
    fn stream(&self, index: usize, e: &Emitter, seconds: f32, out: &mut Vec<Sprite>) {
        let Some(stream) = self.streams.iter().find(|s| s.emitter == index) else { return };
        for q in &stream.ring {
            let age = ((seconds - q.born) / q.life.max(f32::EPSILON)).clamp(0.0, 1.0);
            let local = lerp3(e.triple(88), e.triple(100), age);
            let size = lerp3(e.triple(136), e.triple(148), age);
            out.push(self.quad(e, &q.frame, local, size, fade(e.f(4), e.f(8), e.f(12), age)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::fxid::{Header, TIME_MANUAL, TIME_POINT, TIME_SPEED};
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

    /// `e` with the `uint32` `v` at `at`.
    fn with_word(mut e: Emitter, at: usize, v: u32) -> Emitter {
        e.body[at..at + 4].copy_from_slice(&v.to_le_bytes());
        e
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
                (20, 1.0),
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
        fx.sprites(0.0, true, &mut out);
        assert!(out.is_empty(), "t 0 is before the window");
        fx.value = 0.5;
        fx.sprites(10.0, true, &mut out);
        let s = &out[0];
        // At the window's end: centre 10 × 0.1 along the barrel, 20 × 0.1 long, 4 × 0.1 wide.
        assert!((s.centre - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5, "{:?}", s.centre);
        assert!((s.along - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5 && (s.width - 0.4).abs() < 1e-5);
        assert!(!s.overlay, "no bit 8");
        out.clear();
        fx.value = 0.7;
        fx.sprites(20.0, true, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn a_sprite_fades_from_its_start_to_its_end_by_progress_to_its_power_and_zero_draws_nothing() {
        let glow = block(3, 200, &[(32, 0.0), (36, 1.0), (20, 0.8), (24, 0.0), (28, 2.0), (100, 1.0)], "G");
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![glow]), frame, 1.0, 0.0, None, 1);
        let mut out = Vec::new();
        for (t, alpha) in [(0.0, 0.8), (0.5, 0.6), (1.0 - 1e-6, 0.0)] {
            fx.value = t;
            out.clear();
            fx.sprites(0.0, true, &mut out);
            assert!((out[0].alpha - alpha).abs() < 1e-5, "t {t}: {}", out[0].alpha);
        }
        fx.value = 1.0;
        out.clear();
        fx.sprites(0.0, true, &mut out);
        assert!(out.is_empty(), "the fade reaches 0 at the window's end and draws nothing");
    }

    #[test]
    fn an_impact_runs_once_and_deletes_itself_at_the_end() {
        let burst = block(
            7,
            208,
            &[
                (20, 0.0),
                (24, 0.6),
                (8, 1.0),
                (12, 0.0),
                (16, 1.0),
                (28, 0.5),
                (56, 3.0),
                (92, 0.1),
                (104, 0.3),
            ],
            "splash6",
        );
        let burst = with_word(with_word(burst, 36, 2), 40, 2);
        let fx = Instance::new(
            effect(TIME_ONCE, 1.5, FX_DELETE_AT_END, vec![burst]),
            Frame::along(Vec3::ZERO, Vec3::Z, 1.0),
            10.0,
            0.0,
            None,
            7,
        );
        let mut out = Vec::new();
        fx.sprites(150.0, false, &mut out);
        assert_eq!(out.len(), 4, "two by two particles early in their life");
        assert!(out.iter().all(|s| s.alpha < 1.0 && s.alpha > 0.0));
        assert!(!fx.finished(1000.0) && fx.finished(1500.0));
        out.clear();
        fx.sprites(1000.0, false, &mut out);
        assert!(out.is_empty(), "past the burst's share of the window");
    }

    #[test]
    fn a_sound_plays_once_each_time_its_trigger_is_crossed() {
        let shot = block(2, 148, &[(8, 0.01), (12, 1.0), (64, 10.0), (68, 80.0)], "h_fire_cannon.wav");
        let frame = Frame::along(Vec3::ONE, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_POINT, 0.0, 0, vec![shot]), frame, 1.0, 0.0, None, 1);
        assert!(fx.cues(0.0).is_empty(), "t 0 is short of 0.01");
        fx.value = 0.5;
        let cues = fx.cues(10.0);
        assert_eq!(cues.len(), 1);
        assert_eq!((cues[0].position, cues[0].near, cues[0].far), (Vec3::ONE, 10.0, 80.0));
        fx.value = 1.0;
        assert!(fx.cues(20.0).is_empty(), "already past");
        fx.value = 0.0;
        assert!(fx.cues(30.0).is_empty());
        fx.value = 0.5;
        assert_eq!(fx.cues(40.0).len(), 1, "the next stroke");
    }

    #[test]
    fn a_bursts_count_is_its_two_words_multiplied_and_its_fade_is_shaped_by_its_power() {
        // +16 was once read as the count: here it is the fade's power, 2.
        let burst = block(7, 208, &[(20, 0.0), (24, 1.0), (8, 1.0), (12, 0.2), (16, 2.0), (28, 1.0)], "fire");
        let burst = with_word(with_word(burst, 36, 3), 40, 5);
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![burst]), frame, 1.0, 0.0, None, 3);
        fx.value = 0.5;
        let mut out = Vec::new();
        fx.sprites(0.0, false, &mut out);
        assert_eq!(out.len(), 15);
        // 1 + (0.2 − 1) × 0.5²
        assert!(out.iter().all(|s| (s.alpha - 0.8).abs() < 1e-5), "{}", out[0].alpha);
    }

    /// `hero_laser_bullet`'s red bolt: 20 sprites at most, one per 50, 0.4 wide, fading 1 → 0.
    fn bolt() -> Emitter {
        let e =
            block(5, 112, &[(12, 0.0), (16, 1.0), (4, 1.0), (8, 0.0), (24, 0.4), (28, 0.4), (36, 50.0)], "L");
        with_word(e, 20, 20)
    }

    #[test]
    fn a_bolt_lays_a_sprite_for_every_segment_of_its_line_from_its_start_to_where_it_is() {
        let frame = Frame::along(Vec3::new(10.0, 0.0, 0.0), Vec3::Y, 1.0);
        let mut fx =
            Instance::new(effect(TIME_MANUAL, 0.75, 0x1010, vec![bolt()]), frame, 10.0, 0.0, None, 1);
        let mut out = Vec::new();
        let draw = |fx: &Instance, out: &mut Vec<Sprite>| {
            out.clear();
            fx.sprites(0.0, true, out);
        };
        draw(&fx, &mut out);
        assert_eq!(out.len(), 1, "a line of 0 still lays one sprite");
        fx.frame.origin = Vec3::new(10.0, 175.0, 0.0);
        draw(&fx, &mut out);
        assert_eq!(out.len(), 3, "floor(175 / 50)");
        let piece = 175.0 / 3.0;
        assert!((out[0].centre - Vec3::new(10.0, piece / 2.0, 0.0)).length() < 1e-3);
        assert!((out[2].along - Vec3::new(0.0, piece, 0.0)).length() < 1e-3);
        assert!(out.iter().all(|s| s.width == 0.4 && s.alpha == 1.0 && !s.overlay));
        fx.frame.origin = Vec3::new(10.0, 5000.0, 0.0);
        draw(&fx, &mut out);
        assert_eq!(out.len(), 20, "at most +20");
        fx.value = 0.25;
        draw(&fx, &mut out);
        assert!(out.iter().all(|s| (s.alpha - 0.75).abs() < 1e-6), "+4 → +8 across the window");
    }

    /// A stream over t 0–0.9, every 0.1 s at the window's start and 0.2 s at its end, a
    /// ring of 3, fading 1 → 0, sitting 1 to 3 along the frame's x and sized 1.
    fn stream() -> Emitter {
        let floats = [
            (16, 0.0),
            (20, 0.9),
            (24, 0.1),
            (28, 0.2),
            (4, 1.0),
            (8, 0.0),
            (12, 1.0),
            (88, 1.0),
            (100, 3.0),
            (136, 1.0),
            (140, 1.0),
            (148, 1.0),
            (152, 1.0),
        ];
        with_word(block(8, 248, &floats, "smoke"), 36, 3)
    }

    #[test]
    fn a_stream_emits_by_its_interval_into_its_ring_along_the_path_it_moved() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let start = |t: f32| {
            let mut fx =
                Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![stream()]), frame, 10.0, 0.0, None, 1);
            fx.value = t;
            fx
        };
        let mut fx = start(0.0);
        let mut out = Vec::new();
        fx.update(0.0);
        fx.sprites(0.0, false, &mut out);
        assert_eq!(out.len(), 1, "the first particle leaves at once");
        assert!((out[0].centre - Vec3::X).length() < 1e-5 && out[0].alpha == 1.0);
        // 0.25 s on, moved 25 along y: particles left at 0.1 and 0.2, spaced along the path.
        fx.frame.origin = Vec3::new(0.0, 25.0, 0.0);
        fx.update(250.0);
        out.clear();
        fx.sprites(250.0, false, &mut out);
        assert_eq!(out.len(), 3);
        let mut ys: Vec<f32> = out.iter().map(|s| s.centre.y).collect();
        ys.sort_by(f32::total_cmp);
        assert!((ys[1] - 10.0).abs() < 1e-3 && (ys[2] - 20.0).abs() < 1e-3, "{ys:?}");
        // The first lives 3 × 0.1 s: at 0.25 s it is 5/6 through, 1 + 2 × 5/6 along x.
        let oldest = out.iter().find(|s| s.centre.y.abs() < 1e-3).unwrap();
        assert!((oldest.centre.x - (1.0 + 2.0 * 5.0 / 6.0)).abs() < 1e-3, "{}", oldest.centre.x);
        assert!((oldest.alpha - 1.0 / 6.0).abs() < 1e-3);
        // Outside its window a stream draws nothing.
        fx.value = 1.0;
        out.clear();
        fx.sprites(250.0, false, &mut out);
        assert!(out.is_empty());

        // Half through the window the interval is 0.15 s.
        let mut fx = start(0.45);
        let born = |fx: &Instance| fx.streams[0].ring.iter().map(|q| q.born).collect::<Vec<_>>();
        fx.update(0.0);
        fx.update(140.0);
        assert_eq!(born(&fx), [0.0]);
        fx.update(160.0);
        assert_eq!(born(&fx).len(), 2);
        assert!((born(&fx)[1] - 0.15).abs() < 1e-5);
        // Long after, the ring has gone by: nothing stale is left to show.
        fx.update(5000.0);
        assert!(born(&fx).is_empty(), "{:?}", born(&fx));
    }

    #[test]
    fn bit_8_draws_over_the_scene_while_the_effects_tested_point_is_in_view() {
        let mut glow = block(3, 200, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (100, 1.0)], "G");
        glow.word |= EMITTER_FLAG;
        let plain = block(3, 200, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (100, 1.0)], "P");
        let mut template = effect(TIME_MANUAL, 0.0, 0, vec![glow, plain]);
        Rc::get_mut(&mut template).unwrap().header.point = [1.0, 0.0, 0.0];
        let frame = Frame::along(Vec3::new(5.0, 0.0, 0.0), Vec3::Z, 1.0);
        let fx = Instance::new(template.clone(), frame, 20.0, 0.0, None, 1);
        // Header +0x24 through the frame, at the scale: 20 × 0.1 up.
        assert!((fx.test_point().unwrap() - Vec3::new(5.0, 0.0, 2.0)).length() < 1e-5);
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert_eq!(
            out.iter().map(|s| (s.material.as_str(), s.overlay)).collect::<Vec<_>>(),
            [("G", true), ("P", false)]
        );
        out.clear();
        fx.sprites(0.0, false, &mut out);
        assert!(out.iter().all(|s| !s.overlay) && out.len() == 2, "hidden, depth-tested like the rest");

        let plain_only = effect(TIME_MANUAL, 0.0, 0, vec![block(3, 200, &[(36, 1.0), (20, 1.0)], "P")]);
        assert_eq!(Instance::new(plain_only, frame, 1.0, 0.0, None, 1).test_point(), None, "nothing asks");
    }

    #[test]
    fn flag_0x400_draws_nothing_while_the_tested_point_is_hidden() {
        let glow = block(3, 200, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (100, 1.0)], "B");
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let fx =
            Instance::new(effect(TIME_MANUAL, 0.0, FX_HIDE_OCCLUDED, vec![glow]), frame, 1.0, 0.0, None, 1);
        assert!(fx.test_point().is_some());
        let mut out = Vec::new();
        fx.sprites(0.0, false, &mut out);
        assert!(out.is_empty());
        fx.sprites(0.0, true, &mut out);
        assert_eq!(out.len(), 1);
        assert!(!out[0].overlay, "no bit 8 on the emitter");
    }

    #[test]
    fn time_mode_5_is_the_owners_speed_over_its_top_speed() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_SPEED, 0.5, 0, Vec::new()), frame, 1.0, 0.0, None, 0);
        assert_eq!(fx.t(100.0), 0.0, "an owner that has not moved");
        fx.speed = 0.4;
        assert_eq!(fx.t(100.0), 0.4);
        fx.speed = 1.2;
        assert_eq!(fx.t(100.0), 1.0);
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
