//! What is heard: the hero's steps and breath, a door's sounds and a building's ambience.

use crate::common::*;

#[test]
#[ignore = "needs the game install"]
fn the_hero_sounds_its_steps_as_it_runs_and_its_arm_as_a_gun_is_put_away() {
    use parkan_sim::effects::CueKind;

    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    // The cannon's arm out first: it starts selected and unfolds.
    for _ in 0..120 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    play.cues.clear();
    play.hero.key("SCAN_W_1", true);
    play.hero.update_input();
    play.hero.key("SCAN_W_1", false);
    for _ in 0..60 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let arm: Vec<String> =
        play.cues.iter().filter(|c| c.kind == CueKind::Once).map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(arm.iter().any(|s| s.starts_with("h_gh")), "deselecting the cannon sounds its arm: {arm:?}");

    play.cues.clear();
    play.hero.key("SCAN_W", true);
    for _ in 0..120 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let steps = play.cues.iter().filter(|c| c.sound.to_ascii_lowercase().starts_with("step_h")).count();
    // docs/13: three steps a 0.405 s run cycle, and the first second starts up through
    // walk and transition states.
    assert!(steps >= 6, "{steps} steps in 2 s: {:?}", play.cues.iter().map(|c| &c.sound).collect::<Vec<_>>());
}

#[test]
#[ignore = "needs the game install"]
fn the_heros_breath_is_not_heard_while_its_steps_are() {
    // docs/11, "How a sound is heard": Mission 01's recording has no H_breath.wav.
    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    play.hero.key("SCAN_W", true);
    for _ in 0..300 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let sounds: Vec<String> = play.cues.iter().map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(!sounds.iter().any(|s| s == "h_breath.wav"), "{sounds:?}");
    assert!(sounds.iter().any(|s| s.starts_with("step_h")), "{sounds:?}");
    let breath =
        play.fx.instances.iter().filter(|(_, i)| i.effect.name.eq_ignore_ascii_case("hero_breath")).count();
    assert_eq!(breath, 1, "the breath's effect still runs");
}

#[test]
#[ignore = "needs the game install"]
fn the_factorys_door_sounds_stand_on_their_nodes_origins_the_side_doors_30_8_m_from_the_door() {
    use glam::Vec3;
    use parkan_world::fx::Owner;

    let (mut play, _) = mission_02_play();
    let b = play.buildings.iter().position(|b| b.doors.len() == 3).expect("fr_b_plant's three doors");
    let t = play.buildings[b].target;
    let part_index = play.buildings[b].part;
    // `door_open_01` on each door's node, ids 8000, 8002 and 8004 (docs/13, "A building's
    // load group"). Each takes its node's world matrix as its frame, whose translation is the
    // node's own origin (`Effect.dll:0x1000625a`, `AniMesh.dll:0x10005320`), and their nodes'
    // origins stand 8 m and 30.8 m from the doors themselves.
    for (d, id) in [(0usize, 8000i32), (1, 8002), (2, 8004)] {
        let node = play.buildings[b].doors[d].nodes[0];
        let door = door_centre(&play, t, part_index, node);
        let origin = play.battle.combat.targets[t].parts[part_index].nodes[node].translation;
        let origin = Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);
        let at: Vec<Vec3> = play.fx.owned(Owner::Building(t, id)).map(|i| i.frame.origin).collect();
        assert_eq!(at.len(), 1, "one open sound on door {d}");
        assert!(
            at[0].distance(origin) < 0.01,
            "door {d}'s sound stands on its node's origin: {at:?} against {origin}"
        );
        if d > 0 {
            assert!(
                (origin.distance(door) - 30.8).abs() < 0.5,
                "the side door's node origin is 30.8 m away: {origin} against {door}"
            );
        }
    }
}

/// A building's ambience hums on: the load group starts `f_bunk_sfx` and its like in time
/// mode 2 over a one-second duration, and their loops run over the whole of *t*, so the wrap
/// at the end of each period must not stop and start them again (docs/11, "Type 2 is a
/// sound"). Mission 03 starts the hero among four bunkers, three computers, a store and a
/// generator, which restarted once a second.
#[test]
#[ignore = "needs the game install"]
fn mission_03s_building_ambiences_start_once_and_are_never_stopped() {
    use parkan_sim::effects::CueKind;

    let (mut play, _) = mission_03_play();
    let mut started: std::collections::BTreeMap<String, usize> = Default::default();
    let mut stopped = Vec::new();
    for _ in 0..(30 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        for c in play.cues.drain(..) {
            let sound = c.sound.to_ascii_lowercase();
            match c.kind {
                CueKind::Loop => *started.entry(sound).or_default() += 1,
                CueKind::Stop => stopped.push(sound),
                _ => {}
            }
        }
    }
    assert_eq!(
        (started.get("f_bunk.wav"), started.get("f_comp.wav"), started.get("f_gener.wav")),
        (Some(&4), Some(&3), Some(&1)),
        "one start each in 30 s: {started:?}"
    );
    assert!(stopped.is_empty(), "and none stopped: {stopped:?}");
}
