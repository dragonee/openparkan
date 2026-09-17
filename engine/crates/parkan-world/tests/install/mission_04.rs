//! Mission 04's own run: its objectives, its teleport and its two ways to win.

use crate::common::*;
use parkan_formats::gamedir;

/// Mission 04's Main Teleport's hall way (docs/27, "What the building is"): from the third
/// exit (vertex 10) up the stair to the pod (12); from the arc's east end (4) to its in place
/// (6); and in the chamber from where the in place lands a hero (3) past 1 and 2 to the out
/// place (0), in the world.
const TELEPORT_POD_ROUTE: [[f32; 2]; 3] = [[1189.4, 1226.7], [1186.5, 1229.9], [1178.3, 1238.9]];

const TELEPORT_ARC_ROUTE: [[f32; 2]; 2] = [[1178.7, 1288.7], [1155.1, 1267.2]];

const TELEPORT_CHAMBER_ROUTE: [[f32; 2]; 3] = [[1142.7, 1270.8], [1156.2, 1264.6], [1175.3, 1242.1]];

const TELEPORT_LANDING: [f32; 3] = [1126.305, 1254.558, 68.914];

fn tick_for(play: &mut parkan_world::play::Play, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
}

#[test]
#[ignore = "needs the game install"]
fn mission_04_lists_six_objectives_under_a_repeated_key_and_greets_the_hero_in_route_0() {
    let (mut play, _) = mission_04_play();
    let p = play.progression.as_ref().unwrap();
    let texts = &p.objective_texts;
    assert_eq!(texts.len(), 6, "{texts:?}");
    assert!(texts[3].starts_with("4. Develop") && texts[4].starts_with("5.") && texts[5].starts_with("6."));
    assert!(p.progress.objectives.iter().all(|o| !o.exempt && o.state == 0));
    tick_for(&mut play, 3.0);
    let p = play.progression.as_ref().unwrap();
    assert!(p.progress.played[&9], "T04_I01, the hero in route 0");
    assert!(!p.progress.played[&17] && !p.progress.played[&16]);
}

#[test]
#[ignore = "needs the game install"]
fn mission_04s_hero_climbs_the_teleports_stair_and_its_pod_takes_it_opening_nothing_and_winning_nothing() {
    use parkan_world::play::Mode;
    use parkan_world::progress::{Say, Sender};

    let (mut play, m) = mission_04_play();
    let t = object_target(&play, &m, "mtp_m_n1.dat");
    assert_eq!(play.units[t].clan, Some(1), "neutral");
    // The route messages share one latch, which a run finding the hero in no route opens: out of
    // route 0 for longer than a takt's report and a Mission run take, then outside the third
    // exit, in route 1.
    assert!(play.stand_at(700.0, 700.0, 0.0));
    tick_for(&mut play, 8.0);
    assert!(!play.progression.as_ref().unwrap().progress.areals.holds(0, i64::from(play.hero_id)));
    assert!(play.stand_at(1193.4, 1222.3, 0.74));
    let player = play.player_clan;
    let (_, arrived, next, captured) =
        walk_route(&mut play, &TELEPORT_POD_ROUTE, 30, |p| p.units[t].clan == Some(player));
    let at = play.hero.walker.body.position;
    let arrived = arrived.unwrap_or_else(|| panic!("stopped before vertex {next} of the stair, at {at}"));
    let captured = captured.expect("the pod fired");
    assert!((at.z - 101.6 - 1.4).abs() < 0.5, "on the pod room's floor: {at}");
    let wait = (captured as f32 - arrived as f32) / 60.0;
    assert!((1.0..3.0).contains(&wait), "the pod fired {wait:.2} s after the hero reached it");
    assert!(
        play.says
            .iter()
            .any(|s| matches!(s, Say::Text(Sender::System, text) if text == "Building is captured")),
        "{:?}",
        play.says
    );
    // A main teleport opens no screen and selects nothing (`0x100626f2`).
    assert_eq!(play.mode(), Mode::OnFoot);
    assert!(!play.selected.contains(&t));
    tick_for(&mut play, 3.0);
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[4].state, 1, "5. Find and capture the Teleport");
    assert!(p.progress.played[&14], "T04_I06");
    assert!(p.progress.played[&17], "T04_H03, the hero in route 1: {:?}", p.progress.played);
    assert_eq!(p.progress.objectives[5].state, 0, "6. is never completed");
    assert_eq!(p.progress.outcome, None, "the capture wins nothing");
}

#[test]
#[ignore = "needs the game install"]
fn standing_on_the_main_teleports_pod_puts_the_hero_on_its_room_floor_over_the_node_and_it_fires() {
    let (mut play, m) = mission_04_play();
    let t = object_target(&play, &m, "mtp_m_n1.dat");
    // The pod room's floor stands 4.4 over the computer node's centre.
    assert!(play.stand_on_pod(t));
    tick_for(&mut play, 4.0);
    let at = play.hero.walker.body.position;
    assert!((at.z - 101.6 - 1.4).abs() < 0.5, "on the pod room's floor: {at}");
    assert_eq!(play.units[t].clan, Some(play.player_clan));
}

#[test]
#[ignore = "needs the game install"]
fn mission_04s_teleport_arc_sends_the_hero_down_only_while_the_player_holds_it_and_every_generator() {
    let (mut play, m) = mission_04_play();
    let t = object_target(&play, &m, "mtp_m_n1.dat");
    let generator = object_target(&play, &m, "gener01.dat");
    let player = play.player_clan;
    assert_eq!(play.units[generator].clan, Some(player), "Mission 04's one generator");
    // Under the arc of a neutral teleport nothing happens.
    assert!(play.stand_at(1184.0, 1293.5, 0.84));
    let (_, arrived, _, sent) =
        walk_route(&mut play, &TELEPORT_ARC_ROUTE, 20, |p| p.hero.walker.body.position.z < 90.0);
    assert!(arrived.is_some() && sent.is_none(), "{}", play.hero.walker.body.position);
    // The player's teleport beside a generator of another clan: still nothing.
    play.units[t].clan = Some(player);
    play.units[generator].clan = Some(1);
    tick_for(&mut play, 1.0);
    assert!(play.hero.walker.body.position.z > 100.0);
    // Every generator the player's: the next place tick puts the hero on the landing vertex,
    // turned as it was.
    play.units[generator].clan = Some(player);
    let yaw = play.hero.walker.body.yaw;
    let mut ticks = 0;
    while play.hero.walker.body.position.z > 90.0 && ticks < 60 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        ticks += 1;
    }
    let at = play.hero.walker.body.position;
    assert!(ticks <= 9, "moved {ticks} ticks after the generator came back, to {at}");
    assert!(at.distance(glam::Vec3::from_array(TELEPORT_LANDING)) < 0.05, "on vertex 3: {at}");
    assert!((play.hero.walker.body.yaw - yaw).abs() < 1e-4, "turned as it was");
    // It drops to the chamber's floor under the landscape.
    tick_for(&mut play, 1.5);
    let ground = play.hero.walker.ground.expect("on a floor");
    assert!(ground.solid.is_some_and(|(s, _)| s == t), "the teleport's own floor: {ground:?}");
    assert!((ground.point.z - 65.6).abs() < 1.0, "the chamber's floor: {ground:?}");
}

#[test]
#[ignore = "needs the game install"]
fn mission_04_is_won_once_as_the_hero_walks_up_the_teleports_chamber_to_its_field() {
    use parkan_world::progress::{Say, VOICE_MISSION_COMPLETE};

    let (mut play, m) = mission_04_play();
    let t = object_target(&play, &m, "mtp_m_n1.dat");
    let player = play.player_clan;
    play.units[t].clan = Some(player);
    let [x, y, z] = TELEPORT_LANDING;
    assert!(play.stand_below(x, y, z, 0.79), "the chamber under the landscape");
    let (_, _, next, won) = walk_route(&mut play, &TELEPORT_CHAMBER_ROUTE, 20, |p| {
        p.progression.as_ref().unwrap().progress.outcome.is_some()
    });
    let won =
        won.unwrap_or_else(|| panic!("heading for point {next}, at {}", play.hero.walker.body.position));
    // The recording's hero reaches the field 4.6 s after the chamber: 68.8 m, less the place's 5.
    let seconds = won as f32 / 60.0;
    assert!((3.5..7.0).contains(&seconds), "won {seconds:.2} s up the chamber");
    play.hero.key("SCAN_W", false);
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.outcome, Some(true));
    assert_eq!(p.progress.objectives[5].state, 0, "won with the sixth objective open");
    assert!(p.panel().is_some_and(|panel| panel.won));
    let voice = p.sound(VOICE_MISSION_COMPLETE).expect("its voice");
    let wins = |says: &[Say]| says.iter().filter(|s| **s == Say::Voice(voice.clone())).count();
    assert_eq!(wins(&play.says), 1);
    // Standing on at the field raises the handler each place tick; a repeated outcome is silent.
    tick_for(&mut play, 2.0);
    assert_eq!(wins(&play.says), 1);
}

/// Turns the driven bot's turret toward `to` with mouse counts, as a player does, so that its
/// hull comes round under it; W flies it on while it is farther than `near`, and R and F hold it
/// `above` over the higher of the ground below and 40 m ahead. Whether it got there.
fn fly_to(
    play: &mut parkan_world::play::Play,
    to: glam::Vec2,
    near: f32,
    above: f32,
    seconds: usize,
) -> bool {
    use std::f32::consts::{PI, TAU};
    let wrap = |a: f32| (a + PI).rem_euclid(TAU) - PI;
    let heading = |v: glam::Vec2| (-v.x).atan2(v.y);
    let mut held = [false; 3];
    let mut there = false;
    for _ in 0..(seconds * 60) {
        let body = play.driven().walker.body.position;
        let way = to - body.truncate();
        there = way.length() <= near;
        let ahead = body.truncate() + way.normalize_or_zero() * 40.0;
        let ground = |p: glam::Vec2| play.ground.below(p.x, p.y, 1.0e5).map_or(0.0, |h| h.point.z);
        let want = ground(body.truncate()).max(ground(ahead)) + above;
        let keys = [!there, body.z < want - 3.0, body.z > want + 3.0];
        for (i, key) in ["SCAN_W", "SCAN_R", "SCAN_F"].into_iter().enumerate() {
            if held[i] != keys[i] {
                play.key(key, keys[i]);
            }
        }
        held = keys;
        if there {
            break;
        }
        let look = play.eye().forward.truncate();
        let counts = (-wrap(heading(way) - heading(look)) * 40.0).clamp(-25.0, 25.0);
        play.update_input();
        play.tick(1000.0 / 60.0, [counts, 0.0]);
    }
    for (i, key) in ["SCAN_W", "SCAN_R", "SCAN_F"].into_iter().enumerate() {
        if held[i] {
            play.key(key, false);
        }
    }
    there
}

#[test]
#[ignore = "needs the game install"]
fn mission_04_is_won_by_the_hq_the_helicopters_captures_research_a_large_flyer_and_the_teleport() {
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::factory::Click;
    use parkan_world::hud::Pages;
    use parkan_world::play::Mode;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m, hq) = mission_04_aboard_the_hq();
    let player = play.player_clan;
    let heli = object_target(&play, &m, "tut4_f1.dat");
    let plant = object_target(&play, &m, "lplant01.dat");
    let centre = object_target(&play, &m, "einst01.dat");
    let teleport = object_target(&play, &m, "mtp_m_n1.dat");
    let objectives = |p: &parkan_world::play::Play| {
        p.progression.as_ref().unwrap().progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>()
    };
    let seconds = |p: &parkan_world::play::Play| (p.hero.time_ms / 1000.0) as f32;
    let tick = 1.0 / 60.0;

    // One Enter took and boarded the HQ; a second, aboard, opens its command view.
    assert_eq!(play.mode(), Mode::Driving(hq));
    command_frames(&mut play, 60, |_| {});
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::HqCommand(hq));
    // The Mission handler, every 2 s, completes the first objective.
    command_frames(&mut play, 150, |_| {});
    assert_eq!(objectives(&play)[0], 1, "1. Capture the mobile HQ");

    // The Battle units page gives the helicopter Search and capture: the Large Factory, then the
    // Research Center.
    play.commander.units = vec![heli];
    play.select_unit_alone(heli);
    play.hq_command(2).expect("Search and capture");
    for _ in 0..(240 * 60) {
        command_frames(&mut play, 1, |_| {});
        if play.units[centre].clan == Some(player) {
            break;
        }
    }
    assert_eq!(play.units[plant].clan, Some(player), "the factory is taken");
    assert_eq!(play.units[centre].clan, Some(player), "the research centre is taken");
    play_for(&mut play, 2.5, |_| {});
    assert_eq!(objectives(&play)[1..3], [1, 1]);
    eprintln!("both taken {:.1} s in", seconds(&play));

    // The one row, the Large Battle Turret, researched free in 5 s.
    let rows = play.research_rows();
    assert_eq!(rows.len(), 1);
    assert!(play.order_research(rows[0]));
    play_for(&mut play, 6.0, |_| {});
    assert!(play.research_rows().is_empty(), "researched");

    // The factory's designer: the L-2f with the turret, accepted and built free in a minute.
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    cockpit.designer.open(&mut play, plant, &cockpit.strings).unwrap();
    for part in ["R_B_02", "e_tur_bb_01"] {
        let session = cockpit.designer.session.as_mut().unwrap();
        assert!(session.fit_part(part, &mut play.assembly, &cockpit.strings), "{part} fits");
    }
    cockpit.designer.click(&mut play, [213.0, 462.0], &cockpit.strings);
    let f = play.factories.iter().position(|f| f.target == plant).unwrap();
    play.factories[f].selected = Some(play.factories[f].projects.len() - 1);
    assert_eq!(play.free_minds(player), 1, "the recording's one free mind");
    play.factory_click(plant, Click::Build);
    assert!(play.factories[f].build.is_some(), "the build starts");
    let before: Vec<usize> = play.robots.iter().map(|(t, _)| *t).collect();
    let mut flyer = None;
    for _ in 0..(70 * 60) {
        play_for(&mut play, tick, |_| {});
        flyer = play.robots.iter().map(|(t, _)| *t).find(|t| !before.contains(t));
        if flyer.is_some() {
            break;
        }
    }
    let flyer = flyer.expect("the large flyer is built");
    play_for(&mut play, 3.0, |_| {});
    assert_eq!(objectives(&play)[3], 1, "4. Develop and build a large flying warbot");

    // Routed to the HQ; the player leaves the view and the HQ, walks over and boards the flyer.
    let at = play.battle.combat.targets[hq].position;
    play.select_unit_alone(flyer);
    play.dispatch(parkan_sim::orders::Order {
        code: parkan_sim::orders::GO,
        parameter: 0,
        target: parkan_sim::orders::Target::Place(at.to_array()),
    });
    let by_hq = (0..150).any(|_| {
        play_for(&mut play, 1.0, |_| {});
        play.battle.combat.targets[flyer].position.truncate().distance(at.truncate()) < 30.0
    });
    assert!(by_hq, "the flyer comes to the HQ: {}", play.battle.combat.targets[flyer].position);
    play.roll_back_to_foot();
    assert_eq!(play.mode(), Mode::OnFoot);
    play_for(&mut play, 3.0, |_| {});
    play.hero.key("SCAN_W", true);
    for _ in 0..(30 * 60) {
        let to =
            play.battle.combat.targets[flyer].position.truncate() - play.hero.walker.body.position.truncate();
        if to.length() < 12.0 {
            break;
        }
        play.hero.walker.body.yaw = (-to.x).atan2(to.y);
        play_for(&mut play, tick, |_| {});
    }
    play.hero.key("SCAN_W", false);
    play_for(&mut play, 1.0, |_| {});
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(flyer) {
            break;
        }
        play.targets.select_next();
    }
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::Driving(flyer), "aboard the flyer");

    // Flown up onto the Teleport's plateau, set down with F, and left.
    assert!(fly_to(&mut play, glam::Vec2::new(1200.0, 1210.0), 15.0, 30.0, 240), "over the plateau");
    play.key("SCAN_F", true);
    let mut out = false;
    for _ in 0..(20 * 60) {
        play_for(&mut play, tick, |_| {});
        let body = play.driven().walker.body.position;
        let ground = play.ground.below(body.x, body.y, 1.0e5).map_or(0.0, |h| h.point.z);
        if body.z - ground < 11.0 {
            play.key("SCAN_F", false);
            play_for(&mut play, 0.5, |_| {});
            out = play.roll_back();
            if out {
                break;
            }
            play.key("SCAN_F", true);
        }
    }
    assert!(out && play.mode() == Mode::OnFoot, "out by the Teleport");

    // The hero climbs the stair and takes the Teleport's pod: the fifth objective, no win yet.
    let mut stair = vec![[1193.4, 1222.3]];
    stair.extend(TELEPORT_POD_ROUTE);
    let (_, _, _, taken) = walk_route(&mut play, &stair, 60, |p| p.units[teleport].clan == Some(player));
    assert!(taken.is_some(), "the Teleport is taken, the hero at {}", play.hero.walker.body.position);
    play_for(&mut play, 3.0, |_| {});
    assert_eq!(objectives(&play)[4], 1, "5. Find and capture the Teleport");
    assert_eq!(play.progression.as_ref().unwrap().progress.outcome, None);

    // Back down the stair, round its east side to the arc, dropped into the chamber, and up it to
    // the field: the mission is won with the sixth objective open.
    let round = [
        [1186.5, 1229.9],
        [1189.4, 1226.7],
        [1196.0, 1221.0],
        [1204.0, 1236.0],
        [1204.0, 1292.0],
        [1184.0, 1293.5],
    ];
    let mut arc = round.to_vec();
    arc.extend(TELEPORT_ARC_ROUTE);
    let (_, _, _, dropped) = walk_route(&mut play, &arc, 90, |p| p.hero.walker.body.position.z < 90.0);
    assert!(dropped.is_some(), "the arc sends the hero down, at {}", play.hero.walker.body.position);
    play_for(&mut play, 1.5, |_| {});
    let (_, _, _, won) = walk_route(&mut play, &TELEPORT_CHAMBER_ROUTE, 30, |p| {
        p.progression.as_ref().unwrap().progress.outcome.is_some()
    });
    assert!(won.is_some(), "up the chamber, at {}", play.hero.walker.body.position);
    play.hero.key("SCAN_W", false);
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.outcome, Some(true));
    assert_eq!(objectives(&play), [1, 1, 1, 1, 1, 0]);
    eprintln!("won {:.1} s in", seconds(&play));
}
