//! Mission 03's own run: its pods, the ways the hero walks to them, its patrol and its win.

use crate::common::*;

#[test]
#[ignore = "needs the game install"]
fn every_mission_03_pod_holds_a_hero_on_its_floor_between_its_parents_box_and_fires() {
    use parkan_world::buildings::Phase;

    // docs/27, "The zone's height is the pod node's parent's box": the node the pod plays, its
    // parent, and the parent's box in model space.
    let expected = [
        ("gener01.dat", 4, 3, (-12.48, -6.72)),
        ("sbunk01.dat", 5, 4, (-13.96, -3.41)),
        ("sstore01.dat", 11, 10, (-29.24, -18.69)),
        ("lplant01.dat", 25, 23, (-14.32, -3.77)),
    ];
    for (name, pod_node, parent, (low, high)) in expected {
        let (mut play, m) = mission_03_play();
        let t = object_target(&play, &m, name);
        let b = play.buildings.iter().position(|b| b.target == t).expect("a pod");
        let building = &play.buildings[b];
        let part = &play.battle.combat.targets[t].parts[building.part];
        let pod = building.pod.as_ref().unwrap();
        assert_eq!((pod.node, usize::from(part.mesh.nodes[pod.node].parent)), (pod_node, parent), "{name}");
        let z = play.battle.combat.targets[t].position.z;
        let (l, h) = building.zone_heights(part).unwrap();
        assert!((l - z - low).abs() < 0.02 && (h - z - high).abs() < 0.02, "{name}: {l} to {h} over {z}");
        assert!(play.stand_on_pod(t), "{name}");
        let mut fired_at = None;
        for tick in 0..(60 * 8) {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
            if play.buildings[b].pod.as_ref().unwrap().fired {
                fired_at = Some(tick);
                break;
            }
        }
        let centre = play.hero.collision_centre();
        let feet = play.hero.walker.body.position;
        assert!(
            fired_at.is_some(),
            "{name}: the hero's centre {centre} (feet {feet}) stands between {l} and {h}; phase {:?}",
            play.buildings[b].pod.as_ref().map(|p| p.phase)
        );
        assert_ne!(play.buildings[b].pod.as_ref().unwrap().phase, Phase::Shut);
    }
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_hero_on_the_generators_pod_captures_it_and_completes_the_first_objective() {
    use parkan_world::progress::{Say, Sender};

    let (mut play, m) = mission_03_play();
    let t = object_target(&play, &m, "gener01.dat");
    assert_eq!(play.units[t].clan, Some(2), "neutral");
    assert!(play.stand_on_pod(t));
    let mut says = Vec::new();
    for _ in 0..(60 * 6) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        says.append(&mut play.says);
    }
    assert_eq!(play.units[t].clan, Some(play.player_clan));
    assert!(
        says.iter().any(|s| matches!(s, Say::Text(Sender::System, text) if text == "Building is captured")),
        "{says:?}"
    );
    // `CLASS_BUILDING|7` is the generator's (docs/34, "Mission 03").
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.owner(0x8000_0007_u32 as i32), 0);
    assert_eq!(p.progress.objectives[0].state, 1, "Find and capture the Power Generator");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_patrol_waits_shut_down_until_the_fourth_bot_then_flies_to_the_base_and_its_end_wins() {
    use parkan_sim::behaviour::{Guarded, Task};
    use parkan_world::factory::Project;

    let (mut play, _) = mission_03_play();
    let script = play.progression.as_ref().unwrap().script.as_ref().unwrap();
    // `Init` counts the hero, the builder and the transport: objective 3 wants four bots more.
    assert_eq!((script.dword("df5"), script.dword("df6")), (Some(7), Some(4)));
    let enemy: Vec<usize> =
        play.robots.iter().filter(|(t, _)| play.units[*t].clan == Some(1)).map(|r| r.0).collect();
    assert_eq!(enemy.len(), 3);
    let ids: Vec<i32> = enemy.iter().map(|&t| play.units[t].logical_id).collect();
    assert_eq!(ids, [3, 4, 5]);
    assert!(
        play.robots
            .iter()
            .filter(|(t, _)| enemy.contains(t))
            .all(|(_, r)| r.behaviour.task() == Task::Shutdown)
    );

    // Shut down, the patrol neither moves nor fires at the hero 200 m off.
    assert!(play.stand_facing(enemy[0], 200.0, 0.0));
    let start: Vec<glam::Vec3> = enemy.iter().map(|&t| play.battle.combat.targets[t].position).collect();
    let mut shots = 0;
    play_for(&mut play, 8.0, |p| {
        shots +=
            p.battle.combat.rounds.iter().filter(|r| r.owner.is_some_and(|o| enemy.contains(&o))).count();
    });
    assert_eq!(shots, 0, "a shut-down unit does not fire");
    for (&t, from) in enemy.iter().zip(&start) {
        let at = play.battle.combat.targets[t].position;
        assert!(at.distance(*from) < 2.0, "a shut-down unit does not move: {from} → {at}");
    }

    // Four bots of the player's: the next `Mission` run completes objective 3 and sends the patrol.
    assert!(play.stand_at(1750.0, 250.0, 0.0), "the hero out of the fight");
    let project = Project {
        path: "UNITS\\UNITS\\PREBLD\\tut3_p1.dat".into(),
        name: "SSW-X Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 2,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    for k in 0..4 {
        let (x, y) = (1700.0 + 12.0 * k as f32, 300.0);
        let z = play.ground.below(x, y, 10_000.0).map_or(0.0, |h| h.point.z) + 1.0;
        let clan = play.player_clan;
        assert!(play.spawn(&project, clan, glam::Vec3::new(x, y, z), 0.0).is_some());
    }
    play_for(&mut play, 3.0, |_| {});
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[3].state, 1, "objective 3: four warbots");
    let guarded: Vec<Task> =
        enemy.iter().map(|&t| play.robots.iter().find(|r| r.0 == t).unwrap().1.behaviour.tasks[0]).collect();
    let place = |t: &Task| match t {
        Task::Patrol { guarded: Guarded::Place(p), radius, .. } => Some((p.x, p.y, p.z, *radius)),
        _ => None,
    };
    assert_eq!(place(&guarded[0]), Some((1124.0, 783.0, 0.0, 60.0)));
    assert_eq!(place(&guarded[1]), Some((606.0, 993.0, 0.0, 60.0)));
    assert_eq!(place(&guarded[2]), Some((1124.0, 783.0, 0.0, 60.0)));

    // They fly there at 0.8 of their speed: 1.3 km in well under two minutes.
    let mut nearest = f32::MAX;
    play_for(&mut play, 100.0, |p| {
        let at = p.battle.combat.targets[enemy[0]].position;
        nearest = nearest.min(at.truncate().distance(glam::Vec2::new(1124.0, 783.0)));
    });
    assert!(nearest < 100.0, "tut3_f1 came within {nearest} m of its place");

    // Destroyed, the patrol completes objective 4, and with 0–3 done the mission is won.
    for o in &mut play.progression.as_mut().unwrap().progress.objectives[..3] {
        o.state = 1;
    }
    play_for(&mut play, 6.0, |p| {
        for (t, r) in &mut p.robots {
            if enemy.contains(t) {
                r.ground_damage.read(1.0e6);
            }
        }
    });
    let p = play.progression.as_ref().unwrap();
    assert!(enemy.iter().all(|&t| !play.battle.combat.targets[t].alive));
    assert_eq!(p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>(), [1, 1, 1, 1, 1]);
    assert_eq!(p.progress.outcome, Some(true), "MISSION COMPLETE");
}

/// Mission 03's Small Generator's hall way from its south exit (vertex 51) to its pod (16), from
/// (647.73, 987.06) facing north (docs/24, "The ways into Mission 03's Small Generator and Small
/// Bunker").
const GENERATOR_ROUTE: [[f32; 2]; 14] = [
    [648.26, 995.35],
    [648.55, 999.96],
    [649.76, 1019.04],
    [650.12, 1024.73],
    [650.78, 1035.22],
    [659.57, 1036.86],
    [663.95, 1032.77],
    [668.69, 1029.95],
    [674.04, 1030.13],
    [678.53, 1034.81],
    [678.95, 1041.43],
    [672.62, 1049.86],
    [665.26, 1050.32],
    [659.45, 1050.69],
];

/// Mission 03's Small Bunker's hall way from its one exit (vertex 43) down the ramp, through the
/// door and the corridors to the pod (6), from (1198.14, 829.49) facing east.
const BUNKER_ROUTE: [[f32; 2]; 13] = [
    [1209.39, 825.78],
    [1225.93, 822.23],
    [1243.24, 818.02],
    [1257.07, 814.87],
    [1267.09, 812.64],
    [1265.22, 804.06],
    [1262.78, 792.13],
    [1261.52, 786.69],
    [1265.58, 785.58],
    [1274.34, 783.54],
    [1282.90, 781.75],
    [1286.42, 796.31],
    [1288.90, 807.37],
];

#[test]
#[ignore = "needs the game install"]
fn mission_03s_hero_walks_into_the_small_generator_from_the_south_and_its_pod_captures_it() {
    // The hall way's shortest way from the south exit (vertex 51) to the pod (16), docs/24,
    // "The ways into Mission 03's Small Generator and Small Bunker".
    let route = GENERATOR_ROUTE;
    let (mut play, m) = mission_03_play();
    let t = object_target(&play, &m, "gener01.dat");
    assert!(play.stand_at(647.73, 987.06, 0.0));
    let player = play.player_clan;
    let (on_floor, arrived, next, captured) =
        walk_route(&mut play, &route, 40, |p| p.units[t].clan == Some(player));
    let at = play.hero.walker.body.position;
    let arrived = arrived.unwrap_or_else(|| panic!("stopped before vertex {next} of the way, at {at}"));
    let (on_floor, captured) = (on_floor.expect("on the generator's floor"), captured.expect("captured"));
    assert!((at.z - 81.4).abs() < 1.0, "on the pod room's floor: {at}");
    assert!(arrived < 12 * 60, "on the pod {:.1} s in", arrived as f32 / 60.0);
    // The recording's hero is on the apron at 108.0 s and sees the capture at 116.5 s.
    let apron_to_capture = (captured - on_floor) as f32 / 60.0;
    eprintln!(
        "generator: on the apron {:.2} s, on the pod {:.2} s, captured {:.2} s",
        on_floor as f32 / 60.0,
        arrived as f32 / 60.0,
        captured as f32 / 60.0
    );
    assert!((7.0..11.0).contains(&apron_to_capture), "apron to capture {apron_to_capture:.1} s");
    // The Mission handler, every 2 s, completes the objective.
    for _ in 0..150 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[0].state, 1, "Find and capture the Power Generator");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_hero_walks_down_the_small_bunkers_ramp_to_its_pod_and_command_mode_opens() {
    use parkan_world::play::Mode;

    // The hall way's way from its one exit (vertex 43) down the ramp, through the door and the
    // corridors to the pod (6), docs/24, "The ways into Mission 03's Small Generator and Small
    // Bunker".
    let route = BUNKER_ROUTE;
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    assert!(play.stand_at(1198.14, 829.49, -std::f32::consts::FRAC_PI_2));
    let (on_floor, arrived, next, opened) = walk_route(&mut play, &route, 60, |p| p.mode() != Mode::OnFoot);
    let at = play.hero.walker.body.position;
    let arrived = arrived.unwrap_or_else(|| panic!("stopped before vertex {next} of the way, at {at}"));
    let (on_floor, opened) = (on_floor.expect("on the bunker's ramp"), opened.expect("the pod fired"));
    assert!((at.z - 69.7).abs() < 1.0, "on the pod's floor: {at}");
    assert!(arrived < 20 * 60, "on the pod {:.1} s in", arrived as f32 / 60.0);
    assert_eq!(play.mode(), Mode::Command(bunker), "the capture opens command mode");
    // The recording's hero starts down the ramp by 164.4 s, and command mode is up at 178.2 s.
    let ramp_to_command = (opened - on_floor) as f32 / 60.0;
    eprintln!(
        "bunker: on the ramp {:.2} s, on the pod {:.2} s, command mode {:.2} s",
        on_floor as f32 / 60.0,
        arrived as f32 / 60.0,
        opened as f32 / 60.0
    );
    assert!((12.0..17.0).contains(&ramp_to_command), "ramp to command mode {ramp_to_command:.1} s");
    for _ in 0..150 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[1].state, 1, "Find and capture the Bunker");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03_is_won_by_the_generator_the_bunker_a_mine_four_warbots_and_the_patrol_beaten() {
    use parkan_world::cockpit::commander::Panel;
    use parkan_world::cockpit::factory::Click;
    use parkan_world::pick::Aim;
    use parkan_world::play::Mode;

    let (mut play, m) = mission_03_play();
    let player = play.player_clan;
    let generator = object_target(&play, &m, "gener01.dat");
    let bunker = object_target(&play, &m, "sbunk01.dat");
    let plant = object_target(&play, &m, "lplant01.dat");
    let objectives = |p: &parkan_world::play::Play| {
        p.progression.as_ref().unwrap().progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>()
    };

    // The hero takes the Small Generator from its pod, then walks down the Small Bunker's ramp to
    // its pod, which opens command mode.
    assert!(play.stand_at(647.73, 987.06, 0.0));
    let (_, _, _, taken) =
        walk_route(&mut play, &GENERATOR_ROUTE, 40, |p| p.units[generator].clan == Some(player));
    assert!(taken.is_some(), "the generator is taken");
    assert!(play.stand_at(1198.14, 829.49, -std::f32::consts::FRAC_PI_2));
    let (_, _, _, opened) = walk_route(&mut play, &BUNKER_ROUTE, 60, |p| p.mode() != Mode::OnFoot);
    assert!(opened.is_some() && play.mode() == Mode::Command(bunker), "command mode");
    play_for(&mut play, 2.5, |_| {});
    assert_eq!(objectives(&play)[..2], [1, 1]);

    // The Builders page offers Build Mine; the ghost goes green on the lode and the builder goes.
    let mut now = play.hero.time_ms;
    let mut panel = Panel::default();
    panel.update(&mut play, now);
    panel.turn(&mut play, 3, now);
    assert!(panel.menu.contains(&10), "{:?}", panel.menu);
    let act = play.hq_command(10).expect("Build Mine");
    play.open_pick(act);
    play.command_frame(now / 1000.0, parkan_world::command::Edges::default());
    let lode = play.commander.lodes[0].position;
    let site = play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point;
    let eye = play.eye().position;
    play.update_ghost(Aim::Ray { eye, direction: (site - eye).normalize() });
    assert!(play.commander.ghost.as_ref().is_some_and(|g| g.valid), "green on the lode");
    assert!(play.commit_placement());

    // The transport carries minerals, and the factory builds SSW-X in batch.
    panel.turn(&mut play, 2, now);
    assert!(play.hq_command(8).is_some());
    let f = play.factories.iter().position(|f| f.target == plant).unwrap();
    let ssw = play.factories[f].projects.iter().position(|p| p.name.starts_with("SSW-X")).unwrap();
    play.factories[f].selected = Some(ssw);
    play.factory_click(plant, Click::Batch);
    assert!(play.factories[f].build.is_some(), "batch production starts");

    // Play on until the mission is won: the mine counts, four bots are built, the patrol comes.
    // As the recording's player does (docs/31, "Seen in a recording"), each new warbot is told to
    // guard the Small Generator once it is out of the factory, by the second patrol's place.
    let guard = parkan_sim::orders::Order {
        code: parkan_sim::orders::PATROL,
        parameter: 0,
        target: parkan_sim::orders::Target::LogicId(play.units[generator].logical_id),
    };
    let mut hunting = false;
    let mut done = [None; 5];
    let mut outcome = None;
    for second in 0..900 {
        play_for(&mut play, 1.0, |_| {});
        now = play.hero.time_ms;
        panel.update(&mut play, now);
        for (i, state) in objectives(&play).into_iter().enumerate() {
            if state == 1 && done[i].is_none() {
                done[i] = Some(second);
                eprintln!("objective {} complete {} s after command mode", i + 1, second);
            }
        }
        let out: Vec<usize> = play
            .own_units_within(parkan_world::selection::BATTLE_UNITS)
            .into_iter()
            .filter(|t| {
                play.robots.iter().find(|(rt, _)| rt == t).is_some_and(|(_, r)| {
                    r.order.is_none_or(|o| o.code == parkan_sim::orders::LEAVE) && r.wizard.idle(now)
                })
            })
            .collect();
        for t in out {
            play.select_unit_alone(t);
            play.dispatch(guard);
        }
        // Once a flyer is down, the player sends every warbot to seek and destroy the others.
        let enemies = play
            .robots
            .iter()
            .filter(|(t, _)| play.units[*t].clan == Some(1) && play.battle.combat.targets[*t].alive)
            .count();
        if done[3].is_some() && enemies < 3 && !hunting {
            hunting = true;
            for t in play.own_units_within(parkan_world::selection::BATTLE_UNITS) {
                play.select_unit_alone(t);
                assert!(play.hq_command(3).is_some(), "Seek and destroy");
            }
        }
        outcome = play.progression.as_ref().unwrap().progress.outcome;
        if outcome.is_some() {
            break;
        }
    }
    let robots = play.own_units_within(parkan_world::selection::BATTLE_UNITS).len();
    assert_eq!(outcome, Some(true), "objectives {done:?}, {robots} battle units");
}

/// Every building every shipped mission places is named as `iron3d.dll:0x100338d0` names it,
/// by its Type and its root record's size letter, and none of the 167 is *"Unknown"*
/// (docs/35, "Name and status"): Mission 03's Small Generator, Small Warehouse, Large Factory
/// and Small Bunker among them, and the 19 bridges, 3 ruins, 5 main teleports and 4 enhanced
/// institutes the engine left unnamed or misnamed before.
#[test]
#[ignore = "needs the game install"]
fn every_placed_building_is_named_by_its_type_and_size() {
    use parkan_formats::gamedir;
    use parkan_formats::mission::{self, KIND_BUILDING, Value};
    use parkan_world::assembly::Assembly;
    use parkan_world::selection::{building_name_id, building_size};
    use std::collections::BTreeMap;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let assembly = Assembly::new(&game).unwrap();
    let root = game.join("MISSIONS");
    let with_data = |d: &std::path::Path| d.is_dir() && d.join("data.tma").exists();
    let mut dirs: Vec<_> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).filter(|d| with_data(d)).collect();
    for campaign in std::fs::read_dir(root.join("CAMPAIGN")).unwrap().map(|e| e.unwrap().path()) {
        if campaign.is_dir() {
            dirs.extend(std::fs::read_dir(&campaign).unwrap().map(|e| e.unwrap().path()).filter(|d| with_data(d)));
        }
    }
    let mut names: BTreeMap<String, usize> = BTreeMap::new();
    for dir in &dirs {
        let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), &dir.display().to_string()).unwrap();
        for o in m.objects.iter().filter(|o| o.kind == KIND_BUILDING) {
            let type_word = match o.property("Type").map(|p| p.value) {
                Some(Value::Int(v)) => v as u32,
                Some(Value::Float(v)) => v as i64 as u32,
                None => 0,
            };
            let size = assembly.records(&o.path).first().map_or(0, |r| building_size(r));
            let name = strings.get(&building_name_id(type_word, size)).cloned().unwrap_or_default();
            *names.entry(name).or_default() += 1;
        }
    }
    let expected: BTreeMap<String, usize> = [
        ("Small Generator", 47),
        ("Small Mine", 5),
        ("Medium Mine", 5),
        ("Large Mine", 5),
        ("Small Warehouse", 4),
        ("Small Factory", 10),
        ("Medium Factory", 2),
        ("Large Factory", 16),
        ("Small Outpost", 6),
        ("Small Res. Center", 4),
        ("Medium Res. Center", 1),
        ("Enhanced Res. Center", 4),
        ("Teleport", 5),
        ("Bridge", 19),
        ("Ruins", 3),
        ("Small Bunker", 19),
        ("Medium Bunker", 5),
        ("Large Bunker", 1),
        ("Light Tower", 6),
    ]
    .into_iter()
    .map(|(n, c)| (n.to_owned(), c))
    .collect();
    assert_eq!(dirs.len(), 29, "{dirs:?}");
    assert_eq!(names, expected);
}
