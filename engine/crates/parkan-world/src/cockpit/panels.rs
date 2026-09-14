//! The target panel at the bottom left and the player's own unit at the bottom right
//! (`iron3d.dll:0x10040f30`): the unit seen through a camera, coloured by its parts' life,
//! its shield sectors, its life and battery arcs, its name and, for the target, its distance
//! and a frame about it in the world. See `docs/35-hud.md`, "The target panel and the player's
//! own unit".

use std::collections::BTreeMap;

use glam::{Mat4, Quat, Vec3};
use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT};
use parkan_sim::behaviour::Task;
use parkan_sim::damage::Life;

use super::{Cockpit, Ink, argb};
use crate::hud::{Blend, Piece, Pin};
use crate::play::{Play, ROBOT_HERO};

/// The two panels: the driven unit's target, and the driven unit itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Target,
    Own,
}

/// The name's grey, the status the same, "Dangerous!" red, and the distance green.
pub const NAME_COLOUR: u32 = 0xffc8_c8c8;
pub const DANGEROUS_COLOUR: u32 = 0xffc8_0000;
pub const RANGE_COLOUR: u32 = 0xff00_ff00;
/// The life arc's tint and the battery arc's.
pub const LIFE_TINT: u32 = 0xff19_ffaf;
pub const ENERGY_TINT: u32 = 0xffff_b450;
/// Strings: "Human", "Animal", "m", "no order", the order statuses, and the class words.
pub const STRING_HUMAN: u32 = 6230;
pub const STRING_ANIMAL: u32 = 6253;
pub const STRING_METRES: u32 = 6178;
pub const STRING_NO_ORDER: u32 = 6180;
pub const STRING_TRANSPORT: u32 = 6200;
pub const STRING_BUILDER: u32 = 6201;
pub const STRING_WARRIOR: u32 = 6202;
pub const STRING_HQ: u32 = 6203;
pub const STRING_HERO: u32 = 6204;
pub const STRING_UNKNOWN_CLASS: u32 = 6205;
/// The `Type` words the name's class letter is read from (docs/30-turrets.md).
pub const TYPE_TRANSPORT: u32 = 0x0100_2000;
pub const TYPE_BUILDER: u32 = 0x0100_4000;
pub const TYPE_WARRIOR: u32 = 0x0100_8000;
pub const TYPE_HQ: u32 = 0x0101_0000;
pub const TYPE_ANIMAL: u32 = 0x2000_0000;
/// The arc's art on `ui_menu3`: 40 × 94 at (151, 82), cut again from the top.
pub const ARC: [f32; 4] = [151.0, 82.0, 40.0, 94.0];
/// The panel camera: 1 ÷ sin 30°, the field 1.25 × 60°, and the figures it is given, taken
/// as its near and far planes (`0x10040f00`, `0x100367b0`).
pub const CAMERA_K: f32 = 2.0;
pub const CAMERA_FOV: f32 = 1.25 * std::f32::consts::FRAC_PI_3;
pub const CAMERA_NEAR: f32 = 0.5;
pub const CAMERA_FAR: f32 = 300.0;
/// The frame in the world: its half-side at a scale of 1, the scale's floor, what a scale
/// under it becomes, its cap, and the ease.
pub const FRAME_HALF_SIDE: f32 = 200.0;
pub const FRAME_SCALE_FLOOR: f32 = 0.025;
pub const FRAME_SCALE_LOW: f32 = 0.1;
pub const FRAME_SCALE_CAP: f32 = 1.1;
pub const FRAME_EASE_MS: f64 = 1000.0;
/// The own panel's voices: under 20% life, under 0.2 of the battery, at most once in 20 s.
pub const VOICE_LIFE_LOW: &str = "VOICE_LIFE_LOW";
pub const VOICE_BATTERY_LOW: &str = "VOICE_BATT_LOW";
pub const VOICE_GAP_MS: f64 = 20_000.0;

/// A view of a unit a panel holds: where on the layout, the camera, and the unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitView {
    /// (x₀, y₀)–(x₁, y₁) on the layout, under `pin`.
    pub rect: [f32; 4],
    pub pin: Pin,
    /// The rectangle in screen pixels: x, y, width, height.
    pub viewport: [f32; 4],
    pub eye: Vec3,
    pub centre: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    /// A battle target, or the hero with none.
    pub unit: Option<usize>,
}

impl UnitView {
    /// The camera's matrix, reverse depth as the scene's.
    pub fn view_proj(&self) -> Mat4 {
        let aspect = (self.viewport[2] / self.viewport[3].max(1.0)).max(0.01);
        let forward = (self.centre - self.eye).normalize_or(Vec3::Y);
        let up = if forward.cross(Vec3::Z).length_squared() < 1e-6 { Vec3::Y } else { Vec3::Z };
        Mat4::perspective_rh(self.fov, aspect, self.far, self.near)
            * Mat4::look_at_rh(self.eye, self.centre, up)
    }
}

/// A node's colour in the view (`AniMesh.dll:0x10014b30`): (0.5, 0, 0) and life × (−0.5, 0.5,
/// 0), green whole and red destroyed.
pub fn node_colour(life: f32) -> [f32; 3] {
    let l = life.clamp(0.0, 1.0);
    [0.5 - 0.5 * l, 0.5 * l, 0.0]
}

/// What stays from frame to frame: every target's name, the frame's clock, the voices'.
#[derive(Clone, Debug, Default)]
pub struct Panels {
    pub names: Vec<String>,
    pub hero_name: String,
    frame_clock_ms: f64,
    last_voices_ms: [Option<f64>; 2],
}

/// A unit's name (`0x10075d50`, `0x10076270`): "Human" for a hero, "Animal", or its size,
/// chassis and class letters, its place among its clan's named units, and its class word.
///
/// STAND-IN: docs/35-hud.md#name-and-status--read-and-seen -- which caller hands the name its
/// class word is not read: each class letter takes its own; and a robot whose record answers
/// its query 2 with 0 or less, a *"Tiny Tower"*, is not told apart.
pub fn name(
    type_word: u32,
    size_class: u8,
    chassis_type: u8,
    number: usize,
    strings: &BTreeMap<u32, String>,
) -> String {
    let string = |id: u32| strings.get(&id).cloned().unwrap_or_default();
    if type_word == ROBOT_HERO {
        return string(STRING_HUMAN);
    }
    if type_word & TYPE_ANIMAL != 0 {
        return string(STRING_ANIMAL);
    }
    let size = match size_class {
        1 => 'T',
        2 => 'S',
        3 => 'M',
        4 => 'L',
        _ => '?',
    };
    let chassis = match chassis_type {
        1 => 'F',
        2 => 'S',
        3 => 'W',
        4 => 'T',
        5 => 'A',
        6 => 'U',
        _ => '?',
    };
    let (class, word) = match type_word {
        TYPE_BUILDER => ('B', STRING_BUILDER),
        TYPE_TRANSPORT => ('T', STRING_TRANSPORT),
        TYPE_WARRIOR => ('W', STRING_WARRIOR),
        TYPE_HQ => ('C', STRING_HQ),
        ROBOT_HERO => ('H', STRING_HERO),
        _ => ('?', STRING_UNKNOWN_CLASS),
    };
    format!("{size}{chassis}{class}-{number} {}", string(word))
}

/// The status string of an order (docs/31-packages.md, "The orders"), by its number.
pub fn order_status(order: i32) -> u32 {
    match order {
        1 => 6181,
        2 => 6184,
        3 => 6188,
        4 => 6189,
        5 => 6190,
        6 => 6191,
        7 => 6192,
        8 => 6193,
        9 => 6194,
        17 => 6195,
        19 => 6182,
        20 => 6185,
        21 => 6183,
        22 => 6186,
        23 => 6187,
        24 => 6196,
        _ => 6197,
    }
}

/// A part list's life over its full life, all nodes together (property `0x31`).
pub fn life_share<'a>(lives: impl Iterator<Item = &'a Life>) -> f32 {
    let (life, full) = lives.fold((0.0, 0.0), |(l, f), life| {
        (l + life.total(), f + life.nodes.iter().map(|n| n.max).sum::<f32>())
    });
    if full > 0.0 { life / full } else { 0.0 }
}

impl Panels {
    /// Every unit of `play` named, each clan counting its own in file order.
    pub fn new(play: &Play, strings: &BTreeMap<u32, String>) -> Panels {
        let mut counts: BTreeMap<Option<i64>, usize> = BTreeMap::new();
        let names = play
            .units
            .iter()
            .map(|u| {
                if u.kind != KIND_UNIT {
                    return String::new();
                }
                let d = u.designation;
                let numbered = u.type_word != ROBOT_HERO && u.type_word & TYPE_ANIMAL == 0;
                let number = if numbered {
                    let c = counts.entry(u.clan).or_default();
                    *c += 1;
                    *c
                } else {
                    0
                };
                name(u.type_word, d.size_class, d.chassis_type, number, strings)
            })
            .collect();
        Panels {
            names,
            hero_name: name(ROBOT_HERO, 0, 0, 0, strings),
            frame_clock_ms: 0.0,
            last_voices_ms: [None; 2],
        }
    }

    /// The own panel's voices due at `now_ms`.
    ///
    /// STAND-IN: docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured
    /// -- batteries are not simulated: they read full, so the low battery voice never plays.
    pub fn voices(&mut self, play: &Play, now_ms: f64) -> Vec<&'static str> {
        let life = life_share(play.hero.lives.iter().flatten());
        let mut out = Vec::new();
        let due = |last: Option<f64>| last.is_none_or(|t| now_ms - t >= VOICE_GAP_MS);
        if life < 0.2 && due(self.last_voices_ms[0]) {
            out.push(VOICE_LIFE_LOW);
            self.last_voices_ms[0] = Some(now_ms);
        }
        out
    }
}

/// Both panels, and the views they hold.
pub fn draw(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    view_proj: Mat4,
    now_ms: f64,
) -> Vec<UnitView> {
    let mut views = Vec::new();
    views.extend(panel(cockpit, ink, play, Side::Target, view_proj, now_ms));
    views.extend(panel(cockpit, ink, play, Side::Own, view_proj, now_ms));
    ink.painter.over = false;
    views
}

/// The hero's sphere in the world: its agent sphere carried by its body.
fn hero_sphere(play: &Play) -> (Vec3, f32) {
    let body = &play.driven().walker.body;
    let (centre, radius) = play.driven().collision;
    (body.position + Quat::from_rotation_z(body.heading()) * centre, radius)
}

/// A panel for `side` (`0x10040f30`).
fn panel(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    side: Side,
    view_proj: Mat4,
    now_ms: f64,
) -> Option<UnitView> {
    let own = side == Side::Own;
    ink.painter.pin = if own { Pin::BOTTOM_RIGHT } else { Pin::BOTTOM_LEFT };
    ink.painter.over = false;
    let white = [1.0; 4];
    let put = |ink: &mut Ink, skin: &crate::hud::Skin, name: &str, rect: [f32; 4]| {
        if let Some(p) = skin.get(name) {
            ink.painter.piece(p, rect, white);
        }
    };
    let skin = &cockpit.skin;
    put(
        ink,
        skin,
        "targeter_back",
        if own { [640.0, 306.0, 490.0, 480.0] } else { [0.0, 306.0, 150.0, 480.0] },
    );
    put(
        ink,
        skin,
        "targeter_life",
        if own { [496.0, 411.0, 511.0, 426.0] } else { [129.0, 411.0, 144.0, 426.0] },
    );
    put(
        ink,
        skin,
        "targeter_energy",
        if own { [620.0, 411.0, 635.0, 426.0] } else { [5.0, 411.0, 20.0, 426.0] },
    );

    let contacts = play.contacts();
    let (hero_centre, hero_radius) = hero_sphere(play);
    // Aboard a bot, the own panel is the bot's (docs/39, "The view and the HUD").
    let shown = if own { play.driving.as_ref().map(|d| d.target) } else { Some(play.targets.current?) };
    let (centre, radius, designation, life, building) = match shown {
        None => (
            hero_centre,
            hero_radius,
            play.hero_designation,
            life_share(play.hero.lives.iter().flatten()),
            false,
        ),
        Some(t) => {
            let c = contacts.get(t)?;
            let target = play.battle.combat.targets.get(t)?;
            (
                c.centre,
                c.radius,
                play.units.get(t)?.designation,
                life_share(target.parts.iter().filter_map(|p| p.life.as_ref())),
                play.units[t].kind == KIND_BUILDING,
            )
        }
    };

    // 4: the six sectors, with a fight shield and a deflector.
    //
    // STAND-IN: docs/26-damage.md#shields-a-generator-a-deflector-six-sectors--read-and-measured
    // -- shields are not simulated: every sector reads full.
    if designation.shielded {
        let v = 255u32;
        let colour = argb(0xff00_0000 | ((255 - v) << 16) | (v << 8));
        let dx = if own { 491.0 } else { 0.0 };
        let sectors = [
            ("frwd_shld", [44.0, 321.0, 105.0, 340.0]),
            ("back_shld", [30.0, 409.0, 119.0, 438.0]),
            ("left_shld", [16.0, 334.0, 42.0, 413.0]),
            ("left_shld", [133.0, 334.0, 107.0, 413.0]),
            ("top_shld", [40.0, 308.0, 109.0, 327.0]),
            ("bott_shld", [22.0, 420.0, 127.0, 451.0]),
        ];
        for (name, [x0, y0, x1, y1]) in sectors {
            if let Some(p) = skin.get(name) {
                ink.painter.piece(p, [x0 + dx, y0, x1 + dx, y1], colour);
            }
        }
    }

    // 5: the frame about the target in the world.
    let distance = if own { 0.0 } else { hero_centre.distance(centre) };
    if let Some(t) = shown.filter(|_| !own) {
        frame(cockpit, ink, play, t, centre, radius, distance, view_proj, now_ms);
    }

    // 6: the view. A unit too big for it ends the panel there (`0x10041902`).
    let rect = if own { [503.0, 315.0, 631.0, 443.0] } else { [9.0, 315.0, 137.0, 443.0] };
    let pin = ink.painter.pin;
    let space = ink.painter.space;
    let [px0, py0] = space.pixel([rect[0], rect[1]], pin);
    let [px1, py1] = space.pixel([rect[2], rect[3]], pin);
    let direction = if own {
        let h = play.driven().walker.body.heading();
        Vec3::new(-h.sin(), h.cos(), 0.0)
    } else {
        let mut d = (centre - hero_centre).normalize_or(Vec3::Y);
        if building && d.z > 0.0 {
            d = d.with_z(0.0).normalize_or(Vec3::Y);
        }
        d
    };
    if 2.0 * radius > CAMERA_FAR - CAMERA_NEAR {
        return None;
    }
    let (mut span, mut fov) = (CAMERA_K * radius, CAMERA_FOV);
    if (CAMERA_K - 1.0) * radius < CAMERA_NEAR {
        span = CAMERA_NEAR + radius;
        fov = 1.25 * 2.0 * (radius / span).clamp(-1.0, 1.0).asin();
    }
    if (CAMERA_K + 1.0) * radius > CAMERA_FAR {
        span = CAMERA_FAR - radius;
        fov = 1.25 * 2.0 * (radius / span).clamp(-1.0, 1.0).asin();
    }
    let view = UnitView {
        rect,
        pin,
        viewport: [px0, py0, px1 - px0, py1 - py0],
        eye: centre - direction * span,
        centre,
        fov: fov.max(0.01),
        near: CAMERA_NEAR,
        far: CAMERA_FAR,
        unit: shown,
    };

    // 7–10, over the view.
    ink.painter.over = true;
    let arc = |percent: f32| {
        let p = percent.round().clamp(0.0, 100.0) as i32;
        let t = (94 * (100 - p) / 100) as f32;
        (t, Piece { page: 0, rect: [ARC[0], ARC[1] + t, ARC[2], ARC[3] - t], turns: 0 })
    };
    let skin = &cockpit.skin;
    if let Some(page) = skin.get("targeter_back").map(|p| p.page) {
        let (t, piece) = arc(100.0 * life);
        let piece = Piece { page, ..piece };
        let rect = if own { [491.0, 314.0 + t, 531.0, 408.0] } else { [149.0, 314.0 + t, 109.0, 408.0] };
        ink.painter.piece(piece, rect, argb(LIFE_TINT));
        // STAND-IN: docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured
        // -- batteries are not simulated: a unit with one reads full, one without empty.
        let (t, piece) = arc(if designation.battery { 100.0 } else { 0.0 });
        let piece = Piece { page, ..piece };
        let rect = if own { [640.0, 314.0 + t, 600.0, 408.0] } else { [0.0, 314.0 + t, 40.0, 408.0] };
        if piece.rect[3] > 0.0 {
            ink.painter.piece(piece, rect, argb(ENERGY_TINT));
        }
    }
    let x0 = if own { 498.0 } else { 4.0 };
    let name = match shown {
        None => cockpit.panels.hero_name.clone(),
        Some(t) => cockpit.panels.names.get(t).cloned().unwrap_or_default(),
    };
    ink.centred(&name, x0, 138.0, 460.0, NAME_COLOUR);
    // A unit of the player's clan gets its status under its name; a hero or a building none.
    //
    // STAND-IN: docs/35-hud.md#name-and-status--read-and-seen -- the component value `0x400`
    // the "Dangerous!" test asks for is not read: no other unit is called dangerous.
    if let Some(t) = shown.filter(|&t| {
        !building && play.units[t].type_word != ROBOT_HERO && play.units[t].clan == Some(play.player_clan)
    }) {
        let status = status(play, t);
        ink.centred(&format!("[{}]", cockpit.string(status)), x0, 138.0, 469.0, NAME_COLOUR);
    }
    if !own && distance > 0.0 {
        put(ink, skin, "targeter_range", [108.0, 440.0, 147.0, 456.0]);
        let text = format!("{} {}", distance as i32, cockpit.string(STRING_METRES));
        ink.centred(&text, 112.0, 31.0, 444.0, RANGE_COLOUR);
    }
    Some(view)
}

/// A unit of the player's clan's status: its head order's string, or "no order".
///
/// STAND-IN: docs/31-packages.md#the-orders--measured -- the engine keeps the task an order
/// built rather than the order queue: the running task names the order.
fn status(play: &Play, t: usize) -> u32 {
    let Some((_, robot)) = play.robots.iter().find(|(target, _)| *target == t) else {
        return STRING_NO_ORDER;
    };
    let order = match robot.behaviour.task() {
        Task::Stop => return STRING_NO_ORDER,
        Task::StayGround => 21,
        Task::Follow { .. } => 22,
        Task::Search { .. } => robot.order.map_or(5, |o| if o.code == 17 { 17 } else { 5 }),
        Task::Reload => 8,
        Task::Attack { .. } => 3,
        Task::Leave { .. } => 20,
    };
    order_status(order)
}

/// The frame about the target in the world (`0x10041497`–`0x100416f9`): a square outline in its
/// mark colour, easing in from the screen's middle over the first second after its clock
/// restarts, which it does whenever the target does not project.
///
/// STAND-IN: docs/35-hud.md#the-frame-around-the-target-in-the-world--read -- the driven unit
/// record's `+0x10 ÷ +0x14` in the scale is not read: it is taken as the camera's focal
/// length, so the square's half-side is two thirds of the target's projected radius.
#[allow(clippy::too_many_arguments)]
fn frame(
    cockpit: &mut Cockpit,
    ink: &mut Ink,
    play: &Play,
    t: usize,
    centre: Vec3,
    radius: f32,
    distance: f32,
    view_proj: Mat4,
    now_ms: f64,
) {
    let alive = play.battle.combat.targets.get(t).is_some_and(|x| x.alive);
    let space = ink.painter.space;
    let clip = view_proj * centre.extend(1.0);
    if clip.w <= 1e-6 || !alive {
        cockpit.panels.frame_clock_ms = now_ms;
        return;
    }
    let to_layout = |ndc: [f32; 2]| {
        let pixel = [(ndc[0] + 1.0) * 0.5 * space.width, (1.0 - ndc[1]) * 0.5 * space.height];
        space.layout(pixel, Pin::CENTRE)
    };
    let [x, y] = to_layout([clip.x / clip.w, clip.y / clip.w]);
    // The target's projected radius on the layout, along the camera's up.
    let camera_up = view_proj.inverse().transform_vector3(Vec3::Y).normalize_or(Vec3::Z);
    let edge = view_proj * (centre + camera_up * radius).extend(1.0);
    let [ex, ey] = to_layout([edge.x / edge.w.max(1e-6), edge.y / edge.w.max(1e-6)]);
    let projected = (ey - y).hypot(ex - x);
    let mut s = if distance > 0.0 { 2.0 / 3.0 * projected / FRAME_HALF_SIDE } else { FRAME_SCALE_CAP };
    if s < FRAME_SCALE_FLOOR {
        s = FRAME_SCALE_LOW;
    }
    s = s.min(FRAME_SCALE_CAP);
    let elapsed = now_ms - cockpit.panels.frame_clock_ms;
    let (mut cx, mut cy, mut half) = (x, y, FRAME_HALF_SIDE * s);
    if elapsed < FRAME_EASE_MS {
        let e = (1.0 - elapsed / FRAME_EASE_MS) as f32;
        cx += (320.0 - x) * e;
        cy += (240.0 - y) * e;
        half = FRAME_HALF_SIDE * (s + (FRAME_SCALE_CAP - s) * e);
    }
    let [r, g, b] = play.mark_colour(play.units.get(t).and_then(|u| u.clan)).map(|c| f32::from(c) / 255.0);
    let colour = [r, g, b, 1.0];
    let (pin, over) = (ink.painter.pin, ink.painter.over);
    ink.painter.pin = Pin::CENTRE;
    let line = 1.0 / space.scale();
    let (x0, y0, x1, y1) = (cx - half, cy - half, cx + half, cy + half);
    for rect in [
        [x0, y0, x1 - x0, line],
        [x0, y1 - line, x1 - x0, line],
        [x0, y0, line, y1 - y0],
        [x1 - line, y0, line, y1 - y0],
    ] {
        ink.painter.fill(Blend::Alpha, rect, colour);
    }
    ink.painter.pin = pin;
    ink.painter.over = over;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings() -> BTreeMap<u32, String> {
        [(STRING_HUMAN, "Human"), (STRING_WARRIOR, "Warrior"), (STRING_ANIMAL, "Animal")]
            .into_iter()
            .map(|(k, v)| (k, v.to_owned()))
            .collect()
    }

    #[test]
    fn a_unit_is_named_by_its_size_chassis_and_class_and_its_place_in_its_clan() {
        let s = strings();
        assert_eq!(name(TYPE_WARRIOR, 1, 1, 2, &s), "TFW-2 Warrior");
        assert_eq!(name(TYPE_WARRIOR, 2, 2, 1, &s), "SSW-1 Warrior");
        assert_eq!(name(TYPE_WARRIOR, 3, 1, 1, &s), "MFW-1 Warrior");
        assert_eq!(name(ROBOT_HERO, 2, 2, 0, &s), "Human");
    }

    #[test]
    fn a_node_is_green_whole_and_red_destroyed() {
        assert_eq!(node_colour(1.0), [0.0, 0.5, 0.0]);
        assert_eq!(node_colour(0.0), [0.5, 0.0, 0.0]);
    }
}
