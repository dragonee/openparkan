//! The wingman panel at the top left (`iron3d.dll:0x100431a0`): a line for each wingman
//! whenever the driven unit has one, and while the selector orders, the order menu under it.
//! See `docs/31-packages.md`, "The wingman menu from first person".

use super::commander::{ICON_PIECE, icon, life_bar, put, unit_icons};
use super::weapons::LAMPS;
use super::{Cockpit, Ink, WHITE};
use crate::hud::{LAYOUT, Pin};
use crate::play::Play;

/// A line's height and how far apart the lines stand, from (0, 0) (`0x1004321c`).
pub const LINE_HEIGHT: f32 = 19.0;
/// A line's number box and its name bar (`0x1009d990`).
pub const NUMBER_BOX: f32 = 12.0;
pub const NAME_BAR: f32 = 135.0;
/// The menu's rows: 19 apart from (220, 50) (`0x1007aaa0`), each a number box and a bar
/// (`0x1009c830`).
pub const MENU_X: f32 = 220.0;
pub const MENU_TOP: f32 = 50.0;
pub const ORDER_BAR: f32 = 150.0;
/// A row's whole width: its ending, number box, emitter, order bar and ray ending.
pub const MENU_WIDTH: f32 = 5.0 + NUMBER_BOX + 10.0 + ORDER_BAR + 6.0;
/// The text of a line not chosen and of a disabled row.
pub const GREY: u32 = 0xff80_8080;
/// The emitters, by variant: off, normal, pressed.
pub const EMITTERS: [&str; 3] =
    ["ccres_ray_emitter_off", "ccres_ray_emitter_normal", "ccres_ray_emitter_pressed"];

/// How a line looks (`0x1009d9ed`–`0x1009da35`): its lamp and emitter variants, whether its
/// icons' tint is halved, and its text's colour.
pub fn look(chosen: bool, picking: bool) -> (usize, usize, bool, u32) {
    match (chosen, picking) {
        (true, _) => (3, 2, false, WHITE),
        (false, true) => (2, 0, false, GREY),
        (false, false) => (4, 0, true, GREY),
    }
}

/// A tint halved, kept opaque: (tint >> 1) & `0x7f7f7f` | `0xff000000`.
pub fn halved(tint: u32) -> u32 {
    ((tint >> 1) & 0x007f_7f7f) | 0xff00_0000
}

/// `name` from the pen at `y` across `width`, the pen moved past it; returns where it started.
fn step(cockpit: &Cockpit, ink: &mut Ink, name: &str, pen: &mut f32, y: f32, width: f32) -> f32 {
    let x = *pen;
    put(cockpit, ink, name, [x, y, x + width, y + LINE_HEIGHT], WHITE);
    *pen += width;
    x
}

/// The panel for `play`: the wingman lines pinned to the screen's top left, the menu's rows
/// under them across the top middle.
pub fn draw(cockpit: &Cockpit, ink: &mut Ink, play: &Play) {
    let Some(panel) = play.panel() else { return };
    let pin = std::mem::replace(&mut ink.painter.pin, Pin::TOP_LEFT);
    let text_down = ((LINE_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    for (i, &(number, t, chosen)) in panel.wingmen.iter().enumerate() {
        let y = LINE_HEIGHT * i as f32;
        let (lamp, emitter, dim, text) = look(chosen, panel.picking);
        let pen = &mut 0.0;
        step(cockpit, ink, "ccres_ending_text", pen, y, 5.0);
        let x = step(cockpit, ink, "ccres_body_text", pen, y, NUMBER_BOX);
        ink.centred(&number.to_string(), x, NUMBER_BOX, y + text_down, text);
        step(cockpit, ink, LAMPS[lamp], pen, y, 19.0);
        let (cells, tint) = unit_icons(play, t);
        let tint = if dim { halved(tint) } else { tint };
        for cell in cells {
            let x = step(cockpit, ink, "ccres_body_text", pen, y, ICON_PIECE);
            if let Some(cell) = cell {
                icon(cockpit, ink, cell, [x + 2.0, y + 2.0], tint);
            }
        }
        step(cockpit, ink, "ccres_separator_left_text", pen, y, 5.0);
        step(cockpit, ink, EMITTERS[emitter], pen, y, 10.0);
        let bar = *pen;
        life_bar(cockpit, ink, play, t, [bar, y, bar + NAME_BAR]);
        let name = cockpit.panels.names.get(t).cloned().unwrap_or_default();
        ink.centred(&name, bar, NAME_BAR, y + text_down, text);
        *pen += NAME_BAR;
        step(cockpit, ink, "ccres_ray_ending", pen, y, 6.0);
    }
    // MENU_X stands a row's 183 all but centred across the layout's 640, which is the shape
    // every screen the game ran on had. Pinned to the top left with the lines above them the
    // rows slide off to one side of a wider window, so they are pinned to the top and centred
    // across it, as the rest of the HUD keeps its pin to the screen's edges; `--stretch-hud`,
    // which draws the layout as the game does, leaves them where it puts them (docs/35, "How
    // the radar draws").
    ink.painter.pin = Pin::TOP;
    let menu_x = if ink.painter.space.stretch { MENU_X } else { ((LAYOUT[0] - MENU_WIDTH) / 2.0).round() };
    for (n, &(string, enabled)) in panel.rows.iter().enumerate() {
        let y = MENU_TOP + LINE_HEIGHT * n as f32;
        let text = if enabled { WHITE } else { GREY };
        let pen = &mut { menu_x };
        step(cockpit, ink, "ccres_ending_text", pen, y, 5.0);
        let x = step(cockpit, ink, "ccres_body_text", pen, y, NUMBER_BOX);
        ink.centred(&(n + 1).to_string(), x, NUMBER_BOX, y + text_down, text);
        step(cockpit, ink, EMITTERS[1], pen, y, 10.0);
        let x = step(cockpit, ink, "ccres_ray_body", pen, y, ORDER_BAR);
        let words = cockpit.string(string).to_owned();
        ink.centred(&words, x, ORDER_BAR, y + text_down, text);
        step(cockpit, ink, "ccres_ray_ending", pen, y, 6.0);
    }
    ink.painter.pin = pin;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_line_is_lit_a_picked_one_waits_yellow_and_the_rest_are_dim() {
        assert_eq!(look(true, false), (3, 2, false, WHITE));
        assert_eq!(look(true, true), (3, 2, false, WHITE));
        assert_eq!(look(false, true), (2, 0, false, GREY));
        assert_eq!(look(false, false), (4, 0, true, GREY));
        assert_eq!(halved(0xff80_ff80), 0xff40_7f40);
        assert_eq!(halved(0xffff_e7ff), 0xff7f_737f);
    }
}
