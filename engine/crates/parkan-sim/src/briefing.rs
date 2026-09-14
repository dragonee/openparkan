//! The briefing flythrough: a camera flying `briefing.cfg`'s waypoints while voices speak
//! and subtitles run (`iron3d.dll:0x1002f480`). Each frame it gives the eye, the point it
//! looks at, the fade and the subtitle's id, and the voices that start. See
//! `docs/21-briefing.md`, "How the player runs it" and "How the briefing is shown".

use glam::DVec3;
use parkan_formats::cfg::{NO_LOOP, Waypoint};

/// The view's horizontal angle, and what the zoom takes it to (`0x10031532`, `0x10036c90`).
pub const FOV_X: f64 = 1.04;
pub const ZOOMED_FOV_X: f64 = 0.2;
/// The view's near and far planes, as the camera is created (`0x10031537`).
pub const NEAR: f64 = 3.0;
pub const FAR: f64 = 700.0;
/// A bearing is taken as 0 when the eye is this close to the orbit's centre (`0x100305c0`).
const ORBIT_MIN: f64 = 0.0001;
/// Percentages are whole numbers times this.
const PERCENT: f64 = 0.01;

/// What the camera does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Dwell,
    Edge,
}

/// The briefing player.
#[derive(Clone, Debug, PartialEq)]
pub struct Flythrough {
    pub stops: Vec<Waypoint>,
    current: Option<usize>,
    phase: Phase,
    /// When the current stop was arrived at and left, in seconds.
    arrived: f64,
    left: f64,
    /// The eye when it left, the leaving stop's target, and the destination's.
    from: (DVec3, DVec3),
    to: (DVec3, DVec3),
    destination: Option<usize>,
    /// The spline's end tangents, `EdgeTime` × the velocities.
    tangents: (DVec3, DVec3),
    /// The velocity the last phase left.
    velocity: DVec3,
    /// The fade's and the zoom's from and to levels, 0–1.
    fades: (f64, f64),
    zooms: (f64, f64),
    /// The orbit's bearing, radius and height on arrival.
    orbit: (f64, f64, f64),
    pub eye: DVec3,
    pub look: DVec3,
    /// The black overlay's opacity, 0–1.
    pub fade: f64,
    pub zoom: f64,
    pub finished: bool,
    /// The voices whose stops were arrived at since the caller last took them.
    pub voices: Vec<String>,
}

fn level(on: bool) -> f64 {
    if on { 1.0 } else { 0.0 }
}

/// A cubic Hermite from `p0` to `p1` with end tangents `a` and `b`, at `u` (`0x1002ce90`).
pub fn hermite(p0: DVec3, p1: DVec3, a: DVec3, b: DVec3, u: f64) -> DVec3 {
    let u = u.clamp(0.0, 1.0);
    let (c2, c3) = hermite_coefficients(p0, p1, a, b);
    p0 + a * u + c2 * u * u + c3 * u * u * u
}

fn hermite_coefficients(p0: DVec3, p1: DVec3, a: DVec3, b: DVec3) -> (DVec3, DVec3) {
    let d = p1 - p0;
    (3.0 * d - 2.0 * a - b, a + b - 2.0 * d)
}

impl Flythrough {
    /// A player over `stops`, standing at the first one's camera looking at its target
    /// (`0x10030220`), until it is started.
    pub fn new(stops: Vec<Waypoint>) -> Self {
        let v = |p: [f64; 3]| DVec3::from_array(p);
        let (eye, look, fade) = stops
            .first()
            .map_or((DVec3::ZERO, DVec3::Y, 1.0), |w| (v(w.camera), v(w.target), w.fade * PERCENT));
        Self {
            finished: stops.is_empty(),
            stops,
            current: None,
            phase: Phase::Dwell,
            arrived: 0.0,
            left: 0.0,
            from: (eye, look),
            to: (eye, look),
            destination: None,
            tangents: (DVec3::ZERO, DVec3::ZERO),
            velocity: DVec3::ZERO,
            fades: (fade, fade),
            zooms: (0.0, 0.0),
            orbit: (0.0, 0.0, 0.0),
            eye,
            look,
            fade,
            zoom: 0.0,
            voices: Vec::new(),
        }
    }

    /// Where the camera goes after stop `i`: `LoopIndex` when set, else the next.
    pub fn next_stop(&self, i: usize) -> Option<usize> {
        let loop_index = self.stops.get(i)?.loop_index;
        if loop_index != NO_LOOP {
            return usize::try_from(loop_index).ok().filter(|&l| l < self.stops.len());
        }
        Some(i + 1).filter(|&n| n < self.stops.len())
    }

    /// The stop whose subtitle shows, once started.
    pub fn current(&self) -> Option<&Waypoint> {
        self.stops.get(self.current?)
    }

    /// The subtitle's id: the current stop's `TextResID`.
    pub fn text_id(&self) -> &str {
        self.current().map_or("", |w| w.text_id.as_str())
    }

    /// The horizontal angle of view at the zoom now.
    pub fn fov_x(&self) -> f64 {
        (1.0 - self.zoom) * FOV_X + ZOOMED_FOV_X * self.zoom
    }

    /// The first drawn frame, at `t` seconds: waypoint 0 is arrived at (`0x100315b0`), and
    /// its dwell and edge run at once, as an arrival's do.
    pub fn start(&mut self, t: f64) {
        if self.current.is_none() && !self.stops.is_empty() {
            self.arrive(0, t);
            self.update(t);
        }
    }

    /// Stop now, as Esc does (`0x10070e75`).
    pub fn skip(&mut self) {
        self.finished = true;
    }

    /// One frame at `t` seconds (`0x1002f480`).
    pub fn update(&mut self, t: f64) {
        let Some(mut i) = self.current else { return };
        // An arrival runs its dwell and its edge in the same frame. An edge over in that
        // same frame, which only an `EdgeTime` of 0 would be, ends the briefing there.
        let mut arrived_now = false;
        while !self.finished {
            self.fade_and_zoom(i, t);
            if self.phase == Phase::Dwell {
                let w = &self.stops[i];
                if w.wait == "flyaround" || w.wait == "flyby" {
                    self.fly_around(i, t);
                }
                let w = &self.stops[i];
                if w.wait_for_time && t - self.arrived < w.dwell {
                    return;
                }
                self.leave(i, t);
            }
            if !self.edge(i, t) {
                return;
            }
            // The edge is done: its destination is arrived at now.
            match self.destination {
                Some(d) if !arrived_now => {
                    self.arrive(d, t);
                    i = d;
                    arrived_now = true;
                }
                _ => self.finished = true,
            }
        }
    }

    /// Arriving at stop `i` (`0x1002f0d0`, `0x100305c0`): its voice, the time, the fades
    /// and zooms toward the next stop's, and the orbit about its target.
    fn arrive(&mut self, i: usize, t: f64) {
        let w = &self.stops[i];
        if !w.sound_id.is_empty() {
            self.voices.push(w.sound_id.clone());
        }
        let next = self.next_stop(i).map(|n| &self.stops[n]);
        self.fades = (w.fade * PERCENT, next.map_or(w.fade, |n| n.fade) * PERCENT);
        self.zooms = (level(w.zoom), level(next.map_or(w.zoom, |n| n.zoom)));
        let target = DVec3::from_array(w.target);
        let d = self.eye - target;
        let horizontal = d.truncate().length();
        let bearing = if horizontal < ORBIT_MIN { 0.0 } else { d.y.atan2(d.x) };
        self.orbit = (bearing, horizontal, self.eye.z);
        self.current = Some(i);
        self.phase = Phase::Dwell;
        self.arrived = t;
    }

    /// The fade and the zoom by the time since arrival (`0x10030120`).
    fn fade_and_zoom(&mut self, i: usize, t: f64) {
        let w = &self.stops[i];
        let share = |time: f64| if time > 0.0 { (t - self.arrived) / time } else { 1.0 };
        let fade = share(w.fade_time);
        self.fade = if w.edge == "jump" {
            self.fades.0
        } else if fade < 1.0 {
            self.fades.0 + (self.fades.1 - self.fades.0) * fade
        } else {
            self.fades.1
        };
        let zoom = share(w.zoom_time);
        self.zoom =
            if zoom < 1.0 { self.zooms.0 + (self.zooms.1 - self.zooms.0) * zoom } else { self.zooms.1 };
    }

    /// A `flyaround`'s eye during the dwell (`0x1002fec0`): level about the target,
    /// anticlockwise from above, a turn each `RotateTime`.
    fn fly_around(&mut self, i: usize, t: f64) {
        let w = &self.stops[i];
        let omega = if w.rotate_time != 0.0 { std::f64::consts::TAU / w.rotate_time } else { 0.0 };
        let (bearing, r, z) = self.orbit;
        let theta = bearing + omega * (t - self.arrived);
        let target = DVec3::from_array(w.target);
        self.eye = DVec3::new(target.x + r * theta.cos(), target.y + r * theta.sin(), z);
        self.look = target;
        self.velocity = DVec3::new(-omega * r * theta.sin(), omega * r * theta.cos(), 0.0);
    }

    /// Leaving stop `i` (`0x100308a0`): the edge's ends, and a spline's tangents.
    fn leave(&mut self, i: usize, t: f64) {
        let w = &self.stops[i];
        let from = (self.eye, DVec3::from_array(w.target));
        self.destination = self.next_stop(i);
        self.to = self.destination.map_or(from, |d| {
            (DVec3::from_array(self.stops[d].camera), DVec3::from_array(self.stops[d].target))
        });
        self.from = from;
        self.left = t;
        self.phase = Phase::Edge;
        if w.edge == "spline" {
            let arriving = self.destination.map_or(DVec3::ZERO, |d| self.arriving_velocity(d));
            self.tangents = (self.velocity * w.edge_time, arriving * w.edge_time);
        }
    }

    /// The velocity a spline arrives at stop `d` with (`0x10030a10`).
    fn arriving_velocity(&self, d: usize) -> DVec3 {
        let w = &self.stops[d];
        let p1 = DVec3::from_array(w.camera);
        let after = self.next_stop(d).map(|n| &self.stops[n]);
        match (w.wait.as_str(), after) {
            ("flyaround" | "flyby", _) => {
                let omega = if w.rotate_time != 0.0 { std::f64::consts::TAU / w.rotate_time } else { 0.0 };
                let r = p1 - DVec3::from_array(w.target);
                DVec3::new(-omega * r.y, omega * r.x, 0.0)
            }
            ("continuous", Some(n)) if w.edge == "spline" => {
                let a = p1 - self.from.0;
                let b = DVec3::from_array(n.camera) - p1;
                let total = self.stops[self.current.unwrap_or(0)].edge_time + w.edge_time;
                let unit = |v: DVec3| v.try_normalize().unwrap_or(DVec3::ZERO);
                let direction = unit(unit(a) + unit(b));
                if total > 0.0 { direction * (a.length() + b.length()) / total } else { DVec3::ZERO }
            }
            ("continuous", Some(n)) if w.edge == "linear" && w.edge_time > 0.0 => {
                (DVec3::from_array(n.camera) - p1) / w.edge_time
            }
            _ => DVec3::ZERO,
        }
    }

    /// The edge out of stop `i` at `t` (`0x1002f6d0`); whether it is done.
    fn edge(&mut self, i: usize, t: f64) -> bool {
        let w = &self.stops[i];
        let (s, total) = (t - self.left, w.edge_time);
        let ((p0, a0), (p1, a1)) = (self.from, self.to);
        match w.edge.as_str() {
            "jump" => {
                self.velocity = DVec3::ZERO;
                let half = total / 2.0;
                let (fi, fnext) = self.fades;
                if s < half {
                    (self.eye, self.look) = (p0, a0);
                    self.fade = fi + (1.0 - fi) * s / half;
                } else if s < total {
                    (self.eye, self.look) = (p1, a1);
                    self.fade = 1.0 + (fnext - 1.0) * (s - half) / half;
                } else {
                    (self.eye, self.look) = (p1, a1);
                    self.fade = fnext;
                }
            }
            "spline" if total > 0.0 => {
                let u = s / total;
                let (a, b) = self.tangents;
                self.eye = hermite(p0, p1, a, b, u);
                self.look = hermite(a0, a1, DVec3::ZERO, DVec3::ZERO, u);
                // As read: the tangents themselves at the ends, the curve's rate inside.
                self.velocity = if u <= 0.0 {
                    a
                } else if u >= 1.0 {
                    b
                } else {
                    let (c2, c3) = hermite_coefficients(p0, p1, a, b);
                    (a + 2.0 * c2 * u + 3.0 * c3 * u * u) / total
                };
            }
            _ => {
                if total > 0.0 && (0.0..total).contains(&s) {
                    let u = s / total;
                    self.eye = p0 + (p1 - p0) * u;
                    self.look = a0 + (a1 - a0) * u;
                } else {
                    (self.eye, self.look) = (p1, a1);
                }
                let k = if total > 0.0 { (p1 - p0).length() / total } else { 0.0 };
                self.velocity = DVec3::splat(k);
            }
        }
        s >= total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(camera: [f64; 3], target: [f64; 3], edge: &str, wait: &str, edge_time: f64) -> Waypoint {
        Waypoint {
            name: String::new(),
            camera,
            target,
            edge: edge.into(),
            wait: wait.into(),
            edge_time,
            dwell: 0.0,
            rotate_time: 0.0,
            fade_time: 0.0,
            zoom_time: 0.0,
            wait_for_text: false,
            wait_for_sound: false,
            wait_for_time: true,
            wait_for_click: false,
            text_id: String::new(),
            sound_id: String::new(),
            noise: 0.0,
            fade: 0.0,
            zoom: false,
            night_vision: false,
            loop_index: NO_LOOP,
        }
    }

    #[test]
    fn a_hermite_meets_its_ends_and_tangents() {
        let (p0, p1) = (DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0));
        let (a, b) = (DVec3::new(0.0, 5.0, 0.0), DVec3::new(3.0, 0.0, 0.0));
        assert_eq!(hermite(p0, p1, a, b, 0.0), p0);
        assert!((hermite(p0, p1, a, b, 1.0) - p1).length() < 1e-12);
        let e = 1e-6;
        assert!(((hermite(p0, p1, a, b, e) - p0) / e - a).length() < 1e-4);
        assert!(((p1 - hermite(p0, p1, a, b, 1.0 - e)) / e - b).length() < 1e-4);
    }

    #[test]
    fn a_linear_edge_belongs_to_the_stop_it_leaves_and_the_last_edge_holds_then_ends() {
        let mut f = Flythrough::new(vec![
            stop([0.0; 3], [0.0, 10.0, 0.0], "linear", "continuous", 2.0),
            stop([10.0, 0.0, 0.0], [10.0, 10.0, 0.0], "linear", "continuous", 1.0),
        ]);
        f.start(5.0);
        f.update(6.0);
        assert!((f.eye - DVec3::new(5.0, 0.0, 0.0)).length() < 1e-9);
        f.update(7.5);
        assert_eq!(f.current().unwrap().edge_time, 1.0, "at the second stop");
        assert!((f.eye - DVec3::new(10.0, 0.0, 0.0)).length() < 1e-9 && !f.finished);
        f.update(8.4);
        assert!(!f.finished);
        f.update(8.5);
        assert!(f.finished, "the last edge holds for its time, then the briefing ends");
    }

    #[test]
    fn a_jump_dips_to_black_and_cuts_at_its_middle() {
        let mut f = Flythrough::new(vec![
            stop([0.0; 3], [0.0, 1.0, 0.0], "jump", "continuous", 2.0),
            stop([50.0, 0.0, 0.0], [50.0, 1.0, 0.0], "linear", "continuous", 1.0),
        ]);
        f.start(0.0);
        f.update(0.5);
        assert_eq!((f.eye, f.fade), (DVec3::ZERO, 0.5));
        f.update(1.5);
        assert_eq!((f.eye.x, f.fade), (50.0, 0.5));
    }

    #[test]
    fn a_flyaround_turns_anticlockwise_about_its_target_once_a_rotate_time() {
        let mut orbit = stop([10.0, 0.0, 5.0], [0.0, 0.0, 0.0], "linear", "flyaround", 1.0);
        orbit.rotate_time = 4.0;
        orbit.dwell = 4.0;
        let mut f = Flythrough::new(vec![orbit, stop([0.0; 3], [0.0; 3], "linear", "continuous", 1.0)]);
        f.start(0.0);
        f.update(1.0);
        assert!((f.eye - DVec3::new(0.0, 10.0, 5.0)).length() < 1e-9, "{}", f.eye);
        assert_eq!(f.look, DVec3::ZERO);
    }

    #[test]
    fn the_fade_moves_to_the_next_stops_over_fade_time_from_arrival() {
        let mut first = stop([0.0; 3], [0.0, 1.0, 0.0], "linear", "stay", 5.0);
        first.fade = 100.0;
        first.fade_time = 2.0;
        first.sound_id = "T01_T02".into();
        let mut f =
            Flythrough::new(vec![first, stop([0.0; 3], [0.0, 1.0, 0.0], "linear", "continuous", 1.0)]);
        f.start(0.0);
        assert_eq!(f.voices, ["T01_T02"]);
        f.update(1.0);
        assert!((f.fade - 0.5).abs() < 1e-12);
        f.update(3.0);
        assert_eq!(f.fade, 0.0);
    }

    #[test]
    fn a_skip_finishes_it() {
        let mut f = Flythrough::new(vec![stop([0.0; 3], [0.0, 1.0, 0.0], "linear", "continuous", 9.0)]);
        f.start(0.0);
        f.skip();
        assert!(f.finished);
    }
}
