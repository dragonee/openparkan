//! A mission played: the hero, the ground it walks on, the battle around it and
//! the effects it plays.

use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::control::{ACT_EFFECT_POINTS, ACT_EFFECT_TIME_POINT, ENTRY_LOAD};
use parkan_formats::materials::Library;
use parkan_formats::mission::Mission;
use parkan_formats::{gamedir, landmesh};
use parkan_sim::combat::{Event, Round};
use parkan_sim::effects::{Frame, Sprite};
use parkan_sim::ground::Ground;

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
}

/// A round's own frame: y along its flight, z up, x to its side.
fn round_axes(r: &Round) -> [Vec3; 3] {
    let y = r.forward;
    let x = y.cross(Vec3::Z).normalize_or(Vec3::X);
    [x, y, x.cross(y)]
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
            if let Some(id) = self.battle.combat.fire(kind, None, muzzle, direction, velocity, 1.0) {
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
        // Flight effects follow their rounds, and go with them.
        let rounds: Vec<Round> = self.battle.combat.rounds.clone();
        for r in &rounds {
            let kind = &self.battle.kinds[r.kind];
            let frames: Vec<Frame> = kind.effects.iter().map(|(_, p)| self.round_frame(r, *p)).collect();
            for (instance, frame) in self.fx.owned(Owner::Round(r.id)).zip(frames) {
                instance.frame = frame;
            }
        }
        self.fx.instances.retain(|(o, _)| match o {
            Owner::Round(id) => rounds.iter().any(|r| r.id == *id),
            _ => true,
        });
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
            Event::Struck { round, target, point, .. } => {
                let kind = &self.battle.combat.kinds[round.kind];
                let Some(exp) = kind.hit.clone() else { return };
                // STAND-IN: docs/11-effects.md#not-resolved -- whether a unit answers for
                // its material is not read: a strike on a unit plays slot 0.
                let (surface, axis) = match target {
                    None => {
                        let face = self.ground.segment(round.previous, *point + round.forward * 0.01);
                        let surface = face.and_then(|s| self.surface(s.triangle));
                        let normal = face
                            .map_or(Vec3::Z, |s| Vec3::from_array(self.ground.land.faces[s.triangle].normal));
                        (surface, normal)
                    }
                    Some(_) => (None, -round.forward),
                };
                // Placement 7 turns the effect to the struck face; a round's `.exp` radius is its size.
                self.fx.explode(&exp, surface, Frame::along(*point, axis, 1.0), exp.radius, now);
            }
            Event::Exploded { kind, point, at_range: true } => {
                if let Some(exp) = self.battle.combat.kinds[*kind].range_end.clone() {
                    self.fx.explode(&exp, None, Frame::along(*point, Vec3::Z, 1.0), exp.radius, now);
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

    /// Every effect sprite at the current time.
    pub fn sprites(&self) -> Vec<(usize, Sprite)> {
        self.fx.sprites(self.hero.time_ms)
    }
}
