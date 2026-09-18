//! Mission progression: the routes a script tests as trigger areas, who stands in
//! them, how many robots each clan has, when the player clan's `Mission` handler
//! runs, and what its message and objective calls do. See `docs/34-progression.md`.
//!
//! The script itself runs elsewhere; this answers its functions 30, 31, 32, 34 and 52,
//! finds function 15's unit, and keeps the state they read and write.

use std::collections::BTreeMap;

use glam::Vec3;
use parkan_formats::mission::Route;

/// `ai.dll:0x10001b80`: once the clock reaches the next-run time the `Mission` handler
/// runs, and the next-run time is set 2000 ms on.
pub const MISSION_PERIOD_MS: f64 = 2000.0;
/// The clan's takt, SuperAI slot 3 (`ai.dll:0x10001780`), which the game frame runs for every
/// clan: it comes 7000 + rand % 1000 ms after the last, adds 7 to the seconds clock function 59
/// reads (`+0x854`), and then runs the clan's problem handler. See
/// `docs/34-progression.md`, "When the Mission handler runs".
pub const CLAN_TAKT_MS: f64 = 7000.0;
pub const CLAN_TAKT_RANDOM_MS: u32 = 1000;
pub const CLAN_TAKT_SECONDS: u32 = 7;
/// A unit reports its position once it has moved more than this by |dx| + |dy|
/// (`Behavior.dll:0x1000af70`).
pub const REPORT_MOVE: f32 = 5.0;
/// The object takt's timer, next = now + 31 × 64 + rand8 × 46 × 64 ÷ 256 ms, the share
/// truncated (the words at `Behavior.dll:0x100039c2`, the timer `0x1004c550`, whose stored
/// words count 64 ms each): 1984 to 4916 ms.
pub const TAKT_BASE: u32 = 31 * 64;
pub const TAKT_SPREAD: u32 = 46 * 64;

/// The wait the timer sets from a `rand8`, as the game works it out.
fn takt_wait(rand8: u8) -> f64 {
    f64::from(TAKT_BASE + u32::from(rand8) * TAKT_SPREAD / 256)
}

/// The first value a script hands function 30: the channel-0 cases of the mission
/// callback (`iron3d.dll:0x10060ce0`), named by `varset.var`'s `Messages` constants.
pub const SYSTEM_MESSAGE: i64 = 0;
pub const MESSAGE_INFO: i64 = 1;
pub const CLAN_HERO_KILLED: i64 = 2;
pub const OBJECTIVE_COMPLETE: i64 = 3;
pub const OBJECTIVE_FAILED: i64 = 4;
pub const OBJECTIVE_PROGRESS: i64 = 5;
/// `SYSTEM_MESSAGE`'s two outcomes.
pub const MISSION_FAILED: i64 = 0;
pub const MISSION_COMPLETE: i64 = 1;
/// `varset.var`'s `ERROR`: what a function answers when it has none.
pub const ERROR: u32 = 0xffff_ffff;
/// Function 52's answers besides a clan's index: a destroyed object's owner word
/// (`Behavior.dll:0x1000698d`), and `ERROR` when no object answers the id
/// (`ai.dll:0x1000e153`).
pub const DESTROYED_OWNER: u32 = 0xfffe;
pub const NO_OBJECT: u32 = ERROR;

/// Whether a route's outline holds the point (x, y): the crossing test
/// `ArealMap.dll:0x10017b90` makes. A ray from the point toward +x crosses the outline,
/// closed from its last point to its first, an odd number of times. An edge counts when
/// one end is at or above y and the other below, and it meets the ray at or right of x.
pub fn contains(outline: &[[f32; 3]], x: f32, y: f32) -> bool {
    let Some(last) = outline.last() else { return false };
    let mut inside = false;
    let (mut px, mut py) = (last[0], last[1]);
    for p in outline {
        let (cx, cy) = (p[0], p[1]);
        let right =
            (cx >= x && px >= x) || ((cx >= x) != (px >= x) && cx + (px - cx) * (y - cy) / (py - cy) >= x);
        if (cy >= y) != (py >= y) && right {
            inside = !inside;
        }
        (px, py) = (cx, cy);
    }
    inside
}

/// The system areal map's tactical areals: one per route, areal i taking the points of
/// the route whose id is i (`MisLoad.dll:0x10001380`), each with the logical ids last
/// reported inside it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Areals {
    pub outlines: Vec<Vec<[f32; 3]>>,
    pub lists: Vec<Vec<i32>>,
}

impl Areals {
    pub fn new(routes: &[Route]) -> Self {
        let outlines: Vec<Vec<[f32; 3]>> = (0..routes.len())
            .map(|i| routes.iter().find(|r| r.id as usize == i).map_or_else(Vec::new, |r| r.points.clone()))
            .collect();
        let lists = vec![Vec::new(); outlines.len()];
        Self { outlines, lists }
    }

    /// Slot 18's report (`ArealMap.dll:0x1001fb40`): `id` leaves every areal's list, then
    /// joins the list of each areal whose outline holds it.
    pub fn report(&mut self, id: i32, at: Vec3) {
        self.leave(id);
        for (outline, list) in self.outlines.iter().zip(&mut self.lists) {
            if contains(outline, at.x, at.y) {
                list.push(id);
            }
        }
    }

    pub fn leave(&mut self, id: i32) {
        for list in &mut self.lists {
            list.retain(|&i| i != id);
        }
    }

    /// Function 32 (`ai.dll:0x1000c446`): whether `id` is on areal `areal`'s list.
    pub fn holds(&self, areal: i64, id: i64) -> bool {
        usize::try_from(areal)
            .ok()
            .and_then(|a| self.lists.get(a))
            .is_some_and(|l| l.iter().any(|&i| i64::from(i) == id))
    }
}

/// A unit with a logical id: its clan, its `Type` word, and its position reports.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub id: i32,
    pub clan: i64,
    pub type_word: u32,
    pub alive: bool,
    /// Where it last reported, and when its takt next runs, ms.
    reported: [f32; 2],
    next_takt_ms: f64,
}

/// A placed object with a logical id that is not a unit: a building, whose owner function
/// 52 reads. It never reports, so where it was placed is where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Building {
    pub id: i32,
    pub clan: i64,
    pub type_word: u32,
    pub alive: bool,
    pub at: [f32; 2],
}

/// A clan's SuperAI takt, slot 3 (`ai.dll:0x10001780`): when it next runs, and the seconds
/// clock function 59 reads and 60 tests.
#[derive(Clone, Debug, PartialEq)]
pub struct ClanTakt {
    next_ms: f64,
    clock_s: u32,
    seed: u32,
}

impl ClanTakt {
    /// A clan's takt, its clock starting at `start_s`. The SuperAI's constructor sets the clock
    /// from `timeGetTime` over 1000, so where it starts is arbitrary: only the difference
    /// between a time function 59 wrote and the clock counts.
    pub fn new(start_s: u32, seed: u32) -> Self {
        Self { next_ms: 0.0, clock_s: start_s, seed: seed | 1 }
    }

    /// The seconds clock (`+0x854`).
    pub fn clock(&self) -> u32 {
        self.clock_s
    }

    /// Whether the takt is due at `now_ms`, as the game frame asks every clan. A takt that runs
    /// steps the clock 7 s on and sets the next 7000 + rand % 1000 ms away; the caller then runs
    /// the clan's problem handler.
    ///
    /// STAND-IN: docs/34-progression.md#when-the-mission-handler-runs--read -- the takt's clock
    /// is taken as game milliseconds rather than `timeGetTime`, and its `rand` as the engine's
    /// own generator.
    pub fn due(&mut self, now_ms: f64) -> bool {
        if self.next_ms > now_ms {
            return false;
        }
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let wait = f64::from((self.seed >> 16) % CLAN_TAKT_RANDOM_MS);
        self.next_ms = now_ms + CLAN_TAKT_MS + wait;
        self.clock_s = self.clock_s.wrapping_add(CLAN_TAKT_SECONDS);
        true
    }
}

/// One objective: whether the completion test passes over it, and its state (1 complete).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Objective {
    pub exempt: bool,
    pub state: u8,
}

/// What a script's call to function 30 shows or plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Notice {
    /// `MESSAGE_INFO` (`iron3d.dll:0x100952d0`): the first time, the message's text goes
    /// into the history and its voice to the queue; any later time, only the history's
    /// "already in history" lines (strings 6170 and 6223).
    Message { id: i64, first: bool },
    /// An open objective completed: string 5040 and `VOICE_OBJ_COMPLETE`.
    ObjectiveComplete { index: usize },
    /// String 5041.
    ObjectiveFailed { index: usize },
    /// `VOICE_MISSION_COMPLETE`, and the outcome recorded.
    MissionComplete,
    /// `VOICE_MISSION_FAIL`, and the other outcome.
    MissionFailed,
}

/// The progression state a mission's player script reads and writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub areals: Areals,
    pub units: Vec<Unit>,
    pub buildings: Vec<Building>,
    pub objectives: Vec<Objective>,
    /// Each `messages.cfg` id, and whether it has played.
    pub played: BTreeMap<i64, bool>,
    /// The outcome once one is recorded: true won.
    pub outcome: Option<bool>,
    next_mission_ms: f64,
    seed: u32,
}

impl Progress {
    /// The progression of a mission with `routes`, the objectives' exempt flags in script
    /// order, and `messages.cfg`'s ids. Nothing stands in a route until units join.
    pub fn new(routes: &[Route], exempt: &[bool], messages: impl IntoIterator<Item = i64>) -> Self {
        Self {
            areals: Areals::new(routes),
            units: Vec::new(),
            buildings: Vec::new(),
            objectives: exempt.iter().map(|&exempt| Objective { exempt, state: 0 }).collect(),
            played: messages.into_iter().map(|id| (id, false)).collect(),
            outcome: None,
            next_mission_ms: 0.0,
            seed: 1,
        }
    }

    /// The object takt's `rand8`: the low byte of `Behavior.dll`'s own `rand()`
    /// (`0x1004ce3c`), the CRT's linear generator. Its state (`0x10063c1c`) is 1 in the
    /// file and nothing else writes it, so no `srand` ever moves it: one deterministic
    /// stream, shared by every timer in that module.
    fn rand8(&mut self) -> u8 {
        self.seed = self.seed.wrapping_mul(0x0003_43fd).wrapping_add(0x0026_9ec3);
        (self.seed >> 16) as u8
    }

    /// A unit joins its clan's list, and reports once as its id is attached
    /// (`Behavior.dll:0x10005f91`, `iron3d.dll:0x10077511`). Its takt's next-run word starts
    /// at 0 (`0x100039bc`) and the timer fires on a 0 (`0x1004c550`), so the first object
    /// takt it is given runs, and only the timers after that are spaced.
    pub fn join(&mut self, id: i32, clan: i64, type_word: u32, at: Vec3, now_ms: f64) {
        let _ = now_ms;
        self.areals.report(id, at);
        self.units.push(Unit { id, clan, type_word, alive: true, reported: [at.x, at.y], next_takt_ms: 0.0 });
    }

    /// Run every unit's object takt that is due by `now_ms`, with `position` giving where
    /// a unit stands. A unit that has moved more than 5 by |dx| + |dy| since its last
    /// report reports again (`Behavior.dll:0x1000af70`).
    ///
    /// STAND-IN: docs/34-progression.md#who-stands-in-a-route--read -- a report also goes
    /// when the unit's navigation areal changes; the engine keeps no navigation areals, so
    /// only the move sends one.
    pub fn takt(&mut self, now_ms: f64, position: impl Fn(i32) -> Option<Vec3>) {
        for i in 0..self.units.len() {
            if !self.units[i].alive || self.units[i].next_takt_ms > now_ms {
                continue;
            }
            let wait = takt_wait(self.rand8());
            let u = &mut self.units[i];
            u.next_takt_ms = now_ms + wait;
            let Some(at) = position(u.id) else { continue };
            if (at.x - u.reported[0]).abs() + (at.y - u.reported[1]).abs() > REPORT_MOVE {
                u.reported = [at.x, at.y];
                let id = u.id;
                self.areals.report(id, at);
            }
        }
    }

    /// Whether the `Mission` handler runs now (`ai.dll:0x10001b80`); if it does, the next
    /// run is 2000 ms on.
    ///
    /// STAND-IN: docs/34-progression.md#when-the-mission-handler-runs--read -- the game
    /// times it by `timeGetTime`, real time; the engine by game time.
    pub fn mission_due(&mut self, now_ms: f64) -> bool {
        if self.next_mission_ms > now_ms {
            return false;
        }
        self.next_mission_ms = now_ms + MISSION_PERIOD_MS;
        true
    }

    /// Function 31 (`ai.dll:0x1000c2fd`): how many of clan `clan`'s units and buildings have
    /// a type sharing a bit with `mask`.
    ///
    /// It walks the clan's SuperAI's own list (`+0x8c`), the one function 34 counts by type,
    /// where slot 4's event 2 files a building beside the units (`0x10001880`), so
    /// `CLASS_BUILDING` counts a clan's buildings: four campaign missions end a bonus
    /// objective, the enemy base captured or destroyed, on that count reaching 0.
    pub fn robots(&self, clan: i64, mask: i64) -> usize {
        let counted =
            |c: i64, type_word: u32, alive: bool| alive && c == clan && i64::from(type_word) & mask != 0;
        let units = self.units.iter().filter(|u| counted(u.clan, u.type_word, u.alive)).count();
        let buildings = self.buildings.iter().filter(|b| counted(b.clan, b.type_word, b.alive)).count();
        units + buildings
    }

    /// A building with a logical id, owned by clan `clan`, of `type_word`, standing at `at`. A
    /// building is on its clan's SuperAI list too (`ai.dll:0x10001880`, docs/34, "Mission 03").
    pub fn place_building(&mut self, id: i32, clan: i64, type_word: u32, at: Vec3) {
        self.buildings.push(Building { id, clan, type_word, alive: true, at: [at.x, at.y] });
    }

    /// Function 34 (`ai.dll:0x10009c30`): how many entries of clan `clan`'s own list, units
    /// and buildings, have exactly the type `type_word` and a logical id.
    pub fn count_type(&self, clan: i64, type_word: u32) -> usize {
        let units =
            self.units.iter().filter(|u| u.alive && u.clan == clan && u.type_word == type_word).count();
        let buildings =
            self.buildings.iter().filter(|b| b.alive && b.clan == clan && b.type_word == type_word).count();
        units + buildings
    }

    /// Every object the areal map holds, alive: its logical id, its clan, its type word and
    /// where it last reported (a building, where it was placed).
    pub fn objects(&self) -> impl Iterator<Item = (i32, i64, u32, [f32; 2])> + '_ {
        let units = self.units.iter().filter(|u| u.alive).map(|u| (u.id, u.clan, u.type_word, u.reported));
        let buildings = self.buildings.iter().filter(|b| b.alive).map(|b| (b.id, b.clan, b.type_word, b.at));
        units.chain(buildings)
    }

    /// The strength standing within `radius` of (x, y): `ai.dll:0x10006130` sums it over the
    /// objects the areal map holds inside that circle, and asked for no clan in particular it
    /// counts the enemy's alone — how strongly a place is held against the clan asking.
    ///
    /// STAND-IN: docs/15-behaviour.md#what-is-not-read-here -- an object's own strength
    /// (`0x1000fc70`: `(q + 0.8) × p × 1e-5` over `IControl` property `0x36` and interface
    /// `0x204`'s `+4`) is not followed to what those two are, so every object counts 1 and this
    /// is how many stand there. Only the order between candidates is used below, and a count
    /// leaves the least defended one least.
    pub fn strength_near(&self, hostile: impl Fn(i64) -> bool, x: f32, y: f32, radius: f32) -> f32 {
        self.objects()
            .filter(|&(_, clan, _, at)| hostile(clan) && (at[0] - x).hypot(at[1] - y) < radius)
            .count()
            .min(u32::MAX as usize) as f32
    }

    /// Function 71 (`ai.dll:0x1000f12e`): of the enemy objects whose type word is exactly
    /// `type_word`, the one the least enemy strength stands within `radius` of — the least
    /// defended — and that strength. `hostile` is the clan table's own test: another clan the
    /// running clan's relation word gives 0. `None` where no object answers, which the caller
    /// reads as `ERROR`. Functions 35, 36, 37, 40 and 64 pick by the same score.
    pub fn enemy_of_type(
        &self,
        hostile: impl Fn(i64) -> bool,
        type_word: u32,
        radius: f32,
    ) -> Option<(i32, f32)> {
        self.objects()
            .filter(|&(_, clan, word, _)| word == type_word && hostile(clan))
            .map(|(id, _, _, at)| (id, self.strength_near(&hostile, at[0], at[1], radius)))
            // The first of an equal pair is kept: the game's own loop takes a candidate only
            // when it is strictly below the best so far.
            .reduce(|best, one| if one.1 < best.1 { one } else { best })
    }

    /// Whether any unit or building answers logical id `id`, as function 15 finds its unit
    /// through the clan areal map's slot 7 (`ai.dll:0x1000835e`).
    pub fn knows(&self, id: i32) -> bool {
        self.units.iter().any(|u| u.id == id && u.alive)
            || self.buildings.iter().any(|b| b.id == id && b.alive)
    }

    /// Function 52 (`ai.dll:0x1000e0e4`): the owner word of the object with logical id `id`,
    /// a clan's index; 65534 once it is destroyed; `ERROR` when no object answers the id
    /// (docs/34, "What the scripts ask").
    pub fn owner(&self, id: i32) -> u32 {
        let found = self
            .units
            .iter()
            .map(|u| (u.id, u.clan, u.alive))
            .chain(self.buildings.iter().map(|b| (b.id, b.clan, b.alive)));
        match found.into_iter().find(|&(i, _, _)| i == id) {
            Some((_, _, false)) => DESTROYED_OWNER,
            Some((_, clan, true)) => clan as u32,
            None => NO_OBJECT,
        }
    }

    /// A dead unit deleted, its controller's `+92` ms after it died
    /// (`World3D.dll!KillGameObject`, docs/26, "A dead unit is deleted"): no object answers its
    /// id any more, and function 52 gives `ERROR`. A building is never deleted; its shell
    /// stays, and its owner word reads 65534.
    pub fn deleted(&mut self, id: i32) {
        self.units.retain(|u| u.id != id);
        self.areals.leave(id);
    }

    /// A unit or building destroyed.
    ///
    /// STAND-IN: docs/34-progression.md#function-31-how-many-robots-a-clan-has--read -- the
    /// game's own list never lets an entry go: nothing in `ai.dll` shortens it or clears an
    /// entry's id, so functions 31 and 34 keep counting a destroyed unit. The engine takes
    /// it off the count instead, because reproducing that would leave Single.01's and
    /// Single.02's objectives, and four campaign missions' bonus ones, unreachable.
    pub fn destroyed(&mut self, id: i32) {
        for u in self.units.iter_mut().filter(|u| u.id == id) {
            u.alive = false;
        }
        for b in self.buildings.iter_mut().filter(|b| b.id == id) {
            b.alive = false;
        }
        self.areals.leave(id);
    }

    /// A unit or building captured into `clan`. The game files it with the new clan's
    /// SuperAI as slot 4's event 2 (`iron3d.dll:0x10032fd0`) and tells the old clan nothing,
    /// so it is counted by both; the engine moves it, for the same reason `destroyed` does.
    pub fn captured(&mut self, id: i32, clan: i64) {
        for u in self.units.iter_mut().filter(|u| u.id == id) {
            u.clan = clan;
        }
        for b in self.buildings.iter_mut().filter(|b| b.id == id) {
            b.clan = clan;
        }
    }

    /// Function 30 on channel 0 (`iron3d.dll:0x10060ce0`): `kind` and `value` as the
    /// script hands them.
    pub fn call(&mut self, kind: i64, value: i64) -> Vec<Notice> {
        let mut out = Vec::new();
        match kind {
            MESSAGE_INFO => {
                if let Some(played) = self.played.get_mut(&value) {
                    out.push(Notice::Message { id: value, first: !*played });
                    *played = true;
                }
            }
            SYSTEM_MESSAGE => self.system(value, &mut out),
            OBJECTIVE_COMPLETE => {
                // An open objective completes (`0x10060e89`); the completion test follows
                // every call, open or not (`0x10060f5e`, `0x1006b130`). Nothing bounds the
                // index in the game; past the list the engine changes nothing.
                if let Some((index, o)) =
                    usize::try_from(value).ok().and_then(|i| self.objectives.get_mut(i).map(|o| (i, o)))
                    && o.state != 1
                {
                    o.state = 1;
                    out.push(Notice::ObjectiveComplete { index });
                }
                if self.objectives.iter().all(|o| o.exempt || o.state == 1) {
                    self.system(MISSION_COMPLETE, &mut out);
                }
            }
            OBJECTIVE_FAILED => {
                // STAND-IN: docs/34-progression.md#objectives-and-the-end-of-a-mission--read-and-measured
                // -- past fetching string 5041 the failure is not followed: it shows the
                // string and changes no state.
                if let Some(index) = usize::try_from(value).ok().filter(|&i| i < self.objectives.len()) {
                    out.push(Notice::ObjectiveFailed { index });
                }
            }
            // `CLAN_HERO_KILLED` does nothing in this build (docs/21).
            // STAND-IN: docs/21-briefing.md#messagescfg--the-in-mission-dialogue -- what
            // `OBJECTIVE_PROGRESS` shows is not read; nothing.
            _ => {}
        }
        out
    }

    /// `SYSTEM_MESSAGE` (`0x10060d9f`): an outcome already recorded does nothing again. The
    /// complete tests the state word against won (`0x10060d8c`) and the failure against lost
    /// (`0x10060de9`), so either replaces the other (docs/34, "After the outcome"); a hero
    /// standing at a teleport's out place raises the win every place tick.
    fn system(&mut self, value: i64, out: &mut Vec<Notice>) {
        match value {
            MISSION_COMPLETE if self.outcome != Some(true) => {
                self.outcome = Some(true);
                out.push(Notice::MissionComplete);
            }
            MISSION_FAILED if self.outcome != Some(false) => {
                self.outcome = Some(false);
                out.push(Notice::MissionFailed);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_52_answers_a_buildings_clan_its_destroyed_word_or_error() {
        let mut p = Progress::new(&[], &[], []);
        let factory = 0x8000_0001_u32 as i32;
        p.place_building(factory, 2, 0x8000_0010, Vec3::ZERO);
        p.join(1, 0, 0x0102_0000, Vec3::ZERO, 0.0);
        assert_eq!((p.owner(factory), p.owner(1)), (2, 0));
        p.captured(factory, 0);
        assert_eq!(p.owner(factory), 0);
        p.destroyed(factory);
        assert_eq!(p.owner(factory), DESTROYED_OWNER);
        assert_eq!(p.owner(0x8000_0003_u32 as i32), NO_OBJECT);
        // A dead unit reads 65534 until it is deleted, and then nothing answers its id; a
        // building's shell is never deleted.
        p.destroyed(1);
        assert_eq!(p.owner(1), DESTROYED_OWNER);
        p.deleted(1);
        p.deleted(factory);
        assert_eq!((p.owner(1), p.owner(factory)), (NO_OBJECT, DESTROYED_OWNER));
    }

    #[test]
    fn function_34_counts_a_clans_own_units_and_buildings_of_exactly_one_type() {
        let mut p = Progress::new(&[], &[], []);
        let mine = 0x8000_0004_u32;
        p.place_building(0x8000_0009_u32 as i32, 0, mine, Vec3::ZERO);
        p.place_building(0x8000_000a_u32 as i32, 1, mine, Vec3::ZERO);
        p.place_building(0x8000_000b_u32 as i32, 0, 0x8000_0008, Vec3::ZERO);
        p.join(1, 0, 0x0102_0000, Vec3::ZERO, 0.0);
        assert_eq!(p.count_type(0, mine), 1);
        assert_eq!(p.count_type(0, 0x8000_0000), 0, "the type is compared whole");
        assert_eq!(p.count_type(0, 0x0102_0000), 1);
        assert!(p.knows(1) && p.knows(0x8000_000a_u32 as i32) && !p.knows(3));
    }

    fn square(id: u32, x0: f32, y0: f32, side: f32) -> Route {
        let points =
            vec![[x0, y0, 0.0], [x0 + side, y0, 0.0], [x0 + side, y0 + side, 0.0], [x0, y0 + side, 0.0]];
        Route { id, points }
    }

    #[test]
    fn the_crossing_test_holds_inside_points_and_counts_the_low_edges() {
        let r = square(0, 0.0, 0.0, 10.0);
        assert!(contains(&r.points, 5.0, 5.0));
        assert!(!contains(&r.points, 15.0, 5.0));
        assert!(!contains(&r.points, 5.0, -1.0));
        // An end at or above y and the other below: on the bottom edge's y both ends of
        // each side are at or above it, so nothing counts; on the top edge's the sides do.
        assert!(!contains(&r.points, 5.0, 0.0));
        assert!(contains(&r.points, 5.0, 10.0));
        // A concave outline: an L.
        let l = [
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 4.0, 0.0],
            [4.0, 4.0, 0.0],
            [4.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
        ];
        assert!(contains(&l, 2.0, 8.0) && contains(&l, 8.0, 2.0));
        assert!(!contains(&l, 8.0, 8.0));
        assert!(!contains(&[], 0.0, 0.0));
    }

    #[test]
    fn areal_i_is_the_route_with_id_i_and_a_report_moves_an_id_between_lists() {
        let routes = [square(1, 100.0, 0.0, 10.0), square(0, 0.0, 0.0, 10.0)];
        let mut a = Areals::new(&routes);
        a.report(7, Vec3::new(5.0, 5.0, 0.0));
        assert!(a.holds(0, 7) && !a.holds(1, 7));
        a.report(7, Vec3::new(105.0, 5.0, 0.0));
        assert!(!a.holds(0, 7) && a.holds(1, 7));
        assert!(!a.holds(2, 7) && !a.holds(-1, 7));
    }

    #[test]
    fn a_unit_reports_on_its_takt_only_once_it_has_moved_more_than_five() {
        let mut p = Progress::new(&[square(0, 0.0, 0.0, 10.0)], &[], []);
        p.join(1, 0, 0x0102_0000, Vec3::new(-2.0, 5.0, 0.0), 0.0);
        assert!(!p.areals.holds(0, 1));
        // Inside by 4 by |dx| + |dy|: not reported, however long it waits.
        p.takt(10_000.0, |_| Some(Vec3::new(2.0, 5.0, 0.0)));
        assert!(!p.areals.holds(0, 1));
        // A takt comes 1984 to 4916 ms after the last.
        let next = p.units[0].next_takt_ms;
        assert!((11_984.0..=14_916.0).contains(&next), "next {next}");
        p.takt(next - 1.0, |_| Some(Vec3::new(4.0, 5.0, 0.0)));
        assert!(!p.areals.holds(0, 1), "not yet due");
        p.takt(next, |_| Some(Vec3::new(4.0, 5.0, 0.0)));
        assert!(p.areals.holds(0, 1), "moved 6");
    }

    #[test]
    fn a_units_first_takt_runs_at_once_and_the_ones_after_it_are_two_to_five_seconds_apart() {
        let mut p = Progress::new(&[square(0, 0.0, 0.0, 10.0)], &[], []);
        p.join(1, 0, 0x0102_0000, Vec3::new(-20.0, 5.0, 0.0), 9_000.0);
        assert_eq!(p.units[0].next_takt_ms, 0.0, "the timer's word starts at 0");
        // The first takt runs however long after the unit joined, and reports the move.
        p.takt(9_000.0, |_| Some(Vec3::new(5.0, 5.0, 0.0)));
        assert!(p.areals.holds(0, 1), "the first takt reports");
        let mut at = 9_000.0;
        for _ in 0..200 {
            let next = p.units[0].next_takt_ms;
            let wait = next - at;
            assert!((1984.0..=4916.0).contains(&wait), "wait {wait}");
            at = next;
            p.takt(next, |_| Some(Vec3::new(5.0, 5.0, 0.0)));
        }
    }

    #[test]
    fn the_object_takts_rand8_is_the_crts_own_from_a_seed_of_one() {
        // `rand()` seeded 1 gives 41, 18467, 6334, 26500, ...; rand8 is each one's low byte.
        let mut p = Progress::new(&[], &[], []);
        let drawn: Vec<u8> = (0..4).map(|_| p.rand8()).collect();
        assert_eq!(drawn, vec![41u8, (18467 & 0xff) as u8, (6334 & 0xff) as u8, (26500 & 0xff) as u8]);
    }

    #[test]
    fn the_mission_handler_runs_at_once_then_every_two_seconds() {
        let mut p = Progress::new(&[], &[], []);
        assert!(p.mission_due(0.0));
        assert!(!p.mission_due(1999.0));
        assert!(p.mission_due(2000.0));
        assert!(!p.mission_due(3000.0));
    }

    #[test]
    fn robots_count_by_clan_and_mask_and_follow_deaths_and_captures() {
        let mut p = Progress::new(&[], &[], []);
        let robot = 0x0100_0000;
        p.join(1, 0, 0x0102_0000, Vec3::ZERO, 0.0);
        p.join(13, 3, 0x0108_0000, Vec3::ZERO, 0.0);
        p.join(8, 3, 0x0108_0000, Vec3::ZERO, 0.0);
        p.join(40, 3, 0x0000_0001, Vec3::ZERO, 0.0);
        assert_eq!((p.robots(0, robot), p.robots(3, robot)), (1, 2));
        p.captured(13, 0);
        p.destroyed(8);
        assert_eq!((p.robots(0, robot), p.robots(3, robot)), (2, 0));
    }

    #[test]
    fn function_31_counts_the_buildings_on_the_clans_list_too() {
        // C01 Mission 04's base: the enemy's bunker, generator and factory, none of them a
        // robot, all of them `CLASS_BUILDING`. Its bonus objective waits for the count to
        // reach 0, so a clan that still holds a building is never done.
        let (robot, building) = (0x0100_0000, 0x8000_0000_u32 as i64);
        let mut p = Progress::new(&[], &[], []);
        p.place_building(0x8000_0006_u32 as i32, 1, 0x8001_0000, Vec3::ZERO);
        p.place_building(0x8000_0007_u32 as i32, 1, 0x8000_0002, Vec3::ZERO);
        p.place_building(0x8000_0008_u32 as i32, 1, 0x8000_0010, Vec3::ZERO);
        p.join(24, 1, 0x0108_0000, Vec3::ZERO, 0.0);
        assert_eq!((p.robots(1, building), p.robots(1, robot)), (3, 1));
        // Captured and destroyed buildings both leave their clan's count, as either way of
        // taking the base completes the objective.
        p.captured(0x8000_0006_u32 as i32, 0);
        p.destroyed(0x8000_0007_u32 as i32);
        assert_eq!((p.robots(1, building), p.robots(0, building)), (1, 1));
        p.destroyed(0x8000_0008_u32 as i32);
        assert_eq!(p.robots(1, building), 0);
    }

    #[test]
    fn a_clans_takt_comes_every_seven_seconds_and_its_clock_follows() {
        let mut t = ClanTakt::new(0, 1);
        assert!(t.due(0.0), "the first takt runs at once");
        assert_eq!(t.clock(), CLAN_TAKT_SECONDS);
        assert!(!t.due(CLAN_TAKT_MS - 1.0));
        // Every later takt comes 7000 to 7999 ms after the one before, and steps the clock a
        // flat 7 s however long it waited.
        let mut last = 0.0_f64;
        for step in 2..200_u32 {
            let mut now = last;
            while !t.due(now) {
                now += 1.0;
            }
            let wait = now - last;
            let window = CLAN_TAKT_MS..CLAN_TAKT_MS + f64::from(CLAN_TAKT_RANDOM_MS);
            assert!(window.contains(&wait), "takt {step} waited {wait} ms");
            assert_eq!(t.clock(), step * CLAN_TAKT_SECONDS);
            last = now;
        }
    }

    #[test]
    fn function_71_takes_the_enemy_of_a_type_with_the_least_held_against_it() {
        // C03 Mission 02's shape: the player's Small Bunker and two factories against the
        // enemy's own factory, which is of the same type but not an enemy's.
        let (bunker, plant) = (0x8001_0000, 0x8000_0010);
        let mut p = Progress::new(&[], &[], []);
        p.place_building(0x8000_0001_u32 as i32, 0, bunker, Vec3::new(381.9, 1199.1, 0.0));
        p.place_building(0x8000_0003_u32 as i32, 0, plant, Vec3::new(508.9, 1390.9, 0.0));
        p.place_building(0x8000_0005_u32 as i32, 0, plant, Vec3::new(20.0, 20.0, 0.0));
        p.place_building(0x8000_0009_u32 as i32, 2, plant, Vec3::new(1213.5, 175.1, 0.0));
        // Two of the player's warbots stand by its first factory and one of the clan's own by
        // its second, which counts for nothing: the strength is what is held against us.
        p.join(6, 0, 0x0100_8000, Vec3::new(520.0, 1400.0, 0.0), 0.0);
        p.join(7, 0, 0x0100_8000, Vec3::new(560.0, 1380.0, 0.0), 0.0);
        p.join(9, 2, 0x0100_8000, Vec3::new(40.0, 30.0, 0.0), 0.0);
        let hostile = |clan: i64| clan == 0;
        assert_eq!(
            p.enemy_of_type(hostile, bunker, 100.0),
            Some((0x8000_0001_u32 as i32, 1.0)),
            "the bunker is the only one of its type an enemy holds, and stands alone"
        );
        assert_eq!(
            p.enemy_of_type(hostile, plant, 100.0),
            Some((0x8000_0005_u32 as i32, 1.0)),
            "of the two factories the player holds, the one nothing guards"
        );
        assert_eq!(p.enemy_of_type(hostile, 0x8000_0002, 100.0), None, "no enemy generator");
        // Its own factory is never a candidate, and a destroyed bunker leaves the map.
        assert_eq!(
            p.enemy_of_type(|clan| clan == 2, plant, 100.0).map(|(id, _)| id),
            Some(0x8000_0009_u32 as i32)
        );
        p.destroyed(0x8000_0001_u32 as i32);
        assert_eq!(p.enemy_of_type(hostile, bunker, 100.0), None);
    }

    #[test]
    fn a_message_plays_once_then_only_says_it_is_in_the_history() {
        let mut p = Progress::new(&[], &[], [11, 14]);
        assert_eq!(p.call(MESSAGE_INFO, 11), vec![Notice::Message { id: 11, first: true }]);
        assert_eq!(p.call(MESSAGE_INFO, 11), vec![Notice::Message { id: 11, first: false }]);
        assert!(p.call(MESSAGE_INFO, 99).is_empty(), "no such message");
    }

    #[test]
    fn the_mission_is_won_when_the_last_primary_objective_completes() {
        let mut p = Progress::new(&[], &[false, false, true], []);
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 1), vec![Notice::ObjectiveComplete { index: 1 }]);
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 1), vec![], "already complete");
        assert_eq!(p.outcome, None);
        // The bonus objective, exempt, never holds it back.
        assert_eq!(
            p.call(OBJECTIVE_COMPLETE, 0),
            vec![Notice::ObjectiveComplete { index: 0 }, Notice::MissionComplete]
        );
        assert_eq!(p.outcome, Some(true));
        // The test follows every call, but a repeated outcome does nothing.
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 0), vec![]);
        assert_eq!(p.call(SYSTEM_MESSAGE, MISSION_COMPLETE), vec![]);
        // A failure after a win replaces it, and a win after that replaces the failure.
        assert_eq!(p.call(SYSTEM_MESSAGE, MISSION_FAILED), vec![Notice::MissionFailed]);
        assert_eq!(p.call(SYSTEM_MESSAGE, MISSION_FAILED), vec![]);
        assert_eq!(p.outcome, Some(false));
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 9), vec![Notice::MissionComplete]);
        assert_eq!(p.outcome, Some(true));
    }
}
