//! A unit's own camera's zoom, and the outer camera that looks at the unit from beside and
//! behind it (`iron3d.dll`: the unit record's update `0x10075c51`, `CMD_JAMES_ZOOM_MODE`'s case
//! `0x10072428`, the outer camera `0x10038720`–`0x10038bdf`). See `docs/30-turrets.md`, "The
//! zoom" and "The outer camera".

use glam::Vec3;

use crate::robot::Eye;

/// The zoomed field of view, and how far the field steps toward it or back an update
/// (`0x100e5c68`, `0x100e5c70`).
pub const ZOOMED_FIELD: f32 = 0.2;
pub const ZOOM_STEP: f32 = 0.1;
/// The mouse filter's multiplier while the view is zoomed: the settings handler's slot 4
/// answer, 0.5, in place of `MOUSE_SENS` × 0.01 (`0x10061a50`, `World3D.dll:0x1000adf0`).
pub const ZOOMED_SENSITIVITY: f32 = 0.5;

/// A unit record's zoom: whether it is on (`+0xa3`), and the field now and the widest
/// (`+0x14`, `+0x10`), taken from the unit's camera at its record's first update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Zoom {
    pub on: bool,
    pub field: Option<(f32, f32)>,
}

impl Zoom {
    /// The field of view the camera is handed, `widest` before the first update.
    pub fn field(&self, widest: f32) -> f32 {
        self.field.map_or(widest, |(now, _)| now)
    }

    /// `CMD_JAMES_ZOOM_MODE` (`0x10072428`): the zoom turns on or off only while the field
    /// stands at one end, at most 0.2 or at least its widest. Whether it turned.
    pub fn toggle(&mut self, widest: f32) -> bool {
        let (now, wide) = self.field.unwrap_or((widest, widest));
        let turns = now <= ZOOMED_FIELD || now >= wide;
        if turns {
            self.on = !self.on;
        }
        turns
    }

    /// One update (`0x10075c51`): zoomed, the field steps 0.1 down while it is above 0.2;
    /// not, 0.1 up while it is below the widest. It is per update, not per second, and it
    /// stops past the end rather than on it.
    pub fn step(&mut self, widest: f32) {
        let (mut now, wide) = self.field.unwrap_or((widest, widest));
        if self.on {
            if now > ZOOMED_FIELD {
                now -= ZOOM_STEP;
            }
        } else if now < wide {
            now += ZOOM_STEP;
        }
        self.field = Some((now, wide));
    }
}

/// The outer camera's view (`0x10038b5e`, made by `0x100364a0`): its field across the frame
/// and its near plane.
pub const OUTER_FIELD: f32 = 1.3;
pub const OUTER_NEAR: f32 = 0.5;
/// Its four places in turn (`0x10038920`): the angle off straight behind the unit's look,
/// positive to its right, and how far back, in the unit's bound `r`.
pub const PLACES: [(f32, f32); 4] = [(0.4, 2.5), (0.2, 4.5), (-0.2, 4.5), (-0.4, 2.5)];
/// How far below the eye it stands, in `r`: a walker's, and a flyer's, which stands above.
pub const WALKER_DROP: f32 = 0.15;
pub const FLYER_DROP: f32 = -0.45;
/// A move's time, and the move back into the eye's (`0x10038827`); the move is done at 0.8 of
/// it (`0x100e5d14`), and until then each update takes 1.25 × the seconds since the press of
/// the way left (`0x100e5d10`).
pub const MOVE_S: f64 = 0.5;
pub const BACK_S: f64 = 0.3;
pub const DONE_SHARE: f64 = 0.8;
pub const BLEND_PER_S: f64 = 1.25;
/// The line from the eye to the camera is tested past the camera by this, once it is longer
/// than `CLEAR_FROM`, and a camera behind something stands this far off the face it meets
/// **along that face's own normal** (`0x100384d0`: `0x100e4ccc`, `0x100e5c70`, `0x100e5d0c`).
pub const CLEAR_PAST: f32 = 0.5;
pub const CLEAR_FROM: f32 = 0.1;
pub const CLEAR_OFF: f32 = 0.75;

/// The outer camera (the level's `+4`): the unit it looks at, which of its places it is at,
/// where it is going and where it is now, and its move.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Outer {
    /// The unit it looks at (`+0x38`): `Some(None)` the hero, `Some(Some(t))` target `t`; `None`
    /// while the camera is off.
    pub unit: Option<Option<usize>>,
    /// The next place a press goes to (`+0x58`): 0 to 3, and 4 back into the eye.
    step: usize,
    /// The angle, the drop and the distance back it is going to (`+0x40`–`+0x48`), and those it
    /// stands at now (`+0x4c`–`+0x54`).
    goal: [f32; 3],
    now: [f32; 3],
    /// Moving (`+0x60`), since this game time.
    moving: bool,
    since_ms: f64,
}

impl Outer {
    /// Whether the view is the outer camera's: view state 3.
    pub fn on(&self) -> bool {
        self.unit.is_some()
    }

    /// `CMD_JAMES_OUTER_CAMERA` (`0x10038b30`): a press on another unit than the one looked at
    /// starts from the eye at the first place; on the same unit it goes on to the next place,
    /// and after the fourth back into the eye. A press while the camera moves does nothing.
    pub fn press(&mut self, unit: Option<usize>, flyer: bool, now_ms: f64) {
        if self.unit != Some(unit) {
            self.unit = Some(unit);
            self.step = 0;
            self.moving = false;
        }
        if self.moving {
            return;
        }
        let drop = if flyer { FLYER_DROP } else { WALKER_DROP };
        match PLACES.get(self.step) {
            Some(&(angle, back)) => {
                if self.step == 0 {
                    self.now = [0.0; 3];
                }
                self.goal = [angle, drop, back];
                self.step += 1;
            }
            None => {
                self.goal = [0.0; 3];
                self.step = 0;
            }
        }
        self.moving = true;
        self.since_ms = now_ms;
    }

    /// The camera off (`0x10038ad0`): the view is the unit's own again.
    pub fn off(&mut self) {
        *self = Outer::default();
    }

    /// One update's move (`0x10038819`): each update takes 1.25 × the seconds since the press
    /// of the way still to go, until 0.8 of the move's time has passed, and then stands at
    /// the place; back in the eye, the camera is off.
    pub fn update(&mut self, now_ms: f64) {
        if !self.moving {
            return;
        }
        let time = if self.step == 0 { BACK_S } else { MOVE_S };
        let seconds = (now_ms - self.since_ms) / 1000.0;
        if seconds >= time * DONE_SHARE {
            self.now = self.goal;
            self.moving = false;
            if self.step == 0 {
                self.off();
            }
            return;
        }
        let t = (BLEND_PER_S * seconds) as f32;
        for k in 0..3 {
            self.now[k] = t * self.goal[k] + (1.0 - t) * self.now[k];
        }
    }

    /// Where the camera stands for a unit whose own eye is `eye` and whose bound is `r`
    /// (`0x10038799`–`0x10038816`): back along the eye's heading turned by the angle, `r` ×
    /// the distance, and `r` × the drop below the eye, looking where the eye looks. The line
    /// from the eye through the camera is then tested (`0x100384d0`): `meets(from, to)` gives
    /// the point the line first meets and that face's own normal, and a camera behind it
    /// stands 0.75 off the face along that normal (`0x100386b2`-`0x100386e2`, the world's
    /// answer `+8` being a pointer to the face normal the query has just unpacked --
    /// `Terrain.dll:0x1001a6ec`, docs/30, "What the outer camera's line meets").
    pub fn place(&self, eye: &Eye, r: f32, meets: impl Fn(Vec3, Vec3) -> Option<(Vec3, Vec3)>) -> Eye {
        let [angle, drop, back] = self.now;
        let heading = eye.forward.y.atan2(eye.forward.x) + angle;
        let mut position = Vec3::new(
            eye.position.x - heading.cos() * r * back,
            eye.position.y - heading.sin() * r * back,
            eye.position.z - r * drop,
        );
        let line = position - eye.position;
        let length = line.length();
        if length > CLEAR_FROM {
            let d = line / length;
            let end = position + d * CLEAR_PAST;
            if let Some((point, normal)) = meets(eye.position, end) {
                position = point + normal * CLEAR_OFF;
            }
        }
        Eye { position, forward: eye.forward, up: eye.up, fov_x: OUTER_FIELD, near: OUTER_NEAR }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye() -> Eye {
        Eye { position: Vec3::new(10.0, 20.0, 5.0), forward: Vec3::X, up: Vec3::Z, fov_x: 1.3, near: 0.1 }
    }

    #[test]
    fn the_zoom_steps_eleven_updates_in_and_eleven_out_and_turns_only_at_an_end() {
        let mut z = Zoom::default();
        assert!(z.toggle(1.3));
        let mut steps = 0;
        while z.field(1.3) > ZOOMED_FIELD {
            z.step(1.3);
            steps += 1;
            if steps == 5 {
                assert!(!z.toggle(1.3), "no turning mid-way");
            }
        }
        assert_eq!(steps, 11);
        assert!((z.field(1.3) - 0.2).abs() < 1e-5);
        assert!(z.toggle(1.3));
        let mut back = 0;
        while z.field(1.3) < 1.3 {
            z.step(1.3);
            back += 1;
        }
        assert_eq!(back, 11);
        z.step(1.3);
        assert!((z.field(1.3) - 1.3).abs() < 1e-5, "it stays at its widest");
    }

    #[test]
    fn the_outer_camera_goes_right_near_right_far_left_far_left_near_and_back() {
        let mut o = Outer::default();
        let mut now = 0.0;
        let mut places = Vec::new();
        for _ in 0..5 {
            o.press(None, false, now);
            // A press while it moves is let be.
            o.press(None, false, now + 100.0);
            for _ in 0..60 {
                now += 1000.0 / 60.0;
                o.update(now);
            }
            places.push(o.now);
        }
        let want = [[0.4, 0.15, 2.5], [0.2, 0.15, 4.5], [-0.2, 0.15, 4.5], [-0.4, 0.15, 2.5]];
        assert_eq!(&places[..4], &want);
        assert!(!o.on(), "back in the eye, the camera is off");
    }

    #[test]
    fn a_move_eases_out_from_the_eye_and_is_done_by_four_tenths_of_a_second() {
        let mut o = Outer::default();
        o.press(None, false, 0.0);
        o.update(200.0);
        let halfway = o.now[2];
        assert!(halfway > 0.0 && halfway < 2.5, "{halfway}");
        o.update(399.0);
        assert!(o.now[2] < 2.5 && o.moving);
        o.update(400.0);
        assert_eq!(o.now, [0.4, 0.15, 2.5]);
    }

    #[test]
    fn the_first_place_stands_behind_and_to_the_right_of_a_unit_looking_east() {
        let mut o = Outer::default();
        o.press(None, false, 0.0);
        o.update(1000.0);
        let e = o.place(&eye(), 2.0, |_, _| None);
        // Looking along +x, right is −y: back 5 × cos 0.4, 5 × sin 0.4 to the right, 0.3 down.
        assert!((e.position.x - (10.0 - 5.0 * 0.4_f32.cos())).abs() < 1e-4, "{}", e.position);
        assert!((e.position.y - (20.0 - 5.0 * 0.4_f32.sin())).abs() < 1e-4, "{}", e.position);
        assert!((e.position.z - 4.7).abs() < 1e-4);
        assert_eq!((e.forward, e.fov_x, e.near), (Vec3::X, OUTER_FIELD, OUTER_NEAR));
    }

    #[test]
    fn a_wall_between_stands_the_camera_off_the_face_along_its_own_normal() {
        let mut o = Outer::default();
        o.press(None, true, 0.0);
        o.update(1000.0);
        // A plane through x = 8 whose own normal is `n`: where the line crosses it, and that
        // normal, as the world's answer carries both.
        let plane = |n: Vec3| {
            move |from: Vec3, to: Vec3| {
                let x = 8.0;
                (to.x < x).then(|| (from + (to - from) * ((from.x - x) / (from.x - to.x)), n))
            }
        };
        let e = o.place(&eye(), 2.0, plane(Vec3::X));
        assert!((e.position.x - (8.0 + CLEAR_OFF)).abs() < 1e-4, "{}", e.position);
        assert!(e.position.z > 5.0, "a flyer's camera stands above the eye");
        // A face leaning back pushes the camera up its own normal, not back along the line:
        // the height gained is 0.75 × the normal's z whatever way the line ran.
        let lean = Vec3::new(1.0, 0.0, 1.0).normalize();
        let straight = o.place(&eye(), 2.0, plane(Vec3::X));
        let leaned = o.place(&eye(), 2.0, plane(lean));
        assert!(
            (leaned.position.z - straight.position.z - CLEAR_OFF * lean.z).abs() < 1e-4,
            "{} against {}",
            leaned.position,
            straight.position
        );
        assert!((leaned.position - straight.position).y.abs() < 1e-6);
    }
}
