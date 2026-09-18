//! The robot constructor's designs against the install and the recording of Mission 02
//! (docs/38-designs.md).

use parkan_formats::{gamedir, mission, objects};
use parkan_world::assembly::Assembly;
use parkan_world::designs::{self, Catalogue, Designer, Node, Tab};

/// Mission 02's player clan's research tree, as its clan record names it.
fn mission_02_tree(game: &std::path::Path) -> String {
    let dir = gamedir::resolve(game, gamedir::MISSION_02).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.02").unwrap();
    m.clans.iter().find(|c| c.name == "Plr").expect("the Plr clan").behaviour.clone()
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_player_is_offered_the_32_parts_of_its_tree() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let path = mission_02_tree(&game);
    assert!(path.to_ascii_lowercase().ends_with("tut2_pl.trf"), "{path}");
    let catalogue = Catalogue::open(&game, &path).unwrap();
    let offered: Vec<&String> = catalogue.tree.part_ids.iter().filter(|p| catalogue.offered(p)).collect();
    assert_eq!(offered.len(), 32, "{offered:?}");
    assert_eq!(catalogue.page(&designs::chassis_prefixes(4)), vec!["R_B_02"]);
    assert_eq!(catalogue.page(&["e_tur_bb".to_owned()]), vec!["e_tur_bb_01"]);
    assert_eq!(catalogue.page(&designs::gun_prefixes("universal_bl")), vec!["e_gun_bl_15"]);
    assert_eq!(catalogue.page(&designs::gun_prefixes("central_bc")), vec!["e_gun_bc_06"]);
}

/// A catalogue that offers every part of `data.trf`, as `FULL_RESEARCH_TREE` would.
fn everything(game: &std::path::Path) -> Catalogue {
    let path = gamedir::resolve(game, "MISSIONS/SCRIPTS/data.trf").unwrap();
    Catalogue::new(parkan_formats::research::parse(&std::fs::read(path).unwrap(), "data.trf").unwrap(), true)
}

/// A socket's kind is its stream-10 label, not its node name (docs/30, "Gun sockets"). The
/// Large builder keeps its module socket on `Base_LU_02` and a cannon socket on `Base_LU_01`,
/// the other way round from the small and medium builders; the Large transport has one socket
/// and it takes a cannon.
#[test]
#[ignore = "needs the game install"]
fn the_large_builders_module_socket_is_base_lu_02_and_the_large_transport_has_one() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let mut designer = Designer::new(&game, everything(&game), 4);
    /// Every gun socket a turret on `chassis` offers: its node, its label and its page.
    fn sockets(
        designer: &mut Designer,
        assembly: &mut Assembly,
        chassis: &str,
        turret: &str,
    ) -> Vec<(i32, String, Vec<String>)> {
        let mut design = designer.chassis(assembly, chassis);
        assert!(designer.fit_turret(assembly, &mut design, turret));
        designer
            .places(assembly, Some(&design), Tab::Weapons)
            .into_iter()
            .map(|p| {
                let offers = designer.offers(&p);
                (p.attach, p.label.clone(), offers)
            })
            .collect()
    }
    let builder = sockets(&mut designer, &mut assembly, "r_b_01", "e_tur_bt_08");
    assert_eq!(
        builder.iter().map(|(n, l, _)| (*n, l.as_str())).collect::<Vec<_>>(),
        vec![(6, "universal_bs"), (7, "central_bc")]
    );
    assert!(builder[0].2.iter().all(|p| p.to_ascii_lowercase().starts_with("e_gun_bs")));
    assert!(builder[1].2.iter().all(|p| p.to_ascii_lowercase().starts_with("e_gun_bc")));
    assert!(!builder[0].2.is_empty() && !builder[1].2.is_empty());
    // The small and medium builders put the module on their first socket instead.
    for (turret, chassis, node) in [("e_tur_lt_03", "r_l_03", 6), ("e_tur_mt_03", "r_m_01", 5)] {
        let places = sockets(&mut designer, &mut assembly, chassis, turret);
        let module = places.iter().find(|(_, l, _)| l.ends_with('s')).expect("a module socket");
        assert_eq!(module.0, node, "{turret}");
    }
    // One socket on the Large transport, and it is a cannon's.
    let transport = sockets(&mut designer, &mut assembly, "r_b_01", "e_tur_bt_07");
    assert_eq!(
        transport.iter().map(|(n, l, _)| (*n, l.as_str())).collect::<Vec<_>>(),
        vec![(9, "central_bc")]
    );
}

/// The recording's designer, step by step, and each unit box it shows (docs/38, "The
/// recording, re-derived"; `openparkan/verify.py`'s `DESIGNER_RECORDING`).
const RECORDING: [(&str, [&str; 5], bool); 17] = [
    ("chassis R_B_02", ["26 / 39 t", "66 kph", "12 %", "0 %", "0 m"], false),
    ("turret e_tur_bb_01", ["37 / 28 t", "58 kph", "23 %", "0 %", "350 m"], false),
    ("gun bl e_gun_bl_15", ["41 / 24 t", "55 kph", "23 %", "5 %", "350 m"], false),
    ("gun bl e_gun_bl_15", ["44 / 21 t", "53 kph", "23 %", "9 %", "350 m"], false),
    ("gun bc e_gun_bc_06", ["51 / 14 t", "48 kph", "23 %", "17 %", "350 m"], false),
    ("gun bc e_gun_bc_06", ["58 / 7 t", "43 kph", "23 %", "25 %", "350 m"], false),
    ("i_arm_b_02", ["62 / 3 t", "40 kph", "27 %", "25 %", "350 m"], false),
    ("i_eng_b_01 i_pws_b_01", ["64 / 1 t", "45 kph", "27 %", "25 %", "350 m"], false),
    ("i_fsh_b_01", ["64 / 1 t", "45 kph", "27 %", "25 %", "350 m"], false),
    ("i_dsh_b_01", ["65 / 0 t", "44 kph", "27 %", "25 %", "350 m"], false),
    ("i_rps_b_01", ["66 / 0 t", "44 kph", "27 %", "25 %", "350 m"], true),
    ("i_rdr_b_01", ["66 / 0 t", "44 kph", "27 %", "25 %", "400 m"], true),
    ("i_def_b_01", ["66 / 0 t", "44 kph", "28 %", "25 %", "400 m"], true),
    ("-gun", ["59 / 6 t", "48 kph", "28 %", "17 %", "400 m"], false),
    ("-gun", ["52 / 13 t", "54 kph", "28 %", "9 %", "400 m"], false),
    ("-gun", ["49 / 16 t", "57 kph", "28 %", "5 %", "400 m"], false),
    ("-gun", ["45 / 20 t", "60 kph", "28 %", "0 %", "400 m"], false),
];

#[test]
#[ignore = "needs the game install"]
fn mission_02s_designer_rates_every_unit_box_the_recording_shows_and_prices_the_accepted_one() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let catalogue = Catalogue::open(&game, &mission_02_tree(&game)).unwrap();
    let mut designer = Designer::new(&game, catalogue, 4);
    let mut design: Option<Node> = None;
    let mut names = Vec::new();
    let mut accepted = None;
    for (step, lines, red) in RECORDING {
        let words: Vec<&str> = step.split_whitespace().collect();
        match words[0] {
            "chassis" => design = Some(designer.chassis(&mut assembly, words[1])),
            "turret" => assert!(designer.fit_turret(&mut assembly, design.as_mut().unwrap(), words[1])),
            "gun" => {
                let d = design.as_mut().unwrap();
                let turret = d.turret().unwrap().clone();
                let used: Vec<i32> = turret
                    .children
                    .iter()
                    .filter(|c| c.class == designs::CLASS_GUN)
                    .map(|c| c.attach)
                    .collect();
                let socket = designer
                    .labels(&mut assembly, &turret.part)
                    .iter()
                    .enumerate()
                    .position(|(i, l)| l.ends_with(words[1]) && !used.contains(&(i as i32)))
                    .unwrap() as i32;
                assert!(designer.fit_gun(&mut assembly, d, words[2], socket));
            }
            "-gun" => {
                let d = design.as_mut().unwrap();
                let turret = d.children.iter_mut().find(|c| c.class == designs::CLASS_TURRET).unwrap();
                let last = turret.children.iter().rposition(|c| c.class == designs::CLASS_GUN).unwrap();
                turret.children.remove(last);
            }
            _ => {
                fn replace(node: &mut Node, part: &str) {
                    if node.part.get(..7) == part.get(..7) && node.class != designs::CLASS_CLIP {
                        node.part = part.to_owned();
                    }
                    for c in &mut node.children {
                        replace(c, part);
                    }
                }
                for part in &words {
                    replace(design.as_mut().unwrap(), part);
                }
            }
        }
        let d = design.as_ref().unwrap();
        let rating = designer.rate(&mut assembly, d).expect("a rating");
        let got = rating.lines(designer.offence_range, designer.defence_range);
        assert_eq!(got.to_vec(), lines.to_vec(), "after {step}");
        assert_eq!(rating.full(), red, "after {step}");
        names.push(designer.name(&mut assembly, d, 0, &strings));
        if lines == RECORDING[13].1 && accepted.is_none() {
            accepted = Some(designer.price(d));
            assert!(designer.acceptable(&mut assembly, d));
        }
    }
    assert_eq!(names[0], "LF?-X Unknown");
    assert_eq!(names[1], "LFW-X Warrior");
    assert_eq!(accepted, Some((411.0, 226.5, true)));
    // Built, the player's first bot is numbered after its hero.
    let d = design.as_ref().unwrap();
    assert_eq!(designer.name(&mut assembly, d, 2, &strings), "LFW-2 Warrior");
    // The destination panel's rows and what the source offers for them.
    let places = designer.places(&mut assembly, Some(d), Tab::Weapons);
    assert_eq!(places.len(), 4, "{places:?}");
    assert_eq!(designer.offers(&places[0]).len(), 1);
    let armour = designer.places(&mut assembly, Some(d), Tab::Armour);
    assert_eq!(armour.len(), 1);
    // In the tree's `TRFB` order.
    assert_eq!(designer.offers(&armour[0]), vec!["i_arm_b_01", "i_arm_b_02", "i_arm_b_df"]);
}

#[test]
#[ignore = "needs the game install"]
fn the_installs_factory_designs_rewrite_byte_for_byte_and_load_from_memory() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let designer = Designer::new(&game, everything(&game), 4);
    let units = gamedir::resolve(&game, "UNITS").unwrap();
    let mut written: Vec<std::path::PathBuf> = std::fs::read_dir(&units)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy().to_ascii_lowercase();
            name.ends_with(".dat")
                && (name.starts_with("bld_unit_")
                    || name.starts_with("view_unit_")
                    || name == "temp_unit.dat")
        })
        .collect();
    written.sort();
    assert_eq!(written.len(), 33);
    for path in &written {
        let bytes = std::fs::read(path).unwrap();
        let unit = objects::parse_unit(&bytes, "dat").unwrap();
        let tree = Node::from_unit(&unit).unwrap();
        assert_eq!(designer.dat_bytes(&tree, unit.kind), bytes, "{}", path.display());
    }
    // A written design held in memory loads as the shipped file does.
    let shipped =
        written.iter().find(|p| p.file_name().unwrap().to_string_lossy().starts_with("bld_unit_")).unwrap();
    let name = format!("UNITS\\{}", shipped.file_name().unwrap().to_string_lossy());
    let bytes = std::fs::read(shipped).unwrap();
    let unit = objects::parse_unit(&bytes, "dat").unwrap();
    let again = designer.dat_bytes(&Node::from_unit(&unit).unwrap(), unit.kind);
    let (from_file, records) = (assembly.parts(mission::KIND_UNIT, &name), assembly.records(&name));
    assembly.register_unit("UNITS/in_memory.dat", again);
    assert!(!from_file.is_empty());
    assert_eq!(assembly.parts(mission::KIND_UNIT, "units\\IN_MEMORY.dat"), from_file);
    assert_eq!(assembly.records("UNITS/in_memory.dat"), records);
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_part_boxes_read_as_the_recording_shows() {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut assembly = Assembly::new(&game).unwrap();
    let catalogue = Catalogue::open(&game, &mission_02_tree(&game)).unwrap();
    let mut designer = Designer::new(&game, catalogue, 4);
    let mut values = |part: &str| -> Vec<String> {
        designer
            .part_box(&mut assembly, part)
            .into_iter()
            .map(|l| format!("{} {} {}", l.label, l.value, l.unit))
            .collect()
    };
    // docs/38, "A part's box": the figures the recording's designer shows.
    assert_eq!(values("R_B_02"), ["Weight 10.0 t", "Max payload 55.0 t", "Max speed 110.0 kmph"]);
    assert_eq!(values("e_tur_bb_01")[..2], ["Weight 5.0 t", "Cannon hanger 2 "]);
    assert_eq!(
        values("e_gun_bl_15"),
        ["Weight 2.8 t", "Wattage 0.2 MWt", "Rate of fire 1.3 1/s", "Damage 225.0 HP", "Range 250.0 m"]
    );
    let flame = values("e_gun_bc_06");
    assert_eq!(
        [&flame[0], &flame[2], &flame[3], &flame[4]],
        ["Weight 3.3 t", "Rate of fire 0.7 1/s", "Damage 790.0 HP", "Range 200.0 m"]
    );
    assert_eq!(values("i_eng_b_df")[..2], ["Weight 1.6 t", "Wattage 4.5 MWt"]);
    assert_eq!(values("i_arm_b_df"), ["Density 17.5 kg/m2"]);
    assert_eq!(values("i_fsh_b_01")[2..], ["Regeneration 60.0 HP/s", "Power 3500.0 HP"]);
    assert_eq!(values("i_rps_b_01")[2], "Regeneration 60.0 HP/s");
    assert_eq!(values("i_rdr_b_01")[2], "Sensor range 400.0 m");
    assert_eq!(values("i_def_b_01")[2], "Efficiency 85.0 %");
}

/// Every `data.tma` in the install, campaign missions included.
fn every_mission(game: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.file_name().is_some_and(|n| n.eq_ignore_ascii_case("data.tma")) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&gamedir::resolve(game, "MISSIONS").unwrap(), &mut out);
    out.sort();
    out
}

/// What keeps the six free turrets out of the player's hands is the catalogue's gate, and for
/// the hero turret the chassis page's prefixes (docs/30, "The turrets the player never
/// builds"). Over every tree a player clan reads -- which the missions say, not the file names
/// -- and every factory grade, no page offers one of the six and none lists a hero chassis.
#[test]
#[ignore = "needs the game install"]
fn no_page_of_a_tree_a_player_reads_offers_one_of_the_six_free_turrets() {
    use std::collections::BTreeSet;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let mut trees: BTreeSet<String> = BTreeSet::new();
    for path in every_mission(&game) {
        let m = mission::parse(&std::fs::read(&path).unwrap(), "data.tma").unwrap();
        for clan in m.clans.iter().filter(|c| c.kind == 1 && !c.behaviour.is_empty()) {
            trees.insert(clan.behaviour.to_ascii_lowercase());
        }
    }
    assert_eq!(trees.len(), 17, "{trees:?}");
    assert!(trees.iter().any(|t| t.ends_with("data.trf")), "data.trf is a player's tree too");

    let mut assembly = Assembly::new(&game).unwrap();
    let (mut reachable, mut listed) = (BTreeSet::new(), BTreeSet::new());
    for tree in &trees {
        let catalogue = Catalogue::open(&game, tree).unwrap();
        assert!(!catalogue.full, "the shipped Iron_3D.ini sets no FULL_RESEARCH_TREE");
        for grade in 1..=designs::CHASSIS_PREFIXES.len() {
            let mut designer = Designer::new(&game, catalogue.clone(), grade);
            for chassis in designer.catalogue.page(&designs::chassis_prefixes(grade)) {
                listed.insert(chassis.to_ascii_lowercase());
                let design = designer.chassis(&mut assembly, &chassis);
                for place in designer.places(&mut assembly, Some(&design), Tab::Turrets) {
                    reachable.extend(designer.offers(&place).iter().map(|t| t.to_ascii_lowercase()));
                }
            }
        }
    }
    assert_eq!(reachable.len(), 39, "{reachable:?}");
    for free in ["e_tur_bt_09", "e_tur_bt_10", "e_tur_bt_11", "e_tur_bt_12", "e_tur_lt_07", "e_tur_ht_02"] {
        let hung = format!("{}b{}", &free[..7], &free[8..]);
        assert!(!reachable.contains(free) && !reachable.contains(&hung), "{free} is offered");
    }
    assert!(!listed.iter().any(|c| c.starts_with("r_h")), "{listed:?}");
    assert!(!listed.contains("r_b_05") && !listed.contains("r_b_06"), "{listed:?}");
    // A control: the paid turrets do come back.
    assert!(reachable.contains("e_tur_lt_01") && reachable.contains("e_tur_bt_01"));
}
