//! The satellite map (`iron3d.dll:0x10072f90`, drawn by `0x10073750`): the mission's minimap,
//! tinted green and see-through, at the top right where the weapons list was, with a compass
//! and marks for the player's units. M opens and closes it, ] and [ change how opaque it is.
//! See `docs/35-hud.md`, "The satellite map".

use glam::Vec3;
use parkan_formats::mission::KIND_UNIT;

use super::{Cockpit, Ink, argb, messages};
use crate::hud::{Blend, MINIMAP, Pin};
use crate::play::Play;

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
/// when the alpha last changed.
#[derive(Clone, Debug, PartialEq)]
pub struct SatelliteMap {
    pub open: bool,
    pub alpha: i32,
    changed_ms: Option<f64>,
}

impl Default for SatelliteMap {
    fn default() -> Self {
        Self { open: false, alpha: ALPHA_DEFAULT, changed_ms: None }
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

/// The map, while it is open.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, now_ms: f64) {
    let map = &cockpit.map;
    if !map.open {
        return;
    }
    ink.painter.pin = Pin::TOP_RIGHT;
    let [x0, y0, x1, y1] = PANEL;
    messages::frame(cockpit, ink, PANEL);
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

    // The marks: the player's own units; Mission 01's bridges take no icon.
    //
    // STAND-IN: docs/25-sensors.md#not-established -- what fills the player clan record's
    // `+0x54` list, whose clans' units and buildings are marked too, is not read: only the
    // player's own units are.
    let side = play.ground.world_box().1.truncate().max_element();
    let [sx, sy] = ink.painter.space.scales();
    for (t, u) in play.units.iter().enumerate() {
        if u.kind != KIND_UNIT || u.clan != Some(play.player_clan) || u.logical_id == play.hero_id {
            continue;
        }
        let Some(target) = play.battle.combat.targets.get(t).filter(|t| t.alive) else { continue };
        let flyer = u.designation.chassis_type == 1;
        let colour = play.mark_colour(u.clan).map(|v| f32::from(v) / 255.0);
        unit_mark(
            ink,
            map_point(target.position, side),
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
    let yaw = hero.walker.body.yaw;
    unit_mark(
        ink,
        map_point(hero.walker.body.position, side),
        false,
        [colour[0], colour[1], colour[2], 1.0],
        Some([-yaw.sin(), -yaw.cos()]),
        [sx, sy],
        px,
    );
}

/// A unit's mark at `at`: a cross for a flyer, else a square, in screen pixels about the
/// point; with `heading`, the outline and the heading line in the hero's green.
///
/// STAND-IN: docs/35-hud.md#not-established-4 -- that the unit record's `+0xd8` and `+0xdc`
/// the line runs along are its heading is not read; the line runs along the hero's facing.
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
}
