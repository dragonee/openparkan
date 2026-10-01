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
/// Header flag 0x2000: the sprites' fog factor is held at 1 (docs/11, "How an effect sprite is
/// coloured"). `env_lightning` alone carries it, so a bolt shows across the map.
pub const FX_NO_FOG: u32 = 0x2000;
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

/// The side the game squares one direction with, wherever it builds a frame or a sprite's
/// basis about a single vector (`Control.dll:0x10003ef0`, `Effect.dll:0x100038d0`,
/// `0x1000d110`, and the sprite draw's own at `0x10009453`): the vector's horizontal
/// perpendicular (−y, x, 0), normalised where the vector has a z and left as it is where it
/// has none -- and **on the axes a fixed one**: the x axis where the vector's x is exactly 0,
/// the y axis where its y is and its x is not (the triples at `Control.dll:0x10041d80` and
/// `0x10041d90`, `Effect.dll:0x100247c8` and `0x100247d8`). So a vector straight up or down
/// never degenerates, and the vectors along +y and −x get the opposite of the side their
/// perpendicular would have been.
pub fn side_of(d: Vec3) -> Vec3 {
    if d.x != 0.0 && d.y != 0.0 {
        let side = Vec3::new(-d.y, d.x, 0.0);
        if d.z != 0.0 { side.normalize_or(Vec3::X) } else { side }
    } else if d.x != 0.0 {
        Vec3::Y
    } else {
        Vec3::X
    }
}

impl Frame {
    /// A frame at `origin` built about the one direction `x`, all three axes `size` long: the
    /// direction is the **first** axis, [`side_of`] it the second and their cross product,
    /// direction × side, the third. That is the matrix `Control.dll:0x10003ef0` writes -- its
    /// columns the direction, the side, the cross and the origin (`0x1000400b`–`0x1000405b`)
    /// -- which an explosion's effect is placed with (`0x1001181a`) and a load group's effect
    /// on one control point named three times ([`Frame::from_points`]).
    pub fn along(origin: Vec3, x: Vec3, size: f32) -> Self {
        let x = x.normalize_or(Vec3::X);
        let y = side_of(x);
        let z = x.cross(y);
        Self { origin, axes: [x * size, y * size, z * size], points: false }
    }

    /// The frame three control points give (action 4, `Control.dll:0x10002d8d`): their
    /// centroid, and their vectors as the axes. The handler builds the identity and writes
    /// each direction into a row of it, the centroid into the fourth, and hands that matrix
    /// to the effect -- so the frame *is* a matrix, and the axes' own lengths are in it.
    ///
    /// Three equal directions -- every one of the 690 load-group records that names one point
    /// three times, each sign, lamp and screen -- go down the handler's other branch
    /// (`0x10002c42`): an orientation built about the unit direction (`0x10003ef0`,
    /// [`Frame::along`]), scaled alike on all three axes by the direction's length
    /// (`0x10002c7b`–`0x10002cfd`, `0x10003ea0`). The direction lands where a triple's first
    /// point does, on the first axis; the second is its side and the third their cross
    /// (docs/13, "A frame about one direction").
    pub fn from_points(points: [(Vec3, Vec3); 3]) -> Self {
        let origin = (points[0].0 + points[1].0 + points[2].0) / 3.0;
        let d = points[0].1;
        if points.iter().all(|p| p.1 == d) && d.length() > 0.0 {
            return Self::along(origin, d, d.length());
        }
        Self { origin, axes: [points[0].1, points[1].1, points[2].1], points: true }
    }

    /// The frame as a basis a sprite is drawn through, where it is one: three control-point
    /// directions that are not all in a plane.
    ///
    /// A frame built about one direction is an orientation scaled alike, and is not one of
    /// these: a burst's or a stream's particle on it faces the camera in the world.
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

    /// A point of the world in the frame's own space, its axes `scale` times as long, as the
    /// manager's emitter loop puts the eye there before it hands it to the emitters
    /// (`Effect.dll:0x100080da`, `g_FastProc` slot `0x68`, `Ngi32.dll:0x100248f0`): the point
    /// less the origin, projected on each axis and divided by that axis's squared length. That
    /// is the inverse of a matrix whose axes are square to one another, whatever their lengths;
    /// `None` where an axis has no length.
    pub fn local(&self, scale: f32, world: Vec3) -> Option<Vec3> {
        let to = world - self.origin;
        let mut out = Vec3::ZERO;
        for (i, axis) in self.axes.iter().enumerate() {
            let axis = *axis * scale;
            let squared = axis.length_squared();
            if squared <= 0.0 || !squared.is_finite() {
                return None;
            }
            out[i] = axis.dot(to) / squared;
        }
        Some(out)
    }

    /// How far `eye` stands from the frame's origin in the units of the frame `scale` times
    /// as large: the length of the eye as an emitter's update is handed it, in the instance's
    /// space ([`Frame::local`]). The instance's matrix carries its scale -- the placement's
    /// columns times the size asked for times the header's (`Effect.dll:0x10007c90`), under
    /// the owner's node (`0x10008469`) -- so the scale divides the distance as an axis's
    /// length does. A frame with an axis of no length gives the distance in the world.
    pub fn local_distance(&self, scale: f32, eye: Vec3) -> f32 {
        self.local(scale, eye).map_or((eye - self.origin).length(), Vec3::length)
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

/// A type-1 block's fields (docs/11, "Type 1 is a light"): its kind, and the start and end of
/// its position, colour (RGBA) and range, each lerped by the progress through its window.
pub const LIGHT_KIND_AT: usize = 4;
pub const LIGHT_POSITION_AT: (usize, usize) = (16, 28);
pub const LIGHT_DIRECTION_AT: (usize, usize) = (40, 52);
pub const LIGHT_COLOUR_AT: (usize, usize) = (64, 80);
pub const LIGHT_RANGE_AT: (usize, usize) = (112, 116);
/// The colour's jitter, a spread for each of its four channels, and the range's: every update
/// of a light that is on adds a uniform in ± half of each (`Effect.dll:0x1000f89a`–`0x1000f9a4`,
/// `0x1000fa8e`–`0x1000faf7`).
pub const LIGHT_COLOUR_JITTER_AT: usize = 96;
pub const LIGHT_RANGE_JITTER_AT: usize = 120;
/// The three attenuation terms, handed to the manager as they stand (`0x1000fb94`–`0x1000fbb8`,
/// record `+0x38`, `+0x3c`, `+0x40`): the constant, linear and quadratic terms of a falloff on
/// the distance left to the range, as Direct3D's `D3DLIGHT2` has them
/// ([`PointLight::diffuse_at`]).
pub const LIGHT_ATTENUATION_AT: usize = 124;
/// What a light record says of a light the light manager keeps: a point light, its flags.
/// `0x80000000` lights only its owner; `0x20000000` is passed over by the shade's emulation,
/// and lights a surface only through its vertices.
pub const LIGHT_OWNER_ONLY: u32 = 0x8000_0000;
pub const LIGHT_NOT_EMULATED: u32 = 0x2000_0000;
/// How long an instance keeps what its last update drew: the manager updates one once 100 ms
/// have passed since its last update (`Effect.dll:0x100081a7`–`0x100081bc`).
pub const UPDATE_INTERVAL_MS: f64 = 100.0;
/// What a range that is not above 0 becomes (`Effect.dll:0x1000fb7b`–`0x1000fb8c`).
pub const LIGHT_LEAST_RANGE: f32 = 0.01;

/// A type-1 block's kind as the light it asks the manager for (`Effect.dll:0x1000f649`): a
/// point light's flags, or `None` for the directional (3) and parallel-point (4) kinds.
pub fn point_light_flags(kind: u32) -> Option<u32> {
    match kind {
        1 => Some(LIGHT_OWNER_ONLY),
        2 | 5 => Some(0),
        6 => Some(LIGHT_OWNER_ONLY | LIGHT_NOT_EMULATED),
        7 => Some(LIGHT_NOT_EMULATED),
        _ => None,
    }
}

/// A point light an effect drives this frame, where it stands in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    pub position: Vec3,
    pub colour: Vec3,
    pub range: f32,
    pub flags: u32,
    /// The block's three attenuation terms, as they stand.
    pub attenuation: [f32; 3],
}

impl PointLight {
    /// What the light gives a vertex at `at` whose normal is `normal`, before the material's
    /// diffuse multiplies it, as the shade's own lighter works it out
    /// (`CShade::ShadeIndexedStrided`, `Terrain.dll:0x1004df70`, and the point-light routine it
    /// calls, `Ngi32.dll:0x100164f0`): nothing past the range (`0x100165bb`) or on a vertex
    /// turned away (`0x1001663d`); otherwise the colour times the cosine at the vertex times
    /// a₀ + a₁ x + a₂ x², where x = (range − d) ÷ range is the share of the range left
    /// (`0x10016678`–`0x100166ac`). It is `D3DLIGHT2`'s falloff, on the record's three terms in
    /// their own order: the 447 blocks with (0, 1, 0) fall in a straight line from the light to
    /// nothing at their range, and the 170 with (0, 1, 1) start at twice the colour.
    ///
    /// The device's own lights are not used: `UseDXLighting`, the shade's setting 29, is 0 as
    /// compiled (`0x1005fcce`) and nothing sets it, so every lit item goes to this lighter
    /// (`0x1002fe50`) and the builder that would hand Direct3D the same list is never reached.
    pub fn diffuse_at(&self, at: Vec3, normal: Vec3) -> Vec3 {
        let to = self.position - at;
        let d = to.length();
        if d > self.range || d <= 0.0 || self.range <= 0.0 {
            return Vec3::ZERO;
        }
        let facing = normal.normalize_or_zero().dot(to / d);
        if facing <= 0.0 {
            return Vec3::ZERO;
        }
        let x = (self.range - d) / self.range;
        let [a0, a1, a2] = self.attenuation;
        self.colour * (facing * (a0 + a1 * x + a2 * x * x))
    }
}

/// What one light emitter's last update drew: the colour's jitter and the range's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct LightJitter {
    colour: Vec3,
    range: f32,
}

/// The shade's light template drawn over a lit triangle (`Terrain.dll:0x1002a130`, docs/11,
/// "What a light does to a surface"): the square of half-side sqrt(range² − d²) about the
/// light's foot on the triangle's plane, its first axis the outward normal of the edge from
/// the first vertex to the second, clipped to the triangle by its three edge planes, each
/// corner with its (u, v) on the template's texture. Empty when the light is past the range
/// from the plane or from any edge's side plane; a light behind the face counts.
pub fn light_disc(tri: [Vec3; 3], light: Vec3, range: f32) -> Vec<(Vec3, [f32; 2])> {
    let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
    if n == Vec3::ZERO {
        return Vec::new();
    }
    let d = n.dot(light - tri[0]);
    if d.abs() > range {
        return Vec::new();
    }
    let edges: [(Vec3, f32); 3] = std::array::from_fn(|k| {
        let m = (tri[(k + 1) % 3] - tri[k]).cross(n).normalize_or_zero();
        (m, -m.dot(tri[k]))
    });
    if edges.iter().any(|&(m, w)| m.dot(light) + w > range) {
        return Vec::new();
    }
    let r = (range * range - d * d).max(0.0).sqrt();
    let centre = light - n * d;
    let (u, v) = (edges[0].0 * r, edges[0].0.cross(n) * r);
    // The corners' (u, v) run 0 to 0.99 (`0x1002aacf`–`0x1002ab1f`).
    let mut polygon = vec![
        (centre + u + v, [0.0, 0.0]),
        (centre - u + v, [0.99, 0.0]),
        (centre - u - v, [0.99, 0.99]),
        (centre + u - v, [0.0, 0.99]),
    ];
    // Sutherland–Hodgman against each edge plane, keeping its inside (`0x100504a0`).
    for &(m, w) in &edges {
        let side = |p: Vec3| m.dot(p) + w;
        let mut kept = Vec::with_capacity(polygon.len() + 1);
        for i in 0..polygon.len() {
            let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            let (sa, sb) = (side(a.0), side(b.0));
            if sa <= 0.0 {
                kept.push(a);
            }
            if (sa <= 0.0) != (sb <= 0.0) {
                let f = sa / (sa - sb);
                let uv = [a.1[0] + (b.1[0] - a.1[0]) * f, a.1[1] + (b.1[1] - a.1[1]) * f];
                kept.push((a.0 + (b.0 - a.0) * f, uv));
            }
        }
        polygon = kept;
        if polygon.is_empty() {
            break;
        }
    }
    polygon
}

/// A type-4 block's distance its sprite reaches its full size at, in the frame's units:
/// 500 on every glow the buildings carry.
pub const FLARE_REACH_AT: usize = 200;

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
    /// Its fog factor held at 1, whatever its distance: header flag [`FX_NO_FOG`], effect
    /// draw flag 4 (`Effect.dll:0x1001088c`, `Terrain.dll:0x10028417`).
    pub unfogged: bool,
    /// Drawn as a hemisphere through `matrix` instead of a quad: a type-9 emitter's shape.
    pub dome: Option<Dome>,
    /// The frame to turn the quad through, each axis as long as the sprite is that way:
    /// a camera-facing unit square in this basis rather than on the screen, so a frame
    /// whose axes differ draws a long streak along its first when seen across it and a small
    /// one end-on ([`Frame::basis`], docs/11, "A sprite is drawn through its frame"). A burst's
    /// and a stream's particles carry it; a type-3, 4 or 9 sprite carries its `matrix`.
    pub frame: Option<[Vec3; 3]>,
    /// The sprite's own matrix, the one its mode makes in its frame's space carried out through
    /// the frame and the size (the particle's `+0x14`, [`Instance::through`]): where the unit
    /// vectors of the sprite's own x, y and z end up in the world, about `centre`. A quad is
    /// the unit square across the first two, its (u, v) running with them; a dome's rim spans
    /// them and its pole ends on the third. `None` where the mode needs an eye and none was
    /// given, and the quad is then the camera-facing one the fields above describe.
    pub matrix: Option<[Vec3; 3]>,
}

/// A type-9 emitter's hemisphere, one of the shade's three (`Terrain.dll:0x10027a20`,
/// `0x100273b0`, `0x100276a0`): a **unit-radius** half sphere in the sprite's own space, its
/// pole at (0, 0, 1) and its rim the unit circle of the xy plane, cut into `segments` around and
/// `rings` from the pole to the rim; the sprite's matrix carries it into the world. It is drawn
/// from both sides -- the item's draw flags are 4, which turns culling off
/// (`Terrain.dll:0x100282a3`, `Ngi32.dll:0x10007662`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dome {
    pub segments: u8,
    pub rings: u8,
    /// The texture is laid over the whole dome as seen down its pole, (u, v) = ((x + 1) / 2,
    /// (y + 1) / 2) of each vertex (`0x100275ff`, `0x10027959`); otherwise every facet takes
    /// the whole texture, as a quad does (`0x10028bbe`).
    pub projected: bool,
}

/// A type-9 block's shape (`+200`), which the sprite's update turns into the shade's shape
/// code 3, 5 or 6 (`Effect.dll:0x100138c8`; any other value draws nothing), and the detail
/// each gives its dome at the finest of the four levels the code's top two bits pick (tables
/// `Terrain.dll:0x1009a750` and `0x1009a780`): around, and from pole to rim.
pub const DOME_SHAPE_AT: usize = 200;
pub const DOME_DETAIL: [(u8, u8); 3] = [(8, 3), (16, 6), (24, 9)];
/// A type-9 block's `+204`: 0 lays the texture over the whole dome (code bit `0x8000000`,
/// `Effect.dll:0x1001390e`), anything else gives every facet the whole of it.
pub const DOME_TILED_AT: usize = 204;
/// A type-3, 4 or 9 block's **sprite mode** (`+4`), which the load copies to the particle's
/// `+0xc` (`Effect.dll:0x100103d2`) and the draw switches on (`0x100093ed`).
pub const SPRITE_MODE_AT: usize = 4;
/// Mode 0: the sprite faces the eye, in its frame's space.
pub const SPRITE_FACING: u32 = 0;
/// Mode 1: it lies along its direction channel and turns about that to face the eye.
pub const SPRITE_AXIAL: u32 = 1;
/// Mode 2: it is square to its direction channel, whatever the eye.
pub const SPRITE_TURNED: u32 = 2;
/// A type-3, 4 or 9 block's direction channel, lerped by the progress through the window with
/// no exponent (`Effect.dll:0x1001077e`, the particle's slot 4).
pub const SPRITE_DIRECTION_AT: (usize, usize) = (76, 88);

/// What a sprite's mode makes of it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Through {
    /// Its own matrix in the world.
    Matrix([Vec3; 3]),
    /// The draw gives up: the eye is on the sprite, or a mode-1 sprite is seen straight down
    /// its direction (`Effect.dll:0x1000943e`, `0x100097a6`).
    Hidden,
    /// Not worked out: no eye was given to a mode that needs one, the frame has an axis of no
    /// length, or the mode is one no sprite block carries.
    Unturned,
}

/// A particle a stream left: when, from where, and how long it lives, in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    born: f32,
    frame: Frame,
    life: f32,
    /// The number drawn for it as it left (`Effect.dll:0x10011d84`), which its material's
    /// animation stands at for its whole life when the block's +32 is negative.
    drawn: f32,
    /// Its own far ends, drawn as it left: the block's position high end +100 and size high
    /// end +148, each axis moved by a uniform in ±half of the jitter 12 on, z first
    /// (`0x10011e81`–`0x10011f7f`).
    high: [f32; 3],
    size_high: [f32; 3],
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
    /// mode-14 effects are the burning trees' and wrecks' fires, which a load group makes
    /// switched off (header flag 0x40) for a machine's critical damage, block entry 6, to
    /// switch on, and that entry is not run (docs/13-control.md#critical-damage-block-entries-6-and-7--read-and-measured).
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
    /// The state the lights' jitter is drawn from. The game's light emitter draws from its
    /// translation unit's own state, `Effect.dll:0x10024c80`, apart from the instance's
    /// (`0x10024110`) and the streams', so a light's draws leave the others' as they were.
    light_rng: Rng,
    /// What each light emitter's last update drew, in the emitters' order, and when.
    light_jitter: Vec<LightJitter>,
    light_drawn_ms: Option<f64>,
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
    /// Seconds on the streams' clock, and where the effect was, at the last update.
    updated: Option<(f32, Vec3)>,
    /// What the streams' clock runs at against the instance's: 1 plays them as their blocks
    /// read. Their emission, their particles' life and so their rise all scale with it.
    pub stream_pace: f32,
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

/// A seed for an instance's light draws, apart from its own and its streams'.
fn light_seed(seed: u32) -> u32 {
    stream_seed(seed, 0x4c49_4748)
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
            light_rng: Rng::new(light_seed(seed)),
            light_jitter: Vec::new(),
            light_drawn_ms: None,
            id: 0,
            on,
            silent: false,
            heard_t: 0.0,
            looping: Vec::new(),
            sounding: Vec::new(),
            streams,
            updated: None,
            stream_pace: 1.0,
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
        self.draw_light_jitter(now_ms, t);
        let seconds = self.seconds(now_ms) * self.stream_pace;
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
                let jittered = |rng: &mut Rng, high: usize| {
                    let (end, spread) = (e.triple(high), e.triple(high + 12));
                    let z = end[2] + rng.spread(spread[2]);
                    let y = end[1] + rng.spread(spread[1]);
                    let x = end[0] + rng.spread(spread[0]);
                    [x, y, z]
                };
                let high = jittered(&mut self.rng, 100);
                let size_high = jittered(&mut self.rng, 148);
                stream.ring.push_back(Particle {
                    born: next,
                    frame,
                    life: ring as f32 * interval,
                    drawn,
                    high,
                    size_high,
                });
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

    /// A light emitter's update draws its jitter anew (`Effect.dll:0x1000f6e0`): while the
    /// light is on -- *t* inside its window, the manager's record active (`0x1000f75a`) -- it
    /// takes **five draws**, one for each channel of the colour and one for the range, each a
    /// uniform in ± half of its spread, in the order alpha, blue, green, red, range
    /// (`0x1000f8c7`, `0x1000f90a`, `0x1000f94d`, `0x1000f99f`, `0x1000fa8e`). The draws are
    /// taken whether the spread is 0 or not, and a light that is off takes none.
    ///
    /// The update runs when the manager updates the instance, which is once 100 ms have
    /// passed since its last (`0x100081a7`); this engine updates every instance on every
    /// tick, so the lights keep that interval themselves, and a light holds what it drew
    /// until the next.
    ///
    /// STAND-IN: docs/11-effects.md#the-generator--read-and-measured -- the game draws the
    /// alpha, blue, green and range from the light emitters' module-wide state `0x10024c80`
    /// and the red through the wrapper `0x10002680`, whose state `0x10023688` the bursts'
    /// spawns share; here all five come from the instance's own state for its lights.
    fn draw_light_jitter(&mut self, now_ms: f64, t: f32) {
        if self.light_drawn_ms.is_some_and(|then| now_ms - then < UPDATE_INTERVAL_MS) {
            return;
        }
        self.light_drawn_ms = Some(now_ms);
        let lights = self.effect.emitters.iter().filter(|e| e.kind == EMITTER_LIGHT);
        self.light_jitter.resize(lights.clone().count(), LightJitter::default());
        for (e, jitter) in lights.zip(&mut self.light_jitter) {
            if progress(e, t).is_none() {
                continue;
            }
            let spread = |at: usize| e.f(LIGHT_COLOUR_JITTER_AT + at * 4);
            let _alpha = self.light_rng.spread(spread(3));
            let blue = self.light_rng.spread(spread(2));
            let green = self.light_rng.spread(spread(1));
            let red = self.light_rng.spread(spread(0));
            let range = self.light_rng.spread(e.f(LIGHT_RANGE_JITTER_AT));
            *jitter = LightJitter { colour: Vec3::new(red, green, blue), range };
        }
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
        self.sprites_from(now_ms, None, in_view, out);
    }

    /// The point lights this instance's type-1 emitters drive at `now_ms`: each inside its
    /// window, its position, colour and range lerped by the progress through it, the position
    /// in the effect's frame and the range times the instance's scale (docs/11, "Type 1 is a
    /// light"), with what its last update drew added to the colour and the range.
    ///
    /// The range, its jitter added, is multiplied by the length of the light's own direction
    /// (+40 → +52) put through the instance's matrix, its 3 × 3 part (`Effect.dll:0x1000f884`,
    /// `0x1000f891`, `0x1000fb13`): the frame stretched by the instance's size times the
    /// header's scale (slot 0x20, `0x100047d0`), so an explosion's light grows with the
    /// explosion. A range that is then not above 0 becomes 0.01 (`0x1000fb7b`). The colour is
    /// handed on as it comes out, not held to any bound (`Terrain.dll:0x10080040`).
    pub fn lights(&self, now_ms: f64, out: &mut Vec<PointLight>) {
        if !self.on {
            return;
        }
        let t = self.t(now_ms);
        let lights = self.effect.emitters.iter().filter(|e| e.kind == EMITTER_LIGHT);
        for (k, e) in lights.enumerate() {
            let Some(p) = progress(e, t) else { continue };
            let kind =
                u32::from_le_bytes(e.body[LIGHT_KIND_AT..LIGHT_KIND_AT + 4].try_into().unwrap_or([0; 4]));
            let Some(flags) = point_light_flags(kind) else { continue };
            let jitter = self.light_jitter.get(k).copied().unwrap_or_default();
            let lerp = |a: f32, b: f32| a + (b - a) * p;
            let local = Vec3::from_array(e.triple(LIGHT_POSITION_AT.0))
                .lerp(Vec3::from_array(e.triple(LIGHT_POSITION_AT.1)), p);
            let colour = Vec3::from_array(e.triple(LIGHT_COLOUR_AT.0))
                .lerp(Vec3::from_array(e.triple(LIGHT_COLOUR_AT.1)), p)
                + jitter.colour;
            let direction = Vec3::from_array(e.triple(LIGHT_DIRECTION_AT.0))
                .lerp(Vec3::from_array(e.triple(LIGHT_DIRECTION_AT.1)), p);
            let [x, y, z] = self.frame.axes;
            let factor = (x * direction.x + y * direction.y + z * direction.z).length() * self.scale;
            let range = (lerp(e.f(LIGHT_RANGE_AT.0), e.f(LIGHT_RANGE_AT.1)) + jitter.range) * factor;
            let range = if range > 0.0 { range } else { LIGHT_LEAST_RANGE };
            out.push(PointLight {
                position: self.frame.point(local * self.scale),
                colour,
                range,
                flags,
                attenuation: e.triple(LIGHT_ATTENUATION_AT),
            });
        }
    }

    /// [`Self::sprites`] seen from `eye`, which a type-4 sprite sizes itself by.
    pub fn sprites_from(&self, now_ms: f64, eye: Option<Vec3>, in_view: bool, out: &mut Vec<Sprite>) {
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
                3 | 9 => self.sprite(e, p, eye, age_ms, seconds, out),
                4 => self.sprite(e, self.flare(e, p, eye), eye, age_ms, seconds, out),
                5 => self.bolt(e, p, age_ms, seconds, out),
                7 | 10 => self.burst(e, i as u32, p, age_ms, out),
                8 => self.stream(i, e, seconds * self.stream_pace, out),
                _ => {}
            }
            // A beacon's own emitters carry no bit 8, so its glow is depth-tested like any
            // other sprite: the pass argument its flag 0x800 waits for decides whether the
            // effect draws at all, never its depth state
            // ([11](../../../docs/11-effects.md#a-beacon-lights-glow--read-and-measured)).
            let overlay = in_view && e.word & EMITTER_FLAG != 0;
            let unfogged = self.effect.header.flags & FX_NO_FOG != 0;
            for s in &mut out[first..] {
                s.overlay = overlay;
                s.unfogged = unfogged;
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
    /// otherwise a rectangle facing the camera. A burst's and a stream's particles are drawn
    /// so; a type-3, 4 or 9 sprite is drawn with its own matrix instead
    /// ([`Instance::through`]), and falls back on this only where that cannot be worked.
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
            unfogged: false,
            lengthwise: false,
            age_ms,
            phase,
            dome: None,
            frame: frame.basis().map(|a| std::array::from_fn(|i| a[i] * size.to_array()[i] * self.scale)),
            matrix: None,
        }
    }

    /// A type-4 sprite's progress (`Effect.dll:0x100109b0`): the window's, times the eye's
    /// distance in the instance's own units over the block's `+200` (the constructor keeps its
    /// inverse, `0x100108d9`), and held at 1. Everything the sprite draws then runs on it --
    /// its place, size and fade, and its phase where `+8` is not negative -- so up close it
    /// stays near its low end and grows with the distance: the light glows keep their size on
    /// the screen. With no eye the window's progress stands.
    fn flare(&self, e: &Emitter, p: f32, eye: Option<Vec3>) -> f32 {
        let reach = e.f(FLARE_REACH_AT);
        match eye {
            Some(eye) if reach > 0.0 => (p * self.frame.local_distance(self.scale, eye) / reach).min(1.0),
            _ => p,
        }
    }

    /// A type 3, 4 or 9 sprite, fading +20 + (+24 − +20) × progress^+28 (`0x10010881`).
    ///
    /// It sits at lerp(+40, +52) in the frame and is lerp(+100, +112) in size along its
    /// axes, each axis by progress through the window raised to that axis's exponent —
    /// +64..+72 for the position, +124..+132 for the size (`0x100106f6`, `0x10010784`;
    /// docs/11, "A channel is a (low, high, jitter, exponent) run"). Its **phase**, from
    /// +8, +12 and +16, is where its material's animation stands ([`phase_fraction`]), and its
    /// direction, lerp(+76, +88) straight by the progress, is what its mode turns it by
    /// ([`Instance::through`]). A type-9 block draws a dome through the same matrix.
    fn sprite(
        &self,
        e: &Emitter,
        p: f32,
        eye: Option<Vec3>,
        age_ms: f32,
        seconds: f32,
        out: &mut Vec<Sprite>,
    ) {
        let local = lerp3_axis(e.triple(40), e.triple(52), shaped(p, e.triple(64)));
        let size = lerp3_axis(e.triple(100), e.triple(112), shaped(p, e.triple(124)));
        let direction = Vec3::from_array(e.triple(SPRITE_DIRECTION_AT.0))
            .lerp(Vec3::from_array(e.triple(SPRITE_DIRECTION_AT.1)), p);
        let phase = phase_fraction(e.f(8), e.f(12), e.f(16), p, seconds);
        let mut sprite =
            self.quad(e, &self.frame, local, size, fade(e.f(20), e.f(24), e.f(28), p), age_ms, phase);
        match self.through(word(e, SPRITE_MODE_AT), local, size, direction, eye) {
            Through::Matrix(m) => sprite.matrix = Some(m),
            Through::Hidden => return,
            Through::Unturned => {}
        }
        if e.kind == 9 {
            // A shape past the three the update knows draws nothing (`0x100138d7`).
            //
            // STAND-IN: docs/11-effects.md#type-9-is-a-half-sphere--read-measured-and-seen --
            // what gives an instance its level, which picks the dome's detail and drops it at
            // 4 and over, is not read: every dome is drawn at level 0, its finest.
            let Some(&(segments, rings)) = DOME_DETAIL.get(word(e, DOME_SHAPE_AT) as usize) else { return };
            sprite.dome = Some(Dome { segments, rings, projected: word(e, DOME_TILED_AT) == 0 });
        }
        out.push(sprite);
    }

    /// The matrix a type-3, 4 or 9 sprite is drawn with (`Effect.dll:0x100093d0`, the
    /// particle's draw): the basis its **mode** makes in the instance's own space, carried
    /// out through the instance's matrix with each of that matrix's axes scaled by the size
    /// channel (`0x1000d0c0` on the copy at `0x100098a9`; the product at `0x100098db`,
    /// `g_FastProc` slot `0x5c`, the instance's matrix on the left). So the size is laid along
    /// the **frame's** axes, x, y and z, whichever way the mode turns the sprite in it, and
    /// the sprite's place is the position channel put through the unscaled matrix
    /// (`0x1000989a`, `0x100098ea`).
    ///
    /// The basis's columns are where the sprite's own x, y and z go; a quad is the unit square
    /// of its xy plane, u with x and v with y (`Terrain.dll:0x10027a30`–`0x10027b88`), and a
    /// dome's pole is its z:
    ///
    /// | mode | x | y | z | |
    /// |---|---|---|---|---|
    /// | 0 | side of *v* | *v* × side | *v* | *v* the unit vector from the sprite to the eye (`0x100093f4`–`0x1000956b`) |
    /// | 1 | *d* | *w* | *d* × *w* | *w* the unit *v* × *d*, *v* not normalised; the frame about *d* where that is zero (`0x10009739`–`0x10009835`, `0x100038d0`) |
    /// | 2 | side of *d* | *d* × side | *d* | (`0x1000d110`) |
    ///
    /// with *d* the direction channel as it stands, not normalised, and the side [`side_of`].
    /// The eye is the camera in the instance's space ([`Frame::local`]), so a mode-0 sprite
    /// faces the camera **there** and comes back out stretched by the frame, and a mode-1
    /// sprite is a streak along its direction that turns about it. Mode 3 is a burst's, and
    /// no sprite block carries it (*measured*: 0 of the 2013).
    pub fn through(&self, mode: u32, local: Vec3, size: Vec3, direction: Vec3, eye: Option<Vec3>) -> Through {
        let axes = self.frame.axes.map(|a| a * self.scale);
        let view = || eye.and_then(|eye| self.frame.local(self.scale, eye)).map(|eye| eye - local);
        let d = direction;
        let basis = match mode {
            SPRITE_FACING => {
                let Some(v) = view() else { return Through::Unturned };
                let Some(v) = v.try_normalize() else { return Through::Hidden };
                let side = side_of(v);
                [side, v.cross(side), v]
            }
            SPRITE_AXIAL => {
                let Some(v) = view() else { return Through::Unturned };
                if v.dot(d).abs() == 1.0 {
                    return Through::Hidden;
                }
                match v.cross(d).try_normalize() {
                    Some(w) => [d, w, d.cross(w)],
                    None => {
                        let side = side_of(d);
                        [d, side, d.cross(side)]
                    }
                }
            }
            SPRITE_TURNED => {
                let side = side_of(d);
                [side, d.cross(side), d]
            }
            _ => return Through::Unturned,
        };
        if axes.iter().any(|a| a.length_squared() <= 0.0) {
            return Through::Unturned;
        }
        Through::Matrix(
            basis.map(|b| axes[0] * (size.x * b.x) + axes[1] * (size.y * b.y) + axes[2] * (size.z * b.z)),
        )
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
                unfogged: false,
                lengthwise: true,
                age_ms,
                phase,
                dome: None,
                frame: None,
                matrix: None,
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
    /// It sits at lerp(+88, its own +100) from where it left and is lerp(+136, its own +148)
    /// in size, each axis by its age raised to that axis's exponent — +124..+132 for the
    /// position, +172..+180 for the size (`0x100121f8`, `0x10012276`); its own high ends are
    /// the block's jittered as it left.
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
            let local = lerp3_axis(e.triple(88), q.high, shaped(age, e.triple(124)));
            let size = lerp3_axis(e.triple(136), q.size_high, shaped(age, e.triple(172)));
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
        // An orientation about the direction, scaled alike by its length (`0x10002c7b`): the
        // direction first, its side second, and direction × side third. This one has no x, so
        // its side is the x axis.
        let [x, y, z] = frame.axes;
        let length = point.1.length();
        assert!((x - point.1).length() < 1e-5, "{x:?}");
        assert!((y - Vec3::X * length).length() < 1e-5, "{y:?}");
        assert!((z - point.1.cross(Vec3::X)).length() < 1e-5, "{z:?}");
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![glow]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let fx = Instance::new(whole, frame, 1.0, 0.0, None, 1);
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert_eq!(out[0].frame, None);
        assert_eq!(out[0].along, Vec3::ZERO);
        assert!((out[0].centre - point.0).length() < 1e-6);
    }

    /// `Control.dll:0x10003ef0` and its twins square a direction with its horizontal
    /// perpendicular, and with a fixed axis where the direction has no x or no y.
    #[test]
    fn a_direction_is_squared_with_its_horizontal_perpendicular_and_with_an_axis_on_the_axes() {
        // The general case, normalised where the direction has a z.
        let d = Vec3::new(0.6, 0.0, 0.8).normalize();
        assert_eq!(side_of(d), Vec3::Y, "no y: the y axis");
        let d = Vec3::new(0.48, 0.64, 0.6);
        assert!((side_of(d) - Vec3::new(-0.8, 0.6, 0.0)).length() < 1e-6);
        // With no z the perpendicular is taken as it is: a unit direction's is unit already.
        assert_eq!(side_of(Vec3::new(0.6, 0.8, 0.0)), Vec3::new(-0.8, 0.6, 0.0));
        assert_eq!(side_of(Vec3::new(3.0, 4.0, 0.0)), Vec3::new(-4.0, 3.0, 0.0), "not normalised");
        // On the axes the sign the perpendicular would have carried is dropped.
        for d in [Vec3::Z, Vec3::NEG_Z, Vec3::Y, Vec3::NEG_Y, Vec3::new(0.0, 0.6, 0.8)] {
            assert_eq!(side_of(d), Vec3::X, "{d}: no x, the x axis");
        }
        for d in [Vec3::X, Vec3::NEG_X, Vec3::new(-0.6, 0.0, 0.8)] {
            assert_eq!(side_of(d), Vec3::Y, "{d}: no y, the y axis");
        }
        // So a frame about a direction is the direction, its side, and direction × side.
        let up = Frame::along(Vec3::ZERO, Vec3::Z, 2.0);
        assert_eq!(up.axes, [Vec3::Z * 2.0, Vec3::X * 2.0, Vec3::Y * 2.0]);
        let down = Frame::along(Vec3::ZERO, Vec3::NEG_Z, 1.0);
        assert_eq!(down.axes, [Vec3::NEG_Z, Vec3::X, Vec3::NEG_Y]);
        let back = Frame::along(Vec3::ZERO, Vec3::NEG_X, 1.0);
        assert_eq!(back.axes, [Vec3::NEG_X, Vec3::Y, Vec3::NEG_Z]);
    }

    /// A point of the world in an instance's space is projected on each axis and divided by
    /// that axis's squared length (`Ngi32.dll:0x100248f0`), and the instance's scale is in
    /// its matrix (`Effect.dll:0x10007c90`).
    #[test]
    fn the_eye_in_an_instances_space_is_projected_on_its_axes_over_their_squared_lengths() {
        let frame = Frame {
            origin: Vec3::new(10.0, 0.0, 0.0),
            axes: [Vec3::Z * 4.0, Vec3::X * 2.0, Vec3::Y],
            points: true,
        };
        let local = frame.local(1.0, Vec3::new(16.0, 3.0, 8.0)).unwrap();
        assert!((local - Vec3::new(2.0, 3.0, 3.0)).length() < 1e-6, "{local}");
        // Twice the scale, half the coordinates.
        let half = frame.local(2.0, Vec3::new(16.0, 3.0, 8.0)).unwrap();
        assert!((half - local / 2.0).length() < 1e-6);
        assert!((frame.local_distance(2.0, Vec3::new(16.0, 3.0, 8.0)) - local.length() / 2.0).abs() < 1e-6);
        let flat = Frame { axes: [Vec3::X, Vec3::Y, Vec3::ZERO], ..frame };
        assert_eq!(flat.local(1.0, Vec3::ONE), None, "an axis of no length");
    }

    /// A type-3 block with its mode at `+4`, its direction channel and its size, opaque all
    /// through its window, at the frame's origin.
    fn moded(kind: u8, mode: u32, direction: Vec3, size: Vec3) -> Emitter {
        let e = block(kind, 208, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (28, 1.0)], "M");
        let e = with_word(e, SPRITE_MODE_AT, mode);
        [(76, direction), (88, direction), (100, size), (112, size)]
            .into_iter()
            .fold(e, |e, (at, v)| (0..3).fold(e, |e, i| with_word(e, at + 4 * i, v[i].to_bits())))
    }

    fn drawn(blocks: Vec<Emitter>, frame: Frame, eye: Option<Vec3>) -> Vec<Sprite> {
        let mut whole = effect(TIME_MANUAL, 0.0, 0, blocks);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let mut fx = Instance::new(whole, frame, 1.0, 0.0, None, 1);
        fx.value = 0.5;
        let mut out = Vec::new();
        fx.sprites_from(0.0, eye, true, &mut out);
        out
    }

    /// Mode 2 (`Effect.dll:0x1000d110`): the sprite's z is its direction channel, whatever the
    /// eye, and its size is laid along the frame's axes.
    #[test]
    fn a_mode_2_sprite_is_square_to_its_direction_and_sized_along_its_frames_axes() {
        // A frame about straight up, 10 long: depth z, then x, then y.
        let frame = Frame::along(Vec3::new(0.0, 0.0, 5.0), Vec3::Z, 10.0);
        let size = Vec3::new(1.0, 2.0, 3.0);
        let out = drawn(vec![moded(3, SPRITE_TURNED, Vec3::X, size)], frame, None);
        let m = out[0].matrix.expect("mode 2 needs no eye");
        // Its direction is the frame's depth, world z: the quad lies flat, 20 by 30.
        assert!((m[0] - Vec3::X * 20.0).length() < 1e-4, "{:?}", m[0]);
        assert!((m[1] - Vec3::Y * 30.0).length() < 1e-4, "{:?}", m[1]);
        assert!((m[2] - Vec3::Z * 10.0).length() < 1e-4, "{:?}", m[2]);
        // Turned to the frame's third axis it stands up, and keeps the frame's sizes.
        let out = drawn(vec![moded(3, SPRITE_TURNED, Vec3::Z, size)], frame, Some(Vec3::new(50.0, 0.0, 5.0)));
        let m = out[0].matrix.unwrap();
        assert!((m[2] - Vec3::Y * 30.0).length() < 1e-4, "its z is the frame's third axis: {:?}", m[2]);
        assert!((m[0] - Vec3::Z * 10.0).length() < 1e-4 && (m[1] - Vec3::X * 20.0).length() < 1e-4);
    }

    /// Mode 1 (`0x10009739`): the sprite's x is its direction, its y the unit view × direction.
    #[test]
    fn a_mode_1_sprite_lies_along_its_direction_and_turns_about_it_to_the_eye() {
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let streak = moded(3, SPRITE_AXIAL, Vec3::X, Vec3::new(8.0, 2.0, 2.0));
        // Seen from above: 8 along the frame's depth, 2 across, flat on the ground.
        let above = drawn(vec![streak.clone()], frame, Some(Vec3::new(0.0, 0.0, 30.0)));
        let m = above[0].matrix.unwrap();
        assert!((m[0] - Vec3::X * 8.0).length() < 1e-5, "{:?}", m[0]);
        assert!((m[1].length() - 2.0).abs() < 1e-5 && m[1].z.abs() < 1e-5, "{:?}", m[1]);
        // Seen from the side it stands on edge: its width runs up.
        let side = drawn(vec![streak.clone()], frame, Some(Vec3::new(0.0, -30.0, 0.0)));
        let m = side[0].matrix.unwrap();
        assert!((m[0] - Vec3::X * 8.0).length() < 1e-5 && (m[1].z.abs() - 2.0).abs() < 1e-5, "{m:?}");
        // A direction along the frame's third axis lays the streak there.
        let up = moded(3, SPRITE_AXIAL, Vec3::Z, Vec3::new(1.0, 1.0, 9.0));
        let m = drawn(vec![up], frame, Some(Vec3::new(0.0, -30.0, 0.0)))[0].matrix.unwrap();
        assert!((m[0] - frame.axes[2] * 9.0).length() < 1e-5, "{:?}", m[0]);
        // With no eye the mode cannot be worked, and the old quad stands.
        assert_eq!(drawn(vec![streak], frame, None)[0].matrix, None);
    }

    /// Mode 0 (`0x100093f4`): the sprite's z is the unit vector to the eye **in the frame's
    /// space**, so a frame with unequal axes stretches the square.
    #[test]
    fn a_mode_0_sprite_faces_the_eye_in_its_frames_space() {
        let even = Frame::along(Vec3::ZERO, Vec3::X, 2.0);
        let glow = moded(3, SPRITE_FACING, Vec3::X, Vec3::splat(3.0));
        let eye = Vec3::new(40.0, 30.0, 0.0);
        let m = drawn(vec![glow.clone()], even, Some(eye))[0].matrix.unwrap();
        assert!((m[2].normalize() - eye.normalize()).length() < 1e-5, "it faces the eye: {:?}", m[2]);
        assert!(m.iter().all(|a| (a.length() - 6.0).abs() < 1e-4), "a 6 m square: {m:?}");
        assert!(m[0].dot(m[1]).abs() < 1e-4 && m[0].dot(m[2]).abs() < 1e-4);
        // The energy bridge's frame, seen across the span: 150 m long and 0.414 high.
        let at = Vec3::ZERO;
        let bridge = Frame::from_points([
            (at, Vec3::new(0.0, 150.0, 0.0)),
            (at, Vec3::new(1.932, 0.0, 0.0)),
            (at, Vec3::new(0.0, 0.0, -0.414)),
        ]);
        let unit = moded(3, SPRITE_FACING, Vec3::X, Vec3::ONE);
        let m = drawn(vec![unit.clone()], bridge, Some(Vec3::new(60.0, 0.0, 0.0)))[0].matrix.unwrap();
        let (a, b) = (m[0].length(), m[1].length());
        assert!((a.max(b) - 150.0).abs() < 1e-3 && (a.min(b) - 0.414).abs() < 1e-3, "{a} by {b}");
        // The eye on the sprite: the draw gives up.
        assert!(drawn(vec![unit], bridge, Some(at)).is_empty());
    }

    /// `f_gener_ball` on `fr_l_gener`'s `Sign_Type1`, whose direction is (0, 0, 8.98): two
    /// type-9 blocks alike but for their direction channels, (−1, 0, 0) and (1, 0, 0), sized
    /// (0.3, 0.4, 0.4) under a header scale of 0.75. Each is a half sphere with its pole on its
    /// direction, so the pair is a whole ball, 2.02 m from centre to pole and 2.69 to its rim.
    #[test]
    fn two_type_9_blocks_with_opposite_directions_make_a_ball() {
        let point = (Vec3::new(0.0, 0.0, 30.8), Vec3::new(0.0, 0.0, 8.984_529));
        let size = Vec3::new(0.3, 0.4, 0.4);
        let halves = vec![moded(9, SPRITE_TURNED, Vec3::NEG_X, size), moded(9, SPRITE_TURNED, Vec3::X, size)];
        let mut whole = effect(TIME_MANUAL, 0.0, 0, halves);
        Rc::get_mut(&mut whole).unwrap().header.scale = [0.75; 3];
        let mut fx = Instance::new(whole, Frame::from_points([point; 3]), 1.0, 0.0, None, 1);
        fx.value = 0.5;
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert_eq!(out.len(), 2);
        let (down, up) = (out[0].matrix.unwrap(), out[1].matrix.unwrap());
        let pole = 0.3 * 8.984_529 * 0.75;
        assert!((down[2] - Vec3::NEG_Z * pole).length() < 1e-4, "the first half hangs: {:?}", down[2]);
        assert!((up[2] - Vec3::Z * pole).length() < 1e-4, "the second stands: {:?}", up[2]);
        let rim = 0.4 * 8.984_529 * 0.75;
        for m in [down, up] {
            assert!((m[0].length() - rim).abs() < 1e-4 && (m[1].length() - rim).abs() < 1e-4, "{m:?}");
            assert!(m[0].z.abs() < 1e-5 && m[1].z.abs() < 1e-5, "the rims share the level plane");
        }
        for s in &out {
            assert_eq!(s.dome, Some(Dome { segments: 8, rings: 3, projected: true }));
            assert!((s.centre - point.0).length() < 1e-4);
        }
        // A shape past the three the update knows draws nothing; `+204` set gives every facet
        // the whole texture.
        let odd = with_word(moded(9, SPRITE_TURNED, Vec3::X, size), DOME_SHAPE_AT, 3);
        let tiled =
            with_word(with_word(moded(9, SPRITE_TURNED, Vec3::X, size), DOME_SHAPE_AT, 1), DOME_TILED_AT, 1);
        let out = drawn(vec![odd, tiled], Frame::from_points([point; 3]), None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].dome, Some(Dome { segments: 16, rings: 6, projected: false }));
    }

    /// A light 3 over a big floor triangle draws the template's square, half-side
    /// sqrt(5² − 3²) = 4, about its foot, (u, v) 0.495 at the foot; near an edge the square is
    /// cut to the triangle, and past the range from the plane or an edge it draws nothing.
    #[test]
    fn a_point_light_draws_its_square_on_a_triangle_clipped_to_it() {
        let floor =
            [Vec3::new(-100.0, -100.0, 0.0), Vec3::new(100.0, -100.0, 0.0), Vec3::new(0.0, 100.0, 0.0)];
        let disc = light_disc(floor, Vec3::new(0.0, 0.0, 3.0), 5.0);
        assert_eq!(disc.len(), 4, "{disc:?}");
        for (p, _) in &disc {
            assert!(
                (p.z).abs() < 1e-5 && (p.x.abs() - 4.0).abs() < 1e-4 && (p.y.abs() - 4.0).abs() < 1e-4,
                "{p:?}"
            );
        }
        // The first edge runs along y = −100, its outward normal −y: the first corner is on
        // that side, and (u, v) are 0 there.
        assert!(disc[0].0.y < 0.0 && disc[0].1 == [0.0, 0.0]);
        // From below the plane it lights the same.
        assert_eq!(light_disc(floor, Vec3::new(0.0, 0.0, -3.0), 5.0).len(), 4);
        assert!(light_disc(floor, Vec3::new(0.0, 0.0, 6.0), 5.0).is_empty(), "past the range from the plane");
        assert!(
            light_disc(floor, Vec3::new(0.0, -106.0, 1.0), 5.0).is_empty(),
            "past the range from an edge"
        );
        // Over the first edge the square is cut there: every corner within the triangle, and
        // the cut corners' v interpolated.
        let cut = light_disc(floor, Vec3::new(0.0, -101.0, 3.0), 5.0);
        assert!(!cut.is_empty() && cut.iter().all(|(p, _)| p.y >= -100.0 - 1e-4), "{cut:?}");
        assert!(cut.iter().any(|(_, uv)| uv[0] > 0.0 && uv[0] < 0.99));
    }

    /// `hero_cannon`'s light: kind 5, a point light the shade emulates, its range lerped 30 → 3
    /// and its colour (3, 2, 0.05) → (0.5, 0.3, 0.01) over the window; a kind-7 light is kept
    /// with the flag that keeps the shade from drawing it.
    #[test]
    fn an_effects_light_is_where_its_window_puts_it() {
        let words = |kind: u32, pairs: &[(usize, f32)]| {
            let mut e = block(1, 224, &[(8, 0.0), (12, 1.0)], "");
            e.body[4..8].copy_from_slice(&kind.to_le_bytes());
            pairs.iter().fold(e, |e, &(at, v)| with_word(e, at, v.to_bits()))
        };
        let flash = words(
            5,
            &[
                (64, 3.0),
                (68, 2.0),
                (72, 0.05),
                (80, 0.5),
                (84, 0.3),
                (88, 0.01),
                (112, 30.0),
                (116, 3.0),
                (16, 1.0),
                (28, 1.0),
                (40, 1.0),
                (52, 1.0),
            ],
        );
        let lamp = words(7, &[(112, 7.0), (116, 7.0), (40, 1.0), (52, 1.0)]);
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![flash, lamp]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let mut fx =
            Instance::new(whole, Frame::along(Vec3::new(10.0, 0.0, 0.0), Vec3::X, 1.0), 1.0, 0.0, None, 1);
        fx.value = 0.5;
        let mut out = Vec::new();
        fx.lights(0.0, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0].range - 16.5).abs() < 1e-4 && (out[0].colour.x - 1.75).abs() < 1e-5);
        assert!((out[0].position - Vec3::new(11.0, 0.0, 0.0)).length() < 1e-5, "{:?}", out[0].position);
        assert_eq!((out[0].flags, out[1].flags), (0, LIGHT_NOT_EMULATED));
    }

    /// A light block of `kind` on over the window `low`..`high`, pointing along x, with
    /// `floats` over the rest.
    fn light(kind: u32, (low, high): (f32, f32), floats: &[(usize, f32)]) -> Emitter {
        let mut e = block(1, 224, &[(8, low), (12, high), (40, 1.0), (52, 1.0)], "");
        e.body[4..8].copy_from_slice(&kind.to_le_bytes());
        floats.iter().fold(e, |e, &(at, v)| with_word(e, at, v.to_bits()))
    }

    fn lit(fx: &Instance, now_ms: f64) -> Vec<PointLight> {
        let mut out = Vec::new();
        fx.lights(now_ms, &mut out);
        out
    }

    /// The construction sphere's light, (5, 5, 10) with a colour jitter of (0.3, 0.3, 0.6) and
    /// -- an explosion's -- a range jitter: an update of a light that is on takes five draws
    /// from the lights' state, alpha, blue, green, red, range, each a uniform in ± half of its
    /// spread (`Effect.dll:0x1000f8c7`–`0x1000faf7`), adds the three colours and the range's to
    /// what the window gives, and holds them until 100 ms have passed (`0x100081a7`). A light
    /// outside its window takes no draw (`0x1000f75a`).
    #[test]
    fn a_lights_jitter_is_five_draws_an_update_and_holds_for_100_ms() {
        let colour = [(64, 5.0), (68, 5.0), (72, 10.0), (80, 5.0), (84, 5.0), (88, 10.0)];
        let spread = [(96, 0.3), (100, 0.3), (104, 0.6), (112, 10.0), (116, 10.0), (120, 2.0)];
        let on = light(5, (0.0, 1.0), &[&colour[..], &spread[..]].concat());
        let off = light(5, (0.6, 1.0), &[&colour[..], &spread[..]].concat());
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![off, on]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let mut fx = Instance::new(whole, Frame::along(Vec3::ZERO, Vec3::X, 1.0), 1.0, 0.0, None, 7);
        fx.value = 0.5;
        let still = lit(&fx, 0.0);
        assert_eq!(still.len(), 1, "the first light is outside its window");
        assert_eq!(
            (still[0].colour, still[0].range),
            (Vec3::new(5.0, 5.0, 10.0), 10.0),
            "no update, no draw"
        );

        let mut state = Rng::new(light_seed(7));
        let mut drawn = || {
            let _alpha = state.spread(0.0);
            let blue = state.spread(0.6);
            let green = state.spread(0.3);
            let red = state.spread(0.3);
            (Vec3::new(5.0 + red, 5.0 + green, 10.0 + blue), 10.0 + state.spread(2.0))
        };
        let first = drawn();
        fx.update(0.0);
        let a = lit(&fx, 0.0)[0];
        assert_eq!((a.colour, a.range), first, "the lit light takes the state's first five draws");
        assert!(a.colour != Vec3::new(5.0, 5.0, 10.0) && a.range != 10.0);
        assert!((a.colour - Vec3::new(5.0, 5.0, 10.0)).abs().cmple(Vec3::new(0.15, 0.15, 0.3)).all());
        assert!((a.range - 10.0).abs() <= 1.0);
        assert_ne!(a.colour.x - 5.0, a.colour.y - 5.0, "a draw for each channel, not one for all");

        fx.update(50.0);
        let held = lit(&fx, 50.0)[0];
        assert_eq!((held.colour, held.range), first, "held until the next update, 100 ms on");
        fx.update(100.0);
        let b = lit(&fx, 100.0)[0];
        assert_eq!((b.colour, b.range), drawn(), "then the next five");
        assert_ne!(b.colour, a.colour);
    }

    /// The range's jitter goes on before the frame's stretch, and a range that comes out not
    /// above 0 becomes 0.01 (`Effect.dll:0x1000faf7`–`0x1000fb8c`): a small range is kept as
    /// it is, where an earlier reading here held it to at least 0.01 before the stretch.
    #[test]
    fn a_range_not_above_0_becomes_a_hundredth() {
        let none = light(5, (0.0, 1.0), &[(112, 0.0), (116, 0.0)]);
        let small = light(5, (0.0, 1.0), &[(112, 0.004), (116, 0.004)]);
        let below = light(5, (0.0, 1.0), &[(112, -3.0), (116, -3.0)]);
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![none, small, below]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let fx = Instance::new(whole, Frame::along(Vec3::ZERO, Vec3::X, 2.0), 1.0, 0.0, None, 1);
        let ranges: Vec<f32> = lit(&fx, 0.0).iter().map(|l| l.range).collect();
        assert_eq!(ranges, [LIGHT_LEAST_RANGE, 0.008, LIGHT_LEAST_RANGE]);
    }

    /// `mineglow`'s light, blue (0, 0, 0.9) with the terms (0, 1, 1): the shade's lighter
    /// takes x + x² of it, x the share of the range left (`Ngi32.dll:0x10016678`), so it starts
    /// near twice the colour and is nothing at its range and past it; a vertex turned away
    /// takes nothing, and a gun's (0, 1, 0) falls in a straight line.
    #[test]
    fn a_light_falls_by_the_share_of_its_range_left_and_stops_there() {
        let glow =
            light(7, (0.0, 1.0), &[(72, 0.9), (88, 0.9), (112, 7.0), (116, 7.0), (128, 1.0), (132, 1.0)]);
        let flash = light(5, (0.0, 1.0), &[(64, 3.0), (80, 3.0), (112, 30.0), (116, 30.0), (128, 1.0)]);
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![glow, flash]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let fx =
            Instance::new(whole, Frame::along(Vec3::new(0.0, 0.0, 3.0), Vec3::X, 1.0), 1.0, 0.0, None, 1);
        let out = lit(&fx, 0.0);
        assert_eq!((out[0].attenuation, out[1].attenuation), ([0.0, 1.0, 1.0], [0.0, 1.0, 0.0]));
        // 3.5 below the light, half its range: x = 0.5, so 0.5 + 0.25 of the colour.
        let under = out[0].diffuse_at(Vec3::new(0.0, 0.0, -0.5), Vec3::Z);
        assert!((under - Vec3::new(0.0, 0.0, 0.9 * 0.75)).length() < 1e-6, "{under}");
        let close = out[0].diffuse_at(Vec3::new(0.0, 0.0, 2.93), Vec3::Z);
        assert!((close.z - 0.9 * (0.99 + 0.99 * 0.99)).abs() < 1e-4, "near twice the colour: {close}");
        assert_eq!(out[0].diffuse_at(Vec3::ZERO, Vec3::NEG_Z), Vec3::ZERO, "turned away");
        assert_eq!(out[0].diffuse_at(Vec3::new(7.0, 0.0, 3.0), Vec3::NEG_X), Vec3::ZERO, "at its range");
        assert_eq!(out[0].diffuse_at(Vec3::new(7.5, 0.0, 3.0), Vec3::NEG_X), Vec3::ZERO, "past it");
        // The flash, 15 from a face square to it: half of its range of 30 left.
        let lit = out[1].diffuse_at(Vec3::new(0.0, 0.0, -12.0), Vec3::Z);
        assert!((lit.x - 1.5).abs() < 1e-5, "{lit}");
    }

    /// A building's light glows (`glow_y` and its kin) grow 0.2 → 9 over their window, and a
    /// type-4 sprite runs that on the window's progress times the eye's distance in the frame's
    /// units over its `+200`, 500 (`Effect.dll:0x100109b0`): from 50 units it is 1.08 across,
    /// from 1000 its full 9. C03 M02's generator ball, read as a type 3, drew 60 m wide.
    #[test]
    fn a_type_4_glow_grows_with_the_eyes_distance_in_its_frame() {
        let glow = block(4, 204, &[(32, 0.0), (36, 1.0), (20, 1.0), (24, 1.0), (28, 1.0), (200, 500.0)], "G");
        let glow = [(100, 0.2), (104, 0.2), (108, 0.2), (112, 9.0), (116, 9.0), (120, 9.0)]
            .into_iter()
            .fold(glow, |e, (at, v): (usize, f32)| with_word(e, at, v.to_bits()));
        // One control point named three times, its direction 2 long: distances in its frame halve.
        let point = (Vec3::new(0.0, 0.0, 30.0), Vec3::new(0.0, 0.0, 2.0));
        let mut whole = effect(TIME_MANUAL, 0.0, 0, vec![glow]);
        Rc::get_mut(&mut whole).unwrap().header.scale = [1.0; 3];
        let mut fx = Instance::new(whole, Frame::from_points([point; 3]), 1.0, 0.0, None, 1);
        fx.value = 1.0;
        let width = |eye: Vec3| {
            let mut out = Vec::new();
            fx.sprites_from(0.0, Some(eye), true, &mut out);
            out[0].width
        };
        let near = width(Vec3::new(100.0, 0.0, 30.0));
        assert!((near - 1.08 * 2.0).abs() < 1e-3, "50 frame units off: {near}");
        assert!((width(Vec3::new(2000.0, 0.0, 30.0)) - 9.0 * 2.0).abs() < 1e-3, "past 500 it is whole");
        let mut out = Vec::new();
        fx.sprites(0.0, true, &mut out);
        assert!((out[0].width - 18.0).abs() < 1e-3, "with no eye the window's progress stands");
        // The instance's scale is in its matrix, so it divides the distance as the frame's
        // length does: asked for at half the size, the eye is twice as far in its units.
        fx.scale = 0.5;
        let mut out = Vec::new();
        fx.sprites_from(0.0, Some(Vec3::new(100.0, 0.0, 30.0)), true, &mut out);
        let size = 0.2 + 8.8 * (100.0 / 500.0);
        assert!((out[0].width - size * 2.0 * 0.5).abs() < 1e-3, "{}", out[0].width);
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

    /// Each stream particle draws its own far end and size as it leaves: a uniform in ±half
    /// of the jitter triples at +112 and +160, added to the high ends at +100 and +148, z
    /// first (`Effect.dll:0x10011e81`–`0x10011f7f`). So two particles of the same age stand
    /// apart, and a plume breaks into puffs.
    #[test]
    fn a_stream_particle_draws_its_own_far_end_and_size_as_it_leaves() {
        let floats = [
            (16, 0.0),
            (20, 1.0),
            (24, 0.1),
            (28, 0.1),
            (4, 1.0),
            (8, 1.0),
            (108, 100.0),
            (112, 20.0),
            (116, 20.0),
            (120, 60.0),
            (136, 12.0),
            (140, 12.0),
            (144, 12.0),
            (148, 80.0),
            (152, 80.0),
            (156, 80.0),
            (160, 4.0),
            (164, 4.0),
            (168, 4.0),
        ];
        let plume = with_word(block(8, 248, &floats, "smoke"), 36, 10);
        let frame = Frame::along(Vec3::ZERO, Vec3::X, 1.0);
        let mut fx = Instance::new(effect(TIME_MANUAL, 0.0, 0, vec![plume]), frame, 10.0, 0.0, None, 7);
        fx.value = 0.5;
        // Ten particles, 0.1 s apart; at 1 s each has aged by its place in the ring.
        for ms in (0..=1000).step_by(50) {
            fx.update(f64::from(ms));
        }
        let mut out = Vec::new();
        fx.sprites(1000.0, false, &mut out);
        assert_eq!(out.len(), fx.streams[0].ring.len());
        assert!(out.len() >= 8, "{}", out.len());
        // Each particle's place over its age is its own far end: x and y within ±10, z
        // within 100 ± 30. A particle only just out is left out, its age too near 0.
        let ends: Vec<Vec3> = fx.streams[0]
            .ring
            .iter()
            .zip(&out)
            .map(|(q, s)| (1.0 - q.born, s.centre))
            .filter(|(age, _)| *age > 0.2)
            .map(|(age, centre)| centre / age)
            .collect();
        assert!(ends.len() >= 6, "{ends:?}");
        assert!(ends.iter().all(|e| e.x.abs() <= 10.0 + 1e-2 && e.y.abs() <= 10.0 + 1e-2), "{ends:?}");
        assert!(ends.iter().all(|e| (e.z - 100.0).abs() <= 30.0 + 1e-2), "{ends:?}");
        let spread = |f: fn(&Vec3) -> f32| {
            let v: Vec<f32> = ends.iter().map(f).collect();
            v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert!(spread(|e| e.x) > 2.0 && spread(|e| e.z) > 5.0, "every particle on one line: {ends:?}");
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
