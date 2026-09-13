//! `Iron_3D.ini`, the engine's own settings. See `docs/22-settings.md`.

use std::path::Path;

use parkan_formats::gamedir;

pub const FILE: &str = "Iron_3D.ini";

/// The value of `key` in `[section]`, if the file has it.
pub fn value(game: &Path, section: &str, key: &str) -> Option<String> {
    let bytes = std::fs::read(gamedir::resolve(game, FILE)?).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = name.eq_ignore_ascii_case(section);
        } else if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim().eq_ignore_ascii_case(key)
        {
            return Some(v.trim().to_owned());
        }
    }
    None
}

/// The level ratio `GAME_LEVEL` picks (`iron3d.dll:0x10076010`): 0 `EASY`, 1 `MEDIUM`,
/// anything else `HARD`. See `docs/26-damage.md`, "The difficulty ratio".
pub fn level_ratio(game: &Path) -> f32 {
    let level = value(game, "CS", "GAME_LEVEL").and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
    let (key, default) = match level {
        0 => ("EASY", 0.5),
        1 => ("MEDIUM", 0.7),
        _ => ("HARD", 1.0),
    };
    value(game, "LEVEL_RATIO", key).and_then(|v| v.parse().ok()).unwrap_or(default)
}
