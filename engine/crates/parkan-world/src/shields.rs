//! The fight shield, deflector and armour a placed object carries, as its controllers give them
//! (docs/26-damage.md, "Shields: a generator, a deflector, six sectors" and "Armour").

use parkan_formats::control::{self, Component};
use parkan_sim::shield::{SECTORS, Shield};

use crate::assembly::Assembly;
use crate::designs::ARMOUR_TYPE;
use crate::robot::controller;

/// The fight shield value holding a sector's maximum, its recharge a second and the charge a
/// point costs.
pub const SHIELD_MAX: usize = 0;
pub const SHIELD_RECHARGE: usize = 1;
pub const SHIELD_COST: usize = 2;
/// The armour values that cut a hit: its linear and its square factor. Value 0 is a weight
/// over area, and cuts nothing (docs/26, "Armour").
pub const ARMOUR_LINEAR: usize = 1;
pub const ARMOUR_SQUARE: usize = 2;

/// The components of `types` the object built from `path` ends up with, each as its part, its
/// index in that part's controller and its record, in load order.
///
/// Each part's slots are taken in part order, and an internal part fitted into a slot re-parses
/// it with its own first record, keeping the slot's node (docs/28, "A fitted part takes over its
/// slot").
pub fn slots(
    assembly: &mut Assembly,
    kind: u32,
    path: &str,
    types: &[i32],
) -> Vec<(usize, usize, Component)> {
    let parts = assembly.parts(kind, path);
    let mut slots: Vec<(usize, usize, Component)> = Vec::new();
    for (p, part) in parts.iter().enumerate() {
        let Some(c) = controller(assembly, &part.record).ok().flatten() else { continue };
        for (i, k) in c.components.iter().enumerate() {
            if types.contains(&k.type_id) {
                slots.push((p, i, k.clone()));
            }
        }
    }
    for (p, i, record) in assembly.fitted(path) {
        let Some(c) = controller(assembly, &record).ok().flatten() else { continue };
        let Some(k) = c.components.first() else { continue };
        if let Some(slot) = slots.iter_mut().find(|s| s.0 == p && s.1 == i && s.2.type_id == k.type_id) {
            slot.2 = Component { node: slot.2.node, ..k.clone() };
        }
    }
    slots
}

/// Every component of the object built from `path` as its device manager lists them, each as
/// its part, its class and its node in that part: what the fight module weighs when it picks
/// the part of a target to aim at (`Behavior.dll:0x10025830`, docs/29, "The part the AI aims
/// at").
pub fn devices(assembly: &mut Assembly, kind: u32, path: &str) -> Vec<(usize, i32, usize)> {
    // A component's class is an id from 1 to 30 (docs/13, "The component record").
    let classes: Vec<i32> = (1..=30).collect();
    slots(assembly, kind, path, &classes)
        .into_iter()
        .filter_map(|(p, _, k)| Some((p, k.type_id, usize::try_from(k.node).ok()?)))
        .collect()
}

/// The shield of the object built from `path`, where it has both a fight shield and a
/// deflector; its sectors' maximum times the level ratio `ratio`. The generator's resource is
/// the effect a hit plays.
pub fn load(assembly: &mut Assembly, kind: u32, path: &str, ratio: f32) -> Option<Shield> {
    let slots = slots(assembly, kind, path, &[control::FIGHT_SHIELD_TYPE, control::DEFLECTOR_TYPE]);
    let of = |type_id: i32| slots.iter().find(|s| s.2.type_id == type_id);
    let (generator, deflector) = (of(control::FIGHT_SHIELD_TYPE)?, of(control::DEFLECTOR_TYPE)?);
    let node = |(p, _, k): &(usize, usize, Component)| usize::try_from(k.node).ok().map(|n| (*p, n));
    let g = &generator.2;
    let mut shield = Shield::new(
        g.values[SHIELD_MAX] * ratio,
        g.values[SHIELD_RECHARGE],
        g.values[SHIELD_COST],
        std::array::from_fn(|i| deflector.2.values[i]),
    );
    shield.shield_power = g.power;
    shield.deflector_power = deflector.2.power;
    shield.effect = g.resource.member.clone();
    shield.shield_node = node(generator);
    shield.deflector_node = node(deflector);
    debug_assert_eq!(SECTORS, 6);
    Some(shield)
}

/// The armour of the object built from `path`: the linear and square factors of its last
/// class-27 component, which every hit on any of its nodes passes through
/// (`Control.dll:0x1002d7b6`, `0x10010030`). A chassis's own slot is (0, 1, 0), which cuts
/// nothing; a fitted armour part replaces it.
pub fn armour(assembly: &mut Assembly, kind: u32, path: &str) -> Option<(f32, f32)> {
    slots(assembly, kind, path, &[ARMOUR_TYPE])
        .last()
        .map(|(_, _, k)| (k.values[ARMOUR_LINEAR], k.values[ARMOUR_SQUARE]))
}
