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

use crate::textures::{Animation, Phase, TextureStore};

/// The surfaces an `.exp` names a slot for: slot surface + 1 plays on surface 0..10.
pub const SURFACES: u8 = 11;

/// What an instance hangs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// Placed once, in the world.
    World,
    /// One of a round's load-group effects: the round's id, and the record's id its groups
    /// name it by. It stays while the round is in the world, its flight over or not.
    Round(u64, i32),
    /// One of the hero turret's load-group effects, by its record's id.
    Turret(i32),
    /// One of the hero chassis's load-group effects on a node, by its record's id.
    Chassis(i32),
    /// One of a building's load-group effects: the building's target, and its record's id.
    Building(usize, i32),
    /// A mineral lode's plume, by the lode's index.
    Lode(usize),
    /// One of the three instances of a target's shield effect a hit plays (`0x10025ca0`).
    Shield(usize, usize),
}

/// A material's look for sprites: its texture, the cell of it the entry takes, and its blend
/// mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub texture: Option<usize>,
    pub blend_mode: u8,
    /// The entry's ambient colour, 0..1 as the file gives it: the material's self-light.
    pub ambient: [f32; 3],
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
    look_of: HashMap<String, MaterialLooks>,
    pub looks: Vec<Look>,
}

/// A material's looks for sprites: one per key of its track 0, in the track's own order, and
/// the track itself, which says which of them a sprite of a given age draws with. A material
/// with no track has the one look its first entry gives.
struct MaterialLooks {
    keys: Vec<usize>,
    animation: Option<Animation>,
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

    /// Switch every instance `owner` holds on or off (actions 18 and 19): one switched off
    /// neither updates, draws nor sounds, and its loops stop.
    pub fn switch(&mut self, owner: Owner, on: bool) {
        let mut stops = Vec::new();
        for i in self.owned(owner) {
            if i.on && !on {
                stops.extend(i.silence());
            }
            i.on = on;
        }
        self.stops.extend(stops);
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

    /// The index `look` draws with, kept once: the same texture, cell and colours from two
    /// materials share it, so one draw covers both.
    fn look_index(&mut self, look: Look) -> usize {
        match self.looks.iter().position(|kept| *kept == look) {
            Some(i) => i,
            None => {
                self.looks.push(look);
                self.looks.len() - 1
            }
        }
    }

    /// Resolve the looks of every loaded effect's materials through `store`, so their
    /// textures upload with the world's. A material's track 0 gives one look per key, which
    /// [`Self::sprites`] picks between by the sprite's own age.
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
            let blend_mode = look.blend_mode;
            let phases: Vec<Phase> = match &look.animation {
                Some(a) => a.keys.iter().map(|(phase, _)| *phase).collect(),
                None => vec![look.still],
            };
            let keys = phases
                .into_iter()
                .map(|p| {
                    self.look_index(Look { texture: p.texture, blend_mode, ambient: p.ambient, cell: p.cell })
                })
                .collect();
            self.look_of.insert(key(&m), MaterialLooks { keys, animation: look.animation });
        }
        Ok(())
    }

    /// The look `material` draws with `age_ms` into its own track (docs/07, "Playing a
    /// track"): an effect sprite takes its material entry's texture, cell and colours as a
    /// mesh batch does, and its track steps them as it plays. `smoke_fr_02`'s puff leaves a
    /// chimney on `fire_smoke`'s first five cells, orange, each 50 ms, and is on its later,
    /// black ones a quarter of a second on, as the recording's plumes are.
    ///
    /// STAND-IN: docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured --
    /// that the effect draw takes the entry's cell is not read, nor what a sprite's material
    /// starts from: a stream's particle starts its own track when it leaves, and every other
    /// sprite starts it with its instance. The masked colour lerp between two keys is not
    /// drawn: a sprite takes the key it is in whole.
    fn sprite_look(&self, material: &str, age_ms: f32) -> Option<usize> {
        let m = self.look_of.get(&key(material))?;
        let k = m.animation.as_ref().map_or(0, |a| a.key_at(f64::from(age_ms)).0);
        m.keys.get(k).copied()
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
    /// The pass argument header flag 0x800 waits for is 1 in every call the game makes --
    /// the landscape's and the atmosphere's draws pass it outright, an object's draw passes
    /// it while the object is inside the camera's planes -- so a 0x800 effect draws with
    /// the rest ([11](../../../docs/11-effects.md#who-passes-the-draws-pass-argument--read)).
    pub fn sprites(&self, now_ms: f64, in_view: impl Fn(Vec3) -> bool) -> Vec<(usize, Sprite)> {
        let mut out = Vec::new();
        let mut buffer = Vec::new();
        for (_, instance) in &self.instances {
            buffer.clear();
            instance.sprites(now_ms, instance.test_point().is_none_or(&in_view), &mut buffer);
            out.extend(buffer.drain(..).filter_map(|s| Some((self.sprite_look(&s.material, s.age_ms)?, s))));
        }
        out
    }
}
