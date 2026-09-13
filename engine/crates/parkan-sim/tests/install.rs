//! The hero on Tut_1, against the install: `cargo test -- --ignored`.

use glam::Vec3;
use parkan_formats::nres::Archive;
use parkan_formats::{control, gamedir, landmesh, mesh, mission};
use parkan_sim::ground::Ground;
use parkan_sim::machine::Walker;

fn hero() -> (Walker, Ground, mission::Object) {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let hero = m.objects.iter().find(|o| o.path.to_ascii_uppercase().contains("\\HERO\\")).unwrap().clone();
    let bases = Archive::open(&gamedir::resolve(&game, "bases.rlb").unwrap()).unwrap();
    let c = control::parse(bases.read_name("r_h_02.ctl").unwrap(), "r_h_02.ctl").unwrap();
    let body = mesh::parse(bases.read_name("r_h_02.msh").unwrap(), "r_h_02.msh").unwrap();
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    let walker = Walker::new(c, &body, Vec3::from_array(hero.position), hero.rotation);
    (walker, Ground::new(land), hero)
}

#[test]
#[ignore = "needs the game install"]
fn the_hero_stands_on_tut_1_facing_the_dummies() {
    let (mut w, ground, placed) = hero();
    w.advance(500.0, &ground);
    let f = w.body.forward();
    assert!((f.x - 0.998).abs() < 1e-3 && (f.y + 0.068).abs() < 1e-3, "{f}");
    assert!(
        (w.body.position.z - placed.position[2]).abs() < 0.1,
        "{} against {}",
        w.body.position.z,
        placed.position[2]
    );
    assert_eq!(w.body.velocity, [0.0; 3]);
    assert_eq!(w.machine.current, 0, "standing is state 0");
    assert_eq!(w.machine.step_ms, 50.0);
}

#[test]
#[ignore = "needs the game install"]
fn holding_w_runs_the_hero_at_fourteen_metres_a_second_in_run_steps() {
    let (mut w, ground, _) = hero();
    let start = w.body.position;
    w.body.command = [0.0, 1.0, 0.0];
    w.advance(2000.0, &ground);
    assert_eq!(w.body.velocity[1], 14.0);
    assert!((73..=96).contains(&w.machine.current), "a run cycle state, not {}", w.machine.current);
    // A run step strides 0.46-0.49 (docs/24): 33-35 ms at 14 m/s.
    assert!((32.0..36.0).contains(&w.machine.step_ms), "{}", w.machine.step_ms);
    let travelled = w.body.position - start;
    let along = travelled.dot(w.body.forward());
    // Short of 28: the first 0.3 s start up through transitions that move the body
    // only by their root stride.
    assert!(along > 22.0 && along < 28.0, "{along} m in 2 s");
    let p = w.body.position;
    let ground_z = ground.below(p.x, p.y, p.z).unwrap().point.z;
    assert!((p.z - w.base - ground_z).abs() < 1e-4, "the lowest point on the ground");

    w.body.command = [0.0; 3];
    w.advance(4000.0, &ground);
    assert_eq!(w.body.velocity, [0.0; 3]);
    assert_eq!(w.machine.current, 0, "stopped, standing again");
}
