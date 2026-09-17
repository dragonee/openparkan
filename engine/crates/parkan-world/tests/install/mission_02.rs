//! Mission 02's own run: its briefing, its script, the way in to its factory and its win.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn mission_02s_briefing_flies_its_waypoints_in_the_recordings_time() {
    use parkan_world::briefing::Briefing;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_02).unwrap();
    let mut b = Briefing::open(&game, &dir).unwrap().expect("Mission 02 has a briefing");
    assert_eq!((b.title.as_str(), b.flythrough.stops.len()), ("The Constructor", 18));
    let (mut t, mut changes, mut voices) = (0.0, Vec::new(), Vec::new());
    let mut last = String::from("-");
    while !b.finished() && t < 200.0 {
        voices.extend(b.frame(t).into_iter().map(|s| s.member));
        if b.subtitle() != last {
            last = b.subtitle().to_owned();
            changes.push((t, last.clone()));
        }
        t += 1.0 / 60.0;
    }
    // The recording's subtitle strip, frame by frame: its first subtitle at 2.40 s, and each
    // later change; the objectives screen replaces its briefing at 69.83 s.
    let at = |needle: &str| {
        let blank = needle.is_empty();
        changes.iter().find(|(_, s)| s.starts_with(needle) && s.is_empty() == blank).map(|c| c.0).unwrap()
    };
    let start = 2.40;
    for (needle, recording) in [
        ("Long time", 6.30),
        ("", 20.53),
        ("Watch out", 22.57),
        ("Battle Mission", 42.27),
        ("Take control", 44.30),
        ("Control the factory", 56.00),
        ("Move east", 63.17),
    ] {
        let model = at(needle) + start;
        assert!((model - recording).abs() < 0.35, "{needle}: {model} against {recording}");
    }
    assert!((t + start - 69.83).abs() < 0.35, "the briefing ends at {}", t + start);
    assert_eq!(voices.first().map(String::as_str), Some("t02_t01.wav"));
    assert_eq!(voices.len(), 7, "{voices:?}");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_script_greets_the_hero_and_leaves_its_captures_open_until_they_are_made() {
    let (mut play, _) = mission_02_play();
    play.tick(1000.0 / 60.0, [0.0; 2]);
    let p = play.progression.as_mut().unwrap();
    assert!(p.unanswered.is_empty(), "tut2_pl2 calls only what the engine answers: {:?}", p.unanswered);
    // The hero starts in route 0: message 6, T02_I01 (docs/34, "Mission 02").
    let played: Vec<i64> = p.progress.played.iter().filter(|(_, v)| **v).map(|(k, _)| *k).collect();
    assert_eq!(played, vec![6]);
    // Function 52 reads the neutral factory and Outpost as clan 2's: no objective is done.
    let (factory, outpost) = (0x8000_0001_u32 as i32, 0x8000_0003_u32 as i32);
    assert_eq!((p.progress.owner(factory), p.progress.owner(outpost)), (2, 2));
    assert!(p.progress.objectives.iter().all(|o| o.state == 0));
    // The factory taken: objective 0 and messages 7 and 11 on the next run.
    p.progress.captured(factory, 0);
    let notices = p.run("Mission");
    assert!(notices.contains(&parkan_sim::progression::Notice::ObjectiveComplete { index: 0 }));
    assert!(p.progress.played[&7] && p.progress.played[&11]);
    assert_eq!(p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>(), vec![1, 0, 0, 0]);
}

#[test]
#[ignore = "needs the game install"]
fn mission_02_is_won_by_the_factory_the_warbot_it_builds_and_the_outpost_on_the_island() {
    use parkan_world::factory::Project;
    use parkan_world::play::Mode;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    // 1. The Large Factory's pod: captured, its screen opens, objective 0.
    let factory = play.buildings.iter().find(|b| b.doors.len() == 3).unwrap().target;
    assert!(play.stand_on_pod(factory));
    tick(&mut play, 6.0);
    assert_eq!(play.mode(), Mode::Factory(factory));
    // 2. A design accepted and built: a free L-2f in 60 s; objective 1.
    let f = play.factories.iter().position(|f| f.target == factory).unwrap();
    play.factories[f].accept(Project {
        path: "UNITS\\bld_unit_-2147483647.dat".into(),
        name: "LFW-X Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 4,
        ore: 411.0,
        power: 226.5,
        lines: Vec::new(),
        sphere: None,
    });
    play.factory_click(factory, parkan_world::cockpit::factory::Click::Batch);
    assert!(play.factories[f].build.is_some());
    play.factory_click(factory, parkan_world::cockpit::factory::Click::Exit);
    assert_eq!(play.mode(), Mode::OnFoot);
    tick(&mut play, 64.0);
    let bot = play.robots.last().map(|r| r.0).unwrap();
    assert_eq!(play.names[bot], "LFW-2 Warrior");
    // 3. Aboard, over the island's flat ground 30 m east of the Outpost, down to land, out.
    // Resting on its node sphere's centre the bot reads altitude 11 there, as the recording's does
    // by the Outpost when the hero gets out (docs/39, "Against the recording").
    play.hero.walker.body.position =
        play.robots.last().unwrap().1.walker.body.position + glam::Vec3::new(6.0, 0.0, 0.0);
    assert!(play.board(bot));
    let over_island = glam::Vec3::new(1319.0, 793.5, 170.0);
    play.robots.last_mut().unwrap().1.walker.body.position = over_island;
    play.key("SCAN_F", true);
    tick(&mut play, 8.0);
    play.key("SCAN_F", false);
    let landed = play.driven().walker.body.position;
    assert_eq!((landed.z - 150.0).round(), 11.0, "the altitude figure over Tut_2's water at {landed}");
    assert!(play.roll_back(), "out on the island: the bot at {landed}");
    // 4. The Outpost's pod: captured, objective 2, and the mission is won.
    let outpost = play.buildings.iter().find(|b| b.doors.is_empty()).unwrap().target;
    assert!(play.stand_on_pod(outpost));
    tick(&mut play, 6.0);
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>()[..3], [1, 1, 1]);
    assert_eq!(p.progress.outcome, Some(true), "MISSION COMPLETE");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_hero_walks_in_by_the_factorys_west_door_down_to_its_pod_and_captures_it() {
    use parkan_world::play::Mode;

    // The hall-way vertices of docs/24, "The way to the pod", from outside the west side door.
    let route = [
        [352.77, 789.66],
        [363.75, 789.39],
        [369.87, 789.24],
        [370.33, 793.67],
        [375.68, 793.70],
        [386.35, 793.44],
        [392.52, 792.99],
        [392.30, 783.93],
        [392.19, 778.47],
        [391.63, 755.86],
        [391.47, 749.50],
        [391.24, 740.08],
    ];
    let (mut play, _) = mission_02_play();
    assert!(play.stand_at(337.36, 790.04, -std::f32::consts::FRAC_PI_2));
    play.hero.key("SCAN_W", true);
    let mut next = 0;
    let mut arrived = None;
    for tick in 0..(60 * 60) {
        let at = play.hero.walker.body.position;
        while next < route.len() && glam::Vec2::from_array(route[next]).distance(at.truncate()) < 1.2 {
            next += 1;
        }
        if next == route.len() {
            if arrived.is_none() {
                arrived = Some(tick);
                play.hero.key("SCAN_W", false);
            }
        } else {
            let to = glam::Vec2::from_array(route[next]) - at.truncate();
            play.hero.walker.body.yaw = (-to.x).atan2(to.y);
        }
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if play.mode() != Mode::OnFoot {
            break;
        }
    }
    let at = play.hero.walker.body.position;
    let arrived = arrived.unwrap_or_else(|| panic!("stopped before vertex {next} of the route, at {at}"));
    assert!(arrived < 40 * 60, "on the pod {} s in", arrived / 60);
    assert!((at.z - 140.7).abs() < 1.0, "on the pod room's floor: {at}");
    assert!(matches!(play.mode(), Mode::Factory(0)), "the pod captured the factory and opened its screen");
}
