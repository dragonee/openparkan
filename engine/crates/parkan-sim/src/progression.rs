//! Mission progression: the routes a script tests as trigger areas, who stands in
//! them, how many robots each clan has, when the player clan's `Mission` handler
//! runs, and what its message and objective calls do. See `docs/34-progression.md`.
//!
//! The script itself runs elsewhere; this answers its functions 30, 31 and 32 and
//! keeps the state they read and write.

use std::collections::BTreeMap;

use glam::Vec3;
use parkan_formats::mission::Route;

/// `ai.dll:0x10001b80`: once the clock reaches the next-run time the `Mission` handler
/// runs, and the next-run time is set 2000 ms on.
pub const MISSION_PERIOD_MS: f64 = 2000.0;
/// A unit reports its position once it has moved more than this by |dx| + |dy|
/// (`Behavior.dll:0x1000af70`).
pub const REPORT_MOVE: f32 = 5.0;
/// The object takt's timer, next = now + 31 × 64 + rand8 × 46 × 64 ÷ 256 (the words at
/// `Behavior.dll:0x100039c2`).
pub const TAKT_BASE: f64 = 31.0 * 64.0;
pub const TAKT_STEP: f64 = 46.0 * 64.0 / 256.0;

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
            objectives: exempt.iter().map(|&exempt| Objective { exempt, state: 0 }).collect(),
            played: messages.into_iter().map(|id| (id, false)).collect(),
            outcome: None,
            next_mission_ms: 0.0,
            seed: 1,
        }
    }

    fn rand8(&mut self) -> u8 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.seed >> 16) as u8
    }

    /// A unit joins its clan's list, and reports once as its id is attached
    /// (`Behavior.dll:0x10005f91`, `iron3d.dll:0x10077511`).
    ///
    /// STAND-IN: docs/34-progression.md#who-stands-in-a-route--read -- the takt's clock is
    /// taken as game milliseconds, its rand8 as the engine's own generator, and its first
    /// run as one timer after the unit joins.
    pub fn join(&mut self, id: i32, clan: i64, type_word: u32, at: Vec3, now_ms: f64) {
        self.areals.report(id, at);
        let next_takt_ms = now_ms + TAKT_BASE + f64::from(self.rand8()) * TAKT_STEP;
        self.units.push(Unit { id, clan, type_word, alive: true, reported: [at.x, at.y], next_takt_ms });
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
            let wait = TAKT_BASE + f64::from(self.rand8()) * TAKT_STEP;
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

    /// Function 31 (`ai.dll:0x1000c2fd`): how many of clan `clan`'s units have a type
    /// sharing a bit with `mask`.
    pub fn robots(&self, clan: i64, mask: i64) -> usize {
        self.units.iter().filter(|u| u.alive && u.clan == clan && i64::from(u.type_word) & mask != 0).count()
    }

    /// A unit destroyed.
    ///
    /// STAND-IN: docs/34-progression.md#function-31-how-many-robots-a-clan-has--read -- how
    /// a destroyed unit leaves its clan's list is not read: it leaves it, and every route's.
    pub fn destroyed(&mut self, id: i32) {
        for u in self.units.iter_mut().filter(|u| u.id == id) {
            u.alive = false;
        }
        self.areals.leave(id);
    }

    /// A unit captured into `clan`: it leaves its old clan's count and joins the new one's
    /// (`MBehaviour::Capture`, docs/27; how the lists change is derived, docs/34).
    pub fn captured(&mut self, id: i32, clan: i64) {
        for u in self.units.iter_mut().filter(|u| u.id == id) {
            u.clan = clan;
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

    fn system(&mut self, value: i64, out: &mut Vec<Notice>) {
        match value {
            MISSION_COMPLETE => {
                self.outcome = Some(true);
                out.push(Notice::MissionComplete);
            }
            MISSION_FAILED => {
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
        // A takt comes 1984 to 4917 ms after the last.
        let next = p.units[0].next_takt_ms;
        assert!((10_000.0 + TAKT_BASE..=10_000.0 + TAKT_BASE + 255.0 * TAKT_STEP).contains(&next));
        p.takt(next - 1.0, |_| Some(Vec3::new(4.0, 5.0, 0.0)));
        assert!(!p.areals.holds(0, 1), "not yet due");
        p.takt(next, |_| Some(Vec3::new(4.0, 5.0, 0.0)));
        assert!(p.areals.holds(0, 1), "moved 6");
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
        // The test follows every call: a repeat sends the outcome again.
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 0), vec![Notice::MissionComplete]);
        assert_eq!(p.call(OBJECTIVE_COMPLETE, 9), vec![Notice::MissionComplete]);
    }
}
