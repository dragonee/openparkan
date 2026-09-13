//! A gun: a capacitor, a magazine and a clock. `docs/29-weapons.md`, "A gun is a
//! capacitor, a magazine and a clock" and "Firing, from button to round".
//!
//! A gun wakes when its next event is due, and either continues a barrel stroke or
//! starts one. The round leaves halfway through the stroke; the interval starts
//! only once the stroke is done.

use parkan_formats::control::{CHANNEL_FOLLOWS, Channel, Component};

pub const MAGAZINE: usize = 0;
pub const CAPACITOR: usize = 1;
pub const SHOT_ENERGY: usize = 2;
pub const INTERVAL: usize = 3;
/// A magazine of −1 never empties.
pub const UNLIMITED: i32 = -1;
/// Component record +8: every barrel fires at once.
pub const SALVO: u32 = 0x0200_0000;
/// The state words a gun keeps (`0x1002a100`).
pub const STATE_OFF: i32 = 0;
pub const CONTINUE_FIGHT: i32 = 0x100;
pub const SINGLE_FIGHT: i32 = 0x200;
/// Barrel stroke steps, counting down (`0x1002a190`).
const STROKE: u8 = 4;

/// One barrel: its channel and where its stroke is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barrel {
    pub channel: usize,
    /// The channel's rate, value per second.
    pub rate: f32,
    /// Steps left of the stroke, 0 when idle.
    pub step: u8,
    /// The channel's move: from `from` at `start_ms` to `to` at `end_ms`.
    pub from: f32,
    pub to: f32,
    pub start_ms: f64,
    pub end_ms: f64,
}

impl Barrel {
    /// The channel's value at `t_ms`, linear across its move.
    pub fn value(&self, t_ms: f64) -> f32 {
        if self.end_ms <= self.start_ms || t_ms >= self.end_ms {
            return self.to;
        }
        let s = ((t_ms - self.start_ms) / (self.end_ms - self.start_ms)).clamp(0.0, 1.0) as f32;
        self.from + s * (self.to - self.from)
    }

    /// Head for `to` from wherever the channel is, at the channel's rate; returns
    /// when it arrives (`0x10022120`).
    fn head_for(&mut self, to: f32, t_ms: f64) -> f64 {
        self.from = self.value(t_ms);
        self.to = to;
        self.start_ms = t_ms;
        let ms = if self.rate > 0.0 { 1000.0 * f64::from((to - self.from).abs() / self.rate) } else { 0.0 };
        self.end_ms = t_ms + ms;
        self.end_ms
    }
}

/// A round leaving a barrel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub barrel: usize,
    pub t_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gun {
    /// The component's index among its controller's components.
    pub component: usize,
    pub barrels: Vec<Barrel>,
    pub current: usize,
    pub magazine: i32,
    pub rounds: i32,
    pub capacitor: f32,
    pub charge: f32,
    pub shot_energy: f32,
    pub interval_ms: f32,
    pub salvo: bool,
    /// The state word the fire rows set.
    pub state: i32,
    /// Whether the fire button reaches this gun.
    pub selected: bool,
    /// The ready byte (`+0x118`): the constructor sets it, and a turret's takt rewrites
    /// it from the gun's mount.
    pub ready: bool,
    /// Its round's top speed (`+0x94`), and whether the round falls (`+0x98`: its
    /// controller's mode is not 0, `0x100297ef`).
    pub round_speed: f32,
    pub falls: bool,
    start_ms: f64,
    next_ms: f64,
    started: bool,
}

impl Gun {
    /// A class-2 component and the controller's channels its entries name.
    pub fn new(component: usize, record: &Component, channels: &[Channel]) -> Self {
        let barrels = record
            .entries
            .iter()
            .filter_map(|&e| usize::try_from(e).ok())
            .filter(|&e| e < channels.len() && channels[e].flags & CHANNEL_FOLLOWS == 0)
            .map(|e| Barrel {
                channel: e,
                rate: channels[e].rate,
                step: 0,
                from: 0.0,
                to: 0.0,
                start_ms: 0.0,
                end_ms: 0.0,
            })
            .collect();
        let v = record.values;
        Self {
            component,
            barrels,
            current: 0,
            magazine: v[MAGAZINE] as i32,
            rounds: v[MAGAZINE] as i32,
            capacitor: v[CAPACITOR],
            charge: v[CAPACITOR],
            shot_energy: v[SHOT_ENERGY],
            interval_ms: v[INTERVAL],
            salvo: record.flags & SALVO != 0,
            state: STATE_OFF,
            selected: false,
            ready: true,
            round_speed: 0.0,
            falls: false,
            start_ms: 0.0,
            next_ms: 0.0,
            started: false,
        }
    }

    /// `MCMD_SELECT` on this gun: selecting resets it, deselecting switches it off.
    pub fn toggle(&mut self) {
        self.selected = !self.selected;
        if self.selected {
            self.reset();
        } else {
            self.state = STATE_OFF;
        }
    }

    /// State `0x1000`: the gun's status and delay cleared, its state word kept.
    pub fn reset(&mut self) {
        self.started = false;
    }

    /// Top the capacitor up.
    ///
    /// STAND-IN: docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured
    /// -- a gun draws what its capacitor lacks on the power tick, which is not
    /// modelled; the capacitor is full again every tick.
    pub fn recharge(&mut self) {
        self.charge = self.capacitor;
    }

    /// Run every event due by `now_ms` (`Control.dll:0x1002d260`); returns the rounds
    /// that left.
    pub fn tick(&mut self, now_ms: f64) -> Vec<Shot> {
        let mut shots = Vec::new();
        let mut guard = 0;
        while (now_ms > self.next_ms || !self.started) && guard < 64 {
            if self.next_ms == self.start_ms || !self.started {
                self.next_ms = now_ms;
            }
            self.start_ms = self.next_ms;
            self.fire_step(self.start_ms, &mut shots);
            self.started = true;
            guard += 1;
        }
        shots
    }

    /// `0x10029ca0`: continue a stroke, or start one when the gun has rounds, charge,
    /// its ready byte and a state.
    fn fire_step(&mut self, t: f64, shots: &mut Vec<Shot>) {
        if self.barrels.is_empty() {
            return;
        }
        let stroking: Vec<usize> = (0..self.barrels.len()).filter(|&b| self.barrels[b].step != 0).collect();
        if !stroking.is_empty() {
            for b in stroking {
                self.advance(b, t, shots);
            }
            return;
        }
        if self.rounds == 0
            || (self.capacitor > 0.0 && self.charge < self.shot_energy)
            || !self.ready
            || self.state == STATE_OFF
        {
            return;
        }
        let firing: Vec<usize> =
            if self.salvo { (0..self.barrels.len()).collect() } else { vec![self.current] };
        for b in firing {
            self.barrels[b].step = STROKE;
            self.advance(b, t, shots);
        }
    }

    /// `0x1002a190`: one step of a barrel's stroke.
    fn advance(&mut self, b: usize, t: f64, shots: &mut Vec<Shot>) {
        self.barrels[b].step -= 1;
        match self.barrels[b].step {
            3 => self.next_ms = self.barrels[b].head_for(0.5, t),
            2 => {
                shots.push(Shot { barrel: b, t_ms: t });
                self.next_ms = t;
            }
            1 => self.next_ms = self.barrels[b].head_for(1.0, t),
            _ => {
                let barrel = &mut self.barrels[b];
                barrel.from = 0.0;
                barrel.to = 0.0;
                barrel.end_ms = t;
                if self.magazine != UNLIMITED && self.rounds > 0 {
                    self.rounds -= 1;
                }
                if self.capacitor > 0.0 && self.charge >= self.shot_energy {
                    self.charge -= self.shot_energy;
                }
                self.next_ms = t + f64::from(self.interval_ms);
                if !self.salvo {
                    self.current = (self.current + 1) % self.barrels.len();
                }
                if self.state == SINGLE_FIGHT {
                    self.state = STATE_OFF;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(rate: f32, flags: i32) -> Channel {
        Channel {
            node: 0,
            first: 58.0,
            last: 60.0,
            initial: 0.0,
            origin: -1,
            point: 0,
            rate,
            span: 1.0,
            flags,
        }
    }

    fn gun(values: [f32; 4], rates: &[f32]) -> Gun {
        let channels: Vec<Channel> = rates.iter().map(|&r| channel(r, 0)).collect();
        let mut v = [0.0; 16];
        v[..4].copy_from_slice(&values);
        let record = Component {
            type_id: 2,
            resource: Default::default(),
            index: None,
            entries: (0..rates.len() as i32).collect(),
            label: String::new(),
            values: v,
            power: 0.0,
            node: 0,
            mass: 0.0,
            flags: 0,
            group: -1,
        };
        Gun::new(0, &record, &channels)
    }

    /// Fire with the button held for `ms`, at 60 ticks a second; the shot times.
    fn hold(g: &mut Gun, ms: f64) -> Vec<f64> {
        g.state = CONTINUE_FIGHT;
        let mut out = Vec::new();
        let mut t = 0.0;
        while t <= ms {
            g.recharge();
            out.extend(g.tick(t).into_iter().map(|s| s.t_ms));
            t += 1000.0 / 60.0;
        }
        out
    }

    #[test]
    fn the_cannon_fires_four_a_second_from_its_stroke_alone() {
        let mut cannon = gun([500.0, 20.0, 0.1, 0.0], &[4.0]);
        let shots = hold(&mut cannon, 1000.0);
        assert_eq!(shots.len(), 4, "{shots:?}");
        let gaps: Vec<f64> = shots.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|g| (g - 250.0).abs() < 17.0), "{gaps:?}");
        assert_eq!(cannon.rounds, 497, "the fourth stroke is still finishing");
    }

    #[test]
    fn the_missiles_alternate_barrels_and_wait_their_interval() {
        let mut missiles = gun([4.0, 0.8, 0.2, 1250.0], &[1.0, 2.5]);
        let shots = hold(&mut missiles, 4000.0);
        // Roc_d1 at 0.5 s, then its stroke and 1,250 ms; Roc_d2 0.2 s into its own.
        assert_eq!(shots.len(), 2, "{shots:?}");
        assert!(
            (shots[0] - 500.0).abs() < 17.0 && (shots[1] - (1000.0 + 1250.0 + 200.0)).abs() < 50.0,
            "{shots:?}"
        );
        assert_eq!(missiles.current, 0);
    }

    #[test]
    fn a_stroke_finishes_after_release_and_an_empty_magazine_stops_the_gun() {
        let mut laser = gun([2.0, 200.0, 5.5, 200.0], &[4.0]);
        laser.state = CONTINUE_FIGHT;
        assert_eq!(laser.tick(0.0).len(), 0);
        laser.state = STATE_OFF;
        assert_eq!(laser.tick(200.0).len(), 1, "the round still leaves");
        let shots = hold(&mut laser, 3000.0);
        assert_eq!((shots.len(), laser.rounds), (1, 0));
        let mut free = gun([-1.0, 200.0, 5.5, 200.0], &[4.0]);
        assert_eq!((hold(&mut free, 3000.0).len(), free.rounds), (7, -1));
    }

    #[test]
    fn a_gun_that_is_not_ready_starts_no_stroke_but_finishes_the_one_it_started() {
        let mut laser = gun([-1.0, 200.0, 5.5, 200.0], &[4.0]);
        laser.ready = false;
        laser.state = CONTINUE_FIGHT;
        assert!(laser.tick(0.0).is_empty() && laser.tick(1000.0).is_empty());
        assert_eq!(laser.barrels[0].step, 0);
        laser.ready = true;
        assert!(laser.tick(1100.0).is_empty(), "the stroke starts");
        laser.ready = false;
        assert_eq!(laser.tick(1230.0).len(), 1, "the round still leaves");
        assert!(laser.tick(3000.0).is_empty(), "and no stroke follows");
    }

    #[test]
    fn without_charge_no_stroke_starts_unless_the_gun_has_no_capacitor() {
        let mut laser = gun([-1.0, 200.0, 5.5, 200.0], &[4.0]);
        laser.charge = 1.0;
        laser.state = CONTINUE_FIGHT;
        assert!(laser.tick(0.0).is_empty() && laser.tick(1000.0).is_empty());
        let mut animal = gun([-1.0, 0.0, 5.5, 200.0], &[4.0]);
        animal.charge = 0.0;
        animal.state = CONTINUE_FIGHT;
        assert_eq!(animal.tick(0.0).len() + animal.tick(200.0).len(), 1);
    }
}
