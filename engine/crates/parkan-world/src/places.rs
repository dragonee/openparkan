//! A building's places as `MBehaviour`'s place set ticks them (`Behavior.dll:0x10018ac0`): the
//! hall-way vertices with a place bit, each an upright cylinder about its vertex, whose
//! occupants are refreshed every 64–128 ms. The Main Teleport's two acts ride on it: an in
//! place puts a hero into the chamber, and an out place raises the teleport's clan's
//! `Hero_Teleported`. See `docs/27-ownership.md`, "The main teleport".

use glam::Vec3;
use parkan_formats::hallway::{self, HallWay, Vertex};
use parkan_formats::mission::{self, Mission};

use crate::assembly::Assembly;
use crate::buildings::Child;
use crate::play::{BUILDING_GENERATOR, Play, ROBOT_HERO};

/// The main teleport's in and out places, and the vertex an in place puts a hero on, which is
/// no place (docs/27, "The places").
pub const PLACE_TELEPORT_IN: u32 = 0x8000;
pub const PLACE_TELEPORT_OUT: u32 = 0x4000;
pub const TELEPORT_LANDING: u32 = 0x1_0000;
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
/// The set's and each place's timer: 64 ms and a random share of 64 more (`0x10003749`,
/// `0x1004c550`).
pub const TIMER_MS: f64 = 64.0;

/// Whether a hall-way vertex's flag word makes it a place.
pub fn is_place(flags: u32) -> bool {
    flags & (PLACE_BITS | PLACE_BITS_HIGH) != 0
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

/// One place: its vertex, its occupants when last refreshed, and when it refreshes next.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub index: usize,
    pub vertex: Vertex,
    pub occupants: Vec<Child>,
    pub due_ms: f64,
}

/// One building's teleport places.
///
/// STAND-IN: docs/27-ownership.md#the-places--read-and-measured -- only the main teleport's
/// in and out places are ticked here; a dock's charge, repair and rearm are not modelled, and a
/// transport's loading and unloading places keep the transport's own arrival.
#[derive(Clone, Debug, PartialEq)]
pub struct Places {
    pub target: usize,
    pub part: usize,
    pub places: Vec<Place>,
    /// The first vertex with `0x10000`, where an in place puts a hero.
    pub landing: Option<Vertex>,
    pub due_ms: f64,
    seed: u32,
}

impl Places {
    /// Mission object `object`, target `target`, if its root part's hall way has a teleport
    /// place.
    pub fn load(assembly: &mut Assembly, mission: &Mission, object: usize, target: usize) -> Option<Places> {
        let placed = mission.objects.get(object).filter(|o| o.kind == mission::KIND_BUILDING)?;
        let parts = assembly.parts(placed.kind, &placed.path);
        let (part, root) = parts.iter().enumerate().find(|(_, p)| p.host == -1)?;
        let blob =
            assembly.archive(&root.reference.library)?.read_name(&root.reference.member).ok()?.to_vec();
        let hall_way = hallway::parse(&blob, &root.reference.member).ok()?;
        Self::of(&hall_way, target, part)
    }

    /// The teleport places of `hall_way`, on part `part` of target `target`.
    pub fn of(hall_way: &HallWay, target: usize, part: usize) -> Option<Places> {
        let places: Vec<Place> = hall_way
            .vertices
            .iter()
            .enumerate()
            .filter(|(_, v)| is_place(v.flags) && v.flags & (PLACE_TELEPORT_IN | PLACE_TELEPORT_OUT) != 0)
            .map(|(index, &vertex)| Place { index, vertex, occupants: Vec::new(), due_ms: 0.0 })
            .collect();
        if places.is_empty() {
            return None;
        }
        let landing =
            hall_way.vertices.iter().find(|v| v.flags & TELEPORT_LANDING == TELEPORT_LANDING).copied();
        Some(Places { target, part, places, landing, due_ms: 0.0, seed: 0x9e37_79b9 ^ target as u32 })
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
    }

    /// The objects a place may hold: the hero on foot, and any unit of the hero's Type, each
    /// with its origin and speed.
    fn place_candidates(&self) -> Vec<(Child, Vec3, f32)> {
        let mut out = Vec::new();
        let aboard = self.driving.as_ref().is_some_and(|d| !d.telepresence);
        if !self.hero.dead() && !aboard {
            let body = &self.hero.walker.body;
            out.push((Child::Hero, body.position, Vec3::from_array(body.velocity).length()));
        }
        for (t, robot) in &self.robots {
            if self.units.get(*t).is_some_and(|u| u.type_word == ROBOT_HERO)
                && self.battle.combat.targets.get(*t).is_some_and(|x| x.alive)
            {
                let body = &robot.walker.body;
                out.push((Child::Robot(*t), body.position, Vec3::from_array(body.velocity).length()));
            }
        }
        out
    }

    /// The place set's tick for every building (`Behavior.dll:0x10018ac0`), at `now`.
    pub(crate) fn tick_places(&mut self, now: f64) {
        for i in 0..self.places.len() {
            if now < self.places[i].due_ms {
                continue;
            }
            let period = self.places[i].period();
            self.places[i].due_ms = now + period;
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
                    place.occupants = candidates
                        .iter()
                        .filter(|(_, origin, speed)| {
                            *speed <= TELEPORT_SPEED_BOUND && holds(point, place.vertex.flags, *origin)
                        })
                        .map(|(c, _, _)| *c)
                        .collect();
                }
                let flags = self.places[i].places[k].vertex.flags;
                for occupant in self.places[i].places[k].occupants.clone() {
                    if flags & PLACE_TELEPORT_IN != 0 && self.teleport_in(t, occupant, landing) {
                        self.places[i].places[k].due_ms = now;
                    }
                    if flags & PLACE_TELEPORT_OUT != 0 {
                        self.teleport_out(t, occupant);
                    }
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

    /// An in place (`0x10018c84`–`0x10019132`): a hero of the teleport's owner, while that owner
    /// holds every generator on the map and at least one, is put on the landing vertex, its
    /// rotation and speed kept, nothing played. True when it was moved.
    ///
    /// The game also asks that property `0x208`, a network mirror's flag by its neighbours, be
    /// 0; in single play it is.
    fn teleport_in(&mut self, t: usize, occupant: Child, landing: Option<Vec3>) -> bool {
        let Some(Some(clan)) = self.occupant_clan(occupant) else { return false };
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
        if !matches!(self.occupant_clan(occupant), Some(Some(_))) {
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
    fn only_a_teleport_place_is_kept_and_the_landing_needs_its_bit() {
        let hall = HallWay {
            vertices: vec![
                vertex(hallway::PLACE_POD),
                vertex(PLACE_TELEPORT_IN),
                vertex(TELEPORT_LANDING),
                vertex(PLACE_TELEPORT_OUT),
                vertex(0),
            ],
            links: Vec::new(),
        };
        let p = Places::of(&hall, 7, 0).expect("teleport places");
        assert_eq!(p.places.iter().map(|x| x.index).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(p.landing.map(|v| v.flags), Some(TELEPORT_LANDING));
        assert!(!is_place(TELEPORT_LANDING), "the landing is no place");
        let pods = HallWay { vertices: vec![vertex(hallway::PLACE_POD)], links: Vec::new() };
        assert!(Places::of(&pods, 0, 0).is_none());
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
