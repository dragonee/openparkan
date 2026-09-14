//! The parts that move by themselves: a generic device (class 3) or a radar (class 8)
//! stepping the channels it names — a T-2's rotors, a turret's dish, an M-2f's wings.
//!
//! Both are `Control.dll`'s base item (`0x10020800`; the radar's constructor `0x10024310`
//! calls it and keeps its update). The time driver (`0x1002d260`) starts an item's next
//! step once game time passes the last one's end, the update (`0x10020900`) sets where
//! each channel is headed and how long that takes, and `0x10021a30` plays the channels
//! across the step. See `docs/28-chassis.md`, "What a device's value turns", and
//! `docs/25-sensors.md`, "The dish turns while the radar runs".

use parkan_formats::control::{CHANNEL_WRAP, Channel, Component};

/// The switch word an item starts in: open and wrapping (`0x10020832`).
pub const DEFAULT_STATE: i32 = 5;
/// How far a step moves the progress, times the rate (`0x1003c488`).
pub const STEP: f32 = 0.45;
pub const OPENING: i32 = 1;
pub const CLOSING: i32 = 2;
pub const WRAP: i32 = 4;
pub const BOUNCE: i32 = 8;
/// A step that moves nothing lasts this long (`0x10020d72`).
pub const IDLE_MS: f64 = 100.0;
/// A rate this small does nothing (`0x1003b380`).
pub const MIN_RATE: f32 = 1e-9;
/// The flags word's top bit makes a set-outright channel a switch at its initial value
/// (`0x10020bf0`).
pub const THRESHOLD: u32 = 0x8000_0000;

/// What a selector byte reads of the machine (`0x10020d90`), by byte.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    /// Angular velocity (`+0x1d4`).
    pub spin: [f32; 3],
    /// The lean on each axis over triple 6.
    pub lean: [f32; 3],
    /// Velocity in the machine's frame (`+0x1c8`).
    pub velocity: [f32; 3],
    /// The authored top speed, the controller's third triple (`0x1002104c`).
    pub top: [f32; 3],
}

impl Motion {
    /// The source selector `byte` picks, or `default` for a byte that picks nothing.
    pub fn source(&self, byte: u32, default: f32) -> f32 {
        let over = |v: f32, top: f32| if top != 0.0 { v / top } else { 0.0 };
        let length = |v: [f32; 3]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
        match byte {
            2..=4 => self.spin[(byte - 2) as usize],
            5..=7 => -self.spin[(byte - 5) as usize],
            8..=10 => self.lean[(byte - 8) as usize],
            11..=13 => over(self.velocity[(byte - 11) as usize], self.top[(byte - 11) as usize]),
            14 => over(length(self.velocity), length(self.top)),
            _ => default,
        }
    }
}

/// A generic device or a radar and the channels it drives.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// The component in its controller.
    pub component: usize,
    pub flags: u32,
    pub gains: [f32; 2],
    /// The controller channels its entries name, and those channels.
    pub entries: Vec<usize>,
    pub channels: Vec<Channel>,
    /// The switch word (`+0x50`) and the progress (`+0x94`).
    pub state: i32,
    pub progress: f32,
    pub start_ms: f64,
    pub end_ms: f64,
    started: bool,
    /// Per channel: its value now, and where the step runs from and to. The parser
    /// leaves all three at −1 (`0x10021e35`).
    pub now: Vec<f32>,
    from: Vec<f32>,
    to: Vec<f32>,
}

impl Item {
    /// Component `component` of a controller with these channels.
    pub fn new(component: usize, record: &Component, channels: &[Channel]) -> Self {
        let entries: Vec<usize> = record
            .entries
            .iter()
            .filter_map(|&e| usize::try_from(e).ok())
            .filter(|&e| e < channels.len())
            .collect();
        let n = entries.len();
        Self {
            component,
            flags: record.flags,
            gains: record.weights,
            channels: entries.iter().map(|&e| channels[e]).collect(),
            entries,
            state: record.index.unwrap_or(DEFAULT_STATE),
            progress: 0.0,
            start_ms: 0.0,
            end_ms: 0.0,
            started: false,
            now: vec![-1.0; n],
            from: vec![-1.0; n],
            to: vec![-1.0; n],
        }
    }

    /// Switch the item on, to open, or off, to close: the switch word's low bits 1 or 2, with
    /// no end mode, so the progress holds at its end and the word clears (`0x10020a28`,
    /// docs/28, "What a device's value turns"). A door and a control pod stop this way.
    pub fn switch(&mut self, on: bool) {
        self.state = if on { OPENING } else { CLOSING };
    }

    /// Whether the switch word has cleared: the item has stopped at an end.
    pub fn stopped(&self) -> bool {
        self.state == 0
    }

    /// Bytes 1 and 2's sources, weighted: 1 and 0 where a byte picks nothing.
    pub fn rate(&self, motion: &Motion) -> f32 {
        let first = motion.source(self.flags >> 8 & 0xFF, 1.0);
        let second = motion.source(self.flags >> 16 & 0xFF, 0.0);
        first * self.gains[0] + second * self.gains[1]
    }

    /// One step (`0x10020900`), started at `start_ms`.
    fn update(&mut self, motion: &Motion, alive: bool) {
        if self.state == 0 || !alive {
            return;
        }
        if self.channels.is_empty() {
            self.end_ms = self.end_ms.max(self.start_ms + IDLE_MS);
            return;
        }
        let rate = self.rate(motion);
        if rate.abs() < MIN_RATE {
            return;
        }
        let selector = self.flags & 0xFF;
        if selector != 0 {
            self.progress = motion.source(selector, self.progress);
        } else {
            match self.state & 3 {
                OPENING => self.progress += rate * STEP,
                CLOSING => self.progress -= rate * STEP,
                _ => {}
            }
            let mode = self.state & 0xC;
            if mode == WRAP {
                if self.progress < 0.0 {
                    self.progress += 1.0;
                } else if self.progress > 1.0 {
                    self.progress -= 1.0;
                }
            } else if !(0.0..=1.0).contains(&self.progress) {
                self.progress = self.progress.clamp(0.0, 1.0);
                self.state = if mode != BOUNCE {
                    0
                } else if self.state & 1 != 0 {
                    (self.state & !1) | 2
                } else {
                    (self.state & !2) | 1
                };
            }
        }
        for (i, ch) in self.channels.iter().enumerate() {
            let value = match selector {
                0 => self.progress,
                // STAND-IN: docs/28-chassis.md#not-established -- what byte 0 = 1 adds from
                // the machine's list at +0xc4 (`0x10020c25`) is not read; the channel holds
                // its initial value.
                1 => ch.initial,
                _ if self.flags & THRESHOLD != 0 => {
                    if self.progress / ch.span < ch.initial {
                        0.0
                    } else {
                        1.0
                    }
                }
                _ => self.progress / ch.span + ch.initial,
            };
            self.from[i] = self.now[i];
            self.to[i] = value.clamp(0.0, 1.0);
            let mut gap = (self.to[i] - self.from[i]).abs();
            if ch.flags & CHANNEL_WRAP != 0 && gap > 0.5 {
                gap = 1.0 - gap;
            }
            if ch.rate > 0.0 {
                let ms = 1000.0 * f64::from(gap) / f64::from(rate.abs() * ch.rate);
                self.end_ms = self.end_ms.max(self.start_ms + ms);
            }
        }
        if self.end_ms - self.start_ms < 1.0 {
            self.end_ms = self.start_ms + IDLE_MS;
        }
    }

    /// Each channel's value at `t_ms` across the step (`0x10021a30`).
    fn play(&mut self, t_ms: f64) {
        let s = if t_ms >= self.end_ms || self.end_ms <= self.start_ms {
            1.0
        } else {
            ((t_ms - self.start_ms) / (self.end_ms - self.start_ms)) as f32
        };
        for (i, ch) in self.channels.iter().enumerate() {
            let mut gap = self.to[i] - self.from[i];
            self.now[i] = if ch.flags & CHANNEL_WRAP != 0 {
                if gap.abs() >= 0.5 {
                    gap -= gap.signum();
                }
                let v = self.from[i] + s * gap;
                if v > 1.0 {
                    v - 1.0
                } else if v < 0.0 {
                    v + 1.0
                } else {
                    v
                }
            } else {
                (self.from[i] + s * gap).clamp(0.0, 1.0)
            };
        }
    }

    /// The time driver at `t_ms`: every step due by then, each starting at the last one's
    /// end, then the channels played at `t_ms`. `alive` is whether the component's node
    /// still has life (slot 2, `0x10021820`).
    ///
    /// STAND-IN: docs/28-chassis.md#what-a-devices-value-turns--read-and-measured -- the
    /// game plays an item's channels every tick only while one of two countdowns runs, and
    /// who sets them is not read; they are played every tick, so a rotor turns smoothly
    /// rather than jumping from one step's end to the next.
    pub fn tick(&mut self, t_ms: f64, motion: &Motion, alive: bool) {
        while !self.started || t_ms > self.end_ms {
            self.play(t_ms);
            if !self.started || self.end_ms == self.start_ms {
                self.end_ms = t_ms;
            }
            self.start_ms = self.end_ms;
            self.update(motion, alive);
            self.started = true;
        }
        self.play(t_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(rate: f32, flags: i32, initial: f32) -> Channel {
        Channel { node: 0, first: 2.0, last: 6.0, initial, origin: -1, point: -1, rate, span: 1.0, flags }
    }

    fn record(flags: u32, entries: Vec<i32>) -> Component {
        Component {
            type_id: 3,
            resource: Default::default(),
            index: None,
            entries,
            label: String::new(),
            values: [0.0; 16],
            power: 0.0,
            node: 0,
            mass: 0.0,
            flags,
            group: -1,
            weights: [1.0, 1.0],
        }
    }

    /// The value a wrapping channel has gone round by from `a` to `b`, the short way.
    fn round(a: f32, b: f32) -> f32 {
        let d = b - a;
        d - d.round()
    }

    #[test]
    fn a_constant_device_turns_at_its_channels_rate() {
        // The T-2's rotor: flags 0, weights 1 and 1, a wrapping channel at 7.3.
        let mut item = Item::new(0, &record(0, vec![0]), &[channel(7.3, CHANNEL_WRAP, 0.0)]);
        let motion = Motion::default();
        let (mut t, mut turned) = (0.0, 0.0);
        item.tick(t, &motion, true);
        let mut last = item.now[0];
        while t < 2000.0 {
            t += 1000.0 / 60.0;
            item.tick(t, &motion, true);
            turned += round(last, item.now[0]);
            last = item.now[0];
        }
        // Two seconds at 7.3 turns a second, after the first step snaps −1 to 0.
        assert!((turned - 14.6).abs() < 0.05, "turned {turned}");
    }

    #[test]
    fn a_dead_or_switched_off_device_holds_where_its_last_step_ended() {
        let mut item = Item::new(0, &record(0, vec![0]), &[channel(0.5, CHANNEL_WRAP, 0.0)]);
        let motion = Motion::default();
        for k in 0..60 {
            item.tick(f64::from(k) * 50.0, &motion, true);
        }
        let held = item.end_ms;
        item.tick(held + 1.0, &motion, false);
        let v = item.now[0];
        for k in 1..30 {
            item.tick(held + f64::from(k) * 100.0, &motion, false);
            assert_eq!(item.now[0], v);
        }
    }

    #[test]
    fn a_switch_flips_at_its_initial_value_and_swings_at_the_channels_rate() {
        // An M-2f wing: byte 0 = 12, the forward speed over the top, a switch at 0.5.
        let mut item = Item::new(0, &record(THRESHOLD | 12, vec![0]), &[channel(1.0, 0, 0.5)]);
        let mut motion = Motion { top: [0.0, 34.7, 0.0], ..Motion::default() };
        item.tick(0.0, &motion, true);
        item.tick(200.0, &motion, true);
        assert_eq!(item.now[0], 0.0, "hovering it is at 0");
        motion.velocity[1] = 20.0;
        let mut t = 200.0;
        while item.now[0] < 1.0 && t < 3000.0 {
            t += 1000.0 / 60.0;
            item.tick(t, &motion, true);
        }
        // It notices within an idle step and swings over one second at a rate of 1.
        assert!((990.0..1150.0).contains(&(t - 200.0)), "swung by {t}");
    }

    #[test]
    fn a_switched_item_opens_in_steps_and_stops_at_its_end() {
        // A pod at rate 1 with a channel at 0.2: three steps of 0.45, 0.9, 1, each 2250 ms
        // long but the last, and the word clears as the third step starts (docs/27).
        let mut item = Item::new(0, &record(0, vec![0]), &[channel(0.2, 0, 0.0)]);
        item.state = 0;
        let motion = Motion::default();
        item.tick(0.0, &motion, true);
        item.switch(true);
        let mut t = 0.0;
        while !item.stopped() && t < 10_000.0 {
            t += 1000.0 / 60.0;
            item.tick(t, &motion, true);
        }
        // The idle step runs out at 100 ms, then two steps of 2250 ms.
        assert!((4500.0..4700.0).contains(&t), "stopped at {t}");
        while t < 6000.0 {
            t += 1000.0 / 60.0;
            item.tick(t, &motion, true);
        }
        assert_eq!(item.now[0], 1.0);
        item.switch(false);
        while t < 12_000.0 {
            t += 1000.0 / 60.0;
            item.tick(t, &motion, true);
        }
        assert!(item.stopped() && item.now[0] == 0.0);
    }

    #[test]
    fn selector_bytes_read_spin_velocity_and_speed_over_the_top() {
        let m = Motion {
            spin: [0.1, 0.2, 0.3],
            velocity: [3.0, 4.0, 0.0],
            top: [0.0, 8.0, 0.0],
            ..Motion::default()
        };
        assert_eq!(
            [m.source(4, 9.0), m.source(6, 9.0), m.source(12, 9.0), m.source(11, 9.0)],
            [0.3, -0.2, 0.5, 0.0]
        );
        assert_eq!((m.source(14, 9.0), m.source(1, 9.0)), (5.0 / 8.0, 9.0));
    }
}
