//! A turret's channels: `docs/30-turrets.md`, "Aiming and the camera".
//!
//! Every section-2 record is a channel, a value from 0 to 1 that plays its node's
//! frames. The turret component's two entries are its yaw and pitch channels; each
//! tick they step toward the turret's target at their rate.

use parkan_formats::control::{
    CAMERA_TYPE, CHANNEL_FOLLOWS, CHANNEL_TURRET, CHANNEL_UNDRIVEN, CHANNEL_WRAP, Channel, Controller,
    MOUNT_UPRIGHT, TURRET_TYPE,
};

/// `0x100289f0`: `value` toward `target` by at most `rate × dt`, the short way round
/// on a wrapping channel.
pub fn step_toward(value: f32, target: f32, rate: f32, dt: f32, wrap: bool) -> f32 {
    let mut gap = target - value;
    if wrap && gap.abs() > 0.5 {
        gap -= gap.signum();
    }
    let most = rate * dt;
    let out = if gap.abs() <= most { value + gap } else { value + most.copysign(gap) };
    if wrap { out - out.floor() } else { out }
}

fn settle(channel: &Channel, value: f32) -> f32 {
    if channel.flags & CHANNEL_WRAP != 0 { value - value.floor() } else { value.clamp(0.0, 1.0) }
}

/// A turret's channels and the triples the input edits.
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    pub channels: Vec<Channel>,
    pub values: Vec<f32>,
    /// The turret component's yaw and pitch channels.
    pub yaw: Option<usize>,
    pub pitch: Option<usize>,
    /// The camera component's channel and its values: near, far, field of view.
    pub camera: Option<usize>,
    pub camera_values: [f32; 16],
    /// An upright turret (`0x4000000`) stores its target as 1 − v (`0x100271c7`).
    pub upright: bool,
    /// The turret's stored target triple (`+0x9c`): x yaw, y pitch.
    pub aim: [f32; 3],
    /// The camera's free-look triple (`+0x94`), 0.5 looking ahead.
    pub look: [f32; 3],
}

impl Rig {
    pub fn new(controller: &Controller) -> Self {
        let channels = controller.channels.clone();
        let values = channels.iter().map(|c| settle(c, c.initial)).collect::<Vec<_>>();
        let turret = controller.components.iter().find(|c| c.type_id == TURRET_TYPE);
        let camera = controller.components.iter().find(|c| c.type_id == CAMERA_TYPE);
        let entry = |k: usize| {
            turret.and_then(|t| t.entries.get(k)).map(|&e| e as usize).filter(|&e| e < channels.len())
        };
        let (yaw, pitch) = (entry(0), entry(1));
        let upright = turret.is_some_and(|t| t.flags & MOUNT_UPRIGHT != 0);
        let stored =
            |ch: Option<usize>| ch.map_or(0.5, |c| if upright { 1.0 - values[c] } else { values[c] });
        Self {
            aim: [stored(yaw), stored(pitch), 0.5],
            look: [0.5; 3],
            yaw,
            pitch,
            camera: camera
                .and_then(|c| c.entries.first())
                .map(|&e| e as usize)
                .filter(|&e| e < channels.len()),
            camera_values: camera.map_or([0.0; 16], |c| c.values),
            upright,
            channels,
            values,
        }
    }

    /// One tick of `dt` seconds (`0x10027765`).
    pub fn update(&mut self, dt: f32) {
        for (axis, channel) in [(0, self.yaw), (1, self.pitch)] {
            let Some(c) = channel else { continue };
            let target = if self.upright { 1.0 - self.aim[axis] } else { self.aim[axis] };
            let ch = &self.channels[c];
            self.values[c] = step_toward(self.values[c], target, ch.rate, dt, ch.flags & CHANNEL_WRAP != 0);
        }
        // STAND-IN: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- how
        // a follower (flag 8) aims its gun's mount is not read: it steps toward the
        // pitch channel's value at its own rate, and a flag-0x40 channel takes the
        // previous channel's value.
        let pitch = self.pitch.map(|p| self.values[p]);
        for c in 0..self.channels.len() {
            let ch = self.channels[c];
            if ch.flags & CHANNEL_FOLLOWS != 0 && c > 0 {
                self.values[c] = self.values[c - 1];
            } else if ch.flags & CHANNEL_TURRET != 0
                && let Some(p) = pitch
            {
                self.values[c] = step_toward(self.values[c], p, ch.rate, dt, false);
            }
        }
    }

    /// The frame a mesh node plays, where a channel drives it: the first channel on
    /// that node that is driven and has frames.
    pub fn frame_of(&self, node: usize) -> Option<f32> {
        self.channels
            .iter()
            .zip(&self.values)
            .find(|(c, _)| c.node == node as i32 && c.flags & CHANNEL_UNDRIVEN == 0 && c.first >= 0.0)
            .map(|(c, &v)| c.frame(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::control::Component;

    fn channel(node: i32, first: f32, last: f32, initial: f32, rate: f32, flags: i32) -> Channel {
        Channel { node, first, last, initial, origin: -1, point: -1, rate, span: 1.0, flags }
    }

    /// The hero turret's yaw, pitch and camera channels and a follower.
    fn hero() -> Controller {
        let component = |type_id, entries: Vec<i32>, flags| Component {
            type_id,
            resource: Default::default(),
            index: None,
            entries,
            label: String::new(),
            values: [0.0; 16],
            power: 0.0,
            node: 0,
            mass: 0.0,
            flags,
            group: -1,
        };
        Controller {
            channels: vec![
                channel(35, 0.0, 0.0, 0.0, 0.75, CHANNEL_UNDRIVEN),
                channel(1, 49.0, 53.0, 0.5, 100.0, 3),
                channel(34, 55.0, 57.0, 0.272_727_28, 0.75, 0),
                channel(12, 55.0, 57.0, 0.272_727_28, 1.5, CHANNEL_TURRET),
            ],
            components: vec![
                component(TURRET_TYPE, vec![1, 2], MOUNT_UPRIGHT),
                component(CAMERA_TYPE, vec![0], 0),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_wrapping_channel_takes_the_short_way_round() {
        assert!((step_toward(0.9, 0.1, 1.0, 0.1, true) - 0.0).abs() < 1e-6);
        assert!((step_toward(0.2, 0.9, 1.0, 0.1, false) - 0.3).abs() < 1e-6);
        assert_eq!(step_toward(0.2, 0.25, 1.0, 0.1, false), 0.25);
    }

    #[test]
    fn the_hero_turret_starts_level_and_pitches_at_its_rate() {
        let mut rig = Rig::new(&hero());
        assert_eq!((rig.yaw, rig.pitch, rig.camera), (Some(1), Some(2), Some(0)));
        assert!(rig.upright);
        assert!((rig.aim[1] - (1.0 - 0.272_727_28)).abs() < 1e-6);
        assert!((rig.frame_of(34).unwrap() - 55.545_45).abs() < 1e-4);
        assert_eq!(rig.frame_of(35), None, "the camera channel is not driven");
        // The stored target 0 is the channel's 1: full up, reached at 0.75 a second.
        rig.aim[1] = 0.0;
        rig.update(0.5);
        assert!((rig.values[2] - (0.272_727_28 + 0.375)).abs() < 1e-5);
        assert!((rig.values[3] - rig.values[2]).abs() < 0.2, "the follower tracks the pitch");
        for _ in 0..4 {
            rig.update(0.5);
        }
        assert_eq!(rig.values[2], 1.0);
        assert_eq!(rig.frame_of(34), Some(57.0));
    }
}
