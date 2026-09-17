//! A building's places as `MBehaviour`'s place set ticks them (`Behavior.dll:0x10018ac0`): the
//! hall-way vertices with a place bit, each an upright cylinder about its vertex, whose
//! occupants are refreshed every 64–128 ms. Three acts ride on it: a dock charges, repairs and
//! rearms who stands in it, an in place puts a hero into the main teleport's chamber, and an
//! out place raises the teleport's clan's `Hero_Teleported`. See `docs/27-ownership.md`, "The
//! places", "What a dock gives" and "The main teleport".

use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, Vertex};
use parkan_formats::mission::{self, Mission};
use parkan_sim::damage::{Life, raise_life_share};

use crate::assembly::Assembly;
use crate::buildings::Child;
use crate::play::{BUILDING_GENERATOR, Play, ROBOT_HERO};

/// The main teleport's in and out places, and the vertex an in place puts a hero on, which is
/// no place (docs/27, "The places").
pub const PLACE_TELEPORT_IN: u32 = 0x8000;
pub const PLACE_TELEPORT_OUT: u32 = 0x4000;
pub const TELEPORT_LANDING: u32 = 0x1_0000;
/// A dock: it charges, repairs and rearms who stands in it (`0x10019251`, docs/27, "The
/// places"). The bunkers, towers and the large ruin carry `0x620`, the factories `0x600`, and a
/// generator's and the Outpost's outdoor ones ride with [`GROUND_LEVEL`].
pub const PLACE_DOCK: u32 = 0x20 | 0x200 | 0x400;
/// What a dock gives a second: a tenth of a full battery (`0x10019372`), of full life
/// (`0x10018100`) and of each gun's magazine and capacitor (`0x100181e0`). So a unit standing
/// in one is full in ten seconds, whatever it is (docs/27, "What a dock gives").
pub const DOCK_SHARE: f32 = 0.1;
/// The dock glow a building's load group places over each of its docks, with the charging
/// sound `f_recharge.wav` in it: `f_recharge_r`, and a storage's `f_recharge_b` (docs/13, "A
/// building's load group"). It hangs on the field's own `Rech_*` control points, whose
/// direction vectors size it: 15 up and 11 across over the Outpost's ground-level dock, 5.7
/// and 5.2 over a generator's indoor one.
pub const RECHARGE_EFFECT: &str = "f_recharge";
/// The time mode a charging dock runs its glow in: its own is 0, a value set from outside, so
/// it stands still until something drives it (docs/11, "Effect time t"). Looping, its 3 s
/// duration and its ping-pong flag sweep the timeline 0 → 1 → 0, which runs the two sprites
/// and the light and plays `f_recharge.wav` as the time crosses 0.1 rising.
///
/// STAND-IN: docs/13-control.md#a-buildings-load-group--read-and-measured -- what the game
/// drives a dock's glow with is not read; a charging dock runs it looping and stops it again.
pub const RECHARGE_TIME_MODE: u32 = parkan_formats::fxid::TIME_LOOP;
/// A vertex is a place with a bit of either mask (`0x100189a7`–`0x100189b3`).
pub const PLACE_BITS: u32 = 0xf8;
pub const PLACE_BITS_HIGH: u32 = 0xc600;
/// A ground-level place is 10 across and 12 high rather than 5 and 3 (`0x100184f0`).
pub const GROUND_LEVEL: u32 = 0x1000_0000;
pub const RADIUS: f32 = 5.0;
pub const HEIGHT: f32 = 3.0;
pub const GROUND_RADIUS: f32 = 10.0;
pub const GROUND_HEIGHT: f32 = 12.0;
/// How far below its vertex a place reaches: the whole height at a pod or an out place, this
/// share of it at any other (`0x1001837a`, the float at `0x10059788`).
pub const BELOW_SHARE: f32 = 0.7;
/// The fastest an occupant of a teleport place may move (`+0x2c`).
pub const TELEPORT_SPEED_BOUND: f32 = 1000.0;
/// The fastest an occupant of any other place may move: a unit must stand still to be in a
/// dock or on a pod (`0x100184f0`, docs/27, "A unit must stand still to be in a place").
pub const SPEED_BOUND: f32 = 2.0;
/// The bits that make a place a teleport's, whose occupants may be moving (`0x1001851f`).
pub const TELEPORT_BITS: u32 = 0x7_c000;
/// The set's and each place's timer: 64 ms and a random share of 64 more (`0x10003749`,
/// `0x1004c550`).
pub const TIMER_MS: f64 = 64.0;

/// Whether a hall-way vertex's flag word makes it a place.
pub fn is_place(flags: u32) -> bool {
    flags & (PLACE_BITS | PLACE_BITS_HIGH) != 0
}

/// How fast an occupant of a place with `flags` may move and still count (`0x100184f0`).
pub fn speed_bound(flags: u32) -> f32 {
    if flags & TELEPORT_BITS != 0 { TELEPORT_SPEED_BOUND } else { SPEED_BOUND }
}

/// Whether a place with `flags` about the world point `vertex` holds an object whose origin is
/// `origin` (`0x10018310`, `0x10022c80`): the origin's projection onto the vertical segment
/// from below the vertex to the height above it falls between the segment's ends, and the
/// origin lies within the radius of it.
pub fn holds(vertex: Vec3, flags: u32, origin: Vec3) -> bool {
    let (radius, height) =
        if flags & GROUND_LEVEL != 0 { (GROUND_RADIUS, GROUND_HEIGHT) } else { (RADIUS, HEIGHT) };
    let below =
        if flags & (hallway::PLACE_POD | PLACE_TELEPORT_OUT) != 0 { height } else { height * BELOW_SHARE };
    let dz = origin.z - vertex.z;
    (-below..=height).contains(&dz) && origin.truncate().distance(vertex.truncate()) <= radius
}

/// One place: its vertex, its occupants when last refreshed, and when it refreshes next. A
/// dock also keeps the glow standing over it and whether it was charging anyone last tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub index: usize,
    pub vertex: Vertex,
    pub occupants: Vec<Child>,
    pub due_ms: f64,
    /// The record ids of the load-group [`RECHARGE_EFFECT`]s that stand at this place.
    pub recharge: Vec<i32>,
    pub charging: bool,
}

/// One building's docks and teleport places.
///
/// STAND-IN: docs/27-ownership.md#the-places--read-and-measured -- only a dock and the main
/// teleport's in and out places are ticked here; a transport's loading and unloading places
/// keep the transport's own arrival.
#[derive(Clone, Debug, PartialEq)]
pub struct Places {
    pub target: usize,
    pub part: usize,
    pub places: Vec<Place>,
    /// The first vertex with `0x10000`, where an in place puts a hero.
    pub landing: Option<Vertex>,
    pub due_ms: f64,
    /// When the set last ticked, which is the game time a dock's give is measured over; none
    /// before its first tick.
    last_ms: Option<f64>,
    seed: u32,
}

impl Places {
    /// Mission object `object`, target `target`, if its root part's hall way has a dock or a
    /// teleport place.
    pub fn load(assembly: &mut Assembly, mission: &Mission, object: usize, target: usize) -> Option<Places> {
        let placed = mission.objects.get(object).filter(|o| o.kind == mission::KIND_BUILDING)?;
        let parts = assembly.parts(placed.kind, &placed.path);
        let (part, root) = parts.iter().enumerate().find(|(_, p)| p.host == -1)?;
        let blob =
            assembly.archive(&root.reference.library)?.read_name(&root.reference.member).ok()?.to_vec();
        let hall_way = hallway::parse(&blob, &root.reference.member).ok()?;
        Self::of(&hall_way, target, part)
    }

    /// The docks and teleport places of `hall_way`, on part `part` of target `target`.
    pub fn of(hall_way: &HallWay, target: usize, part: usize) -> Option<Places> {
        let acts = PLACE_DOCK | PLACE_TELEPORT_IN | PLACE_TELEPORT_OUT;
        let places: Vec<Place> = hall_way
            .vertices
            .iter()
            .enumerate()
            .filter(|(_, v)| is_place(v.flags) && v.flags & acts != 0)
            .map(|(index, &vertex)| Place {
                index,
                vertex,
                occupants: Vec::new(),
                due_ms: 0.0,
                recharge: Vec::new(),
                charging: false,
            })
            .collect();
        if places.is_empty() {
            return None;
        }
        let landing =
            hall_way.vertices.iter().find(|v| v.flags & TELEPORT_LANDING == TELEPORT_LANDING).copied();
        Some(Places {
            target,
            part,
            places,
            landing,
            due_ms: 0.0,
            last_ms: None,
            seed: 0x9e37_79b9 ^ target as u32,
        })
    }

    /// The game time since the set last ticked, seconds, and `now` kept as its last.
    fn since(&mut self, now: f64) -> f32 {
        let dt = self.last_ms.map_or(0.0, |last| (now - last).max(0.0));
        self.last_ms = Some(now);
        (dt / 1000.0) as f32
    }

    /// The next timer's length: 64 ms and a share of 64.
    ///
    /// STAND-IN: docs/27-ownership.md#who-stands-in-a-place--read -- the game's generator
    /// (`0x1004c550`) is not followed; a xorshift of its own gives the share.
    fn period(&mut self) -> f64 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        TIMER_MS + TIMER_MS * f64::from(x % 1024) / 1024.0
    }
}

impl Play {
    /// Every building's teleport places, loaded once the play is.
    pub(crate) fn load_places(&mut self, mission: &Mission) {
        self.places = (0..self.battle.combat.targets.len())
            .filter_map(|t| Places::load(&mut self.assembly, mission, self.battle.objects[t], t))
            .collect();
        self.join_dock_glows();
    }

    /// Each dock's own glow: a [`RECHARGE_EFFECT`] of the building's load group belongs to the
    /// dock whose vertex is nearest it (docs/13, "A building's load group"). Every one of them
    /// stands within 4 of a dock and tens of metres from any other, and a dock with none — a
    /// generator's second ground-level one — simply keeps quiet.
    pub(crate) fn join_dock_glows(&mut self) {
        for set in &mut self.places {
            for place in &mut set.places {
                place.recharge.clear();
            }
            let Some((b, _)) = self.building_effects.iter().find(|(b, _)| b.target == set.target) else {
                continue;
            };
            let Some(target) = self.battle.combat.targets.get(set.target) else { continue };
            let (Some(own), Some(theirs)) = (target.parts.get(set.part), target.parts.get(b.part)) else {
                continue;
            };
            let docks: Vec<(usize, Vec3)> = set
                .places
                .iter()
                .enumerate()
                .filter(|(_, p)| p.vertex.flags & PLACE_DOCK != 0)
                .filter_map(|(k, p)| Some((k, crate::factory::vertex_world(&p.vertex, own)?)))
                .collect();
            for e in b.effects.iter().filter(|e| e.name.starts_with(RECHARGE_EFFECT)) {
                let Some(frame) = b.frame(theirs, e.on) else { continue };
                let nearest = docks
                    .iter()
                    .min_by(|(_, a), (_, c)| a.distance(frame.origin).total_cmp(&c.distance(frame.origin)));
                if let Some(&(k, _)) = nearest {
                    set.places[k].recharge.push(e.id);
                }
            }
        }
    }

    /// The objects a place may hold: the hero on foot and every live unit, each with its
    /// origin and speed.
    fn place_candidates(&self) -> Vec<(Child, Vec3, f32)> {
        let mut out = Vec::new();
        let aboard = self.driving.as_ref().is_some_and(|d| !d.telepresence);
        if !self.hero.dead() && !aboard {
            let body = &self.hero.walker.body;
            out.push((Child::Hero, body.position, Vec3::from_array(body.velocity).length()));
        }
        for (t, robot) in &self.robots {
            if self.battle.combat.targets.get(*t).is_some_and(|x| x.alive) {
                let body = &robot.walker.body;
                out.push((Child::Robot(*t), body.position, Vec3::from_array(body.velocity).length()));
            }
        }
        out
    }

    /// Whether an occupant's Type is a hero's (`0x10018d2a`), which both of a teleport's acts
    /// ask for; the hero itself always is.
    fn hero_type(&self, occupant: Child) -> bool {
        match occupant {
            Child::Hero => true,
            Child::Robot(r) => self.units.get(r).is_some_and(|u| u.type_word == ROBOT_HERO),
        }
    }

    /// The place set's tick for every building (`Behavior.dll:0x10018ac0`), at `now`.
    pub(crate) fn tick_places(&mut self, now: f64) {
        for i in 0..self.places.len() {
            if now < self.places[i].due_ms {
                continue;
            }
            let period = self.places[i].period();
            self.places[i].due_ms = now + period;
            let dt = self.places[i].since(now);
            let t = self.places[i].target;
            let Some(part) = self.battle.combat.targets.get(t).and_then(|x| x.parts.get(self.places[i].part))
            else {
                continue;
            };
            let points: Vec<Option<Vec3>> =
                self.places[i].places.iter().map(|p| crate::factory::vertex_world(&p.vertex, part)).collect();
            let landing = self.places[i].landing.and_then(|v| crate::factory::vertex_world(&v, part));
            let candidates = self.place_candidates();
            for (k, point) in points.into_iter().enumerate() {
                let Some(point) = point else { continue };
                if now >= self.places[i].places[k].due_ms {
                    let period = self.places[i].period();
                    let place = &mut self.places[i].places[k];
                    place.due_ms = now + period;
                    let bound = speed_bound(place.vertex.flags);
                    place.occupants = candidates
                        .iter()
                        .filter(|(_, origin, speed)| {
                            *speed <= bound && holds(point, place.vertex.flags, *origin)
                        })
                        .map(|(c, _, _)| *c)
                        .collect();
                }
                let flags = self.places[i].places[k].vertex.flags;
                let mut charging = false;
                for occupant in self.places[i].places[k].occupants.clone() {
                    if flags & PLACE_DOCK != 0 {
                        charging |= self.dock(t, occupant, dt);
                    }
                    if flags & PLACE_TELEPORT_IN != 0 && self.teleport_in(t, occupant, landing) {
                        self.places[i].places[k].due_ms = now;
                    }
                    if flags & PLACE_TELEPORT_OUT != 0 {
                        self.teleport_out(t, occupant);
                    }
                }
                if flags & PLACE_DOCK != 0 {
                    self.glow(i, k, charging, now);
                }
            }
        }
    }

    /// The clan of a place's occupant, and whether it is still there to act on.
    fn occupant_clan(&self, occupant: Child) -> Option<Option<i64>> {
        match occupant {
            Child::Hero => (!self.hero.dead()).then_some(Some(self.player_clan)),
            Child::Robot(r) => self
                .battle
                .combat
                .targets
                .get(r)
                .filter(|x| x.alive)
                .map(|_| self.units.get(r).and_then(|u| u.clan)),
        }
    }

    /// A dock's glow as it starts and stops charging: the [`RECHARGE_EFFECT`]s standing over
    /// that place run in [`RECHARGE_TIME_MODE`] while it charges — their sprites and light
    /// over the field, `f_recharge.wav` once every sweep — and are switched off again when it
    /// stops, which silences them. Only a change is acted on, so the sound is not struck again
    /// every 64–128 ms.
    fn glow(&mut self, i: usize, k: usize, charging: bool, now: f64) {
        if self.places[i].places[k].charging == charging {
            return;
        }
        self.places[i].places[k].charging = charging;
        let t = self.places[i].target;
        for id in self.places[i].places[k].recharge.clone() {
            let owner = crate::fx::Owner::Building(t, id);
            if charging {
                self.fx.restart(owner, now, Some(RECHARGE_TIME_MODE));
            }
            self.fx.switch(owner, charging);
        }
    }

    /// A dock (`0x10019251`): every unit standing in it that belongs to the building's clan or
    /// an ally (`0x10019318`) gains, over `dt` seconds, [`DOCK_SHARE`] a second of a full
    /// battery (`0x10019372`), of its full life, destroyed parts restored with it, and of its
    /// fight shield's mean sector fill (`0x10018100`), and of each of its guns' magazine and
    /// capacitor (`0x100181e0`). See docs/27, "What a dock gives". Returns whether it gave this
    /// occupant anything, which runs the dock's glow.
    ///
    /// STAND-IN: docs/27-ownership.md#what-a-dock-gives--read -- a part still in the air when its
    /// node is put back is taken out of the air rather than drawn beside it.
    fn dock(&mut self, t: usize, occupant: Child, dt: f32) -> bool {
        let Some(Some(clan)) = self.occupant_clan(occupant) else { return false };
        if !self.battle.combat.targets.get(t).is_some_and(|x| x.alive) {
            return false;
        }
        let owner = self.units.get(t).and_then(|u| u.clan);
        if owner != Some(clan) && !self.allied_to(owner, Some(clan)) {
            return false;
        }
        let share = DOCK_SHARE * dt;
        if share <= 0.0 {
            return false;
        }
        match occupant {
            Child::Hero => {
                let mut lives: Vec<&mut Life> = self.hero.lives.iter_mut().flatten().collect();
                raise_life_share(&mut lives, share);
                if let Some(shield) = self.battle.combat.hero.as_mut().and_then(|h| h.shield.as_mut()) {
                    shield.dock(share);
                }
                if let Some(power) = self.hero.robot.power.as_mut() {
                    power.set_fill(power.fill() + share);
                }
                for g in &mut self.hero.robot.guns {
                    g.rearm(share);
                }
            }
            Child::Robot(r) => {
                if let Some(target) = self.battle.combat.targets.get_mut(r) {
                    let mut lives: Vec<&mut Life> =
                        target.parts.iter_mut().filter_map(|p| p.life.as_mut()).collect();
                    raise_life_share(&mut lives, share);
                    if let Some(shield) = target.shield.as_mut() {
                        shield.dock(share);
                    }
                }
                self.flights.retain(|f| f.target != r);
                if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == r) {
                    for g in &mut robot.guns {
                        g.rearm(share);
                    }
                    if let Some(power) = robot.power.as_mut() {
                        power.set_fill(power.fill() + share);
                    }
                }
            }
        }
        true
    }

    /// An in place (`0x10018c84`–`0x10019132`): a hero of the teleport's owner, while that owner
    /// holds every generator on the map and at least one, is put on the landing vertex, its
    /// rotation and speed kept, nothing played. True when it was moved.
    ///
    /// The game also asks that property `0x208`, a network mirror's flag by its neighbours, be
    /// 0; in single play it is.
    fn teleport_in(&mut self, t: usize, occupant: Child, landing: Option<Vec3>) -> bool {
        let Some(Some(clan)) = self.occupant_clan(occupant).filter(|_| self.hero_type(occupant)) else {
            return false;
        };
        let teleport_alive = self.battle.combat.targets.get(t).is_some_and(|x| x.alive);
        if !teleport_alive || self.units.get(t).and_then(|u| u.clan) != Some(clan) {
            return false;
        }
        // STAND-IN: docs/27-ownership.md#teleport-in-0x8000--read -- whether a destroyed
        // generator stays in `World3D.dll`'s queue 3 is not read; the live ones are asked.
        let generators: Vec<Option<i64>> = self
            .units
            .iter()
            .zip(&self.battle.combat.targets)
            .filter(|(u, x)| u.type_word == BUILDING_GENERATOR && x.alive)
            .map(|(u, _)| u.clan)
            .collect();
        if generators.is_empty() || generators.iter().any(|c| *c != Some(clan)) {
            return false;
        }
        let Some(at) = landing else { return false };
        match occupant {
            Child::Hero => {
                let w = &mut self.hero.walker;
                w.body.position = at;
                w.from = (at, w.body.yaw);
                w.ground = None;
            }
            Child::Robot(r) => {
                if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == r) {
                    let w = &mut robot.walker;
                    w.body.position = at;
                    w.from = (at, w.body.yaw);
                    w.ground = None;
                }
            }
        }
        true
    }

    /// An out place (`0x10019158`–`0x1001923e`): a hero, whatever its clan, raises its
    /// teleport's clan's `Hero_Teleported` on every tick it stays (`0x1000c7d0`).
    fn teleport_out(&mut self, t: usize, occupant: Child) {
        if !self.hero_type(occupant) || !matches!(self.occupant_clan(occupant), Some(Some(_))) {
            return;
        }
        let Some(clan) = self.units.get(t).and_then(|u| u.clan) else { return };
        let Some(p) = self.progression.as_mut() else { return };
        let notices = p.hero_teleported(clan);
        for n in &notices {
            let says = p.say(n);
            self.says.extend(says);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(flags: u32) -> Vertex {
        Vertex { position: [0.0; 3], flags, joint: 0 }
    }

    #[test]
    fn a_place_is_an_upright_cylinder_deeper_below_a_pod_or_an_out_place() {
        let v = Vec3::new(10.0, 20.0, 5.0);
        assert!(holds(v, PLACE_TELEPORT_IN, Vec3::new(14.9, 20.0, 5.0)));
        assert!(!holds(v, PLACE_TELEPORT_IN, Vec3::new(15.1, 20.0, 5.0)), "past the radius");
        assert!(holds(v, PLACE_TELEPORT_IN, Vec3::new(10.0, 20.0, 8.0)));
        assert!(!holds(v, PLACE_TELEPORT_IN, Vec3::new(10.0, 20.0, 8.1)), "above the height");
        assert!(holds(v, PLACE_TELEPORT_IN, Vec3::new(10.0, 20.0, 2.9)), "2.1 below an in place");
        assert!(!holds(v, PLACE_TELEPORT_IN, Vec3::new(10.0, 20.0, 2.8)));
        assert!(holds(v, PLACE_TELEPORT_OUT, Vec3::new(10.0, 20.0, 2.0)), "3 below an out place");
        assert!(!holds(v, PLACE_TELEPORT_OUT, Vec3::new(10.0, 20.0, 1.9)));
        let ground = PLACE_TELEPORT_IN | GROUND_LEVEL;
        assert!(holds(v, ground, Vec3::new(19.9, 20.0, 17.0)), "10 across and 12 up at ground level");
        assert!(!holds(v, ground, Vec3::new(10.0, 20.0, -3.5)), "8.4 below");
    }

    #[test]
    fn a_dock_and_a_teleport_place_are_kept_and_the_landing_needs_its_bit() {
        let hall = HallWay {
            vertices: vec![
                vertex(hallway::PLACE_POD),
                vertex(PLACE_TELEPORT_IN),
                vertex(TELEPORT_LANDING),
                vertex(PLACE_TELEPORT_OUT),
                vertex(0),
                // The Outpost's ground-level dock, and a bunker's indoor one.
                vertex(0x1000_0620),
                vertex(0x620),
            ],
            links: Vec::new(),
        };
        let p = Places::of(&hall, 7, 0).expect("a dock and the teleport places");
        assert_eq!(p.places.iter().map(|x| x.index).collect::<Vec<_>>(), [1, 3, 5, 6]);
        assert_eq!(p.landing.map(|v| v.flags), Some(TELEPORT_LANDING));
        assert!(!is_place(TELEPORT_LANDING), "the landing is no place");
        let pods = HallWay { vertices: vec![vertex(hallway::PLACE_POD)], links: Vec::new() };
        assert!(Places::of(&pods, 0, 0).is_none());
    }

    #[test]
    fn only_a_teleports_place_holds_an_occupant_that_is_moving() {
        assert_eq!(speed_bound(PLACE_DOCK), SPEED_BOUND);
        assert_eq!(speed_bound(hallway::PLACE_POD), SPEED_BOUND);
        assert_eq!(speed_bound(PLACE_TELEPORT_IN), TELEPORT_SPEED_BOUND);
        assert_eq!(speed_bound(PLACE_TELEPORT_OUT), TELEPORT_SPEED_BOUND);
        assert_eq!(speed_bound(0x1000_0620), SPEED_BOUND, "a ground-level dock stands still too");
    }

    #[test]
    fn the_set_measures_a_docks_give_over_the_time_since_it_last_ticked() {
        let mut p =
            Places::of(&HallWay { vertices: vec![vertex(0x620)], links: Vec::new() }, 3, 0).expect("a dock");
        assert_eq!(p.since(1000.0), 0.0, "nothing before its first tick");
        assert_eq!(p.since(1100.0), 0.1);
        assert_eq!(p.since(1164.0), 0.064);
    }

    #[test]
    fn the_timer_runs_64_to_128_ms() {
        let mut p =
            Places::of(&HallWay { vertices: vec![vertex(PLACE_TELEPORT_IN)], links: Vec::new() }, 3, 0)
                .expect("a place");
        for _ in 0..100 {
            let ms = p.period();
            assert!((TIMER_MS..TIMER_MS * 2.0).contains(&ms), "{ms}");
        }
    }
}
