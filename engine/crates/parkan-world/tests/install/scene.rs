//! What the install's files build: the ground, its textures, the objects on it, their
//! lightmaps and the effects they run.

use crate::common::*;
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

/// C01 Mission 04 places two Medium Track warbots. A tracked chassis's belt is not a pose: its
/// four skid-steering devices drive channels that span no frames and carry `CHANNEL_MATERIAL`,
/// and the value plays the belt material the same nodes wear (docs/28, "The belt is a material
/// a channel plays"). So the belt runs with the track under it: it holds while the bot stands,
/// runs at the channel's rate at top speed, and the two sides run opposite ways in a turn.
#[test]
#[ignore = "needs the game install"]
fn a_tracked_warbots_belt_is_played_by_its_track_so_it_holds_while_the_bot_stands() {
    use parkan_formats::control::{CHANNEL_MATERIAL, TRIPLE_TOP_SPEED};
    use parkan_world::textures::TextureStore;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut play = campaign_play(gamedir::C01_MISSION_04);
    let r = play
        .robots
        .iter()
        .position(|(_, robot)| {
            robot.walker.controller.channels.iter().any(|c| c.flags & CHANNEL_MATERIAL != 0)
        })
        .expect("C01 Mission 04 places a Medium Track warbot");

    // The belt nodes are exactly the nodes that wear a played material: four of the chassis's
    // seven, the four the skid-steering devices name.
    let (chassis, controller) = {
        let robot = &play.robots[r].1;
        (robot.chassis.clone(), robot.walker.controller.clone())
    };
    let belts: Vec<usize> = controller
        .channels
        .iter()
        .filter(|c| c.flags & CHANNEL_MATERIAL != 0)
        .map(|c| c.node as usize)
        .collect();
    assert_eq!(belts.len(), 4, "two belts, each on two nodes: {belts:?}");
    let mut store = TextureStore::open(&game).unwrap();
    let played: Vec<usize> = (0..chassis.mesh.nodes.len())
        .filter(|&n| {
            let slot = chassis.mesh.nodes[n].slot_index[0];
            let Some(slot) = chassis.mesh.slots.get(usize::from(slot)) else { return false };
            let batches =
                usize::from(slot.first_batch)..usize::from(slot.first_batch) + usize::from(slot.batch_count);
            batches.filter_map(|b| chassis.mesh.batches.get(b)).any(|b| {
                let name = chassis.wear.materials.get(usize::from(b.material)).cloned().unwrap_or_default();
                store.look(&name).unwrap().animation.is_some()
            })
        })
        .collect();
    let mut named = belts.clone();
    named.sort_unstable();
    assert_eq!(named, played, "the channels name the nodes that wear a played material");

    // Standing still -- no velocity, no spin -- the belt holds where it stopped.
    let (_, robot) = &mut play.robots[r];
    let (part, node) = (robot.chassis_part, belts[0]);
    let tick = |robot: &mut parkan_world::robot::Robot, ms: f64| {
        robot.time_ms += ms;
        robot.turn_devices(|_, _| true);
    };
    robot.walker.body.velocity = [0.0; 3];
    robot.walker.body.spin = [0.0; 3];
    for _ in 0..60 {
        tick(robot, 1000.0 / 60.0);
    }
    let held = robot.material_phase(part, node).expect("the belt is played by a value");
    for _ in 0..60 {
        tick(robot, 1000.0 / 60.0);
    }
    assert_eq!(robot.material_phase(part, node), Some(held), "the belt holds while the bot stands");

    // At top speed it runs at its channel's rate, 51.4 loops a second on the medium chassis.
    let rate = controller.channels[0].rate;
    robot.walker.body.velocity = [0.0, controller.triples[TRIPLE_TOP_SPEED][1], 0.0];
    let mut ran = 0.0_f32;
    let mut last = held;
    for _ in 0..1000 {
        tick(robot, 1.0);
        let now = robot.material_phase(part, node).unwrap();
        ran += wrapped(last, now);
        last = now;
    }
    assert!((ran - rate).abs() < rate * 0.05, "{ran} loops a second against the channel's {rate}");

    // Turning on the spot the two sides run opposite ways: skid steering.
    robot.walker.body.velocity = [0.0; 3];
    robot.walker.body.spin = [0.0, 0.0, 0.5];
    let before: Vec<f32> = belts.iter().map(|&n| robot.material_phase(part, n).unwrap()).collect();
    for _ in 0..100 {
        tick(robot, 1.0);
    }
    let sides: Vec<f32> = belts
        .iter()
        .zip(&before)
        .map(|(&n, &was)| wrapped(was, robot.material_phase(part, n).unwrap()))
        .collect();
    assert!(sides.iter().any(|&s| s > 0.0) && sides.iter().any(|&s| s < 0.0), "skid steering: {sides:?}");
}

/// How far a wrapping value went from `a` to `b`, the short way round.
fn wrapped(a: f32, b: f32) -> f32 {
    let d = b - a;
    d - d.round()
}

#[test]
#[ignore = "needs the game install"]
fn the_large_factorys_portal_quads_fade_by_their_word_and_the_open_ones_are_never_drawn() {
    use parkan_formats::mission;
    use parkan_world::assembly::Assembly;
    use parkan_world::models::{self, Portal};
    use parkan_world::textures::TextureStore;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let mut store = TextureStore::open(&game).unwrap();
    // `fr_b_plant` wears 87 `DEFAULT` and 16 `PORTAL_*` triangles: the black doorway a step
    // in front of each door, and the openings between its rooms (docs/24, "The doorways are
    // portal quads").
    let part = assembly
        .parts(mission::KIND_BUILDING, "UNITS\\BUILDS\\PLANT\\lplant01.dat")
        .into_iter()
        .next()
        .expect("the Large Factory's chassis");
    let loaded = assembly.mesh(&part.reference).expect("its mesh");
    let portals = models::portal_triangles(&loaded.mesh, &loaded.wear);
    assert_eq!(portals.iter().filter(|&&p| p).count(), 103, "its portal triangles");
    let model = models::build_model(
        &mut assembly,
        &mut store,
        mission::KIND_BUILDING,
        "UNITS\\BUILDS\\PLANT\\lplant01.dat",
    )
    .unwrap()
    .expect("the Large Factory");
    assert!(!model.groups.is_empty());
    // Its level 0 draws the doorways as gates and the green signs as signs, each by its batch
    // word (docs/24, "A building is drawn cell by cell through its portals"); the open ones,
    // the entrance on `o01` among them, are not drawn, and no portal material is drawn as
    // anything but a portal.
    let kinds: Vec<(String, Portal)> = model
        .groups
        .iter()
        .filter_map(|g| g.portal.map(|p| (g.look.material.to_ascii_uppercase(), p.kind)))
        .collect();
    assert!(kinds.iter().all(|(m, k)| (m == "DEFAULT") == (*k == Portal::Gate)), "{kinds:?}");
    let count = |kind| kinds.iter().filter(|(_, k)| *k == kind).count();
    assert_eq!((count(Portal::Gate), count(Portal::Sign)), (32, 4), "{kinds:?}");
    assert!(
        !model.groups.iter().any(|g| g.portal.is_none() && models::doorway(&g.look.material)),
        "a portal material drawn as no portal"
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

/// Every mission directory the install ships: the single and multiplayer ones and each
/// campaign's.
fn every_mission(game: &std::path::Path) -> Vec<std::path::PathBuf> {
    let root = gamedir::resolve(game, "MISSIONS").unwrap();
    let mut dirs: Vec<std::path::PathBuf> =
        std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).collect();
    let campaigns = gamedir::resolve(game, "MISSIONS/CAMPAIGN").unwrap();
    for c in std::fs::read_dir(&campaigns).unwrap().flatten() {
        dirs.extend(std::fs::read_dir(c.path()).into_iter().flatten().flatten().map(|e| e.path()));
    }
    dirs.retain(|d| gamedir::resolve(d, "data.tma").is_some());
    dirs.sort();
    dirs
}

#[test]
#[ignore = "needs the game install"]
fn every_placed_buildings_basement_tiles_the_ring_between_its_contours() {
    use parkan_formats::{landmesh, mission};
    use parkan_world::assembly::Assembly;
    use parkan_world::basement;

    // docs/03, "The pieces are triangles of a constrained Delaunay triangulation": the band is
    // the ring between the outer contour, cut where it crosses the landscape's edges, and the
    // inner ring, and nothing else.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let area = |ring: &[[f32; 2]]| {
        (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1])
            })
            .sum::<f64>()
            .abs()
            / 2.0
    };
    let (mut buildings, mut tiled, mut on_mean) = (0, 0, 0);
    let mut tut_2 = Vec::new();
    for dir in every_mission(&game) {
        let raw = std::fs::read(gamedir::resolve(&dir, "data.tma").unwrap()).unwrap();
        let Ok(m) = mission::parse(&raw, &dir.display().to_string()) else { continue };
        let rings = basement::rings(&mut assembly, &m);
        if rings.is_empty() {
            continue;
        }
        let map = terrain::map_dir(&game, &m.map_path).unwrap();
        let land = landmesh::load(&gamedir::resolve(&map, "Land.msh").unwrap()).unwrap();
        for (inner, outer) in &rings {
            buildings += 1;
            let f = basement::footing(&land, inner, outer);
            let across: f64 = f
                .faces
                .iter()
                .map(|x| {
                    let [a, b, c] = x.triangle();
                    f64::from((b - a).truncate().perp_dot((c - a).truncate())) / 2.0
                })
                .sum();
            let ring: Vec<[f32; 2]> = inner.iter().map(|p| [p[0], p[1]]).collect();
            let want = area(&f.outline) - area(&ring);
            if (across - want).abs() < want * 1e-4 && f.faces.iter().all(|x| x.normal().z > 0.0) {
                tiled += 1;
            }
            // The building's base, its inner ring's first corner, against the mean height of
            // its cut contour (docs/03, "A building is set down on the mean of its contour").
            let edge = basement::contour(&land, outer, inner[0][2]);
            let mean = edge.iter().map(|p| p[2]).sum::<f32>() / edge.len() as f32;
            if (inner[0][2] - mean).abs() < 0.01 {
                on_mean += 1;
            }
            if dir.ends_with("Mission.02") && dir.parent().is_some_and(|p| p.ends_with("CAMPAIGN.00")) {
                tut_2.push((edge.len(), inner[0][2], mean));
            }
        }
    }
    assert_eq!((buildings, tiled), (167, 167), "every band tiles its ring");
    // 151 of the 167 placed buildings stand on the mean of their cut contour to a centimetre;
    // the other 16 are 8 of the 19 bridges and 8 buildings of four campaign missions. The
    // mean of the ring's corners alone does not: Tut_2's generator stands 1.87 off it.
    assert_eq!(on_mean, 151, "{tut_2:?}");
    // Tut_2's Large Factory, Outpost and Generator: 29, 24 and 40 corners on the cut contour,
    // each standing on its mean.
    assert_eq!(tut_2.iter().map(|t| t.0).collect::<Vec<_>>(), vec![29, 24, 40], "{tut_2:?}");
    assert!(tut_2.iter().all(|&(_, base, mean)| (base - mean).abs() < 0.01), "{tut_2:?}");
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
    // Walked at the entrance, the hero goes through its black doorway into the hall: the door
    // starts opening once the hero's sphere reaches the leaf's capsule, 7.7 wide across the
    // doorway, and is open 2.25 s on (docs/24, "Walking into a building").
    assert!(play.stand_at(395.8, 915.0, 3.117));
    play.hero.key("SCAN_W", true);
    for _ in 0..(7 * 60) {
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

    // The plume: 50 ± 15 m over the chimney, each puff's far end jittered by ±half of +120's
    // 30 as it leaves, and 10 to 30 m across, as lerp(+88, +100) and lerp(+136, +148) give in
    // metres.
    let over = |s: &parkan_sim::effects::Sprite| s.centre.z - chimney.origin.z;
    let top = smoke.iter().map(|(_, s)| over(s)).fold(f32::MIN, f32::max);
    let widths: Vec<f32> = smoke.iter().map(|(_, s)| s.width).collect();
    assert!((0.0..=65.5).contains(&top) && top > 45.0, "the plume stands {top:.1} m over it");
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
    // A pylon's lamp seen from beside it draws depth-tested, so the pylon's own faces cut it
    // (docs/11, "A beacon light's glow": a beacon has no bit-8 emitter, and the pass argument
    // its flag 0x800 waits for decides whether it draws at all, never its depth state); from
    // under the deck its tested point is hidden and it draws nothing.
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
    assert_eq!(near(glam::Vec3::new(766.0, 583.0, 21.0)), [false], "in view, depth-tested");
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
fn a_start_flagged_building_keeps_its_mission_height_and_the_rest_stand_on_their_contours_mean() {
    use parkan_formats::{landmesh, mission};
    use parkan_world::assembly::Assembly;
    use parkan_world::basement;

    // docs/04, "The start flag keeps a building at its file height": the insertion sets a
    // building down on the mean of its cut contour only while `IBuilding` slot 13 answers 1
    // (`Terrain.dll:0x100147cc`), and the start flag makes it 2.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let mut tally = std::collections::BTreeMap::<(bool, bool), usize>::new();
    let mut flagged_off = Vec::new();
    for dir in every_mission(&game) {
        let raw = std::fs::read(gamedir::resolve(&dir, "data.tma").unwrap()).unwrap();
        let Ok(m) = mission::parse(&raw, &dir.display().to_string()) else { continue };
        let rings = basement::rings(&mut assembly, &m);
        if rings.is_empty() {
            continue;
        }
        let map = terrain::map_dir(&game, &m.map_path).unwrap();
        let land = landmesh::load(&gamedir::resolve(&map, "Land.msh").unwrap()).unwrap();
        let buildings: Vec<_> = m.objects.iter().filter(|o| o.kind == mission::KIND_BUILDING).collect();
        assert_eq!(buildings.len(), rings.len());
        for (o, (inner, outer)) in buildings.iter().zip(&rings) {
            let edge = basement::contour(&land, outer, inner[0][2]);
            let mean = edge.iter().map(|p| p[2]).sum::<f32>() / edge.len() as f32;
            let off = inner[0][2] - mean;
            let flagged = o.tail.0 != 0;
            *tally.entry((flagged, off.abs() < 0.01)).or_default() += 1;
            if flagged && off.abs() >= 0.01 {
                flagged_off.push(off.abs());
            }
        }
    }
    // 150 of the 154 unflagged buildings stand on their mean to a centimetre, and the other
    // four within 0.14; 12 of the 13 flagged stand off it, 0.2 to 5.2, and the one that does
    // not, Tut_1's bridge half, stands 0.004 off by its authoring.
    assert_eq!(
        tally,
        [((false, false), 4), ((false, true), 150), ((true, false), 12), ((true, true), 1)].into(),
    );
    assert!(flagged_off.iter().all(|&d| (0.2..5.2).contains(&d)), "{flagged_off:?}");
}

/// A building's insignia is its owner's: Part 6.5 of the let's play shows C03 M02's Enemy 1
/// Medium Mine wearing cell 6 of `PG27`, a filled triangle over a bar, and the player's Small
/// Bunker cell 0, the arrow. `B_LBL_01`'s eight tracks name cells 0, 6, 5, 4, 3, 2, 1 and 7, so
/// the mine draws on track 1, its clan's index (docs/07, "Who picks an object mesh's material
/// track").
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_buildings_wear_their_owners_insignia() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut store = TextureStore::open(&game).unwrap();
    let cell =
        |store: &mut TextureStore, track: usize| store.look_on_track("B_LBL_01", track).unwrap().still.cell;
    let grid = |c: usize| [(c % 4) as f32 * 0.25, (c / 4) as f32 * 0.25, 0.25, 0.25];
    assert_eq!(cell(&mut store, 0), grid(0), "the arrow");
    assert_eq!(cell(&mut store, 1), grid(6), "the triangle over a bar");
    assert_eq!(store.look_on_track("B_LBL_01", 0).unwrap(), store.look("B_LBL_01").unwrap());

    let play = campaign_play(gamedir::C03_MISSION_02);
    let unit = |id: u32| play.units.iter().position(|u| u.logical_id == id as i32).expect("a placed object");
    assert_eq!(play.insignia(unit(0x8000_0006)), 1, "Enemy 1's Medium Mine");
    assert_eq!(play.insignia(unit(0x8000_0001)), 0, "the player's Small Bunker");
    assert_eq!(play.insignia(unit(15)), 0, "a unit keeps track 0");
}

/// An explosion's point light falls on the landscape as the shade's light template, `LIGHT1`
/// out of `system.rlb`'s `shade.wea` (docs/11, "What a light does to a surface"): a winged SSM
/// bursting on C03 M02's ground lights the faces about it, each piece inside its light's range,
/// at a quarter of its colour's length, held at 2, as its alpha. The buildings' lamps, flagged
/// `0x20000000`, light nothing.
#[test]
#[ignore = "needs the game install"]
fn c03_m02s_blast_lights_the_ground_and_the_lamps_do_not() {
    use parkan_sim::effects::LIGHT_NOT_EMULATED;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut store = TextureStore::open(&game).unwrap();
    let mut play = campaign_play(gamedir::C03_MISSION_02);
    assert_eq!(play.fx.light_template.as_deref(), Some("LIGHT1"));
    play_for(&mut play, 0.5, |_| {});
    play.fx.resolve_looks(&mut store).unwrap();
    let eye = play.eye();
    let view = glam::Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
        * glam::Mat4::look_to_rh(eye.position, eye.forward, eye.up);
    let lamps = play.fx.lights(play.hero.time_ms);
    assert!(lamps.iter().all(|l| l.flags & LIGHT_NOT_EMULATED != 0), "only the buildings' lamps so far");
    assert!(play.light_discs(eye.position, view).is_empty(), "and they light nothing");

    let kind = play.battle.combat.kinds.iter().position(|k| k.name.eq_ignore_ascii_case("bm_m_04")).unwrap();
    let at = play.hero.walker.body.position + glam::Vec3::new(40.0, 0.0, 0.0);
    play.battle.combat.fire(
        kind,
        None,
        at + glam::Vec3::new(0.0, 0.0, 30.0),
        -glam::Vec3::Z,
        glam::Vec3::ZERO,
        1.0,
        None,
    );
    play_for(&mut play, 1.0, |_| {});
    play.fx.resolve_looks(&mut store).unwrap();
    let lights: Vec<_> = play.fx.lights(play.hero.time_ms).into_iter().filter(|l| l.flags == 0).collect();
    assert!(!lights.is_empty(), "the burst drives a point light the shade draws");
    let discs = play.light_discs(eye.position, view);
    assert!(!discs.is_empty(), "which falls on the ground about it");
    let reach = lights.iter().map(|l| (l.position, l.range)).collect::<Vec<_>>();
    for d in &discs {
        assert_eq!(Some(d.look), play.fx.light_look);
        assert!(d.alpha > 0.0 && d.alpha <= 0.5, "a quarter of the colour's length, held at 2: {}", d.alpha);
        assert!(d.polygon.iter().all(|&(p, uv)| {
            (0.0..=0.99).contains(&uv[0])
                && (0.0..=0.99).contains(&uv[1])
                && reach.iter().any(|&(c, r)| p.distance(c) <= r * 1.5 + 1.0)
        }));
    }
}
