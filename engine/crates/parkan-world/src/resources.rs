//! A mission's words and voices, found in the install: its objectives, its messages'
//! text and sound, its ambient music, and the game's own strings.
//!
//! See `docs/20-resources.md` and `docs/34-progression.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use parkan_formats::cfg::{self, Blocks, Objective};
use parkan_formats::gamedir;
use parkan_formats::nres::Archive;
use parkan_formats::resources::{self as formats, Descriptor, TextResources};

/// The name table for the game's text.
pub const TEXT_INDEX: &str = "DATA/TextRes.cfg";
/// The descriptors the game's own voices are bound in.
pub const GAME_RESOURCES: &str = "ui/game_resources.cfg";
/// The binary whose string table holds the game's interface words.
pub const GAME_STRINGS: &str = "iron3d.dll";
/// The descriptor roles a mission's ambient sound is read from (`iron3d.dll:0x1005e2e1`,
/// `0x1005f8f1`).
pub const MUSIC_LOOP: &str = "ambient_music_loop";
pub const MUSIC_VARIATION: &str = "ambient_music_variation";
pub const THEME: &str = "THEME";

/// A sound in the install: an archive and one of its members.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sound {
    pub library: PathBuf,
    pub member: String,
}

impl Sound {
    /// The member's bytes.
    pub fn read(&self) -> Result<Vec<u8>> {
        let archive = Archive::open(&self.library)?;
        Ok(archive.read_name(&self.member)?.to_vec())
    }

    /// Whether the archive opens and holds the member.
    pub fn exists(&self) -> bool {
        Archive::open(&self.library).is_ok_and(|a| a.find(&self.member).is_some())
    }
}

/// A descriptor's `library` in the install: backslash-separated, sometimes doubled, in
/// whatever case the author typed. `None` when it does not resolve, or names nothing.
pub fn locate(game: &Path, library: &str) -> Option<PathBuf> {
    if !library.split(['/', '\\']).any(|p| !p.is_empty()) {
        return None;
    }
    gamedir::resolve(game, library)
}

/// A `.cfg` file, parsed.
pub fn read_cfg(path: &Path) -> Result<Blocks> {
    Ok(cfg::parse(&std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?))
}

/// The descriptors of a `.cfg`, or none when the file is absent.
fn descriptors_in(path: Option<PathBuf>) -> Result<Vec<Descriptor>> {
    match path {
        Some(p) if p.is_file() => Ok(formats::descriptors(&read_cfg(&p)?)),
        _ => Ok(Vec::new()),
    }
}

/// The names a mission can play a sound by: its own `mission.cfg`'s descriptors, then
/// the game's `ui/game_resources.cfg`.
pub struct Sounds {
    pub game: PathBuf,
    pub descriptors: Vec<Descriptor>,
}

impl Sounds {
    pub fn open(game: &Path, mission_dir: &Path) -> Result<Self> {
        let mut descriptors = descriptors_in(gamedir::resolve(mission_dir, "mission.cfg"))?;
        descriptors.extend(descriptors_in(gamedir::resolve(game, GAME_RESOURCES))?);
        Ok(Self { game: game.to_path_buf(), descriptors })
    }

    /// The sound `name` is bound to.
    ///
    /// STAND-IN: docs/34-progression.md#messages--read-and-measured -- the resource
    /// manager's lookup order is not read: the first descriptor that binds the name
    /// wins, the mission's in file order before the game's. Every training message
    /// resolves this way to the member the briefing or tutorial descriptor names.
    pub fn get(&self, name: &str) -> Option<Sound> {
        let (d, member) = formats::bound(&self.descriptors, name)?;
        Some(Sound { library: locate(&self.game, &d.library)?, member: member.to_owned() })
    }
}

/// `TextRes.cfg`'s names and `TextRes.dll`'s strings, joined.
pub fn text_resources(game: &Path) -> Result<TextResources> {
    let index = gamedir::resolve(game, TEXT_INDEX).context("the install has no DATA/TextRes.cfg")?;
    let found = formats::descriptors(&read_cfg(&index)?);
    let d = found.first().with_context(|| format!("{}: no resource descriptor", index.display()))?;
    let library = locate(game, &d.library).with_context(|| format!("no library {}", d.library))?;
    let table = formats::strings(&std::fs::read(&library)?, None, &library.display().to_string())?;
    Ok(TextResources::new(d, table))
}

/// The game's interface strings, `iron3d.dll`'s string table by id: 5040 "Objective is
/// completed", 6170 and 6223 for a message heard before.
pub fn game_strings(game: &Path) -> Result<BTreeMap<u32, String>> {
    let path = gamedir::resolve(game, GAME_STRINGS).context("the install has no iron3d.dll")?;
    Ok(formats::strings(&std::fs::read(&path)?, None, &path.display().to_string())?)
}

/// A mission's objective list, in script order; empty without a `mission.cfg`.
pub fn objectives(mission_dir: &Path) -> Result<Vec<Objective>> {
    match gamedir::resolve(mission_dir, "mission.cfg") {
        Some(p) => Ok(cfg::objectives(&read_cfg(&p)?)),
        None => Ok(Vec::new()),
    }
}

/// The `mission.cfg` object whose `script%d` lines function 57 runs as console commands.
pub const SCRIPT_BLOCK: &str = "script";

/// A mission's `script` block, its lines by key (`script1`, `script2`, …); empty without a
/// `mission.cfg` or a block (docs/15, "Channel 2 runs a line of the mission's `script` block").
pub fn console_lines(mission_dir: &Path) -> Result<Vec<(String, String)>> {
    let Some(p) = gamedir::resolve(mission_dir, "mission.cfg") else { return Ok(Vec::new()) };
    Ok(read_cfg(&p)?.get(SCRIPT_BLOCK).map(|b| b.properties.clone()).unwrap_or_default())
}

/// One in-mission message, resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub index: i64,
    pub name: String,
    pub text_id: String,
    pub voice_id: String,
    /// Its text through `TextRes.cfg`, where the name is bound.
    pub text: Option<String>,
    /// Its voice through the mission's descriptors, where the name is bound.
    pub voice: Option<Sound>,
    pub info_system: bool,
}

/// A mission's `messages.cfg`, resolved, in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages(pub Vec<Message>);

impl Messages {
    /// The messages of the mission in `mission_dir`; none without a `messages.cfg`.
    pub fn load(game: &Path, mission_dir: &Path) -> Result<Self> {
        let Some(path) = gamedir::resolve(mission_dir, "messages.cfg") else { return Ok(Self::default()) };
        let text = text_resources(game)?;
        let sounds = Sounds::open(game, mission_dir)?;
        Ok(Self(
            cfg::messages(&read_cfg(&path)?)
                .into_iter()
                .map(|m| Message {
                    index: m.index,
                    text: text.get(&m.text_id).map(str::to_owned),
                    voice: sounds.get(&m.voice_id),
                    name: m.name,
                    text_id: m.text_id,
                    voice_id: m.voice_id,
                    info_system: m.info_system,
                })
                .collect(),
        ))
    }

    /// The message a script asks for by `message_index` (`iron3d.dll:0x10094e30`).
    pub fn get(&self, index: i64) -> Option<&Message> {
        self.0.iter().find(|m| m.index == index)
    }
}

/// A mission's ambient sound: its theme, and the variations gathered for each part of
/// the day.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ambient {
    pub theme: Option<Sound>,
    pub default: Vec<Sound>,
    pub day: Vec<Sound>,
    pub night: Vec<Sound>,
}

/// The `ambient_music_loop` `THEME`, and `ambient_music_variation`'s `DEFAULT_`, `DAY_`
/// and `NIGHT_` + `VARIATION1`, `2`, … for as long as the key exists
/// (`iron3d.dll:0x1005f8f1`).
pub fn ambient(game: &Path, mission_dir: &Path) -> Result<Ambient> {
    let found = descriptors_in(gamedir::resolve(mission_dir, "mission.cfg"))?;
    let role = |name: &str| found.iter().find(|d| d.role == name);
    let sound = |d: &Descriptor, key: &str| {
        let member = d.get(key)?;
        Some(Sound { library: locate(game, &d.library)?, member: member.to_owned() })
    };
    let gather = |prefix: &str| -> Vec<Sound> {
        let Some(d) = role(MUSIC_VARIATION) else { return Vec::new() };
        (1..).map_while(|n| sound(d, &format!("{prefix}VARIATION{n}"))).collect()
    };
    Ok(Ambient {
        theme: role(MUSIC_LOOP).and_then(|d| sound(d, THEME)),
        default: gather("DEFAULT_"),
        day: gather("DAY_"),
        night: gather("NIGHT_"),
    })
}

/// The shortest wait between two ambient variations: the float 10.0 at
/// `iron3d.dll:0x100e5d6c`, in seconds.
pub const VARIATION_BASE_S: f64 = 10.0;
/// The spread the game adds to it, `rand() % 10` seconds (`0x1005eb7a`–`0x1005eb95`).
pub const VARIATION_SPREAD_S: u32 = 10;

/// A mission's ambient variations as the game plays them (`iron3d.dll:0x1005eb49`, the
/// picker `0x1008e690`): every 10 to 19 s the game picks one of the names its part of the
/// day gathered and hands it to the sound server, the same call the theme goes through.
#[derive(Clone, Debug, PartialEq)]
pub struct Ambience {
    ambient: Ambient,
    /// When the next variation is due (`the game's +0x94` over its `+0x90` stamp).
    next_ms: f64,
    /// The index played last, never picked twice running (the picker's `+0x1c`).
    last: Option<usize>,
    /// The picker's two words (`+0x10`, `+0x12`).
    words: (u16, u16),
    /// The CRT `rand()` the wait is drawn from (`iron3d.dll:0x100b47d0`).
    seed: u32,
}

impl Ambience {
    /// The variations of `ambient`, with `seed` for the wait and `words` for the pick.
    pub fn new(ambient: Ambient, seed: u32, words: (u16, u16)) -> Self {
        Self { ambient, next_ms: 0.0, last: None, words, seed }
    }

    /// Which list the part of the day picks (`iron3d.dll:0x1008e760`): the `DEFAULT_` names
    /// when the mission gathered no `DAY_` or `NIGHT_` one, else the night's or the day's.
    pub fn bucket(&self, night: bool) -> &[Sound] {
        if self.ambient.day.is_empty() && self.ambient.night.is_empty() {
            &self.ambient.default
        } else if night {
            &self.ambient.night
        } else {
            &self.ambient.day
        }
    }

    /// The CRT's `rand()`, 0 to 0x7fff.
    fn rand(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0003_43fd).wrapping_add(0x0026_9ec3);
        (self.seed >> 16) & 0x7fff
    }

    /// The next index, drawn again while it repeats the last (`0x1008e6f0`). One name is
    /// always index 0; an empty list picks nothing.
    fn pick(&mut self, count: usize) -> Option<usize> {
        match count {
            0 => {
                self.last = None;
                return None;
            }
            1 => {
                self.last = Some(0);
                return Some(0);
            }
            _ => {}
        }
        loop {
            let (s0, s1) = self.words;
            let s0 = (s0 << 1) ^ s1;
            let s1 = (s1 >> 1) ^ s0;
            self.words = (s0, s1);
            let index = usize::from(s1) % count;
            if Some(index) != self.last {
                self.last = Some(index);
                return Some(index);
            }
        }
    }

    /// The frame's turn at the variations: `Some` names one to play at once. The first
    /// frame plays one, the game's stamp and wait both starting at 0.
    pub fn frame(&mut self, now_ms: f64, night: bool) -> Option<&Sound> {
        if now_ms <= self.next_ms {
            return None;
        }
        let wait = VARIATION_BASE_S + f64::from(self.rand() % VARIATION_SPREAD_S);
        self.next_ms = now_ms + wait * 1000.0;
        let index = self.pick(self.bucket(night).len())?;
        self.bucket(night).get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sounds(names: &[&str]) -> Vec<Sound> {
        names.iter().map(|n| Sound { library: PathBuf::new(), member: (*n).to_owned() }).collect()
    }

    #[test]
    fn a_mission_with_day_and_night_names_never_reaches_its_default_list() {
        let ambient = Ambient {
            theme: None,
            default: sounds(&["d"]),
            day: sounds(&["day1", "day2"]),
            night: sounds(&["night1"]),
        };
        let a = Ambience::new(ambient, 1, (0xace1, 0x1234));
        assert_eq!(a.bucket(false).len(), 2);
        assert_eq!(a.bucket(true)[0].member, "night1");
    }

    #[test]
    fn a_mission_with_only_default_names_plays_them_by_day_and_by_night() {
        let ambient =
            Ambient { theme: None, default: sounds(&["a", "b"]), day: Vec::new(), night: Vec::new() };
        let a = Ambience::new(ambient, 1, (0xace1, 0x1234));
        assert_eq!(a.bucket(false).len(), 2);
        assert_eq!(a.bucket(true).len(), 2);
    }

    #[test]
    fn a_variation_plays_at_once_and_then_every_ten_to_nineteen_seconds() {
        let ambient =
            Ambient { theme: None, default: sounds(&["a", "b", "c"]), day: Vec::new(), night: Vec::new() };
        let mut a = Ambience::new(ambient, 1, (0xace1, 0x1234));
        assert!(a.frame(1.0, false).is_some(), "the first frame plays one");
        let mut plays = Vec::new();
        let mut last = 1.0;
        for step in 1..200_000u32 {
            let now = f64::from(step) * 10.0;
            if a.frame(now, false).is_some() {
                plays.push(now - last);
                last = now;
            }
        }
        assert!(plays.len() > 100, "{} plays over half an hour", plays.len());
        let least = plays.iter().cloned().fold(f64::MAX, f64::min);
        let most = plays.iter().cloned().fold(0.0, f64::max);
        assert!(least >= 10_000.0, "least {least}");
        // A 10 ms frame can land up to one frame past the 19 s the longest wait asks for.
        assert!(most <= 19_010.0, "most {most}");
    }

    #[test]
    fn no_variation_is_picked_twice_running_and_every_one_is_picked() {
        let ambient =
            Ambient { theme: None, default: sounds(&["a", "b", "c"]), day: Vec::new(), night: Vec::new() };
        let mut a = Ambience::new(ambient, 1, (0xace1, 0x1234));
        let mut heard: Vec<String> = Vec::new();
        for step in 1..200_000u32 {
            if let Some(s) = a.frame(f64::from(step) * 10.0, false) {
                heard.push(s.member.clone());
            }
        }
        assert!(heard.len() > 100);
        assert!(heard.windows(2).all(|w| w[0] != w[1]), "no name repeats at once");
        for name in ["a", "b", "c"] {
            assert!(heard.iter().any(|h| h == name), "{name} is heard");
        }
    }

    #[test]
    fn one_name_alone_is_played_over_and_over() {
        let ambient = Ambient { theme: None, default: sounds(&["only"]), day: Vec::new(), night: Vec::new() };
        let mut a = Ambience::new(ambient, 1, (0xace1, 0x1234));
        assert_eq!(a.frame(1.0, false).map(|s| s.member.clone()), Some("only".to_owned()));
        assert_eq!(a.frame(30_000.0, false).map(|s| s.member.clone()), Some("only".to_owned()));
    }

    #[test]
    fn a_mission_with_no_variations_plays_nothing() {
        let mut a = Ambience::new(Ambient::default(), 1, (0xace1, 0x1234));
        assert!(a.frame(1.0, false).is_none());
        assert!(a.frame(60_000.0, false).is_none());
    }

    #[test]
    fn a_library_that_names_nothing_does_not_locate() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(locate(here, ""), None);
        assert_eq!(locate(here, "\\\\"), None);
        assert!(locate(here, "SRC\\\\lib.rs").is_some(), "doubled separators, any case");
    }
}
