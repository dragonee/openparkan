//! A unit's batteries and what draws on them, on the unit's power tick: its engines, its repair
//! system, its detection shield and every device that idles at its power figure, together with
//! its fight shield and its guns. See `docs/23-economy.md`, "Bots spend power through the same
//! code, priced by part", and `docs/26-damage.md`, "Repair".

use parkan_formats::control::{self, BATTERY_CAPACITY, Component};
use parkan_sim::damage::Life;
use parkan_sim::economy::{Battery, POWER_TICK_MS};
use parkan_sim::guns::Gun;
use parkan_sim::input::Switches;
use parkan_sim::power::{self, Draw, Repair};
use parkan_sim::shield::Shield;

use crate::assembly::Assembly;

/// The detection shield's value that adds to its draw while it camouflages (`0x100264b0`).
pub const CAMOUFLAGE_DRAW: usize = 4;
/// The repair system's values: the points a second, and the charge a point.
pub const REPAIR_RATE: usize = 0;
pub const REPAIR_COST: usize = 1;
/// The classes priced their own way, which [`Power::devices`] leaves out: guns, engines, fight
/// and detection shields, repair, batteries and deflectors.
const PRICED_APART: [i32; 7] = [
    control::GUN_TYPE,
    control::ENGINE_TYPE,
    control::FIGHT_SHIELD_TYPE,
    control::DETECT_SHIELD_TYPE,
    control::REPAIR_TYPE,
    control::BATTERY_TYPE,
    control::DEFLECTOR_TYPE,
];

/// A node, as a part and a node of it; `None` for a device that names none.
pub type Node = Option<(usize, usize)>;

/// A unit's power: what gives and what draws, and when its next tick is due.
#[derive(Clone, Debug, PartialEq)]
pub struct Power {
    /// Each battery and its node.
    pub batteries: Vec<(Battery, Node)>,
    pub repair: Option<(Repair, Node)>,
    /// Each engine's power figure and its node.
    pub engines: Vec<(f32, Node)>,
    /// The detection shield's power figure, what camouflage adds to it, and its node.
    pub detection: Option<(f32, f32, Node)>,
    /// Every other device that draws its power figure a second while switched on and whole
    /// (`0x10021860`): its channel, its power figure and its node.
    pub devices: Vec<(u8, f32, Node)>,
    /// When the unit's next power tick is due and when its last ran, ms of the unit's time.
    pub next_ms: f64,
    pub last_ms: f64,
}

/// The node a component names in part `p`.
fn node(p: usize, k: &Component) -> Node {
    usize::try_from(k.node).ok().map(|n| (p, n))
}

impl Power {
    /// The power of the unit built from `path`, from every component it ends up with, a fitted
    /// part's in its slot (docs/28, "A fitted part takes over its slot"); `None` for a unit with
    /// no battery that holds anything.
    ///
    /// A device whose initial state (record `+0x18`) is 0 starts switched off (`0x10021d86`);
    /// every other starts on, and a repair system is switched by its owner.
    pub fn load(assembly: &mut Assembly, kind: u32, path: &str) -> Option<Power> {
        let classes: Vec<i32> = (0..power::CHANNEL_OF_CLASS.len() as i32).collect();
        let slots = crate::shields::slots(assembly, kind, path, &classes);
        let batteries: Vec<(Battery, Node)> = slots
            .iter()
            .filter(|(_, _, k)| k.type_id == control::BATTERY_TYPE && k.values[BATTERY_CAPACITY] > 0.0)
            .map(|(p, _, k)| (Battery::new(k.values[BATTERY_CAPACITY], k.power), node(*p, k)))
            .collect();
        if batteries.is_empty() {
            return None;
        }
        let of = |class: i32| slots.iter().filter(move |(_, _, k)| k.type_id == class);
        let repair = of(control::REPAIR_TYPE).next().map(|(p, _, k)| {
            (Repair { rate: k.values[REPAIR_RATE], cost: k.values[REPAIR_COST], power: k.power }, node(*p, k))
        });
        let engines = of(control::ENGINE_TYPE).map(|(p, _, k)| (k.power, node(*p, k))).collect();
        let detection = of(control::DETECT_SHIELD_TYPE)
            .next()
            .map(|(p, _, k)| (k.power, k.values[CAMOUFLAGE_DRAW], node(*p, k)));
        let devices = slots
            .iter()
            .filter(|(_, _, k)| !PRICED_APART.contains(&k.type_id) && k.power > 0.0 && k.index != Some(0))
            .map(|(p, _, k)| (power::channel(k.type_id), k.power, node(*p, k)))
            .collect();
        Some(Power { batteries, repair, engines, detection, devices, next_ms: 0.0, last_ms: 0.0 })
    }

    /// The batteries' fill, `Σ capacity × charge ÷ Σ capacity` (`Control.dll:0x1002b42b`): what
    /// the device manager answers as id 1, and the panel's battery arc.
    pub fn fill(&self) -> f32 {
        let capacity: f32 = self.batteries.iter().map(|(b, _)| b.capacity).sum();
        let held: f32 = self.batteries.iter().map(|(b, _)| b.capacity * b.charge).sum();
        if capacity > 0.0 { held / capacity } else { 0.0 }
    }

    /// Every battery set to `fill` of full, as id 1 writes it (`0x1002bae6`).
    pub fn set_fill(&mut self, fill: f32) {
        for (b, _) in &mut self.batteries {
            b.charge = fill.clamp(0.0, 1.0);
        }
    }

    /// The seconds since the last power tick when one is due at `now`, the next set `jitter`
    /// ms either side of 250 on (`Control.dll:0x1000c756`).
    pub fn due(&mut self, now: f64, jitter: f64) -> Option<f32> {
        if now < self.next_ms {
            return None;
        }
        let dt = ((now - self.last_ms) / 1000.0) as f32;
        self.last_ms = now;
        self.next_ms = now + POWER_TICK_MS + jitter;
        Some(dt.max(0.0))
    }

    /// One power tick of `dt` seconds (`Control.dll:0x1002d340`). The batteries give what their
    /// charge and condition allow; the engines want their power figure × `engine_share`, the
    /// repair system its idle draw and the points it would restore while `switches` has it on,
    /// the detection shield its figure and camouflage's while on, every other device its figure,
    /// the shield what [`Shield::want`] says and each gun what its capacitor lacks. The channels
    /// are served in their order ([`power::serve`]), the shield takes its group's level for its
    /// recharge, the repair's points go to `lives` (by part), each gun's capacitor fills, and the
    /// batteries drain by what was used. A device on a destroyed node draws nothing.
    pub fn tick(
        &mut self,
        dt: f32,
        lives: &mut [Option<&mut Life>],
        shield: Option<&mut Shield>,
        guns: &mut [Gun],
        engine_share: f32,
        switches: Switches,
    ) {
        let condition = |lives: &[Option<&mut Life>], node: Node| -> f32 {
            let Some((p, n)) = node else { return 1.0 };
            match lives.get(p).and_then(|l| l.as_deref()).and_then(|l| l.nodes.get(n)) {
                Some(l) if l.max > 0.0 => (l.life / l.max).clamp(0.0, 1.0),
                Some(l) => f32::from(u8::from(!l.destroyed)),
                None => 1.0,
            }
        };
        let whole = |lives: &[Option<&mut Life>], node: Node| condition(lives, node) > 0.0;
        let supplies: Vec<f32> =
            self.batteries.iter().map(|(b, n)| b.supply(dt, condition(lives, *n))).collect();
        let supply: f32 = supplies.iter().sum();

        let mut draws = Vec::new();
        for &(figure, n) in &self.engines {
            let want = if whole(lives, n) { figure * engine_share * dt } else { 0.0 };
            draws.push(Draw { channel: power::channel(control::ENGINE_TYPE), want });
        }
        let lack = power::lack(lives.iter().filter_map(|l| l.as_deref()), false);
        // The repair system with its condition, and where its draw stands.
        let repair = self.repair.map(|(r, n)| (r, condition(lives, n), draws.len()));
        if let Some((r, c, _)) = repair {
            let want = r.want(switches.repair, dt, c, lack);
            draws.push(Draw { channel: power::channel(control::REPAIR_TYPE), want });
        }
        for &(channel, figure, n) in &self.devices {
            draws.push(Draw { channel, want: if whole(lives, n) { figure * dt } else { 0.0 } });
        }
        if let Some((figure, camouflage, n)) = self.detection {
            let a_second = figure + if switches.camouflage { camouflage } else { 0.0 };
            let want = if whole(lives, n) { a_second * dt } else { 0.0 };
            draws.push(Draw { channel: power::channel(control::DETECT_SHIELD_TYPE), want });
        }
        let shielded = shield.as_ref().map(|s| (s.want(dt), draws.len()));
        if let Some((want, _)) = shielded {
            draws.push(Draw { channel: power::channel(control::FIGHT_SHIELD_TYPE), want });
        }
        let first_gun = draws.len();
        for g in guns.iter() {
            draws.push(Draw { channel: power::channel(control::GUN_TYPE), want: g.want(dt) });
        }

        let (levels, used) = power::serve(supply, &draws);
        if let Some((r, c, at)) = repair
            && let Some(level) = levels[at]
        {
            let points = r.points(switches.repair, level, dt, c, lack);
            let mut own: Vec<&mut Life> = lives.iter_mut().filter_map(|l| l.as_deref_mut()).collect();
            power::restore(&mut own, points, false);
        }
        if let (Some(shield), Some((_, at))) = (shield, shielded)
            && let Some(level) = levels[at]
        {
            shield.level = level;
        }
        for (g, level) in guns.iter_mut().zip(&levels[first_gun..]) {
            if let Some(level) = level {
                g.serve(*level, dt);
            }
        }
        if supply > 0.0 {
            for ((b, _), give) in self.batteries.iter_mut().zip(&supplies) {
                b.drain(used * give / supply);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::ndp::NodeDamage;
    use parkan_formats::objects::ResourceRef;
    use parkan_sim::shield::SECTORS;

    /// The hero's figures (docs/23, docs/26): a 4,080 battery putting out 6.8, a repair system of
    /// 15 a second at 0.04 and 0.1 idle, a detection shield idling at 0.11 that camouflage adds
    /// 0.3 to, the radar's 0.08 on channel 2, and the shield of 1,850 at 15 a second, 0.04 a point
    /// and 2 idle behind a deflector of 0.2.
    fn hero() -> (Power, Shield, Life) {
        let power = Power {
            batteries: vec![(Battery::new(4080.0, 6.8), Some((0, 0)))],
            repair: Some((Repair { rate: 15.0, cost: 0.04, power: 0.1 }, Some((0, 0)))),
            engines: vec![(0.1, Some((0, 0)))],
            detection: Some((0.11, 0.3, Some((0, 0)))),
            devices: vec![(2, 0.08, Some((0, 1)))],
            next_ms: 0.0,
            last_ms: 0.0,
        };
        let mut shield = Shield::new(1850.0, 15.0, 0.04, [0.9; SECTORS]);
        shield.shield_power = 2.0;
        shield.deflector_power = 0.2;
        let table: Vec<NodeDamage> = [400.0, 200.0]
            .iter()
            .map(|&d| NodeDamage { flags: 0, durability: d, density: 0.0, explosion: ResourceRef::default() })
            .collect();
        (power, shield, Life::new(&table, vec![None, Some(0)], vec![false; 2], 1.0, 1.0))
    }

    #[test]
    fn standing_idle_the_hero_spends_its_devices_idle_draws_and_nothing_on_walking() {
        let (mut power, mut shield, mut life) = hero();
        // A second at a walk: the hero's states carry an engine factor of 0 (docs/24).
        power.tick(1.0, &mut [Some(&mut life)], Some(&mut shield), &mut [], 0.0, Switches::default());
        let spent = (1.0 - power.fill()) * 4080.0;
        assert!((spent - (0.08 + 0.11 + 2.0 + 0.2)).abs() < 1e-3, "{spent}");
        assert_eq!(shield.level, 1.0);
    }

    #[test]
    fn repair_heals_the_heros_nodes_and_camouflage_and_a_recharging_shield_cost_charge() {
        let (mut power, mut shield, mut life) = hero();
        life.nodes[1].life = 100.0;
        shield.fills = [0.0; SECTORS];
        let switches = Switches { repair: true, camouflage: true, infrared: false };
        power.tick(1.0, &mut [Some(&mut life)], Some(&mut shield), &mut [], 0.0, switches);
        // 15 points restored to node 1, costing 0.6 and the 0.1 idle; the repair system's own
        // node 0 is whole, so it works at full condition.
        assert!((life.nodes[1].life - 115.0).abs() < 1e-2, "{}", life.nodes[1].life);
        let spent = (1.0 - power.fill()) * 4080.0;
        let expected = 0.1 + 0.6 + 0.08 + (0.11 + 0.3) + (2.0 + 15.0 * 0.04 + 0.2);
        assert!((spent - expected).abs() < 1e-2, "{spent} of {expected}");
        // Off again, it restores nothing.
        let switches = Switches { repair: false, ..switches };
        power.tick(1.0, &mut [Some(&mut life)], Some(&mut shield), &mut [], 0.0, switches);
        assert!((life.nodes[1].life - 115.0).abs() < 1e-2);
    }

    #[test]
    fn the_guns_are_served_last_and_a_flat_battery_serves_nothing() {
        let (mut power, mut shield, mut life) = hero();
        let mut values = [0.0; 16];
        values[..4].copy_from_slice(&[0.0, 200.0, 5.5, 200.0]);
        let record = Component {
            type_id: control::GUN_TYPE,
            resource: ResourceRef::default(),
            index: None,
            entries: Vec::new(),
            label: String::new(),
            values,
            power: 0.0,
            node: 0,
            mass: 0.0,
            flags: 0,
            group: -1,
            weights: [0.0; 2],
        };
        let mut laser = Gun::new(0, &record, &[]);
        laser.charge = 0.0;
        // A quarter second gives 1.7, of which the idle draws take 0.6: the rest charges the laser.
        power.tick(
            0.25,
            &mut [Some(&mut life)],
            Some(&mut shield),
            std::slice::from_mut(&mut laser),
            0.0,
            Switches::default(),
        );
        assert!((laser.charge - (6.8 - 2.39) * 0.25).abs() < 1e-3, "{}", laser.charge);
        power.set_fill(0.0);
        let before = laser.charge;
        power.tick(
            0.25,
            &mut [Some(&mut life)],
            Some(&mut shield),
            std::slice::from_mut(&mut laser),
            0.0,
            Switches::default(),
        );
        assert_eq!((laser.charge, shield.level), (before, 0.0), "nothing from a flat battery");
    }
}
