//! The effects a mission plays: templates from `effects.rlb`, and instances on
//! barrels, rounds and explosions. See `docs/11-effects.md`.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::exp::Explosion;
use parkan_formats::fxid::{self, Effect};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_sim::effects::{Cue, Frame, Instance, Sprite};

use crate::textures::TextureStore;

/// The surfaces an `.exp` names a slot for: slot surface + 1 plays on surface 0..10.
pub const SURFACES: u8 = 11;

/// What an instance hangs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// Placed once, in the world.
    World,
    /// A round in flight, by its id.
    Round(u64),
    /// One of the hero turret's load-group effects, by its record's id.
    Turret(i32),
    /// One of the hero chassis's load-group effects on a node, by its record's id.
    Chassis(i32),
    /// One of a building's load-group effects: the building's target, and its record's id.
    Building(usize, i32),
}

/// A material's look for sprites: its texture, the cell of it the entry takes, and its blend
/// mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub texture: Option<usize>,
    pub blend_mode: u8,
    /// `(u0, v0, du, dv)`: a sprite's corner (u, v) samples `u0 + u × du`, `v0 + v × dv`.
    pub cell: [f32; 4],
}

pub struct Fx {
    archive: Archive,
    templates: HashMap<String, Option<Rc<Effect>>>,
    pub instances: Vec<(Owner, Instance)>,
    seed: u32,
    /// The number the next instance takes, and the loops its removals stopped.
    next_id: u64,
    stops: Vec<Cue>,
    look_of: HashMap<String, usize>,
    pub looks: Vec<Look>,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

impl Fx {
    pub fn open(game: &Path) -> Result<Fx> {
        let path = gamedir::resolve(game, "effects.rlb").context("no effects.rlb")?;
        Ok(Fx {
            archive: Archive::open(&path)?,
            templates: HashMap::new(),
            instances: Vec::new(),
            seed: 1,
            next_id: 1,
            stops: Vec::new(),
            look_of: HashMap::new(),
            looks: Vec::new(),
        })
    }

    /// An effect by name, loaded once.
    pub fn template(&mut self, name: &str) -> Option<Rc<Effect>> {
        let k = key(name);
        if let Some(found) = self.templates.get(&k) {
            return found.clone();
        }
        let loaded = self
            .archive
            .entries
            .iter()
            .find(|e| e.tag() == fxid::FXID_TAG && key(&e.name) == k)
            .and_then(|e| fxid::parse(self.archive.read(e).ok()?, &e.name).ok())
            .map(Rc::new);
        self.templates.insert(k, loaded.clone());
        loaded
    }

    /// Start `name` for `owner` on `frame`, `size` times its header's scale, in `mode` or
    /// its own. Returns whether the effect loaded.
    pub fn start(
        &mut self,
        owner: Owner,
        name: &str,
        frame: Frame,
        size: f32,
        now_ms: f64,
        mode: Option<u32>,
    ) -> bool {
        let Some(effect) = self.template(name) else { return false };
        // STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the effect manager's
        // random generator is not read: each instance takes the next seed of a linear
        // congruential sequence.
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let mut instance = Instance::new(effect, frame, size, now_ms, mode, self.seed);
        instance.id = self.next_id;
        self.next_id += 1;
        self.instances.push((owner, instance));
        true
    }

    /// Restart every instance `owner` holds at `now_ms`, in `mode` or its own (action 10).
    pub fn restart(&mut self, owner: Owner, now_ms: f64, mode: Option<u32>) {
        for i in self.owned(owner) {
            let length = i.end_ms - i.start_ms;
            i.start_ms = now_ms;
            i.end_ms = now_ms + length;
            i.mode = mode.unwrap_or(i.effect.header.mode);
        }
    }

    /// Keep the instances `keep` says, silencing the loops of the rest.
    pub fn retain(&mut self, mut keep: impl FnMut(&Owner, &Instance) -> bool) {
        let mut stops = Vec::new();
        self.instances.retain_mut(|(o, i)| {
            let kept = keep(o, i);
            if !kept {
                stops.extend(i.silence());
            }
            kept
        });
        self.stops.extend(stops);
    }

    /// The effect an `.exp` plays on `surface` (`Control.dll:0x100117d0`): slot surface + 1
    /// for surface 0..10, and slot 0 when that is unset or does not load.
    pub fn explode(&mut self, e: &Explosion, surface: Option<u8>, frame: Frame, size: f32, now_ms: f64) {
        let by_surface = surface.filter(|&s| s < SURFACES).and_then(|s| e.slots.get(usize::from(s) + 1));
        if let Some(slot) = by_surface.filter(|r| !r.member.is_empty())
            && self.start(Owner::World, &slot.member.clone(), frame, size, now_ms, None)
        {
            return;
        }
        if let Some(slot) = e.slots.first().filter(|r| !r.member.is_empty()) {
            let name = slot.member.clone();
            self.start(Owner::World, &name, frame, size, now_ms, None);
        }
    }

    /// Load every effect an `.exp` can play.
    pub fn preload_explosion(&mut self, e: &Explosion) {
        for slot in &e.slots {
            if !slot.member.is_empty() {
                self.template(&slot.member);
            }
        }
    }

    /// Resolve the looks of every loaded effect's materials through `store`, so their
    /// textures upload with the world's.
    pub fn resolve_looks(&mut self, store: &mut TextureStore) -> Result<()> {
        let materials: Vec<String> = self
            .templates
            .values()
            .flatten()
            .flat_map(|t| t.emitters.iter())
            .filter(|e| e.kind != fxid::EMITTER_SOUND && !e.resource.member.is_empty())
            .map(|e| e.resource.member.clone())
            .collect();
        for m in materials {
            if self.look_of.contains_key(&key(&m)) {
                continue;
            }
            let look = store.look(&m)?;
            // STAND-IN: docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured
            // -- that an effect sprite takes its material entry's cell as a mesh batch does
            // is not read for the effect draw; its first key's cell is taken, and the keys are
            // not played.
            self.looks.push(Look {
                texture: look.still.texture,
                blend_mode: look.blend_mode,
                cell: look.still.cell,
            });
            self.look_of.insert(key(&m), self.looks.len() - 1);
        }
        Ok(())
    }

    /// Forget every instance `owner` holds.
    pub fn remove(&mut self, owner: Owner) {
        self.retain(|o, _| *o != owner);
    }

    /// Every instance `owner` holds.
    pub fn owned(&mut self, owner: Owner) -> impl Iterator<Item = &mut Instance> {
        self.instances.iter_mut().filter(move |(o, _)| *o == owner).map(|(_, i)| i)
    }

    /// Every sound an instance starts by `now_ms`.
    pub fn cues(&mut self, now_ms: f64) -> Vec<Cue> {
        let mut out = std::mem::take(&mut self.stops);
        out.extend(self.instances.iter_mut().flat_map(|(_, i)| i.cues(now_ms)));
        out
    }

    /// Update every instance at `now_ms`, once their owners have placed them, and drop
    /// the instances that have run their course.
    ///
    /// STAND-IN: docs/11-effects.md#how-an-effect-runs--read -- the manager updates an
    /// instance once 100 ms have passed since its last update; here every instance
    /// updates on every tick the caller runs.
    pub fn tick(&mut self, now_ms: f64) {
        for (_, instance) in &mut self.instances {
            instance.update(now_ms);
        }
        self.retain(|_, i| !i.finished(now_ms));
    }

    /// What every instance draws at `now_ms`, with the index of its material's look.
    /// `in_view` says whether a point is in view from the camera, for the instances
    /// that test one.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- which draw call passes the pass
    /// argument header flag 0x800 waits for is not read: 0x800 effects draw with the rest.
    pub fn sprites(&self, now_ms: f64, in_view: impl Fn(Vec3) -> bool) -> Vec<(usize, Sprite)> {
        let mut out = Vec::new();
        let mut buffer = Vec::new();
        for (_, instance) in &self.instances {
            buffer.clear();
            instance.sprites(now_ms, instance.test_point().is_none_or(&in_view), &mut buffer);
            out.extend(buffer.drain(..).filter_map(|s| Some((*self.look_of.get(&key(&s.material))?, s))));
        }
        out
    }
}
