//! A mission's fighting: its placed objects as targets, and the rounds they can be
//! struck by. See `docs/26-damage.md` and `docs/29-weapons.md`.

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;
use glam::{Mat4, Vec3};
use parkan_formats::control::{self, ACT_EXPLODE_NODE, ENTRY_RANGE, TRIPLE_TOP_SPEED};
use parkan_formats::exp::{self, Explosion};
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::ndp::{self, NodeDamage};
use parkan_formats::objects::ResourceRef;
use parkan_formats::pose::Pose;
use parkan_sim::combat::{Combat, Part, RoundKind, Target};
use parkan_sim::damage::{Life, VITAL_NODE_FLAG};

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
}

pub struct Battle {
    pub combat: Combat,
    /// The mission object each target is.
    pub objects: Vec<usize>,
    /// Each round kind's record.
    pub kinds: Vec<Loaded>,
    /// Each round kind's pooled instances, once drawing is set up.
    pub pools: Vec<Vec<usize>>,
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
    mission: &Mission,
    object: &mission::Object,
    player_clan: Option<i64>,
    ratio: f32,
) -> f32 {
    let kind = object.property("Type").map(|p| number(p.value));
    let (Some(kind), Some(clan), Some(player)) = (kind, object.clan_id(), player_clan) else { return 1.0 };
    if !RATIO_TYPES.contains(&kind) {
        return 1.0;
    }
    let name = mission.clans.get(clan as usize).map(|c| c.name.as_str());
    let allied = mission
        .clans
        .get(player as usize)
        .and_then(|p| p.relations.iter().find(|(n, _)| Some(n.as_str()) == name))
        .is_some_and(|&(_, relation)| relation != 0);
    if allied { 1.0 } else { ratio }
}

/// A pose placed at `position`, turned `yaw` about z.
fn placement(position: [f32; 3], yaw: f32) -> Pose {
    let half = f64::from(yaw) / 2.0;
    Pose { translation: position.map(f64::from), rotation: [half.cos(), 0.0, 0.0, half.sin()] }
}

impl Battle {
    /// Every placed object but `hero` as a target. Units and buildings take damage
    /// through their parts' `.ndp`; scenery stops rounds and takes none.
    ///
    /// STAND-IN: docs/04-missions.md#the-scale -- whether vegetation and rock carry
    /// node life is not established; they take no damage.
    pub fn load(
        assembly: &mut Assembly,
        mission: &Mission,
        hero: Option<usize>,
        ratio: f32,
    ) -> Result<Battle> {
        let player = hero.and_then(|h| mission.objects.get(h)).and_then(mission::Object::clan_id);
        let mut combat = Combat::default();
        let mut objects = Vec::new();
        for (i, object) in mission.objects.iter().enumerate() {
            if Some(i) == hero {
                continue;
            }
            let scale = object.placed_scale();
            let place = placement(object.position, object.rotation);
            let damageable = !matches!(object.kind, mission::KIND_VEGETATION | mission::KIND_ROCK);
            let object_ratio = object_ratio(mission, object, player, ratio);
            let mut parts = Vec::new();
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for part in assembly.parts(object.kind, &object.path) {
                let Some(loaded) = assembly.mesh(&part.reference) else { continue };
                let mesh = Rc::new(loaded.mesh.clone());
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
                let life = damageable.then(|| table(assembly, &part.record)).flatten().map(|t| {
                    let parents = mesh
                        .nodes
                        .iter()
                        .map(|n| (n.parent != 0xFFFF).then_some(usize::from(n.parent)))
                        .collect();
                    let vital = mesh.nodes.iter().map(|n| n.flags & VITAL_NODE_FLAG != 0).collect();
                    Life::new(&t, parents, vital, 1.0, object_ratio)
                });
                parts.push(Part { mesh, nodes, scale, life });
            }
            if parts.is_empty() || lo.x > hi.x {
                continue;
            }
            let centre = (lo + hi) / 2.0;
            combat.targets.push(Target { parts, centre, radius: (hi - lo).length() / 2.0, alive: true });
            objects.push(i);
        }
        Ok(Battle { combat, objects, kinds: Vec::new(), pools: Vec::new(), kind_of: HashMap::new() })
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
        });
        self.kinds.push(Loaded { record: record.to_owned(), frame_flags: controller.flags });
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
