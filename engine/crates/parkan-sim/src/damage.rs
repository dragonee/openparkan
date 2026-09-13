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
        Self { nodes, parents, vital, armour: None, dead: false }
    }

    /// `0x10010f30`: a hit on one node, after armour; its life is held at 0, and a node
    /// at 0 is destroyed with its children (`0x10011130`). Node 0 or a vital node dying
    /// kills the object. Returns the nodes this hit destroyed.
    pub fn hit(&mut self, node: usize, damage: f32) -> Vec<usize> {
        let Some(n) = self.nodes.get_mut(node) else { return Vec::new() };
        if n.destroyed || damage <= 0.0 {
            return Vec::new();
        }
        let damage = self.armour.map_or(damage, |(l, s)| armour(damage, l, s));
        let n = &mut self.nodes[node];
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
                self.dead = true;
            }
            stack.extend((0..self.nodes.len()).filter(|&c| self.parents[c] == Some(i)));
        }
        destroyed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn the_level_ratio_scales_every_nodes_life() {
        let life = Life::new(&table(&[500.0]), vec![None], vec![false], 1.0, 0.7);
        assert_eq!(life.nodes[0].max, 350.0);
    }
}
