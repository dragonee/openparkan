//! Putting up a building: the site test a placement asks (`IsPlacementValid`), the build
//! order a builder walks to its site with, the building it makes there, and the construction
//! sphere the new building runs for 41 s. See `docs/32-builder.md`, "Placing a building",
//! "Building a building, tick by tick" and "The construction sphere".

use std::collections::HashMap;

use glam::Vec3;
use parkan_formats::controls::BuildScheme;
use parkan_formats::mission::{self, KIND_BUILDING, Mission, Value};
use parkan_formats::{basement, control, gamedir};
use parkan_sim::behaviour::{BuildState, Task};
use parkan_sim::combat::Event;
use parkan_sim::effects::Frame;
use parkan_sim::orders::{self, Order, Target};
use parkan_sim::solid::Solid;

use crate::building_fx::BuildingEffects;
use crate::buildings::Building;
use crate::factory::Factory;
use crate::fx::Owner;
use crate::play::{Play, SPAWNED_OBJECTS, Unit};
use crate::robot::Robot;

/// A mine's Type, which must stand on a lode (docs/32, "A mine must stand on a lode").
pub const BUILDING_MINE: u32 = 0x8000_0004;
/// The slope limit a basement face's normal z must reach: with a builder named, and without
/// (`Behavior.dll:0x1000ba63`, `0x1000ba6d`).
pub const SLOPE_WITH_BUILDER: f32 = 0.88;
pub const SLOPE_WITHOUT_BUILDER: f32 = 0.8;
/// The outer contour's reach + 5 (`Terrain.dll:0x1005b680`), and the 5 more the query adds
/// (`0x1000bdd9`).
pub const CONTOUR_MARGIN: f32 = 5.0;
pub const QUERY_MARGIN: f32 = 5.0;
/// A found lode strictly within this across the ground (`iron3d.dll:0x10072f07`).
pub const LODE_REACH: f32 = 20.0;
/// A mine's construction sphere is 15 wider (`Terrain.dll:0x1005c50c`).
pub const MINE_SPHERE_EXTRA: f32 = 15.0;
/// The clearing: within the sphere's radius + 15 as a phase starts the task, + 20 on a phase
/// change (`Behavior.dll:0x10031680`).
pub const CLEAR_ON_START: f32 = 15.0;
pub const CLEAR_ON_CHANGE: f32 = 20.0;
/// A kill state takes itself again every 250 ms (docs/32, "The kill repeats").
pub const KILL_STEP_MS: f64 = 250.0;
/// The construction sphere's effect ids: the sign, the ray and the dome (docs/32, "What the
/// building's controller does with the codes").
pub const SIGN: i32 = 9002;
pub const RAY: i32 = 9001;
pub const DOME: i32 = 9100;
/// The time mode the sphere's effects run in: looping.
pub const SPHERE_TIME_MODE: u32 = 2;
/// The construction sphere's action, filed in a building's load group.
pub const ACT_SPHERE_EFFECT: i32 = 5;
/// The takt after a builder arrives: `now − +0x128 ≥ 1000 ms`, `+0x128` only ever zero
/// (`0x10028e89`), so at once.
pub const BUILD_WAIT_MS: f64 = 1000.0;

/// One phase of a construction sphere: the code handed to the building's controller, how
/// long it lasts, whether it clears the area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Phase {
    pub code: Option<i32>,
    pub seconds: f64,
    pub clear: bool,
}

/// Order 18 with parameter 0, a new building's sphere (`0x10031150`): the sign for 5 s, 30 s
/// clearing the area, the dome, ray and kill for 5 s, the ray stopped for 1 s.
pub const NEW_BUILDING: [Phase; 5] = [
    Phase { code: Some(1), seconds: 5.0, clear: false },
    Phase { code: None, seconds: 25.0, clear: true },
    Phase { code: None, seconds: 5.0, clear: true },
    Phase { code: Some(2), seconds: 5.0, clear: false },
    Phase { code: Some(0), seconds: 1.0, clear: false },
];

/// A new building's construction sphere running.
#[derive(Clone, Debug, PartialEq)]
pub struct Sphere {
    pub target: usize,
    pub centre: Vec3,
    pub radius: f32,
    pub phase: usize,
    pub phase_ms: f64,
    pub next_kill_ms: f64,
    /// The load group's sphere effects by id: its name.
    pub effects: Vec<(i32, String)>,
}

/// A building's ground plan as its `.bas` gives it: the inner ring and the outer contour, in
/// the model's frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub inner: Vec<[f32; 3]>,
    pub outer: Vec<[f32; 3]>,
}

impl Plan {
    /// The outer contour's sphere (`IBuilding` slot 16): the middle of its box, and the
    /// farthest point from there + 5; in the model's frame across the ground.
    pub fn contour_sphere(&self) -> Option<([f32; 2], f32)> {
        let ring = if self.outer.is_empty() { &self.inner } else { &self.outer };
        let lo = ring.iter().fold([f32::MAX; 2], |m, p| [m[0].min(p[0]), m[1].min(p[1])]);
        let hi = ring.iter().fold([f32::MIN; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
        if lo[0] > hi[0] {
            return None;
        }
        let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let reach = ring.iter().map(|p| (p[0] - mid[0]).hypot(p[1] - mid[1])).fold(0.0, f32::max);
        Some((mid, reach + CONTOUR_MARGIN))
    }
}

/// A point of the model's frame placed at `at` turned `yaw` about z, across the ground.
fn placed(p: [f32; 2], at: Vec3, yaw: f32) -> [f32; 2] {
    let (s, c) = yaw.sin_cos();
    [at.x + p[0] * c - p[1] * s, at.y + p[0] * s + p[1] * c]
}

/// A lode: where it lies, its amount, and whether it is found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lode {
    pub position: Vec3,
    pub amount: f32,
}

/// What the play keeps for building: the schemes, each Type's ground plan and ore cost, the
/// placements of the buildings standing, and the spheres running.
#[derive(Clone, Debug, Default)]
pub struct Construction {
    pub schemes: Option<Vec<BuildScheme>>,
    pub plans: HashMap<String, Plan>,
    /// Each building target's placement: where it stands and its turn.
    pub placements: HashMap<usize, (Vec3, f32)>,
    pub spheres: Vec<Sphere>,
    pub lodes: Vec<Lode>,
    /// The ore a building of each Type costs by the player's tree (docs/32, "What it costs").
    pub costs: HashMap<u32, Option<f32>>,
}

impl Construction {
    /// The mission's buildings' placements, `objects` being the mission object each target is,
    /// and its lodes.
    pub fn new(mission: &Mission, objects: &[usize]) -> Construction {
        let placements = objects
            .iter()
            .enumerate()
            .filter_map(|(t, &o)| {
                let object = mission.objects.get(o).filter(|o| o.kind == KIND_BUILDING)?;
                Some((t, (Vec3::from_array(object.position), object.rotation)))
            })
            .collect();
        let lodes = mission
            .viewpoints
            .iter()
            .map(|v| Lode { position: Vec3::from_array(v.position), amount: f32::from_bits(v.unknown[2]) })
            .collect();
        Construction { placements, lodes, ..Construction::default() }
    }
}

impl Play {
    /// `BuildDat.lst`'s schemes, read once.
    fn schemes(&mut self) -> &[BuildScheme] {
        if self.construction.schemes.is_none() {
            let schemes = gamedir::resolve(&self.assembly.game, parkan_formats::controls::BUILD_SCHEMES)
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| {
                    parkan_formats::controls::build_schemes(&b, parkan_formats::controls::BUILD_SCHEMES).ok()
                })
                .unwrap_or_default();
            self.construction.schemes = Some(schemes);
        }
        self.construction.schemes.as_deref().unwrap_or_default()
    }

    /// The `.dat` a builder puts up for `type_word`: its scheme's first, whose root part's
    /// `FORT` record is the placement's model (docs/32, "The model under the cursor").
    pub fn placement_model(&mut self, type_word: u32) -> Option<String> {
        self.schemes()
            .iter()
            .find(|s| s.type_word() == Some(type_word))
            .and_then(|s| s.members.first())
            .cloned()
    }

    /// The ground plan of the building at `path`: its root record's `.bas`.
    fn plan(&mut self, path: &str) -> Plan {
        let key = path.to_ascii_lowercase();
        if let Some(p) = self.construction.plans.get(&key) {
            return p.clone();
        }
        let parts = self.assembly.parts(KIND_BUILDING, path);
        let rings = parts
            .iter()
            .find(|p| p.host == -1)
            .and_then(|root| {
                self.assembly.library.record_slot(self.assembly.library.get(&root.record), "bas", 0)
            })
            .and_then(|slot| {
                let data = self.assembly.archive(&slot.library)?.read_name(&slot.member).ok()?.to_vec();
                basement::parse(&data, &slot.member).ok()
            })
            .unwrap_or_default();
        let plan = Plan {
            inner: rings.first().map(|r| r.points.clone()).unwrap_or_default(),
            outer: rings.get(1).map(|r| r.points.clone()).unwrap_or_default(),
        };
        self.construction.plans.insert(key, plan.clone());
        plan
    }

    /// Target `t`'s contour sphere across the ground, where it stands.
    fn building_sphere(&mut self, t: usize) -> Option<(Vec3, f32)> {
        let (at, yaw) = *self.construction.placements.get(&t)?;
        let path = self.commander.paths.get(t)?.clone();
        let (mid, r) = self.plan(&path).contour_sphere()?;
        let [x, y] = placed(mid, at, yaw);
        Some((Vec3::new(x, y, at.z), r))
    }

    /// A found lode strictly within 20 of `at` across the ground (`0x10072f00`).
    pub fn on_lode(&self, at: Vec3) -> bool {
        self.commander
            .lodes
            .iter()
            .any(|l| l.found && l.position.truncate().distance(at.truncate()) < LODE_REACH)
    }

    /// `IsPlacementValid` (`Behavior.dll:0x1000b9e0`) for a building of `type_word` at `at`
    /// turned `yaw`, with the builder named or not, and for a mine a found lode within 20
    /// (docs/32, "The test" and "A mine must stand on a lode").
    ///
    /// STAND-IN: docs/32-builder.md#the-test-isplacementvalid--read -- the path search from
    /// the builder (`0x10020910`) and the hall-way vertices' areal test (step 6) are not
    /// modelled: every site has a path and usable areals. How `StartCheckMaxBasementAngle`
    /// triangulates the basement between its rings is not read: each corner of either ring
    /// against the nearest corner of the other stands for a face, falling along that line.
    pub fn placement_valid(&mut self, builder: Option<usize>, type_word: u32, at: Vec3, yaw: f32) -> bool {
        if type_word == BUILDING_MINE && !self.on_lode(at) {
            return false;
        }
        let Some(path) = self.placement_model(type_word) else { return false };
        let plan = self.plan(&path);
        let Some((mid, reach)) = plan.contour_sphere() else { return false };
        let radius = reach + QUERY_MARGIN;
        let [cx, cy] = placed(mid, at, yaw);
        // The map: the sphere strictly inside the world box across the ground.
        let (lo, hi) = self.ground.bounds();
        if !(cx - radius > lo[0] && cx + radius < hi[0] && cy - radius > lo[1] && cy + radius < hi[1]) {
            return false;
        }
        // Other buildings: the two spheres' centres at least both radii apart.
        let others: Vec<usize> = self.construction.placements.keys().copied().collect();
        for t in others {
            if !self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
                || self.deleted.get(t) == Some(&true)
            {
                continue;
            }
            if let Some((c, r)) = self.building_sphere(t)
                && c.truncate().distance(Vec3::new(cx, cy, 0.0).truncate()) < radius + r
            {
                return false;
            }
        }
        // The basement: no face steeper than the limit.
        let limit = if builder.is_some() { SLOPE_WITH_BUILDER } else { SLOPE_WITHOUT_BUILDER };
        let height = |p: [f32; 2]| self.ground.below(p[0], p[1], 1.0e5).map(|h| h.point.z);
        let outer: Vec<Vec3> = plan
            .outer
            .iter()
            .filter_map(|p| {
                let q = placed([p[0], p[1]], at, yaw);
                Some(Vec3::new(q[0], q[1], height(q)?))
            })
            .collect();
        if outer.len() < plan.outer.len() || outer.is_empty() {
            return false;
        }
        let mean = outer.iter().map(|p| p.z).sum::<f32>() / outer.len() as f32;
        let inner: Vec<Vec3> = plan
            .inner
            .iter()
            .map(|p| {
                let q = placed([p[0], p[1]], at, yaw);
                Vec3::new(q[0], q[1], mean)
            })
            .collect();
        basement_steepest(&inner, &outer) >= limit
    }

    /// Give `builder` the build order a placement's click gives (docs/32, "The click"):
    /// `ORDER_ROBOT_BUILD` with the Type, the placement at `at` turned `yaw`, replacing its
    /// orders. False when it is no robot, or its task refuses (an unresearched building).
    pub fn order_build(&mut self, builder: usize, type_word: u32, at: Vec3, yaw: f32) -> bool {
        if self.building_ore(type_word).is_none() {
            return false;
        }
        let order = Order {
            code: orders::BUILD,
            parameter: type_word as i32,
            target: Target::Placement([at.x, at.y, at.z, yaw]),
        };
        let Some((_, robot)) = self.robots.iter_mut().find(|(t, _)| *t == builder) else { return false };
        if !robot.behaviour.insert_order(&order, orders::INSERT_REPLACE) {
            return false;
        }
        robot.order = Some(order);
        true
    }

    /// A building's ore cost: the sum of its first `.dat`'s parts' build ore in the player's
    /// tree; `None` when a part is not researched or the file does not read ("Cannot build",
    /// `0x10028800`).
    pub fn building_ore(&mut self, type_word: u32) -> Option<f32> {
        if let Some(cost) = self.construction.costs.get(&type_word) {
            return *cost;
        }
        let cost = (|| {
            let path = self.placement_model(type_word)?;
            let unit = self.assembly.unit(&path)?;
            let tree =
                usize::try_from(self.player_clan).ok().and_then(|c| self.clans.get(c))?.behaviour.clone();
            let catalogue = crate::designs::Catalogue::open(&self.assembly.game, &tree).ok()?;
            let mut ore = 0.0;
            for c in &unit.components {
                let item = catalogue.item(&c.reference.member).filter(|i| i.in_tree() && i.researched())?;
                ore += item.build_cost().1;
            }
            Some(ore)
        })();
        self.construction.costs.insert(type_word, cost);
        cost
    }

    /// Whether building `t` is building itself: its construction sphere is running (property
    /// `0x20c`, `Behavior.dll:0x1000a82c`).
    pub fn building_itself(&self, t: usize) -> bool {
        self.construction.spheres.iter().any(|s| s.target == t)
    }

    /// Every builder at its site makes its building (`0x10029240`), and every construction
    /// sphere steps: its phases' effects, the clearing and the kill. The events of the units
    /// the sphere kills.
    pub fn tick_construction(&mut self, now: f64) -> Vec<Event> {
        let arrived: Vec<(usize, u32, [f32; 4])> = self
            .robots
            .iter()
            .filter(|(t, _)| self.battle.combat.targets.get(*t).is_some_and(|x| x.alive))
            .filter_map(|(t, r)| match r.behaviour.task() {
                Task::Build { type_word, site, state: BuildState::Arrived } => Some((*t, type_word, site)),
                _ => None,
            })
            .collect();
        for (t, type_word, site) in arrived {
            let clan = self.units[t].clan.unwrap_or(-1);
            let at = Vec3::new(site[0], site[1], site[2]);
            if self.create_building(clan, type_word, at, site[3], now).is_none() {
                eprintln!("CreateBuilding() failed...");
            }
            // Either way the cost comes off the builder's ore and the task is over ("BuildTask
            // is over"): the builder is left with no order.
            let cost = self.building_ore(type_word).unwrap_or(0.0);
            self.economy.take_ore(t, cost);
            if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == t) {
                robot.behaviour.tasks = vec![Task::Stop];
                robot.order = None;
            }
        }
        self.step_spheres(now)
    }

    /// Make a building of `type_word` for `clan` at `at` turned `yaw`
    /// (`CreateObjectFromScheme`, `Behavior.dll:0x1001d440`), in build mode: a new target with
    /// its unit, name, ground, doors and pod, factory and load group, on its clan's list, and
    /// its construction sphere started. Refused when its sphere meets another building's.
    pub fn create_building(
        &mut self,
        clan: i64,
        type_word: u32,
        at: Vec3,
        yaw: f32,
        now: f64,
    ) -> Option<usize> {
        let path = self.placement_model(type_word)?;
        let plan = self.plan(&path);
        if let Some((mid, reach)) = plan.contour_sphere() {
            let [cx, cy] = placed(mid, at, yaw);
            let others: Vec<usize> = self.construction.placements.keys().copied().collect();
            for t in others {
                if self.battle.combat.targets.get(t).is_some_and(|x| x.alive)
                    && let Some((c, r)) = self.building_sphere(t)
                    && c.truncate().distance(Vec3::new(cx, cy, 0.0).truncate()) < reach + r
                {
                    return None;
                }
            }
        }
        let low = self.units.iter().filter(|u| u.logical_id < 0).map(|u| u.logical_id & 0x7fff_ffff).max();
        let logical_id = (0x8000_0000_u32 | (low.unwrap_or(0) as u32 + 1)) as i32;
        let property = |name: &str, value: Value| mission::Property {
            name: name.to_owned(),
            kind: 0,
            value,
            minimum: value,
            maximum: value,
        };
        let object = mission::Object {
            kind: KIND_BUILDING,
            path: path.clone(),
            unknown_q: 0,
            logical_id,
            position: at.to_array(),
            pad: [0; 2],
            rotation: yaw,
            scale: [1.0; 3],
            name: String::new(),
            tail: (0, 0, 0, 0),
            properties: vec![
                property("ClanID", Value::Int(clan as i32)),
                property("Type", Value::Int(type_word as i32)),
            ],
        };
        let index = SPAWNED_OBJECTS + self.spawned;
        let t = self.battle.add(
            &mut self.assembly,
            &self.clans,
            &object,
            index,
            Some(self.player_clan),
            self.ratio,
        )?;
        let one = Mission {
            version: 0,
            routes: Vec::new(),
            clans: self.clans.clone(),
            unknown_pre_objects: 0,
            objects: vec![object.clone()],
            map_path: String::new(),
            description: String::new(),
            viewpoints: Vec::new(),
        };
        if let Ok(Some(mut robot)) = Robot::load_placed(&mut self.assembly, &object, index) {
            robot.arm(&mut self.battle, &mut self.assembly);
            if !robot.guns.is_empty() {
                self.emplacements.push((t, robot));
            }
        }
        self.units.push(Unit {
            clan: Some(clan),
            type_word,
            logical_id,
            kind: KIND_BUILDING,
            announced: false,
            designation: Default::default(),
        });
        self.commander.paths.push(path.clone());
        let stem = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned();
        self.names.push(stem);
        self.deleted.push(false);
        let target = &self.battle.combat.targets[t];
        self.ground.solids.push(Solid::from_parts(
            &target.parts,
            target.centre,
            target.radius,
            true,
            |_, _| None,
        ));
        self.rebuild_solid(t);
        // STAND-IN: docs/03-terrain.md#for-an-engine -- how a building made in play is drawn is
        // not followed: it is drawn node by node from level 0 without a lightmap, and only the
        // ground queries, not the landscape's drawing, are cut under it.
        if let Some(inner) = crate::terrain::building_cuts(&mut self.assembly, &one).into_iter().next() {
            self.ground.cuts.push(parkan_sim::ground::Cut::new(inner));
        }
        if let Some(mut b) = Building::load(&mut self.assembly, &one, 0, t) {
            if let Some(part) = self.battle.combat.targets.get(t).and_then(|x| x.parts.get(b.part)) {
                b.place_zone(part);
            }
            self.buildings.push(b);
        }
        if let Some(f) = Factory::load(&mut self.assembly, &one, 0, t) {
            self.factories.push(f);
        }
        let sphere_effects = sphere_effects(&mut self.assembly, &path);
        if let Some(effects) = BuildingEffects::load(&mut self.assembly, &one, 0, t) {
            if let Some(part) = self.battle.combat.targets.get(t).and_then(|x| x.parts.get(effects.part)) {
                for e in &effects.effects {
                    if let Some(frame) = effects.frame(part, e.on) {
                        self.fx.start(Owner::Building(t, e.id), &e.name, frame, 1.0, now, None);
                    }
                }
                for &(id, mode) in &effects.starts {
                    self.fx.restart(Owner::Building(t, id), now, Some(mode));
                }
            }
            let n = effects.effects.len();
            self.building_effects.push((effects, vec![0.0; n]));
        }
        for (_, name) in &sphere_effects {
            self.fx.template(name);
        }
        self.construction.placements.insert(t, (at, yaw));
        if let Some(p) = self.progression.as_mut() {
            p.progress.place_building(logical_id, clan, type_word);
        }
        // The sphere, round the outer contour, 15 wider on a mine.
        let (centre, radius) = self.building_sphere(t).unwrap_or((at, self.battle.combat.targets[t].radius));
        let radius = radius + if type_word == BUILDING_MINE { MINE_SPHERE_EXTRA } else { 0.0 };
        self.construction.spheres.push(Sphere {
            target: t,
            centre,
            radius,
            phase: 0,
            phase_ms: now,
            next_kill_ms: now,
            effects: sphere_effects,
        });
        self.start_phase(self.construction.spheres.len() - 1, now, true);
        self.spawned += 1;
        self.added.push(t);
        Some(t)
    }

    /// A sphere's phase starting: its code's effects, and the clearing.
    ///
    /// STAND-IN: docs/32-builder.md#the-construction-sphere--read-and-measured -- the
    /// controller's path between the states the codes open is not read, nor where an action-5
    /// effect is placed: code 1 starts the sign, code 2 the dome and the ray and stops the
    /// sign, code 0 stops the ray, each effect at the sphere's centre sized by its radius.
    ///
    /// STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the three effects' records give
    /// time mode 0, a value set from outside (slot `0x1c`), and what sets it is not read: they
    /// loop on their own durations (mode 2) while their phase runs.
    fn start_phase(&mut self, s: usize, now: f64, first: bool) {
        let sphere = self.construction.spheres[s].clone();
        let Some(phase) = NEW_BUILDING.get(sphere.phase) else { return };
        let frame = Frame::along(sphere.centre, Vec3::X, 1.0);
        let start = |play: &mut Play, id: i32| {
            if let Some((_, name)) = sphere.effects.iter().find(|(i, _)| *i == id) {
                play.fx.remove(Owner::Building(sphere.target, id));
                play.fx.start(
                    Owner::Building(sphere.target, id),
                    name,
                    frame,
                    sphere.radius,
                    now,
                    Some(SPHERE_TIME_MODE),
                );
            }
        };
        match phase.code {
            Some(1) => start(self, SIGN),
            Some(2) => {
                start(self, DOME);
                start(self, RAY);
                self.fx.remove(Owner::Building(sphere.target, SIGN));
            }
            Some(0) => self.fx.remove(Owner::Building(sphere.target, RAY)),
            _ => {}
        }
        if phase.clear {
            let reach = sphere.radius + if first { CLEAR_ON_START } else { CLEAR_ON_CHANGE };
            let leave = Order { code: orders::LEAVE, parameter: 0, target: Target::NotDefined };
            for (t, robot) in &mut self.robots {
                let Some(target) = self.battle.combat.targets.get(*t).filter(|x| x.alive) else { continue };
                if target.position.truncate().distance(sphere.centre.truncate()) > reach
                    || matches!(robot.behaviour.task(), Task::Leave { .. })
                {
                    continue;
                }
                robot.order = Some(leave);
                robot.behaviour.order(&leave);
            }
        }
    }

    /// Every construction sphere's step at `now` (`0x10031680`): the next phase once one runs
    /// out, the kill every 250 ms while code 2 holds, and the task's end.
    fn step_spheres(&mut self, now: f64) -> Vec<Event> {
        let mut events = Vec::new();
        let mut s = 0;
        while s < self.construction.spheres.len() {
            let sphere = &mut self.construction.spheres[s];
            let length = NEW_BUILDING.get(sphere.phase).map_or(0.0, |p| p.seconds * 1000.0);
            if now - sphere.phase_ms >= length {
                sphere.phase += 1;
                sphere.phase_ms += length;
                sphere.next_kill_ms = now;
                if sphere.phase >= NEW_BUILDING.len() {
                    let done = self.construction.spheres.remove(s);
                    for id in [SIGN, RAY, DOME] {
                        self.fx.remove(Owner::Building(done.target, id));
                    }
                    // A mine's order 10, queued behind the sphere, digs now (docs/23).
                    if let Some(site) = self.economy.site_mut(done.target)
                        && site.mine.is_some()
                    {
                        site.digging = true;
                    }
                    continue;
                }
                self.start_phase(s, now, false);
            }
            let sphere = self.construction.spheres[s].clone();
            if NEW_BUILDING[sphere.phase].code == Some(2) && now >= sphere.next_kill_ms {
                self.construction.spheres[s].next_kill_ms = now + KILL_STEP_MS;
                events.extend(self.kill_inside(sphere.target, sphere.centre, sphere.radius));
            }
            s += 1;
        }
        events
    }

    /// The kill (action 21, `Control.dll:0x100033e6`): every unit inside the sphere loses its
    /// life.
    ///
    /// STAND-IN: docs/32-builder.md#the-construction-sphere--read-and-measured -- that the
    /// classes the kill takes (`0x4`, `0x10`, `0x400`) are the units is a guess: every live
    /// robot whose position lies inside, the hero among them.
    fn kill_inside(&mut self, building: usize, centre: Vec3, radius: f32) -> Vec<Event> {
        let mut events = Vec::new();
        let inside: Vec<usize> = self
            .robots
            .iter()
            .map(|(t, _)| *t)
            .filter(|&t| t != building)
            .filter(|&t| {
                self.battle
                    .combat
                    .targets
                    .get(t)
                    .is_some_and(|x| x.alive && x.position.distance(centre) <= radius)
            })
            .collect();
        for t in inside {
            events.extend(self.battle.combat.ground_loss(t, f32::MAX / 4.0));
        }
        if !self.hero.dead() && !self.hero_away() && self.hero.walker.body.position.distance(centre) <= radius
        {
            let mut lives: Vec<&mut parkan_sim::damage::Life> =
                self.hero.lives.iter_mut().flatten().collect();
            parkan_sim::damage::share_loss(&mut lives, f32::MAX / 4.0);
        }
        events
    }
}

/// The steepest basement face's normal z between `inner` and `outer`, 1 with no face: each
/// corner of either ring against the nearest corner of the other, the face between them
/// taken to fall along that line.
fn basement_steepest(inner: &[Vec3], outer: &[Vec3]) -> f32 {
    let mut steepest: f32 = 1.0;
    for (ring, other) in [(outer, inner), (inner, outer)] {
        for &a in ring {
            let Some(&b) = other.iter().min_by(|p, q| {
                p.truncate().distance(a.truncate()).total_cmp(&q.truncate().distance(a.truncate()))
            }) else {
                continue;
            };
            let (across, up) = (a.truncate().distance(b.truncate()), (a.z - b.z).abs());
            let length = across.hypot(up);
            if length > 1e-6 {
                steepest = steepest.min(across / length);
            }
        }
    }
    steepest
}

/// A building's load-group construction-sphere effects (action 5): each id and its name.
fn sphere_effects(assembly: &mut crate::assembly::Assembly, path: &str) -> Vec<(i32, String)> {
    let parts = assembly.parts(KIND_BUILDING, path);
    let Some(root) = parts.iter().find(|p| p.host == -1) else { return Vec::new() };
    let Some(slot) = assembly.library.record_slot(assembly.library.get(&root.record), "ctl", 0) else {
        return Vec::new();
    };
    let Some(controller) = assembly
        .archive(&slot.library)
        .and_then(|a| a.read_name(&slot.member).ok())
        .and_then(|data| control::parse(data, &slot.member).ok())
    else {
        return Vec::new();
    };
    controller
        .group(control::ENTRY_LOAD)
        .iter()
        .filter(|r| r.action() == ACT_SPHERE_EFFECT && !r.resource.member.is_empty())
        .map(|r| (r.values[7], r.resource.member.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_contour_sphere_is_its_box_middle_and_farthest_corner_and_5() {
        let plan = Plan {
            inner: vec![[0.0, 0.0, 0.0]],
            outer: vec![[-10.0, -4.0, 0.0], [10.0, -4.0, 0.0], [10.0, 4.0, 0.0], [-10.0, 4.0, 0.0]],
        };
        let (mid, r) = plan.contour_sphere().unwrap();
        assert_eq!(mid, [0.0, 0.0]);
        assert!((r - (116.0_f32.sqrt() + 5.0)).abs() < 1e-4);
        assert_eq!(placed([1.0, 0.0], Vec3::new(5.0, 5.0, 0.0), std::f32::consts::FRAC_PI_2), [5.0, 6.0]);
    }

    #[test]
    fn a_level_basement_passes_and_a_cliff_fails() {
        let square = |r: f32, z: f32| {
            vec![Vec3::new(-r, -r, z), Vec3::new(r, -r, z), Vec3::new(r, r, z), Vec3::new(-r, r, z)]
        };
        assert!((basement_steepest(&square(5.0, 0.0), &square(8.0, 0.0)) - 1.0).abs() < 1e-6);
        // Three metres down over three across: 45°, steeper than 28.4°.
        assert!(basement_steepest(&square(5.0, 0.0), &square(8.0, -3.0)) < SLOPE_WITH_BUILDER);
    }

    #[test]
    fn a_new_building_runs_its_sphere_41_seconds() {
        let total: f64 = NEW_BUILDING.iter().map(|p| p.seconds).sum();
        assert_eq!(total, 41.0);
        assert_eq!(NEW_BUILDING.iter().filter(|p| p.clear).map(|p| p.seconds).sum::<f64>(), 30.0);
    }
}
