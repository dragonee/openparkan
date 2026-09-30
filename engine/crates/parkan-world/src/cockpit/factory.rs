//! The factory screen a player's factory pod opens (view mode 5, commander page 5): the Ore
//! and Energy rows at the top right, the page header with its exit button, and the factory
//! panel with its project and the production row. See `docs/36-factory.md`, "What is drawn"
//! and "What the controls do".

use super::weapons::fill_colour;
use super::{Cockpit, Ink, WHITE, argb, messages};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// The page header's title, and the panel's lines with no project (`0x10083a6b`, `0x10097a7f`).
pub const STRING_FACTORY: u32 = 1607;
pub const STRING_NO_PROJECTS: u32 = 6240;
pub const STRING_USE_CONSTRUCTOR: u32 = 6241;
/// The resource rows' labels (`0x1006d7b0`).
pub const STRING_ORE: u32 = 5092;
pub const STRING_ENERGY: u32 = 5093;
/// The panel's box (`0x1009716c`), its fill, and its text colours.
pub const BOX: [f32; 4] = [51.0, 20.0, 369.0, 148.0];
pub const BOX_FILL: u32 = 0x8000_8000;
pub const NO_PROJECT_COLOUR: u32 = 0xffc0_c0ff;
pub const NAME_COLOUR: u32 = 0xffff_ff00;
pub const ICON_GREEN: u32 = 0xff80_ff80;
pub const ZERO_RED: u32 = 0xffff_3232;
pub const BRAIN_RED: u32 = 0xffc8_0000;
/// A button icon's colour by variant: grey, light, white (`0x1009a9e0`).
pub const ICON_VARIANTS: [u32; 3] = [0xff80_8080, 0xfff0_f0f0, 0xffff_ffff];
/// The header's exit button, and the bottom row's controls.
pub const EXIT: [f32; 4] = [334.0, 0.0, 369.0, 20.0];
pub const CONSTRUCTOR: [f32; 4] = [56.0, 150.0, 106.0, 169.0];
pub const BAR: [f32; 2] = [116.0, 252.0];
pub const BATCH: [f32; 4] = [263.0, 150.0, 313.0, 169.0];
pub const BUILD: [f32; 4] = [313.0, 150.0, 363.0, 169.0];
pub const ACTIVE: [f32; 4] = [184.0, 122.0, 216.0, 143.0];
/// Recent project i's frame: (59 + 23i, 122)–(82 + 23i, 143).
pub const RECENT_X: f32 = 59.0;
pub const RECENT_STEP: f32 = 23.0;
/// The free-bots icon, the brain icon and the free minds' number.
pub const FREE_BOTS: [f32; 4] = [279.0, 126.0, 309.0, 141.0];
pub const BRAIN: [f32; 4] = [324.0, 126.0, 339.0, 141.0];
pub const MINDS_AT: [f32; 2] = [344.0, 129.0];
/// The message box in mode 5 (docs/36, "The screens' draw in mode 5").
pub const MESSAGES_AT: [f32; 3] = [374.0, 352.0, 266.0];
/// The panel's tooltips (docs/36, "The icons in the box" and "The bottom row").
pub const STRING_RECENT_PROJECTS: u32 = 3068;
pub const STRING_CONSTRUCTED_BOT: u32 = 3069;
pub const STRING_FREE_BOTS: u32 = 6248;
pub const STRING_ORE_REQUIRED: u32 = 6249;
pub const STRING_AVAILABLE_CPUS: u32 = 3067;
pub const STRING_CONSTRUCTOR: u32 = 1511;
pub const STRING_START_BATCH: u32 = 6238;
pub const STRING_STOP_BATCH: u32 = 6239;
pub const STRING_START_PRODUCTION: u32 = 1553;
pub const STRING_STOP_PRODUCTION: u32 = 3070;
/// A displayed resource value steps a point toward its target every 50 ms, and a zero
/// blinks every 0.5 s (docs/23, "What the HUD shows").
pub const STEP_MS: f64 = 50.0;
pub const BLINK_MS: f64 = 500.0;
pub const ROW_HEIGHT: f32 = 19.0;
pub const BAR_WIDTH: f32 = 205.0;

/// What a click on the screen asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Constructor,
    Build,
    Batch,
    Recent(usize),
    Active,
    Exit,
}

/// What the screen keeps between frames: the Ore and Energy percentages as displayed, and the
/// plant whose panel was up at the last update.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    pub shown: [i32; 2],
    pub stepped_ms: f64,
    pub open: Option<usize>,
}

/// The plant whose panel is up in `play`: its own screen, or the commander's page 5 on the
/// selected factory, whose panel it draws above the rows. `page` is the commander's page.
pub fn open(play: &Play, page: u8) -> Option<usize> {
    let factory = |t: usize| play.units.get(t).is_some_and(|u| u.type_word == crate::selection::FACTORY);
    match play.mode() {
        crate::play::Mode::Factory(t) => Some(t).filter(|&t| factory(t)),
        mode if mode.commands() && page == 5 => play.selected.first().copied().filter(|&t| factory(t)),
        _ => None,
    }
}

/// The percentage the production row shows for a build's `progress`: rounded, and 0 in batch
/// at 100 (`0x1009765d`–`0x10097676`).
pub fn shown_progress(progress: f32, batch: bool) -> i32 {
    let percent = (progress * 100.0).round() as i32;
    if batch && percent == 100 { 0 } else { percent }
}

/// The progress bar's fill for `percent`, as (x, y down from the row's top, width, height)
/// and its colour. The panel hands the bar primitive the percentage it prints
/// (`0x10097709`–`0x1009772f`), and the primitive fills from its pen, drawing left to
/// right here, 2 in from the bar's left edge to `percent` of its 136 rounded, 3 in from the
/// row's top and bottom, in the colour it picks for the percentage (`0x1009a63d`–
/// `0x1009a6b4`): the weapons list's. It fills nothing at 0.
pub fn progress_fill(percent: i32) -> Option<([f32; 4], u32)> {
    if percent <= 0 {
        return None;
    }
    let width = BAR[1] - BAR[0];
    let end = BAR[0] + (percent.min(100) as f32 * width / 100.0).round();
    let start = BAR[0] + 2.0;
    Some(([start, 3.0, end - start, ROW_HEIGHT - 6.0], fill_colour(percent)))
}

fn inside([x0, y0, x1, y1]: [f32; 4], [x, y]: [f32; 2]) -> bool {
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
}

/// The control a click at the layout point `at` (the page's, pinned top left) lands on, in
/// the order the panel's handler tests them (`0x10098040`, `0x100846ac`).
pub fn click(play: &Play, target: usize, at: [f32; 2]) -> Option<Click> {
    let f = play.factories.iter().find(|f| f.target == target)?;
    if inside(EXIT, at) {
        return Some(Click::Exit);
    }
    if inside(CONSTRUCTOR, at) {
        return Some(Click::Constructor);
    }
    if inside(BUILD, at) {
        return Some(Click::Build);
    }
    if inside(BATCH, at) {
        return Some(Click::Batch);
    }
    if f.build.is_some() && inside(ACTIVE, at) {
        return Some(Click::Active);
    }
    (0..f.projects.len()).find_map(|i| {
        let x = RECENT_X + RECENT_STEP * i as f32;
        inside([x, 122.0, x + 23.0, 143.0], at).then_some(Click::Recent(i))
    })
}

/// The resource rows' targets, whole percentages: the player clan's held ore over 4,500 and
/// its net power over the map's (docs/23, "What the HUD shows").
pub fn targets(play: &Play) -> [i32; 2] {
    play.resource_rows(play.player_clan)
}

/// The Ore and Energy rows, right to left from x 640 (`0x1006d510`), their shown values
/// stepping toward their targets while drawn.
pub fn resource_rows(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, now_ms: f64) {
    // The displayed values step toward their targets while the panel is drawn.
    let goal = targets(play);
    let mut screen = std::mem::take(&mut cockpit.factory);
    while now_ms - screen.stepped_ms >= STEP_MS {
        screen.stepped_ms += STEP_MS;
        for (shown, goal) in screen.shown.iter_mut().zip(goal) {
            *shown += (goal - *shown).signum();
        }
        if screen.stepped_ms < now_ms - 1000.0 {
            screen.stepped_ms = now_ms;
        }
    }
    let blink_on = (now_ms / BLINK_MS).floor() as i64 % 2 == 0;
    let top = ink.font.line_height.round();
    let text_down = ((ROW_HEIGHT - top) / 2.0).floor();

    // The resource rows, right to left from x 640 (`0x1006d6cc`).
    ink.painter.pin = Pin::TOP_RIGHT;
    for (row, label) in [STRING_ORE, STRING_ENERGY].into_iter().enumerate() {
        let y = 21.0 * row as f32;
        let value = screen.shown[row];
        let mut pen = 640.0;
        let skin = &cockpit.skin;
        let piece = |ink: &mut Ink, pen: &mut f32, name: &str, width: f32| {
            if let Some(p) = skin.get(name) {
                ink.painter.piece(p, [*pen, y, *pen - width - 1.0, y + ROW_HEIGHT], [1.0; 4]);
            }
            *pen -= width;
            *pen
        };
        piece(ink, &mut pen, "ccres_ending_text", 5.0);
        let value_right = pen;
        let value_left = piece(ink, &mut pen, "ccres_body_text", 35.0);
        // *Seen*: the label is `#80ff80`, and both the label and the value `#ff3232` at 0; a
        // value above 0 is white.
        let zero = value == 0;
        if !zero || blink_on {
            let colour = if zero { ZERO_RED } else { WHITE };
            ink.centred(&format!("{value}%"), value_left, value_right - value_left, y + text_down, colour);
        }
        piece(ink, &mut pen, "ccres_separator_left_text", 5.0);
        piece(ink, &mut pen, "ccres_ray_emitter_off", 10.0);
        let bar_right = pen;
        let bar_left = piece(ink, &mut pen, "ccres_ray_body", BAR_WIDTH);
        if value > 0 {
            // The rows hand their bar only the percentage, and the bar primitive picks the
            // fill's colour from it (`0x1009a530`–`0x1009a54c`): the weapons list's.
            let x0 = bar_right - BAR_WIDTH * value.min(100) as f32 / 100.0;
            ink.painter.fill(
                Blend::Alpha,
                [x0, y + 3.0, bar_right - 2.0 - x0, ROW_HEIGHT - 6.0],
                argb(fill_colour(value)),
            );
        }
        let label = cockpit.string(label).to_owned();
        let colour = if zero { ZERO_RED } else { ICON_GREEN };
        ink.centred(&label, bar_left, bar_right - bar_left, y + text_down, colour);
        piece(ink, &mut pen, "ccres_ray_ending", 6.0);
    }

    cockpit.factory = screen;
}

/// The page header from (51, 0) (`0x10083aa8`): its title `title` and the exit button.
pub fn header(cockpit: &mut Cockpit, ink: &mut Ink, title: u32) {
    let text_down = ((ROW_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    // The page header from (51, 0) (`0x10083aa8`).
    ink.painter.pin = Pin::TOP_LEFT;
    let skin = &cockpit.skin;
    let put = |ink: &mut Ink, name: &str, rect: [f32; 4], colour: u32| {
        if let Some(p) = skin.get(name) {
            ink.painter.piece(p, rect, argb(colour));
        }
    };
    put(ink, "ccres_ending_text", [51.0, 0.0, 56.0, ROW_HEIGHT], WHITE);
    put(ink, "ccres_body_text", [56.0, 0.0, 334.0, ROW_HEIGHT], WHITE);
    let title = cockpit.string(title).to_owned();
    ink.centred(&title, 56.0, 278.0, text_down, WHITE);
    put(ink, "ccres_exit_button_pressed", [EXIT[0], 0.0, EXIT[2], ROW_HEIGHT], WHITE);
    put(ink, "exit_icon", [EXIT[0] + 15.0, 3.0, EXIT[0] + 28.0, 16.0], ICON_VARIANTS[2]);
}

/// The screen for the factory that is target `target` (`0x100836f0` with the column left
/// out), and the message box where mode 5 puts it.
pub fn draw(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    target: usize,
    now_ms: f64,
) -> Vec<super::designer::Preview> {
    resource_rows(cockpit, ink, play, now_ms);
    header(cockpit, ink, STRING_FACTORY);
    let previews = panel(cockpit, ink, play, target, now_ms);
    // The message box moves to (374, 352), 266 wide.
    ink.painter.pin = Pin::BOTTOM_RIGHT;
    let [left, top, width] = MESSAGES_AT;
    messages::draw_at(cockpit, ink, now_ms, left, top, width);
    previews
}

/// The factory panel for the factory that is target `target` (`0x10097490`): its box with the
/// project, its icons, and the production row.
pub fn panel(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    target: usize,
    now_ms: f64,
) -> Vec<super::designer::Preview> {
    let mut previews = Vec::new();
    let Some(f) = play.factories.iter().find(|f| f.target == target) else { return previews };
    ink.painter.pin = Pin::TOP_LEFT;
    let blink_on = (now_ms / BLINK_MS).floor() as i64 % 2 == 0;
    let text_down = ((ROW_HEIGHT - ink.font.line_height.round()) / 2.0).floor();
    // The panel's box, framed and filled (`0x100975f0`).
    let [x0, y0, x1, y1] = BOX;
    ink.painter.fill(Blend::Alpha, [x0 + 5.0, y0 + 5.0, x1 - x0 - 10.0, y1 - y0 - 10.0], argb(BOX_FILL));
    messages::frame(cockpit, ink, BOX);
    let step = ink.line_step();
    match f.shown() {
        None => {
            let (a, b) = (
                cockpit.string(STRING_NO_PROJECTS).to_owned(),
                cockpit.string(STRING_USE_CONSTRUCTOR).to_owned(),
            );
            ink.text(&a, [71.0, 31.0], NO_PROJECT_COLOUR);
            ink.text(&b, [71.0, 31.0 + step], NO_PROJECT_COLOUR);
        }
        Some(project) => {
            // The name, the designer's unit lines from the same pen (`0x1006fc00`), and the
            // unit turning in a 115 × 115 preview at (236, 22) (`0x1009e1b0`).
            ink.text(&project.name, [61.0, 31.0], NAME_COLOUR);
            let height = ink.font.line_height;
            let (first, line_step) = ((height + 3.0).round(), (height + 2.0).round());
            for (i, (label, line)) in crate::designs::BOX_LABELS.iter().zip(&project.lines).enumerate() {
                let y = 31.0 + first + line_step * i as f32;
                let label = cockpit.string(*label).to_owned();
                ink.text(&label, [61.0, y], super::designer::GREEN);
                let (value, unit) = line.rsplit_once(' ').unwrap_or((line.as_str(), ""));
                // An accepted design always has spare payload: no line is red.
                let w = ink.font.advance(value);
                ink.text(value, [193.0 - w, y], super::designer::FIGURE);
                ink.text(unit, [196.0, y], super::designer::GREEN);
            }
            if let Some((centre, radius)) = project.sphere {
                let rect = [236.0, 22.0, 115.0, 115.0];
                let space = ink.painter.space;
                let [px, py] = space.pixel([rect[0], rect[1]], Pin::TOP_LEFT);
                let [sx, sy] = space.scales();
                let viewport = [px, py, rect[2] * sx, rect[3] * sy];
                let angle =
                    (now_ms as f32 * super::designer::PREVIEW_TURN_RATE).rem_euclid(std::f32::consts::TAU);
                let (view_proj, model) =
                    super::designer::preview_camera(glam::Vec3::from_array(centre), radius, angle, viewport);
                previews.push(super::designer::Preview {
                    key: super::designer::PreviewKey {
                        kind: parkan_formats::mission::KIND_UNIT,
                        path: project.path.clone(),
                        version: 0,
                    },
                    viewport,
                    view_proj,
                    model,
                    lights: [
                        glam::Vec3::new(-1.0, 0.0, -1.0).normalize(),
                        glam::Vec3::new(1.0, 0.0, -1.0).normalize(),
                    ],
                    paint: None,
                });
            }
        }
    }
    let skin = &cockpit.skin;
    let put = |ink: &mut Ink, name: &str, rect: [f32; 4], colour: u32| {
        if let Some(p) = skin.get(name) {
            ink.painter.piece(p, rect, argb(colour));
        }
    };
    // Each piece under the cursor hands the tooltip manager its text (`+0x8f0`–`+0x914`, made
    // at `0x10096c5e`–`0x10096fce`, handed at `0x1009791e`–`0x10097fb8`).
    let (cursor, tip) = (cockpit.commander.cursor, &cockpit.tip);
    for i in 0..f.projects.len() {
        let x = RECENT_X + RECENT_STEP * i as f32;
        tip.hand([x, 122.0, x + 23.0, 143.0], cursor, STRING_RECENT_PROJECTS);
        put(ink, "short_button_frame_off", [x, 122.0, x + 23.0, 143.0], WHITE);
        let colour = if f.selected == Some(i) { WHITE } else { ICON_GREEN };
        put(ink, "project_icon", [x + 2.0, 124.0, x + 21.0, 141.0], colour);
    }
    if f.build.is_some() {
        tip.hand(ACTIVE, cursor, STRING_CONSTRUCTED_BOT);
        put(ink, "long_button_frame_off", ACTIVE, WHITE);
        let colour = if f.selected.is_none() { WHITE } else { ICON_GREEN };
        put(ink, "active_project", [ACTIVE[0] + 2.0, 124.0, ACTIVE[2] - 2.0, 141.0], colour);
    }
    let free_icon = if f.free_bots > 0 { "free_bots_icon" } else { "no_free_bots_icon" };
    put(ink, free_icon, FREE_BOTS, ICON_GREEN);
    tip.hand(FREE_BOTS, cursor, if f.free_bots > 0 { STRING_FREE_BOTS } else { STRING_ORE_REQUIRED });
    tip.hand(BRAIN, cursor, STRING_AVAILABLE_CPUS);
    let minds = play.free_minds(play.player_clan);
    if minds > 0 || blink_on {
        put(ink, "brain_icon", BRAIN, if minds == 0 { BRAIN_RED } else { ICON_GREEN });
        ink.text(&minds.to_string(), MINDS_AT, if minds == 0 { BRAIN_RED } else { ICON_GREEN });
    }

    // The bottom row from (51, 150).
    let y = 150.0;
    put(ink, "ccres_ending_stub", [51.0, y, 56.0, y + ROW_HEIGHT], WHITE);
    put(ink, "ccres_long_button_normal", [CONSTRUCTOR[0], y, CONSTRUCTOR[2], y + ROW_HEIGHT], WHITE);
    put(ink, "constructor_icon", [66.0, y + 2.0, 96.0, y + 17.0], ICON_VARIANTS[1]);
    tip.hand(CONSTRUCTOR, cursor, STRING_CONSTRUCTOR);
    put(ink, "ccres_ray_emitter_off", [106.0, y, 116.0, y + ROW_HEIGHT], WHITE);
    put(ink, "ccres_ray_body", [BAR[0], y, BAR[1], y + ROW_HEIGHT], WHITE);
    if f.build.is_some() {
        let percent = shown_progress(f.progress(), f.batch);
        if let Some(([x, dy, w, h], colour)) = progress_fill(percent) {
            ink.painter.fill(Blend::Alpha, [x, y + dy, w, h], argb(colour));
        }
        let text = format!("{percent}%");
        ink.centred(&text, BAR[0], BAR[1] - BAR[0], y + text_down, WHITE);
    }
    put(ink, "ccres_ray_emitter_off", [263.0, y, 252.0, y + ROW_HEIGHT], WHITE);
    let idle = f.idle();
    let startable = f.shown().is_some() && minds > 0;
    let (batch_icon, build_icon) =
        if idle { ("batch_build_icon", "build_icon") } else { ("batch_stop_build_icon", "stop_build_icon") };
    let (batch_on, build_on) = if idle { (startable, startable) } else { (f.batch, !f.batch) };
    let (batch_tip, build_tip) = if idle {
        (STRING_START_BATCH, STRING_START_PRODUCTION)
    } else {
        (STRING_STOP_BATCH, STRING_STOP_PRODUCTION)
    };
    tip.hand(BATCH, cursor, batch_tip);
    tip.hand(BUILD, cursor, build_tip);
    for (rect, icon, on) in [(BATCH, batch_icon, batch_on), (BUILD, build_icon, build_on)] {
        let variant = usize::from(on);
        let frame = if on { "ccres_long_button_normal" } else { "ccres_long_button_off" };
        put(ink, frame, [rect[0], y, rect[2], y + ROW_HEIGHT], WHITE);
        put(ink, icon, [rect[0] + 10.0, y + 2.0, rect[0] + 40.0, y + 17.0], ICON_VARIANTS[variant]);
    }
    put(ink, "ccres_ending_stub", [368.0, y, 362.0, y + ROW_HEIGHT], WHITE);
    previews
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cockpit::weapons::{FILL_HIGH, FILL_LOW, FILL_MIDDLE};

    #[test]
    fn the_progress_bar_fills_from_its_left_edge_in_the_bars_own_colour_for_the_percentage() {
        // Nothing at 0, which is also what an idle factory shows.
        assert_eq!(progress_fill(0), None);
        // 1% of 136 rounds to 1, so the fill's right edge, 117, stands left of its left, 118,
        // as the primitive hands the rectangle on.
        assert_eq!(progress_fill(1), Some(([118.0, 3.0, -1.0, 13.0], FILL_LOW)));
        assert_eq!(progress_fill(50), Some(([118.0, 3.0, 66.0, 13.0], FILL_MIDDLE)));
        assert_eq!(progress_fill(100), Some(([118.0, 3.0, 134.0, 13.0], FILL_HIGH)));
    }

    #[test]
    fn a_batch_at_a_hundred_percent_shows_nought() {
        assert_eq!(shown_progress(0.994, false), 99);
        assert_eq!(shown_progress(1.0, false), 100);
        assert_eq!(shown_progress(1.0, true), 0);
        assert_eq!(shown_progress(0.5, true), 50);
    }
}
