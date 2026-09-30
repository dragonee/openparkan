//! A unit's power tick (`Control.dll:0x1002d340`): every device reports what it gives or wants
//! over the tick, the channels are served from what the batteries give in a fixed order, and the
//! batteries drain by what was used. See `docs/23-economy.md`, "Bots spend power through the same
//! code, priced by part", and `docs/26-damage.md`, "Repair".

use crate::damage::Life;

/// The channel each component class draws on, by class (`Control.dll:0x1003ccc8`): the engines
/// 3, doors, computers and repair 0, the radar and camera 2, shields, deflectors and armour 5,
/// turrets and guns 4, and the batteries 1.
pub const CHANNEL_OF_CLASS: [u8; 32] =
    [0, 4, 4, 0, 2, 3, 0, 0, 2, 5, 5, 0, 0, 0, 1, 0, 3, 2, 0, 1, 3, 5, 4, 0, 4, 0, 3, 5, 0, 0, 4, 0];

/// The channel class `class` draws on; 0 for a class past the table.
pub fn channel(class: i32) -> u8 {
    usize::try_from(class).ok().and_then(|c| CHANNEL_OF_CLASS.get(c)).copied().unwrap_or(0)
}

/// The channel groups in the order they are served, each at one level (`0x1002d3c1`–`0x1002d439`):
/// the engines' 3, then 0, then 2 together with 5, then the weapons' 4. The batteries' 1 is
/// served last, with what the others used.
pub const SERVED: [u8; 4] = [1 << 3, 1 << 0, (1 << 2) | (1 << 5), 1 << 4];

/// What one consumer wants over the tick, and the channel it wants it on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub channel: u8,
    pub want: f32,
}

/// `supply` shared over `draws` (`0x1002dca0`): each group of [`SERVED`] in turn is given
/// `min(1, what is left ÷ what it wants)` of what it wants, and a group that wants nothing is
/// passed over, its level not written. Returns each draw's level, `None` where its group was
/// passed over, and what was used.
pub fn serve(supply: f32, draws: &[Draw]) -> (Vec<Option<f32>>, f32) {
    let mut left = supply.max(0.0);
    let mut levels = vec![None; draws.len()];
    for mask in SERVED {
        let in_group = |d: &Draw| mask & (1 << d.channel) != 0;
        let want: f32 = draws.iter().filter(|d| in_group(d)).map(|d| d.want).sum();
        if want == 0.0 {
            continue;
        }
        let level = (left / want).min(1.0);
        left -= level * want;
        for (i, d) in draws.iter().enumerate() {
            if in_group(d) {
                levels[i] = Some(level);
            }
        }
    }
    (levels, supply.max(0.0) - left)
}

/// A repair system, class 15 (`i_rps`): value 0 the points it restores a second at full
/// condition, value 1 the charge a point costs, and its power figure the draw it idles at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Repair {
    pub rate: f32,
    pub cost: f32,
    pub power: f32,
}

impl Repair {
    /// The points it would restore over `dt` s at `condition`, the unit lacking `lack`: at most
    /// value 0 × condition a second, never more than the unit lacks.
    fn wanted(&self, dt: f32, condition: f32, lack: f32) -> f32 {
        (self.rate * condition * dt).min(lack.max(0.0))
    }

    /// Slot 5 (`Control.dll:0x10022b20`): nothing while off or on a destroyed node; otherwise
    /// its idle draw and the charge the points it would restore cost.
    pub fn want(&self, on: bool, dt: f32, condition: f32, lack: f32) -> f32 {
        if !on || condition <= 0.0 {
            return 0.0;
        }
        self.power * dt + self.cost * self.wanted(dt, condition, lack)
    }

    /// Slot 6 (`0x10022bb0`): the points it restores served at `level`. What the level gives
    /// beyond its idle draw buys points at value 1 each; a repair with no cost restores what it
    /// wanted.
    pub fn points(&self, on: bool, level: f32, dt: f32, condition: f32, lack: f32) -> f32 {
        let charge = level * self.want(on, dt, condition, lack) - self.power * dt;
        if !on || condition <= 0.0 || charge < 0.0 {
            0.0
        } else if self.cost != 0.0 {
            charge / self.cost
        } else {
            self.wanted(dt, condition, lack)
        }
    }
}

/// What an object's nodes lack of full (`Control.dll:0x10010b10`): a unit's, over those that
/// still have life; a `building`'s, its whole life's maximum less its life, destroyed nodes and
/// all (`0x10010b1c`), as the repair system of an agent of kind 3 asks (`0x10022b0e`).
pub fn lack<'a>(lives: impl IntoIterator<Item = &'a Life>, building: bool) -> f32 {
    lives
        .into_iter()
        .flat_map(|l| &l.nodes)
        .filter(|n| building || n.life > 0.0)
        .map(|n| n.max - n.life)
        .sum()
}

/// `points` of repair handed to an object's nodes (`0x10010ba0` with a gain, `0x10010cd4`): in
/// index order, part by part, each node filled before the next. A unit's destroyed node is
/// passed over (`0x10010d0b`); a `building`'s takes its share with the rest and has life
/// again ([`Life::gain`]), so a building brings back the parts it lost one by one, each whole
/// before the next begins. What a node's life now says of its stage waits for the next life
/// takt. Returns what was given.
pub fn restore(lives: &mut [&mut Life], points: f32, building: bool) -> f32 {
    let mut left = points.max(0.0);
    for life in lives.iter_mut() {
        for n in 0..life.nodes.len() {
            if left <= 0.0 {
                break;
            }
            if life.nodes[n].destroyed && !building {
                continue;
            }
            left -= life.gain(n, left);
        }
    }
    points.max(0.0) - left
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::ndp::NodeDamage;
    use parkan_formats::objects::ResourceRef;

    fn life(points: &[f32]) -> Life {
        let table: Vec<NodeDamage> = points
            .iter()
            .map(|&d| NodeDamage { flags: 0, durability: d, density: 0.0, explosion: ResourceRef::default() })
            .collect();
        Life::new(&table, vec![None; points.len()], vec![false; points.len()], 1.0, 1.0)
    }

    #[test]
    fn the_classes_draw_on_the_channels_the_table_gives() {
        assert_eq!(
            [channel(5), channel(15), channel(8), channel(9), channel(21), channel(2), channel(19)],
            [3, 0, 2, 5, 5, 4, 1]
        );
        assert_eq!(channel(99), 0);
    }

    #[test]
    fn the_engines_are_served_first_the_shields_with_the_radar_and_the_guns_get_what_is_left() {
        let draws = [
            Draw { channel: 4, want: 2.0 },
            Draw { channel: 5, want: 0.5 },
            Draw { channel: 3, want: 1.0 },
            Draw { channel: 2, want: 0.5 },
            Draw { channel: 0, want: 0.0 },
        ];
        // 2.5 given: the engine whole, the radar and shield whole, the guns a quarter.
        let (levels, used) = serve(2.5, &draws);
        assert_eq!(levels, vec![Some(0.25), Some(1.0), Some(1.0), Some(1.0), None]);
        assert_eq!(used, 2.5);
        // 1.5 given: the radar and the shield share what the engine leaves.
        let (levels, used) = serve(1.5, &draws);
        assert_eq!(levels, vec![Some(0.0), Some(0.5), Some(1.0), Some(0.5), None]);
        assert_eq!(used, 1.5);
        let (levels, used) = serve(10.0, &draws);
        assert!(levels.iter().flatten().all(|&l| l == 1.0) && used == 4.0, "a surplus is not drawn");
    }

    #[test]
    fn a_repair_system_restores_its_rate_node_by_node_at_its_cost_and_only_while_on() {
        // The hero's: 15 a second, 0.04 a point, 0.1 idle.
        let repair = Repair { rate: 15.0, cost: 0.04, power: 0.1 };
        let mut hull = life(&[100.0, 50.0, 80.0]);
        hull.nodes[0].life = 90.0;
        hull.nodes[1].life = 0.0;
        hull.nodes[1].destroyed = true;
        hull.nodes[2].life = 40.0;
        let lack = lack([&hull], false);
        assert_eq!(lack, 50.0, "the destroyed node lacks nothing");
        assert_eq!(repair.want(false, 1.0, 1.0, lack), 0.0);
        assert!((repair.want(true, 1.0, 1.0, lack) - (0.1 + 0.04 * 15.0)).abs() < 1e-6);
        // Served whole: 15 points, node 0 filled first, the destroyed node passed over.
        let points = repair.points(true, 1.0, 1.0, 1.0, lack);
        assert!((points - 15.0).abs() < 1e-3);
        assert!((restore(&mut [&mut hull], points, false) - 15.0).abs() < 1e-3);
        assert_eq!(hull.nodes[0].life, 100.0);
        assert_eq!(hull.nodes[1].life, 0.0);
        assert!((hull.nodes[2].life - 45.0).abs() < 1e-3);
        // Served at half, the idle draw is paid first: (0.35 − 0.1) ÷ 0.04 points.
        let points = repair.points(true, 0.5, 1.0, 1.0, 45.0);
        assert!((points - 0.25 / 0.04).abs() < 1e-3, "{points}");
        // Its node at half condition restores half as fast; a destroyed one, nothing.
        assert!((repair.points(true, 1.0, 1.0, 0.5, 45.0) - 7.5).abs() < 1e-3);
        assert_eq!(repair.points(true, 1.0, 1.0, 0.0, 45.0), 0.0);
    }
    #[test]
    fn a_buildings_repair_brings_back_its_destroyed_parts_one_by_one_in_node_order() {
        // A building's own repair system: 100 a second, 0.0002 a point, 0.01 idle.
        let repair = Repair { rate: 100.0, cost: 0.0002, power: 0.01 };
        let mut hull = life(&[400.0, 150.0, 120.0]);
        hull.building = true;
        hull.nodes[0].life = 380.0;
        for n in [1, 2] {
            hull.lose(n, 1000.0);
        }
        assert!(hull.nodes[1].destroyed && hull.nodes[2].destroyed);
        let lack = lack([&hull], true);
        assert_eq!(lack, 290.0, "a building lacks its destroyed nodes' life too");
        // Each second fills the lowest node that lacks anything before the next takes a point.
        let points = repair.points(true, 1.0, 1.0, 1.0, lack);
        assert!((points - 100.0).abs() < 1e-3, "{points}");
        restore(&mut [&mut hull], points, true);
        assert_eq!(hull.nodes[0].life, 400.0);
        assert!((hull.nodes[1].life - 80.0).abs() < 1e-3 && !hull.nodes[1].destroyed, "{:?}", hull.nodes[1]);
        assert!(hull.nodes[2].destroyed && hull.nodes[2].life == 0.0, "the next waits its turn");
        restore(&mut [&mut hull], 100.0, true);
        assert_eq!(hull.nodes[1].life, 150.0);
        assert!((hull.nodes[2].life - 30.0).abs() < 1e-3 && !hull.nodes[2].destroyed);
        // A unit's passes the destroyed node over.
        let mut unit = life(&[100.0, 50.0]);
        unit.lose(1, 1000.0);
        assert_eq!(restore(&mut [&mut unit], 40.0, false), 0.0);
        assert!(unit.nodes[1].destroyed);
    }
}
