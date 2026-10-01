//! The units the player does not walk: wingmen and their orders, captures, boarding,
//! refits, telepresence and command mode.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn a_neutral_warbot_makes_itself_the_target_and_enter_captures_both_for_the_second_objective() {
    use parkan_world::progress::{STRING_VACANT_VEHICLE, Say, Sender};

    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    let target_of = |path: &str| {
        let object = m.objects.iter().position(|o| o.path.to_ascii_lowercase().ends_with(path)).unwrap();
        play.battle.objects.iter().position(|&o| o == object).unwrap()
    };
    let (mf1, helic) = (target_of("tut1_mf1.dat"), target_of("helic.dat"));
    let vacant = play.progression.as_ref().unwrap().strings[&STRING_VACANT_VEHICLE].clone();

    for (bot, name) in [(mf1, "tut1_mf1"), (helic, "helic")] {
        stand_facing(&mut play, bot, 12.0, 3.5);
        play.tick(tick, [0.0; 2]);
        assert!(play.units[bot].announced, "{name} announced itself");
        // Both stand within the hero's sensor range: Tab steps the list to this one.
        for _ in 0..play.targets.listed.len() {
            if play.targets.current == Some(bot) {
                break;
            }
            play.targets.select_next();
        }
        assert_eq!(play.targets.current, Some(bot));
        play.says.retain(|s| !matches!(s, Say::Voice(_)));
        assert!(play.enter(), "Enter captures {name}");
        assert_eq!(play.units[bot].clan, Some(play.player_clan));
        // docs/27: a unit not boarded answers, tut1_mf1 (class 3) plainly and helic (1) in _S.
        let voices: Vec<String> = play
            .says
            .iter()
            .filter_map(|s| match s {
                Say::Voice(v) => Some(v.member.to_ascii_lowercase()),
                _ => None,
            })
            .collect();
        let suffix = if bot == helic { "_s.wav" } else { ".wav" };
        assert!(
            voices.len() == 1
                && voices[0].starts_with("vr_")
                && voices[0].ends_with(suffix)
                && (bot == helic || !voices[0].ends_with("_s.wav")),
            "{name} acknowledges: {voices:?}"
        );
    }
    assert!(play.says.contains(&Say::Text(Sender::System, vacant)));
    for _ in 0..130 {
        play.tick(tick, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.robots(3, 0x0100_0000), 0);
    assert_eq!(p.progress.robots(0, 0x0100_0000), 3);
    assert_eq!(p.progress.objectives[1].state, 1, "Ntrl has none and Plr has at least two");
    assert!(p.progress.played[&13] && p.progress.played[&19]);
}

#[test]
#[ignore = "needs the game install"]
fn captured_warbots_answer_the_wingman_menu_with_the_order_its_row_gives() {
    use parkan_sim::orders::{FOLLOW, STAYGROUND, State, Target};
    use parkan_world::play::View;

    // A radar takt after the capture both are the player's, and on its radar: wingmen.
    let (mut play, [mf1, helic, _]) = mission_01_wingmen();
    assert_eq!(play.wingmen().len(), 2);
    let eye = play.hero.eye();
    let view = |shift| View { eye: eye.position, look: eye.forward, view_proj: glam::Mat4::IDENTITY, shift };

    // The tilde chooses both and opens the menu; 2 is Follow me.
    play.command("CMD_JAMES_WINGMAN_MENU", &view(false));
    let panel = play.panel().expect("the panel is open");
    assert_eq!(panel.wingmen.len(), 2);
    let strings = &play.progression.as_ref().unwrap().strings;
    assert_eq!(
        panel.rows.iter().map(|r| strings[&r.0].as_str()).collect::<Vec<_>>()[..2],
        ["Standby", "Follow me"]
    );
    play.says.clear();
    assert!(play.wingman_digit(2));
    assert_eq!(play.selector.state, State::Off);
    for (_, robot) in play.robots.iter().filter(|(t, _)| [mf1, helic].contains(t)) {
        let order = robot.order.expect("an order");
        assert_eq!((order.code, order.parameter, order.target), (FOLLOW, 50, Target::LogicId(play.hero_id)));
    }
    assert!(!play.says.is_empty(), "the last one acknowledges");

    // Shift picks: only the second wingman, then Standby. The choice is by place in the
    // wingman list, which the radar's magenta triangle turns back into a unit.
    play.command("CMD_JAMES_WINGMAN_MENU", &view(true));
    assert!(play.chosen_wingmen().is_empty(), "Shift empties the choice");
    assert!(play.wingman_digit(2));
    let second = play.robots[play.wingmen()[1]].0;
    assert_eq!(play.chosen_wingmen(), vec![second]);
    play.command("CMD_JAMES_WINGMAN_MENU", &view(false));
    assert!(play.wingman_digit(1));
    let orders: Vec<i32> = play.robots.iter().filter_map(|(_, r)| r.order.map(|o| o.code)).collect();
    assert_eq!(orders.iter().filter(|&&c| c == STAYGROUND).count(), 1);
    assert_eq!(orders.iter().filter(|&&c| c == FOLLOW).count(), 1);
}

#[test]
#[ignore = "needs the game install"]
fn the_wingman_menu_is_the_driven_bots_and_follow_me_names_it_not_the_hero_left_behind() {
    use glam::Vec3;
    use parkan_sim::behaviour::Task;
    use parkan_sim::orders::{FOLLOW, Target};
    use parkan_world::factory::Project;
    use parkan_world::play::{Mode, View};

    let tick = 1000.0 / 60.0;
    let (mut play, [mf1, helic, _]) = mission_01_wingmen();
    // A large warbot beside the hero for it to board (docs/39, "Boarding").
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
    let bot =
        play.spawn(&project, play.player_clan, hero_at + Vec3::new(8.0, 0.0, 1.0), 0.0).expect("the L-2f");
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(play.board(bot));
    assert_eq!(play.mode(), Mode::Driving(bot));
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }

    // The list is the driven bot's, which is never its own wingman (docs/31, "Who can be a
    // wingman"), and the menu opens from its cockpit.
    let listed: Vec<usize> = play.wingmen().iter().map(|&r| play.robots[r].0).collect();
    assert_eq!(listed.len(), 2, "the two captured warbots: {listed:?}");
    assert!(!listed.contains(&bot), "the bot the player drives is not listed");
    let eye = play.own_eye();
    let view = View { eye: eye.position, look: eye.forward, view_proj: glam::Mat4::IDENTITY, shift: false };
    play.command("CMD_JAMES_WINGMAN_MENU", &view);
    assert_eq!(play.panel().expect("the panel is open").wingmen.len(), 2);
    play.says.clear();
    assert!(play.wingman_digit(2));
    assert!(!play.says.is_empty(), "the last one acknowledges");

    // Follow me is by the driven unit's logic id, not the hero's (docs/31, "The orders").
    let id = play.units[bot].logical_id;
    assert_ne!(id, play.hero_id);
    for (_, robot) in play.robots.iter().filter(|(t, _)| [mf1, helic].contains(t)) {
        let order = robot.order.expect("an order");
        assert_eq!((order.code, order.target), (FOLLOW, Target::LogicId(id)));
    }

    // So they keep up with the bot the player drives while the hero stays out of the world.
    let r = play.robots.iter().position(|(t, _)| *t == bot).expect("the driven bot");
    let start = play.robots[r].1.walker.body.position;
    put(&mut play.robots[r].1.walker, &play.ground, start + Vec3::new(60.0, -120.0, 40.0));
    for _ in 0..(60 * 15) {
        play.tick(tick, [0.0; 2]);
    }
    let at = play.robots[r].1.walker.body.position;
    assert!(at.truncate().distance(hero_at.truncate()) > 100.0, "the hero is well behind");
    for b in [mf1, helic] {
        let (_, robot) = play.robots.iter().find(|(t, _)| *t == b).unwrap();
        let off = robot.walker.body.position.truncate().distance(at.truncate());
        assert!(off < 45.0, "a follower keeps near the driven bot: {off}");
        assert!(matches!(robot.behaviour.task(), Task::Follow { .. }));
    }
}

#[test]
#[ignore = "needs the game install"]
fn the_wingman_panel_draws_a_line_per_wingman_by_name_and_the_menus_rows_under_the_tilde() {
    use glam::Mat4;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::play::View;
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, [mf1, helic, _]) = mission_01_wingmen();
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let space = Space::new(640.0, 480.0);
    let texts = |cockpit: &mut Cockpit, play: &parkan_world::play::Play| {
        let drawn = cockpit.draw(play, space, &font, &menu, Mat4::IDENTITY);
        drawn.text.iter().map(|r| (r.text.clone(), r.colour)).collect::<Vec<_>>()
    };

    // With the selector off the lines show, named as the panels name them, never by path.
    let panel = play.panel().expect("two wingmen");
    assert_eq!(panel.wingmen.iter().map(|w| (w.0, w.2)).collect::<Vec<_>>(), [(1, false), (2, false)]);
    assert!(panel.rows.is_empty());
    let off = texts(&mut cockpit, &play);
    let names: Vec<String> = panel.wingmen.iter().map(|w| cockpit.panels.names[w.1].clone()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(sorted, ["MFW-1 Warrior", "TFW-2 Warrior"]);
    let grey = [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0];
    for name in &names {
        // The panel's line is drawn after the target panel, which may name the same unit.
        let run = off.iter().rev().find(|(t, _)| t == name).expect("the name drawn");
        assert!(run.1.iter().zip(grey).all(|(a, b)| (a - b).abs() < 1e-3), "{name} grey: {:?}", run.1);
    }
    assert!(!off.iter().any(|(t, _)| t.contains("tut1") || t.contains("helic")), "{off:?}");
    assert!([mf1, helic].iter().all(|t| panel.wingmen.iter().any(|w| w.1 == *t)));

    // The tilde: both chosen, white, and the seven rows by the game's strings.
    let eye = play.hero.eye();
    let view = View { eye: eye.position, look: eye.forward, view_proj: Mat4::IDENTITY, shift: false };
    play.command("CMD_JAMES_WINGMAN_MENU", &view);
    let open = texts(&mut cockpit, &play);
    for name in &names {
        let run = open.iter().rev().find(|(t, _)| t == name).unwrap();
        assert_eq!(run.1, [1.0; 4], "{name} chosen, white");
    }
    for row in [
        "Standby",
        "Follow me",
        "Search and capture",
        "Seek and destroy",
        "Attack",
        "Capture building",
        "Refit",
    ] {
        assert!(open.iter().any(|(t, _)| t == row), "{row} in {open:?}");
    }
    // A warbot of size 3 among the chosen: Search and capture is disabled, grey.
    let search = open.iter().find(|(t, _)| t == "Search and capture").unwrap();
    assert!(search.1.iter().zip(grey).all(|(a, b)| (a - b).abs() < 1e-3));
}

#[test]
#[ignore = "needs the game install"]
fn wingmen_follow_seek_and_destroy_stand_by_and_fail_a_refit_with_no_dock() {
    use glam::Vec3;
    use parkan_sim::behaviour::Task;
    use parkan_world::play::{Play, View};

    let tick = 1000.0 / 60.0;
    let order = |play: &mut Play, row: usize| {
        let eye = play.hero.eye();
        let view =
            View { eye: eye.position, look: eye.forward, view_proj: glam::Mat4::IDENTITY, shift: false };
        play.command("CMD_JAMES_WINGMAN_MENU", &view);
        assert!(play.wingman_digit(row));
    };
    let run = |play: &mut Play, seconds: usize| {
        for _ in 0..60 * seconds {
            play.tick(tick, [0.0; 2]);
        }
    };
    let at =
        |play: &Play, t: usize| play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    let task =
        |play: &Play, t: usize| play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.behaviour.task();

    // Follow me (docs/31): the hero leaves them 130 m behind; they close to within the
    // radius and its slack, 20 + 20, and hold there.
    let (mut play, [mf1, helic, e1]) = mission_01_wingmen();
    assert!(play.robots.iter().all(|(t, r)| ![mf1, helic].contains(t) || (r.flyer && !r.guns.is_empty())));
    assert_eq!(play.hero.guns.len(), 4, "the hero's own guns only");
    order(&mut play, 2);
    let start = play.hero.walker.body.position;
    put(&mut play.hero.robot.walker, &play.ground, start + Vec3::new(60.0, -120.0, 40.0));
    run(&mut play, 15);
    let hero = play.hero.walker.body.position;
    for bot in [mf1, helic] {
        let off = at(&play, bot).truncate().distance(hero.truncate());
        assert!(off < 45.0, "a follower keeps near: {off}");
        assert!(matches!(task(&play, bot), Task::Follow { .. }));
    }

    // Standby: they hold where they are.
    order(&mut play, 1);
    run(&mut play, 1);
    let held = [at(&play, mf1), at(&play, helic)];
    put(&mut play.hero.robot.walker, &play.ground, start);
    run(&mut play, 5);
    assert!([mf1, helic].iter().zip(held).all(|(&b, p)| at(&play, b).distance(p) < 2.0), "standby holds");

    // Refit: Mission 01 has no dock, so the task fails at its start and they stop.
    order(&mut play, 7);
    run(&mut play, 1);
    assert!([mf1, helic].iter().all(|&b| !matches!(task(&play, b), Task::Reload { .. })));

    // Seek and destroy: the one hostile warrior on the map, `tut1_e1`, is hunted and shot.
    order(&mut play, 4);
    let mut killed = None;
    for s in 0..60 {
        run(&mut play, 1);
        if !play.battle.combat.targets[e1].alive {
            killed = Some(s);
            break;
        }
    }
    assert!(killed.is_some(), "the wingmen destroy tut1_e1");
    run(&mut play, 6);
    assert!(play.killed.contains(&play.battle.objects[e1]) && play.deleted[e1]);
}

/// A flying wingman told to follow flies to a spot at its leader's height and 5, and holds it
/// there: follow's `SetTarget` (`Behavior.dll:0x1002b059`) builds its place with the word `+0x14`
/// at 1 (`0x1002affc`), so the walker leaves the points on the straight line to it, where every
/// other task's place is raised 15 over the ground (docs/24, "A flyer's walk points").
#[test]
#[ignore = "needs the game install"]
fn a_flying_wingman_follows_at_its_leaders_height_and_five_not_fifteen_over_the_ground() {
    use glam::Vec3;
    use parkan_sim::behaviour::FOLLOW_LIFT;
    use parkan_world::play::{Play, View};

    let (mut play, [mf1, helic, _]) = mission_01_wingmen();
    let eye = play.hero.eye();
    let view = View { eye: eye.position, look: eye.forward, view_proj: glam::Mat4::IDENTITY, shift: false };
    play.command("CMD_JAMES_WINGMAN_MENU", &view);
    assert!(play.wingman_digit(2), "Follow me");
    // The hero goes 127 m north-west, onto open ground away from the bridges, whose decks a
    // spot 5 over a hero on the shore below would lie under.
    let start = play.hero.walker.body.position;
    put(&mut play.hero.robot.walker, &play.ground, start + Vec3::new(-90.0, 90.0, 40.0));
    play_for(&mut play, 20.0, |_| {});
    let at =
        |play: &Play, t: usize| play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    let hero = play.hero.walker.body.position;
    for bot in [mf1, helic] {
        let p = at(&play, bot);
        let over = play.flight_ground(p.x, p.y) + 15.0;
        assert!(p.truncate().distance(hero.truncate()) < 45.0, "it keeps near: {p}");
        assert!(
            (p.z - (hero.z + FOLLOW_LIFT)).abs() < 2.0,
            "at the leader's height and 5: {p} against {hero}"
        );
        assert!((p.z - over).abs() > 4.0, "not 15 over the ground, {over}: {p}");
    }
}

/// Mission 02's warbot, built from the factory's screen, leaves by the front door and its
/// escape takes it back up over the factory: its point inside the hall's footprint stands 115 m
/// over the roof (docs/24, "A flyer's walk points"). Follow me given there, from the forecourt,
/// brings it down to the hero: over the roof is not the inside of the building the bot was made
/// in, and the way out through the hall is not its way.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_built_warbot_follows_the_hero_off_the_factory_roof_its_escape_took_it_onto() {
    use parkan_sim::behaviour::Task;
    use parkan_world::factory::Project;
    use parkan_world::play::{Mode, Play, View};

    let tick = |play: &mut Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    let (mut play, _) = mission_02_play();
    let factory = play.buildings.iter().find(|b| b.doors.len() == 3).unwrap().target;
    assert!(play.stand_on_pod(factory));
    tick(&mut play, 6.0);
    assert_eq!(play.mode(), Mode::Factory(factory));
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
    play.factory_click(factory, parkan_world::cockpit::factory::Click::Build);
    play.factory_click(factory, parkan_world::cockpit::factory::Click::Exit);
    // Out on the forecourt, watching the front door.
    assert!(play.stand_at(395.8, 940.0, -std::f32::consts::FRAC_PI_2));
    let robots = play.robots.len();
    while play.robots.len() == robots {
        tick(&mut play, 1.0 / 60.0);
    }
    let r = play.robots.len() - 1;

    // Out of the front door, the escape turns back for its point over the hall and climbs over
    // the roof: the factory's faces 15 m and more over the hall floor's 154.05 under it, with no
    // floor of the factory above them.
    let on_roof = |play: &Play| {
        play.robots[r]
            .1
            .walker
            .ground
            .is_some_and(|h| h.solid.map(|s| s.0) == Some(factory) && h.point.z - 154.05 > 15.0)
    };
    let mut seconds = 0.0;
    while !on_roof(&play) && seconds < 30.0 {
        tick(&mut play, 1.0 / 60.0);
        seconds += 1.0 / 60.0;
    }
    let at = play.robots[r].1.walker.body.position;
    assert!(on_roof(&play), "the escape takes the bot over the roof: at {at} after {seconds} s");
    assert!(matches!(play.robots[r].1.behaviour.task(), Task::Leave { .. }), "still escaping");

    let eye = play.hero.eye();
    let view = View { eye: eye.position, look: eye.forward, view_proj: glam::Mat4::IDENTITY, shift: false };
    play.command("CMD_JAMES_WINGMAN_MENU", &view);
    assert!(play.wingman_digit(2));
    assert!(matches!(play.robots[r].1.behaviour.task(), Task::Follow { .. }));
    // It comes down off the roof to within the radius and its slack, 20 + 20, of the hero.
    tick(&mut play, 20.0);
    let hero = play.hero.walker.body.position;
    let (at, on) =
        (play.robots[r].1.walker.body.position, play.robots[r].1.walker.ground.and_then(|h| h.solid));
    let off = at.truncate().distance(hero.truncate());
    assert!(
        off < 40.0 && on.is_none_or(|s| s.0 != factory),
        "I wasn't able to make him follow me: the bot holds {off} m off at {at}, on {on:?}"
    );
}

#[test]
#[ignore = "needs the game install"]
fn a_captured_bot_stands_by_until_ordered_and_with_no_order_engages_as_the_games_does() {
    use parkan_sim::behaviour::Task;
    use parkan_world::play::Play;

    let tick = 1000.0 / 60.0;
    let tasks = |play: &Play, bots: &[usize]| -> Vec<Task> {
        bots.iter().map(|&b| play.robots.iter().find(|(t, _)| *t == b).unwrap().1.behaviour.task()).collect()
    };
    // `tut1_e1` stands about 470 from the two bots: within the engagement's 500.
    let (mut play, [mf1, helic, _]) = mission_01_captured(true);
    let at: Vec<_> = [mf1, helic].iter().map(|&b| play.battle.combat.targets[b].position).collect();
    for _ in 0..600 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(tasks(&play, &[mf1, helic]).iter().all(|t| *t == Task::StayGround));
    for (&b, p) in [mf1, helic].iter().zip(at) {
        assert!(play.battle.combat.targets[b].position.distance(p) < 1.0, "standby holds");
    }

    // With no order the base priority takes an engagement up, but only against a contact on
    // the unit's own radar list (docs/25, "What the AI does with it"). `tut1_e1` stands 474
    // from `tut1_mf1`, whose radar reaches 350, and 457 from `helic`, whose reaches 250:
    // inside the engagement's 500 and beyond both radars, so neither takes it up.
    let (mut play, [mf1, helic, _]) = mission_01_captured(false);
    for _ in 0..120 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(
        tasks(&play, &[mf1, helic]).iter().all(|t| !matches!(t, Task::Attack { .. })),
        "beyond both radars: nothing to engage"
    );

    // Brought inside them, both engage. The scan holds 750 ms, so it takes a second to show.
    let (mut play, [mf1, helic, e1]) = mission_01_captured(false);
    let near = play.battle.combat.targets[mf1].position + glam::Vec3::new(150.0, 0.0, 0.0);
    let r = play.robots.iter().position(|(t, _)| *t == e1).expect("tut1_e1 is a robot");
    let w = &mut play.robots[r].1.walker;
    w.body.position = near;
    w.follow_ground(&play.ground);
    w.from = (w.body.position, w.body.yaw);
    for _ in 0..120 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(
        tasks(&play, &[mf1, helic]).iter().any(|t| matches!(t, Task::Attack { .. })),
        "no order, the enemy on the radar: it engages"
    );
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_hero_boards_its_warbot_flies_it_and_gets_out_where_it_may_land() {
    use parkan_world::factory::Project;
    use parkan_world::play::Mode;
    use parkan_world::progress::Say;

    let (mut play, _) = mission_02_play();
    let path = "UNITS\\bld_unit_-2147483647.dat";
    let project = Project {
        path: path.to_owned(),
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
    play.says.clear();
    assert!(play.boardable(t), "the player's own large bot within 20");
    assert!(play.board(t));
    assert_eq!(play.mode(), Mode::Driving(t));
    // A flyer taken over asks for the mission's message 100, T02_H06 (docs/34, "Mission 02").
    assert!(
        play.says.iter().any(|s| matches!(s, Say::Text(_, text) if text.contains("altitude"))),
        "{:?}",
        play.says
    );
    let frozen = play.hero.walker.body.position;
    let eye = play.eye();
    let bot_at = play.driven().walker.body.position;
    assert!(eye.position.distance(bot_at) < 10.0, "the eye is the bot's");
    // R climbs.
    let z0 = play.driven().walker.body.position.z;
    play.key("SCAN_R", true);
    for _ in 0..(3 * 60) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    play.key("SCAN_R", false);
    let z1 = play.driven().walker.body.position.z;
    assert!(z1 - z0 > 15.0, "climbed from {z0} to {z1}");
    assert_eq!(play.hero.walker.body.position, frozen, "the hero is out of the world");
    // W flies forward.
    let a = play.driven().walker.body.position;
    play.key("SCAN_W", true);
    for _ in 0..(2 * 60) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    play.key("SCAN_W", false);
    let b = play.driven().walker.body.position;
    assert!(a.truncate().distance(b.truncate()) > 20.0, "flew from {a} to {b}");
    // High up, getting out is refused: "Risk area! Landing impossible."
    play.says.clear();
    assert!(!play.roll_back());
    assert_eq!(play.mode(), Mode::Driving(t));
    assert!(
        play.says.iter().any(|s| matches!(s, Say::Text(_, text) if text.starts_with("Risk area"))),
        "{:?}",
        play.says
    );
    // F sinks to the ground; then the hero gets out beside the bot.
    play.key("SCAN_F", true);
    for _ in 0..(6 * 60) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    play.key("SCAN_F", false);
    let bot = play.driven().walker.body.position;
    let reach = play.driven().bound.1 + play.hero.bound.1;
    assert!(play.roll_back(), "low over land it may land: bot at {bot}");
    assert_eq!(play.mode(), Mode::OnFoot);
    // The first of the eight places, both node spheres' radii out, whose landscape lies less than
    // 10 below the bot (docs/39, "Leaving"). The L-2f rests with its origin 9.67 over flat ground
    // (docs/24, "Finding the ground"); here the places due +x and the next, over lower ground,
    // are refused, and the hero comes out at the third, north.
    let out = play.hero.walker.body.position;
    let gaps: Vec<f32> = (0..8)
        .map(|i| {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            let p = bot + glam::Vec3::new(a.cos(), a.sin(), 0.0) * reach;
            bot.z - play.ground.below(p.x, p.y, 10_000.0).unwrap().point.z
        })
        .collect();
    let first = gaps.iter().position(|&g| g < 10.0).unwrap();
    assert_eq!(first, 2, "{gaps:?}");
    let a = first as f32 * std::f32::consts::FRAC_PI_4;
    let place = bot.truncate() + glam::Vec2::new(a.cos(), a.sin()) * reach;
    assert!(out.truncate().distance(place) < 0.5, "out at {out}, the bot at {bot}, {reach}: {gaps:?}");
    // Put out due north of the bot, the hero faces (F.x, −F.y) for F = (0, −1): north, away from
    // it (docs/39, "Leaving").
    let forward = play.hero.walker.body.forward();
    assert!(forward.distance(glam::Vec3::Y) < 1e-3, "the hero faces {forward}");
}

/// The Mission 02 warbot spawned beside the hero, as the boarding test makes it.
fn mission_02_warbot() -> (parkan_world::play::Play, usize) {
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
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
    (play, t)
}

/// The boarding test reads the life of the node the bot's first class-1 component names
/// (`iron3d.dll:0x10076d30`, docs/39, "Boarding"): the L-2f's `e_tur_bb_01` names its node 1,
/// its body `BTmn`, where its node 0 is the chassis's socket. Shot off, the bot cannot be
/// boarded.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_warbot_cannot_be_boarded_once_its_turrets_body_is_shot_off() {
    let (mut play, t) = mission_02_warbot();
    let robot = &play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1;
    let (part, node) = robot.turret_life().expect("a turret component");
    assert_eq!((part, node), (robot.turret_part, 1));
    assert_eq!(robot.parts[part].record.to_ascii_lowercase(), "e_tur_bb_01");
    assert!(play.boardable(t));
    let life = play.battle.combat.targets[t].parts[part].life.as_mut().unwrap();
    assert!(life.nodes[1].max > life.nodes[0].max, "the body, not the socket's 1 hit point");
    life.hit(1, f32::MAX / 4.0);
    assert!(!play.boardable(t), "its turret's body is gone");
}

/// A bot's gun is named by the research code of the part it belongs to in the player clan's
/// tree (`0x1008a470`, `0x1008a4b0`, docs/35, "The weapons list"): the design's rocket
/// launchers `e_gun_bl_15` read LRL36S and its flamers `e_gun_bc_06` LFT, the codes the
/// recording's weapons list shows from 249.4 s. Each carries a clip, which keeps the gun's part.
#[test]
#[ignore = "needs the game install"]
fn mission_02s_warbot_names_its_guns_by_their_research_codes() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (play, t) = mission_02_warbot();
    let codes = parkan_world::cockpit::gun_codes(&game, &play);
    let robot = &play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1;
    let names: Vec<&str> = (0..robot.guns.len())
        .map(|i| {
            let record = robot.parts[robot.gun_part(i)].record.to_ascii_lowercase();
            codes.get(&record).map_or("NONAME", String::as_str)
        })
        .collect();
    assert_eq!(names, ["LRL36S", "LRL36S", "LFT", "LFT"]);
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_warbot_lets_the_hero_out_on_the_outpost_islands_flat_ground() {
    use parkan_world::factory::Project;
    use parkan_world::play::Mode;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
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
    tick(&mut play, 0.5);
    // Every place 40 and 55 m from the Outpost, a sixteenth of a turn apart, on the island's flat
    // ground (151-152 over Tut_2's water at 150): the user found "Risk area!" there when the bot
    // rested on the parts' sphere, its origin 10.25 over the ground.
    let outpost = play.buildings.iter().find(|b| b.doors.is_empty()).unwrap().target;
    let centre = play.battle.combat.targets[outpost].position;
    let mut spots = Vec::new();
    for radius in [40.0f32, 55.0] {
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let (x, y) = (centre.x + radius * a.cos(), centre.y + radius * a.sin());
            if play.ground.below(x, y, 1_000.0).is_some_and(|h| (151.0..152.0).contains(&h.point.z)) {
                spots.push(glam::Vec3::new(x, y, 170.0));
            }
        }
    }
    assert!(spots.len() >= 12, "{spots:?}");
    let mut refused = Vec::new();
    for spot in &spots {
        play.hero.walker.body.position =
            play.robots.iter().find(|r| r.0 == t).unwrap().1.walker.body.position
                + glam::Vec3::new(6.0, 0.0, 0.0);
        assert!(play.board(t));
        play.robots.iter_mut().find(|r| r.0 == t).unwrap().1.walker.body.position = *spot;
        play.key("SCAN_F", true);
        tick(&mut play, 5.0);
        play.key("SCAN_F", false);
        let landed = play.driven().walker.body.position;
        // Resting on the sphere about the node sphere's centre, the origin 9.67 over flat ground:
        // under 10 over the ground at every place about it.
        let ground = play.ground.below(landed.x, landed.y, 1_000.0).unwrap().point.z;
        assert!((9.3..9.9).contains(&(landed.z - ground)), "rests at {landed} over {ground}");
        if !play.roll_back() {
            refused.push(landed);
            continue;
        }
        assert_eq!(play.mode(), Mode::OnFoot);
    }
    assert!(refused.is_empty(), "Risk area at {refused:?} of {} places", spots.len());
}

#[test]
#[ignore = "needs the game install"]
fn a_refit_walks_mission_02s_warbot_to_the_outposts_dock_and_one_under_half_its_life_goes_by_itself() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::damage::{Life, share_loss};
    use parkan_sim::orders::{Order, Target};
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    // The Outpost taken, so its one dock — ground level, the only kind a large unit fits —
    // charges the player's units (docs/27, "What sends a bot to a dock").
    tick(&mut play, 0.5);
    let outpost = play.buildings.iter().find(|b| b.doors.is_empty()).unwrap().target;
    assert!(play.stand_on_pod(outpost));
    tick(&mut play, 6.0);
    assert_eq!(play.units[outpost].clan, Some(play.player_clan));
    let set = play.places.iter().find(|p| p.target == outpost).expect("the Outpost's places");
    let vertex = set.places[0].vertex;
    let part = &play.battle.combat.targets[outpost].parts[set.part];
    let dock = parkan_world::factory::vertex_world(&vertex, part).expect("the dock's world point");

    // A large flyer of the player's clan on the island's flat ground, 55 m off the Outpost.
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
    let centre = play.battle.combat.targets[outpost].position;
    let spot = (0..16)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            glam::Vec3::new(centre.x + 55.0 * a.cos(), centre.y + 55.0 * a.sin(), 0.0)
        })
        .find(|p| play.ground.below(p.x, p.y, 1_000.0).is_some_and(|h| (151.0..152.0).contains(&h.point.z)))
        .expect("a place on the island");
    let t = play.spawn(&project, play.player_clan, spot.with_z(165.0), 0.0).expect("the L-2f");
    tick(&mut play, 1.0);

    // Scratched to 0.7 of its life: it needs no service, so it stays where it is, and 0.7 is
    // over the 0.5 below which its repair system goes on -- `Decision_RepairOn` as the
    // difficulty block's compiled default leaves it (docs/26, "What the AI does with the
    // switch").
    let hurt = |play: &mut parkan_world::play::Play, share: f32| {
        let mut lives: Vec<&mut Life> =
            play.battle.combat.targets[t].parts.iter_mut().filter_map(|p| p.life.as_mut()).collect();
        let (left, full): (f32, f32) =
            lives.iter().fold((0.0, 0.0), |(l, f), x| (l + x.total(), f + x.full()));
        share_loss(&mut lives, left - full * share);
    };
    let life = |play: &parkan_world::play::Play| {
        let (l, f): (f32, f32) = play.battle.combat.targets[t]
            .parts
            .iter()
            .filter_map(|p| p.life.as_ref())
            .fold((0.0, 0.0), |(l, f), x| (l + x.total(), f + x.full()));
        l / f
    };
    let bot = |play: &parkan_world::play::Play| {
        let (_, robot) = play.robots.iter().find(|(rt, _)| *rt == t).unwrap();
        (robot.behaviour.task(), robot.walker.body.position, robot.behaviour.repair)
    };
    hurt(&mut play, 0.7);
    tick(&mut play, 1.0);
    let (task, stood, repairing) = bot(&play);
    assert_eq!(task, Task::Stop, "0.7 of its life needs no service");
    assert!(!repairing, "a scratched warbot leaves its own repair system off");

    // Told to refit, it makes for the Outpost's dock and stands in it until it is full.
    let order = Order { code: parkan_sim::orders::RELOAD, parameter: 0, target: Target::NotDefined };
    play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1.behaviour.order(&order);
    tick(&mut play, 0.1);
    assert!(matches!(bot(&play).0, Task::Reload { dock: Some(_), .. }), "{:?}", bot(&play).0);
    let mut arrived = None;
    for s in 0..90 {
        tick(&mut play, 1.0);
        let (task, at, _) = bot(&play);
        if arrived.is_none() && at.distance(dock) <= 12.0 {
            arrived = Some(s);
        }
        if !matches!(task, Task::Reload { .. }) {
            break;
        }
    }
    let (task, at, _) = bot(&play);
    assert!(arrived.is_some(), "it never reached the dock: stopped {at}, {} from it", at.distance(dock));
    assert!(at.distance(stood) > 20.0, "it left where it stood");
    assert!(!matches!(task, Task::Reload { .. }), "the refit ends once it is charged: {task:?}");
    assert!(life(&play) > 0.98, "charged in the dock: {}", life(&play));

    // Charged and standing in the Outpost's dock, its own takt walks it back off the building
    // (docs/31, "The escape"), which ends on open ground.
    let mut settled = false;
    for _ in 0..60 {
        tick(&mut play, 1.0);
        settled = bot(&play).0 == Task::Stop;
        if settled {
            break;
        }
    }
    assert!(settled, "the escape ends and it stands: {:?}", bot(&play).0);

    // Under half its life it sends itself back there, with no order at all (docs/27).
    hurt(&mut play, 0.4);
    let mut sent = false;
    for _ in 0..20 {
        tick(&mut play, 1.0);
        sent = matches!(bot(&play).0, Task::Reload { .. });
        if sent {
            break;
        }
    }
    assert!(sent, "a bot under half its life refits itself: {:?}", bot(&play).0);
    for _ in 0..180 {
        tick(&mut play, 1.0);
        if !matches!(bot(&play).0, Task::Reload { .. }) {
            break;
        }
    }
    assert!(life(&play) > 0.98, "and is charged again: {}", life(&play));
}

#[test]
#[ignore = "needs the game install"]
fn a_small_warbots_refit_walks_it_into_mission_03s_bunker_to_the_dock_a_large_one_cannot_reach() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::damage::{Life, share_loss};
    use parkan_sim::orders::{Order, Target};
    use parkan_world::factory::Project;

    let (mut play, m) = mission_03_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    // The Small Bunker taken: its one place is an indoor dock, `0x620`, which only a unit of
    // size class 1 or 2 is routed to (docs/27, "The places").
    let bunker = object_target(&play, &m, "sbunk01.dat");
    assert!(play.stand_on_pod(bunker));
    tick(&mut play, 6.0);
    assert_eq!(play.units[bunker].clan, Some(play.player_clan));
    let set = play.places.iter().find(|p| p.target == bunker).expect("the bunker's places");
    assert_eq!(set.places.len(), 1);
    assert_eq!(set.places[0].vertex.flags, 0x620, "one indoor dock and no ground-level bit");
    let (dock_vertex, dock_part) = (set.places[0].vertex, set.part);
    assert!(play.roll_back(), "out of command mode");

    // A small warbot of the player's clan, hurt, outside the bunker.
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
    let at = play.battle.combat.targets[bunker].position;
    let spot = glam::Vec3::new(at.x + 60.0, at.y, at.z);
    let t = play.spawn(&project, play.player_clan, spot, 0.0).expect("the SSW-X");
    tick(&mut play, 1.0);
    let mut lives: Vec<&mut Life> =
        play.battle.combat.targets[t].parts.iter_mut().filter_map(|p| p.life.as_mut()).collect();
    let (left, full): (f32, f32) = lives.iter().fold((0.0, 0.0), |(l, f), x| (l + x.total(), f + x.full()));
    // Under half its life, so its own repair system runs on the way in, `Decision_RepairOn`
    // being 0.5 (docs/26, "What the AI does with the switch"). The walk through the bunker's
    // door is timed by the stand-in points of docs/31's hall-way walk, and at the pace a unit
    // scratched to 0.6 keeps with its repair system off -- 4.31 m/s, where a running one mends
    // it up past 4.7 -- it stops on the ramp 11 m short of the door, waits out its points and
    // turns back to the exit, again and again.
    share_loss(&mut lives, left - full * 0.45);
    let life = |play: &parkan_world::play::Play| {
        let (l, f): (f32, f32) = play.battle.combat.targets[t]
            .parts
            .iter()
            .filter_map(|p| p.life.as_ref())
            .fold((0.0, 0.0), |(l, f), x| (l + x.total(), f + x.full()));
        l / f
    };
    let task = |play: &parkan_world::play::Play| {
        play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.behaviour.task()
    };

    // Told to refit, it walks in along the bunker's hall way and charges at the dock inside.
    let order = Order { code: parkan_sim::orders::RELOAD, parameter: 0, target: Target::NotDefined };
    play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1.behaviour.order(&order);
    tick(&mut play, 0.1);
    assert!(matches!(task(&play), Task::Reload { dock: Some(_), .. }), "{:?}", task(&play));
    let part = &play.battle.combat.targets[bunker].parts[dock_part];
    let dockpt = parkan_world::factory::vertex_world(&dock_vertex, part).unwrap();
    let mut inside = None;
    for k in 0..120 {
        tick(&mut play, 1.0);
        let p = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
        if inside.is_none() && p.distance(dockpt) < 5.0 {
            inside = Some(k);
        }
        if !matches!(task(&play), Task::Reload { .. }) {
            break;
        }
    }
    let seconds = inside.expect("it walks down the ramp to the dock inside");
    assert!(life(&play) > 0.98, "charged at the bunker's indoor dock {seconds} s in: {}", life(&play));

    // A large unit fits through no door: it passes the bunker's dock by and takes a
    // ground-level one, the player's own Large Factory's.
    let large = Project { chassis_size: 4, path: "UNITS\\bld_unit_-2147483647.dat".to_owned(), ..project };
    let big =
        play.spawn(&large, play.player_clan, spot + glam::Vec3::new(0.0, 40.0, 0.0), 0.0).expect("the L-2f");
    tick(&mut play, 1.0);
    play.robots.iter_mut().find(|(rt, _)| *rt == big).unwrap().1.behaviour.order(&order);
    tick(&mut play, 0.5);
    let big_task = play.robots.iter().find(|(rt, _)| *rt == big).unwrap().1.behaviour.task();
    let Task::Reload { dock: Some((id, index)), .. } = big_task else { panic!("{big_task:?}") };
    assert_ne!(id, play.units[bunker].logical_id, "never the bunker's indoor dock");
    let docks = play.docks_for(big);
    assert!(docks.iter().any(|d| !d.ground_level), "the bunker's indoor dock is in the world");
    let picked = docks.iter().find(|d| (d.id, d.index) == (id, index)).expect("the dock it picked");
    assert!(picked.ground_level, "a large unit takes a ground-level dock");
}

#[test]
#[ignore = "needs the game install"]
fn taking_mission_03s_bunker_opens_command_mode_whose_camera_moves_about_it_and_esc_leaves() {
    use parkan_world::command::{Edges, HOLD};
    use parkan_world::play::Mode;

    let (mut play, m) = mission_03_play();
    let t = object_target(&play, &m, "sbunk01.dat");
    assert!(play.stand_on_pod(t));
    for _ in 0..(60 * 6) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert_eq!(play.mode(), Mode::Command(t), "the capture's own firing opens command mode");
    assert_eq!(play.selected, vec![t]);
    let at = play.battle.combat.targets[t].position;
    // Placed over the bunker facing north, 32.7° down, and held 36 to 236 over its roof.
    let mut now = 0.0;
    play.command_frame(now, Edges::default());
    let eye = play.eye();
    assert!((eye.position.truncate() - at.truncate()).length() < 1e-3, "{:?}", eye.position);
    assert!(eye.forward.x.abs() < 1e-5 && eye.forward.y > 0.0, "{:?}", eye.forward);
    let roof = play.ground.below(at.x, at.y, 1.0e5).unwrap().point.z;
    assert!(
        eye.position.z >= roof + 36.0 - 1e-3 && eye.position.z <= roof + 236.0,
        "{} over {roof}",
        eye.position.z
    );
    // The keypad's up arrow runs it north; the box stops it 200 out.
    assert!(play.command_key("CMD_JAMES_HQ_MOVE_FORWARD", true));
    for _ in 0..(60 * 4) {
        now += 1.0 / 60.0;
        play.command_frame(now, Edges::default());
    }
    assert!((play.eye().position.y - (at.y + HOLD)).abs() < 1e-3, "{:?}", play.eye().position);
    // The world goes on meanwhile, and Esc takes the hero back where it stands.
    let hero = play.hero.walker.body.position;
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::OnFoot);
    assert!(play.selected.is_empty());
    assert!((play.hero.walker.body.position - hero).length() < 1e-3);
}

#[test]
#[ignore = "needs the game install"]
fn telepresence_takes_a_warbot_from_command_mode_and_esc_returns_to_the_camera_where_it_was() {
    use parkan_world::play::Mode;

    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    // A prebuilt warbot beside the bunker, the player's.
    let project = play.factories[0].projects[0].clone();
    let at = play.battle.combat.targets[bunker].position + glam::Vec3::new(40.0, 0.0, 0.0);
    let bot = play.spawn(&project, play.player_clan, at, 0.0).expect("the prebuilt design is a robot");
    play.tick(1000.0 / 60.0, [0.0; 2]);
    play.command.position.x += 50.0;
    let camera = play.command.position;
    assert!(play.can_take(bot), "any size can be taken over");
    assert!(play.telepresence(bot, 0));
    assert_eq!(play.mode(), Mode::Driving(bot));
    assert!(play.driving.as_ref().is_some_and(|d| d.telepresence));
    let hero = play.hero.walker.body.position;
    for _ in 0..60 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::Command(bunker));
    assert_eq!(play.command.position, camera, "the camera is where the player left it");
    assert!((play.hero.walker.body.position - hero).length() < 1e-3, "the hero stayed in the bunker");
    assert!(play.selected_units().is_empty());
}

/// *Explode!* in the battle units' box (docs/41, "The box"): the press is stamped, a second
/// press while it is pending does nothing, and past 0.6 s the unit's takt kills it through its
/// life system: node 0 loses the whole, and its stage rises to play its explosion.
#[test]
#[ignore = "needs the game install"]
fn explode_in_the_unit_box_blows_the_warbot_up_six_tenths_of_a_second_later() {
    use parkan_sim::combat::Event;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::commander::{Click, EXPLODE_BUTTON};
    use parkan_world::hud::Pages;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    let project = play.factories[0].projects[0].clone();
    let at = play.battle.combat.targets[bunker].position + glam::Vec3::new(40.0, 0.0, 0.0);
    let bot = play.spawn(&project, play.player_clan, at, 0.0).expect("the prebuilt design is a robot");
    play.tick(1000.0 / 60.0, [0.0; 2]);
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let now = play.hero.time_ms;
    cockpit.update(&mut play, now);
    cockpit.commander.turn(&mut play, 1, now);
    play.select_unit_alone(bot);
    assert!(play.battle.explosions[bot][0][0].is_some(), "node 0 names an explosion");

    let [x, y, ..] = EXPLODE_BUTTON;
    let button = [x + 17.0, y + 10.0];
    let mut click = |play: &mut parkan_world::play::Play| {
        let now = play.hero.time_ms;
        cockpit.commander.click(play, &mut cockpit.map, button, button, now)
    };
    assert_eq!(click(&mut play), Click::Taken);
    assert!(play.explode_pending(bot));
    let pressed = play.exploding.clone();
    play.tick(1000.0 / 60.0, [0.0; 2]);
    assert_eq!(click(&mut play), Click::Taken);
    assert_eq!(play.exploding, pressed, "pending: the second press is not stamped");

    let mut events = Vec::new();
    while play.battle.combat.targets[bot].alive {
        assert!(play.hero.time_ms - pressed[0].1 < 700.0, "still alive 0.7 s on");
        events.extend(play.tick(1000.0 / 60.0, [0.0; 2]));
    }
    let waited = play.hero.time_ms - pressed[0].1;
    assert!(waited > 600.0, "killed {waited} ms after the press");
    assert!(!play.explode_pending(bot));
    assert!(events.contains(&Event::Killed { target: bot }));
    assert!(events.contains(&Event::Staged { target: bot, part: 0, node: 0 }), "{events:?}");
}

/// The *Manual* button on a bunker's row of the commander panel's bunker page (`0x10086256`,
/// docs/41, "The building pages") takes the bunker's turret: mode 6 goes on top of the command
/// view it was pressed in, the player sees from the turret's camera, turns it with the mouse and
/// fires its guns with the button, and Esc gives the guns back to its AI and the view back to
/// command mode. With its turret shot off the button is inert (`0x10033e40`).
#[test]
#[ignore = "needs the game install"]
fn the_manual_button_on_a_bunkers_row_takes_its_turret_and_esc_returns_to_command_mode() {
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::commander::{
        BUILDING_BUTTON, BUILDING_BUTTONS_X, BUILDING_ROWS_TOP, Click, ROW_HEIGHT,
    };
    use parkan_world::hud::Pages;
    use parkan_world::play::{Mode, Play};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    let tick = |play: &mut Play, mouse: [f32; 2]| {
        play.update_input();
        play.tick(1000.0 / 60.0, mouse);
    };
    tick(&mut play, [0.0; 2]);
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let now = play.hero.time_ms;
    cockpit.update(&mut play, now);
    cockpit.commander.turn(&mut play, 7, now);
    let command_eye = play.eye().position;

    // Row 0's second button: Strategic control is the first on a bunker's row.
    let manual = [BUILDING_BUTTONS_X + BUILDING_BUTTON + 17.0, BUILDING_ROWS_TOP + ROW_HEIGHT / 2.0];
    let now = play.hero.time_ms;
    assert_eq!(cockpit.commander.click(&mut play, &mut cockpit.map, manual, manual, now), Click::Taken);
    assert_eq!(play.mode(), Mode::Manual(bunker), "the bunker's manual control");
    assert_eq!(play.driven_target(), Some(bunker));
    let eye = play.eye().position;
    let at = play.battle.combat.targets[bunker].position;
    assert!(eye.distance(at) < 30.0, "the turret's camera, at the bunker: {eye} by {at}");
    assert!(eye.distance(command_eye) > 5.0, "not the command camera: {eye} and {command_eye}");

    let aim = |play: &Play| play.emplacements.iter().find(|(e, _)| *e == bunker).unwrap().1.rig.aim;
    // The yaw channel wraps at a whole turn, so the turn is summed tick by tick.
    let mut turned = 0.0_f32;
    for _ in 0..60 {
        let before = aim(&play)[0];
        tick(&mut play, [20.0, 0.0]);
        let d = aim(&play)[0] - before;
        turned += d - d.round();
    }
    assert!(turned.abs() > 0.2, "the mouse turns its turret: {turned} turns");
    let rounds = |play: &Play| play.battle.combat.rounds.len();
    let (mut most, fired_before) = (0, rounds(&play));
    play.key("SCAN_LMOUSE", true);
    for _ in 0..(3 * 60) {
        tick(&mut play, [0.0; 2]);
        most = most.max(rounds(&play));
    }
    play.key("SCAN_LMOUSE", false);
    assert!(most > fired_before, "its guns fire at the button");

    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::Command(bunker), "Esc: back to command mode");
    assert_eq!(play.driven_target(), None);

    // Its turret shot off, the button does nothing.
    let turret = play.emplacements.iter().find(|(e, _)| *e == bunker).unwrap().1.turret_part;
    play.battle.combat.targets[bunker].parts[turret].life.as_mut().unwrap().hit(1, f32::MAX / 4.0);
    play.economy.site_mut(bunker).unwrap().repair = None;
    tick(&mut play, [0.0; 2]);
    assert!(!play.manual_open(bunker));
    let now = play.hero.time_ms;
    cockpit.commander.click(&mut play, &mut cockpit.map, manual, manual, now);
    assert_eq!(play.mode(), Mode::Command(bunker), "inert with its turret dead");
}

/// The commander's map closes on its title bar's exit button, the map's `+0x230` rectangle
/// (605, 43)-(640, 63), which the column's click tests right after the lock
/// (`0x10084343`-`0x1008437d`); the rest of the title bar takes a click and does nothing.
#[test]
#[ignore = "needs the game install"]
fn the_commanders_map_closes_on_its_title_bars_exit_button() {
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::commander::Click;
    use parkan_world::cockpit::map::EXIT;
    use parkan_world::hud::Pages;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let now = play.hero.time_ms;
    cockpit.update(&mut play, now);
    cockpit.map.toggle();
    let title = [500.0, 50.0];
    let click = cockpit.commander.click(&mut play, &mut cockpit.map, title, title, now);
    assert!(cockpit.map.open, "the title itself closes nothing ({click:?})");
    let exit = [(EXIT[0] + EXIT[2]) / 2.0, (EXIT[1] + EXIT[3]) / 2.0];
    assert_eq!(cockpit.commander.click(&mut play, &mut cockpit.map, exit, exit, now), Click::Taken);
    assert!(!cockpit.map.open, "the exit closes the map");
}

/// The line under the commander's map names what the cursor points at on the map, not the
/// selection (`0x10073830`, docs/35, "The commander's variant"): a building's name over its
/// life, a unit's `"%s [%s]"`, nothing on empty ground; its clan piece red for a hostile clan,
/// grey for a neutral one and light blue for the player's own.
#[test]
#[ignore = "needs the game install"]
fn the_line_under_the_commanders_map_names_what_the_cursor_points_at_on_the_map() {
    use glam::Mat4;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::map::{LINE_HOSTILE, LINE_NEUTRAL, LINE_OTHER, Pointed, line_colour, pointed};
    use parkan_world::hud::{Pages, Space};
    use parkan_world::pick::Aim;
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    let (factory, builder, flyer) = (
        object_target(&play, &m, "lplant01.dat"),
        object_target(&play, &m, "tut3_b.dat"),
        object_target(&play, &m, "tut3_f1.dat"),
    );
    let neutral = play.units[bunker].clan;
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    play.hero.radar.range = 1.0e5;
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let now = play.hero.time_ms;
    cockpit.update(&mut play, now);
    cockpit.map.toggle();
    let space = Space::new(640.0, 480.0);
    let line = |cockpit: &mut Cockpit, play: &parkan_world::play::Play, t: Option<usize>| {
        let aim = match t {
            Some(t) => {
                let at = play.battle.combat.targets[t].position;
                Aim::Map([at.x, at.y])
            }
            None => Aim::Map([5.0, 5.0]),
        };
        let pick = play.pick(aim);
        cockpit.map.pointed = pick.object;
        let drawn = cockpit.draw(play, space, &font, &menu, Mat4::IDENTITY);
        // The bar's text is centred in 183 from x 446, under the map: a run's anchor is in NDC.
        let text: Vec<String> = drawn
            .text
            .iter()
            .filter(|r| {
                let [x, y] = [(r.anchor[0] + 1.0) * 320.0, (1.0 - r.anchor[1]) * 240.0];
                (330.0..349.0).contains(&y) && (446.0..629.0).contains(&x)
            })
            .map(|r| r.text.clone())
            .collect();
        (pointed(play, pick.object), text)
    };
    assert_eq!(
        line(&mut cockpit, &play, Some(factory)),
        (Pointed::Building(factory), vec!["Large Factory".into()])
    );
    // The builder, its clan's second name after the hero's (docs/35, "Name and status").
    let (what, text) = line(&mut cockpit, &play, Some(builder));
    assert_eq!(what, Pointed::Unit(builder));
    assert_eq!(text.len(), 1);
    assert!(text[0].starts_with("SWB-2 Builder ["), "{text:?}");
    assert_eq!(line(&mut cockpit, &play, None), (Pointed::Nothing, vec![]));
    // The enemy's flyers start shut down, and a hostile unit so ordered gets no mark and no
    // pick on the map (docs/35, "Which get a mark").
    let shut = play
        .robots
        .iter()
        .any(|(t, r)| *t == flyer && r.order.is_some_and(|o| o.code == parkan_sim::orders::SHUTDOWN));
    assert!(shut);
    assert_eq!(line(&mut cockpit, &play, Some(flyer)), (Pointed::Nothing, vec![]));
    // The clan piece's colour by the player's word towards the object's clan.
    let colour = |t: usize| line_colour(play.units[t].clan.and_then(|c| play.word(play.player_clan, c)));
    assert_eq!(colour(factory), LINE_OTHER, "the player's own: 2 towards itself");
    assert_eq!(colour(flyer), LINE_HOSTILE, "Enm");
    assert_eq!(line_colour(neutral.and_then(|c| play.word(play.player_clan, c))), LINE_NEUTRAL, "Ntrl");
}

/// Telepresence at auto-driver level 1 leaves the unit's walk to its AI and at 0 does not
/// (`iron3d.dll:0x10074ff0`: level 1 gives the Wizard's unit word, the behaviour's movement
/// flag `0x10`, to the AI, and keeps the turret and guns the player's).
#[test]
#[ignore = "needs the game install"]
fn telepresence_at_level_1_lets_the_warbots_ai_walk_it_and_at_level_0_the_player_does() {
    use parkan_sim::orders::{GO, Order, Target};
    use parkan_world::play::Mode;

    let walked = |level: u8| {
        let (mut play, m) = mission_03_play();
        let bunker = object_target(&play, &m, "sbunk01.dat");
        play.units[bunker].clan = Some(play.player_clan);
        play.enter_command(bunker);
        let project = play.factories[0].projects[0].clone();
        let at = play.battle.combat.targets[bunker].position + glam::Vec3::new(40.0, 0.0, 0.0);
        let bot = play.spawn(&project, play.player_clan, at, 0.0).expect("the prebuilt design is a robot");
        play.tick(1000.0 / 60.0, [0.0; 2]);
        assert!(play.telepresence(bot, level));
        assert_eq!((play.mode(), play.auto_driver()), (Mode::Driving(bot), level));
        fn robot(p: &parkan_world::play::Play, bot: usize) -> &parkan_world::hero::Robot {
            &p.robots.iter().find(|(t, _)| *t == bot).unwrap().1
        }
        assert_eq!(robot(&play, bot).walker.body.turret_lock, level == 0, "the lock is level 0's alone");
        // A Route 40 m on, given while the player has it.
        let from = robot(&play, bot).walker.body.position;
        let goal = from + glam::Vec3::new(40.0, 0.0, 0.0);
        play.dispatch(Order { code: GO, parameter: 0, target: Target::Place(goal.to_array()) });
        // The nearest it comes: over at its place, the go, its only order, leaves a patrol of
        // radius 150 about it (docs/42, "Spreading a group").
        let mut to = robot(&play, bot).walker.body.position;
        for _ in 0..(60 * 20) {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
            let now = robot(&play, bot).walker.body.position;
            if now.truncate().distance(goal.truncate()) < to.truncate().distance(goal.truncate()) {
                to = now;
            }
        }
        assert_eq!(robot(&play, bot).walker.body.command, [0.0; 3], "no key of the player's moved it");
        assert!(play.roll_back());
        assert_eq!(play.auto_driver(), 0, "on foot, the hero's level");
        assert_eq!(robot(&play, bot).auto_driver, level, "the unit's record keeps its own");
        (from.truncate().distance(goal.truncate()), to.truncate().distance(goal.truncate()))
    };
    let (before, after) = walked(1);
    assert!(
        after < before - 20.0,
        "at level 1 its AI walks it: {before:.1} m from the goal, then {after:.1}"
    );
    let (before, after) = walked(0);
    assert!(
        (after - before).abs() < 5.0,
        "at level 0 it only settles, waiting for the player: {before:.1}, then {after:.1}"
    );
}

/// A command key pressed from the view the player has.
fn press(play: &mut parkan_world::play::Play, command: &str) {
    let eye = play.eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: glam::Mat4::IDENTITY,
        shift: false,
    };
    play.command(command, &view);
}

/// The auto-driver level lives on the unit's record (`+0x9c`): Y steps nothing on foot
/// (`0x10072645`, modes 1 and 2 only), boarding takes a bot at whatever its record holds (mode 0
/// → 1, `0x100637c0`, writes no level), and a level 2 the key reached from 1 leaves the camera
/// and the repair switch the player's, where one a record held at boarding does not -- the take
/// at 2 writes neither `+0x210` nor `+0x214` (`0x10075072`-`0x10075099`). Letting go keeps the
/// level.
#[test]
#[ignore = "needs the game install"]
fn a_bot_is_boarded_at_the_level_its_record_holds_and_the_key_keeps_the_players_camera_at_2() {
    use parkan_formats::controls::CMD_JAMES_AUTO_DRIVER;
    use parkan_world::hero::Reach;
    use parkan_world::play::{Mode, Play};

    let (mut play, m) = mission_04_play();
    let hq = object_target(&play, &m, "tut4_hq.dat");
    fn record(play: &mut Play, hq: usize) -> &mut parkan_world::hero::Robot {
        &mut play.robots.iter_mut().find(|(t, _)| *t == hq).unwrap().1
    }
    for _ in 0..3 {
        press(&mut play, CMD_JAMES_AUTO_DRIVER);
    }
    assert_eq!((play.auto_driver(), record(&mut play, hq).auto_driver), (0, 0), "on foot Y steps nothing");

    // The record left at 2, as a unit page's C button leaves it: boarding takes it there.
    record(&mut play, hq).auto_driver = 2;
    stand_facing(&mut play, hq, 12.0, 0.0);
    play.tick(1000.0 / 60.0, [0.0; 2]);
    while play.targets.current != Some(hq) {
        play.targets.select_next();
    }
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::Driving(hq), "boarded");
    assert_eq!(
        (play.auto_driver(), play.reach()),
        (2, Reach::Nothing),
        "the AI has it whole, the camera too"
    );

    // Y: 2, 0, 1, 2 -- and the 2 reached from 1 keeps the sensors' words the player's.
    let mut steps = Vec::new();
    for _ in 0..3 {
        press(&mut play, CMD_JAMES_AUTO_DRIVER);
        steps.push((play.auto_driver(), play.reach()));
    }
    assert_eq!(steps, vec![(0, Reach::Whole), (1, Reach::Weapons), (2, Reach::Sensors)]);
    assert!(
        play.reach().sensors() && !play.reach().shields(),
        "camera and repair the player's, shields the AI's"
    );

    // Esc lets it go: the record keeps 2, and the words go back to the AI.
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::OnFoot);
    let robot = record(&mut play, hq);
    assert_eq!((robot.auto_driver, robot.sensors_taken), (2, false));
}

/// A hit reaches a warbot's behaviour whoever drives it (`Behavior.dll:0x100064b0` asks
/// nothing of the take), and its repair decision runs in its takt: at level 1, where flag
/// `0x10` lets the takt run, it turns on the unit that fired and switches its repair system on
/// under the player's hand. At level 0 the takt does not run, but the hit handler gates the
/// hit as it arrives (`0x100179c0`): the attack goes on the stack at once and waits there,
/// and no repair decision is taken while the player has it.
#[test]
#[ignore = "needs the game install"]
fn a_warbot_taken_at_level_1_turns_on_its_firer_and_its_ai_switches_its_repair_on() {
    use parkan_sim::behaviour::Task;

    let run = |level: u8| {
        let (mut play, m) = mission_03_play();
        let bunker = object_target(&play, &m, "sbunk01.dat");
        play.units[bunker].clan = Some(play.player_clan);
        play.enter_command(bunker);
        let project = play.factories[0].projects[0].clone();
        // Clear of the bunker, whose roof a bot would first escape (the ground Mission 03's
        // warbot test stands its four on).
        let ground = |p: &parkan_world::play::Play, x: f32, y: f32| {
            glam::Vec3::new(x, y, p.ground.below(x, y, 10_000.0).map_or(0.0, |h| h.point.z) + 1.0)
        };
        let at = ground(&play, 1700.0, 300.0);
        let bot = play.spawn(&project, play.player_clan, at, 0.0).unwrap();
        // A neutral clan's unit, which no engagement takes up, so an attack on it is the hit's.
        let other = (0..play.clans.len() as i64)
            .find(|&c| play.clans[c as usize].kind == parkan_formats::mission::CLAN_NEUTRAL)
            .expect("Mission 03 has a neutral clan");
        let far = ground(&play, 1700.0, 420.0);
        let firer = play.spawn(&project, other, far, 0.0).unwrap();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        assert!(play.telepresence(bot, level));
        // Every node at 30% of its life: under `Decision_RepairOn`'s 0.5, with a full battery.
        for part in &mut play.battle.combat.targets[bot].parts {
            for node in part.life.iter_mut().flat_map(|l| l.nodes.iter_mut()) {
                node.life = 0.3 * node.max;
            }
        }
        play.hurt(bot, Some(firer));
        let id = play.units[firer].logical_id;
        // At once, before any takt: the gate has run, whatever the level.
        let robot = &play.robots.iter().find(|(t, _)| *t == bot).unwrap().1;
        let at_once = robot.behaviour.hurt_by.is_none()
            && robot
                .behaviour
                .tasks
                .iter()
                .any(|t| matches!(t, Task::Attack { target: Some(t), .. } if *t == id));
        let mut turned = false;
        let mut repair = false;
        play_for(&mut play, 2.0, |p| {
            let robot = &p.robots.iter().find(|(t, _)| *t == bot).unwrap().1;
            // The attack goes on top; a refit the hurt unit sends itself on may go over it.
            turned |= robot
                .behaviour
                .tasks
                .iter()
                .any(|t| matches!(t, Task::Attack { target: Some(t), .. } if *t == id));
            repair |= p.driving.as_ref().is_some_and(|d| d.pilot.switches.repair);
        });
        // Let go, the unit's takt runs, and the hit it took is answered then.
        assert!(play.roll_back());
        play_for(&mut play, 0.1, |_| {});
        let robot = &play.robots.iter().find(|(t, _)| *t == bot).unwrap().1;
        let after = robot
            .behaviour
            .tasks
            .iter()
            .any(|t| matches!(t, Task::Attack { target: Some(t), .. } if *t == id));
        (at_once, turned, repair, after)
    };
    assert_eq!(
        run(1),
        (false, true, true, true),
        "level 1: the takt runs, so the hit is answered at its next one and the repair decision acts"
    );
    assert_eq!(
        run(0),
        (true, true, false, true),
        "level 0: the gate puts the attack on the stack at once; the takt waits for the letting-go"
    );
}

/// Where Mission 04's capturer is when a landing flag is set with building `id` picked: its
/// landing corner.
fn landing_corner(
    play: &parkan_world::play::Play,
    heli: usize,
    id: i32,
    was: &mut bool,
) -> Option<glam::Vec3> {
    use parkan_sim::behaviour::Task;
    let robot = &play.robots.iter().find(|(t, _)| *t == heli).unwrap().1;
    let landing =
        matches!(robot.behaviour.task(), Task::Search { building: Some(b), landing: true, .. } if b == id);
    let corner = (landing && !*was).then_some(robot.walker.body.position);
    *was = landing;
    corner
}

#[test]
#[ignore = "needs the game install"]
fn one_enter_takes_and_boards_mission_04s_hq_and_its_capture_completes_the_first_objective() {
    use parkan_world::play::Mode;

    let (mut play, m, hq) = mission_04_aboard_the_hq();
    let heli = object_target(&play, &m, "tut4_f1.dat");
    assert!(play.is_hq(hq) && !play.is_hq(heli), "the HQ's turret carries 0x8000000");
    assert_eq!(play.units[hq].clan, Some(play.player_clan), "captured");
    assert_eq!(play.mode(), Mode::Driving(hq), "and boarded");
    assert_eq!(play.aboard(), Some(hq));
    assert!(play.driving.as_ref().is_some_and(|d| d.target == hq && !d.telepresence));
    play_for(&mut play, 3.0, |_| {});
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[0].state, 1, "fn52(1) answers the player's clan");
    assert!(p.progress.played[&10] && p.progress.played[&15], "T04_I02 and T04_H01");
}

#[test]
#[ignore = "needs the game install"]
fn enter_aboard_mission_04s_hq_opens_its_command_view_whose_camera_rides_with_it_and_esc_steps_back_out() {
    use parkan_sim::orders::{Order, Target};
    use parkan_world::command::{REACH_TIMES, SNAP_ACROSS};
    use parkan_world::play::Mode;

    let (mut play, m, hq) = mission_04_aboard_the_hq();
    let heli = object_target(&play, &m, "tut4_f1.dat");
    play_for(&mut play, 1.0, |_| {});
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::HqCommand(hq), "Enter aboard an HQ pushes mode 3");
    assert!(play.driving.is_none(), "the HQ is let go to its AI");
    assert!(play.hero_away() && play.aboard() == Some(hq), "the hero stays aboard");
    assert_eq!(play.selected_units(), vec![hq]);
    let at = play.battle.combat.targets[hq].position;
    assert_eq!((play.command.position, play.command.distance), (at, 0.0), "placed on the HQ");
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::HqCommand(hq), "Enter does nothing in mode 3");

    // The camera backs off a metre a frame to 8 r, 61.2 m on this HQ's chassis sphere, and sits
    // d back along its look.
    let reach = play.ride_reach(hq).unwrap();
    assert!((reach - REACH_TIMES * 7.653).abs() < 0.01, "{reach}");
    command_frames(&mut play, 30, |_| {});
    assert_eq!(play.command.distance, 30.0);
    command_frames(&mut play, (reach as usize) + 10, |_| {});
    let d = play.command.distance;
    assert!(d > reach && d <= reach + 1.0, "{d} against {reach}");
    let at = play.battle.combat.targets[hq].position;
    let back = d * play.command.tilt.sin();
    let across = (play.command.position - at).truncate();
    assert!((across.length() - back).abs() <= SNAP_ACROSS, "{} from the HQ against {back}", across.length());

    // Route: the HQ drives off, and the camera goes with it. The goal is 70 m ahead of it, on the
    // walkable pad it stands on; the ground north of the pad is not walkable, and the walker
    // refuses a goal there (docs/24, "The global path").
    // The drive is measured while the go runs: once over, the go, the HQ's only order, leaves a
    // patrol of radius 150 about its place (docs/42, "Spreading a group").
    let goal = at + glam::Vec3::new(-44.0, -54.0, 0.0);
    play.dispatch(Order { code: parkan_sim::hq::GO, parameter: 0, target: Target::Place(goal.to_array()) });
    let mut worst: f32 = 0.0;
    command_frames(&mut play, 600, |p| {
        let going = p
            .robots
            .iter()
            .any(|(t, r)| *t == hq && matches!(r.behaviour.task(), parkan_sim::behaviour::Task::Go { .. }));
        if !going {
            return;
        }
        let at = p.battle.combat.targets[hq].position;
        let across = (p.command.position - at).truncate().length();
        worst = worst.max((across - p.command.distance * p.command.tilt.sin()).abs());
    });
    let moved = (play.battle.combat.targets[hq].position - at).truncate().length();
    assert!(moved > 30.0, "the HQ drove {moved} m");
    assert!(worst < 3.0 * SNAP_ACROSS, "the camera kept to the HQ within {worst}");

    // Telepresence into the helicopter, and Esc back to the HQ's view, pulled out afresh.
    assert!(play.telepresence(heli, 0));
    assert_eq!(play.mode(), Mode::Driving(heli));
    assert_eq!(play.aboard(), Some(hq), "the hero is still aboard the HQ");
    command_frames(&mut play, 30, |_| {});
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::HqCommand(hq));
    let at = play.battle.combat.targets[hq].position;
    assert_eq!((play.command.position, play.command.distance), (at, 0.0), "2 -> 3 re-places the camera");
    command_frames(&mut play, 10, |_| {});

    // Esc: the HQ's cockpit, taken at level 0; Esc again: the hero on foot beside it.
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::Driving(hq));
    assert!(play.driving.as_ref().is_some_and(|d| d.target == hq && !d.telepresence));
    assert_eq!((play.auto_driver(), play.selected_units()), (0, vec![hq]));
    assert!(play.command.follows.is_none());
    command_frames(&mut play, 10, |_| {});
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::OnFoot);
    assert!(!play.hero_away());
    let hero = play.hero.walker.body.position;
    let at = play.battle.combat.targets[hq].position;
    assert!((hero - at).truncate().length() < 25.0, "put down beside the HQ");
}

#[test]
#[ignore = "needs the game install"]
fn the_hero_button_rolls_mission_04s_hq_view_back_to_foot_and_a_lost_hq_puts_the_hero_out() {
    use parkan_world::play::Mode;

    let (mut play, _, hq) = mission_04_aboard_the_hq();
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::HqCommand(hq));
    play.roll_back_to_foot();
    assert_eq!(play.modes, vec![Mode::OnFoot], "through the HQ's cockpit to the hero on foot");

    let (mut play, _, hq) = mission_04_aboard_the_hq();
    press_enter(&mut play);
    play.battle.combat.targets[hq].alive = false;
    command_frames(&mut play, 2, |_| {});
    assert_eq!(play.mode(), Mode::OnFoot, "rolled back and put out");
    assert!(!play.hero_away());
}

#[test]
#[ignore = "needs the game install"]
fn mission_04s_helicopter_searches_out_the_factory_and_the_research_centre_landing_at_their_corners() {
    use parkan_world::cockpit::panels::status_order;
    use parkan_world::progress::{Say, Sender};

    let (mut play, m) = mission_04_play();
    let heli = object_target(&play, &m, "tut4_f1.dat");
    let plant = object_target(&play, &m, "lplant01.dat");
    let centre = object_target(&play, &m, "einst01.dat");
    let player = play.player_clan;
    let objectives = |p: &parkan_world::play::Play| {
        p.progression.as_ref().unwrap().progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>()
    };
    // The Battle units page offers the helicopter Search and capture, a search by type.
    play.commander.units = vec![heli];
    assert!(play.hq_rows().contains(&2), "{:?}", play.hq_rows());
    play.hq_command(2).expect("Search and capture");
    assert_eq!(status_order(&play, heli), 5, "searching");

    let mut says = Vec::new();
    for (building, id, corner, objective) in [
        (plant, play.units[plant].logical_id, glam::Vec2::new(734.3, 892.0), 1),
        (centre, play.units[centre].logical_id, glam::Vec2::new(881.8, 900.3), 2),
    ] {
        let (mut landed, mut was, mut taken) = (None, false, None);
        for tick in 0..(180 * 60) {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
            says.append(&mut play.says);
            if let Some(at) = landing_corner(&play, heli, id, &mut was) {
                landed.get_or_insert(at);
            }
            if play.units[building].clan == Some(player) {
                taken = Some(tick as f32 / 60.0);
                break;
            }
        }
        let taken =
            taken.unwrap_or_else(|| panic!("{} is not taken", m.objects[play.battle.objects[building]].path));
        let landed = landed.expect("it lands first");
        eprintln!(
            "{} taken {taken:.1} s on, landing at {landed}",
            m.objects[play.battle.objects[building]].path
        );
        assert!(landed.truncate().distance(corner) < 25.0, "lands at its corner {corner}: {landed}");
        play_for(&mut play, 2.5, |p| says.append(&mut p.says));
        assert_eq!(objectives(&play)[objective], 1, "objective {objective}");
    }
    let p = play.progression.as_ref().unwrap();
    let captured =
        says.iter().filter(|s| matches!(s, Say::Text(Sender::System, t) if t == "Building is captured"));
    assert_eq!(captured.count(), 2);
    let neutral = p.sound(parkan_world::play::VOICE_NBUILD_CAPTURE).expect("the neutral's voice");
    assert_eq!(says.iter().filter(|s| **s == Say::Voice(neutral.clone())).count(), 2);
    // With nothing left to take it roams, and never goes for the teleport.
    let teleport = object_target(&play, &m, "mtp_m_n1.dat");
    play_for(&mut play, 30.0, |_| {});
    assert_ne!(play.units[teleport].clan, Some(player));
}

#[test]
#[ignore = "needs the game install"]
fn a_click_on_the_research_centre_sends_the_helicopter_to_take_it_alone_then_it_escapes() {
    use parkan_sim::behaviour::{Search, Task};
    use parkan_world::pick::Aim;

    let (mut play, m) = mission_04_play();
    let heli = object_target(&play, &m, "tut4_f1.dat");
    let centre = object_target(&play, &m, "einst01.dat");
    let id = play.units[centre].logical_id;
    let player = play.player_clan;
    // A click straight down onto the neutral centre with the helicopter selected is pick kind 4,
    // the capture cursor, and a search on that one building.
    play.commander.units = vec![heli];
    let over = play.battle.combat.targets[centre].centre + glam::Vec3::new(0.0, 0.0, 300.0);
    let pick = play.pick(Aim::Ray { eye: over, direction: -glam::Vec3::Z });
    assert_eq!((pick.kind, pick.object), (4, Some(centre)));
    assert_eq!(parkan_world::pick::cursor_state(pick.kind), 6, "CAPTURE");
    play.click_world(pick);
    let task =
        |p: &parkan_world::play::Play| p.robots.iter().find(|(t, _)| *t == heli).unwrap().1.behaviour.task();
    assert!(matches!(task(&play), Task::Search { search: Search::Building(b), .. } if b == id));

    let mut seconds = 0.0;
    while play.units[centre].clan != Some(player) && seconds < 180.0 {
        play_for(&mut play, 0.25, |_| {});
        seconds += 0.25;
    }
    assert_eq!(play.units[centre].clan, Some(player), "taken in {seconds} s");
    let pod = play.robots.iter().find(|(t, _)| *t == heli).unwrap().1.walker.body.position;
    // The search ends; standing idle on the building the unit is given the escape, and walks off
    // the building to open ground within 150.
    play_for(&mut play, 0.5, |_| {});
    assert!(matches!(task(&play), Task::Leave { .. }), "{:?}", task(&play));
    let mut off = None;
    for second in 0..60 {
        play_for(&mut play, 1.0, |_| {});
        let robot = &play.robots.iter().find(|(t, _)| *t == heli).unwrap().1;
        if task(&play) == Task::Stop && robot.walker.ground.and_then(|h| h.solid).is_none() {
            off = Some((second, robot.walker.body.position));
            break;
        }
    }
    let (second, at) = off.expect("off the building and at rest within a minute");
    eprintln!("taken in {seconds} s, off the building {second} s later at {at}, from {pod}");
    assert!((at - pod).truncate().abs().max_element() < 175.0, "escaped to {at} from {pod}");
}

/// A building's mark on the satellite map is the building: the pick takes what the map marks
/// within 80 of the place clicked, so an order lands on it as it does on a click in the world
/// (docs/42, "On the open satellite map"). The neutral research centre is a capture for the
/// helicopter and an attack once a big warbot is the one selected.
#[test]
#[ignore = "needs the game install"]
fn a_click_on_a_buildings_mark_on_the_map_captures_with_a_small_bot_and_attacks_with_a_big_one() {
    use parkan_sim::behaviour::{Search, Task};
    use parkan_sim::orders::{ATTACK, Target};
    use parkan_world::pick::{Aim, cursor_state};

    let (mut play, m) = mission_04_play();
    let heli = object_target(&play, &m, "tut4_f1.dat");
    let centre = object_target(&play, &m, "einst01.dat");
    let id = play.units[centre].logical_id;
    // The whole map on the hero's radar, so the map marks every live object on it.
    play.hero.radar.range = 1.0e5;
    play.commander.units = vec![heli];
    let at = play.battle.combat.targets[centre].position;
    // The centre's mark with the helicopter selected: kind 4, the CAPTURE cursor, and a search
    // on that one building.
    let pick = play.pick(Aim::Map([at.x, at.y]));
    assert_eq!((pick.kind, pick.object), (4, Some(centre)), "{pick:?}");
    assert_eq!(cursor_state(pick.kind), 6, "CAPTURE");
    play.click_world(pick);
    let task =
        |p: &parkan_world::play::Play| p.robots.iter().find(|(t, _)| *t == heli).unwrap().1.behaviour.task();
    assert!(matches!(task(&play), Task::Search { search: Search::Building(b), .. } if b == id));
    // The same mark with a big warbot selected: kind 3, the TARGET cursor, and an attack on it.
    play.units[heli].designation.size_class = 3;
    let pick = play.pick(Aim::Map([at.x, at.y]));
    assert_eq!((pick.kind, pick.object), (3, Some(centre)), "{pick:?}");
    assert_eq!(cursor_state(pick.kind), 4, "TARGET");
    play.click_world(pick);
    let order = play.robots.iter().find(|(t, _)| *t == heli).unwrap().1.order;
    assert_eq!(order.map(|o| (o.code, o.target)), Some((ATTACK, Target::LogicId(id))));
    // Well away from every mark the map still sends the unit to the place clicked.
    let empty = [at.x + 400.0, at.y + 400.0];
    let pick = play.pick(Aim::Map(empty));
    assert_eq!((pick.kind, pick.object), (1, None), "{pick:?}");
}

/// Follow me on The Iron Monster: the wingmen follow the hero over the canyon by its bridge.
/// Near the bridge most spots about the hero lie over the canyon, which the walker refuses, so
/// each pick takes the first of 77 on walkable ground (docs/31, "Follow me"); and a follower
/// planned again part way along the bridge's hall way goes on along it rather than back to the
/// vertex it has passed. Over the middle of the deck neither stops: the hall way's vertices
/// stand 4 m over it, and a walker's leg to one is timed across the ground, where its whole
/// length once slowed a leg just short of the halves' joint to a standstill.
#[test]
#[ignore = "needs the game install"]
fn c02_m01s_wingmen_follow_the_hero_over_the_bridge_and_keep_out_of_the_canyon() {
    use glam::Vec3;
    use parkan_sim::orders::{self, Order, Target};
    use parkan_world::play::Play;

    let mut play = campaign_play(gamedir::C02_MISSION_01);
    // This is a test of the walk, and the south bank is inside the enemy clan's patrol: since
    // M18 its planner puts warbots on `PBM_PLACE_PROTECT` there, and they shoot one of the two
    // followers apart before it reaches the deck. The progression goes, so no clan plans.
    play.progression = None;
    let graph = play.graph.clone().expect("KM_4 has an areal map");
    let hero_id = play.hero_id;
    // The player's two `21swlk1`, logical ids 9 and 10, brought to the south bank and told to
    // follow the hero.
    let bots: Vec<usize> =
        [9, 10].iter().map(|&id| play.units.iter().position(|u| u.logical_id == id).unwrap()).collect();
    let south = Vec3::new(470.0, 520.0, 0.0);
    assert!(play.stand_at(south.x, south.y, 0.0));
    for (k, &t) in bots.iter().enumerate() {
        let Play { robots, ground, .. } = &mut play;
        let robot = &mut robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1;
        put(&mut robot.walker, ground, Vec3::new(490.0 + 10.0 * k as f32, 500.0, 100.0));
        let order = Order { code: orders::FOLLOW, parameter: 50, target: Target::LogicId(hero_id) };
        assert!(robot.behaviour.insert_order(&order, orders::INSERT_REPLACE));
        robot.order = Some(order);
    }
    let at =
        |play: &Play, t: usize| play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;

    // The hero walks over the bridge at 10 m/s to the north plateau and waits there.
    let way =
        [south, Vec3::new(421.0, 573.0, 0.0), Vec3::new(427.0, 656.0, 0.0), Vec3::new(434.0, 747.0, 0.0)];
    let north = Vec3::new(433.0, 800.0, 0.0);
    let (mut hero, mut leg) = (south, 1);
    let mut lowest = f32::MAX;
    // When each follower came onto the deck at y 620 and when it left it at y 700. A follower
    // is faster than the hero, 13 m/s to its 10, so it catches its place 20 m behind, stands a
    // second or so and sets off again; where on the way that falls moves with every change to
    // how the two are pushed about. Pushed out as their node spheres, 2.22, they are no longer
    // held up at the bridge's foot, and one of the stops falls on the deck. What the deck must
    // never do is hold a follower: 80 m of it at 13 m/s is six seconds, and twelve is a stop
    // that was no arrival.
    let mut on_deck: Vec<Option<usize>> = vec![None; bots.len()];
    let mut off_deck: Vec<Option<usize>> = vec![None; bots.len()];
    for tick in 0..(120 * 60) {
        let to = if leg < way.len() { way[leg] } else { north };
        let off = (to - hero).truncate();
        if off.length() <= 10.0 / 60.0 {
            hero = to;
            leg += 1;
        } else {
            hero += (off.normalize() * 10.0 / 60.0).extend(0.0);
        }
        let yaw = play.hero.walker.body.yaw;
        play.stand_at(hero.x, hero.y, yaw);
        play.tick(1000.0 / 60.0, [0.0; 2]);
        lowest = bots.iter().map(|&t| at(&play, t).z).fold(lowest, f32::min);
        for (k, &t) in bots.iter().enumerate() {
            let p = at(&play, t);
            if p.y >= 620.0 && p.z > 55.0 {
                on_deck[k].get_or_insert(tick);
            }
            if p.y >= 700.0 && on_deck[k].is_some() {
                off_deck[k].get_or_insert(tick);
            }
        }
    }
    let crossing: Vec<f32> = on_deck
        .iter()
        .zip(&off_deck)
        .map(|(on, off)| on.zip(*off).map_or(f32::MAX, |(on, off)| (off - on) as f32 / 60.0))
        .collect();
    assert!(crossing.iter().all(|&s| s < 12.0), "the deck holds no follower: over it in {crossing:?} s");
    let hero = play.hero.walker.body.position;
    for &t in &bots {
        let p = at(&play, t);
        assert!(
            p.truncate().distance(hero.truncate()) < 40.0,
            "unit {} follows over the bridge: {p}",
            play.units[t].logical_id
        );
        assert!(graph.usable(p.x, p.y), "and stands on walkable ground: {p}");
    }
    assert!(lowest > 35.0, "never down in the canyon: {lowest}");
}

/// The Lost Key: a pod is driven by its controller's channel across frames its node's own
/// animation run does not reach, and past the run the node holds its last key
/// (`AniMesh.dll:0x10012ba2`). The neutral Medium Core mine's pod runs frames 1 to 3 over a
/// node whose run ends at frame 2, and blending on into the key after it — the next node's —
/// threw the pod tens of metres out of the building halfway through its two-second stroke.
/// The zone it fires from goes with the node, so nobody was ever in it when the pod reported
/// open, and the hero or a warbot stood on the pod indefinitely without taking the mine.
/// 27 of the 32 placed building models are driven past a run's end this way.
#[test]
#[ignore = "needs the game install"]
fn a_capture_walks_round_c02_m03s_mine_to_the_door_it_can_reach_rather_than_at_its_wall() {
    use parkan_sim::orders::{Order, Target};
    use parkan_world::factory::Project;

    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let mine = play
        .units
        .iter()
        .position(|u| u.type_word == 0x8000_0004 && u.clan == Some(2))
        .expect("the neutral mine");
    let id = play.units[mine].logical_id;
    let centre = play.battle.combat.targets[mine].position;
    let pod = play.capture_places().into_iter().find(|p| p.id == id).and_then(|p| p.pod).expect("its pod");

    // A small warbot 60 m out on the pod's own side of the mine, where the way to the pod runs
    // through the walls.
    let project = Project {
        path: "UNITS\\UNITS\\PREBLD\\tut3_p1.dat".into(),
        name: "SSW-X Warrior".into(),
        type_word: 0x0100_4000,
        chassis_size: 2,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let away = (pod.truncate() - centre.truncate()).normalize_or_zero();
    let spot = centre + (away * 60.0).extend(0.0);
    let t = play.spawn(&project, play.player_clan, spot, 0.0).expect("the SSW-X");
    for _ in 0..60 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }

    // The way in starts at a door the walk outside can reach (docs/24, "The way to the pod"):
    // the mine's east door stands 28 m over the terrain, up its own ramps, which the areal map
    // does not carry, so the ground-level door on the west is taken instead.
    let from = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    let way = play.way_in(mine, from, pod, false).expect("a way in");
    let door = way[0];
    let under = play.ground.below(door.x, door.y, door.z + 40.0).expect("ground under the door").point.z;
    assert!(door.z - under < 20.0, "a door {:.1} over the ground under it: {door}", door.z - under);
    assert!(door.x < centre.x, "the door on the far side from the pod: {door}");

    // It walks round the building to that door, in along the hall way, and takes the pod. It had
    // driven at the wall on the pod's side and shuffled there for good.
    let order = Order { code: parkan_sim::orders::CAPTURE, parameter: 0, target: Target::LogicId(id) };
    play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1.behaviour.order(&order);
    let mut round = false;
    let mut captured = None;
    for s in 0..150 {
        for _ in 0..60 {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
        let at = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
        round |= at.x < door.x + 20.0;
        if play.units[mine].clan == Some(play.player_clan) {
            captured = Some(s);
            break;
        }
    }
    let seconds = captured.unwrap_or_else(|| {
        let at = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
        panic!("it never took the mine: stopped at {at}, {:.1} from the pod", at.distance(pod))
    });
    assert!(round, "it went round to the door rather than at the wall");
    eprintln!("round the mine and on the pod {seconds} s in");
}
