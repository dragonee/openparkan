//! How a machine moves: walking, strafing, slopes, what stops it, water and flight.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn a_strafe_turns_the_hull_while_the_turret_holds_the_sight() {
    use parkan_formats::{landmesh, mission};
    use parkan_sim::ground::Ground;
    use parkan_world::{assembly::Assembly, hero::Hero};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut assembly = Assembly::new(&game).unwrap();
    let mut hero = Hero::load(&mut assembly, &m).unwrap().expect("Mission 01 has a hero");
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    let ground = Ground::new(land);
    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    // The sight's heading against the unit's: the hull's yaw plus the turret's strafe
    // offset (docs/24, docs/30). A strafe turns the hull; the turret takes it back.
    let against_heading = |hero: &Hero| {
        let (_, sight) = hero.sight().expect("the hero's turret has a sight");
        wrap((-sight.x).atan2(sight.y) - hero.walker.drawn_heading(hero.time_ms))
    };
    let tick = 1000.0 / 60.0;
    hero.tick(tick, [0.0; 2], &ground);
    let rest = against_heading(&hero);

    // Through the strafe turn itself: afterwards the run cycle sways the turret's socket.
    hero.key("SCAN_A", true);
    let mut turned = 0.0_f32;
    for _ in 0..60 {
        hero.tick(tick, [0.0; 2], &ground);
        let t = hero.time_ms;
        turned = turned.max(wrap(hero.walker.drawn(t).1 - hero.walker.drawn_heading(t)).abs());
        let off = wrap(against_heading(&hero) - rest);
        assert!(off.abs() < 0.005, "the sight drifted {off} rad from the heading at {t} ms");
        if turned > std::f32::consts::FRAC_PI_2 - 0.01 {
            return;
        }
    }
    panic!("the hull turned only {turned} rad away from the heading");
}

#[test]
#[ignore = "needs the game install"]
fn backing_up_under_a_strafe_goes_back_to_the_strafes_side_while_the_sight_holds() {
    use parkan_formats::{landmesh, mission};
    use parkan_sim::ground::Ground;
    use parkan_world::{assembly::Assembly, hero::Hero};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    let ground = Ground::new(land);
    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let sight_heading = |hero: &Hero| {
        let (_, sight) = hero.sight().expect("the hero's turret has a sight");
        (-sight.x).atan2(sight.y)
    };
    // Where the hero goes with S and then a strafe key held, in degrees clockwise from where
    // its sight looked before (docs/24, "From input to motion"): the heading of its move over
    // the second and third seconds, the sight's drift from the start the most it got to.
    let walk = |strafe: &str| {
        let mut assembly = Assembly::new(&game).unwrap();
        let mut hero = Hero::load(&mut assembly, &m).unwrap().expect("Mission 01 has a hero");
        let tick = 1000.0 / 60.0;
        hero.tick(tick, [0.0; 2], &ground);
        let ahead = sight_heading(&hero);
        hero.key("SCAN_S", true);
        hero.key(strafe, true);
        let (mut from, mut drift) = (hero.walker.drawn(hero.time_ms).0, 0.0_f32);
        for t in 0..180 {
            hero.tick(tick, [0.0; 2], &ground);
            if t == 59 {
                from = hero.walker.drawn(hero.time_ms).0;
            }
            if t >= 60 {
                drift = drift.max(wrap(sight_heading(&hero) - ahead).abs());
            }
        }
        let moved = hero.walker.drawn(hero.time_ms).0 - from;
        let clockwise = -wrap((-moved.x).atan2(moved.y) - ahead).to_degrees();
        (clockwise.rem_euclid(360.0), moved.truncate().length(), drift)
    };
    let (left, left_moved, left_drift) = walk("SCAN_A");
    let (right, right_moved, right_drift) = walk("SCAN_D");
    assert!((left - 225.0).abs() < 12.0 && left_moved > 5.0, "S and A went {left} degrees, {left_moved} m");
    assert!(
        (right - 135.0).abs() < 12.0 && right_moved > 5.0,
        "S and D went {right} degrees, {right_moved} m"
    );
    assert!(left_drift < 0.2 && right_drift < 0.2, "the sight drifted {left_drift} and {right_drift} rad");
}

/// Walk the hero from `from` facing `yaw` for `seconds`, holding W; where it stood each tick.
fn walk(play: &mut parkan_world::play::Play, from: glam::Vec3, yaw: f32, seconds: usize) -> Vec<glam::Vec3> {
    let w = &mut play.hero.walker;
    w.body.position = from;
    w.body.yaw = yaw;
    w.follow_ground(&play.ground);
    w.from = (w.body.position, w.body.yaw);
    w.from_heading = w.body.yaw;
    play.hero.key("SCAN_W", true);
    let mut out = Vec::new();
    for _ in 0..(60 * seconds) {
        play.hero.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        out.push(play.hero.walker.body.position);
    }
    play.hero.key("SCAN_W", false);
    out
}

#[test]
#[ignore = "needs the game install"]
fn q_walks_the_hero_straight_on_and_off_and_a_strafe_or_a_walk_back_ends_it() {
    use glam::Vec3;

    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    let at = |play: &parkan_world::play::Play| play.hero.walker.body.position;
    let start = at(&play);
    tick(&mut play, 0.5);

    // Q with no key held runs the hero as W does, and it keeps running with nothing held.
    play.key("SCAN_Q", true);
    play.key("SCAN_Q", false);
    tick(&mut play, 3.0);
    let ran = at(&play).truncate().distance(start.truncate());
    assert!(ran > 20.0, "it runs on its own: {ran:.1} m in 3 s");

    // Q again stops it.
    play.key("SCAN_Q", true);
    play.key("SCAN_Q", false);
    tick(&mut play, 1.0);
    let stopped = at(&play);
    tick(&mut play, 2.0);
    assert!(at(&play).truncate().distance(stopped.truncate()) < 1.0, "Q again stands it: {}", at(&play));

    // A strafe ends it: the hero stands once the key comes up, and stays standing.
    for key in ["SCAN_A", "SCAN_D", "SCAN_S"] {
        play.key("SCAN_Q", true);
        play.key("SCAN_Q", false);
        tick(&mut play, 1.0);
        play.key(key, true);
        tick(&mut play, 0.5);
        play.key(key, false);
        tick(&mut play, 0.5);
        let held = at(&play);
        tick(&mut play, 2.0);
        let after = at(&play).truncate().distance(held.truncate());
        assert!(after < 1.0, "{key} ends the walk: {after:.1} m more after it came up");
    }

    // Put back where it started, the hero on Q walks the way it faces.
    let heading = play.hero.walker.body.yaw;
    play.hero.walker.body.position = start;
    play.hero.walker.body.yaw = heading;
    play.key("SCAN_Q", true);
    play.key("SCAN_Q", false);
    tick(&mut play, 2.0);
    let went = at(&play) - start;
    let along = Vec3::new(-heading.sin(), heading.cos(), 0.0);
    assert!(went.truncate().dot(along.truncate()) > 0.9 * went.truncate().length(), "straight ahead: {went}");
    play.key("SCAN_Q", true);
    play.key("SCAN_Q", false);
}

#[test]
#[ignore = "needs the game install"]
fn the_hero_crosses_mission_01s_bridge_on_its_deck() {
    use glam::Vec3;
    let (mut play, _) = mission_01_play();
    // From the south bank, north along the bridge (docs/24, "Standing on a bridge").
    let path = walk(&mut play, Vec3::new(790.5, 540.0, 40.0), 0.0, 22);
    let over_gorge: Vec<&Vec3> = path.iter().filter(|p| (620.0..760.0).contains(&p.y)).collect();
    assert!(!over_gorge.is_empty(), "it reaches the gorge");
    let lowest = over_gorge.iter().map(|p| p.z).fold(f32::MAX, f32::min);
    assert!(lowest > 10.0, "on the deck the whole way over the gorge, not below {lowest}");
    assert!(path.last().unwrap().y > 800.0, "and off the far end: {}", path.last().unwrap());
}

/// A unit the Wizard drives is held by a slope as the player's own machine is: the engine's
/// stand-in (docs/24, "Ground and slope"). The game is read to pass the brake over while a
/// written velocity stands, so a driven machine is never braked; the engine brakes it, since its
/// local path does not keep an AI unit off such a face. A written velocity replaces the
/// machine's own at the top of every step, so the brake's pull never built up on one: an AI
/// unit walked up this face at 8 m/s where the same chassis under the player does not move.
#[test]
#[ignore = "needs the game install"]
fn a_slope_past_the_cone_holds_a_wizard_driven_unit_as_it_holds_the_player() {
    use glam::Vec3;
    let (mut play, _) = mission_01_play();
    assert_eq!(play.hero.walker.controller.mode, 2, "the hero's chassis is one that brakes");
    // A 40 degree face on Tut_1, past the mode-2 cone of 0.6 rad, and the way up it.
    let hit = play.ground.below(125.0, 1050.0, 1.0e5).expect("ground at (125, 1050)");
    let degrees = hit.normal.z.acos().to_degrees();
    assert!((39.0..41.0).contains(&degrees), "a {degrees} degree face");
    let up = -Vec3::new(hit.normal.x, hit.normal.y, 0.0).normalize();
    let yaw = up.y.atan2(up.x) - std::f32::consts::FRAC_PI_2;
    let from = hit.point + Vec3::new(0.0, 0.0, 20.0);

    let path = walk(&mut play, from, yaw, 6);
    let keyed = path.last().unwrap().truncate().distance(path[0].truncate());

    // The same chassis with the Wizard writing its top speed uphill, as a behaviour drives one.
    let top = play.hero.walker.limits.top_speed[1].abs();
    let w = &mut play.hero.walker;
    w.body.position = from;
    w.body.yaw = yaw;
    w.follow_ground(&play.ground);
    w.from = (w.body.position, w.body.yaw);
    let start = play.hero.walker.body.position;
    for _ in 0..(6 * 60) {
        let yaw = play.hero.walker.body.yaw;
        play.hero.walker.drive = Some(parkan_sim::wizard::Drive {
            velocity: Vec3::new(-yaw.sin(), yaw.cos(), 0.0) * top,
            heading: Some(yaw),
            flags: parkan_sim::wizard::GROUND_POINT,
        });
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let driven = play.hero.walker.body.position.truncate().distance(start.truncate());
    assert!(keyed < 2.0, "the player gets nowhere up it: {keyed} m in 6 s");
    assert!(driven < 2.0, "and nor does a unit the Wizard drives: {driven} m in 6 s");
}

#[test]
#[ignore = "needs the game install"]
fn a_stone_stops_the_hero_and_a_tree_turns_it_aside() {
    use glam::Vec3;
    let (mut play, m) = mission_01_play();
    // Object 2, s_stone_07 scaled, and object 12, s_tree_04 (docs/24, "Collision between objects").
    let stone = Vec3::from_array(m.objects[2].position) + Vec3::new(60.0, 0.0, 0.0);
    let path = walk(&mut play, stone + Vec3::new(0.0, -120.0, 40.0), 0.0, 15);
    let end = *path.last().unwrap();
    assert!(end.y < stone.y - 80.0, "held off the stone's face, not through it: {end}");
    let moved = path[path.len() - 60].distance(end);
    assert!(moved < 0.5, "and stopped there, {moved} in the last second");

    let tree = Vec3::from_array(m.objects[12].position);
    let path = walk(&mut play, tree + Vec3::new(0.0, -120.0, 40.0), 0.0, 15);
    let end = *path.last().unwrap();
    assert!(end.y > tree.y + 40.0, "past the tree: {end}");
    assert!((end.x - tree.x).abs() > 1.0, "turned aside by its trunk: {end}");
}

#[test]
#[ignore = "needs the game install"]
fn a_lake_bed_kills_the_hero_and_a_warbot_within_a_second() {
    use glam::Vec3;
    use parkan_sim::combat::Event;
    use parkan_sim::damage::Life;
    use parkan_world::play::Play;
    use parkan_world::progress::{STRING_MISSION_FAILED, Say};

    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    let lives = |play: &Play| -> f32 { play.hero.lives.iter().flatten().map(Life::total).sum() };

    // On dry ground at the start, five seconds take nothing.
    let full = lives(&play);
    for _ in 0..300 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(!play.hero.dead() && lives(&play) == full && full > 7000.0, "{full}");

    // Tut_1's lake east of the start: a `WATER_BOT` bed at -10.9 under water at -1.7
    // (docs/24, "Water and lava beds kill").
    let lake = Vec3::new(682.0, 667.0, 20.0);
    assert!(play.ground.water(lake.x, lake.y, 0.0).is_some_and(|z| (z + 1.73).abs() < 0.01));
    let bot = play.robots[0].0;
    put(&mut play.robots[0].1.walker, &play.ground, lake + Vec3::new(8.0, 0.0, 0.0));
    put(&mut play.hero.robot.walker, &play.ground, lake);
    play.says.clear();

    let (mut hero_died, mut bot_died) = (None, None);
    for k in 1..=90 {
        let events = play.tick(tick, [0.0; 2]);
        if play.hero.dead() && hero_died.is_none() {
            hero_died = Some(k as f32 / 60.0);
        }
        if events.iter().any(|e| matches!(e, Event::Killed { target } if *target == bot)) {
            bot_died = Some(k as f32 / 60.0);
        }
    }
    assert!(hero_died.is_some_and(|s| (0.2..=1.15).contains(&s)), "the hero dies in the lake: {hero_died:?}");
    assert!(bot_died.is_some_and(|s| s <= 1.15), "and the warbot: {bot_died:?}");
    assert!(!play.battle.combat.targets[bot].alive && lives(&play) == 0.0);
    // The hero's loss fails the mission: its panel's red title over Esc, R and L (docs/34).
    let panel = play.progression.as_ref().unwrap().panel().expect("an outcome");
    let strings = &play.progression.as_ref().unwrap().strings;
    assert_eq!((panel.won, &panel.title), (false, &strings[&STRING_MISSION_FAILED]));
    assert_eq!(panel.lines.len(), 3);
    assert!(play.says.iter().any(|s| matches!(s, Say::Voice(_))), "VOICE_MISSION_FAIL: {:?}", play.says);

    let at = play.hero.walker.body.position;
    // View state 4 (docs/40): the world is drawn from the level's second camera, placed over
    // where the hero fell, 16 over the highest surface there, level, along the hero's own x
    // axis, with the command camera's field; in single play it does not move.
    let eye = play.eye();
    assert_eq!(play.fallen, Some(eye));
    assert!((eye.position.truncate() - at.truncate()).length() < 0.01, "{:?} over {at:?}", eye.position);
    let under = play.ground.below(at.x, at.y, 1.0e4).map_or(f32::MIN, |h| h.point.z);
    assert!(
        eye.position.z >= under + parkan_world::play::FALLEN_ABOVE - 1e-3,
        "{} over {under}",
        eye.position.z
    );
    let (_, yaw) = play.hero.walker.drawn(play.hero.time_ms);
    assert!((eye.forward - Vec3::new(yaw.cos(), yaw.sin(), 0.0)).length() < 1e-5, "{:?}", eye.forward);
    assert_eq!((eye.up, eye.fov_x), (Vec3::Z, parkan_world::command::FIELD));
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    assert_eq!(play.hero.walker.body.position, at, "a dead hero stays where it died");
    assert_eq!(play.eye(), eye, "and the camera over it holds still");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_buoys_hold_the_hero_off_their_cones() {
    use glam::Vec3;
    let (_, m) = mission_01_play();
    // The five `s_tree_29` buoys; each cone reaches 1.94 from its axis (docs/24, "What a
    // buoy does to a walker"). What the pair pushes out of it is the hero's node sphere, 1.59
    // about a centre 0.17 behind its origin, not its 2.18 agent sphere (docs/24, "Collision
    // between objects"): the hero comes within 2.7 to 2.9 of the axis, where the larger sphere
    // kept it past 3.
    let buoys: Vec<usize> =
        (0..m.objects.len()).filter(|&i| m.objects[i].path.eq_ignore_ascii_case("s_tree_29")).collect();
    assert_eq!(buoys, vec![25, 26, 27, 29, 30]);
    for &buoy in &buoys {
        let b = Vec3::from_array(m.objects[buoy].position);
        let (mut play, _) = mission_01_play();
        play.hero.steady = true;
        let path = walk(&mut play, b + Vec3::new(0.0, -25.0, 40.0), 0.0, 20);
        let closest = path.iter().map(|p| p.truncate().distance(b.truncate())).fold(f32::MAX, f32::min);
        let eye = play.hero.eye().position.truncate().distance(b.truncate());
        assert!(closest > 2.6 && eye > 2.6, "buoy {buoy}: the hero came within {closest}, its eye {eye}");
    }
}

#[test]
#[ignore = "needs the game install"]
fn a_driven_flyers_hull_comes_round_under_its_turret_while_the_lock_holds_and_its_pitch_never_climbs() {
    use parkan_world::factory::Project;
    use std::f32::consts::{PI, TAU};

    const TICK: f64 = 1000.0 / 60.0;
    let wrap = |a: f32| (a + PI).rem_euclid(TAU) - PI;
    let heading = |f: glam::Vec3| (-f.x).atan2(f.y);

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
        play.tick(TICK, [0.0; 2]);
    }
    assert!(!play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.turret_lock);
    assert!(play.board(t));
    // Taken at auto-driver level 0, the bot's turret lock is on; the hero never has one.
    assert!(play.driven().walker.body.turret_lock && !play.hero.walker.body.turret_lock);
    play.key("SCAN_R", true);
    for _ in 0..(2 * 60) {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }
    play.key("SCAN_R", false);
    for _ in 0..60 {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }

    // Mouse Y tilts the sight down and back up, and the height holds.
    let z0 = play.driven().walker.body.position.z;
    let look0 = play.eye().forward.z;
    for (counts, ticks) in [(6.0, 60), (-6.0, 60)] {
        let mut lowest = f32::MAX;
        for _ in 0..ticks {
            play.update_input();
            play.tick(TICK, [0.0, counts]);
            lowest = lowest.min(play.eye().forward.z);
            let z = play.driven().walker.body.position.z;
            assert!((z - z0).abs() < 1e-3, "pitch moved the flyer from {z0} to {z}");
        }
        if counts > 0.0 {
            assert!(lowest < look0 - 0.5, "the sight tilted down: {look0} to {lowest}");
        }
    }

    // Mouse X right: the turret turns right at once, and the hull comes round under it,
    // each step turning no more than 0.7 of the live yaw rate over the step, while the turret
    // holds the heading it was given.
    let rate = play.driven().walker.limits.turn[2];
    let yaw0 = play.driven().walker.body.yaw;
    for _ in 0..3 {
        play.update_input();
        play.tick(TICK, [180.0, 0.0]);
    }
    let body = play.driven().walker.body;
    let aimed = wrap(body.yaw - (body.lead - 0.5) * TAU);
    assert!(wrap(aimed - yaw0) < -2.0, "a right turn of more than two radians: {yaw0} to {aimed}");
    let mut before = body.yaw;
    let mut bounded = 0;
    let mut at_bound = 0;
    for _ in 0..(3 * 60) {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
        let w = &play.driven().walker;
        let step = wrap(w.body.yaw - before);
        before = w.body.yaw;
        assert!(step <= 1e-6, "the hull only turns right: {step}");
        if step != 0.0 {
            let most = 0.7 * rate * (w.machine.step_ms / 1000.0) as f32;
            assert!(-step <= most + 1e-4, "a step of {step} against {most}");
            bounded += 1;
            at_bound += usize::from(-step > most - 1e-4);
        }
        // Whatever the hull has done, the heading the turret was given holds.
        let heading_now = wrap(w.body.yaw - (w.body.lead - 0.5) * TAU);
        assert!(wrap(heading_now - aimed).abs() < 1e-3, "the turret's heading moved to {heading_now}");
    }
    // A gap of more than two radians takes two 250 ms steps, the first at the bound.
    assert!(at_bound >= 1 && bounded >= 2, "{at_bound} of {bounded} turning steps at the bound");
    let body = play.driven().walker.body;
    assert!((body.lead - 0.5).abs() < 1e-3, "the gap closed: {}", body.lead);
    assert!(wrap(body.yaw - aimed).abs() < 0.02, "the hull faces where the turret was aimed");
    let eye = play.eye();
    assert!(wrap(heading(eye.forward) - aimed).abs() < 0.05, "and the sight still looks there");
    // W now flies that way, and looking down on the way never takes it down.
    let a = play.driven().walker.body.position;
    play.key("SCAN_W", true);
    for _ in 0..60 {
        play.update_input();
        play.tick(TICK, [0.0, 3.0]);
    }
    play.key("SCAN_W", false);
    let b = play.driven().walker.body.position;
    assert!(wrap(heading(b - a) - aimed).abs() < 0.05, "flew along {} for {aimed}", heading(b - a));
    assert!(b.z >= a.z - 1e-3, "never down with the sight: {} to {}", a.z, b.z);

    // Keypad 5 lets the turret turn alone, and the spin key turns the hull at 0.7 of the rate.
    play.key("SCAN_G_5", true);
    assert!(!play.driven().walker.body.turret_lock);
    let yaw1 = play.driven().walker.body.yaw;
    for _ in 0..20 {
        play.update_input();
        play.tick(TICK, [12.0, 0.0]);
    }
    for _ in 0..60 {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }
    assert!(wrap(play.driven().walker.body.yaw - yaw1).abs() < 1e-5, "unlocked, the hull stays");
    play.key("SCAN_COMMA", true);
    for _ in 0..30 {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }
    play.key("SCAN_COMMA", false);
    let spun = wrap(play.driven().walker.body.yaw - yaw1);
    assert!(spun > 0.25 * rate && spun <= 0.7 * rate * 0.5 + 0.1, "a left spin of {spun} in 0.5 s");
    // Locked again, the hull swings round to the turret it left behind.
    play.key("SCAN_G_5", true);
    for _ in 0..(4 * 60) {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }
    assert!((play.driven().walker.body.lead - 0.5).abs() < 1e-3);
    // Getting out lets the lock go.
    play.key("SCAN_F", true);
    for _ in 0..(8 * 60) {
        play.update_input();
        play.tick(TICK, [0.0; 2]);
    }
    play.key("SCAN_F", false);
    assert!(play.roll_back());
    let robot = &play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1;
    assert!(!robot.walker.body.turret_lock && robot.walker.body.spin_set == [0.0; 3]);
}

#[test]
#[ignore = "needs the game install"]
fn a_units_live_limits_come_from_its_engine_and_load_so_a_driven_hull_follows_its_turret_at_the_games_rate() {
    use parkan_world::factory::Project;
    use std::f32::consts::{PI, TAU};
    const TICK: f64 = 1000.0 / 60.0;
    let wrap = |a: f32| (a + PI).rem_euclid(TAU) - PI;

    // Mission 03's transport runs at 24 m/s, not its chassis's authored 33.3; the hero, its 10,000 t
    // payload all but spare, keeps 14 less 2 mm/s.
    let (play, m) = mission_03_play();
    let transport = object_target(&play, &m, "tut3_t.dat");
    let robot = &play.robots.iter().find(|(t, _)| *t == transport).unwrap().1;
    let top = robot.walker.limits.top_speed[1];
    assert!((top - 23.98).abs() < 0.01, "the transport's live top speed {top}");
    let hero = play.hero.walker.limits.top_speed[1];
    assert!((hero - 13.998).abs() < 1e-3, "the hero's {hero}");

    // Aboard, the hull comes round at 0.7 of the live yaw rate: docs/30's worked 1.89, 1.07 and
    // 1.32 rad/s on Mission 02's warbot, Mission 04's HQ and its helicopter.
    for (path, want) in [
        ("UNITS\\UNITS\\PREBLD\\tut2_f1.dat", 1.889),
        ("UNITS\\UNITS\\HQ\\tut4_hq.dat", 1.071),
        ("UNITS\\UNITS\\BATTLE\\tut4_f1.dat", 1.320),
    ] {
        let (mut play, _) = mission_02_play();
        let project = Project {
            path: path.to_owned(),
            name: String::new(),
            type_word: 0x0100_8000,
            chassis_size: 4,
            ore: 0.0,
            power: 0.0,
            lines: Vec::new(),
            sphere: None,
        };
        let at = play.hero.walker.body.position + glam::Vec3::new(12.0, 0.0, 3.0);
        let t = play.spawn(&project, play.player_clan, at, 0.0).unwrap();
        for _ in 0..30 {
            play.tick(TICK, [0.0; 2]);
        }
        let r = play.robots.iter().position(|(rt, _)| *rt == t).unwrap();
        // The helicopter is tiny; boarding it stands in for taking it over from command mode.
        play.robots[r].1.size_class = 4;
        assert!(play.board(t), "{path}");
        for _ in 0..3 {
            play.update_input();
            play.tick(TICK, [180.0, 0.0]);
        }
        let mut before = play.driven().walker.body.yaw;
        let mut rates = Vec::new();
        for _ in 0..60 {
            play.update_input();
            play.tick(TICK, [0.0; 2]);
            let w = &play.driven().walker;
            let step = wrap(w.body.yaw - before);
            before = w.body.yaw;
            if step != 0.0 {
                rates.push(step.abs() / (w.machine.step_ms as f32 / 1000.0));
            }
        }
        assert!(rates.len() >= 2 && (rates[0] - want).abs() < 0.005, "{path}: {rates:?} against {want}");

        // The engine's node at half its life halves the live yaw rate the tick after.
        let turn = play.driven().walker.limits.turn[2];
        let chassis = play.driven().chassis_part;
        let life = play.battle.combat.targets[t].parts[chassis].life.as_mut().unwrap();
        life.nodes[0].life = life.nodes[0].max / 2.0;
        play.tick(TICK, [0.0; 2]);
        let halved = play.driven().walker.limits.turn[2];
        assert!((halved - turn / 2.0).abs() < 1e-3, "{path}: {turn} to {halved}");
    }
}

/// A contact point sits on the node its first triple's second slot names; the third slot
/// is the node it dies with, a damage reference and not a frame (docs/24, "A contact point
/// sits on one node and dies with another"). A wheeled chassis authors its `weel_*` points
/// on the body, at the tyres, so its wheels land on the ground; posing them on the wheel
/// node instead drops them clear under the tyres and the whole bot visibly hovers.
#[test]
#[ignore = "needs the game install"]
fn c03_02s_wheeled_warbots_stand_on_their_tyres_their_contacts_being_authored_there() {
    use parkan_formats::control::CONTACT_SUPPORT;
    use parkan_sim::machine::Frames;

    let mut play = campaign_play("MISSIONS/CAMPAIGN/CAMPAIGN.03/Mission.02");
    play_for(&mut play, 1.0, |_| {});

    let mut heavies = 0;
    for (_, robot) in &play.robots {
        let w = &robot.walker;
        let Some(feet) = &w.feet else { continue };
        let record = &robot.parts[robot.chassis_part].record;
        if !record.starts_with("R_B_03") && !record.starts_with("R_M_04") {
            continue;
        }
        let state = &w.controller.states[w.machine.current];
        let last = Frames { a: state.pair_a[1], b: state.pair_b[1], weight: w.machine.q };
        let placed: Vec<glam::Vec3> = state
            .contacts
            .iter()
            .filter(|c| c.flags & CONTACT_SUPPORT != 0)
            .filter_map(|c| feet.place(c.point, last))
            .collect();
        assert!(!placed.is_empty(), "{record} has support contacts");

        // The tyres: the contacts sit on them, not under them. Posed on the wheel node
        // instead, R_B_03's lowest lands 2.23 below its own lowest vertex.
        let lowest = placed.iter().map(|p| p.z).fold(f32::MAX, f32::min);
        assert!((lowest - -w.base).abs() < 0.05, "{record} contacts at {lowest}, tyres at {}", -w.base);

        // And inside the hull, which is what the wrong node breaks most plainly: it threw
        // R_B_03's front-right contact out to x 6.24 on a hull 8.02 wide.
        let half_width = placed.iter().map(|p| p.x.abs()).fold(0.0, f32::max);
        assert!(half_width < 4.01, "{record} contact {half_width} off centre, hull half-width 4.01");

        if record == "R_B_03" {
            let p = w.body.position;
            let under = play.ground.below(p.x, p.y, p.z + w.sphere_radius).expect("ground below it");
            // Level ground puts it on its tyres; broken ground holds it at the highest
            // wheel, which measured up to 0.67 across the campaign, never the 2.23 the
            // wheel node's frame gave it everywhere.
            let clear = p.z - w.base - under.point.z;
            assert!((-0.1..1.0).contains(&clear), "{record} rides {clear} clear");
            heavies += 1;
        }
    }
    assert_eq!(heavies, 2, "Mission C03/02 stands two R_B_03");
}

/// A walker's feet ask for `CONTACT_PLACE` to be worked out from the state's own pose
/// (docs/24, "A walker's feet lie flat where the animation lays them"). The hero's
/// `r_h_02` authors none of its 210 contacts with `CONTACT_PLACE` and every one with
/// `CONTACT_PLACE_BY_POSE`; the pose gives it to 190 of them and withholds it from the 20
/// whose foot is on its side at the end of the step.
#[test]
#[ignore = "needs the game install"]
fn the_heros_feet_place_in_the_states_whose_last_pose_stands_them_up_and_not_in_the_rest() {
    use parkan_formats::control::{CONTACT_PLACE, CONTACT_PLACE_BY_POSE};
    use parkan_formats::{landmesh, mission};
    use parkan_sim::ground::Ground;
    use parkan_world::{assembly::Assembly, hero::Hero};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut assembly = Assembly::new(&game).unwrap();
    let mut hero = Hero::load(&mut assembly, &m).unwrap().expect("Mission 01 has a hero");
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    hero.tick(1000.0 / 60.0, [0.0; 2], &Ground::new(land));

    let contacts: Vec<u32> =
        hero.walker.controller.states.iter().flat_map(|s| s.contacts.iter().map(|c| c.flags)).collect();
    assert_eq!(contacts.len(), 210, "r_h_02: two feet on 105 states");
    assert!(contacts.iter().all(|f| f & CONTACT_PLACE_BY_POSE != 0), "every one asks the pose");
    let placed = contacts.iter().filter(|f| *f & CONTACT_PLACE != 0).count();
    assert_eq!(placed, 190, "the pose stands 190 of the 210 up");
}

/// A contact whose flags carry `CONTACT_PLACE` lays the node it carries along the ground
/// under it (docs/28, "The belt lies along the ground"). Twelve contacts in the game do:
/// the four `weel_*` of each tracked chassis, whose carriers are the belt nodes. So a
/// Medium Track warbot parked across a slope tilts its four belts onto it and leaves its
/// hull where it stood.
#[test]
#[ignore = "needs the game install"]
fn a_tracked_warbots_belts_lie_along_the_ground_under_them_and_its_hull_does_not_move() {
    use parkan_formats::control::CONTACT_PLACE;
    use parkan_formats::pose::{Pose, multiply};
    use parkan_sim::machine::Frames;
    use std::f32::consts::FRAC_PI_2;

    let mut play = campaign_play(gamedir::C01_MISSION_04);
    play_for(&mut play, 1.0, |_| {});
    let r = play
        .robots
        .iter()
        .position(|(_, rb)| rb.parts[rb.chassis_part].record.starts_with("R_M_04"))
        .expect("C01 Mission 04 places two Medium Track warbots");

    // The four contacts that ask for it, and the nodes they carry. The M-42t's `weel_*`
    // points all sit on node 0, the hull, and each carries a belt node of its own.
    let carriers: Vec<usize> = {
        let w = &play.robots[r].1.walker;
        let feet = w.feet.as_ref().expect("a tracked chassis has contact points");
        let placing: Vec<i32> = w.controller.states[w.machine.current]
            .contacts
            .iter()
            .filter(|c| c.flags & CONTACT_PLACE != 0)
            .map(|c| c.point)
            .collect();
        assert_eq!(placing.len(), 4, "the M-42t's four belts");
        assert!(placing.iter().all(|&p| feet.points[p as usize].nodes().0 == 0), "all on the hull");
        placing.iter().map(|&p| feet.carrier(p).unwrap()).collect()
    };
    let mut sorted = carriers.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 4, "four distinct belt nodes: {carriers:?}");
    assert!(!carriers.contains(&0), "none of them is the hull");

    // Somewhere sloped to park it: a face between 15 and 35 degrees off level.
    let ([lo_x, lo_y], [hi_x, hi_y]) = play.ground.bounds();
    let top = play.ground.world_box().1.z + 10.0;
    let mut slope = None;
    let mut x = lo_x + 20.0;
    while x < hi_x - 20.0 && slope.is_none() {
        let mut y = lo_y + 20.0;
        while y < hi_y - 20.0 {
            if let Some(hit) = play.ground.below(x, y, top)
                && (0.82..0.97).contains(&hit.normal.z)
            {
                slope = Some(glam::Vec3::new(x, y, hit.point.z));
                break;
            }
            y += 8.0;
        }
        x += 8.0;
    }
    let at = slope.expect("Mission 04's map has a slope");

    let turn_of = |robot: &mut parkan_world::robot::Robot, node: usize| {
        let tilted = robot.chassis_pose(node);
        let held = std::mem::take(&mut robot.walker.placed);
        let level = robot.chassis_pose(node);
        robot.walker.placed = held;
        let [w, i, j, k] = level.rotation;
        (tilted, level, multiply(tilted.rotation, [w, -i, -j, -k]))
    };

    for &yaw in &[0.0, FRAC_PI_2] {
        // Steps, not ticks, run the ground contact: a second of them to settle, then one
        // more from a standstill, whose start is where the contacts were searched from.
        let stood = {
            let (robots, ground) = (&mut play.robots, &play.ground);
            let robot = &mut robots[r].1;
            let w = &mut robot.walker;
            w.body.yaw = yaw;
            w.drive = None;
            put(w, ground, at);
            w.advance(w.machine.clock_ms + 1000.0, ground);
            (w.body.velocity, w.body.command, w.body.spin) = ([0.0; 3], [0.0; 3], [0.0; 3]);
            let stood = w.body.position;
            w.advance(w.machine.clock_ms, ground);
            robot.time_ms = robot.walker.machine.step_start_ms;
            stood
        };
        assert_eq!(play.robots[r].1.walker.placed.len(), 4, "four belts laid on the ground");

        let hull = play.robots[r].1.chassis_pose(0);
        let mut steepest = 0.0_f32;
        for (i, &node) in carriers.iter().enumerate() {
            // The ground under this contact, in the machine's own frame.
            let (axis, normal) = {
                let w = &play.robots[r].1.walker;
                let feet = w.feet.as_ref().unwrap();
                let state = &w.controller.states[w.machine.current];
                let last = Frames { a: state.pair_a[1], b: state.pair_b[1], weight: w.machine.q };
                let point = state.contacts.iter().filter(|c| c.flags & CONTACT_PLACE != 0).nth(i).unwrap();
                let place = stood + w.body.to_world(feet.place(point.point, last).unwrap());
                let hit = play.ground.search(place, w.radius).expect("ground under a belt");
                (feet.axis(point.point, last).unwrap(), glam::Quat::from_rotation_z(-yaw) * hit.normal)
            };
            let robot = &mut play.robots[r].1;
            let (tilted, level, turn) = turn_of(robot, node);

            // The node stays where it stood -- only its attitude changes.
            for (a, b) in tilted.translation.iter().zip(level.translation) {
                assert!((a - b).abs() < 1e-9, "belt {node} moved: {tilted:?} against {level:?}");
            }
            // And the turn is the one that takes the contact's own axis onto the ground's
            // normal: the belt lies along the slope under it.
            let turned = Pose { translation: [0.0; 3], rotation: turn }.apply(axis.as_dvec3().to_array());
            let turned = glam::Vec3::new(turned[0] as f32, turned[1] as f32, turned[2] as f32);
            assert!((turned - normal).length() < 1e-4, "belt {node}: {turned:?} against {normal:?}");
            steepest = steepest.max(axis.dot(normal).clamp(-1.0, 1.0).acos().to_degrees());
        }
        assert!(steepest > 10.0, "the ground under the belts is only {steepest} degrees off level");
        // The hull is posed exactly as it was: no contact carries node 0.
        assert_eq!(play.robots[r].1.chassis_pose(0), hull);
    }
}

#[test]
#[ignore = "needs the game install"]
fn mission_04s_helicopter_rides_up_a_slope_on_its_body_sphere_with_its_eye_above_the_ground() {
    const TICK: f64 = 1000.0 / 60.0;
    let (mut play, _) = mission_04_play();
    let r = play.robots.iter().position(|(t, _)| play.units[*t].logical_id == 3).expect("tut4_f1");
    let t = play.robots[r].0;
    {
        let heli = &play.robots[r].1;
        // The body sphere: the agent's sphere's radius, the chassis's, the hung turret's and its
        // two guns' header spheres joined, 2.45, about the node sphere's centre, 0.60 below the
        // origin (docs/24, "Finding the ground").
        assert!((heli.walker.radius - 2.4456).abs() < 1e-3, "{}", heli.walker.radius);
        assert!((heli.walker.centre.z + 0.595).abs() < 1e-3, "{:?}", heli.walker.centre);
    }
    // Taken over as from command mode: the player's, boarded where it stands.
    play.units[t].clan = Some(play.player_clan);
    play.robots[r].1.size_class = 4;
    let at = play.robots[r].1.walker.body.position;
    play.hero.walker.body.position = at + glam::Vec3::new(5.0, 0.0, 0.0);
    for _ in 0..30 {
        play.tick(TICK, [0.0; 2]);
    }
    assert!(play.board(t));
    // W toward the valley's west slope for 12 s at 14 m/s: the helicopter rides 57 m up it, and its
    // eye, 1.6 m under the origin in the hung turret, keeps 0.9 m or more over the ground. On the
    // chassis's own sphere it dipped 0.4 m under.
    let z0 = play.driven().walker.body.position.z;
    let mut lowest = f32::MAX;
    play.key("SCAN_W", true);
    for _ in 0..(12 * 60) {
        play.update_input();
        play.tick(TICK, [0.0, 1.0]);
        let eye = play.eye().position;
        let ground = play.ground.below(eye.x, eye.y, eye.z + 50.0).map_or(f32::MIN, |h| h.point.z);
        lowest = lowest.min(eye.z - ground);
    }
    play.key("SCAN_W", false);
    let climbed = play.driven().walker.body.position.z - z0;
    assert!(climbed > 50.0, "rode up {climbed}");
    assert!(lowest > 0.5, "the eye stays above the ground: at least {lowest}");
}

/// The Iron Monster's valley is cut by a canyon, and one bridge crosses it. The canyon floor
/// and walls are not walkable (their areals' first flag word is 0), so the walker's global path
/// from the south bank to the north goes over the bridge's deck, half to half, and a goal on the
/// canyon floor is refused. A warbot sent across keeps out of the canyon, where a straight walk
/// took it down before (docs/24, "The global path").
#[test]
#[ignore = "needs the game install"]
fn c02_m01s_warbots_cross_the_canyon_by_its_bridge_and_are_never_sent_down_into_it() {
    use glam::Vec3;
    use parkan_sim::orders::{self, Order, Target};
    use parkan_sim::path::{VERTEX_EXIT, VERTEX_JOIN};

    let mut play = campaign_play(gamedir::C02_MISSION_01);
    let graph = play.graph.clone().expect("KM_4 has an areal map");
    let north = Vec3::new(433.0, 780.0, 47.0);
    let floor = Vec3::new(427.0, 645.0, 18.0);
    assert!(graph.usable(502.0, 551.0) && graph.usable(north.x, north.y), "both banks are walkable");
    assert!(!graph.usable(floor.x, floor.y), "the canyon floor is not");

    // The bridge's two halves: an exit on walkable ground at either bank, and ends that join.
    let ways = play.bridge_ways();
    assert_eq!(ways.len(), 2);
    let flagged = |w: &parkan_sim::path::Way, flag: u32| -> Vec<Vec3> {
        (0..w.points.len()).filter(|&v| w.flags[v] & flag != 0).map(|v| w.points[v]).collect()
    };
    for (_, way) in &ways {
        let exits = flagged(way, VERTEX_EXIT);
        assert!(exits.iter().any(|p| graph.usable(p.x, p.y)), "an exit on a bank: {exits:?}");
    }
    let (a, b) = (flagged(&ways[0].1, VERTEX_JOIN), flagged(&ways[1].1, VERTEX_JOIN));
    assert!(a.iter().any(|p| b.iter().any(|q| p.distance(*q) < 50.0)), "the halves join: {a:?} {b:?}");
    let deck: Vec<Vec3> = ways.iter().flat_map(|(_, w)| w.points.clone()).collect();

    // `21mwlk1`, logical id 24, stands on the south bank.
    let t = play.units.iter().position(|u| u.logical_id == 24).expect("unit 24");
    let at = |play: &parkan_world::play::Play| {
        play.robots.iter().find(|(rt, _)| *rt == t).expect("a robot").1.walker.body.position
    };
    let from = at(&play);
    assert!(from.y < 600.0 && graph.usable(from.x, from.y), "{from}");
    let legs = play.route(t, from, north);
    assert_eq!(legs.last(), Some(&north));
    assert!(legs.iter().any(|p| deck.contains(p)), "over the bridge: {legs:?}");
    let mut start = from;
    for &end in &legs {
        if !(deck.contains(&start) && deck.contains(&end)) {
            for s in 0..=50 {
                let p = start.lerp(end, s as f32 / 50.0);
                assert!(graph.usable(p.x, p.y), "{p} off walkable ground, from {start} to {end}: {legs:?}");
            }
        }
        start = end;
    }
    assert!(play.route(t, from, floor).is_empty(), "a goal on the canyon floor is refused");

    // Sent across, it walks over the deck and never down into the canyon, some 30 m below.
    let order = Order { code: orders::GO, parameter: 0, target: Target::Place(north.to_array()) };
    let robot = &mut play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1;
    assert!(robot.behaviour.insert_order(&order, orders::INSERT_REPLACE));
    robot.order = Some(order);
    let (mut lowest, mut crossed) = (f32::MAX, false);
    for _ in 0..(90 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let p = at(&play);
        lowest = lowest.min(p.z);
        crossed |= (620.0..700.0).contains(&p.y) && p.z > 55.0;
    }
    let end = at(&play);
    assert!(crossed, "over the deck");
    assert!(lowest > 35.0, "never below the banks: {lowest}");
    assert!(end.truncate().distance(north.truncate()) < 30.0, "and on to its goal: {end}");
}

/// Mission.02's laser walker, `mwlk1e`, logical id 10, patrols past `s_stone_07` in the middle of
/// the map, a stone whose box is some 69 by 56 m across on ground the areal map leaves walkable. The areal map
/// cuts each tree's and stone's footprint, its mesh's box's lower corners, out of the walkable
/// areals (`ArealMap.dll:0x10022580`), and the walker's path goes round it; on the areal map alone
/// its path crossed the stone, and the walker climbed at it and slid back for most of a minute.
#[test]
#[ignore = "needs the game install"]
fn c02_m02s_laser_walker_walks_round_the_central_stone_and_never_climbs_it() {
    use glam::{Vec2, Vec3};
    use parkan_formats::{arealmap, mission};

    let mut play = campaign_play(gamedir::C02_MISSION_02);
    let graph = play.graph.clone().expect("Mission.02 has an areal map");
    let stone = (0..play.units.len())
        .filter(|&t| play.units[t].kind == mission::KIND_ROCK)
        .any(|t| play.battle.combat.targets[t].position.truncate().distance(Vec2::new(1026.4, 1031.1)) < 1.0);
    assert!(stone, "the central stone");
    // Its mesh's box stands off its placement, to the south-west.
    let middle = Vec2::new(997.0, 1010.0);
    assert!(graph.usable(middle.x, middle.y), "on walkable ground");
    assert!(graph.in_footprint(middle.x, middle.y), "cut out of it");
    // How far inside a footprint `p` stands: the nearest ring about it with a point outside.
    let depth = |p: Vec3| {
        [0.0f32, 0.5, 1.0, 1.5, 2.0, 4.0, 8.0, 16.0, 32.0].into_iter().find(|&r| {
            (0..32).any(|k| {
                let a = k as f32 / 32.0 * std::f32::consts::TAU;
                !graph.in_footprint(p.x + r * a.cos(), p.y + r * a.sin())
            })
        })
    };
    let crosses = |from: Vec3, legs: &[Vec3]| {
        let mut start = from;
        legs.iter().any(|&end| {
            let inside = (0..=100).any(|s| {
                let p = start.lerp(end, s as f32 / 100.0);
                p.truncate().distance(middle) < 60.0 && depth(p) != Some(0.0)
            });
            start = end;
            inside
        })
    };

    let t = play.units.iter().position(|u| u.logical_id == 10).expect("unit 10");
    let at = |play: &parkan_world::play::Play| {
        play.robots.iter().find(|(rt, _)| *rt == t).expect("a robot").1.walker.body.position
    };
    let from = at(&play);
    let goal = Vec3::new(1048.0, 526.0, 0.0);
    let legs = play.route(t, from, goal);
    assert!(!legs.is_empty() && !crosses(from, &legs), "round the stone: {legs:?}");
    let carved = play.graph.take();
    let game = gamedir::find(None).unwrap();
    let data = gamedir::resolve(&game, gamedir::C02_MISSION_02).unwrap().join("data.tma");
    let map_path = mission::parse(&std::fs::read(data).unwrap(), "Mission.02").unwrap().map_path;
    let dir = parkan_world::terrain::map_dir(&game, &map_path).unwrap();
    let land = arealmap::load(&gamedir::resolve(&dir, "Land.map").unwrap()).unwrap();
    play.graph = Some(parkan_sim::path::Graph::new(land));
    let uncut = play.route(t, from, goal);
    assert!(crosses(from, &uncut), "on the areal map alone, over it: {uncut:?}");
    play.graph = carved;

    // On its patrol it passes the stone within 25 s, never more than a stride's curve inside a
    // footprint.
    let tick = 1000.0 / 60.0;
    let mut deepest = 0.0f32;
    for _ in 0..25 {
        for _ in 0..60 {
            play.tick(tick, [0.0; 2]);
        }
        let p = at(&play);
        deepest = deepest.max(depth(p).unwrap_or(f32::MAX));
    }
    let end = at(&play);
    assert!(end.y < 950.0, "past the stone: {end}");
    assert!(deepest <= 1.5, "{deepest} m inside a footprint");
}
