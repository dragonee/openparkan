//! A `.ctl` controller: parameters, animation states, channels, components and
//! action groups. See `docs/13-control.md`, `docs/24-motion.md` and
//! `openparkan/control.py`.

use crate::cursor::{FormatError, latin1, u32_at};
use crate::objects::ResourceRef;

/// The NRes tag every `.ctl` member carries.
pub const CTL_TAG: &str = "CTLD";
pub const HEADER_SIZE: usize = 128;
pub const BLOCK_SIZE: usize = 84;
pub const FRAME_SIZE: usize = HEADER_SIZE + BLOCK_SIZE;
pub const SECTION1_RECORD: usize = 156;
pub const SECTION1_PER_B: usize = 16;
pub const SECTION2_RECORD: usize = 36;
pub const COMPONENT_FIXED: usize = 0xB0;
pub const REFERENCE_STRIDE: usize = 100;
pub const REFERENCE_NAME_AT: usize = 36;
pub const BLOCK_ENTRIES: usize = 21;
/// Block entries: the group run at load, on a hit, at the map edge, at the range end.
pub const ENTRY_LOAD: usize = 0;
pub const ENTRY_HIT: usize = 2;
pub const ENTRY_EDGE: usize = 3;
pub const ENTRY_RANGE: usize = 4;
pub const NO_GROUP: i32 = -1;
/// A section-5 record's int 3 is its action; 27 explodes a node with the named `.exp`.
pub const ACTION_AT: usize = 3;
pub const ACT_EXPLODE_NODE: i32 = 27;
/// Action 4 creates an effect on three control points v4..v6 under id v7; action 14
/// drives an effect id v4 from the value at point v5.
pub const ACT_EFFECT_POINTS: i32 = 4;
/// Action 3 creates an effect by name on node v4 under id v7; action 10 starts effect v4
/// in time mode v5 (docs/13, "The section-5 record").
pub const ACT_EFFECT_NODE: i32 = 3;
pub const ACT_START_EFFECT: i32 = 10;
/// Action 8 deletes effect v4; 18 and 19 switch it on and off.
pub const ACT_DELETE_EFFECT: i32 = 8;
pub const ACT_EFFECT_ON: i32 = 18;
pub const ACT_EFFECT_OFF: i32 = 19;
/// A record's int 0: 0x40000000 runs it when any masked condition holds, 0x20000000 when
/// all do, a record with neither always runs; 0x80000000 opens a run and 0x10000000
/// closes it (`0x100022c0`, `0x100028b2`). Ints 1 and 2 are the mask and the inversion.
pub const REF_OPEN: u32 = 0x8000_0000;
pub const REF_ANY: u32 = 0x4000_0000;
pub const REF_ALL: u32 = 0x2000_0000;
pub const REF_ELSE: u32 = 0x1000_0000;
/// The controller's sixteen condition bytes: 0–10 the ground's surface id, one-hot, then
/// 7 the liquid bed's flag; 14 critical damage; 15 a copy of `+0x618`.
pub const CONDITIONS: usize = 16;
pub const COND_BED: usize = 7;
pub const ACT_EFFECT_TIME_POINT: i32 = 14;
pub const TRIPLE_AT: [usize; 6] = [20, 32, 44, 56, 68, 80];
/// Triples by index: acceleration (live copy doubled), top speed, turn rate.
pub const TRIPLE_ACCELERATION: usize = 0;
pub const TRIPLE_TOP_SPEED: usize = 2;
pub const TRIPLE_TURN: usize = 3;
pub const UNSET: u8 = 0xFF;
/// A transition cost at or above this is no edge.
pub const NO_EDGE: f32 = 1_000_000.0;

pub const STATE_ANCHOR: u32 = 0x1;
/// The body falls until a contact lands; without it the sphere is only lifted out of
/// the ground (`Control.dll:0x1001b3c3`). Set on exactly the states that have contacts.
pub const STATE_GROUND_CONTACTS: u32 = 0x4;
pub const STATE_BY_VELOCITY: u32 = 0x10000;
pub const STATE_FIXED: u32 = 0x100000;
pub const STATE_JITTER: u32 = 0x1000000;
/// A state's request code that any code the controller holds matches (`0x10001140`).
pub const ANY_REQUEST: i32 = -1;

/// A contact's point counts toward the body's ground gap and normal (`0x1001b00a`).
pub const CONTACT_SUPPORT: u32 = 0x1;
/// The contact's **carrier** node lies along the ground under it (`0x1001affd`): the
/// ground contact hands `IAnimation` slot 31 the contact's own axis and the ground normal
/// it has just found, and marks the carrier with node mask `0x10`, which makes the pose
/// walk turn that node's world matrix by the rotation between them and put it back where
/// it was. *Measured*: the 12 contacts that carry it are the four belts of each tracked
/// chassis and nothing else (docs/28-chassis.md, "The belt lies along the ground").
pub const CONTACT_PLACE: u32 = 0x2;
/// Set at load on the states whose last pose holds the contact within 0.1 of its rest
/// height: the states that end on that foot (`0x1001a2d5`).
pub const CONTACT_PLANTED: u32 = 0x1000;
/// How close a state's last pose must hold a contact to its rest height (`0x1003c03c`).
pub const PLANTED_WITHIN: f32 = 0.1;

pub const CHANNEL_WRAP: i32 = 0x1;
pub const CHANNEL_INVERT: i32 = 0x2;
/// Not driven by the component update: the camera's.
pub const CHANNEL_UNDRIVEN: i32 = 0x4;
/// Joins the turret's list: a gun mount that follows the pitch.
pub const CHANNEL_TURRET: i32 = 0x8;
/// Plays its node's material rather than posing it: the value is the fraction the material
/// manager's slot 5 takes (`World3D.dll:0x10003680`), not an animation frame. *Measured*: the
/// 12 channels that carry it are the tracked chassis's belts and nothing else
/// (docs/28-chassis.md, "The belt is a material a channel plays").
pub const CHANNEL_MATERIAL: i32 = 0x10;
/// Takes the previous channel's value.
pub const CHANNEL_FOLLOWS: i32 = 0x40;

pub const TURRET_TYPE: i32 = 1;
pub const GUN_TYPE: i32 = 2;
/// `CICLS_SIMPLE`: a generic device, which turns wheels and rotors from the machine's
/// motion (docs/28-chassis.md).
pub const SIMPLE_TYPE: i32 = 3;
pub const CAMERA_TYPE: i32 = 4;
pub const ENGINE_TYPE: i32 = 5;
/// `CICLS_RADAR`: values 0–2 its sensitivities, 3 its range, 4 how long a scan holds
/// (docs/25-sensors.md).
pub const RADAR_TYPE: i32 = 8;
pub const RADAR_RANGE: usize = 3;
pub const RADAR_PERIOD: usize = 4;
/// A guided round's seeker: value 0 its cone in radians, 1 its reach, 2 the lock its
/// gun waits, in ms (docs/29-weapons.md).
pub const SEEKER_TYPE: i32 = 17;
pub const SEEKER_CONE: usize = 0;
pub const SEEKER_REACH: usize = 1;
pub const SEEKER_LOCK: usize = 2;
/// The hero turret's weapon arms, one a gun, in the guns' order.
pub const ARM_TYPE: i32 = 24;
/// `CICLS_FIGHTSHIELD`, the shield generator's six sectors (docs/26-damage.md).
pub const FIGHT_SHIELD_TYPE: i32 = 9;
/// `CICLS_DETECTSHIELD` (docs/25-sensors.md).
pub const DETECT_SHIELD_TYPE: i32 = 10;
/// `CICLS_REPAIRSYS` (docs/26-damage.md).
pub const REPAIR_TYPE: i32 = 15;
/// `CICLS_POWERSTOR`, a battery: value 0 its capacity (docs/23-economy.md).
pub const BATTERY_TYPE: i32 = 19;
pub const BATTERY_CAPACITY: usize = 0;
/// The deflector, which decides how much of each shield sector stops (docs/26-damage.md).
pub const DEFLECTOR_TYPE: i32 = 21;
pub const MOUNT_UPRIGHT: u32 = 0x0400_0000;
/// A turret's flag that makes its unit an HQ, which `IsHQ` asks (`iron3d.dll:0x10076f50`,
/// docs/30-turrets.md, "An HQ unit in play").
pub const TURRET_HQ: u32 = 0x0800_0000;

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

fn i32_at(b: &[u8], at: usize) -> i32 {
    u32_at(b, at).expect("inside the controller") as i32
}

fn triple(b: &[u8], at: usize) -> [f32; 3] {
    [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)]
}

/// A 32-byte NUL-padded ASCII name, or `None` if the bytes are not one.
fn name(b: &[u8], at: usize) -> Option<String> {
    let field = b.get(at..at + 32)?;
    let end = field.iter().position(|&c| c == 0)?;
    if end == 0 || !field[..end].iter().all(|c| (32..127).contains(c)) {
        return None;
    }
    Some(latin1(&field[..end]))
}

/// One of a state's 16-byte conditions: a foot, wheel or leg (`docs/13-control.md`,
/// "Section 1's conditions are contacts").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contact {
    /// A control point of the object's `.cpt`.
    pub point: i32,
    pub flags: u32,
    /// The section-5 group run when the point lands, or -1.
    pub group: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub flags: u32,
    pub velocity: ([f32; 3], [f32; 3]),
    pub spin: ([f32; 3], [f32; 3]),
    pub engine: f32,
    pub actions: i32,
    /// The code the controller must hold for the state to apply, or `ANY_REQUEST`.
    pub request: i32,
    pub mode: u32,
    pub pair_a: [f32; 2],
    pub pair_b: [f32; 2],
    pub blend: f32,
    /// A fixed step length in ms; 0 lets speed and stride set it.
    pub length: f32,
    /// `counts[1]` contacts, the same points in every state of a controller.
    pub contacts: Vec<Contact>,
}

impl Default for State {
    /// Everything zero, except that the state asks for no request code.
    fn default() -> Self {
        Self {
            flags: 0,
            velocity: ([0.0; 3], [0.0; 3]),
            spin: ([0.0; 3], [0.0; 3]),
            engine: 0.0,
            actions: 0,
            request: ANY_REQUEST,
            mode: 0,
            pair_a: [0.0; 2],
            pair_b: [0.0; 2],
            blend: 0.0,
            length: 0.0,
            contacts: Vec::new(),
        }
    }
}

impl State {
    pub fn anchor(&self) -> bool {
        self.mode & STATE_ANCHOR != 0
    }

    pub fn by_velocity(&self) -> bool {
        self.mode & STATE_BY_VELOCITY != 0
    }

    /// `0x10001000`: whether `velocity` and `spin` lie inside the boxes the flags switch
    /// on, and the state's request code is `ANY_REQUEST` or the code the controller
    /// holds (`0x10001140`). The contacts' conditions are the caller's.
    pub fn applies(&self, velocity: [f32; 3], spin: [f32; 3], request: i32) -> bool {
        (self.request == ANY_REQUEST || self.request == request)
            && (0..3).all(|a| {
                let inside = |bit: u32, value: f32, (lo, hi): ([f32; 3], [f32; 3])| {
                    self.flags & (1 << bit) == 0 || (lo[a] <= value && value <= hi[a])
                };
                inside(a as u32, velocity[a], self.velocity) && inside(a as u32 + 4, spin[a], self.spin)
            })
    }

    /// `velocity` clamped into the velocity box, axis by axis (`0x10001160`). An axis
    /// the flags leave off holds −FLT_MAX..FLT_MAX in every shipped state.
    pub fn clamp_velocity(&self, velocity: [f32; 3]) -> [f32; 3] {
        let (lo, hi) = self.velocity;
        std::array::from_fn(|a| velocity[a].max(lo[a]).min(hi[a]))
    }
}

/// A section-2 record: an animated, rate-limited value from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub node: i32,
    pub first: f32,
    pub last: f32,
    pub initial: f32,
    pub origin: i32,
    pub point: i32,
    /// Value per second.
    pub rate: f32,
    /// Radians from 0 to 1.
    pub span: f32,
    pub flags: i32,
}

impl Channel {
    /// The frame the channel's node plays at `value`.
    pub fn frame(&self, value: f32) -> f32 {
        let mut v =
            if self.flags & CHANNEL_WRAP != 0 { value.rem_euclid(1.0) } else { value.clamp(0.0, 1.0) };
        if self.flags & CHANNEL_INVERT != 0 {
            v = 1.0 - v;
        }
        self.first + v * (self.last - self.first)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Component {
    pub type_id: i32,
    pub resource: ResourceRef,
    pub index: Option<i32>,
    pub entries: Vec<i32>,
    pub label: String,
    pub values: [f32; 16],
    pub power: f32,
    pub node: i32,
    pub mass: f32,
    pub flags: u32,
    pub group: i32,
    /// The two floats at `+0x24`: a generic device's or a radar's input weights
    /// (docs/28-chassis.md, "The class-3 records turn the wheels").
    pub weights: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub resource: ResourceRef,
    pub values: [i32; 9],
    pub group: usize,
}

impl Reference {
    pub fn action(&self) -> i32 {
        self.values[ACTION_AT]
    }

    pub fn flags(&self) -> u32 {
        self.values[0] as u32
    }

    /// Whether the record's own condition lets it run over the condition bytes.
    pub fn holds(&self, conditions: &[bool; CONDITIONS]) -> bool {
        let flags = self.flags();
        if flags & (REF_ANY | REF_ALL) == 0 {
            return true;
        }
        let (mask, inverted) = (self.values[1] as u32 & 0xFFFF, self.values[2] as u32 & 0xFFFF);
        let mut met = (0..CONDITIONS)
            .filter(|&i| mask >> i & 1 != 0)
            .map(|i| conditions[i] != (inverted >> i & 1 != 0));
        if flags & REF_ANY != 0 { met.any(|m| m) } else { met.all(|m| m) }
    }
}

/// The records of one group that run, in order (`Control.dll:0x100028b2`): a record that
/// opens a run opens it unless one is open; the next that closes it also runs when no
/// record since the run opened has run.
pub fn run_group<'a>(records: &[&'a Reference], conditions: &[bool; CONDITIONS]) -> Vec<&'a Reference> {
    let mut out = Vec::new();
    let (mut inside, mut ran) = (false, false);
    for &r in records {
        let mut forced = false;
        if r.flags() & REF_OPEN != 0 && !inside {
            (inside, ran) = (true, false);
        }
        if r.flags() & REF_ELSE != 0 {
            forced = inside && !ran;
            (inside, ran) = (false, false);
        }
        if forced || r.holds(conditions) {
            out.push(r);
            ran = true;
        }
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Controller {
    pub counts: [i32; 5],
    pub triples: [[f32; 3]; 6],
    /// `+92`: how long a dead object lasts before it is deleted, ms (docs/26, "What a
    /// damaged node, a destroyed part and a dead unit draw").
    pub death_ms: i32,
    pub pair: [f32; 2],
    pub mode: i32,
    pub bounds: [f32; 2],
    pub cone: f32,
    pub flags: i32,
    pub payload: f32,
    pub bare: bool,
    pub states: Vec<State>,
    /// `states.len()²` transition costs, the row the destination.
    pub costs: Vec<f32>,
    pub channels: Vec<Channel>,
    pub components: Vec<Component>,
    pub groups: [i32; BLOCK_ENTRIES],
    pub references: Vec<Reference>,
}

impl Controller {
    /// The records of the group block entry `entry` names, in order.
    pub fn group(&self, entry: usize) -> Vec<&Reference> {
        match self.groups.get(entry) {
            Some(&index) if index != NO_GROUP => {
                self.references.iter().filter(|r| r.group as i32 == index).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The records of group `index`, in order.
    pub fn group_at(&self, index: i32) -> Vec<&Reference> {
        self.references.iter().filter(|r| r.group as i32 == index).collect()
    }

    /// What moving from state `from` to state `to` costs; the row is the destination.
    pub fn cost(&self, to: usize, from: usize) -> f32 {
        self.costs[to * self.states.len() + from]
    }

    /// The costs the planner uses, scaled at load (`0x10001790`): the file's cost times
    /// one plus two gaps. The velocity gap is the largest distance, over the axes the
    /// source state switches on, from the centre of the destination's velocity box to
    /// the minimum of the source's; the spin gap is the same over the spin box.
    pub fn live_costs(&self) -> Vec<f32> {
        let n = self.states.len();
        let mut out = Vec::with_capacity(n * n);
        for to in 0..n {
            for from in 0..n {
                let (dest, src) = (&self.states[to], &self.states[from]);
                let gap = |box_to: &([f32; 3], [f32; 3]), box_from: &([f32; 3], [f32; 3]), first_bit: u32| {
                    (0..3)
                        .filter(|&k| src.flags & (first_bit << k) != 0)
                        .map(|k| ((box_to.0[k] + box_to.1[k]) * 0.5 - box_from.0[k]).abs())
                        .fold(0.0_f32, f32::max)
                };
                let scale = 1.0 + gap(&dest.velocity, &src.velocity, 1) + gap(&dest.spin, &src.spin, 0x10);
                out.push(self.cost(to, from) * scale);
            }
        }
        out
    }

    /// The cheapest run of states from `from` to `to` over the file's costs.
    pub fn path(&self, from: usize, to: usize) -> Option<(Vec<usize>, f32)> {
        self.path_by(&self.costs, from, to)
    }

    /// The cheapest run of states from `from` to `to` over `costs`, without `from` and
    /// with `to`, and what it costs.
    ///
    /// Dijkstra rooted at the target, the way the planner runs it (`0x100019d0`):
    /// each state starts at its direct cost to `to` and relaxes through the states
    /// settled before it, until `from` is settled. A cost of `NO_EDGE` or more is no
    /// edge. With `from == to` that is the cheapest cycle back, or the diagonal.
    pub fn path_by(&self, costs: &[f32], from: usize, to: usize) -> Option<(Vec<usize>, f32)> {
        let n = self.states.len();
        if from >= n || to >= n || costs.len() != n * n {
            return None;
        }
        let edge = |to: usize, from: usize| Some(costs[to * n + from]).filter(|&c| c < NO_EDGE);
        let mut dist: Vec<f32> = (0..n).map(|k| edge(to, k).unwrap_or(f32::INFINITY)).collect();
        let mut next = vec![to; n];
        let mut settled = vec![false; n];
        loop {
            let open = (0..n).filter(|&k| !settled[k] && dist[k].is_finite());
            let k = open.min_by(|&a, &b| dist[a].total_cmp(&dist[b]))?;
            settled[k] = true;
            if k == from {
                break;
            }
            if k == to {
                continue;
            }
            for j in (0..n).filter(|&j| !settled[j]) {
                if let Some(step) = edge(k, j)
                    && dist[k] + step < dist[j]
                {
                    dist[j] = dist[k] + step;
                    next[j] = k;
                }
            }
        }
        let mut out = Vec::new();
        let mut state = from;
        loop {
            state = next[state];
            out.push(state);
            if state == to || out.len() > n {
                break;
            }
        }
        Some((out, dist[from]))
    }
}

fn section4_start(counts: &[i32; 5]) -> usize {
    let (a, b, c) = (counts[0] as usize, counts[1] as usize, counts[2] as usize);
    HEADER_SIZE + a * (SECTION1_RECORD + SECTION1_PER_B * b) + 4 * a * a + c * SECTION2_RECORD
}

/// One section-4 record at `pos` and the offset just past it, or `None` if it does
/// not read as one.
fn component(b: &[u8], pos: usize) -> Option<(Component, usize)> {
    if pos + COMPONENT_FIXED > b.len() {
        return None;
    }
    let type_id = i32_at(b, pos);
    let count = i32_at(b, pos + 0xAC);
    if !(1..31).contains(&type_id) || !(0..=4096).contains(&count) {
        return None;
    }
    let mut end = pos + COMPONENT_FIXED;
    if end + 4 * count as usize + 4 > b.len() {
        return None;
    }
    let entries = (0..count as usize).map(|i| i32_at(b, end + 4 * i)).collect();
    end += 4 * count as usize;
    let length = i32_at(b, end);
    end += 4;
    let mut label = String::new();
    if length != 0 {
        if !(0 < length && length < 4096) || end + length as usize + 1 > b.len() {
            return None;
        }
        let raw = &b[end..end + length as usize + 1];
        let stop = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
        label = latin1(&raw[..stop]);
        end += length as usize + 1;
    }
    let index = i32_at(b, pos + 0x18);
    let part = Component {
        type_id,
        resource: ResourceRef {
            library: name(b, pos + 0x6C).unwrap_or_default(),
            member: name(b, pos + 0x6C + 32).unwrap_or_default(),
        },
        index: (index != -1).then_some(index),
        entries,
        label,
        values: std::array::from_fn(|k| f32_at(b, pos + 0x2C + 4 * k)),
        power: f32_at(b, pos + 0x20),
        node: i32_at(b, pos + 0x04),
        mass: f32_at(b, pos + 0x1C),
        flags: u32_at(b, pos + 0x08).expect("inside"),
        group: i32_at(b, pos + 0x0C),
        weights: [f32_at(b, pos + 0x24), f32_at(b, pos + 0x28)],
    };
    Some((part, end))
}

/// Parse a `.ctl` member.
pub fn parse(b: &[u8], source: &str) -> Result<Controller, FormatError> {
    let bad = |m: String| FormatError::invalid(source, m);
    if b.len() < FRAME_SIZE {
        return Err(bad(format!("{} bytes, short of the {FRAME_SIZE}-byte frame", b.len())));
    }
    let counts: [i32; 5] = std::array::from_fn(|k| i32_at(b, 4 * k));
    if counts.iter().any(|&n| n < 0) {
        return Err(bad(format!("negative section count in {counts:?}")));
    }
    if counts.iter().all(|&n| n == 0) && b.len() != FRAME_SIZE {
        return Err(bad("no sections but trailing bytes".into()));
    }
    let (a, per_b, c) = (counts[0] as usize, counts[1] as usize, counts[2] as usize);
    let stride = SECTION1_RECORD + SECTION1_PER_B * per_b;
    if section4_start(&counts) > b.len() {
        return Err(bad("sections run past the end".into()));
    }
    let states = (0..a)
        .map(|i| {
            let at = HEADER_SIZE + i * stride;
            State {
                flags: u32_at(b, at).expect("inside"),
                mode: u32_at(b, at + 0x04).expect("inside"),
                pair_a: [f32_at(b, at + 0x0C), f32_at(b, at + 0x10)],
                pair_b: [f32_at(b, at + 0x14), f32_at(b, at + 0x18)],
                blend: f32_at(b, at + 0x1C),
                length: f32_at(b, at + 0x20),
                velocity: (triple(b, at + 0x24), triple(b, at + 0x30)),
                spin: (triple(b, at + 0x3C), triple(b, at + 0x48)),
                engine: f32_at(b, at + 0x54),
                actions: i32_at(b, at + 0x90),
                request: i32_at(b, at + 0x98),
                contacts: (0..per_b)
                    .map(|j| {
                        let c = at + SECTION1_RECORD + SECTION1_PER_B * j;
                        Contact {
                            point: i32_at(b, c),
                            flags: u32_at(b, c + 4).expect("inside"),
                            group: i32_at(b, c + 8),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let costs_at = HEADER_SIZE + a * stride;
    let costs = (0..a * a).map(|k| f32_at(b, costs_at + 4 * k)).collect();
    let section2 = section4_start(&counts) - c * SECTION2_RECORD;
    let channels = (0..c)
        .map(|i| {
            let at = section2 + i * SECTION2_RECORD;
            Channel {
                node: i32_at(b, at),
                first: f32_at(b, at + 4),
                last: f32_at(b, at + 8),
                initial: f32_at(b, at + 12),
                origin: i32_at(b, at + 16),
                point: i32_at(b, at + 20),
                rate: f32_at(b, at + 24),
                span: f32_at(b, at + 28),
                flags: i32_at(b, at + 32),
            }
        })
        .collect();
    let mut pos = section4_start(&counts);
    let mut components = Vec::new();
    for n in 0..counts[3] {
        let (part, end) =
            component(b, pos).ok_or_else(|| bad(format!("component {n} does not read at {pos}")))?;
        pos = end;
        components.push(part);
    }
    if pos + BLOCK_SIZE > b.len() {
        return Err(bad("no room for the block".into()));
    }
    let groups = std::array::from_fn(|k| i32_at(b, pos + 4 * k));
    let mut at = pos + BLOCK_SIZE;
    let mut references = Vec::new();
    for group in 0..counts[4] as usize {
        if at + 4 > b.len() {
            return Err(bad(format!("reference groups do not read at {at}")));
        }
        let n = i32_at(b, at);
        at += 4;
        if n < 0 || at + n as usize * REFERENCE_STRIDE > b.len() {
            return Err(bad(format!("reference group {group} does not fit")));
        }
        for _ in 0..n {
            references.push(Reference {
                resource: ResourceRef {
                    library: name(b, at + REFERENCE_NAME_AT).unwrap_or_default(),
                    member: name(b, at + REFERENCE_NAME_AT + 32).unwrap_or_default(),
                },
                values: std::array::from_fn(|k| i32_at(b, at + 4 * k)),
                group,
            });
            at += REFERENCE_STRIDE;
        }
    }
    if at != b.len() {
        return Err(bad(format!("consumed {at} of {} bytes", b.len())));
    }
    Ok(Controller {
        counts,
        triples: TRIPLE_AT.map(|t| triple(b, t)),
        death_ms: i32_at(b, 92),
        pair: [f32_at(b, 96), f32_at(b, 100)],
        mode: i32_at(b, 104),
        bounds: [f32_at(b, 108), f32_at(b, 120)],
        cone: f32_at(b, 112),
        flags: i32_at(b, 116),
        payload: f32_at(b, 124),
        bare: b[HEADER_SIZE..].iter().all(|&v| v == UNSET),
        states,
        costs,
        channels,
        components,
        groups,
        references,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_plays_its_first_matching_record_or_else_its_closing_one() {
        // r_h_02's foot group: metal, stone, grass, alloy, flesh, then grass for 9 or else.
        let record = |flags: u32, byte: u32, id: i32| Reference {
            resource: ResourceRef::default(),
            values: [flags as i32, 1 << byte, 0, ACT_START_EFFECT, id, 1, 0, 0, 0],
            group: 0,
        };
        let group = [
            record(REF_OPEN | REF_ANY, 5, 101),
            record(REF_ANY, 1, 201),
            record(REF_ANY, 2, 301),
            record(REF_ANY, 8, 401),
            record(REF_ANY, 10, 501),
            record(REF_ELSE | REF_ANY, 9, 301),
        ];
        let refs: Vec<&Reference> = group.iter().collect();
        let on = |surface: usize| {
            let mut c = [false; CONDITIONS];
            c[surface] = true;
            run_group(&refs, &c).iter().map(|r| r.values[4]).collect::<Vec<_>>()
        };
        assert_eq!(on(5), vec![101]);
        assert_eq!(on(1), vec![201]);
        assert_eq!(on(9), vec![301], "its own condition");
        assert_eq!(on(3), vec![301], "the else");
        // An inversion counts a clear byte.
        let mut clear = record(REF_ANY, 7, 9);
        clear.values[2] = 1 << 7;
        assert!(clear.holds(&[false; CONDITIONS]) && !clear.holds(&[true; CONDITIONS]));
    }

    #[test]
    fn the_smallest_controller_is_the_frame_and_the_block() {
        let mut b = vec![0u8; FRAME_SIZE];
        b[HEADER_SIZE..].fill(UNSET);
        let c = parse(&b, "t").unwrap();
        assert!(c.bare && c.states.is_empty() && c.groups == [-1; BLOCK_ENTRIES]);
        b.push(0);
        assert!(parse(&b, "t").is_err());
    }

    #[test]
    fn a_state_applies_inside_the_boxes_its_flags_switch_on() {
        let s = State { flags: 0b10, velocity: ([0.0, 2.0, 0.0], [0.0, 10.0, 0.0]), ..Default::default() };
        assert!(s.applies([100.0, 5.0, 0.0], [9.0; 3], ANY_REQUEST));
        assert!(!s.applies([0.0, 12.0, 0.0], [0.0; 3], ANY_REQUEST));
        assert!(s.applies([0.0, 5.0, 0.0], [0.0; 3], 6), "a state asking for no code takes any");
    }

    #[test]
    fn a_state_with_a_request_code_applies_only_while_the_controller_holds_it() {
        let s = State { request: 6, ..Default::default() };
        assert!(s.applies([0.0; 3], [0.0; 3], 6));
        assert!(!s.applies([0.0; 3], [0.0; 3], 8));
        assert!(!s.applies([0.0; 3], [0.0; 3], ANY_REQUEST));
    }

    #[test]
    fn a_step_moves_by_the_velocity_clamped_into_the_box() {
        let s = State { velocity: ([-0.5, 2.0, -0.5], [0.5, 10.0, 0.5]), ..Default::default() };
        assert_eq!(s.clamp_velocity([1.0, 14.0, 0.0]), [0.5, 10.0, 0.0]);
        assert_eq!(s.clamp_velocity([0.0, 1.0, -3.0]), [0.0, 2.0, -0.5]);
    }

    #[test]
    fn a_states_contacts_follow_its_record() {
        // One state with two contacts: 156 bytes, then 16 a contact, then one cost.
        let per_state = SECTION1_RECORD + 2 * SECTION1_PER_B;
        let mut b = vec![0u8; HEADER_SIZE + per_state + 4 + BLOCK_SIZE];
        for (k, n) in [1i32, 2, 0, 0, 0].iter().enumerate() {
            b[4 * k..4 * k + 4].copy_from_slice(&n.to_le_bytes());
        }
        let at = HEADER_SIZE;
        b[at + 0x98..at + 0x9c].copy_from_slice(&ANY_REQUEST.to_le_bytes());
        for (j, (point, flags, group)) in [(0i32, 0x125u32, 3i32), (2, 0x125, 4)].into_iter().enumerate() {
            let c = at + SECTION1_RECORD + SECTION1_PER_B * j;
            b[c..c + 4].copy_from_slice(&point.to_le_bytes());
            b[c + 4..c + 8].copy_from_slice(&flags.to_le_bytes());
            b[c + 8..c + 12].copy_from_slice(&group.to_le_bytes());
        }
        let block = HEADER_SIZE + per_state + 4;
        b[block..block + 4 * BLOCK_ENTRIES].fill(0xFF);
        let c = parse(&b, "t").unwrap();
        assert_eq!(c.states[0].request, ANY_REQUEST);
        assert_eq!(
            c.states[0].contacts,
            vec![Contact { point: 0, flags: 0x125, group: 3 }, Contact { point: 2, flags: 0x125, group: 4 }]
        );
    }

    /// Three states in a ring, 0 -> 1 -> 2 -> 0 at 1 each, and a short cut 0 -> 2 at 5.
    fn ring(diagonal: f32) -> Controller {
        let mut costs = vec![NO_EDGE; 9];
        let mut edge = |from: usize, to: usize, c: f32| costs[to * 3 + from] = c;
        edge(0, 1, 1.0);
        edge(1, 2, 1.0);
        edge(2, 0, 1.0);
        edge(0, 2, 5.0);
        edge(0, 0, diagonal);
        Controller { states: vec![State::default(); 3], costs, ..Default::default() }
    }

    #[test]
    fn the_loader_scales_a_cost_by_the_gap_from_the_destinations_centre() {
        let run = |lo: f32, hi: f32| State {
            flags: 0b10,
            velocity: ([0.0, lo, 0.0], [0.0, hi, 0.0]),
            ..Default::default()
        };
        let c = Controller {
            states: vec![run(6.0, 14.0), run(-14.0, -6.0), run(6.0, 14.0)],
            costs: (0..9).map(|k| if k % 4 == 0 { NO_EDGE } else { 1.0 }).collect(),
            ..Default::default()
        };
        let live = c.live_costs();
        // Forward to backward: |-10 - 6| = 16; forward to forward: |10 - 6| = 4.
        assert_eq!(live[3], 17.0);
        assert_eq!(live[2 * 3], 5.0);
        assert_eq!(c.path_by(&live, 0, 0).map(|p| p.0), Some(vec![2, 0]));
    }

    #[test]
    fn the_planner_takes_the_cheapest_run_and_the_row_is_the_destination() {
        let c = ring(NO_EDGE);
        assert_eq!(c.path(0, 2), Some((vec![1, 2], 2.0)));
        assert_eq!(c.path(0, 0), Some((vec![1, 2, 0], 3.0)));
        assert_eq!(c.path(2, 1), Some((vec![0, 1], 2.0)));
        assert_eq!(ring(0.5).path(0, 0), Some((vec![0], 0.5)));
    }

    #[test]
    fn a_wrapping_inverted_channel_plays_its_frames_backwards() {
        let ch = Channel {
            node: 1,
            first: 49.0,
            last: 53.0,
            initial: 0.5,
            origin: -1,
            point: 1,
            rate: 100.0,
            span: 1.0,
            flags: 3,
        };
        assert_eq!(ch.frame(0.25), 52.0);
        assert_eq!(ch.frame(1.25), 52.0);
    }
}
