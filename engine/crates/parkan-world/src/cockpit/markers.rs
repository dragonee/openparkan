//! The markers command mode draws over units in the world (`0x1007d5e0`, `0x10077d80`): the
//! selected units and the unit under the cursor, bracketed in the colour the marking rule
//! gives, named in green when the player's, with their class icon and their bars. See
//! `docs/25-sensors.md`, "How the game colours what it marks".

use glam::{Mat4, Vec3};

use super::panels::life_share;
use super::{Cockpit, Ink};
use crate::hud::{Blend, Pin};
use crate::play::Play;

/// page9's bracket, and how tall it stands about the unit's projected centre.
pub const BRACKET: [f32; 4] = [55.0, 0.0, 14.0, 28.0];
/// The widest the gap either side of the centre grows: 44 × the record's figure.
pub const GAP: f32 = 44.0;
/// The bars' frame, and the life and battery bars' colours.
pub const BARS: [f32; 2] = [27.0, 16.0];
pub const LIFE: [u8; 3] = [30, 180, 80];
pub const BATTERY: [u8; 3] = [255, 130, 50];
pub const NAME_GREEN: u32 = 0xff00_ff00;

fn rgba([r, g, b]: [u8; 3]) -> [f32; 4] {
    [f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, 1.0]
}

/// Every marker this frame: over the selected units and `hovered`, never the hero, through
/// `view_proj`.
///
/// STAND-IN: docs/25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured -- the
/// figure the gap is 44 times (the record's slot 5 for the camera distance), the bars' frame on
/// page9, the clan's sign and where the name, icon and bars stand are not read: the gap is the
/// unit's projected radius held to 4–44, the name over the left bracket, the class icon right
/// of the right one, and the frame a blue box under the left one holding a life bar over a full
/// battery bar.
pub fn draw(cockpit: &Cockpit, ink: &mut Ink, play: &Play, view_proj: Mat4, hovered: Option<usize>) {
    let space = ink.painter.space;
    let mut marked = play.selected_units();
    if let Some(t) =
        hovered.filter(|t| !marked.contains(t) && play.units[*t].kind == parkan_formats::mission::KIND_UNIT)
    {
        marked.push(t);
    }
    let eye_right =
        Vec3::new(view_proj.row(0).x, view_proj.row(0).y, view_proj.row(0).z).normalize_or(Vec3::X);
    ink.painter.pin = Pin::TOP_LEFT;
    for t in marked {
        let (Some(target), Some(unit)) = (play.battle.combat.targets.get(t), play.units.get(t)) else {
            continue;
        };
        if !target.alive || unit.logical_id == play.hero_id {
            continue;
        }
        let to_layout = |p: Vec3| {
            crate::play::on_screen(view_proj, p, 0.0).map(|[nx, ny]| {
                space.layout([(nx + 1.0) * 0.5 * space.width, (1.0 - ny) * 0.5 * space.height], Pin::TOP_LEFT)
            })
        };
        let (Some(c), Some(edge)) =
            (to_layout(target.centre), to_layout(target.centre + eye_right * target.radius))
        else {
            continue;
        };
        let gap = (edge[0] - c[0]).abs().clamp(4.0, GAP);
        let colour = play.mark_colour(unit.clan).map(|v| f32::from(v) / 255.0);
        let colour = [colour[0], colour[1], colour[2], 1.0];
        let [bx, by, bw, bh] = BRACKET;
        let top = c[1] - bh / 2.0;
        if let Some(&page) = cockpit.pages.get("page9") {
            let left = [c[0] - gap - bw, top, bw, bh];
            ink.painter.sprite_to(Blend::Alpha, page, [bx, by, bw, bh], left, colour);
            // The right one mirrored.
            let right = [c[0] + gap + bw, top, -bw, bh];
            ink.painter.sprite_to(Blend::Alpha, page, [bx, by, bw, bh], right, colour);
        }
        let x0 = c[0] - gap - bw;
        if unit.clan == Some(play.player_clan) {
            let name = cockpit.panels.names.get(t).cloned().unwrap_or_default();
            ink.text(&name, [x0, top - ink.font.line_height - 1.0], NAME_GREEN);
        }
        if let Some(&page) = cockpit.pages.get("ui_menu") {
            let cell = if parkan_sim::hq::within(unit.type_word, crate::selection::BUILDERS) {
                [65.0, 110.0]
            } else if parkan_sim::hq::within(unit.type_word, crate::selection::TRANSPORTS) {
                [81.0, 110.0]
            } else {
                [49.0, 110.0]
            };
            ink.painter.sprite(
                Blend::Alpha,
                page,
                [cell[0], cell[1], 15.0, 15.0],
                [c[0] + gap + bw + 2.0, top],
                colour,
            );
        }
        let [w, h] = BARS;
        let y0 = top + bh + 1.0;
        ink.painter.fill(Blend::Alpha, [x0, y0, w, h], [0.1, 0.2, 0.6, 0.8]);
        let share = life_share(target.parts.iter().filter_map(|p| p.life.as_ref())).clamp(0.0, 1.0);
        ink.painter.fill(Blend::Alpha, [x0 + 2.0, y0 + 3.0, (w - 4.0) * share, 4.0], rgba(LIFE));
        ink.painter.fill(Blend::Alpha, [x0 + 2.0, y0 + 9.0, w - 4.0, 4.0], rgba(BATTERY));
    }
}
