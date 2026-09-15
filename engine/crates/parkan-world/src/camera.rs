//! A unit's own camera's zoom (`iron3d.dll`: the unit record's update `0x10075c51`,
//! `CMD_JAMES_ZOOM_MODE`'s case `0x10072428`). See `docs/30-turrets.md`, "The zoom".

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
