//! The satellite map (`iron3d.dll:0x10072f90`, drawn by `0x10073750`): the mission's minimap,
//! tinted green and see-through, at the top right where the weapons list was, with a compass
//! and marks for the player's units. M opens and closes it, ] and [ change how opaque it is.
//! See `docs/35-hud.md`, "The satellite map".

use glam::Vec3;
use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT};
use parkan_sim::orders::SHUTDOWN;

use super::{Cockpit, Ink, argb, messages};
use crate::hud::{Blend, MINIMAP, Pin};
use crate::play::{BUILDING_BRIDGE, BUILDING_RUINE, Play};

/// The page the buildings' icons are cut from, a cell's side there, and the side a mark is
/// drawn at about the building's place (docs/35, "The marks").
pub const ICONS: &str = "icons";
pub const ICON_CELL: f32 = 24.0;
pub const BUILDING_MARK: f32 = 20.0;

/// The panel in the cockpit, (x₀, y₀)–(x₁, y₁) on the 640 × 480 screen (`+0x1bc`).
pub const PANEL: [f32; 4] = [374.0, 0.0, 640.0, 266.0];
/// The minimap's inset from the panel, and its size.
pub const INSET: f32 = 5.0;
pub const SIDE: f32 = 256.0;
/// The minimap's tint under its alpha (`0x10074940`), and the alpha's limits and step.
pub const TINT: u32 = 0x0037_ff37;
pub const ALPHA_DEFAULT: i32 = 128;
pub const ALPHA_MIN: i32 = 30;
pub const ALPHA_MAX: i32 = 255;
pub const ALPHA_STEP: i32 = 12;
/// How long the alpha's label shows after a change (`+0x25c`).
pub const LABEL_MS: f64 = 1000.0;
pub const LABEL_BOX: u32 = 0xff00_7300;
/// `map_compass_icon`'s size, from `ui/hq.cfg`.
pub const COMPASS: [f32; 2] = [21.0, 42.0];
/// The hero's outline and heading line when it is not selected, and the line's length over
/// the mark's size.
pub const HERO_GREEN: u32 = 0xff00_ff00;
pub const HEADING_SHARE: f32 = 1.8;

/// The map's state: open (`+0x261`), its alpha (`+0x90`, `Iron_3D.ini`'s `[CS] MAP_ALPHA`) and
/// when the alpha last changed; and the object the cursor points at on it.
#[derive(Clone, Debug, PartialEq)]
pub struct SatelliteMap {
    pub open: bool,
    pub alpha: i32,
    changed_ms: Option<f64>,
    /// The object of the pick under the cursor when that pick came from the map: the cursor
    /// object at game `+0x24` with its `+0x1c` set (docs/42, "The pick under the cursor").
    /// The line under the commander's map names it; `None` when the pick came from the world
    /// or found nothing.
    pub pointed: Option<usize>,
}

impl Default for SatelliteMap {
    fn default() -> Self {
        Self { open: false, alpha: ALPHA_DEFAULT, changed_ms: None, pointed: None }
    }
}

impl SatelliteMap {
    /// The map with `MAP_ALPHA`, held to 30–255.
    pub fn new(alpha: Option<i32>) -> Self {
        Self { alpha: alpha.unwrap_or(ALPHA_DEFAULT).clamp(ALPHA_MIN, ALPHA_MAX), ..Self::default() }
    }

    /// M (`0x100724dd`).
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if !self.open {
            self.changed_ms = None;
        }
    }

    /// ] or [ while it is open (`0x10074550`, `0x100745b0`): more or less opaque by 12.
    pub fn step_alpha(&mut self, up: bool, now_ms: f64) {
        if !self.open {
            return;
        }
        let step = if up { ALPHA_STEP } else { -ALPHA_STEP };
        self.alpha = (self.alpha + step).clamp(ALPHA_MIN, ALPHA_MAX);
        self.changed_ms = Some(now_ms);
    }

    /// The alpha's label: its share of 255 in whole fives, and a percent sign.
    pub fn label(&self) -> String {
        format!("{}%", self.alpha * 100 / 255 / 5 * 5)
    }
}

/// Where a world point lands on the map (`0x100741a0`), for a map `side` long: the whole map,
/// north up, one texel a unit of 256.
pub fn map_point(world: Vec3, side: f32) -> [f32; 2] {
    let side = side.max(1.0);
    let (x, y) = (world.x.round(), world.y.round());
    [PANEL[0] + INSET + (SIDE * x / side).round(), PANEL[3] - INSET - (SIDE * y / side).round()]
}

/// The map, while it is open, in the cockpit's panel.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, now_ms: f64) {
    draw_in(cockpit, ink, play, now_ms, PANEL, false);
}

/// Where the commander's variant starts its title bar's pen (`0x1007390c`, `0x10073914`), the
/// text box's width after the left `ending_text` (`0x1007392d`), and the title's colour
/// (`0x10073921`).
pub const TITLE_AT: [f32; 2] = [374.0, 43.0];
pub const TITLE_BOX: f32 = 226.0;
pub const TITLE_COLOUR: u32 = 0xff37_ff37;
/// The compound pieces' widths the title bar steps the pen by, and their height, from
/// `ui/compaund.cfg`: `ending_text` 5, `exit_button_*` 35, all 19 tall.
pub const ENDING: f32 = 5.0;
pub const EXIT_WIDE: f32 = 35.0;
pub const PIECE_TALL: f32 = 19.0;
/// The exit button's rectangle, the object's `+0x230` (`+0x238`-`+0x248`), which the draw
/// stores from the pen after the title (`0x10073968`-`0x1007399b`): 35 wide and 20 tall. A
/// click in it closes the map (the column's click, `0x10084362`-`0x1008437d`), and the cursor
/// on it shows the tooltip 6169 *Close* (`+0x24c`).
pub const EXIT: [f32; 4] = [
    TITLE_AT[0] + ENDING + TITLE_BOX,
    TITLE_AT[1],
    TITLE_AT[0] + ENDING + TITLE_BOX + EXIT_WIDE,
    TITLE_AT[1] + 20.0,
];
/// The exit button's variant 1, `exit_button_normal`, tints its icon so (`0x1009a2ad`); the
/// icon stands 15 in and 3 down (`0x1009a33d`-`0x1009a34c`).
pub const EXIT_ICON_TINT: u32 = 0xfff0_f0f0;
pub const EXIT_ICON_AT: [f32; 2] = [15.0, 3.0];

/// The commander's title bar (`0x10073830`, before the panel): from (374, 43) `ending_text`,
/// a `body_text` box 226 wide with 5074 *Satellite map* centred in `#37ff37`, a second
/// `ending_text` mirrored, drawn right to left from 5 past the box's end so that it caps the
/// box, and the exit button at the pen, the box's end, in its `normal` variant with
/// `exit_icon`. Each piece is the pen primitive's: the ends `0x10099a30`, the box
/// `0x10099f60`, the button `0x1009a260`.
fn title_bar(cockpit: &Cockpit, ink: &mut Ink) {
    let put = |ink: &mut Ink, name: &str, rect: [f32; 4], colour: u32| {
        if let Some(p) = cockpit.skin.get(name) {
            ink.painter.piece(p, rect, argb(colour));
        }
    };
    let [x, y] = TITLE_AT;
    let bottom = y + PIECE_TALL;
    put(ink, "ccres_ending_text", [x, y, x + ENDING, bottom], super::WHITE);
    let box_left = x + ENDING;
    put(ink, "ccres_body_text", [box_left, y, box_left + TITLE_BOX, bottom], super::WHITE);
    let title = cockpit.string(5074).to_owned();
    let down = ((PIECE_TALL - ink.font.line_height.round()) / 2.0).floor();
    ink.centred(&title, box_left, TITLE_BOX, y + down, TITLE_COLOUR);
    // Direction 1: the pen moves 5 on, and the end runs back from it, mirrored.
    let pen = box_left + TITLE_BOX + ENDING;
    put(ink, "ccres_ending_text", [pen, y, pen - ENDING - 1.0, bottom], super::WHITE);
    let [x0, y0, x1, _] = EXIT;
    put(ink, "ccres_exit_button_normal", [x0, y0, x1, y0 + PIECE_TALL], super::WHITE);
    let [ix, iy] = [x0 + EXIT_ICON_AT[0], y0 + EXIT_ICON_AT[1]];
    put(ink, "exit_icon", [ix, iy, ix + 13.0, iy + 13.0], EXIT_ICON_TINT);
}

/// Where the line under the commander's map starts its pen (`0x10073a1e`, `0x10073c38`,
/// `0x10073e15`), its bar's width (`0x10073bea`, `0x10073dca`, `0x10073f6a`), and the empty
/// text boxes a building's line and an empty line draw (`0x10073b1e`, `0x10073e89`). Every
/// kind of line puts its emitter at x 436: 5 + 19 × 3, or 5 + 57 (*derived*).
pub const LINE_AT: [f32; 2] = [374.0, 330.0];
pub const LINE_BAR: f32 = 183.0;
pub const LINE_BUILDING_BOX: f32 = 19.0;
pub const LINE_EMPTY_BOX: f32 = 57.0;
/// The line's clan colour (`0x10073a3a`-`0x10073a65`, `0x10073c56`-`0x10073c84`): red where the
/// player's clan's word towards the object's clan is 0, hostile (`0x10039440`); grey where it
/// is 1, neutral (`0x10039460`); light blue for any other, the player's own clan (2 towards
/// itself) and an ally. Both tests ask the player's clan record's SuperAI, slot 8, for its word
/// (docs/25, "Clan relations").
pub const LINE_HOSTILE: u32 = 0xffff_0000;
pub const LINE_NEUTRAL: u32 = 0xff80_8080;
pub const LINE_OTHER: u32 = 0xff80_80ff;

/// The line's clan colour for the player's clan's relation `word` towards the object's clan.
pub fn line_colour(word: Option<u32>) -> u32 {
    match word {
        Some(0) => LINE_HOSTILE,
        Some(1) => LINE_NEUTRAL,
        _ => LINE_OTHER,
    }
}

/// What the line under the commander's map is drawn for (`0x100739e7`-`0x10073a16`,
/// `0x10073c1a`-`0x10073c32`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointed {
    /// A building, its node class 3, not a ruin.
    Building(usize),
    /// Any other object but an animal: a robot, or the hero.
    Unit(usize),
    /// Nothing under the cursor on the map, an animal or a ruin.
    Nothing,
}

/// What the line names for the object the cursor points at on the map.
pub fn pointed(play: &Play, object: Option<usize>) -> Pointed {
    let Some(t) = object else { return Pointed::Nothing };
    if play.is_hero(t) {
        return Pointed::Unit(t);
    }
    match play.units.get(t) {
        Some(u) if u.kind == KIND_BUILDING && u.type_word != BUILDING_RUINE => Pointed::Building(t),
        Some(u) if u.kind == KIND_UNIT && u.type_word != super::panels::TYPE_ANIMAL => Pointed::Unit(t),
        _ => Pointed::Nothing,
    }
}

/// The line under the commander's map (`0x10073830`, from `0x100739e7`), naming what the cursor
/// points at on the map, pieces drawn with the pen from (374, 330) (docs/35, "The commander's
/// variant"):
/// - **a building**: `ending_text`; an icon piece with its clan's sign in the clan colour; an
///   icon piece with its building icon in its size's tint; an empty box 19 wide; the end
///   mirrored back over the box's last 6 (`0x10073b4a`-`0x10073b6d`); `ray_emitter_off`; a bar
///   183 wide with its name over its life; `ray_ending`;
/// - **a unit**: the same with its two icons in place of the building's and no box, the end
///   drawn back from 5 past the pen (`0x10073d7b`-`0x10073d9c`), and in the bar `"%s [%s]"`,
///   its name and its head order's status;
/// - **nothing**: `ending_text`, an empty box 57 wide, the end, `ray_emitter_off`, an empty
///   bar and `ray_ending`.
///
/// The clan's sign is the screens' `+0x20` table, the eight 32 × 32 cells along y 224 of
/// `icons` the unit markers draw, at the clan record's `+0x14`, which a single-player mission
/// sets to the clan's index (`0x10073a83`-`0x10073a97`; docs/25, "The unit marker's layout").
fn line_under(cockpit: &Cockpit, ink: &mut Ink, play: &Play) {
    use super::commander::{
        ICON_PIECE, building_class, building_icon, building_tint, life_bar, name_status, put, unit_icons,
    };
    let [x, y] = LINE_AT;
    let bottom = y + PIECE_TALL;
    let white = super::WHITE;
    let piece = |ink: &mut Ink, pen: f32, page: &str, source: [f32; 4], colour: u32| {
        put(cockpit, ink, "ccres_body_text", [pen, y, pen + ICON_PIECE, bottom], white);
        if let Some(&p) = cockpit.pages.get(page) {
            let inner = ICON_PIECE - 4.0;
            ink.painter.sprite_to(Blend::Alpha, p, source, [pen + 2.0, y + 2.0, inner, inner], argb(colour));
        }
    };
    let text_down = ((PIECE_TALL - ink.font.line_height.round()) / 2.0).floor();
    let what = pointed(play, cockpit.map.pointed);
    let mut pen = x;
    put(cockpit, ink, "ccres_ending_text", [pen, y, pen + ENDING, bottom], white);
    pen += ENDING;
    let object = match what {
        Pointed::Building(t) | Pointed::Unit(t) => Some(t),
        Pointed::Nothing => None,
    };
    if let Some(t) = object {
        let clan =
            if play.is_hero(t) { Some(play.player_clan) } else { play.units.get(t).and_then(|u| u.clan) };
        let colour = line_colour(clan.and_then(|c| play.word(play.player_clan, c)));
        let sign = clan.and_then(|c| usize::try_from(c).ok()).unwrap_or(0) as f32;
        let [cell, row] = [super::markers::SIGN_CELL, super::markers::SIGN_ROW];
        piece(ink, pen, ICONS, [cell * sign, row, cell, cell], colour);
        pen += ICON_PIECE;
    }
    match what {
        Pointed::Building(t) => {
            let tint = building_tint(building_class(play, t));
            match building_icon(play.units[t].type_word) {
                Some((page, [cx, cy])) => piece(ink, pen, page, [cx, cy, 15.0, 15.0], tint),
                None => put(cockpit, ink, "ccres_body_text", [pen, y, pen + ICON_PIECE, bottom], white),
            }
            pen += ICON_PIECE;
            put(cockpit, ink, "ccres_body_text", [pen, y, pen + LINE_BUILDING_BOX, bottom], white);
            pen += LINE_BUILDING_BOX;
            put(cockpit, ink, "ccres_ending_text", [pen, y, pen - ENDING - 1.0, bottom], white);
        }
        Pointed::Unit(t) => {
            let (cells, tint) = unit_icons(play, t);
            for cell in cells {
                match cell {
                    Some([cx, cy]) => piece(ink, pen, "ui_menu", [cx, cy, 15.0, 15.0], tint),
                    None => put(cockpit, ink, "ccres_body_text", [pen, y, pen + ICON_PIECE, bottom], white),
                }
                pen += ICON_PIECE;
            }
            let from = pen + ENDING;
            put(cockpit, ink, "ccres_ending_text", [from, y, from - ENDING - 1.0, bottom], white);
        }
        Pointed::Nothing => {
            put(cockpit, ink, "ccres_body_text", [pen, y, pen + LINE_EMPTY_BOX, bottom], white);
            pen += LINE_EMPTY_BOX;
            put(cockpit, ink, "ccres_ending_text", [pen, y, pen - ENDING - 1.0, bottom], white);
        }
    }
    put(cockpit, ink, "ccres_ray_emitter_off", [pen, y, pen + 10.0, bottom], white);
    pen += 10.0;
    match what {
        Pointed::Building(t) => {
            life_bar(cockpit, ink, play, t, [pen, y, pen + LINE_BAR]);
            let name = play.building_name(t, &cockpit.strings);
            ink.centred(&name, pen, LINE_BAR, y + text_down, white);
        }
        Pointed::Unit(t) => {
            life_bar(cockpit, ink, play, t, [pen, y, pen + LINE_BAR]);
            let text = name_status(cockpit, play, t);
            ink.centred(&text, pen, LINE_BAR, y + text_down, white);
        }
        Pointed::Nothing => put(cockpit, ink, "ccres_ray_body", [pen, y, pen + LINE_BAR, bottom], white),
    }
    pen += LINE_BAR;
    put(cockpit, ink, "ccres_ray_ending", [pen, y, pen + 6.0, bottom], white);
}

/// The map in `panel`: the cockpit's, or with `commander` the commander's, under its title
/// bar ([`title_bar`]), with the camera marked in yellow and the line under it naming what the
/// cursor points at on the map ([`line_under`]) (`0x10073830`, docs/35, "The satellite map").
pub fn draw_in(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    now_ms: f64,
    panel: [f32; 4],
    commander: bool,
) {
    let map = &cockpit.map;
    if !map.open {
        return;
    }
    ink.painter.pin = Pin::TOP_RIGHT;
    let [x0, y0, x1, y1] = panel;
    let shift = [x0 - PANEL[0], y0 - PANEL[1]];
    let at = |p: [f32; 2]| [p[0] + shift[0], p[1] + shift[1]];
    if commander {
        title_bar(cockpit, ink);
    }
    messages::frame(cockpit, ink, panel);
    let map = &cockpit.map;
    if let Some(&page) = cockpit.pages.get(MINIMAP) {
        let colour = argb((map.alpha as u32) << 24 | TINT);
        ink.painter.sprite_to(
            Blend::Alpha,
            page,
            [0.0, 0.0, SIDE, SIDE],
            [x0 + INSET, y0 + INSET, SIDE, SIDE],
            colour,
        );
    }
    let px = 1.0 / ink.painter.space.scale();
    if map.changed_ms.is_some_and(|at| now_ms - at <= LABEL_MS) {
        let text = map.label();
        let x = x0 + INSET + 6.0;
        let top = (y1 - INSET - 6.0 - (ink.font.line_height + 1.0)).round();
        let (w, h) = (ink.font.advance(&text), ink.font.line_height);
        ink.painter.fill(Blend::Alpha, [x - 3.0, top - 3.0, w + 5.0, h + 7.0], argb(LABEL_BOX));
        ink.text(&text, [x, top], 0xff00_0000 | TINT);
    }
    if let Some(compass) = cockpit.skin.get("map_compass_icon") {
        let (right, bottom) = (x1 - INSET - 1.0, y1 - INSET - 1.0);
        ink.painter.piece(compass, [right - COMPASS[0], bottom - COMPASS[1], right, bottom], [1.0; 4]);
    }
    if commander {
        line_under(cockpit, ink, play);
    }

    // The marks (`0x10074220`): every building the player may see by its icon, then every unit.
    let side = play.ground.world_box().1.truncate().max_element();
    let [sx, sy] = ink.painter.space.scales();
    let known = known_to_player(play);
    if let Some(&page) = cockpit.pages.get(ICONS) {
        for (t, u) in play.units.iter().enumerate() {
            if u.kind != KIND_BUILDING || !known.get(t).copied().unwrap_or(false) {
                continue;
            }
            let (Some(target), Some([cx, cy])) =
                (play.battle.combat.targets.get(t).filter(|t| t.alive), building_icon(u.type_word))
            else {
                continue;
            };
            let own_selected = u.clan == Some(play.player_clan) && play.selected.contains(&t);
            let colour = if own_selected {
                [1.0; 4]
            } else {
                let c = play.mark_colour(u.clan).map(|v| f32::from(v) / 255.0);
                [c[0], c[1], c[2], 1.0]
            };
            let [x, y] = at(map_point(target.position, side));
            let h = BUILDING_MARK / 2.0;
            ink.painter.sprite_to(
                Blend::Alpha,
                page,
                [cx, cy, ICON_CELL, ICON_CELL],
                [x - h, y - h, BUILDING_MARK, BUILDING_MARK],
                colour,
            );
        }
    }
    for (t, u) in play.units.iter().enumerate() {
        if u.kind != KIND_UNIT || u.logical_id == play.hero_id || !known.get(t).copied().unwrap_or(false) {
            continue;
        }
        let Some(target) = play.battle.combat.targets.get(t).filter(|t| t.alive) else { continue };
        let flyer = u.designation.chassis_type == 1;
        let colour = play.mark_colour(u.clan).map(|v| f32::from(v) / 255.0);
        unit_mark(
            ink,
            at(map_point(target.position, side)),
            flyer,
            [colour[0], colour[1], colour[2], 1.0],
            None,
            [sx, sy],
            px,
        );
    }
    // The hero, a walker, always outlined and with its heading line, green unselected.
    let hero = play.driven();
    let colour = play.mark_colour(Some(play.player_clan)).map(|v| f32::from(v) / 255.0);
    let forward = hero.walker.body.forward();
    unit_mark(
        ink,
        at(map_point(hero.walker.body.position, side)),
        false,
        [colour[0], colour[1], colour[2], 1.0],
        Some([forward.x, -forward.y]),
        [sx, sy],
        px,
    );
    // The command camera, in view states 2 and 4: a square with its diagonals, and a tick from
    // 2 to 7 out along its view.
    if commander {
        let yellow = argb(0xffff_ff00);
        let c = at(map_point(play.command.position, side));
        let (s2, s7) = (2.0, 7.0);
        let corners =
            [[c[0] - s2, c[1] - s2], [c[0] + s2, c[1] - s2], [c[0] + s2, c[1] + s2], [c[0] - s2, c[1] + s2]];
        for i in 0..4 {
            ink.painter.line(Blend::Alpha, corners[i], corners[(i + 1) % 4], px, yellow);
        }
        ink.painter.line(Blend::Alpha, corners[0], corners[2], px, yellow);
        ink.painter.line(Blend::Alpha, corners[1], corners[3], px, yellow);
        let (dx, dy) = (play.command.yaw.cos(), -play.command.yaw.sin());
        ink.painter.line(
            Blend::Alpha,
            [c[0] + dx * s2, c[1] + dy * s2],
            [c[0] + dx * s7, c[1] + dy * s7],
            px,
            yellow,
        );
    }
}

/// A building's icon cell on the `icons` page by its type (`0x10064f10`, type to index
/// `0x1009f4c0`), and none for the rest, the bridge and the ruin among them.
pub fn building_icon(type_word: u32) -> Option<[f32; 2]> {
    Some(match type_word {
        0x8000_0002 => [0.0, 24.0],
        0x8000_0004 => [24.0, 24.0],
        0x8000_0008 => [48.0, 0.0],
        0x8000_0010 => [72.0, 24.0],
        0x8001_0000 | 0x8002_0000 | 0x8004_0000 => [96.0, 24.0],
        0x8000_0040 => [120.0, 24.0],
        0x8000_0400 => [48.0, 24.0],
        0x8010_0000 | 0x8020_0000 => [168.0, 24.0],
        0x8000_0200 => [216.0, 24.0],
        _ => return None,
    })
}

/// Which objects the player may see on a map this frame (`0x1007e660`): the player's clan's,
/// and another clan's while one of the player's live units has it within its radar's range —
/// neither a bridge nor a ruin, and not a hostile unit shut down.
///
/// STAND-IN: docs/35-hud.md#the-panel-in-the-cockpit--read-and-seen -- the clan's id list is
/// the units' radar contacts refilled each pass; with a scan's three signatures not computed
/// (docs/25), every live object strictly within a radar's range is a contact.
pub fn known_to_player(play: &Play) -> Vec<bool> {
    let player = Some(play.player_clan);
    let mut eyes: Vec<(glam::Vec3, f32)> = play
        .robots
        .iter()
        .filter(|(t, _)| {
            play.units[*t].clan == player && play.battle.combat.targets.get(*t).is_some_and(|x| x.alive)
        })
        .map(|(_, r)| (r.walker.body.position, r.radar.range))
        .collect();
    if !play.hero.dead() {
        eyes.push((play.hero.walker.body.position, play.hero.radar.range));
    }
    play.units
        .iter()
        .enumerate()
        .map(|(t, u)| {
            if u.clan == player {
                return true;
            }
            let Some(target) = play.battle.combat.targets.get(t).filter(|x| x.alive) else { return false };
            let shut_down = play.hostile(u.clan)
                && play.robots.iter().any(|(rt, r)| *rt == t && r.order.is_some_and(|o| o.code == SHUTDOWN));
            u.type_word != BUILDING_BRIDGE
                && u.type_word != BUILDING_RUINE
                && !shut_down
                && eyes.iter().any(|(at, range)| target.position.distance(*at) < *range)
        })
        .collect()
}

/// A unit's mark at `at`: a cross for a flyer, else a square, in screen pixels about the
/// point; with `heading`, the outline and the heading line in the hero's green. The line runs
/// along the record's (`+0xd8`, −`+0xdc`): the x and y of its world matrix's second column
/// (`+4`, `+0x14`), which the unit record's takt copies each takt
/// (`0x10075761`-`0x1007576d`) -- its forward axis, local +y, in the world.
fn unit_mark(
    ink: &mut Ink,
    at: [f32; 2],
    flyer: bool,
    colour: [f32; 4],
    heading: Option<[f32; 2]>,
    [sx, sy]: [f32; 2],
    px: f32,
) {
    let even = |v: f32| ((v.round() as i32) / 2 * 2) as f32;
    let (w, h) =
        if flyer { (even(4.0 * sx), even(4.0 * sy)) } else { ((3.0 * sx).round(), (3.0 * sy).round()) };
    let (w, h) = (w * px, h * px);
    if flyer {
        ink.painter.fill(Blend::Alpha, [at[0] - w / 2.0, at[1] - px / 2.0, w, px], colour);
        ink.painter.fill(Blend::Alpha, [at[0] - px / 2.0, at[1] - h / 2.0, px, h], colour);
    } else {
        ink.painter.fill(Blend::Alpha, [at[0] - w / 2.0, at[1] - h / 2.0, w, h], colour);
    }
    let Some([dx, dy]) = heading else { return };
    let green = argb(HERO_GREEN);
    let beyond = if flyer { 2.0 } else { 1.0 } * px;
    let (x0, y0) = (at[0] - w / 2.0 - 2.0 * px, at[1] - h / 2.0 - 2.0 * px);
    let (x1, y1) = (at[0] + w / 2.0 + beyond, at[1] + h / 2.0 + beyond);
    for rect in
        [[x0, y0, x1 - x0, px], [x0, y1, x1 - x0, px], [x0, y0, px, y1 - y0], [x1, y0, px, y1 - y0 + px]]
    {
        ink.painter.fill(Blend::Alpha, rect, green);
    }
    let length = HEADING_SHARE * w;
    ink.painter.line(Blend::Alpha, at, [at[0] + dx * length, at[1] + dy * length], px, green);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_world_point_lands_on_the_whole_map_north_up() {
        assert_eq!(map_point(Vec3::ZERO, 1746.59), [379.0, 261.0]);
        assert_eq!(map_point(Vec3::new(1746.59, 1746.59, 9.0), 1746.59), [635.0, 5.0]);
        // The hero's start on Tut_1.
        assert_eq!(map_point(Vec3::new(433.0, 477.1, 14.1), 1746.59), [379.0 + 63.0, 261.0 - 70.0]);
    }

    #[test]
    fn the_alpha_steps_by_twelve_while_open_within_its_limits_and_labels_in_fives() {
        let mut m = SatelliteMap::new(Some(128));
        m.step_alpha(true, 0.0);
        assert_eq!(m.alpha, 128, "only while open");
        m.toggle();
        m.step_alpha(true, 0.0);
        assert_eq!((m.alpha, m.label().as_str()), (140, "50%"));
        for _ in 0..20 {
            m.step_alpha(true, 0.0);
        }
        assert_eq!((m.alpha, m.label().as_str()), (255, "100%"));
        for _ in 0..30 {
            m.step_alpha(false, 0.0);
        }
        assert_eq!(m.alpha, 30);
    }

    #[test]
    fn the_commanders_title_bar_ends_in_an_exit_button_at_the_maps_right_edge() {
        // The pen from (374, 43) past a 5-wide end and the 226-wide box: the exit button, the
        // `+0x230` rectangle, from 605 to the map's right edge, 20 tall down to the map's top.
        assert_eq!(EXIT, [605.0, 43.0, 640.0, 63.0]);
        assert_eq!(EXIT[2], PANEL[2]);
        assert_eq!(EXIT[3], crate::cockpit::commander::MAP_PANEL[1]);
    }
}
