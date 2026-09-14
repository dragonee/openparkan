//! A gun: a capacitor, a magazine and a clock. `docs/29-weapons.md`, "A gun is a
//! capacitor, a magazine and a clock" and "Firing, from button to round".
//!
//! A gun wakes when its next event is due, and either continues a barrel stroke or
//! starts one. The round leaves halfway through the stroke; the interval starts
//! only once the stroke is done. A gun whose round has a range keeps a target gate,
//! and one whose round is guided waits for a target and a lock ("A guided gun waits
//! for a lock").

use glam::Vec3;
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
/// What a gun reports of its target gate (`+0x11c`): a shot may go, no target, the
/// target out of range, the target off the barrel.
pub const GATE_CLEAR: i32 = 1;
pub const GATE_NO_TARGET: i32 = 2;
pub const GATE_OUT_OF_RANGE: i32 = 7;
pub const GATE_OFF_BARREL: i32 = 8;

/// A gun's values 8, 10 and 9, as it fills them from the round it links
/// (`Control.dll:0x100297e0`–`0x10029907`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetGate {
    /// The round's range, cut to its seeker's reach; no gate when not positive.
    pub range: f32,
    /// The cosine of the seeker's cone, or −1.
    pub cone_cos: f32,
    /// The seeker's lock in seconds, or −1: below zero the gun is unguided.
    pub lock_s: f32,
}

impl TargetGate {
    pub const NONE: Self = Self { range: 0.0, cone_cos: -1.0, lock_s: -1.0 };

    /// The gate for a round of `range` with a seeker of (cone rad, reach, lock ms).
    pub fn new(range: f32, seeker: Option<(f32, f32, f32)>) -> Self {
        let range = range.max(0.0);
        match seeker {
            Some((cone, reach, lock_ms)) => {
                Self { range: range.min(reach), cone_cos: cone.cos(), lock_s: lock_ms * 0.001 }
            }
            None => Self { range, cone_cos: -1.0, lock_s: -1.0 },
        }
    }

    /// A turret in `CIS_MANUALCONTROL` hands its target only to such a gun (`0x10028164`).
    pub fn guided(&self) -> bool {
        self.lock_s >= 0.0
    }
}

/// What the gate looks at: where the unit is, which way the barrel point faces, and
/// where the gun's target is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sight {
    pub unit: Vec3,
    pub barrel: Vec3,
    pub target: Option<Vec3>,
}

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
    pub gate: TargetGate,
    pub sight: Sight,
    /// The target it hands its rounds (`+0x108`), by the caller's numbering.
    pub target: Option<usize>,
    /// The lock left, s (`+0x170`), and the gate's last report.
    pub lock: f32,
    pub report: i32,
    last_wake_ms: f64,
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
            gate: TargetGate::NONE,
            sight: Sight::default(),
            target: None,
            lock: TargetGate::NONE.lock_s,
            report: GATE_NO_TARGET,
            last_wake_ms: 0.0,
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

    /// Link its round (`0x100297ef`): the gate from the round, and the lock full.
    pub fn link(&mut self, gate: TargetGate) {
        self.gate = gate;
        self.lock = gate.lock_s;
    }

    /// The turret relinks it with `target` (`0x10028130`, `0x1002a160`): state 2 and the
    /// lock full again.
    pub fn relink(&mut self, target: Option<usize>) {
        self.target = target;
        self.report = GATE_NO_TARGET;
        self.lock = self.gate.lock_s;
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
            if !self.started {
                self.last_wake_ms = now_ms;
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
        let since_s = ((t - self.last_wake_ms) * 0.001) as f32;
        self.last_wake_ms = t;
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
        if self.rounds == 0 || (self.capacitor > 0.0 && self.charge < self.shot_energy) || !self.ready {
            return;
        }
        // The gate runs whether or not the button is held (`0x10029d3a`).
        if self.gate.range > 0.0 && !self.gate_passes(since_s) {
            return;
        }
        if self.state == STATE_OFF {
            return;
        }
        self.lock = self.gate.lock_s;
        let firing: Vec<usize> =
            if self.salvo { (0..self.barrels.len()).collect() } else { vec![self.current] };
        for b in firing {
            self.barrels[b].step = STROKE;
            self.advance(b, t, shots);
        }
    }

    /// The target gate (`0x10029d3a`–`0x10029f60`), `since_s` after the last wake: whether
    /// a stroke may start.
    fn gate_passes(&mut self, since_s: f32) -> bool {
        let g = self.gate;
        let Some(target) = self.sight.target else {
            if g.lock_s > 0.0 {
                self.report = GATE_NO_TARGET;
                return false;
            }
            return true;
        };
        let line = target - self.sight.unit;
        if line.length_squared() > g.range * g.range {
            self.report = GATE_OUT_OF_RANGE;
            self.lock = g.lock_s;
            return false;
        }
        if g.cone_cos > 0.0
            && self.sight.barrel.normalize_or_zero().dot(line.normalize_or_zero()) <= g.cone_cos
        {
            self.report = GATE_OFF_BARREL;
            self.lock = g.lock_s;
            return false;
        }
        self.report = GATE_CLEAR;
        if g.lock_s > 0.0 && self.lock > 0.0 {
            self.lock -= since_s;
            return false;
        }
        true
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
    fn a_guided_gun_fires_only_at_a_target_in_range_and_cone_once_its_lock_runs_out() {
        // The hero's plasma rifle: 150 m, 0.25 rad, 0.25 s.
        let plasma = |target: Option<Vec3>| {
            let mut g = gun([-1.0, 0.0, 0.0, 0.0], &[4.0]);
            g.link(TargetGate::new(150.0, Some((0.25, 500.0, 250.0))));
            g.sight = Sight { unit: Vec3::ZERO, barrel: Vec3::Y, target };
            g
        };
        let g = plasma(None);
        assert_eq!(g.gate.range, 150.0);
        assert!(g.gate.guided() && (g.gate.cone_cos - 0.9689).abs() < 1e-4);

        let mut g = plasma(None);
        assert!(hold(&mut g, 1000.0).is_empty(), "no target, no shot");
        assert_eq!(g.report, GATE_NO_TARGET);
        let mut g = plasma(Some(Vec3::new(0.0, 200.0, 0.0)));
        assert!(hold(&mut g, 1000.0).is_empty(), "out of range");
        assert_eq!((g.report, g.lock), (GATE_OUT_OF_RANGE, 0.25));
        let mut g = plasma(Some(Vec3::new(60.0, 100.0, 0.0)));
        assert!(hold(&mut g, 1000.0).is_empty(), "0.54 rad off the barrel");
        assert_eq!((g.report, g.lock), (GATE_OFF_BARREL, 0.25));

        // In range and in the cone: the lock counts down from the first wake, then a stroke,
        // whose start fills the lock again.
        let mut g = plasma(Some(Vec3::new(10.0, 100.0, 0.0)));
        g.relink(Some(3));
        let shots = hold(&mut g, 1000.0);
        assert!(shots.len() >= 2 && shots[0] >= 250.0, "{shots:?}");
        assert!(shots[1] - shots[0] >= 250.0, "{shots:?}");
        assert_eq!(g.report, GATE_CLEAR);

        // The cannon, unguided and with no target, meets no gate.
        let mut cannon = gun([500.0, 20.0, 0.1, 0.0], &[4.0]);
        cannon.link(TargetGate::new(500.0, None));
        assert!(!cannon.gate.guided());
        assert_eq!(hold(&mut cannon, 1000.0).len(), 4);
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
