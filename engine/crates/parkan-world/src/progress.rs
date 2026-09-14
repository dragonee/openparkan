//! A mission's progression as it plays: its player clan's script, run on the handler's
//! cadence against the mission's routes, units and objectives, and what the game says
//! for it. See `docs/34-progression.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use glam::Vec3;
use parkan_formats::mission::{self, Mission, Value};
use parkan_formats::{gamedir, scr};
use parkan_sim::progression::{Notice, Progress};
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

pub struct Progression {
    /// The player clan's script, when it has one that loads.
    pub script: Option<Interpreter>,
    pub progress: Progress,
    /// The player's clan, and its base's centre.
    pub clan: i64,
    pub base: [f32; 2],
    pub messages: Messages,
    pub sounds: Sounds,
    pub strings: BTreeMap<u32, String>,
    /// Function-table slots the script called that the engine does not answer.
    pub unanswered: BTreeSet<i32>,
}

/// The engine below the SuperAI, as the script's calls reach it.
struct Answers<'a> {
    progress: &'a mut Progress,
    clan: i64,
    base: [f32; 2],
    notices: &'a mut Vec<Notice>,
    unanswered: &'a mut BTreeSet<i32>,
}

impl Host for Answers<'_> {
    fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32 {
        let int = |args: &Args<'_>, i: usize| i64::from(args.dword(i) as i32);
        match function {
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
            // STAND-IN: docs/15-behaviour.md#what-the-functions-do -- the engine answers only
            // the functions a campaign's player script needs for its messages and
            // objectives (19, 30, 31, 32); any other call does nothing and answers 0.
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
    /// stands, and the script's `Init` runs once (SuperAI slot 5).
    pub fn load(game: &Path, mission_dir: &Path, mission: &Mission, hero: usize) -> Result<Self> {
        let clan = mission.objects.get(hero).and_then(mission::Object::clan_id).unwrap_or(0);
        let record = usize::try_from(clan).ok().and_then(|c| mission.clans.get(c));
        let exempt: Vec<bool> = resources::objectives(mission_dir)?.iter().map(|o| o.exempt).collect();
        let messages = Messages::load(game, mission_dir)?;
        let mut progress = Progress::new(&mission.routes, &exempt, messages.0.iter().map(|m| m.index));
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
        let mut me = Self {
            script,
            progress,
            clan,
            base: record.map_or([0.0; 2], |c| c.base),
            messages,
            sounds: Sounds::open(game, mission_dir)?,
            strings: resources::game_strings(game).unwrap_or_default(),
            unanswered: BTreeSet::new(),
        };
        me.run("Init");
        Ok(me)
    }

    /// Run one of the script's handlers; what its calls to function 30 raised.
    pub fn run(&mut self, handler: &str) -> Vec<Notice> {
        let mut notices = Vec::new();
        if let Some(script) = self.script.as_mut() {
            let mut answers = Answers {
                progress: &mut self.progress,
                clan: self.clan,
                base: self.base,
                notices: &mut notices,
                unanswered: &mut self.unanswered,
            };
            script.run_named(handler, &mut answers);
        }
        notices
    }

    /// The units' takts, and the `Mission` handler when it is due, at `now_ms`, with
    /// `position` giving where a unit by logical id stands.
    pub fn tick(&mut self, now_ms: f64, position: impl Fn(i32) -> Option<Vec3>) -> Vec<Notice> {
        self.progress.takt(now_ms, position);
        if self.progress.mission_due(now_ms) { self.run("Mission") } else { Vec::new() }
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
