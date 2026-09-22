//! The player's own folder: where the engine writes what the game writes into its install —
//! saved designs, saved games, settings — since an install need not be writable (Program
//! Files, a Steam library, a read-only volume). Nothing is ever written into the install.
//!
//! Its layout mirrors the install's: a file the game keeps at `units/<name>.dat` is kept at
//! `units/<name>.dat` here, so code that writes one hands [`resolve`] the same path it would
//! hand [`crate::gamedir::resolve`] to read the game's own.

use std::path::{Component, Path, PathBuf};

/// Names the folder outright, as `PARKAN_DIR` names the install.
pub const ENV_VAR: &str = "PARKAN_USER_DIR";

/// The folder's name under the system's per-user data folder.
pub const NAME: &str = "openparkan";

/// The game's folder of designs, which the warbot designer saves into (docs/37, "The
/// buttons").
pub const UNITS: &str = "units";

/// The folder: `$PARKAN_USER_DIR`, else `openparkan` in the system's per-user data folder —
/// `~/Library/Application Support` on macOS, `%APPDATA%` on Windows, and `$XDG_DATA_HOME`
/// or `~/.local/share` elsewhere. It is not created here.
pub fn find() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(dir) = var(ENV_VAR) {
        return Some(dir);
    }
    let data = if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else if cfg!(windows) {
        var("APPDATA")
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    Some(data?.join(NAME))
}

/// `relative`, laid out as the install lays the same file out, inside the folder. `None`
/// with no folder, or for a path that would lead out of it: a `..`, or on Windows a drive.
/// Nothing is created.
pub fn resolve(relative: &str) -> Option<PathBuf> {
    within(&find()?, relative)
}

/// `relative` inside `root`, split on either slash; `None` if any part would lead out of it.
pub fn within(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut at = root.to_path_buf();
    for part in relative.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".") {
        if !matches!(Path::new(part).components().next(), Some(Component::Normal(_)))
            || Path::new(part).components().count() != 1
        {
            return None;
        }
        at.push(part);
    }
    Some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_laid_out_as_the_install_lays_it_and_never_leads_out() {
        let root = Path::new("/data/openparkan");
        let tester = root.join("units").join("Tester.dat");
        assert_eq!(within(root, "units/Tester.dat"), Some(tester.clone()));
        assert_eq!(within(root, "units\\Tester.dat"), Some(tester.clone()));
        assert_eq!(within(root, "./units//Tester.dat"), Some(tester));
        assert_eq!(within(root, "../x.dat"), None);
        assert_eq!(within(root, "units/../../x.dat"), None);
        // A leading slash stays inside: every part is a name under the folder.
        assert_eq!(within(root, "/etc/passwd"), Some(root.join("etc").join("passwd")));
    }
}
