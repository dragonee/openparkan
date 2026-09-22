//! The markers command mode draws over units in the world (`0x1007d5e0`, `0x10077d80`): the
//! selected units and the unit under the cursor, bracketed in the colour the marking rule
//! gives, with their clan's sign and class icon, the player's named in green, and their bars.
//! See `docs/25-sensors.md`, "How the game colours what it marks".

use glam::{Mat4, Vec3, Vec4};
use parkan_formats::mission::{CLAN_NATURE, CLAN_NEUTRAL};

use super::panels::life_share;
use super::{Cockpit, Ink};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// page9's bracket, 14 × 28 at (55, 0), and the bars' frame, 27 × 16 at (0, 0) (both cut by
/// `0x100433a0`).
pub const BRACKET: [f32; 4] = [55.0, 0.0, 14.0, 28.0];
pub const FRAME: [f32; 4] = [0.0, 0.0, 27.0, 16.0];
/// The frame's tint, opaque blue (`0x10077fe1`).
pub const FRAME_TINT: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
/// The gap either side of the centre is this × the record's figure (`0x100e5c9c`).
pub const GAP: f32 = 44.0;
/// A unit record's figure for the camera distance d, its slot 5 (`0x10075650`): 2 ÷ d × 30,
/// never below 0.3.
pub const FIGURE_NEAR: f32 = 60.0;
pub const FIGURE_LEAST: f32 = 0.3;
/// A full bar's width: the life percentage × 0.25, the battery's fill × 25.
pub const BAR: f32 = 25.0;
pub const LIFE: [u8; 3] = [30, 180, 80];
pub const BATTERY: [u8; 3] = [255, 130, 50];
pub const NAME_GREEN: u32 = 0xff00_ff00;
/// The clans' signs: 32 × 32 cells of the `icons` page along y 224, by the clan's index
/// (`0x10065230`; a single-player mission gives clan i sign i, `0x100a2407`).
pub const SIGN_ROW: f32 = 224.0;
pub const SIGN_CELL: f32 = 32.0;
/// The class icons: 24 × 24 cells of the `icons` page along y 0 (`0x10064fd7`).
pub const CLASS_CELL: f32 = 24.0;
/// A sign and a class icon are drawn this big.
pub const ICON: f32 = 16.0;

fn rgba([r, g, b]: [u8; 3]) -> [f32; 4] {
    [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0]
}

/// Where the eye of `view_proj` stands: the point it takes to x = y = w = 0.
pub fn eye_of(view_proj: Mat4) -> Vec3 {
    let e = view_proj.inverse() * Vec4::new(0.0, 0.0, 1.0, 0.0);
    if e.w.abs() > 1e-9 { e.truncate() / e.w } else { Vec3::ZERO }
}

/// The gap either side of a unit's projected centre, for the camera `distance` from it:
/// 44 × max(60 ÷ d, 0.3) (`0x10077e4c`, the unit record's slot 5 `0x10075650`).
pub fn gap(distance: f32) -> f32 {
    GAP * (FIGURE_NEAR / distance.max(1e-3)).max(FIGURE_LEAST)
}

/// The class icon's x on the `icons` page by Type (`0x10078157`): a transport the crates, a
/// builder the crane, anything else the crossed swords.
pub fn class_icon(type_word: u32) -> f32 {
    match type_word {
        0x0100_2000 => 48.0,
        0x0100_4000 => 72.0,
        _ => 24.0,
    }
}

/// Every marker this frame: over the selected units and `hovered`, never the hero, through
/// `view_proj` (`0x10077d80`). About the projected centre (cx, cy) with the gap g, and
/// L = cx − g − 14:
/// - the brackets, (L, cy − 14)–(cx − g, cy + 14) and its mirror from cx + g;
/// - for a clan neither nature's nor neutral, its sign at (cx + g + 14, cy − 14) and the class
///   icon at (cx + g + 30, cy − 14), 16 × 16, and a unit of the player's clan named in green
///   at (L + 6, cy − 14 − (the font's height + 2));
/// - the bars' frame at (L, cy + 16) tinted blue, the life bar (L + 1, cy + 18)–(+ its
///   percentage × 0.25, cy + 21), the battery bar (L + 1, cy + 22)–(+ 25 × its fill, cy + 25).
pub fn draw(cockpit: &Cockpit, ink: &mut Ink, play: &Play, view_proj: Mat4, hovered: Option<usize>) {
    let space = ink.painter.space;
    let mut marked = play.selected_units();
    if let Some(t) =
        hovered.filter(|t| !marked.contains(t) && play.units[*t].kind == parkan_formats::mission::KIND_UNIT)
    {
        marked.push(t);
    }
    let eye = eye_of(view_proj);
    ink.painter.pin = Pin::TOP_LEFT;
    for t in marked {
        let (Some(target), Some(unit)) = (play.battle.combat.targets.get(t), play.units.get(t)) else {
            continue;
        };
        if !target.alive || unit.logical_id == play.hero_id {
            continue;
        }
        let Some(c) = crate::play::on_screen(view_proj, target.centre, 0.0).map(|[nx, ny]| {
            space.layout([(nx + 1.0) * 0.5 * space.width, (1.0 - ny) * 0.5 * space.height], Pin::TOP_LEFT)
        }) else {
            continue;
        };
        let g = gap(target.centre.distance(eye));
        let colour = play.mark_colour(unit.clan).map(|v| f32::from(v) / 255.0);
        let colour = [colour[0], colour[1], colour[2], 1.0];
        let [bx, by, bw, bh] = BRACKET;
        let top = c[1] - bh / 2.0;
        let left = c[0] - g - bw;
        if let Some(&page) = cockpit.pages.get("page9") {
            ink.painter.sprite_to(Blend::Alpha, page, [bx, by, bw, bh], [left, top, bw, bh], colour);
            // The right one mirrored.
            ink.painter.sprite_to(
                Blend::Alpha,
                page,
                [bx, by, bw, bh],
                [c[0] + g + bw, top, -bw, bh],
                colour,
            );
        }
        let clan = unit.clan.and_then(|k| usize::try_from(k).ok().map(|i| (i, play.clans.get(i))));
        let signed =
            clan.is_some_and(|(_, c)| c.is_some_and(|c| c.kind != CLAN_NATURE && c.kind != CLAN_NEUTRAL));
        if signed && let Some(&page) = cockpit.pages.get(super::map::ICONS) {
            let index = clan.map_or(0, |(i, _)| i) as f32;
            let sign = [SIGN_CELL * index, SIGN_ROW, SIGN_CELL, SIGN_CELL];
            ink.painter.sprite_to(Blend::Alpha, page, sign, [c[0] + g + bw, top, ICON, ICON], colour);
            let class = [class_icon(unit.type_word), 0.0, CLASS_CELL, CLASS_CELL];
            ink.painter.sprite_to(Blend::Alpha, page, class, [c[0] + g + bw + ICON, top, ICON, ICON], colour);
        }
        if signed && unit.clan == Some(play.player_clan) {
            let name = cockpit.panels.names.get(t).cloned().unwrap_or_default();
            // The font's slot 3, its header height (`Ngi32.dll:0x10010d50`), plus 2: the
            // engine's line height is that height plus 1.
            ink.text(&name, [left + 6.0, top - (ink.font.line_height + 1.0)], NAME_GREEN);
        }
        let y0 = c[1] + 16.0;
        if let Some(&page) = cockpit.pages.get("page9") {
            ink.painter.sprite_to(Blend::Alpha, page, FRAME, [left, y0, FRAME[2], FRAME[3]], FRAME_TINT);
        }
        let life = (100.0 * life_share(target.parts.iter().filter_map(|p| p.life.as_ref()))).round() * 0.25;
        ink.painter.fill(Blend::Alpha, [left + 1.0, y0 + 2.0, life, 3.0], rgba(LIFE));
        let battery = play.battery(Some(t)).unwrap_or(if unit.designation.battery { 1.0 } else { 0.0 });
        ink.painter.fill(Blend::Alpha, [left + 1.0, y0 + 6.0, BAR * battery, 3.0], rgba(BATTERY));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gap_is_44_times_60_over_the_distance_and_never_below_13_2() {
        assert!((gap(60.0) - 44.0).abs() < 1e-4);
        assert!((gap(30.0) - 88.0).abs() < 1e-4);
        assert!((gap(200.0) - 13.2).abs() < 1e-4);
        assert!((gap(1000.0) - 13.2).abs() < 1e-4);
    }

    #[test]
    fn the_eye_of_a_view_is_where_it_looks_from() {
        let eye = Vec3::new(10.0, 20.0, 300.0);
        let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 1.5, 3.0)
            * Mat4::look_to_rh(eye, Vec3::new(0.3, 1.0, -0.8).normalize(), Vec3::Z);
        assert!(eye_of(view_proj).distance(eye) < 1e-2, "{}", eye_of(view_proj));
    }

    #[test]
    fn a_transport_shows_the_crates_a_builder_the_crane_and_the_rest_the_swords() {
        assert_eq!(
            [0x0100_2000, 0x0100_4000, 0x0100_8000, 0x0101_0000].map(class_icon),
            [48.0, 72.0, 24.0, 24.0]
        );
    }
}
