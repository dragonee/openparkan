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
/// A round's frame `+116` for which the HUD draws a guided lock (`iron3d.dll:0x1009ce0d`):
/// every round with a seeker, and no other (docs/35-hud.md, "The guided lock").
pub const LOCK_DRAWN: i32 = 16;
/// The damage above which the AI fires a round only at a building (`Behavior.dll:0x1005994c`,
/// docs/29, "How the AI fires"): the winged SSMs' 60,000 and 100,000, where the next gun down
/// does 3,000.
pub const HEAVY_ROUND: f32 = 10_000.0;

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
    /// Its record's power figure, which it draws a second on top of its capacitor's lack.
    pub power: f32,
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
    /// The word its round's frame keeps at `+116`, which the gun copies with the rest of
    /// the frame's block at link (`0x100297a4`); the HUD draws a lock for [`LOCK_DRAWN`].
    pub round_flags: i32,
    /// The damage its round does, the hit explosion's (property 6, `+0x174`): the AI holds a
    /// round of more than [`HEAVY_ROUND`] for a building, and the target panel calls a unit
    /// carrying one dangerous.
    pub round_damage: f32,
    /// The lock's share (`+0x17c`, property `0xf00`), which slot 10 keeps every frame.
    pub lock_share: f32,
    /// Its node has no life left (slot 2, `0x10021820`), which the owner keeps from the node:
    /// it starts no stroke and reports 5 (`0x10029cc3`), though one under way finishes.
    pub broken: bool,
    /// The barrels whose stroke has started since the owner last took them: a gun's shot group
    /// runs as a barrel starts its stroke (docs/29, "What a shot plays").
    pub stroked: Vec<usize>,
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
            power: record.power,
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
            round_flags: 0,
            round_damage: 0.0,
            lock_share: 0.0,
            broken: false,
            stroked: Vec::new(),
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

    /// The word the HUD's lamp and name read for this gun at `now_ms` (docs/35-hud.md, "The
    /// weapons list"): 5 no rounds, 6 its capacitor short of a shot, 7 not ready, 3 a
    /// stroke under way, 4 waiting its interval; then the gate's 2 for a guided gun with no
    /// target, or 7 or 8 against the target it has; 1 a guided gun locking; else 0, ready.
    ///
    /// STAND-IN: docs/29-weapons.md#the-guns-takt-a-stroke-then-the-interval -- where the
    /// takt stores codes 0 and 3–6 (`0x100295ba`–`0x1002a167`) is not read: the word is
    /// taken from the gun's state now rather than kept from the last point that wrote it.
    pub fn lamp_report(&self, now_ms: f64) -> i32 {
        let guided = self.gate.lock_s > 0.0;
        if self.rounds == 0 || (self.broken && self.barrels.iter().all(|b| b.step == 0)) {
            5
        } else if self.capacitor > 0.0 && self.charge < self.shot_energy {
            6
        } else if !self.ready {
            7
        } else if self.barrels.iter().any(|b| b.step != 0) {
            3
        } else if self.started && now_ms < self.next_ms {
            4
        } else if guided && self.sight.target.is_none() {
            GATE_NO_TARGET
        } else if self.gate.range > 0.0
            && self.sight.target.is_some()
            && matches!(self.report, GATE_OUT_OF_RANGE | GATE_OFF_BARREL)
        {
            self.report
        } else if guided && self.lock > 0.0 {
            1
        } else {
            0
        }
    }

    /// What a dock rearms it by (`Behavior.dll:0x100181e0`, docs/29, "There is no reload"):
    /// `share` of the magazine rounded down, but at least one round, into the rounds left
    /// (properties `0x800` and `0x700`), and `share` of the capacitor into the charge. A gun
    /// whose magazine is [`UNLIMITED`] never spends a round and takes none.
    pub fn rearm(&mut self, share: f32) {
        if self.magazine > 0 && self.rounds < self.magazine {
            let add = (share * self.magazine as f32).floor().max(1.0) as i32;
            self.rounds = (self.rounds + add).min(self.magazine);
        }
        self.charge = (self.charge + share * self.capacitor).min(self.capacitor);
    }

    /// What it wants of its channel over `dt` seconds (`Control.dll:0x10029a40`, class slot 5):
    /// its power and whatever its capacitor lacks of value 1, nothing while its node is
    /// destroyed. **Every one of the 158 shipped gun and builder components has a power of
    /// 0** (*measured*), so on the install this is exactly the lack.
    pub fn want(&self, dt: f32) -> f32 {
        if self.broken {
            return 0.0;
        }
        self.power * dt + (self.capacitor - self.charge).max(0.0)
    }

    /// Served at `level` over `dt` seconds (`0x10029a90`, slot 6): what the level gives beyond
    /// its power goes into the capacitor, which is where a gun's charge comes from and the only
    /// place it comes from. At a level of 1 the capacitor is full again on the next power tick;
    /// short of power it closes `level` of the gap, and a gun with a power of its own could lose
    /// charge. The game adds the difference unguarded and clamps nothing; with a power of 0 and
    /// a level of at most 1 neither guard here can bind.
    pub fn serve(&mut self, level: f32, dt: f32) {
        let charge = level * self.want(dt) - self.power * dt;
        if charge > 0.0 {
            self.charge = (self.charge + charge).min(self.capacitor);
        }
    }

    /// Top the capacitor up, as a full power tick would.
    ///
    /// STAND-IN: docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured
    /// -- a building's guns draw what their capacitors lack from its batteries on its power
    /// tick, and the building's economy is not modelled: a gun on a building, or on a unit with
    /// no battery, is served at a level of 1. A unit with a battery runs the read tick,
    /// [`Gun::want`] and [`Gun::serve`].
    pub fn recharge(&mut self) {
        self.serve(1.0, 0.0);
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
        self.keep_share(now_ms);
        shots
    }

    /// Slot 10 after the events (`0x10029be0`, from the time driver at `0x1002d317`): while
    /// locking, the share is 1 − the lock left ÷ value 9 (1 with no lock), held to 0..1;
    /// in any other report it stays. Slot 10 also writes the wait's progress while the gun
    /// waits its interval (report 4), which nothing drawn reads: the lock draws only in
    /// reports 0 and 1, and a lock starts again at a share of 0.
    fn keep_share(&mut self, now_ms: f64) {
        if self.lamp_report(now_ms) == 1 {
            let lock_s = self.gate.lock_s;
            let share = if lock_s > 0.0 { 1.0 - self.lock / lock_s } else { 1.0 };
            self.lock_share = share.clamp(0.0, 1.0);
        }
    }

    /// `0x10029ca0`: continue a stroke, or start one when the gun's node lives and it has
    /// rounds, charge, its ready byte and a state.
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
        if self.broken
            || self.rounds == 0
            || (self.capacitor > 0.0 && self.charge < self.shot_energy)
            || !self.ready
        {
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
            self.stroked.push(b);
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
            weights: [0.0; 2],
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

    /// `Control.dll:0x10029a40` and `0x10029a90`: the gun asks its channel for whatever its
    /// capacitor lacks of value 1 and takes the tick's share of it, and nothing else fills it.
    /// Every shipped gun's own power figure is 0, so the ask is the lack exactly; the level
    /// is the charge over value 1, and a gun with no capacity has neither
    /// (`0x10029ae2`, `0x1002a03d`).
    #[test]
    fn a_capacitor_closes_the_ticks_share_of_what_it_lacks_and_only_that() {
        let mut g = gun([500.0, 20.0, 5.0, 0.0], &[4.0]);
        assert_eq!(g.power, 0.0, "every shipped gun component's power figure");
        assert_eq!(g.charge, g.capacitor, "the parse leaves it full (`0x1002967f`)");
        g.charge = 0.0;
        assert_eq!(g.want(0.25), 20.0, "the whole lack, whatever dt");
        g.serve(0.5, 0.25);
        assert_eq!(g.charge, 10.0, "half a tick's power closes half the gap");
        g.serve(1.0, 0.25);
        assert_eq!(g.charge, 20.0, "and a full tick fills it");
        g.serve(1.0, 0.25);
        assert_eq!(g.charge, 20.0, "never past value 1");
        // A gun with no capacity is not held up by charge and spends none.
        let mut free = gun([500.0, 0.0, 5.0, 0.0], &[4.0]);
        assert_eq!(free.want(0.25), 0.0);
        let shots = hold(&mut free, 1000.0);
        assert!(!shots.is_empty() && free.charge == 0.0);
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
    fn a_gun_whose_node_is_destroyed_finishes_its_stroke_and_starts_no_other() {
        let mut laser = gun([-1.0, 200.0, 5.5, 200.0], &[4.0]);
        laser.state = CONTINUE_FIGHT;
        assert_eq!(laser.tick(0.0).len(), 0, "a stroke starts");
        laser.broken = true;
        assert_eq!(laser.tick(200.0).len(), 1, "the round under way still leaves");
        assert!(hold(&mut laser, 3000.0).is_empty(), "and no stroke starts after it");
        assert_eq!(laser.lamp_report(3000.0), 5);
        laser.broken = false;
        let again: usize = (0..30).map(|k| laser.tick(3000.0 + f64::from(k) * 1000.0 / 60.0).len()).sum();
        assert!(again > 0, "a node brought back fires again");
    }

    #[test]
    fn a_dock_rearms_a_gun_by_a_tenth_of_its_magazine_a_second_but_never_by_less_than_a_round() {
        // A tenth of a second in a dock, so 0.01 of the magazine a rearm (docs/27).
        let mut cannon = gun([500.0, 20.0, 0.1, 0.0], &[4.0]);
        cannon.rounds = 0;
        cannon.charge = 0.0;
        cannon.rearm(0.01);
        assert_eq!(cannon.rounds, 5, "5 of 500 rounds");
        assert!((cannon.charge - 0.2).abs() < 1e-5, "0.2 of a capacitor of 20: {}", cannon.charge);
        for _ in 0..200 {
            cannon.rearm(0.01);
        }
        assert_eq!((cannon.rounds, cannon.charge), (500, 20.0), "full in ten seconds, and no further");
        // A two-round launcher takes its one round rather than none.
        let mut missiles = gun([2.0, 0.8, 0.2, 1250.0], &[1.0, 2.5]);
        missiles.rounds = 0;
        missiles.rearm(0.01);
        assert_eq!(missiles.rounds, 1);
        let mut free = gun([-1.0, 200.0, 5.5, 200.0], &[4.0]);
        free.rounds = -1;
        free.rearm(0.01);
        assert_eq!(free.rounds, -1, "an unlimited magazine spends nothing and takes nothing");
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
    fn a_locking_guns_share_grows_to_the_lock_and_stays_once_locked() {
        let mut g = gun([-1.0, 0.0, 0.0, 250.0], &[2.0]);
        g.link(TargetGate::new(150.0, Some((0.25, 500.0, 250.0))));
        g.sight = Sight { unit: Vec3::ZERO, barrel: Vec3::Y, target: Some(Vec3::new(0.0, 100.0, 0.0)) };
        let step = 1000.0 / 60.0;
        let mut t = 0.0;
        let mut shares = Vec::new();
        while t <= 500.0 {
            g.tick(t);
            shares.push((t, g.lamp_report(t), g.lock_share));
            t += step;
        }
        // The first wake finds the lock full: nothing yet.
        assert_eq!(shares[0], (0.0, 1, 0.0));
        let at = |ms: f64| shares.iter().rev().find(|s| s.0 <= ms).copied().unwrap();
        let (_, report, half) = at(135.0);
        assert!(report == 1 && (0.4..0.6).contains(&half), "{:?}", at(135.0));
        // Locked and not asked to fire: report 0, the share kept just short of 1.
        let (_, report, kept) = at(500.0);
        assert!(report == 0 && (0.9..1.0).contains(&kept), "{:?}", at(500.0));
        assert!(shares.windows(2).all(|w| w[1].2 >= w[0].2), "it only grows");
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
