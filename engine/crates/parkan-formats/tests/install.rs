//! Against the game's own files. They need an install, so they are ignored by
//! default: `cargo test -- --ignored`, with `PARKAN_DIR` set if the install is
//! not beside this repository.

use parkan_formats::{gamedir, mission, nres};

fn game() -> std::path::PathBuf {
    gamedir::find(None).expect("a Parkan install: set PARKAN_DIR")
}

#[test]
#[ignore = "needs the game install"]
fn mission_01_places_the_hero_at_its_start() {
    let dir = gamedir::resolve(&game(), gamedir::MISSION_01).expect("Mission 01");
    let data = std::fs::read(dir.join("data.tma")).unwrap();
    let m = mission::parse(&data, "Mission.01").unwrap();
    assert_eq!((m.objects.len(), m.clans.len()), (33, 4));
    assert_eq!(m.map_name(), "Tut_1");
    let hero =
        m.objects.iter().find(|o| o.path.to_ascii_uppercase().ends_with("\\HERO\\TUT1_P.DAT")).unwrap();
    let [x, y, z] = hero.position;
    assert!((x - 433.0).abs() < 0.1 && (y - 477.1).abs() < 0.1 && (z - 14.1).abs() < 0.1);
    assert!((hero.rotation + 1.639).abs() < 1e-3);
}

#[test]
#[ignore = "needs the game install"]
fn every_archive_in_the_install_opens() {
    let mut opened = 0;
    let mut stack = vec![game()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if std::fs::read(&path).is_ok_and(|d| d.starts_with(nres::MAGIC)) {
                nres::Archive::open(&path).unwrap_or_else(|e| panic!("{e}"));
                opened += 1;
            }
        }
    }
    assert_eq!(opened, 120);
}
