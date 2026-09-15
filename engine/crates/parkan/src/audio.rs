//! Sound: the effects' cues played from `sounds.lib` through kira, the voices the game
//! queues, and a mission's ambient theme.

use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use glam::Vec3;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning, Tween};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_sim::effects::{Cue, CueKind};
use parkan_world::resources::Sound;

pub struct Audio {
    manager: AudioManager<DefaultBackend>,
    archive: Archive,
    sounds: HashMap<String, Option<StaticSoundData>>,
    /// Sounds from any archive, by library and member.
    named: HashMap<(std::path::PathBuf, String), Option<StaticSoundData>>,
    /// The voice queue (`ISoundServer` slot 4): what waits, and what plays.
    voices: VecDeque<StaticSoundData>,
    voice: Option<StaticSoundHandle>,
    /// The effects' loops playing, by instance and emitter.
    loops: HashMap<(u64, usize), StaticSoundHandle>,
    /// The last one-shot each emitter played, which it stops before playing again.
    onces: HashMap<(u64, usize), StaticSoundHandle>,
}

/// How loud a cue is heard `distance` away, as Direct3D Sound hears a buffer whose
/// minimum distance is `near` and maximum `far` (docs/11, "How a sound is heard"): whole
/// within `near`, then `near ÷ (near + R × (d − near))` with the listener's rolloff R of 1,
/// no quieter past `far`.
pub fn gain(distance: f32, near: f32, far: f32) -> f32 {
    if distance <= near {
        1.0
    } else {
        let d = distance.min(far.max(near));
        if d <= 0.0 { 0.0 } else { near.max(0.0) / (near.max(0.0) + ROLLOFF * (d - near.max(0.0))) }
    }
}

/// The listener's rolloff factor, which nothing sets (`Ngi32.dll:0x1000d123`).
const ROLLOFF: f32 = 1.0;

/// How long a playing sound takes to reach its new gain and pan.
const MOVE_TWEEN: Duration = Duration::from_millis(30);

impl Audio {
    /// The audio device and `sounds.lib`, or `None` when either is missing.
    pub fn open(game: &Path) -> Option<Audio> {
        let archive = gamedir::resolve(game, "sounds.lib").and_then(|p| Archive::open(&p).ok())?;
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(manager) => Some(Audio {
                manager,
                archive,
                sounds: HashMap::new(),
                named: HashMap::new(),
                voices: VecDeque::new(),
                voice: None,
                loops: HashMap::new(),
                onces: HashMap::new(),
            }),
            Err(e) => {
                eprintln!("no sound: {e}");
                None
            }
        }
    }

    fn sound(&mut self, name: &str) -> Option<StaticSoundData> {
        let key = name.to_ascii_lowercase();
        if let Some(found) = self.sounds.get(&key) {
            return found.clone();
        }
        let loaded = self
            .archive
            .entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .and_then(|e| self.archive.read(e).ok())
            .and_then(|bytes| StaticSoundData::from_cursor(Cursor::new(bytes.to_vec())).ok());
        self.sounds.insert(key, loaded.clone());
        loaded
    }

    fn named(&mut self, sound: &Sound) -> Option<StaticSoundData> {
        let key = (sound.library.clone(), sound.member.to_ascii_lowercase());
        if let Some(found) = self.named.get(&key) {
            return found.clone();
        }
        let loaded =
            sound.read().ok().and_then(|bytes| StaticSoundData::from_cursor(Cursor::new(bytes)).ok());
        self.named.insert(key, loaded.clone());
        loaded
    }

    /// A voice, queued behind the ones playing: it starts at once only when none waits
    /// (`services.dll:0x10011ab0`), and each starts once the one before it has stopped
    /// (`0x100119e0`).
    pub fn queue(&mut self, sound: &Sound) {
        if let Some(data) = self.named(sound) {
            self.voices.push_back(data);
            self.update();
        }
    }

    /// A sound played at once, past the queue (`0x10011bb0`).
    pub fn play_now(&mut self, sound: &Sound) {
        if let Some(data) = self.named(sound) {
            let _ = self.manager.play(data);
        }
    }

    /// A mission's theme, from its load on.
    ///
    /// STAND-IN: docs/34-progression.md#ambient-sound--read-in-part -- whether a type-5
    /// descriptor makes its sound loop is not read, nor when the variations play: the theme
    /// loops, and the variations are not played.
    pub fn theme(&mut self, sound: &Sound) {
        if let Some(data) = self.named(sound) {
            let _ = self.manager.play(data.loop_region(..));
        }
    }

    /// Start the next queued voice once the one playing has stopped.
    pub fn update(&mut self) {
        let done = self.voice.as_ref().is_none_or(|h| h.state() == PlaybackState::Stopped);
        if done && let Some(next) = self.voices.pop_front() {
            self.voice = self.manager.play(next).ok();
        }
    }

    /// Play `cue` as heard at `eye`, whose right is `right`.
    ///
    /// STAND-IN: docs/11-effects.md#not-resolved -- how Direct3D Sound places a sound
    /// between the speakers is not read; it pans by its direction.
    pub fn play(&mut self, cue: &Cue, eye: Vec3, right: Vec3) {
        let offset = cue.position - eye;
        let distance = offset.length();
        let volume = Decibels((20.0 * gain(distance, cue.near, cue.far).log10()).max(Decibels::SILENCE.0));
        let pan = Panning(offset.normalize_or_zero().dot(right.normalize_or_zero()).clamp(-1.0, 1.0));
        match cue.kind {
            CueKind::Stop => {
                if let Some(mut handle) = self.loops.remove(&cue.key) {
                    handle.stop(Default::default());
                }
            }
            // A playing sound follows its emitter each update; a one-shot past its far
            // distance is stopped (`Ngi32.dll:0x1000e51b`), a loop plays on.
            CueKind::Move => {
                let tween = Tween { duration: MOVE_TWEEN, ..Default::default() };
                if let Some(handle) = self.loops.get_mut(&cue.key) {
                    handle.set_volume(volume, tween);
                    handle.set_panning(pan, tween);
                }
                let done = self.onces.get(&cue.key).is_some_and(|h| h.state() == PlaybackState::Stopped);
                if done {
                    self.onces.remove(&cue.key);
                } else if let Some(handle) = self.onces.get_mut(&cue.key) {
                    if distance > cue.far {
                        handle.stop(Default::default());
                        self.onces.remove(&cue.key);
                    } else {
                        handle.set_volume(volume, tween);
                        handle.set_panning(pan, tween);
                    }
                }
            }
            CueKind::Loop => {
                let Some(sound) = self.sound(&cue.sound) else { return };
                if let Ok(handle) = self.manager.play(sound.volume(volume).panning(pan).loop_region(..)) {
                    self.loops.insert(cue.key, handle);
                }
            }
            CueKind::Once => {
                if let Some(mut old) = self.onces.remove(&cue.key) {
                    old.stop(Default::default());
                }
                if distance > cue.far {
                    return;
                }
                let Some(sound) = self.sound(&cue.sound) else { return };
                if let Ok(handle) = self.manager.play(sound.volume(volume).panning(pan)) {
                    self.onces.insert(cue.key, handle);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs the game install"]
    fn every_sound_in_sounds_lib_decodes() {
        let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
        let archive = Archive::open(&gamedir::resolve(&game, "sounds.lib").unwrap()).unwrap();
        let bad: Vec<&str> = archive
            .entries
            .iter()
            .filter(|e| StaticSoundData::from_cursor(Cursor::new(archive.read(e).unwrap().to_vec())).is_err())
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(archive.entries.len(), 167);
        assert!(bad.is_empty(), "{bad:?}");
    }

    #[test]
    fn a_cue_is_whole_inside_its_near_distance_and_falls_as_near_over_distance_to_its_far() {
        assert_eq!(gain(5.0, 10.0, 80.0), 1.0);
        assert_eq!(gain(20.0, 10.0, 80.0), 0.5, "6 dB a doubling");
        assert_eq!(gain(40.0, 10.0, 80.0), 0.25);
        assert_eq!(gain(90.0, 10.0, 80.0), 0.125, "no quieter past far");
        assert_eq!(gain(1000.0, 10.0, 80.0), 0.125);
    }
}
