//! Selecting and ordering with the cursor in command mode: the pick under the cursor and the
//! cursor it shows, a left click in the world or on the satellite map, the band, the right
//! button, and the picks an order row leaves open. See `docs/42-selection.md`.

use glam::{Mat4, Vec3};
use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT};
use parkan_sim::hit::{SIGHT_SKIPS_FACE, swept_spheres};
use parkan_sim::hq;
use parkan_sim::orders::{self, Order, Target};

use crate::play::Play;
use crate::selection::{BATTLE_UNITS, BUILDERS, BUNKERS, TRANSPORTS};

/// `CState`'s pick modes (`0x1010c388`, named by the log switch at `0x1005a5cc`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickMode {
    #[default]
    Free,
    AttackTarget,
    Building,
    Guard,
    /// Placing a building other than a mine, and a mine.
    PlaceFb,
    Route,
    PlaceFm,
}

/// What an order row left open on the first selected unit (`+0xa8`–`+0xc0`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub unit: usize,
    pub kind: PendingKind,
    /// The unit's point list: a route's places.
    pub points: Vec<Vec3>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PendingKind {
    Route,
    Guard,
    /// A building of this Type to place.
    Build(u32),
}

/// The building a Build row places, following the cursor (docs/32, "Placing a building").
#[derive(Clone, Debug, PartialEq)]
pub struct Ghost {
    pub type_word: u32,
    /// The scheme's first `.dat`, drawn flat.
    pub path: String,
    pub at: Vec3,
    pub yaw: f32,
    /// Green when the site is good, red when not.
    pub valid: bool,
    /// Whether it has been placed on the world yet.
    pub placed: bool,
}

/// `,` and `.` turn the ghost by this much a press (`0x100725b2`, `0x100e50a4`).
pub const GHOST_TURN: f32 = 0.05;
/// The ghost's colours: good and bad (`0x10058239`).
pub const GHOST_GOOD: u32 = 0xff00_ff00;
pub const GHOST_BAD: u32 = 0xffff_0000;
/// What a ray missing the world says, the first time (docs/32).
pub const VOICE_POINT_LAND: &str = "VOICE_POINT_LAND";

/// Where the cursor points: a ray from the command camera, a world place on the open
/// satellite map, or nothing the world takes (the panel, the resource rows).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aim {
    Ray { eye: Vec3, direction: Vec3 },
    Map([f32; 2]),
    Nothing,
}

/// The pick under the cursor (`0x1008da40`): its kind, the object and the world place.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pick {
    pub kind: u8,
    pub object: Option<usize>,
    pub point: Option<Vec3>,
}

/// A unit's pick on the map reaches this far, a building's this far (`0x10072b70`,
/// `0x100728e0`); a ray takes a building, the world's class 3, within this share of its
/// radius, and a unit, class 4, within all of it (`0x100360f0`, docs/42, "What it looks at").
pub const MAP_UNIT_REACH: f32 = 40.0;
pub const MAP_BUILDING_REACH: f32 = 80.0;
pub const RAY_BUILDING_SHARE: f32 = 0.7;
pub const RAY_UNIT_SHARE: f32 = 1.0;
/// The ray's world place is kept this share of the map's side inside it (`0x10035eee`).
pub const MAP_MARGIN_SHARE: f32 = 0.001;
/// A drag becomes a band once the button has been held this long, and a band selects only
/// when this many window pixels across and down (`0x100714ad`, `0x10071659`).
pub const BAND_HOLD_S: f64 = 0.35;
pub const BAND_LEAST_PIXELS: f32 = 10.0;
/// The band's colour, opaque (`0xff19b419`).
pub const BAND_COLOUR: u32 = 0xff19_b419;
/// What a placement cancelled says (`0x1008fe08`).
pub const STRING_BUILDING_CANCELLED: u32 = 6207;
/// The classes the cursor's ray asks the world for, `1 << class`: the query record's first
/// word (`0x10035e82`, `[0xa, 0, 0, 0, 0, 0, 0, 0]`), the landscape (1) and the buildings (3).
pub const CURSOR_RAY_CLASSES: u32 = 0xa;
/// The patrol radius the Guard row's pick gives (`0x1007a08c`), and the one a click's guard
/// gives outside it (`0x10078e85`; 150 in the auto-demo).
pub const GUARD_PICK_RADIUS: i32 = 300;
pub const GUARD_CLICK_RADIUS: i32 = 100;

/// The buildings a unit's capture pick refuses: a main teleport, a bridge, a ruin.
const UNTAKEABLE: [u32; 3] = [0x8000_0200, 0x8000_1000, 0x8000_2000];

/// Cursor state 1, `ARROW`: the state a pick of kind 0 shows, and so the state wherever the
/// pick answers nothing, as in a building's screen and the warbot designer (docs/36).
pub const ARROW: u8 = 1;

/// The cursor state a pick's kind shows (`0x10058740`, table `0x100587b4`): 1 `ARROW`,
/// 2 `PICK`, 3 `PLACE`, 4 `TARGET`, 5 `GUARD`, 6 `CAPTURE`, 9 `WRONG_PLACE`.
pub fn cursor_state(kind: u8) -> u8 {
    match kind {
        1 | 12 => 3,
        2 => 9,
        3 => 4,
        4 => 6,
        5..=7 => 2,
        8..=10 => 5,
        _ => 1,
    }
}

/// The `ui/cursor.cfg` object a cursor state draws: its sprite strip's offset on `new_ui1` and
/// its hot spot (docs/42, "The files").
pub fn cursor_object(state: u8) -> Option<([f32; 2], [f32; 2])> {
    Some(match state {
        1 | 7 => ([0.0, 0.0], [0.0, 0.0]),
        2 => ([0.0, 16.0], [8.0, 8.0]),
        3 => ([192.0, 16.0], [8.0, 8.0]),
        4 => ([0.0, 32.0], [8.0, 8.0]),
        5 => ([128.0, 0.0], [8.0, 8.0]),
        6 => ([64.0, 0.0], [8.0, 8.0]),
        9 => ([64.0, 32.0], [8.0, 8.0]),
        10 => ([128.0, 32.0], [8.0, 8.0]),
        _ => return None,
    })
}

/// A cursor sprite's side and its four phases' step, ms.
pub const CURSOR_SIDE: f32 = 16.0;
pub const CURSOR_PHASE_MS: f64 = 150.0;

/// The world ray under a point of the window through `view_proj`, as `0x10035e40` casts it.
pub fn ray(view_proj: Mat4, eye: Vec3, cursor: [f32; 2], size: [f32; 2]) -> Aim {
    let ndc = [2.0 * cursor[0] / size[0].max(1.0) - 1.0, 1.0 - 2.0 * cursor[1] / size[1].max(1.0)];
    let far = view_proj.inverse().project_point3(Vec3::new(ndc[0], ndc[1], 0.5));
    let direction = (far - eye).normalize_or_zero();
    if direction == Vec3::ZERO { Aim::Nothing } else { Aim::Ray { eye, direction } }
}

/// The map's side, which the satellite map's texels scale to.
fn side(play: &Play) -> f32 {
    play.ground.world_box().1.truncate().max_element()
}

impl Play {
    /// The kind the selection is (`0x1010c384`): 0 none, 1 units, 2 a building.
    pub fn selection_kind(&self) -> u8 {
        if !self.selected_units().is_empty() {
            1
        } else if !self.selected.is_empty() {
            2
        } else {
            0
        }
    }

    /// Whether a selection may be sent to `point` (`0x10076770`): an areal holds it, and every
    /// selected unit flies or the areal's first flag word is set. The game loads no map without
    /// an areal map; on one here a place is valid where there is ground above any water.
    pub fn valid_place(&self, point: Vec3) -> bool {
        let flyers = self.selected_units().iter().all(|&t| self.units[t].designation.chassis_type == 1);
        if let Some(graph) = &self.graph {
            return graph.areal_at(point.x, point.y).is_some() && (flyers || graph.usable(point.x, point.y));
        }
        if flyers {
            return true;
        }
        let Some(ground) = self.ground.below(point.x, point.y, point.z + 1.0) else { return false };
        self.ground.water(point.x, point.y, ground.point.z).is_none_or(|w| w <= ground.point.z)
    }

    /// The pick for `aim` (`0x1008da40`).
    pub fn pick(&self, aim: Aim) -> Pick {
        let mode = self.commander.pick_mode;
        if matches!(mode, PickMode::PlaceFb | PickMode::PlaceFm) {
            return Pick { kind: 13, ..Pick::default() };
        }
        let (point, object) = match aim {
            Aim::Nothing => return Pick::default(),
            Aim::Map(xy) => self.map_pick(xy),
            Aim::Ray { eye, direction } => self.ray_pick(eye, direction),
        };
        let mut pick = Pick { kind: 0, object, point };
        let is_building = |t: usize| self.units.get(t).is_some_and(|u| u.kind == KIND_BUILDING);
        // A building building itself is dropped, and the hero never takes orders.
        if object.is_some_and(|t| is_building(t) && self.building_itself(t)) {
            pick.object = None;
            pick.kind = 2;
            return pick;
        }
        let selected = self.selected_units();
        if selected.first().is_some_and(|&t| self.units[t].logical_id == self.hero_id) {
            pick.kind = 2;
            return pick;
        }
        let valid = point.is_some_and(|p| self.valid_place(p));
        if mode == PickMode::Route {
            pick.kind = if valid { 12 } else { 2 };
            return pick;
        }
        // The hero, which has no record here beside the others', is the player's own unit.
        let own = |t: usize| self.is_hero(t) || self.units[t].clan == Some(self.player_clan);
        pick.kind = match self.selection_kind() {
            0 => match object {
                Some(t) if own(t) => 7,
                _ => 0,
            },
            2 => match object {
                Some(t) if own(t) && !is_building(t) => 7,
                Some(t) if self.selected.first() == Some(&t) => 17,
                Some(t) if own(t) => 7,
                _ => 0,
            },
            _ => match mode {
                PickMode::AttackTarget => {
                    if object.is_some() {
                        16
                    } else {
                        0
                    }
                }
                PickMode::Building => match object {
                    Some(t)
                        if is_building(t) && !own(t) && !UNTAKEABLE.contains(&self.units[t].type_word) =>
                    {
                        4
                    }
                    _ => 0,
                },
                PickMode::Guard => match object {
                    Some(_) => 9,
                    None if valid => 8,
                    None => 0,
                },
                _ => match object {
                    Some(t) if own(t) && !is_building(t) && selected == [t] => 17,
                    Some(t) if own(t) && !is_building(t) => 7,
                    Some(t) if !is_building(t) => 3,
                    Some(t) if own(t) => 10,
                    Some(_) if selected.iter().all(|&u| matches!(self.record_class(u), 1 | 2)) => 4,
                    Some(_) => 3,
                    None if valid => 1,
                    None => 0,
                },
            },
        };
        pick
    }

    /// Whether target `t` is the hero's, the one after every other target
    /// ([`parkan_sim::combat::Combat::hero_index`]), which has no entry in `units`.
    pub fn is_hero(&self, t: usize) -> bool {
        t == self.battle.combat.hero_index()
    }

    /// Whether the hero's object hangs on a parent, without which the world's object pick
    /// passes it over (`IGameObject` slot 3, `0x100361f6`). Only boarding a bot takes it off
    /// (`0x100637f3`–`0x100637fa`), and leaving puts it back (`0x1006391f`); entering a
    /// bunker's command view from on foot leaves it where it stands (`0x10063ca0`).
    pub fn hero_in_world(&self) -> bool {
        self.aboard().is_none()
    }

    /// On the open satellite map: the place, and the first live unit within 40 of it in the
    /// level's order, or else the first building within 80 (`0x10072b70`, `0x100728e0`), of
    /// those the player knows (`0x1007e660`, docs/42, "On the open satellite map").
    ///
    /// What the map marks is what it picks, so a mark is the object it stands for: a click on
    /// another clan's building sends the selection at it, to capture or to attack by the kinds
    /// table, as a click on the building in the world does.
    ///
    /// The unit walk (`0x10072b70`) passes over a dead record and nothing else: the hero is
    /// one of the level's units, the player's own, its place among them the mission's order
    /// (docs/42, "The hero keeps its parent until it boards"). It is taken at
    /// [`Play::hero_place`]; that its record stands there aboard a bot too is *inferred*.
    fn map_pick(&self, [x, y]: [f32; 2]) -> (Option<Vec3>, Option<usize>) {
        let z = self.ground.below(x, y, 1.0e5).map_or(0.0, |h| h.point.z);
        let point = Vec3::new(x, y, z);
        let known = crate::cockpit::map::known_to_player(self);
        let within = |at: Vec3, reach: f32| at.truncate().distance(point.truncate()) <= reach;
        let near = |kind: u32, reach: f32| {
            (0..self.units.len()).find(|&t| {
                self.units[t].kind == kind
                    && known.get(t).copied().unwrap_or(false)
                    && self.units[t].logical_id != self.hero_id
                    && self.battle.combat.targets.get(t).is_some_and(|x| x.alive && within(x.position, reach))
            })
        };
        let hero = (!self.hero.dead() && within(self.hero_place(), MAP_UNIT_REACH))
            .then_some(self.battle.combat.hero_index());
        let unit = match (hero, near(KIND_UNIT, MAP_UNIT_REACH)) {
            (Some(_), Some(t)) if self.battle.objects[t] < self.hero.object => Some(t),
            (Some(h), _) => Some(h),
            (None, t) => t,
        };
        (Some(point), unit.or_else(|| near(KIND_BUILDING, MAP_BUILDING_REACH)))
    }

    /// Where the cursor's ray from `eye` along `direction` first meets the world, kept
    /// strictly inside the map by 0.001 of its side (`0x10035e40`, `0x10035eee`). The ray goes
    /// into `IWorld` slot 7 with a query record of its own, `[0xa, 0, 0, 0, 0, 0, 0, 0]`
    /// ([`CURSOR_RAY_CLASSES`]): the world's walk tests only an object whose `1 << class`
    /// meets `0xa`, the landscape (class 1) and the buildings (class 3), and excludes no face.
    /// So a unit, a tree or a stone does not stop it, and the ground behind is where it points;
    /// a building's walls and roof do; and a lake stops it on its sheet, as the sight ray's
    /// empty exclusions do (docs/29).
    pub fn cursor_point(&self, eye: Vec3, direction: Vec3) -> Option<Vec3> {
        let (lo, hi) = self.ground.world_box();
        let (p0, p1) = (eye, eye + direction * ((hi - lo).length() + 200.0));
        let mut best = self.ground.segment_including_water(p0, p1);
        for (t, target) in self.battle.combat.every() {
            let building = self.units.get(t).is_some_and(|u| u.kind == KIND_BUILDING);
            if !building
                || !target.alive
                || self.deleted.get(t).copied().unwrap_or(false)
                || swept_spheres((target.centre, target.centre), target.radius, (p0, p1), 0.0).is_none()
            {
                continue;
            }
            for part in &target.parts {
                if let Some(s) = part.segment(p0, p1, SIGHT_SKIPS_FACE)
                    && best.as_ref().is_none_or(|b| s.d2 < b.d2)
                {
                    best = Some(s);
                }
            }
        }
        let margin = side(self) * MAP_MARGIN_SHARE;
        best.map(|s| s.point).filter(|p| {
            p.x > lo.x + margin && p.y > lo.y + margin && p.x < hi.x - margin && p.y < hi.y - margin
        })
    }

    /// In the world: where the cursor's ray meets the world ([`Play::cursor_point`]), and the
    /// object whose bounding sphere the ray passes (`0x100360f0`): the buildings at 0.7 of
    /// their radius, then the units at all of it, the nearest centre to the eye winning
    /// ([`nearest_on_ray`]).
    ///
    /// The frustum test each object passes first (`0x10036280`) is left out: a sphere the
    /// cursor's ray passes ahead of the eye is in view.
    ///
    /// The walk passes over an object whose `IGameObject` slot 3, its parent, answers none. The
    /// hero's is set unless it has boarded a bot ([`Play::hero_in_world`]), so in a bunker's
    /// command view the hero is a unit the ray can take.
    fn ray_pick(&self, eye: Vec3, direction: Vec3) -> (Option<Vec3>, Option<usize>) {
        let (lo, hi) = self.ground.world_box();
        let length = (hi - lo).length() + 200.0;
        let point = self.cursor_point(eye, direction);
        let spheres = self.battle.combat.targets.iter().enumerate().filter_map(|(t, target)| {
            let kind = self.units.get(t).map_or(u32::MAX, |u| u.kind);
            let pickable = target.alive
                && (kind == KIND_UNIT || kind == KIND_BUILDING)
                && self.units[t].logical_id != self.hero_id
                && !self.deleted.get(t).copied().unwrap_or(false);
            pickable.then_some(Sphere {
                index: t,
                centre: target.centre,
                radius: target.radius,
                building: kind == KIND_BUILDING,
            })
        });
        let hero =
            self.battle.combat.hero.as_ref().filter(|_| !self.hero.dead() && self.hero_in_world()).map(|h| {
                Sphere {
                    index: self.battle.combat.hero_index(),
                    centre: h.centre,
                    radius: h.radius,
                    building: false,
                }
            });
        (point, nearest_on_ray(eye, direction, length, spheres.chain(hero)))
    }

    /// A left click in the world or on the map (`0x1008fe80`, table `0x10090758`). Returns
    /// the page the panel turns to: a selected unit's only while a page is open, a clicked
    /// selection's own always.
    ///
    /// A click on the hero (kind 7) lets a building go and does nothing else: selecting a unit
    /// refuses a hero (`0x1007d0d4`), and no page answers its Type (`0x10090207`).
    ///
    /// The Guard row's pick (mode 3) is closed by a unit (kind 9, `0x10090337`) or a place
    /// (kind 8); a building under the cursor also answers 9, and kind 9 wants a unit, so the
    /// click does nothing and the pick stays open. Its patrol has radius 300; a guard clicked
    /// outside it, on an own building (kind 10), is the dispatcher's, radius 100.
    pub fn click_world(&mut self, pick: Pick) -> Option<(u8, bool)> {
        let place = pick.point.map(|p| Target::Place([p.x.round(), p.y.round(), p.z]));
        let hero = self.battle.combat.hero_index();
        let target_id = pick.object.map(|t| if t == hero { self.hero_id } else { self.units[t].logical_id });
        let unit = pick.object.filter(|&t| t == hero || self.units[t].kind != KIND_BUILDING);
        match pick.kind {
            1 => self.dispatch(Order { code: hq::GO, parameter: 0, target: place? }),
            3 => self.dispatch(Order {
                code: orders::ATTACK,
                parameter: 0,
                target: Target::LogicId(target_id?),
            }),
            4 if self.commander.pick_mode == PickMode::Building => self.commander.pick_mode = PickMode::Free,
            4 => self.dispatch(Order {
                code: orders::SEARCH,
                parameter: orders::CAPTURE_TYPES as i32,
                target: Target::LogicId(target_id?),
            }),
            7 => {
                let t = pick.object?;
                if t == hero {
                    self.selected.clear();
                } else if self.units[t].kind == KIND_BUILDING {
                    self.select_building(t);
                } else {
                    self.select_unit_alone(t);
                    self.commander.pick_mode = PickMode::Free;
                    return match unit_page(self.units[t].type_word) {
                        0 => None,
                        page => Some((page, false)),
                    };
                }
            }
            8 if self.commander.pick_mode == PickMode::Guard => {
                let p = pick.point?;
                self.commander.pick_mode = PickMode::Free;
                self.give_pending(Order {
                    code: orders::PATROL,
                    parameter: GUARD_PICK_RADIUS,
                    target: Target::Place(p.to_array()),
                });
            }
            9 if self.commander.pick_mode == PickMode::Guard => {
                unit?;
                self.commander.pick_mode = PickMode::Free;
                self.give_pending(Order {
                    code: orders::PATROL,
                    parameter: GUARD_PICK_RADIUS,
                    target: Target::LogicId(target_id?),
                });
            }
            9 | 10 => self.dispatch(Order {
                code: orders::PATROL,
                parameter: GUARD_CLICK_RADIUS,
                target: Target::LogicId(target_id?),
            }),
            12 => {
                let p = pick.point?;
                if let Some(pending) = self.commander.pending.as_mut() {
                    pending.points.push(p);
                }
            }
            16 => self.commander.pick_mode = PickMode::Free,
            17 => {
                let t = pick.object?;
                let page = if self.units[t].kind == KIND_BUILDING {
                    building_page(self.units[t].type_word)
                } else {
                    unit_page(self.units[t].type_word)
                };
                return Some((page, true));
            }
            _ => {}
        }
        None
    }

    /// Give `order` to every selected unit, replacing its queue, and the last acknowledges
    /// (`0x10079230`, `0x1008e840`).
    pub fn dispatch(&mut self, order: Order) {
        let selected = self.selected_units();
        let mut class = None;
        for t in &selected {
            if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| rt == t) {
                robot.order = Some(order);
                robot.behaviour.order(&order);
                class = Some(robot.size_class);
            }
        }
        if let Some(class) = class {
            self.acknowledge(class);
        }
    }

    /// The pending pick's own unit takes `order`, replacing its queue, and acknowledges.
    fn give_pending(&mut self, order: Order) {
        let Some(pending) = self.commander.pending.take() else { return };
        if let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == pending.unit) {
            robot.order = Some(order);
            robot.behaviour.order(&order);
            let class = robot.size_class;
            self.acknowledge(class);
        }
    }

    /// An order row with a target opens its pick on the first selected unit (`0x1007b740`,
    /// `0x10079700`). Returns whether the satellite map is to be shown, as a route's is.
    pub fn open_pick(&mut self, act: hq::Act) -> bool {
        let Some(&unit) = self.selected_units().first() else { return false };
        let position = self.battle.combat.targets.get(unit).map_or(Vec3::ZERO, |t| t.position);
        match act {
            hq::Act::Route => {
                self.commander.pick_mode = PickMode::Route;
                self.commander.pending =
                    Some(Pending { unit, kind: PendingKind::Route, points: vec![position] });
                true
            }
            hq::Act::Guard => {
                self.commander.pick_mode = PickMode::Guard;
                self.commander.pending = Some(Pending { unit, kind: PendingKind::Guard, points: Vec::new() });
                false
            }
            hq::Act::Build(type_word) => {
                // Pick mode 6 for a mine, 4 for any other building (`0x10079e57`).
                self.commander.pick_mode =
                    if type_word == hq::BUILD_TYPES[0] { PickMode::PlaceFm } else { PickMode::PlaceFb };
                self.commander.pending =
                    Some(Pending { unit, kind: PendingKind::Build(type_word), points: Vec::new() });
                false
            }
            _ => false,
        }
    }

    /// The right button (`0x1008fb00`): the most specific thing open is undone. Returns what
    /// the panel and the map are to do.
    pub fn right_click(&mut self, page: u8, map_open: bool) -> RightClick {
        match self.commander.pick_mode {
            PickMode::Route => {
                // The route is finished and given: a GO a point, the first replacing.
                self.commander.pick_mode = PickMode::Free;
                if let Some(pending) = self.commander.pending.take()
                    && let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == pending.unit)
                {
                    for (i, p) in pending.points.iter().enumerate() {
                        let order = Order { code: hq::GO, parameter: 0, target: Target::Place(p.to_array()) };
                        let insert = if i == 0 { orders::INSERT_REPLACE } else { orders::INSERT_TO_END };
                        robot.behaviour.insert_order(&order, insert);
                        if i == 0 {
                            robot.order = Some(order);
                        }
                    }
                    let class = robot.size_class;
                    self.acknowledge(class);
                }
                return RightClick::default();
            }
            PickMode::PlaceFb | PickMode::PlaceFm => {
                self.cancel_placement();
                return RightClick::default();
            }
            _ => {}
        }
        if self.mode().commands() && !self.selected_units().is_empty() {
            self.commander.units.clear();
            return RightClick { page_zero: (1..=3).contains(&page), close_map: false };
        }
        if !self.selected.is_empty() {
            self.selected.clear();
            return RightClick { page_zero: (4..=8).contains(&page), close_map: false };
        }
        if page != 0 {
            return RightClick { page_zero: true, close_map: false };
        }
        RightClick { page_zero: false, close_map: map_open }
    }

    /// The ghost this frame, the cursor's ray being `aim`: it stands where the ray meets the
    /// ground or a building strictly inside the map ([`Play::cursor_point`], the same
    /// `0x10035e40`), and a miss leaves it where it was.
    pub fn update_ghost(&mut self, aim: Aim) {
        let Some(Pending { kind: PendingKind::Build(type_word), unit, .. }) = self.commander.pending.clone()
        else {
            self.commander.ghost = None;
            return;
        };
        if self.commander.ghost.as_ref().is_none_or(|g| g.type_word != type_word) {
            let Some(path) = self.placement_model(type_word) else { return };
            self.commander.ghost =
                Some(Ghost { type_word, path, at: Vec3::ZERO, yaw: 0.0, valid: false, placed: false });
        }
        let hit = match aim {
            Aim::Ray { eye, direction } => self.cursor_point(eye, direction),
            _ => None,
        };
        let Some(at) = hit else {
            let first_miss = self.commander.ghost.as_ref().is_some_and(|g| !g.placed);
            if first_miss && !self.commander.missed {
                self.commander.missed = true;
                if let Some(v) = self.progression.as_ref().and_then(|p| p.sound(VOICE_POINT_LAND)) {
                    self.says.push(crate::progress::Say::Voice(v));
                }
            }
            return;
        };
        let yaw = self.commander.ghost.as_ref().map_or(0.0, |g| g.yaw);
        let valid = self.placement_valid(Some(unit), type_word, at, yaw);
        if let Some(g) = self.commander.ghost.as_mut() {
            g.at = at;
            g.placed = true;
            g.valid = valid;
        }
    }

    /// `CMD_JAMES_BASE_ROTLEFT` / `_ROTRIGHT` in a place mode (`0x100725b2`).
    pub fn turn_ghost(&mut self, left: bool) -> bool {
        if !matches!(self.commander.pick_mode, PickMode::PlaceFb | PickMode::PlaceFm) {
            return false;
        }
        if let Some(g) = self.commander.ghost.as_mut() {
            g.yaw += if left { GHOST_TURN } else { -GHOST_TURN };
        }
        true
    }

    /// A left click with the ghost up (`0x1008ff2d`): on a good site the builder is ordered to
    /// build there, and the pick closes; a bad site does nothing.
    pub fn commit_placement(&mut self) -> bool {
        let Some(g) = self.commander.ghost.clone().filter(|g| g.placed && g.valid) else { return false };
        let Some(pending) = self.commander.pending.take() else { return false };
        self.commander.pick_mode = PickMode::Free;
        self.commander.ghost = None;
        self.commander.missed = false;
        self.order_build(pending.unit, g.type_word, g.at, g.yaw)
    }

    /// A placement put away (`0x1008fe08`): string 6207 as a System line.
    pub fn cancel_placement(&mut self) {
        self.commander.pick_mode = PickMode::Free;
        self.commander.pending = None;
        self.commander.ghost = None;
        self.commander.missed = false;
        let text = self.progression.as_ref().and_then(|p| p.strings.get(&STRING_BUILDING_CANCELLED).cloned());
        if let Some(text) = text {
            self.says.push(crate::progress::Say::Text(crate::progress::Sender::System, text));
        }
    }

    /// A band released (`0x10076820`): the selections cleared, then the player's live units
    /// other than heroes inside it taken — by where each lands on the layout through
    /// `view_proj` over `size` window pixels, or on the open map by world place — with one
    /// `VOICE_SELECTED`.
    pub fn band_select(&mut self, corners: [[f32; 2]; 2], space: BandSpace) {
        let [x0, x1] = [corners[0][0].min(corners[1][0]), corners[0][0].max(corners[1][0])];
        let [y0, y1] = [corners[0][1].min(corners[1][1]), corners[0][1].max(corners[1][1])];
        self.clear_selection();
        let inside = |[x, y]: [f32; 2]| x >= x0 && x <= x1 && y >= y0 && y <= y1;
        let taken: Vec<usize> = self
            .own_units_within(u32::MAX)
            .into_iter()
            .filter(|&t| {
                let at = self.battle.combat.targets[t].position;
                match space {
                    BandSpace::World(view_proj, [w, h]) => crate::play::on_screen(view_proj, at, 0.0)
                        .is_some_and(|[nx, ny]| inside([(nx + 1.0) * 0.5 * w, (1.0 - ny) * 0.5 * h])),
                    BandSpace::Map => inside([at.x, at.y]),
                }
            })
            .collect();
        if !taken.is_empty() {
            self.commander.units = taken;
            self.ui_voice_selected();
        }
    }
}

/// Where a band's corners are: window pixels for the world through a camera, or world x and y
/// on the map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BandSpace {
    World(Mat4, [f32; 2]),
    Map,
}

/// What the panel does after a right click.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RightClick {
    pub page_zero: bool,
    pub close_map: bool,
}

/// An object the world ray may pick: its target, its bounding sphere, and whether it is a
/// building, the world's class 3, or a unit, class 4.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    pub index: usize,
    pub centre: Vec3,
    pub radius: f32,
    pub building: bool,
}

/// The object the ray from `eye` along the unit `direction`, `length` long, picks
/// (`0x100360f0`, walking the world's classes 3 and 4 with `0x100361a0`; docs/42, "What it
/// looks at"). The buildings are walked first, at 0.7 of their radius, then the units at all
/// of it, and the two walks share one nearest distance. An object is passed over when the eye
/// is inside its whole sphere, when its centre is not ahead of the eye, or not nearer the eye
/// than the ray is long. Of the rest, the one whose centre is nearest the eye, in a straight
/// line, wins; a later one only when strictly nearer, so a tie keeps the building.
pub fn nearest_on_ray(
    eye: Vec3,
    direction: Vec3,
    length: f32,
    spheres: impl Iterator<Item = Sphere> + Clone,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for building in [true, false] {
        let share = if building { RAY_BUILDING_SHARE } else { RAY_UNIT_SHARE };
        for s in spheres.clone().filter(|s| s.building == building) {
            let to = s.centre - eye;
            let distance = to.length();
            let along = to.dot(direction);
            let off = (to - direction * along).length();
            if distance < s.radius || along <= 0.0 || distance >= length || off > s.radius * share {
                continue;
            }
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, s.index));
            }
        }
    }
    best.map(|(_, t)| t)
}

/// The unit page a unit's Type opens: builders 3, transports 2, warriors and HQs 1.
pub fn unit_page(type_word: u32) -> u8 {
    if hq::within(type_word, BUILDERS) {
        3
    } else if hq::within(type_word, TRANSPORTS) {
        2
    } else if hq::within(type_word, BATTLE_UNITS) {
        1
    } else {
        0
    }
}

/// The building page a building's Type opens: bunkers 7, towers 6, the plant 5, the institute
/// 4, any other 8.
pub fn building_page(type_word: u32) -> u8 {
    match type_word {
        t if hq::within(t, BUNKERS) => 7,
        0x8010_0000 | 0x8020_0000 => 6,
        0x8000_0010 => 5,
        0x8000_0400 => 4,
        _ => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_show_their_cursors() {
        assert_eq!([0, 1, 2, 3, 4, 7, 9, 12, 13, 17].map(cursor_state), [1, 3, 9, 4, 6, 2, 5, 3, 1, 1]);
        assert_eq!(cursor_object(3), Some(([192.0, 16.0], [8.0, 8.0])));
        assert_eq!(cursor_object(8), None);
    }

    #[test]
    fn the_cursors_ray_asks_for_the_landscape_and_the_buildings_only() {
        let classes: Vec<u32> = (0..16).filter(|c| CURSOR_RAY_CLASSES & (1 << c) != 0).collect();
        assert_eq!(classes, [1, 3]);
    }

    #[test]
    fn a_unit_opens_its_kinds_page_and_a_building_its_own() {
        assert_eq!([0x0100_4000, 0x0100_2000, 0x0100_8000, 0x0101_0000].map(unit_page), [3, 2, 1, 1]);
        assert_eq!(
            [0x8001_0000, 0x8020_0000, 0x8000_0010, 0x8000_0400, 0x8000_0004].map(building_page),
            [7, 6, 5, 4, 8]
        );
    }

    fn sphere(index: usize, centre: [f32; 3], radius: f32, building: bool) -> Sphere {
        Sphere { index, centre: Vec3::from(centre), radius, building }
    }

    #[test]
    fn a_building_is_picked_within_0_7_of_its_radius_and_a_unit_within_all_of_it() {
        let pick = |s: Sphere| nearest_on_ray(Vec3::ZERO, Vec3::X, 1000.0, [s].into_iter());
        // 8 off the ray: a building of radius 10 reaches 7, a unit of radius 10 all 10.
        assert_eq!(pick(sphere(1, [100.0, 8.0, 0.0], 10.0, true)), None);
        assert_eq!(pick(sphere(1, [100.0, 6.5, 0.0], 10.0, true)), Some(1));
        assert_eq!(pick(sphere(2, [100.0, 8.0, 0.0], 10.0, false)), Some(2));
        // Behind the eye, holding the eye in its whole sphere, or beyond the ray's length.
        assert_eq!(pick(sphere(2, [-100.0, 0.0, 0.0], 10.0, false)), None);
        assert_eq!(pick(sphere(1, [9.0, 0.0, 0.0], 10.0, true)), None);
        assert_eq!(pick(sphere(2, [1000.0, 0.0, 0.0], 10.0, false)), None);
    }

    #[test]
    fn the_centre_nearest_the_eye_wins_and_a_tie_keeps_the_building() {
        let pick = |s: &[Sphere]| nearest_on_ray(Vec3::ZERO, Vec3::X, 1000.0, s.iter().copied());
        // Nearer along the ray (100) but farther from the eye (√(100² + 30²) ≈ 104.4) than a
        // centre on the ray at 103: the straight-line distance decides.
        let off = sphere(1, [100.0, 30.0, 0.0], 40.0, false);
        let on = sphere(2, [103.0, 0.0, 0.0], 5.0, false);
        assert_eq!(pick(&[off, on]), Some(2));
        // The building walk comes first and a unit must be strictly nearer to replace it.
        let building = sphere(3, [50.0, 0.0, 0.0], 10.0, true);
        let tied = sphere(4, [50.0, 0.0, 0.0], 10.0, false);
        assert_eq!(pick(&[tied, building]), Some(3));
        let nearer = sphere(5, [40.0, 0.0, 0.0], 10.0, false);
        assert_eq!(pick(&[building, nearer]), Some(5));
    }

    #[test]
    fn a_ray_through_the_middle_of_the_window_runs_along_the_look() {
        let eye = Vec3::new(10.0, 20.0, 30.0);
        let look = Vec3::new(0.0, 1.0, -0.5).normalize();
        let view_proj =
            Mat4::perspective_infinite_reverse_rh(1.0, 1.5, 3.0) * Mat4::look_to_rh(eye, look, Vec3::Z);
        let Aim::Ray { direction, .. } = ray(view_proj, eye, [300.0, 200.0], [600.0, 400.0]) else {
            panic!()
        };
        assert!((direction - look).length() < 1e-4, "{direction}");
    }
}
