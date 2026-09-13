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
    assert!(t.groups.iter().all(|g| g.layer1.texture.is_some()), "every layer-1 material resolves");
    let water: Vec<_> = t.groups.iter().filter(|g| g.water).collect();
    assert!(!water.is_empty() && water.iter().all(|g| g.layer1.material == "WATER"));
    let blue = water[0].layer1.diffuse;
    assert!(blue[2] > blue[0], "water is tinted blue by its material: {blue:?}");
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
    w.from = (w.body.position, w.body.yaw);
    w.from_heading = w.body.yaw;
    w.follow_ground(&play.ground);
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
    assert_eq!(play.killed, vec![object]);
}
