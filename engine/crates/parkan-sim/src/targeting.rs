//! The player's target: the list the driven unit keeps from its radar's contacts, the
//! picks the keys make, and what the takt drops and picks by itself. See
//! `docs/25-sensors.md`, "The player's target".

use glam::Vec3;

/// A building's sphere counts at 0.4 of its radius in the right button's pick, anything
/// else's at 0.6 (`iron3d.dll:0x10091780`).
pub const AIM_BUILDING_SPHERE: f32 = 0.4;
pub const AIM_SPHERE: f32 = 0.6;

/// What the list knows of an object, by the caller's numbering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    /// The object's placement, and its bounding sphere.
    pub position: Vec3,
    pub centre: Vec3,
    pub radius: f32,
    pub alive: bool,
    pub building: bool,
    /// Its clan's relation word toward the unit's is 0, and its clan is not nature's.
    pub hostile: bool,
    /// Of the unit's own clan.
    pub friend: bool,
    /// Never listed: a hero of the player's clan, a bridge or a ruin (`0x10091c80`).
    pub unlisted: bool,
}

/// The radar's answer, kept for its period (`Control.dll:0x10024390`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Radar {
    /// Value 3, m, and value 4, ms.
    pub range: f32,
    pub period_ms: f32,
    /// Its node has no life left (slot 2, `0x10021820`), which the owner keeps from the node.
    pub broken: bool,
    contacts: Vec<usize>,
    next_ms: f64,
}

impl Radar {
    pub fn new(range: f32, period_ms: f32) -> Self {
        Self { range, period_ms, broken: false, contacts: Vec::new(), next_ms: f64::NEG_INFINITY }
    }

    /// The contacts of a radar at `at` by `now_ms`: none while its node is destroyed
    /// (`0x10024390` asks slot 2 first), the last answer while it is fresh, otherwise a new scan
    /// of the objects within range.
    ///
    /// STAND-IN: docs/25-sensors.md#a-scan-is-a-sphere-a-falloff-and-three-tests--read --
    /// the three signatures a detection weighs are not computed: every live object within
    /// the range is detected. A unit's mass alone is thousands of kilograms, which a fitted
    /// radar's 0.05 sees out to nearly its whole range.
    pub fn scan(&mut self, now_ms: f64, at: Vec3, world: &[Contact]) -> &[usize] {
        if self.broken {
            return &[];
        }
        if now_ms >= self.next_ms {
            self.next_ms = now_ms + f64::from(self.period_ms);
            self.contacts = (0..world.len())
                .filter(|&i| world[i].alive && world[i].position.distance(at) < self.range)
                .collect();
        }
        &self.contacts
    }
}

/// The distance across the ground, in x and y.
fn across(a: Vec3, b: Vec3) -> f32 {
    a.truncate().distance(b.truncate())
}

/// The driven unit's target list (`iron3d.dll`, the record's `+0x38`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetList {
    /// The listed contacts, in the radar's order.
    pub listed: Vec<usize>,
    /// The current target.
    pub current: Option<usize>,
    /// The last rebuild listed a hostile contact (`+0x20`).
    hostile_listed: bool,
}

/// What a list change asks the game to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// The target changed: `TARGET_SELECTED`, and the guns relinked (`0x10090a70`).
    pub target: bool,
    /// A rebuild found a hostile after one that found none: `VOICE_ENEMY_DETECTED`.
    pub enemy_detected: bool,
}

impl TargetList {
    /// Set the target, and whether it changed.
    pub fn set(&mut self, target: Option<usize>) -> bool {
        let changed = self.current != target;
        self.current = target;
        changed
    }

    /// The list's takt on the unit the player drives (`0x10090bb0`): rebuild from
    /// `contacts` (`0x10091b90`), drop a target that is dead or out of `range` across the
    /// ground, and pick one when there is none: the nearest hostile, else the nearest
    /// friend, else the nearest listed (`0x10090c28`–`0x10090d13`).
    pub fn takt(&mut self, unit: Vec3, range: f32, contacts: &[usize], world: &[Contact]) -> Changes {
        let mut out = Changes::default();
        self.listed =
            contacts.iter().copied().filter(|&i| world.get(i).is_some_and(|c| !c.unlisted)).collect();
        let hostile = self.listed.iter().any(|&i| world[i].hostile);
        out.enemy_detected = hostile && !self.hostile_listed;
        self.hostile_listed = hostile;

        let before = self.current;
        if let Some(t) = self.current
            && world.get(t).is_none_or(|c| !c.alive || across(c.position, unit) > range)
        {
            self.current = None;
        }
        if self.listed.is_empty() {
            self.current = None;
        } else if self.current.is_none() {
            self.current = self
                .nearest(unit, world, |c| c.hostile)
                .or_else(|| self.nearest(unit, world, |c| c.friend))
                .or_else(|| self.nearest(unit, world, |_| true));
        }
        out.target = self.current != before;
        out
    }

    fn nearest(&self, unit: Vec3, world: &[Contact], pick: impl Fn(&Contact) -> bool) -> Option<usize> {
        self.listed
            .iter()
            .copied()
            .filter(|&i| pick(&world[i]))
            .min_by(|&a, &b| across(world[a].position, unit).total_cmp(&across(world[b].position, unit)))
    }

    /// `CMD_JAMES_SELECT_TARGET` (`0x10090dc0`): the listed entry after the target,
    /// wrapping; the first when the target is not listed.
    pub fn select_next(&mut self) -> bool {
        let Some(&first) = self.listed.first() else { return false };
        let next = match self.current.and_then(|t| self.listed.iter().position(|&i| i == t)) {
            Some(at) => self.listed[(at + 1) % self.listed.len()],
            None => first,
        };
        self.set(Some(next))
    }

    /// `CMD_JAMES_SELECT_ENEMY` (`0x10090e30`) with `pick` hostile, or
    /// `CMD_JAMES_SELECT_FRIEND` (`0x10091070`) with it the unit's own clan: with such a
    /// target, the next such in list order, wrapping; otherwise the nearest such across
    /// the ground. Nothing found leaves the target.
    pub fn select_nearest(&mut self, unit: Vec3, world: &[Contact], pick: impl Fn(&Contact) -> bool) -> bool {
        let of_kind: Vec<usize> = self.listed.iter().copied().filter(|&i| pick(&world[i])).collect();
        let next = match self.current.and_then(|t| of_kind.iter().position(|&i| i == t)) {
            Some(at) => of_kind.get((at + 1) % of_kind.len()).copied(),
            None => self.nearest(unit, world, &pick),
        };
        match next {
            Some(n) => self.set(Some(n)),
            None => false,
        }
    }

    /// `CMD_JAMES_AIM_TARGET` (`0x100911f0`): a pick along the middle of the view, from
    /// `eye` along `look`, from `start` out to `range`. The candidates are the listed
    /// objects but the target that `visible` places on screen (in NDC). The first choice
    /// is the one nearest the ray's start that the ray passes through, its sphere taken at
    /// 0.4 or 0.6, with its centre ahead within the ray and farther from `unit` than that
    /// radius plus `margin`; otherwise the one whose centre is nearest the screen's middle.
    /// The result, or none, is the target.
    #[allow(clippy::too_many_arguments)]
    pub fn aim(
        &mut self,
        unit: Vec3,
        eye: Vec3,
        look: Vec3,
        start: f32,
        margin: f32,
        range: f32,
        world: &[Contact],
        visible: impl Fn(Vec3, f32) -> Option<[f32; 2]>,
    ) -> bool {
        if range <= start {
            return false;
        }
        let look = look.normalize_or(Vec3::Y);
        let from = eye + look * start;
        let length = range - start;
        let candidates: Vec<(usize, [f32; 2])> = self
            .listed
            .iter()
            .copied()
            .filter(|&i| Some(i) != self.current)
            .filter_map(|i| visible(world[i].centre, world[i].radius).map(|ndc| (i, ndc)))
            .collect();
        let through = candidates
            .iter()
            .map(|&(i, _)| i)
            .filter(|&i| {
                let c = &world[i];
                let r = c.radius * if c.building { AIM_BUILDING_SPHERE } else { AIM_SPHERE };
                let along = (c.centre - from).dot(look);
                let off = (c.centre - (from + look * along)).length();
                (0.0..=length).contains(&along) && off <= r && c.centre.distance(unit) > r + margin
            })
            .min_by(|&a, &b| world[a].centre.distance(from).total_cmp(&world[b].centre.distance(from)));
        let nearest_middle = || {
            candidates
                .iter()
                .min_by(|a, b| (a.1[0].hypot(a.1[1])).total_cmp(&b.1[0].hypot(b.1[1])))
                .map(|&(i, _)| i)
        };
        let pick = through.or_else(nearest_middle);
        self.set(pick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(x: f32, y: f32, hostile: bool, friend: bool) -> Contact {
        let position = Vec3::new(x, y, 0.0);
        Contact {
            position,
            centre: position + Vec3::Z * 3.0,
            radius: 4.0,
            alive: true,
            building: false,
            hostile,
            friend,
            unlisted: false,
        }
    }

    #[test]
    fn the_radar_sees_live_objects_within_range_and_holds_its_answer_for_its_period() {
        let mut world = vec![unit(0.0, 100.0, false, false), unit(0.0, 400.0, true, false)];
        let mut radar = Radar::new(300.0, 750.0);
        assert_eq!(radar.scan(0.0, Vec3::ZERO, &world), &[0]);
        world[1].position.y = 200.0;
        assert_eq!(radar.scan(700.0, Vec3::ZERO, &world), &[0], "still the last answer");
        assert_eq!(radar.scan(750.0, Vec3::ZERO, &world), &[0, 1]);
        world[0].alive = false;
        assert_eq!(radar.scan(1500.0, Vec3::ZERO, &world), &[1]);
        radar.broken = true;
        assert!(
            radar.scan(3000.0, Vec3::ZERO, &world).is_empty(),
            "a radar whose node is destroyed sees nothing"
        );
    }

    #[test]
    fn the_takt_picks_the_nearest_hostile_then_friend_then_anything_and_drops_what_is_gone() {
        let mut world = vec![
            unit(0.0, 50.0, false, false),
            unit(0.0, 120.0, true, false),
            unit(0.0, 90.0, true, false),
            unit(0.0, 30.0, false, true),
        ];
        let mut list = TargetList::default();
        let changes = list.takt(Vec3::ZERO, 300.0, &[0, 1, 2, 3], &world);
        assert_eq!(list.current, Some(2), "the nearest hostile");
        assert!(changes.target && changes.enemy_detected);
        assert!(!list.takt(Vec3::ZERO, 300.0, &[0, 1, 2, 3], &world).enemy_detected, "only on the first");

        world[2].alive = false;
        list.takt(Vec3::ZERO, 300.0, &[0, 1, 3], &world);
        assert_eq!(list.current, Some(1));
        world[1].alive = false;
        list.takt(Vec3::ZERO, 300.0, &[0, 3], &world);
        assert_eq!(list.current, Some(3), "then the nearest friend");
        // Out of range across the ground: dropped. While the radar still lists it, it is
        // the nearest friend again; once the radar drops it, the nearest listed is taken.
        world[3].position = Vec3::new(0.0, 350.0, 0.0);
        assert!(!list.takt(Vec3::ZERO, 300.0, &[0, 3], &world).target);
        assert_eq!(list.current, Some(3));
        list.takt(Vec3::ZERO, 300.0, &[0], &world);
        assert_eq!(list.current, Some(0));
        list.takt(Vec3::ZERO, 300.0, &[], &world);
        assert_eq!(list.current, None, "an empty list leaves no target");
    }

    #[test]
    fn tab_steps_through_the_list_and_e_through_the_hostiles() {
        let world = vec![
            unit(0.0, 50.0, false, false),
            unit(0.0, 120.0, true, false),
            unit(0.0, 90.0, true, false),
            Contact { unlisted: true, ..unit(0.0, 10.0, false, true) },
        ];
        let mut list = TargetList::default();
        list.takt(Vec3::ZERO, 300.0, &[0, 1, 2, 3], &world);
        assert_eq!(list.listed, vec![0, 1, 2], "a hero of the player's clan is never listed");
        list.set(Some(0));
        assert!(list.select_next() && list.current == Some(1));
        list.select_next();
        list.select_next();
        assert_eq!(list.current, Some(0), "wrapping");
        list.select_nearest(Vec3::ZERO, &world, |c| c.hostile);
        assert_eq!(list.current, Some(2), "not hostile: the nearest hostile");
        list.select_nearest(Vec3::ZERO, &world, |c| c.hostile);
        assert_eq!(list.current, Some(1), "hostile: the next in list order");
        assert!(!list.select_nearest(Vec3::ZERO, &world, |c| c.friend), "no friend listed");
        assert_eq!(list.current, Some(1));
    }

    #[test]
    fn the_right_button_takes_what_the_view_passes_through_then_what_is_nearest_the_middle() {
        let world = vec![
            unit(0.0, 100.0, false, false),
            unit(2.0, 60.0, false, false),
            unit(30.0, 80.0, true, false),
        ];
        let mut list = TargetList::default();
        list.takt(Vec3::ZERO, 300.0, &[0, 1, 2], &world);
        list.set(None);
        let eye = Vec3::new(0.0, 0.0, 3.0);
        // A simple screen: x and z over y.
        let visible = |c: Vec3, _r: f32| (c.y > 0.0).then(|| [(c.x - eye.x) / c.y, (c.z - eye.z) / c.y]);
        assert!(list.aim(Vec3::ZERO, eye, Vec3::Y, 0.0, 0.0, 300.0, &world, visible));
        assert_eq!(list.current, Some(1), "2 m off the line, inside 0.6 of its 4 m sphere, and nearest");
        list.aim(Vec3::ZERO, eye, Vec3::Y, 0.0, 0.0, 300.0, &world, visible);
        assert_eq!(list.current, Some(0), "pressed again: the next along the line");
        // Looking away from everything: the one nearest the middle of the screen.
        let right = Vec3::new(1.0, 1.0, 0.0);
        list.aim(Vec3::ZERO, eye, right, 0.0, 0.0, 300.0, &world, visible);
        assert_eq!(list.current, Some(1), "nearest the middle of those on screen, the target excepted");
        let nothing = |_: Vec3, _: f32| None;
        list.aim(Vec3::ZERO, eye, Vec3::Y, 0.0, 0.0, 300.0, &world, nothing);
        assert_eq!(list.current, None, "nothing on screen clears it");
    }
}
