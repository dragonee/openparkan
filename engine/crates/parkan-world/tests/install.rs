//! Against the install: `cargo test -- --ignored`.

use parkan_formats::gamedir;
use parkan_world::terrain;
use parkan_world::textures::TextureStore;

#[test]
#[ignore = "needs the game install"]
fn tut_1_builds_its_ground_from_resolved_textures() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = terrain::map_dir(&game, "DATA\\MAPS\\Tut_1\\land").unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    let t = terrain::build(&dir, &mut store).unwrap();
    assert_eq!(t.indices.len() as usize / 3, t.land.lod_split());
    assert!(t.groups.iter().all(|g| g.layer1.still.texture.is_some()), "every layer-1 material resolves");
    let water: Vec<_> = t.groups.iter().filter(|g| g.water).collect();
    assert!(!water.is_empty() && water.iter().all(|g| g.layer1.material == "WATER"));
    let blue = water[0].layer1.still.diffuse;
    assert!(blue[2] > blue[0], "water is tinted blue by its material: {blue:?}");
    // The water's box, as docs/03-terrain.md measures it.
    let w = t.water.expect("Tut_1 has water");
    let near = |a: f32, b: f32| (a - b).abs() < 0.06;
    assert!(
        near(w.min[0], 385.5) && near(w.max[0], 1480.5) && near(w.min[1], 255.8) && near(w.max[1], 1531.7),
        "{w:?}"
    );
    assert!(near(w.level, -1.7255), "{w:?}");
}

#[test]
#[ignore = "needs the game install"]
fn a_buoys_beam_flickers_through_three_cells_of_sun4_in_pinkish_red() {
    use parkan_world::textures::WHOLE_CELL;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut store = TextureStore::open(&game).unwrap();
    // docs/07-objects.md, "How a material reaches the device": flags 4, `SUN4.0`, ambient
    // #dc1414, #f00019 and #ffb97d on cells 0, 1 and 2, keys at 50, 100, 150 and 200 ms.
    let beam = store.look("HLP_RAY_R").unwrap();
    assert_eq!(beam.blend_mode, 4);
    let texture = &store.textures[beam.still.texture.unwrap()];
    assert_eq!((texture.name.to_ascii_uppercase().as_str(), texture.width), ("SUN4.0", 256));
    let a = beam.animation.as_ref().expect("its track plays");
    let cells: Vec<[f32; 4]> = a.keys.iter().map(|k| k.0.cell).collect();
    let strip = |x: f32, y: f32| [x, y, 0.25, 0.5];
    assert_eq!(cells, vec![strip(0.0, 0.0), strip(0.25, 0.0), strip(0.0, 0.5), strip(0.25, 0.0)]);
    assert_eq!(a.keys.iter().map(|k| k.1).collect::<Vec<_>>(), vec![50.0, 100.0, 150.0, 200.0]);
    let byte = |c: [f32; 3]| c.map(|v| (v * 255.0).round() as u8);
    assert_eq!(byte(beam.at(0.0).ambient), [0xdc, 0x14, 0x14]);
    assert_eq!(byte(beam.at(100.0).ambient), [0xff, 0xb9, 0x7d], "entry 2 has arrived at key 1's end");
    assert_eq!(beam.at(125.0).cell, strip(0.0, 0.5));
    assert_eq!((beam.still.alpha, beam.still.diffuse), (1.0, [0.0; 3]), "a black diffuse: the unlit glow");
    // The lamp and the base ring draw whole textures, opaque.
    let lamp = store.look("HLP_LAMP_R").unwrap();
    assert_eq!((lamp.blend_mode, lamp.still.cell), (0, WHOLE_CELL));
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_objects_build_and_stand_on_the_ground() {
    use parkan_formats::mission;
    use parkan_world::{assembly::Assembly, models};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    let mut assembly = Assembly::new(&game).unwrap();
    let objects = models::build(&mut assembly, &mut store, &m).unwrap();
    assert_eq!(objects.instances.len(), m.objects.len(), "every Mission 01 object resolves");
    let land = terrain::build(&terrain::map_dir(&game, &m.map_path).unwrap(), &mut store).unwrap().land;
    // A unit's mission z puts its lowest vertex on the terrain: the median unit
    // is 0.03 off it (docs/07-objects.md, "The ground datum"). The target
    // dummies are the exceptions here, as the Python reference measures too.
    let mut offsets = Vec::new();
    for (instance, &i) in objects.instances.iter().zip(&objects.placed) {
        let o = &m.objects[i];
        if o.kind != mission::KIND_UNIT {
            continue;
        }
        let ground = land.height_at(o.position[0], o.position[1]).unwrap();
        let feet = o.position[2] + objects.models[instance.model].lowest();
        if o.path.to_ascii_uppercase().contains("\\HERO\\") {
            assert!((feet - ground).abs() < 0.1, "the hero stands {:.2} off the ground", feet - ground);
        }
        offsets.push((feet - ground).abs());
    }
    offsets.sort_by(f32::total_cmp);
    assert!(offsets[offsets.len() / 2] < 3.0, "median unit off the ground by {}", offsets[offsets.len() / 2]);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_leaves_blend_and_write_depth_and_nothing_on_it_is_translucent() {
    use parkan_formats::mission;
    use parkan_world::{assembly::Assembly, models};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    let mut assembly = Assembly::new(&game).unwrap();
    let objects = models::build(&mut assembly, &mut store, &m).unwrap();
    // A batch is translucent, drawn last without writing depth, only for an ambient alpha below
    // 1; the palm's `TF2` leaves blend at mode 4 in the first list, writing depth
    // (docs/07, "What a blended batch writes").
    let groups: Vec<_> = objects.models.iter().flat_map(|model| &model.groups).collect();
    assert!(groups.iter().all(|g| !g.look.translucent()), "no Mission 01 look is translucent");
    let leaves: Vec<_> = groups.iter().filter(|g| g.look.material.eq_ignore_ascii_case("TF2")).collect();
    assert!(!leaves.is_empty(), "the palms' leaves");
    assert!(leaves.iter().all(|g| g.look.blend_mode == 4), "leaves blend SRCALPHA/INVSRCALPHA");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_bridge_halves_meet() {
    use parkan_formats::mission;
    use parkan_world::{assembly::Assembly, models};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    let mut assembly = Assembly::new(&game).unwrap();
    let objects = models::build(&mut assembly, &mut store, &m).unwrap();
    // Placed as T(position) Rz(rotation) S(scale), the two halves of a bridge,
    // turned pi apart, join end to end (docs/04-missions.md).
    let world = |i: usize, sense: f32| -> Vec<[f32; 3]> {
        let instance = objects.instances[i];
        let (s, c) = (sense * instance.rotation).sin_cos();
        objects.models[instance.model]
            .vertices
            .iter()
            .map(|v| {
                let [x, y, z] = v.position.map(|p| p * instance.scale);
                let [px, py, pz] = instance.position;
                [px + c * x - s * y, py + s * x + c * y, pz + z]
            })
            .collect()
    };
    let gap = |a: &[[f32; 3]], b: &[[f32; 3]]| {
        a.iter()
            .flat_map(|p| b.iter().map(move |q| (0..3).map(|k| (p[k] - q[k]).powi(2)).sum::<f32>().sqrt()))
            .fold(f32::MAX, f32::min)
    };
    let halves: Vec<usize> = (0..objects.instances.len())
        .filter(|&i| m.objects[objects.placed[i]].path.to_ascii_uppercase().contains("BRIDGE"))
        .collect();
    assert_eq!(halves.len(), 2);
    let joined = gap(&world(halves[0], 1.0), &world(halves[1], 1.0));
    let negated = gap(&world(halves[0], -1.0), &world(halves[1], -1.0));
    assert!(joined < 0.05, "the bridge halves are {joined:.2} apart");
    assert!(negated > 1.0, "control: with the angle negated they are {negated:.2} apart");
}

#[test]
#[ignore = "needs the game install"]
fn the_heros_eye_looks_level_along_its_heading_and_pitches_with_the_turret() {
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
    hero.tick(1000.0 / 60.0, [0.0; 2], &ground);

    let eye = hero.eye();
    let facing = hero.walker.body.forward();
    // The pitch channel starts at 0.2727, within 0.6 degrees of level (docs/30).
    assert!(eye.forward.z.abs() < 0.02, "level: {}", eye.forward);
    assert!(eye.forward.dot(facing) > 0.999, "{} against {facing}", eye.forward);
    assert!((eye.fov_x - 1.3).abs() < 1e-6 && (eye.near - 0.1).abs() < 1e-6);
    let origin = hero.walker.body.position;
    let lift = eye.position.z - origin.z;
    assert!(lift > 0.5 && lift < 3.0, "the eye {lift} above the origin");
    assert!((eye.position - origin).truncate().length() < 1.0);

    // Full up is frame 57, the sight at +79.4 degrees.
    hero.rig.aim[1] = 0.0;
    for _ in 0..120 {
        hero.tick(1000.0 / 60.0, [0.0; 2], &ground);
    }
    let up = hero.eye().forward;
    assert!((up.z.asin().to_degrees() - 79.4).abs() < 1.0, "{}", up.z.asin().to_degrees());
    assert!((hero.eye().position - eye.position).length() < 0.05, "pitch does not move the eye");
}

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

#[test]
#[ignore = "needs the game install"]
fn the_heros_eye_swings_with_the_run_but_never_lunges_and_holds_steady_when_asked() {
    use parkan_formats::{landmesh, mission};
    use parkan_sim::ground::Ground;
    use parkan_world::{assembly::Assembly, hero::Hero};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    let ground = Ground::new(land);
    let wrap = |a: f32| (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    // Over 3 s holding W, from the first second on: the most the eye's look turns from the
    // unit's heading, and the most the eye stands out from the body across the ground.
    let run = |steady: bool| {
        let mut assembly = Assembly::new(&game).unwrap();
        let mut hero = Hero::load(&mut assembly, &m).unwrap().expect("Mission 01 has a hero");
        hero.steady = steady;
        hero.key("SCAN_W", true);
        let (mut swing, mut reach) = (0.0_f32, 0.0_f32);
        for tick in 0..180 {
            hero.tick(1000.0 / 60.0, [0.0; 2], &ground);
            if tick < 60 {
                continue;
            }
            let t = hero.time_ms;
            let eye = hero.eye();
            swing =
                swing.max(wrap((-eye.forward.x).atan2(eye.forward.y) - hero.walker.drawn_heading(t)).abs());
            reach = reach.max((eye.position - hero.walker.drawn(t).0).truncate().length());
        }
        (swing, reach)
    };
    // The body node yaws 10 degrees each way on a run (docs/30), and its travel is cleared (docs/07).
    let (swing, reach) = run(false);
    assert!((swing.to_degrees() - 10.0).abs() < 1.5, "the game's view swings {} degrees", swing.to_degrees());
    assert!(reach < 1.0, "the eye stands {reach} m out from the body");
    let (swing, reach) = run(true);
    assert!(swing.to_degrees() < 0.5, "the steady view still swings {} degrees", swing.to_degrees());
    assert!(reach < 1.0, "the steady eye stands {reach} m out from the body");
}

#[test]
#[ignore = "needs the game install"]
fn the_game_font_opens_and_lays_out_a_line() {
    use parkan_world::text::{GameFont, TextRun};

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let font = GameFont::open(&game).unwrap();
    // A 128 × 128 atlas of seven rows 18 pixels apart (docs/12).
    assert_eq!((font.width, font.height), (128, 128));
    assert_eq!(font.atlas.len(), 128 * 128 * 4);
    assert_eq!(font.line_height, 18.0);
    let run = TextRun::new("Objective is completed", [0.0, 0.0]);
    let placed = font.layout(&run);
    assert_eq!(placed.len(), "Objectiveiscompleted".len(), "every letter is drawn, no space is");
    let width = font.width(&run);
    assert!(width > 100.0 && width < 300.0, "the line is {width} pixels");
    assert!(placed.windows(2).all(|w| w[1].at[0] > w[0].at[0]), "the pen only moves right");
}

/// Mission 01's play with its progression, the hero standing still.
fn mission_01_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 01 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// Stand the hero `distance` from `target`'s placement, facing it, on the ground.
fn stand_facing(play: &mut parkan_world::play::Play, target: usize, distance: f32, around: f32) {
    assert!(play.stand_facing(target, distance, around), "somewhere level to stand");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01_greets_the_hero_and_completes_its_first_objective_once_the_targets_are_gone() {
    use parkan_world::progress::{Say, Sender};

    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    play.tick(tick, [0.0; 2]);
    let p = play.progression.as_ref().unwrap();
    assert!(p.unanswered.is_empty(), "tut1_pl2 calls only what the engine answers: {:?}", p.unanswered);
    // The hero starts inside route 0: messages 11 and 14 on the first run (docs/34).
    let played: Vec<i64> = p.progress.played.iter().filter(|(_, v)| **v).map(|(k, _)| *k).collect();
    assert_eq!(played, vec![11, 14]);
    let welcome = p.messages.get(11).expect("message 11");
    let sender = if welcome.info_system { Sender::Information } else { Sender::Training };
    assert!(play.says.contains(&Say::Text(sender, welcome.text.clone().expect("T01_I01's text"))));
    let voices = play.says.iter().filter(|s| matches!(s, Say::Voice(v) if v.exists())).count();
    assert_eq!(voices, 2, "{:?}", play.says);
    play.says.clear();

    // The five targets go: Trgt's robots count down, and the next run completes objective 0.
    let trgt: Vec<i32> = play
        .progression
        .as_ref()
        .unwrap()
        .progress
        .units
        .iter()
        .filter(|u| u.clan == 1)
        .map(|u| u.id)
        .collect();
    assert_eq!(trgt.len(), 5);
    for id in trgt {
        play.progression.as_mut().unwrap().progress.destroyed(id);
    }
    for _ in 0..130 {
        play.tick(tick, [0.0; 2]);
    }
    let p = play.progression.as_ref().unwrap();
    assert_eq!(p.progress.objectives.iter().map(|o| o.state).collect::<Vec<_>>(), vec![1, 0, 0]);
    assert!(p.progress.played[&17] && p.progress.played[&12]);
    assert_eq!(p.progress.outcome, None);
    let done = p.strings[&parkan_world::progress::STRING_OBJECTIVE_COMPLETE].clone();
    assert!(play.says.contains(&Say::Text(Sender::System, done)), "{:?}", play.says);
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
fn the_hero_sounds_its_steps_as_it_runs_and_its_arm_as_a_gun_is_put_away() {
    use parkan_sim::effects::CueKind;

    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    // The cannon's arm out first: it starts selected and unfolds.
    for _ in 0..120 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    play.cues.clear();
    play.hero.key("SCAN_W_1", true);
    play.hero.update_input();
    play.hero.key("SCAN_W_1", false);
    for _ in 0..60 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let arm: Vec<String> =
        play.cues.iter().filter(|c| c.kind == CueKind::Once).map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(arm.iter().any(|s| s.starts_with("h_gh")), "deselecting the cannon sounds its arm: {arm:?}");

    play.cues.clear();
    play.hero.key("SCAN_W", true);
    for _ in 0..120 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let steps = play.cues.iter().filter(|c| c.sound.to_ascii_lowercase().starts_with("step_h")).count();
    // docs/13: three steps a 0.405 s run cycle, and the first second starts up through
    // walk and transition states.
    assert!(steps >= 6, "{steps} steps in 2 s: {:?}", play.cues.iter().map(|c| &c.sound).collect::<Vec<_>>());
}

#[test]
#[ignore = "needs the game install"]
fn the_heros_breath_is_not_heard_while_its_steps_are() {
    // docs/11, "How a sound is heard": Mission 01's recording has no H_breath.wav.
    let (mut play, _) = mission_01_play();
    let tick = 1000.0 / 60.0;
    play.hero.key("SCAN_W", true);
    for _ in 0..300 {
        play.hero.update_input();
        play.tick(tick, [0.0; 2]);
    }
    let sounds: Vec<String> = play.cues.iter().map(|c| c.sound.to_ascii_lowercase()).collect();
    assert!(!sounds.iter().any(|s| s == "h_breath.wav"), "{sounds:?}");
    assert!(sounds.iter().any(|s| s.starts_with("step_h")), "{sounds:?}");
    let breath =
        play.fx.instances.iter().filter(|(_, i)| i.effect.name.eq_ignore_ascii_case("hero_breath")).count();
    assert_eq!(breath, 1, "the breath's effect still runs");
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

/// A unit the Wizard drives is held by a slope as the player's own machine is (docs/24,
/// "Ground and slope"). A written velocity replaces the machine's own at the top of every
/// step, so the brake's pull never built up on one: an AI unit walked up this face at 8 m/s
/// where the same chassis under the player does not move at all.
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

    // Shift picks: only the second wingman, then Standby.
    play.command("CMD_JAMES_WINGMAN_MENU", &view(true));
    assert!(play.wingman_digit(2));
    play.command("CMD_JAMES_WINGMAN_MENU", &view(false));
    assert!(play.wingman_digit(1));
    let orders: Vec<i32> = play.robots.iter().filter_map(|(_, r)| r.order.map(|o| o.code)).collect();
    assert_eq!(orders.iter().filter(|&&c| c == STAYGROUND).count(), 1);
    assert_eq!(orders.iter().filter(|&&c| c == FOLLOW).count(), 1);
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

/// Stand a machine at `at`'s xy on the ground below it.
fn put(w: &mut parkan_sim::machine::Walker, ground: &parkan_sim::ground::Ground, at: glam::Vec3) {
    w.body.position = at;
    w.follow_ground(ground);
    w.from = (w.body.position, w.body.yaw);
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
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    assert_eq!(play.hero.walker.body.position, at, "a dead hero stays where it died");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_buoys_hold_the_hero_off_their_cones() {
    use glam::Vec3;
    let (_, m) = mission_01_play();
    // The five `s_tree_29` buoys; each cone reaches 1.94 from its axis (docs/24, "What a
    // buoy does to a walker").
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
        assert!(closest > 3.0 && eye > 3.0, "buoy {buoy}: the hero came within {closest}, its eye {eye}");
    }
}

/// Mission 01 with both neutral warbots captured and a radar takt later on the hero's
/// radar: the play, and the targets of `tut1_mf1`, `helic` and the hostile `tut1_e1`.
fn mission_01_wingmen() -> (parkan_world::play::Play, [usize; 3]) {
    mission_01_captured(false)
}

/// [`mission_01_wingmen`], a captured bot given Standby when `standby`.
fn mission_01_captured(standby: bool) -> (parkan_world::play::Play, [usize; 3]) {
    let (mut play, m) = mission_01_play();
    play.capture_standby = standby;
    let tick = 1000.0 / 60.0;
    let target_of = |path: &str| {
        let object = m.objects.iter().position(|o| o.path.to_ascii_lowercase().ends_with(path)).unwrap();
        play.battle.objects.iter().position(|&o| o == object).unwrap()
    };
    let bots = [target_of("tut1_mf1.dat"), target_of("helic.dat"), target_of("tut1_e1.dat")];
    for bot in [bots[0], bots[1]] {
        stand_facing(&mut play, bot, 12.0, 3.5);
        play.tick(tick, [0.0; 2]);
        while play.targets.current != Some(bot) {
            play.targets.select_next();
        }
        assert!(play.enter());
    }
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    (play, bots)
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
    assert!([mf1, helic].iter().all(|&b| !matches!(task(&play, b), Task::Reload)));

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
fn mission_01_marks_dummies_magenta_the_enemy_red_neutral_bots_grey_and_its_own_light_blue() {
    use parkan_world::play::{MARK_HOSTILE, MARK_NEUTRAL, MARK_NEUTRAL_CLAN, MARK_OWN};

    // docs/25, "How the game colours what it marks".
    let (play, [mf1, helic, e1]) = mission_01_captured(false);
    let (fresh, m) = mission_01_play();
    let colour = |play: &parkan_world::play::Play, t: usize| play.mark_colour(play.units[t].clan);
    let dummies: Vec<usize> = (0..fresh.units.len())
        .filter(|&t| m.objects[fresh.battle.objects[t]].path.to_ascii_lowercase().contains("targ.dat"))
        .collect();
    assert_eq!(dummies.len(), 5);
    assert!(dummies.iter().all(|&t| colour(&fresh, t) == MARK_NEUTRAL), "the Trgt clan's word 1: magenta");
    assert_eq!(colour(&fresh, e1), MARK_HOSTILE);
    assert_eq!((colour(&fresh, mf1), colour(&fresh, helic)), (MARK_NEUTRAL_CLAN, MARK_NEUTRAL_CLAN));
    assert_eq!((colour(&play, mf1), colour(&play, helic)), (MARK_OWN, MARK_OWN), "captured: the player's");
    assert_eq!(play.mark_colour(Some(play.player_clan)), [128, 128, 255]);
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
fn the_outcome_panels_fonts_are_the_640_by_480_menu_and_game_fonts_of_font_lib() {
    use parkan_world::text::GameFont;
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    // `ui/menu_resources.cfg`: MENU_FONT_640x480 = 8, mf_640.tft; GAME_FONT_640x480 = 6.
    let menu = GameFont::ui(&game, "MENU_FONT").unwrap();
    let text = GameFont::ui(&game, "GAME_FONT").unwrap();
    assert_eq!((menu.width, text.width), (256, 128));
    // The recording of Mission 01's win shows the title 13 pixels a line on 640 x 480.
    assert!((menu.line_height - 13.0).abs() < 0.5, "{}", menu.line_height);
    assert!((text.line_height - 8.0).abs() < 0.5, "{}", text.line_height);
    // Its glyphs of "MISSION COMPLETE !" start their advances apart: 125 in all, the
    // recording's ink running 128 from the first to the end of the "!".
    assert_eq!(menu.advance("MISSION COMPLETE !"), 125.0);
    assert!(menu.advance("MISSION COMPLETE !") > text.advance("MISSION COMPLETE !"));
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_cockpit_names_its_units_lists_its_guns_and_frames_its_target_as_read() {
    use glam::{Mat4, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::play::Play;
    use parkan_world::progress::Sender;
    use parkan_world::text::GameFont;

    // docs/35-hud.md: the pages by the textures resource, the two skin files' pieces.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_01_play();
    let pages = Pages::open(&game).unwrap();
    let page = |name: &str| pages.pages[pages.index(name).unwrap()].name.to_ascii_lowercase();
    assert_eq!(
        (page("ui_menu"), page("ui_menu3"), page("page9")),
        ("ui_menu1.tex".into(), "ui_menu3.tex".into(), "ui_tex9.tex".into())
    );
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    for name in [
        "ccres_ray_body",
        "ccres_green_lamp",
        "ccres_frame_corner_3",
        "targeter_back",
        "back_shld",
        "bott_shld",
    ] {
        assert!(cockpit.skin.get(name).is_some(), "{name}");
    }
    assert_eq!(cockpit.skin.get("ccres_frame_edge_v").unwrap().turns, 1);
    assert!((cockpit.water_level + 1.725).abs() < 1e-3, "Tut_1's water: {}", cockpit.water_level);

    // "Name and status": each clan counts its own units in file order.
    let target_of = |play: &Play, suffix: &str| {
        (0..play.units.len())
            .filter(|&t| m.objects[play.battle.objects[t]].path.to_ascii_lowercase().ends_with(suffix))
            .collect::<Vec<_>>()
    };
    let name = |t: usize| cockpit.panels.names[t].clone();
    let (mf1, helic, e1) = (
        target_of(&play, "tut1_mf1.dat")[0],
        target_of(&play, "helic.dat")[0],
        target_of(&play, "tut1_e1.dat")[0],
    );
    assert_eq!(
        (name(mf1), name(helic), name(e1)),
        ("MFW-1 Warrior".into(), "TFW-2 Warrior".into(), "TSW-1 Warrior".into())
    );
    let mut dummies = target_of(&play, "targ.dat");
    dummies.sort_by_key(|&t| play.battle.objects[t]);
    assert_eq!(
        dummies.iter().map(|&t| name(t)).collect::<Vec<_>>(),
        (1..=5).map(|n| format!("SSW-{n} Warrior")).collect::<Vec<_>>()
    );
    assert_eq!(cockpit.panels.hero_name, "Human");
    // "Mission 01": the hero and the bots carry a battery and shields; the dummies neither.
    let d = |t: usize| play.units[t].designation;
    assert!(play.hero_designation.battery && play.hero_designation.shielded);
    assert!([mf1, helic, e1].iter().all(|&t| d(t).battery && d(t).shielded));
    assert!(dummies.iter().all(|&t| !d(t).battery && !d(t).shielded));
    assert_eq!((d(helic).chassis_type, d(mf1).chassis_type, d(dummies[0]).chassis_type), (1, 1, 2));

    // A frame with the helicopter targeted and a long message in the box.
    assert!(play.stand_facing(helic, 40.0, 0.0));
    for _ in 0..30 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    play.targets.set(Some(helic));
    let now = play.hero.time_ms;
    cockpit.messages.show(Sender::Information, "word ".repeat(80), now);
    let font = GameFont::ui(&game, "GAME_FONT").unwrap();
    let menu = GameFont::ui(&game, "MENU_FONT").unwrap();
    let eye = play.hero.eye();
    let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
        * Mat4::look_to_rh(eye.position, eye.forward, Vec3::Z);
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    let texts: Vec<&str> = drawn.text.iter().map(|r| r.text.as_str()).collect();
    for want in [
        "AUTOCANNON 25mm",
        "PLASMA RIFLE LS",
        "BATTLE LASER ER",
        "AWB MISSILE",
        " 500",
        "INF",
        "   4",
        "300",
        "TFW-2 Warrior",
        "Human",
        "from: Information assistant",
        "Press F1 to see more",
    ] {
        assert!(texts.contains(&want), "{want:?} in {texts:?}");
    }
    assert_eq!(texts.iter().filter(|t| t.starts_with("word")).count(), 6, "six lines at most");
    assert!(texts.iter().any(|t| t.ends_with(" m") && t.len() <= 5), "the distance: {texts:?}");
    assert_eq!(drawn.views.len(), 2);
    assert_eq!((drawn.views[0].unit, drawn.views[1].unit), (Some(helic), None));
    assert_eq!(drawn.views[0].viewport, [9.0, 315.0, 128.0, 128.0]);
    assert_eq!(drawn.views[1].viewport, [503.0, 315.0, 128.0, 128.0]);
    // 20 s on the box is gone.
    play.hero.time_ms += 20_001.0;
    let later = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    assert!(!later.text.iter().any(|r| r.text.starts_with("from:")), "the box lives 20 s");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_helicopter_turns_its_rotors_and_the_warbots_their_dishes_as_read() {
    use glam::DQuat;
    use parkan_formats::pose::Pose;

    let (mut play, m) = mission_01_play();
    let robot = |play: &parkan_world::play::Play, name: &str| {
        play.robots
            .iter()
            .position(|(t, _)| m.objects[play.battle.objects[*t]].path.to_ascii_lowercase().ends_with(name))
            .unwrap_or_else(|| panic!("{name} is a robot"))
    };
    let (helic, mf1, e1) =
        (robot(&play, "helic.dat"), robot(&play, "tut1_mf1.dat"), robot(&play, "tut1_e1.dat"));
    let node = |mesh: &parkan_formats::mesh::Mesh, name: &str| {
        mesh.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("{name}"))
    };
    let r = &play.robots[helic].1;
    let (top, bottom) = (node(&r.chassis.mesh, "Tup_m1o1"), node(&r.chassis.mesh, "Tdn_m1o1"));
    let dish = [
        (helic, node(&r.turret.mesh, "TTrad_m1o1")),
        (mf1, node(&play.robots[mf1].1.turret.mesh, "TMrad_m1o1")),
        (e1, node(&play.robots[e1].1.turret.mesh, "TTrad_m1o1")),
    ];
    let quat = |p: Pose| DQuat::from_xyzw(p.rotation[1], p.rotation[2], p.rotation[3], p.rotation[0]);
    let chassis = |play: &parkan_world::play::Play, n| quat(play.robots[helic].1.chassis_pose(n));
    let turret = |play: &parkan_world::play::Play, (r, n): (usize, usize)| {
        let robot = &play.robots[r].1;
        quat(robot.turret_node(&robot.mount(), n))
    };
    // The short way about the node's own z from one pose to the next.
    let turn = |a: DQuat, b: DQuat| {
        let d = a.inverse() * b;
        let angle = 2.0 * d.z.atan2(d.w);
        (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
    };
    play.tick(1000.0 / 60.0, [0.0; 2]);
    let (mut rotors, mut dishes) =
        ([chassis(&play, top), chassis(&play, bottom)], dish.map(|d| turret(&play, d)));
    let (mut turned, mut dish_turned) = ([0.0; 2], [0.0; 3]);
    for _ in 0..120 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        for (k, n) in [top, bottom].into_iter().enumerate() {
            let now = chassis(&play, n);
            turned[k] += turn(rotors[k], now) / std::f64::consts::TAU;
            rotors[k] = now;
        }
        for (k, d) in dish.into_iter().enumerate() {
            let now = turret(&play, d);
            dish_turned[k] += turn(dishes[k], now) / std::f64::consts::TAU;
            dishes[k] = now;
        }
    }
    // Two seconds: the rotors 7.3 turns a second the opposite ways, the dishes 0.5.
    assert!(
        (turned[0].abs() - 14.6).abs() < 0.3 && (turned[1].abs() - 14.6).abs() < 0.3,
        "rotors {turned:?}"
    );
    assert!(turned[0].signum() != turned[1].signum(), "opposite ways: {turned:?}");
    for t in dish_turned {
        assert!((t.abs() - 1.0).abs() < 0.05, "dishes {dish_turned:?}");
    }
    // The M-2f hovers: its wings hold flat, frame 2.
    let wing = node(&play.robots[mf1].1.chassis.mesh, "LTwng_m1o1");
    assert_eq!(play.robots[mf1].1.device_frame(wing), Some(2.0));
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_briefing_flies_its_waypoints_in_the_recordings_time_under_its_title_and_subtitles() {
    use parkan_world::briefing::Briefing;
    use parkan_world::hud::Space;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let mut b = Briefing::open(&game, &dir).unwrap().expect("Mission 01 has a briefing");
    assert_eq!((b.title.as_str(), b.subtitles, b.flythrough.stops.len()), ("Line of Fire", true, 26));
    // A frame every 1/60 s: when each subtitle first shows, against the recording's times less
    // its first frame at 1.20 s (docs/21, "Against the recording").
    let (mut t, mut changes, mut voices) = (0.0, Vec::new(), Vec::new());
    let mut last = String::from("-");
    while !b.finished() && t < 200.0 {
        voices.extend(b.frame(t).into_iter().map(|s| s.member));
        if b.subtitle() != last {
            last = b.subtitle().to_owned();
            changes.push((t, last.clone()));
        }
        if (t - 8.3_f64).abs() < 0.5 / 60.0 {
            // Waypoint 1 holds waypoint 0's camera, looking at its target.
            let eye = b.eye();
            assert!((eye.position - glam::Vec3::new(767.965, 164.519, 118.899)).length() < 1e-3);
            assert!((eye.fov_x - 1.04).abs() < 1e-6 && eye.up.z > 0.0);
        }
        t += 1.0 / 60.0;
    }
    assert!((97.9..98.6).contains(&t), "the briefing ends at {t}");
    let at = |needle: &str| changes.iter().find(|(_, s)| s.starts_with(needle)).map(|c| c.0).unwrap();
    for (needle, recording) in [
        ("Tara, The Home Base", 1.20),
        ("Listen up", 4.93),
        ("The Tara range", 37.83),
        ("Battle Mission", 66.37),
    ] {
        let model = at(needle) + 1.20;
        assert!((model - recording).abs() < 0.35, "{needle}: {model} against {recording}");
    }
    assert_eq!(voices.first().map(String::as_str), Some("t01_t01.wav"));
    assert_eq!(voices.len(), 11, "{voices:?}");

    // The screen: the fade, and the bars at 75 and 405 across the whole of a wide screen.
    let font = parkan_world::text::GameFont::ui(&game, "GAME_FONT").unwrap();
    let mut b = Briefing::open(&game, &dir).unwrap().unwrap();
    b.frame(0.0);
    let screen = b.screen(Space::new(1280.0, 720.0), &font);
    assert_eq!(screen.fills.len(), 3);
    assert_eq!(screen.fills[0].colour[3], 1.0, "the briefing opens black");
    let (top, bottom) = (screen.fills[1], screen.fills[2]);
    assert_eq!((top.min[0], top.max[0], top.max[1]), (-1.0, 1.0, 1.0));
    assert!(
        (top.min[1] - (1.0 - 2.0 * 75.0 / 480.0)).abs() < 1e-5
            && (bottom.max[1] - (1.0 - 2.0 * 405.0 / 480.0)).abs() < 1e-5
    );
    assert_eq!(screen.title[0].text, "Line of Fire");
    assert!(!screen.lines.is_empty() && screen.lines[0].text.starts_with("Tara,"));
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_scripts_wait_while_its_briefing_plays_and_greet_the_hero_after() {
    let (mut play, _) = mission_01_play();
    play.paused = true;
    for _ in 0..600 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(play.says.is_empty(), "nothing said in the briefing: {:?}", play.says.len());
    play.paused = false;
    for _ in 0..600 {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert!(!play.says.is_empty(), "the greeting once it is over");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_objectives_screen_lists_its_three_objectives_and_its_map_takes_the_weapons_place() {
    use glam::{Mat4, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{MINIMAP, Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let (mut play, _) = mission_01_play();
    let mut pages = Pages::open(&game).unwrap();
    assert!(pages.add_minimap(&game, &dir).unwrap());
    let minimap = &pages.pages[pages.index(MINIMAP).unwrap()];
    assert_eq!((minimap.name.as_str(), minimap.width, minimap.height), ("tut1.tex", 256, 256));
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.1)
        * Mat4::look_to_rh(play.hero.eye().position, play.hero.eye().forward, Vec3::Z);
    let space = Space::new(640.0, 480.0);

    cockpit.objectives.open_at_start();
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    let lines: Vec<&str> = drawn.menu_text.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        lines,
        [
            "Primary objectives",
            "1. Destroy all the targets on the island : in progress",
            "2. Capture the neutral warbots : in progress",
            "3. Destroy the enemy warbot : in progress",
            "Press F12 to close"
        ]
    );
    assert!(drawn.text.is_empty() && drawn.views.is_empty(), "it alone is drawn");
    play.hero.time_ms += 7001.0;
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    assert!(drawn.menu_text.is_empty() && !drawn.text.is_empty(), "closed 7 s after its first draw");

    cockpit.map.toggle();
    let drawn = cockpit.draw(&play, space, &font, &menu, view_proj);
    assert!(!drawn.text.iter().any(|r| r.text == "AUTOCANNON 25mm"), "no weapon rows under the map");
    let page = pages.index(MINIMAP).unwrap() as u16;
    assert!(
        drawn.batches.iter().flat_map(|b| &b.vertices).any(|v| v.page == Some(page)),
        "the minimap is drawn"
    );
}

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

fn mission_02_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_02).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.02").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 02 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
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
    assert!(play.factories[f].start(true, free));
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
    let at = robot.walker.body.position;
    assert!((at.truncate() - glam::Vec2::new(393.75, 854.91)).length() < 1.0, "at the creation vertex: {at}");
    assert!(matches!(robot.behaviour.task(), parkan_sim::behaviour::Task::Leave { .. }));

    // The escape is routed out along the factory's own paths (docs/31, "The escape"): the
    // hall way from the creation vertex north through the front door, at world y 854.9,
    // 873.4 just inside the door, and 885.4 on the forecourt outside it.
    let bot = play.robots.len() - 1;
    let way = play.way_out(t, at, glam::Vec3::new(535.7, 727.9, 154.0)).expect("a way out");
    let ys: Vec<f32> = way.iter().map(|p| p.y).collect();
    assert!(way.len() == 5 && ys[0] < 855.0 && ys[3] > 885.0, "out through the front door: {way:?}");

    // Walking it: the front door (node 3) opens for the bot as it nears it, no shot fired,
    // and its shut faces hold the bot inside until it has (docs/24, "Walking into a
    // building"). The doorway stands at about y 876.
    const DOORWAY_Y: f32 = 876.0;
    let door = |play: &parkan_world::play::Play| {
        play.buildings.iter().find(|b| b.target == t).expect("the factory's doors").doors[0].phase
    };
    let mut opened_at = None;
    let mut out_at = None;
    for tick in 0..(12 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        let phase = door(&play);
        let robot = &play.robots[bot].1;
        let at = robot.walker.body.position;
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
    // The door is rate 0.4, and the building sees it open at 0.9 / rate, 2.25 s (docs/27,
    // "Capture"). The bot is through it and back on the landscape within four seconds of that.
    let opened = opened_at.expect("the front door opens for the bot");
    let out = out_at.expect("the bot walks out onto the landscape");
    assert!((2.2..2.4).contains(&opened), "the door opens in {opened} s");
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
fn the_large_factorys_portal_quads_are_not_drawn_so_its_doors_and_rooms_show() {
    use parkan_formats::mission;
    use parkan_world::assembly::Assembly;
    use parkan_world::models;
    use parkan_world::textures::TextureStore;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    // `fr_b_plant` wears 87 `DEFAULT` and 16 `PORTAL_*` triangles: the black doorway a step
    // in front of each door, and the openings between its rooms (docs/24, "The doorways are
    // black quads").
    let part = assembly
        .parts(mission::KIND_BUILDING, "UNITS\\BUILDS\\PLANT\\lplant01.dat")
        .into_iter()
        .next()
        .expect("the Large Factory's chassis");
    let loaded = assembly.mesh(&part.reference).expect("its mesh");
    let portals = models::portal_triangles(&loaded.mesh, &loaded.wear);
    assert_eq!(portals.iter().filter(|&&p| p).count(), 103, "its portal triangles");
    // None of them reaches the drawn model.
    let model = models::build_model(
        &mut assembly,
        &mut store,
        mission::KIND_BUILDING,
        "UNITS\\BUILDS\\PLANT\\lplant01.dat",
    )
    .unwrap()
    .expect("the Large Factory");
    assert!(!model.groups.is_empty());
    assert!(
        !model.groups.iter().any(|g| models::doorway(&g.look.material)),
        "a portal quad is drawn: {:?}",
        model.groups.iter().map(|g| g.look.material.clone()).collect::<Vec<_>>()
    );
    // A robot wears none at all, so nothing of it is dropped.
    let hero = assembly
        .parts(mission::KIND_UNIT, "UNITS\\UNITS\\BATTLE\\w_b_trk1.dat")
        .into_iter()
        .next()
        .expect("a battle unit's chassis");
    let hero = assembly.mesh(&hero.reference).expect("its mesh");
    assert!(models::portal_triangles(&hero.mesh, &hero.wear).is_empty());
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
fn the_factorys_door_sounds_stand_on_their_doors_not_on_their_nodes_origins() {
    use glam::Vec3;
    use parkan_world::fx::Owner;

    let (mut play, _) = mission_02_play();
    let b = play.buildings.iter().position(|b| b.doors.len() == 3).expect("fr_b_plant's three doors");
    let t = play.buildings[b].target;
    let part_index = play.buildings[b].part;
    // `door_open_01` on each door's node, ids 8000, 8002 and 8004 (docs/13, "A building's
    // load group"). Their nodes' origins stand 8 m and 30.8 m from the doors themselves.
    for (d, id) in [(0usize, 8000i32), (1, 8002), (2, 8004)] {
        let node = play.buildings[b].doors[d].nodes[0];
        let door = door_centre(&play, t, part_index, node);
        let origin = play.battle.combat.targets[t].parts[part_index].nodes[node].translation;
        let origin = Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);
        let at: Vec<Vec3> = play.fx.owned(Owner::Building(t, id)).map(|i| i.frame.origin).collect();
        assert_eq!(at.len(), 1, "one open sound on door {d}");
        assert!(at[0].distance(door) < 0.01, "door {d}'s sound stands on it: {at:?} against {door}");
        if d > 0 {
            assert!(
                (origin.distance(door) - 30.8).abs() < 0.5,
                "the side door's node origin is 30.8 m away: {origin} against {door}"
            );
        }
    }
}

/// The centre of the level-0 bounding sphere of `node` of a target's part, in the world.
fn door_centre(play: &parkan_world::play::Play, target: usize, part_index: usize, node: usize) -> glam::Vec3 {
    let part = &play.battle.combat.targets[target].parts[part_index];
    let slot = &part.mesh.slots[usize::from(part.mesh.nodes[node].slot_index[0])];
    let [cx, cy, cz, _] = slot.sphere;
    let c = part.nodes[node].apply([cx, cy, cz].map(|v| f64::from(v * part.scale)));
    glam::Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_large_factory_cuts_the_ground_from_under_it_and_lets_the_hero_through_its_doorway() {
    let (mut play, _) = mission_02_play();
    // Over the pod the landscape is gone: the ground there is the factory's pod floor, 12.4
    // under its base, not Tut_2's sand at 151.7 (docs/03, "Placing a building cuts the
    // landscape").
    let under = play.ground.below(391.28, 740.08, 145.0).expect("a floor under the pod");
    assert_eq!(under.solid.map(|s| s.0), Some(0), "{under:?}");
    assert!((under.point.z - 139.35).abs() < 0.5, "{under:?}");
    assert!(play.ground.cut(391.28, 740.08) && !play.ground.cut(300.0, 556.0));
    // Walked at the entrance, the hero goes through its black doorway into the hall.
    assert!(play.stand_at(395.8, 915.0, 3.117));
    play.hero.key("SCAN_W", true);
    for _ in 0..(6 * 60) {
        play.hero.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    let at = play.hero.walker.body.position;
    assert!(at.y < 860.0, "in the hall, past the door at 877: {at}");
    assert_eq!(play.hero.walker.ground.and_then(|h| h.solid).map(|s| s.0), Some(0));
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_large_factory_runs_its_load_group_smoke_screens_and_lamps_on_their_points() {
    use parkan_world::building_fx::On;
    use parkan_world::fx::Owner;

    let (play, _) = mission_02_play();
    let t = play.factories.first().expect("the Large Factory").target;
    let (b, _) = play.building_effects.iter().find(|(b, _)| b.target == t).expect("its load group");
    let named = |prefix: &str| b.effects.iter().filter(|e| e.name.starts_with(prefix)).count();
    assert_eq!(
        (named("smoke_fr_02"), named("f_pict_"), named("f_recharge_r"), named("door_")),
        (3, 22, 2, 6),
        "docs/13, \"A building's load group\""
    );
    let lamps = named("f_signlight") + named("f_smalllight") + named("f_blinklight");
    assert_eq!(lamps, 17);
    // The smokes hang on the chimneys' points 122–130, the screens on 0–21.
    let smoke: Vec<On> = b.effects.iter().filter(|e| e.name == "smoke_fr_02").map(|e| e.on).collect();
    assert_eq!(
        smoke,
        vec![On::Points([122, 123, 124]), On::Points([125, 126, 127]), On::Points([128, 129, 130])]
    );
    // Every one of them is running, placed on the building.
    let part = &play.battle.combat.targets[t].parts[b.part];
    let centre = play.battle.combat.targets[t].position;
    for e in &b.effects {
        let frame = b.frame(part, e.on).expect("its points resolve");
        assert!(frame.origin.distance(centre) < 150.0, "{} at {}", e.name, frame.origin);
        let running = play.fx.instances.iter().filter(|(o, _)| *o == Owner::Building(t, e.id)).count();
        assert_eq!(running, 1, "{} is started once", e.name);
    }
    // The chimneys stand above the building's roof.
    let smoke_at = b.frame(part, smoke[0]).unwrap().origin;
    assert!(smoke_at.z > centre.z + 40.0, "a chimney's smoke at {smoke_at}");
}

/// A chimney's puff leaves orange and is black a quarter of a second on, and its plume is the
/// size the recording shows (docs/07, "How a material reaches the device"). `fire_smoke`'s
/// track steps through 26 of `DUST.0`'s cells, the first five of them the orange flame row and
/// the rest the black smoke rows, and an effect sprite plays it from its own start; before, it
/// drew the first cell for ever, so the whole plume was orange.
#[test]
#[ignore = "needs the game install"]
fn a_chimneys_puff_leaves_orange_turns_black_and_its_plume_is_the_recordings_size() {
    use parkan_world::textures::TextureStore;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, _) = mission_02_play();
    let mut store = TextureStore::open(&game).unwrap();
    play.fx.resolve_looks(&mut store).unwrap();
    let t = play.factories.first().expect("the Large Factory").target;
    let (b, _) = play.building_effects.iter().find(|(b, _)| b.target == t).expect("its load group");
    let part = &play.battle.combat.targets[t].parts[b.part];
    let chimney = b
        .effects
        .iter()
        .find(|e| e.name == "smoke_fr_02")
        .and_then(|e| b.frame(part, e.on))
        .expect("a chimney's frame");
    for _ in 0..(10 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }

    // The cell a look draws, as the mean of its texture's pixels weighted by their alpha.
    let colour = |look: usize| {
        let l = play.fx.looks[look];
        let tex = &store.textures[l.texture.expect("fire_smoke's texture")];
        let (w, h) = (tex.width as f32, tex.height as f32);
        let [u0, v0, du, dv] = l.cell;
        let (x0, y0) = ((u0 * w) as u32, (v0 * h) as u32);
        let (x1, y1) = (((u0 + du) * w) as u32, ((v0 + dv) * h) as u32);
        let rgba = &tex.levels[0];
        let (mut sum, mut weight) = ([0.0f32; 3], 0.0f32);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * tex.width + x) * 4) as usize;
                let a = f32::from(rgba[i + 3]) / 255.0;
                for c in 0..3 {
                    sum[c] += f32::from(rgba[i + c]) * a;
                }
                weight += a;
            }
        }
        sum.map(|c| c / weight.max(1e-6))
    };

    let mut smoke: Vec<(usize, parkan_sim::effects::Sprite)> = play
        .sprites(play.hero.eye().position)
        .into_iter()
        .filter(|(_, s)| s.material.eq_ignore_ascii_case("fire_smoke"))
        .filter(|(_, s)| s.centre.distance(chimney.origin) < 60.0 || s.age_ms < 100.0)
        .collect();
    assert!(smoke.len() > 20, "a plume of {} puffs", smoke.len());
    smoke.sort_by(|a, b| a.1.age_ms.total_cmp(&b.1.age_ms));

    // The youngest puff is on the flame row, warm and bright; one 250 ms old is on a smoke row.
    let young = colour(smoke[0].0);
    assert!(young[0] > 180.0 && young[0] > young[2] * 1.5, "the flame is orange: {young:?}");
    let (old, age) = smoke
        .iter()
        .find(|(_, s)| s.age_ms > 250.0)
        .map(|&(look, ref s)| (colour(look), s.age_ms))
        .expect("a puff older than 250 ms");
    assert!(old.iter().all(|&c| c < 90.0), "black smoke {age:.0} ms on: {old:?}");

    // The plume: 50 m over the chimney, 10 to 30 m across, as lerp(+88, +100) and
    // lerp(+136, +148) give in metres.
    let over = |s: &parkan_sim::effects::Sprite| s.centre.z - chimney.origin.z;
    let top = smoke.iter().map(|(_, s)| over(s)).fold(f32::MIN, f32::max);
    let widths: Vec<f32> = smoke.iter().map(|(_, s)| s.width).collect();
    assert!((0.0..=50.5).contains(&top) && top > 45.0, "the plume stands {top:.1} m over it");
    assert!(
        widths.iter().all(|&w| (9.5..=30.5).contains(&w)),
        "10 to 30 m across: {:.1}..{:.1}",
        widths.iter().copied().fold(f32::MAX, f32::min),
        widths.iter().copied().fold(f32::MIN, f32::max)
    );
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_bridges_start_their_signal_lights() {
    use parkan_world::fx::Owner;

    let (play, m) = mission_01_play();
    let bridges: Vec<_> = play
        .building_effects
        .iter()
        .filter(|(b, _)| {
            m.objects[play.battle.objects[b.target]].path.to_ascii_lowercase().contains("bridge")
        })
        .collect();
    // Both `m_bridge.dat` placements run `fr_m_brige.ctl`'s load group: two green and two red
    // signal lights and a small red light (its sphere effects are construction's).
    assert_eq!(bridges.len(), 2, "Mission 01's bridges have load groups");
    for (b, _) in bridges {
        let names: Vec<&str> = b.effects.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            ["f_signlight_g", "f_signlight_g", "f_signlight_r", "f_signlight_r", "f_smalllight_r"]
        );
        for e in &b.effects {
            assert!(play.fx.instances.iter().any(|(o, _)| *o == Owner::Building(b.target, e.id)));
        }
    }
    // A pylon's lamp seen from beside it draws over the pylon's faces, not cut by them (the
    // beacon stand-in, docs/11, "A beacon light's glow"); from under the deck it is hidden.
    let mut play = play;
    let game = gamedir::find(None).unwrap();
    play.fx.resolve_looks(&mut TextureStore::open(&game).unwrap()).unwrap();
    for _ in 0..60 {
        play.tick(1000.0 / 60.0, [0.0, 0.0]);
    }
    let lamp = glam::Vec3::new(775.4, 590.4, 24.3);
    let near = |eye: glam::Vec3| {
        play.sprites(eye)
            .into_iter()
            .filter(|(_, s)| s.material.eq_ignore_ascii_case("lamp") && s.centre.distance(lamp) < 1.0)
            .map(|(_, s)| s.overlay)
            .collect::<Vec<_>>()
    };
    assert_eq!(near(glam::Vec3::new(766.0, 583.0, 21.0)), [true], "in view, over the scene");
    assert!(near(glam::Vec3::new(790.0, 600.0, 5.0)).is_empty(), "hidden under the bridge");
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_large_factorys_lit_batches_take_its_256_lightmap_page() {
    use parkan_formats::mission;
    use parkan_world::assembly::Assembly;
    use parkan_world::textures::TextureStore;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    let model = parkan_world::models::build_model(
        &mut assembly,
        &mut store,
        mission::KIND_BUILDING,
        "UNITS\\BUILDS\\PLANT\\lplant01.dat",
    )
    .unwrap()
    .expect("the Large Factory");
    let lit: Vec<_> = model.groups.iter().filter(|g| g.lightmap.is_some()).collect();
    assert!(!lit.is_empty(), "its lit batches");
    let pages: std::collections::BTreeSet<usize> = lit.iter().filter_map(|g| g.lightmap).collect();
    assert_eq!(pages.len(), 1, "one page for the whole building");
    let page = &store.textures[*pages.first().unwrap()];
    assert_eq!(
        (page.name.to_ascii_lowercase().as_str(), page.width, page.height),
        ("fr_b_plant_00.0", 256, 256)
    );
    // Stream 18's coordinates reach the lit batches' vertices, inside the page.
    let reached: Vec<[f32; 2]> = lit
        .iter()
        .flat_map(|g| model.indices[g.start as usize..(g.start + g.count) as usize].iter())
        .map(|&i| model.vertices[i as usize].lightmap)
        .collect();
    assert!(reached.iter().all(|uv| (0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1])));
    assert!(reached.iter().any(|uv| uv[0] > 0.0 || uv[1] > 0.0));
    // A unit's batches stay unlit.
    let hero = parkan_world::models::build_model(
        &mut assembly,
        &mut store,
        mission::KIND_UNIT,
        "UNITS\\UNITS\\HERO\\tut2_p.dat",
    )
    .unwrap()
    .expect("the hero");
    assert!(hero.groups.iter().all(|g| g.lightmap.is_none()));
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
    // F sinks to the ground; then the hero gets out beside the bot, facing it.
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
fn leaving_the_window_lets_shift_up_and_the_free_look_centres_on_foot_and_aboard() {
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, counts: [f32; 2], n: usize| {
        for _ in 0..n {
            play.update_input();
            play.tick(1000.0 / 60.0, counts);
        }
    };
    tick(&mut play, [0.0; 2], 30);
    // Shift + the mouse turns the camera's free look alone: the sight stays (docs/30).
    let sight = play.hero.sight().expect("the hero's sight").1;
    play.key("SCAN_LSHIFT", true);
    tick(&mut play, [40.0, 30.0], 20);
    tick(&mut play, [0.0; 2], 10);
    let look = play.hero.rig.look;
    assert!((look[0] - 0.5).abs() > 0.05 && (look[1] - 0.5).abs() > 0.05, "free look {look:?}");
    assert!(play.hero.sight().unwrap().1.dot(sight) > 0.9999, "the guns stay put");
    assert!(play.eye().forward.dot(sight) < 0.95, "the eye turns away from the sight");
    // The window left with Shift down: its key-up never arrives, and every key comes up
    // (`stdSetApplicationState`, docs/14): Shift's release rows centre the free look.
    play.release_keys();
    assert_eq!(&play.hero.rig.look[..2], &[0.5, 0.5]);
    tick(&mut play, [40.0, 0.0], 10);
    assert_eq!(&play.hero.rig.look[..2], &[0.5, 0.5], "the mouse no longer moves the camera");

    // Aboard, the same for the bot's own table.
    let project = Project {
        path: "UNITS\\bld_unit_-2147483647.dat".into(),
        name: "LFW-2 Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 4,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let hero_at = play.hero.walker.body.position;
    let t = play.spawn(&project, play.player_clan, hero_at + glam::Vec3::new(8.0, 0.0, 1.0), 0.0).unwrap();
    tick(&mut play, [0.0; 2], 30);
    assert!(play.board(t));
    play.key("SCAN_LSHIFT", true);
    play.key("SCAN_LMOUSE", true);
    tick(&mut play, [40.0, 30.0], 20);
    assert_ne!(&play.driven().rig.look[..2], &[0.5, 0.5]);
    assert!(play.driven().guns.iter().any(|g| g.state == parkan_sim::guns::CONTINUE_FIGHT));
    play.release_keys();
    assert_eq!(&play.driven().rig.look[..2], &[0.5, 0.5]);
    tick(&mut play, [0.0; 2], 1);
    assert!(play.driven().guns.iter().all(|g| g.state == parkan_sim::guns::STATE_OFF), "the guns stop");
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

fn mission_03_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 03 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// A building's ambience hums on: the load group starts `f_bunk_sfx` and its like in time
/// mode 2 over a one-second duration, and their loops run over the whole of *t*, so the wrap
/// at the end of each period must not stop and start them again (docs/11, "Type 2 is a
/// sound"). Mission 03 starts the hero among four bunkers, three computers, a store and a
/// generator, which restarted once a second.
#[test]
#[ignore = "needs the game install"]
fn mission_03s_building_ambiences_start_once_and_are_never_stopped() {
    use parkan_sim::effects::CueKind;

    let (mut play, _) = mission_03_play();
    let mut started: std::collections::BTreeMap<String, usize> = Default::default();
    let mut stopped = Vec::new();
    for _ in 0..(30 * 60) {
        play.tick(1000.0 / 60.0, [0.0; 2]);
        for c in play.cues.drain(..) {
            let sound = c.sound.to_ascii_lowercase();
            match c.kind {
                CueKind::Loop => *started.entry(sound).or_default() += 1,
                CueKind::Stop => stopped.push(sound),
                _ => {}
            }
        }
    }
    assert_eq!(
        (started.get("f_bunk.wav"), started.get("f_comp.wav"), started.get("f_gener.wav")),
        (Some(&4), Some(&3), Some(&1)),
        "one start each in 30 s: {started:?}"
    );
    assert!(stopped.is_empty(), "and none stopped: {stopped:?}");
}

/// The target of the mission object whose path ends in `name`.
fn object_target(play: &parkan_world::play::Play, m: &parkan_formats::mission::Mission, name: &str) -> usize {
    play.battle
        .objects
        .iter()
        .position(|&o| m.objects[o].path.to_ascii_lowercase().ends_with(name))
        .unwrap_or_else(|| panic!("no {name}"))
}

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

/// Play `seconds` at 60 ticks a second, calling `each` after every tick.
fn play_for(
    play: &mut parkan_world::play::Play,
    seconds: f32,
    mut each: impl FnMut(&mut parkan_world::play::Play),
) {
    for _ in 0..(seconds * 60.0) as usize {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        each(play);
    }
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
    let free = play.free_minds(player);
    assert!(play.factories[f].start(false, free));
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

/// Walk the hero along `route`, W held and turned toward each point in turn, for at most
/// `seconds`, W let go at the last point: the tick it reached the first point on the building's
/// floor (the first whose ground is a building's face), the tick it reached the last point, the
/// index of the point it was heading for, and the tick `done` first held.
fn walk_route(
    play: &mut parkan_world::play::Play,
    route: &[[f32; 2]],
    seconds: usize,
    done: impl Fn(&parkan_world::play::Play) -> bool,
) -> (Option<usize>, Option<usize>, usize, Option<usize>) {
    play.hero.key("SCAN_W", true);
    let (mut on_floor, mut arrived, mut finished, mut next) = (None, None, None, 0);
    for tick in 0..(60 * seconds) {
        let at = play.hero.walker.body.position;
        while next < route.len() && glam::Vec2::from_array(route[next]).distance(at.truncate()) < 1.2 {
            next += 1;
        }
        if on_floor.is_none() && play.hero.walker.ground.is_some_and(|g| g.solid.is_some()) {
            on_floor = Some(tick);
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
        if done(play) {
            finished = Some(tick + 1);
            break;
        }
    }
    (on_floor, arrived, next, finished)
}

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

fn mission_04_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 04 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// Enter, `CMD_ENTER_STATE`, from the view the player has.
fn press_enter(play: &mut parkan_world::play::Play) {
    let eye = play.eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: glam::Mat4::IDENTITY,
        shift: false,
    };
    play.command(parkan_formats::controls::CMD_ENTER_STATE, &view);
}

/// Mission 04 with its HQ targeted from 12 m and Enter pressed.
fn mission_04_aboard_the_hq() -> (parkan_world::play::Play, parkan_formats::mission::Mission, usize) {
    let (mut play, m) = mission_04_play();
    let hq = object_target(&play, &m, "tut4_hq.dat");
    stand_facing(&mut play, hq, 12.0, 0.0);
    play.tick(1000.0 / 60.0, [0.0; 2]);
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(hq) {
            break;
        }
        play.targets.select_next();
    }
    assert_eq!(play.targets.current, Some(hq), "the HQ is on the hero's list");
    press_enter(&mut play);
    (play, m, hq)
}

/// A frame of command mode's camera and a tick of play, `n` times.
fn command_frames(
    play: &mut parkan_world::play::Play,
    n: usize,
    mut each: impl FnMut(&parkan_world::play::Play),
) {
    for _ in 0..n {
        play.update_input();
        play.command_frame(play.hero.time_ms / 1000.0, parkan_world::command::Edges::default());
        play.tick(1000.0 / 60.0, [0.0; 2]);
        each(play);
    }
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
    let goal = at + glam::Vec3::new(-44.0, -54.0, 0.0);
    play.dispatch(Order { code: parkan_sim::hq::GO, parameter: 0, target: Target::Place(goal.to_array()) });
    let mut worst: f32 = 0.0;
    command_frames(&mut play, 600, |p| {
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
    assert_eq!((play.auto_driver, play.selected_units()), (0, vec![hq]));
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

/// A game command as its key runs it, from the unit's own view.
fn press_command(play: &mut parkan_world::play::Play, command: &str) {
    let eye = play.own_eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: glam::Mat4::IDENTITY,
        shift: false,
    };
    play.command(command, &view);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_zoom_narrows_the_eye_to_0_2_in_eleven_updates_and_halves_the_mouse() {
    use parkan_formats::controls::CMD_JAMES_ZOOM_MODE;

    // docs/30-turrets.md, "The zoom": 0.1 an update toward 0.2, the mouse at 0.5 while zoomed.
    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play| {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    };
    tick(&mut play);
    assert!((play.eye().fov_x - 1.3).abs() < 1e-6);
    assert!((play.hero.pilot.sensitivity - 1.0).abs() < 1e-6, "MOUSE_SENS=100");
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    let mut fields = Vec::new();
    for _ in 0..12 {
        tick(&mut play);
        fields.push(play.eye().fov_x);
    }
    assert!((fields[10] - 0.2).abs() < 1e-5 && (fields[9] - 0.3).abs() < 1e-5, "{fields:?}");
    assert!((fields[11] - fields[10]).abs() < 1e-6, "it holds at 0.2");
    assert!((play.hero.pilot.sensitivity - 0.5).abs() < 1e-6);
    // Mid-way a press is let be; at the end it turns back out.
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    for _ in 0..3 {
        tick(&mut play);
    }
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    for _ in 0..8 {
        tick(&mut play);
    }
    assert!((play.eye().fov_x - 1.3).abs() < 1e-5, "{}", play.eye().fov_x);
    tick(&mut play);
    assert!((play.hero.pilot.sensitivity - 1.0).abs() < 1e-6);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_outer_camera_stands_at_its_four_places_about_the_hero_and_goes_back_into_the_eye() {
    use parkan_formats::controls::{CMD_JAMES_OUTER_CAMERA, CMD_JAMES_ZOOM_MODE};
    use parkan_world::camera::PLACES;

    // docs/30-turrets.md, "The outer camera".
    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play, n: usize| {
        for _ in 0..n {
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    tick(&mut play, 1);
    let r = play.outer_bound(None);
    assert!((r - 1.473).abs() < 2e-3, "the hero's chassis box's half-diagonal: {r}");
    for (k, &(angle, back)) in PLACES.iter().enumerate() {
        press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
        tick(&mut play, 30);
        let own = play.own_eye();
        let eye = play.eye();
        let heading = own.forward.y.atan2(own.forward.x) + angle;
        let want = own.position
            - glam::Vec3::new(heading.cos(), heading.sin(), 0.0) * r * back
            - glam::Vec3::Z * r * parkan_world::camera::WALKER_DROP;
        assert!(eye.position.distance(want) < 1e-3, "place {k}: {} against {want}", eye.position);
        assert_eq!((eye.forward, eye.fov_x, eye.near), (own.forward, 1.3, 0.5));
        // On the hero's right for the first two, on its left for the others.
        let right = own.forward.cross(own.up);
        assert_eq!((eye.position - own.position).dot(right) > 0.0, k < 2, "place {k}");
    }
    // Z does nothing from the outer camera's view.
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    tick(&mut play, 12);
    assert!(!play.hero.zoom.on);
    press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
    tick(&mut play, 13);
    assert!(play.outer.on(), "the move back is 0.24 s");
    tick(&mut play, 2);
    assert!(!play.outer.on() && play.eye() == play.own_eye());

    // A change of mode turns it off at once.
    press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
    tick(&mut play, 30);
    assert!(play.outer_shows());
    play.modes.push(parkan_world::play::Mode::Factory(0));
    assert!(!play.outer_shows());
    tick(&mut play, 1);
    play.modes.pop();
    assert!(!play.outer.on());
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_own_panel_keeps_the_running_hero_steady_in_its_view() {
    use glam::{Mat4, Quat, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::text::GameFont;

    // docs/35-hud.md, "The unit in the middle": the view's camera stands off the object as it is
    // drawn, so the hero running holds still in it. Placed by each step's end, the camera jumps a
    // stride at a time.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, _) = mission_01_play();
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let font = GameFont::ui(&game, "GAME_FONT").unwrap();
    let menu = GameFont::ui(&game, "MENU_FONT").unwrap();
    play.hero.key("SCAN_W", true);
    let (mut drawn_step, mut stepped_step) = (0.0_f32, 0.0_f32);
    let (mut last_drawn, mut last_stepped): (Option<Vec3>, Option<Vec3>) = (None, None);
    for tick in 0..180 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if tick < 60 {
            continue;
        }
        let hero = &play.hero;
        let (position, yaw) = hero.walker.drawn(hero.time_ms);
        let eye = hero.eye();
        let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
            * Mat4::look_to_rh(eye.position, eye.forward, Vec3::Z);
        let frame = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
        let own = frame.views.iter().find(|v| v.unit.is_none()).expect("the own panel's view");
        // The camera against the hero as drawn, in the hero's frame.
        let offset = Quat::from_rotation_z(-yaw) * (own.eye - position);
        let body = &hero.walker.body;
        let stepped = Quat::from_rotation_z(-yaw) * (body.position - position);
        if let (Some(a), Some(b)) = (last_drawn, last_stepped) {
            drawn_step = drawn_step.max(offset.distance(a));
            stepped_step = stepped_step.max(stepped.distance(b));
        }
        (last_drawn, last_stepped) = (Some(offset), Some(stepped));
    }
    eprintln!("camera against the drawn hero {drawn_step} m a tick at most; the step's end {stepped_step} m");
    assert!(stepped_step > 0.2, "the step's end jumps {stepped_step} m in a tick");
    assert!(drawn_step < 0.05, "the camera moves {drawn_step} m against the drawn hero in a tick");
}

/// A campaign mission's play with its progression, the hero standing still.
fn campaign_play(path: &str) -> parkan_world::play::Play {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, path).unwrap();
    let name = path.rsplit('/').next().unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), name).unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    play
}

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
        .map(|(r, (t, robot))| (r, (*t, robot.clone())))
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
    // Where each follower stood a second ago, and the least it has gone in a second over the
    // deck between y 620 and 700.
    let mut second_ago: Vec<Vec3> = bots.iter().map(|&t| at(&play, t)).collect();
    let mut slowest = vec![f32::MAX; bots.len()];
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
        if tick % 60 == 59 {
            for (k, &t) in bots.iter().enumerate() {
                let p = at(&play, t);
                if (620.0..700.0).contains(&p.y) && (620.0..700.0).contains(&second_ago[k].y) && p.z > 55.0 {
                    slowest[k] = slowest[k].min(p.truncate().distance(second_ago[k].truncate()));
                }
                second_ago[k] = p;
            }
        }
    }
    assert!(
        slowest.iter().all(|&m| m > 5.0 && m < f32::MAX),
        "no stop on the deck: {slowest:?} m in a second"
    );
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
