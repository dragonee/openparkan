//! Tooltips (`iron3d.dll:0x1009bbc0`, the manager): each frame the game frame clears the
//! manager's text (`0x10060aac`), a widget whose rectangle holds the cursor hands it its own
//! as the interface pass draws it, and after the pass, unless the objectives screen is up, its
//! timer (`0x1009bc20`) draws the one handed last once the cursor has rested (`0x1009b970`).
//! See `docs/37-designer.md`, "A tooltip".

use std::cell::Cell;

use super::argb;
use crate::hud::{Batch, Blend, Layer, Painter, Pin, Space};
use crate::text::{GameFont, TextRun};

/// How long the cursor must rest, ms by `timeGetTime`, and how far it may wander in each axis
/// meanwhile, in window pixels (`0x1009bc42`, `0x1009bc61`).
pub const REST_MS: f64 = 250.0;
pub const STILL: f32 = 2.0;
/// The box: the text's width and the font's height, each plus 8; 16 below the cursor (12 with
/// the system's cursor), filled pale yellow and outlined black, the text black 5 in and 4 down
/// (`0x1009b9ab`–`0x1009bab2`).
pub const PAD: f32 = 8.0;
pub const BELOW: f32 = 16.0;
pub const BELOW_SYSTEM_CURSOR: f32 = 12.0;
pub const FILL: u32 = 0xfff5_f596;
pub const INK: u32 = 0xff00_0000;
pub const TEXT_AT: [f32; 2] = [5.0, 4.0];

/// The manager: the text handed this frame, and where and when the cursor last came to rest.
#[derive(Debug, Default)]
pub struct Tip {
    handed: Cell<Option<u32>>,
    rested: ([f32; 2], f64),
}

impl Tip {
    /// The frame's start: nothing handed yet (`0x10060aac`).
    pub fn clear(&self) {
        self.handed.set(None);
    }

    /// A widget on `rect` (x0, y0, x1, y1, half open) with the cursor at `cursor`, both on one
    /// layout, hands its text `string` (`0x1009c3a0` and the widgets like it).
    pub fn hand(&self, rect: [f32; 4], cursor: Option<[f32; 2]>, string: u32) {
        let [x0, y0, x1, y1] = rect;
        if cursor.is_some_and(|[x, y]| (x0..x1).contains(&x) && (y0..y1).contains(&y)) {
            self.handed.set(Some(string));
        }
    }

    /// The text handed this frame.
    pub fn handed(&self) -> Option<u32> {
        self.handed.get()
    }

    /// The timer (`0x1009bc20`) with the cursor at window pixel `cursor` at `clock_ms`: with a
    /// text handed, a move of more than 2 in either axis restarts the rest, and a rest past
    /// 250 ms gives the text to draw. With none handed nothing is stamped.
    pub fn tick(&mut self, cursor: [f32; 2], clock_ms: f64) -> Option<u32> {
        let string = self.handed.get()?;
        let ([x, y], since) = self.rested;
        if (cursor[0] - x).abs() > STILL || (cursor[1] - y).abs() > STILL {
            self.rested = (cursor, clock_ms);
            return None;
        }
        (clock_ms - since > REST_MS).then_some(string)
    }
}

/// The box of `text` for the cursor at `cursor` on the layout pinned to the top left
/// (`0x1009b970`): at the cursor and `below` under it, turned to the cursor's left where it
/// would reach the screen's right side and above it where it would reach the bottom.
pub fn place(space: Space, cursor: [f32; 2], width: f32, height: f32, below: f32) -> [f32; 4] {
    let [right, bottom] = space.layout([space.width, space.height], Pin::TOP_LEFT);
    let (w, h) = (width + PAD, height + PAD);
    let x = if cursor[0] + w < right { cursor[0] } else { cursor[0] - w - 1.0 };
    let y = if cursor[1] + below + h < bottom { cursor[1] + below } else { cursor[1] - h - 1.0 };
    [x, y, x + w, y + h]
}

/// The tooltip `text` drawn for the cursor at `cursor` (layout, top left) in `font`,
/// `TOOL_FONT`: the box in [`Layer::Tip`] and the text over it.
pub fn draw(
    space: Space,
    cursor: [f32; 2],
    text: &str,
    font: &GameFont,
    below: f32,
) -> (Vec<Batch>, TextRun) {
    let [x0, y0, x1, y1] = place(space, cursor, font.advance(text), font.line_height, below);
    let mut painter = Painter { pin: Pin::TOP_LEFT, layer: Layer::Tip, ..Painter::new(space) };
    painter.fill(Blend::Alpha, [x0, y0, x1 - x0, y1 - y0], argb(FILL));
    let w = 1.0 / space.scale();
    for (a, b) in [([x0, y0], [x1, y0]), ([x1, y0], [x1, y1]), ([x1, y1], [x0, y1]), ([x0, y1], [x0, y0])] {
        painter.line(Blend::Alpha, a, b, w, argb(INK));
    }
    let run = TextRun {
        colour: argb(INK),
        scale: space.scale(),
        ..TextRun::new(text, space.ndc([x0 + TEXT_AT[0], y0 + TEXT_AT[1]], Pin::TOP_LEFT))
    };
    (painter.batches, run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tip_shows_once_the_cursor_has_rested_within_2_pixels_for_more_than_250_ms() {
        let mut tip = Tip::default();
        // Nothing handed: nothing shows, and nothing is stamped.
        assert_eq!(tip.tick([100.0, 100.0], 1000.0), None);
        tip.hand([90.0, 90.0, 120.0, 110.0], Some([100.0, 100.0]), 1612);
        assert_eq!(tip.handed(), Some(1612));
        assert_eq!(tip.tick([100.0, 100.0], 1000.0), None, "the rest starts");
        assert_eq!(tip.tick([102.0, 98.0], 1250.0), None, "250 ms is not past");
        assert_eq!(tip.tick([102.0, 98.0], 1251.0), Some(1612));
        assert_eq!(tip.tick([103.0, 98.0], 1300.0), None, "3 pixels restart it");
        assert_eq!(tip.tick([103.0, 98.0], 1551.0), Some(1612));
        tip.clear();
        assert_eq!(tip.tick([103.0, 98.0], 2000.0), None, "the next frame hands nothing");
        // The rectangle is half open.
        tip.hand([90.0, 90.0, 120.0, 110.0], Some([120.0, 100.0]), 1612);
        assert_eq!(tip.handed(), None);
    }

    #[test]
    fn the_box_turns_left_and_up_at_the_screens_edges() {
        let space = Space::new(640.0, 480.0);
        assert_eq!(place(space, [100.0, 100.0], 40.0, 9.0, BELOW), [100.0, 116.0, 148.0, 133.0]);
        // 48 wide from 600 would reach 648: it goes left of the cursor.
        assert_eq!(place(space, [600.0, 100.0], 40.0, 9.0, BELOW), [551.0, 116.0, 599.0, 133.0]);
        // 16 + 17 under 450 reaches 483: it goes above.
        assert_eq!(place(space, [100.0, 450.0], 40.0, 9.0, BELOW), [100.0, 432.0, 148.0, 449.0]);
    }
}
