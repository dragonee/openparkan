//! A control system's critical damage: when it runs block entries 6 and 7, and the life fraction
//! its owner's effects read (docs/13-control.md, "Critical damage: block entries 6 and 7").
//!
//! The control tick hands it the seconds since the last life update (`Control.dll:0x1000c7d8`),
//! and an agent of kind 10, a tree or a stone, goes there and nowhere else (`0x1000c294`), so a
//! tree's entries run as a machine's do. Once critical a machine burns, losing a tenth of its
//! life at that moment a second (`0x10012c1c`–`0x10012c37`); that loss is the wreck's burn of
//! docs/26-damage.md and is not here.

/// The share of the total life under which an agent of kind 10 is critical, and anything else's
/// (`Control.dll:0x1000fa82`–`0x1000faaf`, the constants written by `0x100063c5`, `0x100063cf`).
pub const SCENERY_MARK: f32 = 0.3;
pub const MARK: f32 = 0.2;

/// What a life update did to the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    /// It became critical: block entry 6 runs, then condition byte 14 is set (`0x10012bfb`).
    Critical,
    /// It is critical no more: block entry 7 runs, then the byte is cleared (`0x10012b12`).
    Recovered,
}

/// The life update's own state: the mark at `+0x594` and condition byte 14 (`+0x386`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Critical {
    /// Under it the machine is critical. The load makes it a share of the total (`0x1000fa82`)
    /// and becoming critical moves it to the life left then (`0x10012c09`–`0x10012c16`).
    pub mark: f32,
    /// Condition byte 14.
    pub on: bool,
    /// The life left the last update saw: the update runs only after the life changed, under
    /// the control system's byte `+0x598` (`0x10012a83`).
    seen: f32,
}

impl Critical {
    /// A control system just loaded with `full` life in all, an agent of kind 10 or not.
    pub fn new(full: f32, scenery: bool) -> Self {
        Self { mark: full * if scenery { SCENERY_MARK } else { MARK }, on: false, seen: full }
    }

    /// One life update with `total` life left (`Control.dll:0x10012a40`). `gear` is the running
    /// gear's life fractions summed and how many nodes carry it (`.ndp` flags `0x70`); with two or
    /// more, the machine is critical when the sum is half their number less one or under
    /// (`0x10012ab2`–`0x10012b04`), and otherwise when its life left is under the mark
    /// (`0x10012bdc`–`0x10012bed`).
    pub fn update(&mut self, total: f32, gear: Option<(f32, usize)>) -> Option<Turn> {
        if total == self.seen {
            return None;
        }
        self.seen = total;
        let critical = match gear {
            Some((sum, n)) if n > 1 => sum <= n as f32 * 0.5 - 1.0,
            _ => total < self.mark,
        };
        match (critical, self.on) {
            (true, false) => {
                self.on = true;
                self.mark = total;
                Some(Turn::Critical)
            }
            (false, true) => {
                self.on = false;
                Some(Turn::Recovered)
            }
            _ => None,
        }
    }
}

/// Property `0x31`, the life fraction a time-mode-14 effect reads (`Effect.dll:0x10005f2d`): the
/// control system's life left over its life at load, `+0x590 ÷ +0x58c` (`Control.dll:0x1000df55`).
/// 1 where it has none.
pub fn life_fraction(total: f32, full: f32) -> f32 {
    if full > 0.0 { total / full } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree of 100 is critical under 30, a machine under 20; the mark then moves to the life
    /// left, so the next loss keeps it critical and a gain back over it ends it.
    #[test]
    fn a_tree_is_critical_under_three_tenths_and_a_machine_under_two() {
        let mut tree = Critical::new(100.0, true);
        assert_eq!(tree.update(31.0, None), None, "over 30");
        assert_eq!(tree.update(29.0, None), Some(Turn::Critical));
        assert_eq!(tree.mark, 29.0, "the mark moves to the life left as it goes critical");
        assert_eq!(tree.update(20.0, None), None, "still critical, entry 6 not run again");
        assert_eq!(tree.update(25.0, None), None, "under the mark");
        assert_eq!(tree.update(29.5, None), Some(Turn::Recovered), "over the moved mark");

        let mut machine = Critical::new(100.0, false);
        assert_eq!(machine.update(25.0, None), None, "a machine at 25 is not");
        assert_eq!(machine.update(19.0, None), Some(Turn::Critical));
    }

    /// The update runs only on a change of life: at the life it went critical with, with
    /// nothing lost since, it does not end.
    #[test]
    fn nothing_turns_while_the_life_stands() {
        let mut tree = Critical::new(100.0, true);
        assert_eq!(tree.update(10.0, None), Some(Turn::Critical));
        assert_eq!(tree.update(10.0, None), None, "the life has not changed");
        assert!(tree.on);
    }

    /// With two or more nodes of running gear the gear decides: their life fractions summed to
    /// half their number less one, and the total is not asked. One node of gear does not count.
    #[test]
    fn running_gear_decides_where_there_are_two_nodes_of_it() {
        let mut walker = Critical::new(100.0, false);
        assert_eq!(walker.update(10.0, Some((0.5, 2))), None, "a leg left: not critical at 10");
        assert_eq!(walker.update(90.0, Some((0.0, 2))), Some(Turn::Critical), "both legs gone at 90");
        let mut one = Critical::new(100.0, false);
        assert_eq!(one.update(10.0, Some((1.0, 1))), Some(Turn::Critical), "one node: the total");
    }

    #[test]
    fn the_life_fraction_is_the_life_left_over_the_life_at_load() {
        assert_eq!(life_fraction(25.0, 100.0), 0.25);
        assert_eq!(life_fraction(0.0, 0.0), 1.0);
    }
}
