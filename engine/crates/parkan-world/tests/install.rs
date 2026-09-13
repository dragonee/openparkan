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
