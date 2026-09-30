//! Putting up a building: the site test a placement asks (`IsPlacementValid`), the build
//! order a builder walks to its site with, the building it makes there, and the construction
//! sphere the new building runs for 41 s. See `docs/32-builder.md`, "Placing a building",
//! "Building a building, tick by tick" and "The construction sphere".

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use glam::Vec3;
use parkan_formats::controls::BuildScheme;
use parkan_formats::mission::{self, KIND_BUILDING, Mission, Value};
use parkan_formats::{basement, control, gamedir, hallway};
use parkan_sim::behaviour::{BuildState, Task, UpgradeState};
use parkan_sim::combat::Event;
use parkan_sim::economy::{POWER_TICK_MS, ShiftJitter};
use parkan_sim::effects::Frame;
use parkan_sim::orders::{self, Order, Target};
use parkan_sim::solid::Solid;

use crate::building_fx::BuildingEffects;
use crate::buildings::Building;
use crate::capture::PLACE_EXIT;
use crate::factory::Factory;
use crate::fx::Owner;
use crate::places::Places;
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
/// The construction sphere's effect ids: the sign, the ray and the dome (docs/32, "What the
/// building's controller does with the codes").
pub const SIGN: i32 = 9002;
pub const RAY: i32 = 9001;
pub const DOME: i32 = 9100;
/// The modes action 10 starts the sphere's effects in (docs/11, "Effect time"): the sign
/// loops, the ray and the dome play once through, and the dome plays back out.
pub const TIME_ONCE: u32 = 1;
pub const TIME_LOOP: u32 = 2;
pub const TIME_REVERSE: u32 = 3;
/// The construction sphere's action, filed in a building's load group.
pub const ACT_SPHERE_EFFECT: i32 = 5;
/// The actions a building's construction states run besides the effects' (docs/13, "The
/// section-5 record"): hide the building and show it (`IAnimation` slot 8 setting and clearing
/// flag 1 on every node, `Control.dll:0x10002936`, `0x10002954`), place it in the landscape
/// (`0x1000352e`) and kill in its sphere (`0x100033e6`).
pub const ACT_HIDE: i32 = 1;
pub const ACT_SHOW: i32 = 2;
pub const ACT_PLACE: i32 = 20;
pub const ACT_KILL_IN_SPHERE: i32 = 21;
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

/// Order 18 with parameter 0, a new building's sphere (`0x10031150`, the 16-byte records
/// "Construct array set" writes: code, flags, clear, seconds): code 1 for 5 s, 30 s with no
/// code sent (−1), clearing the area, then 2 for 5 s and 0 for 1. A phase with no code leaves
/// the controller on the last one, so the sign shows for all of the first 35 s.
pub const NEW_BUILDING: [Phase; 5] = [
    Phase { code: Some(1), seconds: 5.0, clear: false },
    Phase { code: None, seconds: 25.0, clear: true },
    Phase { code: None, seconds: 5.0, clear: true },
    Phase { code: Some(2), seconds: 5.0, clear: false },
    Phase { code: Some(0), seconds: 1.0, clear: false },
];

/// Order 18 with parameter 1, the building an upgrade is taking (`0x100335a1`, "CloseSphere
/// array set"): `0x309` for 25 s, clearing the area, a second with no code, then 8 for 90 s.
/// `0x309` is kept as property `0x205` and never reaches the controller. The swap comes 50 s
/// in, part way through the last phase, and the building is gone before it ends.
pub const UPGRADING: [Phase; 3] = [
    Phase { code: Some(0x309), seconds: 25.0, clear: true },
    Phase { code: None, seconds: 1.0, clear: false },
    Phase { code: Some(8), seconds: 90.0, clear: false },
];
/// The code the task keeps from the controller (`Behavior.dll:0x10031307`).
pub const CODE_UPGRADING: i32 = 0x309;
/// Order 18 with parameter 2, the building an upgrade made (`0x10033790`): 10 for 3 s, then 0
/// for 1. The builder waits while it runs.
pub const UPGRADED: [Phase; 2] = [
    Phase { code: Some(10), seconds: 3.0, clear: false },
    Phase { code: Some(0), seconds: 1.0, clear: false },
];
/// How long after the builder arrives the old building is removed and the next one made
/// (`0x1003363f`).
pub const UPGRADE_SWAP_MS: f64 = 50_000.0;

/// A building being walked one step up its scheme (docs/32, "Upgrading a building"): the
/// builder waiting on it, the building, when its sphere began, and the building the swap made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Upgrading {
    pub builder: usize,
    pub building: usize,
    pub since_ms: f64,
    pub made: Option<usize>,
}

/// A new building's construction sphere running.
#[derive(Clone, Debug, PartialEq)]
pub struct Sphere {
    pub target: usize,
    pub centre: Vec3,
    pub radius: f32,
    /// The phases the order's parameter picked (`0x10031150`).
    pub phases: &'static [Phase],
    pub phase: usize,
    pub phase_ms: f64,
    /// The load group's sphere effects by id: its name.
    pub effects: Vec<(i32, String)>,
}

/// A building's controller as the construction sphere drives it: the code `IControl` slot 19
/// hands it, and its states played one step at a time on the controller's own clock, each
/// state's action group run as the state is taken off the queue (`Control.dll:0x1000c2a5`–
/// `0x1000c38c`; docs/24, "Playing a state"). Slot 19 only stores the code
/// (`Control.dll:0x10004800`), so a code takes effect at the controller's next step, when the
/// anchor it stands on no longer applies and the planner queues the way to the one that does
/// (docs/32, "What the building's controller does with the codes").
#[derive(Clone, Debug, PartialEq)]
pub struct CodeMachine {
    controller: Rc<control::Controller>,
    /// The transition costs as the loader scales them (`0x10001790`).
    costs: Rc<Vec<f32>>,
    /// The state being played. `None` is the constructor's record, a zeroed copy whose use
    /// count stops it applying, so the first plan runs from index 0 (docs/24, "The state a
    /// machine starts in").
    current: Option<usize>,
    queue: VecDeque<usize>,
    /// `+0xdc`: when the next step is due, in game ms. The load sets it to the time the object
    /// is made (`0x10007aaf`).
    pub clock_ms: f64,
    /// `+0x1ac`: the code the controller holds, 0 from its constructor (`0x10006ecf`).
    pub code: i32,
    /// The construction sphere its kill asks the world about (action 21's `+0x38` slot 12).
    pub sphere: (Vec3, f32),
    /// The step jitter's generator (`0x100057de`). The game's is one pair of words the whole
    /// module shares, seeded from the clock at load (`0x10006330`); each machine here keeps its
    /// own, from a fixed seed, as the walkers do.
    jitter: ShiftJitter,
}

/// A state the machine took: when its step began, and the records its action group ran.
pub type Taken = (f64, Vec<[i32; 9]>);

/// Steps one advance may run before it gives the clock up to the caller's time.
const MAX_CODE_STEPS: usize = 4000;

impl CodeMachine {
    /// The machine the controller's constructor and load leave: on the constructor's record,
    /// holding code 0, its clock at `made_ms`.
    pub fn new(controller: control::Controller, made_ms: f64, sphere: (Vec3, f32)) -> Self {
        let costs = Rc::new(controller.live_costs());
        CodeMachine {
            controller: Rc::new(controller),
            costs,
            current: None,
            queue: VecDeque::new(),
            clock_ms: made_ms,
            code: control::FIRST_REQUEST,
            sphere,
            jitter: ShiftJitter::seeded(0x2545_F491),
        }
    }

    /// The state being played, `None` before the first plan.
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// Whether the state being played is the anchor code `code` opens.
    pub fn on_anchor(&self, code: i32) -> bool {
        self.current.is_some_and(|c| {
            let s = &self.controller.states[c];
            s.anchor() && s.request == code
        })
    }

    /// Every step due by `now`, as the machine tick runs them while its clock is not ahead of
    /// the game's (`0x1000c2a5`, `0x1000c717`): an anchor plans, the next state comes off the
    /// queue and its group runs, and the clock moves on by the state's step. A building's
    /// states carry no condition, so every record of a group runs.
    pub fn advance(&mut self, now: f64) -> Vec<Taken> {
        let mut taken = Vec::new();
        let mut steps = 0;
        while now >= self.clock_ms && steps < MAX_CODE_STEPS {
            if self.current.is_none_or(|c| self.controller.states[c].anchor()) {
                self.plan();
            }
            if let Some(next) = self.queue.pop_front() {
                self.current = Some(next);
            }
            let Some(c) = self.current else { break };
            let state = &self.controller.states[c];
            let group = self.controller.group_at(state.actions);
            let ran =
                control::run_group(&group, &[false; control::CONDITIONS]).iter().map(|r| r.values).collect();
            taken.push((self.clock_ms, ran));
            // `0x10005370`: a fixed step, jittered by up to ±12.5% when the state asks
            // (`0x100057de`: step × 0.25 × (r − 0.5), which is step ÷ 250 × the power tick's
            // own draw), and held to 0.01–5 s (`0x100057d6`).
            let mut step = f64::from(state.length);
            if state.mode & control::STATE_JITTER != 0 {
                step += step * self.jitter.next_ms() / POWER_TICK_MS;
            }
            self.clock_ms += step.clamp(10.0, 5000.0);
            steps += 1;
        }
        if steps == MAX_CODE_STEPS {
            self.clock_ms = now;
        }
        taken
    }

    /// `0x100051c0`: an anchor that still applies queues the cheapest cycle back to itself;
    /// otherwise the way to the cheapest other anchor that applies. A building stands still, so
    /// its states' boxes see no velocity and no spin; every building state's use count is
    /// unlimited (docs/24, "A state's use count and its request code").
    fn plan(&mut self) {
        let states = &self.controller.states;
        let applies = |j: usize| states[j].applies([0.0; 3], [0.0; 3], self.code);
        if let Some(c) = self.current
            && applies(c)
            && let Some((path, _)) = self.controller.path_by(&self.costs, c, c)
        {
            self.queue = path.into();
            return;
        }
        let from = self.current.unwrap_or(0);
        let best = (0..states.len())
            .filter(|&j| j != from && states[j].anchor() && applies(j))
            .filter_map(|j| self.controller.path_by(&self.costs, from, j))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((path, _)) = best {
            self.queue = path.into();
        }
    }
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
pub(crate) fn placed(p: [f32; 2], at: Vec3, yaw: f32) -> [f32; 2] {
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
    /// The upgrades under way (docs/32, "Upgrading a building").
    pub upgrades: Vec<Upgrading>,
    pub lodes: Vec<Lode>,
    /// The ore a building of each Type costs by the player's tree (docs/32, "What it costs").
    pub costs: HashMap<u32, Option<f32>>,
    /// The buildings' hall ways and the units walked into them (docs/31, "The capture, tick by
    /// tick").
    pub ways: crate::capture::Ways,
    /// Each model's hall-way exits in its own frame, `None` for a model with no hall way.
    pub exits: HashMap<String, Option<Vec<Vec3>>>,
    /// The buildings made in play that their controller has not yet placed in the landscape
    /// (action 20), by target: what placing them lets in.
    pub unplaced: HashMap<usize, Unplaced>,
    /// Each building's controller once a construction sphere has driven it, by target: it
    /// goes on stepping after the sphere's task ends, as the code-0 anchor does every 5 s.
    pub machines: HashMap<usize, CodeMachine>,
    /// The buildings their controller has hidden (action 1) and not shown again (action 2).
    pub hidden: HashSet<usize>,
}

/// How a building made in play comes into the world.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Coming {
    /// In build mode, create flag bit 1: the construction sphere's `phases` run on it, and its
    /// controller places it in the landscape when they are done.
    Built(&'static [Phase]),
    /// Finished, as a mission's building stands: its controller places it on its first step,
    /// and its clan's list has it under this logical id already.
    Standing(i32),
}

/// A building made in play, not yet placed in the landscape (action 20,
/// `CLandscape::PlaceBuilding`): not in the world's faces, not struck and not in the way of a
/// sight ray, the landscape not cut under it and its load group's effects not shown. Whether it
/// is drawn is the controller's actions 1 and 2 ([`Construction::hidden`]): a builder's building
/// hides itself as its sign starts and shows itself in the state that places it. *Seen*: in the
/// recording of *The Field Base* the site shows only the sign, then the ray and the dome, until
/// the dome plays back out and the building stands (docs/32, "Mission 03's mine").
#[derive(Clone, Debug, Default)]
pub struct Unplaced {
    /// The cut its footing makes in the landscape.
    pub cut: Option<parkan_sim::ground::Cut>,
    /// Its load group's effects.
    pub effects: Option<BuildingEffects>,
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
    pub(crate) fn schemes(&mut self) -> &[BuildScheme] {
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
    pub(crate) fn plan(&mut self, path: &str) -> Plan {
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

    /// The exits of the hall way of the building at `path` (vertices with flag 1), in the
    /// model's own frame: each posed through its node's rest pose on the root part, as
    /// `MHallWay` slot 5 carries a vertex into the world (`ArealMap.dll:0x1000a760`). `None`
    /// for a model that carries no hall way.
    pub fn hall_exits(&mut self, path: &str) -> Option<Vec<Vec3>> {
        let key = path.to_ascii_lowercase();
        if let Some(e) = self.construction.exits.get(&key) {
            return e.clone();
        }
        let exits = (|| {
            let parts = self.assembly.parts(KIND_BUILDING, path);
            let root = parts.iter().find(|p| p.host == -1)?.clone();
            let blob = self
                .assembly
                .archive(&root.reference.library)?
                .read_name(&root.reference.member)
                .ok()?
                .to_vec();
            let way = hallway::parse(&blob, &root.reference.member).ok()?;
            if way.vertices.is_empty() {
                return None;
            }
            let mesh = self.assembly.mesh(&root.reference)?;
            Some(
                way.vertices
                    .iter()
                    .filter(|v| v.flags & PLACE_EXIT != 0 && (v.joint as usize) < mesh.mesh.nodes.len())
                    .map(|v| {
                        let pose = root.pose.compose(&mesh.mesh.world_pose(v.joint as usize));
                        let p = pose.apply(v.position.map(f64::from));
                        Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
                    })
                    .collect(),
            )
        })();
        self.construction.exits.insert(key, exits.clone());
        exits
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
    /// The basement is the constrained Delaunay triangulation of the ring between the two
    /// `.bas` rings, as `StartCheckMaxBasementAngle` builds it (`0x100150f0`: the outer ring's
    /// edges and the inner's laid in as constraints), the outer corners on the ground and the
    /// inner ones at their mean (docs/32, "The test").
    ///
    /// Steps 3 to 6 run only for a model that answers its hall way (interface `0x303`,
    /// `0x1000baa4`, which the agent files at `AniMesh.dll:0x1000350c`); without one the query
    /// goes straight to the basement (`0x1000c108`). With a builder named that does not fly,
    /// the walker's own search (`0x10020910` on `MBehaviour` `+0x1e0`, as `MWalker::SetTarget`
    /// runs it) must find a way from the builder's place to the sphere's centre, gated by the
    /// builder's size class and crossing no flyer's link (`+0x210` 0, `+0x214` its variable
    /// `0x201`). And every exit of the model's hall way (vertex flag 1), posed into the world
    /// through its node (`MHallWay` slot 4 → slot 5), must stand on an areal of the system map
    /// whose first flag word is set: *"HallVertex … is out of map"* or *"… is in Non-Reachable
    /// Areal"* else (`0x1000bf4a`–`0x1000bfa9`).
    ///
    /// The wrapper's own rule is left out on purpose: within 400 of another clan's base point
    /// a site is red only in a network game, the game's `+0xe4`, which single play never sets
    /// (`iron3d.dll:0x10033d36`, `0x1005c74e`; docs/32, "In a network game a site near another
    /// clan's base is red").
    pub fn placement_valid(&mut self, builder: Option<usize>, type_word: u32, at: Vec3, yaw: f32) -> bool {
        if type_word == BUILDING_MINE && !self.on_lode(at) {
            return false;
        }
        let Some(path) = self.placement_model(type_word) else { return false };
        let plan = self.plan(&path);
        if let Some(exits) = self.hall_exits(&path) {
            let Some((mid, reach)) = plan.contour_sphere() else { return false };
            let radius = reach + QUERY_MARGIN;
            let [cx, cy] = placed(mid, at, yaw);
            // The path: the walker's search from the builder to the sphere's centre.
            if let Some(b) = builder {
                let from = self.battle.combat.targets.get(b).map(|x| x.position);
                if let Some(from) = from
                    && self.walker_search(b, from, Vec3::new(cx, cy, at.z)).is_some_and(|r| r.is_err())
                {
                    return false;
                }
            }
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
            // The exits: each on a walkable areal. A map with no areal map tests none.
            if let Some(graph) = self.graph.as_ref()
                && !exits.iter().all(|e| {
                    let [x, y] = placed([e.x, e.y], at, yaw);
                    graph.usable(x, y)
                })
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
            let catalogue = self.catalogue()?;
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
        // A builder standing beside the building it was sent to upgrade takes it up.
        let beside: Vec<(usize, i32)> = self
            .robots
            .iter()
            .filter(|(t, _)| self.battle.combat.targets.get(*t).is_some_and(|x| x.alive))
            .filter_map(|(t, r)| match r.behaviour.task() {
                Task::Upgrade { building, state: UpgradeState::Arrived, .. } => Some((*t, building)),
                _ => None,
            })
            .collect();
        for (t, id) in beside {
            self.start_upgrade(t, id, now);
        }
        self.step_upgrades(now);
        self.step_spheres(now)
    }

    /// A builder that has arrived takes up its upgrade (`0x100335a1`): the building's own
    /// sphere starts on it and the builder waits beside it. A building gone, taken by another
    /// clan or already at the top of its scheme leaves the builder with no order, as the task's
    /// target test refuses one ("dead, enemy or fully upgraded building").
    ///
    /// STAND-IN: docs/32-builder.md#upgrading-a-building--read -- the builder's invulnerability
    /// (property 162) while it works is modelled against its life system's kill alone, its
    /// building's and *Explode!*'s, which pass it over ([`Play::invulnerable`]): anything else
    /// hurts it as it always does.
    fn start_upgrade(&mut self, builder: usize, id: i32, now: f64) {
        let clan = self.units.get(builder).and_then(|u| u.clan);
        let building = self.units.iter().position(|u| {
            u.logical_id == id && u.kind == KIND_BUILDING && u.clan.is_some() && u.clan == clan
        });
        let building = building
            .filter(|&b| self.battle.combat.targets.get(b).is_some_and(|x| x.alive))
            .filter(|&b| self.upgrade_model(b).is_some())
            .filter(|&b| !self.construction.upgrades.iter().any(|u| u.building == b));
        let Some(building) = building else {
            if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == builder) {
                robot.behaviour.tasks = vec![Task::Stop];
                robot.order = None;
            }
            return;
        };
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == builder)
            && let Some(Task::Upgrade { state, .. }) = robot.behaviour.tasks.last_mut()
        {
            *state = UpgradeState::Working;
        }
        self.construction.upgrades.push(Upgrading { builder, building, since_ms: now, made: None });
        self.start_sphere(building, &UPGRADING, now);
    }

    /// Every upgrade's step: the swap 50 s in (`0x1003363f`), and the builder let go once the
    /// new building's own sphere has stopped. One whose building or builder has gone is dropped.
    fn step_upgrades(&mut self, now: f64) {
        let mut u = 0;
        while u < self.construction.upgrades.len() {
            let up = self.construction.upgrades[u];
            let builder_gone = !self.battle.combat.targets.get(up.builder).is_some_and(|x| x.alive);
            // A builder given another order leaves the building as it stands, its sphere with
            // it, as the task's own end does.
            let left = !self.robots.iter().any(|(t, r)| {
                *t == up.builder
                    && matches!(r.behaviour.task(), Task::Upgrade { state: UpgradeState::Working, .. })
            });
            match up.made {
                None if builder_gone
                    || left
                    || !self.battle.combat.targets.get(up.building).is_some_and(|x| x.alive) =>
                {
                    self.construction.upgrades.remove(u);
                    self.construction.spheres.retain(|s| s.target != up.building);
                    self.end_upgrade(up.builder);
                    continue;
                }
                None if now - up.since_ms >= UPGRADE_SWAP_MS => {
                    let made = self.swap_building(up.building, now);
                    self.construction.upgrades[u].made = Some(made.unwrap_or(up.building));
                    if made.is_none() {
                        self.construction.upgrades.remove(u);
                        self.end_upgrade(up.builder);
                        continue;
                    }
                }
                Some(made) if builder_gone || !self.construction.spheres.iter().any(|s| s.target == made) => {
                    self.construction.upgrades.remove(u);
                    self.end_upgrade(up.builder);
                    continue;
                }
                _ => {}
            }
            u += 1;
        }
    }

    /// The builder is done: it is left with no order, as every task of its own end leaves it.
    fn end_upgrade(&mut self, builder: usize) {
        if let Some((_, robot)) = self.robots.iter_mut().find(|(rt, _)| *rt == builder)
            && matches!(robot.behaviour.task(), Task::Upgrade { .. })
        {
            robot.behaviour.tasks = vec![Task::Stop];
            robot.order = None;
        }
    }

    /// Building `b` walked one step up its scheme (`0x10033790`): it is taken out of the world
    /// and the scheme's next `.dat` made where it stood, with its ore carried over and the
    /// sphere an upgrade's new building runs. Returns the new target.
    fn swap_building(&mut self, b: usize, now: f64) -> Option<usize> {
        let path = self.upgrade_model(b)?;
        let &(at, yaw) = self.construction.placements.get(&b)?;
        let (clan, type_word) = (self.units.get(b)?.clan?, self.units.get(b)?.type_word);
        let ore = self.economy.held(b);
        self.remove_building(b);
        let made =
            self.place_building(clan, type_word, &path, at, yaw, now, Coming::Built(&UPGRADED), false)?;
        if ore > 0.0 {
            let most = self.economy.most(made);
            self.economy.add_ore(made, if most > 0.0 { ore.min(most) } else { ore });
        }
        Some(made)
    }

    /// Building `b` taken out of the world: its target dies unseen, its own records go, and the
    /// drawing drops it.
    fn remove_building(&mut self, b: usize) {
        if let Some(target) = self.battle.combat.targets.get_mut(b) {
            target.alive = false;
        }
        if let Some(solid) = self.ground.solids.get_mut(b) {
            solid.present = false;
        }
        if let Some(d) = self.deleted.get_mut(b) {
            *d = true;
        }
        if let Some(&object) = self.battle.objects.get(b) {
            self.killed.push(object);
        }
        self.buildings.retain(|x| x.target != b);
        self.places.retain(|x| x.target != b);
        self.factories.retain(|x| x.target != b);
        self.building_effects.retain(|(x, _)| x.target != b);
        self.emplacements.retain(|(t, _)| *t != b);
        self.construction.spheres.retain(|s| s.target != b);
        self.construction.unplaced.remove(&b);
        self.construction.machines.remove(&b);
        self.construction.hidden.remove(&b);
        self.battle.combat.absent.remove(&b);
        self.construction.placements.remove(&b);
        self.economy.sites.retain(|s| s.target != b);
        self.economy.ore.remove(&b);
        self.fx.retain(|o, _| !matches!(o, Owner::Building(t, _) if *t == b));
        self.selected.retain(|&t| t != b);
        if let (Some(unit), Some(p)) = (self.units.get(b), self.progression.as_mut()) {
            p.progress.deleted(unit.logical_id);
        }
    }

    /// Sphere `phases` started on building `t`, round its outer contour, 15 wider on a mine.
    /// The building's load group has made the three effects already (action 5), idle in their
    /// header's time mode 0 until a code starts them.
    ///
    /// The building's controller takes the first phase's code before it first plans: a
    /// building made in play has its controller's clock at the moment it is made, and plays the
    /// way from its constructor's record to the anchor that code opens at once, which is how
    /// the new building hides itself and shows the sign (docs/32, "What the building's
    /// controller does with the codes"). A mission's building has stood on its code-0 anchor
    /// since the load, its 5 s steps counted from there.
    fn start_sphere(&mut self, t: usize, phases: &'static [Phase], now: f64) {
        let path = self.commander.paths.get(t).cloned().unwrap_or_default();
        let controller = building_controller(&mut self.assembly, &path);
        let effects = controller.as_ref().map(sphere_effects).unwrap_or_default();
        let Some((position, radius)) =
            self.battle.combat.targets.get(t).map(|target| (target.position, target.radius))
        else {
            return;
        };
        let (centre, radius) = self.building_sphere(t).unwrap_or((position, radius));
        let mine = self.units.get(t).is_some_and(|u| u.type_word == BUILDING_MINE);
        let radius = radius + if mine { MINE_SPHERE_EXTRA } else { 0.0 };
        let frame = sphere_frame(centre, radius);
        for (id, name) in &effects {
            let owner = Owner::Building(t, *id);
            if self.fx.owned(owner).next().is_none() {
                self.fx.start(owner, name, frame, 1.0, now, None);
            }
        }
        if let Some(controller) = controller {
            let made = self.construction.unplaced.contains_key(&t);
            let machine = self.construction.machines.entry(t).or_insert_with(|| {
                let mut m = CodeMachine::new(controller, if made { now } else { 0.0 }, (centre, radius));
                if !made {
                    // What it played at the load, placing itself, ran long ago.
                    m.advance(now);
                }
                m
            });
            machine.sphere = (centre, radius);
        }
        self.construction.spheres.retain(|s| s.target != t);
        self.construction.spheres.push(Sphere {
            target: t,
            centre,
            radius,
            phases,
            phase: 0,
            phase_ms: now,
            effects,
        });
        self.start_phase(self.construction.spheres.len() - 1, true);
        self.run_machine(t, now);
    }

    /// Make a building of `type_word` for `clan` at `at` turned `yaw`
    /// (`CreateObjectFromScheme`, `Behavior.dll:0x1001d440`), in build mode: its scheme's first
    /// `.dat`, with a new building's sphere, refused where that sphere meets another
    /// building's.
    pub fn create_building(
        &mut self,
        clan: i64,
        type_word: u32,
        at: Vec3,
        yaw: f32,
        now: f64,
    ) -> Option<usize> {
        let path = self.placement_model(type_word)?;
        self.place_building(clan, type_word, &path, at, yaw, now, Coming::Built(&NEW_BUILDING), true)
    }

    /// The building at `path` for `clan`, standing finished at `at` unturned: the console's
    /// `bcreate` (`iron3d.dll:0x1003d6a0`, docs/15), under the logical id its clan's list has it
    /// under already. Made through the mission loader's own maker (`0x10033cb0`) with no
    /// create flag, it is refused where its sphere meets another building's, as any building
    /// is; it runs no construction sphere, which only create flag bit 1 gives
    /// (`ArealMap.dll:0x10015df3`); and its controller, sent no code, places it in the
    /// landscape on its first step, which sets it down on the mean of its cut contour since its
    /// start flag is clear (docs/04, "The start flag keeps a building at its file height").
    pub fn stand_building(
        &mut self,
        clan: i64,
        type_word: u32,
        path: &str,
        at: Vec3,
        logical_id: i32,
    ) -> Option<usize> {
        let now = self.hero.time_ms;
        self.place_building(clan, type_word, path, at, 0.0, now, Coming::Standing(logical_id), true)
    }

    /// The building `path` puts up for `clan` at `at` turned `yaw`: a new target with its unit,
    /// name, ground, doors and pod, factory and load group, on its clan's list, and as it
    /// `comes`. `clear` asks for the ground to be free of another building's sphere, which an
    /// upgrade standing where its own building stood does not.
    #[allow(clippy::too_many_arguments)]
    fn place_building(
        &mut self,
        clan: i64,
        type_word: u32,
        path: &str,
        at: Vec3,
        yaw: f32,
        now: f64,
        comes: Coming,
        clear: bool,
    ) -> Option<usize> {
        let path = path.to_owned();
        let plan = self.plan(&path);
        let at = match comes {
            Coming::Standing(_) => {
                let lift = crate::basement::set_down(
                    &self.ground.land,
                    &plan.inner,
                    &plan.outer,
                    at.to_array(),
                    yaw,
                );
                at + Vec3::Z * lift
            }
            Coming::Built(_) => at,
        };
        if let Some((mid, reach)) = plan.contour_sphere().filter(|_| clear) {
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
        let logical_id = match comes {
            Coming::Standing(id) => id,
            Coming::Built(_) => crate::progress::next_ids(self.units.iter().map(|u| u.logical_id)).1,
        };
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
            named_clan: Some(clan),
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
        let cut = crate::basement::footings(&mut self.assembly, &one, &self.ground.land)
            .into_iter()
            .next()
            .map(crate::basement::cut);
        if let Some(mut b) = Building::load(&mut self.assembly, &one, 0, t) {
            if let Some(part) = self.battle.combat.targets.get(t).and_then(|x| x.parts.get(b.part)) {
                b.place_zone(part);
            }
            self.buildings.push(b);
        }
        if let Some(f) = Factory::load(&mut self.assembly, &one, 0, t) {
            self.factories.push(f);
        }
        // A building put up in play docks units like any other: an Outpost or a generator is
        // built to be the charging station its ground-level dock makes it (docs/27). Its glows
        // are joined once its load group has placed them, as its controller places it.
        if let Some(places) = Places::load(&mut self.assembly, &one, 0, t) {
            self.places.push(places);
        }
        let effects = BuildingEffects::load(&mut self.assembly, &one, 0, t);
        self.construction.unplaced.insert(t, Unplaced { cut, effects });
        self.battle.combat.absent.insert(t);
        if let Some(s) = self.ground.solids.get_mut(t) {
            s.present = false;
        }
        self.construction.placements.insert(t, (at, yaw));
        // On its clan's SuperAI list at once: `AddObjectToGame`'s game message 1 makes its
        // record, and message 1's building case or the record's first pass through the frame
        // files it as slot 4's event 2 (`iron3d.dll:0x10060418`, `0x100333f6`; docs/34, "A
        // builder's building is filed as it is made").
        match comes {
            Coming::Built(phases) => {
                if let Some(p) = self.progression.as_mut() {
                    p.progress.place_building(logical_id, clan, type_word, at);
                }
                self.start_sphere(t, phases, now);
            }
            Coming::Standing(_) => self.place_in_landscape(t, now),
        }
        self.spawned += 1;
        self.added.push(t);
        Some(t)
    }

    /// Whether target `t` stands in the landscape: every building but one made in play that its
    /// controller has not yet placed.
    pub fn placed(&self, t: usize) -> bool {
        !self.construction.unplaced.contains_key(&t)
    }

    /// Building `t`'s controller placing it in the landscape (action 20,
    /// `CLandscape::PlaceBuilding`, docs/13): the landscape cut under it, its faces in the
    /// world, and its load group's effects shown.
    fn place_in_landscape(&mut self, t: usize, now: f64) {
        let Some(Unplaced { cut, effects }) = self.construction.unplaced.remove(&t) else { return };
        self.battle.combat.absent.remove(&t);
        if let Some(cut) = cut {
            self.ground.cuts.push(cut);
        }
        if let Some(s) = self.ground.solids.get_mut(t) {
            s.present = self.battle.combat.targets.get(t).is_some_and(|x| x.alive);
        }
        if let Some(effects) = effects {
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
        if self.places.iter().any(|p| p.target == t) {
            self.join_dock_glows();
        }
    }

    /// A sphere's phase starting: its code sent to the building's controller, and the clearing.
    ///
    /// Every unit within the sphere's radius + 15 (as the task starts) or + 20 (on a later
    /// phase) is ordered out, unless it is leaving already or upgrading (`0x10031680`).
    fn start_phase(&mut self, s: usize, first: bool) {
        let sphere = self.construction.spheres[s].clone();
        let Some(phase) = sphere.phases.get(sphere.phase) else { return };
        if let Some(code) = phase.code.filter(|&c| c != CODE_UPGRADING) {
            self.enter_code(s, code);
        }
        if phase.clear {
            let reach = sphere.radius + if first { CLEAR_ON_START } else { CLEAR_ON_CHANGE };
            let leave = Order { code: orders::LEAVE, parameter: 0, target: Target::NotDefined };
            for (t, robot) in &mut self.robots {
                let Some(target) = self.battle.combat.targets.get(*t).filter(|x| x.alive) else { continue };
                if target.position.truncate().distance(sphere.centre.truncate()) > reach
                    || matches!(robot.behaviour.task(), Task::Leave { .. } | Task::Upgrade { .. })
                {
                    continue;
                }
                robot.order = Some(leave);
                robot.behaviour.order(&leave);
            }
        }
    }

    /// The building's controller taking `code` (`IControl` slot 19, `Control.dll:0x10004800`):
    /// it stores the code, and nothing else. The anchor the controller stands on stops applying,
    /// and at its next step the planner queues the way to the anchor the code opens
    /// ([`CodeMachine::advance`]).
    fn enter_code(&mut self, s: usize, code: i32) {
        let t = self.construction.spheres[s].target;
        if let Some(machine) = self.construction.machines.get_mut(&t) {
            machine.code = code;
        }
    }

    /// Building `t`'s controller played up to `now`: each state it takes runs its action
    /// group, at the moment its step begins (docs/32, "What the building's controller does
    /// with the codes"). *Measured* on all 30 `fortif.rlb` controllers, whose construction
    /// states have one shape, every step 250 ms but the code-0 anchor's:
    ///
    /// | the code changes | the states it plays, and their groups | then, on the anchor |
    /// |---|---|---|
    /// | first plan → 1 | action 1, start the sign in mode 2 | nothing, every 250 ms |
    /// | 1 → 2 | start the dome and the ray in mode 1, the sign off, kill | kill every 250 ms |
    /// | 2 → 0 | the ray off, place, action 2 · start the dome in mode 3 | nothing, 5 s ± 12.5% |
    /// | 0 → 8 | start the dome in mode 1 (on the factories, their smoke off) | kill every 250 ms |
    /// | first plan → 10 | start the dome in mode 3 · place | kill every 250 ms |
    /// | 10 → 0 | — | nothing, 5 s ± 12.5% |
    ///
    /// Actions 1 and 2 are `IAnimation` slot 8 on node 0 with mode `0x200` and `0x201`, flag
    /// 1: they set and clear the node's flag 1 through the whole node tree, and the mesh's draw
    /// passes over a node that carries it (`AniMesh.dll:0x10005500`, `0x10014e57`). So action 1
    /// hides the building and action 2 shows it. Action 20 places it in the landscape
    /// (`CLandscape::PlaceBuilding`): until then it has no faces and has cut nothing
    /// ([`Unplaced`]).
    fn run_machine(&mut self, t: usize, now: f64) -> Vec<Event> {
        let Some(machine) = self.construction.machines.get_mut(&t) else { return Vec::new() };
        let taken = machine.advance(now);
        let (centre, radius) = machine.sphere;
        let mut events = Vec::new();
        for (at, records) in taken {
            for values in records {
                let [_, _, _, action, v4, v5, ..] = values;
                let owner = Owner::Building(t, v4);
                match action {
                    ACT_HIDE => {
                        self.construction.hidden.insert(t);
                    }
                    ACT_SHOW => {
                        self.construction.hidden.remove(&t);
                    }
                    control::ACT_START_EFFECT => self.fx.restart(owner, at, u32::try_from(v5).ok()),
                    control::ACT_EFFECT_ON => self.fx.switch(owner, true),
                    control::ACT_EFFECT_OFF => self.fx.switch(owner, false),
                    ACT_PLACE => self.place_in_landscape(t, at),
                    ACT_KILL_IN_SPHERE => events.extend(self.kill_inside(t, centre, radius)),
                    _ => {}
                }
            }
        }
        events
    }

    /// Every construction sphere's step at `now` (`0x10031680`): the next phase once one runs
    /// out, its code handed to the controller, and the task's end; then every building's
    /// controller played to `now`. The controller outlives the task: it stays on its code-0
    /// anchor, and the dome plays itself back out there.
    fn step_spheres(&mut self, now: f64) -> Vec<Event> {
        let mut s = 0;
        while s < self.construction.spheres.len() {
            let sphere = &mut self.construction.spheres[s];
            let length = sphere.phases.get(sphere.phase).map_or(0.0, |p| p.seconds * 1000.0);
            if now - sphere.phase_ms >= length {
                sphere.phase += 1;
                sphere.phase_ms += length;
                if sphere.phase >= sphere.phases.len() {
                    let done = self.construction.spheres.remove(s);
                    // A mine's order 10, queued behind the sphere, digs now (docs/23).
                    if let Some(site) = self.economy.site_mut(done.target)
                        && site.mine.is_some()
                    {
                        site.digging = true;
                    }
                    continue;
                }
                self.start_phase(s, false);
            }
            s += 1;
        }
        let mut machines: Vec<usize> = self.construction.machines.keys().copied().collect();
        machines.sort_unstable();
        machines.into_iter().flat_map(|t| self.run_machine(t, now)).collect()
    }

    /// Whether building `t` is drawn: every building but one its controller has hidden
    /// (action 1) and not yet shown again (action 2).
    pub fn shown(&self, t: usize) -> bool {
        !self.construction.hidden.contains(&t)
    }

    /// The kill (action 21, `Control.dll:0x100033e6`): the world's objects of classes 2, 4 and
    /// 10 that meet the building's construction sphere — the mask `0x414`, bits `1 << class` from
    /// the table at `0x1003b1a0` — each through its life system's slot 7, which does nothing to
    /// an invulnerable one (`0x1000eb76`, the byte property 162 sets). Class 4 is a unit and 10
    /// a tree or a stone (docs/30, "A class is slot 11"); a builder upgrading is invulnerable.
    /// Class 2 is an agent loaded from a `WPNS` record, and nothing in the install makes one
    /// (docs/32, "The kill").
    ///
    /// The world's query (`Terrain.dll` `IWorld` slot 3, `0x10025f40`) walks its object tree from
    /// the landscape and takes an object whose class is in the mask when its own bounding sphere
    /// (interface `0x18` slot 9 with 2) and the construction sphere meet: the distance between
    /// their centres, in three dimensions, no more than the two radii together (`0x10025d10`).
    fn kill_inside(&mut self, building: usize, centre: Vec3, radius: f32) -> Vec<Event> {
        let mut events = Vec::new();
        let robots = self.robots.iter().map(|(t, _)| *t).filter(|&t| !self.invulnerable(t));
        let scenery = self.scenery();
        let meets = |x: &parkan_sim::combat::Target| x.centre.distance(centre) <= x.radius + radius;
        let inside: Vec<usize> = robots
            .chain(scenery)
            .filter(|&t| t != building)
            .filter(|&t| self.battle.combat.targets.get(t).is_some_and(|x| x.alive && meets(x)))
            .collect();
        for t in inside {
            events.extend(self.battle.combat.ground_loss(t, f32::MAX / 4.0));
        }
        if !self.hero.dead() && !self.hero_away() && self.battle.combat.hero.as_ref().is_some_and(meets) {
            let mut lives: Vec<&mut parkan_sim::damage::Life> =
                self.hero.lives.iter_mut().flatten().collect();
            parkan_sim::damage::share_loss(&mut lives, f32::MAX / 4.0);
        }
        events
    }

    /// Every target that is scenery, a tree or a stone: the objects of class 10.
    fn scenery(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.units.len())
            .filter(|&t| matches!(self.units[t].kind, mission::KIND_VEGETATION | mission::KIND_ROCK))
    }

    /// The console's `death` (`iron3d.dll:0x1003db10`, docs/15): every object of class 10 alone
    /// (the world query's mask `0x400`, `IWorld` slot 3) whose own sphere meets the sphere of
    /// `radius` about the ground at (x, y) is killed through its life system's slot 7, as the
    /// construction sphere's kill kills (docs/32), at once. The sphere stands on the level's
    /// probe for the landscape and the buildings there, 0 where it finds nothing. So it fells
    /// trees and stones, and passes over units, buildings and the hero.
    pub(crate) fn death(&mut self, x: f32, y: f32, radius: f32) -> Vec<Event> {
        let ground = self.ground.below(x, y, crate::play::PROBE_TOP).map_or(0.0, |h| h.point.z);
        let centre = Vec3::new(x, y, ground);
        let meets = |x: &parkan_sim::combat::Target| x.centre.distance(centre) <= x.radius + radius;
        let inside: Vec<usize> = self
            .scenery()
            .filter(|&t| self.battle.combat.targets.get(t).is_some_and(|x| x.alive && meets(x)))
            .collect();
        inside.into_iter().flat_map(|t| self.battle.combat.ground_loss(t, f32::MAX / 4.0)).collect()
    }

    /// Whether target `t`'s life system refuses its kill, slot 7: its invulnerability byte
    /// (`Control.dll:0x1000eb76`), which property 162 sets on a builder working at an upgrade.
    pub(crate) fn invulnerable(&self, t: usize) -> bool {
        self.robots.iter().any(|(rt, r)| {
            *rt == t && matches!(r.behaviour.task(), Task::Upgrade { state: UpgradeState::Working, .. })
        })
    }
}

/// The frame an action-5 effect hangs in (`Control.dll:0x10002e72`): the identity kept at
/// `0x10041ba8`, which its initialiser (`0x10003d70`) writes with its axes turned round — the
/// first along z, the second along x, the third along y — each column scaled by the sphere's
/// radius and its translation the sphere's centre, handed to the effect as a world frame
/// (manager slot 10 with 2). So the effect's depth, the axis its position channel travels and
/// a dome's pole, stands up, and a sprite is drawn through a frame the sphere's size.
pub fn sphere_frame(centre: Vec3, radius: f32) -> Frame {
    Frame { origin: centre, axes: [Vec3::Z * radius, Vec3::X * radius, Vec3::Y * radius], points: true }
}

/// The steepest basement face's normal z between `inner` and `outer`, 1 with no face: the
/// smallest z of the unit normals of the band's faces (`FindMinNormalZProc`,
/// `Terrain.dll:0x1000da20`), each face wound counter-clockwise across the ground
/// ([`crate::basement::band_faces`]).
fn basement_steepest(inner: &[Vec3], outer: &[Vec3]) -> f32 {
    let ring = |r: &[Vec3]| r.iter().map(|p| p.to_array()).collect::<Vec<_>>();
    crate::basement::band_faces(&ring(inner), &ring(outer))
        .into_iter()
        .map(|t| {
            let [a, b, c] = t.map(Vec3::from_array);
            (b - a).cross(c - a).normalize_or_zero().z
        })
        .fold(1.0, f32::min)
}

/// The controller of the building at `path`: its root record's `.ctl`.
fn building_controller(assembly: &mut crate::assembly::Assembly, path: &str) -> Option<control::Controller> {
    let parts = assembly.parts(KIND_BUILDING, path);
    let root = parts.iter().find(|p| p.host == -1)?;
    let slot = assembly.library.record_slot(assembly.library.get(&root.record), "ctl", 0)?;
    assembly
        .archive(&slot.library)
        .and_then(|a| a.read_name(&slot.member).ok())
        .and_then(|data| control::parse(data, &slot.member).ok())
}

/// A building's load-group construction-sphere effects (action 5): each id and its name.
fn sphere_effects(controller: &control::Controller) -> Vec<(i32, String)> {
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
    fn a_sphere_effect_stands_up_the_spheres_radius_long() {
        let frame = sphere_frame(Vec3::new(10.0, 20.0, 30.0), 40.0);
        assert_eq!(frame.origin, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(frame.axes, [Vec3::Z * 40.0, Vec3::X * 40.0, Vec3::Y * 40.0]);
        // A real matrix, so a sprite is drawn through it.
        assert!(frame.basis().is_some());
        // The ray's plasma falls along the first axis from a radius up to the centre.
        assert_eq!(frame.point(Vec3::X), Vec3::new(10.0, 20.0, 70.0));
    }

    #[test]
    fn an_upgrade_clears_the_area_as_it_starts_and_keeps_0x309_from_the_controller() {
        assert!(UPGRADING[0].clear && !UPGRADING[1].clear && !UPGRADING[2].clear);
        assert_eq!(UPGRADING[0].code, Some(CODE_UPGRADING));
        assert!(UPGRADED.iter().all(|p| !p.clear));
    }

    #[test]
    fn a_new_building_runs_its_sphere_41_seconds() {
        let total: f64 = NEW_BUILDING.iter().map(|p| p.seconds).sum();
        assert_eq!(total, 41.0);
        assert_eq!(NEW_BUILDING.iter().filter(|p| p.clear).map(|p| p.seconds).sum::<f64>(), 30.0);
    }

    /// A controller of the construction states' shape (docs/32, "What the building's
    /// controller does with the codes"): 0 the source; 1 hide and start the sign, into the
    /// code-1 anchor 2; 3 start the dome and the ray, the sign off, kill, into the code-2 anchor
    /// 4, which kills; 5 the ray off, place, show, and 6 the dome backward, into the code-0
    /// anchor 7, 5 s and jittered. Every step but the last anchor's is 250 ms.
    fn construction_states() -> control::Controller {
        use control::{ANY_REQUEST, NO_EDGE, Reference, STATE_ANCHOR, STATE_FIXED, STATE_JITTER, State};
        let state = |request: i32, anchor: bool, group: i32, length: f32, jitter: bool| State {
            request,
            actions: group,
            length,
            mode: STATE_FIXED | if anchor { STATE_ANCHOR } else { 0 } | if jitter { STATE_JITTER } else { 0 },
            ..State::default()
        };
        let states = vec![
            state(ANY_REQUEST, false, -1, 250.0, false),
            state(ANY_REQUEST, false, 0, 250.0, false),
            state(1, true, -1, 250.0, false),
            state(ANY_REQUEST, false, 1, 250.0, false),
            state(2, true, 2, 250.0, false),
            state(ANY_REQUEST, false, 3, 250.0, false),
            state(ANY_REQUEST, false, 4, 250.0, false),
            state(0, true, -1, 5000.0, true),
        ];
        let n = states.len();
        let mut costs = vec![NO_EDGE; n * n];
        for (from, to) in [(0, 1), (1, 2), (2, 2), (2, 3), (3, 4), (4, 4), (4, 5), (5, 6), (6, 7), (7, 7)] {
            costs[to * n + from] = 1.0;
        }
        let record = |group: usize, action: i32, v4: i32, v5: i32| Reference {
            resource: Default::default(),
            values: [0, 0, 0, action, v4, v5, 0, 0, 0],
            group,
        };
        let references = vec![
            record(0, ACT_HIDE, 0, 0),
            record(0, control::ACT_START_EFFECT, SIGN, TIME_LOOP as i32),
            record(1, control::ACT_START_EFFECT, DOME, TIME_ONCE as i32),
            record(1, control::ACT_START_EFFECT, RAY, TIME_ONCE as i32),
            record(1, control::ACT_EFFECT_OFF, SIGN, 0),
            record(1, ACT_KILL_IN_SPHERE, 0, 0),
            record(2, ACT_KILL_IN_SPHERE, 0, 0),
            record(3, control::ACT_EFFECT_OFF, RAY, 0),
            record(3, ACT_PLACE, 0, 0),
            record(3, ACT_SHOW, 0, 0),
            record(4, control::ACT_START_EFFECT, DOME, TIME_REVERSE as i32),
        ];
        control::Controller { states, costs, references, ..control::Controller::default() }
    }

    fn actions(taken: &[Taken]) -> Vec<(f64, Vec<i32>)> {
        taken.iter().map(|(at, records)| (*at, records.iter().map(|v| v[3]).collect())).collect()
    }

    #[test]
    fn a_code_takes_effect_at_the_controllers_next_step_and_each_state_on_the_way_plays_a_step() {
        let mut m = CodeMachine::new(construction_states(), 1000.0, (Vec3::ZERO, 10.0));
        // The first code comes before the first plan: from the constructor's record the way to
        // code 1's anchor, its first state hiding the building and starting the sign at once.
        m.code = 1;
        assert_eq!(actions(&m.advance(1000.0)), vec![(1000.0, vec![ACT_HIDE, control::ACT_START_EFFECT])]);
        assert_eq!(actions(&m.advance(1600.0)), vec![(1250.0, vec![]), (1500.0, vec![])]);
        assert!(m.on_anchor(1));
        // Slot 19 stores code 2 at 1.6 s; nothing runs until the anchor's step ends at 1.75 s.
        m.code = 2;
        assert!(m.advance(1700.0).is_empty(), "not as the code arrives");
        let kill = vec![ACT_KILL_IN_SPHERE];
        let start = vec![
            control::ACT_START_EFFECT,
            control::ACT_START_EFFECT,
            control::ACT_EFFECT_OFF,
            ACT_KILL_IN_SPHERE,
        ];
        assert_eq!(
            actions(&m.advance(2250.0)),
            vec![(1750.0, start), (2000.0, kill.clone()), (2250.0, kill)]
        );
        // Code 0 from the kill anchor: the state that places and shows, and a step later the one
        // that turns the dome back, then the 5 s anchor.
        m.code = 0;
        let taken = actions(&m.advance(3100.0));
        assert_eq!(taken[0], (2500.0, vec![control::ACT_EFFECT_OFF, ACT_PLACE, ACT_SHOW]));
        assert_eq!(taken[1], (2750.0, vec![control::ACT_START_EFFECT]));
        assert_eq!(taken[2], (3000.0, vec![]));
        assert!(m.on_anchor(0));
        // Its step is 5 s less up to 12.5%, and held to 5 s at most.
        let mut last = 3000.0;
        for (at, _) in actions(&m.advance(3000.0 + 500_000.0)) {
            if at > 3000.0 {
                assert!((4375.0..=5000.0).contains(&(at - last)), "{}", at - last);
            }
            last = at;
        }
    }
}
