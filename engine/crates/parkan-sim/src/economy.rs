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

/// The distribution step's random source: `Behavior.dll`'s own copy of the C library's
/// `rand()` (`0x1004ce3c`), one stream for the whole module, which nothing in the module
/// seeds, so it starts at 1 (`0x10063c1c`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModuleRand(pub u32);

impl Default for ModuleRand {
    fn default() -> Self {
        Self(1)
    }
}

impl ModuleRand {
    /// The next draw, 0..32767: the state times 214013 plus 2531011, bits 16 to 30.
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.0 >> 16) & 0x7fff
    }

    /// A timer's random share (`0x1004c569`): the draw's low byte times `units` × 64 ms over
    /// 256. The distribution step's timer takes one unit of it on its three (`0x10019e1b`), so
    /// 0..63 ms.
    pub fn timer_share_ms(&mut self, units: u32) -> f64 {
        f64::from(((self.next() & 0xff) * (units << 6)) >> 8)
    }
}

/// The power tick's jitter (`Control.dll:0x1000c756`): two 16-bit words the whole module
/// shares, stepped on every controller's tick. The first becomes itself doubled crossed with
/// the second; the second, itself halved crossed with the new first; and the second's value
/// over 65,536 moves the 250 ms by a quarter of it either side of centre, so ±31.25 ms. Two
/// zero words stay zero, so it wants [`ShiftJitter::seeded`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShiftJitter {
    pub a: u16,
    pub b: u16,
}

impl ShiftJitter {
    /// The words a dword seeds, low word first (`0x1000dc39`).
    pub fn seeded(seed: u32) -> Self {
        Self { a: seed as u16, b: (seed >> 16) as u16 }
    }

    /// The next tick's offset from [`POWER_TICK_MS`], ms.
    pub fn next_ms(&mut self) -> f64 {
        self.a = self.a.wrapping_shl(1) ^ self.b;
        self.b = (self.b >> 1) ^ self.a;
        POWER_TICK_MS * 0.25 * (f64::from(self.b) / 65_536.0 - 0.5)
    }
}

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

    /// What a unit's battery gives over `dt` seconds at `condition`, its node's life over its
    /// maximum (`Control.dll:0x100229a0`): `min(output × charge × condition × dt, capacity ×
    /// charge × condition)`.
    pub fn supply(&self, dt: f32, condition: f32) -> f32 {
        if self.capacity <= 0.0 || condition <= 0.0 {
            return 0.0;
        }
        (self.output * self.charge * condition * dt).min(self.capacity * self.charge * condition)
    }

    /// `used` of what it gave taken from its charge, served last (`0x1002d439`).
    pub fn drain(&mut self, used: f32) {
        if self.capacity > 0.0 {
            self.charge = (self.charge - used.max(0.0) / self.capacity).max(0.0);
        }
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

/// What one of a mine's takts leaves (docs/23, "A mine digs to 500").
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dug {
    /// The mine now holds this, and the task runs on (`0x1002d0e4`).
    Digging(f32),
    /// "All Ore mined..." (`0x1002d142`): the mine holds this — the last dig is banked
    /// whole — the progress goes to 1, `SetPowerUsage(0)` is called and the task ends.
    AllMined(f32),
    /// A takt that finds nothing left to mine ends the task at once and writes nothing
    /// (`0x1002cf68`).
    Spent,
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
    /// (`0x1002cf60`). With the efficiency under 0.1 the total and the held ore fall to 0
    /// and the task runs on.
    ///
    /// The dig is `dt × 50 × kpd`, capped so the total does not pass `most`; the takt then
    /// **adds it to the total and takes it off `to_mine`**, and ends when what stood
    /// between them before the dig was no more than the dig itself. The two therefore meet
    /// half way: a mine yields about `to_mine / 2`, or `most`, whichever is less.
    pub fn takt(&mut self, dt: f32, kpd: f32, most: f32) -> Dug {
        // The game's test is an x87 compare an unordered pair passes, so only a real zero
        // or less ends the takt here.
        if self.to_mine <= 0.0 {
            return Dug::Spent;
        }
        if kpd < BAD_KPD {
            self.total = 0.0;
            return Dug::Digging(0.0);
        }
        let dig = (dt * MINE_ORE_PER_SECOND * kpd).min((most - self.total).max(0.0));
        let left = self.to_mine - self.total;
        self.total += dig;
        self.to_mine -= dig;
        if left <= dig { Dug::AllMined(self.total) } else { Dug::Digging(self.total) }
    }

    /// The task's progress, the total over what is left to mine (`0x1002d109`); a task that
    /// has ended reads 1.
    pub fn progress(&self) -> f32 {
        if self.to_mine <= 0.0 { 1.0 } else { (self.total / self.to_mine).min(1.0) }
    }
}

/// The share of what each asks for the distribution step gives when `available` is shared
/// over `total` asked (`0x10019e80`, every priority equal): `min(1, available / total)`.
pub fn share(available: f32, total: f32) -> f32 {
    if total <= 0.0 { 0.0 } else { (available / total).min(1.0) }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_steps_rand_is_the_c_librarys_from_1_and_its_share_is_0_to_63_ms() {
        let mut r = super::ModuleRand::default();
        // The C library's first three draws from its unseeded state.
        assert_eq!([r.next(), r.next(), r.next()], [41, 18467, 6334]);
        let mut r = super::ModuleRand::default();
        let shares: Vec<f64> = (0..1000).map(|_| r.timer_share_ms(1)).collect();
        assert!(shares.iter().all(|&s| (0.0..64.0).contains(&s)));
        assert!(shares.iter().any(|&s| s >= 60.0) && shares.iter().any(|&s| s < 4.0));
    }

    #[test]
    fn the_power_ticks_jitter_stays_within_a_quarter_of_250_ms_about_it() {
        let mut j = super::ShiftJitter::seeded(0x1234_5678);
        let offsets: Vec<f64> = (0..1000).map(|_| j.next_ms()).collect();
        assert!(offsets.iter().all(|&o| (-31.25..31.25).contains(&o)), "{offsets:?}");
        assert!(offsets.iter().any(|&o| o > 20.0) && offsets.iter().any(|&o| o < -20.0));
        // Two zero words stay zero: the register needs its seed.
        let mut z = super::ShiftJitter::seeded(0);
        assert_eq!(z.next_ms(), -31.25);
    }

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
        // The smallest ToMine any of the 15 shipped mines carries is 999,999, so none of
        // them is ever bounded by its lode: it digs the full 500 and stops digging.
        let mut m = Mine::new(999_999.0).unwrap();
        assert_eq!(m.takt(1.0, 1.0, MINE_MAX_ORE), Dug::Digging(50.0));
        for _ in 0..20 {
            assert!(matches!(m.takt(1.0, 1.0, MINE_MAX_ORE), Dug::Digging(_)));
        }
        assert_eq!(m.total, 500.0);
        // Every 500 dug comes off ToMine, and nothing more.
        assert_eq!(m.to_mine, 999_999.0 - 500.0);
        // A full mine never runs dry: the dig is 0, so the task cannot reach its end.
        assert_eq!(m.takt(1.0, 1.0, MINE_MAX_ORE), Dug::Digging(500.0));
        assert_eq!(m.takt(0.2, 0.05, MINE_MAX_ORE), Dug::Digging(0.0), "BAD KPD holds nothing");
        assert_eq!(Mine::new(0.0), None);
    }

    #[test]
    fn a_takt_moves_the_total_and_the_lode_towards_each_other_so_a_mine_yields_half_its_lode() {
        // 600 of lode: the total climbs 50 a second and ToMine falls 50 a second, so they
        // meet at 300 and the seventh takt ends the task -- banking its dig all the same.
        let mut m = Mine::new(600.0).unwrap();
        for takt in 1..=6 {
            assert_eq!(m.takt(1.0, 1.0, MINE_MAX_ORE), Dug::Digging(50.0 * takt as f32), "takt {takt}");
        }
        assert_eq!((m.total, m.to_mine), (300.0, 300.0));
        assert_eq!(m.takt(1.0, 1.0, MINE_MAX_ORE), Dug::AllMined(350.0), "all ore mined");
        assert_eq!(m.progress(), 1.0);
        // A takt on a spent task ends it before anything else, writing nothing.
        let mut over = Mine { to_mine: 0.0, total: 120.0 };
        assert_eq!(over.takt(1.0, 1.0, MINE_MAX_ORE), Dug::Spent);
        assert_eq!(over.total, 120.0);
    }

    #[test]
    fn every_asker_gets_the_same_share_of_what_it_asks() {
        assert_eq!(share(1.0, 4.0), 0.25);
        assert_eq!(share(10.0, 4.0), 1.0);
        assert_eq!(share(1.0, 0.0), 0.0);
    }
}
