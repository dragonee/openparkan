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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_library_that_names_nothing_does_not_locate() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(locate(here, ""), None);
        assert_eq!(locate(here, "\\\\"), None);
        assert!(locate(here, "SRC\\\\lib.rs").is_some(), "doubled separators, any case");
    }
}
