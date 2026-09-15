//! The music: the CD's tracks, which the install ships as `MUSIC/TrackNN.ogg`, played as
//! `iron3d.dll`'s CD player and `Ngi32.dll`'s CD object drive them. See
//! `docs/34-progression.md`, "Music: the CD's tracks".

use std::path::{Path, PathBuf};

use parkan_formats::gamedir;

/// The directory the install's `FORCE_CD_SOUND` names.
pub const MUSIC_DIR: &str = "MUSIC";

/// How long past a track's length the CD object's tick waits before it plays a looping
/// track again or marks the track finished (`Ngi32.dll:0x1000ca52`).
pub const FINISH_GAP_MS: f64 = 2000.0;

/// The most CD tracks looked for.
const MOST_TRACKS: u32 = 99;

/// What the player asks of the audio device.
#[derive(Clone, Debug, PartialEq)]
pub enum Cd {
    /// Play CD track `track` (1-based) from `path`, once or looping.
    Play {
        track: u32,
        path: PathBuf,
        looping: bool,
    },
    Stop,
}

/// `iron3d.dll`'s CD player (`0x1008e260`) over `Ngi32.dll`'s CD object.
#[derive(Clone, Debug)]
pub struct CdPlayer {
    /// A file for each CD track, index 0 being track 1.
    files: Vec<Option<PathBuf>>,
    /// Track 1 is audio (`0x1000f050`); the install's is the data track.
    first_is_audio: bool,
    /// `PLAY_CD_MUSIC` is not 0 (`0x1008e2dd`).
    enabled: bool,
    /// The track index last played (`+4`), which a random pick never repeats.
    current: Option<usize>,
    /// The CD object's loop flag (`+0x2c`) and finished flag (`+0x3c`).
    looping: bool,
    finished: bool,
    /// When the track playing will have run (`+0x38`), or none before the first play.
    ends_at_ms: Option<f64>,
    /// The CRT's `rand()` state (`iron3d.dll:0x100b47d0`).
    seed: u32,
}

impl CdPlayer {
    /// The install's tracks and `Iron_3D.ini`'s `PLAY_CD_MUSIC`.
    ///
    /// STAND-IN: docs/34-progression.md#music-the-cds-tracks--read-and-measured -- the
    /// track count the install's `winmm.dll` reports is not read: it is the highest
    /// `TrackNN.ogg` present, and a track with no file is not audio.
    pub fn open(game: &Path, seed: u32) -> CdPlayer {
        let dir = gamedir::resolve(game, MUSIC_DIR);
        let mut files: Vec<Option<PathBuf>> = (1..=MOST_TRACKS)
            .map(|n| dir.as_ref().and_then(|d| gamedir::resolve(d, &format!("Track{n:02}.ogg"))))
            .collect();
        while files.last().is_some_and(Option::is_none) {
            files.pop();
        }
        let enabled = crate::settings::value(game, "CS", "PLAY_CD_MUSIC")
            .is_none_or(|v| v.trim().parse::<i64>() != Ok(0));
        CdPlayer::new(files, enabled, seed)
    }

    pub fn new(files: Vec<Option<PathBuf>>, enabled: bool, seed: u32) -> CdPlayer {
        let first_is_audio = files.first().is_some_and(Option::is_some);
        CdPlayer {
            files,
            first_is_audio,
            enabled,
            current: None,
            looping: false,
            finished: false,
            ends_at_ms: None,
            seed,
        }
    }

    /// The CRT's `rand()`: 0 to 0x7fff.
    fn rand(&mut self) -> usize {
        self.seed = self.seed.wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        ((self.seed >> 16) & 0x7fff) as usize
    }

    /// Play track index `index` (`0x1008e5d0`, the CD object's slot 3).
    pub fn play(&mut self, index: usize, looping: bool) -> Option<Cd> {
        if !self.enabled || index >= self.files.len() {
            return None;
        }
        self.current = Some(index);
        self.looping = looping;
        self.finished = false;
        // Its length is known once the device has it (`started`).
        self.ends_at_ms = Some(f64::INFINITY);
        let path = self.files[index].clone()?;
        Some(Cd::Play { track: index as u32 + 1, path, looping })
    }

    /// A random track (`0x1008e4d0`): with one track that is audio, it; with two and track 1
    /// not audio, the second; with more, `rand()` over the count until the pick is not the
    /// track last played, nor track 1 unless it is audio.
    pub fn random(&mut self, looping: bool) -> Option<Cd> {
        let count = self.files.len();
        if !self.enabled {
            return None;
        }
        if count == 1 && self.first_is_audio {
            return self.play(0, looping);
        }
        if count == 2 && !self.first_is_audio {
            return self.play(1, looping);
        }
        if count <= 1 {
            return None;
        }
        let pick = loop {
            let i = self.rand() % count;
            if Some(i) != self.current && (i != 0 || self.first_is_audio) {
                break i;
            }
        };
        self.play(pick, looping)
    }

    /// Stop the CD (`0x1008e620`, the CD object's slot 4, which clears its loop flag).
    pub fn stop(&mut self) -> Cd {
        self.looping = false;
        Cd::Stop
    }

    /// The track just started runs `length_ms` from `now_ms` (`Ngi32.dll:0x1000ee25`); a
    /// track the device could not start is given a length of 0.
    pub fn started(&mut self, now_ms: f64, length_ms: f64) {
        self.ends_at_ms = Some(now_ms + length_ms);
    }

    /// One frame at `now_ms`, `briefing` while a briefing runs. The CD object's tick, 2 s
    /// past the track's end with the CD stopped, plays a looping track again or marks it
    /// finished (`0x1000ca42`–`0x1000ca85`); outside a briefing a finished track makes way
    /// for another random one (`iron3d.dll:0x1005ea34`–`0x1005ea4e`).
    pub fn frame(&mut self, now_ms: f64, briefing: bool) -> Option<Cd> {
        if !self.finished && now_ms > self.ends_at_ms.unwrap_or(f64::NEG_INFINITY) + FINISH_GAP_MS {
            match (self.looping, self.current) {
                (true, Some(index)) => return self.play(index, true),
                _ => self.finished = true,
            }
        }
        if !briefing && self.finished {
            return self.random(false);
        }
        None
    }
}

/// The track index a mission's briefing plays, looping: its `briefing` object's `cd_track`,
/// so CD track `cd_track` + 1 (`iron3d.dll:0x10031452`).
pub fn briefing_track(mission_dir: &Path) -> Option<usize> {
    let cfg = gamedir::resolve(mission_dir, "mission.cfg")?;
    let blocks = crate::resources::read_cfg(&cfg).ok()?;
    blocks.get("briefing")?.get("cd_track")?.trim().parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install() -> CdPlayer {
        // Tracks 2 to 10 have files, as the install's MUSIC does.
        let files = (1..=10).map(|n| (n > 1).then(|| PathBuf::from(format!("Track{n:02}.ogg")))).collect();
        CdPlayer::new(files, true, 1)
    }

    #[test]
    fn a_random_track_is_never_the_data_track_nor_the_one_just_played() {
        let mut cd = install();
        let mut last = None;
        for _ in 0..200 {
            let Some(Cd::Play { track, looping, .. }) = cd.random(false) else { panic!("a track plays") };
            assert!((2..=10).contains(&track) && !looping);
            assert_ne!(Some(track), last);
            last = Some(track);
        }
    }

    #[test]
    fn a_track_makes_way_for_another_two_seconds_after_it_ends_but_not_in_a_briefing() {
        let mut cd = install();
        assert!(cd.frame(0.0, true).is_none(), "silent through a briefing");
        let Some(Cd::Play { track: first, .. }) = cd.frame(100.0, false) else { panic!("after it") };
        cd.started(100.0, 5000.0);
        assert_eq!(cd.frame(7000.0, false), None, "still inside its length and the gap");
        let Some(Cd::Play { track: next, .. }) = cd.frame(7200.0, false) else { panic!("the next") };
        assert_ne!(first, next);
    }

    #[test]
    fn a_looping_track_plays_again_and_a_stop_ends_its_loop() {
        let mut cd = install();
        assert!(matches!(cd.play(2, true), Some(Cd::Play { track: 3, looping: true, .. })));
        cd.started(0.0, 1000.0);
        assert!(matches!(cd.frame(3100.0, true), Some(Cd::Play { track: 3, looping: true, .. })));
        cd.started(3100.0, 1000.0);
        assert_eq!(cd.stop(), Cd::Stop);
        assert_eq!(cd.frame(3200.0, true), None);
        assert!(matches!(cd.frame(6200.0, false), Some(Cd::Play { looping: false, .. })));
    }

    #[test]
    fn play_cd_music_0_plays_nothing() {
        let mut cd = install();
        cd.enabled = false;
        assert_eq!(cd.random(false), None);
        assert_eq!(cd.frame(10_000.0, false), None);
    }
}
