//! An input table (`.tbl`): what a key or mouse event sends, and where.
//!
//! Plain text, one row of eleven whitespace-separated fields and a `//` note.
//! See `docs/14-controls.md` and `openparkan/controls.py`.

use std::path::Path;

use crate::cursor::{FormatError, latin1};

/// The three tables: the pilot on foot, then two machine schemes.
pub const TABLES: [&str; 3] = ["hero.tbl", "m1.tbl", "m2.tbl"];
pub const TABLE_FIELDS: usize = 11;
pub const UNRESOLVED: i32 = -1;
pub const UNKNOWN_CLASS: i32 = 0;
pub const NO_MODIFIER: &str = "SCAN_NULL";

/// Movement command -> the number `World3D.dll` dispatches on.
pub const MCMD: [(&str, i32); 22] = [
    ("MCMD_DUMMY", 0),
    ("MCMD_STATE", 1),
    ("MCMD_ROTATE_X", 2),
    ("MCMD_ROTATE_Y", 3),
    ("MCMD_ROTATE_Z", 4),
    ("MCMD_ANGLE_X", 5),
    ("MCMD_ANGLE_Y", 6),
    ("MCMD_FORWARD", 7),
    ("MCMD_BACK", 8),
    ("MCMD_LEFT", 9),
    ("MCMD_RIGHT", 10),
    ("MCMD_UP", 11),
    ("MCMD_DOWN", 12),
    ("MCMD_SELECT", 13),
    ("MCMD_SELECT_NEXT", 14),
    ("MCMD_TABLE", 15),
    ("MCMD_ANGLE_Z", 16),
    ("MCMD_MISSILE", 17),
    ("MCMD_FIRE_ALL", 18),
    ("MCMD_WALK_F", 19),
    ("MCMD_WALK_B", 20),
    ("MCMD_LOCK", 21),
];

/// Component class -> its id; anything else is 0.
pub const CICLS: [(&str, i32); 13] = [
    ("CICLS_TURRET", 1),
    ("CICLS_MULTIGUN", 2),
    ("CICLS_SIMPLE", 3),
    ("CICLS_CAMERA", 4),
    ("CICLS_ENGINE", 5),
    ("CICLS_RADAR", 8),
    ("CICLS_FIGHTSHIELD", 9),
    ("CICLS_DETECTSHIELD", 10),
    ("CICLS_ELEVATOR", 11),
    ("CICLS_DOOR", 12),
    ("CICLS_COMPUTER", 13),
    ("CICLS_REPAIRSYS", 15),
    ("CICLS_POWERSTOR", 19),
];

/// Component state -> its bit.
pub const CIS: [(&str, i32); 15] = [
    ("CIS_SWITCHOFF", 0),
    ("CIS_SWITCHON", 32),
    ("CIS_SWITCH_INV", 64),
    ("CIS_ANGLETRACE", 256),
    ("CIS_CONTINUEFIGHT", 256),
    ("CIS_TURRETCONTROL", 256),
    ("CIS_MANUALCONTROL", 512),
    ("CIS_MANUALTRACE", 512),
    ("CIS_SINGLEFIGHT", 512),
    ("CIS_GROUPFIGHT", 1024),
    ("CIS_POINTTRACE", 1024),
    ("CIS_INFRARED_ON", 4096),
    ("CIS_INFRARED_OFF", 8192),
    ("CIS_CHAMELEON_INV", 16384),
    ("CIS_INFRARED_INV", 16384),
];

/// Whether an angle wraps at a whole turn.
pub const MAN_WRAP: i32 = 0;
pub const MAN_NOTWRAP: i32 = 1;
pub const MAN: [(&str, i32); 2] = [("MAN_WRAP", MAN_WRAP), ("MAN_NOTWRAP", MAN_NOTWRAP)];

pub const MCMD_STATE: i32 = 1;
pub const MCMD_ANGLE_X: i32 = 5;
pub const MCMD_ANGLE_Y: i32 = 6;
pub const MCMD_FORWARD: i32 = 7;
pub const MCMD_LEFT: i32 = 9;
pub const MCMD_RIGHT: i32 = 10;
pub const MCMD_SELECT: i32 = 13;
pub const MCMD_ANGLE_Z: i32 = 16;
pub const MCMD_WALK_F: i32 = 19;
pub const MCMD_WALK_B: i32 = 20;
pub const CICLS_TURRET: i32 = 1;
pub const CICLS_MULTIGUN: i32 = 2;
pub const CICLS_CAMERA: i32 = 4;
pub const CICLS_DETECTSHIELD: i32 = 10;
pub const CICLS_REPAIRSYS: i32 = 15;
pub const CIS_SWITCHON: i32 = 0x20;
pub const CIS_SWITCH_INV: i32 = 0x40;
pub const CIS_ON: i32 = 0x1000;
pub const CIS_OFF: i32 = 0x2000;
/// `CIS_CHAMELEON_INV` to a detection shield, `CIS_INFRARED_INV` to a camera.
pub const CIS_INV: i32 = 0x4000;

fn lookup(table: &[(&str, i32)], name: &str) -> Option<i32> {
    table.iter().find(|(n, _)| *n == name).map(|&(_, v)| v)
}

/// One row: a key event, and the command it sends where.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// `KEY` or `MOUSE`.
    pub device: String,
    pub modifier: String,
    pub key: String,
    /// True on the press row, false on the release row.
    pub pressed: bool,
    /// `CICLS_*`.
    pub target: String,
    /// `MCMD_*`.
    pub command: String,
    /// The magnitude: set on a key, added times the filtered counts on an axis.
    pub value: f32,
    pub index: i32,
    /// `0`, a `MAN_*` wrap flag, or a `CIS_*` state.
    pub state: String,
    pub ramp: f32,
    pub ramp_time: i32,
    pub note: String,
}

impl Action {
    /// The command as the engine numbers it.
    pub fn code(&self) -> i32 {
        lookup(&MCMD, &self.command).unwrap_or(UNRESOLVED)
    }

    /// The target class as the engine numbers it; `CICLS_UNKNOWN` is 0.
    pub fn class_id(&self) -> i32 {
        lookup(&CICLS, &self.target).unwrap_or(UNKNOWN_CLASS)
    }

    /// The state field's value: a `CIS_` bit, a `MAN_` flag, or 0.
    pub fn bits(&self) -> i32 {
        lookup(&CIS, &self.state).or_else(|| lookup(&MAN, &self.state)).unwrap_or(0)
    }

    /// Whether an angle this row moves wraps at a whole turn.
    pub fn wraps(&self) -> bool {
        self.state == "MAN_WRAP"
    }
}

/// Parse a table's text.
pub fn parse(data: &[u8], source: &str) -> Result<Vec<Action>, FormatError> {
    let text = latin1(data).replace("\r\n", "\n");
    let mut out = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let line = line.trim_end();
        let (body, note) = line.split_once("//").unwrap_or((line, ""));
        let fields: Vec<&str> = body.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let bad = |what: &str| FormatError::invalid(source, format!("line {}: {what}", number + 1));
        if fields.len() != TABLE_FIELDS {
            return Err(bad(&format!("{} fields, not {TABLE_FIELDS}", fields.len())));
        }
        let float = |s: &str| s.parse::<f32>().map_err(|_| bad(&format!("{s:?} is not a number")));
        let int = |s: &str| s.parse::<i32>().map_err(|_| bad(&format!("{s:?} is not an integer")));
        out.push(Action {
            device: fields[0].to_owned(),
            modifier: fields[1].to_owned(),
            key: fields[2].to_owned(),
            pressed: fields[3] == "1",
            target: fields[4].to_owned(),
            command: fields[5].to_owned(),
            value: float(fields[6])?,
            index: int(fields[7])?,
            state: fields[8].to_owned(),
            ramp: float(fields[9])?,
            ramp_time: int(fields[10])?,
            note: note.trim().to_owned(),
        });
    }
    Ok(out)
}

pub fn load(path: &Path) -> Result<Vec<Action>, FormatError> {
    parse(&std::fs::read(path)?, &path.display().to_string())
}

/// The key bindings `iron3d.dll` reads for the game itself (docs/14-controls.md).
pub const GAME_BINDINGS: &str = "ui_other.man";
/// `iron3d.dll`'s commands the player's target and the hero's Enter answer to
/// (`openparkan/controls.py`'s `CMD_GAME`).
pub const CMD_ENTER_STATE: &str = "CMD_ENTER_STATE";
pub const CMD_JAMES_SELECT_TARGET: &str = "CMD_JAMES_SELECT_TARGET";
pub const CMD_JAMES_SELECT_ENEMY: &str = "CMD_JAMES_SELECT_ENEMY";
pub const CMD_JAMES_SELECT_FRIEND: &str = "CMD_JAMES_SELECT_FRIEND";
pub const CMD_JAMES_AIM_TARGET: &str = "CMD_JAMES_AIM_TARGET";
pub const CMD_JAMES_WINGMAN_MENU: &str = "CMD_JAMES_WINGMAN_MENU";
pub const CMD_JAMES_AUTO_DRIVER: &str = "CMD_JAMES_AUTO_DRIVER";
pub const CMD_PAGER: &str = "CMD_PAGER";

/// One line of a `.man`: a command, and the key chord that runs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub command: String,
    /// A held key, or `SCAN_NULL`.
    pub modifier: String,
    pub key: String,
}

/// Parse a `.man`'s text: every line that is not blank is three fields.
pub fn bindings(data: &[u8], source: &str) -> Result<Vec<Binding>, FormatError> {
    let text = latin1(data).replace("\r\n", "\n");
    let mut out = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields[..] {
            [] => {}
            [command, modifier, key] => out.push(Binding {
                command: command.to_owned(),
                modifier: modifier.to_owned(),
                key: key.to_owned(),
            }),
            _ => {
                return Err(FormatError::invalid(
                    source,
                    format!("line {}: {} fields, not 3", number + 1, fields.len()),
                ));
            }
        }
    }
    Ok(out)
}

/// The command `key` runs with `held` down: a chord whose modifier is held first, else
/// one with none.
pub fn command_for<'a>(bindings: &'a [Binding], key: &str, held: impl Fn(&str) -> bool) -> Option<&'a str> {
    let on_key = || bindings.iter().filter(move |b| b.key == key);
    on_key()
        .find(|b| b.modifier != NO_MODIFIER && held(&b.modifier))
        .or_else(|| on_key().find(|b| b.modifier == NO_MODIFIER))
        .map(|b| b.command.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binding_line_is_three_fields_and_a_held_modifier_wins() {
        let text = b"CMD_CAMERA_CENTER SCAN_LSHIFT SCAN_RMOUSE\r\n\r\nCMD_JAMES_AIM_TARGET SCAN_NULL SCAN_RMOUSE\r\n";
        let b = bindings(text, "m").unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(command_for(&b, "SCAN_RMOUSE", |_| false), Some(CMD_JAMES_AIM_TARGET));
        assert_eq!(command_for(&b, "SCAN_RMOUSE", |m| m == "SCAN_LSHIFT"), Some("CMD_CAMERA_CENTER"));
        assert_eq!(command_for(&b, "SCAN_TAB", |_| false), None);
        assert!(bindings(b"CMD_X SCAN_NULL\n", "m").is_err());
    }

    #[test]
    fn a_row_resolves_its_names() {
        let text = b"// header\r\nMOUSE SCAN_NULL SCAN_MOUSE_Y 1 CICLS_TURRET MCMD_ANGLE_Y 0.25 1 MAN_NOTWRAP 0.0 0 // TURRET_UP\r\n\r\n";
        let rows = parse(text, "t").unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!((r.code(), r.class_id(), r.bits()), (MCMD_ANGLE_Y, CICLS_TURRET, MAN_NOTWRAP));
        assert!(r.pressed && !r.wraps());
        assert_eq!(r.note, "TURRET_UP");
    }

    #[test]
    fn a_short_row_is_refused() {
        assert!(parse(b"KEY SCAN_NULL SCAN_W 1\n", "t").is_err());
    }
}
