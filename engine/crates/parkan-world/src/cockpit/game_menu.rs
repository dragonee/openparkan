//! The game menu, `CState` mode 7 (the screens' `+0x0c`, `iron3d.dll:0x10065500`): its title
//! and four buttons in a framed box over the dimmed, paused world, drawn in place of every
//! other screen. See `docs/39-boarding.md`, "The game menu".

use super::{Cockpit, Ink, WHITE, argb, messages};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// The box on the 640 × 480 layout (`0x10065550`–`0x1006556c`), its fill inside its frame
/// (`0x10065916`, `0x1009afa0`), and the dim over the whole screen under it (`0x100658dc`).
pub const RECT: [f32; 4] = [200.0, 150.0, 440.0, 330.0];
pub const FILL: u32 = 0xcc32_8032;
pub const DIM: u32 = 0x9900_0000;
/// The title bar (`0x10066840`): from (210, 160) `ending_text`, a text box 215 wide with 5085
/// *Game Menu* in white, and `ending_text` mirrored.
pub const TITLE_AT: [f32; 2] = [210.0, 160.0];
pub const TITLE_BOX: f32 = 215.0;
pub const STRING_TITLE: u32 = 5085;
/// The compound pieces' widths the rows step the pen by, and their height (`ui/compaund.cfg`).
pub const ENDING: f32 = 5.0;
pub const LAMP: f32 = 10.0;
pub const PIECE_TALL: f32 = 19.0;
/// A button's text: `#80ff80`, white under the cursor, grey `#808080` when disabled
/// (`0x10065e6e`–`0x10065ea3`).
pub const TEXT: u32 = 0xff80_ff80;
pub const TEXT_HOVERED: u32 = WHITE;
pub const TEXT_DISABLED: u32 = 0xff80_8080;

/// What a button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// 3080 *Resume game*: the stack rolled back (`0x10065a6f`).
    Resume,
    /// 3081 *Save game*: the menu grows to (200, 150)–(440, 420) and shows its six save slots
    /// (`0x10065a9d`, page 1).
    Save,
    /// 3082 *Load game*: the game exits with code 3 (`0x10065acd`), which the shell answers with
    /// its load-game screen (docs/34, "After the outcome").
    Load,
    /// 3083 *Quit game*: exit code 1 (`0x10065af0`), back to the shell's menus.
    Quit,
}

/// The four buttons (`0x10066360`): each a widget of its string on its rectangle, 25 apart
/// (`0x100670d0`), in the order they are laid out.
pub const BUTTONS: [(Button, u32, [f32; 4]); 4] = [
    (Button::Resume, 3080, [210.0, 210.0, 430.0, 235.0]),
    (Button::Save, 3081, [210.0, 235.0, 430.0, 260.0]),
    (Button::Load, 3082, [210.0, 260.0, 430.0, 285.0]),
    (Button::Quit, 3083, [210.0, 285.0, 430.0, 310.0]),
];

/// A widget's hit test (slot 5, `0x1009b740`): x0 ≤ x < x1 and y0 ≤ y < y1.
fn inside([x0, y0, x1, y1]: [f32; 4], [x, y]: [f32; 2]) -> bool {
    (x0..x1).contains(&x) && (y0..y1).contains(&y)
}

/// Whether `button` takes a click: *Save game* and *Load game* are disabled in a network game
/// and in the training campaign (the game's `+0xe4` and `+0xe6`, `0x10066786`–`0x100667b5`);
/// the engine plays no network game.
pub fn enabled(play: &Play, button: Button) -> bool {
    !(matches!(button, Button::Save | Button::Load) && play.training)
}

/// The button under `at`, on the layout pinned to the centre, whether or not it is enabled.
pub fn under(at: [f32; 2]) -> Option<Button> {
    BUTTONS.iter().find(|(_, _, rect)| inside(*rect, at)).map(|(b, ..)| *b)
}

/// A left click at `at` (`0x10065a40`): the button it lands on if that button takes it.
/// *Resume game* and *Quit game* test no enabled flag.
pub fn click(play: &Play, at: [f32; 2]) -> Option<Button> {
    let button = under(at)?;
    match button {
        Button::Resume | Button::Quit => Some(button),
        Button::Save | Button::Load => enabled(play, button).then_some(button),
    }
}

/// The menu's draw (slot 9, `0x100658a0`): the whole screen dimmed, the box filled and framed,
/// then its main page (`0x10066840`): the title bar and the four buttons, each a pen row from
/// its rectangle's corner (`0x10065e10`) -- `lamp_text_ending`, `_pressed` under the cursor,
/// a `body_text` box its width less 10 wide with its string centred, and the lamp mirrored.
pub fn draw(cockpit: &Cockpit, ink: &mut Ink, play: &Play) {
    let space = ink.painter.space;
    ink.painter.pin = Pin::TOP_LEFT;
    let [x0, y0] = space.layout([0.0, 0.0], Pin::TOP_LEFT);
    let [x1, y1] = space.layout([space.width, space.height], Pin::TOP_LEFT);
    ink.painter.fill(Blend::Alpha, [x0, y0, x1 - x0, y1 - y0], argb(DIM));
    ink.painter.pin = Pin::CENTRE;
    let [bx0, by0, bx1, by1] = RECT;
    let edge = messages::EDGE;
    ink.painter.fill(
        Blend::Alpha,
        [bx0 + edge, by0 + edge, bx1 - bx0 - 2.0 * edge, by1 - by0 - 2.0 * edge],
        argb(FILL),
    );
    messages::frame(cockpit, ink, RECT);
    let put = |ink: &mut Ink, name: &str, rect: [f32; 4]| {
        if let Some(p) = cockpit.skin.get(name) {
            ink.painter.piece(p, rect, argb(WHITE));
        }
    };
    let down = ((PIECE_TALL - ink.font.line_height.round()) / 2.0).floor();
    // The title bar.
    let [x, y] = TITLE_AT;
    put(ink, "ccres_ending_text", [x, y, x + ENDING, y + PIECE_TALL]);
    let left = x + ENDING;
    put(ink, "ccres_body_text", [left, y, left + TITLE_BOX, y + PIECE_TALL]);
    let title = cockpit.string(STRING_TITLE).to_owned();
    ink.centred(&title, left, TITLE_BOX, y + down, WHITE);
    // Direction 1: the end drawn back from the pen, mirrored.
    let pen = left + TITLE_BOX;
    put(ink, "ccres_ending_text", [pen, y, pen - ENDING - 1.0, y + PIECE_TALL]);
    // The buttons.
    let cursor = cockpit.commander.cursor.map(|c| space.layout(space.pixel(c, Pin::TOP_LEFT), Pin::CENTRE));
    for (button, string, rect) in BUTTONS {
        let [x, y, x1, _] = rect;
        let hovered = cursor.is_some_and(|c| inside(rect, c));
        let colour = match (enabled(play, button), hovered) {
            (false, _) => TEXT_DISABLED,
            (true, true) => TEXT_HOVERED,
            (true, false) => TEXT,
        };
        let look = if hovered { "pressed" } else { "normal" };
        let lamp = format!("ccres_lamp_text_ending_{look}");
        put(ink, &lamp, [x, y, x + LAMP, y + PIECE_TALL]);
        let (left, width) = (x + LAMP, x1 - x - LAMP);
        put(ink, "ccres_body_text", [left, y, left + width, y + PIECE_TALL]);
        let text = cockpit.string(string).to_owned();
        ink.centred(&text, left, width, y + down, colour);
        let pen = left + width;
        put(ink, &lamp, [pen, y, pen - LAMP - 1.0, y + PIECE_TALL]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_buttons_stand_25_apart_inside_the_box_and_meet_their_rows() {
        for (i, (_, _, [x0, y0, x1, y1])) in BUTTONS.iter().enumerate() {
            assert_eq!((*x0, *x1), (210.0, 430.0));
            assert_eq!(*y0, 210.0 + 25.0 * i as f32);
            assert_eq!(y1 - y0, 25.0);
            assert!(*y1 <= RECT[3]);
        }
        assert_eq!(under([210.0, 210.0]), Some(Button::Resume));
        assert_eq!(under([429.9, 234.9]), Some(Button::Resume));
        assert_eq!(under([300.0, 235.0]), Some(Button::Save), "the rectangle is half open");
        assert_eq!(under([300.0, 310.0]), None);
        assert_eq!(under([209.0, 220.0]), None);
    }
}
