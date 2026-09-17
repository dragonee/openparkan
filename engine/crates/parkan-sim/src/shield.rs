//! A unit's fight shield: six sectors, and the deflector that decides how much of each stops
//! damage. See `docs/26-damage.md`, "Shields: a generator, a deflector, six sectors".

use glam::{Quat, Vec3};

pub const SECTORS: usize = 6;
/// The sectors by the dominant axis of a hit in the object's frame (`Control.dll:0x1002c590`).
pub const FRONT: usize = 0;
pub const BACK: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const TOP: usize = 4;
pub const BOTTOM: usize = 5;

/// The sector a hit from `direction` lands in, the direction from the bubble's centre in the
/// object's own frame: the dominant axis, +y the front, a tie going to the lower sector.
pub fn sector(direction: Vec3) -> usize {
    let Vec3 { x, y, z } = direction;
    if y >= x.abs() && y >= z.abs() {
        FRONT
    } else if y <= -x.abs() && y <= -z.abs() {
        BACK
    } else if x <= -y.abs() && x <= -z.abs() {
        LEFT
    } else if x >= y.abs() && x >= z.abs() {
        RIGHT
    } else if z >= x.abs() && z >= y.abs() {
        TOP
    } else {
        BOTTOM
    }
}

/// Where a round's sphere of radius `r` first meets a bubble of radius `bubble` about
/// `centre` over its move from `from` to `to` (`Control.dll:0x1001e9f0`): the point on the
/// bubble's surface along the line between the two centres at that moment, and its squared
/// distance from `from`. A round that starts inside the bubble's reach, or moves away from
/// it, makes no contact.
pub fn contact(from: Vec3, to: Vec3, r: f32, centre: Vec3, bubble: f32) -> Option<(Vec3, f32)> {
    let reach = bubble + r;
    let d = centre - from;
    if d.length_squared() < reach * reach {
        return None;
    }
    let v = from - to;
    let vv = v.length_squared();
    if vv < 1e-6 {
        return None;
    }
    let b = -d.dot(v);
    if b < 0.0 {
        return None;
    }
    let disc = b * b - (d.length_squared() - reach * reach) * vv;
    if disc < 0.0 {
        return None;
    }
    let t = (b - disc.sqrt()) / vv;
    if t > 1.0 {
        return None;
    }
    let at = from + (to - from) * t;
    let point = at + (centre - at) * (r / reach);
    Some((point, (point - from).length_squared()))
}

/// A fight shield and its deflector, as the object's device manager holds them.
#[derive(Clone, Debug, PartialEq)]
pub struct Shield {
    /// The generator's value 0, the points a full sector holds, times the object's level
    /// ratio (`+0x11c`).
    pub max: f32,
    /// Value 1: the most it recharges a second, times its condition.
    pub recharge: f32,
    /// Value 2: the charge a point costs.
    pub cost: f32,
    /// The generator's draw a second, and the deflector's.
    pub shield_power: f32,
    pub deflector_power: f32,
    /// The deflector's values 0–5, one a sector.
    pub coefficients: [f32; SECTORS],
    /// Each sector's fill, 0 to 1 (`+0x98 + 4i`); full when made.
    pub fills: [f32; SECTORS],
    /// The effect a contact plays: the generator's resource.
    pub effect: String,
    /// The generator's node and the deflector's, each as a part and a node of it, whose life
    /// is the device's condition.
    pub shield_node: Option<(usize, usize)>,
    pub deflector_node: Option<(usize, usize)>,
    /// The share of what their power channel wants that it is served, 0 to 1.
    pub level: f32,
    /// The generator's and the deflector's conditions, which the owner keeps from their
    /// nodes' lives.
    pub shield_condition: f32,
    pub deflector_condition: f32,
}

impl Shield {
    pub fn new(max: f32, recharge: f32, cost: f32, coefficients: [f32; SECTORS]) -> Self {
        Self {
            max,
            recharge,
            cost,
            shield_power: 0.0,
            deflector_power: 0.0,
            coefficients,
            fills: [1.0; SECTORS],
            effect: String::new(),
            shield_node: None,
            deflector_node: None,
            level: 1.0,
            shield_condition: 1.0,
            deflector_condition: 1.0,
        }
    }

    /// Whether the bubble is up (`0x1002c500`): both devices on and neither destroyed. Power
    /// is not asked; an unpowered deflector stops nothing.
    pub fn up(&self) -> bool {
        self.shield_condition > 0.0 && self.deflector_condition > 0.0
    }

    /// What the deflector makes of a sector: its coefficient × condition × power level.
    fn deflects(&self, s: usize) -> f32 {
        self.coefficients[s] * self.deflector_condition * self.level
    }

    /// A sector's effective strength (`0x1002ca30`): the deflector's share of the points the
    /// sector holds.
    pub fn strength(&self, s: usize) -> f32 {
        self.deflects(s) * self.max * self.fills[s]
    }

    /// A hit of `damage` on sector `s` (`0x1002ca80`): it takes what the sector can stop out
    /// of the sector, the sector losing that divided by what the deflector makes of it, and
    /// returns the damage stopped.
    pub fn absorb(&mut self, s: usize, damage: f32) -> f32 {
        let stopped = damage.max(0.0).min(self.strength(s));
        let deflects = self.deflects(s);
        if stopped > 0.0 && deflects > 0.0 && self.max > 0.0 {
            self.fills[s] = (self.fills[s] - stopped / deflects / self.max).max(0.0);
        }
        stopped
    }

    /// A sector spent whole, as a round that passes through it leaves it (slot 15 with 0).
    pub fn empty(&mut self, s: usize) {
        self.fills[s] = 0.0;
    }

    /// `dt` seconds of the power tick (`0x10025700`, `0x100257b0`): the generator draws its idle
    /// power and the charge its recharge wants; what the level serves beyond the idle draw
    /// buys points, shared by what each sector lacks, and a charge short of the idle draw
    /// drains the sectors by what each holds.
    pub fn tick(&mut self, dt: f32) {
        if self.shield_condition <= 0.0 || self.cost <= 0.0 || self.max <= 0.0 || dt <= 0.0 {
            return;
        }
        let held: f32 = self.fills.iter().sum();
        let lack = SECTORS as f32 - held;
        let wanted = (self.recharge * self.shield_condition * dt).min(self.max * lack);
        let idle = self.shield_power * dt;
        let charge = self.level * (idle + self.cost * wanted) - idle;
        let points = charge / self.cost;
        if points > 0.0 && lack > 0.0 {
            let share = points.min(self.max * lack) / self.max / lack;
            for f in &mut self.fills {
                *f = (*f + share * (1.0 - *f)).min(1.0);
            }
        } else if points < 0.0 && held > 0.0 {
            let share = (-points / self.max / held).min(1.0);
            for f in &mut self.fills {
                *f = (*f - share * *f).max(0.0);
            }
        }
    }

    /// The sector a hit at `point` lands in, for a bubble about `centre` on an object turned by
    /// `rotation`: the offset taken into the object's frame (`0x1002c666`).
    pub fn sector_of(centre: Vec3, rotation: Quat, point: Vec3) -> usize {
        sector(rotation.inverse() * (point - centre))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shield() -> Shield {
        Shield::new(350.0, 4.0, 0.03, [0.7; SECTORS])
    }

    #[test]
    fn the_dominant_axis_picks_the_sector_and_a_tie_goes_to_the_lower() {
        assert_eq!(sector(Vec3::new(0.0, 1.0, 0.0)), FRONT);
        assert_eq!(sector(Vec3::new(0.0, -1.0, 0.2)), BACK);
        assert_eq!(sector(Vec3::new(-1.0, 0.5, 0.0)), LEFT);
        assert_eq!(sector(Vec3::new(1.0, 0.5, 0.0)), RIGHT);
        assert_eq!(sector(Vec3::new(0.1, 0.5, 1.0)), TOP);
        assert_eq!(sector(Vec3::new(0.1, 0.5, -1.0)), BOTTOM);
        assert_eq!(sector(Vec3::new(1.0, 1.0, 0.0)), FRONT, "front and right tie");
        assert_eq!(sector(Vec3::ZERO), FRONT);
        // Turned a quarter left, the unit's front faces world -x.
        let turned = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        assert_eq!(Shield::sector_of(Vec3::ZERO, turned, Vec3::new(-5.0, 0.0, 0.0)), FRONT);
    }

    #[test]
    fn a_sector_stops_its_deflected_strength_and_pays_for_it_over_the_coefficient() {
        let mut s = shield();
        assert!((s.strength(FRONT) - 245.0).abs() < 1e-3, "0.7 × 350");
        let stopped = s.absorb(FRONT, 100.0);
        assert_eq!(stopped, 100.0);
        // 100 stopped at 0.7 costs 142.9 of the sector's 350 points.
        assert!((s.fills[FRONT] * 350.0 - (350.0 - 100.0 / 0.7)).abs() < 1e-2);
        // A hit past what is left is stopped only in part, and the sector is spent.
        let left = s.strength(FRONT);
        assert!((s.absorb(FRONT, 1000.0) - left).abs() < 1e-3);
        assert!(s.fills[FRONT] < 1e-4);
        assert_eq!(s.fills[BACK], 1.0, "the other sectors keep theirs");
    }

    #[test]
    fn an_unpowered_or_broken_deflector_stops_nothing() {
        let mut s = shield();
        s.level = 0.0;
        assert_eq!(s.absorb(LEFT, 50.0), 0.0);
        let mut s = shield();
        s.deflector_condition = 0.0;
        assert!(!s.up());
        assert_eq!(s.strength(LEFT), 0.0);
    }

    #[test]
    fn a_powered_shield_recharges_by_what_each_sector_lacks_and_an_unpowered_one_drains() {
        let mut s = shield();
        s.shield_power = 0.8;
        s.fills = [0.0, 0.5, 1.0, 1.0, 1.0, 1.0];
        s.tick(1.0);
        // 4 points a second, shared 2 : 1 between the sectors lacking 350 and 175.
        let points = |f: f32, was: f32| (f - was) * 350.0;
        assert!((points(s.fills[0], 0.0) - 8.0 / 3.0).abs() < 1e-3, "{:?}", s.fills);
        assert!((points(s.fills[1], 0.5) - 4.0 / 3.0).abs() < 1e-3, "{:?}", s.fills);
        assert_eq!(s.fills[2], 1.0);

        let mut s = shield();
        s.shield_power = 0.8;
        s.level = 0.0;
        s.tick(1.0);
        // The idle draw unserved: 0.8 / 0.03 points out, shared by what each holds.
        let lost: f32 = s.fills.iter().map(|f| (1.0 - f) * 350.0).sum();
        assert!((lost - 0.8 / 0.03).abs() < 1e-2, "{lost}");
    }

    #[test]
    fn a_round_meets_the_bubble_on_its_surface_but_not_from_inside_it() {
        let centre = Vec3::new(0.0, 10.0, 0.0);
        let (point, d2) = contact(Vec3::ZERO, Vec3::new(0.0, 20.0, 0.0), 0.5, centre, 3.0).unwrap();
        assert!((point - Vec3::new(0.0, 7.0, 0.0)).length() < 1e-4, "{point}");
        assert!((d2 - 49.0).abs() < 1e-3);
        assert_eq!(contact(Vec3::new(0.0, 8.0, 0.0), Vec3::new(0.0, 30.0, 0.0), 0.5, centre, 3.0), None);
        assert_eq!(contact(Vec3::ZERO, Vec3::new(0.0, 5.0, 0.0), 0.5, centre, 3.0), None, "short of it");
        assert_eq!(contact(Vec3::ZERO, Vec3::new(0.0, -20.0, 0.0), 0.5, centre, 3.0), None, "away");
    }
}
