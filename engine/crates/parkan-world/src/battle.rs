//! A mission's fighting: its placed objects as targets, and the rounds they can be
//! struck by. See `docs/26-damage.md` and `docs/29-weapons.md`.

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;
use glam::{Mat4, Vec3};
use parkan_formats::control::{
    self, ACT_DELETE_EFFECT, ACT_EFFECT_OFF, ACT_EFFECT_ON, ACT_EFFECT_POINTS, ACT_EXPLODE_NODE,
    ACT_START_EFFECT, ENTRY_EDGE, ENTRY_HIT, ENTRY_LOAD, ENTRY_RANGE, SEEKER_CONE, SEEKER_LOCK, SEEKER_REACH,
    SEEKER_TYPE, TRIPLE_TOP_SPEED, TRIPLE_TURN,
};
use parkan_formats::cpt::{self, ControlPoint};
use parkan_formats::exp::{self, Explosion};
use parkan_formats::mesh::{Mesh, NO_SLOT, SLOTS_PER_VARIANT};
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::ndp::{self, NodeDamage};
use parkan_formats::objects::ResourceRef;
use parkan_formats::pose::Pose;
use parkan_sim::combat::{Combat, Part, RoundKind, Seeker, Target};
use parkan_sim::damage::{Life, NEVER_HIDDEN_NODE_FLAG, VITAL_NODE_FLAG};

use crate::assembly::Assembly;
use crate::models::{Instance, Objects, build_model};
use crate::textures::TextureStore;

/// The `Type` words of a warrior, an HQ and a hero: the units a hostile clan's level
/// ratio scales (`iron3d.dll:0x10076010`).
pub const RATIO_TYPES: [i64; 3] = [0x0100_8000, 0x0101_0000, 0x0102_0000];
/// A round's frame +116 bit: its gun starts selected (`World3D.dll:0x1000ed20`).
pub const STARTS_SELECTED: i32 = 4;
/// How many of one round can be drawn at once.
pub const POOL: usize = 32;

/// A round record's controller frame flags, beside what the simulation keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub record: String,
    pub frame_flags: i32,
    /// The effects its load group creates.
    pub effects: Vec<RoundEffect>,
    /// Its record's control points.
    pub points: Vec<ControlPoint>,
    /// What its hit, edge and range groups (block entries 2, 3 and 4) do to its effects, in
    /// [`RoundEnd`](parkan_sim::combat::RoundEnd)'s order.
    pub ends: [Vec<EffectCommand>; 3],
    /// Its controller's `+92`: the ms it stays in the world once its flight is over
    /// (docs/29-weapons.md, "A beam outlives its round").
    pub death_ms: f64,
}

/// An effect a round's load group creates (action 4): its name, the three control points it
/// sits on, and the id its groups name it by (v7).
#[derive(Clone, Debug, PartialEq)]
pub struct RoundEffect {
    pub name: String,
    pub points: [usize; 3],
    pub id: i32,
}

/// What a group does to one of its object's effects, by id (docs/13, "The section-5 record").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectCommand {
    /// Action 10: start it now in a time mode.
    Start(i32, u32),
    /// Actions 18 and 19: switch it on or off.
    Switch(i32, bool),
    /// Action 8: delete it.
    Delete(i32),
}

/// The effect commands of a group's records.
fn effect_commands(records: &[&control::Reference]) -> Vec<EffectCommand> {
    records
        .iter()
        .filter_map(|r| {
            let id = r.values[4];
            Some(match r.action() {
                ACT_START_EFFECT => EffectCommand::Start(id, u32::try_from(r.values[5]).ok()?),
                ACT_EFFECT_ON => EffectCommand::Switch(id, true),
                ACT_EFFECT_OFF => EffectCommand::Switch(id, false),
                ACT_DELETE_EFFECT => EffectCommand::Delete(id),
                _ => return None,
            })
        })
        .collect()
}

pub struct Battle {
    pub combat: Combat,
    /// The mission object each target is.
    pub objects: Vec<usize>,
    /// Each round kind's record.
    pub kinds: Vec<Loaded>,
    /// Each round kind's pooled instances, once drawing is set up.
    pub pools: Vec<Vec<usize>>,
    /// Each target's parts' nodes' `.exp`: what a node plays when it is destroyed.
    pub explosions: Vec<Vec<Vec<Option<Explosion>>>>,
    /// Each target's parts' wears: the material names their batches index.
    pub wears: Vec<Vec<Vec<String>>>,
    /// Each target's mission object kind: building, unit, vegetation or rock.
    pub placed_kinds: Vec<u32>,
    /// Each target's parts' meshes with their wears, for drawing them node by node.
    pub meshes: Vec<Vec<Rc<crate::assembly::LoadedMesh>>>,
    /// How long each target lasts once dead: its first part's controller's `+92`, ms.
    pub death_ms: Vec<f64>,
    kind_of: HashMap<String, usize>,
}

fn read(assembly: &mut Assembly, r: &ResourceRef) -> Option<Vec<u8>> {
    assembly.archive(&r.library)?.read_name(&r.member).ok().map(<[u8]>::to_vec)
}

fn table(assembly: &mut Assembly, record: &str) -> Option<Vec<NodeDamage>> {
    let slot = assembly.library.get(record)?.slot_with_suffix("ndp")?.clone();
    ndp::parse(&read(assembly, &slot)?, &slot.member).ok()
}

fn explosion(assembly: &mut Assembly, r: &ResourceRef) -> Option<Explosion> {
    if !r.is_set() {
        return None;
    }
    exp::parse(&read(assembly, r)?, &r.member).ok()
}

fn number(v: Value) -> i64 {
    match v {
        Value::Int(i) => i64::from(i),
        Value::Float(f) => f as i64,
    }
}

/// The level ratio an object's hit points take: the difficulty's, for a warrior, HQ
/// or hero of a clan not allied with the player's, and 1 otherwise.
pub fn object_ratio(
    clans: &[mission::Clan],
    object: &mission::Object,
    player_clan: Option<i64>,
    ratio: f32,
) -> f32 {
    let kind = object.property("Type").map(|p| number(p.value));
    let (Some(kind), Some(clan), Some(player)) = (kind, object.clan_id(), player_clan) else { return 1.0 };
    if !RATIO_TYPES.contains(&kind) {
        return 1.0;
    }
    let words = mission::relation_words(clans);
    let word = usize::try_from(player).ok().zip(usize::try_from(clan).ok());
    let allied = word.and_then(|(p, c)| words.get(p)?.get(c)).is_some_and(|&relation| relation != 0);
    if allied { 1.0 } else { ratio }
}

/// A part's nodes' hit points from its record's `.ndp`, at the volume scale `volume_scale`
/// and the level ratio `ratio`, and what each node plays when it is destroyed; no life where
/// the record names no table.
pub fn part_damage(
    assembly: &mut Assembly,
    part: &crate::assembly::Part,
    mesh: &Mesh,
    volume_scale: f32,
    ratio: f32,
    building: bool,
) -> (Option<Life>, Vec<Option<Explosion>>) {
    let Some(nodes_table) = table(assembly, &part.record) else { return (None, Vec::new()) };
    let blasts = nodes_table
        .iter()
        .map(|d| {
            let r = if d.explosion.library.is_empty() {
                ResourceRef { library: part.reference.library.clone(), ..d.explosion.clone() }
            } else {
                d.explosion.clone()
            };
            explosion(assembly, &r)
        })
        .collect();
    let parents = mesh.nodes.iter().map(|n| (n.parent != 0xFFFF).then_some(usize::from(n.parent))).collect();
    let vital = mesh.nodes.iter().map(|n| n.flags & VITAL_NODE_FLAG != 0).collect();
    // `AniMesh.dll:0x10005840`: variants 0, 1 and 2 in a row with a level-0 slot; none is 1.
    let stages: Vec<u8> = mesh
        .nodes
        .iter()
        .map(|n| {
            let run = (0..3).take_while(|&v| n.slot_index[v * SLOTS_PER_VARIANT] != NO_SLOT).count();
            run.max(1) as u8
        })
        .collect();
    let never_hidden: Vec<bool> = mesh.nodes.iter().map(|n| n.flags & NEVER_HIDDEN_NODE_FLAG != 0).collect();
    let mut life =
        Life::new(&nodes_table, parents, vital, volume_scale, ratio).staged(&stages, &never_hidden);
    life.building = building;
    (Some(life), blasts)
}

/// A pose placed at `position`, turned `yaw` about z.
fn placement(position: [f32; 3], yaw: f32) -> Pose {
    let half = f64::from(yaw) / 2.0;
    Pose { translation: position.map(f64::from), rotation: [half.cos(), 0.0, 0.0, half.sin()] }
}

impl Battle {
    /// Every placed object but `hero` as a target. Every one of them takes damage through
    /// its parts' `.ndp`, scenery included: a tree and a stone are agents like any other
    /// and carry a life system (`docs/26-damage.md`, "Vegetation and rock carry node
    /// life"). A mission's building is a `CBuilding` around its agent, agent kind 3, which
    /// node 0's death only marks (`docs/26-damage.md`, "Hit points").
    pub fn load(
        assembly: &mut Assembly,
        mission: &Mission,
        hero: Option<usize>,
        ratio: f32,
    ) -> Result<Battle> {
        let player = hero.and_then(|h| mission.objects.get(h)).and_then(mission::Object::clan_id);
        let mut battle = Battle {
            combat: Combat::default(),
            objects: Vec::new(),
            kinds: Vec::new(),
            pools: Vec::new(),
            explosions: Vec::new(),
            wears: Vec::new(),
            placed_kinds: Vec::new(),
            meshes: Vec::new(),
            death_ms: Vec::new(),
            kind_of: HashMap::new(),
        };
        for (i, object) in mission.objects.iter().enumerate() {
            if Some(i) == hero {
                continue;
            }
            battle.add(assembly, &mission.clans, object, i, player, ratio);
        }
        Ok(battle)
    }

    /// Placed object `object`, numbered `index`, as a new target, with its clans' relations
    /// to `player` for the level ratio; its target, or `None` when nothing of it loads.
    pub fn add(
        &mut self,
        assembly: &mut Assembly,
        clans: &[mission::Clan],
        object: &mission::Object,
        index: usize,
        player: Option<i64>,
        ratio: f32,
    ) -> Option<usize> {
        let scale = object.placed_scale();
        let place = placement(object.position, object.rotation);
        // The control system re-reads its mesh's scale every tick and rescales every node's
        // life by the three factors multiplied (`Control.dll:0x10007ac6` -> `0x10009ee0`).
        // Only scenery is built at its placement scale, so only a tree or a stone has one
        // other than 1 (docs/04-missions.md, "The scale").
        let volume_scale = scale * scale * scale;
        let object_ratio = object_ratio(clans, object, player, ratio);
        let armour = crate::shields::armour(assembly, object.kind, &object.path);
        let mut parts = Vec::new();
        let mut blasts = Vec::new();
        let mut part_wears = Vec::new();
        let mut part_meshes = Vec::new();
        let mut lasts = 0.0;
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        // Each assembly part's index among the parts that load, which a part's host names.
        let mut loaded_as: Vec<Option<usize>> = Vec::new();
        for part in assembly.parts(object.kind, &object.path) {
            loaded_as.push(None);
            let Some(loaded) = assembly.mesh(&part.reference) else { continue };
            if parts.is_empty() {
                let ctl = assembly.library.get(&part.record).and_then(|r| r.slot_with_suffix("ctl")).cloned();
                lasts = ctl
                    .and_then(|c| read(assembly, &c).and_then(|b| control::parse(&b, &c.member).ok()))
                    .map_or(0.0, |c| f64::from(c.death_ms));
            }
            part_meshes.push(loaded.clone());
            let mesh = Rc::new(loaded.mesh.clone());
            // STAND-IN: docs/24-motion.md#playing-a-state--read-and-measured -- the poses
            // other units are struck in are not played: every target's nodes stay at
            // their rest poses.
            let nodes: Vec<Pose> = (0..mesh.nodes.len())
                .map(|n| {
                    let mut local = part.pose.compose(&mesh.world_pose(n));
                    local.translation = local.translation.map(|v| v * f64::from(scale));
                    place.compose(&local)
                })
                .collect();
            for (n, node) in mesh.nodes.iter().enumerate() {
                let Some(slot) = mesh.slots.get(usize::from(node.slot_index[0])) else { continue };
                let [cx, cy, cz, r] = slot.sphere;
                let c = nodes[n].apply([cx, cy, cz].map(|v| f64::from(v * scale)));
                let c = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                lo = lo.min(c - Vec3::splat(r * scale));
                hi = hi.max(c + Vec3::splat(r * scale));
            }
            let (mut life, part_blasts) = part_damage(
                assembly,
                &part,
                &mesh,
                volume_scale,
                object_ratio,
                object.kind == mission::KIND_BUILDING,
            );
            // One control system holds every part's nodes, so the one armour covers them all.
            if let Some(life) = life.as_mut() {
                life.armour = armour;
            }
            blasts.push(part_blasts);
            let portals = Rc::new(crate::models::portal_triangles(&mesh, &loaded.wear));
            let host = usize::try_from(part.host)
                .ok()
                .and_then(|h| loaded_as.get(h).copied().flatten())
                .zip(usize::try_from(part.node).ok());
            *loaded_as.last_mut().unwrap() = Some(parts.len());
            parts.push(Part { mesh, nodes, scale, life, portals, host });
            part_wears.push(loaded.wear.materials.clone());
        }
        if parts.is_empty() || lo.x > hi.x {
            return None;
        }
        let centre = (lo + hi) / 2.0;
        // The node sphere's centre, placed; a unit's follows it as it moves ([`crate::play`]).
        let assembly_parts = assembly.parts(object.kind, &object.path);
        let (local, _) = crate::robot::node_sphere(assembly, &assembly_parts);
        let aim = place.apply((local * scale).to_array().map(f64::from));
        let aim = Vec3::new(aim[0] as f32, aim[1] as f32, aim[2] as f32);
        let shield = if matches!(object.kind, mission::KIND_UNIT | mission::KIND_BUILDING) {
            crate::shields::load(assembly, object.kind, &object.path, object_ratio)
        } else {
            None
        };
        self.combat.targets.push(Target {
            parts,
            centre,
            radius: (hi - lo).length() / 2.0,
            alive: true,
            position: Vec3::from_array(object.position),
            aim,
            shield,
        });
        self.objects.push(index);
        self.explosions.push(blasts);
        self.wears.push(part_wears);
        self.placed_kinds.push(object.kind);
        self.meshes.push(part_meshes);
        self.death_ms.push(lasts);
        Some(self.combat.targets.len() - 1)
    }

    /// The round kind a `BULL` record fires, loaded once.
    pub fn round_kind(&mut self, assembly: &mut Assembly, record: &str) -> Option<usize> {
        let key = record.to_ascii_lowercase();
        if let Some(&k) = self.kind_of.get(&key) {
            return Some(k);
        }
        let slots = assembly.library.get(record)?.slots.clone();
        let find = |suffix: &str| slots.iter().find(|s| s.is_set() && s.suffix() == suffix).cloned();
        let ctl_ref = find("ctl")?;
        let controller = control::parse(&read(assembly, &ctl_ref)?, &ctl_ref.member).ok()?;
        let nodes = table(assembly, record).unwrap_or_default();
        let hit_points = nodes.first().map_or(0.0, |n| n.durability);
        let hit = nodes.first().and_then(|n| explosion(assembly, &n.explosion));
        let range_ref = controller
            .group(ENTRY_RANGE)
            .into_iter()
            .find(|r| r.action() == ACT_EXPLODE_NODE)
            .map(|r| r.resource.clone());
        let range_end = range_ref.and_then(|r| {
            let r =
                if r.library.is_empty() { ResourceRef { library: ctl_ref.library.clone(), ..r } } else { r };
            explosion(assembly, &r)
        });
        let radius =
            find("msh").and_then(|m| assembly.mesh(&m)).and_then(|m| m.mesh.sphere).map_or(0.1, |(_, r)| r);
        self.combat.kinds.push(RoundKind {
            name: record.to_owned(),
            top_speed: controller.triples[TRIPLE_TOP_SPEED][1],
            range: controller.bounds[0],
            radius,
            hit_points,
            hit,
            range_end,
            // A missile's class-17 seeker, and the frame's fourth triple it turns at (docs/29).
            seeker: controller.components.iter().find(|c| c.type_id == SEEKER_TYPE).map(|c| Seeker {
                cone: c.values[SEEKER_CONE],
                reach: c.values[SEEKER_REACH],
                lock_ms: c.values[SEEKER_LOCK],
            }),
            turn_rate: controller.triples[TRIPLE_TURN],
            mode: controller.mode,
        });
        let effects = controller
            .group(ENTRY_LOAD)
            .into_iter()
            .filter(|r| r.action() == ACT_EFFECT_POINTS && !r.resource.member.is_empty())
            .map(|r| RoundEffect {
                name: r.resource.member.clone(),
                points: [4, 5, 6].map(|k| usize::try_from(r.values[k]).unwrap_or(0)),
                id: r.values[7],
            })
            .collect();
        let points = find("cpt")
            .and_then(|c| read(assembly, &c).and_then(|b| cpt::parse(&b, &c.member).ok()))
            .unwrap_or_default();
        let ends =
            [ENTRY_HIT, ENTRY_EDGE, ENTRY_RANGE].map(|entry| effect_commands(&controller.group(entry)));
        self.kinds.push(Loaded {
            record: record.to_owned(),
            frame_flags: controller.flags,
            effects,
            points,
            ends,
            death_ms: f64::from(controller.death_ms),
        });
        self.pools.push(Vec::new());
        let k = self.combat.kinds.len() - 1;
        self.kind_of.insert(key, k);
        Some(k)
    }

    /// Give every round kind a model and a pool of hidden instances in `objects`.
    pub fn draw_rounds(
        &mut self,
        assembly: &mut Assembly,
        store: &mut TextureStore,
        objects: &mut Objects,
    ) -> Result<()> {
        for (k, loaded) in self.kinds.iter().enumerate() {
            if !self.pools[k].is_empty() {
                continue;
            }
            let Some(model) = build_model(assembly, store, mission::KIND_ROCK, &loaded.record)? else {
                continue;
            };
            objects.models.push(model);
            let model = objects.models.len() - 1;
            for _ in 0..POOL {
                objects.instances.push(Instance {
                    model,
                    position: [0.0; 3],
                    rotation: 0.0,
                    scale: 1.0,
                    hidden: true,
                });
                objects.placed.push(usize::MAX);
                self.pools[k].push(objects.instances.len() - 1);
            }
        }
        Ok(())
    }

    /// Where every pooled round instance goes this frame: its matrix, and whether it shows.
    pub fn round_instances(&self) -> Vec<(usize, Mat4, bool)> {
        let mut used = vec![0usize; self.pools.len()];
        let mut out = Vec::new();
        for r in &self.combat.rounds {
            let Some(pool) = self.pools.get(r.kind) else { continue };
            let Some(&instance) = pool.get(used[r.kind]) else { continue };
            used[r.kind] += 1;
            let y = r.forward;
            let x = y.cross(Vec3::Z).normalize_or(Vec3::X);
            let z = x.cross(y);
            out.push((
                instance,
                Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), r.position.extend(1.0)),
                true,
            ));
        }
        for (k, pool) in self.pools.iter().enumerate() {
            out.extend(pool.iter().skip(used[k]).map(|&i| (i, Mat4::IDENTITY, false)));
        }
        out
    }
}
