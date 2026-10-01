//! The quick save and the quick load, F7 and F8 (docs/14, "Quick save and quick load").

use crate::common::*;
use parkan_formats::gamedir;
use parkan_world::play::{Mode, Play};
use parkan_world::progress::Say;

const TICK: f64 = 1000.0 / 60.0;

fn run(play: &mut Play, seconds: usize) {
    for _ in 0..seconds * 60 {
        play.tick(TICK, [0.0; 2]);
    }
}

/// Where every target stands and what it has left, with the hero's own.
fn standing(play: &Play) -> Vec<([f32; 3], bool, u32)> {
    let life = |t: &parkan_sim::combat::Target| {
        t.parts
            .iter()
            .filter_map(|p| p.life.as_ref())
            .flat_map(|l| l.nodes.iter())
            .map(|n| n.life)
            .sum::<f32>()
    };
    let mut out: Vec<([f32; 3], bool, u32)> = play
        .battle
        .combat
        .targets
        .iter()
        .map(|t| (t.position.to_array(), t.alive, life(t).to_bits()))
        .collect();
    out.push((play.hero.walker.body.position.to_array(), !play.hero.dead(), 0));
    out
}

/// A quick save is the play as it stood: played on from, it goes as the play it was taken from
/// went. C03 Mission 02 twenty seconds in, saved, and both run forty more: the enemy clans'
/// factories make units and the arachnids graze in both alike, to the last bit.
#[test]
#[ignore = "needs the game install"]
fn a_quick_save_played_on_goes_as_the_play_it_was_taken_from() {
    let mut play = campaign_play_at(gamedir::C03_MISSION_02, Some(0));
    run(&mut play, 20);
    let units = play.units.len();
    let save = play.quick_save("Game saved...").expect("a campaign mission in play saves");
    assert!(
        matches!(play.says.last(), Some(Say::Text(parkan_world::progress::Sender::System, t)) if t == "Game saved..."),
        "the message box is told, from the system"
    );
    assert!(save.says.len() < play.says.len(), "and the save was taken before it was");
    play.says.clear();
    let at_save = standing(&save);

    run(&mut play, 40);
    assert!(play.units.len() > units, "units were made after the save: {} then {units}", play.units.len());
    assert_ne!(standing(&play)[..at_save.len() - 1], at_save[..at_save.len() - 1], "and the world moved on");

    let mut loaded = Play::quick_load(&save).expect("F8 loads it");
    assert_eq!(standing(&loaded), at_save, "the load is the world as it was saved");
    assert_eq!(loaded.units.len(), units);
    run(&mut loaded, 40);
    assert_eq!(loaded.hero.time_ms, play.hero.time_ms);
    assert_eq!(standing(&loaded), standing(&play), "and forty seconds on it is where the first play is");
    // The save is still there to load again.
    assert_eq!(standing(&Play::quick_load(&save).unwrap()), at_save);
}

/// The save does not hold the interface's mode stack: one made in the bunker's command view
/// loads with the hero on foot in the bunker, as the let's play's Part 6 has it at 13:57 and
/// 26:16. And F8 does not ask whether the mission is over, where F7 does.
#[test]
#[ignore = "needs the game install"]
fn a_quick_save_made_in_command_mode_loads_on_foot_and_loads_over_the_outcome() {
    let mut play = campaign_play_at(gamedir::C03_MISSION_02, Some(0));
    let bunker = play.names.iter().position(|n| n == "sbunk02").expect("the player's Small Bunker");
    assert!(play.stand_on_pod(bunker));
    run(&mut play, 8);
    assert_eq!(play.mode(), Mode::Command(bunker), "the pod opened command mode");
    let save = play.quick_save("Game saved...").expect("a command view does not refuse it");
    let room = play.hero.walker.body.position;

    // The mission lost: F7 is refused, F8 is not.
    play.progression.as_mut().unwrap().progress.outcome = Some(false);
    assert!(play.quick_save("Game saved...").is_none(), "the state word is no longer 4");
    let loaded = Play::quick_load(&save).expect("the quick load does not ask the state word");
    assert_eq!(loaded.mode(), Mode::OnFoot, "back on foot");
    assert_eq!(loaded.hero.walker.body.position, room, "in the bunker's pod room");
    assert_eq!(loaded.progression.as_ref().unwrap().progress.outcome, None, "and the mission is on again");
}

/// The training campaign takes neither key: its launch mode forbids saving (`+0xe6`).
#[test]
#[ignore = "needs the game install"]
fn the_training_campaign_takes_no_quick_save() {
    let mut play = campaign_play(gamedir::MISSION_03);
    assert!(play.training);
    run(&mut play, 2);
    assert!(play.quick_save("Game saved...").is_none());
    assert!(play.says.iter().all(|s| !matches!(s, Say::Text(_, t) if t == "Game saved...")));
}
