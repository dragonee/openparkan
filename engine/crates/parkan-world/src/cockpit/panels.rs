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
use crate::hud::{Blend, Layer, Piece, Pin};
use crate::play::{Play, ROBOT_HERO};
use crate::robot::Designation;

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
/// Strings: "Human", "Animal", "Tiny Tower", "m", "no order", the order statuses, and the class
/// words.
pub const STRING_HUMAN: u32 = 6230;
pub const STRING_ANIMAL: u32 = 6253;
pub const STRING_TINY_TOWER: u32 = 6076;
pub const STRING_METRES: u32 = 6178;
/// "Dangerous!", under a unit carrying a heavy gun (`0x100766f0`).
pub const STRING_DANGEROUS: u32 = 6255;
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
/// What the cockpit says as the player's unit's repair system is switched on and off
/// (`iron3d.dll:0x100a5485`, docs/26, "Repair").
pub const VOICE_REPAIR_ON: &str = "VOICE_REPAIR_SYS_ON";
pub const VOICE_REPAIR_OFF: &str = "VOICE_REPAIR_SYS_OFF";

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
    /// The unit's two lights for this view ([`view_lights`]).
    pub lights: [ViewLight; 2],
}

/// A light of a unit's view: the way it travels in the world, the vector's length the
/// light's own, and its grey.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewLight {
    pub travel: Vec3,
    pub colour: f32,
}

/// The first of the two lights a unit's mesh keeps for a panel's view
/// (`AniMesh.dll:0x10007102`-`0x1000723b`, docs/35, "The unit in the middle"): directional,
/// of its own light manager, flagged `0x84000000`, which is what the view's draw gathers and
/// the world's passes over. It travels (−1, 1, −1) **in the unit's own frame** -- the gather
/// turns it through the unit's placement (`Terrain.dll:0x100802f0`) -- so it falls on the
/// unit's right, back and top however the unit stands. The vector is stored and used as it is,
/// not normalised, and the colour is grey 0.25.
pub const VIEW_LIGHT_TRAVEL: Vec3 = Vec3::new(-1.0, 1.0, -1.0);
pub const VIEW_LIGHT_COLOUR: f32 = 0.25;
/// The second light travels the other way at this share of the first's colour (`0x10005a7d`).
pub const VIEW_COUNTER_SHARE: f32 = 0.35;

/// How many times over a unit that carries a turret is lit by them, against a unit that is
/// its chassis alone.
///
/// STAND-IN: docs/35-hud.md#the-unit-in-the-middle--read-and-seen -- the read finds one pair of
/// lights on a unit's mesh, and the recordings show that much on a unit of one part and twice
/// it on one of two or more. Measured: Mission 01's dummy, a chassis alone, is 0.136 over its
/// node's green in the recording and 0.139 here at one pair; C03 M02's hero, a chassis and a
/// turret, is lit 2.06, 1.98 and 2.03 times what one pair gives (the median green, and blue's
/// ninetieth and ninety-ninth percentiles). What makes the second pair is not read.
pub const VIEW_LIGHT_PAIRS_WITH_A_TURRET: f32 = 2.0;

/// The two lights of a unit standing at `turn`, lit `pairs` times over.
pub fn view_lights(turn: Quat, pairs: f32) -> [ViewLight; 2] {
    let travel = turn * VIEW_LIGHT_TRAVEL;
    [
        ViewLight { travel, colour: VIEW_LIGHT_COLOUR * pairs },
        ViewLight { travel: -travel, colour: VIEW_LIGHT_COLOUR * VIEW_COUNTER_SHARE * pairs },
    ]
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
    /// The player's unit and whether its repair system was on, last seen.
    last_repair: Option<(Option<usize>, bool)>,
}

/// A unit's name, the unit record's slot 1 (`0x10075d50`, `0x10076270`): "Human" for a hero,
/// "Animal", "Tiny Tower" for a robot whose device manager answers its id 2 with 0 or less, or
/// its size, chassis and class letters, its place among its clan's named units, and its class
/// word. The record's bind hands it the class word `0x10076490` gives by Type (`0x10074e88`,
/// `0x10074f2b`), so each class letter takes its own.
pub fn name(
    type_word: u32,
    designation: Designation,
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
    if designation.tiny_tower(type_word) {
        return string(STRING_TINY_TOWER);
    }
    let Designation { size_class, chassis_type, .. } = designation;
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

/// Target `t`'s name as a target made in play takes it: a building's is the string its
/// record's slot 1 hands its behaviour (`0x10033720` → `0x100338d0`), by its Type and its size
/// class ([`Play::building_name`], docs/35, "Name and status"), which the target panel prints as
/// it prints a unit's; a unit's is the one its build gave it ([`Play::names`]).
pub fn target_name(play: &Play, t: usize, strings: &BTreeMap<u32, String>) -> String {
    match play.units.get(t) {
        Some(u) if u.kind == KIND_BUILDING => play.building_name(t, strings),
        _ => play.names.get(t).cloned().unwrap_or_default(),
    }
}

/// The status string of an order (docs/31-packages.md, "The orders"), by its number.
pub fn order_status(order: i32) -> u32 {
    match order {
        0 => STRING_NO_ORDER,
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
    /// Every unit and building of `play` named. Each clan counts its own units in file order,
    /// and the name raises its clan's count for every unit it names, whichever string it gives:
    /// the hero's "Human", an animal's "Animal" and a *Tiny Tower*'s too (`0x10075eb2`). So the
    /// player's first bot after the hero is its clan's second: *seen*, Mission 03's builder is
    /// **SWB-2** and *Outflanking Maneuver*'s wingman **SWW-2** (docs/35, "Name and status"). A
    /// building is named by its Type and its size class alone ([`target_name`]).
    pub fn new(play: &Play, strings: &BTreeMap<u32, String>) -> Panels {
        // The units in the mission's order, the hero among them at its own place.
        let mut order: Vec<(usize, Option<usize>)> = play
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.kind == KIND_UNIT)
            .map(|(t, _)| (play.battle.objects.get(t).copied().unwrap_or(usize::MAX), Some(t)))
            .collect();
        order.push((play.hero.object, None));
        order.sort_by_key(|&(object, _)| object);
        let mut counts: BTreeMap<Option<i64>, usize> = BTreeMap::new();
        let mut names = vec![String::new(); play.units.len()];
        for (_, t) in order {
            let clan = t.map_or(Some(play.player_clan), |t| play.units[t].named_clan);
            let count = counts.entry(clan).or_default();
            *count += 1;
            if let Some(t) = t {
                let u = &play.units[t];
                names[t] = name(u.type_word, u.designation, *count, strings);
            }
        }
        for (t, u) in play.units.iter().enumerate() {
            if u.kind == KIND_BUILDING {
                names[t] = target_name(play, t, strings);
            }
        }
        Panels {
            names,
            hero_name: name(ROBOT_HERO, Designation::default(), 0, strings),
            frame_clock_ms: 0.0,
            last_voices_ms: [None; 2],
            last_repair: None,
        }
    }

    /// The own panel's voices due at `now_ms`, and the repair system's as the player switches it
    /// (`iron3d.dll:0x10076e10`): a change on the same unit, one with a repair system.
    pub fn voices(&mut self, play: &Play, now_ms: f64) -> Vec<&'static str> {
        let life = life_share(play.hero.lives.iter().flatten());
        let mut out = Vec::new();
        let due = |last: Option<f64>| last.is_none_or(|t| now_ms - t >= VOICE_GAP_MS);
        if life < 0.2 && due(self.last_voices_ms[0]) {
            out.push(VOICE_LIFE_LOW);
            self.last_voices_ms[0] = Some(now_ms);
        }
        let unit = play.driven_target();
        if play.battery(unit).is_some_and(|b| b < 0.2) && due(self.last_voices_ms[1]) {
            out.push(VOICE_BATTERY_LOW);
            self.last_voices_ms[1] = Some(now_ms);
        }
        let robot = match unit {
            None => Some(&play.hero.robot),
            Some(t) => play.robots.iter().find(|(rt, _)| *rt == t).map(|(_, r)| r),
        };
        if robot.is_some_and(|r| r.power.as_ref().is_some_and(|p| p.repair.is_some())) {
            let on = play.driving.as_ref().map_or(play.hero.pilot.switches, |d| d.pilot.switches).repair;
            if matches!(self.last_repair, Some((u, was)) if u == unit && was != on) {
                out.push(if on { VOICE_REPAIR_ON } else { VOICE_REPAIR_OFF });
            }
            self.last_repair = Some((unit, on));
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
    ink.painter.layer = Layer::UnderViews;
    views
}

/// The driven unit's sphere in the world as its object stands this frame (interface `0x20`
/// slot 3 answers from the object's matrix, which the machine places between its step's two
/// poses, `Control.dll:0x10015a50`): its agent sphere carried by the drawn placement, not by
/// the step's end, which jumps a stride at a time.
fn hero_sphere(play: &Play) -> (Vec3, f32) {
    let unit = play.driven();
    let (position, _) = unit.walker.drawn(unit.time_ms);
    let (centre, radius) = unit.collision;
    (position + unit.walker.drawn_turn(unit.time_ms) * centre, radius)
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
    ink.painter.layer = Layer::UnderViews;
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
    let (centre, radius, designation, life, building, shield) = match shown {
        None => (
            hero_centre,
            hero_radius,
            play.hero_designation,
            life_share(play.hero.lives.iter().flatten()),
            false,
            play.battle.combat.hero.as_ref().and_then(|h| h.shield.as_ref()),
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
                target.shield.as_ref(),
            )
        }
    };

    // 4: the six sectors, with a fight shield and a deflector: each its fill × the deflector's
    // level × its condition, red at 0 and green full.
    if designation.shielded {
        let dx = if own { 491.0 } else { 0.0 };
        let sectors = [
            ("frwd_shld", [44.0, 321.0, 105.0, 340.0]),
            ("back_shld", [30.0, 409.0, 119.0, 438.0]),
            ("left_shld", [16.0, 334.0, 42.0, 413.0]),
            ("left_shld", [133.0, 334.0, 107.0, 413.0]),
            ("top_shld", [40.0, 308.0, 109.0, 327.0]),
            ("bott_shld", [22.0, 420.0, 127.0, 451.0]),
        ];
        for (s, (name, [x0, y0, x1, y1])) in sectors.into_iter().enumerate() {
            let share = shield.map_or(1.0, |sh| sh.fills[s] * sh.level * sh.deflector_condition);
            let v = (255.0 * share.clamp(0.0, 1.0)) as u32;
            let colour = argb(0xff00_0000 | ((255 - v) << 16) | (v << 8));
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
    // The shown object's turn, which its two lights turn with, and how many parts it is: the
    // hero is its chassis and its turret.
    let (turn, parts) = match shown {
        Some(t) => play
            .battle
            .combat
            .targets
            .get(t)
            .map_or((Quat::IDENTITY, 1), |target| (target.rotation(), target.parts.len())),
        None => (play.hero.walker.drawn_turn(play.hero.time_ms), 2),
    };
    let pairs = if parts > 1 { VIEW_LIGHT_PAIRS_WITH_A_TURRET } else { 1.0 };
    let direction = if own {
        // The object's y column, as it is drawn.
        let unit = play.driven();
        unit.walker.drawn_turn(unit.time_ms) * Vec3::Y
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
        lights: view_lights(turn, pairs),
    };

    // 7–10, over the view.
    ink.painter.layer = Layer::OverViews;
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
        let battery = play.battery(shown).unwrap_or(if designation.battery { 1.0 } else { 0.0 });
        let (t, piece) = arc(100.0 * battery);
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
    // Any other unit is "Dangerous!" in red while it carries a gun whose node has life left and
    // whose round does at least 10,000 (`0x100766f0`, `IDeviceManager` values `0x400` and 6).
    if let Some(t) = shown.filter(|&t| !building && play.units[t].type_word != ROBOT_HERO) {
        if play.units[t].clan == Some(play.player_clan) {
            let status = status(play, t);
            ink.centred(&format!("[{}]", cockpit.string(status)), x0, 138.0, 469.0, NAME_COLOUR);
        } else if dangerous(play, t) {
            ink.centred(cockpit.string(STRING_DANGEROUS), x0, 138.0, 469.0, DANGEROUS_COLOUR);
        }
    }
    if !own && distance > 0.0 {
        put(ink, skin, "targeter_range", [108.0, 440.0, 147.0, 456.0]);
        let text = format!("{} {}", distance as i32, cockpit.string(STRING_METRES));
        ink.centred(&text, 112.0, 31.0, 444.0, RANGE_COLOUR);
    }
    Some(view)
}

/// Whether unit `t` carries a heavy gun: one whose node has life left (value `0x400` above 0) and
/// whose round does at least 10,000 (value 6, docs/35, "Name and status").
pub fn dangerous(play: &Play, t: usize) -> bool {
    play.machine(t)
        .is_some_and(|r| r.guns.iter().any(|g| !g.broken && g.round_damage >= parkan_sim::guns::HEAVY_ROUND))
}

/// A unit of the player's clan's status: its head order's string, or "no order".
///
/// STAND-IN: docs/31-packages.md#the-orders--measured -- the engine keeps the task an order
/// built rather than the order queue: the running task names the order.
fn status(play: &Play, t: usize) -> u32 {
    order_status(status_order(play, t))
}

/// The order a unit's status line names: its running task's (0 for none).
pub fn status_order(play: &Play, t: usize) -> i32 {
    let Some((_, robot)) = play.robots.iter().find(|(target, _)| *target == t) else {
        return 0;
    };
    match robot.behaviour.task() {
        Task::Stop => 0,
        Task::StayGround => 21,
        Task::Shutdown => 19,
        Task::Migrate { .. } => 15,
        Task::Patrol { .. } => 4,
        Task::Follow { .. } => 22,
        Task::Search { .. } => robot.order.map_or(5, |o| if o.code == 17 { 17 } else { 5 }),
        Task::Reload { .. } => 8,
        Task::Attack { .. } => 3,
        Task::Leave { .. } => 20,
        Task::Go { .. } => 2,
        Task::Build { .. } => 7,
        Task::Transport { .. } => 6,
        Task::Upgrade { .. } => 24,
    }
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
    let (pin, layer) = (ink.painter.pin, ink.painter.layer);
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
    ink.painter.layer = layer;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings() -> BTreeMap<u32, String> {
        [
            (STRING_HUMAN, "Human"),
            (STRING_WARRIOR, "Warrior"),
            (STRING_ANIMAL, "Animal"),
            (STRING_TINY_TOWER, "Tiny Tower"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_owned()))
        .collect()
    }

    #[test]
    fn a_unit_is_named_by_its_size_chassis_and_class_and_its_place_in_its_clan() {
        let s = strings();
        let d = |size_class, chassis_type| Designation { size_class, chassis_type, ..Designation::default() };
        assert_eq!(name(TYPE_WARRIOR, d(1, 1), 2, &s), "TFW-2 Warrior");
        assert_eq!(name(TYPE_WARRIOR, d(2, 2), 1, &s), "SSW-1 Warrior");
        assert_eq!(name(TYPE_WARRIOR, d(3, 1), 1, &s), "MFW-1 Warrior");
        assert_eq!(name(ROBOT_HERO, d(2, 2), 0, &s), "Human");
    }

    #[test]
    fn a_robot_with_a_battery_of_negative_capacity_is_a_tiny_tower() {
        // `0x10075e17`: the device manager's id 2 answers the first capacity below 0, which
        // only the Small Tower chassis R_B_06's unslotted battery, -1, has.
        let s = strings();
        let tower = Designation {
            size_class: 4,
            chassis_type: 2,
            battery: true,
            negative_battery: true,
            ..Designation::default()
        };
        assert_eq!(name(TYPE_WARRIOR, tower, 10, &s), "Tiny Tower");
        // The hero and an animal are named before the query is asked.
        assert_eq!(name(TYPE_ANIMAL, tower, 1, &s), "Animal");
        assert_eq!(name(ROBOT_HERO, tower, 1, &s), "Human");
        // No battery at all fails the query: the dummies read "SSW-1 Warrior" (docs/35).
        let dummy = Designation { size_class: 2, chassis_type: 2, ..Designation::default() };
        assert_eq!(name(TYPE_WARRIOR, dummy, 1, &s), "SSW-1 Warrior");
    }

    #[test]
    fn the_views_lights_turn_with_the_unit() {
        // Standing as placed, the first falls on the faces that look right, back and up, and
        // the second, 0.35 of it, on the opposite ones.
        // 0x3e800000 four times but the alpha, and 0x3eb33333 (`AniMesh.dll:0x100071d3`-
        // `0x1000721f`).
        assert_eq!((f32::from_bits(0x3e80_0000), f32::from_bits(0x3eb3_3333)), (0.25, 0.35));
        let [a, b] = view_lights(Quat::IDENTITY, 1.0);
        assert_eq!((a.travel, a.colour), (Vec3::new(-1.0, 1.0, -1.0), 0.25));
        assert_eq!(b.travel, Vec3::new(1.0, -1.0, 1.0));
        assert!((b.colour - 0.0875).abs() < 1e-6);
        // Turned a quarter left about z, the unit's right is the world's +y and its back the
        // world's +x: the light still comes from its right, its back and above.
        let [a, _] = view_lights(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), 1.0);
        assert!(a.travel.abs_diff_eq(Vec3::new(-1.0, -1.0, -1.0), 1e-5), "{:?}", a.travel);
        // A unit with a turret has both at twice that.
        let [a, b] = view_lights(Quat::IDENTITY, VIEW_LIGHT_PAIRS_WITH_A_TURRET);
        assert!((a.colour - 0.5).abs() < 1e-6 && (b.colour - 0.175).abs() < 1e-6);
    }

    #[test]
    fn a_node_is_green_whole_and_red_destroyed() {
        assert_eq!(node_colour(1.0), [0.0, 0.5, 0.0]);
        assert_eq!(node_colour(0.0), [0.5, 0.0, 0.0]);
    }
}
