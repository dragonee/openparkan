//! The weapons list at the top right (`iron3d.dll:0x1003ecb0`, `0x1009cd30`): a row a gun,
//! laid right to left from x 640 out of `ui/compaund.cfg`'s pieces. See `docs/35-hud.md`,
//! "The weapons list".

use parkan_sim::guns::{GATE_NO_TARGET, GATE_OFF_BARREL, GATE_OUT_OF_RANGE, Gun, UNLIMITED};
use parkan_sim::orders::State;

use super::{Cockpit, Ink, WHITE};
use crate::play::Play;

pub const ROW_HEIGHT: f32 = 19.0;
pub const RIGHT_EDGE: f32 = 640.0;
/// The hero's guns' names, in turn (`0x10074b71`).
pub const HERO_GUN_NAMES: [u32; 4] = [3071, 3072, 3073, 3074];
pub const STRING_INF: u32 = 5094;
/// A gun whose name is not found (`0x1003ecb0`).
pub const NONAME: &str = "NONAME";
pub const STRING_OUT_OF_RANGE: u32 = 6250;
/// The row's text while the wingman selector is on, and a name out of range.
pub const GREY: u32 = 0xff80_8080;
pub const OUT_OF_RANGE: u32 = 0xffff_5c5c;
/// The charge fill under 20%, under 80%, and from 80%.
pub const FILL_LOW: u32 = 0x8080_0000;
pub const FILL_MIDDLE: u32 = 0x8080_8000;
pub const FILL_HIGH: u32 = 0x8000_8000;
/// The name bar's width, and how far in from its right edge its fill stops.
pub const BAR_WIDTH: f32 = 120.0;
/// The lamps, kind 8's variants: red, yellow1, yellow2, green, black.
pub const LAMPS: [&str; 5] =
    ["ccres_red_lamp", "ccres_yellow1_lamp", "ccres_yellow2_lamp", "ccres_green_lamp", "ccres_black_lamp"];
pub const VOICE_WEAPON_DESTROYED: &str = "VOICE_WEAPON_DESTR";
pub const VOICE_AMMO_OUT: &str = "VOICE_WEAP_AMMO_OUT";
pub const VOICE_ENERGY_OUT: &str = "VOICE_WEAP_ENERGY_OUT";

/// A gun's lamp (`0x1009d8cc`): black unless it is selected, then by its report word.
pub fn lamp(selected: bool, report: i32) -> usize {
    if !selected {
        return 4;
    }
    match report {
        0 => 3,
        4 | 6 => 1,
        5 => 0,
        _ => 2,
    }
}

/// A gun's charge as the bar shows it, a whole percentage: its capacitor ÷ capacity, 0
/// outside 0..1.
///
/// STAND-IN: docs/29-weapons.md#a-gun-is-a-capacitor-a-magazine-and-a-clock--read -- the
/// level of a gun with no capacity, which only a shot sets, is not read; it shows full.
pub fn charge_percent(gun: &Gun) -> i32 {
    let level = if gun.capacitor > 0.0 { gun.charge / gun.capacitor } else { 1.0 };
    if (0.0..=1.0).contains(&level) { (level * 100.0).round() as i32 } else { 0 }
}

/// The fill's colour for a percentage.
pub fn fill_colour(percent: i32) -> u32 {
    if percent < 20 {
        FILL_LOW
    } else if percent < 80 {
        FILL_MIDDLE
    } else {
        FILL_HIGH
    }
}

/// The three latches each gun keeps for its voices: no rounds, no energy, destroyed.
#[derive(Clone, Debug, Default)]
pub struct Latches(Vec<[bool; 3]>);

/// The list for the driven unit's guns, and the voices a latch rising plays.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play) -> Vec<&'static str> {
    let now_ms = play.hero.time_ms;
    let selecting = play.selector.state != State::Off;
    let guns = &play.driven().guns;
    let mut voices = Vec::new();
    cockpit.weapons.0.resize(guns.len(), [false; 3]);
    let top = (ink.font.line_height).round();
    let text_down = ((ROW_HEIGHT - top) / 2.0).floor();
    for (i, gun) in guns.iter().enumerate() {
        let y = ROW_HEIGHT * i as f32;
        let report = gun.lamp_report(now_ms);
        let row_colour = if selecting { GREY } else { WHITE };
        let lamp_index = if selecting { 4 } else { lamp(gun.selected, report) };
        let out_of_range = matches!(report, GATE_NO_TARGET | GATE_OUT_OF_RANGE | GATE_OFF_BARREL);
        let mut pen = RIGHT_EDGE;
        let skin = &cockpit.skin;
        // `0x100999b0`: each piece from the pen to w + 1 left of it, mirrored; the pen moves w.
        let piece = |ink: &mut Ink, pen: &mut f32, name: &str, width: f32| {
            if let Some(p) = skin.get(name) {
                ink.painter.piece(p, [*pen, y, *pen - width - 1.0, y + ROW_HEIGHT], [1.0; 4]);
            }
            *pen -= width;
            *pen
        };
        piece(ink, &mut pen, "ccres_ending_text", 5.0);
        let key_right = pen;
        let key_left = piece(ink, &mut pen, "ccres_body_text", 12.0);
        ink.centred(&(i + 1).to_string(), key_left, key_right - key_left, y + text_down, row_colour);
        piece(ink, &mut pen, LAMPS[lamp_index], 19.0);
        let rounds_right = pen;
        let rounds_left = piece(ink, &mut pen, "ccres_body_text", 24.0);
        let rounds = if gun.magazine == UNLIMITED || gun.rounds < 0 {
            cockpit.string(STRING_INF).to_owned()
        } else {
            format!("{:4}", gun.rounds)
        };
        ink.centred(&rounds, rounds_left, rounds_right - rounds_left, y + text_down, row_colour);
        piece(ink, &mut pen, "ccres_separator_left_text", 5.0);
        piece(ink, &mut pen, "ccres_ray_emitter_off", 10.0);
        let bar_right = pen;
        let bar_left = piece(ink, &mut pen, "ccres_ray_body", BAR_WIDTH);
        let percent = charge_percent(gun);
        if percent > 0 {
            let x0 = bar_right - BAR_WIDTH * percent as f32 / 100.0;
            ink.painter.fill(
                crate::hud::Blend::Alpha,
                [x0, y + 3.0, bar_right - 2.0 - x0, ROW_HEIGHT - 6.0],
                super::argb(fill_colour(percent)),
            );
        }
        let (name, name_colour) = if out_of_range && gun.selected && !selecting {
            (cockpit.string(STRING_OUT_OF_RANGE).to_owned(), OUT_OF_RANGE)
        } else if play.driving.is_some() {
            // Any unit but the hero looks its gun's name up from the component
            // (`0x1008a470`), and falls back to NONAME.
            //
            // STAND-IN: docs/35-hud.md#the-weapons-list--read-and-measured -- the lookup is not
            // followed: the gun part's code in the player clan's research tree.
            let robot = play.driven();
            let code = robot
                .gun_parts
                .get(i)
                .and_then(Option::as_ref)
                .and_then(|g| robot.parts.get(g.part))
                .and_then(|p| cockpit.gun_codes.get(&p.record.to_ascii_lowercase()))
                .cloned();
            (code.unwrap_or_else(|| NONAME.to_owned()), row_colour)
        } else {
            let id = HERO_GUN_NAMES.get(i).copied().unwrap_or(0);
            (cockpit.string(id).to_owned(), row_colour)
        };
        ink.centred(&name, bar_left, bar_right - bar_left, y + text_down, name_colour);
        piece(ink, &mut pen, "ccres_ray_ending", 6.0);

        // `0x1009d5ba`–`0x1009d834`: each voice once, as its latch rises.
        let latches = &mut cockpit.weapons.0[i];
        let now = [gun.rounds == 0, gun.capacitor > 0.0 && gun.charge < 0.01, false];
        for (k, voice) in [VOICE_AMMO_OUT, VOICE_ENERGY_OUT, VOICE_WEAPON_DESTROYED].into_iter().enumerate() {
            if now[k] && !latches[k] {
                voices.push(voice);
            }
            latches[k] = now[k];
        }
    }
    voices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selected_guns_lamp_follows_its_report_and_an_unselected_one_is_black() {
        assert_eq!(lamp(false, 0), 4);
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 8].map(|r| lamp(true, r)), [3, 2, 2, 2, 1, 0, 1, 2, 2]);
    }

    #[test]
    fn the_fill_is_red_under_a_fifth_olive_under_four_fifths_and_green_after() {
        assert_eq!(
            [0, 19, 20, 79, 80, 100].map(fill_colour),
            [FILL_LOW, FILL_LOW, FILL_MIDDLE, FILL_MIDDLE, FILL_HIGH, FILL_HIGH]
        );
    }
}
