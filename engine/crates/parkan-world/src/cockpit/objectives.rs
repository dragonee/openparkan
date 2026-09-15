//! The objectives screen (`iron3d.dll:0x1006a210`, drawn by `0x1006a9d0`): the mission's
//! objectives and their states over a dimmed view. It opens by itself as a mission starts and
//! closes 7 s after it is first drawn; F12 opens and closes it. See `docs/35-hud.md`, "The
//! objectives screen".

use super::{Cockpit, Ink, WHITE, argb};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// Its strings: the headers, the footer and the three state words.
pub const STRING_PRIMARY: u32 = 3062;
pub const STRING_BONUS: u32 = 3061;
pub const STRING_CLOSE: u32 = 1017;
pub const STRING_COMPLETE: u32 = 1014;
pub const STRING_IN_PROGRESS: u32 = 1015;
pub const STRING_FAILED: u32 = 1027;
/// The key `addition.man` binds to `CMD_JAMES_MISSION_OBJ` (731).
pub const CLOSE_KEY: &str = "F12";
/// The dim over the whole screen, black at 60% (`0x1006a9d0`).
pub const DIM: u32 = 0x9900_0000;
pub const GREEN: u32 = 0xff00_ff00;
pub const COMPLETE: u32 = 0xffeb_ebeb;
pub const IN_PROGRESS: u32 = 0xff78_7878;
pub const FAILED: u32 = 0xffff_6464;
/// Where its lines go on the 640 × 480 screen, centred on x 320.
pub const CENTRE_X: f32 = 320.0;
pub const HEADER_Y: f32 = 80.0;
pub const FIRST_Y: f32 = 110.0;
pub const BONUS_HEADER_Y: f32 = 260.0;
pub const BONUS_FIRST_Y: f32 = 290.0;
pub const STEP: f32 = 20.0;
pub const FOOTER_Y: f32 = 450.0;
/// How long after its first draw a screen that opened by itself closes (`0x100e63a8`).
pub const CLOSES_AFTER_MS: f64 = 7000.0;

/// The screen's three bytes (`+0x592` up, `+0x590` armed to close, `+0x591` waiting for its
/// first draw) and the time that draw stamped (`+0x58c`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub up: bool,
    armed: bool,
    waiting: bool,
    stamped_ms: f64,
}

impl Screen {
    /// As a mission starts, not from a save (`0x1005e117`): up, and armed to close.
    pub fn open_at_start(&mut self) {
        (self.up, self.armed, self.waiting) = (true, true, true);
    }

    /// F12 (`0x1007210e`): closing disarms the timer, so a screen opened again stays up.
    pub fn toggle(&mut self) {
        self.up = !self.up;
        if !self.up {
            self.armed = false;
        }
    }

    /// Esc (`0x10070e85`).
    pub fn close(&mut self) {
        self.up = false;
        self.armed = false;
    }

    /// The timer, as a draw runs it at `now_ms` (`0x1006ab94`).
    fn tick(&mut self, now_ms: f64) {
        if !self.armed {
            return;
        }
        if self.waiting {
            self.stamped_ms = now_ms;
            self.waiting = false;
        } else if now_ms - self.stamped_ms > CLOSES_AFTER_MS {
            self.up = false;
            self.armed = false;
        }
    }
}

/// The state word and its colour (`0x1006af90`).
pub fn state_word(state: u8) -> (u32, u32) {
    match state {
        1 => (STRING_COMPLETE, COMPLETE),
        u8::MAX => (STRING_FAILED, FAILED),
        _ => (STRING_IN_PROGRESS, IN_PROGRESS),
    }
}

/// The screen, when it is up; whether it is.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, now_ms: f64) -> bool {
    if !cockpit.objectives.up {
        return false;
    }
    cockpit.objectives.tick(now_ms);
    if !cockpit.objectives.up {
        return false;
    }
    let space = ink.painter.space;
    ink.painter.pin = Pin::TOP_LEFT;
    let [x0, y0] = space.layout([0.0, 0.0], Pin::TOP_LEFT);
    let [x1, y1] = space.layout([space.width, space.height], Pin::TOP_LEFT);
    ink.painter.fill(Blend::Alpha, [x0, y0, x1 - x0, y1 - y0], argb(DIM));
    ink.painter.pin = Pin::TOP;
    let centred = |ink: &mut Ink, text: &str, y: f32, colour: u32| {
        let left = CENTRE_X - (ink.menu.advance(text) * 0.5).round();
        ink.menu_text(text, [left, y], colour);
    };
    centred(ink, cockpit.string(STRING_PRIMARY), HEADER_Y, GREEN);
    let Some(progression) = play.progression.as_ref() else { return true };
    let lines: Vec<(bool, String, u8)> = progression
        .progress
        .objectives
        .iter()
        .zip(&progression.objective_texts)
        .map(|(o, text)| (o.exempt, text.clone(), o.state))
        .collect();
    let mut row = [0.0, 0.0];
    for (exempt, text, state) in &lines {
        let (word, colour) = state_word(*state);
        let line = format!("{text} : {}", cockpit.string(word));
        let (first, k) = if *exempt { (BONUS_FIRST_Y, 1) } else { (FIRST_Y, 0) };
        centred(ink, &line, first + STEP * row[k], colour);
        row[k] += 1.0;
    }
    if lines.iter().any(|l| l.0) {
        // The bonus header halves after rounding.
        let header = cockpit.string(STRING_BONUS).to_owned();
        let left = CENTRE_X - (ink.menu.advance(&header).round() / 2.0).floor();
        ink.menu_text(&header, [left, BONUS_HEADER_Y], GREEN);
    }
    let footer = cockpit.string(STRING_CLOSE).replace("%s", CLOSE_KEY);
    centred(ink, &footer, FOOTER_Y, WHITE);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_opened_at_the_start_closes_seven_seconds_after_its_first_draw_and_f12_keeps_it() {
        let mut s = Screen::default();
        s.open_at_start();
        s.tick(100_000.0);
        s.tick(107_000.0);
        assert!(s.up, "seven seconds are not past");
        s.tick(107_001.0);
        assert!(!s.up);
        s.toggle();
        s.tick(200_000.0);
        assert!(s.up, "opened with F12 it stays");
        s.toggle();
        assert!(!s.up);
    }

    #[test]
    fn the_state_words_are_complete_failed_and_in_progress() {
        assert_eq!(state_word(1), (STRING_COMPLETE, COMPLETE));
        assert_eq!(state_word(0), (STRING_IN_PROGRESS, IN_PROGRESS));
        assert_eq!(state_word(u8::MAX), (STRING_FAILED, FAILED));
    }
}
