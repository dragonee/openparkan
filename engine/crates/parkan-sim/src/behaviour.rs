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
/// The escape's rings of tries (`0x1002ba50`): so many points within so far of the unit along
/// each axis, each inside the map by the roaming inset and on a usable areal.
pub const LEAVE_RINGS: [(usize, f32); 4] = [(400, 150.0), (300, 300.0), (200, 400.0), (200, 1000.0)];
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
/// The interrupt gate's pause between interrupts of any reason, (fixed, random) ms
/// (`0x1000388e`): it restarts each time the gate reaches it.
pub const INTERRUPT_PAUSE_MS: (f64, f64) = (2000.0, 3000.0);
/// An interrupt-made attack answers a new contact by `1000 / (d + 10)` across the ground,
/// nothing from this far (`0x10026b10`).
pub const ATTACK_SWITCH_SCORE: f32 = 1000.0;
pub const ATTACK_SWITCH_RANGE: f32 = 700.0;
/// A follower answers a retaliation with a limit of `2 × radius + 20` about its leader
/// (`0x1002acd0`).
pub const FOLLOW_RETALIATE_SLACK: f32 = 20.0;
/// A migrating animal's answer to a hit (`0x1002c640`), by where it and the firer stand
/// against its pasture, across the ground: the attack's time, s, and its circle past the outer
/// radius about the pasture's centre, `None` for no circle.
pub const MIGRATE_AWAY: (f64, Option<f32>) = (10.0, None);
pub const MIGRATE_FIRER_OUTSIDE: (f64, Option<f32>) = (20.0, Some(20.0));
pub const MIGRATE_FIRER_ON_PASTURE: (f64, Option<f32>) = (25.0, Some(80.0));
pub const MIGRATE_FIRER_INSIDE: (f64, Option<f32>) = (35.0, Some(100.0));
/// A migrating animal's answer to an engagement (`0x1002c6c9`), when it stands inside the outer
/// radius and the contact within it: the attack's time, s, and its circle past the outer
/// radius, for a contact inside the inner radius and for one on the rest of the pasture.
pub const MIGRATE_ENGAGE_INSIDE: (f64, f32) = (20.0, 80.0);
pub const MIGRATE_ENGAGE_ON_PASTURE: (f64, f32) = (10.0, 20.0);
/// The migrate task's own timers, (fixed, random) ms (`0x1002c9d0`): the long one, after which
/// it asks for its clan's pasture again, and the short one an idle animal waits at its point.
pub const MIGRATE_LONG_MS: (f64, f64) = (60_000.0, 120_000.0);
pub const MIGRATE_SHORT_MS: (f64, f64) = (5_000.0, 10_000.0);
/// A point's two offsets from the pasture's centre, each a random share of the inner radius
/// held to at least this (`0x100597e8`), both positive; and how many points it tries before
/// the walker takes one (`0x1002cc92`).
pub const MIGRATE_POINT_FLOOR: f32 = 0.2;
pub const MIGRATE_TRIES: usize = 50;
/// A clan's pasture timer (`ArealMap.dll:0x10022230`, the words 937 and 1875 at
/// `Behavior.dll:0x1002ab37`, `0x1002ab3d`, × 64 ms): (fixed, random) ms.
pub const PASTURE_TIMER_MS: (f64, f64) = (59_968.0, 120_000.0);
/// An animal's attack fires on its target from within this, and on the nearest hostile
/// contact beyond (`0x10027580`).
pub const ANIMAL_FIXED_FIRE: f32 = 200.0;

/// The go task's `Go_SpeedPercent`, and how near its place it is over (`0x1002b670`).
pub const GO_SPEED: f32 = 1.0;
pub const GO_ARRIVED: f32 = 30.0;
/// `Build_SpeedPercent` and `Transport_SpeedPercent` (`Behavior.dll:0x10016250`).
pub const BUILD_SPEED: f32 = 1.0;
pub const TRANSPORT_SPEED: f32 = 1.0;

/// A refit (`M_Task_Reload`, docs/27, "What sends a bot to a dock"): it stands in its dock
/// until life, charge and ammunition are all at 98% (`0x1002ed3b`).
pub const REFIT_FULL: f32 = 0.98;
/// The largest size class `MakeInsideDest` routes into a building (*"TypedSizes missmached"*,
/// `0x10001270`): a bigger unit fits through no door, so only a ground-level dock serves it.
pub const INSIDE_SIZE_MAX: u8 = 2;
/// What sends a bot to a dock on its own (`0x10017d50`, docs/27): its life under half (under
/// 90% for a building, `0x1001c700`), its charge under half (`0x1001cbe0`), or more than 80%
/// of its guns under 20% of their magazine (`0x1001ca40`).
pub const SERVICE_LIFE: f32 = 0.5;
pub const SERVICE_BUILDING_LIFE: f32 = 0.9;
pub const SERVICE_CHARGE: f32 = 0.5;
pub const SERVICE_GUNS: f32 = 0.8;
pub const SERVICE_MAGAZINE: f32 = 0.2;
/// A capture answers 0 for a refit unless its life is at most 0.2 or its charge at most 0.3
/// (`0x10030000`).
pub const CAPTURE_REFIT_LIFE: f32 = 0.2;
pub const CAPTURE_REFIT_CHARGE: f32 = 0.3;
/// The AI's repair decision (`0x10017c70`, docs/26, "What the AI does with the switch"): the
/// life it switches the repair system on below and off above, and the charge it needs to
/// switch on and falls back off at.
///
/// STAND-IN: docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured
/// -- which difficulty profile a unit's behaviour holds (`+0x8d4`) is not read:
/// `diff_strong.var`'s `Decision_RepairOn` and `Decision_RepairOff` (*measured*), so a unit
/// looks after itself while it is only lightly damaged.
pub const REPAIR_ON: f32 = 0.8;
pub const REPAIR_OFF: f32 = 0.9;
pub const REPAIR_CHARGE_ON: f32 = 0.3;
pub const REPAIR_CHARGE_OFF: f32 = 0.1;

/// An upgrade (`M_Task_Upgrade`, docs/32, "Upgrading a building"): the builder walks to a point
/// beside the building, this far out past its sphere, and counts itself there within this of
/// the point it picked. It tries this many points on that ring for one on usable ground.
pub const UPGRADE_BESIDE: f32 = 20.0;
pub const UPGRADE_ARRIVED: f32 = 10.0;
pub const UPGRADE_TRIES: usize = 77;

/// How far an upgrade has got (`+0x5c`, `0x100332e0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpgradeState {
    /// GoToBuild: walking to the point beside the building.
    Going,
    /// Standing beside it, for the play to start the sphere.
    Arrived,
    /// Waiting while the play walks the building up its scheme.
    Working,
}

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
    /// On the unit's own radar list now (docs/25, "What the AI does with it"). A search
    /// looks over the clan's areal map, which lists whatever stands in an areal a unit's
    /// report has reached; only what the unit's own radar holds may be engaged or fired on.
    pub sensed: bool,
}

/// A contact the fire control may take up: on the unit's radar list, hostile, and not a
/// building, which the radar module never lists (docs/25, "What the AI does with it").
fn engageable(s: &Seen) -> bool {
    s.sensed && s.hostile && !s.building
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

/// A dock a refit may go to (docs/27, "The places"), in the world: a hall-way vertex that
/// charges, repairs and rearms who stands in it. Only the docks of a building that would
/// charge this unit — its own clan's or an ally's — are handed to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dock {
    /// The building's logic id.
    pub id: i32,
    /// Which of that building's docks, as the world lists them.
    pub index: usize,
    /// Where it stands, its vertex carried into the world through its node.
    pub at: Vec3,
    /// Ground level (`0x10000000`): a place 10 across and 12 high, outside the building, which
    /// any unit may stand in. An indoor dock is 5 and 3, and only a unit that fits inside may
    /// be routed to it.
    pub ground_level: bool,
    /// The place's own cylinder about its vertex (`0x100184f0`): how far across, how far above
    /// and how far below a unit's origin may stand and still be in it.
    pub radius: f32,
    pub height: f32,
    pub below: f32,
}

impl Dock {
    /// Whether a unit whose origin is `at` stands in the place (`0x10018310`, docs/27, "Who
    /// stands in a place"), which is what the dock charges and what a refit waits in.
    pub fn holds(&self, at: Vec3) -> bool {
        let dz = at.z - self.at.z;
        (-self.below..=self.height).contains(&dz) && at.truncate().distance(self.at.truncate()) <= self.radius
    }
}

/// What a unit's own systems report to its behaviour: the life fraction over its whole control
/// system (property `0x31`), its batteries' fill, the share of its guns' magazines left, and
/// whether more than [`SERVICE_GUNS`] of its guns are under [`SERVICE_MAGAZINE`] of their
/// magazine (`0x1001ca40`). See docs/27, "What sends a bot to a dock".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condition {
    pub life: f32,
    pub charge: f32,
    pub ammo: f32,
    pub guns_dry: bool,
}

impl Default for Condition {
    fn default() -> Self {
        Self { life: 1.0, charge: 1.0, ammo: 1.0, guns_dry: false }
    }
}

impl Condition {
    /// Whether the unit needs service (`0x10017d50`): its life or its charge under half, or its
    /// guns mostly dry; a building under 90% of its life.
    pub fn needs_service(&self, building: bool) -> bool {
        let bar = if building { SERVICE_BUILDING_LIFE } else { SERVICE_LIFE };
        self.life < bar || self.charge < SERVICE_CHARGE || self.guns_dry
    }

    /// Whether a refit is over: life, charge and ammunition all at 98% (`0x1002ed3b`).
    pub fn refitted(&self) -> bool {
        self.life >= REFIT_FULL && self.charge >= REFIT_FULL && self.ammo >= REFIT_FULL
    }
}

/// Whether a point lies on a usable areal, one a walker may be sent to (docs/24, "The global
/// path"; docs/31, "Where a search looks").
#[derive(Clone, Copy)]
pub struct Usable<'a>(pub &'a dyn Fn(f32, f32) -> bool);

fn anywhere(_: f32, _: f32) -> bool {
    true
}

impl Usable<'_> {
    /// Every point, as on a map with no areal map.
    pub const ANYWHERE: Usable<'static> = Usable(&anywhere);

    pub fn at(&self, p: Vec3) -> bool {
        (self.0)(p.x, p.y)
    }
}

impl std::fmt::Debug for Usable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Usable")
    }
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
    /// Every dock that would charge this unit, for a refit (docs/27, "What sends a bot to a
    /// dock").
    pub docks: &'a [Dock],
    /// What its own systems report: what a refit waits for, and what sends it to a dock.
    pub condition: Condition,
    /// The unit's size class (`+0x30`, variable `0x201`) and whether its chassis has `CanFly`.
    pub size_class: u8,
    pub flyer: bool,
    /// The map's extent in x and y.
    pub bounds: ([f32; 2], [f32; 2]),
    /// Where on the map a unit may be sent.
    pub usable: Usable<'a>,
    pub has_weapon: bool,
    /// Whether the walker has nothing left to follow.
    pub walker_idle: bool,
    /// A neutral clan's unit never engages on its own.
    pub neutral: bool,
    /// A building: it runs its fire control and no unit takt, so it takes up no engagement
    /// and its tasks do not move it (docs/31, "Which objects run a behaviour").
    pub building: bool,
    /// An animal: it takes up no engagement unless it migrates (`0x10017a1e`), and its default
    /// order is migrate.
    pub animal: bool,
    /// Where a migrating animal asks for its clan's pasture.
    pub pastures: Pastures<'a>,
}

/// A nature clan's zone an animal grazes in (docs/31, "Migrate: an animal's pasture").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pasture {
    pub centre: Vec3,
    pub inner: f32,
    pub outer: f32,
}

/// The pasture the unit's clan grazes, as the system areal map answers a migrating animal
/// that asks for it (slot 47, `ArealMap.dll:0x10022230`). Asking may pick a new one, so the
/// task asks only when the game's does: as it starts and when its long timer runs out.
#[derive(Clone, Copy)]
pub struct Pastures<'a>(pub &'a dyn Fn() -> Option<Pasture>);

fn no_pasture() -> Option<Pasture> {
    None
}

impl Pastures<'_> {
    /// A clan with no zones, as every clan but a nature clan's is (docs/31).
    pub const NONE: Pastures<'static> = Pastures(&no_pasture);

    pub fn ask(&self) -> Option<Pasture> {
        (self.0)()
    }
}

impl std::fmt::Debug for Pastures<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pastures")
    }
}

/// A clan's current pasture, as the system areal map keeps it (slot 47,
/// `ArealMap.dll:0x10022230`): it answers the same zone until the clan's timer runs out, then
/// picks one of the clan's zones at random — possibly the same again — and starts the timer
/// again; the first question picks at once. With no zones it answers none ("*** Migration Place
/// Error: Clan … has no MigrationAreals").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClanPasture {
    pub current: Option<usize>,
    pub until_ms: f64,
}

impl ClanPasture {
    /// The zone, of `count`, the clan grazes when asked at `now_ms`; `random` gives 0..1.
    ///
    /// STAND-IN: docs/31-packages.md#migrate-an-animals-pasture--read-and-measured -- the areal
    /// map's `rand()` is not the play's: the pick and the timer's share take the random handed in.
    pub fn ask(&mut self, count: usize, now_ms: f64, mut random: impl FnMut() -> f32) -> Option<usize> {
        if count == 0 {
            return None;
        }
        if self.current.is_none() || now_ms >= self.until_ms {
            self.until_ms = now_ms + PASTURE_TIMER_MS.0 + PASTURE_TIMER_MS.1 * f64::from(random());
            self.current = Some(((random() * count as f32) as usize).min(count - 1));
        }
        self.current
    }
}

impl Senses<'_> {
    fn find(&self, id: i32) -> Option<&Seen> {
        self.seen.iter().find(|s| s.id == id)
    }

    /// The dock a refit goes to (`0x10023b60`): the nearest one this unit fits, by the dock's
    /// own distance. A ground-level dock stands outside the building and takes any unit; an
    /// indoor one is reached along the hall way, so only a unit of size class at most
    /// [`INSIDE_SIZE_MAX`] is routed to one (*"TypedSizes missmached"*, `0x10001270`).
    ///
    /// STAND-IN: docs/27-ownership.md#what-sends-a-bot-to-a-dock--read -- the pick
    /// (`0x10023b60`) is not read, and the game's own refit asks `MakeInsideDest` for the
    /// ground-level bit on every dock, so it never sends anybody indoors: here a tiny or small
    /// unit takes whichever dock is nearest, indoors or out, and a medium or large one only a
    /// ground-level dock.
    pub fn dock(&self) -> Option<&Dock> {
        self.docks
            .iter()
            .filter(|d| d.ground_level || self.size_class <= INSIDE_SIZE_MAX)
            .min_by(|a, b| a.at.distance(self.position).total_cmp(&b.at.distance(self.position)))
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
/// in three dimensions, or past its time, the task is ended. A radius of 0 limits nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
    pub centre: Limited,
    pub radius: f32,
    /// When it runs out, ms, stamped as the task starts.
    pub until_ms: Option<f64>,
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
        (self.radius > 0.0 && centre.is_some_and(|c| at.distance(c) > self.radius))
            || self.until_ms.is_some_and(|t| senses.now_ms >= t)
    }

    /// Two limits taken together, the tighter of each winning.
    fn merged(self, other: Option<Limit>) -> Limit {
        let Some(other) = other else { return self };
        let radius = match (self.radius > 0.0, other.radius > 0.0) {
            (true, true) if other.radius < self.radius => other,
            (false, true) => other,
            _ => self,
        };
        let until_ms = match (self.until_ms, other.until_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        Limit { until_ms, ..radius }
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
    /// Migrate (order 15, vtable `0x10059aac`), an animal's default order: it grazes its clan's
    /// pasture (`+0x60`), walking to a point on it and on to another once it has stood 5 to 15
    /// s, and asks for the pasture again when its long timer (`+0x58`) runs out. `short_ms` is
    /// the short timer (`+0x64`); `started` whether slot 6 has run, which it does again when an
    /// attack over it ends.
    Migrate {
        pasture: Option<Pasture>,
        long_ms: f64,
        short_ms: f64,
        started: bool,
    },
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
    /// Refit (order 8, `M_Task_Reload`, vtable `0x10059be0`, which `ORDER_ROBOT_REPARE` builds
    /// too): a trip to a dock, where it stands until life, charge and ammunition are all at
    /// 98%. `dock` is the one picked at its start, by building and index, and `walking`
    /// whether the walker has been sent there.
    Reload {
        dock: Option<(i32, usize)>,
        walking: bool,
    },
    Attack {
        target: Option<i32>,
        fighting: bool,
        next_ms: f64,
        /// The limit the task beneath handed it, if any.
        limit: Option<Limit>,
        /// Given as an order (`+0x54` 1), which no interrupt switches.
        ordered: bool,
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
    /// Upgrade (order 24, `M_Task_Upgrade`, vtable `0x10059c20`): a trip to a building of the
    /// unit's own clan, which the play then walks one step up its scheme (docs/32, "Upgrading a
    /// building"). The task holds the building by logic id, the point beside it the walker was
    /// sent to, and how far it has got; the play takes it up once it has arrived and ends it
    /// when the new building's sphere stops.
    Upgrade {
        building: i32,
        state: UpgradeState,
        goal: Option<Vec3>,
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
            // Migrate answers by where the animal and the contact stand (`0x1002c640`), which
            // its engagement weighs.
            Task::Migrate { .. } => 1.0,
            Task::Search { search: Search::Enemies, .. } => 1.0,
            Task::Attack { .. } => 1.0,
            // The patrol lets every reason but a refit through (`0x1002d250`).
            Task::Patrol { .. } => 1.0,
            _ => 0.0,
        }
    }

    /// The interrupt priority for a refit, reason 3 (docs/31, "Between orders"): the base's,
    /// which lets one through while a dock is reachable (`0x100018a0`), except that a capture
    /// answers 0 unless the unit's life is at most 0.2 or its charge at most 0.3
    /// (`0x10030000`), and that a task that neither moves nor fights answers 0.
    ///
    /// STAND-IN: docs/31-packages.md#between-orders--read -- only the route's answer (reasons 3
    /// and 4 alone interrupt it, `0x1002b390`), the patrol's and the capture's are read: every
    /// other task that moves takes the base's, and standby, shutdown, the escape and a refit
    /// already running answer 0.
    fn refit_priority(&self, condition: Condition) -> f32 {
        match self {
            Task::StayGround | Task::Shutdown | Task::Leave { .. } | Task::Reload { .. } => 0.0,
            Task::Search { search: Search::Capture(_) | Search::Building(_), .. } => {
                let low = condition.life <= CAPTURE_REFIT_LIFE || condition.charge <= CAPTURE_REFIT_CHARGE;
                if low { 1.0 } else { 0.0 }
            }
            _ => 1.0,
        }
    }

    /// A search not yet started.
    pub fn search(search: Search) -> Task {
        Task::Search { search, next_ms: 0.0, building: None, landing: false, started: false }
    }

    /// Migrate not yet started: an animal's default order (docs/31, "Between orders").
    pub fn migrate() -> Task {
        Task::Migrate { pasture: None, long_ms: 0.0, short_ms: 0.0, started: false }
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
            (orders::RELOAD | orders::REPARE, _) => Task::Reload { dock: None, walking: false },
            (orders::LEAVE, _) => Task::Leave { goal: None },
            (orders::ATTACK, Target::LogicId(id)) => {
                Task::Attack { target: Some(id), fighting: false, next_ms: 0.0, limit: None, ordered: true }
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
            // The upgrade takes a building of the unit's own clan by logic id (`0x100332e0`);
            // the play refuses one it cannot walk up its scheme before the order is given.
            (orders::UPGRADE, Target::LogicId(id)) => {
                Task::Upgrade { building: id, state: UpgradeState::Going, goal: None }
            }
            (orders::UPGRADE, _) => return None,
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
    /// The unit that last hurt it, by logic id, whose retaliation the next takt asks for
    /// (message `0x19`, slot 67, `0x100064b0`).
    pub hurt_by: Option<i32>,
    /// When the interrupt gate next lets one through (`+0x5e0`).
    pub interrupt_ms: f64,
    /// What the AI's repair decision last sent the device manager (`+0xaf`, `0x10017c70`):
    /// its own repair system switched on. Starting any task turns it off (`0x10034930`).
    pub repair: bool,
    seed: u32,
}

impl Behaviour {
    pub fn new(seed: u32) -> Self {
        Self {
            tasks: vec![Task::Stop],
            fire: FireMode::Nearest,
            patrol_loop: Vec::new(),
            hurt_by: None,
            interrupt_ms: 0.0,
            repair: false,
            seed: seed | 1,
        }
    }

    /// A hit on the unit, fired by the unit of logic id `firer` (message `0x19`): it asks for
    /// an attack on it at its next takt. Delivered even when the hit does no damage.
    pub fn hurt(&mut self, firer: i32) {
        self.hurt_by = Some(firer);
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
        self.repair = false;
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
        // Starting any task turns the repair system off (`0x10034930`); the next decision
        // turns it back on if it is needed.
        self.repair = false;
        true
    }

    /// The task running.
    pub fn task(&self) -> Task {
        self.tasks.last().copied().unwrap_or(Task::Stop)
    }

    /// One behaviour takt: the self-refit and the engagement, then the running task's takt; a
    /// task that ends or fails gives way to the one beneath it, and with none left the unit
    /// stops. The repair decision runs unless this takt sent the unit to a dock or into an
    /// attack (docs/26, "What the AI does with the switch").
    pub fn takt(&mut self, senses: &Senses) -> Takt {
        // Engaging (`0x10017e70`): the best hostile contact within 500 becomes an attack on
        // top, unless the clan is neutral or the task holds the unit below the bar.
        //
        // STAND-IN: docs/31-packages.md#between-orders--read -- how the radar module's
        // contacts are scored through the task is not read (the contact record's three
        // unnamed fields): the nearest hostile unit within 500 is the best, and for a patrol
        // the one nearest its centre inside its radius. An attack already running is not
        // given another.
        // An animal's empty stack holds its default order, migrate (docs/31, "Between orders").
        if senses.animal && self.tasks.first() == Some(&Task::Stop) {
            self.tasks[0] = Task::migrate();
        }
        if let Some(firer) = self.hurt_by.take() {
            self.retaliate(firer, senses);
        }
        let refitting = self.self_refit(senses);
        let engaging = !refitting && self.engage(senses);
        if !refitting && !engaging {
            self.repair_decision(senses);
        }
        for _ in 0..4 {
            match self.run(senses) {
                Some(takt) => {
                    // The stack's takt tests the task's limit after its takt (`0x10034a53`):
                    // an attack past it ends, and the task beneath starts again.
                    if let Task::Attack { limit: Some(limit), .. } = self.task()
                        && limit.passed(senses.position, senses)
                    {
                        self.tasks.pop();
                        self.restart_beneath();
                    }
                    // The fire control does nothing for an animal whose walker is idle
                    // (`0x10024069`).
                    let target = if senses.animal && senses.walker_idle { None } else { takt.target };
                    return Takt { target, ..takt };
                }
                None => {
                    self.tasks.pop();
                    if self.tasks.is_empty() {
                        self.tasks.push(if senses.animal { Task::migrate() } else { Task::Stop });
                    }
                }
            }
        }
        Takt { walk: Walk::Clear, target: self.fire_target(senses), fire_freely: false }
    }

    /// Slot 9 ended the attack on top (`0x10001660`) and the task beneath starts again: a
    /// patrol draws a fresh loop, and migrate asks for its pasture and walks to a new point.
    fn restart_beneath(&mut self) {
        match self.tasks.last_mut() {
            Some(Task::Patrol { next_loop_ms, .. }) => *next_loop_ms = None,
            Some(Task::Migrate { started, .. }) => *started = false,
            _ => {}
        }
    }

    /// The engagement (`0x10017e70`), inserted as a reason-0 task: whether one was taken up.
    /// An animal takes one up only while it migrates (`0x10017a1e`).
    fn engage(&mut self, senses: &Senses) -> bool {
        let task = self.task();
        if !senses.neutral
            && !senses.building
            && (!senses.animal || matches!(task, Task::Migrate { .. }))
            && senses.has_weapon
            && task.engage_priority() >= ENGAGE_BAR
            && !matches!(task, Task::Attack { .. })
            && let Some((enemy, limit)) = Self::engagement(task, senses)
            && self.pause_passed(senses.now_ms)
        {
            let limit = limit.map(|l| Self::stamped(l, senses.now_ms));
            self.tasks.push(Task::Attack {
                target: Some(enemy),
                fighting: false,
                next_ms: 0.0,
                limit,
                ordered: false,
            });
            self.repair = false;
            return true;
        }
        false
    }

    /// The self-refit (`0x10017d50`, docs/27, "What sends a bot to a dock"), inserted as a
    /// reason-3 task: a unit that needs service — its life or its charge under half, or its
    /// guns mostly dry — sends itself to a dock, provided its task lets a refit through, a dock
    /// that would take it is in the world, and the interrupt gate's pause has run out. A
    /// building, an animal and a neutral clan's unit ask for none. Answers whether one was
    /// taken up.
    fn self_refit(&mut self, senses: &Senses) -> bool {
        let task = self.task();
        if senses.building
            || senses.animal
            || senses.neutral
            || !senses.condition.needs_service(false)
            || task.refit_priority(senses.condition) <= ENGAGE_BAR
            || senses.dock().is_none()
            || !self.pause_passed(senses.now_ms)
        {
            return false;
        }
        self.tasks.push(Task::Reload { dock: None, walking: false });
        self.repair = false;
        true
    }

    /// The AI's repair decision (`0x10017c70`, docs/26): it switches the unit's own repair
    /// system on while the unit needs service or its life is under [`REPAIR_ON`], provided its
    /// charge is over [`REPAIR_CHARGE_ON`]; and off once it needs none and its life is over
    /// [`REPAIR_OFF`], or whenever its charge falls under [`REPAIR_CHARGE_OFF`].
    fn repair_decision(&mut self, senses: &Senses) {
        let c = senses.condition;
        let needs = c.needs_service(senses.building);
        if (needs || c.life < REPAIR_ON) && c.charge > REPAIR_CHARGE_ON {
            self.repair = true;
        }
        if (!needs && c.life > REPAIR_OFF) || c.charge < REPAIR_CHARGE_OFF {
            self.repair = false;
        }
    }

    /// The interrupt gate's pause (step 7 of `0x100179c0`): whether it has run out, restarting
    /// it when it has.
    fn pause_passed(&mut self, now_ms: f64) -> bool {
        if now_ms < self.interrupt_ms {
            return false;
        }
        self.interrupt_ms = self.timer(now_ms, INTERRUPT_PAUSE_MS);
        true
    }

    /// A limit's time counted from `now_ms`, as the attack's start stamps it (`0x100349a8`).
    fn stamped(limit: Limit, now_ms: f64) -> Limit {
        Limit { until_ms: limit.until_ms.map(|t| now_ms + t), ..limit }
    }

    /// The attack on the unit that hurt it (reason 1, `0x10018060` through the gate
    /// `0x100179c0`): refused by an animal that is not migrating, a building, a neutral clan's
    /// unit, a task that answers 0.3 or less, the gate's pause, or a firer that is gone, the
    /// unit itself or of its own clan, and by an unarmed unit. No radar, relation or distance is
    /// asked. The attack goes on top, its limit the task's answer merged with the task's own.
    fn retaliate(&mut self, firer: i32, senses: &Senses) {
        let task = self.task();
        if senses.building || senses.neutral || (senses.animal && !matches!(task, Task::Migrate { .. })) {
            return;
        }
        let Some((priority, limit)) = self.retaliation(task, firer, senses) else { return };
        if priority <= ENGAGE_BAR || !self.pause_passed(senses.now_ms) {
            return;
        }
        let Some(target) = senses.find(firer).filter(|s| !s.own) else { return };
        if !senses.has_weapon {
            return;
        }
        let own = match task {
            Task::Attack { limit, .. } => limit,
            _ => None,
        };
        let limit = limit.map(|l| Self::stamped(l, senses.now_ms).merged(own)).or(own);
        self.tasks.push(Task::Attack {
            target: Some(target.id),
            fighting: false,
            next_ms: 0.0,
            limit,
            ordered: false,
        });
    }

    /// What `task` answers a hit from `firer` with (its slot 12 for reason 1, and slot 13's
    /// limit), `None` for 0.
    fn retaliation(&self, task: Task, firer: i32, senses: &Senses) -> Option<(f32, Option<Limit>)> {
        let at = senses.position;
        // The base priority (`0x100018a0`): 1 wherever the firer is, 1000 about where the unit
        // stands.
        let base =
            Some((1.0, Some(Limit { centre: Limited::Place(at), radius: STOP_LIMIT, until_ms: None })));
        match task {
            // A migrating animal answers 1 wherever the firer is; its pasture limits the attack,
            // by where it and the firer stand across the ground (`0x1002c799`). With no pasture,
            // or a firer on the outer circle itself, it answers as the base does (`0x1002c8df`).
            Task::Migrate { pasture: Some(p), .. } => {
                let firer_at = senses.find(firer)?.position;
                let me = at.truncate().distance(p.centre.truncate());
                let d = firer_at.truncate().distance(p.centre.truncate());
                let (time, circle) = if me >= p.outer {
                    MIGRATE_AWAY
                } else if d > p.outer {
                    MIGRATE_FIRER_OUTSIDE
                } else if d < p.inner {
                    MIGRATE_FIRER_INSIDE
                } else if d < p.outer {
                    MIGRATE_FIRER_ON_PASTURE
                } else {
                    return base;
                };
                let limit = Limit {
                    centre: Limited::Place(p.centre),
                    radius: circle.map_or(0.0, |c| p.outer + c),
                    until_ms: Some(time * 1000.0),
                };
                Some((1.0, Some(limit)))
            }
            // The base priority, and the tasks that embed it.
            Task::Stop
            | Task::Migrate { pasture: None, .. }
            | Task::Search { search: Search::Enemies, .. } => base,
            // A patrol answers 1 wherever the firer is, limited about what it guards.
            Task::Patrol { guarded, radius, .. } => {
                let limit = match guarded {
                    Guarded::Place(p) => Limit {
                        centre: Limited::Place(p),
                        radius: radius + PATROL_PLACE_LIMIT,
                        until_ms: None,
                    },
                    Guarded::Building(id) => Limit {
                        centre: Limited::Place(senses.find(id)?.position),
                        radius: radius + PATROL_BUILDING_LIMIT,
                        until_ms: None,
                    },
                    Guarded::Unit(id) => Limit {
                        centre: Limited::Unit(id),
                        radius: radius + PATROL_UNIT_LIMIT,
                        until_ms: None,
                    },
                };
                Some((1.0, Some(limit)))
            }
            Task::Follow { leader, radius, .. } if senses.has_weapon => Some((
                1.0,
                Some(Limit {
                    centre: Limited::Unit(leader),
                    radius: 2.0 * radius + FOLLOW_RETALIATE_SLACK,
                    until_ms: None,
                }),
            )),
            // An attack an interrupt made takes a firer that scores above its own target.
            Task::Attack { target, ordered: false, .. } => {
                let score = |id: i32| {
                    senses.find(id).map_or(0.0, |s| {
                        let d = s.position.truncate().distance(at.truncate());
                        if d > ATTACK_SWITCH_RANGE { 0.0 } else { ATTACK_SWITCH_SCORE / (d + 10.0) }
                    })
                };
                let now = target.map_or(0.0, score);
                let new = score(firer);
                (new > now && target != Some(firer)).then_some((new, None))
            }
            _ => None,
        }
    }

    /// The contact `task` lets an engagement take up, and the limit it hands the attack
    /// (`0x10017e70`, the task's slots 12 and 13).
    fn engagement(task: Task, senses: &Senses) -> Option<(i32, Option<Limit>)> {
        let hostile = |s: &&Seen| engageable(s);
        match task {
            // A patrol scores only a contact strictly inside its radius of its centre, across
            // the ground (`0x1002d4a1`), and limits the attack about what it guards.
            Task::Patrol { guarded, radius, .. } => {
                let (centre, share, limit) = match guarded {
                    Guarded::Place(p) => (
                        p,
                        1.0,
                        Limit {
                            centre: Limited::Place(p),
                            radius: radius + PATROL_PLACE_LIMIT,
                            until_ms: None,
                        },
                    ),
                    Guarded::Building(id) => {
                        let p = senses.find(id)?.position;
                        (
                            p,
                            1.0,
                            Limit {
                                centre: Limited::Place(p),
                                radius: radius + PATROL_BUILDING_LIMIT,
                                until_ms: None,
                            },
                        )
                    }
                    Guarded::Unit(id) => (
                        senses.find(id)?.position,
                        PATROL_UNIT_SHARE,
                        Limit {
                            centre: Limited::Unit(id),
                            radius: radius + PATROL_UNIT_LIMIT,
                            until_ms: None,
                        },
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
            // Migrate scores a contact within the inner radius of its pasture's centre by
            // 1 / (d + 10), d across the ground (`0x1002c910`), so the one nearest the centre is
            // best; it lets it through while the animal stands inside the outer radius, for 20 s
            // within outer + 80, or 10 s within outer + 20 for one on the inner circle itself
            // (`0x1002c6c9`). With no pasture nothing scores.
            Task::Migrate { pasture, .. } => {
                let p = pasture?;
                let d = |s: &Seen| s.position.truncate().distance(p.centre.truncate());
                let enemy = senses
                    .seen
                    .iter()
                    .filter(hostile)
                    .filter(|s| d(s) <= p.inner)
                    .min_by(|a, b| d(a).total_cmp(&d(b)))?;
                if senses.position.truncate().distance(p.centre.truncate()) >= p.outer {
                    return None;
                }
                let (time, circle) =
                    if d(enemy) < p.inner { MIGRATE_ENGAGE_INSIDE } else { MIGRATE_ENGAGE_ON_PASTURE };
                let limit = Limit {
                    centre: Limited::Place(p.centre),
                    radius: p.outer + circle,
                    until_ms: Some(time * 1000.0),
                };
                Some((enemy.id, Some(limit)))
            }
            // The base priority's limit: 1000 about where the unit stands (`0x100018a0`).
            Task::Stop => {
                let enemy = senses.nearest(engageable, ENGAGE_RANGE)?;
                let limit =
                    Limit { centre: Limited::Place(senses.position), radius: STOP_LIMIT, until_ms: None };
                Some((enemy.id, Some(limit)))
            }
            _ => senses.nearest(engageable, ENGAGE_RANGE).map(|s| (s.id, None)),
        }
    }

    /// The fire control's target (`0x100240a6`): the nearest hostile contact on the unit's own
    /// radar list, within 500 (docs/25, "What the AI does with it"). The radar module lists no
    /// buildings, so a picked target is always a unit's (docs/31, "Which objects run a
    /// behaviour"); a target an order names is taken whatever the radar holds.
    fn fire_target(&self, senses: &Senses) -> Option<i32> {
        match self.fire {
            FireMode::None => None,
            FireMode::Fixed(id) => senses.find(id).map(|s| s.id),
            FireMode::Nearest => senses.nearest(engageable, ENGAGE_RANGE).map(|s| s.id),
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
    /// and up to 4 more, each within the radius on x and y, inside the map less 100 and, but
    /// for a flyer's, on a usable areal, the last try kept; a building's points about it.
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
                    if inside(point) && (senses.flyer || senses.usable.at(point)) {
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
            Task::Stop => at_rest(FireMode::Nearest, Walk::Keep, self),
            // Migrate (its start `0x1002c9d0`, each tick `0x1002ca60`): the fire control asked
            // for nothing. As it starts and whenever its long timer runs out it asks for its
            // clan's pasture, walks to a new point of it and starts both timers again. Otherwise
            // the short timer starts again while the walker is busy, and once the walker is idle
            // and the short timer has run out the animal walks to a new point.
            Task::Migrate { pasture, long_ms, short_ms, started } => {
                let (mut pasture, mut long_ms, mut short_ms) = (pasture, long_ms, short_ms);
                let mut walk = Walk::Keep;
                if !started || now >= long_ms {
                    pasture = senses.pastures.ask();
                    long_ms = self.timer(now, MIGRATE_LONG_MS);
                    walk = self.graze(pasture, senses).map_or(Walk::Keep, |p| Walk::To(p, GO_SPEED));
                    short_ms = self.timer(now, MIGRATE_SHORT_MS);
                } else if !senses.walker_idle {
                    short_ms = self.timer(now, MIGRATE_SHORT_MS);
                } else if now >= short_ms {
                    walk = self.graze(pasture, senses).map_or(Walk::Keep, |p| Walk::To(p, GO_SPEED));
                }
                *self.tasks.last_mut()? = Task::Migrate { pasture, long_ms, short_ms, started: true };
                at_rest(FireMode::None, walk, self)
            }
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
            // Refit (`M_Task_Reload`, docs/27, "What sends a bot to a dock"): it walks to the
            // dock picked at its start and stands in it until life, charge and ammunition are
            // all at 98%, then leaves. With no dock to go to it fails at once ("No Where to
            // reX..."), and so does one whose dock has gone.
            Task::Reload { dock, walking } => {
                let (id, index) = match dock {
                    Some(key) => key,
                    None => senses.dock().map(|d| (d.id, d.index))?,
                };
                let &found = senses.docks.iter().find(|d| (d.id, d.index) == (id, index))?;
                // It is there once it stands in the place itself: a unit must stand still to
                // be in one (docs/27), so it holds there and the dock's own tick charges it.
                //
                // STAND-IN: docs/27-ownership.md#what-sends-a-bot-to-a-dock--read -- where the
                // walk to a dock is counted over is not read: the place the unit is to stand
                // in, a cylinder 10 across at a ground-level dock and 5 indoors.
                let arrived = found.holds(at);
                if arrived && senses.condition.refitted() {
                    return None;
                }
                let mut walking = walking;
                let mut walk = Walk::Keep;
                if !arrived && (!walking || senses.walker_idle) {
                    // The walk runs along the building's hall way, which reaches an indoor
                    // dock through its doors and a ground-level one from its exits.
                    walk = Walk::Inside(found.id, found.at, GO_SPEED);
                    walking = true;
                } else if arrived && walking {
                    walk = Walk::Clear;
                    walking = false;
                }
                *self.tasks.last_mut()? = Task::Reload { dock: Some((id, index)), walking };
                self.fire = FireMode::Nearest;
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: false })
            }
            Task::Follow { leader, radius, next_ms } => {
                let lead = *senses.find(leader).filter(|s| s.own)?;
                let mut walk = Walk::Keep;
                if senses.walker_idle || now >= next_ms {
                    let across = lead.position.truncate().distance(at.truncate());
                    let up = (lead.position.z - at.z).abs();
                    // The first of 77 spots about the leader the walker takes (`0x1002b059`), a
                    // refused one having emptied its queues.
                    //
                    // STAND-IN: docs/31-packages.md#what-each-package-does--read -- `SetTarget`'s
                    // acceptance is tested by the areal under the spot alone, as a flyer's by
                    // none: a spot the search then finds no way to is refused by the walker.
                    if across > radius + FOLLOW_SLACK || up > radius + FOLLOW_SLACK_UP {
                        walk = Walk::Clear;
                        for _ in 0..FOLLOW_TRIES {
                            let (dx, dy) = (self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0);
                            let spot = lead.position + Vec3::new(dx * radius, dy * radius, FOLLOW_LIFT);
                            if senses.flyer || senses.usable.at(spot) {
                                walk = Walk::To(spot, 1.0);
                                break;
                            }
                        }
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
                    let goal = goal.unwrap_or_else(|| self.roam(senses, at));
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
            // The first point of the rings inside the map by 100 on a usable areal; with none
            // the escape fails ("Cannot Leave").
            Task::Leave { goal: None } => {
                let (lo, hi) = senses.bounds;
                let inside = |p: Vec3| (0..2).all(|a| p[a] > lo[a] + ROAM_INSET && p[a] < hi[a] - ROAM_INSET);
                let mut goal = None;
                'rings: for (tries, reach) in LEAVE_RINGS {
                    for _ in 0..tries {
                        let (dx, dy) = (self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0);
                        let p = at + Vec3::new(dx * reach, dy * reach, 0.0);
                        if inside(p) && senses.usable.at(p) {
                            goal = Some(p);
                            break 'rings;
                        }
                    }
                }
                let goal = goal?;
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
            // The upgrade (`0x100338a0`, docs/32): it walks to a point beside the building and
            // stands there; the play takes it from there and ends the task when it is done. A
            // building gone, lost to another clan or no longer seen ends it ("dead, enemy or
            // fully upgraded building").
            Task::Upgrade { building, state, goal } => {
                let it = *senses.find(building).filter(|s| s.building && s.own)?;
                let (mut state, mut goal) = (state, goal);
                let mut walk = Walk::Keep;
                if state == UpgradeState::Going {
                    let spot = match goal {
                        Some(spot) => spot,
                        None => {
                            let spot = self.beside(&it, senses);
                            walk = Walk::To(spot, GO_SPEED);
                            spot
                        }
                    };
                    goal = Some(spot);
                    if at.truncate().distance(spot.truncate()) <= UPGRADE_ARRIVED {
                        (state, walk) = (UpgradeState::Arrived, Walk::Clear);
                    } else if senses.walker_idle && walk == Walk::Keep {
                        walk = Walk::To(spot, GO_SPEED);
                    }
                }
                *self.tasks.last_mut()? = Task::Upgrade { building, state, goal };
                self.fire = FireMode::Nearest;
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
            Task::Attack { target, fighting, next_ms, limit, ordered } => {
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
                // An animal fires on its target from within 200, and on the nearest hostile contact
                // beyond (`0x10027580`).
                if senses.animal {
                    let near = victim.position.distance(at) <= ANIMAL_FIXED_FIRE;
                    self.fire = if near { FireMode::Fixed(id) } else { FireMode::Nearest };
                }
                *self.tasks.last_mut()? =
                    Task::Attack { target: Some(id), fighting, next_ms: next, limit, ordered };
                Some(Takt { walk, target: self.fire_target(senses), fire_freely: true })
            }
        }
    }

    /// A point of `pasture` to graze at (`0x1002cba0`): its centre plus a random share of the
    /// inner radius along x and along y, each held to at least [`MIGRATE_POINT_FLOOR`], so it
    /// lies in the square off the centre's +x, +y side, at the centre's own height. The first
    /// of [`MIGRATE_TRIES`] the walker takes; none with no pasture.
    ///
    /// STAND-IN: docs/31-packages.md#migrate-an-animals-pasture--read-and-measured -- what makes
    /// the walker take a point (`0x10001960`) is not read: the areal under it, as a flyer's by
    /// none.
    fn graze(&mut self, pasture: Option<Pasture>, senses: &Senses) -> Option<Vec3> {
        let p = pasture?;
        for _ in 0..MIGRATE_TRIES {
            let u = self.random().max(MIGRATE_POINT_FLOOR);
            let v = self.random().max(MIGRATE_POINT_FLOOR);
            let point = p.centre + Vec3::new(u * p.inner, v * p.inner, 0.0);
            if senses.flyer || senses.usable.at(point) {
                return Some(point);
            }
        }
        None
    }

    /// The first of up to 150 random points at least 100 inside the map that lies on a usable
    /// areal (`0x10030ed2`).
    ///
    /// STAND-IN: docs/31-packages.md#where-a-search-looks--read-and-measured -- what a roam
    /// with no usable point does is not read: it takes the last point tried.
    fn roam(&mut self, senses: &Senses, at: Vec3) -> Vec3 {
        let (lo, hi) = senses.bounds;
        let pick = |me: &mut Self, a: usize| {
            let (from, to) = (lo[a] + ROAM_INSET, hi[a] - ROAM_INSET);
            if to > from { from + me.random() * (to - from) } else { (lo[a] + hi[a]) / 2.0 }
        };
        let mut point = at;
        for _ in 0..ROAM_TRIES {
            point = Vec3::new(pick(self, 0), pick(self, 1), at.z);
            if senses.usable.at(point) {
                break;
            }
        }
        point
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
    /// read to lie off the map is not modelled: a plan with nowhere to go roams.
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
        Walk::To(self.roam(senses, at), GO_SPEED)
    }

    /// A point beside building `it` for an upgrade (`0x100338a0`): the first of
    /// [`UPGRADE_TRIES`] on the ring [`UPGRADE_BESIDE`] out past its sphere that stands on
    /// usable ground, and the last tried with none.
    ///
    /// STAND-IN: docs/32-builder.md#upgrading-a-building--read -- how the point beside the
    /// building is drawn is not read: a random one on that ring, as the attack draws its own.
    fn beside(&mut self, it: &Seen, senses: &Senses) -> Vec3 {
        let reach = it.radius + UPGRADE_BESIDE;
        let mut spot = it.position;
        for _ in 0..UPGRADE_TRIES {
            let a = self.random() * std::f32::consts::TAU;
            spot = it.position + Vec3::new(a.cos(), a.sin(), 0.0) * reach;
            if senses.flyer || senses.usable.at(spot) {
                break;
            }
        }
        spot
    }
}

/// A round frame's flags (`+116`) the distance score skips the distance for: bit `0x10` scores
/// 1.1, bit 8 1.0 (`0x1001b9f0`).
pub const SCORE_FLAG_ANYWHERE_HIGH: i32 = 0x10;
pub const SCORE_FLAG_ANYWHERE: i32 = 0x8;
pub const SCORE_ANYWHERE_HIGH: f32 = 1.1;
/// The ramp a score rises over from 0: 5 m, and none for an animal (`+0x44`, `0x1001b5e0`).
pub const SCORE_RAMP: f32 = 5.0;

/// The fight module's distance score (`0x1001b9f0`): for a round whose frame flags carry bit
/// `0x10`, 1.1, and bit 8, 1.0, wherever the target stands; otherwise 0 to 1 over the first
/// `ramp` m, 1 out to (v + 1) ÷ 2, 0 at 2 (v + 1), times 1 − height ÷ v.
pub fn distance_score(distance: f32, height: f32, round_speed: f32, flags: i32, ramp: f32) -> f32 {
    if flags & SCORE_FLAG_ANYWHERE_HIGH != 0 {
        return SCORE_ANYWHERE_HIGH;
    }
    if flags & SCORE_FLAG_ANYWHERE != 0 {
        return 1.0;
    }
    let v = round_speed.max(1e-3);
    let (hold, gone) = ((v + 1.0) / 2.0, 2.0 * (v + 1.0));
    let score = if distance < ramp {
        distance / ramp
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
            sensed: true,
        }
    }

    fn senses<'a>(seen: &'a [Seen], now_ms: f64, at: Vec3, idle: bool) -> Senses<'a> {
        Senses {
            now_ms,
            position: at,
            seen,
            places: &[],
            docks: &[],
            condition: Condition::default(),
            size_class: 2,
            flyer: false,
            bounds: ([0.0; 2], [2000.0; 2]),
            usable: Usable::ANYWHERE,
            has_weapon: true,
            walker_idle: idle,
            neutral: false,
            building: false,
            animal: false,
            pastures: Pastures::NONE,
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

        // An animal of a clan with no pasture migrates on the spot: nothing scores, and it
        // asks its fire control for nothing.
        let mut medusa = Behaviour::new(3);
        let t = medusa.takt(&Senses { animal: true, ..senses(&[enemy], 0.0, Vec3::ZERO, true) });
        assert!(
            matches!(medusa.task(), Task::Migrate { pasture: None, started: true, .. }),
            "{:?}",
            medusa.task()
        );
        assert_eq!((t.walk, t.target), (Walk::Keep, None));
    }

    #[test]
    fn a_grazing_medusa_hit_from_off_its_pasture_attacks_the_firer_for_20_s_within_outer_plus_20() {
        // Mission 02's western pasture: outer 50, inner 20. The hero fires from 150 m off it, far
        // past the radar's reach of the test's world, which retaliation does not ask.
        let pasture = Pasture { centre: Vec3::new(0.0, 0.0, 0.0), inner: 20.0, outer: 50.0 };
        let ask = || Some(pasture);
        let hero = Seen { sensed: false, ..unit(11, 200.0, 0.0) };
        let medusa = |at: Vec3, idle: bool| Senses {
            animal: true,
            flyer: true,
            pastures: Pastures(&ask),
            ..senses(std::slice::from_ref(&hero), 1000.0, at, idle)
        };
        let mut b = Behaviour::new(5);
        b.takt(&medusa(Vec3::new(8.0, 0.0, 11.0), true));
        assert!(matches!(b.task(), Task::Migrate { started: true, .. }), "grazing, it takes up nothing");
        b.hurt(11);
        let t = b.takt(&medusa(Vec3::new(8.0, 0.0, 11.0), false));
        let Task::Attack { target: Some(11), limit: Some(limit), ordered: false, .. } = b.task() else {
            panic!("{:?}", b.task())
        };
        assert_eq!(limit.radius, 70.0);
        assert_eq!(limit.until_ms, Some(21_000.0));
        assert!(matches!(t.walk, Walk::To(..)), "it goes for the firer");
        // An attacking animal takes no other interrupt.
        let other = Seen { sensed: false, ..unit(12, 30.0, 0.0) };
        b.hurt(12);
        b.takt(&Senses { seen: &[hero, other], now_ms: 9000.0, ..medusa(Vec3::new(20.0, 0.0, 11.0), false) });
        assert!(matches!(b.task(), Task::Attack { target: Some(11), .. }), "{:?}", b.task());
        // Past 70 m of the pasture's centre the attack is dropped, and migrate starts again: it
        // asks for its pasture and walks back onto it.
        b.takt(&Senses { now_ms: 9500.0, ..medusa(Vec3::new(75.0, 0.0, 11.0), false) });
        assert!(matches!(b.task(), Task::Migrate { started: false, .. }), "{:?}", b.task());
        let t = b.takt(&Senses { now_ms: 9600.0, ..medusa(Vec3::new(75.0, 0.0, 11.0), false) });
        assert!(matches!(t.walk, Walk::To(p, GO_SPEED) if p.x <= 20.0 && p.y <= 20.0), "{t:?}");

        // Hit from inside the inner circle: 35 s, outer + 100.
        let near = Seen { sensed: false, ..unit(11, 10.0, 0.0) };
        let grazing = || {
            let mut b = Behaviour::new(5);
            b.takt(&Senses { now_ms: 0.0, ..medusa(Vec3::new(8.0, 0.0, 11.0), true) });
            b
        };
        let mut b = grazing();
        b.hurt(11);
        b.takt(&Senses { seen: std::slice::from_ref(&near), ..medusa(Vec3::new(8.0, 0.0, 11.0), false) });
        assert!(matches!(
            b.task(),
            Task::Attack { limit: Some(Limit { radius: 150.0, until_ms: Some(36_000.0), .. }), .. }
        ));

        // Standing off its pasture, it goes for the firer for 10 s, held by no circle.
        let mut b = grazing();
        b.hurt(11);
        b.takt(&Senses { seen: std::slice::from_ref(&near), ..medusa(Vec3::new(60.0, 0.0, 11.0), false) });
        assert!(matches!(
            b.task(),
            Task::Attack { limit: Some(Limit { radius: 0.0, until_ms: Some(11_000.0), .. }), .. }
        ));

        // With no pasture it answers a hit as a stopped unit does: 1000 about where it stands.
        let mut b = Behaviour::new(5);
        b.hurt(11);
        b.takt(&Senses {
            pastures: Pastures::NONE,
            seen: std::slice::from_ref(&near),
            ..medusa(Vec3::new(8.0, 0.0, 11.0), false)
        });
        assert!(matches!(
            b.task(),
            Task::Attack { limit: Some(Limit { radius: 1000.0, until_ms: None, .. }), .. }
        ));
    }

    /// Migrate (docs/31, "Migrate: an animal's pasture"): Mission 02's western pasture, centre
    /// (915.1, 990.0) at 198.8, inner 20 and outer 50.
    #[test]
    fn a_grazing_medusa_walks_its_pasture_point_to_point_and_asks_for_it_again_on_its_long_timer() {
        let pasture = Pasture { centre: Vec3::new(915.1, 990.0, 198.8), inner: 20.0, outer: 50.0 };
        let asked = std::cell::Cell::new(0);
        let ask = || {
            asked.set(asked.get() + 1);
            Some(pasture)
        };
        let medusa = |now: f64, idle: bool| Senses {
            animal: true,
            flyer: true,
            pastures: Pastures(&ask),
            ..senses(&[], now, Vec3::new(924.3, 987.4, 215.0), idle)
        };
        let on_pasture = |walk: Walk| match walk {
            Walk::To(p, share) => {
                let (dx, dy) = (p.x - pasture.centre.x, p.y - pasture.centre.y);
                share == GO_SPEED
                    && (4.0..=20.0).contains(&dx)
                    && (4.0..=20.0).contains(&dy)
                    && p.z == pasture.centre.z
            }
            _ => false,
        };
        let mut b = Behaviour::new(9);
        // Its start: the pasture asked, a point in the square off the centre's +x, +y side at
        // 0.2 to 1 inner radius, at the centre's height, and no target.
        let t = b.takt(&medusa(0.0, true));
        assert!(on_pasture(t.walk), "{t:?}");
        assert_eq!((t.target, asked.get()), (None, 1));
        let Task::Migrate { long_ms, .. } = b.task() else { panic!("{:?}", b.task()) };
        assert!((60_000.0..=180_000.0).contains(&long_ms), "{long_ms}");
        // On its way, the short timer keeps starting again; arrived, it stands 5 to 15 s.
        assert_eq!(b.takt(&medusa(1000.0, false)).walk, Walk::Keep);
        let Task::Migrate { short_ms, .. } = b.task() else { panic!() };
        assert!((6000.0..=16_000.0).contains(&short_ms), "{short_ms}");
        assert_eq!(b.takt(&medusa(2000.0, true)).walk, Walk::Keep, "standing");
        let t = b.takt(&medusa(16_500.0, true));
        assert!(on_pasture(t.walk), "{t:?}");
        assert_eq!(asked.get(), 1, "the pasture is asked for only at the start and on the long timer");
        // Past its long timer it asks again, whatever the walker is doing.
        let t = b.takt(&medusa(180_500.0, false));
        assert!(on_pasture(t.walk), "{t:?}");
        assert_eq!(asked.get(), 2);
    }

    #[test]
    fn a_migrating_medusa_takes_up_a_contact_inside_its_inner_circle_while_it_stands_inside_the_outer() {
        let pasture = Pasture { centre: Vec3::ZERO, inner: 20.0, outer: 50.0 };
        let ask = || Some(pasture);
        let at = |x: f32| Vec3::new(x, 0.0, 15.0);
        let medusa = |seen: &[Seen], now: f64, x: f32| {
            let mut b = Behaviour::new(4);
            b.takt(&Senses {
                animal: true,
                flyer: true,
                pastures: Pastures(&ask),
                ..senses(&[], 0.0, at(x), false)
            });
            b.takt(&Senses {
                animal: true,
                flyer: true,
                pastures: Pastures(&ask),
                ..senses(seen, now, at(x), false)
            });
            b.task()
        };
        let hero = |x: f32| Seen { hostile: true, ..unit(1, x, 0.0) };
        // Inside the inner circle while the medusa grazes the pasture: 20 s within outer + 80.
        assert!(matches!(
            medusa(&[hero(-10.0)], 3000.0, 30.0),
            Task::Attack {
                target: Some(1),
                limit: Some(Limit { radius: 130.0, until_ms: Some(23_000.0), .. }),
                ..
            }
        ));
        // On the rest of the pasture, or with the medusa off it, nothing is taken up.
        assert!(matches!(medusa(&[hero(-25.0)], 3000.0, 30.0), Task::Migrate { .. }));
        assert!(matches!(medusa(&[hero(-10.0)], 3000.0, 55.0), Task::Migrate { .. }));
        // Of two inside, the one nearer the centre scores more.
        let near = Seen { hostile: true, ..unit(2, 5.0, 0.0) };
        assert!(matches!(medusa(&[hero(-15.0), near], 3000.0, 30.0), Task::Attack { target: Some(2), .. }));
    }

    #[test]
    fn a_clan_grazes_one_pasture_until_its_timer_runs_out_and_then_picks_again() {
        let mut clan = ClanPasture::default();
        assert_eq!(clan.ask(0, 0.0, || 0.5), None, "a clan with no zones has no pasture");
        // The first question picks at once; the timer is 59.968 s and up to 120 more.
        assert_eq!(clan.ask(2, 1000.0, || 0.5), Some(1));
        assert_eq!(clan.until_ms, 1000.0 + 59_968.0 + 60_000.0);
        assert_eq!(clan.ask(2, 100_000.0, || 0.25), Some(1), "the same until the timer runs out");
        assert_eq!(clan.ask(2, 121_000.0, || 0.25), Some(0), "then a pick at random");
        assert_eq!(clan.until_ms, 121_000.0 + 59_968.0 + 30_000.0);
    }

    #[test]
    fn a_hit_pulls_a_patrol_into_an_attack_on_a_firer_past_its_radar_and_radius_once_the_gate_pauses() {
        let firer = Seen { sensed: false, ..unit(11, 600.0, 0.0) };
        let mut b = Behaviour::new(2);
        b.order(&place_patrol(0.0, 0.0));
        b.takt(&senses(std::slice::from_ref(&firer), 0.0, Vec3::ZERO, true));
        assert!(matches!(b.task(), Task::Patrol { .. }), "not hostile and not on its radar: no engagement");
        b.hurt(11);
        b.takt(&senses(std::slice::from_ref(&firer), 100.0, Vec3::ZERO, true));
        assert!(
            matches!(
                b.task(),
                Task::Attack { target: Some(11), limit: Some(Limit { radius: 120.0, .. }), .. }
            ),
            "{:?}",
            b.task()
        );
        // A friend's round is not answered, and nor is anything while the gate pauses.
        let mut b = Behaviour::new(2);
        let own = Seen { own: true, ..firer };
        b.hurt(11);
        b.takt(&senses(std::slice::from_ref(&own), 0.0, Vec3::ZERO, true));
        assert_eq!(b.task(), Task::Stop);
        b.hurt(11);
        b.takt(&senses(std::slice::from_ref(&firer), 1000.0, Vec3::ZERO, true));
        assert_eq!(b.task(), Task::Stop, "the gate paused at the first hit");
        b.hurt(11);
        b.takt(&senses(std::slice::from_ref(&firer), 5000.0, Vec3::ZERO, true));
        assert!(matches!(b.task(), Task::Attack { target: Some(11), .. }));
        // A standing-by or shut-down unit answers nothing.
        let mut b = Behaviour::new(2);
        b.order(&Order { code: orders::STAYGROUND, parameter: 0, target: Target::NotDefined });
        b.hurt(11);
        b.takt(&senses(std::slice::from_ref(&firer), 0.0, Vec3::ZERO, true));
        assert_eq!(b.task(), Task::StayGround);
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
        // It takes the first spot on usable ground, and with none of its 77 it holds.
        let east = |x: f32, _: f32| x > 110.0;
        let t = b.takt(&Senses {
            usable: Usable(&east),
            ..senses(&[leader], 3000.0, Vec3::new(200.0, 100.0, 0.0), true)
        });
        assert!(matches!(t.walk, Walk::To(spot, _) if spot.x > 110.0), "{t:?}");
        let nowhere = |_: f32, _: f32| false;
        let t = b.takt(&Senses {
            usable: Usable(&nowhere),
            ..senses(&[leader], 4000.0, Vec3::new(200.0, 100.0, 0.0), true)
        });
        assert_eq!(t.walk, Walk::Clear);
        let flies = b.takt(&Senses {
            usable: Usable(&nowhere),
            flyer: true,
            ..senses(&[leader], 5000.0, Vec3::new(200.0, 100.0, 0.0), true)
        });
        assert!(matches!(flies.walk, Walk::To(..)), "a flyer's spot needs no walkable areal");
        let gone = Seen { own: false, ..leader };
        b.takt(&senses(&[gone], 6000.0, Vec3::ZERO, true));
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
        assert_eq!(refit.task(), Task::Stop, "with no dock a refit fails at its start");
    }

    #[test]
    fn a_refit_walks_to_the_nearest_dock_its_size_fits_and_ends_once_it_is_charged() {
        let dock = |id: i32, index: usize, x: f32, ground_level: bool| Dock {
            id,
            index,
            at: Vec3::new(x, 0.0, 0.0),
            ground_level,
            radius: if ground_level { 10.0 } else { 5.0 },
            height: if ground_level { 12.0 } else { 3.0 },
            below: if ground_level { 8.4 } else { 2.1 },
        };
        // A bunker's indoor dock at 100, and the Outpost's ground-level one at 300.
        let docks = [dock(-1, 0, 100.0, false), dock(-2, 0, 300.0, true)];
        let hurt = Condition { life: 0.4, charge: 0.5, ammo: 0.5, guns_dry: false };
        let refitting = |x: f32, size: u8, idle: bool, condition: Condition| Senses {
            docks: &docks,
            condition,
            size_class: size,
            ..senses(&[], 0.0, Vec3::new(x, 0.0, 0.0), idle)
        };
        let order = Order { code: orders::RELOAD, parameter: 0, target: Target::NotDefined };

        // A small unit takes the nearer dock, indoors, and walks in along the hall way.
        let mut small = Behaviour::new(5);
        small.order(&order);
        let t = small.takt(&refitting(0.0, 2, true, hurt));
        assert_eq!(t.walk, Walk::Inside(-1, docks[0].at, GO_SPEED));
        assert!(matches!(small.task(), Task::Reload { dock: Some((-1, 0)), walking: true }));
        // On the dock it holds still, so the dock's own tick may charge it.
        assert_eq!(small.takt(&refitting(100.0, 2, true, hurt)).walk, Walk::Clear);
        assert_eq!(small.takt(&refitting(100.0, 2, true, hurt)).walk, Walk::Keep);
        // Charged, it leaves: the task ends and the one beneath it runs.
        small.takt(&refitting(100.0, 2, true, Condition::default()));
        assert_eq!(small.task(), Task::Stop);

        // A medium or large unit fits through no door: only the ground-level dock serves it.
        for size in [3, 4] {
            let mut big = Behaviour::new(7);
            big.order(&order);
            let t = big.takt(&refitting(0.0, size, true, hurt));
            assert_eq!(t.walk, Walk::Inside(-2, docks[1].at, GO_SPEED), "size {size}");
            assert!(matches!(big.task(), Task::Reload { dock: Some((-2, 0)), .. }));
        }
        // With none it fits, a refit fails at its start.
        let indoor = [docks[0]];
        let mut big = Behaviour::new(7);
        big.order(&order);
        big.takt(&Senses {
            docks: &indoor,
            condition: hurt,
            size_class: 4,
            ..senses(&[], 0.0, Vec3::ZERO, true)
        });
        assert_eq!(big.task(), Task::Stop);
        // A dock that has gone ends the trip.
        let mut small = Behaviour::new(5);
        small.order(&order);
        small.takt(&refitting(0.0, 2, true, hurt));
        small.takt(&Senses { docks: &[], condition: hurt, ..senses(&[], 0.0, Vec3::ZERO, true) });
        assert_eq!(small.task(), Task::Stop);
    }

    #[test]
    fn a_unit_that_needs_service_sends_itself_to_a_dock_and_repairs_itself_while_it_is_scratched() {
        let docks = [Dock {
            id: -1,
            index: 0,
            at: Vec3::new(100.0, 0.0, 0.0),
            ground_level: true,
            radius: 10.0,
            height: 12.0,
            below: 8.4,
        }];
        let of = |condition: Condition| Senses {
            docks: &docks,
            condition,
            ..senses(&[], 0.0, Vec3::new(0.0, 0.0, 0.0), true)
        };
        let scratched = Condition { life: 0.7, ..Condition::default() };

        // Only scratched: it switches its own repair system on and stays on its order.
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::STAYGROUND, parameter: 0, target: Target::NotDefined });
        b.takt(&of(scratched));
        assert_eq!(b.task(), Task::StayGround);
        assert!(b.repair, "the repair decision switches it on under 0.8");
        // Repaired past 0.9 it switches off again, and a flat battery switches it off whatever.
        b.takt(&of(Condition { life: 0.95, ..Condition::default() }));
        assert!(!b.repair);
        b.takt(&of(Condition { life: 0.7, charge: 0.05, ..Condition::default() }));
        assert!(!b.repair, "under a tenth of a battery nothing is repaired");

        // Under half its life it sends itself to a dock, and standby is not interrupted.
        let hurt = Condition { life: 0.4, ..Condition::default() };
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::STAYGROUND, parameter: 0, target: Target::NotDefined });
        b.takt(&of(hurt));
        assert_eq!(b.task(), Task::StayGround, "standby answers 0 for a refit");
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::GO, parameter: 0, target: Target::Place([500.0, 0.0, 0.0]) });
        b.takt(&of(hurt));
        assert!(matches!(b.task(), Task::Reload { .. }), "a route lets a refit through: {:?}", b.task());
        // Guns mostly dry send it too, and starting the trip turns its repair system off.
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::SEARCH, parameter: 0, target: Target::Any });
        b.takt(&of(Condition { guns_dry: true, ..Condition::default() }));
        assert!(matches!(b.task(), Task::Reload { .. }));
        assert!(!b.repair);
        // With no dock in the world it stays on its order.
        let mut b = Behaviour::new(3);
        b.order(&Order { code: orders::SEARCH, parameter: 0, target: Target::Any });
        b.takt(&Senses { condition: hurt, ..senses(&[], 0.0, Vec3::ZERO, true) });
        assert!(matches!(b.task(), Task::Search { .. }));
    }

    #[test]
    fn roaming_patrol_and_escape_points_lie_on_usable_areals_but_a_flyers_patrol_points_need_not() {
        // Only the strip x < 300 is usable.
        let west = |x: f32, _: f32| x < 300.0;
        let usable =
            Senses { usable: Usable(&west), ..senses(&[], 20_000.0, Vec3::new(1000.0, 1000.0, 0.0), true) };
        let mut seek = Behaviour::new(11);
        seek.order(&Order { code: orders::SEARCH, parameter: 0, target: Target::Any });
        let Walk::To(p, _) = seek.takt(&usable).walk else { panic!() };
        assert!(p.x < 300.0 && p.x > 100.0, "roams onto usable ground: {p}");

        let mut walker = Behaviour::new(21);
        walker.order(&place_patrol(300.0, 1000.0));
        walker.takt(&Senses { now_ms: 0.0, ..usable });
        assert!(walker.patrol_loop.iter().all(|p| p.x < 300.0), "{:?}", walker.patrol_loop);
        let mut flyer = Behaviour::new(21);
        flyer.order(&place_patrol(300.0, 1000.0));
        flyer.takt(&Senses { now_ms: 0.0, flyer: true, ..usable });
        assert!(flyer.patrol_loop.iter().any(|p| p.x >= 300.0), "{:?}", flyer.patrol_loop);

        // The escape looks out to 1,000 for usable ground, and fails with none.
        let mut leave = Behaviour::new(4);
        leave.order(&Order { code: orders::LEAVE, parameter: 0, target: Target::NotDefined });
        let Walk::To(p, _) = leave.takt(&usable).walk else { panic!() };
        assert!(p.x < 300.0 && (p.x - 1000.0).abs() <= 1000.0, "{p}");
        let nowhere = |_: f32, _: f32| false;
        let mut stuck = Behaviour::new(4);
        stuck.order(&Order { code: orders::LEAVE, parameter: 0, target: Target::NotDefined });
        stuck.takt(&Senses { usable: Usable(&nowhere), ..usable });
        assert_eq!(stuck.task(), Task::Stop, "Cannot Leave");
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
        assert_eq!(distance_score(2.5, 0.0, 350.0, 4, SCORE_RAMP), 0.5);
        assert_eq!(distance_score(2.5, 0.0, 350.0, 4, 0.0), 1.0, "an animal's has no ramp");
        assert_eq!(distance_score(175.0, 0.0, 350.0, 4, SCORE_RAMP), 1.0);
        assert!((distance_score(438.75, 0.0, 350.0, 4, SCORE_RAMP) - 0.5).abs() < 1e-3);
        assert_eq!(distance_score(702.0, 0.0, 350.0, 4, SCORE_RAMP), 0.0);
        // The Small Bunker's lobbed `bf_f_01` (flags 12) and a missile (16) score at any range.
        assert_eq!(distance_score(180.0, 10.0, 45.0, 12, SCORE_RAMP), 1.0);
        assert_eq!(distance_score(900.0, 0.0, 80.0, 16, SCORE_RAMP), 1.1);
        assert_eq!(fire_wait_ms(-1, 0.0), 500.0);
        assert_eq!(fire_wait_ms(10, 1.0), 6000.0);
    }
}
