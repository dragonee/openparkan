//! The middle of the cockpit: the radar and its two gauges (`iron3d.dll:0x1003fb90`), the row
//! of indicators under it (`0x1003f150`), and the reticle (`0x10042ed0`). See `docs/35-hud.md`,
//! "The radar and the indicators below it".

use std::f32::consts::{FRAC_PI_2, PI};

use parkan_formats::mission::KIND_BUILDING;

use super::{Cockpit, Ink, WHITE, argb};
use crate::hud::{Blend, Piece, Pin};
use crate::play::Play;

/// The radar's centre, and the radius a contact at the sensor range lands at.
pub const CENTRE: [f32; 2] = [320.0, 396.0];
pub const CONTACT_RADIUS: f32 = 60.0;
/// The view wedge: its radius on screen, and its fan's centre and radius on page7.
pub const WEDGE_RADIUS: f32 = 75.0;
pub const FAN_CENTRE: [f32; 2] = [192.0, 64.0];
pub const FAN_RADIUS: f32 = 64.0;
/// North and south: the arrow's rows' radii and its half-width in angle.
pub const ARROW_OUTER: f32 = 68.0;
pub const ARROW_INNER: f32 = 56.0;
pub const ARROW_HALF_ANGLE: f32 = 5.0 * PI / 180.0;
pub const NORTH: u32 = 0xff0a_0ae1;
pub const SOUTH: u32 = 0xffe1_0a0a;
/// The sweep ring: its colour, and its period over the radar's.
pub const RING: u32 = 0x8c00_9b00;
pub const RING_PERIOD_PER_MS: f64 = 0.002;
pub const RING_SEGMENTS: usize = 20;
pub const RING_SOUND: &str = "RADAR";
/// The gauges' bar colour, and their figures' ranges.
pub const BAR: u32 = 0xffff_b450;
pub const GREEN: u32 = 0xff00_ff00;
pub const WEDGE: u32 = 0xffff_ffff;
/// The magenta triangle a wingman chosen in the selector gets.
pub const CHOSEN: u32 = 0xffff_00ff;
/// The indicators: their slots' x, the lamps' art, and the icons' art.
pub const SLOTS: [f32; 8] = [222.0, 240.0, 258.0, 280.0, 344.0, 366.0, 384.0, 402.0];
pub const ICONS: [(&str, [f32; 2]); 8] = [
    ("ui_menu", [97.0, 126.0]),
    ("ui_menu", [238.0, 222.0]),
    ("ui_menu", [239.0, 126.0]),
    ("ui_menu", [223.0, 142.0]),
    ("ui_menu3", [180.0, 27.0]),
    ("ui_menu", [113.0, 94.0]),
    ("ui_menu", [113.0, 110.0]),
    ("ui_menu", [81.0, 94.0]),
];
pub const ICON_OFF: u32 = 0xff80_8080;
pub const ICON_ALARM: u32 = 0xffff_0000;
/// The reticle's green, and its dot's.
pub const RETICLE: u32 = 0xff37_ff37;

/// A point `radius` from the radar's centre, `angle` clockwise from up.
pub fn on_ring(angle: f32, radius: f32) -> [f32; 2] {
    [CENTRE[0] + radius * angle.sin(), CENTRE[1] - radius * angle.cos()]
}

/// Where a contact at `offset` (x, y) from the unit lands for a camera heading `theta` and a
/// sensor range `range` (`0x100402ef`): `60 × d ÷ R` from the centre at `β − θ + π/2`
/// counter-clockwise from the right, y up, unclamped.
pub fn contact_at(offset: [f32; 2], theta: f32, range: f32) -> [f32; 2] {
    let d = offset[0].hypot(offset[1]);
    let gamma = offset[1].atan2(offset[0]) - theta + FRAC_PI_2;
    let r = CONTACT_RADIUS * d / range.max(f32::EPSILON);
    [CENTRE[0] + r * gamma.cos(), CENTRE[1] - r * gamma.sin()]
}

/// A gauge's bar: its fill, a whole percentage, and how many rows down its art starts.
pub fn bar(percent: i32) -> i32 {
    (100 - percent.clamp(0, 100)) * 105 / 100
}

/// The altitude's percentage: empty 100 m under the water, full 100 above.
pub fn altitude_percent(altitude: i32) -> i32 {
    ((altitude + 100) * 100 / 200).clamp(0, 100)
}

/// The radar's middle, and the sounds it starts.
pub fn draw(cockpit: &mut Cockpit, ink: &mut Ink, play: &Play, water_level: f32) -> Vec<&'static str> {
    let mut sounds = Vec::new();
    ink.painter.pin = Pin::BOTTOM;
    let page = |name: &str| cockpit.pages.get(name).copied();
    let (Some(page6), Some(page7), Some(ui_menu), Some(ui_menu3)) =
        (page("page6"), page("page7"), page("ui_menu"), page("ui_menu3"))
    else {
        return sounds;
    };
    let white = [1.0; 4];
    let piece = |page: u16, rect: [f32; 4]| Piece { page, rect, turns: 0 };
    let p = &mut ink.painter;
    p.piece(piece(page6, [156.0, 0.0, 99.0, 153.0]), [320.0, 327.0, 221.0, 480.0], white);
    p.piece(piece(page6, [155.0, 0.0, 100.0, 153.0]), [320.0, 327.0, 421.0, 480.0], white);
    p.piece(piece(ui_menu3, [215.0, 0.0, 15.0, 28.0]), [230.0, 383.0, 245.0, 411.0], white);
    p.piece(piece(ui_menu3, [199.0, 0.0, 15.0, 28.0]), [396.0, 383.0, 411.0, 411.0], white);

    // The altitude (`0x1003f6a0`): height above the map's water.
    let body = &play.driven().walker.body;
    let altitude = (body.position.z - water_level).round() as i32;
    let e = bar(altitude_percent(altitude)) as f32;
    ink.painter.piece(piece(page6, [121.0, e, 31.0, 105.0 - e]), [271.0, 347.0 + e, 240.0, 452.0], argb(BAR));
    ink.centred(&altitude.to_string(), 226.0, 28.0, 446.0, WHITE);

    // The speed (`0x1003f900`): the step velocity, over the forward top speed, in km/h.
    let speed = (glam::Vec3::from_array(body.velocity).length() * 3.6).round() as i32;
    let top = (play.driven().walker.controller.triples[parkan_formats::control::TRIPLE_TOP_SPEED][1] * 3.6)
        .round() as i32;
    let percent = if top != 0 { (speed * 100 / top).clamp(0, 100) } else { 0 };
    let e = bar(percent) as f32;
    ink.painter.piece(piece(page6, [121.0, e, 31.0, 105.0 - e]), [370.0, 347.0 + e, 401.0, 452.0], argb(BAR));
    ink.centred(&speed.to_string(), 388.0, 28.0, 446.0, WHITE);

    // The view wedge: the camera's field, always up.
    let eye = play.eye();
    let half = eye.fov_x / 2.0;
    let corner = |a: f32| {
        (
            [CENTRE[0] + WEDGE_RADIUS * a.cos(), CENTRE[1] - WEDGE_RADIUS * a.sin()],
            [FAN_CENTRE[0] + FAN_RADIUS * a.cos(), FAN_CENTRE[1] - FAN_RADIUS * a.sin()],
        )
    };
    let (left, left_uv) = corner(FRAC_PI_2 + half);
    let (right, right_uv) = corner(FRAC_PI_2 - half);
    ink.painter.textured_triangle(
        Blend::Alpha,
        [CENTRE, left, right],
        [FAN_CENTRE, left_uv, right_uv],
        page7,
        argb(WEDGE),
    );

    // North and south turn with the camera's heading.
    let theta = eye.forward.y.atan2(eye.forward.x);
    let north = theta - FRAC_PI_2;
    for (alpha, colour) in [(north, NORTH), (north + PI, SOUTH)] {
        ink.painter.quad(
            Blend::Alpha,
            [
                on_ring(alpha - ARROW_HALF_ANGLE, ARROW_OUTER),
                on_ring(alpha + ARROW_HALF_ANGLE, ARROW_OUTER),
                on_ring(alpha + ARROW_HALF_ANGLE, ARROW_INNER),
                on_ring(alpha - ARROW_HALF_ANGLE, ARROW_INNER),
            ],
            [[192.0, 154.0], [201.0, 154.0], [201.0, 165.0], [192.0, 165.0]],
            Some(page6),
            argb(colour),
        );
    }

    // The sweep ring, and its ping each period.
    let period_ms = f64::from(play.driven().radar.period_ms) * RING_PERIOD_PER_MS * 1000.0;
    let now_ms = play.hero.time_ms;
    if period_ms > 0.0 {
        if now_ms - cockpit.ring_since_ms > period_ms {
            cockpit.ring_since_ms = now_ms;
            sounds.push(RING_SOUND);
        }
        let r = (CONTACT_RADIUS as f64 * (now_ms - cockpit.ring_since_ms) / period_ms).floor() as f32;
        let space = ink.painter.space;
        let width = 1.0 / space.scale();
        for k in 0..RING_SEGMENTS {
            let a = |k: usize| 2.0 * PI * k as f32 / RING_SEGMENTS as f32;
            ink.painter.line(Blend::Alpha, on_ring(a(k), r), on_ring(a(k + 1), r), width, argb(RING));
        }
    }

    // The contacts: the driven unit's target list.
    let range = play.driven().radar.range;
    let contacts = play.contacts();
    let [sx, sy] = ink.painter.space.scales();
    let scale = ink.painter.space.scale();
    let px = 1.0 / scale;
    let unit = body.position;
    for &t in &play.targets.listed {
        let (Some(c), Some(u)) = (contacts.get(t), play.units.get(t)) else { continue };
        if !c.alive {
            continue;
        }
        let at = contact_at([c.position.x - unit.x, c.position.y - unit.y], theta, range);
        let [r, g, b] = play.mark_colour(u.clan).map(|v| f32::from(v) / 255.0);
        let colour = [r, g, b, 1.0];
        let flyer = u.designation.chassis_type == 1;
        let even = |v: f32| ((v as i32) / 2 * 2) as f32;
        let (w, h) = if flyer {
            (even(4.0 * sx), even(4.0 * sy))
        } else if u.kind == KIND_BUILDING {
            ((5.0 * sx).trunc(), (5.0 * sy).trunc())
        } else {
            ((3.0 * sx).trunc(), (3.0 * sy).trunc())
        };
        let (w, h) = (w * px, h * px);
        if flyer {
            ink.painter.fill(Blend::Alpha, [at[0] - w / 2.0, at[1] - px / 2.0, w, px], colour);
            ink.painter.fill(Blend::Alpha, [at[0] - px / 2.0, at[1] - h / 2.0, px, h], colour);
        } else {
            ink.painter.fill(Blend::Alpha, [at[0] - w / 2.0, at[1] - h / 2.0, w, h], colour);
        }
        if play.targets.current == Some(t) {
            let beyond = if flyer { 2.0 } else { 1.0 } * px;
            let (x0, y0) = (at[0] - w / 2.0 - 2.0 * px, at[1] - h / 2.0 - 2.0 * px);
            let (x1, y1) = (at[0] + w / 2.0 + beyond, at[1] + h / 2.0 + beyond);
            for rect in [
                [x0, y0, x1 - x0, px],
                [x0, y1, x1 - x0, px],
                [x0, y0, px, y1 - y0],
                [x1, y0, px, y1 - y0 + px],
            ] {
                ink.painter.fill(Blend::Alpha, rect, colour);
            }
        }
        if play.selector.chosen.iter().any(|&w| play.robots.get(w).is_some_and(|(target, _)| *target == t)) {
            let radius = (5.0 * sx).trunc() * px;
            let apex = [at[0], at[1] - radius];
            let left = [at[0] - radius * 0.866, at[1] + radius * 0.5];
            let right = [at[0] + radius * 0.866, at[1] + radius * 0.5];
            for (a, b) in [(apex, right), (right, left), (left, apex)] {
                ink.painter.line(Blend::Alpha, a, b, px, argb(CHOSEN));
            }
        }
    }

    // The range figure.
    let figure = range.round().to_string();
    let left = (303.0 + (37.0 - ink.font.advance(&figure)) / 2.0).round();
    ink.text(&figure, [left, 467.0], GREEN);

    indicators(ink, play, page6, ui_menu, ui_menu3);
    sounds
}

/// The eight indicators (`0x1003f270`): a lamp dark, lit or red, and an icon grey, white or red.
///
/// STAND-IN: docs/35-hud.md#the-indicators--read-and-seen -- the interface's `CState` and the
/// landing warning are not built: the figure is lit and the warning grey; and the auto-driver
/// level is the hero's, which nothing steps.
fn indicators(ink: &mut Ink, play: &Play, page6: u16, ui_menu: u16, ui_menu3: u16) {
    let switches = play.driving.as_ref().map_or(play.hero.pilot.switches, |d| d.pilot.switches);
    let level = play.auto_driver;
    let states = [
        u8::from(switches.repair),
        u8::from(switches.infrared),
        u8::from(switches.camouflage),
        1,
        0,
        u8::from(level == 0),
        u8::from(level == 1),
        u8::from(level == 2),
    ];
    for ((x, state), (page, [u, v])) in SLOTS.iter().zip(states).zip(ICONS) {
        let lamp =
            Piece { page: page6, rect: [202.0 + 18.0 * f32::from(state), 154.0, 17.0, 20.0], turns: 0 };
        ink.painter.piece(lamp, [*x, 459.0, x + 18.0, 484.0], [1.0; 4]);
        let icon_page = if page == "ui_menu3" { ui_menu3 } else { ui_menu };
        let tint = match state {
            0 => ICON_OFF,
            1 => WHITE,
            _ => ICON_ALARM,
        };
        let icon = Piece { page: icon_page, rect: [u, v, 15.0, 15.0], turns: 0 };
        ink.painter.piece(icon, [x + 1.0, 463.0, x + 16.0, 478.0], argb(tint));
    }
}

/// The reticle (`0x10042ed0`): a circle, four corner arcs and two tick strips about the screen's
/// middle, and a dot.
///
/// STAND-IN: docs/35-hud.md#the-reticle--read -- what the camera view's property 0 the dot is
/// placed by is not read: the dot stays in the middle.
pub fn reticle(cockpit: &Cockpit, ink: &mut Ink) {
    let Some(page9) = cockpit.pages.get("page9").copied() else { return };
    ink.painter.pin = Pin::CENTRE;
    let green = argb(RETICLE);
    let piece = |rect: [f32; 4]| Piece { page: page9, rect, turns: 0 };
    let p = &mut ink.painter;
    p.piece(piece([77.0, 0.0, 64.0, 64.0]), [288.0, 208.0, 352.0, 272.0], green);
    let arc = piece([54.0, 28.0, 23.0, 23.0]);
    for rect in [
        [279.0, 260.0, 300.0, 281.0],
        [279.0, 220.0, 300.0, 199.0],
        [361.0, 220.0, 340.0, 199.0],
        [361.0, 260.0, 340.0, 281.0],
    ] {
        p.piece(arc, rect, green);
    }
    let strip = piece([168.0, 0.0, 82.0, 15.0]);
    p.piece(strip, [232.0, 233.0, 314.0, 248.0], green);
    p.piece(strip, [408.0, 233.0, 326.0, 248.0], green);
    p.fill(Blend::Alpha, [319.0, 239.0, 2.0, 2.0], argb(GREEN));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_contact_ahead_is_straight_up_and_one_to_the_left_is_left() {
        // Facing east (θ 0): a contact 150 east, half the range, is 30 above the centre.
        let [x, y] = contact_at([150.0, 0.0], 0.0, 300.0);
        assert!((x - 320.0).abs() < 1e-3 && (y - 366.0).abs() < 1e-3, "{x} {y}");
        // North of an east-facing unit is to its left.
        let [x, y] = contact_at([0.0, 300.0], 0.0, 300.0);
        assert!((x - 260.0).abs() < 1e-3 && (y - 396.0).abs() < 1e-3, "{x} {y}");
    }

    #[test]
    fn the_gauges_fill_from_the_bottom() {
        assert_eq!(bar(100), 0);
        assert_eq!(bar(0), 105);
        assert_eq!(altitude_percent(0), 50);
        assert_eq!(altitude_percent(-150), 0);
        assert_eq!(altitude_percent(100), 100);
    }
}
