//! What a unit does with its orders: the task each order starts, the engagement that
//! pulls a unit into a fight between orders, the fire control that picks what its guns
//! point at, and when an AI gun fires. See `docs/31-packages.md`, "What each package
//! does", "Between orders", "The fire control" and "The attack, tick by tick", and
//! `docs/29-weapons.md`, "How the AI fires".

use glam::Vec3;

use crate::orders::{self, Order, Target};

/// Follow me's radius: 20, unless the order's parameter lies strictly between 20 and 30
/// (`Behavior.dll:0x1002adab`).
pub const FOLLOW_RADIUS: f32 = 20.0;
/// A follower picks a new place once the leader is this much past the radius across the
/// ground, or this much past it above or below (`0x1002aed7`).
pub const FOLLOW_SLACK: f32 = 20.0;
pub const FOLLOW_SLACK_UP: f32 = 10.0;
/// It tries this many random spots in the square ±radius about the leader, this far above
/// the leader (`0x1002b067`).
pub const FOLLOW_TRIES: usize = 77;
pub const FOLLOW_LIFT: f32 = 5.0;
/// The engagement: a contact scores nothing beyond this (`0x10001010`), and a task whose
/// interrupt priority is below the bar is not pulled off (`0x100179c0`).
pub const ENGAGE_RANGE: f32 = 500.0;
pub const ENGAGE_BAR: f32 = 0.3;
/// Seek and destroy looks this far for an enemy (`0x100306f0`).
pub const SEEK_RANGE: f32 = 3000.0;
/// The warrior, builder and transport types a search hunts (`0x100e000`).
pub const HUNTED_TYPES: u32 = 0x0100_e000;
/// A search's rescans, (fixed, random) ms: capture, and the other modes (`0x100301b2`).
pub const CAPTURE_RESCAN_MS: (f64, f64) = (3000.0, 3000.0);
pub const SEARCH_RESCAN_MS: (f64, f64) = (15_000.0, 15_000.0);
/// Roaming tries this many points at least this far inside the map (`0x10030ed2`).
pub const ROAM_TRIES: usize = 150;
pub const ROAM_INSET: f32 = 100.0;
/// A capture skips bridges and main teleports (`0x10030859`).
pub const BRIDGE: u32 = 0x8000_1000;
pub const MAIN_TELEPORT: u32 = 0x8000_0200;
/// The attack (`0x10027580`): the band short of the target, the reach aside, the distance
/// under which it fights, its speeds while fighting, and its timers (fixed, random) ms.
pub const ATTACK_SHORT: (f32, f32) = (50.0, 50.0);
pub const ATTACK_ASIDE: f32 = 80.0;
pub const ATTACK_MAX_FIRE_DISTANCE: f32 = 200.0;
pub const ATTACK_SPEED: (f32, f32) = (0.7, 0.3);
pub const ATTACK_TIMER_MS: (f64, f64) = (8000.0, 8000.0);
/// A building target's points lie this far inside its sphere (`0x10027580`).
pub const ATTACK_BUILDING_INSIDE: (f32, f32) = (20.0, 5.0);
/// The attack's tries at a point.
pub const ATTACK_TRIES: usize = 77;
/// The fight module's bars (`0x10024e7a`): a flyer's, and a walker's.
pub const FIRE_BAR_FLYER: f32 = 0.45;
pub const FIRE_BAR_WALKER: f32 = 0.85;

/// What the behaviour sees of another object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub id: i32,
    pub position: Vec3,
    /// Its sphere's radius.
    pub radius: f32,
    pub type_word: u32,
    pub building: bool,
    /// Of the unit's own clan, and hostile to it (`0x1000d460`).
    pub own: bool,
    pub hostile: bool,
}

/// What a takt has to go on.
#[derive(Clone, Copy, Debug)]
pub struct Senses<'a> {
    pub now_ms: f64,
    pub position: Vec3,
    /// Every live object but the unit, the clan's areal map being whole from its first
    /// tick (docs/31, "Where a search looks").
    pub seen: &'a [Seen],
    /// The map's extent in x and y.
    pub bounds: ([f32; 2], [f32; 2]),
    pub has_weapon: bool,
    /// Whether the walker has nothing left to follow.
    pub walker_idle: bool,
    /// A neutral clan's unit never engages on its own.
    pub neutral: bool,
}

impl Senses<'_> {
    fn find(&self, id: i32) -> Option<&Seen> {
        self.seen.iter().find(|s| s.id == id)
    }

    fn nearest(&self, pick: impl Fn(&Seen) -> bool, within: f32) -> Option<&Seen> {
        self.seen
            .iter()
            .filter(|s| pick(s) && s.position.distance(self.position) <= within)
            .min_by(|a, b| a.position.distance(self.position).total_cmp(&b.position.distance(self.position)))
    }
}

/// What a search looks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Search {
    /// Seek and destroy: the nearest hostile warrior, builder or transport.
    Enemies,
    /// Search and capture: the nearest building of the mask not the unit's own.
    Capture(u32),
    /// Capture building: one building, by logic id; done once it is the unit's clan's.
    Building(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
    /// No order: the unit stands, and engages on its own.
    Stop,
    /// Standby: clears the walker every takt, and is pulled into nothing.
    StayGround,
    Follow {
        leader: i32,
        radius: f32,
        next_ms: f64,
    },
    Search {
        search: Search,
        next_ms: f64,
    },
    /// Refit: a trip to a dock.
    Reload,
    Attack {
        target: Option<i32>,
        fighting: bool,
        next_ms: f64,
    },
}

impl Task {
    /// The interrupt priority for an engagement, reason 0 (docs/31, "Between orders").
    ///
    /// STAND-IN: docs/31-packages.md#between-orders--read -- follow's and refit's
    /// priorities for an engagement are not read: 0, like the other tasks that move.
    fn engage_priority(&self) -> f32 {
        match self {
            Task::Stop => 1.0,
            Task::Search { search: Search::Enemies, .. } => 1.0,
            Task::Attack { .. } => 1.0,
            _ => 0.0,
        }
    }

    /// The task an order builds (the dispatcher's `INSERT_ORDER_REPLACE`).
    pub fn from_order(order: &Order) -> Task {
        match (order.code, order.target) {
            (orders::STAYGROUND, _) => Task::StayGround,
            (orders::FOLLOW, Target::LogicId(leader)) => {
                let p = order.parameter as f32;
                let radius = if p > 20.0 && p < 30.0 { p } else { FOLLOW_RADIUS };
                Task::Follow { leader, radius, next_ms: 0.0 }
            }
            (orders::SEARCH, Target::TypeMask(mask)) => {
                Task::Search { search: Search::Capture(mask), next_ms: 0.0 }
            }
            (orders::SEARCH, Target::LogicId(id)) => {
                Task::Search { search: Search::Building(id), next_ms: 0.0 }
            }
            (orders::SEARCH, _) => Task::Search { search: Search::Enemies, next_ms: 0.0 },
            (orders::RELOAD, _) => Task::Reload,
            (orders::ATTACK, Target::LogicId(id)) => {
                Task::Attack { target: Some(id), fighting: false, next_ms: 0.0 }
            }
            _ => Task::Stop,
        }
    }
}

/// What the fire control points the guns at (`Behavior.dll:0x10023ff0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    None,
    /// Mode 1: the object it was given.
    Fixed(i32),
    /// Mode 2: the hostile contact nearest the unit, within 500.
    Nearest,
}

/// What the walker is told.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Walk {
    Keep,
    /// Its queues emptied: the machine holds.
    Clear,
    /// To a place, at a share of the unit's speed.
    To(Vec3, f32),
}

/// What a takt asks of the unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Takt {
    pub walk: Walk,
    /// The fire control's target, by logic id.
    pub target: Option<i32>,
    /// A go, attack or search task: every gun fires on its timer whatever its aim
    /// (`0x10024e58`).
    pub fire_freely: bool,
}

/// A unit's tasks, the last the one running, and its random source.
#[derive(Clone, Debug, PartialEq)]
pub struct Behaviour {
    pub tasks: Vec<Task>,
    pub fire: FireMode,
    seed: u32,
}

impl Behaviour {
    pub fn new(seed: u32) -> Self {
        Self { tasks: vec![Task::Stop], fire: FireMode::Nearest, seed: seed | 1 }
    }

    /// STAND-IN: docs/31-packages.md#what-each-package-does--read -- the behaviour's random
    /// source is not read: a 32-bit xorshift. 0..1.
    pub fn random(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    fn timer(&mut self, now_ms: f64, (fixed, random): (f64, f64)) -> f64 {
        now_ms + fixed + random * f64::from(self.random())
    }

    /// An order replaces every task.
    pub fn order(&mut self, order: &Order) {
        self.tasks = vec![Task::from_order(order)];
    }

    /// The task running.
    pub fn task(&self) -> Task {
        self.tasks.last().copied().unwrap_or(Task::Stop)
    }

    /// One behaviour takt: the engagement, then the running task's takt; a task that ends
    /// or fails gives way to the one beneath it, and with none left the unit stops.
    pub fn takt(&mut self, senses: &Senses) -> Takt {
        // Engaging (`0x10017e70`): the best hostile contact within 500 becomes an attack on
        // top, unless the clan is neutral or the task holds the unit below the bar.
        //
        // STAND-IN: docs/31-packages.md#between-orders--read -- how the radar module's
        // contacts are scored through the task is not read: the nearest hostile unit within
        // 500 is the best. An attack already running is not given another.
        let task = self.task();
        if !senses.neutral
            && senses.has_weapon
            && task.engage_priority() >= ENGAGE_BAR
            && !matches!(task, Task::Attack { .. })
            && let Some(enemy) = senses.nearest(|s| s.hostile && !s.building, ENGAGE_RANGE)
        {
            self.tasks.push(Task::Attack { target: Some(enemy.id), fighting: false, next_ms: 0.0 });
        }
        for _ in 0..4 {
            match self.run(senses) {
                Some(takt) => return takt,
                None => {
                    self.tasks.pop();
                    if self.tasks.is_empty() {
                        self.tasks.push(Task::Stop);
                    }
                }
            }
        }
        Takt { walk: Walk::Clear, target: self.fire_target(senses), fire_freely: false }
    }

    /// The fire control's target (`0x100240a6`).
    fn fire_target(&self, senses: &Senses) -> Option<i32> {
        match self.fire {
            FireMode::None => None,
            FireMode::Fixed(id) => senses.find(id).map(|s| s.id),
            FireMode::Nearest => senses.nearest(|s| s.hostile, ENGAGE_RANGE).map(|s| s.id),
        }
    }

    /// The running task's takt; `None` when it ended or failed.
    fn run(&mut self, senses: &Senses) -> Option<Takt> {
        let now = senses.now_ms;
        let at = senses.position;
        let at_rest = |fire: FireMode, walk: Walk, me: &mut Self| {
            me.fire = fire;
            Some(Takt { walk, target: me.fire_target(senses), fire_freely: false })
        };
        match self.task() {
            Task::Stop => at_rest(FireMode::Nearest, Walk::Keep, self),
            Task::StayGround => at_rest(FireMode::Nearest, Walk::Clear, self),
            // STAND-IN: docs/31-packages.md#what-each-package-does--read -- the refit's dock
            // pick (`0x10023b60`) is not read, and no dock is modelled: a refit fails at its
            // start, as it does on a map without one.
            Task::Reload => None,
            Task::Follow { leader, radius, next_ms } => {
                let lead = *senses.find(leader).filter(|s| s.own)?;
                let mut walk = Walk::Keep;
                if senses.walker_idle || now >= next_ms {
                    let across = lead.position.truncate().distance(at.truncate());
                    let up = (lead.position.z - at.z).abs();
                    if across > radius + FOLLOW_SLACK || up > radius + FOLLOW_SLACK_UP {
                        let (dx, dy) = (self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0);
                        let spot = lead.position + Vec3::new(dx * radius, dy * radius, FOLLOW_LIFT);
                        walk = Walk::To(spot, 1.0);
                    }
                    // STAND-IN: docs/31-packages.md#what-each-package-does--read -- neither of
                    // the follower's timers' periods is read: it measures once a second.
                    let next = now + 1000.0;
                    *self.tasks.last_mut()? = Task::Follow { leader, radius, next_ms: next };
                }
                self.fire = FireMode::Nearest;
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: false })
            }
            Task::Search { search, next_ms } => {
                if let Search::Building(id) = search
                    && senses.find(id).is_none_or(|b| b.own)
                {
                    return None;
                }
                let mut walk = Walk::Keep;
                if senses.walker_idle || now >= next_ms {
                    let goal = match search {
                        Search::Enemies => senses
                            .nearest(
                                |s| s.hostile && s.type_word & !HUNTED_TYPES == 0 && !s.building,
                                SEEK_RANGE,
                            )
                            .map(|s| s.position),
                        // STAND-IN: docs/31-packages.md#what-each-package-does--read -- a
                        // building's pod, the generator's half distance and the construction
                        // phase are not modelled: the nearest building's placement; and the
                        // capturer's retreat, read to lie off the map, roams.
                        Search::Capture(mask) => senses
                            .nearest(
                                |s| {
                                    s.building
                                        && !s.own
                                        && s.type_word & !mask & 0x7fff_ffff == 0
                                        && s.type_word != BRIDGE
                                        && s.type_word != MAIN_TELEPORT
                                },
                                f32::MAX,
                            )
                            .map(|s| s.position),
                        Search::Building(id) => senses.find(id).map(|s| s.position),
                    };
                    let goal = goal.unwrap_or_else(|| self.roam(senses.bounds, at));
                    walk = Walk::To(goal, 1.0);
                    let rescan =
                        if matches!(search, Search::Enemies) { SEARCH_RESCAN_MS } else { CAPTURE_RESCAN_MS };
                    let next = self.timer(now, rescan);
                    *self.tasks.last_mut()? = Task::Search { search, next_ms: next };
                }
                self.fire = FireMode::Nearest;
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
            Task::Attack { target, fighting, next_ms } => {
                if !senses.has_weapon {
                    return None;
                }
                let id = match target {
                    Some(id) => id,
                    None => senses.nearest(|s| s.hostile, f32::MAX)?.id,
                };
                let victim = *senses.find(id)?;
                self.fire = if fighting { FireMode::Fixed(id) } else { FireMode::Nearest };
                let mut walk = Walk::Keep;
                let mut fighting = fighting;
                let mut next = next_ms;
                if senses.walker_idle || now >= next_ms {
                    let point = if victim.building {
                        let r = victim.radius;
                        let d = r - ATTACK_BUILDING_INSIDE.0
                            + self.random() * (ATTACK_BUILDING_INSIDE.0 - ATTACK_BUILDING_INSIDE.1);
                        let a = self.random() * std::f32::consts::TAU;
                        victim.position + Vec3::new(a.cos(), a.sin(), 0.0) * d.max(0.0)
                    } else {
                        let toward = (victim.position - at).with_z(0.0).normalize_or(Vec3::Y);
                        let aside = Vec3::new(-toward.y, toward.x, 0.0);
                        let short = ATTACK_SHORT.0 + self.random() * ATTACK_SHORT.1;
                        victim.position - toward * short
                            + aside * ((self.random() * 2.0 - 1.0) * ATTACK_ASIDE)
                    };
                    fighting = at.distance(point) < ATTACK_MAX_FIRE_DISTANCE;
                    let speed = if fighting { ATTACK_SPEED.0 + self.random() * ATTACK_SPEED.1 } else { 1.0 };
                    walk = Walk::To(point, speed);
                    next = self.timer(now, ATTACK_TIMER_MS);
                    self.fire = FireMode::Fixed(id);
                }
                *self.tasks.last_mut()? = Task::Attack { target: Some(id), fighting, next_ms: next };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
        }
    }

    /// A random point at least 100 inside the map (`0x10030ed2`).
    ///
    /// STAND-IN: docs/31-packages.md#where-a-search-looks--read-and-measured -- which areals
    /// are usable is not modelled: the first point tried is taken.
    fn roam(&mut self, (lo, hi): ([f32; 2], [f32; 2]), at: Vec3) -> Vec3 {
        let pick = |me: &mut Self, a: usize| {
            let (from, to) = (lo[a] + ROAM_INSET, hi[a] - ROAM_INSET);
            if to > from { from + me.random() * (to - from) } else { (lo[a] + hi[a]) / 2.0 }
        };
        Vec3::new(pick(self, 0), pick(self, 1), at.z)
    }
}

/// The fight module's distance score (`0x1001b9f0`): 0 to 1 over the first 5 m, 1 out to
/// (v + 1) ÷ 2, 0 at 2 (v + 1); times 1 − height ÷ v.
pub fn distance_score(distance: f32, height: f32, round_speed: f32) -> f32 {
    let v = round_speed.max(1e-3);
    let (hold, gone) = ((v + 1.0) / 2.0, 2.0 * (v + 1.0));
    let score = if distance < 5.0 {
        distance / 5.0
    } else if distance <= hold {
        1.0
    } else if distance < gone {
        (gone - distance) / (gone - hold)
    } else {
        0.0
    };
    score * (1.0 - height / v)
}

/// An AI gun's wait before its next shot (`0x1001b5ec`): 30 ÷ the magazine s plus up to
/// as much again for a magazine of more than 2, otherwise 0.5 s plus up to 1.5; `random`
/// 0..1.
pub fn fire_wait_ms(magazine: i32, random: f32) -> f64 {
    let seconds = if magazine > 2 {
        let base = 30.0 / magazine as f32;
        base + random * base
    } else {
        0.5 + random * 1.5
    };
    f64::from(seconds) * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: i32, x: f32, y: f32) -> Seen {
        Seen {
            id,
            position: Vec3::new(x, y, 0.0),
            radius: 3.0,
            type_word: 0x0100_8000,
            building: false,
            own: false,
            hostile: false,
        }
    }

    fn senses<'a>(seen: &'a [Seen], now_ms: f64, at: Vec3, idle: bool) -> Senses<'a> {
        Senses {
            now_ms,
            position: at,
            seen,
            bounds: ([0.0; 2], [2000.0; 2]),
            has_weapon: true,
            walker_idle: idle,
            neutral: false,
        }
    }

    #[test]
    fn follow_me_picks_a_spot_about_its_leader_only_once_it_is_left_behind() {
        let leader = Seen { own: true, ..unit(1, 100.0, 100.0) };
        let order = Order { code: orders::FOLLOW, parameter: 50, target: Target::LogicId(1) };
        assert_eq!(Task::from_order(&order), Task::Follow { leader: 1, radius: 20.0, next_ms: 0.0 });
        let mut b = Behaviour::new(7);
        b.order(&order);
        let near = b.takt(&senses(&[leader], 0.0, Vec3::new(130.0, 100.0, 0.0), true));
        assert_eq!(near.walk, Walk::Keep, "30 off is within 20 + 20");
        let far = b.takt(&senses(&[leader], 2000.0, Vec3::new(200.0, 100.0, 0.0), true));
        let Walk::To(spot, share) = far.walk else { panic!("{far:?}") };
        assert!((spot.x - 100.0).abs() <= 20.0 && (spot.y - 100.0).abs() <= 20.0 && spot.z == 5.0);
        assert_eq!(share, 1.0);
        let gone = Seen { own: false, ..leader };
        b.takt(&senses(&[gone], 4000.0, Vec3::ZERO, true));
        assert_eq!(b.task(), Task::Stop, "a leader of another clan ends the follow");
    }

    #[test]
    fn standby_holds_and_is_never_pulled_into_a_fight_but_a_stopped_unit_is() {
        let enemy = Seen { hostile: true, ..unit(9, 300.0, 0.0) };
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::STAYGROUND, parameter: 0, target: Target::NotDefined });
        let t = b.takt(&senses(&[enemy], 0.0, Vec3::ZERO, true));
        assert_eq!(
            (t.walk, t.target, b.task()),
            (Walk::Clear, Some(9), Task::StayGround),
            "its guns still aim"
        );
        let mut idle = Behaviour::new(3);
        let t = idle.takt(&senses(&[enemy], 0.0, Vec3::ZERO, true));
        assert!(matches!(idle.task(), Task::Attack { target: Some(9), .. }));
        let Walk::To(point, _) = t.walk else { panic!("{t:?}") };
        assert!(
            (200.0..=250.0).contains(&point.x) && point.y.abs() <= 80.0,
            "50–100 short, up to 80 aside: {point}"
        );
        assert!(t.fire_freely);
        // The target dies: the attack ends and the unit stops again.
        idle.takt(&senses(&[], 1000.0, Vec3::ZERO, true));
        assert_eq!(idle.task(), Task::Stop);
    }

    #[test]
    fn seek_and_destroy_walks_to_the_nearest_enemy_and_otherwise_roams_and_refit_fails() {
        let far_enemy = Seen { hostile: true, ..unit(5, 1500.0, 0.0) };
        let mut b = Behaviour::new(11);
        b.order(&Order { code: orders::SEARCH, parameter: 0, target: Target::Any });
        let t = b.takt(&senses(&[far_enemy], 0.0, Vec3::ZERO, true));
        assert_eq!(t.walk, Walk::To(far_enemy.position, 1.0));
        let t = b.takt(&senses(&[], 20_000.0, Vec3::ZERO, true));
        let Walk::To(p, _) = t.walk else { panic!() };
        assert!((100.0..=1900.0).contains(&p.x) && (100.0..=1900.0).contains(&p.y), "roams: {p}");

        let mut capture = Behaviour::new(2);
        capture.order(&Order {
            code: orders::SEARCH,
            parameter: 0,
            target: Target::TypeMask(orders::CAPTURE_TYPES),
        });
        let bridge = Seen { building: true, type_word: BRIDGE, ..unit(3, 50.0, 0.0) };
        let t = capture.takt(&senses(&[bridge], 0.0, Vec3::ZERO, true));
        assert!(matches!(t.walk, Walk::To(p, _) if p != bridge.position), "a bridge is never captured");

        let mut refit = Behaviour::new(5);
        refit.order(&Order { code: orders::RELOAD, parameter: 0, target: Target::NotDefined });
        refit.takt(&senses(&[], 0.0, Vec3::ZERO, true));
        assert_eq!(refit.task(), Task::Stop);
    }

    #[test]
    fn a_gun_scores_fully_out_to_half_its_round_speed_and_waits_by_its_magazine() {
        assert_eq!(distance_score(2.5, 0.0, 350.0), 0.5);
        assert_eq!(distance_score(175.0, 0.0, 350.0), 1.0);
        assert!((distance_score(438.75, 0.0, 350.0) - 0.5).abs() < 1e-3);
        assert_eq!(distance_score(702.0, 0.0, 350.0), 0.0);
        assert_eq!(fire_wait_ms(-1, 0.0), 500.0);
        assert_eq!(fire_wait_ms(10, 1.0), 6000.0);
    }
}
