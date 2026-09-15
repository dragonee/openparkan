//! A clan's ore and power: what each unit and building holds, a mine's digging, the
//! distribution step that shares ore and power out, and the rows the commander reads. See
//! `docs/23-economy.md`, "Mission 03's economy, tick by tick".

use std::collections::HashMap;

use parkan_formats::mission::{Mission, Value};

/// The mission properties a unit's ore is placed with.
pub const CURRENT_ORE: &str = "CurrentOre";
pub const MAXIMUM_ORE: &str = "MaximumOre";

/// What the economy keeps.
#[derive(Clone, Debug, Default)]
pub struct Economy {
    /// Each unit's and building's ore held and its most, by target: those the mission gives a
    /// maximum, and every building that joins.
    pub ore: HashMap<usize, (f32, f32)>,
}

fn float(v: Value) -> f32 {
    match v {
        Value::Int(i) => i as f32,
        Value::Float(f) => f,
    }
}

impl Economy {
    /// Every placed object's `CurrentOre` of its `MaximumOre`, `objects` being the mission
    /// object each target is.
    pub fn new(mission: &Mission, objects: &[usize]) -> Economy {
        let ore = objects
            .iter()
            .enumerate()
            .filter_map(|(t, &o)| {
                let object = mission.objects.get(o)?;
                let most = object.property(MAXIMUM_ORE).map_or(0.0, |p| float(p.value));
                let held = object.property(CURRENT_ORE).map_or(0.0, |p| float(p.value));
                (most > 0.0 || held != 0.0).then_some((t, (held, most)))
            })
            .collect();
        Economy { ore }
    }

    /// The ore target `t` holds.
    pub fn held(&self, t: usize) -> f32 {
        self.ore.get(&t).map_or(0.0, |o| o.0)
    }

    /// Take `amount` off what target `t` holds, going below zero as the property setter lets
    /// it (`Behavior.dll:0x100092c0`, docs/32, "Building a building").
    pub fn take_ore(&mut self, t: usize, amount: f32) {
        self.ore.entry(t).or_insert((0.0, 0.0)).0 -= amount;
    }

    /// A building joins as target `t`, holding nothing.
    pub fn join_building(&mut self, t: usize) {
        self.ore.entry(t).or_insert((0.0, 0.0));
    }
}
