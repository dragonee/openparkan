//! What a gun does: the laser, the plasma rifle, the missiles, a battery's charge, armour
//! and the damage a round leaves.

use crate::common::*;
use parkan_formats::gamedir;

#[test]
#[ignore = "needs the game install"]
fn the_heros_laser_kills_a_small_target_in_two_hits() {
    use glam::Vec3;
    use parkan_formats::mission;
    use parkan_sim::combat::Event;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 01 has a hero");
    let hero_guns: Vec<bool> = play.hero.guns.iter().map(|g| g.selected).collect();
    assert_eq!(hero_guns, vec![true, false, true, false], "cannon and laser start selected (docs/29)");

    // Object 1 is an l_targ, r_h_01: node 0 has 500 hit points, and the Trgt clan is
    // the player's ally, so no level ratio.
    let object = 1;
    assert!(m.objects[object].path.to_ascii_lowercase().ends_with("l_targ.dat"));
    let t = play.battle.objects.iter().position(|&o| o == object).unwrap();
    let life = play.battle.combat.targets[t].parts[0].life.as_ref().unwrap();
    assert_eq!(life.nodes[0].max, 500.0);

    // Stand 30 m off it, facing it, on the ground its post stands on: not in Tut_1's
    // lake, whose bed lies far below.
    let centre = play.battle.combat.targets[t].centre;
    let base = m.objects[object].position[2];
    let at = (0..16)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            centre + Vec3::new(a.cos(), a.sin(), 0.0) * 30.0
        })
        .find(|p| play.ground.below(p.x, p.y, 1000.0).is_some_and(|h| h.point.z > base - 9.0))
        .expect("somewhere level to stand");
    let facing = (centre - at).with_z(0.0).normalize();
    let w = &mut play.hero.walker;
    w.body.position = Vec3::new(at.x, at.y, centre.z + 20.0);
    w.body.yaw = (-facing.x).atan2(facing.y);
    w.follow_ground(&play.ground);
    w.from = (w.body.position, w.body.yaw);
    w.from_heading = w.body.yaw;
    play.tick(1000.0 / 60.0, [0.0; 2]);

    // Tilt the sight until it meets the target's node 0, the base whose death kills it.
    let pitch = play.hero.rig.pitch.unwrap();
    let meets = |play: &mut Play, value: f32| {
        play.hero.rig.values[pitch] = value;
        let (o, s) = play.hero.sight().unwrap();
        play.battle
            .combat
            .first_hit(&play.ground, None, o + s * 5.0, o + s * 200.0, 0.0)
            .is_some_and(|(strike, target, _)| target == Some(t) && strike.node == Some(0))
    };
    let hitting: Vec<f32> = (0..=100).map(|k| k as f32 / 100.0).filter(|&v| meets(&mut play, v)).collect();
    assert!(!hitting.is_empty(), "some pitch puts the sight on the target's base");
    let value = hitting[hitting.len() / 2];
    play.hero.rig.values[pitch] = value;
    play.hero.rig.aim[1] = 1.0 - value;

    // A unit answers for its material: the base's skin is class 5, so a strike on it plays
    // the `.exp`'s `mt` slot (docs/11-effects.md, "What an explosion plays").
    let (o, s) = play.hero.sight().unwrap();
    let (_, _, part) =
        play.battle.combat.first_hit(&play.ground, None, o + s * 5.0, o + s * 200.0, 0.0).unwrap();
    let struck = parkan_world::play::struck_wear(
        &play.battle.combat.targets[t].parts[part],
        &play.battle.wears[t][part],
        o + s * 5.0,
        o + s * 200.0,
    )
    .expect("a wear entry");
    assert_eq!(play.materials.get(struck).map(|m| m.surface), Some(5), "{struck}");

    // The laser alone: key 1 deselects the cannon.
    play.hero.key("SCAN_W_1", true);
    play.hero.key("SCAN_LMOUSE", true);
    let mut damage = Vec::new();
    let mut killed_at = None;
    for tick in 0..90 {
        for e in play.tick(1000.0 / 60.0, [0.0; 2]) {
            match e {
                Event::Damaged { target, node, damage: d, .. } if target == t => damage.push((node, d)),
                Event::Killed { target } if target == t => killed_at = Some(tick),
                _ => {}
            }
        }
    }
    assert!(killed_at.is_some(), "killed; damage {damage:?}");
    assert_eq!(damage, vec![(0, 250.0), (0, 250.0)], "each laser hit is 249 + 1 on node 0");
    // Deleted once its controller's +92, 3000 ms, has passed (docs/26).
    play.hero.key("SCAN_LMOUSE", false);
    for _ in 0..200 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert_eq!(play.killed, vec![object]);
}

/// The hero's battery pays for what it runs (docs/23, "Bots spend power through the same code,
/// priced by part"): standing, its shield, deflector, detection shield and radar idle at 2.39 a
/// second of its 4,080; walking costs nothing more, every state of its chassis carrying an engine
/// factor of 0 (docs/24); camouflage adds its 0.3; G switches its repair system on, which heals its
/// nodes at 15 × its node's condition a second for 0.04 a point and 0.1 idle, and off again; and a
/// laser shot's 5.5 is drawn back into its capacitor from the battery, served after the rest.
#[test]
#[ignore = "needs the game install"]
fn the_heros_battery_pays_for_its_shield_repair_camouflage_and_laser_but_not_its_walking() {
    use parkan_formats::mission;
    use parkan_sim::damage::{Life, share_loss};
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 01 has a hero");
    let tick = |play: &mut Play, seconds: f32| {
        for _ in 0..(seconds * 60.0) as usize {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    let power = play.hero.robot.power.clone().expect("the hero carries a battery");
    let (battery, _) = power.batteries[0];
    assert_eq!((power.batteries.len(), battery.capacity, battery.output), (1, 4080.0, 6.8));
    let spent = |play: &Play| (1.0 - play.battery(None).unwrap()) * 4080.0;
    // What it spends a second over `seconds`, timed by its own power ticks.
    let last = |play: &Play| play.hero.robot.power.as_ref().unwrap().last_ms;
    let rate = |play: &mut Play, seconds: f32| {
        let (before, from) = (spent(play), last(play));
        tick(play, seconds);
        (spent(play) - before) / ((last(play) - from) / 1000.0) as f32
    };
    // A key's press and release rows, for a class's state row.
    let press = |play: &mut Play, class: &str| {
        let key = play
            .hero
            .pilot
            .rows
            .iter()
            .find(|r| r.target == class && r.command == "MCMD_STATE")
            .unwrap()
            .key
            .clone();
        play.key(&key, true);
        play.update_input();
        play.key(&key, false);
    };

    tick(&mut play, 0.5);
    let idle = rate(&mut play, 10.0);
    assert!((idle - 2.39).abs() < 0.05, "standing, the idle draws: {idle} a second");
    let from = play.hero.walker.body.position;
    play.key("SCAN_W", true);
    let walking = rate(&mut play, 5.0);
    play.key("SCAN_W", false);
    assert!(play.hero.walker.body.position.distance(from) > 20.0, "the hero walks");
    assert!((walking - idle).abs() < 0.05, "walking costs nothing more: {walking} against {idle}");

    press(&mut play, "CICLS_DETECTSHIELD");
    assert!(play.hero.pilot.switches.camouflage);
    let camouflaged = rate(&mut play, 5.0);
    assert!((camouflaged - idle - 0.3).abs() < 0.05, "camouflage adds 0.3: {camouflaged}");
    press(&mut play, "CICLS_DETECTSHIELD");

    // Half its life gone from every node. The repair system sits on node 0, the first it heals,
    // so it starts at 7.5 points a second and speeds up with its node's condition: node 0's 190
    // grow by 15 ÷ 380 of themselves a second, 190 × (e^(150/380) − 1) ≈ 92 in ten seconds.
    let full: f32 = play.hero.lives.iter().flatten().map(Life::full).sum();
    let total = |play: &Play| play.hero.lives.iter().flatten().map(Life::total).sum::<f32>();
    let mut lives: Vec<&mut Life> = play.hero.lives.iter_mut().flatten().collect();
    share_loss(&mut lives, full * 0.5);
    let hurt = total(&play);
    let unrepaired = rate(&mut play, 3.0);
    assert!((total(&play) - hurt).abs() < 1e-3, "nothing heals it while repair is off");
    assert!((unrepaired - idle).abs() < 0.05, "hurt, it idles as before: {unrepaired} against {idle}");
    press(&mut play, "CICLS_REPAIRSYS");
    assert!(play.hero.pilot.switches.repair, "G switches repair on");
    let before = total(&play);
    let repairing = rate(&mut play, 10.0);
    let healed = total(&play) - before;
    assert!((86.0..98.0).contains(&healed), "{healed} in 10 s");
    assert!(
        (repairing - idle - (0.1 + 0.04 * healed / 10.0)).abs() < 0.05,
        "at 0.04 a point and 0.1: {repairing}"
    );
    press(&mut play, "CICLS_REPAIRSYS");
    assert!(!play.hero.pilot.switches.repair);
    let off = total(&play);
    tick(&mut play, 2.0);
    assert!((total(&play) - off).abs() < 1.0, "off again, it stops");

    // The laser alone, fired for three seconds: its capacitor spends 5.5 a shot and is refilled
    // from what the battery has left after the idle draws.
    play.key("SCAN_W_1", true);
    play.key("SCAN_W_1", false);
    tick(&mut play, 0.1);
    let selected: Vec<usize> =
        (0..play.hero.robot.guns.len()).filter(|&g| play.hero.robot.guns[g].selected).collect();
    assert_eq!(selected.len(), 1, "key 1 leaves the laser alone selected");
    let laser = selected[0];
    let capacitor = play.hero.robot.guns[laser].capacitor;
    play.key("SCAN_LMOUSE", true);
    let firing = rate(&mut play, 3.0);
    play.key("SCAN_LMOUSE", false);
    let drawn = play.hero.robot.guns[laser].charge;
    assert!(drawn < capacitor - 10.0, "its shots spend the capacitor: {drawn} of {capacitor}");
    assert!(
        firing > idle + 2.0 && firing <= 6.8 + 1e-3,
        "the battery refills it at up to its output: {firing}"
    );
    // Its battery's node, the hurt chassis's node 0, stands at 74%: it gives 4.9 a second, 2.5
    // of it past the idle draws.
    tick(&mut play, 15.0);
    let refilled = play.hero.robot.guns[laser].charge;
    assert!(
        (refilled - capacitor).abs() < 1e-3,
        "full again: {refilled} of {capacitor}, {drawn} after firing"
    );
    assert!(play.battery(None).unwrap() < 1.0 - (idle * 50.0) / 4080.0);
}

#[test]
#[ignore = "needs the game install"]
fn the_hero_destroys_mission_01s_five_targets() {
    use glam::Vec3;
    use parkan_formats::mission;
    use parkan_sim::combat::Event;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 01 has a hero");
    let dummies: Vec<usize> = m
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.path.to_ascii_lowercase().ends_with("targ.dat"))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(dummies.len(), 5);
    play.hero.key("SCAN_W_1", true); // the laser alone
    let pitch = play.hero.rig.pitch.unwrap();
    for &object in &dummies {
        let t = play.battle.objects.iter().position(|&o| o == object).unwrap();
        let centre = play.battle.combat.targets[t].centre;
        let base = m.objects[object].position[2];
        let mut aimed = false;
        'stand: for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let at = centre + Vec3::new(a.cos(), a.sin(), 0.0) * 30.0;
            if !play.ground.below(at.x, at.y, 1000.0).is_some_and(|h| h.point.z > base - 9.0) {
                continue;
            }
            let facing = (centre - at).with_z(0.0).normalize();
            let w = &mut play.hero.walker;
            w.body.position = Vec3::new(at.x, at.y, centre.z + 20.0);
            w.body.yaw = (-facing.x).atan2(facing.y);
            w.follow_ground(&play.ground);
            w.from = (w.body.position, w.body.yaw);
            w.from_heading = w.body.yaw;
            play.tick(1000.0 / 60.0, [0.0; 2]);
            for v in (0..=100).map(|s| s as f32 / 100.0) {
                play.hero.rig.values[pitch] = v;
                // The gun mounts follow the pitch (flag 8, and 0x40 copies them): settle them.
                for c in 0..play.hero.rig.channels.len() {
                    let flags = play.hero.rig.channels[c].flags;
                    if flags & 0x40 != 0 && c > 0 {
                        play.hero.rig.values[c] = play.hero.rig.values[c - 1];
                    } else if flags & 0x8 != 0 {
                        play.hero.rig.values[c] = v;
                    }
                }
                let (o, s) = play.hero.sight().unwrap();
                let hit = play.battle.combat.first_hit(&play.ground, None, o + s * 5.0, o + s * 200.0, 0.0);
                let on_base = |h: Option<(parkan_sim::hit::Strike, Option<usize>, usize)>| {
                    h.is_some_and(|(strike, target, _)| target == Some(t) && strike.node == Some(0))
                };
                if !on_base(hit) {
                    continue;
                }
                // The laser leaves its arm's muzzle, below and beside the sight: its line
                // to the aim point must be clear too.
                play.hero.rig.aim[1] = 1.0 - v;
                let laser = play.hero.guns[2].barrels[0].channel;
                let (muzzle, _) = play.hero.muzzle(laser).unwrap();
                let aim = play.battle.combat.aim_point(&play.ground, None, o, s).unwrap();
                let past = aim + (aim - muzzle).normalize() * 2.0;
                if on_base(play.battle.combat.first_hit(&play.ground, None, muzzle, past, 0.0)) {
                    aimed = true;
                    break 'stand;
                }
            }
        }
        assert!(aimed, "a stand with the sight on object {object}'s base");
        play.hero.key("SCAN_LMOUSE", true);
        let mut killed = false;
        for _ in 0..600 {
            let events = play.tick(1000.0 / 60.0, [0.0; 2]);
            if events.iter().any(|e| matches!(e, Event::Killed { target } if *target == t)) {
                killed = true;
                break;
            }
        }
        play.hero.key("SCAN_LMOUSE", false);
        assert!(killed, "object {object} dies within 10 s of fire");
    }
    // The big dummies are deleted five seconds after they die.
    for _ in 0..330 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let mut dead = play.killed.clone();
    dead.sort_unstable();
    assert_eq!(dead, dummies);
}

#[test]
#[ignore = "needs the game install"]
fn the_plasma_rifle_holds_its_fire_without_a_target_and_its_bolt_follows_one() {
    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    // The plasma rifle alone: 1 and 3 deselect the cannon and the laser, 2 selects it.
    for key in ["SCAN_W_1", "SCAN_W_3", "SCAN_W_2"] {
        play.hero.key(key, true);
        play.tick(tick, [0.0; 2]);
        play.hero.key(key, false);
    }
    let selected: Vec<bool> = play.hero.guns.iter().map(|g| g.selected).collect();
    assert_eq!(selected, vec![false, true, false, false]);
    // The fitted Small sensor module, not the turret's own radar slot (docs/25).
    assert_eq!((play.hero.radar.range, play.hero.radar.period_ms), (300.0, 750.0));
    assert!(play.hero.guns[1].gate.guided());
    play.hero.key("SCAN_LMOUSE", true);
    for _ in 0..180 {
        play.tick(tick, [0.0; 2]);
        assert!(play.battle.combat.rounds.is_empty(), "nothing on the radar at the start, so no bolt");
    }
    assert_eq!(play.targets.current, None);

    // 60 m off a dummy: the list's takt picks it, and the rifle fires once its 0.25 s lock
    // runs out, handing the bolt its target.
    let object = m
        .objects
        .iter()
        .position(|o| {
            o.path
                .to_ascii_lowercase()
                .to_ascii_lowercase()
                .ends_with(&std::env::var("TARG").unwrap_or("l_targ.dat".into()).to_ascii_lowercase())
        })
        .unwrap();
    let t = play.battle.objects.iter().position(|&o| o == object).unwrap();
    stand_facing(&mut play, t, 60.0, 0.0);
    let mut fired = None;
    for _ in 0..180 {
        play.tick(tick, [0.0; 2]);
        if let Some(r) = play.battle.combat.rounds.first() {
            fired = Some(*r);
            break;
        }
    }
    assert!(play.targets.current.is_some(), "the takt picked a target");
    let round = fired.expect("a bolt leaves once a target is held");
    assert_eq!(round.target, play.targets.current);
    assert!(play.battle.combat.kinds[round.kind].seeker.is_some());
}

#[test]
#[ignore = "needs the game install"]
fn the_missiles_lock_draws_corners_closing_on_the_target_and_beeps_as_it_locks_and_once_locked() {
    use glam::{Mat4, Vec3};
    use parkan_sim::guns::LOCK_DRAWN;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::weapons::{TARGET_READY, TARGET_ZOOM, draws_lock, lock_edges};
    use parkan_world::hud::{Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    // The plasma rifle's and the missiles' rounds are marked; the cannon's and the laser's not.
    let marked: Vec<bool> = play.hero.guns.iter().map(|g| g.round_flags == LOCK_DRAWN).collect();
    assert_eq!(marked, vec![false, true, false, true]);
    // The missiles alone.
    for key in ["SCAN_W_1", "SCAN_W_3", "SCAN_W_4"] {
        play.hero.key(key, true);
        play.tick(tick, [0.0; 2]);
        play.hero.key(key, false);
    }
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let object = m.objects.iter().position(|o| o.path.to_ascii_lowercase().ends_with("l_targ.dat")).unwrap();
    let t = play.battle.objects.iter().position(|&o| o == object).unwrap();
    stand_facing(&mut play, t, 60.0, 0.0);
    // The fire button stays up: the lock counts all the same.
    let (mut zooms, mut readies, mut shares) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..(6 * 60) {
        play.tick(tick, [0.0; 2]);
        let eye = play.hero.eye();
        let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
            * Mat4::look_to_rh(eye.position, eye.forward, Vec3::Z);
        let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
        let now = play.hero.time_ms;
        let gun = &play.hero.guns[3];
        let report = gun.lamp_report(now);
        shares.push((now, report, gun.lock_share, draws_lock(gun, report)));
        for sound in drawn.sounds {
            match sound {
                s if s == TARGET_ZOOM => zooms.push(now),
                s if s == TARGET_READY => readies.push(now),
                _ => {}
            }
        }
    }
    assert!(play.targets.current.is_some(), "the dummy is the target");
    let drawn: Vec<_> = shares.iter().filter(|s| s.3).collect();
    let first = drawn.first().expect("the lock is drawn");
    assert_eq!(first.1, 1, "locking first");
    let locked = drawn.iter().find(|s| s.1 == 0).expect("then locked");
    assert!((locked.0 - first.0 - 4000.0).abs() < 100.0, "a 4 s lock: {first:?} to {locked:?}");
    assert!(locked.2 > 0.99 && locked.2 < 1.0, "the share kept short of 1: {}", locked.2);
    // Beeps: TARGET_ZOOM a little over every 0.35 s while locking, TARGET_READY every 0.2 s on.
    let gaps = |v: &[f64]| v.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>();
    assert!(zooms.len() >= 10 && gaps(&zooms).iter().all(|g| (350.0..=370.0).contains(g)), "{zooms:?}");
    assert!(zooms.iter().all(|&z| z < locked.0) && readies.iter().all(|&r| r >= locked.0));
    assert!(readies.len() >= 5 && gaps(&readies).iter().all(|g| (200.0..=220.0).contains(g)), "{readies:?}");
    // The corners close from the HUD's edges onto 20 about the target.
    assert_eq!(lock_edges([320.0, 240.0], 0.0, 0.0), [30.0, 30.0, 630.0, 450.0]);
}

#[test]
#[ignore = "needs the game install"]
fn a_laser_beam_stands_from_the_muzzle_three_quarters_of_a_second_after_its_round_stops() {
    use parkan_sim::effects::Sprite;
    use parkan_world::fx::Owner;

    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    // The laser alone: 1 deselects the cannon. Gun 2 is the laser.
    play.hero.key("SCAN_W_1", true);
    play.tick(tick, [0.0; 2]);
    play.hero.key("SCAN_W_1", false);
    let object = m.objects.iter().position(|o| o.path.to_ascii_lowercase().ends_with("l_targ.dat")).unwrap();
    let t = play.battle.objects.iter().position(|&o| o == object).unwrap();
    stand_facing(&mut play, t, 60.0, 0.0);
    play.hero.key("SCAN_LMOUSE", true);
    let mut stopped = None;
    for _ in 0..240 {
        play.tick(tick, [0.0; 2]);
        if let Some((r, _)) = play.spent.first() {
            stopped = Some((*r, play.hero.time_ms));
            break;
        }
    }
    play.hero.key("SCAN_LMOUSE", false);
    let (round, at) = stopped.expect("a laser round stops within 4 s");
    assert_eq!(play.battle.kinds[round.kind].record.to_ascii_lowercase(), "bl_h_01");
    assert_eq!(play.battle.kinds[round.kind].death_ms, 3000.0);

    // The round's bolt effect, `hero_laser_bullet`, and what it draws at `now`.
    let beam = |play: &parkan_world::play::Play, now: f64| -> Option<Vec<Sprite>> {
        let (_, i) = play.fx.instances.iter().find(|(o, _)| *o == Owner::Round(round.id, 0))?;
        let mut out = Vec::new();
        i.sprites(now, true, &mut out);
        Some(out)
    };
    let muzzle = |play: &parkan_world::play::Play| play.hero.gun_muzzle(2, 0).expect("the laser's muzzle").0;
    let ends = |sprites: &[Sprite]| {
        let first = &sprites[0];
        let last = sprites.last().unwrap();
        (first.centre - first.along / 2.0, last.centre + last.along / 2.0)
    };

    // As it stops the beam is whole: from the muzzle to where the round stopped.
    let now = beam(&play, at).expect("the bolt outlives its round's flight");
    assert!(!now.is_empty() && now.iter().all(|s| s.lengthwise && (s.alpha - 1.0).abs() < 1e-3));
    let (from, to) = ends(&now);
    assert!((from - muzzle(&play)).length() < 0.5, "starts at the muzzle: {from} against {}", muzzle(&play));
    assert!((to - round.position).length() < 1e-3, "ends where the round stopped");
    assert!((round.position - play.battle.combat.targets[t].centre).length() < 10.0, "on the dummy");

    // Its start rides on the hero: stood 10 m nearer, the beam starts at the muzzle there.
    stand_facing(&mut play, t, 50.0, 0.0);
    while play.hero.time_ms < at + 375.0 {
        play.tick(tick, [0.0; 2]);
    }
    let half = beam(&play, play.hero.time_ms).expect("still there");
    let fade = 1.0 - ((play.hero.time_ms - at) / 750.0) as f32;
    assert!(half.iter().all(|s| (s.alpha - fade).abs() < 1e-3), "{} against {fade}", half[0].alpha);
    let (from, to) = ends(&half);
    assert!((from - muzzle(&play)).length() < 0.5, "{from} against {}", muzzle(&play));
    assert!((to - round.position).length() < 1e-3);

    // Out at 0.75 s, and gone with the round at 3 s.
    while play.hero.time_ms < at + 750.0 {
        play.tick(tick, [0.0; 2]);
    }
    assert_eq!(beam(&play, play.hero.time_ms).map(|s| s.len()), Some(0), "a fade of 0 draws nothing");
    while play.hero.time_ms < at + 3000.0 + tick {
        play.tick(tick, [0.0; 2]);
    }
    assert!(beam(&play, play.hero.time_ms).is_none(), "the round and its effects are deleted");
    assert!(play.spent.iter().all(|(r, _)| r.id != round.id));
}

#[test]
#[ignore = "needs the game install"]
fn a_dummys_part_is_damaged_at_half_knocked_off_at_nothing_and_its_base_takes_the_rest() {
    use parkan_sim::combat::Event;
    use parkan_world::play::Play;

    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    let target_of =
        |play: &Play, object: usize| play.battle.objects.iter().position(|&o| o == object).unwrap();
    // Object 1, an `l_targ` on `r_h_01`: base ASbs, ASd1 and ASd2 on it, ASd3 on ASd2
    // (docs/26, "What the shipped files give").
    assert!(m.objects[1].path.to_ascii_lowercase().ends_with("l_targ.dat"));
    let t = target_of(&play, 1);
    let big = target_of(&play, 31);
    assert_eq!((play.battle.death_ms[t], play.battle.death_ms[big]), (3000.0, 5000.0));
    let life = |play: &Play| play.battle.combat.targets[t].parts[0].life.clone().unwrap();
    assert_eq!(life(&play).nodes.iter().map(|n| n.stages).collect::<Vec<_>>(), vec![1, 2, 2, 2]);
    let hit = |play: &mut Play, node: usize, damage: f32| {
        play.battle.combat.targets[t].parts[0].life.as_mut().unwrap().hit(node, damage);
        play.tick(tick, [0.0; 2])
    };
    let has = |events: &[Event], want: &dyn Fn(&Event) -> bool| events.iter().any(want);

    // Half its life: the damaged block, and its explosion.
    let sprites = play.fx.instances.len();
    let events = hit(&mut play, 1, 400.0);
    assert!(has(&events, &|e| matches!(e, Event::Staged { target, node: 1, .. } if *target == t)));
    assert_eq!(life(&play).nodes[1].block(), 1);
    assert!(play.fx.instances.len() > sprites, "explode_aim_S plays");

    // Nothing left: knocked off, it drops, drawn, until it meets the ground, then explodes
    // and goes.
    let before = play.battle.combat.targets[t].parts[0].nodes[1].translation;
    let events = hit(&mut play, 1, 400.0);
    assert!(has(&events, &|e| matches!(e, Event::KnockedOff { target, node: 1, .. } if *target == t)));
    let (mut gone_at, mut dropped) = (None, 0.0);
    for k in 1..=200 {
        let events = play.tick(tick, [0.0; 2]);
        if has(&events, &|e| matches!(e, Event::Hidden { target, node: 1, .. } if *target == t)) {
            gone_at = Some(k);
            break;
        }
        let now = play.battle.combat.targets[t].parts[0].nodes[1].translation;
        dropped = before[2] - now[2];
    }
    let seconds = gone_at.map(|k| k as f32 / 60.0);
    assert!(seconds.is_some_and(|s| (0.1..=1.5).contains(&s)), "gone once it lands: {seconds:?}");
    assert!(dropped > 0.2, "it fell {dropped} before it went");
    assert!(life(&play).nodes[1].hidden());
    assert!(play.battle.combat.targets[t].alive, "the rest stands");
    assert!(!play.ground.solids[t].faces.is_empty());

    // The base: every part still standing goes with it, and the unit is deleted 3 s on.
    let events = hit(&mut play, 0, 500.0);
    assert!(has(&events, &|e| matches!(e, Event::Killed { target } if *target == t)));
    play.tick(tick, [0.0; 2]);
    assert!([0, 2, 3].iter().all(|&n| life(&play).nodes[n].hidden()), "{:?}", life(&play).nodes);
    assert!(play.ground.solids[t].faces.is_empty(), "nothing left to collide with");
    assert!(!play.deleted[t]);
    for _ in 0..185 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(play.deleted[t] && play.killed.contains(&1));
}

#[test]
#[ignore = "needs the game install"]
fn a_small_bunker_taken_by_the_player_turns_its_flamers_on_an_enemy_holds_them_close_in_and_a_neutral_one_does_not()
 {
    let (mut play, _) = mission_03_play();
    // The engine's departure: a building holds its fire below its turret's reach.
    play.building_fire_floor = true;
    let (bunker, _) = play.emplacements[0];
    assert_eq!(play.units[bunker].clan, Some(2), "the Small Bunker starts neutral");
    assert_eq!(play.emplacements[0].1.guns.len(), 2, "two HFTB");
    let lowest = play.emplacements[0].1.lowest_sight().expect("a pitch channel");
    assert!((lowest.to_degrees() + 15.0).abs() < 0.1, "its sight looks down to 15°: {}", lowest.to_degrees());
    // An enemy flyer, shut down, held 5 m over the ground east of the bunker. The flame's frame
    // flags carry 8, so its distance scores 1 wherever the target stands (docs/29, "How the AI
    // fires").
    let flyer = play.robots.iter().position(|(t, _)| play.units[*t].logical_id == 3).unwrap();
    let flyer_target = play.robots[flyer].0;
    let base = play.battle.combat.targets[bunker].position;
    let hold = |p: &mut parkan_world::play::Play, off: f32, fired: &mut usize, aimed: &mut bool| {
        let at = base + glam::Vec3::new(off, 0.0, 0.0);
        let ground = p.ground.below(at.x, at.y, 1.0e5).map_or(at.z, |h| h.point.z);
        p.robots[flyer].1.walker.body.position = at.with_z(ground + 5.0);
        *fired += p.battle.combat.rounds.iter().filter(|r| r.owner == Some(bunker)).count();
        *aimed |= p.emplacements[0].1.fire_target == Some(flyer_target);
    };
    let (mut fired, mut aimed) = (0, false);
    play_for(&mut play, 6.0, |p| hold(p, 80.0, &mut fired, &mut aimed));
    assert_eq!(fired, 0, "a neutral clan's bunker runs no fire control");
    assert_eq!(play.emplacements[0].1.fire_target, None);
    assert!(!aimed);

    // Taken, as its pod's capture changes its clan (docs/27), it aims. At its door, 20 m out and
    // below the lowest its sight looks, it holds its fire.
    play.units[bunker].clan = Some(play.player_clan);
    play_for(&mut play, 6.0, |p| hold(p, 20.0, &mut fired, &mut aimed));
    assert!(aimed, "it traces the flyer at its door");
    assert_eq!(fired, 0, "and holds its fire on it");
    // 80 m out it fires.
    aimed = false;
    play_for(&mut play, 6.0, |p| hold(p, 80.0, &mut fired, &mut aimed));
    assert!(fired > 0, "the player's bunker fires at the enemy");
    assert!(aimed, "on the flyer");
}

/// Outflanking Maneuver's units wear the armour fitted into their chassis's slot, and every hit on
/// any node of theirs passes through it (docs/26, "Armour"): the hero and the enemy's `12wel2`
/// warbots `i_arm_l_02`, the helicopters `i_arm_t_df`, the tower `i_arm_b_05`. A laser bolt of 250
/// takes 165 off a `12wel2`'s hull of 224, so it stands after one bolt where it fell to one before.
#[test]
#[ignore = "needs the game install"]
fn c01_m03s_units_wear_their_fitted_armour_on_every_node_and_it_cuts_a_laser_bolt_by_a_third() {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::C01_MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let mut play = parkan_world::play::Play::load(&game, &m).unwrap().expect("a hero");

    let near = |a: Option<(f32, f32)>, b: (f32, f32)| {
        a.is_some_and(|a| (a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-7)
    };
    let light = (0.62362, 0.00014);
    assert!(
        play.hero.lives.iter().flatten().all(|l| near(l.armour, light)),
        "the hero's i_arm_l_02 on every part"
    );
    let of = |path: &str| {
        let objects = &play.battle.objects;
        (0..objects.len())
            .filter(|&t| m.objects[objects[t]].path.eq_ignore_ascii_case(path))
            .collect::<Vec<_>>()
    };
    for (path, armour) in [
        ("UNITS\\UNITS\\BATTLE\\12wel2.dat", light),
        ("UNITS\\UNITS\\BATTLE\\12hel1.dat", (0.85425, 0.00008)),
        ("UNITS\\UNITS\\BATTLE\\12tower.dat", (0.21893, 0.00028)),
    ] {
        let targets = of(path);
        assert!(!targets.is_empty(), "{path} is placed");
        for t in targets {
            let lives: Vec<_> =
                play.battle.combat.targets[t].parts.iter().filter_map(|p| p.life.as_ref()).collect();
            assert!(
                lives.len() > 1 && lives.iter().all(|l| near(l.armour, armour)),
                "{path}: {:?}",
                lives[0].armour
            );
        }
    }

    // An enemy `12wel2`'s hull: 320 hit points at MEDIUM's level ratio of 0.7.
    let wel = *of("UNITS\\UNITS\\BATTLE\\12wel2.dat")
        .iter()
        .find(|&&t| m.objects[play.battle.objects[t]].clan_id() == Some(1))
        .expect("an enemy 12wel2");
    let hull = play.battle.combat.targets[wel].parts[0].life.as_mut().unwrap();
    assert!((hull.nodes[0].max - 224.0).abs() < 0.01);
    let bolt = play
        .battle
        .combat
        .kinds
        .iter()
        .find(|k| k.name.eq_ignore_ascii_case("bl_h_01"))
        .map(|k| k.hit_points + k.hit.as_ref().unwrap().damage);
    assert_eq!(bolt, Some(250.0), "the hero's laser bolt");
    assert!(!hull.hit(0, 250.0), "one bolt leaves the hull standing");
    assert!(
        (hull.nodes[0].life - (224.0 - (0.62362 * 250.0 + 0.00014 * 250.0 * 250.0))).abs() < 0.05,
        "{}",
        hull.nodes[0].life
    );
    assert!(hull.hit(0, 250.0), "and a second destroys it");
}

/// A turret shot off takes what hangs on it: in the game's one merged model a gun part's node 0
/// is the turret's socket, so the walk from the turret's main node down destroys the socket and
/// the gun's nodes below it (docs/26, "Children go with their parent"), and a gun whose node has
/// no life starts no stroke (`Control.dll:0x10029cc3`), nor a radar a scan. Outflanking
/// Maneuver's first `12tower`, firing on the hero from 150 m, falls silent once its turret goes;
/// an enemy `12wel2` shot through one gun keeps firing the other two.
#[test]
#[ignore = "needs the game install"]
fn c01_m03s_tower_whose_turret_is_shot_off_loses_its_gun_and_radar_and_fires_no_more() {
    let mut play = campaign_play(gamedir::C01_MISSION_03);
    let tower = play.units.iter().position(|u| u.logical_id == 23).expect("the first 12tower");
    let e = play.robots.iter().position(|(t, _)| *t == tower).expect("the tower is a robot");
    // The rounds the tower has in the air, by id.
    let rounds = |play: &parkan_world::play::Play| -> Vec<u64> {
        play.battle.combat.rounds.iter().filter(|r| r.owner == Some(tower)).map(|r| r.id).collect()
    };
    assert!(play.stand_facing(tower, 150.0, 0.0));
    let mut seen = std::collections::HashSet::new();
    for _ in 0..(5 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        seen.extend(rounds(&play));
    }
    assert!(!seen.is_empty(), "the tower fires on the hero");

    // The turret's main node `TTur` destroyed where it stands.
    let robot = &play.robots[e].1;
    let turret = robot.turret_part;
    let gun_part = robot.gun_parts.iter().flatten().next().expect("the tower's gun is a part").part;
    assert_eq!(play.battle.combat.targets[tower].parts[gun_part].host, Some((turret, 5)), "on `Base_gun`");
    assert!(robot.radar_node.is_some_and(|(p, _)| p == turret), "{:?}", robot.radar_node);
    play.battle.combat.targets[tower].parts[turret].life.as_mut().unwrap().lose(1, 1.0e9);
    // A stroke under way may still send its round in the first frames.
    for _ in 0..30 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        seen.extend(rounds(&play));
    }
    let mut after = Vec::new();
    for _ in 0..(8 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        after.extend(rounds(&play).into_iter().filter(|id| !seen.contains(id)));
    }
    let target = &play.battle.combat.targets[tower];
    assert!(target.alive, "the tower's base stands");
    let gun = target.parts[gun_part].life.as_ref().unwrap();
    assert!(
        gun.nodes.iter().all(|n| n.destroyed && n.hidden()),
        "the gun goes with its socket: {:?}",
        gun.nodes
    );
    let robot = &play.robots[e].1;
    assert!(robot.guns.iter().all(|g| g.broken) && robot.radar.broken);
    assert!(after.is_empty(), "and the tower fires no more: {after:?}");

    // An enemy `12wel2` with one gun part shot through: that gun is broken, the others are not.
    let mut play = campaign_play(gamedir::C01_MISSION_03);
    let (r, (wel, robot)) = play
        .robots
        .iter()
        .enumerate()
        .find(|(_, (t, robot))| {
            robot.gun_parts.iter().flatten().count() == 3 && play.units[*t].clan == Some(1)
        })
        .map(|(r, (t, robot))| (r, (*t, robot)))
        .expect("an enemy warbot with three gun parts");
    let shot = robot.gun_parts.iter().flatten().next().unwrap().part;
    play.battle.combat.targets[wel].parts[shot].life.as_mut().unwrap().lose(1, 1.0e9);
    for _ in 0..3 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let robot = &play.robots[r].1;
    let broken: Vec<bool> = robot
        .gun_parts
        .iter()
        .zip(&robot.guns)
        .filter_map(|(p, g)| p.as_ref().map(|p| (p.part == shot) == g.broken))
        .collect();
    assert!(broken.iter().all(|&b| b), "only the gun shot through is broken: {broken:?}");
    assert!(!robot.radar.broken);
}

/// Ballen's Crossing: the hero boards the neutral HQ, whose two `LWML2M` winged missiles
/// (round `bm_m_04`: range 700, a seeker of 500 reach, a 0.7 rad cone and a 7 s lock) are the
/// only guns that reach the enemy towers on the hill. The target list is the driven unit's,
/// and its target goes to the driven unit's guns (`iron3d.dll:0x10091a80`): from 380 m, looking
/// at a tower, the missiles lock on and fire, where they kept reporting out of range because
/// the hero's own guns took the target. They steer at the tower's node sphere's centre, not
/// its foot on the crest (`Control.dll:0x100248b6`), and bring it down.
#[test]
#[ignore = "needs the game install"]
fn c02_m02s_hq_driven_by_the_hero_locks_its_winged_missiles_on_a_tower_and_brings_it_down() {
    use parkan_formats::controls::CMD_JAMES_SELECT_TARGET;
    use parkan_sim::guns::GATE_CLEAR;
    use parkan_world::play::Play;

    let mut play = campaign_play(gamedir::C02_MISSION_02);
    let tick = 1000.0 / 60.0;
    let hq = play.units.iter().position(|u| u.logical_id == 11).expect("the HQ");
    let tower = play.units.iter().position(|u| u.logical_id == 20).expect("a tower");
    stand_facing(&mut play, hq, 12.0, 0.0);
    play.tick(tick, [0.0; 2]);
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(hq) {
            break;
        }
        play.targets.select_next();
    }
    assert_eq!(play.targets.current, Some(hq));
    press_enter(&mut play);
    assert!(play.driving.as_ref().is_some_and(|d| d.target == hq), "aboard the HQ");

    // The HQ 380 m north-east of the tower on the high ground, facing it: inside its own radar's
    // 400 and its missiles' 500, outside both towers' radar's 350, and with nothing of the hill
    // between it and the tower's middle. Toward the tower from the HQ's own start the hill's
    // crest hides it: a winged missile flies straight at its target, and meets the crest.
    let aim = play.battle.combat.targets[tower].aim;
    let way = -glam::Vec2::new(30f32.to_radians().cos(), 30f32.to_radians().sin());
    {
        let Play { robots, ground, .. } = &mut play;
        let robot = &mut robots.iter_mut().find(|(t, _)| *t == hq).unwrap().1;
        robot.walker.body.yaw = (-way.x).atan2(way.y);
        put(&mut robot.walker, ground, (aim.truncate() - way * 380.0).extend(aim.z + 150.0));
    }
    for _ in 0..30 {
        play.tick(tick, [0.0; 2]);
    }
    // Tab, as the player picks it.
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(tower) {
            break;
        }
        let eye = play.eye();
        let view = parkan_world::play::View {
            eye: eye.position,
            look: eye.forward,
            view_proj: glam::Mat4::IDENTITY,
            shift: false,
        };
        play.command(CMD_JAMES_SELECT_TARGET, &view);
    }
    assert_eq!(play.targets.current, Some(tower), "the tower is on the HQ's radar");
    let reports = |play: &Play| play.driven().guns.iter().map(|g| g.report).collect::<Vec<_>>();
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    let at = play.driven().walker.body.position;
    assert!((at.truncate().distance(aim.truncate()) - 380.0).abs() < 5.0, "{at}");
    assert!(play.driven().guns.iter().all(|g| g.target == Some(tower)), "the HQ's missiles take the target");
    assert_eq!(reports(&play), vec![GATE_CLEAR; 2], "in range and on the barrel, locking");
    assert!(play.hero.guns.iter().all(|g| g.target != Some(tower)), "not the hero's own guns");

    // Held on it, the lock runs out and the button sends the missiles off at it.
    let selected: Vec<usize> =
        (0..play.driven().guns.len()).filter(|&i| play.driven().guns[i].selected).collect();
    assert!(!selected.is_empty(), "a missile rack is selected");
    for _ in 0..(8 * 60) {
        play.tick(tick, [0.0; 2]);
    }
    let before: Vec<i32> = play.driven().guns.iter().map(|g| g.rounds).collect();
    let life = |play: &Play| {
        play.battle.combat.targets[tower]
            .parts
            .iter()
            .filter_map(|p| p.life.as_ref())
            .map(|l| l.total())
            .sum::<f32>()
    };
    let whole = life(&play);
    play.driving.as_mut().unwrap().pilot.fire = true;
    let mut fired = false;
    for _ in 0..(3 * 60) {
        play.tick(tick, [0.0; 2]);
        fired |= play.battle.combat.rounds.iter().any(|r| r.target == Some(tower));
    }
    play.driving.as_mut().unwrap().pilot.fire = false;
    let after: Vec<i32> = play.driven().guns.iter().map(|g| g.rounds).collect();
    assert!(
        fired && after.iter().sum::<i32>() < before.iter().sum::<i32>(),
        "a missile leaves for the tower: {before:?} -> {after:?}"
    );
    // They fly the 380 m in under 9 s, and each does 60,000 where the tower holds 16,452.
    for _ in 0..(15 * 60) {
        play.tick(tick, [0.0; 2]);
    }
    assert!(!play.battle.combat.targets[tower].alive, "the tower falls: {whole} -> {}", life(&play));
}

/// A tree and a stone are agents like any other: their `STAT` record names a `.ndp` and a
/// `.ctl`, the agent loader gives them a control system and so a life system, and the
/// control tick rescales every node's life by the placement scale's three factors
/// multiplied (docs/26, "Vegetation and rock carry node life").
#[test]
#[ignore = "needs the game install"]
fn mission_01s_trees_and_stones_carry_node_life_at_their_placement_scale_cubed() {
    use parkan_formats::mission::{KIND_ROCK, KIND_VEGETATION};
    use parkan_sim::combat::Event;

    let (mut play, m) = mission_01_play();
    let tick = 1000.0 / 60.0;
    let target_of = |object: usize| play.battle.objects.iter().position(|&o| o == object).unwrap();
    let maxima = |t: usize| {
        play.battle.combat.targets[t].parts[0]
            .life
            .as_ref()
            .map(|l| l.nodes.iter().map(|n| n.max).collect::<Vec<_>>())
    };

    let scenery: Vec<usize> =
        (0..m.objects.len()).filter(|&i| matches!(m.objects[i].kind, KIND_VEGETATION | KIND_ROCK)).collect();
    assert_eq!(scenery.len(), 22, "Mission 01 places 11 s_tree_04, 5 s_tree_29 and 6 stones");
    assert!(scenery.iter().all(|&i| maxima(target_of(i)).is_some()), "every one of them has life");

    // `s_stone_07` is one node of 500,000; object 3 stands at scale 20, so 8,000 times it.
    let stone = target_of(3);
    assert_eq!(m.objects[3].placed_scale(), 20.0);
    assert_eq!(maxima(stone), Some(vec![500_000.0 * 8_000.0]));
    // `s_tree_04` is a trunk of 3,000 and ten leaves of 1; object 15 stands at scale 3.
    let tree = target_of(15);
    assert_eq!(m.objects[15].placed_scale(), 3.0);
    assert_eq!(maxima(tree), Some([81_000.0].into_iter().chain([27.0; 10]).collect::<Vec<_>>()));
    // `s_tree_29` stands at 1: three nodes of 28,000, untouched by a scale.
    assert_eq!(maxima(target_of(25)), Some(vec![28_000.0; 3]));

    // Its controller's +92 is 5,000 ms on 79 of the 81 scenery records, and it is agent
    // kind 10, not a building: node 0's death kills it and it is deleted five seconds on.
    assert_eq!(play.battle.death_ms[tree], 5000.0);
    play.battle.combat.targets[tree].parts[0].life.as_mut().unwrap().hit(0, 81_000.0);
    let events = play.tick(tick, [0.0; 2]);
    assert!(events.iter().any(|e| matches!(e, Event::Killed { target } if *target == tree)));
    assert!(!play.battle.combat.targets[tree].alive && play.ground.solids[tree].faces.is_empty());
    for _ in 0..(6 * 60) {
        play.tick(tick, [0.0; 2]);
    }
    assert!(play.deleted[tree] && play.killed.contains(&15), "the felled tree is removed");
}
