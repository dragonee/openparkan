//! The behaviour scripts, against the install: `cargo test -- --ignored`.

use std::path::{Path, PathBuf};

use parkan_formats::{gamedir, scr};
use parkan_sim::script::{Args, Host, Interpreter};

fn scripts_dir() -> PathBuf {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    gamedir::resolve(&game, "MISSIONS/SCRIPTS").expect("MISSIONS/SCRIPTS")
}

fn load(dir: &Path, stem: &str) -> Interpreter {
    let table = scr::parse_variables(&std::fs::read(dir.join(scr::VARSET)).unwrap(), scr::VARSET).unwrap();
    let script = scr::parse(&std::fs::read(dir.join(format!("{stem}.scr"))).unwrap(), stem).unwrap();
    let formulas =
        scr::parse_formulas(&std::fs::read(dir.join(format!("{stem}.fml"))).unwrap(), stem).unwrap();
    Interpreter::new(script, &formulas, &table).unwrap()
}

/// Answers every function with 0.
struct Silent;

impl Host for Silent {
    fn call(&mut self, _function: i32, _args: &mut Args<'_>) -> u32 {
        0
    }
}

#[test]
#[ignore = "needs the game install"]
fn every_shipped_script_loads_with_its_formulas_and_every_handler_runs() {
    let dir = scripts_dir();
    let mut stems: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".scr").map(str::to_owned)
        })
        .collect();
    stems.sort();
    assert_eq!(stems.len(), 58);
    for stem in &stems {
        let mut i = load(&dir, stem);
        for h in 0..i.script.handlers.len() {
            i.run(h, &mut Silent);
        }
    }
}

/// Mission 01's world, scripted run by run: where the hero (logical id 1) stands and
/// how many robots each clan has.
struct Mission01 {
    hero_route: Option<u32>,
    robots: [u32; 4],
    class_robot: u32,
    calls: Vec<(u32, u32)>,
}

impl Host for Mission01 {
    fn call(&mut self, function: i32, args: &mut Args<'_>) -> u32 {
        match function {
            // Function 19 writes the base's centre and the clan's number.
            19 => {
                args.set_dword(0, 549);
                args.set_dword(1, 630);
                args.set_dword(2, 0);
                0
            }
            30 => {
                self.calls.push((args.dword(0), args.dword(1)));
                0
            }
            31 => {
                assert_eq!(args.dword(1), self.class_robot, "the count asks for robots");
                self.robots[args.dword(0) as usize]
            }
            32 => u32::from(args.dword(1) == 1 && self.hero_route == Some(args.dword(0))),
            other => panic!("tut1_pl2 calls function {other}"),
        }
    }
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_player_script_plays_its_messages_and_objectives_as_docs_34_reads_them() {
    let dir = scripts_dir();
    let mut i = load(&dir, "tut1_pl2");
    let info = i.dword("MESSAGE_INFO").unwrap();
    let objective = i.dword("OBJECTIVE_COMPLETE").unwrap();
    let mut world = Mission01 {
        hero_route: Some(0),
        robots: [1, 5, 1, 2],
        class_robot: i.dword("CLASS_ROBOT").unwrap(),
        calls: Vec::new(),
    };
    assert!(i.run_named("Init", &mut world));
    assert_eq!(i.dword("ClanBaseX"), Some(549));
    let mission = i.handler("Mission").unwrap();
    let mut run = |world: &mut Mission01, change: &dyn Fn(&mut Mission01)| {
        change(world);
        world.calls.clear();
        i.run(mission, world);
        world.calls.clone()
    };

    // The hero starts in route 0: the welcome and the movement help, once.
    assert_eq!(run(&mut world, &|_| {}), [(info, 11), (info, 14)]);
    assert_eq!(run(&mut world, &|_| {}), []);
    // A route plays only after a run in no route.
    assert_eq!(run(&mut world, &|w| w.hero_route = Some(2)), []);
    assert_eq!(run(&mut world, &|w| w.hero_route = None), []);
    assert_eq!(run(&mut world, &|w| w.hero_route = Some(2)), [(info, 16)]);
    // The first target down: the guided-missile help, once.
    assert_eq!(run(&mut world, &|w| w.robots[1] = 4), [(info, 17)]);
    assert_eq!(run(&mut world, &|_| {}), []);
    // The last target down: objective 0, then message 12.
    assert_eq!(run(&mut world, &|w| w.robots[1] = 0), [(objective, 0), (info, 12)]);
    assert_eq!(run(&mut world, &|_| {}), []);
    // Both neutrals gone and one of them the player's: objective 1, then 13 and 19.
    assert_eq!(run(&mut world, &|w| w.robots[3] = 0), []);
    assert_eq!(run(&mut world, &|w| w.robots[0] = 2), [(objective, 1), (info, 13), (info, 19)]);
    // The enemy down: objective 2.
    assert_eq!(run(&mut world, &|w| w.robots[2] = 0), [(objective, 2)]);
    assert_eq!(run(&mut world, &|_| {}), []);
}
