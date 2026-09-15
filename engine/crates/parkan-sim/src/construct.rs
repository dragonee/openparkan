//! A factory building a bot: `M_Task_Construct`'s three budgets, time, ore and power, and
//! the progress the factory screen shows. See `docs/23-economy.md`, "Construction", and
//! `docs/36-factory.md`, "Production".

/// A paid bot's time budget, whatever the bot (`Behavior.dll:0x10029ba0`).
pub const PAID_SECONDS: f32 = 5.0;
/// A free bot costs no ore and this much power (`0x10029f13`).
pub const FREE_POWER: f32 = 1.0;
/// A free bot's time for a factory size the table has no row for, or a chassis size it has
/// no column for.
pub const FREE_OTHER_SECONDS: f32 = 20.0;

/// A size as its letter counts it: a chassis's third character, a building's fourth
/// (`0x10029e10`, `0x1000cee0`): `t` 1, `l` and `h` 2, `m` 3, `b` 4, `e` 5.
pub fn size_of_letter(letter: u8) -> u8 {
    match letter.to_ascii_lowercase() {
        b't' => 1,
        b'l' | b'h' => 2,
        b'm' => 3,
        b'b' => 4,
        b'e' => 5,
        _ => 0,
    }
}

/// A free bot's time budget, seconds, by the factory's size and the chassis's (`0x1002a000`):
///
/// | factory | tiny | small | medium | large |
/// |---|---|---|---|---|
/// | small | 30 | 60 | — | — |
/// | medium | 20 | 35 | 60 | — |
/// | large | 10 | 20 | 40 | 60 |
pub fn free_seconds(factory: u8, chassis: u8) -> f32 {
    match (factory, chassis) {
        (2, 1) => 30.0,
        (2, 2) => 60.0,
        (3, 1) => 20.0,
        (3, 2) => 35.0,
        (3, 3) => 60.0,
        (4, 1) => 10.0,
        (4, 2) => 20.0,
        (4, 3) => 40.0,
        (4, 4) => 60.0,
        _ => FREE_OTHER_SECONDS,
    }
}

/// Whether a factory of `factory`'s size builds a chassis of `chassis`'s ("Robot SizedType
/// not match"): a factory refuses a chassis bigger than itself.
pub fn builds(factory: u8, chassis: u8) -> bool {
    chassis <= factory
}

/// One build's budgets and what has been collected toward them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Construct {
    pub seconds: f32,
    pub ore_cost: f32,
    pub power_cost: f32,
    pub time: f32,
    pub ore: f32,
    pub power: f32,
    /// A free bot, which spends one of the factory's `FreeBotNum` when it completes.
    pub free: bool,
}

impl Construct {
    /// A bot of chassis size `chassis` in a factory of size `factory`: free while the factory's
    /// `FreeBotNum` is above zero, else at its design's ore and power with 5 s of time.
    pub fn new(factory: u8, chassis: u8, free: bool, ore_cost: f32, power_cost: f32) -> Self {
        let (seconds, ore_cost, power_cost) = if free {
            (free_seconds(factory, chassis), 0.0, FREE_POWER)
        } else {
            (PAID_SECONDS, ore_cost, power_cost)
        };
        Self { seconds, ore_cost, power_cost, time: 0.0, ore: 0.0, power: 0.0, free }
    }

    /// The ore it asks for this takt (`0x1002a4f0`): `(cost − collected) × k × KPD × 0.2 + 0.07`,
    /// k being 0.2 below 20% collected, the share collected to 80%, and 3 above; nothing once
    /// all is in.
    pub fn request(&self, kpd: f32) -> f32 {
        if self.ore >= self.ore_cost {
            return 0.0;
        }
        let f = self.ore / self.ore_cost;
        let k = if f < 0.2 {
            0.2
        } else if f <= 0.8 {
            f
        } else {
            3.0
        };
        (self.ore_cost - self.ore) * k * kpd * 0.2 + 0.07
    }

    /// Whether its power is still to collect, while it draws `Use_Power` (`0x1002a308`,
    /// `0x1002a722`).
    pub fn drawing_power(&self) -> bool {
        self.power < self.power_cost
    }

    /// One takt of `dt` seconds with the efficiency `kpd` and a power use of `use_power` a
    /// second: time accrues as `dt` and power as KPD × use × dt, each held to its cost; the
    /// ore it takes is as much of its request as `held` holds, and `held` is left empty
    /// (`0x10015540`, `0x1002a604`). Returns the request, which is what the factory asks the
    /// next distribution step for.
    pub fn takt(&mut self, dt: f32, kpd: f32, use_power: f32, held: &mut f32) -> f32 {
        self.time = (self.time + dt).min(self.seconds);
        self.power = (self.power + kpd * use_power * dt).min(self.power_cost);
        let request = self.request(kpd);
        let take = held.max(0.0).min(request);
        self.ore = (self.ore + take).min(self.ore_cost);
        *held = 0.0;
        self.request(kpd)
    }

    /// The progress, min(time, ore, power) as fractions, held to 1.
    pub fn progress(&self) -> f32 {
        let share = |got: f32, cost: f32| if cost > 0.0 { got / cost } else { 1.0 };
        share(self.time, self.seconds)
            .min(share(self.ore, self.ore_cost))
            .min(share(self.power, self.power_cost))
            .min(1.0)
    }

    /// Complete when all three are collected.
    pub fn done(&self) -> bool {
        self.time >= self.seconds && self.ore >= self.ore_cost && self.power >= self.power_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_large_bot_in_a_large_factory_takes_a_minute() {
        let mut c = Construct::new(4, 4, true, 411.0, 226.5);
        assert_eq!((c.seconds, c.ore_cost, c.power_cost), (60.0, 0.0, 1.0));
        let mut t: f32 = 0.0;
        while !c.done() && t < 120.0 {
            c.takt(1.0 / 60.0, 1.0, 1.0, &mut 0.0);
            t += 1.0 / 60.0;
        }
        assert!((t - 60.0).abs() < 0.05, "done at {t}");
        assert_eq!(c.progress(), 1.0);
    }

    #[test]
    fn a_paid_bot_takes_only_the_ore_that_arrives_and_asks_again_for_the_rest() {
        // SSW-X in the Large Factory: 125.6 ore over efficiency 5, 72.7 power at 5 × 4 a second.
        let mut c = Construct::new(4, 2, false, 125.6 / 5.0, 72.7);
        let mut held = 0.0;
        let first = c.takt(0.25, 5.0, 4.0, &mut held);
        assert!((first - (25.12 * 0.2 * 5.0 * 0.2 + 0.07)).abs() < 1e-4, "{first}");
        assert_eq!(c.ore, 0.0, "nothing arrived");
        held = 0.22;
        c.takt(0.25, 5.0, 4.0, &mut held);
        assert!((c.ore - 0.22).abs() < 1e-6 && held == 0.0);
        // Power is in by 3.6 s; the time by 5; the ore at a second each second it arrives.
        let mut t: f32 = 0.5;
        while !c.done() && t < 60.0 {
            let mut arrived = 0.25;
            c.takt(0.25, 5.0, 4.0, &mut arrived);
            t += 0.25;
            if t > 3.7 {
                assert!(!c.drawing_power());
            }
        }
        assert!((t - 25.5).abs() < 0.6, "done at {t}");
    }

    #[test]
    fn the_free_table_and_sizes_read_as_the_letters_do() {
        assert_eq!(
            [free_seconds(2, 1), free_seconds(3, 2), free_seconds(4, 3), free_seconds(2, 3)],
            [30.0, 35.0, 40.0, 20.0]
        );
        assert_eq!(b"tlhmbe".map(size_of_letter), [1, 2, 2, 3, 4, 5]);
        assert!(builds(4, 4) && !builds(2, 3));
    }
}
