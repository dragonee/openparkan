//! A mission as M0 draws it: every placed object as a box, over a grid.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glam::{Vec2, Vec3};
use parkan_formats::{gamedir, mission};
use parkan_render::{Marker, SceneData};

/// A loaded mission and where its hero stands.
pub struct Loaded {
    pub mission: mission::Mission,
    pub scene: SceneData,
    /// The player's hero: position and heading, radians about z.
    pub hero: Option<(Vec3, f32)>,
}

pub fn mission_dir(game: &Path, relative: &str) -> Result<PathBuf> {
    gamedir::resolve(game, relative)
        .with_context(|| format!("no mission {relative} under {}", game.display()))
}

fn colour(object: &mission::Object) -> [f32; 3] {
    match object.kind {
        mission::KIND_BUILDING => [0.75, 0.55, 0.3],
        mission::KIND_UNIT => [0.35, 0.6, 0.9],
        mission::KIND_VEGETATION => [0.3, 0.7, 0.35],
        _ => [0.55, 0.55, 0.55],
    }
}

fn is_hero(object: &mission::Object) -> bool {
    object.path.to_ascii_uppercase().contains("\\HERO\\")
}

pub fn load(game: &Path, relative: &str) -> Result<Loaded> {
    let dir = mission_dir(game, relative)?;
    let tma = gamedir::resolve(&dir, "data.tma").context("the mission has no data.tma")?;
    let data = std::fs::read(&tma)?;
    let mission = mission::parse(&data, &tma.display().to_string())?;

    let mut markers = Vec::new();
    let mut hero = None;
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for object in &mission.objects {
        let position = Vec3::from_array(object.position);
        lo = lo.min(position.truncate());
        hi = hi.max(position.truncate());
        let (half_size, colour) =
            if is_hero(object) { (2.0, [1.0, 0.85, 0.2]) } else { (1.5, colour(object)) };
        if is_hero(object) && hero.is_none() {
            hero = Some((position, object.rotation));
        }
        markers.push(Marker { position, half_size, colour });
    }
    let step = 50.0;
    let scene = if markers.is_empty() {
        SceneData::default()
    } else {
        SceneData {
            markers,
            grid_min: ((lo - Vec2::splat(step)) / step).floor() * step,
            grid_max: ((hi + Vec2::splat(step)) / step).ceil() * step,
            grid_step: step,
        }
    };
    Ok(Loaded { mission, scene, hero })
}
