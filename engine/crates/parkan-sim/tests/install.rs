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

#[test]
#[ignore = "needs the game install"]
fn a_running_hero_lands_three_steps_a_run_cycle() {
    let (mut w, ground, _) = hero();
    w.body.command = [0.0, 1.0, 0.0];
    let mut t = 0.0;
    while t < 1000.0 {
        t += 1000.0 / 60.0;
        w.advance(t, &ground);
    }
    w.landed.clear();
    let mut entered = Vec::new();
    let mut last = w.machine.step_start_ms;
    while t < 2000.0 {
        t += 1000.0 / 60.0;
        w.advance(t, &ground);
        if w.machine.step_start_ms != last {
            last = w.machine.step_start_ms;
            for c in w.landed.drain(..) {
                entered.push((w.machine.current, c));
            }
        }
    }
    // docs/13, "A footstep, end to end": the left foot lands entering 90 and 78, the right
    // entering 78, once a 0.405 s cycle.
    let per_second = entered.len() as f32;
    assert!((6.0..=9.0).contains(&per_second), "{per_second} steps in a second: {entered:?}");
    let states: std::collections::BTreeSet<usize> = entered.iter().map(|&(s, _)| s).collect();
    assert!(states.iter().all(|s| [78, 90].contains(s)), "{entered:?}");
}

/// Every mission's `sky.wea` names a material for the sun and for the moon, and
/// [`parkan_sim::sky::body_texture`] picks each body's own slot — 3 and 4, which the
/// engine and the file agree on without either deriving the index from the other
/// (`docs/10-sky.md`, "Where the sun stands").
#[test]
#[ignore = "needs the game install"]
fn every_missions_sky_names_a_sun_and_a_moon_the_body_slots_pick_out() {
    use parkan_sim::sky::{Body, body_texture};
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut seen = 0;
    let mut extents: Vec<f32> = Vec::new();
    for entry in walkdir(&game) {
        let Ok(bytes) = std::fs::read(&entry) else { continue };
        let Ok(mut a) = parkan_formats::sky::parse(&bytes, &entry.display().to_string()) else {
            continue;
        };
        let wea = entry.with_file_name("sky.wea");
        let Ok(w) = std::fs::read(&wea) else { continue };
        a.textures = parkan_formats::wea::parse(&w).materials;
        seen += 1;
        let sun = body_texture(&a, Body::Sun).unwrap_or_default().to_ascii_uppercase();
        let moon = body_texture(&a, Body::Moon).unwrap_or_default().to_ascii_uppercase();
        assert!(sun.starts_with("ENV_SUN"), "{}: sun slot 3 is {sun:?}", entry.display());
        assert!(
            moon.starts_with("ENV_MOON") || moon.starts_with("ENV_SUN"),
            "{}: moon slot 4 is {moon:?}",
            entry.display()
        );
        for k in &a.keyframes {
            let (across, up) = (k.intensity[0], k.intensity[1]);
            assert!(across >= up, "{}: a body is never taller than it is wide", entry.display());
            extents.extend([across, up]);
        }
    }
    assert_eq!(seen, 29, "the shipped skies");
    assert_eq!(extents.len(), 656 * 2, "every keyframe's two extents");
    // The extents are a factor, not a length: none is anywhere near a world size.
    let (lo, hi) = extents.iter().fold((f32::MAX, 0.0_f32), |(l, h), &v| (l.min(v), h.max(v)));
    assert!((lo - 0.4).abs() < 1e-6, "the smallest extent is 0.4, not {lo}");
    assert!((hi - 3.3).abs() < 1e-6, "the largest is 3.3, not {hi}");
}

/// Every mission's `sky.wea` names a nebula and a cloud sheet in slots 0 and 2, and a
/// lens-flare sprite in each of 5 and 6, which is what the sky's other layers draw with
/// (`docs/10-sky.md`, "The three layers and their texture coordinates").
///
/// Slot 1, the stars, is named on all 29 too -- and nothing draws it.
#[test]
#[ignore = "needs the game install"]
fn every_missions_sky_names_a_nebula_a_cloud_sheet_and_two_flare_sprites() {
    use parkan_sim::sky::role_texture;
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut seen = 0;
    for entry in walkdir(&game) {
        let Ok(bytes) = std::fs::read(&entry) else { continue };
        let Ok(mut a) = parkan_formats::sky::parse(&bytes, &entry.display().to_string()) else {
            continue;
        };
        let wea = entry.with_file_name("sky.wea");
        let Ok(w) = std::fs::read(&wea) else { continue };
        a.textures = parkan_formats::wea::parse(&w).materials;
        seen += 1;
        let at = |role| role_texture(&a, role).unwrap_or_default().to_ascii_uppercase();
        let (file, nebula, clouds) = (entry.display(), at("nebula"), at("clouds"));
        assert!(nebula.starts_with("ENV_NEBULA"), "{file}: slot 0 is {nebula:?}");
        // 28 of the 29 name `ENV_CLOUDS`; CAMPAIGN.01/Mission.01 names `TOK51`, whose
        // texture is 16 pixels square and wholly transparent, so it has no clouds at all.
        assert!(clouds.starts_with("ENV_CLOUDS") || clouds == "TOK51", "{file}: slot 2 is {clouds:?}");
        assert_eq!(at("stars"), "ENV_STARS", "{file}: slot 1, which nothing draws");
        assert_eq!(at("flare"), "ENV_FLARE_00", "{file}: slot 5");
        assert_eq!(at("flare2"), "ENV_FLARE_01", "{file}: slot 6");
    }
    assert_eq!(seen, 29, "the shipped skies");
}

/// The dome's colours carry a day's worth of alpha, which is what shows the nebula under
/// them: clear overhead at night, solid by day, and solid at the rim throughout.
#[test]
#[ignore = "needs the game install"]
fn the_domes_apex_goes_clear_at_night_while_its_rim_stays_solid() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut clear_apex, mut apexes, mut horizons, mut solid_horizons) = (0, 0, 0, 0);
    let mut scene_solid = 0;
    for entry in walkdir(&game) {
        let Ok(bytes) = std::fs::read(&entry) else { continue };
        let Ok(a) = parkan_formats::sky::parse(&bytes, &entry.display().to_string()) else { continue };
        for k in &a.keyframes {
            apexes += 1;
            clear_apex += usize::from(k.colour(parkan_formats::sky::APEX_SLOT)[3] == 0);
            scene_solid += usize::from(k.colour(parkan_formats::sky::SCENE_COLOUR_SLOT)[3] == 255);
            for slot in parkan_formats::sky::HORIZON_SLOTS {
                horizons += 1;
                solid_horizons += usize::from(k.colour(slot)[3] == 255);
            }
        }
    }
    assert_eq!(apexes, 656, "every shipped keyframe");
    assert_eq!(clear_apex, 355, "the apex is wholly clear on 355 of them");
    assert_eq!((horizons, solid_horizons), (2624, 2594), "the horizon is solid on all but 30");
    // The control: a slot that carries no alpha of its own is 255 on every one.
    assert_eq!(scene_solid, 656, "slot 20, the scene colour, is opaque throughout");
}

fn walkdir(game: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![game.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("sky.ske")) {
                out.push(p);
            }
        }
    }
    out
}
