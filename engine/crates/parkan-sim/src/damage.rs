//! Hit points and what a hit does to them: `docs/26-damage.md`.

use parkan_formats::ndp::NodeDamage;

/// A mesh node flag the life system reads as vital.
///
/// STAND-IN: docs/26-damage.md#hit-points--read-and-measured -- the node record's
/// bit 0 comes from AniMesh query 0xe & 0x200 (`Control.dll:0x1000f9aa`); that the
/// query is the mesh node's flags word is not read.
pub const VITAL_NODE_FLAG: u16 = 0x200;

/// `min(damage, linear × damage + square × damage²)` (`0x10010030`).
pub fn armour(damage: f32, linear: f32, square: f32) -> f32 {
    damage.min(linear * damage + square * damage * damage)
}

/// What a blast of `damage` in `radius` does to a node sphere of radius `r` whose
/// centre is `distance` away (`0x10010030`): nothing from `R + r` out, the whole of
/// it while one sphere lies strictly inside the other, and between them
/// `((R + r − d) ÷ 2R)³` of it. A node of radius 0 takes nothing.
pub fn blast(damage: f32, radius: f32, r: f32, distance: f32) -> f32 {
    if r == 0.0 || distance >= radius + r {
        0.0
    } else if distance < radius - r || distance < r - radius {
        damage
    } else {
        damage * ((radius + r - distance) / (2.0 * radius)).powi(3)
    }
}

/// A round dies from full health, so it hits for `ratio × (its hit points + the .exp damage)`.
pub fn round_hit(ratio: f32, hit_points: f32, explosion_damage: f32) -> f32 {
    ratio * (hit_points + explosion_damage)
}

/// A mesh node flag the loader reads as never hidden: a building's shell
/// (`Control.dll:0x1000f9bd`).
pub const NEVER_HIDDEN_NODE_FLAG: u16 = 0x100;

/// A node's status bits (the life record's `+0x28`, docs/26, "What a damaged node, a
/// destroyed part and a dead unit draw").
pub const STATUS_VITAL: u32 = 0x1;
pub const STATUS_NEVER_HIDDEN: u32 = 0x2;
pub const STATUS_DESTROYED: u32 = 0x10;
pub const STATUS_HIDDEN: u32 = 0x20;
/// Knocked off and flying; taken down by a part that is flying; taken down where it stood.
pub const STATUS_FLYING: u32 = 0x40;
pub const STATUS_CARRIED: u32 = 0x80;
pub const STATUS_DOWN: u32 = 0x100;
/// A destroyed node with any of these is not knocked off (`0x100102dc`).
pub const KNOCK_OFF_BARS: u32 = 0x1ce;
/// How long a knocked-off part flies (`Control.dll:0x100424a4`, set by `0x10006350`).
pub const FLIGHT_MS: f64 = 3000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeLife {
    pub life: f32,
    pub max: f32,
    pub destroyed: bool,
    /// Its stage count: how many of variants 0, 1 and 2 in a row have a level-0 slot, 1
    /// when none does (`AniMesh.dll:0x10005840`).
    pub stages: u8,
    pub stage: u8,
    pub status: u32,
}

impl NodeLife {
    /// The variant it draws: its stage held below its count (`0x100118b1`).
    pub fn block(&self) -> usize {
        usize::from(self.stage.min(self.stages.saturating_sub(1)))
    }

    /// Gone from the world: not drawn, struck, collided with or stood on.
    pub fn hidden(&self) -> bool {
        self.status & STATUS_HIDDEN != 0
    }

    /// Flying off, or carried by a part that is.
    pub fn flying(&self) -> bool {
        self.status & (STATUS_FLYING | STATUS_CARRIED) != 0
    }

    /// `N − ceil(N × life ÷ max)`, held to N (`0x10011346`–`0x10011389`); a node with no hit
    /// points stays whole.
    fn stage_now(&self) -> u8 {
        if self.max <= 0.0 {
            return 0;
        }
        let n = f32::from(self.stages);
        (n - (n * self.life / self.max).ceil()).clamp(0.0, n) as u8
    }
}

/// What a life takt did to a node, for the world to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Its stage rose: its `.exp` plays (`0x100113b2`).
    Staged(usize),
    /// It reached its last stage and is gone (`0x10011920`).
    Hidden(usize),
    /// It was knocked off, and flies from now (`0x100102a0`).
    KnockedOff(usize),
}

/// An object's nodes' hit points.
#[derive(Clone, Debug, PartialEq)]
pub struct Life {
    pub nodes: Vec<NodeLife>,
    /// Each node's parent, from the mesh.
    pub parents: Vec<Option<usize>>,
    /// Whether a node's death kills the object: node 0, and nodes flagged vital.
    pub vital: Vec<bool>,
    /// The armour fitted: its linear and square factors.
    pub armour: Option<(f32, f32)>,
    /// Agent kind 3, a building: node 0 or a vital node dying only marks it, and no part is
    /// knocked off.
    pub building: bool,
    /// The owner word reads `0xfffe` (`0x10011098`): node 0 or a vital node is destroyed.
    pub marked: bool,
    /// The object died: marked, and not a building.
    pub dead: bool,
    /// Each flying part and when its flight ends, ms.
    pub flights: Vec<(usize, f64)>,
}

impl Life {
    /// `0x1000f940`: life = maximum = the `.ndp` hit points × the volume scale × the
    /// level ratio; one stage a node until [`Life::staged`] says otherwise.
    pub fn new(
        table: &[NodeDamage],
        parents: Vec<Option<usize>>,
        vital: Vec<bool>,
        volume_scale: f32,
        level_ratio: f32,
    ) -> Self {
        let nodes = (0..parents.len())
            .map(|i| {
                let max = table.get(i).map_or(0.0, |d| d.durability) * volume_scale * level_ratio;
                let status = if vital.get(i).copied().unwrap_or(false) { STATUS_VITAL } else { 0 };
                NodeLife { life: max, max, destroyed: false, stages: 1, stage: 0, status }
            })
            .collect();
        Self {
            nodes,
            parents,
            vital,
            armour: None,
            building: false,
            marked: false,
            dead: false,
            flights: Vec::new(),
        }
    }

    /// Each node's stage count, and which nodes are never hidden.
    ///
    /// STAND-IN: docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured
    /// -- the statuses 4 and 8, a node copying its parent's life fraction or stage, are
    /// read but not modelled; no Mission 01 node carries them.
    pub fn staged(mut self, stages: &[u8], never_hidden: &[bool]) -> Self {
        for (i, n) in self.nodes.iter_mut().enumerate() {
            n.stages = stages.get(i).copied().unwrap_or(1).max(1);
            if never_hidden.get(i).copied().unwrap_or(false) {
                n.status |= STATUS_NEVER_HIDDEN;
            }
        }
        self
    }

    /// `0x10010f30`: a hit on one node, after armour; its life is held at 0, and a node at
    /// 0 is destroyed. Node 0 or a vital node dying marks the object and kills it
    /// (`0x10011098`), unless it is a building, which is only marked (`0x100110ab`). What
    /// the loss does to the node's stage, its children and its drawing waits for the
    /// object's next [`Life::takt`]. Returns whether this hit destroyed the node.
    pub fn hit(&mut self, node: usize, damage: f32) -> bool {
        let damage = self.armour.map_or(damage, |(l, s)| armour(damage, l, s));
        self.lose(node, damage)
    }

    /// `0x10010f30` past the armour: `damage` off one node, as [`Life::hit`] takes it.
    pub fn lose(&mut self, node: usize, damage: f32) -> bool {
        let Some(n) = self.nodes.get_mut(node) else { return false };
        if n.destroyed || damage <= 0.0 {
            return false;
        }
        n.life = (n.life - damage).max(0.0);
        if n.life > 0.0 {
            return false;
        }
        self.destroy(node);
        true
    }

    fn destroy(&mut self, node: usize) {
        let n = &mut self.nodes[node];
        n.life = 0.0;
        n.destroyed = true;
        n.status |= STATUS_DESTROYED;
        if node == 0 || n.status & STATUS_VITAL != 0 {
            self.marked = true;
            self.dead = !self.building;
        }
    }

    /// The nodes in walk order, parents before children.
    fn walk_order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.nodes.len());
        let mut stack: Vec<usize> =
            (0..self.nodes.len()).rev().filter(|&i| self.parents[i].is_none()).collect();
        while let Some(i) = stack.pop() {
            order.push(i);
            stack.extend((0..self.nodes.len()).rev().filter(|&c| self.parents[c] == Some(i)));
        }
        order
    }

    /// The object's life after a tick's hits, at `now_ms` (`0x10012fce`, `0x10011130`,
    /// `0x100131da`):
    ///
    /// 1. **Flights end** once their time is up: flying turns to down (`0x10013568`).
    /// 2. **Destroyed parts are knocked off**: a destroyed node with a parent, on an object
    ///    that is not a building, with none of the bars, flies for three seconds.
    /// 3. **The walk** from the roots down: a node not flying takes its stage; a rise plays
    ///    its explosion, and the last stage hides it unless it is never hidden. A node whose
    ///    stage rose, or which is destroyed, destroys each child, carried along when it is
    ///    flying and down where it stands otherwise.
    pub fn takt(&mut self, now_ms: f64) -> Vec<Change> {
        let mut changes = Vec::new();
        let (ended, flying): (Vec<_>, Vec<_>) = self.flights.iter().partition(|(_, end)| now_ms >= *end);
        self.flights = flying;
        for (n, _) in ended {
            let status = &mut self.nodes[n].status;
            *status = (*status & !STATUS_FLYING) | STATUS_DOWN;
        }
        for i in 0..self.nodes.len() {
            let n = self.nodes[i];
            if n.destroyed && self.parents[i].is_some() && !self.building && n.status & KNOCK_OFF_BARS == 0 {
                self.nodes[i].status |= STATUS_FLYING;
                self.flights.push((i, now_ms + FLIGHT_MS));
                changes.push(Change::KnockedOff(i));
            }
        }
        for i in self.walk_order() {
            let mut rose = false;
            if !self.nodes[i].flying() {
                let (old, new) = (self.nodes[i].stage, self.nodes[i].stage_now());
                if new != old {
                    rose = new > old;
                    self.nodes[i].stage = new;
                    if rose {
                        changes.push(Change::Staged(i));
                    }
                    let n = &mut self.nodes[i];
                    if new == n.stages && n.status & (STATUS_NEVER_HIDDEN | STATUS_HIDDEN) == 0 {
                        n.status |= STATUS_HIDDEN;
                        changes.push(Change::Hidden(i));
                    }
                }
            }
            if rose || self.nodes[i].destroyed {
                let mark = if self.nodes[i].flying() { STATUS_CARRIED } else { STATUS_DOWN };
                let children: Vec<usize> =
                    (0..self.nodes.len()).filter(|&c| self.parents[c] == Some(i)).collect();
                for c in children {
                    if !self.nodes[c].destroyed {
                        self.destroy(c);
                    }
                    let status = &mut self.nodes[c].status;
                    if *status & STATUS_FLYING == 0 {
                        *status = (*status & !(STATUS_CARRIED | STATUS_DOWN)) | mark;
                    }
                }
            }
        }
        changes
    }

    /// A flight ended early, as the world query at `0x100134c1` ends one: its time is up
    /// at the next takt.
    pub fn end_flight(&mut self, node: usize, now_ms: f64) {
        for (n, end) in &mut self.flights {
            if *n == node {
                *end = end.min(now_ms);
            }
        }
    }

    /// The life its nodes have left.
    pub fn total(&self) -> f32 {
        self.nodes.iter().map(|n| n.life).sum()
    }
}

/// A life update's wait, and how far either side of it one may fall
/// (`Control.dll:0x10006364`, `0x1000c774`–`0x1000c7c1`).
pub const LIFE_UPDATE_MS: f64 = 250.0;
pub const LIFE_UPDATE_SPREAD_MS: f64 = 125.0;

/// `0x10010ba0` with its first flag clear, the way the ground's damage is taken: the loss
/// is held to the unit's total life, and every node of every one of `lives` loses the
/// same share of its own life, with no armour asked. No node reaches 0 until the loss
/// reaches the total, and then all do. Returns each life's destroyed nodes.
pub fn share_loss(lives: &mut [&mut Life], loss: f32) -> Vec<Vec<usize>> {
    let total: f32 = lives.iter().map(|l| l.total()).sum();
    if total <= 0.0 || loss <= 0.0 {
        return vec![Vec::new(); lives.len()];
    }
    let share = (loss / total).min(1.0);
    lives
        .iter_mut()
        .map(|life| {
            (0..life.nodes.len())
                .filter(|&n| {
                    let take = share * life.nodes[n].life;
                    life.lose(n, take)
                })
                .collect()
        })
        .collect()
}

/// What the ground under a machine deals it (docs/24-motion.md, "Water and lava beds
/// kill"): the rate its ground contact last read, spent in life updates about every
/// quarter second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundDamage {
    /// Damage a second: the material record's `+0x1a4` of the last face touched.
    pub rate: f32,
    last_ms: f64,
    next_ms: f64,
    seed: u16,
}

impl GroundDamage {
    /// Its first update due one wait from `now_ms`; `seed` sets the unit's spread apart.
    pub fn new(now_ms: f64, seed: u16) -> Self {
        let mut g = Self { rate: 0.0, last_ms: now_ms, next_ms: now_ms, seed: seed | 1 };
        g.next_ms = now_ms + g.wait();
        g
    }

    /// 250 ms, plus a 16-bit random × 250/65536, less 125.
    ///
    /// STAND-IN: docs/24-motion.md#water-and-lava-beds-kill--read-and-measured -- the game's
    /// random source is not read; a 16-bit xorshift (7, 9, 8).
    fn wait(&mut self) -> f64 {
        let mut x = self.seed;
        x ^= x << 7;
        x ^= x >> 9;
        x ^= x << 8;
        self.seed = x;
        LIFE_UPDATE_MS + f64::from(x) * LIFE_UPDATE_MS / 65536.0 - LIFE_UPDATE_SPREAD_MS
    }

    /// The contact read a face's record: a touched face's rate, or a wet bed's.
    pub fn read(&mut self, rate: f32) {
        self.rate = rate;
    }

    /// The loss the life update due by `now_ms` deals, if one is due: the rate × the seconds
    /// since the last update (`0x1000c7e6`, `0x10012a7e`).
    pub fn update(&mut self, now_ms: f64) -> Option<f32> {
        if now_ms < self.next_ms {
            return None;
        }
        let dt = ((now_ms - self.last_ms) * 0.001) as f32;
        self.last_ms = now_ms;
        self.next_ms = now_ms + self.wait();
        Some(self.rate.max(0.0) * dt)
    }
}

/// Whether a body sphere touches the ground point found under its centre: within √2 r
/// (`Control.dll:0x1001a9f9`).
pub fn touching(ground_point: glam::Vec3, centre: glam::Vec3, r: f32) -> bool {
    (ground_point - centre).length_squared() <= 2.0 * r * r
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use parkan_formats::objects::ResourceRef;

    fn table(points: &[f32]) -> Vec<NodeDamage> {
        points
            .iter()
            .map(|&p| NodeDamage { flags: 0, durability: p, density: 0.0, explosion: ResourceRef::default() })
            .collect()
    }

    #[test]
    fn a_blast_falls_off_with_overlap() {
        // R 7, r 1, d 7: ((7 + 1 - 7) / 14)³ of 370; at d 6 the node is not strictly inside.
        assert!((blast(370.0, 7.0, 1.0, 7.0) - 370.0 / 14.0_f32.powi(3)).abs() < 1e-4);
        assert!((blast(370.0, 7.0, 1.0, 6.0) - 370.0 / 7.0_f32.powi(3)).abs() < 1e-4);
        assert_eq!(blast(370.0, 7.0, 1.0, 8.0), 0.0);
        assert_eq!(blast(370.0, 7.0, 1.0, 4.0), 370.0);
        assert_eq!(blast(370.0, 1.0, 7.0, 5.0), 370.0);
        assert_eq!(blast(370.0, 7.0, 0.0, 1.0), 0.0);
    }

    #[test]
    fn armour_takes_most_off_small_hits_and_nothing_off_big_ones() {
        assert_eq!(armour(100.0, 0.5, 0.0001), 51.0);
        assert_eq!(armour(5000.0, 0.5, 0.0001), 5000.0);
    }

    #[test]
    fn a_round_hits_for_its_own_hit_points_and_its_explosion() {
        assert_eq!(round_hit(1.0, 199.0, 1.0), 200.0);
        assert_eq!(round_hit(0.7, 149.0, 1.0), 105.0);
    }

    /// The small target dummy `r_h_01`: a base, two parts on it and a third on the second,
    /// the parts with a damaged block (docs/26, "What the shipped files give").
    fn dummy() -> Life {
        let parents = vec![None, Some(0), Some(0), Some(2)];
        Life::new(&table(&[500.0, 800.0, 800.0, 600.0]), parents, vec![false; 4], 1.0, 1.0)
            .staged(&[1, 2, 2, 2], &[false; 4])
    }

    #[test]
    fn a_part_draws_its_damaged_block_at_half_life_and_is_knocked_off_at_nothing() {
        let mut life = dummy();
        assert!(life.takt(0.0).is_empty());
        assert!(!life.hit(1, 400.0));
        assert_eq!(
            life.takt(10.0),
            vec![Change::Staged(1)],
            "half its life: the damaged block, and its .exp"
        );
        assert_eq!((life.nodes[1].stage, life.nodes[1].block()), (1, 1));
        assert!(life.hit(1, 400.0) && !life.dead);
        assert_eq!(life.takt(20.0), vec![Change::KnockedOff(1)]);
        assert!(life.nodes[1].flying() && !life.nodes[1].hidden(), "it flies, drawn");
        assert!(life.takt(3000.0).is_empty(), "its stage waits while it flies");
        assert_eq!(life.takt(3020.0), vec![Change::Staged(1), Change::Hidden(1)], "then explodes and goes");
        assert!(life.nodes[1].status & STATUS_DOWN != 0);
        assert!(life.takt(4000.0).is_empty(), "once");
    }

    #[test]
    fn a_damaged_parent_takes_its_child_down_where_it_stands_and_a_flying_one_carries_it() {
        let mut life = dummy();
        life.hit(2, 400.0);
        assert_eq!(life.takt(0.0), vec![Change::Staged(2), Change::Staged(3), Change::Hidden(3)]);
        assert!(life.nodes[3].destroyed && life.nodes[3].status & STATUS_DOWN != 0);
        assert!(life.takt(10.0).is_empty(), "a node taken down is not knocked off");

        let mut life = dummy();
        life.hit(2, 800.0);
        assert_eq!(life.takt(0.0), vec![Change::KnockedOff(2)]);
        assert!(
            life.nodes[3].destroyed && life.nodes[3].flying() && !life.nodes[3].hidden(),
            "carried along"
        );
        let mut end = life.takt(3000.0);
        end.sort_by_key(|c| format!("{c:?}"));
        assert_eq!(end, vec![Change::Hidden(2), Change::Hidden(3), Change::Staged(2), Change::Staged(3)]);
    }

    #[test]
    fn node_zero_dying_kills_the_object_and_hides_every_part_still_standing() {
        let mut life = dummy();
        life.hit(1, 800.0);
        life.takt(0.0);
        assert!(life.hit(0, 500.0) && life.dead && life.marked);
        let changes = life.takt(10.0);
        assert!(changes.contains(&Change::Hidden(0)) && changes.contains(&Change::Hidden(2)));
        assert!(changes.contains(&Change::Hidden(3)) && !changes.contains(&Change::Hidden(1)), "{changes:?}");
        assert!(life.nodes[1].flying(), "a part already flying keeps flying");
        assert!(life.nodes.iter().all(|n| n.destroyed && n.life == 0.0));
    }

    #[test]
    fn a_building_whose_node_0_dies_is_only_marked_and_its_shell_stays() {
        // Node 1 hangs off node 0 and dies with it; node 2 is a root of its own; node 0 is
        // never hidden.
        let parents = vec![None, Some(0), None];
        let mut life = Life::new(&table(&[1.0, 40_000.0, 40_000.0]), parents, vec![false; 3], 1.0, 1.0)
            .staged(&[1, 1, 1], &[true, false, false]);
        life.building = true;
        assert!(life.hit(0, 5.0));
        assert!(life.marked && !life.dead, "a shell, not a dead object");
        assert_eq!(life.takt(0.0), vec![Change::Staged(0), Change::Staged(1), Change::Hidden(1)]);
        assert!(!life.nodes[0].hidden(), "never hidden");
        assert!(!life.hit(2, 10_000.0) && life.nodes[2].life == 30_000.0);
        assert!(life.hit(2, 30_000.0) && !life.dead);
        assert_eq!(
            life.takt(10.0),
            vec![Change::Staged(2), Change::Hidden(2)],
            "no part of a building flies"
        );

        let mut unit = Life::new(&table(&[1.0]), vec![None], vec![false], 1.0, 1.0);
        unit.hit(0, 5.0);
        assert!(unit.marked && unit.dead);
    }

    /// A 2 by 2 wall facing -y at `y`, one node of `hit_points`, placed as a building or not.
    fn wall(y: f32, hit_points: f32, building: bool) -> crate::combat::Target {
        use parkan_formats::mesh::{Mesh, NO_SLOT, Node, Slot};
        use parkan_formats::pose::{IDENTITY, Pose};
        let mesh = Mesh {
            name: "wall".into(),
            positions: vec![[-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 2.0], [-1.0, 0.0, 2.0]],
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            nodes: vec![Node {
                name: "wall".into(),
                flags: 0,
                parent: 0xFFFF,
                anim_start: 0xFFFF,
                fallback_key: 0,
                slot_index: std::array::from_fn(|k| if k == 0 { 0 } else { NO_SLOT }),
            }],
            slots: vec![Slot {
                first_triangle: 0,
                triangle_count: 2,
                first_batch: 0,
                batch_count: 0,
                aabb_min: [-1.0, 0.0, 0.0],
                aabb_max: [1.0, 0.0, 2.0],
                sphere: [0.0, 0.0, 1.0, 1.5],
                area: 0.0,
                volume: 0.0,
            }],
            batches: Vec::new(),
            face_flags: vec![0, 0],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
        };
        let mut life = Life::new(&table(&[hit_points]), vec![None], vec![false], 1.0, 1.0);
        life.building = building;
        let at = Vec3::new(20.0, y, 0.0);
        let pose = Pose { translation: [20.0, f64::from(y), 0.0], ..IDENTITY };
        crate::combat::Target {
            parts: vec![crate::combat::Part {
                mesh: std::rc::Rc::new(mesh),
                nodes: vec![pose],
                scale: 1.0,
                life: Some(life),
            }],
            centre: at + Vec3::Z,
            radius: 1.5,
            alive: true,
            position: at,
        }
    }

    #[test]
    fn a_dead_buildings_shell_stays_in_the_world_and_still_stops_rounds() {
        use crate::combat::{Combat, Event, RoundKind};
        use parkan_formats::exp::{Explosion, HIT_DIRECT};
        let laser = RoundKind {
            name: "bl_h_01".into(),
            top_speed: 10_000.0,
            range: 1000.0,
            radius: 0.12,
            hit_points: 249.0,
            hit: Some(Explosion {
                kind: HIT_DIRECT,
                damage: 1.0,
                radius: 1.0,
                values: [1.0; 2],
                placement: 7,
                slots: Vec::new(),
            }),
            range_end: None,
            ..RoundKind::default()
        };
        let g = crate::ground::tests::floor();
        let mut c = Combat { kinds: vec![laser], targets: vec![wall(30.0, 1.0, true)], ..Default::default() };
        let muzzle = Vec3::new(20.0, 5.0, 1.0);
        for _ in 0..2 {
            c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
            let events = c.tick(1.0 / 60.0, &g);
            assert!(events.iter().any(|e| matches!(e, Event::Struck { target: Some(0), .. })), "{events:?}");
            assert!(!events.iter().any(|e| matches!(e, Event::Killed { .. })), "a building is never killed");
        }
        let life = c.targets[0].parts[0].life.as_ref().unwrap();
        assert!(life.marked && life.nodes[0].destroyed && c.targets[0].alive);

        // A unit in its place dies at the first hit and lets the next round by.
        c.targets = vec![wall(30.0, 1.0, false)];
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        assert!(c.tick(1.0 / 60.0, &g).iter().any(|e| matches!(e, Event::Killed { target: 0 })));
        c.fire(0, None, muzzle, Vec3::Y, Vec3::ZERO, 1.0, None);
        assert!(c.tick(1.0 / 60.0, &g).iter().any(|e| matches!(e, Event::Gone { .. })));
    }

    #[test]
    fn the_level_ratio_scales_every_nodes_life() {
        let life = Life::new(&table(&[500.0]), vec![None], vec![false], 1.0, 0.7);
        assert_eq!(life.nodes[0].max, 350.0);
    }

    #[test]
    fn a_ground_loss_takes_a_share_of_every_node_and_kills_them_all_at_the_total() {
        // The hero's two models, 2881 and 4474, with a node in each vital to nothing.
        let mut chassis = Life::new(&table(&[881.0, 2000.0]), vec![None, Some(0)], vec![false; 2], 1.0, 1.0);
        let mut turret = Life::new(&table(&[4000.0, 474.0]), vec![None, None], vec![false; 2], 1.0, 1.0);
        chassis.armour = Some((0.5, 0.0001));
        for _ in 0..2 {
            let gone = share_loss(&mut [&mut chassis, &mut turret], 2500.0);
            assert!(gone.iter().all(Vec::is_empty) && !chassis.dead && !turret.dead);
        }
        let left = chassis.total() + turret.total();
        assert!((left - 2355.0).abs() < 0.1, "no armour asked: {left}");
        assert!((chassis.nodes[1].life / chassis.nodes[0].life - 2000.0 / 881.0).abs() < 1e-3);
        let gone = share_loss(&mut [&mut chassis, &mut turret], 2500.0);
        assert_eq!(gone, vec![vec![0, 1], vec![0, 1]]);
        assert!(chassis.dead && turret.dead && chassis.total() == 0.0);
    }

    #[test]
    fn the_life_update_comes_every_quarter_second_give_or_take_an_eighth() {
        let mut g = GroundDamage::new(0.0, 7);
        assert_eq!(g.update(100.0), None);
        g.read(10_000.0);
        let (mut last, mut t) = (0.0, 0.0);
        let mut updates = 0;
        while updates < 200 {
            t += 1.0;
            if let Some(loss) = g.update(t) {
                let gap = t - last;
                assert!((125.0..=376.0).contains(&gap), "{gap}");
                assert!((loss - 10.0 * gap as f32).abs() < 1e-2, "rate × the seconds since the last");
                last = t;
                updates += 1;
            }
        }
        assert!((t / 200.0 - 250.0).abs() < 25.0, "{}", t / 200.0);
        let wide = glam::Vec3::new(0.0, 0.0, 2.0);
        assert!(touching(glam::Vec3::ZERO, wide, 1.5) && !touching(glam::Vec3::ZERO, wide, 1.4));
    }
}
