//! The commander's order rows in command mode: `iron3d.dll`'s HQ table (`0x10104f98`), which
//! rows a selection is offered (`0x1007aaa0`, `0x1007bbb0`), and the order a row gives
//! straight away (`0x10079230`). See `docs/31-packages.md`, "The commander's menus", and
//! `docs/41-commander.md`, "The order menu".

use crate::orders::{CAPTURE_TYPES, Order, RELOAD, SEARCH, STAYGROUND, Target};

/// Every robot type a row may be offered to, transports, and builders (the table's masks).
pub const EVERY_ROBOT: u32 = 0x0103_e000;
pub const TRANSPORTS: u32 = 0x0100_2000;
pub const BUILDERS: u32 = 0x0100_4000;
/// `ORDER_ROBOT_GO`, `_TRANSPORT` and `_BUILD` (docs/31, "The orders").
pub use crate::orders::{BUILD, GO, TRANSPORT};
/// Search minerals' type (`0x10001000`).
pub const MINERALS: u32 = 0x1000_1000;

/// What a row does once clicked (`0x1007b740`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    /// An order for every selected unit, replacing its queue.
    Order(Order),
    /// Pick mode 4, then `GO` to the places picked (Route).
    Route,
    /// Pick mode 3, then `PATROL` a unit, a building or a place (Guard).
    Guard,
    /// Place a building of this Type (pick mode 6 for a mine, 4 otherwise; docs/32).
    Build(u32),
    /// Upgrade the clan's building of this Type.
    Upgrade(u32),
}

/// One row of the HQ table: its command id, the types it is offered to, and its string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub command: u8,
    pub mask: u32,
    pub string: u32,
}

/// The HQ table's 22 rows, in command order: 0–3 and 6–23 (docs/31).
pub const ROWS: [Row; 22] = [
    Row { command: 0, mask: EVERY_ROBOT, string: 5000 },
    Row { command: 1, mask: EVERY_ROBOT, string: 5001 },
    Row { command: 2, mask: EVERY_ROBOT, string: 5002 },
    Row { command: 3, mask: EVERY_ROBOT, string: 5003 },
    Row { command: 6, mask: EVERY_ROBOT, string: 5006 },
    Row { command: 7, mask: EVERY_ROBOT, string: 5007 },
    Row { command: 8, mask: TRANSPORTS, string: 5020 },
    Row { command: 9, mask: BUILDERS, string: 5030 },
    Row { command: 10, mask: BUILDERS, string: 5031 },
    Row { command: 11, mask: BUILDERS, string: 5032 },
    Row { command: 12, mask: BUILDERS, string: 5033 },
    Row { command: 13, mask: BUILDERS, string: 5034 },
    Row { command: 14, mask: BUILDERS, string: 5035 },
    Row { command: 15, mask: BUILDERS, string: 5036 },
    Row { command: 16, mask: BUILDERS, string: 5046 },
    Row { command: 17, mask: BUILDERS, string: 1008 },
    Row { command: 18, mask: BUILDERS, string: 1029 },
    Row { command: 19, mask: BUILDERS, string: 1030 },
    Row { command: 20, mask: BUILDERS, string: 1031 },
    Row { command: 21, mask: BUILDERS, string: 1032 },
    Row { command: 22, mask: BUILDERS, string: 1033 },
    Row { command: 23, mask: BUILDERS, string: 5047 },
];

/// The seven building Types the Build and Upgrade rows name, in command order (Mine,
/// Warehouse, Factory, Outpost, Res. Center, Light Tower, Heavy Tower; `0x1007a902`).
pub const BUILD_TYPES: [u32; 7] =
    [0x8000_0004, 0x8000_0008, 0x8000_0010, 0x8000_0040, 0x8000_0400, 0x8010_0000, 0x8020_0000];

/// The building Types `0x10034230` accepts for no upgrade, whatever their ladder
/// (`iron3d.dll:0x10034282`-`0x100342b5`): the generator, the hangar (the Outpost), the main
/// teleport, the bridge and the ruin (`varset.var`'s `BUILDING_GENERATOR`, `_HANGAR`,
/// `_MAINTELEPORT`, `_BRIDGE`, `_RUINE`).
pub const NEVER_UPGRADED: [u32; 5] = [0x8000_0002, 0x8000_0040, 0x8000_0200, 0x8000_1000, 0x8000_2000];

/// A type lies within a mask when the mask holds all its bits: `(mask & type) == type`.
pub fn within(type_word: u32, mask: u32) -> bool {
    type_word != 0 && mask & type_word == type_word
}

/// The row with command `command`.
pub fn row(command: u8) -> Option<&'static Row> {
    ROWS.iter().find(|r| r.command == command)
}

/// What the row test reads of the play (`0x1007bbb0`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Situation {
    /// The first selected unit's record `+0x30`.
    pub first_class: u8,
    /// A lode of the map is not yet found.
    pub lode_unfound: bool,
    /// The first selected unit can build: alive, with an intact beam (`0x10076da0`).
    pub can_build: bool,
    /// Per build Type: offered by the tree, owned now or marked owned, or already heading off.
    pub buildable: [bool; 7],
    /// Per build Type: the clan has a building of it an upgrade would take
    /// (`0x10034230`).
    pub upgradable: [bool; 7],
}

/// The rows offered to units of these Types, each command once, in command order
/// (`0x1007aaa0`, `0x1007aef9`).
pub fn offered(types: &[u32], situation: &Situation) -> Vec<u8> {
    let mut out: Vec<u8> = ROWS
        .iter()
        .filter(|r| types.iter().any(|&t| within(t, r.mask)))
        .filter(|r| test(r.command, situation))
        .map(|r| r.command)
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The row test (`0x1007bbb0`). An Upgrade row asks for a builder that can build and one of
/// the clan's buildings of its Type that `0x10034230` accepts, which the play reads into
/// [`Situation::upgradable`] (docs/41, "Which rows it offers").
fn test(command: u8, s: &Situation) -> bool {
    match command {
        2 => matches!(s.first_class, 1 | 2),
        9 => s.lode_unfound,
        10..=16 => s.can_build && s.buildable[usize::from(command - 10)],
        17..=23 => s.can_build && s.upgradable[usize::from(command - 17)],
        _ => true,
    }
}

/// What row `command` does (the dispatcher's and the executor's cases, docs/31).
pub fn act(command: u8) -> Option<Act> {
    let order = |code, parameter, target| Some(Act::Order(Order { code, parameter, target }));
    match command {
        0 => order(STAYGROUND, 0, Target::NotDefined),
        1 => Some(Act::Route),
        2 => order(SEARCH, 0, Target::TypeMask(CAPTURE_TYPES)),
        3 => order(SEARCH, -1, Target::Any),
        6 => Some(Act::Guard),
        7 => order(RELOAD, 0, Target::NotDefined),
        8 => order(TRANSPORT, -1, Target::NotDefined),
        9 => order(SEARCH, 0, Target::TypeMask(MINERALS)),
        10..=16 => Some(Act::Build(BUILD_TYPES[usize::from(command - 10)])),
        17..=23 => Some(Act::Upgrade(BUILD_TYPES[usize::from(command - 17)])),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILDER: u32 = 0x0100_4000;
    const TRANSPORT_TYPE: u32 = 0x0100_2000;
    const WARRIOR: u32 = 0x0100_8000;

    #[test]
    fn mission_03s_builder_is_offered_build_mine_alone() {
        let s = Situation {
            first_class: 2,
            can_build: true,
            buildable: [true, false, false, false, false, false, false],
            ..Situation::default()
        };
        assert_eq!(offered(&[BUILDER], &s), vec![0, 1, 2, 3, 6, 7, 10]);
    }

    #[test]
    fn an_upgrade_row_needs_a_builder_and_a_building_of_its_type_to_take_up() {
        let s = Situation {
            first_class: 2,
            can_build: true,
            upgradable: [false, false, true, false, false, false, false],
            ..Situation::default()
        };
        // Command 19 is the Factory's, third of the seven, as 12 is Build Factory.
        assert_eq!(offered(&[BUILDER], &s), vec![0, 1, 2, 3, 6, 7, 19]);
        assert_eq!(act(19), Some(Act::Upgrade(BUILD_TYPES[2])));
        // A unit that cannot build gets none of them, and neither does a warrior.
        let cannot = Situation { can_build: false, ..s };
        assert_eq!(offered(&[BUILDER], &cannot), vec![0, 1, 2, 3, 6, 7]);
        assert_eq!(offered(&[WARRIOR], &s), vec![0, 1, 2, 3, 6, 7], "no Upgrade row off the mask");
    }

    #[test]
    fn of_the_seven_upgrade_rows_only_the_outposts_type_is_refused_outright() {
        // `0x10034230` turns the hangar away before it looks at a ladder, so row 20, Upgrade
        // Outpost, is never offered; the other six Types go on to the ladder and the tree.
        let refused: Vec<u8> =
            (17..=23).filter(|c| NEVER_UPGRADED.contains(&BUILD_TYPES[usize::from(c - 17)])).collect();
        assert_eq!(refused, vec![20]);
    }

    #[test]
    fn a_transport_gets_transport_minerals_and_a_mixed_selection_the_union() {
        let s = Situation { first_class: 3, ..Situation::default() };
        assert_eq!(offered(&[TRANSPORT_TYPE], &s), vec![0, 1, 3, 6, 7, 8]);
        assert_eq!(offered(&[WARRIOR, TRANSPORT_TYPE], &s), vec![0, 1, 3, 6, 7, 8]);
    }

    #[test]
    fn a_row_needs_the_mask_to_hold_the_whole_type() {
        assert!(within(BUILDER, EVERY_ROBOT) && within(BUILDER, BUILDERS));
        assert!(!within(TRANSPORT_TYPE, BUILDERS) && !within(WARRIOR, TRANSPORTS));
    }

    #[test]
    fn seek_and_destroy_searches_anything_and_build_mine_places_a_mine() {
        assert_eq!(act(3), Some(Act::Order(Order { code: SEARCH, parameter: -1, target: Target::Any })));
        assert_eq!(act(10), Some(Act::Build(0x8000_0004)));
        assert_eq!(act(4), None);
    }
}
