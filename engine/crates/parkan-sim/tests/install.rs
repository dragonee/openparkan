//! The hero on Tut_1, against the install: `cargo test -- --ignored`.

use glam::Vec3;
use parkan_formats::control::CONTACT_SUPPORT;
use parkan_formats::nres::Archive;
use parkan_formats::{control, cpt, gamedir, landmesh, mesh, mission};
use parkan_sim::ground::Ground;
use parkan_sim::machine::{Frames, Walker};

fn hero() -> (Walker, Ground, mission::Object) {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let hero = m.objects.iter().find(|o| o.path.to_ascii_uppercase().contains("\\HERO\\")).unwrap().clone();
    let bases = Archive::open(&gamedir::resolve(&game, "bases.rlb").unwrap()).unwrap();
    let c = control::parse(bases.read_name("r_h_02.ctl").unwrap(), "r_h_02.ctl").unwrap();
    let body = mesh::parse(bases.read_name("r_h_02.msh").unwrap(), "r_h_02.msh").unwrap();
    let points = cpt::parse(bases.read_name("r_h_02.cpt").unwrap(), "r_h_02.cpt").unwrap();
    let land = landmesh::load(&gamedir::resolve(&game, "DATA/MAPS/Tut_1/Land.msh").unwrap()).unwrap();
    let walker = Walker::new(c, &body, &points, Vec3::from_array(hero.position), hero.rotation);
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

/// How far the lowest supporting contact stands above the ground under it, the mesh
/// posed as the ground contact poses it.
fn lowest_contact(w: &Walker, ground: &Ground) -> f32 {
    let state = &w.controller.states[w.machine.current];
    let last = Frames { a: state.pair_a[1], b: state.pair_b[1], weight: w.machine.q };
    let feet = w.feet.as_ref().expect("the hero's feet are its control points");
    state
        .contacts
        .iter()
        .filter(|c| c.flags & CONTACT_SUPPORT != 0)
        .filter_map(|c| feet.place(c.point, last))
        .map(|at| {
            let p = w.body.position + w.body.to_world(at);
            p.z - ground.search(p, w.radius).expect("ground under a foot").point.z
        })
        .fold(f32::MAX, f32::min)
}

#[test]
#[ignore = "needs the game install"]
fn holding_w_runs_the_hero_at_fourteen_metres_a_second_in_run_steps() {
    let (mut w, ground, _) = hero();
    let start = w.body.position;
    w.body.command = [0.0, 1.0, 0.0];
    let (mut t, mut last, mut steps, mut landed) = (0.0, -1.0, 0, 0);
    while t < 2000.0 {
        t += 1000.0 / 60.0;
        w.advance(t, &ground);
        if w.machine.step_start_ms == last {
            continue;
        }
        last = w.machine.step_start_ms;
        steps += 1;
        // A fall is taken only while it stays above the lift, and a lift puts the
        // lowest foot on the ground (docs/24, "Holding the body on the ground").
        let gap = lowest_contact(&w, &ground);
        assert!(gap > -1e-3, "a foot {gap} under the ground at {t} ms");
        if w.body.fall_speed == 0.0 {
            assert!(gap < 1e-3, "landed, yet the lowest foot is {gap} up at {t} ms");
            landed += 1;
        }
    }
    assert!(landed > steps / 3, "{landed} of {steps} steps end on a foot");
    assert_eq!(w.body.velocity[1], 14.0);
    assert!((73..=96).contains(&w.machine.current), "a run cycle state, not {}", w.machine.current);
    // A run step strides 0.46-0.49 (docs/24): 33-35 ms at 14 m/s.
    assert!((32.0..36.0).contains(&w.machine.step_ms), "{}", w.machine.step_ms);
    let travelled = w.body.position - start;
    let along = travelled.dot(w.body.forward());
    // Short of 28: the first 0.3 s start up through transitions that move the body
    // only by their root stride.
    assert!(along > 22.0 && along < 28.0, "{along} m in 2 s");

    w.body.command = [0.0; 3];
    w.advance(4000.0, &ground);
    assert_eq!(w.body.velocity, [0.0; 3]);
    assert_eq!(w.machine.current, 0, "stopped, standing again");
    assert!(lowest_contact(&w, &ground).abs() < 1e-3, "standing on a foot");
}
