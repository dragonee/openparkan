//! A base at work: factories and their doors, research, the builder, a mine, its ore and
//! the outpost.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn mission_02s_factory_pod_captures_the_factory_and_opens_its_screen() {
    use parkan_world::play::Mode;
    use parkan_world::progress::{Say, Sender};

    let (mut play, _) = mission_02_play();
    // The pod, node 25 of fr_b_plant, over the pod room's floor 12.4 below the entrance
    // (docs/24, "Walking into a building").
    let w = &mut play.hero.walker;
    w.body.position = glam::Vec3::new(391.28, 740.08, 146.0);
    w.follow_ground(&play.ground);
    w.from = (w.body.position, w.body.yaw);
    let factory = play.buildings.iter().position(|b| b.doors.len() == 3).unwrap();
    let t = play.buildings[factory].target;
    let (mut tick, mut says) = (0, Vec::new());
    while play.mode() == Mode::OnFoot && tick < 600 {
        play.hero.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        says.append(&mut play.says);
        tick += 1;
    }
    eprintln!(
        "hero at {:?} on {:?}",
        play.hero.walker.body.position,
        play.hero.walker.ground.and_then(|h| h.solid)
    );
    // It opens in 4.5 s at a rate of 0.2, and fires then (docs/27's table).
    let seconds = tick as f32 / 60.0;
    assert!((4.4..4.8).contains(&seconds), "the pod fired at {seconds} s");
    assert_eq!(play.mode(), Mode::Factory(t));
    assert_eq!(play.units[t].clan, Some(play.player_clan));
    assert!(
        says.iter().any(|s| matches!(s, Say::Text(Sender::System, text) if text == "Building is captured")),
        "{says:?}"
    );
    let p = play.progression.as_mut().unwrap();
    assert_eq!(p.progress.owner(0x8000_0001_u32 as i32), 0);
}

/// Mode 5 shows the cursor as command mode does, and in view state 1 the pick answers
/// nothing, so it is `ARROW`, drawn last over the screen's text and over the designer's too.
/// The designer's opening replaces the view's flag word, the infrared's `0x20` with it
/// (docs/36, "The cursor in mode 5", "The designer does not pause the world").
#[test]
#[ignore = "needs the game install"]
fn the_factory_screen_and_its_designer_draw_the_arrow_last_and_the_designer_puts_out_the_night_sight() {
    use glam::Mat4;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Layer, Pages, Space};
    use parkan_world::play::Mode;
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, _) = mission_02_play();
    let factory = play.buildings.iter().position(|b| b.doors.len() == 3).unwrap();
    let t = play.buildings[factory].target;
    play.units[t].clan = Some(play.player_clan);
    play.modes.push(Mode::Factory(t));
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let arrow_page = *cockpit.pages.get("new_ui1").expect("the cursors' page");
    // Left over from command mode: the screen's own pick decides, not this.
    cockpit.commander.cursor_state = 2;
    cockpit.commander.cursor = Some([200.0, 100.0]);
    let last_is_the_arrow = |drawn: &parkan_world::cockpit::Drawn| {
        let top: Vec<_> = drawn.batches.iter().filter(|b| b.layer == Layer::OverText).collect();
        let last = drawn.batches.last().unwrap();
        top.len() == 1
            && last.layer == Layer::OverText
            && last.vertices.len() == 6
            && last.vertices.iter().all(|v| v.page == Some(arrow_page) && v.uv[1] <= 16.0)
    };
    cockpit.update(&mut play, 0.0);
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, Mat4::IDENTITY);
    assert!(!drawn.text.is_empty(), "the screen's words");
    assert!(last_is_the_arrow(&drawn), "the factory screen ends in ARROW's quad on new_ui1's top row");

    play.hero.pilot.switches.infrared = true;
    cockpit.designer.open(&mut play, t, &cockpit.strings).unwrap();
    assert!(!play.hero.pilot.switches.infrared, "the designer's 9 leaves no 0x20 in the hero's view");
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, Mat4::IDENTITY);
    assert!(last_is_the_arrow(&drawn), "over the designer too");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_factory_door_opens_for_the_hero_on_its_forecourt_and_shuts_after_it_leaves() {
    use parkan_world::buildings::Phase;

    let (mut play, _) = mission_02_play();
    assert!(play.stand_at(395.8, 915.0, 3.117), "north of the entrance, facing it");
    let b = play.buildings.iter().position(|b| b.doors.len() == 3).expect("fr_b_plant's three doors");
    let t = play.buildings[b].target;
    let node = play.buildings[b].doors[0].nodes[0];
    let z0 = play.battle.combat.targets[t].parts[play.buildings[b].part].nodes[node].translation[2];
    play.hero.key("SCAN_W", true);
    let (mut held_at, mut open_at) = (None, None);
    for tick in 0..400 {
        play.hero.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let d = &play.buildings[b].doors[0];
        if d.held && held_at.is_none() {
            held_at = Some(tick);
            assert_eq!(
                play.hero.walker.ground.and_then(|h| h.solid).map(|s| s.0),
                Some(t),
                "on the forecourt"
            );
        }
        if d.phase == Phase::Open && open_at.is_none() {
            open_at = Some(tick);
        }
    }
    // A rate of 0.4 opens in 2.5 s, its word clearing at 0.9 / 0.4 = 2.25 s after the first
    // step starts, which waits up to 100 ms.
    let (held, open) = (held_at.expect("the hero holds the door"), open_at.expect("the door opens"));
    let seconds = (open - held) as f32 / 60.0;
    assert!((2.2..2.45).contains(&seconds), "open {seconds} s after it was held");
    let z1 = play.battle.combat.targets[t].parts[play.buildings[b].part].nodes[node].translation[2];
    assert!(z0 - z1 > 10.0, "the entrance door sinks: {z0} to {z1}");
    // Walked back out, it closes 5 s after it opened.
    play.hero.key("SCAN_W", false);
    assert!(play.stand_at(395.8, 940.0, 0.0));
    let mut shut_at = None;
    for tick in 0..900 {
        play.hero.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if play.buildings[b].doors[0].phase == Phase::Shut && shut_at.is_none() {
            shut_at = Some(tick);
        }
    }
    assert!(shut_at.is_some(), "the door shuts once free");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_factory_builds_a_free_warbot_which_leaves_by_the_front_door_and_completes_the_objective() {
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let path = "UNITS\\bld_unit_-2147483647.dat";
    let data =
        std::fs::read(gamedir::resolve(&game, path).expect("the install's design for Tut_2's factory"))
            .unwrap();
    let type_word = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let f =
        play.factories.iter().position(|f| f.logic_id == 0x8000_0001_u32 as i32).expect("the Large Factory");
    assert_eq!((play.factories[f].size, play.factories[f].free_bots), (4, 100));
    let project = Project {
        path: path.to_owned(),
        name: "LFW-2 Warrior".into(),
        type_word,
        chassis_size: 4,
        ore: 411.0,
        power: 226.5,
        lines: Vec::new(),
        sphere: None,
    };
    let t = play.factories[f].target;
    play.units[t].clan = Some(play.player_clan);
    play.factories[f].accept(project);
    let free = play.free_minds(play.player_clan);
    assert_eq!(free, 1, "two minds, the hero holds one");
    assert!(play.start_factory(f, true));
    assert_eq!(play.free_minds(play.player_clan), 0);
    let robots = play.robots.len();
    let mut ticks = 0;
    while play.robots.len() == robots && ticks < 70 * 60 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        ticks += 1;
    }
    let seconds = ticks as f32 / 60.0;
    assert!((59.9..60.2).contains(&seconds), "built in {seconds} s");
    let (unit, robot) = play.robots.last().unwrap();
    assert_eq!(play.units[*unit].clan, Some(play.player_clan));
    let (at, flyer) = (robot.walker.body.position, robot.flyer);
    assert!((at.truncate() - glam::Vec2::new(393.75, 854.91)).length() < 1.0, "at the creation vertex: {at}");
    assert!(matches!(robot.behaviour.task(), parkan_sim::behaviour::Task::Leave { .. }));

    // The escape is routed out along the factory's own paths (docs/31, "The escape"): the
    // hall way from the creation vertex north through the front door, at world y 854.9,
    // 873.4 just inside the door, and 885.4 on the forecourt outside it.
    let bot = play.robots.len() - 1;
    let way = play.way_out(t, at, glam::Vec3::new(535.7, 727.9, 154.0), flyer).expect("a way out");
    let ys: Vec<f32> = way.iter().map(|p| p.y).collect();
    assert!(way.len() == 5 && ys[0] < 855.0 && ys[3] > 885.0, "out through the front door: {way:?}");

    // Walking it: the front door (node 3) opens for the bot as it nears it, no shot fired,
    // and its shut faces hold the bot inside until it has (docs/24, "Walking into a
    // building"). The doorway stands at about y 876.
    const DOORWAY_Y: f32 = 876.0;
    let door = |play: &parkan_world::play::Play| {
        play.buildings.iter().find(|b| b.target == t).expect("the factory's doors").doors[0].phase
    };
    let mut opening_at = None;
    let mut opened_at = None;
    let mut out_at = None;
    for tick in 0..(12 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let phase = door(&play);
        let robot = &play.robots[bot].1;
        let at = robot.walker.body.position;
        if phase == parkan_world::buildings::Phase::Opening && opening_at.is_none() {
            opening_at = Some((tick as f32 / 60.0, at.y));
        }
        if phase == parkan_world::buildings::Phase::Open && opened_at.is_none() {
            opened_at = Some(tick as f32 / 60.0);
        }
        if opened_at.is_none() {
            assert!(at.y < DOORWAY_Y, "the shut door holds it in: {at} at tick {tick}");
        }
        if out_at.is_none() && at.y > DOORWAY_Y && robot.walker.ground.and_then(|h| h.solid).is_none() {
            out_at = Some(tick as f32 / 60.0);
        }
    }
    // The door starts opening once the bot's sphere reaches the leaf's capsule, which runs
    // across the doorway at mid-height 7.7 wide (`IJointMesh` slot 5, docs/24, "Walking into a
    // building"): not from the creation vertex 21 m in, as the leaf's 13.5 sphere had it, but
    // a few metres on. The door is rate 0.4, and the building sees it open at 0.9 / rate,
    // 2.25 s later (docs/27, "Capture"). The bot is through it and back on the landscape
    // within four seconds of that.
    let (opening, from_y) = opening_at.expect("the front door starts opening for the bot");
    let opened = opened_at.expect("the front door opens for the bot");
    let out = out_at.expect("the bot walks out onto the landscape");
    assert!(opening > 0.0 && from_y > 855.5, "held from {from_y} at {opening} s, not the creation vertex");
    assert!(
        (2.2..2.4).contains(&(opened - opening)),
        "the door opens {} s after it starts",
        opened - opening
    );
    assert!(out > opened && out - opened < 4.0, "out {out} s after the door opened at {opened} s");

    assert_eq!(play.factories[f].free_bots, 99);
    // Batch starts no second bot: no mind is free.
    assert!(play.factories[f].build.is_none());
    // "Build a warbot" completes on a Mission run: Plr robots = 2 (docs/34, "Mission 02").
    for _ in 0..(3 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[1].state, 1, "{:?}", p.progress.objectives);
}

#[test]
#[ignore = "needs the game install"]
fn a_shot_at_mission_02s_factory_door_passes_the_black_doorway_in_front_of_it_and_opens_it() {
    use glam::Vec3;
    use parkan_sim::combat::Event;
    use parkan_world::buildings::Phase;

    let (mut play, _) = mission_02_play();
    let b = play.buildings.iter().position(|b| b.doors.len() == 3).expect("fr_b_plant's three doors");
    let t = play.buildings[b].target;
    let part_index = play.buildings[b].part;
    let node = play.buildings[b].doors[0].nodes[0];
    // The entrance door `i05` stands between two black `DEFAULT` quads, one on the outer node
    // `o01` and one in the hall (docs/24, "The doorways are black quads"). A round passes
    // them, as a walker does, and strikes the door.
    let door = door_centre(&play, t, part_index, node);
    let muzzle = Vec3::new(door.x, door.y + 40.0, door.z);
    let (strike, struck, _) =
        play.battle.combat.first_hit(&play.ground, None, muzzle, door, 0.0).expect("a line to the door");
    assert_eq!((strike.node, struck), (Some(node), Some(t)), "the round reaches the door");
    let laser = play.hero.robot.rounds[2].expect("the battle laser's round");
    let direction = (door - muzzle).normalize();
    play.battle.combat.fire(laser, None, muzzle, direction, Vec3::ZERO, 1.0, None).unwrap();
    let mut struck_at = None;
    let mut open_at = None;
    for tick in 0..(60 * 15) {
        for e in play.tick(1000.0 / 60.0, [0.0; 2]) {
            if matches!(e, Event::Struck { target: Some(x), node: Some(n), .. } if x == t && n == node) {
                struck_at.get_or_insert(tick);
            }
        }
        if play.buildings[b].doors[0].phase == Phase::Open {
            open_at.get_or_insert(tick);
        }
    }
    let struck = struck_at.expect("the round strikes the door, not the doorway quad");
    let open = open_at.expect("the door opens");
    assert!(open > struck, "the shot opened it: struck {struck}, open {open}");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_outpost_charges_repairs_rearms_and_shields_the_hero_standing_on_its_dock() {
    use parkan_sim::damage::{Life, share_loss};
    use parkan_world::cockpit::panels::life_share;
    use parkan_world::places::PLACE_DOCK;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    // Half the hero's life gone, its shield half spent, one sector wholly, and its first gun
    // empty.
    let hurt = |play: &mut parkan_world::play::Play| {
        let full: f32 = play.hero.lives.iter().flatten().map(Life::full).sum();
        let mut lives: Vec<&mut Life> = play.hero.lives.iter_mut().flatten().collect();
        let left: f32 = lives.iter().map(|l| l.total()).sum();
        share_loss(&mut lives, left - full * 0.5);
        play.hero.guns[0].rounds = 0;
        let shield = play.battle.combat.hero.as_mut().and_then(|h| h.shield.as_mut()).unwrap();
        shield.fills = [0.0, 1.0, 0.5, 0.5, 0.5, 0.5];
    };
    let life = |play: &parkan_world::play::Play| life_share(play.hero.lives.iter().flatten());
    let shield = |play: &parkan_world::play::Play| {
        play.battle.combat.hero.as_ref().and_then(|h| h.shield.as_ref()).unwrap().mean_fill()
    };
    // What `hero`'s generator recharges by itself in a second, as a mean fill: 15 of 6 × 1,850.
    let recharge = 15.0 / (6.0 * 1850.0);

    tick(&mut play, 0.5);
    // `fr_l_angar`'s one place is its ground-level dock (docs/27, "The places"): 10 across, 12
    // up and 8.4 down about its vertex, so the hero stands on it from the ground outside.
    let outpost = play.buildings.iter().find(|b| b.doors.is_empty()).unwrap().target;
    let set = play.places.iter().find(|p| p.target == outpost).expect("the Outpost's places");
    assert_eq!(set.places.len(), 1);
    let vertex = set.places[0].vertex;
    assert_eq!(vertex.flags, 0x1000_0620, "a ground-level dock and no pod bit");
    assert!(vertex.flags & PLACE_DOCK != 0);
    let part = &play.battle.combat.targets[outpost].parts[set.part];
    let dock = parkan_world::factory::vertex_world(&vertex, part).expect("the dock's world point");

    // A neutral Outpost charges nobody.
    let away = play.hero.walker.body.position;
    hurt(&mut play);
    let magazine = play.hero.guns[0].magazine;
    assert!(magazine > 0, "the hero's first gun holds a clip");
    put(&mut play.hero.walker, &play.ground, dock);
    tick(&mut play, 2.0);
    assert_ne!(play.units[outpost].clan, Some(play.player_clan));
    assert!((life(&play) - 0.5).abs() < 1e-3, "no charge from another clan's dock: {}", life(&play));
    assert_eq!(play.hero.guns[0].rounds, 0);
    assert!(shield(&play) < 0.5 + 2.5 * recharge, "the shield only recharges itself: {}", shield(&play));

    // Taken, and the hero hurt again away from it.
    assert!(play.stand_on_pod(outpost));
    tick(&mut play, 6.0);
    assert_eq!(play.units[outpost].clan, Some(play.player_clan));
    put(&mut play.hero.walker, &play.ground, away);
    tick(&mut play, 0.5);
    hurt(&mut play);

    // On the dock: a tenth of full life, of the shield's mean fill and of the magazine a second,
    // full in ten seconds.
    play.cues.clear();
    put(&mut play.hero.walker, &play.ground, dock);
    tick(&mut play, 2.0);
    let after = life(&play);
    assert!((after - 0.7).abs() < 0.01, "a tenth of full life a second: {after}");
    let shielded = shield(&play);
    assert!((shielded - 0.7).abs() < 0.01 + 2.0 * recharge, "a tenth of the shield a second: {shielded}");
    // A tenth of the magazine a second less what each tick's rounding down drops: 93 of 100.
    let rounds = play.hero.guns[0].rounds;
    assert!((85..=100).contains(&rounds), "a tenth of a magazine of {magazine} a second: {rounds}");
    tick(&mut play, 9.0);
    assert_eq!(life(&play), 1.0, "full in ten seconds, whatever it is");
    assert_eq!(play.hero.guns[0].rounds, magazine);
    assert!(shield(&play) > 0.9999, "every sector full, the spent one too: {}", shield(&play));

    // The dock's glow runs while it charges, with `f_recharge.wav` in it (docs/13).
    let charged: Vec<String> = play.cues.iter().map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(charged.iter().any(|s| s == "f_recharge.wav"), "the charging sound: {charged:?}");
    let ids = play.places.iter().find(|p| p.target == outpost).unwrap().places[0].recharge.clone();
    assert_eq!(ids.len(), 1, "the Outpost's one dock glow");
    let owner = parkan_world::fx::Owner::Building(outpost, ids[0]);
    let (_, glow) = play.fx.instances.iter().find(|(o, _)| *o == owner).expect("its glow runs");
    assert!(glow.effect.name.eq_ignore_ascii_case("f_recharge_r"), "{}", glow.effect.name);
    assert!(glow.on && glow.mode == parkan_world::places::RECHARGE_TIME_MODE);
    // It hangs on the field's own `Rech_*` points, which size it to the ground-level dock's
    // cylinder: 10 across and 12 up.
    let axes = glow.frame.axes.map(glam::Vec3::length);
    assert!((14.0..17.0).contains(&axes[0]), "{axes:?} up the field");
    assert!((10.0..13.0).contains(&axes[1]) && (10.0..13.0).contains(&axes[2]), "{axes:?} across it");
    assert!(glow.frame.origin.distance(dock) < 4.0, "over the dock: {} to {dock}", glow.frame.origin);

    // Stepped off it, the charge stops within a place's own 64-128 ms timer, and the glow with
    // it.
    hurt(&mut play);
    put(&mut play.hero.walker, &play.ground, away);
    tick(&mut play, 0.5);
    let off = life(&play);
    play.cues.clear();
    tick(&mut play, 3.0);
    assert_eq!(life(&play), off, "off the dock: {off}");
    let quiet: Vec<String> = play.cues.iter().map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(!quiet.iter().any(|s| s == "f_recharge.wav"), "the glow is quiet again: {quiet:?}");
    assert!(!play.fx.instances.iter().any(|(o, i)| *o == owner && i.on), "and switched off");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_large_factory_starts_with_its_two_prebuilt_designs_the_last_named_first() {
    let (play, m) = mission_03_play();
    let t = object_target(&play, &m, "lplant01.dat");
    let f = play.factories.iter().find(|f| f.target == t).expect("the Large Factory");
    // docs/23, "What `prebuild` does", priced from `tut3_pl.trf`.
    let shown: Vec<(&str, &str, String, String, &[String])> = f
        .projects
        .iter()
        .map(|p| {
            (
                p.path.as_str(),
                p.name.as_str(),
                format!("{:.1}", p.ore),
                format!("{:.1}", p.power),
                &p.lines[..],
            )
        })
        .collect();
    let lines = |l: [&str; 5]| l.map(str::to_owned).to_vec();
    let (p2, p1) = (
        lines(["6 / 2 t", "64 kph", "7 %", "8 %", "300 m"]),
        lines(["7 / 0 t", "43 kph", "8 %", "18 %", "300 m"]),
    );
    assert_eq!(
        shown,
        vec![
            ("units\\units\\prebld\\tut3_p2.dat", "SWW-X Warrior", "118.6".into(), "63.7".into(), &p2[..]),
            ("units\\units\\prebld\\tut3_p1.dat", "SSW-X Warrior", "125.6".into(), "72.7".into(), &p1[..]),
        ]
    );
    assert_eq!(f.selected, Some(0));
    assert!(f.projects.iter().all(|p| p.chassis_size == 2 && p.sphere.is_some()));
}

#[test]
#[ignore = "needs the game install"]
fn a_laser_round_on_mission_03s_bunker_door_opens_it_and_it_shuts_again_once_free() {
    use glam::Vec3;
    use parkan_sim::combat::Event;
    use parkan_world::buildings::Phase;

    let (mut play, m) = mission_03_play();
    let t = object_target(&play, &m, "sbunk01.dat");
    let b = play.buildings.iter().position(|b| b.target == t).unwrap();
    // One door, `i03`, on node 9 at a rate of 0.25 (docs/24, "A shot opens a door").
    assert_eq!(play.buildings[b].doors.len(), 1);
    assert_eq!(play.buildings[b].doors[0].nodes, vec![9]);
    let part = &play.battle.combat.targets[t].parts[play.buildings[b].part];
    let slot = &part.mesh.slots[usize::from(part.mesh.nodes[9].slot_index[0])];
    let [cx, cy, cz, _] = slot.sphere;
    let c = part.nodes[9].apply([cx, cy, cz].map(|v| f64::from(v * part.scale)));
    let door = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
    let z0 = part.nodes[9].translation[2];
    // A muzzle 15 m from the door's middle, the hero far away: the first heading whose line to
    // the door meets the door before anything else.
    let muzzle = (0..32)
        .map(|k| k as f32 * std::f32::consts::TAU / 32.0)
        .flat_map(|a| [0.0, 2.0, 4.0].map(|dz| door + Vec3::new(a.cos(), a.sin(), 0.0) * 15.0 + Vec3::Z * dz))
        .find(|&p| {
            play.battle
                .combat
                .first_hit(&play.ground, None, p, door, 0.0)
                .is_some_and(|(s, x, _)| x == Some(t) && s.node == Some(9))
        })
        .expect("a line to the door");
    let direction = (door - muzzle).normalize();
    let laser = play.hero.robot.rounds[2].expect("the battle laser's round");
    play.battle.combat.fire(laser, None, muzzle, direction, Vec3::ZERO, 1.0, None).unwrap();
    let (mut struck_at, mut open_at, mut shut_at) = (None, None, None);
    for tick in 0..(60 * 20) {
        for e in play.tick(1000.0 / 60.0, [0.0; 2]) {
            if matches!(e, Event::Struck { target: Some(x), node: Some(9), .. } if x == t) {
                struck_at.get_or_insert(tick);
            }
        }
        let d = &play.buildings[b].doors[0];
        assert!(!d.held, "nothing holds the door");
        match d.phase {
            Phase::Open => {
                open_at.get_or_insert(tick);
            }
            Phase::Shut if open_at.is_some() => {
                shut_at.get_or_insert(tick);
            }
            _ => {}
        }
        if open_at == Some(tick) {
            let z1 = play.battle.combat.targets[t].parts[play.buildings[b].part].nodes[9].translation[2];
            assert!(z0 - z1 > 5.0, "the door lowers its 5.76 m: {z0} to {z1}");
        }
    }
    let struck = struck_at.expect("the round strikes the door");
    let open = open_at.expect("the door opens");
    // A rate of 0.25: the building sees it open 3.6 s after the first step, which waits up
    // to 100 ms.
    let seconds = (open - struck) as f32 / 60.0;
    assert!((3.55..3.8).contains(&seconds), "open {seconds} s after the hit");
    let shut = shut_at.expect("the door shuts once free");
    let closed = (shut - open) as f32 / 60.0;
    assert!((5.0..10.0).contains(&closed), "shut {closed} s after it opened");
}

#[test]
#[ignore = "needs the game install"]
fn c03_m01s_builder_upgrades_the_captured_factory_to_the_medium_one_its_clan_has_researched() {
    use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT};
    use parkan_sim::behaviour::{Task, UpgradeState};
    use parkan_world::cockpit::commander::Panel;

    let mut play = campaign_play(gamedir::C03_MISSION_01);
    // This is a test of the upgrade, and the builder works 180 m from a building the enemy's
    // planner sets a warbot to patrol at 150: where the patrol has brought it by the time the
    // sphere starts turns on how every AI unit steers, and once it has shot the builder dead
    // 42 s in. The progression goes, so no clan plans.
    play.progression = None;
    let player = play.player_clan;
    // The neutral clan's Small Factory, bunker and builder, as capturing them would leave them.
    let of = |play: &parkan_world::play::Play, kind: u32, which: u32| {
        (0..play.units.len())
            .find(|&t| {
                play.units[t].kind == which
                    && play.units[t].type_word == kind
                    && play.units[t].clan == Some(2)
            })
            .unwrap_or_else(|| panic!("the neutral clan's 0x{kind:08x}"))
    };
    let factory = of(&play, 0x8000_0010, KIND_BUILDING);
    let bunker = of(&play, 0x8001_0000, KIND_BUILDING);
    let builder = of(&play, 0x0100_4000, KIND_UNIT);
    assert!(play.commander.paths[factory].to_ascii_lowercase().contains("splant01"), "the small one");
    for t in [factory, bunker, builder] {
        play.units[t].clan = Some(player);
    }
    let stood = play.battle.combat.targets[factory].position;
    let id = play.units[factory].logical_id;

    // The builders page offers Upgrade Factory (19) and no Build row: this mission's tree has
    // the Medium Factory researched and none of the seven first buildings.
    play.enter_command(bunker);
    let mut panel = Panel::default();
    panel.update(&mut play, 0.0);
    panel.turn(&mut play, 3, 0.0);
    assert_eq!(play.selected_units(), vec![builder]);
    assert_eq!(panel.menu, vec![0, 1, 2, 3, 6, 7, 19], "{:?}", panel.menu);
    assert_eq!(play.upgrade_target(0x8000_0010), Some(factory));
    assert_eq!(play.hq_command(19), Some(parkan_sim::hq::Act::Upgrade(0x8000_0010)));
    let task = |play: &parkan_world::play::Play| {
        play.robots.iter().find(|(t, _)| *t == builder).unwrap().1.behaviour.task()
    };
    assert!(
        matches!(task(&play), Task::Upgrade { building, state: UpgradeState::Going, .. } if building == id),
        "{:?}",
        task(&play)
    );
    assert!(play.roll_back(), "out of command mode");

    // It walks to the factory and stands beside it, and the factory's own sphere starts.
    let run = |play: &mut parkan_world::play::Play, seconds: usize| {
        for _ in 0..(60 * seconds) {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    let mut working = None;
    for s in 0..90 {
        run(&mut play, 1);
        if matches!(task(&play), Task::Upgrade { state: UpgradeState::Working, .. }) {
            working = Some(s);
            break;
        }
    }
    let arrived = working.unwrap_or_else(|| panic!("it never reached the factory: {:?}", task(&play)));
    assert!(play.construction.spheres.iter().any(|s| s.target == factory), "the factory's sphere runs");

    // 50 s on, the small factory goes and the scheme's next stands where it did, the player's.
    for _ in 0..90 {
        run(&mut play, 1);
        if !matches!(task(&play), Task::Upgrade { .. }) {
            break;
        }
    }
    assert!(!matches!(task(&play), Task::Upgrade { .. }), "the builder is let go: {:?}", task(&play));
    // It stood through the old factory's kill every 250 ms from 26 s and the new one's:
    // invulnerable while it upgrades, which the kill's life-system slot 7 respects.
    assert!(play.battle.combat.targets[builder].alive, "the builder lives");
    assert!(!play.battle.combat.targets[factory].alive && play.deleted[factory], "the small one is gone");
    let made = (0..play.units.len())
        .find(|&t| {
            play.units[t].kind == KIND_BUILDING
                && play.units[t].clan == Some(player)
                && play.commander.paths[t].to_ascii_lowercase().contains("mplant01")
        })
        .expect("the Medium Factory stands");
    let at = play.battle.combat.targets[made].position;
    assert!(at.truncate().distance(stood.truncate()) < 1.0, "where the small one stood: {at} to {stood}");
    assert_eq!(play.units[made].type_word, 0x8000_0010);
    assert!(play.buildings.iter().any(|b| b.target == made), "with its doors and pod");
    eprintln!("beside the factory {arrived} s in");

    // The row is gone: the Large Factory above it is not researched in this mission's tree.
    play.enter_command(bunker);
    let mut panel = Panel::default();
    panel.update(&mut play, 0.0);
    panel.turn(&mut play, 3, 0.0);
    assert!(!panel.menu.contains(&19), "no Upgrade Factory left: {:?}", panel.menu);
}

/// A click on an Upgrade row sends each builder to the nearest building of the row's Type it
/// would take (`iron3d.dll:0x10078f60`), not to the first the row test found.
#[test]
#[ignore = "needs the game install"]
fn an_upgrade_row_sends_each_builder_to_the_nearest_building_it_would_take() {
    use parkan_formats::mission::{KIND_BUILDING, KIND_UNIT};

    let mut play = campaign_play("MISSIONS/Single.01");
    play.progression = None;
    let player = play.player_clan;
    let stores: Vec<usize> = (0..play.units.len())
        .filter(|&t| {
            play.units[t].kind == KIND_BUILDING
                && play.commander.paths[t].to_ascii_lowercase().contains("sstore01")
        })
        .collect();
    assert_eq!(stores.len(), 2, "Single.01's two Small Warehouses");
    for &t in &stores {
        play.units[t].clan = Some(player);
    }
    let kind = play.units[stores[0]].type_word;
    // Neither's Medium Warehouse is researched in this mission's tree, so no row is offered.
    assert_eq!(play.upgradable(kind), Vec::<usize>::new(), "the entry above is researched whole, or nothing");
    // With `FULL_RESEARCH_TREE` every part is offered.
    play.research.full = true;
    assert_eq!(play.upgradable(kind), stores, "both would be taken, in the level's order");
    assert_eq!(play.upgrade_target(kind), Some(stores[0]), "the row is offered on the first");
    // A builder of the player's beside the second.
    let builder = (0..play.units.len())
        .find(|&t| play.units[t].kind == KIND_UNIT && play.units[t].type_word == 0x0100_4000)
        .expect("a builder");
    play.units[builder].clan = Some(player);
    let near = play.battle.combat.targets[stores[1]].position + glam::Vec3::new(10.0, 0.0, 0.0);
    play.battle.combat.targets[builder].position = near;
    assert_eq!(play.upgrade_target_for(kind, builder), Some(stores[1]));
    let far = play.battle.combat.targets[stores[0]].position + glam::Vec3::new(10.0, 0.0, 0.0);
    play.battle.combat.targets[builder].position = far;
    assert_eq!(play.upgrade_target_for(kind, builder), Some(stores[0]));
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_builders_page_selects_the_builder_and_offers_build_mine_and_standby_stands_it_by() {
    use parkan_world::cockpit::commander::Panel;
    use parkan_world::play::Mode;

    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    let builder = object_target(&play, &m, "tut3_b.dat");
    let transport = object_target(&play, &m, "tut3_t.dat");
    // As the bunker's capture leaves it.
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    assert_eq!(play.mode(), Mode::Command(bunker));
    let mut panel = Panel::default();
    panel.update(&mut play, 0.0);
    // Builders and transports are enabled, battle units and the research centre not
    // (docs/41, "What enables a button").
    let enabled = |p: &Panel, page: u8| {
        parkan_world::cockpit::commander::COLUMN.iter().zip(p.enabled).any(|(item, on)| {
            on && matches!(item, parkan_world::cockpit::commander::Item::Button { control: parkan_world::cockpit::commander::Control::Page(n), .. } if *n == page)
        })
    };
    assert!(enabled(&panel, 3) && enabled(&panel, 2) && enabled(&panel, 5) && enabled(&panel, 7));
    assert!(!enabled(&panel, 1) && !enabled(&panel, 4) && !enabled(&panel, 6));
    panel.turn(&mut play, 3, 0.0);
    assert_eq!(play.selected_units(), vec![builder]);
    // Standby, Route, Search and capture, Seek and destroy, Guard, Refit, Build Mine.
    assert_eq!(panel.menu, vec![0, 1, 2, 3, 6, 7, 10]);
    panel.turn(&mut play, 2, 0.0);
    assert_eq!(play.selected_units(), vec![transport]);
    assert!(panel.menu.contains(&8) && !panel.menu.contains(&10), "{:?}", panel.menu);
    // A row with no pick gives its order at once.
    assert_eq!(
        play.hq_command(0),
        Some(parkan_sim::hq::Act::Order(parkan_sim::orders::Order {
            code: parkan_sim::orders::STAYGROUND,
            parameter: 0,
            target: parkan_sim::orders::Target::NotDefined,
        }))
    );
    let robot = &play.robots.iter().find(|(t, _)| *t == transport).unwrap().1;
    assert_eq!(robot.behaviour.task(), parkan_sim::behaviour::Task::StayGround);
    // Its box's lines, rated from its design.
    play.rate_units(&[builder]);
    let lines = play.unit_lines(builder).expect("the builder rates");
    assert_eq!(lines[4], "250 m", "{lines:?}");
}

#[test]
#[ignore = "needs the game install"]
fn in_command_mode_a_click_selects_the_builder_sends_it_and_build_mine_places_a_ghost_green_at_the_lode() {
    use parkan_world::pick::{Aim, PickMode, cursor_state};

    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    let builder = object_target(&play, &m, "tut3_b.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    play.command_frame(0.0, parkan_world::command::Edges::default());
    let eye = play.eye().position;
    let at = |p: glam::Vec3| Aim::Ray { eye, direction: (p - eye).normalize() };
    // Nothing selected: the builder is the player's own, kind 7, the PICK cursor.
    let centre = play.battle.combat.targets[builder].centre;
    let pick = play.pick(at(centre));
    assert_eq!((pick.kind, pick.object), (7, Some(builder)));
    assert_eq!(cursor_state(pick.kind), 2);
    assert_eq!(play.click_world(pick), Some((3, false)));
    assert_eq!(play.selected_units(), vec![builder]);
    // Ground ahead of it, with the builder selected: a Go, the PLACE cursor.
    let ground = play.battle.combat.targets[builder].position + glam::Vec3::new(30.0, 0.0, 0.0);
    let pick = play.pick(at(ground));
    assert_eq!(pick.kind, 1, "{pick:?}");
    assert_eq!(cursor_state(pick.kind), 3);
    play.click_world(pick);
    let robot = &play.robots.iter().find(|(t, _)| *t == builder).unwrap().1;
    assert_eq!(robot.order.map(|o| o.code), Some(parkan_sim::hq::GO));
    // Build Mine: pick mode 6, the ghost red away from the lode and green on it.
    assert!(!play.open_pick(parkan_sim::hq::Act::Build(0x8000_0004)));
    assert_eq!(play.commander.pick_mode, PickMode::PlaceFm);
    let lode = play.commander.lodes[0].position;
    let lode_ground = play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point;
    play.update_ghost(at(lode_ground + glam::Vec3::new(60.0, 0.0, 0.0)));
    assert!(!play.commander.ghost.as_ref().unwrap().valid, "no lode within 20");
    play.update_ghost(at(lode_ground));
    let ghost = play.commander.ghost.clone().unwrap();
    assert!(ghost.valid && ghost.path.to_ascii_lowercase().ends_with("smine01.dat"), "{ghost:?}");
    assert!(play.turn_ghost(true));
    assert!((play.commander.ghost.as_ref().unwrap().yaw - 0.05).abs() < 1e-6);
    // `.` held: the key-down and each repeat after it turn it again (docs/32, "Turning it").
    for _ in 0..4 {
        assert!(play.command_key(parkan_formats::controls::CMD_JAMES_BASE_ROTRIGHT, true));
    }
    assert!((play.commander.ghost.as_ref().unwrap().yaw + 0.15).abs() < 1e-6);
    assert!(play.commit_placement());
    assert_eq!(play.commander.pick_mode, PickMode::Free);
    let robot = &play.robots.iter().find(|(t, _)| *t == builder).unwrap().1;
    assert_eq!(robot.order.map(|o| (o.code, o.parameter as u32)), Some((parkan_sim::hq::BUILD, 0x8000_0004)));
    // A second placement put away with the right button says so.
    play.open_pick(parkan_sim::hq::Act::Build(0x8000_0004));
    play.says.clear();
    play.right_click(3, false);
    assert!(play.commander.ghost.is_none() && play.commander.pick_mode == PickMode::Free);
    assert!(play.says.iter().any(
        |s| matches!(s, parkan_world::progress::Say::Text(_, t) if t == "Building was cancelled by user")
    ));
    // A band over the map about the builder takes it again, alone of the clan's units there.
    play.clear_selection();
    let b = play.battle.combat.targets[builder].position;
    play.band_select(
        [[b.x - 10.0, b.y - 10.0], [b.x + 10.0, b.y + 10.0]],
        parkan_world::pick::BandSpace::Map,
    );
    assert_eq!(play.selected_units(), vec![builder]);
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_builder_puts_a_mine_on_the_lode_that_counts_at_once_and_runs_its_41_second_sphere() {
    use parkan_sim::behaviour::Task;
    use parkan_world::construction::BUILDING_MINE;
    use parkan_world::progress::Say;

    let (mut play, m) = mission_03_play();
    let builder = object_target(&play, &m, "tut3_b.dat");
    let plant = object_target(&play, &m, "lplant01.dat");
    // The lode (docs/32, "Mission 03's mine").
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    assert_eq!(play.placement_model(BUILDING_MINE).as_deref(), Some("UNITS\\BUILDS\\MINE\\smine01.dat"));
    assert!(play.placement_valid(Some(builder), BUILDING_MINE, at, 0.0), "on the lode");
    assert!(!play.placement_valid(Some(builder), BUILDING_MINE, at + glam::Vec3::X * 100.0, 0.0), "no lode");
    let factory = play.battle.combat.targets[plant].position;
    assert!(!play.placement_valid(Some(builder), 0x8000_0008, factory, 0.0), "over the Large Factory");
    assert!(play.placement_valid(Some(builder), 0x8000_0008, at, 0.0), "a storage needs no lode");
    // The mine costs 540 by `tut3_pl.trf`, and tut3_b holds 200.
    assert_eq!(play.building_ore(BUILDING_MINE), Some(540.0));
    assert_eq!(play.economy.held(builder), 200.0);
    assert!(play.order_build(builder, BUILDING_MINE, at, 0.3));

    let targets = play.battle.combat.targets.len();
    let mut made = None;
    let mut seconds = 0.0;
    let mut says = Vec::new();
    while made.is_none() && seconds < 60.0 {
        play_for(&mut play, 0.5, |_| {});
        seconds += 0.5;
        says.append(&mut play.says);
        if play.battle.combat.targets.len() > targets {
            made = Some(targets);
        }
    }
    let mine = made.expect("the builder puts the mine up within a minute");
    let near = play.robots.iter().find(|(t, _)| *t == builder).unwrap().1.walker.body.position;
    assert!(near.truncate().distance(at.truncate()) < 30.0, "the builder stands on the site: {near}");
    assert_eq!(play.units[mine].type_word, BUILDING_MINE);
    assert_eq!(play.units[mine].clan, Some(play.player_clan));
    assert_eq!(play.economy.held(builder), -340.0, "it goes into debt");
    // Function 34 counts it from the moment it exists (docs/34).
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.count_type(play.player_clan, BUILDING_MINE), 1);
    assert!(play.building_itself(mine));
    // The sphere sends the builder out once its sign's 5 s are up.
    play_for(&mut play, 6.0, |_| {});
    let task = play.robots.iter().find(|(t, _)| *t == builder).unwrap().1.behaviour.task();
    assert!(matches!(task, Task::Leave { .. } | Task::Stop), "{task:?}");
    // Objective 3 completes on the Mission handler's next run, with its messages.
    play_for(&mut play, 3.0, |p| says.append(&mut p.says));
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives[2].state, 1, "Prospect for mineral deposits and build a Mine");
    assert!(says.iter().any(|s| matches!(s, Say::Text(_, t) if t.contains("Excellent"))), "{says:?}");
    // 41 s in all.
    play_for(&mut play, 30.0, |_| {});
    assert!(play.building_itself(mine), "still building itself at 39 s");
    play_for(&mut play, 3.0, |_| {});
    assert!(!play.building_itself(mine), "done by 42 s");
    assert!(play.battle.combat.targets[mine].alive);
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_site_test_asks_for_the_builders_way_there_and_walkable_ground_under_every_exit() {
    use parkan_world::construction::BUILDING_MINE;
    const STORAGE: u32 = 0x8000_0008;

    let (mut play, m) = mission_03_play();
    let builder = object_target(&play, &m, "tut3_b.dat");
    // Every first building a builder puts up has a hall way with an exit (docs/32, "The test").
    for (t, exits) in [
        (0x8000_0002, 11),
        (BUILDING_MINE, 3),
        (STORAGE, 4),
        (0x8000_0010, 20),
        (0x8000_0040, 2),
        (0x8000_0200, 3),
        (0x8000_0400, 3),
        (0x8001_0000, 1),
        (0x8002_0000, 3),
        (0x8004_0000, 3),
        (0x8010_0000, 1),
        (0x8020_0000, 1),
    ] {
        let path = play.placement_model(t).unwrap();
        assert_eq!(play.hall_exits(&path).map(|e| e.len()), Some(exits), "{path}");
    }
    // The lode: the builder's way there is found and the mine's three exits stand on walkable
    // areals at every turn, so the site the recording's mine went up on stays green.
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let from = play.battle.combat.targets[builder].position;
    assert!(matches!(play.walker_search(builder, from, at), Some(Ok(_))));
    for k in 0..16 {
        let yaw = k as f32 * std::f32::consts::TAU / 16.0;
        assert!(play.placement_valid(Some(builder), BUILDING_MINE, at, yaw), "turned {yaw}");
    }
    // West of the base a storage's basement is level, its way there is found, and an exit falls
    // on an areal whose word is 0: "HallVertex … is in Non-Reachable Areal".
    let west = glam::Vec3::new(410.0, 730.0, play.ground.below(410.0, 730.0, 1.0e5).unwrap().point.z);
    assert!(matches!(play.walker_search(builder, from, west), Some(Ok(_))));
    let storage = play.placement_model(STORAGE).unwrap();
    let exits = play.hall_exits(&storage).unwrap();
    let graph = play.graph.as_ref().unwrap();
    assert!(!exits.iter().all(|e| graph.usable(west.x + e.x, west.y + e.y)));
    assert!(!play.placement_valid(Some(builder), STORAGE, west, 0.0), "an exit off the walkable ground");
    let graph = play.graph.take();
    assert!(play.placement_valid(Some(builder), STORAGE, west, 0.0), "the rest of the test passes it");
    play.graph = graph;
    // A builder standing on an areal no link leaves finds no way anywhere: "BAD PATH".
    let stranded = glam::Vec3::new(300.0, 700.0, from.z);
    assert!(!play.graph.as_ref().unwrap().usable(stranded.x, stranded.y));
    play.battle.combat.targets[builder].position = stranded;
    assert!(matches!(play.walker_search(builder, stranded, at), Some(Err(_))));
    assert!(!play.placement_valid(Some(builder), BUILDING_MINE, at, 0.0), "no way from the builder");
    assert!(play.placement_valid(None, BUILDING_MINE, at, 0.0), "with no builder named, no search");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_mine_plays_its_sphere_as_the_controllers_codes_start_and_stop_its_three_effects() {
    use parkan_world::construction::{BUILDING_MINE, DOME, RAY, SIGN};
    use parkan_world::fx::Owner;

    let (mut play, _) = mission_03_play();
    let player = play.player_clan;
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let now = play.hero.time_ms;
    let mine = play.create_building(player, BUILDING_MINE, at, 0.0, now).expect("the mine stands");
    let state = |play: &mut parkan_world::play::Play, id: i32| {
        let i = play.fx.owned(Owner::Building(mine, id)).next().expect("made at load");
        (i.mode, i.on, i.frame)
    };
    // Action 5 made all three, idle in their header's mode 0; code 1 started the sign looping.
    play_for(&mut play, 1.0, |_| {});
    assert_eq!(state(&mut play, SIGN).0, 2);
    assert_eq!(state(&mut play, DOME).0, 0);
    assert_eq!(state(&mut play, RAY).0, 0);
    // The sign loops through the 30 s the task sends no code.
    play_for(&mut play, 33.0, |_| {});
    assert_eq!((state(&mut play, SIGN).0, state(&mut play, SIGN).1), (2, true));
    // Code 2 at 35 s: the dome and the ray once through, the sign off.
    play_for(&mut play, 1.5, |_| {});
    assert_eq!(state(&mut play, DOME).0, 1);
    assert_eq!(state(&mut play, RAY).0, 1);
    assert!(!state(&mut play, SIGN).1, "the sign is off");
    // The frame stands up: its first axis, the ray's travel and the dome's pole, is z, the
    // sphere's radius long.
    let (_, _, frame) = state(&mut play, DOME);
    let sphere = play.construction.spheres.iter().find(|s| s.target == mine).unwrap().clone();
    assert!((frame.axes[0] - glam::Vec3::Z * sphere.radius).length() < 1e-3, "{frame:?}");
    assert_eq!(frame.origin, sphere.centre);
    // Code 0 at 40 s: the ray off, the dome played back out; it outlives the task's end.
    play_for(&mut play, 5.0, |_| {});
    assert!(!state(&mut play, RAY).1, "the ray is off");
    assert_eq!(state(&mut play, DOME).0, 3);
    play_for(&mut play, 2.0, |_| {});
    assert!(!play.building_itself(mine));
    assert_eq!(state(&mut play, DOME).0, 3, "the controller stays on its code-0 state");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_mine_stands_only_once_its_controller_places_it_as_the_dome_turns_back_at_40_seconds() {
    use parkan_world::construction::{BUILDING_MINE, SIGN};
    use parkan_world::fx::Owner;

    let (mut play, _) = mission_03_play();
    let player = play.player_clan;
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let now = play.hero.time_ms;
    let cuts = play.ground.cuts.len();
    let mine = play.create_building(player, BUILDING_MINE, at, 0.0, now).expect("the mine stands");
    let present = |play: &parkan_world::play::Play| play.ground.solids[mine].present;
    // Seen in The Field Base: the site shows the sign, then the ray and the dome, and no mine.
    // Nothing of it is ground from the moment it is made.
    assert!(!present(&play));
    play_for(&mut play, 1.0, |_| {});
    assert!(!play.placed(mine) && !present(&play) && play.ground.cuts.len() == cuts);
    // Nor is it in the way: the sign draws nothing while its tested point is hidden, and seen
    // from any side its point is clear.
    let sign = play.fx.owned(Owner::Building(mine, SIGN)).next().unwrap().test_point().unwrap();
    for (x, y) in [(900.0f32, 943.0f32), (1100.0, 1000.0), (1026.0, 800.0), (960.0, 1050.0)] {
        let eye = glam::Vec3::new(x, y, play.ground.below(x, y, 1.0e5).unwrap().point.z + 3.0);
        assert!(play.battle.combat.clear_line(&play.ground, eye, sign), "from {eye:?}");
    }
    play_for(&mut play, 38.5, |_| {});
    assert!(!play.placed(mine) && !present(&play), "not through code 2");
    // Code 0 at 40 s: action 20 places it, cutting the landscape under it.
    play_for(&mut play, 1.0, |_| {});
    assert!(play.placed(mine) && present(&play));
    assert_eq!(play.ground.cuts.len(), cuts + 1);
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_large_factory_waits_on_ore_and_builds_an_ssw_x_in_25_seconds_once_the_mine_digs() {
    use parkan_world::construction::BUILDING_MINE;

    let (mut play, m) = mission_03_play();
    let player = play.player_clan;
    // The generator and the bunker taken, as the hero takes them: 10.07 power a second.
    for name in ["gener01.dat", "sbunk01.dat"] {
        let t = object_target(&play, &m, name);
        play.units[t].clan = Some(player);
    }
    let plant = object_target(&play, &m, "lplant01.dat");
    let f = play.factories.iter().position(|f| f.target == plant).unwrap();
    assert_eq!((play.factories[f].efficiency, play.factories[f].use_power), (5.0, 4.0));
    let ssw = play.factories[f].projects.iter().position(|p| p.name.starts_with("SSW-X")).unwrap();
    play.factories[f].selected = Some(ssw);
    assert!(play.start_factory(f, false));
    let cost = play.factories[f].build.as_ref().unwrap().construct.ore_cost;
    assert!((cost - 125.6 / 5.0).abs() < 0.1, "the price over the factory's efficiency: {cost}");

    // With no ore anywhere it collects its power in 3.6 s and its time, and waits.
    play_for(&mut play, 12.0, |_| {});
    let c = play.factories[f].build.as_ref().expect("still building").construct;
    assert!(c.power >= c.power_cost && c.time >= c.seconds && c.ore == 0.0, "{c:?}");
    assert_eq!(play.resource_rows(player)[0], 0);

    // A mine on the lode: its 41 s sphere, then it digs at 50 a second to 500, 11%.
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let now = play.hero.time_ms;
    let mine = play.create_building(player, BUILDING_MINE, at, 0.0, now).expect("the mine stands");
    play_for(&mut play, 41.5, |_| {});
    assert!(!play.building_itself(mine));
    let mut seconds: f32 = 0.0;
    let mut energies = Vec::new();
    while play.factories[f].build.is_some() && seconds < 60.0 {
        play_for(&mut play, 0.25, |_| {});
        seconds += 0.25;
        if (10.0..15.0).contains(&seconds) {
            energies.push(play.resource_rows(player)[1]);
            let ore = play.resource_rows(player)[0];
            assert!((10..=12).contains(&ore), "a mine of 500 reads 11%: {ore}");
        }
    }
    // (10.07 − 1.03) ÷ 10.07 with the mine at work, the target sampled across its steps.
    let energy = energies.iter().sum::<i32>() as f32 / energies.len() as f32;
    assert!((85.0..=95.0).contains(&energy), "the mine at work draws 1: {energy} of {energies:?}");
    // 25.1 s with the mine alone (docs/23, "A warbot from the Large Factory").
    assert!((seconds - 25.1).abs() < 2.0, "the bot is done {seconds} s after the mine digs");
}

/// Mission 03's "Available CPUs" as the recording reads it (docs/23, "The bot limit is the
/// clan's mind count"): 4 before a build, 3 once it starts, 4 again after the clan's next takt
/// sweeps the reservation, 5 once the builder is gone, and the finished bot takes one.
#[test]
#[ignore = "needs the game install"]
fn mission_03s_free_minds_come_back_at_the_clans_takt_and_the_finished_bot_takes_one() {
    let (mut play, m) = mission_03_play();
    let player = play.player_clan;
    // The generator and the bunker taken, so the factory has its power.
    for name in ["gener01.dat", "sbunk01.dat"] {
        let t = object_target(&play, &m, name);
        play.units[t].clan = Some(player);
    }
    let plant = object_target(&play, &m, "lplant01.dat");
    let builder = object_target(&play, &m, "tut3_b.dat");
    let f = play.factories.iter().position(|f| f.target == plant).unwrap();
    let ssw = play.factories[f].projects.iter().position(|p| p.name.starts_with("SSW-X")).unwrap();
    play.factories[f].selected = Some(ssw);
    // Let the takt that runs at the start pass, so the next is 7 to 8 s off.
    play_for(&mut play, 0.1, |_| {});
    assert_eq!(play.free_minds(player), 4, "7 minds; the hero, the builder and the transport");
    assert!(play.start_factory(f, false));
    assert_eq!(play.free_minds(player), 3, "the build reserves one");
    let mut back = None;
    for tick in 0..(8.1 * 60.0) as usize {
        play_for(&mut play, 1.0 / 60.0, |_| {});
        if play.free_minds(player) == 4 {
            back = Some(tick as f32 / 60.0);
            break;
        }
    }
    let back = back.expect("the clan's takt frees the reservation within 8 s");
    assert!(play.factories[f].build.is_some(), "while the build still runs, {back} s in");
    // The builder gone, as it goes at 237 s in the recording: 5, as at 260 s.
    play.battle.combat.targets[builder].alive = false;
    assert_eq!(play.free_minds(player), 5);
    // Ore in the factory's hands: the build completes and the bot claims a mind.
    let robots = play.robots.len();
    for _ in 0..60 * 60 {
        if play.factories[f].build.is_none() {
            break;
        }
        let most = play.economy.most(plant);
        play.economy.ore.insert(plant, (1000.0, most));
        play_for(&mut play, 1.0 / 60.0, |_| {});
    }
    assert!(play.factories[f].build.is_none(), "the build completed");
    assert_eq!(play.robots.len(), robots + 1, "the bot is made");
    assert_eq!(play.free_minds(player), 4, "and holds a mind, as at 261.5 s with no build");

    // With no mind free at completion the bot is not made, and the build is over all the
    // same ("No free Mind... CreateObjectFromScheme failed").
    assert!(play.start_factory(f, false));
    let c = usize::try_from(player).unwrap();
    play.clans[c].minds = 3;
    assert_eq!(play.free_minds(player), 0);
    let robots = play.robots.len();
    for _ in 0..60 * 60 {
        if play.factories[f].build.is_none() {
            break;
        }
        let most = play.economy.most(plant);
        play.economy.ore.insert(plant, (1000.0, most));
        play_for(&mut play, 1.0 / 60.0, |_| {});
    }
    assert!(play.factories[f].build.is_none(), "the build completed");
    assert_eq!(play.robots.len(), robots, "no bot without a free mind");
}

#[test]
#[ignore = "needs the game install"]
fn mission_03s_transport_carries_2000_ore_from_the_mine_to_the_small_warehouse() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::orders::{Order, TRANSPORT, Target};
    use parkan_world::construction::BUILDING_MINE;

    let (mut play, m) = mission_03_play();
    let player = play.player_clan;
    let transport = object_target(&play, &m, "tut3_t.dat");
    let storage = object_target(&play, &m, "sstore01.dat");
    assert_eq!((play.economy.held(transport), play.economy.most(transport)), (0.0, 2000.0));
    // With no mine the task cannot run.
    let order = Order { code: TRANSPORT, parameter: -1, target: Target::NotDefined };
    let give = |play: &mut parkan_world::play::Play| {
        let robot = &mut play.robots.iter_mut().find(|(t, _)| *t == transport).unwrap().1;
        robot.order = Some(order);
        robot.behaviour.order(&order);
    };
    give(&mut play);
    play_for(&mut play, 1.0, |_| {});
    let task = play.robots.iter().find(|(t, _)| *t == transport).unwrap().1.behaviour.task();
    assert_eq!(task, Task::Stop, "no mine: \"Task Ended\"");

    // A mine on the lode, built and full; it digs on the generator's power.
    let generator = object_target(&play, &m, "gener01.dat");
    play.units[generator].clan = Some(player);
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let now = play.hero.time_ms;
    play.create_building(player, BUILDING_MINE, at, 0.0, now).expect("the mine stands");
    play_for(&mut play, 52.0, |_| {});
    assert_eq!(play.resource_rows(player)[0], 11, "a full mine, 500 of 4,500");

    give(&mut play);
    let mut loaded_at = None;
    let mut seconds: f32 = 0.0;
    while play.economy.held(storage) < 2000.0 && seconds < 180.0 {
        play_for(&mut play, 0.5, |_| {});
        seconds += 0.5;
        if loaded_at.is_none() && play.economy.held(transport) >= 2000.0 {
            loaded_at = Some(seconds);
        }
    }
    let loaded_at = loaded_at.expect("it fills its 2,000 at the mine, which never runs dry");
    assert!(
        play.economy.held(storage) >= 1999.9,
        "{} in the warehouse after {seconds} s",
        play.economy.held(storage)
    );
    // The unloading's 20 s at 100 a second after the walk from the mine (docs/23).
    assert!(seconds - loaded_at >= 20.0 + 8.0, "unloaded {} s after loading", seconds - loaded_at);
    let rows = play.resource_rows(player);
    assert!((55..=57).contains(&rows[0]), "2,500 of 4,500: {rows:?}");
    // It goes back for more.
    play_for(&mut play, 3.0, |_| {});
    let task = play.robots.iter().find(|(t, _)| *t == transport).unwrap().1.behaviour.task();
    assert!(matches!(task, Task::Transport { goal: Some(_), .. }), "{task:?}");
}

/// A building's batteries are its root controller's slots with its `i_pws_f_*` parts fitted
/// into them, 8 held and 500 a second each (docs/23, "A power shortage lowers efficiency"):
/// Mission 03's Large Factory holds 32, not its root's 20, and its Small Storage 16; the
/// generator still reads full, and efficiency is the root's class-26 value.
#[test]
#[ignore = "needs the game install"]
fn mission_03s_buildings_run_on_the_batteries_fitted_into_their_slots() {
    let (mut play, m) = mission_03_play();
    play_for(&mut play, 0.1, |_| {});
    let site = |play: &parkan_world::play::Play, name: &str| {
        let t = object_target(play, &m, name);
        play.economy.site(t).cloned().unwrap_or_else(|| panic!("{name} joined"))
    };
    let plant = site(&play, "lplant01.dat");
    assert_eq!((plant.battery.capacity, plant.battery.output, plant.efficiency), (32.0, 2000.0, 5.0));
    let store = site(&play, "sstore01.dat");
    assert_eq!((store.battery.capacity, store.battery.output, store.efficiency), (16.0, 1000.0, 1.0));
    assert_eq!((store.on_board, store.off_board), (20.0, 1.0));
    assert!(site(&play, "gener01.dat").battery.capacity < 0.0, "a generator reads full");
}

/// The ore a place moves by itself (docs/23, "The ore a place moves by itself"): a transport
/// standing in a storage's unloading place gives it nothing, a unit's efficiency being 0;
/// standing in a mine's loading place it is given the mine's 1 a second.
#[test]
#[ignore = "needs the game install"]
fn a_loading_place_gives_a_transport_one_ore_a_second_and_an_unloading_place_takes_none() {
    use parkan_world::construction::BUILDING_MINE;
    use parkan_world::places::{PLACE_LOADING, PLACE_UNLOADING};

    let (mut play, m) = mission_03_play();
    let player = play.player_clan;
    let generator = object_target(&play, &m, "gener01.dat");
    play.units[generator].clan = Some(player);
    let transport = object_target(&play, &m, "tut3_t.dat");
    let store = object_target(&play, &m, "sstore01.dat");
    let lode = glam::Vec3::new(1026.1, 942.7, 0.0);
    let at = lode.with_z(play.ground.below(lode.x, lode.y, 1.0e5).unwrap().point.z);
    let now = play.hero.time_ms;
    let mine = play.create_building(player, BUILDING_MINE, at, 0.0, now).expect("the mine stands");
    play_for(&mut play, 52.0, |_| {});
    assert_eq!(play.resource_rows(player)[0], 11, "a full mine");
    // Where a place stands in the world, by its flag.
    let place = |play: &parkan_world::play::Play, t: usize, flag: u32| {
        let set = play.places.iter().find(|s| s.target == t).expect("its places");
        let p = set.places.iter().find(|p| p.vertex.flags & flag != 0).expect("the place");
        let part = &play.battle.combat.targets[t].parts[set.part];
        parkan_world::factory::vertex_world(&p.vertex, part).unwrap()
    };
    let park = |play: &mut parkan_world::play::Play, at: glam::Vec3| {
        let r = play.robots.iter().position(|(t, _)| *t == transport).unwrap();
        let body = &mut play.robots[r].1.walker.body;
        body.position = at;
        body.velocity = [0.0; 3];
    };
    // Parked in the storage's unloading place with 1000 aboard: nothing moves by itself.
    let unloading = place(&play, store, PLACE_UNLOADING);
    play.economy.ore.insert(transport, (1000.0, 2000.0));
    let stored = play.economy.held(store);
    for _ in 0..5 * 60 {
        park(&mut play, unloading);
        play_for(&mut play, 1.0 / 60.0, |_| {});
    }
    assert_eq!(play.economy.held(transport), 1000.0, "a transport's efficiency is 0");
    assert!(play.economy.held(store) <= stored + 1e-3);
    // In the mine's loading place: about 1 a second, the mine's efficiency times its 1.
    let loading = place(&play, mine, PLACE_LOADING);
    for _ in 0..10 * 60 {
        park(&mut play, loading);
        play_for(&mut play, 1.0 / 60.0, |_| {});
    }
    let gained = play.economy.held(transport) - 1000.0;
    assert!((8.0..=11.0).contains(&gained), "{gained} in 10 s");
}

/// Mission 04, loaded with its scripts, for the research tests.
fn mission_04_research_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, "MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.04").unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 04 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

#[test]
#[ignore = "needs the game install"]
fn a_research_centre_built_in_play_takes_the_research_panels_orders() {
    use parkan_world::selection::RESEARCH_CENTRE;

    let (mut play, _) = mission_04_research_play();
    let player = play.player_clan;
    let turret = play.research_rows()[0];
    // The mission's own centre is the enemy's, so the panel has nowhere to send an order.
    assert!(!play.order_research(turret), "no centre of the player's");

    // A Small Research Center of the player's, put up east of the hero.
    let at = glam::Vec3::new(683.17, 146.46, 215.13);
    // Not a site the placement test passes: its three exits stand 80 m out, and on Mission 04
    // no 20 m grid point at any of eight turns has all three on walkable areals and a level
    // basement besides. `CreateObjectFromScheme` asks neither (docs/32, "The test").
    assert!(!play.placement_valid(None, RESEARCH_CENTRE, at, 0.0), "an exit off the walkable ground");
    let now = play.hero.time_ms;
    let centre = play.create_building(player, RESEARCH_CENTRE, at, 0.0, now).expect("it stands");
    play_for(&mut play, 1.0 / 60.0, |_| {});
    let of = |play: &parkan_world::play::Play| {
        play.research.centres.iter().find(|c| c.target == centre).cloned().expect("it joined")
    };
    // MBehaviour's own grants, none of them a mission's (docs/23, "The four grants").
    assert_eq!(of(&play).grants, parkan_sim::research::Grants::default());
    // The order waits for its construction sphere (docs/41: "the living, unsphered centre").
    assert!(play.building_itself(centre));
    assert!(!play.order_research(turret), "not while its sphere runs");

    play_for(&mut play, 60.0, |_| {});
    assert!(!play.building_itself(centre));
    assert!(play.order_research(turret));
    assert!(play.research_queued(turret));
    assert_eq!(of(&play).orders, [turret]);
}

/// The parts the large flyer's turret socket offers in the player's tree as it stands.
fn large_flyer_turrets(play: &mut parkan_world::play::Play) -> Vec<String> {
    let game = play.assembly.game.clone();
    let mut designer = parkan_world::designs::Designer::new(&game, play.catalogue().unwrap(), 4);
    let sockets: Vec<String> = designer
        .labels(&mut play.assembly, "R_B_02")
        .into_iter()
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.starts_with("e_tur_"))
        .collect();
    designer.catalogue.page(&sockets)
}

#[test]
#[ignore = "needs the game install"]
fn mission_04s_research_centre_opens_its_screen_and_researches_the_large_battle_turret_free_in_five_seconds()
{
    use glam::{Mat4, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::research::Click;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::play::Mode;
    use parkan_world::progress::{Say, Sender};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_04_research_play();
    let player = play.player_clan;
    let turret = {
        let tree = play.research.clan(player).expect("Plr reads tut4_pl.trf");
        tree.tree.items.iter().position(|i| i.code == "4L1").unwrap()
    };
    // At the start the turret is the one row, and the large flyer's turret socket offers nothing.
    assert_eq!(play.research_rows(), [turret]);
    assert!(large_flyer_turrets(&mut play).is_empty());

    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let space = Space::new(640.0, 480.0);
    let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.1)
        * Mat4::look_to_rh(play.hero.eye().position, play.hero.eye().forward, Vec3::Z);
    // The research page's button wants a centre of the clan's.
    let now = play.hero.time_ms;
    cockpit.commander.update(&mut play, now);
    assert!(!cockpit.commander.enabled[6], "no centre of the player's yet");

    // The Enhanced Research Center's pod: captured, objective 2, and its research screen.
    let centre = object_target(&play, &m, "einst01.dat");
    assert!(play.stand_on_pod(centre));
    play_for(&mut play, 6.0, |_| {});
    assert_eq!(play.units[centre].clan, Some(player));
    assert_eq!(play.mode(), Mode::Factory(centre));
    let objectives = &play.progression.as_ref().unwrap().progress.objectives;
    assert_eq!(objectives[2].state, 1, "{objectives:?}");
    let now = play.hero.time_ms;
    cockpit.commander.update(&mut play, now);
    assert!(cockpit.commander.enabled[6], "the research page's button");
    cockpit.update(&mut play, now);
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    let text: Vec<&str> = drawn.text.iter().map(|r| r.text.as_str()).collect();
    for words in [
        "Research Center",
        "Large Battle Turret",
        "DESCENDANTS",
        "Large Battle Turret (5L1)",
        "Large Battle Turret (4L1)",
    ] {
        assert!(text.contains(&words), "{words:?} in {text:?}");
    }
    assert!(
        drawn.previews.iter().any(|p| p.key.path.eq_ignore_ascii_case("e_tur_bb_01")),
        "the turret turns"
    );

    // The batch button orders the row; two seconds in, the row's button cancels it.
    assert_eq!(cockpit.commander.research.click(&mut play, [186.0, 137.0]), Click::Taken);
    assert!(play.research_queued(turret));
    play_for(&mut play, 2.0, |_| {});
    let progress =
        |play: &parkan_world::play::Play| play.research.clan(player).unwrap().state.progress[turret];
    assert!((progress(&play) - 0.4).abs() < 0.01, "{}", progress(&play));
    cockpit.commander.research.click(&mut play, [70.0, 188.0]);
    assert!(!play.research_queued(turret));
    play_for(&mut play, 1.0, |_| {});
    assert!((progress(&play) - 0.4).abs() < 0.01, "a cancel keeps the progress: {}", progress(&play));
    let c = play.research.centres.iter().position(|c| c.target == centre).unwrap();
    assert_eq!(play.research.centres[c].grants.free_technologies, 4);

    // Ordered again it takes another free technology and only the rest of its 5 s.
    cockpit.commander.research.click(&mut play, [70.0, 188.0]);
    play.says.clear();
    let mut ticks = 0;
    while play.research_queued(turret) && ticks < 600 {
        play_for(&mut play, 1.0 / 60.0, |_| {});
        ticks += 1;
    }
    assert!((ticks as f32 / 60.0 - 3.0).abs() < 0.05, "done after {ticks} ticks");
    assert_eq!(play.research.centres[c].grants.free_technologies, 3);
    assert!(play.research.clan(player).unwrap().state.researched(turret));
    assert!(
        play.says.contains(&Say::Text(Sender::System, "Research complete... (Large Battle Turret)".into())),
        "{:?}",
        play.says
    );
    assert!(play.says.iter().any(|s| matches!(s, Say::Voice(_))), "VOICE_RSRCH_COMPLETE");

    // The row has gone, the box reads no item, and the flyer's socket offers the turret.
    let now = play.hero.time_ms;
    cockpit.update(&mut play, now);
    assert!(cockpit.commander.research.rows.is_empty());
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    assert!(drawn.text.iter().any(|r| r.text == "No item selected"));
    let offered = large_flyer_turrets(&mut play);
    assert!(offered.iter().any(|p| p.eq_ignore_ascii_case("e_tur_bb_01")), "{offered:?}");
}
