//! Sound: the effects' cues played from `sounds.lib` through kira, the voices the game
//! queues, a mission's ambient theme, and the music, the CD's tracks as Ogg files.

use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use glam::Vec3;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::sound::{FromFileError, PlaybackState};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Panning, Tween};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_sim::effects::{Cue, CueKind};
use parkan_world::music::Cd;
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
    /// The CD track playing.
    music: Option<StreamingSoundHandle<FromFileError>>,
    /// The rain's background sound, by its name.
    rain: Option<(String, StaticSoundHandle)>,
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
                music: None,
                rain: None,
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

    /// A mission's theme, from the briefing's end on. It loops because its descriptor is
    /// type 5: that binding case hands the sound flag 2 (`services.dll:0x10009ae3`) where
    /// type 4 hands 0, and the play passes bit 1 on as `DSBPLAY_LOOPING`
    /// (`Ngi32.dll:0x1000eb15`). See docs/34, "Ambient sound".
    pub fn theme(&mut self, sound: &Sound) {
        if let Some(data) = self.named(sound) {
            let _ = self.manager.play(data.loop_region(..));
        }
    }

    /// The CD: a track played from its file, or silence. Returns a started track's length in
    /// milliseconds, 0 for one that cannot be played.
    ///
    /// STAND-IN: docs/34-progression.md#music-the-cds-tracks--read-and-measured -- how a
    /// mixer control's share becomes loudness is not read: the music plays at the sounds'
    /// level, as the install's equal `CD_VOLUME` and `SFX_VOLUME` and Mission 01's recording
    /// have it.
    pub fn cd(&mut self, command: &Cd) -> Option<f64> {
        if let Some(mut playing) = self.music.take() {
            playing.stop(Tween::default());
        }
        let Cd::Play { path, .. } = command else { return None };
        // A looping track is played again by the player, 2 s after it ends.
        let started = StreamingSoundData::from_file(path)
            .map_err(|e| eprintln!("no music: {e}"))
            .ok()
            .and_then(|data| {
                let length = data.duration().as_secs_f64() * 1000.0;
                self.manager.play(data).ok().map(|handle| (handle, length))
            });
        match started {
            Some((handle, length)) => {
                self.music = Some(handle);
                Some(length)
            }
            None => Some(0.0),
        }
    }

    /// Start the next queued voice once the one playing has stopped.
    pub fn update(&mut self) {
        let done = self.voice.as_ref().is_none_or(|h| h.state() == PlaybackState::Stopped);
        if done && let Some(next) = self.voices.pop_front() {
            self.voice = self.manager.play(next).ok();
        }
    }

    /// The rain's background sound at `decibels`, or none: `CRain` makes it from `sounds.lib`
    /// and plays it as it is made, sets its volume from the rain's intensity every takt, and
    /// stops it as it goes (`Terrain.dll:0x10075e37`, `0x100765d0`, `0x100761a0`).
    ///
    /// STAND-IN: docs/10-sky.md#rain--read -- the flags the sound is made with, `0x102`, are
    /// not read: it is played as a loop, heard alike from everywhere.
    pub fn rain(&mut self, sound: Option<(String, f32)>) {
        let Some((name, decibels)) = sound else {
            if let Some((_, mut handle)) = self.rain.take() {
                handle.stop(Default::default());
            }
            return;
        };
        let volume = Decibels(decibels.max(Decibels::SILENCE.0));
        match self.rain.as_mut() {
            Some((playing, handle)) if *playing == name => {
                handle.set_volume(volume, Tween { duration: MOVE_TWEEN, ..Default::default() });
            }
            _ => {
                if let Some((_, mut old)) = self.rain.take() {
                    old.stop(Default::default());
                }
                let Some(data) = self.sound(&name) else { return };
                if let Ok(handle) = self.manager.play(data.volume(volume).loop_region(..)) {
                    self.rain = Some((name, handle));
                }
            }
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
    #[ignore = "needs the game install"]
    fn every_cd_track_the_player_can_pick_opens_as_ogg_vorbis() {
        let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
        let mut cd = parkan_world::music::CdPlayer::open(&game, 1);
        let mut tracks = std::collections::BTreeMap::new();
        for _ in 0..200 {
            if let Some(Cd::Play { track, path, looping }) = cd.random(false) {
                assert!(!looping);
                let data = StreamingSoundData::from_file(&path).expect("an Ogg Vorbis track");
                tracks.entry(track).or_insert_with(|| data.duration().as_secs_f64());
            }
        }
        // docs/34: tracks 2 to 10, 223 to 317 s long.
        assert_eq!(tracks.keys().copied().collect::<Vec<_>>(), (2..=10).collect::<Vec<u32>>());
        assert!(tracks.values().all(|&s| (220.0..320.0).contains(&s)), "{tracks:?}");
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
