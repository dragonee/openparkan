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
/// A capture's plan takes a generator at half its distance (`0x100308ac`).
pub const GENERATOR: u32 = 0x8000_0002;
/// The largest size class that may capture (`0x100301a9`), and the bit a building's logic id
/// and a capture's type mask carry.
pub const CAPTURE_SIZE_MAX: u8 = 2;
pub const BUILDING_BIT: u32 = 0x8000_0000;
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
/// The escape's first ring of tries: points within this of the unit along each axis,
/// inside the map by the roaming inset (`0x1002ba50`).
pub const LEAVE_REACH: f32 = 150.0;
/// The fight module's bars (`0x10024e7a`): a flyer's, and a walker's.
pub const FIRE_BAR_FLYER: f32 = 0.45;
pub const FIRE_BAR_WALKER: f32 = 0.85;
/// The base priority's limit, which a stopped unit hands the attack it takes up: this far
/// from where it stood (`0x1000193e`).
pub const STOP_LIMIT: f32 = 1000.0;
/// The patrol's compiled defaults (`Behavior.dll:0x10016250`): a place's speed figure and
/// radius, a building's radius and a unit's, and each kind's loop delays, (fixed, random) s.
/// A building's speed figure, 80, and a unit's, 1.0, both hold the walker at full speed.
pub const PATROL_PLACE_SPEED: f32 = 0.8;
pub const PATROL_PLACE_RADIUS: f32 = 60.0;
pub const PATROL_BUILDING_RADIUS: f32 = 30.0;
pub const PATROL_UNIT_RADIUS: f32 = 60.0;
pub const PATROL_PLACE_DELAY_S: (f64, f64) = (20.0, 10.0);
pub const PATROL_BUILDING_DELAY_S: (f64, f64) = (60.0, 60.0);
pub const PATROL_UNIT_DELAY_S: (f64, f64) = (5.0, 10.0);
/// A loop's points: a place's 15 and a unit's 3, and a 16-bit random % 5 more
/// (`0x1002de5f`, `0x1002e246`); each tried this many times inside the map less the
/// roaming inset, the last try kept (`0x1002df68`).
pub const PATROL_PLACE_POINTS: usize = 15;
pub const PATROL_UNIT_POINTS: usize = 3;
pub const PATROL_EXTRA_POINTS: u32 = 5;
pub const PATROL_TRIES: usize = 350;
/// The limit a patrol hands the attack it lets through, past its radius (jump table
/// `0x1002d374`): about a place, a building, a unit, and a unit's for a call for help as
/// radius × 1.3 + 78.
pub const PATROL_PLACE_LIMIT: f32 = 60.0;
pub const PATROL_BUILDING_LIMIT: f32 = 80.0;
pub const PATROL_UNIT_LIMIT: f32 = 60.0;
pub const PATROL_HELP_SCALE: f32 = 1.3;
pub const PATROL_HELP_LIMIT: f32 = 78.0;
/// A unit guard scores a contact by its distance from the unit × this (`0x10059968`).
pub const PATROL_UNIT_SHARE: f32 = 0.7;
/// The go task's `Go_SpeedPercent`, and how near its place it is over (`0x1002b670`).
pub const GO_SPEED: f32 = 1.0;
pub const GO_ARRIVED: f32 = 30.0;
/// `Build_SpeedPercent` and `Transport_SpeedPercent` (`Behavior.dll:0x10016250`).
pub const BUILD_SPEED: f32 = 1.0;
pub const TRANSPORT_SPEED: f32 = 1.0;

/// Where a build task stands (`+0x5c`, `0x10028b80`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildState {
    /// Given, not yet walking.
    Start,
    /// 3, GoToBuild: walking to the site.
    Going,
    /// 4, Wait: at the site; the building is made on the takt after.
    Arrived,
}

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

/// The places of a building a capturer goes to (docs/31, "The capture, tick by tick"), in the
/// world.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Places {
    pub id: i32,
    /// Standing, its construction sphere done (variable `0x202` not above 0).
    pub complete: bool,
    /// Its hall way's pod vertex (`0x40`), carried through its node.
    pub pod: Option<Vec3>,
    /// Its contour's vertices a flyer may land at (variable `0x203`, on areals whose flag word
    /// is not 0).
    pub contour: Vec<Vec3>,
}

/// What a takt has to go on.
#[derive(Clone, Copy, Debug)]
pub struct Senses<'a> {
    pub now_ms: f64,
    pub position: Vec3,
    /// Every live object but the unit, the clan's areal map being whole from its first
    /// tick (docs/31, "Where a search looks").
    pub seen: &'a [Seen],
    /// The buildings' places, for a capture; empty when the unit runs none.
    pub places: &'a [Places],
    /// The unit's size class (`+0x30`, variable `0x201`) and whether its chassis has `CanFly`.
    pub size_class: u8,
    pub flyer: bool,
    /// The map's extent in x and y.
    pub bounds: ([f32; 2], [f32; 2]),
    pub has_weapon: bool,
    /// Whether the walker has nothing left to follow.
    pub walker_idle: bool,
    /// A neutral clan's unit never engages on its own.
    pub neutral: bool,
    /// A building: it runs its fire control and no unit takt, so it takes up no engagement
    /// and its tasks do not move it (docs/31, "Which objects run a behaviour").
    pub building: bool,
    /// An animal: it takes up no engagement unless it migrates (`0x10017a1e`).
    pub animal: bool,
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

/// What a patrol guards (slot 3, `0x1002d520`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Guarded {
    /// A place, `TARGET_BY_PLACE`, with its z as given.
    Place(Vec3),
    /// A building, by a logic id with bit 31.
    Building(i32),
    /// A unit, by any other logic id: of the patroller's own clan only.
    Unit(i32),
}

/// A task's limit (`+0x2c`, tested by slot 9, `0x10001660`): past `radius` from its centre,
/// in three dimensions, the task is ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
    pub centre: Limited,
    pub radius: f32,
}

/// Where a limit is measured from: a place, or a unit by logic id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Limited {
    Place(Vec3),
    Unit(i32),
}

impl Limit {
    /// Whether a unit at `at` is past the limit; a named unit that is gone tests nothing.
    fn passed(&self, at: Vec3, senses: &Senses) -> bool {
        let centre = match self.centre {
            Limited::Place(p) => Some(p),
            Limited::Unit(id) => senses.find(id).map(|s| s.position),
        };
        self.radius > 0.0 && centre.is_some_and(|c| at.distance(c) > self.radius)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
    /// No order: the unit stands, and engages on its own.
    Stop,
    /// Standby: clears the walker every takt, and is pulled into nothing.
    StayGround,
    /// Shutdown (order 19, vtable `0x10059eec`): its takt clears the walker and asks the fire
    /// control for nothing, and its priority is 0 for every reason (`0x10031b10`).
    Shutdown,
    /// Patrol (order 4, vtable `0x10059d38`): a loop of points about what it guards, walked
    /// in turn and drawn afresh on its timer. `next_loop_ms` is `None` until the task
    /// starts, or starts again once an attack over it is dropped.
    Patrol {
        guarded: Guarded,
        radius: f32,
        speed: f32,
        index: usize,
        next_loop_ms: Option<f64>,
    },
    Follow {
        leader: i32,
        radius: f32,
        next_ms: f64,
    },
    /// Search (order 5, vtable `0x10059cf8`): `building` is the one picked (`+0x6c`),
    /// `landing` the landing flag (`+0x7c`, set once a flyer is to walk in), `next_ms` the
    /// rescan timer (`+0x74`) and `started` whether slot 6 has run.
    Search {
        search: Search,
        next_ms: f64,
        building: Option<i32>,
        landing: bool,
        started: bool,
    },
    /// Refit: a trip to a dock.
    Reload,
    Attack {
        target: Option<i32>,
        fighting: bool,
        next_ms: f64,
        /// The limit the task beneath handed it, if any.
        limit: Option<Limit>,
    },
    /// The escape (`ORDER_ROBOT_LEAVE`): off a building to open ground, ending when the
    /// walker has nothing left to do; its priority is 0 for every reason (`0x1002b9f0`).
    Leave {
        goal: Option<Vec3>,
    },
    /// Route (order 2, vtable `0x10059e74`): to a place at `Go_SpeedPercent`, over within 30
    /// of it once the walker stops; interrupted only by reasons 3 and 4 (`0x1002b390`).
    Go {
        goal: Vec3,
        walking: bool,
    },
    /// Build (order 7, `M_Task_Build`, vtable `0x10059c60`): to the site's origin, where the
    /// play makes the building of `type_word` at the placement `site` (x, y, z, turn).
    Build {
        type_word: u32,
        site: [f32; 4],
        state: BuildState,
    },
    /// Transport minerals (order 6, `M_Task_Transport`, vtable `0x10059ca8`): the play picks
    /// the mine and the storage and hands the task each place to walk to; the task tells it
    /// when it has arrived (docs/32, "Transporting ore").
    Transport {
        goal: Option<Vec3>,
        going: bool,
        arrived: bool,
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
            // The patrol lets every reason but a refit through (`0x1002d250`).
            Task::Patrol { .. } => 1.0,
            _ => 0.0,
        }
    }

    /// A search not yet started.
    pub fn search(search: Search) -> Task {
        Task::Search { search, next_ms: 0.0, building: None, landing: false, started: false }
    }

    /// The task an order builds (the dispatcher's `INSERT_ORDER_REPLACE`).
    pub fn from_order(order: &Order) -> Task {
        Self::try_from_order(order).unwrap_or(Task::Stop)
    }

    /// The task an order builds, or `None` where its task refuses the target, as a patrol
    /// of anything but a place, a building or a unit does ("*** Task_Patrol has incorrect
    /// target").
    pub fn try_from_order(order: &Order) -> Option<Task> {
        Some(match (order.code, order.target) {
            (orders::SHUTDOWN, _) => Task::Shutdown,
            (orders::PATROL, target) => {
                // The radius is the kind's default unless the parameter is neither 0 nor −1.
                let p = order.parameter;
                let radius = |default: f32| if p == 0 || p == -1 { default } else { p as f32 };
                let (guarded, radius, speed) = match target {
                    Target::Place(at) => (
                        Guarded::Place(Vec3::from_array(at)),
                        radius(PATROL_PLACE_RADIUS),
                        PATROL_PLACE_SPEED,
                    ),
                    Target::LogicId(id) if id < 0 => {
                        (Guarded::Building(id), radius(PATROL_BUILDING_RADIUS), 1.0)
                    }
                    Target::LogicId(id) => (Guarded::Unit(id), radius(PATROL_UNIT_RADIUS), 1.0),
                    _ => return None,
                };
                Task::Patrol { guarded, radius, speed, index: 0, next_loop_ms: None }
            }
            (orders::STAYGROUND, _) => Task::StayGround,
            (orders::FOLLOW, Target::LogicId(leader)) => {
                let p = order.parameter as f32;
                let radius = if p > 20.0 && p < 30.0 { p } else { FOLLOW_RADIUS };
                Task::Follow { leader, radius, next_ms: 0.0 }
            }
            (orders::SEARCH, Target::TypeMask(mask)) => Task::search(Search::Capture(mask)),
            // A script's `ORDER_ROBOT_CAPTURE` builds the same search on one building (docs/31,
            // "The orders").
            (orders::SEARCH | orders::CAPTURE, Target::LogicId(id)) => Task::search(Search::Building(id)),
            (orders::SEARCH, _) => Task::search(Search::Enemies),
            (orders::RELOAD, _) => Task::Reload,
            (orders::LEAVE, _) => Task::Leave { goal: None },
            (orders::ATTACK, Target::LogicId(id)) => {
                Task::Attack { target: Some(id), fighting: false, next_ms: 0.0, limit: None }
            }
            (orders::GO, Target::Place(at)) => Task::Go { goal: Vec3::from_array(at), walking: false },
            (orders::GO, _) => return None,
            // A `0x202` place becomes an unturned placement at it (`SetTarget`, `0x100285f0`).
            (orders::BUILD, Target::Placement(site)) => {
                Task::Build { type_word: order.parameter as u32, site, state: BuildState::Start }
            }
            (orders::BUILD, Target::Place([x, y, z])) => Task::Build {
                type_word: order.parameter as u32,
                site: [x, y, z, 0.0],
                state: BuildState::Start,
            },
            (orders::BUILD, _) => return None,
            // The transport ignores its target (`0x10031f70`).
            (orders::TRANSPORT, _) => Task::Transport { goal: None, going: false, arrived: false },
            _ => Task::Stop,
        })
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
    /// Into the building of logic id `.0` to its pod at `.1`, at a share of the unit's speed
    /// (`MakeInsideDest`, `0x10001270`): the play routes the walk through its hall way.
    Inside(i32, Vec3, f32),
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
    /// The running patrol's loop of points (`+0xa4`).
    pub patrol_loop: Vec<Vec3>,
    seed: u32,
}

impl Behaviour {
    pub fn new(seed: u32) -> Self {
        Self { tasks: vec![Task::Stop], fire: FireMode::Nearest, patrol_loop: Vec::new(), seed: seed | 1 }
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

    /// An order put in the list as `insert` says (`INSERT_ORDER_*`): replacing every task,
    /// before the running one, or after the rest. A stop at the bottom is the empty
    /// stack's own task, which an order to the end takes the place of. False when the task
    /// refuses its target, leaving the tasks as they were.
    pub fn insert_order(&mut self, order: &Order, insert: u32) -> bool {
        let Some(task) = Task::try_from_order(order) else { return false };
        match insert {
            orders::INSERT_TO_START => self.tasks.push(task),
            orders::INSERT_TO_END => match self.tasks.first() {
                Some(Task::Stop) | None => {
                    if self.tasks.is_empty() {
                        self.tasks.push(task);
                    } else {
                        self.tasks[0] = task;
                    }
                }
                Some(_) => self.tasks.insert(0, task),
            },
            _ => self.tasks = vec![task],
        }
        true
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
        // contacts are scored through the task is not read (the contact record's three
        // unnamed fields): the nearest hostile unit within 500 is the best, and for a patrol
        // the one nearest its centre inside its radius. An attack already running is not
        // given another.
        let task = self.task();
        if !senses.neutral
            && !senses.building
            && !senses.animal
            && senses.has_weapon
            && task.engage_priority() >= ENGAGE_BAR
            && !matches!(task, Task::Attack { .. })
            && let Some((enemy, limit)) = Self::engagement(task, senses)
        {
            self.tasks.push(Task::Attack { target: Some(enemy), fighting: false, next_ms: 0.0, limit });
        }
        for _ in 0..4 {
            match self.run(senses) {
                Some(takt) => {
                    // The stack's takt tests the task's limit after its takt (`0x10034a53`):
                    // an attack past it ends, and the patrol beneath starts again.
                    if let Task::Attack { limit: Some(limit), .. } = self.task()
                        && limit.passed(senses.position, senses)
                    {
                        self.tasks.pop();
                        if let Some(Task::Patrol { next_loop_ms, .. }) = self.tasks.last_mut() {
                            *next_loop_ms = None;
                        }
                    }
                    return takt;
                }
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

    /// The contact `task` lets an engagement take up, and the limit it hands the attack
    /// (`0x10017e70`, the task's slots 12 and 13).
    fn engagement(task: Task, senses: &Senses) -> Option<(i32, Option<Limit>)> {
        let hostile = |s: &&Seen| s.hostile && !s.building;
        match task {
            // A patrol scores only a contact strictly inside its radius of its centre, across
            // the ground (`0x1002d4a1`), and limits the attack about what it guards.
            Task::Patrol { guarded, radius, .. } => {
                let (centre, share, limit) = match guarded {
                    Guarded::Place(p) => {
                        (p, 1.0, Limit { centre: Limited::Place(p), radius: radius + PATROL_PLACE_LIMIT })
                    }
                    Guarded::Building(id) => {
                        let p = senses.find(id)?.position;
                        (p, 1.0, Limit { centre: Limited::Place(p), radius: radius + PATROL_BUILDING_LIMIT })
                    }
                    Guarded::Unit(id) => (
                        senses.find(id)?.position,
                        PATROL_UNIT_SHARE,
                        Limit { centre: Limited::Unit(id), radius: radius + PATROL_UNIT_LIMIT },
                    ),
                };
                let d = |s: &Seen| s.position.truncate().distance(centre.truncate()) * share;
                let enemy = senses
                    .seen
                    .iter()
                    .filter(hostile)
                    .filter(|s| d(s) < radius)
                    .min_by(|a, b| d(a).total_cmp(&d(b)))?;
                Some((enemy.id, Some(limit)))
            }
            // The base priority's limit: 1000 about where the unit stands (`0x100018a0`).
            Task::Stop => {
                let enemy = senses.nearest(|s| s.hostile && !s.building, ENGAGE_RANGE)?;
                let limit = Limit { centre: Limited::Place(senses.position), radius: STOP_LIMIT };
                Some((enemy.id, Some(limit)))
            }
            _ => senses.nearest(|s| s.hostile && !s.building, ENGAGE_RANGE).map(|s| (s.id, None)),
        }
    }

    /// The fire control's target (`0x100240a6`). The radar module lists no buildings, so the
    /// nearest hostile contact is a unit's (docs/31, "Which objects run a behaviour").
    fn fire_target(&self, senses: &Senses) -> Option<i32> {
        match self.fire {
            FireMode::None => None,
            FireMode::Fixed(id) => senses.find(id).map(|s| s.id),
            FireMode::Nearest => senses.nearest(|s| s.hostile && !s.building, ENGAGE_RANGE).map(|s| s.id),
        }
    }

    /// A 16-bit random, as the patrol's point counts take one (`0x10066bfc`).
    ///
    /// STAND-IN: docs/31-packages.md#what-each-package-does--read -- the behaviour's random
    /// source is not read: the same xorshift, its high bits.
    fn random16(&mut self) -> u32 {
        (self.random() * 65_536.0) as u32
    }

    /// A patrol's loop about `centre` (`0x1002dd90`): a place's or a unit's 15 or 3 points
    /// and up to 4 more, each within the radius on x and y and inside the map less 100; a
    /// building's points about it.
    ///
    /// STAND-IN: docs/31-packages.md#where-a-search-looks--read-and-measured -- which areals
    /// are usable is not modelled: a walker's point needs no usable areal, as a flyer's does
    /// not.
    fn draw_loop(&mut self, guarded: Guarded, centre: Vec3, radius: f32, senses: &Senses) -> Vec<Vec3> {
        let (lo, hi) = senses.bounds;
        let inside = |p: Vec3| {
            p.x > lo[0] + ROAM_INSET
                && p.x < hi[0] - ROAM_INSET
                && p.y > lo[1] + ROAM_INSET
                && p.y < hi[1] - ROAM_INSET
        };
        let count = match guarded {
            // STAND-IN: docs/31-packages.md#the-patrol-tick-by-tick--read -- a building's
            // contour (property `0x203`) is not modelled: eight points on its sphere, pushed
            // out by the block's 30, stand in for its vertices.
            Guarded::Building(id) => {
                let reach = senses.find(id).map_or(0.0, |s| s.radius) + PATROL_BUILDING_RADIUS;
                return (0..8)
                    .map(|k| {
                        let a = k as f32 * std::f32::consts::FRAC_PI_4;
                        centre + Vec3::new(a.cos(), a.sin(), 0.0) * reach
                    })
                    .collect();
            }
            Guarded::Place(_) => PATROL_PLACE_POINTS,
            Guarded::Unit(_) => PATROL_UNIT_POINTS,
        } + (self.random16() % PATROL_EXTRA_POINTS) as usize;
        (0..count)
            .map(|_| {
                let mut point = centre;
                for _ in 0..PATROL_TRIES {
                    let (dx, dy) = (self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0);
                    point = centre + Vec3::new(dx * radius, dy * radius, 0.0);
                    if inside(point) {
                        break;
                    }
                }
                point
            })
            .collect()
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
            // STAND-IN: docs/31-packages.md#migrate-an-animals-pasture--read-and-measured -- an
            // animal's default order, migrate, is not modelled: it stands, asking the fire
            // control for nothing, as a grazing animal does.
            Task::Stop if senses.animal => at_rest(FireMode::None, Walk::Keep, self),
            Task::Stop => at_rest(FireMode::Nearest, Walk::Keep, self),
            Task::StayGround => at_rest(FireMode::Nearest, Walk::Clear, self),
            Task::Shutdown => at_rest(FireMode::None, Walk::Clear, self),
            Task::Patrol { guarded, radius, speed, index, next_loop_ms } => {
                // A unit guarded must stand, of the patroller's own clan; a building must
                // stand ("PatrolUnit failed", "PatrolBuilding failed").
                let centre = match guarded {
                    Guarded::Place(p) => p,
                    Guarded::Building(id) => senses.find(id)?.position,
                    Guarded::Unit(id) => senses.find(id).filter(|s| s.own)?.position,
                };
                self.fire = FireMode::Nearest;
                let (mut index, mut next) = (index, next_loop_ms);
                let mut walk = Walk::Keep;
                if next.is_none_or(|n| now >= n) {
                    // The start, and the loop's timer run out: a fresh loop, from its first point.
                    self.patrol_loop = self.draw_loop(guarded, centre, radius, senses);
                    let delay = match guarded {
                        Guarded::Place(_) => PATROL_PLACE_DELAY_S,
                        Guarded::Building(_) => PATROL_BUILDING_DELAY_S,
                        Guarded::Unit(_) => PATROL_UNIT_DELAY_S,
                    };
                    next = Some(self.timer(now, (delay.0 * 1000.0, delay.1 * 1000.0)));
                    index = 0;
                    walk = Walk::To(*self.patrol_loop.first()?, speed);
                } else if senses.walker_idle && !self.patrol_loop.is_empty() {
                    index = (index + 1) % self.patrol_loop.len();
                    walk = Walk::To(self.patrol_loop[index], speed);
                }
                *self.tasks.last_mut()? = Task::Patrol { guarded, radius, speed, index, next_loop_ms: next };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: false })
            }
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
            Task::Search { search: Search::Enemies, next_ms, .. } => {
                let mut walk = Walk::Keep;
                if senses.walker_idle || now >= next_ms {
                    let goal = senses
                        .nearest(|s| s.hostile && s.type_word & !HUNTED_TYPES == 0 && !s.building, SEEK_RANGE)
                        .map(|s| s.position);
                    let goal = goal.unwrap_or_else(|| self.roam(senses.bounds, at));
                    walk = Walk::To(goal, 1.0);
                    let next = self.timer(now, SEARCH_RESCAN_MS);
                    *self.tasks.last_mut()? = Task::search(Search::Enemies);
                    if let Some(Task::Search { next_ms, started, .. }) = self.tasks.last_mut() {
                        (*next_ms, *started) = (next, true);
                    }
                }
                self.fire = FireMode::Nearest;
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
            Task::Search { search, next_ms, building, landing, started } => {
                let walk = self.capture_takt(search, next_ms, building, landing, started, senses)?;
                self.fire = FireMode::Nearest;
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
            // STAND-IN: docs/31-packages.md#the-escape--read -- which areals are usable is not
            // modelled: the first point tried within 150 of the unit, inside the map by 100.
            Task::Leave { goal: None } => {
                let (lo, hi) = senses.bounds;
                let pick = |me: &mut Self, a: usize| {
                    let (from, to) = (lo[a] + ROAM_INSET, hi[a] - ROAM_INSET);
                    let v = at[a] + (me.random() * 2.0 - 1.0) * LEAVE_REACH;
                    if to > from { v.clamp(from, to) } else { (lo[a] + hi[a]) / 2.0 }
                };
                let goal = Vec3::new(pick(self, 0), pick(self, 1), at.z);
                *self.tasks.last_mut()? = Task::Leave { goal: Some(goal) };
                self.fire = FireMode::None;
                Some(Takt { walk: Walk::To(goal, 1.0), target: None, fire_freely: false })
            }
            Task::Leave { goal: Some(goal) } => {
                if senses.walker_idle {
                    return None;
                }
                self.fire = FireMode::None;
                let _ = goal;
                Some(Takt { walk: Walk::Keep, target: None, fire_freely: false })
            }
            Task::Go { goal, walking } => {
                self.fire = FireMode::Nearest;
                let mut walk = Walk::Keep;
                if !walking {
                    walk = Walk::To(goal, GO_SPEED);
                } else if senses.walker_idle {
                    // STAND-IN: docs/31-packages.md#what-each-package-does--read -- whether the
                    // go task's 30 is measured in three dimensions is not read: across the
                    // ground, as a script's place carries no height.
                    if at.truncate().distance(goal.truncate()) <= GO_ARRIVED {
                        return None;
                    }
                    walk = Walk::To(goal, GO_SPEED);
                }
                *self.tasks.last_mut()? = Task::Go { goal, walking: true };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
            Task::Build { type_word, site, state } => {
                self.fire = FireMode::Nearest;
                let (walk, state) = match state {
                    BuildState::Start => {
                        (Walk::To(Vec3::new(site[0], site[1], site[2]), BUILD_SPEED), BuildState::Going)
                    }
                    BuildState::Going if senses.walker_idle => (Walk::Clear, BuildState::Arrived),
                    BuildState::Going => (Walk::Keep, BuildState::Going),
                    BuildState::Arrived => (Walk::Clear, BuildState::Arrived),
                };
                *self.tasks.last_mut()? = Task::Build { type_word, site, state };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: false })
            }
            Task::Transport { goal, going, arrived } => {
                self.fire = FireMode::Nearest;
                let (walk, going, arrived) = match goal {
                    Some(g) if !going && !arrived => (Walk::To(g, TRANSPORT_SPEED), true, false),
                    Some(_) if going && senses.walker_idle => (Walk::Clear, false, true),
                    Some(_) if going => (Walk::Keep, true, false),
                    _ => (Walk::Clear, false, arrived),
                };
                *self.tasks.last_mut()? = Task::Transport { goal, going, arrived };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: false })
            }
            Task::Attack { target, fighting, next_ms, limit } => {
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
                *self.tasks.last_mut()? = Task::Attack { target: Some(id), fighting, next_ms: next, limit };
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

    /// A capture search's takt (docs/31, "The capture, tick by tick"): its start, then slot 7.
    /// `None` when it ends or refuses its target.
    fn capture_takt(
        &mut self,
        search: Search,
        next_ms: f64,
        building: Option<i32>,
        landing: bool,
        started: bool,
        senses: &Senses,
    ) -> Option<Walk> {
        let now = senses.now_ms;
        let (mut building, mut landing, mut next_ms) = (building, landing, next_ms);
        let walk = if !started {
            // Slot 3 (`0x10030110`): a logic id must be another clan's building, not a main
            // teleport, for a unit of size class 2 at most; a mask with the top bit wants the
            // same size. The timer is 15 s and up to 15, or 3 and up to 3 by type.
            let rescan = match search {
                Search::Building(id) => {
                    let b = senses.find(id)?;
                    if id as u32 & BUILDING_BIT == 0
                        || !b.building
                        || b.own
                        || b.type_word == MAIN_TELEPORT
                        || senses.size_class > CAPTURE_SIZE_MAX
                    {
                        return None;
                    }
                    building = Some(id);
                    SEARCH_RESCAN_MS
                }
                Search::Capture(mask) if mask & BUILDING_BIT != 0 => {
                    if senses.size_class > CAPTURE_SIZE_MAX {
                        return None;
                    }
                    CAPTURE_RESCAN_MS
                }
                _ => SEARCH_RESCAN_MS,
            };
            next_ms = self.timer(now, rescan);
            // Slot 6 (`0x10030280`): the landing flag from `CanFly`, then a plan.
            landing = !senses.flyer;
            self.plan_capture(search, &mut building, &mut landing, senses)
        } else {
            // Slot 7 (`0x10030300`): a building picked and gone or turned the unit's ends a
            // single building, and a search by type plans the next; one still another
            // clan's skips the timer.
            let picked = building.map(|id| senses.find(id).filter(|b| !b.own).is_some());
            match picked {
                Some(false) => match search {
                    Search::Building(_) => return None,
                    _ => {
                        building = None;
                        landing = !senses.flyer;
                        self.plan_capture(search, &mut building, &mut landing, senses)
                    }
                },
                Some(true) if !senses.walker_idle => Walk::Keep,
                // A flyer at its corner sets the flag and walks in; one on the pod asks again.
                Some(true) => {
                    landing = true;
                    self.plan_capture(search, &mut building, &mut landing, senses)
                }
                None if now >= next_ms || senses.walker_idle => {
                    let rescan = if matches!(search, Search::Capture(m) if m & BUILDING_BIT != 0) {
                        CAPTURE_RESCAN_MS
                    } else {
                        SEARCH_RESCAN_MS
                    };
                    next_ms = self.timer(now, rescan);
                    self.plan_capture(search, &mut building, &mut landing, senses)
                }
                None => Walk::Keep,
            }
        };
        *self.tasks.last_mut()? = Task::Search { search, next_ms, building, landing, started: true };
        Some(walk)
    }

    /// A capture's plan (slot 15, `0x100306f0`): a search by type picks the nearest building of
    /// its mask no other clan's but another's, finished, no bridge or main teleport, across the
    /// ground with a generator at half; a walker, or a flyer to walk in, goes to its pod; a
    /// flyer first to its contour's nearest vertex; otherwise it roams.
    ///
    /// STAND-IN: docs/31-packages.md#where-a-search-looks--read-and-measured -- the retreat
    /// read to lie off the map, and which areals are usable, are not modelled: a plan with
    /// nowhere to go roams.
    fn plan_capture(
        &mut self,
        search: Search,
        building: &mut Option<i32>,
        landing: &mut bool,
        senses: &Senses,
    ) -> Walk {
        let at = senses.position;
        if let Search::Capture(mask) = search {
            let finished = |id: i32| senses.places.iter().find(|p| p.id == id).is_none_or(|p| p.complete);
            let weight = |s: &Seen| {
                let d = s.position.truncate().distance(at.truncate());
                if s.type_word == GENERATOR { d / 2.0 } else { d }
            };
            *building = senses
                .seen
                .iter()
                .filter(|s| {
                    s.building
                        && !s.own
                        && s.type_word & !mask & 0x7fff_ffff == 0
                        && s.type_word != BRIDGE
                        && s.type_word != MAIN_TELEPORT
                        && finished(s.id)
                })
                .min_by(|a, b| weight(a).total_cmp(&weight(b)))
                .map(|s| s.id);
        }
        if let Some(id) = *building {
            let places = senses.places.iter().find(|p| p.id == id);
            // `MakeInsideDest` (`0x10001270`): a finished building with a pod, a small unit.
            if !senses.flyer || *landing {
                let pod = places
                    .filter(|p| p.complete && senses.size_class <= CAPTURE_SIZE_MAX)
                    .and_then(|p| p.pod);
                if let Some(pod) = pod {
                    *landing = true;
                    return Walk::Inside(id, pod, GO_SPEED);
                }
            }
            if senses.flyer {
                *landing = false;
                let across = |p: &Vec3| p.truncate().distance(at.truncate());
                let corner =
                    places.and_then(|p| p.contour.iter().min_by(|a, b| across(a).total_cmp(&across(b))));
                if let Some(&corner) = corner {
                    return Walk::To(corner, GO_SPEED);
                }
            }
        }
        Walk::To(self.roam(senses.bounds, at), GO_SPEED)
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
            places: &[],
            size_class: 2,
            flyer: false,
            bounds: ([0.0; 2], [2000.0; 2]),
            has_weapon: true,
            walker_idle: idle,
            neutral: false,
            building: false,
            animal: false,
        }
    }

    fn place_patrol(x: f32, y: f32) -> Order {
        Order { code: orders::PATROL, parameter: 0, target: Target::Place([x, y, 0.0]) }
    }

    #[test]
    fn a_shut_down_unit_neither_moves_aims_nor_takes_up_a_fight() {
        let enemy = Seen { hostile: true, ..unit(9, 100.0, 0.0) };
        let mut b = Behaviour::new(3);
        assert!(
            b.insert_order(&Order { code: orders::SHUTDOWN, parameter: 0, target: Target::NotDefined }, 3)
        );
        let t = b.takt(&senses(&[enemy], 0.0, Vec3::ZERO, true));
        assert_eq!((t.walk, t.target, b.task()), (Walk::Clear, None, Task::Shutdown));
    }

    #[test]
    fn a_place_patrol_walks_a_loop_of_15_to_19_points_within_its_radius_at_0_8_and_redraws_it_on_its_timer() {
        let mut b = Behaviour::new(21);
        assert!(b.insert_order(&place_patrol(1124.0, 783.0), 3));
        let t = b.takt(&senses(&[], 0.0, Vec3::new(1800.0, 1800.0, 250.0), true));
        let Walk::To(first, share) = t.walk else { panic!("{t:?}") };
        assert_eq!(share, PATROL_PLACE_SPEED);
        assert!((15..=19).contains(&b.patrol_loop.len()), "{}", b.patrol_loop.len());
        assert!(
            b.patrol_loop
                .iter()
                .all(|p| (p.x - 1124.0).abs() <= 60.0 && (p.y - 783.0).abs() <= 60.0 && p.z == 0.0)
        );
        assert_eq!(first, b.patrol_loop[0]);
        assert_eq!(t.target, None);
        // Busy, it keeps on; idle, it moves on to the next point.
        let busy = b.takt(&senses(&[], 1000.0, first, false));
        assert_eq!(busy.walk, Walk::Keep);
        let next = b.takt(&senses(&[], 2000.0, first, true));
        assert_eq!(next.walk, Walk::To(b.patrol_loop[1], PATROL_PLACE_SPEED));
        // Past 30 s a fresh loop is always due.
        let old = b.patrol_loop.clone();
        let fresh = b.takt(&senses(&[], 31_000.0, first, false));
        assert!(matches!(fresh.walk, Walk::To(..)) && b.patrol_loop != old);
        // A guard of a unit of another clan fails.
        let mut guard = Behaviour::new(4);
        guard.order(&Order { code: orders::PATROL, parameter: 0, target: Target::LogicId(7) });
        guard.takt(&senses(&[unit(7, 10.0, 0.0)], 0.0, Vec3::ZERO, true));
        assert_eq!(guard.task(), Task::Stop);
        assert!(
            !Behaviour::new(1)
                .insert_order(&Order { code: orders::PATROL, parameter: 0, target: Target::Any }, 3)
        );
    }

    #[test]
    fn a_patrol_fights_only_inside_its_ground_and_drops_an_attack_that_strays_past_radius_plus_60() {
        let place = Vec3::new(1000.0, 1000.0, 0.0);
        let mut b = Behaviour::new(8);
        b.order(&Order { code: orders::PATROL, parameter: 0, target: Target::Place(place.to_array()) });
        // An enemy 61 m from the place, however near the patroller, is not taken up.
        let outside = Seen { hostile: true, ..unit(5, 1061.0, 1000.0) };
        let t = b.takt(&senses(&[outside], 0.0, Vec3::new(1060.0, 1000.0, 0.0), true));
        assert!(matches!(b.task(), Task::Patrol { .. }));
        assert_eq!(t.target, Some(5), "its guns still aim at the nearest within 500");
        let inside = Seen { hostile: true, ..unit(6, 1030.0, 1000.0) };
        b.takt(&senses(&[inside], 1000.0, Vec3::new(1030.0, 1000.0, 0.0), true));
        assert!(matches!(
            b.task(),
            Task::Attack { target: Some(6), limit: Some(Limit { radius: 120.0, .. }), .. }
        ));
        // Past 120 m of the place (in 3D) the attack ends and the patrol draws afresh.
        b.takt(&senses(&[inside], 2000.0, Vec3::new(1000.0, 1000.0, 121.0), true));
        assert!(matches!(b.task(), Task::Patrol { next_loop_ms: None, .. }), "{:?}", b.task());
    }

    #[test]
    fn a_stopped_unit_limits_its_attack_to_1000_from_where_it_stood_and_buildings_and_animals_take_up_none() {
        let enemy = Seen { hostile: true, ..unit(9, 300.0, 0.0) };
        let mut b = Behaviour::new(3);
        b.takt(&senses(&[enemy], 0.0, Vec3::ZERO, true));
        assert!(matches!(b.task(), Task::Attack { limit: Some(Limit { radius: 1000.0, .. }), .. }));

        let mut bunker = Behaviour::new(3);
        let t = bunker.takt(&Senses { building: true, ..senses(&[enemy], 0.0, Vec3::ZERO, true) });
        assert_eq!((bunker.task(), t.target), (Task::Stop, Some(9)), "a building only aims and fires");
        let hall = Seen { building: true, hostile: true, ..unit(4, 10.0, 0.0) };
        let t = bunker.takt(&Senses { building: true, ..senses(&[hall], 0.0, Vec3::ZERO, true) });
        assert_eq!(t.target, None, "the radar lists no buildings");

        let mut medusa = Behaviour::new(3);
        let t = medusa.takt(&Senses { animal: true, ..senses(&[enemy], 0.0, Vec3::ZERO, true) });
        assert_eq!((medusa.task(), t.target), (Task::Stop, None));
    }

    #[test]
    fn an_order_to_the_end_waits_behind_the_running_one_and_takes_the_empty_stacks_place() {
        let mut b = Behaviour::new(1);
        assert!(b.insert_order(&place_patrol(500.0, 500.0), orders::INSERT_TO_END));
        assert_eq!(b.tasks.len(), 1);
        let standby = Order { code: orders::STAYGROUND, parameter: 0, target: Target::NotDefined };
        assert!(b.insert_order(&standby, orders::INSERT_TO_END));
        assert!(matches!(b.task(), Task::Patrol { .. }) && b.tasks[0] == Task::StayGround);
        assert!(b.insert_order(&standby, orders::INSERT_REPLACE));
        assert_eq!(b.tasks, vec![Task::StayGround]);
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

    fn capturer<'a>(seen: &'a [Seen], places: &'a [Places], at: Vec3, idle: bool) -> Senses<'a> {
        Senses { places, size_class: 1, flyer: true, ..senses(seen, 0.0, at, idle) }
    }

    #[test]
    fn a_capture_takes_a_generator_at_half_distance_lands_a_flyer_at_its_corner_then_walks_it_in() {
        let building =
            |id: i32, x: f32, type_word: u32| Seen { building: true, type_word, ..unit(id, x, 0.0) };
        let generator = building(-2_147_483_643, 500.0, GENERATOR);
        let plant = building(-2_147_483_646, 300.0, 0x8000_0010);
        let teleport = building(-2_147_483_647, 100.0, MAIN_TELEPORT);
        let own = Seen { own: true, ..building(-2_147_483_644, 50.0, 0x8000_0400) };
        let places = |id: i32, x: f32| Places {
            id,
            complete: true,
            pod: Some(Vec3::new(x, 0.0, 1.0)),
            contour: vec![Vec3::new(x - 30.0, 5.0, 0.0), Vec3::new(x + 30.0, 5.0, 0.0)],
        };
        let all = [places(generator.id, 500.0), places(plant.id, 300.0), places(teleport.id, 100.0)];
        let seen = [generator, plant, teleport, own];
        let search =
            Order { code: orders::SEARCH, parameter: 0, target: Target::TypeMask(orders::CAPTURE_TYPES) };

        let mut b = Behaviour::new(7);
        b.order(&search);
        // The generator's 500 counts as 250: nearer than the plant, and the teleport and the
        // unit's own building are never picked. A flyer first makes for the nearest corner.
        let t = b.takt(&capturer(&seen, &all, Vec3::ZERO, true));
        assert_eq!(t.walk, Walk::To(Vec3::new(470.0, 5.0, 0.0), GO_SPEED));
        assert!(
            matches!(b.task(), Task::Search { building: Some(id), landing: false, .. } if id == generator.id)
        );
        assert_eq!(b.takt(&capturer(&seen, &all, Vec3::new(200.0, 0.0, 0.0), false)).walk, Walk::Keep);
        // Idle at its corner it sets the flag and walks in, and asks again while waiting.
        let inside = Walk::Inside(generator.id, Vec3::new(500.0, 0.0, 1.0), GO_SPEED);
        assert_eq!(b.takt(&capturer(&seen, &all, Vec3::new(470.0, 5.0, 0.0), true)).walk, inside);
        assert!(matches!(b.task(), Task::Search { landing: true, .. }));
        assert_eq!(b.takt(&capturer(&seen, &all, Vec3::new(500.0, 0.0, 1.0), true)).walk, inside);
        // Taken, the search by type goes on to the plant, landing first again.
        let taken = [Seen { own: true, ..generator }, plant, teleport, own];
        let t = b.takt(&capturer(&taken, &all, Vec3::new(500.0, 0.0, 1.0), false));
        assert_eq!(t.walk, Walk::To(Vec3::new(330.0, 5.0, 0.0), GO_SPEED));

        // A walker goes straight in.
        let mut walker = Behaviour::new(3);
        walker.order(&search);
        assert_eq!(
            walker.takt(&Senses { flyer: false, ..capturer(&seen, &all, Vec3::ZERO, true) }).walk,
            inside
        );

        // One building: never a main teleport or the unit's own, never for a unit bigger than 2;
        // it ends once the building is the unit's clan's.
        for (id, size) in [(teleport.id, 1), (own.id, 1), (plant.id, 3)] {
            let mut one = Behaviour::new(5);
            one.order(&Order { code: orders::CAPTURE, parameter: 0, target: Target::LogicId(id) });
            one.takt(&Senses { size_class: size, ..capturer(&seen, &all, Vec3::ZERO, true) });
            assert_eq!(one.task(), Task::Stop, "{id} size {size}");
        }
        let mut one = Behaviour::new(5);
        one.order(&Order { code: orders::SEARCH, parameter: 0, target: Target::LogicId(plant.id) });
        let t = one.takt(&capturer(&seen, &all, Vec3::ZERO, true));
        assert_eq!(t.walk, Walk::To(Vec3::new(270.0, 5.0, 0.0), GO_SPEED));
        let taken = [Seen { own: true, ..plant }];
        one.takt(&capturer(&taken, &all, Vec3::new(300.0, 0.0, 1.0), true));
        assert_eq!(one.task(), Task::Stop);
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
    fn a_go_walks_to_its_place_and_is_over_within_30_once_the_walker_stops() {
        let order = Order { code: orders::GO, parameter: 0, target: Target::Place([500.0, 0.0, 0.0]) };
        let mut b = Behaviour::new(2);
        assert!(b.insert_order(&order, orders::INSERT_REPLACE));
        let t = b.takt(&senses(&[], 0.0, Vec3::ZERO, true));
        assert_eq!((t.walk, t.fire_freely), (Walk::To(Vec3::new(500.0, 0.0, 0.0), 1.0), true));
        assert_eq!(b.takt(&senses(&[], 100.0, Vec3::new(100.0, 0.0, 0.0), false)).walk, Walk::Keep);
        // Stopped short, it sets off again; stopped within 30, it is over.
        assert!(matches!(b.takt(&senses(&[], 200.0, Vec3::new(400.0, 0.0, 0.0), true)).walk, Walk::To(..)));
        b.takt(&senses(&[], 300.0, Vec3::new(475.0, 0.0, 9.0), true));
        assert_eq!(b.task(), Task::Stop);
        // An enemy in reach does not pull it off its way.
        let enemy = Seen { hostile: true, ..unit(9, 50.0, 0.0) };
        let mut c = Behaviour::new(2);
        c.order(&order);
        c.takt(&senses(&[enemy], 0.0, Vec3::ZERO, true));
        assert!(matches!(c.task(), Task::Go { .. }));
    }

    #[test]
    fn a_build_walks_to_its_site_and_waits_there_for_the_play() {
        let order = Order {
            code: orders::BUILD,
            parameter: 0x8000_0004_u32 as i32,
            target: Target::Placement([1026.0, 942.0, 81.0, 0.5]),
        };
        let mut b = Behaviour::new(2);
        b.order(&order);
        let t = b.takt(&senses(&[], 0.0, Vec3::ZERO, true));
        assert_eq!(t.walk, Walk::To(Vec3::new(1026.0, 942.0, 81.0), 1.0));
        b.takt(&senses(&[], 100.0, Vec3::ZERO, false));
        assert!(matches!(b.task(), Task::Build { state: BuildState::Going, .. }));
        b.takt(&senses(&[], 200.0, Vec3::ZERO, true));
        assert_eq!(
            b.task(),
            Task::Build {
                type_word: 0x8000_0004,
                site: [1026.0, 942.0, 81.0, 0.5],
                state: BuildState::Arrived
            }
        );
    }

    #[test]
    fn a_transport_walks_where_the_play_sends_it_and_says_when_it_is_there() {
        let mut b = Behaviour::new(2);
        b.order(&Order { code: orders::TRANSPORT, parameter: -1, target: Target::NotDefined });
        assert_eq!(b.takt(&senses(&[], 0.0, Vec3::ZERO, true)).walk, Walk::Clear);
        *b.tasks.last_mut().unwrap() = Task::Transport { goal: Some(Vec3::X), going: false, arrived: false };
        assert_eq!(b.takt(&senses(&[], 10.0, Vec3::ZERO, true)).walk, Walk::To(Vec3::X, 1.0));
        b.takt(&senses(&[], 20.0, Vec3::X, true));
        assert_eq!(b.task(), Task::Transport { goal: Some(Vec3::X), going: false, arrived: true });
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
