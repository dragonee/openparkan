//! `--log`: what happens in a headless run, a line as it happens. A unit made, lost or
//! captured, a building changing hands or losing a quarter of its life, a robot taking up
//! another task, an objective changing state. It reads the play and changes nothing.

use parkan_sim::behaviour::Task;
use parkan_world::play::Play;

/// What the chronicle last saw of each target.
#[derive(Clone, Default, PartialEq)]
struct Seen {
    alive: bool,
    clan: Option<i64>,
    task: String,
    /// Its life in quarters of its full life, rounded up.
    quarters: u8,
}

#[derive(Default)]
pub struct Chronicle {
    seen: Vec<Seen>,
    objectives: Vec<u8>,
    /// The hero's life in quarters, whether it has fallen, and the mission's outcome.
    hero: (u8, bool, Option<bool>),
}

impl Chronicle {
    /// Where the play stands as the run begins, said once.
    pub fn open(play: &Play) -> Self {
        let mut c = Chronicle::default();
        for t in 0..play.units.len() {
            let s = seen(play, t);
            println!("       start  {}  {}", label(play, t), s.task);
            c.seen.push(s);
        }
        c.objectives = objectives(play);
        c.hero = hero(play);
        c
    }

    /// What changed since the last call.
    pub fn step(&mut self, play: &Play) {
        let at = play.hero.time_ms / 1000.0;
        for t in 0..play.units.len() {
            let now = seen(play, t);
            let Some(was) = self.seen.get(t).cloned() else {
                println!("t {at:7.1}  made   {}  {}", label(play, t), now.task);
                self.seen.push(now);
                continue;
            };
            if was == now {
                continue;
            }
            if was.alive && !now.alive {
                println!("t {at:7.1}  lost   {}", label(play, t));
            } else if now.alive {
                if was.clan != now.clan {
                    println!("t {at:7.1}  taken  {}  from clan {:?}", label(play, t), was.clan);
                }
                if was.task != now.task {
                    println!("t {at:7.1}  task   {}  {} -> {}", label(play, t), was.task, now.task);
                }
                if now.quarters < was.quarters {
                    println!("t {at:7.1}  hurt   {}  to {}/4 of its life", label(play, t), now.quarters);
                }
            }
            self.seen[t] = now;
        }
        let now = objectives(play);
        if now != self.objectives {
            println!("t {at:7.1}  objectives {:?} -> {now:?}", self.objectives);
            self.objectives = now;
        }
        let now = hero(play);
        if now.0 < self.hero.0 {
            println!("t {at:7.1}  hurt   the hero  to {}/4 of its life", now.0);
        }
        if now.1 && !self.hero.1 {
            println!("t {at:7.1}  lost   the hero");
        }
        if now.2 != self.hero.2 {
            println!("t {at:7.1}  mission {}", if now.2 == Some(true) { "complete" } else { "failed" });
        }
        self.hero = now;
    }
}

fn hero(play: &Play) -> (u8, bool, Option<bool>) {
    let quarters = play.battle.combat.hero.as_ref().map_or(4, quarters);
    (quarters, play.fallen.is_some(), play.progression.as_ref().and_then(|p| p.progress.outcome))
}

/// A target's life in quarters of its full life, rounded up.
fn quarters(target: &parkan_sim::combat::Target) -> u8 {
    let (life, max) = target
        .parts
        .iter()
        .filter_map(|p| p.life.as_ref())
        .flat_map(|l| l.nodes.iter())
        .fold((0.0, 0.0), |a, n| (a.0 + if n.destroyed { 0.0 } else { n.life.max(0.0) }, a.1 + n.max));
    if max > 0.0 { (4.0 * life / max).ceil().clamp(0.0, 4.0) as u8 } else { 4 }
}

fn objectives(play: &Play) -> Vec<u8> {
    play.progression
        .as_ref()
        .map_or_else(Vec::new, |p| p.progress.objectives.iter().map(|o| o.state).collect())
}

/// A target's name, clan, logical id and place.
fn label(play: &Play, t: usize) -> String {
    let u = &play.units[t];
    let at = play.battle.combat.targets.get(t).map(|x| x.position).unwrap_or_default();
    let name = play.names.get(t).map_or("", String::as_str);
    let clan = u.clan.map_or_else(|| "-".to_owned(), |c| c.to_string());
    format!("#{t} {name:?} clan {clan} id {} at ({:.0}, {:.0})", id(u.logical_id), at.x, at.y)
}

/// A logical id as the scripts write it: a building's with its class bit said apart.
fn id(logical: i32) -> String {
    if logical < -1 { format!("B{}", logical & 0x7fff_ffff) } else { logical.to_string() }
}

fn seen(play: &Play, t: usize) -> Seen {
    let target = play.battle.combat.targets.get(t);
    let alive =
        target.is_some_and(|x| x.alive && !x.dead()) && !play.deleted.get(t).copied().unwrap_or(false);
    let quarters = target.map_or(4, quarters);
    let task = play
        .robots
        .iter()
        .chain(&play.emplacements)
        .find(|(r, _)| *r == t)
        .map_or_else(String::new, |(_, robot)| task(play, &robot.behaviour.task()));
    Seen { alive, clan: play.units[t].clan, task, quarters }
}

/// A task in a word, with what it is aimed at.
fn task(play: &Play, task: &Task) -> String {
    let named = |logical: i32| {
        play.units.iter().position(|u| u.logical_id == logical).map_or_else(
            || id(logical),
            |t| format!("{} {:?}", id(logical), play.names.get(t).map_or("", String::as_str)),
        )
    };
    match task {
        Task::Attack { target: Some(target), ordered, .. } => {
            format!("attack{} {}", if *ordered { " (ordered)" } else { "" }, named(*target))
        }
        Task::Follow { leader, .. } => format!("follow {}", named(*leader)),
        Task::Search { building: Some(building), .. } => format!("search, at {}", named(*building)),
        Task::Go { goal, .. } => format!("go ({:.0}, {:.0})", goal.x, goal.y),
        Task::Upgrade { building, .. } => format!("upgrade {}", named(*building)),
        other => {
            let text = format!("{other:?}");
            text.split([' ', '{', '(']).next().unwrap_or("").to_ascii_lowercase()
        }
    }
}
