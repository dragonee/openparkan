//! The message box at the top (`iron3d.dll:0x1007f580`, `0x1007fad0`, `0x1007fe80`): the
//! newest message, framed, headed by whom it is from, for 20 seconds. See `docs/35-hud.md`,
//! "The message box".

use super::{Cockpit, Ink, WHITE, argb};
use crate::progress::Sender;
use crate::text::{GameFont, TextRun};

/// How long a box lives (`0x1007f4f0`).
pub const LIFETIME_MS: f64 = 20_000.0;
/// Where the box stands and how wide it is.
pub const LEFT: f32 = 230.0;
pub const TOP: f32 = 0.0;
pub const WIDTH: f32 = 182.0;
/// The text wraps to the box less this.
pub const TEXT_MARGIN: f32 = 10.0;
pub const MOST_LINES: usize = 6;
/// A truncated sixth line and its ellipsis fit this share of the wrap.
pub const TRUNCATED_SHARE: f32 = 0.95;
pub const ELLIPSIS: &str = "...";
pub const STRING_SEE_MORE: u32 = 6208;
/// The key `CMD_HELP` is bound to in `addition.man` and `ui_other.man`.
pub const HELP_KEY: &str = "F1";
pub const FILL: u32 = 0x8000_8000;
pub const LINE_COLOUR: u32 = 0xffdc_dcdc;
/// The frame's pieces: its corners 8 square, its edges 5 thick, and the fill 5 in.
pub const CORNER: f32 = 8.0;
pub const EDGE: f32 = 5.0;

struct Shown {
    sender: Sender,
    text: String,
    made_ms: f64,
}

/// The one box, if any, and whether it is shown.
#[derive(Default)]
pub struct MessageBox {
    shown: Option<Shown>,
    hidden: bool,
}

impl MessageBox {
    /// A new message replaces the box at once (`0x1007ed50`).
    pub fn show(&mut self, sender: Sender, text: String, now_ms: f64) {
        self.shown = Some(Shown { sender, text, made_ms: now_ms });
        self.hidden = false;
    }

    /// `CMD_PAGER` (`0x10071ed5`): hide the box, or show it again while it lives.
    pub fn pager(&mut self) {
        self.hidden = !self.hidden;
    }

    /// Whether a box is on screen at `now_ms`: made, not hidden and not yet 20 s old (the
    /// shown byte `0x1010c080`, which the lifetime clears, `0x1007f532`).
    pub fn on_screen(&self, now_ms: f64) -> bool {
        !self.hidden && self.shown.as_ref().is_some_and(|s| now_ms - s.made_ms <= LIFETIME_MS)
    }

    /// The text on screen and whom it is from, while a box lives.
    pub fn current(&self) -> Option<(Sender, &str)> {
        self.shown.as_ref().map(|s| (s.sender, s.text.as_str()))
    }
}

/// `text` wrapped to `width` in `font`, at most six lines; a longer text's sixth line loses
/// words until it and an ellipsis fit 0.95 of the width, and whether it was cut.
pub fn wrap(font: &GameFont, text: &str, width: f32) -> (Vec<String>, bool) {
    let mut lines = font.lines(&TextRun { wrap: Some(width), ..TextRun::new(text, [0.0; 2]) });
    if lines.len() <= MOST_LINES {
        return (lines, false);
    }
    lines.truncate(MOST_LINES);
    let last = &mut lines[MOST_LINES - 1];
    let mut words: Vec<&str> = last.split(' ').collect();
    while words.len() > 1 && font.advance(&format!("{}{ELLIPSIS}", words.join(" "))) > TRUNCATED_SHARE * width
    {
        words.pop();
    }
    *last = format!("{}{ELLIPSIS}", words.join(" "));
    (lines, true)
}

/// The box, while one lives and is shown.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, now_ms: f64) {
    draw_at(cockpit, ink, now_ms, LEFT, TOP, WIDTH);
}

/// The box at (`left`, `top`), `width` wide: the cockpit's at the top, a building screen's at
/// the bottom right (docs/36, "The screens' draw in mode 5").
pub fn draw_at(cockpit: &mut Cockpit, ink: &mut Ink, now_ms: f64, left: f32, top: f32, width: f32) {
    if cockpit.messages.shown.as_ref().is_some_and(|s| now_ms - s.made_ms > LIFETIME_MS) {
        cockpit.messages.shown = None;
    }
    let Some(shown) = cockpit.messages.shown.as_ref().filter(|_| !cockpit.messages.hidden) else { return };
    let (lines, cut) = wrap(ink.font, &shown.text, width - TEXT_MARGIN);
    let l = ink.line_step();
    let k = if cut { 2.0 } else { 1.0 };
    let n = lines.len() as f32;
    let height = (n + k) * l + (k * l / 2.0).floor() + 16.0;
    let (x0, y0, x1, y1) = (left, top, left + width, top + height);
    ink.painter.fill(
        crate::hud::Blend::Alpha,
        [x0 + EDGE, y0 + EDGE, width - 2.0 * EDGE, height - 2.0 * EDGE],
        argb(FILL),
    );
    frame(cockpit, ink, [x0, y0, x1, y1]);
    let header = cockpit.string(shown.sender.header()).to_owned();
    let header_y = y0 + 9.0;
    ink.text(&header, [x0 + 8.0, header_y], WHITE);
    let mut y = header_y + 1.5 * l;
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            y += l;
        }
        ink.text(line, [x0 + 8.0, y], LINE_COLOUR);
    }
    if cut {
        let footer = cockpit.string(STRING_SEE_MORE).replace("%s", HELP_KEY);
        ink.text(&footer, [x0 + 8.0, y + 1.5 * l], WHITE);
    }
}

/// The frame (`0x1009afa0`, `0x1009abf0`): four corners, and the edges stretched between them,
/// each as thick as its piece is tall, the bottom edge flipped and the left mirrored
/// (docs/35-hud.md, "The panel in the cockpit").
pub fn frame(cockpit: &Cockpit, ink: &mut Ink, [x0, y0, x1, y1]: [f32; 4]) {
    let get = |name: &str| cockpit.skin.get(name);
    let white = [1.0; 4];
    for (name, rect) in [
        ("ccres_frame_corner_1", [x0, y0, x0 + CORNER, y0 + CORNER]),
        ("ccres_frame_corner_2", [x1 - CORNER, y0, x1, y0 + CORNER]),
        ("ccres_frame_corner_3", [x1 - CORNER, y1 - CORNER, x1, y1]),
        ("ccres_frame_corner_4", [x0, y1 - CORNER, x0 + CORNER, y1]),
    ] {
        if let Some(p) = get(name) {
            ink.painter.piece(p, rect, white);
        }
    }
    if let Some(h) = get("ccres_frame_edge_h") {
        ink.painter.piece(h, [x0 + CORNER, y0, x1 - CORNER + 1.0, y0 + EDGE], white);
        ink.painter.piece(h, [x0 + CORNER, y1, x1 - CORNER + 1.0, y1 - EDGE], white);
    }
    if let Some(v) = get("ccres_frame_edge_v") {
        ink.painter.piece(v, [x0 + EDGE, y0 + CORNER, x0, y1 - CORNER + 1.0], white);
        ink.painter.piece(v, [x1 - EDGE, y0 + CORNER, x1, y1 - CORNER + 1.0], white);
    }
}
