//! Mission 01's own run: its briefing, its scripts and its objectives.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn mission_01_greets_the_hero_and_completes_its_first_objective_once_the_targets_are_gone() {
    use parkan_world::progress::{Say, Sender};

    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    play.tick(tick, [0.0; 2]);
    let p = play.progression.as_ref().unwrap();
    assert!(p.unanswered.is_empty(), "tut1_pl2 calls only what the engine answers: {:?}", p.unanswered);
    // The hero starts inside route 0: messages 11 and 14 on the first run (docs/34).
    let played: Vec<i64> = p.progress.played.iter().filter(|(_, v)| **v).map(|(k, _)| *k).collect();
    assert_eq!(played, vec![11, 14]);
    let welcome = p.messages.get(11).expect("message 11");
    let sender = if welcome.info_system { Sender::Information } else { Sender::Training };
    assert!(play.says.contains(&Say::Text(sender, welcome.text.clone().expect("T01_I01's text"))));
    let voices = play.says.iter().filter(|s| matches!(s, Say::Voice(v) if v.exists())).count();
    assert_eq!(voices, 2, "{:?}", play.says);
    play.says.clear();

    // The five targets go: Trgt's robots count down, and the next run completes objective 0.
    let trgt: Vec<i32> = play
        .progression
        .as_ref()
        .unwrap()
        .progress
        .units
        .iter()
        .filter(|u| u.clan == 1)
        .map(|u| u.id)
        .collect();
    assert_eq!(trgt.len(), 5);
    for id in trgt {
        play.progression.as_mut().unwrap().progress.destroyed(id);
    }
    for _ in 0..130 {
        play.tick(tick, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>(), vec![1, 0, 0]);
    assert!(p.progress.played[&17] && p.progress.played[&12]);
    assert_eq!(p.progress.outcome, None);
    let done = p.strings[&parkan_world::progress::STRING_OBJECTIVE_COMPLETE].clone();
    assert!(play.says.contains(&Say::Text(Sender::System, done)), "{:?}", play.says);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_briefing_flies_its_waypoints_in_the_recordings_time_under_its_title_and_subtitles() {
    use parkan_world::briefing::Briefing;
    use parkan_world::hud::Space;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let mut b = Briefing::open(&game, &dir).unwrap().expect("Mission 01 has a briefing");
    assert_eq!((b.title.as_str(), b.subtitles, b.flythrough.stops.len()), ("Line of Fire", true, 26));
    // A frame every 1/60 s: when each subtitle first shows, against the recording's times less
    // its first frame at 1.20 s (docs/21, "Against the recording").
    let (mut t, mut changes, mut voices) = (0.0, Vec::new(), Vec::new());
    let mut last = String::from("-");
    while !b.finished() && t < 200.0 {
        voices.extend(b.frame(t).into_iter().map(|s| s.member));
        if b.subtitle() != last {
            last = b.subtitle().to_owned();
            changes.push((t, last.clone()));
        }
        if (t - 8.3_f64).abs() < 0.5 / 60.0 {
            // Waypoint 1 holds waypoint 0's camera, looking at its target.
            let eye = b.eye();
            assert!((eye.position - glam::Vec3::new(767.965, 164.519, 118.899)).length() < 1e-3);
            assert!((eye.fov_x - 1.04).abs() < 1e-6 && eye.up.z > 0.0);
        }
        t += 1.0 / 60.0;
    }
    assert!((97.9..98.6).contains(&t), "the briefing ends at {t}");
    let at = |needle: &str| changes.iter().find(|(_, s)| s.starts_with(needle)).map(|c| c.0).unwrap();
    for (needle, recording) in [
        ("Tara, The Home Base", 1.20),
        ("Listen up", 4.93),
        ("The Tara range", 37.83),
        ("Battle Mission", 66.37),
    ] {
        let model = at(needle) + 1.20;
        assert!((model - recording).abs() < 0.35, "{needle}: {model} against {recording}");
    }
    assert_eq!(voices.first().map(String::as_str), Some("t01_t01.wav"));
    assert_eq!(voices.len(), 11, "{voices:?}");

    // The screen: the fade, and the bars at 75 and 405 across the whole of a wide screen.
    let font = parkan_world::text::GameFont::ui(&game, "GAME_FONT").unwrap();
    let mut b = Briefing::open(&game, &dir).unwrap().unwrap();
    b.frame(0.0);
    let screen = b.screen(Space::new(1280.0, 720.0), &font);
    assert_eq!(screen.fills.len(), 3);
    assert_eq!(screen.fills[0].colour[3], 1.0, "the briefing opens black");
    let (top, bottom) = (screen.fills[1], screen.fills[2]);
    assert_eq!((top.min[0], top.max[0], top.max[1]), (-1.0, 1.0, 1.0));
    assert!(
        (top.min[1] - (1.0 - 2.0 * 75.0 / 480.0)).abs() < 1e-5
            && (bottom.max[1] - (1.0 - 2.0 * 405.0 / 480.0)).abs() < 1e-5
    );
    assert_eq!(screen.title[0].text, "Line of Fire");
    assert!(!screen.lines.is_empty() && screen.lines[0].text.starts_with("Tara,"));
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_scripts_wait_while_its_briefing_plays_and_greet_the_hero_after() {
    let (mut play, _) = mission_01_play();
    play.paused = true;
    for _ in 0..600 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(play.says.is_empty(), "nothing said in the briefing: {:?}", play.says.len());
    play.paused = false;
    for _ in 0..600 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(!play.says.is_empty(), "the greeting once it is over");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_objectives_screen_lists_its_three_objectives_and_its_map_takes_the_weapons_place() {
    use glam::{Mat4, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{MINIMAP, Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let (mut play, _) = mission_01_play();
    let mut pages = Pages::open(&game).unwrap();
    assert!(pages.add_minimap(&game, &dir).unwrap());
    let minimap = &pages.pages[pages.index(MINIMAP).unwrap()];
    assert_eq!((minimap.name.as_str(), minimap.width, minimap.height), ("tut1.tex", 256, 256));
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.1)
        * Mat4::look_to_rh(play.hero.eye().position, play.hero.eye().forward, Vec3::Z);
    let space = Space::new(640.0, 480.0);

    cockpit.objectives.open_at_start();
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    let lines: Vec<&str> = drawn.menu_text.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        lines,
        [
            "Primary objectives",
            "1. Destroy all the targets on the island : in progress",
            "2. Capture the neutral warbots : in progress",
            "3. Destroy the enemy warbot : in progress",
            "Press F12 to close"
        ]
    );
    assert!(drawn.text.is_empty() && drawn.views.is_empty(), "it alone is drawn");
    play.hero.time_ms += 7001.0;
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    assert!(drawn.menu_text.is_empty() && !drawn.text.is_empty(), "closed 7 s after its first draw");

    cockpit.map.toggle();
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    assert!(!drawn.text.iter().any(|r| r.text == "AUTOCANNON 25mm"), "no weapon rows under the map");
    let page = pages.index(MINIMAP).unwrap() as u16;
    assert!(
        drawn.batches.iter().flat_map(|b| &b.vertices).any(|v| v.page == Some(page)),
        "the minimap is drawn"
    );
}
