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

/// `[CS] GAME_LEVEL`: 0 easy, 1 medium, 2 hard. The value is the *index* the SuperAI is
/// built with, which is what picks `fDifficulty` ([`crate::progress::Progression::load`]).
pub fn game_level(game: &Path) -> usize {
    value(game, "CS", "GAME_LEVEL").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1)
}

/// `[CS] FORCE_SOFTWARE_CURSOR`, read as `atoi` reads it (`iron3d.dll:0x100614e8`): anything
/// but 0 has the display draw the cursor itself (its slot 21 handed 0, `0x10061534`). The
/// install's own file sets 1. See `docs/42-selection.md`, "The cursor shows a state".
pub fn software_cursor(game: &Path) -> bool {
    value(game, "CS", "FORCE_SOFTWARE_CURSOR").is_some_and(|v| atoi(&v) != 0)
}

/// The C library's `atoi`: leading white space, a sign, and the digits up to the first that
/// is not one; 0 for none.
fn atoi(text: &str) -> i64 {
    let t = text.trim_start();
    let (sign, digits) = match t.as_bytes().first() {
        Some(b'-') => (-1, &t[1..]),
        Some(b'+') => (1, &t[1..]),
        _ => (1, t),
    };
    let n = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |n, d| n.saturating_mul(10).saturating_add(i64::from(d - b'0')));
    sign * n
}

#[cfg(test)]
mod tests {
    use super::atoi;

    #[test]
    fn atoi_reads_a_leading_number_and_nothing_else() {
        assert_eq!([atoi("1"), atoi(" 12x"), atoi("-3"), atoi("x"), atoi("")], [1, 12, -3, 0, 0]);
    }
}
