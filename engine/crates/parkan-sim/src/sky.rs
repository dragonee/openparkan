//! The sky at a moment: `docs/10-sky.md`, "The clock: sections in turn, and where it
//! starts", "Events", and "The dome, the fog and the scene colour".
//!
//! A file's sections play one after another as one cycle, and the clock starts at the
//! file's closing time. The atmosphere lerps the two keyframes of the current section
//! around its clock, colours channel by channel and floats linearly; the sky takes the
//! horizon, rings and apex for the dome, 700 × slots 5 and 6 for the fog, and slot 20
//! as the scene colour; the sun object takes slots 19, 17 and 21 for its two lights.
//! A body is up from its start opcode to the first stop at or after it.
//!
//! The port of the clock, events and flare gates in `openparkan/sky.py`.

use std::f32::consts::{PI, TAU};

use glam::Vec3;
use parkan_formats::sky::{
    APEX_SLOT, Atmosphere, FOG_END_SLOT, FOG_SCALE, FOG_START_SLOT, HORIZON_SLOTS, Keyframe, RING2_SLOTS,
    RING3_SLOTS, SCENE_COLOUR_SLOT, SUN_LIGHT_SLOT,
};

/// The dome's parameter block (`0x1006f1ba`): height, cap angle, rings, and 2 to the
/// `AtmSkyDetail` 4 segments.
pub const DOME_HEIGHT: f32 = 10_000.0;
pub const DOME_ANGLE: f32 = PI / 4.0;
pub const DOME_RINGS: usize = 5;
pub const DOME_SEGMENTS: usize = 16;

/// Where the sun and the moon stand **at the top of their arc**: `CSun`'s constant
/// azimuth and tilt as a direction, `(sin A sin B, −cos A sin B, cos B)`
/// (`docs/10-sky.md`, "Where the sun stands").
pub const SUN_DIRECTION: Vec3 = Vec3::new(0.5, 0.0, 0.866_025_4);
pub const MOON_DIRECTION: Vec3 = Vec3::new(0.0, -0.766_044_4, 0.642_787_6);

/// A body travels an arc of `(SPAN × progress + START) × π` through its lifetime
/// (`Terrain.dll:0x1007ed40`), so it rises 18° below one horizon and sets 18° below the
/// other, standing at its constant azimuth and tilt halfway through.
pub const BODY_ARC_START: f32 = -0.1;
pub const BODY_ARC_SPAN: f32 = 1.2;

/// The sun object's second directional light's colour (`0x1007ed34`).
pub const SUN_SECOND_LIGHT_SLOT: usize = 21;
/// Its alpha scales how far the flare gates lift the main light (`0x1007ea14`).
pub const SUN_BOOST_SLOT: usize = 17;
/// The third float: the main light is slot 19 × this (`0x1006ac9a`).
pub const LIGHT_FLOAT: usize = 2;

/// Seconds in the 24-hour clock a keyframe's stamp is scaled against.
pub const CLOCK_DAY: u64 = 86_400;

/// The flare is out once the body is this far off the view axis (`docs/10-sky.md`,
/// "The lens flare").
pub const FLARE_CONE_DEGREES: f32 = 15.0;
/// The second gate ramps on the body's height between `cos 60°` and `cos 30°`; the
/// sun reaches exactly the top edge at the top of its arc.
pub const FLARE_HEIGHT_ZERO: f32 = 0.5;
pub const FLARE_HEIGHT_FULL: f32 = SUN_DIRECTION.z;
/// How far the gates lift the main light: from its colour `c` to `5c`.
pub const FLARE_LIGHT_BOOST: f32 = 5.0;

type Rgb = [f32; 3];

/// A place on the atmosphere's clock: a section, and seconds into its day.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub section: usize,
    pub seconds: f64,
}

impl Position {
    fn before(&self, other: &Position) -> bool {
        (self.section, self.seconds) < (other.section, other.seconds)
    }
}

/// The objects an event starts and stops (`docs/10-sky.md`, "The ten opcodes").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sun,
    Rain,
    Snow,
    Lightning,
}

/// `GetEvents`' switch (`0x1006e829`): an opcode as whether it starts its object, and
/// which. 2 and 7 share the default and do nothing.
pub fn event_of(opcode: u32) -> Option<(bool, Kind)> {
    Some(match opcode {
        0 => (true, Kind::Sun),
        1 => (false, Kind::Sun),
        3 => (true, Kind::Rain),
        4 => (false, Kind::Rain),
        5 => (true, Kind::Snow),
        6 => (false, Kind::Snow),
        8 => (true, Kind::Lightning),
        9 => (false, Kind::Lightning),
        _ => return None,
    })
}

/// One event of the cycle, where the clock passes its keyframe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event<'a> {
    pub position: Position,
    pub start: bool,
    pub kind: Kind,
    pub keyframe: &'a Keyframe,
}

/// Which body a start-`SUN` event makes: `GetEvents` compares the keyframe's first name
/// with the literal `"sun"`, and anything else is the moon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Sun,
    Moon,
}

impl Body {
    pub fn named(name: &str) -> Body {
        if name == "sun" { Body::Sun } else { Body::Moon }
    }

    /// Its azimuth and its tilt from the zenith, the two constants `GetEvents` picks by
    /// the keyframe's name (`docs/10-sky.md`, "Where the sun stands").
    pub fn angles(self) -> (f32, f32) {
        match self {
            Body::Sun => (90f32.to_radians(), 30f32.to_radians()),
            Body::Moon => (0.0, 50f32.to_radians()),
        }
    }

    /// The unit direction **to** the body `progress` of the way through its lifetime.
    ///
    /// `CSun` turns `(cos θ, 0, −sin θ)` through `Rz(A)·Rx(B)` each takt and hands the
    /// result to the light manager as the first light's direction; this is its negation,
    /// the way a viewer sees the body. At `progress` ½ it is the fixed direction the
    /// azimuth and tilt name.
    pub fn direction(self, progress: f32) -> Vec3 {
        let (a, b) = self.angles();
        let theta = (BODY_ARC_SPAN * progress + BODY_ARC_START) * PI;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let (st, ct) = theta.sin_cos();
        Vec3::new(sa * sb * st - ca * ct, -sa * ct - ca * sb * st, cb * st)
    }
}

/// How many sections the cycle walks.
pub fn section_count(atmosphere: &Atmosphere) -> usize {
    atmosphere.section_headers.len().max(1)
}

/// How long one section's day lasts in real seconds: its header's second time, as
/// `CAtmData` keeps it (`0x1006a070`).
pub fn day_seconds(atmosphere: &Atmosphere, section: usize) -> u64 {
    match atmosphere.section_headers.get(section) {
        Some(s) => u64::from(s.day.hour()) * 3600 + u64::from(s.day.minute()) * 60,
        None => atmosphere.day_seconds() as u64,
    }
}

/// Every section's day end to end: the whole cycle (`0x1006efcd`).
pub fn cycle_seconds(atmosphere: &Atmosphere) -> u64 {
    (0..section_count(atmosphere)).map(|s| day_seconds(atmosphere, s)).sum()
}

/// When in its section's day a stamp falls (`0x1006d460`): its seconds since midnight
/// × that section's day length ÷ 86400, in integers.
pub fn stamp_seconds(atmosphere: &Atmosphere, section: usize, hour: u32, minute: u32) -> u64 {
    (u64::from(hour) * 3600 + u64::from(minute) * 60) * day_seconds(atmosphere, section) / CLOCK_DAY
}

/// When in its section's day a keyframe fires.
pub fn keyframe_seconds(atmosphere: &Atmosphere, keyframe: &Keyframe) -> u64 {
    stamp_seconds(atmosphere, keyframe.section, keyframe.hour, keyframe.minute)
}

/// Seconds forward from `from` to `to` (`CAtmData::GetTimeDiffInSec`, `0x1006a850`): out of
/// `from`'s section, through every section between, into `to`'s, wrapping round the
/// cycle when `to` is behind.
pub fn between(atmosphere: &Atmosphere, from: Position, to: Position) -> f64 {
    if from.section == to.section && to.seconds >= from.seconds {
        return to.seconds - from.seconds;
    }
    let count = section_count(atmosphere);
    let day = |s: usize| day_seconds(atmosphere, s) as f64;
    let mut total = day(from.section) - from.seconds;
    let mut s = (from.section + 1) % count;
    for _ in 0..count {
        if s == to.section {
            break;
        }
        total += day(s);
        s = (s + 1) % count;
    }
    total + to.seconds
}

/// Where the clock starts (`0x1006fab0`): the file's closing time, scaled by its
/// section's day.
pub fn start_position(atmosphere: &Atmosphere) -> Position {
    let start = &atmosphere.start;
    let section = (start.section() as usize).min(section_count(atmosphere) - 1);
    Position { section, seconds: stamp_seconds(atmosphere, section, start.hour(), start.minute()) as f64 }
}

/// Seconds since the atmosphere's epoch `elapsed` real seconds after the mission loads:
/// the epoch is set the start's distance from *(0, 0)* back (`0x10070330`).
pub fn since_epoch(atmosphere: &Atmosphere, elapsed: f64) -> f64 {
    between(atmosphere, Position::default(), start_position(atmosphere)) + elapsed
}

/// The position `elapsed` real seconds after the mission loads (`0x10070040`): the time
/// since the epoch, modulo the cycle, walked through the sections from section 0.
pub fn position(atmosphere: &Atmosphere, elapsed: f64) -> Position {
    let cycle = cycle_seconds(atmosphere) as f64;
    if cycle <= 0.0 {
        return Position::default();
    }
    let mut t = since_epoch(atmosphere, elapsed).rem_euclid(cycle);
    let count = section_count(atmosphere);
    for section in 0..count {
        let day = day_seconds(atmosphere, section) as f64;
        if t < day || section + 1 == count {
            return Position { section, seconds: t.min(day) };
        }
        t -= day;
    }
    Position::default()
}

/// One section's keyframes in time order, as `0x10067500`'s bubble sort leaves them.
pub fn section_keyframes(atmosphere: &Atmosphere, section: usize) -> Vec<&Keyframe> {
    let mut frames: Vec<&Keyframe> = atmosphere.keyframes.iter().filter(|k| k.section == section).collect();
    frames.sort_by_key(|k| k.minutes());
    frames
}

/// Every event of the cycle in order, section then time; opcodes 2 and 7 are left out.
pub fn events(atmosphere: &Atmosphere) -> Vec<Event<'_>> {
    (0..section_count(atmosphere))
        .flat_map(|section| section_keyframes(atmosphere, section))
        .filter_map(|k| {
            let (start, kind) = event_of(k.opcode)?;
            let position = Position { section: k.section, seconds: keyframe_seconds(atmosphere, k) as f64 };
            Some(Event { position, start, kind, keyframe: k })
        })
        .collect()
}

/// How long a body started at `start` is given (`0x1006dcb7`): forward to the first
/// stop-`SUN` keyframe at or after it, in its section and then the later ones, and
/// before the end of the cycle, the last section's full day. It never wraps back to
/// section 0, and a stop stamped 24:00 lies on that end and does not count. `None` when
/// nothing is found.
pub fn lifetime(atmosphere: &Atmosphere, start: Position) -> Option<f64> {
    let last = section_count(atmosphere) - 1;
    let end = Position { section: last, seconds: day_seconds(atmosphere, last) as f64 };
    events(atmosphere)
        .iter()
        .filter(|e| !e.start && e.kind == Kind::Sun)
        .map(|e| e.position)
        .find(|p| !p.before(&start) && p.before(&end))
        .map(|p| between(atmosphere, start, p))
}

/// Every body the cycle starts: which, where it starts, and how long it is given. When
/// the search finds no stop, the block keeps what the previous search left, as on the two
/// 24-hour skies' second sun; a first start with no stop, which no shipped file has, is
/// given nothing.
pub fn bodies(atmosphere: &Atmosphere) -> Vec<(Body, Position, f64)> {
    let mut previous = 0.0;
    events(atmosphere)
        .iter()
        .filter(|e| e.start && e.kind == Kind::Sun)
        .map(|e| {
            previous = lifetime(atmosphere, e.position).unwrap_or(previous);
            (Body::named(&e.keyframe.name), e.position, previous)
        })
        .collect()
}

/// The bodies up `elapsed` real seconds after the mission loads, in the order they
/// start: each from the last time the clock passed its start, for its lifetime. The last
/// event position starts at *(0, 0)*, so the first takt fires every event stamped before
/// the clock's start, and a body due by then is up as the mission begins; one stamped
/// after it waits for the clock.
pub fn bodies_up(atmosphere: &Atmosphere, elapsed: f64) -> Vec<Body> {
    bodies_aloft(atmosphere, elapsed).into_iter().map(|(body, _)| body).collect()
}

/// The same, each with how far through its lifetime it is, 0 to 1 — the fraction `CSun`
/// forms as `(now − start) ÷ lifetime` and clamps to 1 (`Terrain.dll:0x1007ed4c`), which
/// is what carries a body along its arc.
pub fn bodies_aloft(atmosphere: &Atmosphere, elapsed: f64) -> Vec<(Body, f32)> {
    let now = since_epoch(atmosphere, elapsed);
    let cycle = (cycle_seconds(atmosphere) as f64).max(1.0);
    bodies(atmosphere)
        .into_iter()
        .filter_map(|(body, start, life)| {
            let fired = between(atmosphere, Position::default(), start);
            if now < fired {
                return None;
            }
            let into = (now - fired).rem_euclid(cycle);
            (into < life).then(|| (body, (into / life.max(1.0)) as f32))
        })
        .collect()
}

/// What the sky holds at one moment. Colours are the files' own, 0..1; the lights may
/// run past 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Each group runs north, east, south, west: index k at k × 90° from +y to +x.
    pub horizon: [Rgb; 4],
    pub ring3: [Rgb; 4],
    pub ring2: [Rgb; 4],
    pub apex: Rgb,
    pub fog_start: f32,
    pub fog_end: f32,
    pub scene_colour: Rgb,
    /// The sun object's main light before the flare lifts it: slot 19 × the third float.
    pub sun_light: Rgb,
    /// Slot 17's alpha ÷ 255: how far the flare gates may lift the main light.
    pub sun_boost: f32,
    /// The sun object's second light: slot 21.
    pub second_light: Rgb,
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// Lerp one colour slot channel by channel, alpha too, then to 0..1.
fn channels(k0: &Keyframe, k1: &Keyframe, slot: usize, t: f32) -> [f32; 4] {
    let (a, b) = (k0.colour(slot), k1.colour(slot));
    std::array::from_fn(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t) / 255.0)
}

fn colour(k0: &Keyframe, k1: &Keyframe, slot: usize, t: f32) -> Rgb {
    let [r, g, b, _] = channels(k0, k1, slot, t);
    [r, g, b]
}

/// The sky at `now` (`0x1006a970`): the two keyframes of `now`'s section around its
/// clock, wrapping from the section's last keyframe to its first.
pub fn at(atmosphere: &Atmosphere, now: Position) -> Option<Sky> {
    let frames = section_keyframes(atmosphere, now.section);
    let first = *frames.first()?;
    let day = day_seconds(atmosphere, now.section).max(1) as f64;
    let when = |k: &Keyframe| keyframe_seconds(atmosphere, k) as f64;
    let clock = now.seconds.rem_euclid(day);
    let next = frames.iter().position(|k| when(k) > clock).unwrap_or(frames.len());
    let (k0, k1, span_start, span_end) = if next == 0 || next == frames.len() {
        let last = *frames.last()?;
        let start = when(last) - if next == 0 { day } else { 0.0 };
        (last, first, start, when(first) + if next == 0 { 0.0 } else { day })
    } else {
        let (a, b) = (frames[next - 1], frames[next]);
        (a, b, when(a), when(b))
    };
    let t = if span_end > span_start { ((clock - span_start) / (span_end - span_start)) as f32 } else { 0.0 };
    let t = t.clamp(0.0, 1.0);
    let group = |slots: [usize; 4]| slots.map(|s| colour(k0, k1, s, t));
    let number = |slot: usize| k0.number(slot) + (k1.number(slot) - k0.number(slot)) * t;
    let light = k0.intensity[LIGHT_FLOAT] + (k1.intensity[LIGHT_FLOAT] - k0.intensity[LIGHT_FLOAT]) * t;
    Some(Sky {
        horizon: group(HORIZON_SLOTS),
        ring3: group(RING3_SLOTS),
        ring2: group(RING2_SLOTS),
        apex: colour(k0, k1, APEX_SLOT, t),
        fog_start: FOG_SCALE * number(FOG_START_SLOT),
        fog_end: FOG_SCALE * number(FOG_END_SLOT),
        scene_colour: colour(k0, k1, SCENE_COLOUR_SLOT, t),
        sun_light: colour(k0, k1, SUN_LIGHT_SLOT, t).map(|c| c * light),
        sun_boost: channels(k0, k1, SUN_BOOST_SLOT, t)[3],
        second_light: colour(k0, k1, SUN_SECOND_LIGHT_SLOT, t),
    })
}

/// The flare's first gate (`docs/10-sky.md`, "The lens flare"): out once `toward` is
/// more than 15° off the view axis, linear in the cosine to full on it.
pub fn flare_view_gate(view: Vec3, toward: Vec3) -> f32 {
    let cos = view.normalize_or_zero().dot(toward.normalize_or_zero());
    let edge = FLARE_CONE_DEGREES.to_radians().cos();
    ((cos - edge) / (1.0 - edge)).clamp(0.0, 1.0)
}

/// The flare's second gate, on the height of a body's unit direction.
pub fn flare_height_gate(height: f32) -> f32 {
    ((height - FLARE_HEIGHT_ZERO) / (FLARE_HEIGHT_FULL - FLARE_HEIGHT_ZERO)).clamp(0.0, 1.0)
}

/// The sun object's two directional lights' colours with the body at `toward`, seen along `view`
/// (`docs/10-sky.md`, "What the sun does with its seven values"): the main light is
/// `lerp(c, 5c, gate1² × gate2 × slot 17's alpha)` of slot 19 × the third float
/// (`0x1007ea14`), the second slot 21.
pub fn sun_lights(sky: &Sky, toward: Vec3, view: Vec3) -> [Rgb; 2] {
    let gate = flare_view_gate(view, toward);
    let lift = gate * gate * flare_height_gate(toward.z) * sky.sun_boost;
    let main = sky.sun_light.map(|c| c + (FLARE_LIGHT_BOOST * c - c) * lift);
    [main, sky.second_light]
}

impl Sky {
    /// The fog colour looking along `heading`, radians from +y towards +x
    /// (`Terrain.dll:0x10079730`): whole degrees from −180, the quadrant and the
    /// fraction through it blending two horizon colours.
    pub fn fog_colour(&self, heading: f32) -> Rgb {
        let heading = (heading + PI).rem_euclid(TAU) - PI;
        let deg = ((heading + PI) * 180.0 / PI).floor() as i32;
        let k = ((deg / 90 + 2) % 4) as usize;
        let f = (deg % 90) as f32 / 90.0;
        lerp(self.horizon[k], self.horizon[(k + 1) % 4], f)
    }

    /// The dome's vertex colours in [`dome`]'s order: the apex and ring 1 the apex, rings
    /// 2–4 their compass groups, the rim `rim` (`0x1007ac60`).
    pub fn dome_colours(&self, rim: Rgb) -> Vec<Rgb> {
        let quarter = DOME_SEGMENTS / 4;
        let around = |group: [Rgb; 4], j: usize| {
            let (q, m) = (j / quarter, j % quarter);
            lerp(group[q], group[(q + 1) % 4], m as f32 / quarter as f32)
        };
        let mut out = vec![self.apex];
        for ring in 1..=DOME_RINGS {
            for j in 0..DOME_SEGMENTS {
                out.push(match ring {
                    1 => self.apex,
                    2 => around(self.ring2, j),
                    3 => around(self.ring3, j),
                    4 => around(self.horizon, j),
                    _ => rim,
                });
            }
        }
        out
    }
}

/// The dome around the camera (`0x100787f0`): the apex at the dome's height, then
/// ring by ring a spherical cap whose rim lies at eye height.
pub fn dome() -> Vec<Vec3> {
    let radius = DOME_HEIGHT / (2.0 * (DOME_ANGLE / 2.0).sin().powi(2));
    let mut out = vec![Vec3::new(0.0, 0.0, DOME_HEIGHT)];
    for r in 1..=DOME_RINGS {
        let theta = r as f32 / DOME_RINGS as f32 * DOME_ANGLE;
        for j in 0..DOME_SEGMENTS {
            let phi = j as f32 * TAU / DOME_SEGMENTS as f32;
            out.push(Vec3::new(
                radius * theta.sin() * phi.sin(),
                radius * theta.sin() * phi.cos(),
                radius * theta.cos() + DOME_HEIGHT - radius,
            ));
        }
    }
    out
}

/// The dome's triangles over [`dome`]'s vertices, wound to face its centre.
pub fn dome_indices() -> Vec<u32> {
    let at = |ring: usize, j: usize| (1 + (ring - 1) * DOME_SEGMENTS + j % DOME_SEGMENTS) as u32;
    let mut out = Vec::new();
    for j in 0..DOME_SEGMENTS {
        out.extend([0, at(1, j + 1), at(1, j)]);
    }
    for ring in 1..DOME_RINGS {
        for j in 0..DOME_SEGMENTS {
            let (a, b, c, d) = (at(ring, j), at(ring, j + 1), at(ring + 1, j), at(ring + 1, j + 1));
            out.extend([a, b, c, b, d, c]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use parkan_formats::sky::{ClockTime, Section};

    use super::*;

    fn keyframe(section: usize, hour: u32, minute: u32, opcode: u32, name: &str) -> Keyframe {
        Keyframe {
            hour,
            minute,
            section,
            slots: [[0u8; 4]; 22],
            name: name.to_owned(),
            names: vec![
                name.to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            effects: Vec::new(),
            intensity: [0.0; 4],
            opcode,
            version: 3,
            time: ClockTime::of(hour, minute),
        }
    }

    fn sky_keyframe(hour: u32, horizon_red: u8, fog_end: f32) -> Keyframe {
        let mut k = keyframe(0, hour, 0, 7, "");
        k.slots[HORIZON_SLOTS[0]] = [0, 0, horizon_red, 255];
        k.slots[FOG_END_SLOT] = fog_end.to_le_bytes();
        k
    }

    /// Sections whose days last these many minutes, starting at `start`.
    fn atmosphere(days: &[u32], keyframes: Vec<Keyframe>, start: ClockTime) -> Atmosphere {
        let section_headers = days
            .iter()
            .enumerate()
            .map(|(index, &minutes)| Section {
                index,
                version: 1,
                count: 0,
                end: ClockTime::of(23, 59),
                day: ClockTime::of(minutes / 60, minutes % 60),
            })
            .collect();
        Atmosphere {
            sections: days.len() as u32,
            header: vec![0u8; parkan_formats::sky::HEADER_SIZE],
            section_headers,
            keyframes,
            start,
            trailer_word: 0,
            sky_flag: 0,
        }
    }

    fn day(keyframes: Vec<Keyframe>) -> Atmosphere {
        atmosphere(&[24], keyframes, ClockTime::default())
    }

    fn at_seconds(section: usize, seconds: f64) -> Position {
        Position { section, seconds }
    }

    #[test]
    fn keyframes_lerp_around_the_clock_and_wrap_at_midnight() {
        let a = day(vec![sky_keyframe(18, 200, 1.0), sky_keyframe(6, 100, 0.5)]);
        // A 24-minute day: 6h is 360 s, 18h 1080 s.
        let noon = at(&a, at_seconds(0, 720.0)).unwrap();
        assert!((noon.horizon[0][0] - 150.0 / 255.0).abs() < 1e-5);
        assert!((noon.fog_end - 525.0).abs() < 1e-3);
        let midnight = at(&a, at_seconds(0, 0.0)).unwrap();
        assert!((midnight.horizon[0][0] - 150.0 / 255.0).abs() < 1e-5, "halfway from 18h to 6h");
        let evening = at(&a, at_seconds(0, 1080.0 + 1440.0)).unwrap();
        assert!((evening.fog_end - 700.0).abs() < 1e-3, "a whole day later");
    }

    #[test]
    fn a_named_keyframe_is_one_of_the_two_around_the_clock() {
        // The sun's start keyframe carries colours like any other.
        let mut sun = sky_keyframe(12, 0, 0.6);
        sun.name = "sun".to_owned();
        sun.opcode = 0;
        let a = day(vec![sky_keyframe(6, 100, 0.5), sun, sky_keyframe(18, 200, 1.0)]);
        let sky = at(&a, at_seconds(0, 720.0)).unwrap();
        assert!((sky.fog_end - 420.0).abs() < 1e-3, "{}", sky.fog_end);
    }

    #[test]
    fn colours_lerp_within_the_current_section() {
        let mut second = sky_keyframe(12, 250, 1.0);
        second.section = 1;
        let a = atmosphere(&[24, 48], vec![sky_keyframe(12, 50, 0.5), second], ClockTime::default());
        // Section 1's noon is 1440 s into its 48-minute day, and it has one keyframe.
        let sky = at(&a, at_seconds(1, 1440.0)).unwrap();
        assert!((sky.horizon[0][0] - 250.0 / 255.0).abs() < 1e-5);
        assert!((at(&a, at_seconds(0, 0.0)).unwrap().horizon[0][0] - 50.0 / 255.0).abs() < 1e-5);
    }

    #[test]
    fn a_stamp_is_scaled_by_its_own_sections_day_in_integers() {
        let a = atmosphere(&[15, 40], Vec::new(), ClockTime::default());
        // Mission 01's start: 01:30 of a 900-second day is 56 s in, not 56.25.
        assert_eq!(stamp_seconds(&a, 0, 1, 30), 56);
        assert_eq!(stamp_seconds(&a, 1, 12, 0), 1200);
        assert_eq!(cycle_seconds(&a), 900 + 2400);
    }

    #[test]
    fn the_clock_starts_at_the_files_closing_time_and_plays_the_sections_in_turn() {
        let mut start = ClockTime::of(1, 30);
        start.0[0] = 1;
        let a = atmosphere(&[15, 15], Vec::new(), start);
        assert_eq!(start_position(&a), at_seconds(1, 56.0));
        assert_eq!(position(&a, 0.0), at_seconds(1, 56.0));
        assert_eq!(position(&a, 843.0), at_seconds(1, 899.0), "section 1 runs out");
        assert_eq!(position(&a, 845.0), at_seconds(0, 1.0), "then section 0 plays again");
        assert_eq!(position(&a, 845.0 + 900.0), at_seconds(1, 1.0));
        // Forward only, wrapping round the cycle.
        assert_eq!(between(&a, at_seconds(1, 100.0), at_seconds(0, 50.0)), 850.0);
        assert_eq!(between(&a, at_seconds(0, 50.0), at_seconds(1, 100.0)), 950.0);
    }

    #[test]
    fn the_opcode_says_what_starts_and_stops() {
        assert_eq!(event_of(0), Some((true, Kind::Sun)));
        assert_eq!(event_of(1), Some((false, Kind::Sun)));
        assert_eq!(event_of(4), Some((false, Kind::Rain)));
        assert_eq!(event_of(8), Some((true, Kind::Lightning)));
        assert_eq!((event_of(2), event_of(7)), (None, None));
        let a = day(vec![keyframe(0, 12, 0, 7, ""), keyframe(0, 6, 0, 3, ""), keyframe(0, 1, 0, 0, "sun")]);
        let e = events(&a);
        assert_eq!(e.len(), 2, "opcode 7 does nothing");
        assert_eq!((e[0].kind, e[0].position), (Kind::Sun, at_seconds(0, 60.0)), "in time order");
    }

    /// Mission 01's day: the sun from 00:30 to 14:30 and the moon from 15:30 to 23:30 of a
    /// 15-minute day, starting at 01:30.
    fn mission_01() -> Atmosphere {
        atmosphere(
            &[15],
            vec![
                keyframe(0, 0, 30, 0, "sun"),
                keyframe(0, 14, 30, 1, "sun"),
                keyframe(0, 15, 30, 0, "moon"),
                keyframe(0, 23, 30, 1, "moon"),
                keyframe(0, 24, 0, 7, ""),
            ],
            ClockTime::of(1, 30),
        )
    }

    #[test]
    fn a_body_is_up_from_its_start_to_the_first_stop_after_it() {
        let a = mission_01();
        let b = bodies(&a);
        assert_eq!(b[0], (Body::Sun, at_seconds(0, 18.0), 525.0), "{b:?}");
        assert_eq!(b[1], (Body::Moon, at_seconds(0, 581.0), 300.0));
        // The clock opens 56 s into the day.
        assert_eq!(bodies_up(&a, 0.0), vec![Body::Sun], "the mission opens with the sun up");
        assert_eq!(bodies_up(&a, 542.0 - 56.0), vec![Body::Sun]);
        assert!(bodies_up(&a, 543.0 - 56.0).is_empty(), "set at its stop");
        assert_eq!(bodies_up(&a, 600.0 - 56.0), vec![Body::Moon]);
        assert!(bodies_up(&a, 900.0 + 10.0 - 56.0).is_empty(), "before the next sunrise");
        // A cycle later the sun is up again.
        assert_eq!(bodies_up(&a, 900.0), vec![Body::Sun]);
    }

    #[test]
    fn a_body_stamped_after_the_start_waits_for_the_clock_and_then_outlasts_the_cycle() {
        // A 24-minute day starting at 10:00 (600 s), with suns started at 00:00 and 22:00;
        // the second finds no stop before the cycle's end and keeps the first's 1200 s.
        let a = atmosphere(
            &[24],
            vec![
                keyframe(0, 0, 0, 0, "sun"),
                keyframe(0, 20, 0, 1, "sun"),
                keyframe(0, 22, 0, 0, "sun"),
                keyframe(0, 24, 0, 1, "sun"),
            ],
            ClockTime::of(10, 0),
        );
        assert_eq!(bodies(&a)[1], (Body::Sun, at_seconds(0, 1320.0), 1200.0));
        assert_eq!(bodies_up(&a, 0.0), vec![Body::Sun], "only the start due by 10:00 has fired");
        assert!(bodies_up(&a, 1300.0 - 600.0).is_empty(), "between 20:00 and 22:00");
        assert_eq!(bodies_up(&a, 1320.0 - 600.0), vec![Body::Sun]);
        // Into the next cycle both are up: the first from 00:00, the second still going.
        assert_eq!(bodies_up(&a, 1440.0 + 60.0 - 600.0), vec![Body::Sun, Body::Sun]);
    }

    #[test]
    fn a_lifetime_never_wraps_back_to_section_0_and_a_stop_at_the_cycles_end_does_not_count() {
        let a = atmosphere(
            &[24, 24],
            vec![
                keyframe(0, 1, 0, 1, "sun"),
                keyframe(0, 20, 0, 0, "sun"),
                keyframe(1, 2, 0, 1, "sun"),
                keyframe(1, 22, 0, 0, "sun"),
                keyframe(1, 24, 0, 1, "sun"),
            ],
            ClockTime::default(),
        );
        // From section 0's 20:00 (1200 s) on through its end to section 1's 02:00 (120 s).
        assert_eq!(lifetime(&a, at_seconds(0, 1200.0)), Some(240.0 + 120.0));
        // Section 1's 22:00 finds only the 24:00 stop, on the cycle's end.
        assert_eq!(lifetime(&a, at_seconds(1, 1320.0)), None);
        let b = bodies(&a);
        assert_eq!(b[1].2, 360.0, "the block keeps what the previous search left");
    }

    #[test]
    fn a_name_other_than_sun_is_the_moon() {
        assert_eq!(Body::named("sun"), Body::Sun);
        assert_eq!(Body::named("moon"), Body::Moon);
        assert_eq!(Body::named("Sun"), Body::Moon);
    }

    /// `CSun` turns `(cos θ, 0, −sin θ)` through `Rz(A)·Rx(B)` with θ running from −0.1π
    /// to 1.1π across the body's lifetime, and hands the result to the light manager as
    /// the first light's direction and its negation as the second's
    /// (`Terrain.dll:0x1007ed40`, `0x1007ee2a`, `0x1007eea8`).
    #[test]
    fn a_body_travels_an_arc_and_stands_at_its_constant_angles_halfway() {
        for (body, top) in [(Body::Sun, SUN_DIRECTION), (Body::Moon, MOON_DIRECTION)] {
            let mid = body.direction(0.5);
            assert!((mid - top).length() < 1e-5, "{body:?} {mid:?} against {top:?}");
            for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
                assert!((body.direction(p).length() - 1.0).abs() < 1e-5, "unit at {p}");
            }
            // It rises from below one horizon and sets below the other.
            assert!(body.direction(0.0).z < 0.0 && body.direction(1.0).z < 0.0);
            assert!(body.direction(0.25).z > 0.0 && body.direction(0.75).z > 0.0);
            // Rise and set are mirror images about the top of the arc.
            let (rise, set) = (body.direction(0.2), body.direction(0.8));
            assert!((rise.z - set.z).abs() < 1e-5, "{rise:?} {set:?}");
            // The height is cos(tilt) × sin θ, so the top of the arc is cos(tilt).
            assert!((mid.z - body.angles().1.cos()).abs() < 1e-6);
        }
        // The sun's zenith is exactly the top edge of the flare's second gate, and the
        // moon's 0.39 of the way up it; away from the top the sun's flare fades.
        assert_eq!(flare_height_gate(Body::Sun.direction(0.5).z), 1.0);
        assert!(flare_height_gate(Body::Sun.direction(0.15).z) < 1.0);
        assert_eq!(flare_height_gate(Body::Sun.direction(0.0).z), 0.0);
    }

    #[test]
    fn a_body_carries_how_far_through_its_lifetime_it_is() {
        let a = mission_01();
        // The sun is up 525 s from 18 s into the day, and the clock opens 56 s in.
        let aloft = |t: f64| bodies_aloft(&a, t);
        assert_eq!(aloft(0.0).len(), 1);
        assert!((aloft(0.0)[0].1 - 38.0 / 525.0).abs() < 1e-6, "{:?}", aloft(0.0));
        assert!((aloft(262.5 - 38.0)[0].1 - 0.5).abs() < 1e-6, "halfway, at its zenith");
        assert!(aloft(524.0 - 38.0)[0].1 < 1.0);
        assert!(aloft(525.0 - 38.0).is_empty(), "down at its stop");
        // The moon runs its own arc over its own 300 s.
        assert_eq!(aloft(600.0 - 56.0)[0].0, Body::Moon);
    }

    #[test]
    fn the_flare_gates_lift_the_main_light_up_to_five_times() {
        let mut sky = at(&day(vec![sky_keyframe(0, 0, 1.0)]), at_seconds(0, 0.0)).unwrap();
        sky.sun_light = [0.5, 0.2, 0.0];
        sky.second_light = [0.3, 0.3, 0.4];
        sky.sun_boost = 1.0;
        // On the view axis the sun, on the top edge of the height ramp, lights at 5×.
        let [main, second] = sun_lights(&sky, SUN_DIRECTION, SUN_DIRECTION);
        assert!((main[0] - 2.5).abs() < 1e-4 && (main[1] - 1.0).abs() < 1e-4, "{main:?}");
        assert_eq!(second, [0.3, 0.3, 0.4], "the second light is slot 21, unlifted");
        // The moon's height gives 0.39 of the lift.
        assert!((flare_height_gate(MOON_DIRECTION.z) - 0.390).abs() < 1e-3);
        assert_eq!(flare_height_gate(SUN_DIRECTION.z), 1.0);
        // Past 15° off the axis the light is its colour.
        assert_eq!(sun_lights(&sky, SUN_DIRECTION, Vec3::Y)[0], [0.5, 0.2, 0.0]);
        // Halfway along the cosine the gate is a half, and squared a quarter of the lift.
        let edge = FLARE_CONE_DEGREES.to_radians().cos();
        let half = ((1.0 + edge) / 2.0).acos() + 30f32.to_radians();
        let view = Vec3::new(half.sin(), 0.0, half.cos());
        assert!((flare_view_gate(view, SUN_DIRECTION) - 0.5).abs() < 1e-3);
        let lifted = sun_lights(&sky, SUN_DIRECTION, view)[0];
        assert!((lifted[0] - 0.5 * (1.0 + 4.0 * 0.25)).abs() < 5e-3, "{lifted:?}");
        sky.sun_boost = 0.0;
        assert_eq!(
            sun_lights(&sky, SUN_DIRECTION, SUN_DIRECTION)[0],
            [0.5, 0.2, 0.0],
            "slot 17's alpha scales it"
        );
    }

    #[test]
    fn the_sky_carries_the_suns_seven_values() {
        let mut k = sky_keyframe(0, 0, 1.0);
        k.slots[SUN_LIGHT_SLOT] = [0, 40, 255, 255];
        k.slots[SUN_BOOST_SLOT] = [0, 0, 255, 51];
        k.slots[SUN_SECOND_LIGHT_SLOT] = [115, 80, 80, 255];
        k.intensity[LIGHT_FLOAT] = 0.2;
        let sky = at(&day(vec![k]), at_seconds(0, 0.0)).unwrap();
        assert!(
            (sky.sun_light[0] - 0.2).abs() < 1e-6 && (sky.sun_light[1] - 0.2 * 40.0 / 255.0).abs() < 1e-6
        );
        assert!((sky.sun_boost - 0.2).abs() < 1e-6);
        assert!((sky.second_light[2] - 115.0 / 255.0).abs() < 1e-6, "stored BGRA");
    }

    #[test]
    fn the_fog_colour_is_the_horizon_ahead() {
        let mut sky = at(&day(vec![sky_keyframe(6, 100, 0.5)]), at_seconds(0, 360.0)).unwrap();
        sky.horizon = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
        assert_eq!(sky.fog_colour(0.0), [1.0, 0.0, 0.0], "north");
        assert_eq!(sky.fog_colour(PI / 2.0), [0.0, 1.0, 0.0], "east");
        assert_eq!(sky.fog_colour(PI / 4.0), [0.5, 0.5, 0.0]);
    }

    #[test]
    #[ignore = "needs the game install"]
    fn mission_01s_sky_opens_at_01_30_with_the_sun_up() {
        use parkan_formats::gamedir;
        let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
        let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
        let a = parkan_formats::sky::parse(&std::fs::read(dir.join("sky.ske")).unwrap(), "sky.ske").unwrap();
        // docs/10-sky.md: 01:30 of a 900-second day is 56 s in; the sun is given 525 s from
        // 00:30 and the moon 300 from 15:30.
        assert_eq!(position(&a, 0.0), at_seconds(0, 56.0));
        let b = bodies(&a);
        assert_eq!(
            b.iter().map(|&(body, _, life)| (body, life)).collect::<Vec<_>>(),
            [(Body::Sun, 525.0), (Body::Moon, 300.0)]
        );
        assert_eq!(bodies_up(&a, 0.0), vec![Body::Sun]);
        // The sky at the start is the 01:30 keyframe's: slot 19 × 0.7, slot 21 and slot 17.
        let sky = at(&a, position(&a, 0.0)).unwrap();
        assert!((sky.sun_light[0] - 0.7).abs() < 1e-5, "{:?}", sky.sun_light);
        assert!((sky.second_light[2] - 160.0 / 255.0).abs() < 1e-5, "{:?}", sky.second_light);
        assert!((sky.sun_boost - 75.0 / 255.0).abs() < 1e-5);
        assert!((sky.fog_end - 525.0).abs() < 1e-3);
    }

    #[test]
    #[ignore = "needs the game install"]
    fn every_mission_opens_with_the_sun_up() {
        fn skies(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    skies(&path, out);
                } else if path.file_name().is_some_and(|n| n.eq_ignore_ascii_case("sky.ske")) {
                    out.push(path);
                }
            }
        }
        let game = parkan_formats::gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
        let mut files = Vec::new();
        skies(&parkan_formats::gamedir::resolve(&game, "MISSIONS").unwrap(), &mut files);
        assert_eq!(files.len(), 29);
        // docs/10-sky.md, "Where the clock starts": every start is inside the sun's window.
        for f in &files {
            let a = parkan_formats::sky::parse(&std::fs::read(f).unwrap(), "sky.ske").unwrap();
            assert_eq!(bodies_up(&a, 0.0), vec![Body::Sun], "{}", f.display());
        }
    }

    #[test]
    fn the_dome_is_a_cap_from_its_height_to_eye_level() {
        let v = dome();
        assert_eq!(v.len(), 1 + DOME_RINGS * DOME_SEGMENTS);
        assert_eq!(v[0].z, DOME_HEIGHT);
        let rim = v[v.len() - DOME_SEGMENTS];
        assert!(rim.z.abs() < 0.5 && (rim.length() - 24_142.1).abs() < 1.0, "{rim}");
        assert!(v[1].y > 0.0 && v[1].x.abs() < 1e-3, "segment 0 lies north");
        assert_eq!(dome_indices().len(), 3 * DOME_SEGMENTS * (2 * DOME_RINGS - 1));
    }
}
