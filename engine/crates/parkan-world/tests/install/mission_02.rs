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

/// Mission 02's medusas graze (docs/31, "Migrate: an animal's pasture"; docs/34, "The
/// medusas"): with no order an animal migrates, asking for its clan's current pasture — one of
/// `Anml`'s two zones, the same for both — and is sent to a point off its centre's +x, +y side.
///
/// How far they get is the motion's, not the task's: the walk asks for the medusa's live top
/// speed, 13 m/s forward, `Speed_MaximumFactor` being 1 for every behaviour, and an animal's
/// walk points stand 45 to 95 m over the ground (`Behavior.dll:0x10040f20`), so the Wizard
/// writes a climb of more than 5.5 m/s. No moving anchor state of `a_a_l3.ctl` takes a forward
/// speed past 10 or a climb past 5.5, and with none to go to the planner keeps the one it is in
/// (`Control.dll:0x1000531a`): the machine holds its hover (docs/34, "The medusas").
#[test]
#[ignore = "needs the game install"]
fn mission_02s_medusas_migrate_over_their_clans_one_pasture() {
    use parkan_sim::behaviour::Task;
    use parkan_world::play::CLASS_ANIMAL;

    let (mut play, m) = mission_02_play();
    let zones = &m.clans.iter().find(|c| c.name == "Anml").expect("the animals' clan").zones;
    assert_eq!(zones.len(), 2);
    let medusas: Vec<usize> = (0..play.robots.len())
        .filter(|&r| play.units[play.robots[r].0].type_word & CLASS_ANIMAL != 0)
        .collect();
    assert_eq!(medusas.len(), 2);
    for _ in 0..30 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let pastures: Vec<_> = medusas
        .iter()
        .map(|&r| match play.robots[r].1.behaviour.task() {
            Task::Migrate { pasture: Some(p), started: true, .. } => p,
            other => panic!("medusa {r} is not grazing: {other:?}"),
        })
        .collect();
    assert_eq!(pastures[0], pastures[1], "a clan grazes one pasture at a time");
    let p = pastures[0];
    assert!(
        zones.iter().any(|z| glam::Vec3::from_array(z.position) == p.centre
            && z.inner == p.inner
            && z.outer == p.outer),
        "{p:?} is one of Anml's zones"
    );
    for &r in &medusas {
        let robot = &play.robots[r].1;
        assert!(!robot.wizard.idle(robot.time_ms), "medusa {r} has been sent to a point");
        assert_eq!(play.robots[r].1.fire_target, None, "a grazing medusa aims at nothing");
    }
}

/// Mission 02's L-2f, the design the recording builds, spawned beside the hero at its start and
/// boarded; and the mission.
fn boarded_warbot() -> (parkan_world::play::Play, parkan_formats::mission::Mission, usize) {
    use parkan_world::factory::Project;

    let (mut play, m) = mission_02_play();
    let project = Project {
        path: "UNITS\\bld_unit_-2147483647.dat".to_owned(),
        name: "LFW-2 Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 4,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let hero_at = play.hero.walker.body.position;
    let t = play
        .spawn(&project, play.player_clan, hero_at + glam::Vec3::new(8.0, 0.0, 1.0), 0.0)
        .expect("the L-2f");
    for _ in 0..30 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(play.board(t));
    (play, m, t)
}

/// Hold the boarded bot at `at` for `seconds` of play.
fn hover(play: &mut parkan_world::play::Play, t: usize, at: glam::Vec3, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        let bot = &mut play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1;
        bot.walker.body.position = at;
        bot.walker.body.velocity = [0.0; 3];
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
}

/// Aboard, the hero's behaviour still reports it to the routes, from where the game frame puts
/// its object every frame: the boarded bot's place, its y lowered by the bot's node sphere's
/// radius, 11.84 on the L-2f (`iron3d.dll:0x1005ead6`–`0x1005eb39`, docs/39, "What becomes of
/// the hero"). So route 4 takes the hero only once the bot is 11.84 past its south edge, and
/// already while the bot is 11.84 short of its north-east one: T02_H05, message 14, plays over
/// the second and not the first.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_boarded_hero_is_reported_to_the_routes_from_its_bots_place_less_its_node_sphere_in_y() {
    use glam::Vec3;
    use parkan_sim::progression::contains;

    let (mut play, m, t) = boarded_warbot();
    let r = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.bound.1;
    assert!((r - 11.84).abs() < 0.01, "the L-2f's node sphere: {r}");
    let route = |id: u32| &m.routes.iter().find(|x| x.id == id).unwrap().points;
    let in_any = |p: Vec3| m.routes.iter().any(|x| contains(&x.points, p.x, p.y));
    let south = Vec3::new(1170.0, 695.0, 170.0);
    let north = Vec3::new(1200.0, 1103.0, 170.0);
    let lowered = |p: Vec3| p - Vec3::new(0.0, r, 0.0);
    assert!(contains(route(4), south.x, south.y) && !in_any(lowered(south)));
    assert!(!in_any(north) && contains(route(4), lowered(north).x, lowered(north).y));
    // Over the lake west of route 4, in no route: the script's latch is let go.
    let west = Vec3::new(900.0, 700.0, 170.0);
    assert!(!in_any(west) && !in_any(lowered(west)));
    hover(&mut play, t, west, 12.0);
    let bot = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    assert_eq!(play.hero_place(), lowered(bot));
    let played = |play: &parkan_world::play::Play| {
        play.progression.as_ref().unwrap().progress.played.get(&14).copied()
    };
    assert_ne!(played(&play), Some(true));
    // The bot inside the south edge, the hero reported outside it: nothing.
    hover(&mut play, t, south, 12.0);
    assert_ne!(played(&play), Some(true), "the bot is in route 4, the hero's place is not");
    // The bot outside the north-east edge, the hero reported inside it: T02_H05.
    hover(&mut play, t, north, 12.0);
    assert_eq!(played(&play), Some(true), "the hero's place is in route 4");
}

/// The game frame tests the bot the hero boarded every frame, and once the component test
/// refuses it rolls the stack back to mode 0 (`iron3d.dll:0x1005eab3`–`0x1005eacf`, docs/39,
/// "When the driven bot is lost"): with its turret's body shot off, the L-2f flies on, but
/// the hero is put out at (x − 1, y − 1) beside it, the untested place of a broken bot.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_hero_is_put_out_of_its_warbot_once_the_turrets_body_is_shot_off() {
    use parkan_world::play::Mode;

    let (mut play, _, t) = boarded_warbot();
    assert_eq!(play.mode(), Mode::Driving(t));
    play.tick(1000.0 / 60.0, [0.0; 2]);
    assert_eq!(play.mode(), Mode::Driving(t), "a sound bot keeps the hero aboard");
    let (part, node) = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.turret_life().unwrap();
    let life = play.battle.combat.targets[t].parts[part].life.as_mut().unwrap();
    life.hit(node, f32::MAX / 4.0);
    let bot = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    play.tick(1000.0 / 60.0, [0.0; 2]);
    assert!(play.battle.combat.targets[t].alive, "the bot itself lives");
    assert_eq!(play.mode(), Mode::OnFoot);
    let out = play.hero.walker.body.position;
    assert!((out.truncate() - (bot.truncate() - glam::Vec2::ONE)).length() < 0.05, "{out} beside {bot}");
}

/// The height a flyer's walk point is given (`Behavior.dll:0x10040f20`, docs/24, "A flyer's walk
/// points"): the ground under it as the behaviour's ground routine answers it and a compiled 15.
/// That routine looks straight down from above the world at buildings, trees and stones first,
/// and over one answers its top face and 100 more (`0x100148a5`); over the landscape, the
/// landscape. An animal's point stands 30 higher again, and up to 50 more at random.
#[test]
#[ignore = "needs the game install"]
fn a_flyers_walk_point_stands_15_over_the_ground_and_100_more_over_a_building_an_animals_45_to_95() {
    let (mut play, _) = mission_02_play();
    let factory = play.buildings.iter().find(|b| b.doors.len() == 3).expect("the Large Factory").target;
    // On the forecourt, clear of the factory: the landscape.
    let (x, y) = (395.8, 940.0);
    let land = play.ground.landscape_top(x, y).expect("the forecourt's ground");
    assert_eq!(play.flight_ground(x, y), land);
    assert_eq!(play.flight_height(x, y, false), land + 15.0);
    // Over the hall, where Mission 02's escape once picked its point: the factory's top face
    // there, 17 m and more over the hall floor's 154.05, and 100 more.
    let (x, y) = (408.7, 868.5);
    let roof = play.ground.solid_top(x, y, |i| i == factory).expect("the factory over the point");
    assert!(roof - 154.05 > 15.0, "the roof, not the hall floor: {roof}");
    assert_eq!(play.flight_ground(x, y), roof + 100.0);
    assert_eq!(play.flight_height(x, y, false), roof + 115.0);
    // An animal's, drawn for each point.
    let heights: Vec<f32> = (0..200).map(|_| play.flight_height(395.8, 940.0, true) - land).collect();
    assert!(heights.iter().all(|h| (45.0..=95.0).contains(h)), "{heights:?}");
    let (lo, hi) = heights.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &h| (lo.min(h), hi.max(h)));
    assert!(lo < 50.0 && hi > 90.0, "spread over the 50: {lo} to {hi}");
}

/// Mission 02's warbot, built in the Large Factory, is given the escape, and in the engine its
/// first point falls inside the factory's footprint. That point's height is the factory's roof
/// and 115 more (docs/24, "A flyer's walk points"), so the bot leaves by the front door and
/// climbs clear of the roof to it, where the stand-in's 15 over the hall floor had it pushed
/// up over the shut door onto the roof. Hanging over the factory when the escape ends, it is
/// given the escape again by the unit takt, and ends it on open ground to the east, 15 m up.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_built_warbots_escape_to_a_point_over_its_factory_takes_it_115_m_over_the_roof() {
    use parkan_sim::behaviour::Task;
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
    let f = play.factories.iter().position(|f| f.logic_id == 0x8000_0001_u32 as i32).expect("the factory");
    let factory = play.factories[f].target;
    play.units[factory].clan = Some(play.player_clan);
    play.factories[f].accept(Project {
        path: "UNITS\\bld_unit_-2147483647.dat".into(),
        name: "LFW-2 Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 4,
        ore: 411.0,
        power: 226.5,
        lines: Vec::new(),
        sphere: None,
    });
    let free = play.free_minds(play.player_clan);
    assert!(play.factories[f].start(true, free));
    let robots = play.robots.len();
    while play.robots.len() == robots {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let r = play.robots.len() - 1;
    for _ in 0..10 {
        if matches!(play.robots[r].1.behaviour.task(), Task::Leave { goal: Some(_) }) {
            break;
        }
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let Task::Leave { goal: Some(goal) } = play.robots[r].1.behaviour.task() else {
        panic!("{:?}", play.robots[r].1.behaviour.task())
    };
    let roof = play.ground.solid_top(goal.x, goal.y, |i| i == factory).expect("a point over the factory");

    // The escape's first point reached: over the roof, 115 m up, and the escape ends there.
    let mut highest = f32::MIN;
    let mut ended = None;
    for tick in 0..(30 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        highest = highest.max(play.robots[r].1.walker.body.position.z);
        if !matches!(play.robots[r].1.behaviour.task(), Task::Leave { goal: Some(g) } if g == goal) {
            ended = Some(tick);
            break;
        }
    }
    let at = play.robots[r].1.walker.body.position;
    assert!(ended.is_some(), "the first escape ends: at {at}");
    assert!(highest - roof > 100.0, "it rose {} m over the roof, to {highest}", highest - roof);
    assert!(at.truncate().distance(goal.truncate()) < 5.0, "over its point: {at} to {goal}");

    // Then off the factory: the takt's escape takes it out onto the landscape and it hangs 15 m
    // over the ground where that escape ends.
    let mut settled = None;
    for tick in 0..(40 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if play.robots[r].1.behaviour.task() == Task::Stop {
            settled = Some(tick);
            break;
        }
    }
    let at = play.robots[r].1.walker.body.position;
    assert!(settled.is_some(), "the escape off the factory ends: at {at}");
    let land = play.ground.landscape_top(at.x, at.y).expect("off the factory, on the landscape");
    assert!(play.ground.solid_top(at.x, at.y, |i| i == factory).is_none(), "clear of the factory: {at}");
    assert!((at.z - land - 15.0).abs() < 3.0, "15 m over the ground: {} over {land}", at.z - land);
}
