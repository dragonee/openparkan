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
        assert!(play.enter(), "Enter captures {name}");
        assert_eq!(play.units[bot].clan, Some(play.player_clan));
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
    assert_eq!(panel.rows.iter().map(|r| r.0.as_str()).collect::<Vec<_>>()[..2], ["Standby", "Follow me"]);
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

    let (mut play, [mf1, helic, _]) = mission_01_captured(false);
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    assert!(
        tasks(&play, &[mf1, helic]).iter().any(|t| matches!(t, Task::Attack { .. })),
        "no order: it engages"
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
fn mission_02s_factory_builds_a_free_warbot_in_a_minute_which_escapes_and_completes_the_objective() {
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
