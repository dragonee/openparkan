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
    assert_eq!(play.robots[r].1.behaviour.task(), Task::Stop, "grazing, it lets the hero be");

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

    // Twenty minutes of takts, the units standing where they were placed.
    let (mut raids, mut messages) = (Vec::new(), Vec::new());
    let mut now = 0.0;
    while now < 1_200_000.0 {
        now += 100.0;
        for notice in p.tick(now, |_| None) {
            if let Notice::Message { id, first: true } = notice {
                messages.push(id);
            }
        }
        if !p.orders.is_empty() {
            let orders: Vec<ScriptOrder> = std::mem::take(&mut p.orders);
            raids.push((now / 1000.0, p.others[enemy].takt.clock(), orders));
        }
    }
    assert_eq!(raids.len(), 2, "two raids and no more: {raids:#?}");
    assert_eq!(messages, [0, 1], "each raid says its own message");

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
