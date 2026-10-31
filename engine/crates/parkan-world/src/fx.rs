//! The effects a mission plays: templates from `effects.rlb`, and instances on
//! barrels, rounds and explosions. See `docs/11-effects.md`.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use glam::{Mat4, Vec3, Vec4};
use parkan_formats::exp::Explosion;
use parkan_formats::fxid::{self, Effect};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_sim::effects::{
    Arm, Cue, Frame, Instance, LEVEL_HIDDEN, Lens, PointLight, Rng, Sprite, debris_material,
};

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
    /// One of a fitted gun's load-group effects: the target, the gun, and its record's id.
    Gun(usize, usize, i32),
    /// An effect a fitted gun's shot group created as a stroke started: the target and the gun.
    Shot(usize, usize),
    /// A mineral lode's plume, by the lode's index.
    Lode(usize),
    /// One of the three instances of a target's shield effect a hit plays (`0x10025ca0`).
    Shield(usize, usize),
    /// The lightning object's one instance of its effect (`Terrain.dll:0x10071a8d`).
    Lightning,
}

impl Owner {
    /// The target the instance hangs on, where it hangs on one: a building's or a tree's own
    /// load group, and the guns and the shield fitted to a target, which are its agent's.
    pub fn target(self) -> Option<usize> {
        match self {
            Owner::Building(t, _) | Owner::Gun(t, _, _) | Owner::Shot(t, _) | Owner::Shield(t, _) => Some(t),
            _ => None,
        }
    }
}

/// The camera's six planes in the world, each facing in, its length 1 (the camera's `+0x1f0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Planes(pub [Vec4; 6]);

impl Planes {
    /// The six of a view-projection whose depth runs 0 to 1, as the renderer's does.
    pub fn of(view_proj: Mat4) -> Planes {
        let (x, y, z, w) = (view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3));
        Planes([w + x, w - x, w + y, w - y, z, w - z].map(|p| p / p.truncate().length().max(f32::EPSILON)))
    }

    /// Whether a sphere is inside all six: nothing of it may lie at or past its own radius on
    /// the outer side of any (`AniMesh.dll:0x10014c3d`–`0x10014c70`).
    pub fn hold(&self, centre: Vec3, radius: f32) -> bool {
        self.0.iter().all(|p| p.truncate().dot(centre) + p.w > -radius)
    }
}

/// Whether `instance` draws, its owner seen or not. Only the instance's draw waits for the pass
/// its header's `0x800` asks for (`Effect.dll:0x10007d44`): an owner that is not seen hands its
/// manager a pass of 0 and the instance draws nothing. Its emitters are updated all the same,
/// by the manager's tick and every 100 ms, and a light and a sound do all their work there --
/// their draw slot is a bare `ret` (`0x100026f0`) -- so an unseen owner's lights stay lit and
/// its loops go on being played and moved, whatever the flag says (docs/11, "Which draw reaches
/// a unit's own manager"). Those are [`Fx`]'s other outputs, which do not ask this.
pub fn draws(instance: &Instance, owner_seen: bool) -> bool {
    instance.effect.header.flags & fxid::FX_PASS_ONLY == 0 || owner_seen
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

#[derive(Clone)]
pub struct Fx {
    archive: Archive,
    templates: HashMap<String, Option<Rc<Effect>>>,
    pub instances: Vec<(Owner, Instance)>,
    /// The manager's own state of the game's generator, and the seed it last handed out.
    rng: Rng,
    seed: u32,
    /// The number the next instance takes, and the loops its removals stopped.
    next_id: u64,
    stops: Vec<Cue>,
    look_of: HashMap<String, MaterialLooks>,
    pub looks: Vec<Look>,
    /// The shade's light template: material 0 of `system.rlb`'s `shade.wea` (`Terrain.dll:
    /// 0x100420c9`, `0x1004690b`), and its look once resolved.
    pub light_template: Option<String>,
    pub light_look: Option<usize>,
}

/// The wear the shade's light template is material 0 of (`Terrain.dll:0x100420c9`).
pub const SHADE_ARCHIVE: &str = "system.rlb";
pub const SHADE_WEAR: &str = "shade.wea";

/// A material's looks for sprites: one per key of its track 0, in the track's own order, and
/// the track itself, which says which of them a sprite of a given age draws with. A material
/// with no track has the one look its first entry gives.
#[derive(Clone)]
struct MaterialLooks {
    keys: Vec<usize>,
    animation: Option<Animation>,
}

/// The name of `shade.wea`'s first material, where the install has one.
fn light_template(game: &Path) -> Option<String> {
    let path = gamedir::resolve(game, SHADE_ARCHIVE)?;
    let archive = Archive::open(&path).ok()?;
    let entry = archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case(SHADE_WEAR))?;
    let wear = parkan_formats::wea::parse(archive.read(entry).ok()?);
    wear.materials.first().cloned()
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
            rng: Rng::new(0x5EED_1234),
            seed: 1,
            next_id: 1,
            stops: Vec::new(),
            look_of: HashMap::new(),
            looks: Vec::new(),
            light_template: light_template(game),
            light_look: None,
        })
    }

    /// The point lights every instance drives at `now_ms`.
    pub fn lights(&self, now_ms: f64) -> Vec<PointLight> {
        self.owned_lights(now_ms).into_iter().map(|(_, light)| light).collect()
    }

    /// [`Self::lights`], each with what its instance hangs on: the agent whose light manager
    /// holds it, which an owner-only light lights alone.
    pub fn owned_lights(&self, now_ms: f64) -> Vec<(Owner, PointLight)> {
        let mut out = Vec::new();
        let mut buffer = Vec::new();
        for (owner, instance) in &self.instances {
            buffer.clear();
            instance.lights(now_ms, &mut buffer);
            out.extend(buffer.drain(..).map(|light| (*owner, light)));
        }
        out
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
        // STAND-IN: docs/11-effects.md#the-generator--read-and-measured -- the game draws
        // from thirteen states shared by the whole of Effect.dll, seeded from the clock as
        // it loads, so what a particle gets depends on every draw every other effect in
        // the process has already made. Here the manager keeps one state of that same
        // generator and hands each instance a seed of its own, so a particle redrawn next
        // frame gets the number it had.
        self.seed = u32::from(self.rng.next16()) << 16 | u32::from(self.rng.next16());
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

    /// The look `material`'s first key draws with, for a draw that is no effect's: the rain's
    /// and the snow's (docs/10, "The weather").
    pub fn material_look(&mut self, store: &mut TextureStore, material: &str) -> Result<usize> {
        let look = store.look(material)?;
        let p = look.still;
        Ok(self.look_index(Look {
            texture: p.texture,
            blend_mode: look.blend_mode,
            ambient: p.ambient,
            cell: p.cell,
        }))
    }

    /// Resolve the looks of every loaded effect's materials through `store`, so their
    /// textures upload with the world's. A material's track 0 gives one look per key, which
    /// [`Self::sprites`] picks between by the sprite's own age.
    pub fn resolve_looks(&mut self, store: &mut TextureStore) -> Result<()> {
        if self.light_look.is_none()
            && let Some(name) = self.light_template.clone()
        {
            let look = store.look(&name)?;
            let p = look.still;
            self.light_look = Some(self.look_index(Look {
                texture: p.texture,
                blend_mode: look.blend_mode,
                ambient: p.ambient,
                cell: p.cell,
            }));
        }
        let materials: Vec<String> = self
            .templates
            .values()
            .flatten()
            .flat_map(|t| t.emitters.iter())
            .filter(|e| e.kind != fxid::EMITTER_SOUND && (e.kind == 10 || !e.resource.member.is_empty()))
            .map(|e| if e.kind == 10 { debris_material(e).to_string() } else { e.resource.member.clone() })
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

    /// The look `material` draws at `phase`, where the sprite's own phase stands in its
    /// track: the draw hands that fraction to the material manager's `GetMaterialPhase`
    /// (vtable slot 5, `World3D.dll:0x10003680`), which multiplies it by the track's whole
    /// length and takes the keys that bracket it (docs/11, "A phase is where its material's
    /// animation stands"). So a sprite runs its animation over its **window**, not at the
    /// track's own key rate, and a phase that runs past 1 plays it over again.
    /// `smoke_fr_02`'s puff leaves a chimney on `fire_smoke`'s early, orange cells and is
    /// on its later, black ones as it ages, as the recording's plumes are.
    ///
    /// STAND-IN: docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured --
    /// that the effect draw takes the entry's cell is not read. The masked colour lerp
    /// between two keys is not drawn: a sprite takes the key it is in whole.
    fn sprite_look(&self, material: &str, phase: f32) -> Option<usize> {
        let m = self.look_of.get(&key(material))?;
        let k = m.animation.as_ref().map_or(0, |a| a.key_at(a.time_of(phase)).0);
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

    /// Put every instance on the arm of the shade's level its owner's class takes
    /// (docs/11, "An instance's level"): `building` says whether an owner is a building,
    /// class 3; everything else an effect hangs on -- a unit, a round, scenery, the landscape
    /// -- takes the robot's arm.
    pub fn set_arms(&mut self, building: impl Fn(Owner) -> bool) {
        for (owner, instance) in &mut self.instances {
            instance.arm = if building(*owner) { Arm::Building } else { Arm::Robot };
        }
    }

    /// [`Fx::sprites`] through `lens`: each instance at the level the shade answers for its
    /// sphere seen from `eye` (`Effect.dll:0x10007e6d`, `Terrain.dll:0x10047fe0`), which the
    /// manager's draw asks afresh each time. At 8 or over nothing of it is drawn; under that
    /// the level thins its bursts and streams and picks its domes' detail.
    ///
    /// `owner_seen` is the pass an instance's owner hands its manager ([`draws`]).
    ///
    /// STAND-IN: docs/11-effects.md#which-draw-reaches-a-units-own-manager--read-and-measured
    /// -- the pass is the sphere against the camera's six planes alone, where the game asks the
    /// object's cell too, and an object in no cell the landscape's draw walks is not drawn at
    /// all, its effects without `0x800` with it. A sphere inside the six planes stands in a cell
    /// whose box is not wholly outside any of them, so the first goes the same way but at a
    /// frustum's corners; the second is a chimney's smoke rising into view from out of sight.
    pub fn sprites_seen(
        &self,
        now_ms: f64,
        eye: Vec3,
        lens: Lens,
        in_view: impl Fn(Vec3) -> bool,
        owner_seen: impl Fn(Owner, &Instance) -> bool,
    ) -> Vec<(usize, Sprite)> {
        let mut out = Vec::new();
        let mut buffer = Vec::new();
        for (owner, instance) in &self.instances {
            let level = instance.level(now_ms, eye, lens);
            if level >= LEVEL_HIDDEN || !draws(instance, owner_seen(*owner, instance)) {
                continue;
            }
            buffer.clear();
            let seen = instance.test_point().is_none_or(&in_view);
            instance.sprites_at(now_ms, Some(eye), seen, level, &mut buffer);
            out.extend(buffer.drain(..).filter_map(|s| Some((self.sprite_look(&s.material, s.phase)?, s))));
        }
        out
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
    /// The game updates an effect that is being drawn on every pass -- its manager's draw and
    /// its own each open the 100 ms gate behind them -- as every instance here updates on
    /// every tick the caller runs (docs/11, "Who hands a manager its time").
    ///
    /// STAND-IN: docs/11-effects.md#who-hands-a-manager-its-time--read-and-seen -- an effect
    /// that is not being drawn updates once more than 100 ms have passed since its last
    /// update (`Effect.dll:0x10003e5a`, `0x100081a7`); here it too updates on every tick.
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
    pub fn sprites(&self, now_ms: f64, eye: Vec3, in_view: impl Fn(Vec3) -> bool) -> Vec<(usize, Sprite)> {
        let mut out = Vec::new();
        let mut buffer = Vec::new();
        for (_, instance) in &self.instances {
            buffer.clear();
            instance.sprites_from(now_ms, Some(eye), instance.test_point().is_none_or(&in_view), &mut buffer);
            out.extend(buffer.drain(..).filter_map(|s| Some((self.sprite_look(&s.material, s.phase)?, s))));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkan_formats::fxid::{Emitter, Header};
    use parkan_formats::objects::ResourceRef;

    fn instance(flags: u32) -> Instance {
        // A light: its window 0 to 1 at +8 and +12, and the rest at its zeros.
        let mut body = vec![0u8; 256];
        body[0] = 1;
        body[4] = 1;
        body[12..16].copy_from_slice(&1.0f32.to_le_bytes());
        let light = Emitter {
            kind: 1,
            word: 1,
            resource: ResourceRef { library: String::new(), member: String::new() },
            body,
        };
        let effect = Effect {
            name: "t".into(),
            header: Header {
                count: 1,
                mode: 1,
                duration: 1.0,
                jitter: 0.0,
                flags,
                gate: 0,
                offset: [0.0; 3],
                point: [0.0; 3],
                scale: [1.0; 3],
            },
            emitters: vec![light],
        };
        Instance::new(Rc::new(effect), Frame::along(Vec3::ZERO, Vec3::X, 1.0), 1.0, 0.0, None, 1)
    }

    #[test]
    fn an_unseen_owner_draws_none_of_its_0x800_effects_and_its_others_draw_as_ever() {
        let (waits, plain) = (instance(fxid::FX_PASS_ONLY), instance(0));
        assert!(draws(&waits, true) && !draws(&waits, false));
        assert!(draws(&plain, true) && draws(&plain, false));
    }

    #[test]
    fn a_sphere_is_seen_while_any_of_it_is_inside_all_six_planes() {
        // Looking along +x from the origin, a quarter turn wide, out to 100.
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::X, Vec3::Z);
        let projection = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.5, 100.0);
        let planes = Planes::of(projection * view);
        assert!(planes.hold(Vec3::new(10.0, 0.0, 0.0), 0.0), "straight ahead");
        assert!(!planes.hold(Vec3::new(-10.0, 0.0, 0.0), 1.0), "behind the eye");
        assert!(!planes.hold(Vec3::new(10.0, 20.0, 0.0), 1.0), "off to the side");
        assert!(planes.hold(Vec3::new(10.0, 20.0, 0.0), 8.0), "off to the side, but it reaches in");
        assert!(!planes.hold(Vec3::new(200.0, 0.0, 0.0), 1.0), "past the far plane");
        assert!(planes.hold(Vec3::new(-0.1, 0.0, 0.0), 1.0), "the eye inside its own unit's sphere");
    }
}
