//! Orders a commander gives, and the wingman menu that gives them from first person.
//! See `docs/31-packages.md`, "The commander's menus" and "The wingman menu from first
//! person".

/// Order codes (`varset.var`'s `ORDER_*`, the packet's `+0`).
/// `ORDER_ROBOT_GO`, the route (docs/31, "What each package does").
pub const GO: i32 = 2;
pub const ATTACK: i32 = 3;
pub const PATROL: i32 = 4;
pub const SEARCH: i32 = 5;
/// `ORDER_ROBOT_TRANSPORT` and `ORDER_ROBOT_BUILD` (docs/32, "Transporting ore" and "Building
/// a building").
pub const TRANSPORT: i32 = 6;
pub const BUILD: i32 = 7;
/// `ORDER_ROBOT_RELOAD` and `ORDER_ROBOT_REPARE`, which build the same refit
/// (`MTaskStack::CreateTaskFromOrder`, docs/27, "What sends a bot to a dock").
pub const RELOAD: i32 = 8;
pub const REPARE: i32 = 9;
/// `ORDER_ROBOT_CAPTURE`: a script's capture of one building by logic id (docs/27, "Capture").
pub const CAPTURE: i32 = 17;
/// `ORDER_BUILDING_MINE`, which every mine is given as it joins (docs/23, "A mine digs to 500").
pub const MINE: i32 = 10;
/// The hidden construction sphere a new building runs (docs/32, "The construction sphere").
pub const SHOW_UPGRADE: i32 = 18;
/// `ORDER_ROBOT_UPGRADE`: a builder walks a building of its own clan one step up its scheme
/// (docs/32, "Upgrading a building").
pub const UPGRADE: i32 = 24;
/// `ORDER_ROBOT_LEAVE`, the escape (docs/31, "The escape").
pub const LEAVE: i32 = 20;
/// `ORDER_ROBOT_SHUTDOWN` (docs/31, "Which objects run a behaviour").
pub const SHUTDOWN: i32 = 0x13;
pub const STAYGROUND: i32 = 0x15;
pub const FOLLOW: i32 = 0x16;
/// Follow me's parameter: the radius it keeps within.
pub const FOLLOW_RADIUS: i32 = 50;
/// Search and capture's building types (`0x8017365e`), and the building Capture building
/// refuses (`0x80000200`).
pub const CAPTURE_TYPES: u32 = 0x8017_365e;
pub const UNCAPTURABLE_BUILDING: u32 = 0x8000_0200;

/// The target kinds an order names (`varset.var`'s `TARGET_*`).
pub const TARGET_BY_LOGIC_ID: u32 = 0x201;
pub const TARGET_BY_PLACE: u32 = 0x202;
pub const TARGET_BY_TYPE: u32 = 0x203;
pub const TARGET_NOT_DEFINED: u32 = 0x204;
/// A full placement matrix, as a placement's build order names its site (docs/32).
pub const TARGET_BY_MATRIX: u32 = 0x206;
/// Where an order goes in the unit's list (`varset.var`'s `INSERT_ORDER_*`).
pub const INSERT_TO_END: u32 = 1;
pub const INSERT_TO_START: u32 = 2;
pub const INSERT_REPLACE: u32 = 3;

/// What an order aims at (the packet's `+0xc` and `+0x10`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// `TARGET_NOT_DEFINED` (0x204) with the value 0.
    NotDefined,
    /// `TARGET_NOT_DEFINED` with −1: seek and destroy's any.
    Any,
    LogicId(i32),
    TypeMask(u32),
    /// `TARGET_BY_PLACE`: x, y and z. A script's two words give x and y, and z stays 0
    /// (docs/31, "Mission 03's last battle").
    Place([f32; 3]),
    /// `TARGET_BY_MATRIX` (0x206): a placement turned about z, its origin x, y, z and the
    /// turn.
    Placement([f32; 4]),
}

/// One order packet (`iron3d.dll:0x1007d000`), given with `INSERT_ORDER_REPLACE`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Order {
    pub code: i32,
    pub parameter: i32,
    pub target: Target,
}

/// A row of the wingman table (`iron3d.dll:0x10105150`): its string, and what enables it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub string: u32,
    /// Needs the driven unit's current target, not of the player's clan.
    pub needs_target: bool,
    /// Needs that target to be a building (pick mode 2).
    pub building: bool,
    /// Needs every chosen unit's record `+0x30` to be 1 or 2.
    pub capturers: bool,
}

/// The wingman menu's rows, in key order 1–7.
pub const ROWS: [Row; 7] = [
    Row { string: 5000, needs_target: false, building: false, capturers: false },
    Row { string: 5008, needs_target: false, building: false, capturers: false },
    Row { string: 5002, needs_target: false, building: false, capturers: true },
    Row { string: 5003, needs_target: false, building: false, capturers: false },
    Row { string: 5004, needs_target: true, building: false, capturers: false },
    Row { string: 5005, needs_target: true, building: true, capturers: true },
    Row { string: 5007, needs_target: false, building: false, capturers: false },
];

/// The driven unit's current target as the rows test it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picked {
    pub logic_id: i32,
    pub building: bool,
    pub type_word: u32,
    pub own_clan: bool,
}

/// Whether row `row` is enabled (`iron3d.dll:0x1007acb4`–`0x1007ad7e`) for a target and
/// whether every chosen unit may capture.
pub fn enabled(row: usize, target: Option<Picked>, capturers: bool) -> bool {
    let Some(r) = ROWS.get(row) else { return false };
    let target = target.filter(|t| !t.own_clan);
    if r.needs_target && target.is_none() {
        return false;
    }
    if r.building && !target.is_some_and(|t| t.building && t.type_word != UNCAPTURABLE_BUILDING) {
        return false;
    }
    !r.capturers || capturers
}

/// The order row `row` gives (the dispatcher's cases `0x100792ab`…`0x100794aa`), the
/// driven unit being `leader`.
pub fn order_for(row: usize, leader: i32, target: Option<Picked>) -> Option<Order> {
    let order = |code, parameter, target| Some(Order { code, parameter, target });
    match row {
        0 => order(STAYGROUND, 0, Target::NotDefined),
        1 => order(FOLLOW, FOLLOW_RADIUS, Target::LogicId(leader)),
        2 => order(SEARCH, 0, Target::TypeMask(CAPTURE_TYPES)),
        3 => order(SEARCH, 0, Target::Any),
        4 => order(ATTACK, 0, Target::LogicId(target?.logic_id)),
        5 => order(SEARCH, 0, Target::LogicId(target.filter(|t| t.building)?.logic_id)),
        6 => order(RELOAD, 0, Target::NotDefined),
        _ => None,
    }
}

/// The selector's states (`iron3d.dll:0x1006db40`, `+8`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Off,
    Picking,
    Ordering,
}

/// What a digit did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Digit {
    /// The selector is off: the digit goes on to the rest of the input.
    Passed,
    /// Taken, and nothing more to do.
    Taken,
    /// Taken: give row n (0-based).
    Row(usize),
}

/// The wingman selector: its state and the chosen wingmen, by their place in the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selector {
    pub state: State,
    pub chosen: Vec<usize>,
}

impl Selector {
    /// The tilde, with `wingmen` in the list: off, it chooses every wingman and opens the
    /// menu, or with Shift held empties the choice and starts picking; picking, it opens
    /// the menu when anyone is chosen and closes otherwise; ordering, it closes.
    pub fn tilde(&mut self, shift: bool, wingmen: usize) {
        match self.state {
            State::Off if wingmen == 0 => {}
            State::Off if shift => {
                self.chosen.clear();
                self.state = State::Picking;
            }
            State::Off => {
                self.chosen = (0..wingmen).collect();
                self.state = State::Ordering;
            }
            State::Picking if !self.chosen.is_empty() => self.state = State::Ordering,
            State::Picking | State::Ordering => self.close(),
        }
    }

    /// Digit `n`, 1–9 (`0x100710fa`): picking, it toggles wingman n; ordering, it gives
    /// row n.
    pub fn digit(&mut self, n: usize, wingmen: usize) -> Digit {
        match self.state {
            State::Off => Digit::Passed,
            State::Picking => {
                let i = n.wrapping_sub(1);
                if i < wingmen {
                    match self.chosen.iter().position(|&c| c == i) {
                        Some(at) => {
                            self.chosen.remove(at);
                        }
                        None => self.chosen.push(i),
                    }
                }
                Digit::Taken
            }
            State::Ordering => Digit::Row(n.wrapping_sub(1)),
        }
    }

    /// After an order, or the tilde: closed and the choice emptied (`0x1006e2a9`).
    pub fn close(&mut self) {
        self.chosen.clear();
        self.state = State::Off;
    }
}

/// The acknowledgement a unit speaks for its record's `+0x30` (`iron3d.dll:0x1008e840`):
/// 1 or 2 the `_S` voices, 4 or 5 the `_B` voices, anything else the plain ones.
pub fn voice_suffix(class: u8) -> &'static str {
    match class {
        1 | 2 => "_S",
        4 | 5 => "_B",
        _ => "",
    }
}

pub const ACKNOWLEDGEMENTS: [&str; 5] =
    ["VOICE_ACKNOWLEDGE", "VOICE_AFFIRMATIVE", "VOICE_YES_SIR", "VOICE_OK", "VOICE_EXECUTE"];

/// The pick among the five (`0x1008e690`): a 16-bit xorshift that never repeats the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoicePick {
    state: u16,
    last: Option<usize>,
}

impl Default for VoicePick {
    fn default() -> Self {
        Self { state: 0xACE1, last: None }
    }
}

impl VoicePick {
    /// STAND-IN: docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured
    /// -- the xorshift's shifts and seed are not transcribed: 7, 9, 8 from 0xACE1.
    pub fn pick(&mut self) -> usize {
        loop {
            let mut x = self.state;
            x ^= x << 7;
            x ^= x >> 9;
            x ^= x << 8;
            self.state = x;
            let i = usize::from(x) % ACKNOWLEDGEMENTS.len();
            if Some(i) != self.last {
                self.last = Some(i);
                return i;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tilde_chooses_everyone_or_with_shift_lets_digits_pick() {
        let mut s = Selector::default();
        s.tilde(false, 0);
        assert_eq!(s.state, State::Off, "no wingmen, nothing");
        s.tilde(false, 3);
        assert_eq!((s.state, s.chosen.clone()), (State::Ordering, vec![0, 1, 2]));
        assert_eq!(s.digit(2, 3), Digit::Row(1));
        s.tilde(false, 3);
        assert_eq!((s.state, s.chosen.len()), (State::Off, 0));

        s.tilde(true, 3);
        assert_eq!(s.state, State::Picking);
        assert_eq!(s.digit(2, 3), Digit::Taken);
        assert_eq!(s.digit(7, 3), Digit::Taken, "past the list");
        assert_eq!(s.chosen, vec![1]);
        s.digit(2, 3);
        s.tilde(false, 3);
        assert_eq!(s.state, State::Off, "picking with nobody chosen closes");
        s.tilde(true, 3);
        s.digit(3, 3);
        s.tilde(false, 3);
        assert_eq!((s.state, s.chosen.clone()), (State::Ordering, vec![2]));
        assert_eq!(Selector::default().digit(1, 3), Digit::Passed);
    }

    #[test]
    fn rows_need_a_hostile_target_or_a_building_and_give_their_orders() {
        let unit = Picked { logic_id: 7, building: false, type_word: 0x0100_8000, own_clan: false };
        let bridge = Picked { logic_id: 9, building: true, type_word: 0x8000_1000, own_clan: false };
        let friend = Picked { own_clan: true, ..unit };
        assert!(enabled(0, None, false) && enabled(1, None, false) && enabled(6, None, false));
        assert!(!enabled(4, None, true) && !enabled(4, Some(friend), true) && enabled(4, Some(unit), true));
        assert!(
            !enabled(5, Some(unit), true)
                && enabled(5, Some(bridge), true)
                && !enabled(5, Some(bridge), false)
        );
        assert!(!enabled(2, None, false) && enabled(2, None, true));
        assert_eq!(
            order_for(1, 1, None),
            Some(Order { code: FOLLOW, parameter: 50, target: Target::LogicId(1) })
        );
        assert_eq!(order_for(4, 1, Some(unit)).unwrap().target, Target::LogicId(7));
        assert_eq!(order_for(3, 1, None).unwrap().target, Target::Any);
        assert_eq!(order_for(5, 1, Some(unit)), None, "capture building wants a building");
    }

    #[test]
    fn the_acknowledgement_never_repeats_the_last_voice() {
        let mut pick = VoicePick::default();
        let mut last = pick.pick();
        for _ in 0..50 {
            let next = pick.pick();
            assert_ne!(next, last);
            last = next;
        }
        assert_eq!((voice_suffix(2), voice_suffix(4), voice_suffix(3)), ("_S", "_B", ""));
    }
}
