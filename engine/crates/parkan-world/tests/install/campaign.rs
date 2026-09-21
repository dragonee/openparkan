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
    let mut way = play.way_in(plant, from, pod).expect("a way in");
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
    assert_eq!(building.running.len(), 1, "one item runs by itself: the mast");
    assert_eq!(
        building.running[0].channels.iter().map(|c| c.node).collect::<Vec<_>>(),
        vec![1, 2, 3, 13],
        "the arm's three nodes and the mast"
    );

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
