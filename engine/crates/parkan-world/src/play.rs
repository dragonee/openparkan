//! A mission played: the hero, the ground it walks on, the battle around it and
//! the effects it plays.

use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::control::{ACT_EFFECT_POINTS, ACT_EFFECT_TIME_POINT, ENTRY_LOAD};
use parkan_formats::materials::Library;
use parkan_formats::mission::{KIND_BUILDING, Mission};
use parkan_formats::{gamedir, landmesh};
use parkan_sim::combat::{Event, Part, Round};
use parkan_sim::effects::{Cue, Frame, Sprite};
use parkan_sim::ground::Ground;
use parkan_sim::hit::segment_mesh;

use crate::assembly::Assembly;
use crate::battle::Battle;
use crate::fx::{Fx, Owner};
use crate::hero::Hero;
use crate::models::Objects;
use crate::textures::TextureStore;
use crate::{settings, terrain};

/// A turret load-group effect: its name, the three control points it sits on, its id,
/// and the node whose channel value drives it (action 14).
#[derive(Clone, Debug, PartialEq)]
pub struct TurretEffect {
    pub name: String,
    pub points: [usize; 3],
    pub id: i32,
    pub driven_by: Option<usize>,
}

pub struct Play {
    pub hero: Hero,
    pub ground: Ground,
    pub battle: Battle,
    pub assembly: Assembly,
    pub fx: Fx,
    pub materials: Library,
    pub turret_effects: Vec<TurretEffect>,
    /// Mission objects that have died, not yet taken out of the drawing.
    pub killed: Vec<usize>,
    /// Sounds started and not yet played.
    pub cues: Vec<Cue>,
}

/// A round's own frame: y along its flight, z up, x to its side.
fn round_axes(r: &Round) -> [Vec3; 3] {
    let y = r.forward;
    let x = y.cross(Vec3::Z).normalize_or(Vec3::X);
    [x, y, x.cross(y)]
}

/// The wear entry a segment from `p0` to `p1` strikes on `part`: the struck triangle's
/// batch, in its node's level-0 slot, names it by its material byte
/// (`docs/11-effects.md`, "What an explosion plays").
///
/// STAND-IN: docs/11-effects.md#what-an-explosion-plays--read-and-measured -- the node's
/// wear base the material byte is ORed with is not read: the byte alone indexes the wear,
/// as the drawing does.
pub fn struck_wear<'a>(part: &Part, wear: &'a [String], p0: Vec3, p1: Vec3) -> Option<&'a str> {
    let strike = segment_mesh(&part.mesh, &part.nodes, part.scale, p0, p1)?;
    let node = part.mesh.nodes.get(strike.node?)?;
    let slot = part.mesh.slots.get(usize::from(node.slot_index[0]))?;
    let first = usize::from(slot.first_batch);
    let batches = part.mesh.batches.get(first..first + usize::from(slot.batch_count))?;
    let batch = batches.iter().find(|b| {
        let (from, count) = b.triangles();
        (from..from + count).contains(&strike.triangle)
    })?;
    wear.get(usize::from(batch.material & 0xFF)).map(String::as_str)
}

impl Play {
    /// The mission's hero armed, its map's ground, every other object a target, and the
    /// effects they can play loaded.
    pub fn load(game: &Path, mission: &Mission) -> Result<Option<Play>> {
        let mut assembly = Assembly::new(game)?;
        let Some(mut hero) = Hero::load(&mut assembly, mission)? else { return Ok(None) };
        let dir = terrain::map_dir(game, &mission.map_path)?;
        let land = landmesh::load(&gamedir::resolve(&dir, "Land.msh").context("the map has no Land.msh")?)?;
        let mut battle =
            Battle::load(&mut assembly, mission, Some(hero.object), settings::level_ratio(game))?;
        hero.arm(&mut battle, &mut assembly);
        let materials = Library::open(&gamedir::resolve(game, "Material.lib").context("no Material.lib")?)?;

        let mut fx = Fx::open(game)?;
        let load = hero.turret_controller.group(ENTRY_LOAD);
        let mut turret_effects: Vec<TurretEffect> = load
            .iter()
            .filter(|r| r.action() == ACT_EFFECT_POINTS && !r.resource.member.is_empty())
            .map(|r| TurretEffect {
                name: r.resource.member.clone(),
                points: [4, 5, 6].map(|k| usize::try_from(r.values[k]).unwrap_or(0)),
                id: r.values[7],
                driven_by: None,
            })
            .collect();
        for r in load.iter().filter(|r| r.action() == ACT_EFFECT_TIME_POINT) {
            if let Some(e) = turret_effects.iter_mut().find(|e| e.id == r.values[4]) {
                e.driven_by = usize::try_from(r.values[5]).ok();
            }
        }
        for e in &turret_effects {
            fx.template(&e.name);
        }
        for k in &battle.kinds {
            for (name, _) in &k.effects {
                fx.template(name);
            }
        }
        for kind in &battle.combat.kinds {
            for e in kind.hit.iter().chain(kind.range_end.iter()) {
                fx.preload_explosion(e);
            }
        }
        for e in battle.explosions.iter().flatten().flatten().flatten() {
            fx.preload_explosion(e);
        }
        let mut play = Play {
            hero,
            ground: Ground::new(land),
            battle,
            assembly,
            fx,
            materials,
            turret_effects,
            killed: Vec::new(),
            cues: Vec::new(),
        };
        for i in 0..play.turret_effects.len() {
            let e = play.turret_effects[i].clone();
            let frame = play.turret_frame(&e);
            play.fx.start(Owner::Turret(e.id), &e.name, frame, 1.0, 0.0, None);
        }
        Ok(Some(play))
    }

    /// Give the rounds models and pooled instances among `objects`, and resolve the
    /// effects' looks in `store`.
    pub fn draw_rounds(&mut self, store: &mut TextureStore, objects: &mut Objects) -> Result<()> {
        self.fx.resolve_looks(store)?;
        self.battle.draw_rounds(&mut self.assembly, store, objects)
    }

    fn turret_frame(&self, e: &TurretEffect) -> Frame {
        let at = |i: usize| self.hero.point(i).unwrap_or((Vec3::ZERO, Vec3::Y));
        Frame::from_points([at(e.points[0]), at(e.points[1]), at(e.points[2])])
    }

    /// The ground's surface id under a strike's face (`Control.dll:0x100114fd`).
    fn surface(&self, face: usize) -> Option<u8> {
        let land = &self.ground.land;
        let name = land.layer1.get(usize::from(land.faces.get(face)?.tex1))?;
        self.materials.get(name).map(|m| m.surface)
    }

    /// The class of the material `round` struck on part `part` of target `target`: an
    /// agent with no outer object answers for its own material, so a strike on a unit
    /// plays the slot for the struck batch's material class (`docs/11-effects.md`, "What
    /// an explosion plays"). Scenery is built by the same agent loader with no outer
    /// object (`docs/22-settings.md`), and answers the same way.
    ///
    /// STAND-IN: docs/11-effects.md#what-an-explosion-plays--read-and-measured -- what a
    /// building, whose `CBuilding` aggregates its agent, answers for its material is not
    /// read: a strike on a building plays slot 0.
    fn struck_class(&self, target: usize, part: usize, round: &Round, point: Vec3) -> Option<u8> {
        if self.battle.placed_kinds.get(target) == Some(&KIND_BUILDING) {
            return None;
        }
        let p = self.battle.combat.targets.get(target)?.parts.get(part)?;
        let wear = self.battle.wears.get(target)?.get(part)?;
        let name = struck_wear(p, wear, round.previous, point + round.forward * 0.01)?;
        self.materials.get(name).map(|m| m.surface)
    }

    /// One tick: the hero, then every round that left one of its barrels, then the
    /// battle's frame, then the effects.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2]) -> Vec<Event> {
        let shots = self.hero.tick(dt_ms, mouse, &self.ground);
        let now = self.hero.time_ms;
        for (g, shot) in shots {
            let (Some(Some(kind)), Some(gun)) = (self.hero.rounds.get(g).copied(), self.hero.guns.get(g))
            else {
                continue;
            };
            let Some((muzzle, barrel)) = self.hero.muzzle(gun.barrels[shot.barrel].channel) else { continue };
            // `0x1002a34c`: a gun on a turret aims a mode-0 round at what the sight meets,
            // and with no hit leaves it its barrel's direction. Every hero round is mode 0.
            let aim =
                self.hero.sight().and_then(|(o, s)| self.battle.combat.aim_point(&self.ground, None, o, s));
            let direction = aim.map_or(barrel, |p| p - muzzle);
            // The player's guns carry no level ratio: property 180 goes to hostile units only.
            let velocity: Vec3 = self.hero.world_velocity();
            if let Some(id) = self.battle.combat.fire(kind, None, muzzle, direction, velocity, 1.0, None) {
                // The round's load group creates its flight effects at spawn.
                let round = *self.battle.combat.rounds.last().expect("just fired");
                for (name, points) in self.battle.kinds[kind].effects.clone() {
                    let frame = self.round_frame(&round, points);
                    self.fx.start(Owner::Round(id), &name, frame, 1.0, now, None);
                }
            }
        }
        let events = self.battle.combat.tick((dt_ms / 1000.0) as f32, &self.ground);
        for e in &events {
            self.effects_for(e, now);
        }
        // Turret effects follow their points and the channels that drive them.
        for i in 0..self.turret_effects.len() {
            let e = self.turret_effects[i].clone();
            let frame = self.turret_frame(&e);
            let value = e.driven_by.and_then(|node| {
                let c = self.hero.rig.channels.iter().position(|c| c.node == node as i32)?;
                self.hero.rig.values.get(c).copied()
            });
            for instance in self.fx.owned(Owner::Turret(e.id)) {
                instance.frame = frame;
                instance.value = value.unwrap_or(0.0);
            }
        }
        // Flight effects follow their rounds, and go with them. Time modes 5–15 read the
        // round's speed over its top speed: the plasma bolt's and the missile's trails.
        let rounds: Vec<Round> = self.battle.combat.rounds.clone();
        for r in &rounds {
            let kind = &self.battle.kinds[r.kind];
            let frames: Vec<Frame> = kind.effects.iter().map(|(_, p)| self.round_frame(r, *p)).collect();
            let top = self.battle.combat.kinds[r.kind].top_speed;
            let speed = if top > 0.0 { r.velocity.length() / top } else { 0.0 };
            for (instance, frame) in self.fx.owned(Owner::Round(r.id)).zip(frames) {
                instance.frame = frame;
                instance.speed = speed;
            }
        }
        self.fx.instances.retain(|(o, _)| match o {
            Owner::Round(id) => rounds.iter().any(|r| r.id == *id),
            _ => true,
        });
        let cues = self.fx.cues(now);
        self.cues.extend(cues);
        self.fx.tick(now);
        for e in &events {
            if let Event::Killed { target } = e {
                self.killed.push(self.battle.objects[*target]);
            }
        }
        events
    }

    /// A round's control points in the world, as action 4's frame.
    fn round_frame(&self, r: &Round, points: [usize; 3]) -> Frame {
        let axes = round_axes(r);
        let world = |v: [f32; 3]| axes[0] * v[0] + axes[1] * v[1] + axes[2] * v[2];
        let at = |i: usize| {
            self.battle.kinds[r.kind]
                .points
                .get(i)
                .map_or((r.position, Vec3::Y), |p| (r.position + world(p.position), world(p.direction)))
        };
        Frame::from_points([at(points[0]), at(points[1]), at(points[2])])
    }

    /// What an event plays: a round's explosion where it struck or ran out, and a
    /// destroyed node's own.
    fn effects_for(&mut self, e: &Event, now: f64) {
        match e {
            Event::Struck { round, target, part, point, .. } => {
                let kind = &self.battle.combat.kinds[round.kind];
                let Some(exp) = kind.hit.clone() else { return };
                let (surface, axis) = match target {
                    None => {
                        let face = self.ground.segment(round.previous, *point + round.forward * 0.01);
                        let surface = face.and_then(|s| self.surface(s.triangle));
                        let normal = face
                            .map_or(Vec3::Z, |s| Vec3::from_array(self.ground.land.faces[s.triangle].normal));
                        (surface, normal)
                    }
                    Some(t) => (self.struck_class(*t, *part, round, *point), -round.forward),
                };
                // Placement 7 turns the effect to the struck face; a round's `.exp` radius is its size.
                self.fx.explode(&exp, surface, Frame::along(*point, axis, 1.0), exp.radius, now);
            }
            Event::Exploded { kind, point, forward, at_range: true } => {
                // Placement 0 on a round: along its second axis, the way it flies (docs/29).
                if let Some(exp) = self.battle.combat.kinds[*kind].range_end.clone() {
                    self.fx.explode(&exp, None, Frame::along(*point, *forward, 1.0), exp.radius, now);
                }
            }
            Event::Damaged { target, part, destroyed, .. } => {
                for &n in destroyed {
                    let Some(Some(exp)) = self
                        .battle
                        .explosions
                        .get(*target)
                        .and_then(|p| p.get(*part))
                        .and_then(|p| p.get(n))
                        .cloned()
                    else {
                        continue;
                    };
                    let p = &self.battle.combat.targets[*target].parts[*part];
                    let Some(slot) = p.mesh.slots.get(usize::from(p.mesh.nodes[n].slot_index[0])) else {
                        continue;
                    };
                    let [cx, cy, cz, r] = slot.sphere;
                    let c = p.nodes[n].apply([cx, cy, cz].map(|v| f64::from(v * p.scale)));
                    let centre = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                    let y = parkan_formats::pose::rotate(p.nodes[n].rotation, [0.0, 1.0, 0.0]);
                    let axis = Vec3::new(y[0] as f32, y[1] as f32, y[2] as f32);
                    // Placement 0 is the node's second axis; a node's size is the radius × its sphere's.
                    self.fx.explode(
                        &exp,
                        None,
                        Frame::along(centre, axis, 1.0),
                        exp.radius * r * p.scale,
                        now,
                    );
                }
            }
            _ => {}
        }
    }

    /// Every effect sprite at the current time, as seen from `eye`: an instance that tests
    /// its point casts a ray to it from the eye, and nothing struck is in view
    /// (`docs/11-effects.md`, "Bit 8 and the tested point").
    ///
    /// STAND-IN: docs/11-effects.md#bit-8-and-the-tested-point--read-and-measured -- how
    /// often an instance tests its point, and what the ray through the world meets, are
    /// not read: every frame, against what a round meets (the ground less its water
    /// surface, and every live object's level-0 mesh).
    pub fn sprites(&self, eye: Vec3) -> Vec<(usize, Sprite)> {
        self.fx.sprites(self.hero.time_ms, |point| {
            self.battle.combat.first_hit(&self.ground, None, eye, point, 0.0).is_none()
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use parkan_formats::mesh::{Batch, Mesh, NO_SLOT, Node, Slot};
    use parkan_formats::pose::IDENTITY;

    use super::*;

    /// One node facing -y at y = 5: a left triangle in batch 0 and a right one in batch 1,
    /// whose material byte is 2 with the high byte set.
    fn panel() -> Part {
        let batch = |material: u16, first: u16| Batch {
            material,
            flag: 0,
            first_index: first * 3,
            index_count: 3,
            first_vertex: 0,
            vertex_count: 4,
        };
        let mesh = Mesh {
            name: "panel".into(),
            positions: vec![[-2.0, 5.0, -1.0], [0.0, 5.0, -1.0], [0.0, 5.0, 1.0], [2.0, 5.0, -1.0]],
            normals: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            triangles: vec![[0, 1, 2], [1, 3, 2]],
            nodes: vec![Node {
                name: "panel".into(),
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
                batch_count: 2,
                aabb_min: [0.0; 3],
                aabb_max: [0.0; 3],
                sphere: [0.0; 4],
                area: 0.0,
                volume: 0.0,
            }],
            batches: vec![batch(0, 0), batch(0xFF02, 1)],
            face_flags: vec![0, 0],
            face_normals: vec![[0.0, -1.0, 0.0]; 2],
            keys: Vec::new(),
            frame_map: Vec::new(),
            frame_count: 0,
            sphere: None,
        };
        Part { mesh: Rc::new(mesh), nodes: vec![IDENTITY], scale: 1.0, life: None }
    }

    #[test]
    fn a_strike_names_the_wear_entry_of_the_batch_it_struck() {
        let wear: Vec<String> = ["B_SKIN", "B_GLASS", "R_H_01"].map(String::from).into();
        let part = panel();
        let at = |x: f32| struck_wear(&part, &wear, Vec3::new(x, 0.0, -0.5), Vec3::new(x, 10.0, -0.5));
        assert_eq!(at(-1.0), Some("B_SKIN"));
        assert_eq!(at(1.0), Some("R_H_01"), "the material byte's low byte indexes the wear");
        assert_eq!(at(5.0), None, "a miss");
    }
}
