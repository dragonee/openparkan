//! The campaign missions' own checks: who takes whom as an enemy, and an objective's end.

use crate::common::*;
use parkan_formats::gamedir;

/// The Arrival's first patrol walks its loop about (579, 381) within 150. The hero standing in
/// that ground is on both warbots' radars and hostile to their clan: they take it up and fire on
/// it, the hero being a target rounds strike like any unit (docs/26, "The hit test"), and its
/// shield's sectors meet their rounds first and flash (docs/26, "Shields").
#[test]
#[ignore = "needs the game install"]
fn c01_m02s_patrol_takes_on_the_hero_in_its_ground_and_the_heros_shield_meets_their_rounds() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::combat::Event;

    let mut play = campaign_play(gamedir::C01_MISSION_02);
    let hero = play.battle.combat.hero_index();
    let shield = play.battle.combat.hero.as_ref().and_then(|h| h.shield.clone()).expect("hero11 is shielded");
    // docs/26's measured figures for `hero11`: 1,850 a sector, 15 a second, 0.04 a point, 0.9.
    assert_eq!((shield.max, shield.recharge, shield.cost), (1850.0, 15.0, 0.04));
    assert!(shield.coefficients.iter().all(|&c| (c - 0.9).abs() < 1e-6));
    assert_eq!(shield.effect.to_ascii_lowercase(), "r_shield_b");

    let patrol: Vec<usize> =
        play.robots.iter().map(|(t, _)| *t).filter(|&t| [3, 4].contains(&play.units[t].logical_id)).collect();
    assert_eq!(patrol.len(), 2);
    assert!(play.stand_at(579.0, 330.0, 0.0));
    let (mut attacked, mut flashes, mut lowest) = (false, 0, 1.0_f32);
    for _ in 0..(20 * 60) {
        for e in play.tick(1000.0 / 60.0, [0.0; 2]) {
            if matches!(e, Event::ShieldHit { target, .. } if target == hero) {
                flashes += 1;
            }
        }
        attacked |= play.robots.iter().any(|(t, r)| {
            patrol.contains(t)
                && matches!(r.behaviour.task(), Task::Attack { target: Some(id), .. } if id == play.hero_id)
        });
        let fills = play.battle.combat.hero.as_ref().unwrap().shield.as_ref().unwrap().fills;
        lowest = fills.into_iter().fold(lowest, f32::min);
    }
    assert!(attacked, "the patrol takes the hero up");
    assert!(flashes > 0 && lowest < 1.0, "its rounds meet the hero's shield: {flashes} flashes, {lowest}");
    assert!(
        play.fx
            .instances
            .iter()
            .any(|(o, _)| matches!(o, parkan_world::fx::Owner::Shield(t, _) if *t == hero)),
        "and the generator's effect plays on it"
    );
    assert!(!play.hero.dead());
}

/// Outflanking Maneuver's relation records name every clan but `player`. The loader files each
/// record under its name and leaves a clan no record names at 0 (docs/25, "Clan relations"), so
/// the enemy is hostile to the player both ways, its Small Tower fires on the hero once its radar
/// holds it, and its Small Bunker carries a shield from its deflector part.
#[test]
#[ignore = "needs the game install"]
fn c01_m03s_enemy_takes_the_player_as_hostile_though_no_relation_record_names_it() {
    use parkan_world::play::{MARK_HOSTILE, RELATION_HOSTILE};

    let mut play = campaign_play(gamedir::C01_MISSION_03);
    assert!(play.clans[1].relations.iter().all(|(name, _)| name != "player"));
    assert_eq!((play.relations[0][1], play.relations[1][0]), (RELATION_HOSTILE, RELATION_HOSTILE));
    assert!(play.hostile_to(Some(1), Some(0)) && play.hostile(Some(1)));
    assert_eq!(play.mark_colour(Some(1)), MARK_HOSTILE);

    let bunker = play.units.iter().position(|u| u.logical_id == -2147483645).expect("l_bunk1");
    let shield = play.battle.combat.targets[bunker].shield.clone().expect("the bunker's shield");
    assert_eq!((shield.max, shield.effect.to_ascii_lowercase().as_str()), (11_000.0, "r_shield_g"));
    assert!(shield.coefficients.iter().all(|&c| (c - 0.36).abs() < 1e-6), "{:?}", shield.coefficients);

    let tower = play.units.iter().position(|u| u.logical_id == 23).expect("the first 12tower");
    let e = play.robots.iter().position(|(t, _)| *t == tower).expect("the tower is a robot");
    assert!(play.stand_facing(tower, 150.0, 0.0));
    let hero = play.battle.combat.hero_index();
    let mut aimed = false;
    for _ in 0..(3 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        aimed |= play.robots[e].1.fire_target == Some(hero);
    }
    assert!(aimed, "the tower's fire control takes the hero");
    assert!(play.battle.combat.fired > 0, "and it fires");

    // The bunker's lobbed `bf_f_01` carries frame flag 8, so its distance scores 1 at any range
    // (`Behavior.dll:0x1001b9f0`): from 150 m, where a speed of 45 would score nothing, it fires.
    let mut play = campaign_play(gamedir::C01_MISSION_03);
    let hero = play.battle.combat.hero_index();
    let e = play.emplacements.iter().position(|(t, _)| *t == bunker).expect("the bunker carries guns");
    assert!(play.stand_facing(bunker, 150.0, 0.0));
    let at = play.hero.walker.body.position;
    let (mut shells, mut nearest, mut aimed) = (0, f32::MAX, false);
    // A shell takes about 3.7 s over 150 m on its arc.
    for _ in 0..(8 * 60) {
        aimed |= play.emplacements[e].1.fire_target == Some(hero);
        for event in play.tick(1000.0 / 60.0, [0.0; 2]) {
            if let parkan_sim::combat::Event::Ended { round, .. } = event
                && round.owner == Some(bunker)
            {
                shells += 1;
                nearest = nearest.min(round.position.distance(at));
            }
        }
    }
    assert!(shells > 0, "the bunker fires on the hero 150 m off");
    assert!(aimed, "at the hero");
    // Its guns hang on the turret as parts with no mount; their shells leave on the lower arc.
    assert!(nearest < 5.0, "and its shells come down on it: {nearest} m off");

    // With the engine's departure on, the hero standing on its pod, inside and below the lowest
    // its sight looks, is not fired on while the bunker is the enemy's; the pod takes it for the
    // player a few seconds in.
    let mut play = campaign_play(gamedir::C01_MISSION_03);
    play.building_fire_floor = true;
    assert!(play.stand_on_pod(bunker));
    let (mut shells, mut traced) = (0, false);
    while play.units[bunker].clan == Some(1) && play.hero.time_ms < 10_000.0 {
        traced |= play.emplacements[e].1.fire_target == Some(hero);
        shells += play.battle.combat.rounds.iter().filter(|r| r.owner == Some(bunker)).count();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(traced, "it traces the hero on its pod");
    assert_eq!(shells, 0, "and holds its fire on it");
    assert_eq!(play.units[bunker].clan, Some(play.player_clan), "the pod takes the bunker");
    assert!(!play.hero.dead());
}

/// A grazing medusa takes up no fight, but a hit tells it who fired, and it attacks that unit
/// whatever its radar holds (message `0x19`, docs/31, "A hit pulls a unit in"): shot by the hero
/// from off its pasture, it turns on the hero.
#[test]
#[ignore = "needs the game install"]
fn c01_m02s_medusa_shot_by_the_hero_turns_on_it() {
    use parkan_sim::behaviour::Task;
    use parkan_world::play::CLASS_ANIMAL;

    let mut play = campaign_play(gamedir::C01_MISSION_02);
    let medusa = play.units.iter().position(|u| u.type_word & CLASS_ANIMAL != 0).expect("a medusa");
    let r = play.robots.iter().position(|(t, _)| *t == medusa).expect("the medusa is a robot");
    let at = play.battle.combat.targets[medusa].position;
    assert!(play.stand_at(at.x + 60.0, at.y, 0.0));
    for _ in 0..60 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(
        matches!(play.robots[r].1.behaviour.task(), Task::Migrate { .. }),
        "grazing, it lets the hero be: {:?}",
        play.robots[r].1.behaviour.task()
    );

    // A laser of the hero's at the medusa's sphere.
    let kinds = &play.battle.combat.kinds;
    let kind = play
        .hero
        .rounds
        .iter()
        .flatten()
        .copied()
        .find(|&k| kinds[k].name.starts_with("bl_"))
        .expect("the hero's laser");
    let from = play.hero.collision_centre() + glam::Vec3::Z * 2.0;
    let at = play.battle.combat.targets[medusa].centre;
    play.battle.combat.fire(kind, None, from, at - from, glam::Vec3::ZERO, 1.0, None);
    let mut turned = false;
    for _ in 0..120 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        turned |= matches!(play.robots[r].1.behaviour.task(), Task::Attack { target: Some(id), .. } if id == play.hero_id);
    }
    assert!(turned, "shot, it attacks the hero: {:?}", play.robots[r].1.behaviour.task());
}

/// The Iron Monster's third objective, the enemy's heavy warbot destroyed: the player's script
/// ticks it once function 52 answers `ERROR` for logical id 22, the `21bwlk1`, which it does
/// only once the dead unit has been deleted; until then its owner word reads 65534 (docs/26, "A
/// dead unit is deleted"; docs/15, "65534 is a destroyed object's owner").
#[test]
#[ignore = "needs the game install"]
fn c02_m01s_heavy_warbot_objective_completes_once_the_dead_warbot_is_deleted() {
    let mut play = campaign_play(gamedir::C02_MISSION_01);
    let tick = 1000.0 / 60.0;
    let t = play.units.iter().position(|u| u.logical_id == 22).expect("the heavy warbot");
    assert!(play.battle.combat.targets[t].parts[0].life.is_some());
    let state =
        |play: &parkan_world::play::Play| play.progression.as_ref().unwrap().progress.objectives[2].state;
    for _ in 0..(5 * 60) {
        play.tick(tick, [0.0; 2]);
    }
    assert_eq!(state(&play), 0, "not yet");
    assert_eq!(play.progression.as_ref().unwrap().progress.owner(22), 1, "the enemy's");

    // Its base node destroyed: it dies on the next tick.
    let life = play.battle.combat.targets[t].parts[0].life.as_mut().unwrap();
    life.hit(0, f32::MAX / 4.0);
    play.tick(tick, [0.0; 2]);
    assert!(!play.battle.combat.targets[t].alive);
    assert_eq!(play.progression.as_ref().unwrap().progress.owner(22), 65534, "destroyed, not yet deleted");
    let lasts = play.battle.death_ms[t];
    for _ in 0..((lasts / 1000.0).ceil() as usize * 60 + 5 * 60) {
        play.tick(tick, [0.0; 2]);
    }
    assert!(play.deleted[t], "deleted after its controller's {lasts} ms");
    assert_eq!(play.progression.as_ref().unwrap().progress.owner(22), u32::MAX, "no object answers id 22");
    assert_eq!(state(&play), 1, "the objective is complete");
}

/// C02 Mission 01's Factory is the enemy's, and building, when the hero takes it from its pod:
/// the build is dropped and the screen opens on the mission's prebuilt SWW-X Warrior with
/// nothing in production, as the recording shows at 9:36. While a build of the player's runs, a
/// recent project's button shows that design and the active project's the build, and the
/// screen opened again goes back to the build.
#[test]
#[ignore = "needs the game install"]
fn c02_m01s_captured_factory_drops_the_enemys_build_and_its_designs_can_be_looked_through_while_one_runs() {
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::factory::Click;
    use parkan_world::hud::Pages;
    use parkan_world::play::{Mode, Play};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut play = campaign_play(gamedir::C02_MISSION_01);
    let plant = play.units.iter().position(|u| u.type_word == 0x8000_0010).expect("the enemy's Factory");
    let f = play.factories.iter().position(|f| f.target == plant).unwrap();
    let shown = |play: &Play| play.factories[f].shown().map(|p| p.name.clone()).unwrap_or_default();
    let building = |play: &Play| play.factories[f].build.as_ref().map(|b| b.project.name.clone());
    play_for(&mut play, 30.0, |_| {});
    assert!(building(&play).is_some(), "the enemy is building");

    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let update = |cockpit: &mut Cockpit, play: &mut Play| {
        let now = play.hero.time_ms;
        cockpit.update(play, now);
    };
    assert!(play.stand_on_pod(plant));
    play_for(&mut play, 6.0, |_| {});
    update(&mut cockpit, &mut play);
    assert_eq!(play.units[plant].clan, Some(play.player_clan));
    assert_eq!(play.mode(), Mode::Factory(plant));
    assert_eq!((building(&play), play.factories[f].batch), (None, false), "the enemy's build is dropped");
    assert!(shown(&play).starts_with("SWW-X"), "{}", shown(&play));

    // The player builds it, and accepts a second design while the build runs: the screen stays
    // open over the designer, so the new design stays shown.
    play.factory_click(plant, Click::Build);
    let made = building(&play).expect("production starts");
    let mut other = play.factories[f].projects[0].clone();
    other.name = "SWW-Y Warrior".to_owned();
    play.factories[f].accept(other);
    update(&mut cockpit, &mut play);
    assert_eq!(shown(&play), "SWW-Y Warrior");
    play.factory_click(plant, Click::Active);
    assert_eq!(shown(&play), made);
    play.factory_click(plant, Click::Recent(0));
    assert_eq!(shown(&play), "SWW-Y Warrior");
    play_for(&mut play, 1.0, |_| {});
    assert_eq!(building(&play).as_deref(), Some(made.as_str()), "the build runs on");

    // Left and opened again, the screen shows the build, the active project's button lit.
    play.factory_click(plant, Click::Exit);
    update(&mut cockpit, &mut play);
    assert_eq!(play.mode(), Mode::OnFoot);
    play.modes.push(Mode::Factory(plant));
    update(&mut cockpit, &mut play);
    assert_eq!((play.factories[f].selected, shown(&play)), (None, made));
}

#[test]
#[ignore = "needs the game install"]
fn c02_m03s_neutral_mine_holds_its_pod_still_while_it_opens_and_the_hero_on_it_captures() {
    use parkan_world::buildings::Phase;

    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let mine = play
        .units
        .iter()
        .position(|u| u.type_word == 0x8000_0004 && u.clan == Some(2))
        .expect("the neutral mine");
    let b = play.buildings.iter().position(|x| x.target == mine).expect("it has a pod");
    let part = play.buildings[b].part;
    assert!(play.stand_on_pod(mine), "the hero stands on its pod");

    // Through the whole stroke the zone stays under the hero standing on it, and the pod opens.
    let (mut opened, mut left) = (false, 0);
    for tick in 0..(6 * 60) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let centre = play.hero.collision_centre();
        let target = &play.battle.combat.targets[mine];
        if !play.buildings[b].in_zone(&target.parts[part], centre) {
            left += 1;
            if left == 1 {
                eprintln!("out of the zone from tick {tick}");
            }
        }
        opened |= play.buildings[b].pod.as_ref().is_some_and(|p| p.phase == Phase::Open);
    }
    assert_eq!(left, 0, "the zone stayed under the hero every tick");
    assert!(opened, "the pod opened");
    assert_eq!(play.units[mine].clan, Some(play.player_clan), "and it captured the mine");
}

/// C01 Mission 04's second bonus objective, the enemy base captured or destroyed, ends on
/// `fn31(1, CLASS_BUILDING)` reaching 0 (docs/34, "Function 31"). The enemy holds five
/// buildings, so it stands in progress while the mission starts and the objectives screen
/// opens; a count that walked the units alone would show it complete before the hero moves.
#[test]
#[ignore = "needs the game install"]
fn c01_m04s_enemy_base_objective_counts_the_buildings_the_enemy_still_holds() {
    use parkan_sim::behaviour::BUILDING_BIT;

    let mut play = campaign_play(gamedir::C01_MISSION_04);
    let base = i64::from(BUILDING_BIT);
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.objective_texts[3], "2. Capture or destroy the enemy base");
    assert_eq!(p.progress.robots(1, base), 5, "the enemy's bunker, generator, factory, hangar and teleport");
    assert_eq!(p.progress.robots(1, 0x0100_0000), 7, "and its robots are counted apart");

    // Ten seconds of the Mission handler, the hero standing still.
    for _ in 0..(10 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert!(p.progress.objectives.iter().all(|o| o.state == 0), "every objective is still in progress");

    // The enemy's buildings taken and destroyed, the count falls to 0 and the objective ends.
    let enemy: Vec<i32> = p.progress.buildings.iter().filter(|b| b.clan == 1).map(|b| b.id).collect();
    let p = play.progression.as_mut().unwrap();
    p.progress.captured(enemy[0], 0);
    for id in &enemy[1..] {
        p.progress.destroyed(*id);
    }
    assert_eq!(p.progress.robots(1, base), 0);
    p.run("Mission");
    assert_eq!(p.progress.objectives[3].state, 1, "the bonus objective is complete");
}

/// C03 Mission 02, *The Convoy*: the second enemy clan's script, `c3m2e2`, starts three timers in
/// its `Init` and its problem handler runs them on the clan's takt (docs/34, "The Convoy's two
/// raids"). Two of them send a warbot with winged SSMs at the player's Small Bunker — unit 15, a
/// Medium Wheel Chassis, at 622 − 300 × difficulty s, and unit 14, a Large Wheel Chassis, at
/// 1120 − 400 × difficulty, which goes on to the player's factory. Each raid says its message and
/// is latched behind its own flag, so it runs once: the script runs twice, and no more.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_enemy_raids_the_players_bunker_twice_on_its_own_timers() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::orders::{self, Target};
    use parkan_sim::progression::Notice;
    use parkan_world::progress::ScriptOrder;

    let (mut p, _) = campaign_progression(gamedir::C03_MISSION_02);
    let (bunker, plant) = (0x8000_0001_u32 as i32, 0x8000_0003_u32 as i32);
    assert_eq!(p.progress.owner(bunker), 0, "the Small Bunker is the player's");
    assert_eq!(p.progress.owner(plant), 0, "and so is the factory");
    let enemy = p.others.iter().position(|o| o.clan == 2).expect("Enm2 runs c3m2e2");
    // The `Init`s gave two orders and left the raiders waiting: the player's script shuts the
    // large arach (12) down, and `c3m2e2` sends unit 8 on patrol.
    let init: Vec<(i32, i32)> = p.orders.iter().map(|o| (o.id, o.order.code)).collect();
    assert_eq!(init, [(12, orders::SHUTDOWN), (8, orders::PATROL)]);
    p.orders.clear();

    // Twenty minutes of takts, the units standing where they were placed. Both enemy clans
    // plan as well, so their build orders are counted apart from the two raids.
    let (mut raids, mut messages, mut builds) = (Vec::new(), Vec::new(), 0);
    let mut now = 0.0;
    while now < 1_200_000.0 {
        now += 100.0;
        for notice in p.tick(now, |_| None) {
            if let Notice::Message { id, first: true } = notice {
                messages.push(id);
            }
        }
        if !p.orders.is_empty() {
            let given: Vec<ScriptOrder> = std::mem::take(&mut p.orders);
            builds += given.iter().filter(|o| o.order.code == orders::CONSTRUCT).count();
            let given: Vec<ScriptOrder> =
                given.into_iter().filter(|o| o.order.code != orders::CONSTRUCT).collect();
            if !given.is_empty() {
                raids.push((now / 1000.0, p.others[enemy].takt.clock(), given));
            }
        }
    }
    assert_eq!(raids.len(), 2, "two raids and no more: {raids:#?}");
    assert_eq!(messages, [0, 1], "each raid says its own message");
    assert!(builds > 0, "and the clans' planners ask their factories for warbots besides");

    // `fDifficulty` stands at `varset.var`'s own 0.5, so the first timer is 472 s and the second
    // 920; the clock steps a flat 7 s a takt, and a timer is passed once the clock is above it.
    let (at, clock, orders) = &raids[0];
    assert_eq!(*clock, 476, "the first raid on the takt whose clock passes 472");
    assert!((469.0..537.0).contains(at), "which comes at {at} s, 68 takts of 7 to 8 s in");
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].id, 15, "the Medium Wheel Chassis with two winged SSMs");
    assert_eq!(orders[0].insert, orders::INSERT_REPLACE);
    assert_eq!(orders[0].order.code, orders::ATTACK);
    assert_eq!(orders[0].order.target, Target::LogicId(bunker));
    assert!(
        matches!(Task::from_order(&orders[0].order), Task::Attack { target: Some(id), .. } if id == bunker)
    );

    let (at, clock, orders) = &raids[1];
    assert_eq!(*clock, 924, "and the second on the one that passes 920");
    assert!((917.0..1049.0).contains(at), "which comes at {at} s");
    // The bunker first, then the factory behind it; the player holds no mine, so the third order
    // the script would give is never built.
    let given: Vec<(i32, i32, u32, Target)> =
        orders.iter().map(|o| (o.id, o.order.code, o.insert, o.order.target)).collect();
    assert_eq!(
        given,
        [
            (14, orders::ATTACK, orders::INSERT_REPLACE, Target::LogicId(bunker)),
            (14, orders::ATTACK, orders::INSERT_TO_END, Target::LogicId(plant)),
        ]
    );
}

/// On the easy level, which both recordings of the mission play, `fDifficulty` is 0 and the
/// two raids come on the timers as `c3m2e2`'s `Init` writes them: 622 and 1120. The let's
/// play's Part 6 has the first warning at 12:44, the clock past 622, and the second at 22:32.
/// The enemy clans' units take half their hit points, the easy level's ratio.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_raids_come_at_622_and_1120_on_the_easy_level() {
    use parkan_sim::orders;
    use parkan_sim::progression::Notice;

    let (mut p, _) = campaign_progression_at(gamedir::C03_MISSION_02, 0);
    let enemy = p.others.iter().position(|o| o.clan == 2).expect("Enm2 runs c3m2e2");
    assert_eq!(p.others[enemy].script.float("fDifficulty"), Some(0.0), "easy is 0");
    p.orders.clear();
    let (mut raids, mut messages) = (Vec::new(), Vec::new());
    let mut now = 0.0;
    while now < 1_400_000.0 {
        now += 100.0;
        for notice in p.tick(now, |_| None) {
            if let Notice::Message { id, first: true } = notice {
                messages.push(id);
            }
        }
        // Both enemy clans' planners order builds on the same takts; the raid is the attack.
        let given = std::mem::take(&mut p.orders);
        if let Some(attack) = given.iter().find(|o| o.order.code == orders::ATTACK) {
            raids.push((p.others[enemy].takt.clock(), attack.id));
        }
    }
    // The clock steps 7 a takt, and a timer is passed once the clock is above it.
    assert_eq!(raids, [(623, 15), (1127, 14)], "unit 15 past 622, unit 14 past 1120");
    assert_eq!(messages, [0, 1], "each raid says its own message");

    let play = campaign_play_at(gamedir::C03_MISSION_02, Some(0));
    assert_eq!(play.ratio, 0.5, "the easy level's ratio, `Iron_3D.ini`'s `[LEVEL_RATIO] EASY`");
    assert_eq!(play.level, Some(0));
}

/// C03 Mission 02's bonus objective, the enemy's mobile forces destroyed: `c3m2p`'s `Mission`
/// completes it once `fn31(1, CLASS_ROBOT)` and `fn31(2, CLASS_ROBOT)` are both 0, and reopens
/// it with `OBJECTIVE_PROGRESS` when clan 1 has a robot again -- its node 60 asks Enemy 1's
/// count alone, so a unit Enemy 2 makes leaves it complete. The let's play's Part 6.5 shows
/// *"Objective is completed"* for it at 33:32. Played through the play itself: the six placed
/// warbots lost, the objective completes and says so; Enemy 2's factory makes units and it
/// stays; Enemy 1's makes one and it is in progress again, without a word.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_bonus_objective_completes_with_the_last_enemy_robot_and_reopens_at_enemy_1s_next() {
    use parkan_world::progress::Say;

    const ROBOTS: i64 = 0x0100_0000;
    let mut play = campaign_play_at(gamedir::C03_MISSION_02, Some(0));
    for _ in 0..(5 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert_eq!(play.progression.as_ref().unwrap().progress.objectives[3].state, 0);
    let robots: Vec<usize> = (0..play.units.len())
        .filter(|&t| matches!(play.units[t].clan, Some(1 | 2)) && play.robots.iter().any(|(r, _)| *r == t))
        .collect();
    assert_eq!(robots.len(), 6, "two of Enemy 1's and four of Enemy 2's are placed");
    for &t in &robots {
        let events = play.battle.combat.life_kill(t);
        play.pending_events.extend(events);
    }
    play.says.clear();
    let made = play.units.len();
    let (mut completed, mut reopened, mut held) = (None, None, false);
    for tick in 0..(120 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let p = play.progression.as_ref().unwrap();
        let state = p.progress.objectives[3].state;
        let (first, second) = (p.progress.robots(1, ROBOTS), p.progress.robots(2, ROBOTS));
        if completed.is_none() && state == 1 {
            completed = Some(tick);
            assert_eq!(play.units.len(), made, "before either factory has made another");
            assert!(
                play.says.iter().any(|s| matches!(s, Say::Text(_, t) if t == "Objective is completed")),
                "and the message box says so"
            );
            play.says.clear();
        }
        // Enemy 2's robots alone do not reopen it.
        held |= completed.is_some() && state == 1 && first == 0 && second > 0;
        if completed.is_some() && state == 0 {
            assert!(first > 0, "it reopens on Enemy 1's count");
            reopened = Some(tick);
            break;
        }
    }
    let completed = completed.expect("the bonus objective completed");
    assert!(completed < 120, "on the next `Mission` handler, within two seconds: tick {completed}");
    assert!(held, "it stood complete while Enemy 2 alone had made a unit");
    reopened.expect("and it is in progress again once Enemy 1's factory has made one");
    assert!(
        !play.says.iter().any(|s| matches!(s, Say::Text(_, t) if t.starts_with("Objective"))),
        "`OBJECTIVE_PROGRESS` says nothing"
    );
    let p = play.progression.as_ref().unwrap();
    assert!(p.progress.objectives[..3].iter().all(|o| o.state == 0), "the primaries are untouched");
}

/// And the raid order reaches the warbot: given the attack its clan's script gives at the first
/// timer, unit 15 takes it up and drives at the player's bunker across the map (docs/31, "The
/// attack"), rather than standing where it was placed.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_raider_takes_up_the_attack_on_the_bunker_and_closes_on_it() {
    use parkan_sim::behaviour::Task;
    use parkan_sim::orders::{self, Order, Target};
    use parkan_world::progress::ScriptOrder;

    let mut play = campaign_play(gamedir::C03_MISSION_02);
    let bunker = 0x8000_0001_u32 as i32;
    let raider = play.units.iter().position(|u| u.logical_id == 15).expect("the medium raider");
    let target = play.units.iter().position(|u| u.logical_id == bunker).expect("the player's bunker");
    let order = Order { code: orders::ATTACK, parameter: 0, target: Target::LogicId(bunker) };
    play.progression.as_mut().unwrap().orders.push(ScriptOrder {
        id: 15,
        order,
        insert: orders::INSERT_REPLACE,
    });

    let bunker_at = play.battle.combat.targets[target].position;
    let start = play.battle.combat.targets[raider].position.distance(bunker_at);
    for _ in 0..(30 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let (_, robot) = play.robots.iter().find(|(t, _)| *t == raider).expect("a robot");
    assert!(
        matches!(robot.behaviour.task(), Task::Attack { target: Some(id), .. } if id == bunker),
        "it is attacking the bunker, not {:?}",
        robot.behaviour.task()
    );
    let now = play.battle.combat.targets[raider].position.distance(bunker_at);
    eprintln!("the raider closed from {start:.0} to {now:.0} of the bunker in 30 s");
    assert!(now < start - 100.0, "it closed on the bunker: {start:.0} → {now:.0}");

    // And the clock behind the timers runs in a real play: four or five takts of 7 to 8 s in
    // half a minute, each stepping the enemy clan's seconds clock 7 on.
    let p = play.progression.as_ref().unwrap();
    let clock = p.others.iter().find(|o| o.clan == 2).expect("Enm2").takt.clock();
    assert!((28..=42).contains(&clock), "the enemy clan's clock reads {clock} after 30 s");
}

/// C03 Mission 02's raider lands its winged SSM on the player's bunker from afar, as the let's
/// play's Part 6 has it: the MWW-4 stands 345 to 292 m off and its missile strikes at 13:54. An
/// attack on a building holds the building as its fire target from the first pick (docs/31,
/// "Making a move"), so the guided gun's gate opens at 500 m and its 7 s lock runs out short of
/// 400; and a round of more than 10,000 is held for a building (docs/29, "How the AI fires"), so
/// the hero the raider passes on its way draws none.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_raider_lands_its_winged_ssm_on_the_bunker_from_afar_and_spends_none_on_the_hero() {
    use parkan_sim::combat::Event;
    use parkan_sim::orders::{self, Order, Target};
    use parkan_world::progress::ScriptOrder;

    let mut play = campaign_play(gamedir::C03_MISSION_02);
    let bunker_id = 0x8000_0001_u32 as i32;
    let raider = play.units.iter().position(|u| u.logical_id == 15).expect("the medium raider");
    let bunker = play.units.iter().position(|u| u.logical_id == bunker_id).expect("the player's bunker");
    let order = Order { code: orders::ATTACK, parameter: 0, target: Target::LogicId(bunker_id) };
    play.progression.as_mut().unwrap().orders.push(ScriptOrder {
        id: 15,
        order,
        insert: orders::INSERT_REPLACE,
    });
    let ssm = play.battle.combat.kinds.iter().position(|k| k.name.eq_ignore_ascii_case("bm_m_04")).unwrap();
    let bunker_at = play.battle.combat.targets[bunker].position;

    let (mut fired, mut seen) = (Vec::new(), std::collections::BTreeSet::new());
    let mut struck = None;
    for _ in 0..(90 * 60) {
        let events = play.tick(1000.0 / 60.0, [0.0; 2]);
        let from = play.battle.combat.targets[raider].position.distance(bunker_at);
        for r in play.battle.combat.rounds.iter().filter(|r| r.owner == Some(raider) && r.kind == ssm) {
            if seen.insert(r.id) {
                fired.push((r.target, from));
            }
        }
        for e in events {
            if let Event::Struck { round, target, .. } = e
                && round.owner == Some(raider)
                && round.kind == ssm
                && struck.is_none()
            {
                struck = Some((target, from));
            }
        }
        if struck.is_some() {
            break;
        }
    }
    assert!(!fired.is_empty(), "the raider fired a winged SSM");
    assert!(fired.iter().all(|&(t, _)| t == Some(bunker)), "every one at the bunker: {fired:?}");
    let (_, from) = fired[0];
    let (target, at) = struck.expect("and lands");
    eprintln!("the first leaves {from:.0} m off the bunker and lands with the raider {at:.0} m off");
    // How far off the first one leaves is not pinned to the recording's 345 to 292 m. The gun is
    // ready by 400 m, as read above, and then waits on a clear line from its muzzle to the
    // bunker's centre, which lies 3 to 4 m under the plateau the raider crosses: the line
    // grazes the ground 90 to 160 m ahead of it nearly all the way in. It cleared for a moment
    // at 373 m on the way the raider took while the pair pushed agent spheres out, and on the
    // way it takes now -- the same road a few metres aside, the mission's animals and tracked
    // bots moving otherwise about it -- it first clears at 106 m. That the missile leaves from
    // afar at all hangs on that line, which is the open item here.
    assert!(from > 80.0, "the first leaves {from:.0} m off the bunker");
    assert_eq!(target, Some(bunker), "on the bunker");
    assert!(at > 50.0, "with the raider still {at:.0} m off");
}

/// C03 Mission 02's raider outlives its own winged SSM. A hit names the object that fired it and
/// that object's nodes take nothing of it (`Control.dll:0x1000ed4e`, docs/26, "Whose hit it is,
/// and whom it spares"), so the 45 m, 60,000 blast of a `bm_m_04` the MWW-4 lands 25 m from
/// itself leaves every node of it whole; the same round out of another's gun, landed on the same
/// spot, destroys it. The test is one id against another, so who else stands in the blast is hurt
/// either way.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_raider_outlives_its_own_winged_ssm_at_25_m_and_not_anothers() {
    use parkan_sim::combat::Event;

    let land = |own: bool| {
        let mut play = campaign_play(gamedir::C03_MISSION_02);
        let raider = play.units.iter().position(|u| u.logical_id == 15).expect("the medium raider");
        let bunker =
            play.units.iter().position(|u| u.logical_id == 0x8000_0001_u32 as i32).expect("the bunker");
        let ssm = play.battle.combat.kinds.iter().position(|k| k.name.eq_ignore_ascii_case("bm_m_04"));
        let ssm = ssm.expect("the winged SSM is loaded with the raider's guns");
        let life = |play: &parkan_world::play::Play| -> f32 {
            let parts = &play.battle.combat.targets[raider].parts;
            parts.iter().filter_map(|p| p.life.as_ref()).map(|l| l.total()).sum()
        };
        let whole = life(&play);
        let at = play.battle.combat.targets[raider].position;
        let muzzle = glam::Vec3::new(at.x + 25.0, at.y, at.z + 40.0);
        let owner = if own { raider } else { bunker };
        play.battle.combat.fire(ssm, Some(owner), muzzle, -glam::Vec3::Z, glam::Vec3::ZERO, 1.0, None);
        let mut burst = None;
        for _ in 0..(3 * 60) {
            for e in play.tick(1000.0 / 60.0, [0.0; 2]) {
                if let Event::Struck { round, point, .. } = e
                    && round.kind == ssm
                {
                    burst = Some(point.distance(play.battle.combat.targets[raider].position));
                }
            }
        }
        (burst.expect("the missile lands"), play.battle.combat.targets[raider].alive, whole, life(&play))
    };

    let (from, alive, whole, left) = land(true);
    eprintln!("its own missile bursts {from:.1} m off: {left:.0} of {whole:.0} left");
    assert!((15.0..40.0).contains(&from), "inside its own 45 m blast, {from:.1} m off");
    assert!(alive && left >= whole, "no node of the firer is touched: {left} of {whole}");

    let (from, alive, whole, left) = land(false);
    eprintln!("another's bursts {from:.1} m off: {left:.0} of {whole:.0} left");
    assert!((15.0..40.0).contains(&from));
    assert!(!alive, "the same blast from another's gun destroys it: {left} of {whole}");
}

/// C03 Mission 02's raider is *"Dangerous!"* on the target panel, as the let's play's Part 6 shows
/// it at 13:44: its winged SSM launchers' round does 60,000, at least the 10,000 the panel asks
/// for (docs/35, "Name and status"). The enemy's tracked warbots carry nothing so heavy.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_raider_is_dangerous_and_a_tracked_warbot_is_not() {
    use parkan_world::cockpit::panels::dangerous;

    let play = campaign_play(gamedir::C03_MISSION_02);
    let unit = |id: i32| play.units.iter().position(|u| u.logical_id == id).expect("a placed unit");
    assert!(dangerous(&play, unit(15)), "the Medium Wheel Chassis with two winged SSMs");
    assert!(dangerous(&play, unit(14)), "the Large one with four");
    assert!(!dangerous(&play, unit(6)), "Enemy 1's tracked warbot");
}

/// C03 Mission 02's first objective, every generator captured, follows the player's count both
/// ways: `c3m2p` completes it on `fn34(BUILDING_GENERATOR) == 3` and calls `OBJECTIVE_PROGRESS`
/// when the count falls, which puts it back to open without a word (`iron3d.dll:0x10060e44`). The
/// let's play's Part 6 shows *"Objective is completed"* at 53:57 and, once Enemy 2 has taken its
/// generator back and the player retaken it, again at 59:17.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_generators_objective_reopens_when_one_is_lost_and_completes_again() {
    use parkan_sim::progression::{COMPLETE, Notice, OPEN};

    let (mut p, _) = campaign_progression(gamedir::C03_MISSION_02);
    let (enemy_1, enemy_2) = (0x8000_0004_u32 as i32, 0x8000_000a_u32 as i32);
    let completed = |n: &[Notice]| n.contains(&Notice::ObjectiveComplete { index: 0 });
    assert!(!completed(&p.run("Mission")), "the player holds one generator of three");

    p.progress.captured(enemy_1, 0);
    p.progress.captured(enemy_2, 0);
    assert!(completed(&p.run("Mission")), "all three are the player's");
    assert_eq!(p.progress.objectives[0].state, COMPLETE);

    p.progress.captured(enemy_2, 2);
    assert!(!completed(&p.run("Mission")));
    assert_eq!(p.progress.objectives[0].state, OPEN, "Enemy 2 has taken its generator back");

    p.progress.captured(enemy_2, 0);
    assert!(completed(&p.run("Mission")), "and the player's retaking it completes it again");
}

/// C03 Mission 02 as the let's play's Part 6 has it at 13:54: the player at the Small Bunker's
/// guns is lost as the raider's winged SSM lands on the bunker, and the mission fails. Taking a
/// building's guns (mode 6) leaves the hero standing in the pod room (docs/27, "What the modes
/// show"); only boarding a bot takes it out of the world (docs/39, "Boarding"). So the missile's
/// 45 m blast on the bunker's roof reaches the hero under it.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_hero_at_the_bunkers_guns_is_lost_to_a_winged_ssm_on_the_roof() {
    use parkan_world::play::Mode;

    let mut play = campaign_play(gamedir::C03_MISSION_02);
    let bunker = play.units.iter().position(|u| u.logical_id == 0x8000_0001_u32 as i32).expect("the bunker");
    let raider = play.units.iter().position(|u| u.logical_id == 15).expect("the medium raider");
    assert!(play.stand_on_pod(bunker));
    play_for(&mut play, 1.0, |_| {});
    assert!(play.enter_manual(bunker), "the bunker's guns are open");
    assert_eq!(play.mode(), Mode::Manual(bunker));
    assert!(!play.hero_away(), "the hero stays in the pod room");
    assert!(play.battle.combat.hero.as_ref().is_some_and(|h| h.alive), "and can be struck there");

    // The raider's own round, `bm_m_04`, dropped on the roof straight above the pod.
    let kind = play.battle.combat.kinds.iter().position(|k| k.name.eq_ignore_ascii_case("bm_m_04"));
    let kind = kind.expect("the winged SSM is loaded with the raider's guns");
    let at = play.hero.walker.body.position;
    let muzzle = glam::Vec3::new(at.x, at.y, at.z + 45.0);
    play.battle.combat.fire(kind, Some(raider), muzzle, -glam::Vec3::Z, glam::Vec3::ZERO, 1.0, None);
    play_for(&mut play, 3.0, |_| {});
    assert!(play.hero.dead(), "the blast reached the hero in the pod room");
    assert_eq!(play.progression.as_ref().unwrap().progress.outcome, Some(false), "and the mission fails");
}

/// Mission 01's three clans hold the words its file gives them, and the clan brains' takt
/// keeps them there: each attitude drifts 0.0033 a takt toward its band's rest point and stops
/// (docs/25, "Clan relations"), so `Plr`'s hostility toward `Enm` climbs from 0.16665 to 0.2833
/// and never reaches the 1/3 edge. Only a hit moves a word, and only downwards: 42 hits inside
/// one 7-8 s takt take the player's clan's neutral word for `Trgt` to hostile, both ways.
#[test]
#[ignore = "needs the game install"]
fn mission_01s_relations_drift_within_their_bands_and_only_a_hit_moves_one() {
    use parkan_sim::relations::{LOW_EDGE, REST, SET};
    use parkan_world::play::{RELATION_HOSTILE, RELATION_NEUTRAL};

    let mut play = campaign_play(gamedir::MISSION_01);
    let names: Vec<&str> = play.clans.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Plr", "Trgt", "Enm", "Ntrl"], "{names:?}");
    assert_eq!((play.relations[0][1], play.relations[1][0]), (RELATION_NEUTRAL, RELATION_NEUTRAL));
    assert_eq!((play.relations[0][2], play.relations[2][0]), (RELATION_HOSTILE, RELATION_HOSTILE));
    assert_eq!(play.attitudes.attitude(0, 2), Some(SET[RELATION_HOSTILE as usize]));

    // Twenty seconds is two or three takts of 7 to 8 s: the hostile attitude rises by 0.0033
    // each and the words stand, the drift never crossing a band's edge.
    let before = play.attitudes.attitude(0, 2).unwrap();
    for _ in 0..(20 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let after = play.attitudes.attitude(0, 2).unwrap();
    eprintln!("Plr toward Enm rose from {before} to {after} in 20 s");
    assert!(after > before && after < LOW_EDGE, "it rises toward {} but stops short", REST[0]);
    assert!((after - before - 3.0 * parkan_sim::relations::DRIFT).abs() < 2.0 * parkan_sim::relations::DRIFT);
    assert_eq!(play.relations[0][2], RELATION_HOSTILE);
    assert_eq!(play.relations[0][1], RELATION_NEUTRAL, "no drift crosses an edge");

    // 42 hits, each the 0.004 `Behavior.dll:0x1000658c` adds for one round, inside a takt.
    for _ in 0..42 {
        play.attitudes.hurt(0, 1);
    }
    for _ in 0..(9 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert_eq!(play.relations[0][1], RELATION_HOSTILE, "the player's clan turns on Trgt");
    assert_eq!(play.relations[1][0], RELATION_HOSTILE, "and Trgt hears of it");
    assert_eq!(
        play.progression.as_ref().unwrap().relations[0][1],
        RELATION_HOSTILE,
        "the scripts read the same word"
    );
}

/// C02 Mission 04's gorge is crossed on `e_bridge`, the *Enh Bridge BS-52/30*, placed as two
/// halves π apart whose 185.5 m decks meet in the middle — the shape docs/24 measured on
/// Mission 01's `m_bridge` ("Standing on a bridge"). Each half carries a flat end cap at the
/// join, and on this one alone that cap is flagged `0x20`, the bit its additive `B_A_BRIGE`
/// material carries, where the other three bridges flag theirs 4. Both are see-through to a
/// round, and the collision passes both here, so the hero walks the whole span.
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_energy_bridge_halves_meet_and_the_hero_walks_over_the_join() {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let bridges: Vec<&mission::Object> =
        m.objects.iter().filter(|o| o.path.to_ascii_lowercase().ends_with("e_bridge.dat")).collect();
    assert_eq!(bridges.len(), 2, "the mission places two halves");
    let (near, far) = (bridges[0], bridges[1]);
    let turn = (far.rotation - near.rotation - std::f32::consts::PI).abs();
    assert!(turn < 1e-5, "the halves stand pi apart: {turn}");
    // Model +y is the span; the two origins lie along it, 371.0 apart, so each deck is 185.5
    // long and they meet exactly.
    let along = glam::Vec2::new(-near.rotation.sin(), near.rotation.cos());
    let delta = glam::Vec2::new(far.position[0] - near.position[0], far.position[1] - near.position[1]);
    assert!(delta.perp_dot(along).abs() < 0.01, "the second half is straight on: {delta:?}");
    let span = delta.dot(along);
    assert!((span - 371.02).abs() < 0.05, "371 m between the origins: {span}");

    let mut play = campaign_play(gamedir::C02_MISSION_04);
    let start = glam::Vec2::new(near.position[0], near.position[1]) + along * 3.0;
    assert!(play.stand_below(start.x, start.y, 170.0, near.rotation), "the hero stands on the deck");
    let on_deck = play.hero.walker.body.position.z;
    play.hero.key("SCAN_W", true);
    let (mut farthest, mut over) = (0.0f32, None);
    for tick in 0..(60 * 40) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let at = play.hero.walker.body.position;
        let walked = (at.truncate() - glam::Vec2::new(near.position[0], near.position[1])).dot(along);
        farthest = farthest.max(walked);
        if over.is_none() && walked > span / 2.0 + 5.0 {
            over = Some((tick, at.z, play.hero.walker.ground.is_some_and(|g| g.solid.is_some())));
        }
    }
    let (tick, z, on_building) = over.unwrap_or_else(|| {
        panic!("it stopped {:.1} m along, short of the join at {:.1}", farthest, span / 2.0)
    });
    assert!(on_building, "past the join it stands on the far half's deck, not on the ground");
    assert!(z > on_deck, "the deck climbs from the bank: {z} against {on_deck}");
    assert!(farthest > span - 10.0, "and on to the far bank: {farthest:.1} of {span:.1}");
    eprintln!("over the join {:.1} s in, {farthest:.1} m along", tick as f32 / 60.0);
}

/// The same bridge's deck is five `f_brige_ray` sprites, each hung on a `RayD`/`RayW`/`RayH`
/// triple of control points whose direction lengths are the ray's depth, width and height
/// (docs/07, "CTPT — control points"). Those three directions are the frame's three axes, and
/// a sprite is drawn **through** that frame (docs/11, "A sprite is drawn through its frame"),
/// so a deck ray is a 150 m streak down the span and 1.93 m across seen from the bank, and a
/// 1.93 m flicker seen down the deck. The three deck rays hang on nodes 2, 3 and 4 of the
/// mesh, three spokes 120° apart on a circle of radius 20 about node 1, and the controller's
/// one channel turns node 1 a whole turn every ten seconds, so they sweep around the bridge's
/// own axis (docs/28, "What a device's value turns").
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_bridge_rays_run_the_span_and_sweep_around_the_decks_axis() {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let near = m.objects.iter().find(|o| o.path.to_ascii_lowercase().ends_with("e_bridge.dat")).unwrap();
    let span = glam::Vec3::new(-near.rotation.sin(), near.rotation.cos(), 0.0);

    let mut play = campaign_play(gamedir::C02_MISSION_04);
    // Straight off the effect instances: a scene's own draw needs the texture store's looks,
    // which a headless play does not build.
    let rays = |play: &parkan_world::play::Play| {
        let mut drawn = Vec::new();
        for (_, instance) in &play.fx.instances {
            instance.sprites(play.hero.time_ms, true, &mut drawn);
        }
        drawn.retain(|s| s.material.eq_ignore_ascii_case("b_a_brige"));
        drawn
    };
    let first = rays(&play);
    assert_eq!(first.len(), 10, "five rays on each of the two halves");
    let mut deck = 0;
    for s in &first {
        let axes = s.frame.expect("each ray hangs on three distinct points");
        assert!((s.width - 1.932).abs() < 0.01 || (s.width - 0.6).abs() < 0.01, "width {}", s.width);
        // The first axis is the depth point's, which runs the deck's 150 m or a tower's 147.
        let depth = axes[0].length();
        assert!((depth - 150.0).abs() < 0.01 || (depth - 147.0).abs() < 0.01, "depth {depth}");
        if (depth - 150.0).abs() < 0.01 {
            deck += 1;
            let along = axes[0].normalize().dot(span).abs();
            assert!(along > 0.999, "a deck ray lies along the span: {along}");
        }
    }
    assert_eq!(deck, 6, "three deck rays on each half");

    // A third of a turn on, each spoke stands where the next one did, 120° round a circle of
    // radius 20 — a chord of 34.6 m.
    let at = |rays: &[parkan_sim::effects::Sprite]| -> Vec<glam::Vec3> {
        rays.iter()
            .filter(|s| s.frame.is_some_and(|a| (a[0].length() - 150.0).abs() < 0.01))
            .map(|s| s.centre)
            .collect()
    };
    // The arms carrying them turn with them, and the scene has to draw the bridge node by
    // node to show it: the part takes no damage, so without [`Building::plays_nodes`] it is
    // drawn as one model at its placement and the rays sweep round arms that stand still.
    let target = object_target(&play, &m, "e_bridge.dat");
    assert!(
        play.battle.combat.targets[target].parts.iter().all(|p| p.life.is_none()),
        "a bridge half takes no damage, so nothing else makes the scene draw it node by node"
    );
    let half = play.buildings.iter().find(|b| b.target == target).expect("the half is a building");
    assert!(half.plays_nodes(), "and its items play its nodes");
    assert!(play.drawn_by_node(target), "so the scene draws it node by node");
    let arms = |play: &parkan_world::play::Play, t: usize| -> Vec<f64> {
        (2..5).map(|n| play.battle.combat.targets[t].parts[0].nodes[n].translation[2]).collect()
    };
    let arms_at_rest = arms(&play, target);

    let before = at(&first);
    while play.hero.time_ms < 10_000.0 / 3.0 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let after = at(&rays(&play));
    let moved = before.iter().zip(&after).map(|(a, b)| a.distance(*b)).fold(f32::MAX, f32::min);
    assert!((moved - 34.64).abs() < 1.0, "a spoke carries its ray 120° round: {moved}");
    for b in &after {
        assert!(before.iter().any(|a| a.distance(*b) < 1.0), "onto the next spoke's place: {b}");
    }
    let swung = arms(&play, target).iter().zip(&arms_at_rest).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
    assert!(swung > 20.0, "the arms swing with them: {swung:.1} m");
    eprintln!("the deck's three rays sweep {moved:.1} m a third of a turn, their arms {swung:.1} m");
}

/// C02 Mission 03's Large Factory, taken by a wheeled warbot the player drives himself.
///
/// The ground contact's lift is the largest rise over the flag-1 contacts, whatever its height
/// (docs/24, "Holding the body on the ground"). A wheel's own up pass had reached as far as the
/// agent sphere's r, so on the first ramp down past the factory's door the two rear wheels kept
/// finding the floor they had just left, 2.9 m above themselves, while the front two found the
/// ramp below: the lift took the larger and hoisted the machine back up the ramp, into the
/// structure over it, and the collision pass then put the whole move back where it started —
/// every tick, for good. The bound is the body sphere's r2 now.
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_wheeled_warbot_drives_down_the_factorys_first_ramp_to_its_pod() {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let plant = object_target(&play, &m, "lplant01.dat");
    let bunker = object_target(&play, &m, "sbunk02.dat");

    // An SWW-X Warrior, the shipped wheeled design: chassis R_L_03 and two guns.
    let path = "UNITS\\UNITS\\PREBLD\\tut3_p2.dat".to_owned();
    let data = parkan_formats::gamedir::resolve(&play.assembly.game, &path)
        .and_then(|p| std::fs::read(p).ok())
        .expect("tut3_p2.dat");
    let project = parkan_world::factory::Project {
        path,
        name: String::new(),
        type_word: u32::from_le_bytes(data[4..8].try_into().unwrap()),
        chassis_size: 2,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let origin = play.battle.combat.targets[plant].parts[0].nodes[0].translation;
    let origin = glam::Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);
    let spot = origin + glam::Vec3::new(70.0, 70.0, 0.0);
    let z = play.ground.below(spot.x, spot.y, 1.0e5).map_or(spot.z, |h| h.point.z);
    let clan = play.player_clan;
    let bot = play.spawn(&project, clan, spot.with_z(z + 1.0), 0.0).expect("a wheeled warbot");
    play.tick(1000.0 / 60.0, [0.0; 2]);

    // The way in the capture order would take, driven by hand instead of by the AI.
    let pod = play
        .capture_places()
        .into_iter()
        .find(|p| p.id == play.units[plant].logical_id)
        .and_then(|p| p.pod)
        .expect("the factory has a pod");
    let from = play.robots.iter().find(|(t, _)| *t == bot).unwrap().1.walker.body.position;
    let mut way = play.way_in(plant, from, pod, false).expect("a way in");
    way.push(pod);
    let ramp = way.windows(2).position(|w| w[0].z - w[1].z > 4.0).expect("a ramp down on the way in");

    play.enter_command(bunker);
    assert!(play.telepresence(bot, 0), "the player takes the warbot over");
    play.key("SCAN_W", true);
    let (mut next, mut furthest) = (0, 0);
    for tick in 0..(120 * 60) {
        let (_, r) = play.robots.iter_mut().find(|(t, _)| *t == bot).unwrap();
        let at = r.walker.body.position;
        while next < way.len() && way[next].truncate().distance(at.truncate()) < 4.0 {
            next += 1;
            furthest = furthest.max(next);
        }
        if next == way.len() {
            eprintln!("on the pod {} s in, {:.1} m below the door", tick / 60, way[ramp].z - at.z);
            assert!(at.z < way[ramp].z - 4.0, "and it is down the ramp: {at}");
            return;
        }
        let to = way[next].truncate() - at.truncate();
        r.walker.body.yaw = (-to.x).atan2(to.y);
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let at = play.robots.iter().find(|(t, _)| *t == bot).unwrap().1.walker.body.position;
    panic!(
        "it never reached the pod: stopped at {at} on leg {furthest} of {}, the ramp down being leg {}",
        way.len(),
        ramp + 1
    );
}

/// C02 Mission 03's Small Warehouse holds the warbot the mission is about, `22mwlk1`, logical
/// id 28: a medium walker placed standing on the warehouse's floor, its feet at 36.63 on the
/// floor face at 36.62, its origin 3.0 m over it.
///
/// The recording shows it upright on its legs, taller than the hero, in the briefing and in
/// play ("Let's Play - Parkan: Iron Strategy, Part 4", 1:43 and 15:13–15:15, every frame at
/// 60 fps). The warehouse's ceiling is 6.3–7.5 m over the floor. What the pair pushes out is the
/// walker's node sphere, 4.26 about a centre 0.67 under its origin, which tops out 6.63 up: the
/// ceiling presses it 0.28 m a tick, a machine standing on a building takes a down push whole,
/// and the push lands after the ground contact (docs/24, "Collision between objects"). Pushed
/// out as its 5.87 m agent sphere it went down 2.85 m, its hull at the floor and its legs through
/// it. It is drawn before the push, where the contact holds it, and the simulation keeps the
/// push.
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_warehouse_warbot_is_drawn_on_its_legs_under_the_ceiling() {
    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let t = (0..play.units.len()).find(|&t| play.units[t].logical_id == 28).expect("the warbot, id 28");
    fn robot(play: &parkan_world::play::Play, t: usize) -> &parkan_world::hero::Robot {
        &play.robots.iter().find(|(r, _)| *r == t).unwrap().1
    }
    let placed = robot(&play, t).walker.body.position;
    let floor = play.ground.below(placed.x, placed.y, placed.z).expect("the warehouse's floor").point.z;
    assert!((placed.z - floor - 3.04).abs() < 0.1, "placed standing: {} over {floor}", placed.z);
    let (mut drawn_lowest, mut posed_lowest, mut body_lowest) = (f32::MAX, f32::MAX, f32::MAX);
    for _ in 0..(10 * 60) {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let r = robot(&play, t);
        body_lowest = body_lowest.min(r.walker.body.position.z - floor);
        drawn_lowest = drawn_lowest.min(r.walker.drawn(r.time_ms).0.z - floor);
        // What the renderer draws: the chassis's root node as the tick posed it.
        let chassis = r.chassis_part;
        let root = play.battle.combat.targets[t].parts[chassis].nodes[0].translation[2] as f32;
        posed_lowest = posed_lowest.min(root - floor);
    }
    assert!(drawn_lowest > 2.9, "drawn {drawn_lowest:.2} m over its floor, its legs through it");
    assert!(posed_lowest > 2.9, "posed {posed_lowest:.2} m over its floor, its legs through it");
    // The body the simulation holds dips by what the ceiling presses the node sphere, 0.28 m.
    assert!((2.7..2.9).contains(&body_lowest), "its body is pressed to {body_lowest:.2} m over its floor");
    let at = robot(&play, t).walker.body.position;
    assert!(at.truncate().distance(placed.truncate()) < 3.0, "and it stays where it stood: {at}");
    eprintln!(
        "the warbot is drawn {drawn_lowest:.2} m over its floor at the lowest; its body is held {:.2}",
        at.z - floor
    );
}

/// A tower's gun mast stands in the ground until its class-29 item raises it.
///
/// Nothing files that item: `CBuilding` looks for doors and control pods and no other class
/// (`Terrain.dll:0x100583a2`), while the component factory puts **every** class it builds in
/// the controller's timed list (`Control.dll:0x1002d70a`), which the time driver steps. Left
/// unstepped, the Small Tower's `o07` node stays at the rest pose its mesh's frame 0 gives
/// it, 10.4 m below where frame 4 puts it — and the turret, guns, radar and deflector that
/// hang on nodes of that mast stay buried with it.
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_tower_raises_its_gun_mast_out_of_the_ground_and_leaves_it_up() {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_04);
    let tower = object_target(&play, &m, "mtow01.dat");

    let building = play.buildings.iter().find(|b| b.target == tower).expect("the tower is a building");
    // Two items run by themselves: the second computer, which `CBuilding` never switches
    // (`Terrain.dll:0x1005858e`), turning its node 10 round for ever, and the mast.
    let nodes: Vec<Vec<i32>> =
        building.running.iter().map(|i| i.channels.iter().map(|c| c.node).collect()).collect();
    assert_eq!(nodes, vec![vec![10], vec![1, 2, 3, 13]], "node 10, and the arm's three nodes and the mast");

    // The mast (node 13) and everything standing on it: the turret is part 1.
    let z = |play: &parkan_world::play::Play, part: usize, node: usize| {
        play.battle.combat.targets[tower].parts[part].nodes[node].translation[2] as f32
    };
    let (mast, turret) = (z(&play, 0, 13), z(&play, 1, 0));
    while play.hero.time_ms < 6000.0 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let up = z(&play, 0, 13) - mast;
    assert!((up - 10.37).abs() < 0.1, "frame 0 to frame 4 lifts the mast 10.4 m: {up}");
    let carried = z(&play, 1, 0) - turret;
    assert!((carried - up).abs() < 0.1, "and the turret rides it: {carried}");

    // It holds there: the word bounces as read, and `buildings::started` stops it at the top.
    let held = z(&play, 0, 13);
    while play.hero.time_ms < 20_000.0 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!((z(&play, 0, 13) - held).abs() < 0.2, "the mast stays up: {}", z(&play, 0, 13));
    eprintln!("the mast rises {up:.2} m and holds");
}

/// C02 Mission 04's Light Towers, taken at their pods by the hero on foot. The pod's opening
/// for the player's own unit switches the view by the building's Type, and a medium or large
/// tower goes to state 6, its manual control, unless its first class-1 item, the turret, has
/// no life left (`iron3d.dll:0x10062bc0`, `0x10033e40`; docs/27, "Capture" and "What the modes
/// show"). The mode's handler hands the tower's guns to the player as a take hands a bot's
/// (docs/40, "What a bunker's guns do in command mode"). The recording's player is at the
/// valley tower's gun a second after *"Building is captured"*, shoots tanks with it and leaves
/// with Esc to the pod room ("Let's Play - Parkan: Iron Strategy, Part 4", 26:04–26:14.5), and
/// at the plateau tower's at 37:23.
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_light_tower_taken_at_its_pod_hands_the_player_its_guns() {
    use parkan_formats::mission;
    use parkan_world::play::{Mode, Play};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_04);
    let tower = object_target(&play, &m, "mtow02.dat");
    let player = play.player_clan;
    let tick = |play: &mut Play, mouse: [f32; 2]| {
        play.update_input();
        play.tick(1000.0 / 60.0, mouse);
    };
    fn aim(play: &Play, t: usize) -> [f32; 3] {
        play.emplacements.iter().find(|(e, _)| *e == t).expect("the tower carries guns").1.rig.aim
    }
    // The mast up first, as the recording's tower stood.
    for _ in 0..(6 * 60) {
        tick(&mut play, [0.0; 2]);
    }
    assert!(play.stand_on_pod(tower));
    tick(&mut play, [0.0; 2]);
    let hero_eye = play.eye().position;
    for _ in 0..(10 * 60) {
        tick(&mut play, [0.0; 2]);
        if play.units[tower].clan == Some(player) {
            break;
        }
    }
    assert_eq!(play.units[tower].clan, Some(player), "the pod takes the tower");
    assert_eq!(play.mode(), Mode::Manual(tower), "and opens its manual control");
    assert_eq!(play.driven_target(), Some(tower), "the view's own unit is the tower");
    let eye = play.eye().position;
    let top = play.battle.combat.targets[tower].position.z;
    assert!(eye.z > top + 10.0, "the eye is the turret's, up the mast: {eye} over the tower at {top}");
    assert!(eye.distance(hero_eye) > 10.0, "not the hero's: {eye} and {hero_eye}");

    // Its turret follows the mouse, as a boarded bot's does (`m1.tbl`: mouse X is the turret's
    // yaw, the table a tower's chassis names none of and the reader falls back to, docs/14).
    let before = aim(&play, tower);
    for _ in 0..60 {
        tick(&mut play, [20.0, 0.0]);
    }
    let turned = (aim(&play, tower)[0] - before[0]).abs();
    assert!(turned > 0.2, "the mouse turns its turret: {turned}");

    // Z zooms its camera, the building record's own zoom in view state 6, stepped 0.1 a frame
    // to 0.2 by the building records' update, and slows the mouse to half (docs/30, "The zoom").
    let press = |play: &mut Play, command: &str| {
        let eye = play.own_eye();
        let view = parkan_world::play::View {
            eye: eye.position,
            look: eye.forward,
            view_proj: glam::Mat4::IDENTITY,
            shift: false,
        };
        play.command(command, &view);
    };
    let wide = play.eye().fov_x;
    press(&mut play, parkan_formats::controls::CMD_JAMES_ZOOM_MODE);
    for _ in 0..30 {
        tick(&mut play, [0.0; 2]);
    }
    let narrow = play.eye().fov_x;
    assert!((narrow - 0.2).abs() < 0.05, "Z zooms the tower's camera from {wide} to 0.2: {narrow}");
    assert!((play.driving.as_ref().unwrap().pilot.sensitivity - 0.5).abs() < 1e-6, "the mouse at half");
    press(&mut play, parkan_formats::controls::CMD_JAMES_ZOOM_MODE);
    for _ in 0..30 {
        tick(&mut play, [0.0; 2]);
    }
    assert!((play.eye().fov_x - wide).abs() < 0.05, "and back out: {}", play.eye().fov_x);

    // The button fires its guns, and each stroke of the cannon's barrel plays its report:
    // `L152mmMC`'s load group hangs `gun_can152_fx` on the barrel's point, in time mode 4 on the
    // node the barrel channel plays (docs/29, "What a shot plays").
    let rounds = |play: &Play| play.battle.combat.rounds.len();
    let (mut most, before) = (0, rounds(&play));
    play.cues.clear();
    play.key("SCAN_LMOUSE", true);
    for _ in 0..(3 * 60) {
        tick(&mut play, [0.0; 2]);
        most = most.max(rounds(&play));
    }
    play.key("SCAN_LMOUSE", false);
    assert!(most > before, "its guns fire at the button: {before} rounds, then at most {most}");
    let sounds: Vec<String> = play.cues.iter().map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(sounds.iter().any(|s| s.starts_with("gun_can")), "the cannon sounds as it fires: {sounds:?}");

    // Esc (`CMD_ROLLBACK_STATE`, 6 → 0) gives the guns back to its AI and the view to the hero,
    // still on the pod.
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::OnFoot);
    assert_eq!(play.driven_target(), None);
    let back = play.eye().position;
    assert!(back.distance(hero_eye) < 1.0, "the hero's view again, on the pod: {back} and {hero_eye}");
    let still = aim(&play, tower);
    for _ in 0..60 {
        tick(&mut play, [20.0, 0.0]);
    }
    assert!((aim(&play, tower)[0] - still[0]).abs() < 0.2, "the mouse no longer turns it");

    // The valley tower with its turret shot off takes the hero's capture and opens nothing.
    // Its own repair would hand the turret its first points within a second, and the component
    // test asks only that it has some (docs/26, "Repair"), so here it has none to repair with.
    let valley = object_target(&play, &m, "mtow01.dat");
    play.economy.site_mut(valley).unwrap().repair = None;
    let turret = play.emplacements.iter().find(|(e, _)| *e == valley).unwrap().1.turret_part;
    play.battle.combat.targets[valley].parts[turret].life.as_mut().unwrap().hit(1, f32::MAX / 4.0);
    assert!(play.stand_on_pod(valley));
    for _ in 0..(10 * 60) {
        tick(&mut play, [0.0; 2]);
        if play.units[valley].clan == Some(player) {
            break;
        }
    }
    assert_eq!(play.units[valley].clan, Some(player), "the valley tower is taken");
    assert_eq!(play.mode(), Mode::OnFoot, "and, its turret gone, opens no manual control");
}

/// A building repairs itself (docs/26, "Repair"): its own repair system, switched on by its
/// takt's repair decision under 90% of its life, restores its rate a second to its nodes in
/// index order, each whole before the next takes a point, and a building's takes destroyed
/// nodes with the rest. So a tower whose turret is shot off rebuilds the turret first, then
/// one gun, then the other, as the player remembers C02 M04's plateau Light Tower doing: its
/// `mtow02` repairs 110 a second, its turret carries 3,506 and each `e_gun_fc_01` 6,000.
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_light_tower_rebuilds_its_turret_and_then_its_guns_one_by_one() {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_04);
    let tower = object_target(&play, &m, "mtow02.dat");
    let tick = |play: &mut Play| {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    };
    for _ in 0..(6 * 60) {
        tick(&mut play);
    }
    let (turret, socket) = play.machine(tower).and_then(|r| r.turret_life()).expect("a turret");
    let part = |play: &Play, p: usize| play.battle.combat.targets[tower].parts[p].life.clone().unwrap();
    let whole = |play: &Play, p: usize| part(play, p).nodes.iter().all(|n| n.life >= n.max);
    let gone = |play: &Play, p: usize| part(play, p).nodes.iter().all(|n| n.destroyed && n.hidden());
    let broken = |play: &Play| play.machine(tower).unwrap().guns.iter().filter(|g| g.broken).count();
    assert!(!play.economy.site(tower).unwrap().repairing, "whole, its repair system is off");

    // The turret shot off: its socket, what hangs on it and the guns it carries go with it.
    play.battle.combat.targets[tower].parts[turret].life.as_mut().unwrap().lose(socket, 1.0e9);
    for _ in 0..2 {
        tick(&mut play);
    }
    let guns = [2, 3];
    assert!(part(&play, turret).nodes[socket].hidden() && guns.iter().all(|&g| gone(&play, g)));
    assert!(broken(&play) >= 2, "its cannons silent");

    // Each part whole in turn, the next untouched until then.
    let mut done: Vec<(usize, f64)> = Vec::new();
    let order = [turret, guns[0], guns[1]];
    for frame in 0..(200 * 60) {
        tick(&mut play);
        let t = f64::from(frame) / 60.0;
        let next = order[done.len()];
        if whole(&play, next) {
            done.push((next, t));
            if done.len() == order.len() {
                break;
            }
        }
        for &later in &order[done.len() + 1..] {
            assert!(gone(&play, later), "part {later} waits for part {next} at {t:.1} s");
        }
    }
    eprintln!("whole at {done:?}");
    assert_eq!(done.iter().map(|d| d.0).collect::<Vec<_>>(), order, "turret, then one gun, then the other");
    assert!(play.economy.site(tower).unwrap().repairing, "its repair system on");
    let (turret_s, gun_s) = (done[0].1, done[1].1 - done[0].1);
    assert!((25.0..45.0).contains(&turret_s), "3,506 at 110 a second: {turret_s:.1} s");
    assert!((45.0..70.0).contains(&gun_s), "6,000 at 110 a second: {gun_s:.1} s");
    assert!(!part(&play, turret).nodes[socket].hidden() && !gone(&play, guns[1]), "shown again");
    tick(&mut play);
    assert_eq!(broken(&play), 0, "and its guns fire again");
}

/// A wheeled chassis rights its hull along the ground (docs/24, "The hull leans and rights
/// itself"): its states carry bits `0x30`, which stand the hull toward the averaged ground
/// normal by triple 5's share each step, and its contact points are placed through that pitch
/// and roll. Level, as the engine held every hull before, a small wheeled warrior sent at C02
/// M04's plateau Light Tower dipped its wheels under a floor overhanging the way down, was
/// lifted onto it and circled the entrance for good (reported from play, 2026-09-30); nose
/// down, it drives down the ramp to the pod 21 m below and takes the tower.
#[test]
#[ignore = "needs the game install"]
fn c02_m04s_small_wheeled_warrior_noses_down_the_plateau_towers_ramp_and_takes_it() {
    use parkan_formats::mission;
    use parkan_sim::orders::{CAPTURE, Order, Target};
    use parkan_world::factory::Project;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_04);
    let tower = object_target(&play, &m, "mtow02.dat");
    let id = play.units[tower].logical_id;
    let centre = play.battle.combat.targets[tower].position;
    let pod = play.capture_places().into_iter().find(|p| p.id == id).and_then(|p| p.pod).expect("its pod");
    let tick = |play: &mut Play| {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    };

    // The enemy's own small wheeled warrior, the player's here, 60 m north of the tower, whose
    // guns are taken off so that it lives to reach the pod.
    let path = "UNITS\\UNITS\\BATTLE\\24swele1.dat".to_owned();
    let data = gamedir::resolve(&game, &path).and_then(|p| std::fs::read(p).ok()).expect("24swele1.dat");
    let project = Project {
        path,
        name: String::new(),
        type_word: u32::from_le_bytes(data[4..8].try_into().unwrap()),
        chassis_size: 2,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let spot = centre + glam::Vec3::new(0.0, 60.0, 0.0);
    let z = play.ground.below(spot.x, spot.y, 1.0e4).expect("ground north of the tower").point.z;
    let t = play.spawn(&project, play.player_clan, spot.with_z(z + 1.0), 0.0).expect("the warrior");
    play.emplacements.retain(|(e, _)| *e != tower);
    for _ in 0..60 {
        tick(&mut play);
    }
    let order = Order { code: CAPTURE, parameter: 0, target: Target::LogicId(id) };
    play.robots.iter_mut().find(|(rt, _)| *rt == t).unwrap().1.behaviour.order(&order);

    let (mut nosed, mut taken) = (0.0_f32, None);
    for s in 0..(120 * 60) {
        tick(&mut play);
        let body = &play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body;
        if body.position.z < pod.z + 15.0 {
            nosed = nosed.max(body.tilt[0]);
        }
        if play.units[tower].clan == Some(play.player_clan) {
            taken = Some(f64::from(s) / 60.0);
            break;
        }
    }
    let at = play.robots.iter().find(|(rt, _)| *rt == t).unwrap().1.walker.body.position;
    let taken = taken
        .unwrap_or_else(|| panic!("it never took the tower: at {at}, {:.1} from the pod", at.distance(pod)));
    assert!(nosed > 0.1, "nose down on the way down: {nosed:.3} rad");
    eprintln!("the tower taken {taken:.1} s after the order");
}

/// C02 Mission 03, *The Lost Key*: the enemy clan's planner, `c2m3e`'s, first takt. Its
/// `Problems0` counts its free minds and its factories and raises `PBM_ROBOT_NEEDED` for a
/// `ROBOT_BATTLEUNIT` by `SELECT_BEST_COMBAT`; the `_Start` pass finds the clan's
/// `BUILDING_PLANT` and hands it `ORDER_BUILDING_CONSTRUCT` to the end of its queue, with the
/// `SELECT_*` as the order's `TARGET_BY_NAME` (docs/15, "The planner", and docs/36,
/// "Production"). Its three `PBM_PLACE_PROTECT` raises put its warbots on patrol about the
/// places `Problems0` names, and `PBM_N_OPTIMAL_TRANSPORT` sends its transport out.
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_enemy_planner_orders_its_factory_to_build_a_warbot() {
    use parkan_sim::orders;
    use parkan_sim::planner::{self, ROBOT_BATTLEUNIT, SELECT_BEST_COMBAT};
    use parkan_world::progress::ScriptOrder;

    let (mut p, _) = campaign_progression(gamedir::C02_MISSION_03);
    let enemy = 1_i64;
    let plant = -2147483608_i32;
    assert_eq!(p.progress.owner(plant), enemy as u32, "the enemy's Large Factory");
    let init: Vec<(i32, i32)> = std::mem::take(&mut p.orders).iter().map(|o| (o.id, o.order.code)).collect();
    assert_eq!(init, [(28, orders::SHUTDOWN)], "`Init` shuts one unit down and plans nothing");

    // One clan takt, which is the first thing after `Init` that plans anything.
    p.tick(100.0, |_| None);
    let given: Vec<ScriptOrder> = std::mem::take(&mut p.orders);
    let build = given.iter().find(|o| o.order.code == orders::CONSTRUCT).expect("a build order");
    assert_eq!(build.id, plant, "to its own Large Factory");
    assert_eq!(build.order.parameter as u32, ROBOT_BATTLEUNIT);
    assert_eq!(build.order.target, orders::Target::Select(SELECT_BEST_COMBAT));
    assert_eq!(build.insert, orders::INSERT_TO_END, "to the end of the factory's queue");

    // What the takt left standing: the build, the three places and the transport.
    let planner = p.planner_of(enemy).expect("the enemy runs a script");
    let mut codes: Vec<u32> = planner.standing().map(|(_, q)| q.code).collect();
    codes.sort_unstable();
    assert_eq!(codes, [4, 6, 11, 11, 11], "one transport, one robot wanted, its three places");
    let robot = planner.standing().find(|(_, q)| q.code == 6).expect("PBM_ROBOT_NEEDED").1;
    assert_eq!(robot.name, "PBM_ROBOT_NEEDED");
    assert_eq!((robot.life, robot.drain), (25, 24), "raised 25/24, so it stands two takts");
    assert_eq!(robot.p, [ROBOT_BATTLEUNIT, 0, SELECT_BEST_COMBAT]);
    assert_eq!(robot.state, planner::ST_SOLVING, "the factory took the order");
    // The factory is the problem's own critical unit, and its idling is what ends the problem.
    assert_eq!(robot.units.iter().map(|u| (u.id, u.critical)).collect::<Vec<_>>(), [(plant, true)]);
    assert_eq!(
        robot.actions.iter().map(|a| (a.action, a.target)).collect::<Vec<_>>(),
        [(planner::ACTION_NOTHING_DOING, plant)]
    );

    // Its patrols went to its own warbots, at the places `Problems0` names.
    let patrols: Vec<[f32; 3]> = given
        .iter()
        .filter(|o| o.order.code == orders::PATROL)
        .filter_map(|o| match o.order.target {
            orders::Target::Place(at) => Some(at),
            _ => None,
        })
        .collect();
    assert!(!patrols.is_empty(), "the place-protect problems put warbots on patrol");
    let places = [[1301.0, 1077.0, 0.0], [265.0, 1079.0, 0.0], [265.0, 1540.0, 0.0]];
    assert!(patrols.iter().all(|at| places.contains(at)), "at the places `Problems0` names: {patrols:?}");
    assert!(given.iter().any(|o| o.order.code == orders::TRANSPORT), "and its transport goes out");
}

/// And the order reaches the factory: its own design store ranks the designs of the type
/// asked for and the factory builds one, until the clan's minds run out and it stands idle
/// (docs/23, "The bot limit is the clan's mind count").
///
/// `SELECT_BEST_COMBAT` scores a design by the **strength formula**, guns over armour
/// (`ai.dll:0x1001099d`), which puts the small, heavily armed `23_swlk1` class above the
/// 93,008-hit-point `AI_LS_10` — so the enemy turns out *SSW-X Warriors* of chassis size 2,
/// not the biggest hull in the store. Each costs its design's ore, which its one Small Mine
/// has to dig: only the first is free.
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_enemy_factory_turns_out_small_warbots_until_the_clans_minds_run_out() {
    use parkan_world::play::CLASS_ROBOT;

    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let tick = 1000.0 / 30.0;
    let enemy = 1_i64;
    let placed = play.units.len();
    let plant = play.units.iter().position(|u| u.logical_id == -2147483608).expect("its Large Factory");
    assert!(play.factories.iter().any(|f| f.target == plant), "which the play knows as a factory");

    let mut made: Vec<(f32, u32, u8)> = Vec::new();
    for step in 0..(100 * 30) {
        play.tick(tick, [0.0; 2]);
        while placed + made.len() < play.units.len() {
            let t = placed + made.len();
            let u = &play.units[t];
            assert_eq!(u.clan, Some(enemy), "made for the enemy clan");
            let size = play.robots.iter().find(|(rt, _)| *rt == t).map_or(0, |(_, r)| r.size_class);
            made.push((step as f32 / 30.0, u.type_word, size));
        }
    }
    assert_eq!(made.len(), 3, "three warbots, one a mind: {made:?}");
    assert!(made.iter().all(|&(_, t, _)| t & CLASS_ROBOT != 0), "every one a robot: {made:?}");
    assert!(made.iter().all(|&(_, _, size)| size == 2), "and every one a small chassis: {made:?}");
    // The free bot first: a large factory building a small chassis takes 20 s (docs/23,
    // "Construction"). The two behind it are paid, and wait on the mine's ore.
    assert!((15.0..30.0).contains(&made[0].0), "the free one at {} s", made[0].0);
    assert!(made[1].0 > made[0].0 + 15.0, "and the paid ones dig for theirs: {made:?}");
    assert_eq!(play.free_minds(enemy), 0, "the clan's minds are all held now");
    let factory = play.factories.iter().find(|f| f.target == plant).unwrap();
    assert!(factory.idle(), "so the factory stands idle rather than queueing another");
    // The design it built is one of the AI's own store, and it was paid for.
    let store = play.stores.get(&enemy).expect("the enemy's design store is loaded");
    assert!(store.designs.len() > 50, "the whole of UNITS\\UNITS\\AI: {}", store.designs.len());
    let best = store.pick(0x0100_8000, parkan_sim::planner::SELECT_BEST_COMBAT, 0).expect("a design");
    assert!(best.chassis_size <= 2, "the best combat design is a small one: {best:?}");
    assert!(best.ore > 100.0, "and it is not free: {} ore", best.ore);
}

/// And the capture: the player takes the enemy's Generator, the enemy's SuperAI runs
/// `Fort_Captured` for the building it has just lost — which raises `PBM_BUILDING_INF_CAPTURE`
/// at 0.7 plus what function 66 says a generator is worth — and its nearest capturer is sent
/// to take it back (docs/15, "What the functions do", and docs/27, "Capture").
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_enemy_sends_a_warbot_to_take_its_generator_back() {
    use parkan_sim::orders;
    use parkan_sim::planner::{ACTION_CAPTURE_BUILDING, ACTION_DESTROY, ST_SOLVING};

    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let tick = 1000.0 / 30.0;
    let enemy = 1_i64;
    let generator = play.units.iter().position(|u| u.logical_id == -2147483647).expect("the generator");
    assert_eq!(play.units[generator].type_word, 0x8000_0002, "a generator");
    assert_eq!(play.units[generator].clan, Some(enemy), "the enemy's");
    assert!(play.stand_on_pod(generator), "the hero stands on its pod");

    let (mut taken, mut ordered, mut back) = (None, None, None);
    for step in 0..(100 * 30) {
        play.update_input();
        play.tick(tick, [0.0; 2]);
        let at = step as f32 / 30.0;
        if taken.is_none() && play.units[generator].clan == Some(play.player_clan) {
            taken = Some(at);
            // The hero steps off, as a player does: a pod stays with the one standing on it
            // while it stays in the zone (docs/27, "Capture"), so an enemy on it beside the
            // hero would take nothing back.
            let c = play.battle.combat.targets[generator].position;
            assert!(play.stand_at(c.x + 60.0, c.y, 0.0), "ground beside the generator");
        }
        if taken.is_some() && ordered.is_none() {
            ordered = play
                .robots
                .iter()
                .find(|(t, r)| {
                    play.units[*t].clan == Some(enemy) && r.order.map(|o| o.code) == Some(orders::CAPTURE)
                })
                .map(|(t, _)| (at, play.units[*t].logical_id));
        }
        if taken.is_some() && back.is_none() && play.units[generator].clan == Some(enemy) {
            back = Some(at);
            break;
        }
    }
    let taken = taken.expect("the hero's pod fires and the generator changes hands");
    let (ordered_at, capturer) = ordered.expect("the enemy answers with a capture order");

    // The problem the event raised: weight 0.7 + 0.04, the generator as its first parameter,
    // and the two actions that end it — the building destroyed, or taken.
    let planner = play.progression.as_ref().unwrap().planner_of(enemy).unwrap();
    let problem = planner.standing().map(|(_, q)| q).find(|q| q.code == 14);
    if let Some(q) = problem {
        assert_eq!(q.name, "PBM_BUILDING_INF_CAPTURE");
        assert!((q.weight - 0.74).abs() < 1e-5, "0.7 + a generator's 0.04: {}", q.weight);
        assert_eq!(q.p[0] as i32, play.units[generator].logical_id);
        assert_eq!(q.state, ST_SOLVING);
        let mut actions: Vec<u32> = q.actions.iter().map(|a| a.action).collect();
        actions.sort_unstable();
        assert_eq!(actions, [ACTION_DESTROY, ACTION_CAPTURE_BUILDING]);
    }
    let back = back.expect("and its warbot walks in and takes the generator back");
    assert!(
        ordered_at > taken && back > ordered_at,
        "{taken} s taken, {ordered_at} s ordered, {back} s back"
    );
    eprintln!("taken at {taken:.1} s, unit {capturer} ordered at {ordered_at:.1} s, back at {back:.1} s");
}

/// The game level reaches the scripts. The SuperAI's constructor writes `fDifficulty` from a
/// six-float table indexed by `Iron_3D.ini`'s `[CS] GAME_LEVEL` — 0 easy, 1 medium, 2 hard
/// giving 0.0, 0.5 and 1.0 (`ai.dll:0x10005d00`) — and every difficulty branch the campaign's
/// scripts carry hangs off it. On C02 M03 `c2m3e`'s `Init` sets the design store's spread to
/// `6 − 4 × fDifficulty`, its `Problems0` gates the whole `PBM_BASE_DEFENCE` block on
/// `fDifficulty > 0`, and `dPlaceProtectHits` is `100 − 40 × fDifficulty`.
#[test]
#[ignore = "needs the game install"]
fn c02_m03s_enemy_reads_the_game_level_through_f_difficulty() {
    use parkan_formats::gamedir;
    use parkan_sim::planner;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let level = parkan_world::settings::game_level(&game);
    let difficulty = planner::difficulty(level);
    assert_eq!(planner::DIFFICULTY[..3], [0.0, 0.5, 1.0], "easy, medium, hard");

    let (p, _) = campaign_progression(gamedir::C02_MISSION_03);
    let enemy = p.others.iter().find(|o| o.clan == 1).expect("the enemy runs c2m3e");
    assert_eq!(enemy.script.float("fDifficulty"), Some(difficulty), "written before `Init` runs");
    // What `Init` made of it: `fn69(6 − 4 × fDifficulty)`, the draw's spread.
    let spread = (6.0 - 4.0 * difficulty) as u32;
    assert_eq!(enemy.planner.spread, spread, "the design spread at game level {level}");
    // And `dPlaceProtectHits`, which `Init` sets to 100 at difficulty 0 and 100 − 40·d above.
    let hits = enemy.script.dword("dPlaceProtectHits").expect("the variable");
    let expected = if difficulty <= 0.0 { 100 } else { (100.0 - 40.0 * difficulty) as u32 };
    assert_eq!(hits, expected);
}

/// Ballen's Crossing: once the neutral HQ, logical id 11, is no longer its own clan's, a timer
/// runs, and when it is up the player's script says `C02M02_I01` and runs `script1` to
/// `script3` through function 57: three heavy warbots `create`d for `Enm2`, clan 4, which the
/// mission places no robot for. Each stands 2 over the ground at its x, y, on its clan's list
/// before the handler goes on, so the bonus objective, which the same run completes once
/// neither `Enm` nor `Enm2` has a robot, waits for them to fall (docs/15, "What the console's
/// `create`, `bcreate` and `death` do").
#[test]
#[ignore = "needs the game install"]
fn c02_m02s_reinforcements_are_created_for_enm2_on_its_list_before_the_bonus_objective_asks() {
    use parkan_world::play::{CLASS_ROBOT, PROBE_TOP, Play};

    let mut play = campaign_play(gamedir::C02_MISSION_02);
    let robots = |play: &Play, clan: i64| {
        play.progression.as_ref().unwrap().progress.robots(clan, i64::from(CLASS_ROBOT))
    };
    let bonus = |play: &Play| play.progression.as_ref().unwrap().progress.objectives[1].state;
    // `Enm`'s robots all destroyed first: the bonus then waits on `Enm2` alone.
    let kill = |play: &mut Play, t: usize| {
        play.battle.combat.targets[t].parts[0].life.as_mut().unwrap().hit(0, f32::MAX / 4.0);
    };
    let enemy: Vec<usize> = (0..play.units.len())
        .filter(|&t| play.units[t].clan == Some(1) && play.units[t].logical_id > 0)
        .collect();
    assert!(!enemy.is_empty());
    for &t in &enemy {
        kill(&mut play, t);
    }
    play_for(&mut play, 1.0, |_| {});
    assert_eq!((robots(&play, 1), robots(&play, 4), bonus(&play)), (0, 0, 0), "the timer has not run");

    // The HQ taken: `fn52(11) != 2` starts the timer, at most 40 + 40 s on the easiest level.
    play.progression.as_mut().unwrap().progress.captured(11, 0);
    let units = play.units.len();
    let mut made = None;
    for tick in 0..(100 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if play.units.len() > units {
            made = Some(tick);
            break;
        }
    }
    assert!(made.is_some(), "the reinforcements arrive");
    let new: Vec<usize> = (units..play.units.len()).collect();
    assert_eq!(new.len(), 3);
    let paths: Vec<String> = new.iter().map(|&t| play.commander.paths[t].clone()).collect();
    assert_eq!(paths, ["UNITS\\AUTO\\22lwhl1.dat", "UNITS\\AUTO\\22lwhl2.dat", "UNITS\\AUTO\\22ltrk1.dat"]);
    for (&t, (x, y)) in new.iter().zip([(918.0, 683.0), (919.0, 684.0), (917.0, 683.0)]) {
        assert_eq!(play.units[t].clan, Some(4));
        let at = play.battle.combat.targets[t].position;
        let ground = play.ground.below(x, y, PROBE_TOP).unwrap().point.z;
        assert_eq!((at.x, at.y), (x, y), "{paths:?}");
        assert!((at.z - (ground + 2.0)).abs() < 0.01, "{at} over {ground}");
    }
    assert_eq!(robots(&play, 4), 3, "on Enm2's list");
    assert_eq!(bonus(&play), 0, "the same run counted them");
    let ids: Vec<i32> = new.iter().map(|&t| play.units[t].logical_id).collect();
    assert!(ids.windows(2).all(|w| w[1] == w[0] + 1) && ids[0] > 17, "{ids:?}");

    // They fall: the bonus objective completes.
    for &t in &new {
        kill(&mut play, t);
    }
    play_for(&mut play, 3.0, |_| {});
    assert_eq!((robots(&play, 4), bonus(&play)), (0, 1));
}

/// The Dead City: the hero in the ruins, route 0, completes objective 2 and runs `script1` to
/// `script5`. `death(1246, 1051, 100, 0)` fells the stone standing on the Teleport's site and
/// the one tree whose sphere reaches it, 140 off with a radius of 88, and no unit or building;
/// `bcreate(1246, 1051, 10, 0, teleport.dat, 0)` puts the player's Teleport there
/// finished, no construction sphere run, set down on the mean of its contour though the line
/// asks for z 10, 41 under the ground; and three `42_mtp` go to `Enm1` (docs/15, "What the
/// console's `create`, `bcreate` and `death` do").
#[test]
#[ignore = "needs the game install"]
fn c04_m02s_ruins_clear_the_teleports_ground_and_put_it_down_standing_for_the_player() {
    use parkan_formats::mission;

    let mut play = campaign_play(gamedir::C04_MISSION_02);
    let scenery: Vec<usize> = (0..play.units.len())
        .filter(|&t| matches!(play.units[t].kind, mission::KIND_VEGETATION | mission::KIND_ROCK))
        .collect();
    // The two by their name and place: the mission has another `s_tree_44` far off.
    let named = |name: &str, x: f32, y: f32| {
        *scenery
            .iter()
            .find(|&&t| {
                let p = play.battle.combat.targets[t].position;
                play.names[t] == name && (p.x - x).abs() < 0.1 && (p.y - y).abs() < 0.1
            })
            .expect(name)
    };
    let (stone, tree) = (named("s_stone_10", 1258.2, 1066.385), named("s_tree_44", 1210.032, 1185.813));
    let (units, buildings) = (play.units.len(), play.construction.placements.len());
    assert!(play.stand_at(380.0, 1360.0, 0.0), "the ruins");
    let mut made = false;
    for _ in 0..(10 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if play.units.len() > units {
            made = true;
            break;
        }
    }
    assert!(made, "the ruins' objective runs the block");
    assert_eq!(play.progression.as_ref().unwrap().progress.objectives[2].state, 1);
    play_for(&mut play, 0.1, |_| {});

    // `death`: the stone on the site and the tree whose sphere reaches in; no other scenery.
    let felled: Vec<usize> =
        scenery.iter().copied().filter(|&t| !play.battle.combat.targets[t].alive).collect();
    assert_eq!(felled, [stone, tree]);
    assert!(
        play.battle.combat.targets[..units].iter().enumerate().all(|(t, x)| x.alive || scenery.contains(&t))
    );

    // `bcreate`: the Teleport, the player's, standing on the ground there and placed.
    let teleport = (units..play.units.len())
        .find(|&t| play.commander.paths[t].eq_ignore_ascii_case("UNITS\\AUTO\\teleport.dat"))
        .expect("the Teleport");
    assert_eq!(play.units[teleport].clan, Some(0));
    assert_eq!(play.construction.placements.len(), buildings + 1);
    assert!(play.placed(teleport) && play.construction.spheres.iter().all(|s| s.target != teleport));
    let at = play.battle.combat.targets[teleport].position;
    assert_eq!((at.x, at.y), (1246.0, 1051.0));
    // Its base on the mean of its cut contour, the origin 3.3 under the ground at 51.1, as
    // Tut_4's placed Main Teleport's stands 3.7 under its own; not left at 10.
    assert!((at.z - 47.82).abs() < 0.01, "{at}");
    let id = play.units[teleport].logical_id;
    assert!(id < 0, "a building's id carries the top bit: {id:#x}");
    assert_eq!(play.progression.as_ref().unwrap().progress.owner(id), 0);

    // `create` three times: `Enm1`'s mobile HQs.
    let hqs: Vec<usize> = (units..play.units.len())
        .filter(|&t| play.commander.paths[t].eq_ignore_ascii_case("UNITS\\AUTO\\42_mtp.dat"))
        .collect();
    assert_eq!(hqs.len(), 3);
    assert!(hqs.iter().all(|&t| play.units[t].clan == Some(1)));
}
