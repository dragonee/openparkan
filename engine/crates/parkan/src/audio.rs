//! Sound: the effects' cues played from `sounds.lib` through kira.

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use glam::Vec3;
use kira::sound::static_sound::StaticSoundData;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_sim::effects::Cue;

pub struct Audio {
    manager: AudioManager<DefaultBackend>,
    archive: Archive,
    sounds: HashMap<String, Option<StaticSoundData>>,
}

/// How loud a cue is heard `distance` away: whole within `near`, nothing past `far`.
///
/// STAND-IN: docs/11-effects.md#emitter-types--read-and-measured -- a sound's near and
/// far distances are read; the falloff between them is not, and is taken as linear.
pub fn gain(distance: f32, near: f32, far: f32) -> f32 {
    if distance <= near {
        1.0
    } else if distance >= far || far <= near {
        0.0
    } else {
        (far - distance) / (far - near)
    }
}

impl Audio {
    /// The audio device and `sounds.lib`, or `None` when either is missing.
    pub fn open(game: &Path) -> Option<Audio> {
        let archive = gamedir::resolve(game, "sounds.lib").and_then(|p| Archive::open(&p).ok())?;
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(manager) => Some(Audio { manager, archive, sounds: HashMap::new() }),
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

    /// Play `cue` as heard at `eye`, whose right is `right`.
    ///
    /// STAND-IN: docs/11-effects.md#emitter-types--read-and-measured -- how a sound is
    /// placed between the speakers is not read; it pans by its direction.
    pub fn play(&mut self, cue: &Cue, eye: Vec3, right: Vec3) {
        let offset = cue.position - eye;
        let g = gain(offset.length(), cue.near, cue.far);
        if g <= 0.0 {
            return;
        }
        let Some(sound) = self.sound(&cue.sound) else { return };
        let pan = offset.normalize_or_zero().dot(right.normalize_or_zero()).clamp(-1.0, 1.0);
        let volume = Decibels((20.0 * g.log10()).max(Decibels::SILENCE.0));
        let _ = self.manager.play(sound.volume(volume).panning(Panning(pan)));
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
    fn a_cue_is_whole_inside_its_near_distance_and_silent_past_its_far() {
        assert_eq!(gain(5.0, 10.0, 80.0), 1.0);
        assert_eq!(gain(45.0, 10.0, 80.0), 0.5);
        assert_eq!(gain(90.0, 10.0, 80.0), 0.0);
    }
}
