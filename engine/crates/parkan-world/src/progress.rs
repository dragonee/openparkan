//! A mission's progression as it plays: every clan's script's `Init`, the player clan's
//! `Mission` handler on its cadence, and each clan's takt walking its planner's problem
//! list — the drain, the `_Continue` pass, `Problems<n>` and the `_Start` pass. What the
//! game says for it, and the orders the scripts give. See `docs/34-progression.md` and
//! `docs/15-behaviour.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::{gamedir, scr};
use parkan_sim::orders::{self, Order, Target};
use parkan_sim::planner::{self, Action, Candidate, Planner, Problem};
use parkan_sim::progression::{self, ClanTakt, Notice, Progress};
use parkan_sim::script::{Args, Host, Interpreter};

use crate::console::{self, Command};
use crate::resources::{self, Messages, Sound, Sounds};

/// The interface strings the progression shows (`iron3d.dll`'s string table).
pub const STRING_MISSION_COMPLETE: u32 = 1012;
pub const STRING_MISSION_FAILED: u32 = 1013;
pub const STRING_VACANT_VEHICLE: u32 = 3040;
pub const STRING_OBJECTIVE_COMPLETE: u32 = 5040;
pub const STRING_OBJECTIVE_FAILED: u32 = 5041;
pub const STRING_IN_HISTORY: u32 = 6170;
/// The message box's headers (docs/35-hud.md, "The message box").
pub const STRING_FROM_SYSTEM: u32 = 1541;
pub const STRING_FROM_TRAINING: u32 = 3057;
pub const STRING_FROM_INFORMATION: u32 = 6214;
/// The outcome panel's lines (docs/34, "After the outcome").
pub const STRING_PRESS_ESC: u32 = 5082;
pub const STRING_PRESS_R: u32 = 3075;
pub const STRING_PRESS_L: u32 = 3076;
/// The sounds `ui/game_resources.cfg` binds for them.
pub const VOICE_OBJ_COMPLETE: &str = "VOICE_OBJ_COMPLETE";
pub const VOICE_MISSION_COMPLETE: &str = "VOICE_MISSION_COMPLETE";
pub const VOICE_MISSION_FAIL: &str = "VOICE_MISSION_FAIL";
pub const VOICE_UNIT_DETECTED: &str = "VOICE_UNIT_DETECTED";
pub const VOICE_ENEMY_DETECTED: &str = "VOICE_ENEMY_DETECTED";
pub const TARGET_SELECTED: &str = "TARGET_SELECTED";

/// The building types function 35 ranks its capture targets by, best first (docs/15, "What
/// the functions do"): a generator, a factory, a mine, a research centre, a storage.
pub const CAPTURE_RANK: [u32; 5] = [0x8000_0002, 0x8000_0010, 0x8000_0004, 0x8000_0400, 0x8000_0008];
/// What function 66 writes beside a building's type: a generator's, a factory's, and any
/// other building's.
pub const WORTH_GENERATOR: f32 = 0.04;
pub const WORTH_FACTORY: f32 = 0.05;
/// The `dMax*` variables function 11 tests a type against, with the type each one bounds.
pub const LIMITS: [(&str, u32); 6] = [
    ("dMaxBuilder", planner::ROBOT_BUILDER),
    ("dMaxTransport", planner::ROBOT_TRANSPORT),
    ("dMaxMine", 0x8000_0004),
    ("dMaxPlant", 0x8000_0010),
    ("dMaxStorage", 0x8000_0008),
    ("dMaxTower", 0x8010_0000),
];

/// Whom a line is from, which heads its message box (`iron3d.dll:0x1007f750`, docs/35-hud.md,
/// "The message box"): the game's own lines are kind 2, the System's; a script's message
/// is the Information assistant's when it sets `info_system`, the Training assistant's
/// otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sender {
    System,
    Training,
    Information,
}

impl Sender {
    /// The string the box's header reads.
    pub fn header(self) -> u32 {
        match self {
            Sender::System => STRING_FROM_SYSTEM,
            Sender::Training => STRING_FROM_TRAINING,
            Sender::Information => STRING_FROM_INFORMATION,
        }
    }
}

/// What the game says: a line into the message history, a voice queued behind the ones
/// playing (`ISoundServer` slot 4), or a sound played at once (slot 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Say {
    Text(Sender, String),
    Voice(Sound),
    Sound(Sound),
}

/// What the outcome panel shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomePanel {
    pub won: bool,
    pub title: String,
    pub lines: Vec<String>,
}

/// The unit handlers' base, which the SuperAI constructor finds by name (`ai.dll:0x1000165f`),
/// and event 8's step past it: `Hero_Teleported` (docs/27, "Teleport out").
pub const HANDLER_GENERATOR_FOUND: &str = "Mech_GeneratorFound";
pub const EVENT_HERO_TELEPORTED: usize = 3;
/// The building handlers' base (`0x10001686`) and event 7's step past it: `Fort_Captured`,
/// which a clan runs for a building it has just lost (docs/27, "Teleport out", for the
/// dispatcher's two tables).
pub const HANDLER_FORT_TASK_COMPLETE: &str = "Fort_Task_Complete";
pub const EVENT_FORT_CAPTURED: usize = 1;

/// The variable the SuperAI's constructor writes the game level's difficulty into
/// (`ai.dll:0x10005d17`).
pub const DIFFICULTY_VARIABLE: &str = "fDifficulty";

/// Function 15's answer when no object answers the logical id (`ai.dll:0x10008376`), and
/// when the unit takes the order.
pub const ORDER_NO_UNIT: u32 = 5;
pub const ORDER_TAKEN: u32 = 1;

/// An order a script gave through function 15: the unit's logical id, the order, and where
/// it goes in the unit's list (`INSERT_ORDER_*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptOrder {
    pub id: i32,
    pub order: Order,
    pub insert: u32,
}

/// The handler every clan's takt runs (docs/34, "When the Mission handler runs").
pub use parkan_sim::planner::HANDLER_PROBLEMS;

/// Another clan's script: its SuperAI runs `Init` once, as every clan's does, and its takt
/// walks its planner, but it never runs `Mission`, which the game frame runs for the local
/// player's clan alone (docs/34, "When the Mission handler runs").
pub struct ClanScript {
    pub clan: i64,
    pub base: [f32; 2],
    pub script: Interpreter,
    /// Its handler names, which a raise looks its `_Start`/`_Continue` pair up in.
    pub handlers: Vec<String>,
    pub takt: ClanTakt,
    pub planner: Planner,
}

/// Whose script a run is for: the player clan's, or the clan of `others[i]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    Player,
    Other(usize),
}

pub struct Progression {
    /// The player clan's script, when it has one that loads, and its handler names.
    pub script: Option<Interpreter>,
    pub handlers: Vec<String>,
    /// Its clan's takt, which walks its planner as every other clan's does.
    pub takt: ClanTakt,
    pub planner: Planner,
    /// Every other clan's script that loads.
    pub others: Vec<ClanScript>,
    /// The orders the scripts gave and the play has not yet handed their units.
    pub orders: Vec<ScriptOrder>,
    pub progress: Progress,
    /// The player's clan, and its base's centre.
    pub clan: i64,
    pub base: [f32; 2],
    /// Each clan's relation word towards each other, as the loader filed them: 0 is an enemy
    /// (docs/25, "Clan relations").
    pub relations: Vec<Vec<u32>>,
    /// Each clan's minds not held, which function 49 answers. [`Progression::load`] fills it
    /// from the mission's own counts and `Play` refreshes it each tick (docs/23, "The bot
    /// limit is the clan's mind count").
    pub free_minds: Vec<usize>,
    /// Each clan's minds a build's start holds reserved, as `Play` last counted them: a
    /// clan's takt frees them before its handlers run (`ai.dll:0x10005910` ends in the sweep
    /// `0x10006580`, ahead of `Problems<n>`), and names the clan in `swept`.
    pub reserved: Vec<usize>,
    /// The clans whose takt swept their reservations since `Play` last looked.
    pub swept: Vec<i64>,
    pub messages: Messages,
    pub sounds: Sounds,
    pub strings: BTreeMap<u32, String>,
    /// Function-table slots the script called that the engine does not answer.
    pub unanswered: BTreeSet<i32>,
    /// Each objective's text as `mission.cfg` writes it, in script order.
    pub objective_texts: Vec<String>,
    /// The mission's `script` block, each line as the console reads it.
    pub console_lines: Vec<ConsoleLine>,
    /// What function 57 ran that the play has not yet carried out.
    pub console: Vec<ConsoleCall>,
    /// The logical ids the next unit and the next building made in play take, as `Play` last
    /// counted them: the areal map keeps one counter for robots and one for buildings, each
    /// bumped before it is used (`ArealMap.dll:0x1002b305`, `0x1002b3e9`; docs/23).
    pub next_ids: (i32, i32),
}

/// One line of a mission's `script` block: its key, `script1` and on, what the console makes
/// of it, and the Type word of the file a `create` or `bcreate` names, the word after its
/// magic.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsoleLine {
    pub key: String,
    pub command: Option<Command>,
    pub type_word: u32,
}

/// A console line function 57 ran: the command, the Type word of its file, and the logical id
/// its object is on its clan's list under already (0 for `death`).
#[derive(Clone, Debug, PartialEq)]
pub struct ConsoleCall {
    pub command: Command,
    pub type_word: u32,
    pub id: i32,
}

/// The engine below the SuperAI, as the script's calls reach it.
struct Answers<'a> {
    progress: &'a mut Progress,
    planner: &'a mut Planner,
    /// The running script's handler names: a raise is abandoned unless both of its are there.
    handlers: &'a [String],
    clan: i64,
    /// The running clan's seconds clock, which functions 59 and 60 read.
    clock: u32,
    relations: &'a [Vec<u32>],
    notices: &'a mut Vec<Notice>,
    unanswered: &'a mut BTreeSet<i32>,
    orders: &'a mut Vec<ScriptOrder>,
    console_lines: &'a [ConsoleLine],
    console: &'a mut Vec<ConsoleCall>,
    next_ids: &'a mut (i32, i32),
    /// Every clan's free minds, which a `create` takes one of.
    minds: &'a mut [usize],
    /// The clan's free minds, function 49's answer.
    free_minds: usize,
    /// The `dMax*` limits function 11 tests, as the script's variables stand.
    limits: Vec<(u32, usize)>,
}

impl Answers<'_> {
    /// The clan table's own test, as functions 35, 36, 64 and 71 make it (`ai.dll:0x1000f2c4`):
    /// another clan whose relation word from the running clan is 0.
    fn hostile(&self, other: i64) -> bool {
        other != self.clan
            && usize::try_from(self.clan)
                .ok()
                .zip(usize::try_from(other).ok())
                .and_then(|(us, them)| self.relations.get(us)?.get(them).copied())
                == Some(mission::RELATION_HOSTILE)
    }

    /// Whether the script defines a handler of that name.
    fn defines(&self, name: &str) -> bool {
        self.handlers.iter().any(|h| h == name)
    }

    /// The clan's own units and buildings, the list functions 13, 14, 25 and 38 pick from.
    fn own(&self) -> Vec<Candidate> {
        self.progress.own(self.clan)
    }

    /// A console line function 57 runs (docs/15, "What the console's `create`, `bcreate` and
    /// `death` do"). The game runs it before the call returns, and `create` and `bcreate` file
    /// their object on its clan's list as they make it (SuperAI slot 4, events 1 and 2,
    /// `iron3d.dll:0x1003d560`, `0x1003dad8`), so the rest of the script's run counts it: C02
    /// Mission 02's `Mission` handler asks whether `Enm2` has any robots straight after its three
    /// `create`s. The object is filed here under the id it will have, and the play makes it
    /// once the run is over.
    fn run_console(&mut self, line: &ConsoleLine) {
        let Some(command) = line.command.clone() else { return };
        let id = match &command {
            // Made with create flag 8 (`0x10077664`) unless the game is the auto-demo: it takes
            // its clan's first free mind, and with none free it is not made at all
            // (`ArealMap.dll:0x100152a9`-`0x10015307`; docs/23).
            &Command::Create { x, y, clan, .. } => {
                let Some(free) = usize::try_from(clan).ok().and_then(|c| self.minds.get_mut(c)) else {
                    return;
                };
                if *free == 0 {
                    return;
                }
                *free -= 1;
                if i64::from(clan) == self.clan {
                    self.free_minds = self.free_minds.saturating_sub(1);
                }
                let id = self.next_ids.0;
                self.next_ids.0 += 1;
                self.progress.join(
                    id,
                    i64::from(clan),
                    line.type_word,
                    Vec3::new(x as f32, y as f32, 0.0),
                    0.0,
                );
                id
            }
            &Command::BCreate { x, y, z, clan, .. } => {
                let id = self.next_ids.1;
                self.next_ids.1 += 1;
                let at = Vec3::new(x as f32, y as f32, z as f32);
                self.progress.place_building(id, i64::from(clan), line.type_word, at);
                id
            }
            Command::Death { .. } => 0,
        };
        self.console.push(ConsoleCall { command, type_word: line.type_word, id });
    }

    /// The place an order's target names, from operand `at` on: a logic id's object, or a
    /// place's two words.
    fn target_place(&self, args: &Args<'_>, at: usize) -> Option<[f32; 2]> {
        match args.dword(at) {
            orders::TARGET_BY_LOGIC_ID => self.progress.contact(args.dword(at + 1) as i32).map(|c| c.at),
            orders::TARGET_BY_PLACE => {
                Some([args.dword(at + 1) as i32 as f32, args.dword(at + 2) as i32 as f32])
            }
            _ => None,
        }
    }

    /// The strength held against the clan asking within `radius` of a place.
    fn held(&self, at: [f32; 2], radius: f32) -> f32 {
        self.progress.strength_near(|c| self.hostile(c), at[0], at[1], radius)
    }

    /// Of `objects`, the one the least strength stands within `radius` of, and that strength:
    /// how the six picking functions choose (`0x1000f12e`). The first of an equal pair is
    /// kept, as the game's loop takes a candidate only when it is strictly below the best.
    fn least_defended(&self, objects: impl Iterator<Item = Candidate>, radius: f32) -> Option<(i32, f32)> {
        objects
            .map(|c| (c.id, self.held(c.at, radius)))
            .reduce(|best, one| if one.1 < best.1 { one } else { best })
    }

    /// Write a strength into an "out" operand. A `DWORD` operand takes it truncated, which
    /// is what the float setter does, so a script's `dTemp3` holds whole strengths alone.
    fn write_strength(&self, args: &mut Args<'_>, at: usize, strength: f32) {
        args.set_float(at, strength);
    }

    /// The order packet operands 1 to 10 make, shared by functions 15 and 28.
    fn packet(&self, args: &Args<'_>) -> (Order, u32) {
        let word = |i: usize| args.dword(i) as i32;
        let target = match args.dword(8) {
            orders::TARGET_BY_PLACE => Target::Place([word(9) as f32, word(10) as f32, 0.0]),
            orders::TARGET_BY_LOGIC_ID => Target::LogicId(word(9)),
            orders::TARGET_BY_TYPE => Target::TypeMask(args.dword(9)),
            orders::TARGET_BY_NAME => Target::Select(args.dword(9)),
            _ => Target::NotDefined,
        };
        (Order { code: word(1), parameter: word(3), target }, args.dword(2))
    }

    /// Function 15's body: give the unit of logical id `id` the packet.
    fn give(&mut self, id: i32, order: Order, insert: u32) -> u32 {
        if !self.progress.knows(id) {
            return ORDER_NO_UNIT;
        }
        self.orders.push(ScriptOrder { id, order, insert });
        ORDER_TAKEN
    }

    /// Functions 13 and 14: pick a unit of the clan's own.
    fn pick(&mut self, args: &Args<'_>) -> u32 {
        let mode = args.dword(0);
        let (type_word, to) = match mode {
            planner::UNIT_ANY_NEAREST_CAPTURER => (0, self.target_place(args, 1)),
            _ => (args.dword(1), None),
        };
        let mut own = self.own();
        // The nearest capturer ranks by how soon each would arrive; the rest take the first
        // on the clan's list.
        if mode == planner::UNIT_ANY_NEAREST_CAPTURER
            && let Some(to) = to
        {
            own.sort_by(|a, b| a.reach(to).total_cmp(&b.reach(to)));
        }
        self.planner.pick_unit(mode, type_word, to, &own).map_or(planner::ERROR, |id| id as u32)
    }

    /// Function 25: open a group and fill it.
    fn group(&mut self, args: &mut Args<'_>) -> u32 {
        let take = args.dword(0);
        let mut amount = args.float(1);
        let mut own = self.own();
        if take == planner::TAKE_BY_HITS
            && let Some(to) = self.target_place(args, 2)
        {
            let far = |c: &Candidate| (c.at[0] - to[0]).hypot(c.at[1] - to[1]);
            own.sort_by(|a, b| far(a).total_cmp(&far(b)));
        }
        let made = self.planner.make_group(take, &own, &mut amount);
        if take == planner::TAKE_BY_HITS {
            self.write_strength(args, 1, amount);
        }
        made.map_or(planner::ERROR, |g| g as u32)
    }
}

impl Host for Answers<'_> {
    fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32 {
        let int = |args: &Args<'_>, i: usize| i64::from(args.dword(i) as i32);
        match function {
            // Stubs: 0 leaves 1 in the result, 1 reads a float and drops it, 9 does nothing.
            0 => 1,
            1 | 9 => 0,
            // Raise the problem the code's *variable name* names, with its weight, its life
            // counter, its drain and its three parameters. A raise whose `_Start` or
            // `_Continue` the script does not define is abandoned and makes no record
            // (`0x10005aa1`): 21 of the corpus's 176 raises are dead that way.
            2 => {
                let Some(name) = args.name(0).map(str::to_owned) else { return 0 };
                if !self.defines(&format!("{name}{}", planner::START))
                    || !self.defines(&format!("{name}{}", planner::CONTINUE))
                {
                    return 0;
                }
                let p = [args.dword(4), args.dword(5), args.dword(6)];
                self.planner.raise(&name, args.dword(0), args.float(1), int(args, 2), int(args, 3), p);
                0
            }
            // Attach a unit to the running problem, and release its units.
            6 => {
                self.planner.attach(args.dword(0) as i32, args.dword(1) == planner::UNIT_CRITICAL);
                0
            }
            7 => {
                self.planner.release_units();
                0
            }
            8 => {
                self.planner.set_state(args.dword(0));
                0
            }
            // 1 when the clan already holds as many of the type as its `dMax*` allows.
            //
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- the second half of the
            // test, a per-type counter the brain keeps at `+0x3e0`..`+0x3f8`, is not read; a
            // type no `dMax*` bounds — every robot the scripts build is `ROBOT_BATTLEUNIT`,
            // which none does — answers `FALSE`.
            11 => {
                let type_word = args.dword(0);
                let held = self.progress.count_type(self.clan, type_word);
                u32::from(self.limits.iter().any(|&(t, max)| t == type_word && held >= max))
            }
            12 => self.planner.current().map_or(0.0, |p| p.weight).to_bits(),
            13 | 14 => self.pick(args),
            // An order packet for the unit of any clan with that logical id (`0x10008054`).
            //
            // STAND-IN: docs/34-progression.md#what-the-scripts-ask--read-and-measured-1 -- the
            // unit's own answer (1 taken, 0 refused) is not waited for: the order is handed to
            // the unit after the handler's run, and a known id answers 1. The scripts' own
            // guard is ahead of it — eight of the nine build handlers drop the problem when the
            // clan has no free mind before they ever order (docs/23).
            15 => {
                let (order, insert) = self.packet(args);
                self.give(args.dword(0) as i32, order, insert)
            }
            // The base's centre and the clan's number, written into the three operands.
            19 => {
                args.set_float(0, self.planner.base[0]);
                args.set_float(1, self.planner.base[1]);
                args.set_dword(2, self.clan as u32);
                0
            }
            24 => {
                self.planner.release_group(args.dword(0) as usize);
                0
            }
            25 => self.group(args),
            // File an action on the running problem.
            27 => {
                let action =
                    Action { action: args.dword(0), kind: args.dword(1), target: args.dword(2) as i32 };
                self.planner.expect(action);
                0
            }
            // Give every unit of the group the order; 1 when they all took it.
            28 => {
                let (order, insert) = self.packet(args);
                let units = self.planner.group(args.dword(0) as usize).map(|g| g.units.clone());
                let Some(units) = units else { return 0 };
                let taken =
                    units.into_iter().map(|id| self.give(id, order, insert)).all(|a| a == ORDER_TAKEN);
                u32::from(taken)
            }
            29 => self.planner.parameter(args.dword(0) as usize),
            30 => {
                let notices = self.progress.call(int(args, 0), int(args, 1));
                self.notices.extend(notices);
                0
            }
            31 => self.progress.robots(int(args, 0), i64::from(args.dword(1))) as u32,
            32 => u32::from(self.progress.areals.holds(int(args, 0), int(args, 1))),
            // Run `Problems<n>` as the problem handler from now on.
            33 => {
                self.planner.handler = format!("Problems{}", args.dword(0));
                0
            }
            34 => self.progress.count_type(self.clan, args.dword(0)) as u32,
            // The enemy object no `PBM_BUILDING_CAPTURE` is raised for, the least defended of
            // its rank, and that strength.
            35 => {
                let radius = args.float(0);
                let raised = |id: i32| self.planner.raised_for(planner::PBM_BUILDING_CAPTURE, id as u32);
                let open: Vec<Candidate> =
                    self.progress.contacts().filter(|c| self.hostile(c.clan) && !raised(c.id)).collect();
                let rank = |c: &Candidate| {
                    CAPTURE_RANK.iter().position(|&t| t == c.type_word).unwrap_or(CAPTURE_RANK.len())
                };
                let best = open.iter().map(rank).min();
                let found = best.and_then(|best| {
                    self.least_defended(open.iter().copied().filter(|c| rank(c) == best), radius)
                });
                match found {
                    Some((id, strength)) => {
                        self.write_strength(args, 1, strength);
                        id as u32
                    }
                    None => planner::ERROR,
                }
            }
            // The enemy object with the least held against it, and the one inside the base
            // radius with the least about it.
            36 | 64 => {
                let radius = args.float(0);
                let inside = |c: &Candidate| {
                    function == 36
                        || (c.at[0] - self.planner.base[0]).hypot(c.at[1] - self.planner.base[1])
                            <= self.planner.radius
                };
                let open: Vec<Candidate> =
                    self.progress.contacts().filter(|c| self.hostile(c.clan) && inside(c)).collect();
                match self.least_defended(open.into_iter(), radius) {
                    Some((id, strength)) => {
                        self.write_strength(args, 1, strength);
                        id as u32
                    }
                    None => planner::ERROR,
                }
            }
            // The clan's own building with the least held against it, and its factory.
            //
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- neither call passes a
            // radius, and what the handler measures over is not read: the base radius here.
            37 | 67 => {
                let radius = self.planner.radius;
                let mine: Vec<Candidate> = self
                    .own()
                    .into_iter()
                    .filter(|c| {
                        c.type_word & parkan_sim::behaviour::BUILDING_BIT != 0
                            && (function == 37 || c.type_word == crate::play::BUILDING_PLANT)
                    })
                    .collect();
                match self.least_defended(mine.into_iter(), radius) {
                    Some((id, strength)) => {
                        self.write_strength(args, 0, strength);
                        id as u32
                    }
                    None => planner::ERROR,
                }
            }
            // The summed strength of that clan's battle units, free ones or all of them.
            38 => {
                let clan = int(args, 0);
                let free_only = args.dword(1) == planner::FREE_UNITS && clan == self.clan;
                let total: f32 = self
                    .progress
                    .own(clan)
                    .into_iter()
                    .filter(|c| c.battle() && (!free_only || self.planner.free(c.id)))
                    .map(|c| c.full)
                    .sum();
                total as u32
            }
            // 1 when the id names an object.
            39 => u32::from(self.progress.knows(args.dword(0) as i32)),
            // Load `UNITS\UNITS\AI\` into the design store. The store itself lives in the
            // play, which holds the assembly the designs load through, so this only marks it
            // wanted; a clan whose script never calls it builds nothing by name.
            43 => {
                self.planner.store_loaded = true;
                0
            }
            // The strength standing within `f` of a place or an object, of that clan or the
            // enemy's for `ERROR`.
            //
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- every call site assigns
            // it to a `DWORD`, which takes the result slot's four bytes as they stand, so the
            // handler must leave a whole number there: the strength is truncated.
            44 => {
                let radius = args.float(0);
                let clan = int(args, 1);
                let Some(at) = self.target_place(args, 2) else { return 0 };
                let strength = if args.dword(1) == planner::ERROR {
                    self.held(at, radius)
                } else {
                    self.progress.strength_near(|c| c == clan, at[0], at[1], radius)
                };
                strength.max(0.0) as u32
            }
            45 => self.planner.units_strength(&self.own()).max(0.0) as u32,
            // An enemy within range of the base that no `PBM_BASE_DEFENCE` is raised for: the
            // hero if one is there, else the strongest.
            47 => {
                let range = args.float(0);
                let base = self.planner.base;
                let near = |c: &Candidate| (c.at[0] - base[0]).hypot(c.at[1] - base[1]) <= range;
                let raised = |id: i32| self.planner.raised_for(planner::PBM_BASE_DEFENCE, id as u32);
                let open: Vec<Candidate> = self
                    .progress
                    .contacts()
                    .filter(|c| self.hostile(c.clan) && near(c) && !raised(c.id))
                    .collect();
                let hero = open.iter().find(|c| c.type_word & planner::ROBOT_HERO == planner::ROBOT_HERO);
                let found = hero.copied().or_else(|| {
                    open.iter().copied().reduce(|best, one| if one.full > best.full { one } else { best })
                });
                match found {
                    Some(c) => {
                        self.write_strength(args, 1, c.full);
                        c.id as u32
                    }
                    None => planner::ERROR,
                }
            }
            49 => self.free_minds as u32,
            // A unit of the running problem executing that order.
            50 => {
                let code = args.dword(0) as i32;
                let own = self.own();
                self.planner.unit_on_order(code, &own).map_or(planner::ERROR, |id| id as u32)
            }
            51 => {
                self.planner.reserve(args.dword(0) as i32, args.dword(1) != 0);
                0
            }
            52 => self.progress.owner(args.dword(0) as i32),
            // Channel 2 of the message callback (`ai.dll:0x1000e4f0`): the mission's `script%d`
            // line for the first value, run as a console command; the second value is never
            // read, and neither is the result (docs/15, "Channel 2 runs a line of the
            // mission's `script` block").
            57 => {
                let key = format!("script{}", args.dword(0) as i32);
                let lines = self.console_lines;
                if let Some(line) = lines.iter().find(|l| l.key == key) {
                    self.run_console(line);
                }
                0
            }
            // The clan's seconds clock plus a delay, whole seconds (`ai.dll:0x1000e643`).
            59 => self.clock.wrapping_add(args.dword(0)),
            // 1 once that time has passed, else `ERROR` (`0x1000e6a0`): the comparison is
            // unsigned and strict, so a time is passed only once the clock is above it.
            60 => {
                if args.dword(0) < self.clock {
                    1
                } else {
                    progression::ERROR
                }
            }
            // The object's type word, and the same with what a generator or a factory is
            // worth written beside it.
            61 | 66 => {
                let Some(c) = self.progress.contact(args.dword(0) as i32) else { return planner::ERROR };
                if function == 66 {
                    let worth = match c.type_word {
                        0x8000_0002 => WORTH_GENERATOR,
                        0x8000_0010 => WORTH_FACTORY,
                        _ => 0.0,
                    };
                    args.set_float(1, worth);
                }
                c.type_word
            }
            62 => {
                if let Some(p) = self.planner.current_mut() {
                    p.weight = args.float(0);
                }
                0
            }
            // 1 when the object stands inside the base radius.
            63 => {
                let base = self.planner.base;
                let radius = self.planner.radius;
                let inside = self
                    .progress
                    .contact(args.dword(0) as i32)
                    .is_some_and(|c| (c.at[0] - base[0]).hypot(c.at[1] - base[1]) <= radius);
                u32::from(inside)
            }
            // Recompute the base radius from the clan's buildings within range of the centre.
            68 => {
                let range = args.float(0);
                let base = self.planner.base;
                let far = |c: &Candidate| (c.at[0] - base[0]).hypot(c.at[1] - base[1]);
                self.planner.radius = self
                    .own()
                    .iter()
                    .filter(|c| c.type_word & parkan_sim::behaviour::BUILDING_BIT != 0)
                    .map(far)
                    .filter(|&d| d <= range)
                    .fold(0.0, f32::max);
                0
            }
            // How far down its ranking the clan's next build may reach. Negative is ignored,
            // and the result is 1.
            69 => {
                let n = args.dword(0) as i32;
                if n >= 0 {
                    self.planner.spread = n as u32;
                }
                1
            }
            70 => self.planner.random(args.dword(0)),
            // The enemy object of exactly that type with the least strength about it, and that
            // strength written into the third operand, truncated (`0x1000f12e`).
            71 => {
                let (type_word, radius) = (args.dword(1), args.float(0));
                match self.progress.enemy_of_type(|other| self.hostile(other), type_word, radius) {
                    Some((id, strength)) => {
                        self.write_strength(args, 2, strength);
                        id as u32
                    }
                    None => progression::ERROR,
                }
            }
            // The object's property `0x201`, its size class.
            72 => self
                .progress
                .contact(args.dword(0) as i32)
                .map_or(planner::ERROR, |c| u32::from(c.size_class)),
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- the engine answers the
            // functions the campaign's scripts need for their messages, objectives, orders,
            // targets and planning; any other call does nothing and answers 0. Left
            // unanswered: 3, 16 and 17, the clan's power and resource totals a SuperAI asks
            // for; 18 and 40, a building site and a mineral place, which want the place list
            // function 43 loads; 41, 56 and 65, flags of that same object; and the fourteen
            // no shipped script calls.
            other => {
                self.unanswered.insert(other);
                0
            }
        }
    }
}

fn number(v: Value) -> i64 {
    match v {
        Value::Int(i) => i64::from(i),
        Value::Float(f) => f as i64,
    }
}

/// A script by the path a clan record names, with its formulas and `varset.var`.
fn load_script(game: &Path, path: &str) -> Result<Interpreter> {
    let file = |suffix: &str| {
        let name = format!("{path}.{suffix}");
        let found = gamedir::resolve(game, &name).with_context(|| format!("no {name}"))?;
        std::fs::read(&found).with_context(|| format!("cannot read {}", found.display()))
    };
    let (dir, _) = path.rsplit_once(['\\', '/']).unwrap_or(("", path));
    let varset = gamedir::resolve(game, &format!("{dir}/{}", scr::VARSET)).context("no varset.var")?;
    let table = scr::parse_variables(&std::fs::read(&varset)?, scr::VARSET)?;
    let script = scr::parse(&file("scr")?, path)?;
    let formulas = scr::parse_formulas(&file("fml").unwrap_or_default(), path)?;
    Ok(Interpreter::new(script, &formulas, &table)?)
}

/// Every handler name a script defines.
fn handler_names(script: &Interpreter) -> Vec<String> {
    script.script.handlers.iter().map(|h| h.name.clone()).collect()
}

/// The mission's `script` block as the console reads it, each `create` and `bcreate` with
/// its file's Type word.
fn console_lines(game: &Path, mission_dir: &Path) -> Result<Vec<ConsoleLine>> {
    let type_word = |file: &str| {
        let path = gamedir::resolve(game, &format!("{}{file}", console::UNITS_AUTO))?;
        let data = std::fs::read(path).ok()?;
        parkan_formats::objects::parse_unit(&data, file).ok().map(|u| u.kind)
    };
    Ok(resources::console_lines(mission_dir)?
        .into_iter()
        .map(|(key, text)| {
            let command = console::parse(&text);
            let type_word = match &command {
                Some(Command::Create { file, .. } | Command::BCreate { file, .. }) => {
                    type_word(file).unwrap_or(0)
                }
                _ => 0,
            };
            ConsoleLine { key, command, type_word }
        })
        .collect())
}

/// The logical ids the next robot and the next building take after `ids`: one more than the
/// highest of each counter, a building's carrying the top bit (docs/23).
pub fn next_ids(ids: impl IntoIterator<Item = i32>) -> (i32, i32) {
    let (mut robot, mut building) = (0, 0);
    for id in ids {
        if id >= 0 {
            robot = robot.max(id);
        } else if id != -1 {
            building = building.max(id & 0x7fff_ffff);
        }
    }
    (robot + 1, (0x8000_0000_u32 | (building as u32 + 1)) as i32)
}

impl Progression {
    /// The progression of `mission`, in `mission_dir`, for the clan of its object `hero`.
    /// Every placed unit with a logical id joins its clan's list and reports where it
    /// stands, and every clan's script runs its `Init` once (SuperAI slot 5), the placed
    /// units already in their clans (docs/34, "Mission 03").
    pub fn load(game: &Path, mission_dir: &Path, mission: &Mission, hero: usize) -> Result<Self> {
        let clan = mission.objects.get(hero).and_then(mission::Object::clan_id).unwrap_or(0);
        let record = usize::try_from(clan).ok().and_then(|c| mission.clans.get(c));
        let objectives = resources::objectives(mission_dir)?;
        let exempt: Vec<bool> = objectives.iter().map(|o| o.exempt).collect();
        let messages = Messages::load(game, mission_dir)?;
        let mut progress = Progress::new(&mission.routes, &exempt, messages.0.iter().map(|m| m.index));
        // Every building with a logical id answers function 52 with its owner, and 34 its type.
        for o in mission.objects.iter().filter(|o| o.kind == mission::KIND_BUILDING) {
            let type_word = o.property("Type").map_or(0, |p| number(p.value)) as u32;
            progress.place_building(
                o.logical_id,
                o.clan_id().unwrap_or(-1),
                type_word,
                Vec3::from_array(o.position),
            );
        }
        for o in mission.objects.iter().filter(|o| o.kind == mission::KIND_UNIT && o.logical_id >= 0) {
            let type_word = o.property("Type").map_or(0, |p| number(p.value)) as u32;
            progress.join(
                o.logical_id,
                o.clan_id().unwrap_or(-1),
                type_word,
                Vec3::from_array(o.position),
                0.0,
            );
        }
        // `fDifficulty` as the SuperAI's constructor writes it, out of the game level
        // (docs/15, "The planner"): every clan's script is built with the same one.
        let difficulty = planner::difficulty(crate::settings::game_level(game));
        let script = match record.map(|c| c.ai_script.as_str()).filter(|s| !s.is_empty()) {
            Some(path) => Some(load_script(game, path)?),
            None => None,
        };
        let script = script.map(|mut s| {
            s.set_float(DIFFICULTY_VARIABLE, difficulty);
            s
        });
        // Another clan's script that does not load leaves that clan without one.
        let others: Vec<ClanScript> = mission
            .clans
            .iter()
            .enumerate()
            .filter(|&(i, c)| i as i64 != clan && !c.ai_script.is_empty())
            .filter_map(|(i, c)| {
                let mut script = load_script(game, &c.ai_script).ok()?;
                script.set_float(DIFFICULTY_VARIABLE, difficulty);
                let takt = ClanTakt::new(0, i as u32 + 1);
                Some(ClanScript {
                    clan: i as i64,
                    base: c.base,
                    handlers: handler_names(&script),
                    script,
                    takt,
                    planner: Planner::new(c.base, i as u32 + 1),
                })
            })
            .collect();
        let base = record.map_or([0.0; 2], |c| c.base);
        // Each clan's minds less the robots it was placed with (docs/23); `Play` refreshes it.
        let free_minds = mission
            .clans
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let held = progress.robots(i as i64, i64::from(crate::play::CLASS_ROBOT));
                (c.minds as usize).saturating_sub(held)
            })
            .collect();
        let mut me = Self {
            handlers: script.as_ref().map(handler_names).unwrap_or_default(),
            script,
            takt: ClanTakt::new(0, clan as u32 + 1),
            planner: Planner::new(base, clan as u32 + 1),
            others,
            orders: Vec::new(),
            progress,
            clan,
            base,
            relations: mission::relation_words(&mission.clans),
            reserved: vec![0; mission.clans.len()],
            swept: Vec::new(),
            free_minds,
            messages,
            sounds: Sounds::open(game, mission_dir)?,
            strings: resources::game_strings(game).unwrap_or_default(),
            unanswered: BTreeSet::new(),
            objective_texts: objectives.into_iter().map(|o| o.text).collect(),
            console_lines: console_lines(game, mission_dir)?,
            console: Vec::new(),
            next_ids: next_ids(mission.objects.iter().map(|o| o.logical_id)),
        };
        me.run("Init");
        for i in 0..me.others.len() {
            me.with_clan(Which::Other(i), |script, host| {
                script.run_named("Init", host);
            });
        }
        Ok(me)
    }

    /// Clan `clan`'s planner, when it runs a script.
    pub fn planner_of(&self, clan: i64) -> Option<&Planner> {
        if clan == self.clan {
            return self.script.as_ref().map(|_| &self.planner);
        }
        self.others.iter().find(|o| o.clan == clan).map(|o| &o.planner)
    }

    /// Run `run` on one clan's script, with the engine below it answering its calls; what its
    /// calls to function 30 raised. A clan without a script does nothing.
    fn with_clan(
        &mut self,
        which: Which,
        run: impl FnOnce(&mut Interpreter, &mut Answers<'_>),
    ) -> Vec<Notice> {
        let Self {
            script,
            handlers,
            takt,
            planner,
            others,
            orders,
            progress,
            clan,
            relations,
            unanswered,
            free_minds,
            console_lines,
            console,
            next_ids,
            ..
        } = self;
        let (script, handlers, planner, clan, clock) = match which {
            Which::Player => (script.as_mut(), handlers.as_slice(), planner, *clan, takt.clock()),
            Which::Other(i) => match others.get_mut(i) {
                Some(o) => {
                    (Some(&mut o.script), o.handlers.as_slice(), &mut o.planner, o.clan, o.takt.clock())
                }
                None => return Vec::new(),
            },
        };
        let mut notices = Vec::new();
        if let Some(script) = script {
            // The `dMax*` variables as the script's own table stands, for function 11.
            let limits = LIMITS
                .iter()
                .filter_map(|&(name, type_word)| Some((type_word, script.dword(name)? as usize)))
                .collect();
            let mut answers = Answers {
                progress,
                planner,
                handlers,
                clan,
                clock,
                relations: relations.as_slice(),
                notices: &mut notices,
                unanswered,
                orders,
                console_lines: console_lines.as_slice(),
                console,
                next_ids,
                free_minds: usize::try_from(clan).ok().and_then(|c| free_minds.get(c)).copied().unwrap_or(0),
                minds: free_minds.as_mut_slice(),
                limits,
            };
            run(script, &mut answers);
        }
        notices
    }

    /// Run one of the player clan's script's handlers; what its calls to function 30 raised.
    pub fn run(&mut self, handler: &str) -> Vec<Notice> {
        self.with_clan(Which::Player, |script, host| {
            script.run_named(handler, host);
        })
    }

    /// The draw a clan's design pick makes over its spread: `(rand() + timeGetTime()) %
    /// (spread + 1)` in the game, one clan-long stream here (docs/15, "Function 69").
    pub fn draw(&mut self, clan: i64) -> usize {
        let planner = if clan == self.clan {
            Some(&mut self.planner)
        } else {
            self.others.iter_mut().find(|o| o.clan == clan).map(|o| &mut o.planner)
        };
        planner.map_or(0, |p| {
            let spread = p.spread;
            p.random(spread + 1) as usize
        })
    }

    /// Clan `clan`'s free minds, which function 49 answers, and the reservations among the
    /// held ones, as `Play` last counted them.
    pub fn set_free_minds(&mut self, clan: i64, free: usize, reserved: usize) {
        let Ok(c) = usize::try_from(clan) else { return };
        if let Some(slot) = self.free_minds.get_mut(c) {
            *slot = free;
        }
        if let Some(slot) = self.reserved.get_mut(c) {
            *slot = reserved;
        }
    }

    /// The first thing a clan's takt does to its minds (`ai.dll:0x10006580`, the end of the
    /// drain `0x10005910`): every entry that is not free and names no object is freed. A
    /// build's reservation (0) names none -- the areal map's logic ids count up from 1
    /// (`ArealMap.dll:0x1002b3ec`) -- so all of them go, before `Problems<n>` asks function 49.
    fn sweep(&mut self, clan: i64) {
        let Ok(c) = usize::try_from(clan) else { return };
        let reserved = self.reserved.get_mut(c).map_or(0, std::mem::take);
        if let Some(free) = self.free_minds.get_mut(c) {
            *free += reserved;
        }
        self.swept.push(clan);
    }

    fn which(&self, clan: i64) -> Option<Which> {
        if clan == self.clan {
            return Some(Which::Player);
        }
        self.others.iter().position(|o| o.clan == clan).map(Which::Other)
    }

    /// A hero at a teleport-out place, handed to the SuperAI of `clan`, the teleport's owner
    /// (`Behavior.dll:0x1000c7d0`, docs/27, "Teleport out"): slot 18 (`ai.dll:0x100020a0`) runs
    /// event 8 for a unit as the handler three after `Mech_GeneratorFound` (`0x10005e1c`),
    /// `Hero_Teleported` in every shipped script. What that clan's calls to function 30
    /// raised. A clan with no script, or a script with no such handler, does nothing.
    ///
    /// The game does nothing while a handler is already running (`0x10005d88`); here every
    /// handler runs to its end before the next is started, so none is.
    pub fn hero_teleported(&mut self, clan: i64) -> Vec<Notice> {
        // The game hands slot 18 the hero's id; the progression does not know it, and no
        // shipped `Hero_Teleported` reads `dCurrentSender`, so the variable is left alone.
        self.event(clan, HANDLER_GENERATOR_FOUND, EVENT_HERO_TELEPORTED, None)
    }

    /// A building of clan `clan` taken by somebody else: its own SuperAI runs the handler one
    /// after `Fort_Task_Complete`, `Fort_Captured` in every shipped script, with the
    /// building's logical id in `dCurrentSender` (`0x10005d7c`). `c2m3e` answers it by raising
    /// `PBM_BUILDING_INF_CAPTURE` on the building it has just lost.
    pub fn fort_captured(&mut self, clan: i64, building: i32) -> Vec<Notice> {
        self.event(clan, HANDLER_FORT_TASK_COMPLETE, EVENT_FORT_CAPTURED, Some(building))
    }

    /// The event dispatcher (`0x10005d70`): `dCurrentSender` takes the id and the handler
    /// `step` past `base` runs.
    fn event(&mut self, clan: i64, base: &str, step: usize, sender: Option<i32>) -> Vec<Notice> {
        let Some(which) = self.which(clan) else { return Vec::new() };
        self.with_clan(which, |script, host| {
            if let Some(sender) = sender {
                script.set_dword("dCurrentSender", sender as u32);
            }
            let handler = script.handler(base).map(|h| h + step);
            if let Some(handler) = handler.filter(|&h| h < script.script.handlers.len()) {
                script.run(handler, host);
            }
        })
    }

    /// One clan's takt over its planner (`ai.dll:0x10001780`, docs/15, "The planner"): the
    /// drain, every `ST_SOLVING` problem's `_Continue`, `Problems<n>`, and then the `_Start`
    /// pass from the heaviest weight down.
    ///
    /// STAND-IN: docs/15-behaviour.md#what-the-functions-do -- what the engine below does
    /// with a problem's action records is not read. They are plainly conditions to watch —
    /// `varset.var` names them *"When building capture"*, *"When all units in group killed"*
    /// and *"When all units in group nothing to do"* — and nothing else ends a problem that
    /// a handler left `ST_SOLVING`: `PBM_ROBOT_NEEDED_Start` files `ACTION_NOTHING_DOING` on
    /// the factory it ordered and its `_Continue` is empty, so without them a clan builds one
    /// warbot and never another. So an action that comes true retires its problem, which the
    /// next want then raises afresh.
    fn clan_takt(&mut self, which: Which) -> Vec<Notice> {
        self.actions(which);
        self.with_clan(which, |script, host| {
            host.planner.drain();
            for slot in host.planner.solving() {
                let Some(name) = host.planner.problem(slot).map(Problem::continue_handler) else {
                    continue;
                };
                host.planner.running = Some(slot);
                script.set_dword("dCurrentProblem", slot as u32);
                script.run_named(&name, host);
            }
            host.planner.running = None;
            let handler = host.planner.handler.clone();
            script.run_named(&handler, host);
            let mut done = BTreeSet::new();
            for _ in 0..planner::MAX_START_PASSES {
                let batch = host.planner.start_batch(&done);
                if batch.is_empty() {
                    break;
                }
                for slot in batch {
                    done.insert(slot);
                    let Some(name) = host.planner.problem(slot).map(Problem::start_handler) else {
                        continue;
                    };
                    host.planner.running = Some(slot);
                    script.set_dword("dCurrentProblem", slot as u32);
                    script.run_named(&name, host);
                }
            }
            host.planner.running = None;
        })
    }

    /// Retire every problem of that clan whose action has come true (see [`Self::clan_takt`]).
    fn actions(&mut self, which: Which) {
        let (planner, clan) = match which {
            Which::Player => (&mut self.planner, self.clan),
            Which::Other(i) => match self.others.get_mut(i) {
                Some(o) => (&mut o.planner, o.clan),
                None => return,
            },
        };
        let progress = &self.progress;
        // A unit with nothing to do: one the engine knows and that is running no order. An id
        // nothing answers any more counts as done with it.
        let idle = |id: i32| progress.contact(id).is_none_or(|c| c.order.is_none());
        let gone = |id: i32| !progress.knows(id);
        let taken = |id: i32| progress.owner(id) == clan as u32;
        let fired: Vec<usize> = planner
            .standing()
            .filter(|(_, p)| {
                p.actions.iter().any(|a| match a.action {
                    planner::ACTION_DESTROY => gone(a.target),
                    planner::ACTION_NOTHING_DOING => idle(a.target),
                    planner::ACTION_CAPTURE_BUILDING => taken(a.target),
                    planner::ACTION_GROUP_DESTROY | planner::ACTION_GROUP_NOTHING_DOING => {
                        let test = |id: i32| {
                            if a.action == planner::ACTION_GROUP_DESTROY { gone(id) } else { idle(id) }
                        };
                        planner
                            .group(a.target as usize)
                            .is_some_and(|g| !g.units.is_empty() && g.units.iter().copied().all(test))
                    }
                    _ => false,
                })
            })
            .map(|(slot, _)| slot)
            .collect();
        for slot in fired {
            planner.retire(slot);
        }
    }

    /// The units' takts, every clan's takt, and the `Mission` handler when it is due, at
    /// `now_ms`, with `position` giving where a unit by logical id stands.
    ///
    /// The game frame runs slot 3 for every clan and slot 9 for the local player's alone
    /// (docs/34, "When the Mission handler runs"), so an enemy's planner is what gives it its
    /// orders, and only the player's `Mission` handler sees the world.
    pub fn tick(&mut self, now_ms: f64, position: impl Fn(i32) -> Option<Vec3>) -> Vec<Notice> {
        self.progress.takt(now_ms, position);
        let mut notices = Vec::new();
        if self.takt.due(now_ms) {
            self.sweep(self.clan);
            notices.extend(self.clan_takt(Which::Player));
        }
        for i in 0..self.others.len() {
            if self.others[i].takt.due(now_ms) {
                self.sweep(self.others[i].clan);
                notices.extend(self.clan_takt(Which::Other(i)));
            }
        }
        if self.progress.mission_due(now_ms) {
            notices.extend(self.run("Mission"));
        }
        notices
    }

    /// The panel that gives way to the HUD once the outcome is recorded (`iron3d.dll:0x1009f8b0`,
    /// docs/34, "After the outcome"): a won mission's title, string 1012, over 5082 "Press
    /// 'Esc' to continue"; a lost one's, 1013, over 5082, 3075 and 3076, the restart and the
    /// load. The title is green when won and red when lost.
    pub fn panel(&self) -> Option<OutcomePanel> {
        let won = self.progress.outcome?;
        let text = |id: u32| self.strings.get(&id).cloned().unwrap_or_default();
        let (title, lines) = if won {
            (text(STRING_MISSION_COMPLETE), vec![text(STRING_PRESS_ESC)])
        } else {
            (
                text(STRING_MISSION_FAILED),
                vec![text(STRING_PRESS_ESC), text(STRING_PRESS_R), text(STRING_PRESS_L)],
            )
        };
        Some(OutcomePanel { won, title, lines })
    }

    fn string(&self, id: u32) -> Option<Say> {
        // The game's own lines go through `0x1007eb60`, as kind 2 (docs/35-hud.md).
        self.strings.get(&id).map(|s| Say::Text(Sender::System, s.clone()))
    }

    /// A sound `ui/game_resources.cfg` or the mission binds `name` to.
    pub fn sound(&self, name: &str) -> Option<Sound> {
        self.sounds.get(name)
    }

    /// What the game says for `notice` (docs/34, "Messages", "Objectives and the end of a
    /// mission").
    ///
    /// The outcome's title is the panel's, not a message ([`Progression::panel`]).
    ///
    /// STAND-IN: docs/34-progression.md#not-established -- a repeated message says string
    /// 6170 alone: string 6223's key is not filled in.
    pub fn say(&self, notice: &Notice) -> Vec<Say> {
        let mut out = Vec::new();
        match *notice {
            Notice::Message { id, first: true } => {
                if let Some(m) = self.messages.get(id) {
                    let sender = if m.info_system { Sender::Information } else { Sender::Training };
                    out.extend(m.text.clone().map(|t| Say::Text(sender, t)));
                    out.extend(m.voice.clone().map(Say::Voice));
                }
            }
            Notice::Message { first: false, .. } => out.extend(self.string(STRING_IN_HISTORY)),
            Notice::ObjectiveComplete { .. } => {
                out.extend(self.string(STRING_OBJECTIVE_COMPLETE));
                out.extend(self.sound(VOICE_OBJ_COMPLETE).map(Say::Voice));
            }
            Notice::ObjectiveFailed { .. } => out.extend(self.string(STRING_OBJECTIVE_FAILED)),
            Notice::MissionComplete => out.extend(self.sound(VOICE_MISSION_COMPLETE).map(Say::Voice)),
            Notice::MissionFailed => out.extend(self.sound(VOICE_MISSION_FAIL).map(Say::Voice)),
        }
        out
    }
}
