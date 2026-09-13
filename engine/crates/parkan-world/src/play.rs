//! A mission played: the hero, the ground it walks on, and the battle around it.

use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::mission::Mission;
use parkan_formats::{gamedir, landmesh};
use parkan_sim::combat::Event;
use parkan_sim::ground::Ground;

use crate::assembly::Assembly;
use crate::battle::Battle;
use crate::hero::Hero;
use crate::models::Objects;
use crate::textures::TextureStore;
use crate::{settings, terrain};

pub struct Play {
    pub hero: Hero,
    pub ground: Ground,
    pub battle: Battle,
    pub assembly: Assembly,
    /// Mission objects that have died, not yet taken out of the drawing.
    pub killed: Vec<usize>,
}

impl Play {
    /// The mission's hero armed, its map's ground, and every other object a target.
    pub fn load(game: &Path, mission: &Mission) -> Result<Option<Play>> {
        let mut assembly = Assembly::new(game)?;
        let Some(mut hero) = Hero::load(&mut assembly, mission)? else { return Ok(None) };
        let dir = terrain::map_dir(game, &mission.map_path)?;
        let land = landmesh::load(&gamedir::resolve(&dir, "Land.msh").context("the map has no Land.msh")?)?;
        let mut battle =
            Battle::load(&mut assembly, mission, Some(hero.object), settings::level_ratio(game))?;
        hero.arm(&mut battle, &mut assembly);
        Ok(Some(Play { hero, ground: Ground::new(land), battle, assembly, killed: Vec::new() }))
    }

    /// Give the rounds models and pooled instances among `objects`.
    pub fn draw_rounds(&mut self, store: &mut TextureStore, objects: &mut Objects) -> Result<()> {
        self.battle.draw_rounds(&mut self.assembly, store, objects)
    }

    /// One tick: the hero, then every round that left one of its barrels, then the
    /// battle's frame.
    pub fn tick(&mut self, dt_ms: f64, mouse: [f32; 2]) -> Vec<Event> {
        for (g, shot) in self.hero.tick(dt_ms, mouse, &self.ground) {
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
            self.battle.combat.fire(kind, None, muzzle, direction, velocity, 1.0);
        }
        let events = self.battle.combat.tick((dt_ms / 1000.0) as f32, &self.ground);
        for e in &events {
            if let Event::Killed { target } = e {
                self.killed.push(self.battle.objects[*target]);
            }
        }
        events
    }
}
