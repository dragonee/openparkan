//! The sky at a moment: `docs/10-sky.md`, "The dome, the fog and the scene colour".
//!
//! The atmosphere lerps the two keyframes around its clock, colours channel by
//! channel and floats linearly; the sky takes the horizon, rings and apex for the
//! dome, 700 × slots 5 and 6 for the fog, and slot 20 as the scene colour.

use std::f32::consts::{PI, TAU};

use glam::Vec3;
use parkan_formats::sky::{
    APEX_SLOT, Atmosphere, CLOCK_DAY, FOG_END_SLOT, FOG_SCALE, FOG_START_SLOT, HORIZON_SLOTS, Keyframe,
    RING2_SLOTS, RING3_SLOTS, SCENE_COLOUR_SLOT, SUN_LIGHT_SLOT,
};

/// The dome's parameter block (`0x1006f1ba`): height, cap angle, rings, and 2 to the
/// `AtmSkyDetail` 4 segments.
pub const DOME_HEIGHT: f32 = 10_000.0;
pub const DOME_ANGLE: f32 = PI / 4.0;
pub const DOME_RINGS: usize = 5;
pub const DOME_SEGMENTS: usize = 16;

/// Where the sun and the moon stand: `CSun`'s constant azimuth and tilt as a direction,
/// `(sin A sin B, −cos A sin B, cos B)` (`docs/10-sky.md`, "Where the sun stands").
pub const SUN_DIRECTION: Vec3 = Vec3::new(0.5, 0.0, 0.866_025_4);
pub const MOON_DIRECTION: Vec3 = Vec3::new(0.0, -0.766_044_4, 0.642_787_6);

type Rgb = [f32; 3];

/// What the sky holds at one moment, colours 0..1.
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
    /// Slot 19 × the third intensity of the sky's own keyframes.
    pub sun_light: Rgb,
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// Lerp one colour slot channel by channel, as integers, then to 0..1.
fn colour(k0: &Keyframe, k1: &Keyframe, slot: usize, t: f32) -> Rgb {
    let (a, b) = (k0.colour(slot), k1.colour(slot));
    std::array::from_fn(|i| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t) / 255.0)
}

/// The sky of section `section` at `seconds` of real time into its day (`0x1006a970`).
///
/// Each object type interpolates its own keyframes; the sky's are the ones that name
/// no object (`sun`, `moon` and `env_lightning` keyframes belong to those objects).
pub fn at(atmosphere: &Atmosphere, section: usize, seconds: f64) -> Option<Sky> {
    let mut frames: Vec<&Keyframe> =
        atmosphere.keyframes.iter().filter(|k| k.section == section && k.name.is_empty()).collect();
    frames.sort_by_key(|k| k.minutes());
    let first = *frames.first()?;
    let day = atmosphere.day_seconds().max(1.0);
    let when = |k: &Keyframe| f64::from(k.minutes() * 60) * day / CLOCK_DAY;
    let now = seconds.rem_euclid(day);
    let next = frames.iter().position(|k| when(k) > now).unwrap_or(frames.len());
    let (k0, k1, span_start, span_end) = if next == 0 || next == frames.len() {
        let last = *frames.last()?;
        let start = when(last) - if next == 0 { day } else { 0.0 };
        (last, first, start, when(first) + if next == 0 { 0.0 } else { day })
    } else {
        let (a, b) = (frames[next - 1], frames[next]);
        (a, b, when(a), when(b))
    };
    let t = if span_end > span_start { ((now - span_start) / (span_end - span_start)) as f32 } else { 0.0 };
    let t = t.clamp(0.0, 1.0);
    let group = |slots: [usize; 4]| slots.map(|s| colour(k0, k1, s, t));
    let number = |slot: usize| k0.number(slot) + (k1.number(slot) - k0.number(slot)) * t;
    let light = k0.intensity[2] + (k1.intensity[2] - k0.intensity[2]) * t;
    Some(Sky {
        horizon: group(HORIZON_SLOTS),
        ring3: group(RING3_SLOTS),
        ring2: group(RING2_SLOTS),
        apex: colour(k0, k1, APEX_SLOT, t),
        fog_start: FOG_SCALE * number(FOG_START_SLOT),
        fog_end: FOG_SCALE * number(FOG_END_SLOT),
        scene_colour: colour(k0, k1, SCENE_COLOUR_SLOT, t),
        sun_light: colour(k0, k1, SUN_LIGHT_SLOT, t).map(|c| c * light),
    })
}

/// Whether the body `name` names is up at `seconds` into the day: from its first keyframe
/// to its second in clock order, wrapping past midnight. With one keyframe it never
/// stops, and with none it is never up.
///
/// STAND-IN: docs/10-sky.md#not-resolved -- which keyframe field carries the start and
/// stop opcodes is not read; a body's keyframes are taken as one start and one stop.
pub fn body_up(atmosphere: &Atmosphere, section: usize, name: &str, seconds: f64) -> bool {
    let mut times: Vec<u32> = atmosphere
        .keyframes
        .iter()
        .filter(|k| k.section == section && k.name.eq_ignore_ascii_case(name))
        .map(Keyframe::minutes)
        .collect();
    times.sort_unstable();
    let day = atmosphere.day_seconds().max(1.0);
    let minute = (seconds.rem_euclid(day) / day * CLOCK_DAY / 60.0) as u32;
    match times.as_slice() {
        [] => false,
        [_] => true,
        [start, stop, ..] if start <= stop => (*start..*stop).contains(&minute),
        [start, stop, ..] => minute >= *start || minute < *stop,
    }
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
    use super::*;

    fn keyframe(hour: u32, horizon_red: u8, fog_end: f32) -> Keyframe {
        let mut slots = [[0u8; 4]; 22];
        slots[HORIZON_SLOTS[0]] = [0, 0, horizon_red, 255];
        slots[FOG_END_SLOT] = fog_end.to_le_bytes();
        Keyframe {
            hour,
            minute: 0,
            section: 0,
            slots,
            name: String::new(),
            names: vec![String::new(); 6],
            effects: Vec::new(),
            intensity: [0.0; 4],
            opcode: 7,
            version: 3,
            time: parkan_formats::sky::ClockTime::of(hour, 0),
        }
    }

    fn atmosphere() -> Atmosphere {
        let mut header = vec![0u8; parkan_formats::sky::HEADER_SIZE];
        header[64..68].copy_from_slice(&0u32.to_le_bytes());
        header[68..72].copy_from_slice(&24u32.to_le_bytes());
        Atmosphere {
            sections: 1,
            header,
            section_headers: Vec::new(),
            keyframes: vec![keyframe(18, 200, 1.0), keyframe(6, 100, 0.5)],
            start: Default::default(),
            trailer_word: 0,
            sky_flag: 0,
        }
    }

    #[test]
    fn keyframes_lerp_around_the_clock_and_wrap_at_midnight() {
        let a = atmosphere();
        // A 24-minute day: 6h is 360 s, 18h 1080 s.
        let noon = at(&a, 0, 720.0).unwrap();
        assert!((noon.horizon[0][0] - 150.0 / 255.0).abs() < 1e-5);
        assert!((noon.fog_end - 525.0).abs() < 1e-3);
        let midnight = at(&a, 0, 0.0).unwrap();
        assert!((midnight.horizon[0][0] - 150.0 / 255.0).abs() < 1e-5, "halfway from 18h to 6h");
        let evening = at(&a, 0, 1080.0 + 1440.0).unwrap();
        assert!((evening.fog_end - 700.0).abs() < 1e-3, "a whole day later");
    }

    #[test]
    fn the_fog_colour_is_the_horizon_ahead() {
        let mut sky = at(&atmosphere(), 0, 360.0).unwrap();
        sky.horizon = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
        assert_eq!(sky.fog_colour(0.0), [1.0, 0.0, 0.0], "north");
        assert_eq!(sky.fog_colour(PI / 2.0), [0.0, 1.0, 0.0], "east");
        assert_eq!(sky.fog_colour(PI / 4.0), [0.5, 0.5, 0.0]);
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
