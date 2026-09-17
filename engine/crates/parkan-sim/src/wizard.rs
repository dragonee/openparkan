//! How the AI moves a machine: the walker cuts a place and a speed into timed movement
//! points, and the Wizard follows them, writing the machine's velocity and spin. See
//! `docs/24-motion.md`, "How the AI drives a machine", and `docs/31-packages.md`, "How a
//! walk's speed is held".

use std::collections::VecDeque;

use glam::Vec3;

/// A point's flags: one nibble per axis of the machine's frame, x at bit 4, y at 8, z at
/// 12 (`Wizard.dll:0x10003750`); 3 zeroes that axis. A walker's points carry `0x3030`,
/// a flyer's none (`Behavior.dll:0x1003daab`); a hold carries `0x3331`.
pub const GROUND_POINT: u32 = 0x3030;
pub const HOLD_POINT: u32 = 0x3331;
/// A stop: half the last point's velocity on, at rest, a second later
/// (`Behavior.dll:0x1003d7f0`).
pub const STOP_AFTER_MS: f64 = 1000.0;
/// `MWalker::SetTarget`'s holds (`Behavior.dll:0x1003be4f`): `Movement_SpeedPercent`,
/// `Movement_MaxSpeed`, `Movement_MinSpeedPercent`, and the slowest walk.
pub const MOVEMENT_SPEED_PERCENT: f32 = 1.0;
pub const MOVEMENT_MAX_SPEED: f32 = 600.0;
pub const MOVEMENT_MIN_SPEED_PERCENT: f32 = 1.0;
pub const MIN_WALK_SPEED: f32 = 2.0;
/// A walker's trajectory holds at least this many points (`PathFind_MinPointInTrajectory`).
pub const MIN_POINTS: usize = 3;

/// The speed the walker takes a request at: no more than the unit's top × the speed
/// percent × the difficulty's `Speed_MaximumFactor` and 600, no less than its second
/// figure × the minimum percent and 2 m/s.
pub fn walk_speed(requested: f32, top: f32, low: f32, maximum_factor: f32) -> f32 {
    requested
        .min(top * MOVEMENT_SPEED_PERCENT * maximum_factor)
        .min(MOVEMENT_MAX_SPEED)
        .max(low * MOVEMENT_MIN_SPEED_PERCENT)
        .max(MIN_WALK_SPEED)
}

/// A movement point: where to be, how fast, when, and its flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub position: Vec3,
    /// In the world.
    pub velocity: Vec3,
    pub time_ms: f64,
    pub flags: u32,
}

/// What the Wizard writes to the machine this takt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drive {
    /// In the world, through the point's flags once turned into the machine's frame.
    pub velocity: Vec3,
    /// The heading, as a yaw, the machine turns toward; none holds it.
    pub heading: Option<f32>,
    pub flags: u32,
}

impl Drive {
    pub const HOLD: Drive = Drive { velocity: Vec3::ZERO, heading: None, flags: HOLD_POINT };

    /// The velocity in a machine's frame at `yaw`, through the flags
    /// (`Wizard.dll:0x10003750`): a nibble of 1 keeps the axis from going negative, 2
    /// from going positive, 3 zeroes it, and 5 or 6 set it to ± `top` on that axis.
    pub fn in_frame(&self, yaw: f32, top: [f32; 3]) -> [f32; 3] {
        let (s, c) = yaw.sin_cos();
        let v = self.velocity;
        let mut local = [v.x * c + v.y * s, -v.x * s + v.y * c, v.z];
        for (axis, value) in local.iter_mut().enumerate() {
            match (self.flags >> (4 * (axis + 1))) & 0xF {
                1 => *value = value.max(0.0),
                2 => *value = value.min(0.0),
                3 => *value = 0.0,
                5 => *value = top[axis],
                6 => *value = -top[axis],
                _ => {}
            }
        }
        local
    }
}

/// A cubic from one point to the next (`Wizard.dll:0x10002c80`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Segment {
    from: Vec3,
    from_velocity: Vec3,
    to: Point,
    start_ms: f64,
}

impl Segment {
    /// Position and velocity at `t_ms`: a Hermite cubic over the segment's time.
    fn at(&self, t_ms: f64) -> (Vec3, Vec3) {
        let span = ((self.to.time_ms - self.start_ms) / 1000.0).max(1e-3) as f32;
        let u = (((t_ms - self.start_ms) / 1000.0) as f32 / span).clamp(0.0, 1.0);
        let (p0, p1) = (self.from, self.to.position);
        let (m0, m1) = (self.from_velocity * span, self.to.velocity * span);
        let (u2, u3) = (u * u, u * u * u);
        let position = p0 * (2.0 * u3 - 3.0 * u2 + 1.0)
            + m0 * (u3 - 2.0 * u2 + u)
            + p1 * (-2.0 * u3 + 3.0 * u2)
            + m1 * (u3 - u2);
        let slope = p0 * (6.0 * u2 - 6.0 * u)
            + m0 * (3.0 * u2 - 4.0 * u + 1.0)
            + p1 * (-6.0 * u2 + 6.0 * u)
            + m1 * (3.0 * u2 - 2.0 * u);
        (position, slope / span)
    }
}

/// The yaw facing along `v` in x and y, the placement's sense: facing `(−sin, cos)`.
pub fn yaw_along(v: Vec3) -> Option<f32> {
    (v.x * v.x + v.y * v.y > 1e-6).then(|| (-v.x).atan2(v.y))
}

/// The points a machine follows, and the segment it is on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Wizard {
    points: VecDeque<Point>,
    stop: Option<Point>,
    segment: Option<Segment>,
    /// The velocity last written, which the next segment starts from.
    last_velocity: Vec3,
}

impl Wizard {
    /// Slot 17: points appended to those held.
    pub fn give(&mut self, points: impl IntoIterator<Item = Point>) {
        self.points.extend(points);
    }

    /// Slot 19: the stop the points end in.
    pub fn stop_at(&mut self, stop: Point) {
        self.stop = Some(stop);
    }

    /// Every point, the stop and the segment dropped: the machine holds.
    pub fn clear(&mut self) {
        self.points.clear();
        self.stop = None;
        self.segment = None;
    }

    /// Nothing left to follow at `now_ms`.
    pub fn idle(&self, now_ms: f64) -> bool {
        self.points.is_empty() && self.stop.is_none() && self.segment.is_none_or(|s| now_ms >= s.to.time_ms)
    }

    /// The Wizard's takt (`Wizard.dll:0x10001d50`) for a machine at `position` at `now_ms`,
    /// `dt_ms` since the last: a new segment once the last is due, from where the machine
    /// is and the velocity last written to the next point whose time has not passed, or
    /// the stop; with neither it holds. The velocity is the curve's at t + dt/2 and the
    /// heading its direction at t + dt.
    ///
    /// STAND-IN: docs/24-motion.md#not-established -- the Wizard's heading curve
    /// (`0x10003d80`) is not read: the heading is the curve's velocity's direction.
    pub fn takt(&mut self, now_ms: f64, position: Vec3, dt_ms: f64) -> Drive {
        if self.segment.is_none_or(|s| now_ms >= s.to.time_ms) {
            while self.points.front().is_some_and(|p| p.time_ms <= now_ms) {
                self.points.pop_front();
            }
            let next = self.points.pop_front().or_else(|| self.stop.take());
            self.segment = next.map(|to| Segment {
                from: position,
                from_velocity: self.last_velocity,
                to,
                start_ms: now_ms,
            });
        }
        let Some(segment) = self.segment else {
            self.last_velocity = Vec3::ZERO;
            return Drive::HOLD;
        };
        let (_, velocity) = segment.at(now_ms + dt_ms / 2.0);
        let (_, ahead) = segment.at(now_ms + dt_ms);
        self.last_velocity = velocity;
        Drive { velocity, heading: yaw_along(ahead).or_else(|| yaw_along(velocity)), flags: segment.to.flags }
    }
}

/// The part of a leg `span` the points move a machine along: across the ground when the
/// flags zero the z axis, as a walker's do, the ground contact holding its height; the whole
/// leg for a flyer.
fn moving(span: Vec3, flags: u32) -> Vec3 {
    if (flags >> 12) & 0xF == 3 { span.with_z(0.0) } else { span }
}

/// The points a walker hands the Wizard for a walk from `from` to `to` at `speed`,
/// starting at `now_ms`, and the stop it ends in.
///
/// STAND-IN: docs/24-motion.md#not-established -- the local path and its obstacle contours
/// are not read: the trajectory is the straight line, cut into at least three points a second
/// or more apart, each at the walk's velocity. A walker's walk is measured and timed across the
/// ground, since its points neither let it climb nor sink: a point above the ground under it,
/// as a hall way's vertex over a bridge's deck is, does not slow it.
pub fn straight_walk(from: Vec3, to: Vec3, speed: f32, now_ms: f64, flags: u32) -> (Vec<Point>, Point) {
    let span = to - from;
    let length = moving(span, flags).length();
    let speed = speed.max(MIN_WALK_SPEED);
    let direction = moving(span, flags).normalize_or_zero();
    let seconds = f64::from(length / speed);
    let count = (seconds.ceil() as usize).max(MIN_POINTS);
    let velocity = direction * speed;
    let points: Vec<Point> = (1..=count)
        .map(|k| {
            let f = k as f32 / count as f32;
            Point {
                position: from + span * f,
                velocity,
                time_ms: now_ms + seconds * 1000.0 * f64::from(f),
                flags,
            }
        })
        .collect();
    let last = points.last().copied().unwrap_or(Point { position: from, velocity, time_ms: now_ms, flags });
    let stop = Point {
        position: last.position + last.velocity * 0.5,
        velocity: Vec3::ZERO,
        time_ms: last.time_ms + STOP_AFTER_MS,
        flags: flags | 1,
    };
    (points, stop)
}

/// The points for a walk from `from` through each of `path` in turn at `speed`, starting at
/// `now_ms`, and the stop it ends in: each leg as [`straight_walk`] cuts it, one after the
/// other, the whole at least three points. With `rest` the stop is the last point itself, at
/// rest.
///
/// STAND-IN: docs/31-packages.md#not-established -- how the walker's path joins a building's
/// hall way (`MGraph`), and how it brings a unit to rest on a place in it, are not read: the
/// hall way's vertices are handed on as legs, and a way in stops on its last vertex rather than
/// half its velocity beyond it.
pub fn path_walk(
    from: Vec3,
    path: &[Vec3],
    speed: f32,
    now_ms: f64,
    flags: u32,
    rest: bool,
) -> (Vec<Point>, Point) {
    let speed = speed.max(MIN_WALK_SPEED);
    let mut points = Vec::new();
    let (mut start, mut time) = (from, now_ms);
    for &to in path {
        let span = to - start;
        let length = moving(span, flags).length();
        if length < 1e-3 {
            continue;
        }
        let seconds = f64::from(length / speed);
        let count = seconds.ceil().max(1.0) as usize;
        let velocity = moving(span, flags) / length * speed;
        points.extend((1..=count).map(|k| {
            let f = k as f32 / count as f32;
            Point {
                position: start + span * f,
                velocity,
                time_ms: time + seconds * 1000.0 * f64::from(f),
                flags,
            }
        }));
        start = to;
        time += seconds * 1000.0;
    }
    if !rest && points.len() < MIN_POINTS {
        return straight_walk(from, start, speed, now_ms, flags);
    }
    let Some(&last) = points.last() else {
        return (
            Vec::new(),
            Point { position: from, velocity: Vec3::ZERO, time_ms: now_ms, flags: flags | 1 },
        );
    };
    if rest {
        points.pop();
        return (points, Point { velocity: Vec3::ZERO, flags: flags | 1, ..last });
    }
    let stop = Point {
        position: last.position + last.velocity * 0.5,
        velocity: Vec3::ZERO,
        time_ms: last.time_ms + STOP_AFTER_MS,
        flags: flags | 1,
    };
    (points, stop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_walk_passes_each_point_in_turn_at_its_speed() {
        let path = [Vec3::new(10.0, 0.0, 0.0), Vec3::new(10.0, 20.0, 0.0)];
        let (points, stop) = path_walk(Vec3::ZERO, &path, 5.0, 1000.0, GROUND_POINT, false);
        let corner = points.iter().find(|p| p.position == path[0]).expect("the corner is a point");
        assert_eq!((corner.time_ms, corner.velocity), (3000.0, Vec3::new(5.0, 0.0, 0.0)));
        let end = points.last().unwrap();
        assert_eq!((end.position, end.time_ms, end.velocity), (path[1], 7000.0, Vec3::new(0.0, 5.0, 0.0)));
        assert_eq!((stop.velocity, stop.time_ms), (Vec3::ZERO, 8000.0));
        assert!(points.windows(2).all(|w| w[1].time_ms > w[0].time_ms));
        // A way in comes to rest on its last point.
        let (points, stop) = path_walk(Vec3::ZERO, &path, 5.0, 1000.0, GROUND_POINT, true);
        assert_eq!((stop.position, stop.velocity, stop.time_ms), (path[1], Vec3::ZERO, 7000.0));
        assert!(points.last().is_some_and(|p| p.time_ms < 7000.0));
    }

    #[test]
    fn a_walkers_leg_to_a_point_above_the_ground_keeps_its_speed_across_the_ground_and_a_flyers_climbs() {
        // A hall-way vertex 4.3 m over the deck, 0.65 m ahead, then one 21.6 m on.
        let from = Vec3::new(0.0, 0.0, 61.7);
        let path = [Vec3::new(0.0, 0.65, 66.0), Vec3::new(0.0, 22.25, 65.0)];
        let (points, _) = path_walk(from, &path, 6.5, 0.0, GROUND_POINT, false);
        assert!(points.iter().all(|p| p.velocity == Vec3::new(0.0, 6.5, 0.0)), "{points:?}");
        assert!((points.last().unwrap().time_ms - 22_250.0 / 6.5).abs() < 1e-3);
        // The Wizard keeps it walking forward all the way, once under way from rest.
        let mut w = Wizard::default();
        w.give(points);
        let (mut at, mut t) = (from, 0.0);
        while t < 3000.0 {
            let drive = w.takt(t, at, 1000.0 / 60.0);
            let forward = drive.velocity.y > 5.0 || t < 300.0;
            assert!(forward && drive.heading.is_some_and(|h| h.abs() < 0.1), "at {t}: {drive:?}");
            at += drive.velocity.with_z(0.0) * (1.0 / 60.0);
            t += 1000.0 / 60.0;
        }
        let (points, _) = straight_walk(from, path[0], 6.5, 0.0, 0);
        assert!(points[0].velocity.z > 6.0, "a flyer's points climb: {points:?}");
    }

    #[test]
    fn a_walk_is_held_between_two_metres_a_second_and_the_units_capped_top() {
        assert_eq!(walk_speed(80.0 * 14.0, 14.0, 0.6, 1.0), 14.0, "a figure of 80 walks at full speed");
        assert_eq!(walk_speed(14.0, 14.0, 0.6, 0.7), 14.0 * 0.7);
        assert_eq!(walk_speed(0.5, 14.0, 0.6, 1.0), 2.0);
        assert_eq!(walk_speed(1000.0, 900.0, 0.0, 1.0), 600.0);
    }

    #[test]
    fn a_walker_neither_side_slips_nor_climbs_and_a_hold_stops_every_axis() {
        let d = Drive { velocity: Vec3::new(3.0, 4.0, 5.0), heading: None, flags: GROUND_POINT };
        let v = d.in_frame(0.0, [1.0; 3]);
        assert_eq!(v, [0.0, 4.0, 0.0]);
        // Turned a quarter left, world +y is the machine's +x and world +x its −y.
        let v = Drive { flags: 0, ..d }.in_frame(std::f32::consts::FRAC_PI_2, [1.0; 3]);
        assert!((v[0] - 4.0).abs() < 1e-5 && (v[1] + 3.0).abs() < 1e-5 && v[2] == 5.0, "{v:?}");
        assert_eq!(Drive { flags: HOLD_POINT, ..d }.in_frame(0.3, [1.0; 3]), [0.0; 3]);
    }

    #[test]
    fn the_wizard_follows_a_straight_walk_to_its_stop_and_then_holds() {
        let (points, stop) = straight_walk(Vec3::ZERO, Vec3::new(0.0, 100.0, 0.0), 10.0, 0.0, 0);
        assert_eq!(points.len(), 10);
        assert_eq!((stop.position, stop.time_ms), (Vec3::new(0.0, 105.0, 0.0), 11_000.0));
        let mut w = Wizard::default();
        w.give(points);
        w.stop_at(stop);
        let mut at = Vec3::ZERO;
        let mut t = 0.0;
        let dt = 1000.0 / 60.0;
        while t < 13_000.0 {
            let drive = w.takt(t, at, dt);
            if (2000.0..9000.0).contains(&t) {
                assert!((drive.velocity.y - 10.0).abs() < 0.5, "at {t}: {}", drive.velocity);
                assert!(drive.heading.is_some_and(|h| h.abs() < 1e-3), "heading up +y");
            }
            at += drive.velocity * (dt / 1000.0) as f32;
            t += dt;
        }
        assert!((at.y - 105.0).abs() < 2.0, "it ends near the stop: {at}");
        assert!(w.idle(t) && w.takt(t, at, dt) == Drive::HOLD);
    }
}
