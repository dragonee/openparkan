//! Effects as they run: `docs/11-effects.md`, "How an effect runs" and "Bolts,
//! streams and fades", and `engine/design/r5-effects.md`.
//!
//! An instance places a template on a frame and keeps its clock; effect time
//! *t* runs 0 to 1 by the time mode, and each emitter acts only inside its window
//! of *t*. Read and followed: the window, every fade value, a burst's count, a
//! bolt's sprites along its line and their width, a stream's emission into its ring,
//! bit 8's draw over the scene, and the (low, high, jitter, exponent) channels that
//! place and size a sprite or a particle. When each of a burst's particles spawns is
//! not read: the drawing there is a stand-in, marked where it chooses.

use std::collections::VecDeque;
use std::rc::Rc;

use glam::Vec3;
use parkan_formats::fxid::{
    EMITTER_FLAG, EMITTER_LIGHT, EMITTER_SOUND, Effect, Emitter, FX_DELETE_AT_END, FX_JITTER, FX_PING_PONG,
    FX_START_OFF, FX_TIMES_LINEAR, TIME_LIFE_INVERSE, TIME_LOOP, TIME_MOTION, TIME_ONCE, TIME_POINT_INVERSE,
    TIME_REVERSE, TIME_SPEED, TIME_SPIN,
};

/// Header flag 0x400: draw nothing while the tested point is hidden (`Effect.dll:0x10008016`).
pub const FX_HIDE_OCCLUDED: u32 = 0x400;
/// Header flag 0x1000: hand the emitters the manager's target point every manager tick
/// (`Effect.dll:0x10006349`), which a bolt takes for its start (`0x10003070`).
pub const FX_TARGET_POINT: u32 = 0x1000;

/// Where an effect sits: an origin and three axes whose lengths carry a size, as
/// a control point's vector does (`docs/07-objects.md`, "CTPT").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub origin: Vec3,
    pub axes: [Vec3; 3],
    /// Built from three control points, so the axes are those points' own direction vectors
    /// and the frame is the matrix the draw turns a sprite through ([`Frame::basis`]).
    pub points: bool,
}

impl Frame {
    /// A frame at `origin` whose first axis is `x`, the others square to it, all `size` long.
    pub fn along(origin: Vec3, x: Vec3, size: f32) -> Self {
        let x = x.normalize_or(Vec3::X);
        let helper = if x.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
        let y = helper.cross(x).normalize();
        let z = x.cross(y);
        Self { origin, axes: [x * size, y * size, z * size], points: false }
    }

    /// The frame three control points give (action 4, `Control.dll:0x10002d8d`): their
    /// centroid, and their vectors as the axes. The handler builds the identity and writes
    /// each direction into a row of it, the centroid into the fourth, and hands that matrix
    /// to the effect -- so the frame *is* a matrix, and the axes' own lengths are in it.
    pub fn from_points(points: [(Vec3, Vec3); 3]) -> Self {
        let origin = (points[0].0 + points[1].0 + points[2].0) / 3.0;
        Self { origin, axes: [points[0].1, points[1].1, points[2].1], points: true }
    }

    /// The frame as a basis a sprite is drawn through, where it is one: three control-point
    /// directions that are not all in a plane.
    ///
    /// Where they are -- which is every frame the handler's other three branches build, and
    /// every one of the 690 load-group records that names the same point three times -- the
    /// matrix cannot be inverted and the draw's camera-facing basis stands alone
    /// ([`Frame::from_points`] keeps the repeated direction in all three axes there).
    /// *Measured*: 189 records name three distinct points and exactly one of those, the
    /// hero chassis's `aim_fire_S`, has its directions in a plane.
    pub fn basis(&self) -> Option<[Vec3; 3]> {
        let [x, y, z] = self.axes;
        let scale = x.length() * y.length() * z.length();
        (self.points && x.cross(y).dot(z).abs() > scale * 1e-3).then_some(self.axes)
    }

    pub fn point(&self, local: Vec3) -> Vec3 {
        self.origin + self.axes[0] * local.x + self.axes[1] * local.y + self.axes[2] * local.z
    }

    /// The same frame with its axes a unit long: it turns what it places without sizing it.
    /// It is no basis to draw through either — a stream's particle is sized by its own
    /// channel and by nothing else ([`Instance::stream`]).
    pub fn turned(&self) -> Self {
        let fallback = [Vec3::X, Vec3::Y, Vec3::Z];
        Self {
            origin: self.origin,
            axes: std::array::from_fn(|i| self.axes[i].normalize_or(fallback[i])),
            points: false,
        }
    }
}

/// A sound block's +4: 2 or 3 make it a loop (`Effect.dll:0x10012d3e`).
pub const SOUND_MODE_AT: usize = 4;
pub const SOUND_LOOPS: [u32; 2] = [2, 3];

/// What a sound emitter asks of the sound server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CueKind {
    /// Play once, stopping and starting again a copy still playing.
    Once,
    /// Play, and play again each time it ends, until stopped.
    Loop,
    /// Stop the loop with this cue's key.
    Stop,
    /// Where the sound with this cue's key is now, while it plays (`0x1001300b`): the
    /// server is handed its position, near and far each update. Its `sound` is empty.
    Move,
}

/// A sound to play: a type-2 emitter's `sounds.lib` member, where, and the distances
/// it is heard fully and last heard at (+64, +68). `key` names the instance and the
/// emitter, so a loop can be stopped.
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    pub sound: String,
    pub position: Vec3,
    pub near: f32,
    pub far: f32,
    pub key: (u64, usize),
    pub kind: CueKind,
}

/// A quad to draw: a material, a centre, a length along a direction and a width.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub material: String,
    pub centre: Vec3,
    /// The quad's long side, as a vector; a camera-facing rectangle when this is zero.
    pub along: Vec3,
    /// Across the quad: its cross-section where `along` is set, and across the camera where
    /// it faces one.
    pub width: f32,
    /// Up the camera, for a quad that faces one; `along` gives the long side of one that does
    /// not, and this is its width over again.
    pub height: f32,
    /// The fade value the emitter hands the renderer; 0 is not drawn. It stands in for the
    /// material's ambient alpha, so it scales the texture's alpha (docs/11, "Bolts, streams
    /// and fades").
    pub alpha: f32,
    /// Drawn with the depth test off: an emitter with bit 8 whose effect's tested point
    /// is in view (`Effect.dll:0x10009930`, `Terrain.dll:0x100282c6`), or a beacon's.
    pub overlay: bool,
    /// The texture's u runs along the quad's long side and v across it, as a bolt's sprites
    /// take theirs (`Effect.dll:0x10009b90`); otherwise u runs across.
    pub lengthwise: bool,
    /// How long this sprite's own material has been running, ms. A stream's particle counts
    /// from when it left, and everything else from the instance's start.
    pub age_ms: f32,
    /// Where its material's animation stands, 0 to 1 of the track's whole length: the draw
    /// hands this to the material manager's `GetMaterialPhase` (vtable slot 5,
    /// `World3D.dll:0x10003680`), which multiplies it by the last key's time and takes the
    /// two keys that bracket it. A fraction outside 0..1 becomes 0.5 there. See docs/11,
    /// "A phase is where its material's animation stands".
    pub phase: f32,
    /// Drawn as a hemisphere about `centre` instead of a quad: a type-9 emitter's shape.
    pub dome: Option<Dome>,
    /// The frame to turn the quad through, each axis as long as the sprite is that way:
    /// a camera-facing unit square in this basis rather than on the screen, so a frame
    /// whose axes differ draws a long streak along its first when seen across it and a small
    /// one end-on ([`Frame::basis`], docs/11, "A sprite is drawn through its frame").
    pub frame: Option<[Vec3; 3]>,
}

/// A type-9 emitter's hemisphere (`Terrain.dll:0x100273b0`): a unit dome with its pole on the
/// mesh's own z, carried by `axes` (each as long as the emitter's size along it), cut into
/// `segments` around and `rings` from the rim to the pole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dome {
    pub axes: [Vec3; 3],
    pub segments: u8,
    pub rings: u8,
}

/// A type-9 block's shape (`+200`), and the detail each gives its dome at the finest level
/// (tables `Terrain.dll:0x1009a750` and `0x1009a780`): around, and from rim to pole.
pub const DOME_SHAPE_AT: usize = 200;
pub const DOME_DETAIL: [(u8, u8); 3] = [(8, 3), (16, 6), (24, 9)];
/// Where an emitter is placed (`+4`): 2 in the effect's own frame.
pub const PLACEMENT_AT: usize = 4;

/// A particle a stream left: when, from where, and how long it lives, in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    born: f32,
    frame: Frame,
    life: f32,
    /// The number drawn for it as it left (`Effect.dll:0x10011d84`), which its material's
    /// animation stands at for its whole life when the block's +32 is negative.
    drawn: f32,
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
    /// The value mode 0 reads, and modes 4, 16 and 17 the owner's point value. Mode 13
    /// takes one minus it.
    pub value: f32,
    /// The owner's speed over its top speed, which modes 5–8 read and mode 15 takes the
    /// larger of (properties `0x21` and `0x11`, `Effect.dll:0x10005d56`).
    pub speed: f32,
    /// Its spin over its top spin, the same way, for modes 9–12 and 15 (`0x10005e2e`).
    ///
    /// STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- nothing sets it: no shipped
    /// effect is in modes 9–12, and the 31 in mode 15 are dust and engine plumes whose
    /// speed term is the one that moves.
    pub spin: f32,
    /// The owner's life over its life at load, property `0x31`, which mode 14 takes one
    /// minus (`0x10005f2d`). 1 is undamaged, and a mode-14 effect then holds *t* at 0.
    ///
    /// STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- nothing sets it: the eight
    /// mode-14 effects are the burning trees and wrecks, which nothing here starts.
    pub life: f32,
    /// Where a bolt starts: where the effect was when it started, or the manager's target
    /// point, which its owner hands it every tick when the header asks ([`FX_TARGET_POINT`]).
    pub start_point: Vec3,
    seed: u32,
    /// The state header flag 1's jitter is drawn from, one per instance rather than the
    /// module-wide one the game shares.
    rng: Rng,
    /// What flag 1 last moved *t* by: the game redraws it every time it works *t* out,
    /// which is at most once per manager tick, so it is drawn on the update instead.
    jitter: f32,
    /// The caller's number for the instance, which its sounds' keys carry.
    pub id: u64,
    /// Switched on (actions 18 and 19); header flag 0x40 starts it off. An instance
    /// switched off neither updates, draws nor sounds (docs/11, "Starting, ending, switching").
    pub on: bool,
    /// Plays none of its sounds, while it updates and draws as ever.
    pub silent: bool,
    /// The sound emitters' previous time (`+0x9c`), 0 at load, and the loops playing.
    heard_t: f32,
    looping: Vec<usize>,
    /// The one-shots started, which may still be playing.
    sounding: Vec<usize>,
    streams: Vec<Stream>,
    /// Seconds since the start, and where the effect was, at the last update.
    updated: Option<(f32, Vec3)>,
}

fn lerp3(lo: [f32; 3], hi: [f32; 3], s: f32) -> Vec3 {
    Vec3::from_array(lo).lerp(Vec3::from_array(hi), s)
}

/// A lerp whose parameter is given per axis, which is how a particle's channels move
/// (`Effect.dll:0x1000d390`, `0x1000d450`).
fn lerp3_axis(lo: [f32; 3], hi: [f32; 3], s: Vec3) -> Vec3 {
    let (lo, hi) = (Vec3::from_array(lo), Vec3::from_array(hi));
    lo + (hi - lo) * s
}

/// `x` to the power of each axis's exponent (`Effect.dll:0x10011170`), which shapes a
/// channel's lerp per axis. An exponent of exactly 1.0 is taken straight, which is what
/// 6818 of the 7142 channels across the shipped drawing blocks carry on all three axes.
fn shaped(x: f32, powers: [f32; 3]) -> Vec3 {
    let curve = |p: f32| if p == 1.0 { x } else { x.max(0.0).powf(p) };
    Vec3::new(curve(powers[0]), curve(powers[1]), curve(powers[2]))
}

/// A block's four bytes at `at` read as a `uint32`.
fn word(e: &Emitter, at: usize) -> u32 {
    e.body.get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

/// A fade value (`0x100013c2`, `0x10012322`, `0x10010881`): start + (end − start) × x^power.
pub fn fade(start: f32, end: f32, power: f32, x: f32) -> f32 {
    start + (end - start) * x.max(0.0).powf(power)
}

/// The **phase** a sprite (types 3, 4 and 9) or a bolt runs, and the fraction of its
/// material's animation the draw takes from it.
///
/// The phase is `base + (end − base) × x^power` with `base` the block's start held at 0
/// when it is negative, and *x* the progress through the window — or the **seconds since
/// the instance started** when that start is negative, which is how a phase outruns a
/// window (`Effect.dll:0x100104d4`, `0x100105f0`; the bolt's at `0x100029f0`,
/// `0x10002a5f`, whose power is fixed at 1). The draw hands on its **fractional part**
/// (`0x10010817`, `0x10002f96`), so a phase of 9 plays the animation nine times over.
pub fn phase_fraction(start: f32, end: f32, power: f32, x: f32, seconds: f32) -> f32 {
    let base = start.max(0.0);
    let x = if start < 0.0 { seconds } else { x };
    let shaped = if power == 1.0 { x } else { x.max(0.0).powf(power) };
    let v = base + (end - base) * shaped;
    v - v.floor()
}

/// The same for a burst's particle (types 7 and 10, `Effect.dll:0x10001625`): its age over
/// the block's `+32` when that is 1 or less, and the fractional part of age × `+32` when it
/// is more. A value at 1 or over is held at **0.99**, and a negative `+32` — on 350 of the
/// 1161 type-7 blocks — sends it negative, which the draw clamps to 0, so those particles
/// never leave their material's first key.
pub fn burst_fraction(age: f32, rate: f32) -> f32 {
    let v = if rate <= 1.0 {
        if rate == 0.0 { 0.0 } else { age / rate }
    } else {
        let scaled = age * rate;
        scaled - scaled.floor()
    };
    if v >= 1.0 { 0.99 } else { v.clamp(0.0, 1.0) }
}

/// Where `t` is in `e`'s window, 0 to 1, or `None` outside it.
fn progress(e: &Emitter, t: f32) -> Option<f32> {
    let (lo, hi) = e.window()?;
    (lo..=hi).contains(&t).then(|| if hi > lo { (t - lo) / (hi - lo) } else { 1.0 })
}

/// The generator every random draw in `Effect.dll` comes from
/// (`0x10002220`, and inlined at `0x10001ad7`, `0x10002dfc`, `0x10007f9b`, `0x1000bf8d`,
/// `0x1000ec50`, `0x1000fa8e`): a 32-bit state read as two 16-bit halves, stepped
///
/// ```text
/// lo = (lo << 1) ^ hi;   hi = (hi >> 1) ^ lo;   draw = hi
/// ```
///
/// It is linear over GF(2), so its cycle is exact: one transient step, then a period of
/// 1_065_353_089 = 127 x 47 x 178_481, and a state of 0 is a fixed point that only ever
/// draws 0. The module seeds thirteen copies of it from `ngiGetClocks` as it loads, one
/// per translation unit (`0x10002660` and its twelve twins), and every manager, template
/// and instance in the process shares them — so a draw cannot be reproduced from an
/// instance's own state. See `docs/11-effects.md`, "The generator".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rng {
    lo: u16,
    hi: u16,
}

impl Rng {
    /// A state from a 32-bit seed, never the zero state the generator cannot leave.
    pub const fn new(seed: u32) -> Rng {
        let seed = if seed == 0 { 1 } else { seed };
        Rng { lo: seed as u16, hi: (seed >> 16) as u16 }
    }

    /// The next draw, 0 to 65535 (`Effect.dll:0x10002220`).
    pub fn next16(&mut self) -> u16 {
        self.lo = (self.lo << 1) ^ self.hi;
        self.hi = (self.hi >> 1) ^ self.lo;
        self.hi
    }

    /// A uniform in 0..1: the draw over 65536, as every caller scales it.
    pub fn unit(&mut self) -> f32 {
        f32::from(self.next16()) / 65536.0
    }

    /// A uniform in +-half of `v`: `draw x v / 65536 - v / 2` (`Effect.dll:0x10002680`).
    pub fn spread(&mut self, v: f32) -> f32 {
        self.unit() * v - v * 0.5
    }
}

/// A seed for one emitter's spawn draws: the game takes them from a state shared by the
/// whole module, which nothing here can follow, so each burst gets its own stream.
fn stream_seed(seed: u32, index: u32) -> u32 {
    seed.wrapping_mul(0x9E37_79B9) ^ index.wrapping_mul(0x85EB_CA6B) | 1
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
        let on = effect.header.flags & FX_START_OFF == 0;
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
            spin: 0.0,
            life: 1.0,
            start_point: frame.origin,
            seed,
            rng: Rng::new(seed),
            jitter: 0.0,
            id: 0,
            on,
            silent: false,
            heard_t: 0.0,
            looping: Vec::new(),
            sounding: Vec::new(),
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
            // Modes 6–8 are the speed on one axis and 10–12 the spin on one; both are
            // taken whole here, and no shipped effect is in any of the six.
            TIME_SPEED..=8 => self.speed,
            TIME_SPIN..=12 => self.spin,
            TIME_POINT_INVERSE => 1.0 - self.value,
            TIME_LIFE_INVERSE => 1.0 - self.life,
            TIME_MOTION => self.speed.max(self.spin),
            _ => self.value,
        };
        if self.effect.header.flags & FX_TIMES_LINEAR != 0 {
            t *= linear;
        }
        if self.effect.header.flags & FX_PING_PONG != 0 {
            t = if t < 0.5 { 2.0 * t } else { 2.0 * (1.0 - t) };
        }
        // Flag 1 moves t by a uniform in ± half of the header's +0xc and then clamps
        // (`Effect.dll:0x1000830e`).
        (t + self.jitter).clamp(0.0, 1.0)
    }

    /// Whether the header asks for the manager's target point every tick (flag 0x1000).
    pub fn takes_target_point(&self) -> bool {
        self.effect.header.flags & FX_TARGET_POINT != 0
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
        if !self.on {
            return;
        }
        // Header flag 1's jitter on *t*, redrawn each tick (`Effect.dll:0x1000830e`): 58
        // of the 923 effects carry it, and its spread is 0.05 to 0.4 on 56 of them.
        if self.effect.header.flags & FX_JITTER != 0 {
            let spread = self.effect.header.jitter;
            self.jitter = self.rng.spread(spread);
        }
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
                let frame = Frame { origin: from.lerp(origin, share), ..self.frame };
                let drawn = self.rng.unit();
                stream.ring.push_back(Particle { born: next, frame, life: ring as f32 * interval, drawn });
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

    /// What the sound emitters ask for at `now_ms` (`Effect.dll:0x10012eb0`). A one-shot
    /// takes a *t* of exactly 1 as 0 (`0x10012f2d`) and plays when the previous time is
    /// at or below both that *t* and its trigger +8, and the trigger is below *t*; the
    /// taken *t* becomes the previous time. A loop plays while low +8 ≤ *t* ≤ high +12 and
    /// stops outside (`0x10012fca`, `0x10013008`). Every sound started and not stopped is
    /// then told where it is now (`0x1001300b`).
    pub fn cues(&mut self, now_ms: f64) -> Vec<Cue> {
        if !self.on || self.silent {
            return Vec::new();
        }
        let t = self.t(now_ms);
        let taken = if t == 1.0 { 0.0 } else { t };
        let before = self.heard_t;
        self.heard_t = taken;
        let mut out = Vec::new();
        for (i, e) in self.effect.emitters.iter().enumerate() {
            if e.kind != EMITTER_SOUND || e.resource.member.is_empty() {
                continue;
            }
            let cue = |kind| Cue {
                sound: e.resource.member.clone(),
                position: self.frame.origin,
                near: e.f(64),
                far: e.f(68),
                key: (self.id, i),
                kind,
            };
            if SOUND_LOOPS.contains(&word(e, SOUND_MODE_AT)) {
                let playing = self.looping.contains(&i);
                // A looping mode's *t* comes round to 0 at the end of every period, below a
                // window that starts at 0.001, so a loop over the whole of *t* falls outside
                // it for one update each period and is stopped and started again. The game's
                // manager updates on wall time and lands in that millisecond about once in a
                // thousand updates; this one steps an exact 1/60 s from 0 and lands there at
                // every wrap, which restarted Mission 03's four building ambiences once a
                // second. The wrap itself is not taken as leaving the window.
                let wrapped = self.mode == TIME_LOOP && playing && t < e.f(8) && e.f(12) >= 1.0;
                let inside = (e.f(8) <= t && t <= e.f(12)) || wrapped;
                if inside && !playing {
                    self.looping.push(i);
                    out.push(cue(CueKind::Loop));
                } else if !inside && playing {
                    self.looping.retain(|&l| l != i);
                    out.push(cue(CueKind::Stop));
                }
            } else if before <= taken && before <= e.f(8) && e.f(8) < taken {
                if !self.sounding.contains(&i) {
                    self.sounding.push(i);
                }
                out.push(cue(CueKind::Once));
            }
        }
        for &i in self.looping.iter().chain(&self.sounding) {
            let e = &self.effect.emitters[i];
            out.push(Cue {
                sound: String::new(),
                position: self.frame.origin,
                near: e.f(64),
                far: e.f(68),
                key: (self.id, i),
                kind: CueKind::Move,
            });
        }
        out
    }

    /// Stops for the loops still playing, as the instance goes (slot 5, `0x10013170`).
    pub fn silence(&mut self) -> Vec<Cue> {
        let effect = self.effect.clone();
        std::mem::take(&mut self.looping)
            .into_iter()
            .map(|i| Cue {
                sound: effect.emitters[i].resource.member.clone(),
                position: self.frame.origin,
                near: 0.0,
                far: 0.0,
                key: (self.id, i),
                kind: CueKind::Stop,
            })
            .collect()
    }

    /// What the instance draws at `now_ms`; `in_view` is whether its tested point
    /// (`test_point`) is in view from the camera. While it is hidden flag 0x400 draws
    /// nothing (`0x10008016`); while it is in view an emitter with bit 8 draws over the
    /// scene (`0x10009930`), and nothing else does.
    pub fn sprites(&self, now_ms: f64, in_view: bool, out: &mut Vec<Sprite>) {
        if !self.on || (self.effect.header.flags & FX_HIDE_OCCLUDED != 0 && !in_view) {
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
            let age_ms = seconds * 1000.0;
            match e.kind {
                3 | 4 | 9 => self.sprite(e, p, age_ms, seconds, out),
                5 => self.bolt(e, p, age_ms, seconds, out),
                7 | 10 => self.burst(e, i as u32, p, age_ms, out),
                8 => self.stream(i, e, seconds, out),
                _ => {}
            }
            // A beacon's own emitters carry no bit 8, so its glow is depth-tested like any
            // other sprite: the pass argument its flag 0x800 waits for decides whether the
            // effect draws at all, never its depth state
            // ([11](../../../docs/11-effects.md#a-beacon-lights-glow--read-and-measured)).
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
    /// scale: stretched along the frame's first axis when the first two sizes differ, and
    /// otherwise a rectangle facing the camera.
    ///
    /// **The frame's three axes are depth, width and height, in that order**, so a sprite
    /// spans the second and the third and never the first. The size channel's x lies along
    /// the frame's first axis, which is the axis the position channel travels, and its y and
    /// z are the cross-section (docs/11, "Bolts, streams and fades": a bolt's size channel is
    /// laid out (1, +24, +24), the 1 along the beam). The axes carry their own lengths
    /// ([`Frame::from_points`]), so on a control-point frame the quad comes out the size the
    /// `_w` and `_h` points give it: *measured*, the 47 `_d`/`_w`/`_h` triples the install
    /// ships, `fr_e_brige`'s `RayD/RayW/RayH_1` among them at 150 × 1.93 × 0.41 m
    /// ([07](../../../docs/07-objects.md#ctpt--control-points)). Sizing it by the first axis
    /// instead had the energy bridge's five rays as 150 m squares of white over half the sky.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &self,
        e: &Emitter,
        frame: &Frame,
        local: Vec3,
        size: Vec3,
        alpha: f32,
        age_ms: f32,
        phase: f32,
    ) -> Sprite {
        let [x, y, z] = frame.axes;
        let stretched = (size.x - size.y).abs() > f32::EPSILON;
        let width = size.y * self.scale * y.length();
        Sprite {
            material: e.resource.member.clone(),
            centre: frame.point(local * self.scale),
            along: if stretched { x * size.x * self.scale } else { Vec3::ZERO },
            width,
            height: if stretched { width } else { size.z * self.scale * z.length() },
            alpha,
            overlay: false,
            lengthwise: false,
            age_ms,
            phase,
            dome: None,
            frame: frame.basis().map(|a| std::array::from_fn(|i| a[i] * size.to_array()[i] * self.scale)),
        }
    }

    /// A type 3, 4 or 9 sprite, fading +20 + (+24 − +20) × progress^+28 (`0x10010881`).
    ///
    /// It sits at lerp(+40, +52) in the frame and is lerp(+100, +112) in size along its
    /// axes, each axis by progress through the window raised to that axis's exponent —
    /// +64..+72 for the position, +124..+132 for the size (`0x100106f6`, `0x10010784`;
    /// docs/11, "A channel is a (low, high, jitter, exponent) run"). Its **phase**, from
    /// +8, +12 and +16, is where its material's animation stands ([`phase_fraction`]).
    fn sprite(&self, e: &Emitter, p: f32, age_ms: f32, seconds: f32, out: &mut Vec<Sprite>) {
        let local = lerp3_axis(e.triple(40), e.triple(52), shaped(p, e.triple(64)));
        let size = lerp3_axis(e.triple(100), e.triple(112), shaped(p, e.triple(124)));
        let phase = phase_fraction(e.f(8), e.f(12), e.f(16), p, seconds);
        let mut sprite =
            self.quad(e, &self.frame, local, size, fade(e.f(20), e.f(24), e.f(28), p), age_ms, phase);
        if e.kind == 9 {
            sprite.dome = self.dome(e, size);
        }
        out.push(sprite);
    }

    /// A type-9 emitter's dome in the effect's frame, sized along each of its axes.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- which of the frame's axes the dome's pole
    /// ends on is not established (the read of `Effect.dll:0x1000d110` would put it on the
    /// second axis): here it is the first, so a shield's flash bulges out of the bubble toward
    /// the hit, the round glow a recording shows. Its texture's u runs around the dome and v
    /// from the rim to the pole, which is not read.
    fn dome(&self, e: &Emitter, size: Vec3) -> Option<Dome> {
        let shape = e.body.get(DOME_SHAPE_AT..DOME_SHAPE_AT + 4)?;
        let shape = u32::from_le_bytes(shape.try_into().ok()?) as usize;
        let (segments, rings) = DOME_DETAIL.get(shape).copied().unwrap_or(DOME_DETAIL[0]);
        let [x, y, z] = self.frame.axes;
        let s = size * self.scale;
        Some(Dome { axes: [y * s.y, z * s.z, x * s.x], segments, rings })
    }

    /// A type-5 bolt: floor(length / +36) sprites, at least 1 and at most +20, along the
    /// line from its start point to where the effect is now (`0x10002c53`), fading
    /// +4 → +8 straight across the window (`0x10002dd4`), the texture's u along the line.
    ///
    /// Every sprite is the same width, lerp(+24, +28) by that same progress: the load lays
    /// the size channel's base out as (1, +24, +24) and its delta as (0, +28 − +24,
    /// +28 − +24) (`0x10002944`–`0x100299be`) and the draw hands the channel the progress
    /// (`0x10002fb4`). So +24 and +28 are the width at the window's ends, not at the beam's.
    ///
    /// STAND-IN: docs/11-effects.md#bolts-streams-and-fades--read-and-measured -- its texture
    /// repeats every +32 along the line (`0x10002e79`); here each sprite spans its cell once.
    fn bolt(&self, e: &Emitter, p: f32, age_ms: f32, seconds: f32, out: &mut Vec<Sprite>) {
        let line = self.frame.origin - self.start_point;
        // Its phase is +40 to +44, its power fixed at 1 (`0x100029f0`).
        let phase = phase_fraction(e.f(40), e.f(44), 1.0, p, seconds);
        let step = e.f(36);
        let n = if step > 0.0 { (line.length() / step) as u32 } else { 0 };
        let n = n.min(word(e, 20)).max(1);
        let alpha = e.f(4) + (e.f(8) - e.f(4)) * p;
        for k in 0..n {
            out.push(Sprite {
                material: e.resource.member.clone(),
                centre: self.start_point + line * ((k as f32 + 0.5) / n as f32),
                along: line / n as f32,
                width: (e.f(24) + (e.f(28) - e.f(24)) * p) * self.scale,
                height: (e.f(24) + (e.f(28) - e.f(24)) * p) * self.scale,
                alpha,
                overlay: false,
                lengthwise: true,
                age_ms,
                phase,
                dome: None,
                frame: None,
            });
        }
    }

    /// A type 7 or 10 burst of +0x24 × +0x28 particles (`0x10001720`), each fading
    /// +8 + (+12 − +8) × age^+16 (`0x100013c2`).
    ///
    /// A particle runs from +44 to +56 in the frame and from +92 to +104 in size, each
    /// axis by its age raised to that axis's exponent — +80..+88 for the position,
    /// +128..+136 for the size (`0x10001684`, `0x10001693`). The spawn jitters each high
    /// end by ±half of the triple that follows it, +68 on the position and +116 on the
    /// size (`0x1000186a`, `0x1000195b`, the generator at `0x10002680`).
    ///
    /// STAND-IN: docs/11-effects.md#bolts-streams-and-fades--read-and-measured -- when each
    /// particle spawns is not read: every one is as old as the emitter's progress through
    /// the window over +28.
    fn burst(&self, e: &Emitter, index: u32, p: f32, age_ms: f32, out: &mut Vec<Sprite>) {
        let count = word(e, 36).saturating_mul(word(e, 40)).min(BURST_CAP);
        let life = if e.f(28) > 0.0 { e.f(28) } else { 1.0 };
        let age = (p / life).clamp(0.0, 1.0);
        if p > life {
            return;
        }
        let alpha = fade(e.f(8), e.f(12), e.f(16), age);
        let phase = burst_fraction(age, e.f(32));
        let mut rng = Rng::new(stream_seed(self.seed, index));
        for _ in 0..count {
            let jitter = |rng: &mut Rng, at: usize| {
                let (hi, spread) = (e.triple(at), e.triple(at + 12));
                [0, 1, 2].map(|i| hi[i] + rng.spread(spread[i]))
            };
            let local = lerp3_axis(e.triple(44), jitter(&mut rng, 56), shaped(age, e.triple(80)));
            let size = lerp3_axis(e.triple(92), jitter(&mut rng, 104), shaped(age, e.triple(128)));
            out.push(self.quad(e, &self.frame, local, size, alpha, age_ms, phase));
        }
    }

    /// A type-8 stream's particles, each fading +4 + (+8 − +4) × age^+12 (`0x10012322`),
    /// its age running 0 to 1 over its life.
    ///
    /// It sits at lerp(+88, +100) from where it left and is lerp(+136, +148) in size, each
    /// axis by its age raised to that axis's exponent — +124..+132 for the position,
    /// +172..+180 for the size (`0x100121f8`, `0x10012276`).
    ///
    /// STAND-IN: docs/11-effects.md#bolts-streams-and-fades--read-and-measured -- both are
    /// taken in metres times the instance's scale, and its age in seconds since it
    /// left. The frame only turns them: a control-point frame's axes, which size a type 3,
    /// 4 or 9 sprite, do not, and the shipped streams read as metres either way — a muzzle
    /// puff 0.2 to 0.5, a missile's trail 3 long, a chimney's plume 10 to 30 across. *Seen*:
    /// on the recording of Mission 02 the Large Factory's plume stands about 29 m over the
    /// chimney and is about 31 m across at its widest, where its points' 2.6-long axes had
    /// made it 130 and 78.
    fn stream(&self, index: usize, e: &Emitter, seconds: f32, out: &mut Vec<Sprite>) {
        let Some(stream) = self.streams.iter().find(|s| s.emitter == index) else { return };
        for q in &stream.ring {
            let age = ((seconds - q.born) / q.life.max(f32::EPSILON)).clamp(0.0, 1.0);
            let local = lerp3_axis(e.triple(88), e.triple(100), shaped(age, e.triple(124)));
            let size = lerp3_axis(e.triple(136), e.triple(148), shaped(age, e.triple(172)));
            let alpha = fade(e.f(4), e.f(8), e.f(12), age);
            let since = (seconds - q.born).max(0.0) * 1000.0;
            // Its material's animation stands at age^+32, or at the number drawn for the
            // particle as it left when +32 is negative (`0x10012165`, `0x1001217f`).
            let rate = e.f(32);
            let phase = if rate < 0.0 {
                q.drawn
            } else if rate == 1.0 {
                age
            } else {
                age.powf(rate)
            };
            out.push(self.quad(e, &q.frame.turned(), local, size, alpha, since, phase.clamp(0.0, 1.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::fxid::{Header, TIME_MANUAL, TIME_POINT};
    use parkan_formats::objects::ResourceRef;

    /// The cues that start or stop a sound, leaving out where the playing ones are.
    fn heard(fx: &mut Instance, now_ms: f64) -> Vec<Cue> {
        fx.cues(now_ms).into_iter().filter(|c| c.kind != CueKind::Move).collect()
    }

    /// A block of `kind`, its per-axis exponent triples 1.0 as every shipped block that
    /// does not bend them carries them (an exponent of 0 would put the lerp at its high
    /// end from the first update), and `floats` written over the zeros.
    fn block(kind: u8, size: usize, floats: &[(usize, f32)], material: &str) -> Emitter {
        let mut body = vec![0u8; size];
        body[0] = kind;
        let powers: &[usize] = match kind {
            3 | 4 | 9 => &[64, 124],
            7 | 10 => &[80, 128],
            8 => &[124, 172],
            _ => &[],
        };
        for &at in powers {
            for axis in 0..3 {
                body[at + axis * 4..at + axis * 4 + 4].copy_from_slice(&1.0f32.to_le_bytes());
            }
        }
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
        let frame = Frame { origin: Vec3::ZERO, axes: [Vec3::Y, Vec3::Z, Vec3::NEG_X], points: false };
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

    /// `f_brige_ray`, the effect C02 Mission 04's energy bridge hangs five of on its deck:
    /// a type-3 sprite on the three control points `RayD_1`, `RayW_1` and `RayH_1`, which
    /// share a position and carry (0, 150, 0), (1.932, 0, 0) and (0, 0, -0.414) as their
    /// directions -- the span's depth, the ray's width and its height.
    #[test]
    fn a_sprite_on_a_control_point_frame_is_its_width_and_height_across_and_travels_its_depth() {
        // The shipped block fades 0 → 1 across its window; here it is opaque throughout, so
        // the sprite draws at both ends of it.
        let ray =
            block(3, 200, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (28, 1.0), (40, 0.5)], "b_a_brige");
        let ray = [52, 56, 60, 100, 104, 108, 112, 116, 120]
            .into_iter()
            .fold(ray, |e, at| with_word(e, at, 1.0f32.to_bits()));
        let at = Vec3::new(0.0, 0.0, 10.0);
        let frame = Frame::from_points([
            (at, Vec3::new(0.0, 150.0, 0.0)),
            (at, Vec3::new(1.932, 0.0, 0.0)),
            (at, Vec3::new(0.0, 0.0, -0.414)),
        ]);
        // The shared `effect` helper carries a header scale of 0.1; the bridge's is 1.
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![ray]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let mut fx = Instance::new(whole, frame, 1.0, 0.0, None, 1);
        let mut out = Vec::new();
        fx.value = 0.0;
        fx.sprites(0.0, true, &mut out);
        let s = &out[0];
        assert!((s.width - 1.932).abs() < 1e-4, "the width point's length across: {}", s.width);
        assert!((s.height - 0.414).abs() < 1e-4, "the height point's length up: {}", s.height);
        assert_eq!(s.along, Vec3::ZERO, "it faces the camera");
        // The three points are not in a plane, so the quad is turned through the frame and
        // carries all three of its axes: 150 m along the span and 1.93 across.
        let axes = s.frame.expect("a frame of three control points is a basis");
        assert_eq!(
            axes,
            [Vec3::new(0.0, 150.0, 0.0), Vec3::new(1.932, 0.0, 0.0), Vec3::new(0.0, 0.0, -0.414)]
        );
        // It starts half way along the 150 m span and runs to the far end over the window.
        assert!((s.centre - Vec3::new(0.0, 75.0, 10.0)).length() < 1e-3, "{:?}", s.centre);
        out.clear();
        fx.value = 1.0 - 1e-6;
        fx.sprites(0.0, true, &mut out);
        assert!((out[0].centre.y - 150.0).abs() < 1e-2, "{:?}", out[0].centre);
    }

    #[test]
    fn a_frame_of_one_point_named_three_times_is_no_basis_and_its_sprite_faces_the_camera() {
        // The load groups name the same point three times on 460-odd records -- every sign,
        // lamp and console screen -- and the handler builds an orientation from the one
        // direction there instead of a matrix of three.
        let glow = block(3, 200, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (28, 1.0)], "G");
        let glow =
            [100, 104, 108, 112, 116, 120].into_iter().fold(glow, |e, at| with_word(e, at, 1.0f32.to_bits()));
        let point = (Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, -0.349, -0.937));
        let frame = Frame::from_points([point; 3]);
        assert_eq!(frame.basis(), None);
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![glow]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let fx = Instance::new(whole, frame, 1.0, 0.0, None, 1);
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert_eq!(out[0].frame, None);
        assert_eq!(out[0].along, Vec3::ZERO);
        assert!((out[0].centre - point.0).length() < 1e-6);
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
        assert!(heard(&mut fx, 0.0).is_empty(), "t 0 is short of 0.01");
        fx.value = 0.5;
        let cues = heard(&mut fx, 10.0);
        assert_eq!(cues.len(), 1);
        assert_eq!((cues[0].position, cues[0].near, cues[0].far), (Vec3::ONE, 10.0, 80.0));
        fx.value = 1.0;
        assert!(heard(&mut fx, 20.0).is_empty(), "already past");
        fx.value = 0.0;
        assert!(heard(&mut fx, 30.0).is_empty());
        fx.value = 0.5;
        assert_eq!(heard(&mut fx, 40.0).len(), 1, "the next stroke");
        // While it may still play, each update says where it is now.
        fx.frame.origin = Vec3::new(5.0, 0.0, 0.0);
        let moves: Vec<Cue> = fx.cues(50.0).into_iter().filter(|c| c.kind == CueKind::Move).collect();
        assert_eq!(moves.len(), 1);
        assert_eq!((moves[0].position, moves[0].key, moves[0].far), (Vec3::new(5.0, 0.0, 0.0), (0, 0), 80.0));
    }

    #[test]
    fn an_effect_flagged_to_start_off_neither_draws_nor_sounds_until_switched_on() {
        let burst = block(7, 208, &[(20, 0.0), (24, 1.0), (8, 1.0), (12, 0.2), (16, 2.0), (28, 1.0)], "fire");
        let smoke = with_word(with_word(burst, 36, 3), 40, 5);
        let chime = block(2, 148, &[(8, 0.1), (12, 1.0), (64, 1.0), (68, 10.0)], "chime.wav");
        let mut e = effect(TIME_MANUAL, 0.0, 0, vec![smoke, chime]);
        Rc::get_mut(&mut e).unwrap().header.flags |= FX_START_OFF;
        let mut fx = Instance::new(e, Frame::along(Vec3::ZERO, Vec3::X, 1.0), 1.0, 0.0, None, 1);
        fx.value = 0.5;
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert!(!fx.on && out.is_empty() && heard(&mut fx, 0.0).is_empty());
        fx.on = true;
        fx.sprites(10.0, true, &mut out);
        assert!(!out.is_empty() && heard(&mut fx, 10.0).len() == 1);
    }

    #[test]
    fn a_one_shot_that_sat_at_one_plays_again_on_the_way_down_and_a_loop_plays_inside_its_window() {
        // An arm's sound: a trigger of 0.15. Out to exactly 1, then folding.
        let arm = block(2, 148, &[(8, 0.15), (12, 1.0), (64, 2.0), (68, 20.0)], "H_gh_cannon.wav");
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_POINT, 0.0, 0, vec![arm]), frame, 1.0, 0.0, None, 1);
        fx.value = 0.5;
        assert_eq!(heard(&mut fx, 0.0).len(), 1, "unfolding past 0.15");
        fx.value = 1.0;
        assert!(heard(&mut fx, 10.0).is_empty(), "out: 1 is taken as 0");
        fx.value = 0.9;
        assert_eq!(heard(&mut fx, 20.0).len(), 1, "folding: from 0 past 0.15 again");
        fx.value = 0.5;
        assert!(heard(&mut fx, 30.0).is_empty());
        // Folded from short of 1: silent.
        fx.value = 0.8;
        heard(&mut fx, 40.0);
        fx.value = 0.3;
        assert!(heard(&mut fx, 50.0).is_empty());

        let breath =
            with_word(block(2, 148, &[(8, 0.2), (12, 0.8), (64, 1.0), (68, 4.0)], "H_breath.wav"), 4, 2);
        let mut fx = Instance::new(effect(TIME_POINT, 0.0, 0, vec![breath]), frame, 1.0, 0.0, None, 1);
        fx.id = 7;
        fx.value = 0.1;
        assert!(heard(&mut fx, 0.0).is_empty());
        fx.value = 0.5;
        let start = heard(&mut fx, 10.0);
        assert_eq!((start.len(), start[0].kind, start[0].key), (1, CueKind::Loop, (7, 0)));
        assert!(heard(&mut fx, 20.0).is_empty(), "still playing");
        fx.value = 0.9;
        assert_eq!(heard(&mut fx, 30.0)[0].kind, CueKind::Stop);
        fx.value = 0.5;
        heard(&mut fx, 40.0);
        assert_eq!(fx.silence()[0].kind, CueKind::Stop, "the instance goes");

        let mut quiet = Instance::new(fx.effect.clone(), frame, 1.0, 0.0, None, 1);
        quiet.silent = true;
        quiet.value = 0.5;
        assert!(heard(&mut quiet, 10.0).is_empty(), "a silent instance starts no loop");
    }

    /// A building ambience: `f_bunk_sfx`'s loop over the whole of a one-second mode-2 time.
    #[test]
    fn a_loop_over_the_whole_of_a_looping_time_plays_on_through_the_wrap() {
        let hum =
            with_word(block(2, 148, &[(8, 0.001), (12, 1.0), (64, 5.0), (68, 28.0)], "f_bunk.wav"), 4, 2);
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_LOOP, 1.0, 0, vec![hum]), frame, 1.0, 0.0, None, 1);
        assert!(heard(&mut fx, 0.0).is_empty(), "t 0 is short of 0.001");
        assert_eq!(heard(&mut fx, 500.0)[0].kind, CueKind::Loop);
        for tick in 1..=180 {
            let cues = heard(&mut fx, f64::from(tick) * 1000.0 / 60.0);
            assert!(cues.is_empty(), "nothing at tick {tick}: {cues:?}");
        }

        // A loop that gives up the end of its time is still stopped there and started again.
        let half =
            with_word(block(2, 148, &[(8, 0.0), (12, 0.5), (64, 5.0), (68, 28.0)], "f_bunk.wav"), 4, 2);
        let mut fx = Instance::new(effect(TIME_LOOP, 1.0, 0, vec![half]), frame, 1.0, 0.0, None, 1);
        assert_eq!(heard(&mut fx, 0.0)[0].kind, CueKind::Loop);
        assert_eq!(heard(&mut fx, 600.0)[0].kind, CueKind::Stop);
        assert_eq!(heard(&mut fx, 1100.0)[0].kind, CueKind::Loop);
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

    #[test]
    fn a_channels_two_ends_are_lerped_per_axis_by_the_progress_raised_to_that_axiss_exponent() {
        // A sprite moving (0,0,0) → (4,4,4) and growing 1 → 3 on its first axis, with the
        // position's exponents (1, 2, 0.5) and the size's 2 (`0x100106f6`, `0x10010784`).
        let sprite = block(
            3,
            200,
            &[
                (32, 0.0),
                (36, 1.0),
                (20, 1.0),
                (24, 1.0),
                (28, 1.0),
                (52, 4.0),
                (56, 4.0),
                (60, 4.0),
                (68, 2.0),
                (72, 0.5),
                (100, 1.0),
                (104, 1.0),
                (108, 1.0),
                (112, 3.0),
                (116, 3.0),
                (120, 3.0),
                (124, 2.0),
                (128, 2.0),
                (132, 2.0),
            ],
            "flash",
        );
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        // The header's scale is 0.1, so a size of 10 puts the instance at scale 1.
        let mut fx = Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![sprite]), frame, 10.0, 0.0, None, 1);
        fx.value = 0.25;
        let mut out = Vec::new();
        fx.sprites(0.0, false, &mut out);
        assert_eq!(out.len(), 1);
        // x straight, y by 0.25² = 0.0625, z by sqrt(0.25) = 0.5; the size by 0.25².
        assert!((out[0].centre - Vec3::new(1.0, 0.25, 2.0)).length() < 1e-4, "{:?}", out[0].centre);
        assert!((out[0].width - (1.0 + 2.0 * 0.0625)).abs() < 1e-5, "{}", out[0].width);

        // An exponent of exactly 1.0 is taken straight, and every axis of 3518 of the 3571
        // shipped drawing blocks carries it.
        assert_eq!(shaped(0.25, [1.0, 1.0, 1.0]), Vec3::splat(0.25));
    }

    #[test]
    fn a_bursts_particle_runs_44_to_56_in_place_and_92_to_104_in_size_over_its_life() {
        // One particle, no jitter (+68 and +116 left at 0), running to (2, 0, 0) and from
        // 1 to 5 in size over a life of +28 = 1.
        let e = block(
            7,
            208,
            &[
                (20, 0.0),
                (24, 1.0),
                (28, 1.0),
                (8, 1.0),
                (12, 1.0),
                (16, 1.0),
                (56, 2.0),
                (92, 1.0),
                (96, 1.0),
                (100, 1.0),
                (104, 5.0),
                (108, 5.0),
                (112, 5.0),
            ],
            "fire",
        );
        let burst = with_word(with_word(e, 36, 1), 40, 1);
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![burst]), frame, 10.0, 0.0, None, 7);
        let mut out = Vec::new();
        for (age, x, width) in [(0.0, 0.0, 1.0), (0.5, 1.0, 3.0), (1.0, 2.0, 5.0)] {
            fx.value = age;
            out.clear();
            fx.sprites(0.0, false, &mut out);
            assert_eq!(out.len(), 1);
            assert!((out[0].centre.x - x).abs() < 1e-5, "{} at {age}", out[0].centre.x);
            assert!((out[0].width - width).abs() < 1e-5, "{} at {age}", out[0].width);
        }
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
        assert!(fx.takes_target_point() && out.iter().all(|s| s.lengthwise));

        // Handed the muzzle for its start, and run once through as its round stops, it fades
        // out over the header's 0.75 s (docs/29, "A beam outlives its round").
        fx.start_point = Vec3::new(10.0, 4900.0, 0.0);
        fx.mode = TIME_ONCE;
        (fx.start_ms, fx.end_ms) = (1000.0, 1750.0);
        for (now, alpha) in [(1000.0, 1.0), (1375.0, 0.5), (1750.0, 0.0)] {
            out.clear();
            fx.sprites(now, true, &mut out);
            if alpha == 0.0 {
                assert!(out.is_empty(), "a fade of 0 draws nothing");
            } else {
                assert_eq!(out.len(), 2, "floor(100 / 50)");
                assert!(out.iter().all(|s| (s.alpha - alpha).abs() < 1e-5), "{} at {now}", out[0].alpha);
                assert!((out[0].centre - Vec3::new(10.0, 4925.0, 0.0)).length() < 1e-2);
            }
        }
    }

    #[test]
    fn a_bolts_sprites_are_as_wide_as_lerp_24_28_by_the_progress_through_its_window() {
        // `las_l_tail_g`'s wide beam: 1.5 at the window's start, 0.5 at its end.
        let e =
            block(5, 112, &[(12, 0.0), (16, 1.0), (4, 1.0), (8, 1.0), (24, 1.5), (28, 0.5), (36, 50.0)], "L");
        let wide = with_word(e, 20, 20);
        let frame = Frame::along(Vec3::new(0.0, 100.0, 0.0), Vec3::Y, 1.0);
        let mut fx = Instance::new(effect(TIME_MANUAL, 1.0, 0, vec![wide]), frame, 10.0, 0.0, None, 1);
        fx.start_point = Vec3::ZERO;
        let mut out = Vec::new();
        for (p, width) in [(0.0, 1.5), (0.5, 1.0), (1.0, 0.5)] {
            fx.value = p;
            out.clear();
            fx.sprites(0.0, false, &mut out);
            assert_eq!(out.len(), 2, "floor(100 / 50)");
            assert!(out.iter().all(|s| (s.width - width).abs() < 1e-5), "{} at {p}", out[0].width);
        }
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
    fn flag_0x400_draws_nothing_while_its_point_is_hidden_and_depth_tested_while_in_view() {
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
        assert!(!out[0].overlay, "a beacon carries no bit 8, so its glow is cut by the faces it hangs on");
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

    /// The generator itself: the recurrence, and that its period is what the linear
    /// algebra over GF(2) gives -- one transient step, then 127 x 47 x 178_481.
    #[test]
    fn the_generators_recurrence_and_its_cycle() {
        // Stepped by hand from (0x1234, 0x5678), as `Effect.dll:0x10002220` does.
        let mut rng = Rng::new(0x5678_1234);
        let (mut lo, mut hi) = (0x1234u16, 0x5678u16);
        for _ in 0..1000 {
            lo = (lo << 1) ^ hi;
            hi = (hi >> 1) ^ lo;
            assert_eq!(rng.next16(), hi);
        }
        // A zero state is a fixed point, which is why the seed steps off it.
        assert_ne!(Rng::new(0), Rng { lo: 0, hi: 0 });
        // The cycle: stepping the period brings the state back, and no prime factor of it
        // does. 1_065_353_089 draws is too many to walk, so the check is on the map's
        // order, which the doc works out; here the sequence is only checked not to repeat
        // early, over a window longer than the shortest factor.
        let mut seen = std::collections::HashSet::new();
        let mut rng = Rng::new(0xDEAD_BEEF);
        for _ in 0..200_000 {
            assert!(seen.insert(rng), "the state repeats inside 200000 steps");
            rng.next16();
        }
        // Uniform: the mean of a long run sits on a half.
        let mut rng = Rng::new(0x1234_5678);
        let mean: f64 = (0..200_000).map(|_| f64::from(rng.unit())).sum::<f64>() / 200_000.0;
        assert!((mean - 0.5).abs() < 0.01, "mean {mean}");
        // And `spread` lands inside +-half of what it is given (`0x10002680`).
        let mut rng = Rng::new(7);
        for _ in 0..1000 {
            let v = rng.spread(0.2);
            assert!((-0.1..=0.1).contains(&v), "{v}");
        }
    }

    /// Header flag 1 moves *t* by a uniform in +-half of the header's +0xc and clamps
    /// (`Effect.dll:0x1000830e`), and does nothing without the flag.
    #[test]
    fn flag_1_jitters_effect_time_by_the_headers_spread() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let jittered = |flags: u32, spread: f32| {
            let mut e = effect(TIME_MANUAL, 1.0, flags, Vec::new());
            Rc::get_mut(&mut e).expect("one owner").header.jitter = spread;
            let mut fx = Instance::new(e, frame, 1.0, 0.0, None, 12345);
            let mut seen = Vec::new();
            for step in 0..64 {
                fx.value = 0.5;
                fx.update(f64::from(step) * 100.0);
                seen.push(fx.t(f64::from(step) * 100.0));
            }
            seen
        };
        let still = jittered(0, 0.2);
        assert!(still.iter().all(|&t| t == 0.5), "no flag, no jitter");
        let moved = jittered(FX_JITTER, 0.2);
        assert!(moved.iter().any(|&t| t != 0.5), "flag 1 moves t");
        assert!(moved.iter().all(|&t| (0.4..=0.6).contains(&t)), "inside +-0.1 of 0.5");
        // It is clamped to 0..1 after the jitter, not before.
        let mut e = effect(TIME_MANUAL, 1.0, FX_JITTER, Vec::new());
        Rc::get_mut(&mut e).expect("one owner").header.jitter = 2.0;
        let mut fx = Instance::new(e, frame, 1.0, 0.0, None, 99);
        for step in 0..32 {
            fx.value = 0.0;
            fx.update(f64::from(step) * 100.0);
            let t = fx.t(f64::from(step) * 100.0);
            assert!((0.0..=1.0).contains(&t), "{t}");
        }
    }

    /// Time mode 14 is one minus the owner's life fraction (property `0x31`,
    /// `Effect.dll:0x10005f2d`), 13 one minus its point's value (`0x10005f06`), and 15 the
    /// larger of the speed and the spin (`0x10005f50`).
    #[test]
    fn the_owner_values_of_the_later_time_modes() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let make = |mode| Instance::new(effect(mode, 1.0, 0, Vec::new()), frame, 1.0, 0.0, None, 0);

        let mut burning = make(TIME_LIFE_INVERSE);
        assert_eq!(burning.t(100.0), 0.0, "an undamaged owner holds a mode-14 effect at 0");
        burning.life = 0.25;
        assert_eq!(burning.t(100.0), 0.75, "three quarters gone, three quarters of the way on");

        let mut point = make(TIME_POINT_INVERSE);
        point.value = 0.3;
        assert_eq!(point.t(100.0), 0.7);

        let mut motion = make(TIME_MOTION);
        motion.speed = 0.2;
        motion.spin = 0.6;
        assert_eq!(motion.t(100.0), 0.6, "mode 15 takes the larger");
        motion.speed = 0.9;
        assert_eq!(motion.t(100.0), 0.9);

        let mut spinning = make(TIME_SPIN);
        spinning.spin = 0.4;
        spinning.speed = 1.0;
        assert_eq!(spinning.t(100.0), 0.4, "modes 9-12 read the spin, not the speed");
    }

    /// A phase is where its material's animation stands: it runs from the block's +8 to
    /// its +12 over the window, or in seconds when +8 is negative, and the draw takes its
    /// fractional part (`Effect.dll:0x100104d4`, `0x10010817`).
    #[test]
    fn a_sprites_phase_runs_its_materials_animation() {
        // (0, 1, 1) -- 1087 of the 2013 shipped sprite blocks -- plays it once across the
        // window, ending just short of the last key.
        assert_eq!(phase_fraction(0.0, 1.0, 1.0, 0.0, 0.0), 0.0);
        assert_eq!(phase_fraction(0.0, 1.0, 1.0, 0.25, 0.0), 0.25);
        assert_eq!(phase_fraction(0.0, 1.0, 1.0, 1.0, 0.0), 0.0, "1 wraps to the first key");
        // (-1, 9, 1) -- 387 of them -- is clocked in seconds and plays nine times over.
        assert_eq!(phase_fraction(-1.0, 9.0, 1.0, 0.5, 0.0), 0.0);
        assert!((phase_fraction(-1.0, 9.0, 1.0, 0.5, 0.1) - 0.9).abs() < 1e-5);
        assert!((phase_fraction(-1.0, 9.0, 1.0, 0.5, 0.25) - 0.25).abs() < 1e-5);
        // A burst's rate is +32: 1.0 plays it once over the particle's life, a rate below
        // 1 finishes early and holds at 0.99, and -1 -- 350 of the 1161 type-7 blocks --
        // never leaves the first key.
        assert_eq!(burst_fraction(0.4, 1.0), 0.4);
        assert!((burst_fraction(0.3, 0.6) - 0.5).abs() < 1e-6);
        assert_eq!(burst_fraction(0.9, 0.6), 0.99);
        assert_eq!(burst_fraction(0.4, -1.0), 0.0);
        assert!((burst_fraction(0.4, 1.5) - 0.6).abs() < 1e-6);
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
