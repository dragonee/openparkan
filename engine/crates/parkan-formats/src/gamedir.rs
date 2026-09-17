//! Finding a Parkan: Iron Strategy install.

use std::path::{Path, PathBuf};

pub const ENV_VAR: &str = "PARKAN_DIR";

/// What must all be present for a directory to be an install.
pub const MARKERS: [&str; 3] = ["Textures.lib", "MISSIONS", "DATA"];

/// Mission 01, *Line of Fire*: the first campaign mission.
pub const MISSION_01: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.01";
pub const MISSION_02: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.02";
pub const MISSION_03: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.03";
pub const MISSION_04: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.04";
/// The first chapter's *The Arrival* and *Outflanking Maneuver*.
pub const C01_MISSION_02: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.01/Mission.02";
pub const C01_MISSION_03: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.01/Mission.03";
/// The second chapter's *The Iron Monster*.
pub const C02_MISSION_01: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.01";
/// The second chapter's *Ballen's Crossing*.
pub const C02_MISSION_02: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.02";
/// The second chapter's *The Lost Key*.
pub const C02_MISSION_03: &str = "MISSIONS/CAMPAIGN/CAMPAIGN.02/Mission.03";

pub fn looks_like_install(path: &Path) -> bool {
    MARKERS.iter().all(|m| path.join(m).exists())
}

/// The install from an explicit path, `$PARKAN_DIR`, or the usual places.
pub fn find(explicit: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = explicit.map(Path::to_path_buf).into_iter().collect();
    if let Some(env) = std::env::var_os(ENV_VAR) {
        candidates.push(PathBuf::from(env));
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(here.join("../../../../Parkan Iron Strategy"));
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(
            PathBuf::from(home)
                .join("Library/Application Support/Steam/steamapps/common/Parkan Iron Strategy"),
        );
    }
    candidates.push(PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/Parkan Iron Strategy"));
    candidates.into_iter().find(|c| looks_like_install(c))
}

/// A file inside the install, matching each path component without regard to
/// case, as the game's own lookups do.
pub fn resolve(game: &Path, relative: &str) -> Option<PathBuf> {
    let mut at = game.to_path_buf();
    for part in relative.split(['/', '\\']).filter(|p| !p.is_empty()) {
        let exact = at.join(part);
        if exact.exists() {
            at = exact;
            continue;
        }
        at = std::fs::read_dir(&at)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(part)))?;
    }
    Some(at)
}
