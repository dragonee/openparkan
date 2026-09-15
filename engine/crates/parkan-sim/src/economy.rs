//! Ore and power as `Behavior.dll` moves them: a building's batteries and the level its
//! efficiency is served at, a mine's digging, and the distribution step's shares. See
//! `docs/23-economy.md`, "A power shortage lowers efficiency", "How ore reaches a consumer"
//! and "Mission 03's economy, tick by tick".

/// The behaviour constants (`Behavior.dll:0x10016250`): a mine's dig a second and its most,
/// a storage's most, a transport's most and its load and unload a second.
pub const MINE_ORE_PER_SECOND: f32 = 50.0;
pub const MINE_MAX_ORE: f32 = 500.0;
pub const STORAGE_MAX_ORE: f32 = 4000.0;
pub const TRANSPORT_MAX_ORE: f32 = 2000.0;
pub const TRANSPORT_ORE_PER_SECOND: f32 = 100.0;
/// A mine's lodes lie within this across the ground (`Behavior.dll:0x10020f70`).
pub const LODE_REACH: f32 = 250.0;
/// Below this efficiency a mine logs `BAD KPD` and holds nothing (`0x1002cff6`).
pub const BAD_KPD: f32 = 0.1;
/// The efficiency component's own draw a second (docs/23, "So a building's efficiency is
/// served first and alone").
pub const EFFICIENCY_POWER: f32 = 0.01;
/// A consumer asks for nothing under this (`0x1001a1af`).
pub const WANT_FLOOR: f32 = 0.01;
/// The Ore row's scale: a full mine and a full storage (`iron3d.dll:0x1006d8f0`).
pub const ORE_ROW_SCALE: f32 = 4500.0;
/// The distribution step's timer, 3 × 64 ms and a random 0..63 (`Behavior.dll:0x10019e1b`).
pub const STEP_MS: f64 = 192.0;
pub const STEP_RANDOM_MS: f64 = 64.0;
/// A building's power tick, 250 ± 31 ms (`Control.dll:0x1000c756`).
pub const POWER_TICK_MS: f64 = 250.0;
pub const POWER_JITTER_MS: f64 = 31.0;

/// A building's batteries, its class-19 components together: their capacity, their output a
/// second, and their charge 0..1. A negative capacity reads full whatever is drawn (a
/// generator's); none at all serves every draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Battery {
    pub capacity: f32,
    pub output: f32,
    pub charge: f32,
}

impl Battery {
    pub fn new(capacity: f32, output: f32) -> Self {
        Self { capacity, output, charge: 1.0 }
    }

    /// One power tick of `dt` seconds with the efficiency drawing `need` a second
    /// (`Control.dll:0x1002d340`): the batteries give `min(output × charge × dt, capacity ×
    /// charge)`, and the level the draw is served at is what they give over what it wants,
    /// held to 1 (`0x1002dca0`); the charge falls by what was used. Returns the level and the
    /// charge used.
    pub fn spend(&mut self, need: f32, dt: f32) -> (f32, f32) {
        if self.capacity <= 0.0 {
            return (1.0, 0.0);
        }
        let want = need * dt;
        let give = (self.output * self.charge * dt).min(self.capacity * self.charge);
        let level = if want > 0.0 { (give / want).min(1.0) } else { 1.0 };
        let used = want.min(give);
        self.charge = (self.charge - used / self.capacity).max(0.0);
        (level, used)
    }

    /// What the batteries lack of full (`Control.dll:0x1002b42b`, `0x1002b4e9`).
    pub fn lack(&self) -> f32 {
        if self.capacity <= 0.0 { 0.0 } else { self.capacity * (1.0 - self.charge) }
    }

    /// Topped up by `share` of what they lack (`0x1002bae6`).
    pub fn top_up(&mut self, share: f32) {
        if self.capacity > 0.0 {
            self.charge += (1.0 - self.charge) * share.clamp(0.0, 1.0);
        }
    }
}

/// `M_Task_Mine`: the lodes' amount it may dig and the running total dug.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mine {
    pub to_mine: f32,
    pub total: f32,
}

impl Mine {
    /// A mine over lodes of `to_mine` in all; `None` when there are none (the order refused).
    pub fn new(to_mine: f32) -> Option<Self> {
        (to_mine > 0.0).then_some(Self { to_mine, total: 0.0 })
    }

    /// One takt of `dt` seconds at efficiency `kpd` with the ore property's most `most`
    /// (`0x1002cf60`): the ore the mine now holds, or `None` once all is mined. With the
    /// efficiency under 0.1 the total and the held ore fall to 0.
    pub fn takt(&mut self, dt: f32, kpd: f32, most: f32) -> Option<f32> {
        if kpd < BAD_KPD {
            self.total = 0.0;
            return Some(0.0);
        }
        let dig = (dt * MINE_ORE_PER_SECOND * kpd).min((most - self.total).max(0.0));
        if self.to_mine - self.total <= dig {
            return None;
        }
        self.total += dig;
        Some(self.total)
    }
}

/// The share of what each asks for the distribution step gives when `available` is shared
/// over `total` asked (`0x10019e80`, every priority equal): `min(1, available / total)`.
pub fn share(available: f32, total: f32) -> f32 {
    if total <= 0.0 { 0.0 } else { (available / total).min(1.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_factorys_batteries_serve_a_builds_draw_and_run_down_only_past_their_charge() {
        // Four batteries of 5 putting out 12.5 each.
        let mut b = Battery::new(20.0, 50.0);
        let (level, used) = b.spend(EFFICIENCY_POWER + 4.0, 0.25);
        assert_eq!(level, 1.0);
        assert!((used - 1.0025).abs() < 1e-5 && (b.charge - (1.0 - 1.0025 / 20.0)).abs() < 1e-5);
        // Nearly empty, the draw is served at what the batteries give.
        b.charge = 0.01;
        let (level, _) = b.spend(4.01, 1.0);
        assert!((level - 0.2 / 4.01).abs() < 1e-4, "{level}");
        assert!((b.lack() - 20.0).abs() < 0.01);
        b.top_up(0.5);
        assert!((b.charge - 0.5).abs() < 1e-3);
        // A generator reads full whatever it gives.
        let mut g = Battery::new(-1.0, 0.0);
        assert_eq!((g.spend(1.0, 1.0), g.lack()), ((1.0, 0.0), 0.0));
    }

    #[test]
    fn a_mine_digs_50_a_second_to_500_and_holds_nothing_below_a_tenth() {
        let mut m = Mine::new(1e19).unwrap();
        assert_eq!(m.takt(1.0, 1.0, MINE_MAX_ORE), Some(50.0));
        for _ in 0..20 {
            m.takt(1.0, 1.0, MINE_MAX_ORE);
        }
        assert_eq!(m.total, 500.0);
        assert_eq!(m.takt(0.2, 0.05, MINE_MAX_ORE), Some(0.0));
        assert_eq!(Mine::new(0.0), None);
        let mut small = Mine::new(60.0).unwrap();
        assert_eq!(small.takt(1.0, 1.0, MINE_MAX_ORE), Some(50.0));
        assert_eq!(small.takt(1.0, 1.0, MINE_MAX_ORE), None, "all ore mined");
    }

    #[test]
    fn every_asker_gets_the_same_share_of_what_it_asks() {
        assert_eq!(share(1.0, 4.0), 0.25);
        assert_eq!(share(10.0, 4.0), 1.0);
        assert_eq!(share(1.0, 0.0), 0.0);
    }
}
