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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeLife {
    pub life: f32,
    pub max: f32,
    pub destroyed: bool,
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
    /// Agent kind 3, a building: node 0 or a vital node dying only marks it.
    pub building: bool,
    /// The owner word reads `0xfffe` (`0x10011098`): node 0 or a vital node is destroyed.
    pub marked: bool,
    /// The object died: marked, and not a building.
    pub dead: bool,
}

impl Life {
    /// `0x1000f940`: life = maximum = the `.ndp` hit points × the volume scale × the
    /// level ratio.
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
                NodeLife { life: max, max, destroyed: false }
            })
            .collect();
        Self { nodes, parents, vital, armour: None, building: false, marked: false, dead: false }
    }

    /// `0x10010f30`: a hit on one node, after armour; its life is held at 0, and a node
    /// at 0 is destroyed with its children (`0x10011130`). Node 0 or a vital node dying
    /// marks the object and kills it (`0x10011098`), unless it is a building, which is only
    /// marked (`0x100110ab`): a shell whose model and other nodes stay, to be shot apart.
    /// Returns the nodes this hit destroyed.
    pub fn hit(&mut self, node: usize, damage: f32) -> Vec<usize> {
        let damage = self.armour.map_or(damage, |(l, s)| armour(damage, l, s));
        self.lose(node, damage)
    }

    /// `0x10010f30` past the armour: `damage` off one node, as [`Life::hit`] takes it.
    pub fn lose(&mut self, node: usize, damage: f32) -> Vec<usize> {
        let Some(n) = self.nodes.get_mut(node) else { return Vec::new() };
        if n.destroyed || damage <= 0.0 {
            return Vec::new();
        }
        n.life = (n.life - damage).max(0.0);
        if n.life > 0.0 {
            return Vec::new();
        }
        let mut destroyed = Vec::new();
        let mut stack = vec![node];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut self.nodes[i].destroyed, true) {
                continue;
            }
            self.nodes[i].life = 0.0;
            destroyed.push(i);
            if i == 0 || self.vital.get(i).copied().unwrap_or(false) {
                self.marked = true;
                self.dead = !self.building;
            }
            stack.extend((0..self.nodes.len()).filter(|&c| self.parents[c] == Some(i)));
        }
        destroyed
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
                .flat_map(|n| {
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

    #[test]
    fn node_zero_dying_kills_the_object_and_its_children_with_it() {
        let parents = vec![None, Some(0), Some(0), Some(2)];
        let mut life = Life::new(&table(&[500.0, 800.0, 800.0, 600.0]), parents, vec![false; 4], 1.0, 1.0);
        assert!(life.hit(3, 600.0) == vec![3] && !life.dead);
        assert!(life.hit(0, 200.0).is_empty());
        assert_eq!(life.nodes[0].life, 300.0);
        life.hit(0, 200.0);
        let mut gone = life.hit(0, 200.0);
        gone.sort_unstable();
        assert_eq!(gone, vec![0, 1, 2]);
        assert!(life.dead && life.nodes.iter().all(|n| n.destroyed && n.life == 0.0));
    }

    #[test]
    fn a_building_whose_node_0_dies_is_only_marked_and_its_other_nodes_still_take_hits() {
        // Node 1 hangs off node 0 and dies with it; node 2 is a root of its own.
        let parents = vec![None, Some(0), None];
        let mut life = Life::new(&table(&[1.0, 40_000.0, 40_000.0]), parents, vec![false; 3], 1.0, 1.0);
        life.building = true;
        let mut gone = life.hit(0, 5.0);
        gone.sort_unstable();
        assert_eq!(gone, vec![0, 1]);
        assert!(life.marked && !life.dead, "a shell, not a dead object");
        assert!(life.hit(2, 10_000.0).is_empty() && life.nodes[2].life == 30_000.0);
        assert_eq!(life.hit(2, 30_000.0), vec![2]);
        assert!(!life.dead);

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
