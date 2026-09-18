//! A mission's progression as it plays: every clan's script's `Init`, and the player
//! clan's `Mission` handler run on its cadence against the mission's routes, units and
//! objectives; what the game says for it, and the orders the scripts give. See
//! `docs/34-progression.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::{gamedir, scr};
use parkan_sim::orders::{self, Order, Target};
use parkan_sim::progression::{self, ClanTakt, Notice, Progress};
use parkan_sim::script::{Args, Host, Interpreter};

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

/// The handler every clan's takt runs (docs/34, "When the Mission handler runs"). Function 33
/// can name another, `Problems<n>`; no shipped script calls it.
pub const HANDLER_PROBLEMS: &str = "Problems0";

/// Another clan's script: its SuperAI runs `Init` once, as every clan's does, and its problem
/// handler on its own takt, but never `Mission`, which the game frame runs for the local
/// player's clan alone (docs/34, "When the Mission handler runs").
pub struct ClanScript {
    pub clan: i64,
    pub base: [f32; 2],
    pub script: Interpreter,
    pub takt: ClanTakt,
}

/// Whose script a run is for: the player clan's, or the clan of `others[i]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    Player,
    Other(usize),
}

pub struct Progression {
    /// The player clan's script, when it has one that loads.
    pub script: Option<Interpreter>,
    /// Its clan's takt, which runs its problem handler as every other clan's does.
    pub takt: ClanTakt,
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
    pub messages: Messages,
    pub sounds: Sounds,
    pub strings: BTreeMap<u32, String>,
    /// Function-table slots the script called that the engine does not answer.
    pub unanswered: BTreeSet<i32>,
    /// Each objective's text as `mission.cfg` writes it, in script order.
    pub objective_texts: Vec<String>,
}

/// The engine below the SuperAI, as the script's calls reach it.
struct Answers<'a> {
    progress: &'a mut Progress,
    clan: i64,
    base: [f32; 2],
    /// The running clan's seconds clock, which functions 59 and 60 read.
    clock: u32,
    relations: &'a [Vec<u32>],
    notices: &'a mut Vec<Notice>,
    unanswered: &'a mut BTreeSet<i32>,
    orders: &'a mut Vec<ScriptOrder>,
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
}

impl Host for Answers<'_> {
    fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32 {
        let int = |args: &Args<'_>, i: usize| i64::from(args.dword(i) as i32);
        match function {
            // An order packet for the unit of any clan with logical id 0 (`ai.dll:0x10008054`):
            // the order, the insert mode, the parameter, four floats no task reads, the target
            // kind and the target, a place's two words as its x and y.
            //
            // STAND-IN: docs/34-progression.md#what-the-scripts-ask--read-and-measured-1 -- the
            // unit's own answer (1 taken, 0 refused) is not waited for: the order is handed to
            // the unit after the handler's run, and a known id answers 1.
            15 => {
                let id = args.dword(0) as i32;
                if !self.progress.knows(id) {
                    return ORDER_NO_UNIT;
                }
                let word = |i: usize| args.dword(i) as i32;
                let target = match args.dword(8) {
                    orders::TARGET_BY_PLACE => Target::Place([word(9) as f32, word(10) as f32, 0.0]),
                    orders::TARGET_BY_LOGIC_ID => Target::LogicId(word(9)),
                    orders::TARGET_BY_TYPE => Target::TypeMask(args.dword(9)),
                    _ => Target::NotDefined,
                };
                let order = Order { code: word(1), parameter: word(3), target };
                self.orders.push(ScriptOrder { id, order, insert: args.dword(2) });
                ORDER_TAKEN
            }
            // The base's centre and the clan's number, written into the three operands.
            19 => {
                args.set_float(0, self.base[0]);
                args.set_float(1, self.base[1]);
                args.set_dword(2, self.clan as u32);
                0
            }
            30 => {
                let notices = self.progress.call(int(args, 0), int(args, 1));
                self.notices.extend(notices);
                0
            }
            31 => self.progress.robots(int(args, 0), i64::from(args.dword(1))) as u32,
            32 => u32::from(self.progress.areals.holds(int(args, 0), int(args, 1))),
            34 => self.progress.count_type(self.clan, args.dword(0)) as u32,
            52 => self.progress.owner(args.dword(0) as i32),
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
            // The enemy object of exactly that type with the least strength about it, and that
            // strength written into the third operand, truncated (`0x1000f12e`).
            71 => {
                let (type_word, radius) = (args.dword(1), args.float(0));
                match self.progress.enemy_of_type(|other| self.hostile(other), type_word, radius) {
                    Some((id, strength)) => {
                        args.set_dword(2, strength as u32);
                        id as u32
                    }
                    None => progression::ERROR,
                }
            }
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- the engine answers only
            // the functions the campaign's scripts need for their messages, objectives, timed
            // orders and targets (15, 19, 30, 31, 32, 34, 52, 59, 60, 71); any other call does
            // nothing and answers 0. Nothing raises or runs an AI problem, so a clan's takt
            // reaches only what its problem handler does itself.
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
        let script = match record.map(|c| c.ai_script.as_str()).filter(|s| !s.is_empty()) {
            Some(path) => Some(load_script(game, path)?),
            None => None,
        };
        // Another clan's script that does not load leaves that clan without one.
        let others = mission
            .clans
            .iter()
            .enumerate()
            .filter(|&(i, c)| i as i64 != clan && !c.ai_script.is_empty())
            .filter_map(|(i, c)| {
                let script = load_script(game, &c.ai_script).ok()?;
                let takt = ClanTakt::new(0, i as u32 + 1);
                Some(ClanScript { clan: i as i64, base: c.base, script, takt })
            })
            .collect();
        let mut me = Self {
            script,
            takt: ClanTakt::new(0, clan as u32 + 1),
            others,
            orders: Vec::new(),
            progress,
            clan,
            base: record.map_or([0.0; 2], |c| c.base),
            relations: mission::relation_words(&mission.clans),
            messages,
            sounds: Sounds::open(game, mission_dir)?,
            strings: resources::game_strings(game).unwrap_or_default(),
            unanswered: BTreeSet::new(),
            objective_texts: objectives.into_iter().map(|o| o.text).collect(),
        };
        me.run("Init");
        for i in 0..me.others.len() {
            me.run_clan(Which::Other(i), |script, host| {
                script.run_named("Init", host);
            });
        }
        Ok(me)
    }

    /// Run `run` on one clan's script, with the engine below it answering its calls; what its
    /// calls to function 30 raised. A clan without a script does nothing.
    fn run_clan(&mut self, which: Which, run: impl FnOnce(&mut Interpreter, &mut dyn Host)) -> Vec<Notice> {
        let Self { script, takt, others, orders, progress, clan, base, relations, unanswered, .. } = self;
        let (script, clan, base, clock) = match which {
            Which::Player => (script.as_mut(), *clan, *base, takt.clock()),
            Which::Other(i) => match others.get_mut(i) {
                Some(o) => (Some(&mut o.script), o.clan, o.base, o.takt.clock()),
                None => (None, 0, [0.0; 2], 0),
            },
        };
        let mut notices = Vec::new();
        if let Some(script) = script {
            let mut answers = Answers {
                progress,
                clan,
                base,
                clock,
                relations: relations.as_slice(),
                notices: &mut notices,
                unanswered,
                orders,
            };
            run(script, &mut answers);
        }
        notices
    }

    /// Run one of the player clan's script's handlers; what its calls to function 30 raised.
    pub fn run(&mut self, handler: &str) -> Vec<Notice> {
        self.run_clan(Which::Player, |script, host| {
            script.run_named(handler, host);
        })
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
        let which = if clan == self.clan {
            Which::Player
        } else {
            match self.others.iter().position(|o| o.clan == clan) {
                Some(i) => Which::Other(i),
                None => return Vec::new(),
            }
        };
        self.run_clan(which, |script, host| {
            let handler = script.handler(HANDLER_GENERATOR_FOUND).map(|h| h + EVENT_HERO_TELEPORTED);
            if let Some(handler) = handler.filter(|&h| h < script.script.handlers.len()) {
                script.run(handler, host);
            }
        })
    }

    /// The units' takts, every clan's takt, and the `Mission` handler when it is due, at
    /// `now_ms`, with `position` giving where a unit by logical id stands.
    ///
    /// The game frame runs slot 3 for every clan and slot 9 for the local player's alone
    /// (docs/34, "When the Mission handler runs"), so an enemy's problem handler is what gives
    /// it its timed orders, and only the player's `Mission` handler sees the world.
    pub fn tick(&mut self, now_ms: f64, position: impl Fn(i32) -> Option<Vec3>) -> Vec<Notice> {
        self.progress.takt(now_ms, position);
        let mut notices = Vec::new();
        let problems = |script: &mut Interpreter, host: &mut dyn Host| {
            script.run_named(HANDLER_PROBLEMS, host);
        };
        if self.takt.due(now_ms) {
            notices.extend(self.run_clan(Which::Player, problems));
        }
        for i in 0..self.others.len() {
            if self.others[i].takt.due(now_ms) {
                notices.extend(self.run_clan(Which::Other(i), problems));
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
