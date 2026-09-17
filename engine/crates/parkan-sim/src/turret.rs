//! A turret's channels, its arms and its camera: `docs/30-turrets.md`, "Aiming and
//! the camera", and `docs/29-weapons.md`, "The button reaches the selected guns" and
//! "A gun is ready once its arm is out".
//!
//! Every section-2 record is a channel, a value from 0 to 1 that plays its node's
//! frames. The turret component's two entries are its yaw and pitch channels; each
//! tick they step toward the turret's target at their rate. The follower channels
//! are the guns' mounts: each heads for the pitch as its gun's arm unfolds, and says
//! whether the gun is ready.

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};
use parkan_formats::control::{
    ARM_TYPE, CAMERA_TYPE, CHANNEL_FOLLOWS, CHANNEL_TURRET, CHANNEL_UNDRIVEN, CHANNEL_WRAP, Channel,
    Controller, GUN_TYPE, MOUNT_UPRIGHT, TURRET_HQ, TURRET_TYPE,
};

use crate::guns::Gun;

/// Type 30, the builder's beam, is built by the gun class too (`Control.dll:0x100294c0`).
pub const BUILDER_TYPE: i32 = 30;
/// An item's state word, low two bits: 1 moves its progress toward 1, 2 toward 0
/// (`Control.dll:0x10020a28`).
pub const ITEM_OPENING: i32 = 1;
pub const ITEM_CLOSING: i32 = 2;
/// What `World3D.dll` sends an arm: unfold and fold (`0x1001084a`, `0x1000ef4e`).
/// The item reads only their low bits.
pub const ARM_UNFOLD: i32 = 0x21;
pub const ARM_FOLD: i32 = 0x22;
/// An item's progress moves this far a step.
pub const ITEM_STEP: f32 = 0.45;
/// The world's gravity, which a falling round's mount solves against.
pub const GRAVITY: f32 = 10.0;
/// A falling round's mount rises by this share of its launch angle (`0x10028401`).
pub const LOB_SHARE: f32 = 0.83;
/// The camera shake (`0x10023ab0`): a jolt of this squared size starts a blend.
pub const SHAKE_JOLT_SQ: f32 = 0.3;
/// The blend's length, s (`+0xac`), and the ring-down's frequency (`+0xa8`) and decay
/// (`+0xa4`), as the constructor sets them.
pub const SHAKE_BLEND_S: f32 = 2.5;
pub const SHAKE_FREQUENCY: f32 = 3.0;
pub const SHAKE_DECAY: f32 = 3.0;
/// The eye moves by the offset held to unit length, times this (`0x10023677`).
pub const SHAKE_SCALE: f32 = 0.02;

/// `0x100289f0`: `value` toward `target` by at most `rate × dt`, the short way round
/// on a wrapping channel.
pub fn step_toward(value: f32, target: f32, rate: f32, dt: f32, wrap: bool) -> f32 {
    let mut gap = target - value;
    if wrap && gap.abs() > 0.5 {
        gap -= gap.signum();
    }
    let most = rate * dt;
    let out = if gap.abs() <= most { target } else { value + most.copysign(gap) };
    if wrap { out - out.floor() } else { out }
}

fn settle(channel: &Channel, value: f32) -> f32 {
    if channel.flags & CHANNEL_WRAP != 0 { value - value.floor() } else { value.clamp(0.0, 1.0) }
}

/// The angle a falling round's mount raises it by to reach `to`, or `None` when it
/// cannot (`0x10028401`): the flight time from t² = 2 (A ∓ √D) ÷ g², A = v² − g·to.z,
/// D = A² − g²|to|², the lower arc first; the angle between that launch and the line.
pub fn lobbed_elevation(speed: f32, gravity: f32, to: Vec3) -> Option<f32> {
    let launch = lobbed_launch(speed, gravity, to)?;
    let square = to.length_squared();
    Some((launch.dot(to) / (launch.length() * square.sqrt())).clamp(-1.0, 1.0).acos())
}

/// The velocity, `speed` long, that carries a round falling at `gravity` to `to` on the lower
/// arc, or `None` when it cannot reach (the solve of `0x10028401`).
pub fn lobbed_launch(speed: f32, gravity: f32, to: Vec3) -> Option<Vec3> {
    let reach = speed * speed - gravity * to.z;
    let square = to.length_squared();
    let disc = reach * reach - gravity * gravity * square;
    if gravity <= 0.0 || square <= 0.0 || disc <= 0.0 {
        return None;
    }
    let scale = 2.0 / (gravity * gravity);
    let (near, spread) = (reach * scale, disc.sqrt() * scale);
    let t2 = if near < spread { near + spread } else { near - spread };
    if t2 <= 0.0 {
        return None;
    }
    let t = t2.sqrt();
    Some(Vec3::new(to.x / t, to.y / t, (to.z + 0.5 * gravity * t2) / t))
}

/// A class-24 arm, the base item (`0x10020800`): a progress its channels head for.
#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    /// The component's index among its controller's components.
    pub component: usize,
    pub channels: Vec<usize>,
    /// `+0x94`: 0 folded, 1 out.
    pub progress: f32,
    /// The state word; its low two bits move the progress.
    pub state: i32,
    /// Seconds until the item wakes again.
    wait: f32,
}

impl Arm {
    /// A state from the manual controller. The item acts on it at once, as docs/29
    /// derives when it has a gun selected with its number key ready about half a second
    /// after the key: the progress is out as the last 50-ms step starts.
    pub fn send(&mut self, state: i32) {
        self.state = state;
        self.wait = 0.0;
    }

    /// `0x10020900`: each wake moves the progress 0.45 toward the end the state names,
    /// clearing the state once the progress reaches it, and sleeps until the channels
    /// arrive. The hero's arms, at 2 a second, take 225, 225 and 50 ms.
    fn wake(&mut self, dt: f32, channels: &[Channel], values: &[f32]) {
        self.wait -= dt;
        for _ in 0..8 {
            if self.wait > 0.0 {
                return;
            }
            let end = match self.state & (ITEM_OPENING | ITEM_CLOSING) {
                ITEM_OPENING => 1.0,
                ITEM_CLOSING => 0.0,
                _ => {
                    self.wait = 0.0;
                    return;
                }
            };
            if (end - self.progress).abs() <= ITEM_STEP {
                self.progress = end;
                self.state = 0;
            } else {
                self.progress += ITEM_STEP.copysign(end - self.progress);
            }
            let arrive = |&c: &usize| {
                let (ch, v) = (channels.get(c)?, values.get(c)?);
                Some(if ch.rate > 0.0 { (self.progress - v).abs() / ch.rate } else { 0.0 })
            };
            self.wait += self.channels.iter().filter_map(arrive).fold(0.0, f32::max);
        }
    }
}

/// A gun's mount: a follower channel, and the gun and the arm it pairs with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub channel: usize,
    /// The gun's component index.
    pub gun: Option<usize>,
    /// The arm's index in [`Rig::arms`].
    pub arm: Option<usize>,
}

/// `0x10027170`: each channel flagged 8, in load order, less those also flagged
/// `0x40`, takes the next gun (class 2 or 30) and the next arm (class 24), each
/// searched from just after the last one taken.
pub fn mounts(controller: &Controller, arms: &[Arm]) -> Vec<Mount> {
    let kinds = &controller.components;
    let next = |from: usize, of: &dyn Fn(i32) -> bool| (from..kinds.len()).find(|&i| of(kinds[i].type_id));
    let (mut gun_from, mut arm_from) = (0, 0);
    let mut out = Vec::new();
    for (c, ch) in controller.channels.iter().enumerate() {
        if ch.flags & CHANNEL_TURRET == 0 || ch.flags & CHANNEL_FOLLOWS != 0 {
            continue;
        }
        let gun = next(gun_from, &|t| t == GUN_TYPE || t == BUILDER_TYPE);
        let arm = next(arm_from, &|t| t == ARM_TYPE);
        gun_from = gun.map_or(gun_from, |g| g + 1);
        arm_from = arm.map_or(arm_from, |a| a + 1);
        out.push(Mount {
            channel: c,
            gun,
            arm: arm.and_then(|a| arms.iter().position(|x| x.component == a)),
        });
    }
    out
}

/// The camera's shake (`0x10023ab0`, `0x10023566`–`0x100235c3`): an offset that blends
/// toward half of a large jolt and rings down after a small one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shake {
    /// `+0xc8`: where the blend started; the ring-down's amplitude.
    from: Vec3,
    /// `+0xb0`: half the jolt.
    toward: Vec3,
    /// `+0xa0`: when the blend or the ring-down started, s.
    start: f64,
    /// `+0xd4`: a blend is running.
    blending: bool,
    ringing: bool,
}

impl Shake {
    /// The offset at `t` s. A blend runs from where it started to half the jolt over
    /// 2.5 s, at (t − start) ÷ 2.5; a ring-down is amplitude × cos(π/2 × 3 × t) ÷
    /// (t + 1)³, t from its start.
    pub fn offset(&self, t: f64) -> Vec3 {
        let since = (t - self.start).max(0.0) as f32;
        if self.blending {
            self.from.lerp(self.toward, (since / SHAKE_BLEND_S).min(1.0))
        } else if self.ringing {
            self.from * (PI / 2.0 * SHAKE_FREQUENCY * since).cos() / (since + 1.0).powf(SHAKE_DECAY)
        } else {
            Vec3::ZERO
        }
    }

    /// A machine tick's jolt at `t` s, (previous − current velocity) ÷ the step in
    /// seconds (`0x1000c6e7`). One of squared size 0.3 or more starts a blend from the
    /// offset reached; a smaller one during a blend ends it, and the offset rings
    /// down; a smaller one at any other time does nothing.
    pub fn jolt(&mut self, jolt: Vec3, t: f64) {
        if jolt.length_squared() >= SHAKE_JOLT_SQ {
            *self =
                Shake { from: self.offset(t), toward: jolt * 0.5, start: t, blending: true, ringing: false };
        } else if self.blending {
            *self =
                Shake { from: self.offset(t), toward: Vec3::ZERO, start: t, blending: false, ringing: true };
        }
    }

    /// How far the eye moves at `t` s: the offset held to unit length, × 0.02
    /// (`0x10023646`, `0x10023677`).
    pub fn eye(&self, t: f64) -> Vec3 {
        self.offset(t).clamp_length_max(1.0) * SHAKE_SCALE
    }
}

/// The camera's view (`0x100234c0`, free look `0x10023788`): the look, and the up
/// the camera builds from `up`, turned by the free-look triple `free`. Returns the
/// look and the up.
///
/// The side is `up` × look and the up look × side (`0x100236bc`, `0x100236eb`).
/// Free look pitches (0.5 − y) × π about the side and yaws (0.5 − x) × 2π about the
/// up. The side points left for an up of +z, so mouse down tips the look up: the
/// vertical runs opposite to the main controls (docs/30, *derived*).
pub fn view(look: Vec3, up: Vec3, free: [f32; 3]) -> (Vec3, Vec3) {
    let look = look.normalize_or(Vec3::Y);
    let side = up.cross(look);
    let (side, up) = if side.length_squared() > 1e-12 {
        let side = side.normalize();
        (side, look.cross(side))
    } else {
        // STAND-IN: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -- how
        // the look-only frame for an up parallel to the look is built (`0x10023769`)
        // is not read; any frame about the look. No shipped camera's pitch reaches it.
        let (side, up) = look.any_orthonormal_pair();
        (side, up)
    };
    let [x, y, _] = free;
    // STAND-IN: docs/14-controls.md#from-a-row-to-a-command--read-and-measured -- the
    // game's mouse X invert is +1 and its hull integrator negates the turn; the
    // engine's X rows run the other way (`input::INVERT`), so the yaw is negated to
    // keep Shift + mouse right turning right, like the hull.
    let yaw = -(0.5 - x) * TAU;
    let turn = Quat::from_axis_angle(up, yaw) * Quat::from_axis_angle(side, (0.5 - y) * PI);
    (turn * look, turn * up)
}

/// A turret's channels, its arms and mounts, and the triples the input edits.
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
    /// An HQ's turret (`0x8000000`), which opens its unit's command view (docs/40).
    pub hq: bool,
    /// The turret's stored target triple (`+0x9c`): x yaw, y pitch.
    pub aim: [f32; 3],
    /// The camera's free-look triple (`+0x94`), 0.5 looking ahead.
    pub look: [f32; 3],
    /// The class-24 arms, in component order.
    pub arms: Vec<Arm>,
    /// The gun mounts, in the order the turret pairs them.
    pub mounts: Vec<Mount>,
    /// The z of `TurretCenter`'s direction in the world, which signs a falling round's
    /// elevation; the caller keeps it.
    pub center_up: f32,
    /// The point a turret in `CIS_POINTTRACE` traces, less `TurretCenter`'s position: what a
    /// falling round's mount solves for (`0x1001b4f0`, `0x10028200`). `None` for a turret under
    /// `CIS_MANUALCONTROL`, the player's, whose mounts take no lift.
    pub traced: Option<Vec3>,
    pub shake: Shake,
    /// The strafe offset the control takt hands the turret, in radians (`0x10005ab8`);
    /// the caller keeps it. The yaw channel plays it on top of its value ([`Rig::frame_of`]).
    pub strafe: f32,
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
        let hq = turret.is_some_and(|t| t.flags & TURRET_HQ != 0);
        let stored =
            |ch: Option<usize>| ch.map_or(0.5, |c| if upright { 1.0 - values[c] } else { values[c] });
        let arms: Vec<Arm> = controller
            .components
            .iter()
            .enumerate()
            .filter(|(_, c)| c.type_id == ARM_TYPE)
            .map(|(i, c)| Arm {
                component: i,
                channels: c.entries.iter().filter_map(|&e| usize::try_from(e).ok()).collect(),
                progress: 0.0,
                state: 0,
                wait: 0.0,
            })
            .collect();
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
            hq,
            mounts: mounts(controller, &arms),
            arms,
            center_up: 1.0,
            traced: None,
            shake: Shake::default(),
            strafe: 0.0,
            channels,
            values,
        }
    }

    /// The value the yaw (axis 0) or pitch (axis 1) channel heads for: the stored
    /// target, as an upright turret's channel sees it.
    pub fn target(&self, axis: usize) -> f32 {
        if self.upright { 1.0 - self.aim[axis] } else { self.aim[axis] }
    }

    /// One tick of `dt` seconds (`0x10027765`): the yaw and pitch, the arms, then each
    /// mount, which sets its gun's ready byte (`0x10027ecd`).
    pub fn update(&mut self, dt: f32, guns: &mut [Gun]) {
        for (axis, channel) in [(0, self.yaw), (1, self.pitch)] {
            let Some(c) = channel else { continue };
            let target = self.target(axis);
            let ch = &self.channels[c];
            self.values[c] = step_toward(self.values[c], target, ch.rate, dt, ch.flags & CHANNEL_WRAP != 0);
        }
        for arm in &mut self.arms {
            arm.wake(dt, &self.channels, &self.values);
            for &c in arm.channels.iter().filter(|&&c| c < self.channels.len()) {
                self.values[c] = step_toward(self.values[c], arm.progress, self.channels[c].rate, dt, false);
            }
        }
        for i in 0..self.mounts.len() {
            let m = self.mounts[i];
            let ch = self.channels[m.channel];
            let pitch = self.pitch.map_or(ch.initial, |_| self.target(1));
            let p = m.arm.and_then(|a| self.arms.get(a)).map_or(1.0, |a| a.progress);
            let gun = m.gun.and_then(|g| guns.iter_mut().find(|x| x.component == g));
            let falls = gun.as_ref().filter(|g| g.falls).map(|g| g.round_speed);
            let (target, ready) = match falls {
                // `0x10028200`: while the arm moves, the mount blends from its rest.
                _ if p < 1.0 => ((1.0 - p) * ch.initial + p * pitch, false),
                None => (pitch, true),
                // `0x10028200`: a traced target's offset is solved for, the lift signed by
                // `TurretCenter`'s z; a target out of the round's reach leaves the gun unready.
                // A manual turret's mount takes no lift and its gun is ready.
                Some(speed) => match self.traced {
                    None => (pitch, true),
                    Some(to) => match lobbed_elevation(speed, GRAVITY, to) {
                        Some(angle) => (pitch + LOB_SHARE * angle / ch.span * self.center_up.signum(), true),
                        None => (pitch, false),
                    },
                },
            };
            self.values[m.channel] =
                settle(&ch, step_toward(self.values[m.channel], target, ch.rate, dt, false));
            if let Some(g) = gun {
                g.ready = ready;
            }
        }
        // A channel flagged 0x40 takes the previous channel's value.
        for c in 1..self.channels.len() {
            if self.channels[c].flags & CHANNEL_FOLLOWS != 0 {
                self.values[c] = self.values[c - 1];
            }
        }
    }

    /// The frame a mesh node plays, where a channel drives it: the first channel on
    /// that node that is driven and has frames.
    ///
    /// The yaw channel adds the strafe offset ÷ its span, negated on a hung turret, before
    /// it wraps and inverts (docs/30): the offset turns the turret against the hull's
    /// strafe turn without entering the channel's value or its rate.
    pub fn frame_of(&self, node: usize) -> Option<f32> {
        self.channels
            .iter()
            .zip(&self.values)
            .enumerate()
            .find(|(_, (c, _))| c.node == node as i32 && c.flags & CHANNEL_UNDRIVEN == 0 && c.first >= 0.0)
            .map(|(i, (c, &v))| c.frame(v + self.strafe_share(i, c)))
    }

    /// What channel `index` adds to its value for the strafe offset.
    fn strafe_share(&self, index: usize, channel: &Channel) -> f32 {
        if Some(index) != self.yaw || channel.span == 0.0 {
            return 0.0;
        }
        let share = self.strafe / channel.span;
        if self.upright { share } else { -share }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::control::Component;

    const TICK: f32 = 1.0 / 60.0;

    fn channel(node: i32, first: f32, last: f32, initial: f32, rate: f32, flags: i32) -> Channel {
        Channel { node, first, last, initial, origin: -1, point: -1, rate, span: 1.0, flags }
    }

    fn component(type_id: i32, entries: Vec<i32>, flags: u32) -> Component {
        Component {
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
            weights: [0.0; 2],
        }
    }

    const LEVEL: f32 = 0.272_727_28;

    /// The hero turret's camera, yaw and pitch, one gun with its mount (and a flag-0x48
    /// copy of it) and its arm, and a second follower before a gun of its own.
    fn hero() -> Controller {
        Controller {
            channels: vec![
                channel(35, 0.0, 0.0, 0.0, 0.75, CHANNEL_UNDRIVEN),
                channel(1, 49.0, 53.0, 0.5, 100.0, 3),
                channel(34, 55.0, 57.0, LEVEL, 0.75, 0),
                channel(12, 55.0, 57.0, LEVEL, 1.5, CHANNEL_TURRET),
                channel(29, 55.0, 57.0, LEVEL, 1.5, CHANNEL_TURRET | CHANNEL_FOLLOWS),
                channel(13, 58.0, 60.0, 0.0, 4.0, 0),
                channel(11, 42.0, 48.0, 0.0, 2.0, 0),
                channel(28, 42.0, 48.0, 0.0, 2.0, 0),
                channel(21, 55.0, 57.0, LEVEL, 1.5, CHANNEL_TURRET),
            ],
            components: vec![
                component(TURRET_TYPE, vec![1, 2], MOUNT_UPRIGHT),
                component(CAMERA_TYPE, vec![0], 0),
                component(GUN_TYPE, vec![5], 0),
                component(ARM_TYPE, vec![6, 7], 0),
            ],
            ..Default::default()
        }
    }

    fn gun(index: usize) -> Gun {
        Gun::new(index, &component(GUN_TYPE, vec![], 0), &[])
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
        assert!((rig.aim[1] - (1.0 - LEVEL)).abs() < 1e-6);
        assert!((rig.frame_of(34).unwrap() - 55.545_45).abs() < 1e-4);
        assert_eq!(rig.frame_of(35), None, "the camera channel is not driven");
        // The stored target 0 is the channel's 1: full up, reached at 0.75 a second.
        rig.aim[1] = 0.0;
        rig.update(0.5, &mut []);
        assert!((rig.values[2] - (LEVEL + 0.375)).abs() < 1e-5);
        // The second follower has no arm, so it heads for the pitch's target at its own
        // 1.5 a second, ahead of the pitch channel itself.
        assert_eq!(rig.values[8], 1.0);
        for _ in 0..4 {
            rig.update(0.5, &mut []);
        }
        assert_eq!(rig.values[2], 1.0);
        assert_eq!(rig.frame_of(34), Some(57.0));
    }

    #[test]
    fn the_yaw_channel_plays_the_strafe_offset_over_its_span_and_nothing_else_does() {
        let mut rig = Rig::new(&hero());
        let yaw = rig.yaw.unwrap();
        let ch = rig.channels[yaw];
        let node = ch.node as usize;
        let pitch_node = rig.channels[rig.pitch.unwrap()].node as usize;
        let (yaw_frame, pitch_frame) = (rig.frame_of(node), rig.frame_of(pitch_node));
        // A quarter turn of strafe offset is a quarter of a 2π span on the yaw alone.
        rig.strafe = -std::f32::consts::FRAC_PI_2;
        let share = rig.strafe / ch.span;
        assert_eq!(rig.frame_of(node), Some(ch.frame(rig.values[yaw] + share)));
        assert_ne!(rig.frame_of(node), yaw_frame);
        assert_eq!(rig.frame_of(pitch_node), pitch_frame);
        assert_eq!(rig.values[yaw], Rig::new(&hero()).values[yaw], "the value does not take it");
        // A hung turret takes it negated.
        rig.upright = false;
        assert_eq!(rig.frame_of(node), Some(ch.frame(rig.values[yaw] - share)));
    }

    #[test]
    fn followers_pair_with_the_next_gun_and_arm_and_a_copy_is_passed_over() {
        let rig = Rig::new(&hero());
        assert_eq!(rig.arms.len(), 1);
        assert_eq!(rig.arms[0].channels, vec![6, 7]);
        assert_eq!(
            rig.mounts,
            vec![
                Mount { channel: 3, gun: Some(2), arm: Some(0) },
                Mount { channel: 8, gun: None, arm: None }
            ]
        );
    }

    #[test]
    fn an_arm_unfolds_in_half_a_second_and_its_gun_is_ready_once_its_progress_is_out() {
        let mut rig = Rig::new(&hero());
        let mut guns = [gun(2)];
        assert!(guns[0].ready, "the constructor sets the ready byte");
        rig.arms[0].send(ARM_UNFOLD);
        let mut progress = Vec::new();
        let mut ready_at = None;
        let mut out_at = None;
        for tick in 1..=40 {
            rig.update(TICK, &mut guns);
            progress.push(rig.arms[0].progress);
            if guns[0].ready && ready_at.is_none() {
                ready_at = Some(tick);
            }
            if rig.values[6] == 1.0 && out_at.is_none() {
                out_at = Some(tick);
            }
        }
        // Steps of 0.45, 0.45 and 0.1, lasting 225, 225 and 50 ms at 2 a second: each wake
        // falls on the first tick past its time.
        assert!(progress[..13].iter().all(|&p| p == ITEM_STEP), "{progress:?}");
        assert!(progress[13..27].iter().all(|&p| (p - 0.9).abs() < 1e-6), "{progress:?}");
        assert!(progress[27..].iter().all(|&p| p == 1.0), "{progress:?}");
        assert_eq!(rig.arms[0].state, 0, "the state clears at the end");
        assert_eq!(ready_at, Some(28), "ready as the last step starts, 450 ms in");
        assert_eq!(out_at, Some(30), "the channels are out at 500 ms");
        assert_eq!(rig.values[7], 1.0);

        // Folding: the next wake takes the progress back under 1, and the gun is not ready.
        rig.arms[0].send(ARM_FOLD);
        rig.update(TICK, &mut guns);
        assert!((rig.arms[0].progress - 0.55).abs() < 1e-6 && !guns[0].ready);
        for _ in 0..40 {
            rig.update(TICK, &mut guns);
        }
        assert_eq!((rig.arms[0].progress, rig.values[6], rig.arms[0].state), (0.0, 0.0, 0));
    }

    #[test]
    fn a_mount_blends_from_its_rest_toward_the_pitch_target_by_its_arms_progress() {
        let mut rig = Rig::new(&hero());
        let mut guns = [gun(2)];
        rig.aim[1] = 0.0; // the pitch channel's target is 1
        // Folded, the mount holds its rest and the copy follows it.
        rig.update(TICK, &mut guns);
        assert_eq!((rig.values[3], rig.values[4]), (LEVEL, LEVEL));
        // At p = 0.45 the mount heads for 0.55 × its rest + 0.45 × the pitch's target.
        rig.arms[0].progress = ITEM_STEP;
        rig.update(TICK, &mut guns);
        assert!((rig.values[3] - (LEVEL + 1.5 * TICK)).abs() < 1e-6, "at its own rate");
        rig.update(1.0, &mut guns);
        let blend = 0.55 * LEVEL + 0.45;
        assert!((rig.values[3] - blend).abs() < 1e-6, "{}", rig.values[3]);
        assert_eq!(rig.values[4], rig.values[3], "flag 0x40 copies the previous channel");
        assert!(!guns[0].ready);
        rig.arms[0].send(ARM_UNFOLD);
        rig.update(1.0, &mut guns);
        assert_eq!((rig.arms[0].progress, rig.values[3], guns[0].ready), (1.0, 1.0, true));
    }

    #[test]
    fn a_falling_rounds_mount_rises_by_its_solution_and_refuses_without_one() {
        let v = lobbed_elevation(50.0, GRAVITY, Vec3::new(100.0, 0.0, 0.0)).unwrap();
        // On flat ground sin 2θ = g R ÷ v².
        assert!((v - 0.5 * (0.4_f32).asin()).abs() < 1e-4, "{v}");
        // The launch is the round's speed long, and it lands where it was aimed.
        let to = Vec3::new(120.0, 40.0, -9.0);
        let launch = lobbed_launch(45.0, GRAVITY, to).unwrap();
        assert!((launch.length() - 45.0).abs() < 1e-3, "{launch}");
        let t = to.truncate().length() / launch.truncate().length();
        let landed = launch * t - Vec3::Z * (0.5 * GRAVITY * t * t);
        assert!((landed - to).length() < 1e-2, "{landed} for {to}");
        assert_eq!(lobbed_elevation(50.0, GRAVITY, Vec3::new(300.0, 0.0, 0.0)), None);
        assert_eq!(lobbed_elevation(50.0, 0.0, Vec3::new(100.0, 0.0, 0.0)), None);

        let mut rig = Rig::new(&hero());
        let mut guns = [gun(2)];
        guns[0].falls = true;
        guns[0].round_speed = 50.0;
        rig.arms[0].progress = 1.0;
        rig.aim = [0.5, 1.0 - LEVEL, 0.5];
        // A manual turret traces nothing: its mount takes no lift, and its gun is ready.
        rig.update(1.0, &mut guns);
        assert!(guns[0].ready);
        assert!((rig.values[3] - LEVEL).abs() < 1e-5, "{}", rig.values[3]);
        // Tracing a target 100 m off, level: the lower arc's lift.
        let to = Vec3::new(0.0, 100.0, 0.0);
        rig.traced = Some(to);
        rig.update(1.0, &mut guns);
        let lift = LOB_SHARE * lobbed_elevation(50.0, GRAVITY, to).unwrap();
        assert!(guns[0].ready);
        assert!((rig.values[3] - (LEVEL + lift)).abs() < 1e-5, "{}", rig.values[3]);
        // Past its reach, v² ÷ g = 250 m, the gun is not ready.
        rig.traced = Some(Vec3::new(0.0, 300.0, 0.0));
        rig.update(1.0, &mut guns);
        assert!(!guns[0].ready, "no solution, no shot");
    }

    #[test]
    fn a_large_jolt_blends_the_offset_and_a_small_one_rings_it_down() {
        let mut s = Shake::default();
        s.jolt(Vec3::new(0.5, 0.0, 0.0), 0.0);
        assert_eq!(s.offset(1.0), Vec3::ZERO, "a small jolt with no blend does nothing");
        let jolt = Vec3::new(0.0, -140.0, 0.0);
        s.jolt(jolt, 1.0);
        assert_eq!(s.offset(2.25), jolt * 0.25, "halfway to half the jolt at 1.25 s");
        assert_eq!(s.offset(10.0), jolt * 0.5, "it holds there");
        assert!((s.eye(2.25) - Vec3::new(0.0, -0.02, 0.0)).length() < 1e-7, "2 cm at most");
        // A second large jolt blends on from where the offset got.
        s.jolt(Vec3::ZERO.with_x(2.0), 2.25);
        assert_eq!(s.offset(2.25), jolt * 0.25);
        assert!((s.offset(4.75) - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4);
        // A small one ends the blend: cos(1.5π t) ÷ (t + 1)³ from the offset reached.
        s.jolt(Vec3::ZERO, 4.75);
        let amplitude = s.offset(4.75);
        assert!((amplitude - Vec3::X).length() < 1e-4);
        for t in [1.0_f32 / 3.0, 0.5, 1.0] {
            let k = (1.5 * PI * t).cos() / (t + 1.0).powi(3);
            assert!((s.offset(4.75 + f64::from(t)) - amplitude * k).length() < 1e-5, "{t}");
        }
        // Another small one while it rings changes nothing.
        s.jolt(Vec3::ZERO, 5.0);
        let k = (0.75 * PI).cos() / 1.5_f32.powi(3);
        assert!((s.offset(5.25) - amplitude * k).length() < 1e-5);
    }

    #[test]
    fn the_view_is_built_from_the_up_point_and_free_look_runs_vertically_opposite() {
        let (look, up) = view(Vec3::new(0.0, 2.0, 0.0), Vec3::Z, [0.5; 3]);
        assert!((look - Vec3::Y).length() < 1e-6 && (up - Vec3::Z).length() < 1e-6);
        // The up is re-made square to a pitched look.
        let pitched = Vec3::new(0.0, 1.0, 1.0).normalize();
        let (_, up) = view(pitched, Vec3::Z, [0.5; 3]);
        assert!(up.dot(pitched).abs() < 1e-6 && up.z > 0.7);
        // A hung turret's CameraCenter points down: its frame is upside down.
        let (_, up) = view(Vec3::Y, -Vec3::Z, [0.5; 3]);
        assert!((up + Vec3::Z).length() < 1e-6);
        // Shift + mouse down raises y: the look tips up; the engine's mouse right lowers x:
        // the look turns right.
        let (look, _) = view(Vec3::Y, Vec3::Z, [0.5, 0.75, 0.5]);
        assert!((look - Vec3::new(0.0, 1.0, 1.0).normalize()).length() < 1e-5, "{look}");
        let (look, _) = view(Vec3::Y, Vec3::Z, [0.25, 0.5, 0.5]);
        assert!((look - Vec3::X).length() < 1e-5, "{look}");
        // Parallel vectors still give a square frame.
        let (look, up) = view(Vec3::Z, Vec3::Z, [0.5; 3]);
        assert!(look.dot(up).abs() < 1e-6 && (look - Vec3::Z).length() < 1e-6);
    }
}
