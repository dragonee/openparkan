//! What the player sees: the eye, the zoom, the outer camera, the cockpit and the panels
//! drawn over them.

use crate::common::*;
use parkan_formats::gamedir;

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
    // Value 1 is the far plane the sun's ray reaches half of (docs/10, "The lens flare").
    assert_eq!(eye.far, 1000.0);
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
    // A 128 × 128 atlas of seven rows 18 pixels apart, and a header that says so:
    // height 17, so the cell is 18, and a v span of 18/128 (docs/12).
    assert_eq!((font.width, font.height), (128, 128));
    assert_eq!(font.atlas.len(), 128 * 128 * 4);
    assert_eq!(font.line_height, 18.0);
    assert_eq!(font.v_span * font.height as f32, font.line_height, "one texel to the pixel");
    assert_eq!(font.spacing, 1.0, "header word 3, 1 on all eleven shipped fonts");
    // The atlas is keyed on palette index 0, cleared to alpha 0 on the font's alpha
    // surface, and lit by index 73, which is white.
    assert_eq!(font.atlas[..4], [0, 0, 0, 0], "the corner is the key");
    assert!(font.atlas.as_chunks::<4>().0.contains(&[255, 255, 255, 255]), "and the glyphs are white");
    let run = TextRun::new("Objective is completed", [0.0, 0.0]);
    let placed = font.layout(&run);
    assert_eq!(placed.len(), "Objectiveiscompleted".len(), "every letter is drawn, no space is");
    let width = font.width(&run);
    assert!(width > 100.0 && width < 300.0, "the line is {width} pixels");
    assert!(placed.windows(2).all(|w| w[1].at[0] > w[0].at[0]), "the pen only moves right");
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
fn the_outcome_panels_fonts_are_the_640_by_480_menu_and_game_fonts_of_font_lib() {
    use parkan_world::text::GameFont;
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    // `ui/menu_resources.cfg`: MENU_FONT_640x480 = 8, mf_640.tft; GAME_FONT_640x480 = 6.
    let menu = GameFont::ui(&game, "MENU_FONT").unwrap();
    let text = GameFont::ui(&game, "GAME_FONT").unwrap();
    assert_eq!((menu.width, text.width), (256, 128));
    // Each font's header gives its height, and the cell is one more: mf_640 is 12 and
    // gf_640 is 7 (docs/12, "The header's four words are the font's metrics").
    assert!((menu.line_height - 13.0).abs() < 0.5, "{}", menu.line_height);
    assert!((text.line_height - 8.0).abs() < 0.5, "{}", text.line_height);
    // The pen steps a glyph's advance plus the header's spacing, 1 on both: the
    // 18 glyphs of "MISSION COMPLETE !" sum 125 advances and 18 steps of spacing.
    assert_eq!((menu.spacing, text.spacing), (1.0, 1.0));
    assert_eq!(menu.advance("MISSION COMPLETE !"), 125.0 + 18.0);
    assert!(menu.advance("MISSION COMPLETE !") > text.advance("MISSION COMPLETE !"));
    // The nine ui/font.lib fonts are laid out in Windows-1251, so a Cyrillic capital
    // draws at 0xC0 and there is a glyph there; gamefont.rlb's ARIALTEX.TFT is CP866.
    use parkan_world::text::{Encoding, glyph_index};
    assert_eq!((menu.encoding, glyph_index(menu.encoding, 'А')), (Encoding::Windows1251, 0xC0));
    assert!(menu.glyphs[0xC0].drawn() && menu.glyphs[0xFF].drawn(), "А and я are drawn");
    assert!(!menu.glyphs[0x81].drawn(), "and CP866's Б is not: 0x80-0xBF is the 1251 punctuation");
    let game_font = GameFont::open(&game).unwrap();
    assert_eq!(game_font.encoding, Encoding::Cp866);
    assert!(game_font.glyphs[0x80].drawn() && !game_font.glyphs[0xC0].drawn());
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
    // Each view carries its unit's own lights (docs/35, "The unit in the middle"): the first
    // travels (-1, 1, -1) in the unit's own frame, the vector as it is, and the second the
    // other way at 0.35 of its colour. Every unit in the game was attached twice, by
    // `CreateObject` and by `AddObjectToGame`, and its mesh made a pair each time: two pairs.
    for v in &drawn.views {
        let [a, b] = v.lights;
        assert!((a.colour - 0.5).abs() < 1e-6 && (b.colour - 0.175).abs() < 1e-6, "{:?}", v.lights);
        assert!(a.travel.abs_diff_eq(-b.travel, 1e-6));
        assert!((a.travel.length() - 3f32.sqrt()).abs() < 1e-4, "not normalised: {:?}", a.travel);
    }
    let turn = play.hero.walker.drawn_turn(play.hero.time_ms);
    assert!(drawn.views[1].lights[0].travel.abs_diff_eq(turn * Vec3::new(-1.0, 1.0, -1.0), 1e-5));
    let turn = play.battle.combat.targets[helic].rotation();
    assert!(drawn.views[0].lights[0].travel.abs_diff_eq(turn * Vec3::new(-1.0, 1.0, -1.0), 1e-5));
    // A dummy is its chassis alone, and has the two pairs like any unit: its parts do not
    // count. Its aim face looks along its +y, at the hero coming from the west, and takes the
    // counter lights alone: 0.175 on the face's green of 198 over a node's 0.5 is the
    // recording's 162, where one pair would leave 145.
    assert_eq!(play.battle.combat.targets[dummies[0]].parts.len(), 1);
    play.targets.set(Some(dummies[0]));
    let dummy = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    let [a, b] = dummy.views[0].lights;
    assert_eq!(dummy.views[0].unit, Some(dummies[0]));
    assert!((a.colour - 0.5).abs() < 1e-6 && (b.colour - 0.175).abs() < 1e-6, "{a:?} {b:?}");
    let target = &play.battle.combat.targets[dummies[0]];
    let face = target.rotation() * Vec3::Y;
    let start = m.objects.iter().find(|o| o.path.to_ascii_lowercase().ends_with("tut1_p.dat")).unwrap();
    let to_start = (Vec3::from_array(start.position) - target.position).with_z(0.0).normalize();
    assert!(face.dot(to_start) > 0.95, "the aim face looks back at the hero's start: {face}, {to_start}");
    let lit: f32 = [a, b].iter().map(|l| l.colour * (-face.dot(l.travel)).max(0.0)).sum();
    assert_eq!((255.0 * (0.5 + 198.0 / 255.0 * lit)).round(), 162.0);
    play.targets.set(Some(helic));
    // 20 s on the box is gone.
    play.hero.time_ms += 20_001.0;
    let later = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    assert!(!later.text.iter().any(|r| r.text.starts_with("from:")), "the box lives 20 s");
}

/// The name raises its clan's count for every unit it names, the hero's "Human" and a *Tiny
/// Tower* among them (`0x10075eb2`); a robot whose device manager answers its id 2 with 0 or
/// less, which only the Small Tower chassis `R_B_06`'s battery of capacity -1 makes it do, is
/// a *Tiny Tower* (`0x10075e17`), and on a walking warrior its panel icons are the towers'
/// cell alone (`0x10077342`). *Seen*: Mission 03's builder is SWB-2, and in the recording of
/// *Outflanking Maneuver* the player's wingman reads SWW-2 and the enemy's TSW-4 to TSW-6.
#[test]
#[ignore = "needs the game install"]
fn every_unit_named_raises_its_clans_count_and_the_small_towers_robots_are_tiny_towers() {
    use parkan_formats::mission;
    use parkan_world::cockpit::commander::{TOWER_CELL, unit_cells};
    use parkan_world::cockpit::panels::Panels;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let names = |play: &parkan_world::play::Play, m: &mission::Mission| {
        let panels = Panels::new(play, &strings);
        (0..play.units.len())
            .filter(|&t| !panels.names[t].is_empty())
            .map(|t| {
                let path = &m.objects[play.battle.objects[t]].path;
                (path.rsplit('\\').next().unwrap().to_owned(), panels.names[t].clone())
            })
            .collect::<Vec<_>>()
    };
    let (play, m) = mission_03_play();
    let named = names(&play, &m);
    let of = |named: &[(String, String)], dat: &str| {
        named.iter().filter(|(p, _)| p == dat).map(|(_, n)| n.clone()).collect::<Vec<_>>()
    };
    assert_eq!(of(&named, "tut3_b.dat"), ["SWB-2 Builder"], "the hero is Plr's first");
    assert_eq!(of(&named, "tut3_t.dat"), ["SWT-3 Transport"]);
    assert_eq!(of(&named, "tut3_f2.dat"), ["SFW-2 Warrior"], "Enm has no hero");

    let dir = gamedir::resolve(&game, gamedir::C01_MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let play = parkan_world::play::Play::load(&game, &m).unwrap().expect("a hero");
    let named = names(&play, &m);
    assert_eq!(of(&named, "12wel1.dat"), ["SWW-2 Warrior"]);
    assert_eq!(of(&named, "12wel2.dat"), ["SWW-2 Warrior", "SWW-1 Warrior", "SWW-2 Warrior"]);
    assert_eq!(of(&named, "12wlk1.dat"), ["SSW-8 Warrior"]);
    assert_eq!(of(&named, "12tower.dat"), ["Tiny Tower", "Tiny Tower"]);
    let towers: Vec<usize> = (0..play.units.len())
        .filter(|&t| m.objects[play.battle.objects[t]].path.ends_with("12tower.dat"))
        .collect();
    for &t in &towers {
        let (u, d) = (&play.units[t], play.units[t].designation);
        assert!(d.negative_battery && d.battery, "R_B_06's -1 beside the fitted battery");
        assert_eq!(
            unit_cells(u.type_word, d.chassis_type, d.tiny_tower(u.type_word)),
            [Some(TOWER_CELL), None]
        );
    }

    // Over every placed robot: 28 are Tiny Towers, every one a walking warrior on R_B_06.
    let mut assembly = parkan_world::assembly::Assembly::new(&game).unwrap();
    let profiles = gamedir::resolve(&game, parkan_formats::profiles::ARCHIVE)
        .and_then(|p| parkan_formats::nres::Archive::open(&p).ok());
    let mut files = vec![game.join("MISSIONS")];
    let (mut robots, mut tiny, mut walking_warriors) = (0, 0, 0);
    while let Some(dir) = files.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.push(path);
                continue;
            }
            if !path.file_name().is_some_and(|n| n.eq_ignore_ascii_case("data.tma")) {
                continue;
            }
            let m = mission::parse(&std::fs::read(&path).unwrap(), "data.tma").unwrap();
            for o in m.objects.iter().filter(|o| o.kind == mission::KIND_UNIT) {
                let type_word = o.property("Type").map_or(0, |p| match p.value {
                    parkan_formats::mission::Value::Int(i) => i as u32,
                    parkan_formats::mission::Value::Float(f) => f as u32,
                });
                if type_word & parkan_world::robot::TYPE_ROBOT == 0 {
                    continue;
                }
                robots += 1;
                let d = parkan_world::robot::designation(&mut assembly, profiles.as_ref(), o.kind, &o.path);
                if d.tiny_tower(type_word) {
                    tiny += 1;
                    walking_warriors += usize::from(type_word == 0x0100_8000 && d.chassis_type == 2);
                }
            }
        }
    }
    assert_eq!((robots, tiny, walking_warriors), (267, 28, 28));
}

/// A building is named on the target panel as a unit is, from the string its behaviour holds:
/// the building record's slot 1 hands it `iron3d.dll:0x100338d0`'s, by its Type and its size
/// class (docs/35, "Name and status"), and the panel prints no status under it. The recording
/// names every building it points at so ("Let's Play - Parkan: Iron Strategy, Part 4"): on *The
/// Lost Key* the Small Bunker (1:50), the Small Generator (2:45), the Medium Mine (4:47), the
/// Large Factory (9:07) and the Small Warehouse (15:08); on *The Last Bastion* the Light Tower
/// (16:27), the Small Outpost (18:50) and the Teleport (38:37).
#[test]
#[ignore = "needs the game install"]
fn a_building_is_named_on_the_target_panel_by_its_type_and_size_class() {
    use glam::{Mat4, Vec3};
    use parkan_formats::mission;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::panels::Panels;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let names = |path: &str, of: &[&str]| {
        let dir = gamedir::resolve(&game, path).unwrap();
        let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "mission").unwrap();
        let play = campaign_play(path);
        let panels = Panels::new(&play, &strings);
        of.iter().map(|dat| panels.names[object_target(&play, &m, dat)].clone()).collect::<Vec<_>>()
    };
    let buildings = ["sbunk02.dat", "gener01.dat", "mmine01.dat", "lplant01.dat", "sto_l_n1.dat"];
    assert_eq!(
        names(gamedir::C02_MISSION_03, &buildings),
        ["Small Bunker", "Small Generator", "Medium Mine", "Large Factory", "Small Warehouse"]
    );
    assert_eq!(
        names(gamedir::C02_MISSION_04, &["mtow02.dat", "shang03.dat", "mtp_m_n1.dat"]),
        ["Light Tower", "Small Outpost", "Teleport"]
    );

    // And drawn on the panel, the Small Bunker targeted from the hero's place in it.
    let dir = gamedir::resolve(&game, gamedir::C02_MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let mut play = campaign_play(gamedir::C02_MISSION_03);
    let bunker = object_target(&play, &m, "sbunk02.dat");
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    play.targets.set(Some(bunker));
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    let eye = play.hero.eye();
    let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
        * Mat4::look_to_rh(eye.position, eye.forward, Vec3::Z);
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    let texts: Vec<&str> = drawn.text.iter().map(|r| r.text.as_str()).collect();
    assert!(texts.contains(&"Small Bunker"), "the name line: {texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with('[')), "and no status under it: {texts:?}");
}

#[test]
#[ignore = "needs the game install"]
fn leaving_the_window_lets_shift_up_and_the_free_look_centres_on_foot_and_aboard() {
    use parkan_world::factory::Project;

    let (mut play, _) = mission_02_play();
    let tick = |play: &mut parkan_world::play::Play, counts: [f32; 2], n: usize| {
        for _ in 0..n {
            play.update_input();
            play.tick(1000.0 / 60.0, counts);
        }
    };
    tick(&mut play, [0.0; 2], 30);
    // Shift + the mouse turns the camera's free look alone: the sight stays (docs/30).
    let sight = play.hero.sight().expect("the hero's sight").1;
    play.key("SCAN_LSHIFT", true);
    tick(&mut play, [40.0, 30.0], 20);
    tick(&mut play, [0.0; 2], 10);
    let look = play.hero.rig.look;
    assert!((look[0] - 0.5).abs() > 0.05 && (look[1] - 0.5).abs() > 0.05, "free look {look:?}");
    assert!(play.hero.sight().unwrap().1.dot(sight) > 0.9999, "the guns stay put");
    assert!(play.eye().forward.dot(sight) < 0.95, "the eye turns away from the sight");
    // The window left with Shift down: its key-up never arrives, and every key comes up
    // (`stdSetApplicationState`, docs/14): Shift's release rows centre the free look.
    play.release_keys();
    assert_eq!(&play.hero.rig.look[..2], &[0.5, 0.5]);
    tick(&mut play, [40.0, 0.0], 10);
    assert_eq!(&play.hero.rig.look[..2], &[0.5, 0.5], "the mouse no longer moves the camera");

    // Aboard, the same for the bot's own table.
    let project = Project {
        path: "UNITS\\bld_unit_-2147483647.dat".into(),
        name: "LFW-2 Warrior".into(),
        type_word: 0x0100_8000,
        chassis_size: 4,
        ore: 0.0,
        power: 0.0,
        lines: Vec::new(),
        sphere: None,
    };
    let hero_at = play.hero.walker.body.position;
    let t = play.spawn(&project, play.player_clan, hero_at + glam::Vec3::new(8.0, 0.0, 1.0), 0.0).unwrap();
    tick(&mut play, [0.0; 2], 30);
    assert!(play.board(t));
    play.key("SCAN_LSHIFT", true);
    play.key("SCAN_LMOUSE", true);
    tick(&mut play, [40.0, 30.0], 20);
    assert_ne!(&play.driven().rig.look[..2], &[0.5, 0.5]);
    assert!(play.driven().guns.iter().any(|g| g.state == parkan_sim::guns::CONTINUE_FIGHT));
    play.release_keys();
    assert_eq!(&play.driven().rig.look[..2], &[0.5, 0.5]);
    tick(&mut play, [0.0; 2], 1);
    assert!(play.driven().guns.iter().all(|g| g.state == parkan_sim::guns::STATE_OFF), "the guns stop");
}

/// The reticle's dot is placed by the camera's placement in its unit, property 0 of the camera
/// object (docs/35, "The reticle"): `(320 − 24 x, 240 − 24 y)` by the first column, the way the
/// camera looks in the unit's own frame. The hero standing looks down its frame's y, so the dot
/// stands above the middle, 24 × the cosine of the sight's pitch from it, where Part 1's recording
/// holds it through most of its walking; walking with a strafe key the hull turns half a right
/// angle under the turret, and the dot goes 45° round toward the strafe, as the recording
/// shows it going round to (±17, −18).
#[test]
#[ignore = "needs the game install"]
fn the_reticles_dot_stands_above_the_middle_and_goes_round_toward_a_strafe() {
    use parkan_world::cockpit::radar::reticle_dot;

    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play, n: usize| {
        for _ in 0..n {
            play.update_input();
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    // The dot's centre from the screen's middle, as a distance and an angle clockwise from up;
    // and the distance a level look would put it at, 24 × the look's share across the ground.
    let dot = |play: &parkan_world::play::Play| {
        let look = play.driven().look_in_frame().expect("the hero's camera");
        let [x, y] = reticle_dot(look);
        let [dx, dy] = [x + 1.0 - 320.0, y + 1.0 - 240.0];
        (dx.hypot(dy), dx.atan2(-dy).to_degrees(), 24.0 * look.truncate().length())
    };
    tick(&mut play, 30);
    let (r, angle, level) = dot(&play);
    assert!((r - level).abs() <= 1.0 && angle.abs() < 1.0, "standing: {r} at {angle}°, {level}");
    assert!(level > 20.0);
    for (key, side) in [("SCAN_A", -1.0), ("SCAN_D", 1.0)] {
        play.key("SCAN_W", true);
        play.key(key, true);
        tick(&mut play, 90);
        let (r, angle, level) = dot(&play);
        assert!(
            (r - level).abs() <= 1.5 && (angle - side * 45.0).abs() < 4.0,
            "W and {key}: {r} at {angle}°"
        );
        play.key(key, false);
        play.key("SCAN_W", false);
        tick(&mut play, 90);
        let (_, angle, _) = dot(&play);
        assert!(angle.abs() < 1.0, "the hull back under the turret: {angle}°");
    }
}

/// The interface's controls step their lines by the font's height as its slot `+0x0c` answers
/// it, the header's word 1, plus a constant of their own (docs/12, "Where a caller puts the next
/// line"), not by the glyph cell. On the 640 × 480 `GAME_FONT`, `gf_640`, the height is 7 under
/// a cell of 8: the message box (`0x1007ff32`) steps 9, the unit lines 9 after a first 10, the
/// briefing's subtitles (`0x10031763`) and the research descendants (`0x10089213`) 8. At
/// 1024 × 768 `gf_1024`'s 11 over the scale of 1.6 gives the message box and the unit box 8, as
/// the recordings show both (docs/35, "The message box"; docs/41, "The box"), where the cell's 12
/// would give 9.
#[test]
#[ignore = "needs the game install"]
fn the_interfaces_line_steps_are_the_fonts_height_and_a_constant_of_their_own() {
    use parkan_world::cockpit::Ink;
    use parkan_world::hud::{Painter, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let font = GameFont::ui(&game, "GAME_FONT").unwrap();
    assert_eq!((font.height(), font.line_height), (7.0, 8.0), "gf_640: the height under a cell of 8");
    let ink = Ink {
        painter: Painter::new(Space::new(640.0, 480.0)),
        text: Vec::new(),
        font: &font,
        menu_runs: Vec::new(),
        menu: &font,
    };
    assert_eq!(ink.line_step(), 9.0, "the message box: the height and 2");
    let (first, step) = ((font.height() + 3.0).round(), (font.height() + 2.0).round());
    assert_eq!((first, step), (10.0, 9.0), "the unit lines");
    assert_eq!(
        (font.height() + 1.0).round(),
        font.line_height,
        "the briefing's and the descendants' step is the cell"
    );

    // At 1024 × 768: gf_1024's header word 1 is 11.
    let path = parkan_formats::gamedir::resolve(&game, "ui/font.lib").unwrap();
    let archive = parkan_formats::nres::Archive::open(&path).unwrap();
    let entry = archive.entries.iter().find(|e| e.name.eq_ignore_ascii_case("gf_1024.tft")).unwrap();
    let tft = parkan_formats::font::parse_font(archive.read(entry).unwrap(), &entry.name).unwrap();
    let sy = 768.0 / 480.0;
    assert_eq!(tft.height, 11);
    assert_eq!(((tft.height as f32 + 2.0) / sy).round(), 8.0, "8 apart, as the recordings show");
    assert_eq!(((tft.height as f32 + 1.0 + 2.0) / sy).round(), 9.0, "the cell would set them 9 apart");
}

/// A game command as its key runs it, from the unit's own view.
fn press_command(play: &mut parkan_world::play::Play, command: &str) {
    let eye = play.own_eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: glam::Mat4::IDENTITY,
        shift: false,
    };
    play.command(command, &view);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_zoom_narrows_the_eye_to_0_2_in_eleven_updates_and_halves_the_mouse() {
    use parkan_formats::controls::CMD_JAMES_ZOOM_MODE;

    // docs/30-turrets.md, "The zoom": 0.1 an update toward 0.2, the mouse at 0.5 while zoomed.
    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play| {
        play.tick(1000.0 / 60.0, [0.0; 2]);
    };
    tick(&mut play);
    assert!((play.eye().fov_x - 1.3).abs() < 1e-6);
    assert!((play.hero.pilot.sensitivity - 1.0).abs() < 1e-6, "MOUSE_SENS=100");
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    let mut fields = Vec::new();
    for _ in 0..12 {
        tick(&mut play);
        fields.push(play.eye().fov_x);
    }
    assert!((fields[10] - 0.2).abs() < 1e-5 && (fields[9] - 0.3).abs() < 1e-5, "{fields:?}");
    assert!((fields[11] - fields[10]).abs() < 1e-6, "it holds at 0.2");
    assert!((play.hero.pilot.sensitivity - 0.5).abs() < 1e-6);
    // Mid-way a press is let be; at the end it turns back out.
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    for _ in 0..3 {
        tick(&mut play);
    }
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    for _ in 0..8 {
        tick(&mut play);
    }
    assert!((play.eye().fov_x - 1.3).abs() < 1e-5, "{}", play.eye().fov_x);
    tick(&mut play);
    assert!((play.hero.pilot.sensitivity - 1.0).abs() < 1e-6);
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_outer_camera_stands_at_its_four_places_about_the_hero_and_goes_back_into_the_eye() {
    use parkan_formats::controls::{CMD_JAMES_OUTER_CAMERA, CMD_JAMES_ZOOM_MODE};
    use parkan_world::camera::PLACES;

    // docs/30-turrets.md, "The outer camera".
    let (mut play, _) = mission_01_play();
    let tick = |play: &mut parkan_world::play::Play, n: usize| {
        for _ in 0..n {
            play.tick(1000.0 / 60.0, [0.0; 2]);
        }
    };
    tick(&mut play, 1);
    // r, the hero's record's `+0x98`: its agent's bounding cylinder's radius, half the diagonal
    // of the end face of the chassis's and the turret's boxes joined, 1.502 by 2.478 across and
    // 3.372 tall (docs/40, "r, the unit record's `+0x98`").
    let r = play.record_bound(None);
    assert!((r - 1.449).abs() < 2e-3, "half of the hypotenuse of 1.502 and 2.478: {r}");
    for (k, &(angle, back)) in PLACES.iter().enumerate() {
        press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
        tick(&mut play, 30);
        let own = play.own_eye();
        let eye = play.eye();
        let heading = own.forward.y.atan2(own.forward.x) + angle;
        let want = own.position
            - glam::Vec3::new(heading.cos(), heading.sin(), 0.0) * r * back
            - glam::Vec3::Z * r * parkan_world::camera::WALKER_DROP;
        assert!(eye.position.distance(want) < 1e-3, "place {k}: {} against {want}", eye.position);
        assert_eq!((eye.forward, eye.fov_x, eye.near, eye.far), (own.forward, 1.3, 0.5, 700.0));
        // On the hero's right for the first two, on its left for the others.
        let right = own.forward.cross(own.up);
        assert_eq!((eye.position - own.position).dot(right) > 0.0, k < 2, "place {k}");
    }
    // Z does nothing from the outer camera's view.
    press_command(&mut play, CMD_JAMES_ZOOM_MODE);
    tick(&mut play, 12);
    assert!(!play.hero.zoom.on);
    press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
    tick(&mut play, 13);
    assert!(play.outer.on(), "the move back is 0.24 s");
    tick(&mut play, 2);
    assert!(!play.outer.on() && play.eye() == play.own_eye());

    // A change of mode turns it off at once.
    press_command(&mut play, CMD_JAMES_OUTER_CAMERA);
    tick(&mut play, 30);
    assert!(play.outer_shows());
    play.modes.push(parkan_world::play::Mode::Factory(0));
    assert!(!play.outer_shows());
    tick(&mut play, 1);
    play.modes.pop();
    assert!(!play.outer.on());
}

/// The right button's pick on Mission 01's hero takes its two figures off the hero's record
/// (docs/25, "The player's target"): the ray starts `+0x98` out along the view, the hero's
/// bounding cylinder's 1.449, and its first choice keeps `+0x94`, the hero's node sphere's
/// radius, clear of the hero.
#[test]
#[ignore = "needs the game install"]
fn mission_01s_right_button_starts_its_ray_the_heros_bound_out_and_keeps_its_node_sphere_clear() {
    use glam::{Mat4, Vec3};
    use parkan_formats::controls::CMD_JAMES_AIM_TARGET;

    let (mut play, _) = mission_01_play();
    play.tick(1000.0 / 60.0, [0.0; 2]);
    let (r, margin) = (play.record_bound(None), play.hero.bound.1);
    assert!((r - 1.449).abs() < 2e-3 && margin > 0.5, "{r} and {margin}");
    let eye = play.own_eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: Mat4::perspective_rh(1.0, 4.0 / 3.0, 0.1, 2000.0)
            * Mat4::look_to_rh(eye.position, eye.forward, eye.up),
        shift: false,
    };
    // Two of the mission's units listed and put by hand: one half a metre past the ray's start
    // and 0.2 off it, one 50 m down the line.
    let units: Vec<usize> = play.robots.iter().map(|(t, _)| *t).collect();
    assert!(units.len() >= 2, "{units:?}");
    let (near, far) = (units[0], units[1]);
    play.targets.listed = vec![near, far];
    let side = eye.forward.cross(eye.up).normalize();
    let hero = play.hero.walker.body.position;
    let put = |play: &mut parkan_world::play::Play, t: usize, centre: Vec3, radius: f32| {
        let target = &mut play.battle.combat.targets[t];
        (target.position, target.centre, target.radius) = (centre, centre, radius);
    };
    let at = eye.position + eye.forward * (r + 0.5) + side * 0.2;
    put(&mut play, near, at, 2.0);
    put(&mut play, far, eye.position + eye.forward * 50.0, 4.0);
    let apart = at.distance(hero);
    assert!(apart > 0.6 * 2.0 && apart <= 0.6 * 2.0 + margin, "{apart} against {margin}");
    // The ray passes through the near one's sphere, ahead of its start; it stands inside the
    // margin, so the far one is the first choice.
    play.targets.current = None;
    play.command(CMD_JAMES_AIM_TARGET, &view);
    assert_eq!(play.targets.current, Some(far));
    // Out of the margin, the near one is taken: 0.6 of a 0.5 m sphere and the margin fall short.
    put(&mut play, near, at, 0.5);
    assert!(apart > 0.6 * 0.5 + margin, "{apart}");
    play.targets.current = None;
    play.command(CMD_JAMES_AIM_TARGET, &view);
    assert_eq!(play.targets.current, Some(near));
    // A sensor range that does not reach past the ray's start: nothing happens, and the target
    // is kept (`0x100914e2`). From the eye itself the pick would run and take the far one.
    play.hero.radar.range = r - 0.1;
    play.command(CMD_JAMES_AIM_TARGET, &view);
    assert_eq!(play.targets.current, Some(near));
    play.hero.radar.range = r + 0.1;
    play.command(CMD_JAMES_AIM_TARGET, &view);
    assert_eq!(play.targets.current, Some(far), "the pick runs, and the far one is nearest the middle");
}

#[test]
#[ignore = "needs the game install"]
fn mission_01s_own_panel_keeps_the_running_hero_steady_in_its_view() {
    use glam::{Mat4, Quat, Vec3};
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Pages, Space};
    use parkan_world::text::GameFont;

    // docs/35-hud.md, "The unit in the middle": the view's camera stands off the object as it is
    // drawn, so the hero running holds still in it. Placed by each step's end, the camera jumps a
    // stride at a time.
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, _) = mission_01_play();
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let font = GameFont::ui(&game, "GAME_FONT").unwrap();
    let menu = GameFont::ui(&game, "MENU_FONT").unwrap();
    play.hero.key("SCAN_W", true);
    let (mut drawn_step, mut stepped_step) = (0.0_f32, 0.0_f32);
    let (mut last_drawn, mut last_stepped): (Option<Vec3>, Option<Vec3>) = (None, None);
    for tick in 0..180 {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if tick < 60 {
            continue;
        }
        let hero = &play.hero;
        let (position, yaw) = hero.walker.drawn(hero.time_ms);
        let eye = hero.eye();
        let view_proj = Mat4::perspective_infinite_reverse_rh(1.0, 4.0 / 3.0, 0.5)
            * Mat4::look_to_rh(eye.position, eye.forward, Vec3::Z);
        let frame = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
        let own = frame.views.iter().find(|v| v.unit.is_none()).expect("the own panel's view");
        // The camera against the hero as drawn, in the hero's frame.
        let offset = Quat::from_rotation_z(-yaw) * (own.eye - position);
        let body = &hero.walker.body;
        let stepped = Quat::from_rotation_z(-yaw) * (body.position - position);
        if let (Some(a), Some(b)) = (last_drawn, last_stepped) {
            drawn_step = drawn_step.max(offset.distance(a));
            stepped_step = stepped_step.max(stepped.distance(b));
        }
        (last_drawn, last_stepped) = (Some(offset), Some(stepped));
    }
    eprintln!("camera against the drawn hero {drawn_step} m a tick at most; the step's end {stepped_step} m");
    assert!(stepped_step > 0.2, "the step's end jumps {stepped_step} m in a tick");
    assert!(drawn_step < 0.05, "the camera moves {drawn_step} m against the drawn hero in a tick");
}

/// The software mouse cursor is the last thing the frame draws. Every other batch of the HUD
/// lies under the text, which the renderer lays down in its own pass after the art, so a
/// pointer over the commander's rows was painted on by their words; it draws in the top layer
/// instead, over the text as well (docs/42, "The cursor shows a state").
#[test]
#[ignore = "needs the game install"]
fn in_command_mode_the_cursor_is_the_one_thing_drawn_over_the_panels_text() {
    use glam::Mat4;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::hud::{Layer, Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    play.command_frame(0.0, parkan_world::command::Edges::default());
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    // The builders' page, with the cursor resting on its rows as PICK.
    cockpit.update(&mut play, 0.0);
    cockpit.commander.turn(&mut play, 3, 0.0);
    cockpit.commander.cursor = Some([200.0, 140.0]);
    cockpit.commander.cursor_state = 2;
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, Mat4::IDENTITY);
    assert!(drawn.text.iter().any(|r| !r.text.is_empty()), "the page's rows are drawn in GAME_FONT");
    let top: Vec<usize> = drawn
        .batches
        .iter()
        .enumerate()
        .filter(|(_, b)| b.layer == Layer::OverText)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(top.len(), 1, "one batch draws over the text");
    assert_eq!(top[0], drawn.batches.len() - 1, "and it is the last of the frame");
    // The cursor's one quad, two triangles of three vertices.
    assert_eq!(drawn.batches[top[0]].vertices.len(), 6);
}

/// The game menu, `CState` mode 7 (docs/39, "The game menu"): F3 or Esc on foot, or the
/// commander column's button, pushes it; the world stands still under it; it alone is drawn,
/// its title and its four buttons, *Save game* and *Load game* grey in the training campaign;
/// *Resume game* or F3 again rolls it back to the view under it, which it never left.
#[test]
#[ignore = "needs the game install"]
fn the_game_menu_pauses_the_world_draws_alone_and_resume_returns_to_the_view_under_it() {
    use glam::Mat4;
    use parkan_world::cockpit::Cockpit;
    use parkan_world::cockpit::game_menu::{self, Button};
    use parkan_world::hud::{Pages, Space};
    use parkan_world::play::Mode;
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    assert!(play.training, "Mission 03 is the training campaign's");
    play_for(&mut play, 0.5, |_| {});

    // On foot: the menu goes on top, and nothing moves while it is up.
    assert!(play.game_menu());
    assert_eq!((play.mode(), play.view_mode()), (Mode::GameMenu, Mode::OnFoot));
    assert!(play.mode().shows_cursor());
    let (clock, hero, builder) =
        (play.hero.time_ms, play.hero.walker.body.position, play.robots[0].1.time_ms);
    play_for(&mut play, 1.0, |_| {});
    assert_eq!((play.hero.time_ms, play.hero.walker.body.position), (clock, hero), "the game is paused");
    assert_eq!(play.robots[0].1.time_ms, builder);
    // F3 again closes it; the game goes on.
    assert!(play.game_menu());
    assert_eq!(play.mode(), Mode::OnFoot);
    play_for(&mut play, 0.1, |_| {});
    assert!(play.hero.time_ms > clock);

    // Over a bunker's view the world is still drawn from the command camera, and the menu is
    // drawn alone: no page, no map, no message box.
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    play.command_frame(0.0, parkan_world::command::Edges::default());
    let camera = play.eye();
    assert!(play.game_menu());
    assert_eq!(play.view_mode(), Mode::Command(bunker));
    assert_eq!(play.eye(), camera, "the view under the menu is the command camera's");
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    let (font, menu) = (GameFont::ui(&game, "GAME_FONT").unwrap(), GameFont::ui(&game, "MENU_FONT").unwrap());
    cockpit.commander.cursor = Some([300.0, 220.0]);
    let drawn = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, Mat4::IDENTITY);
    let words: Vec<String> = drawn.text.iter().map(|r| r.text.clone()).collect();
    let strings = |id: u32| cockpit.strings[&id].clone();
    assert_eq!(
        words,
        [5085, 3080, 3081, 3082, 3083].map(strings),
        "the title and the four buttons, nothing else in GAME_FONT"
    );
    let colour = |i: usize| drawn.text[i].colour;
    let argb = parkan_world::cockpit::argb;
    assert_eq!(colour(1), argb(game_menu::TEXT_HOVERED), "Resume game under the cursor");
    assert_eq!(colour(2), argb(game_menu::TEXT_DISABLED), "Save game, grey in training");
    assert_eq!(colour(3), argb(game_menu::TEXT_DISABLED), "Load game, grey in training");
    assert_eq!(colour(4), argb(game_menu::TEXT), "Quit game");
    // A click on Save game does nothing in training; Resume game rolls the menu back.
    assert_eq!(game_menu::click(&play, [300.0, 240.0]), None);
    assert_eq!(game_menu::click(&play, [300.0, 300.0]), Some(Button::Quit));
    assert_eq!(game_menu::click(&play, [300.0, 220.0]), Some(Button::Resume));
    assert!(play.roll_back());
    assert_eq!(play.mode(), Mode::Command(bunker));
}

/// Tooltips (docs/37, "A tooltip"): each frame the manager is cleared, a widget under the
/// cursor hands it its text as it is drawn -- the commander column's buttons, enabled or not,
/// and the map's exit -- and once the cursor has rested within 2 pixels for more than 250 ms
/// the text shows in `TOOL_FONT` on a pale yellow box 16 under the cursor.
#[test]
#[ignore = "needs the game install"]
fn a_column_button_under_a_resting_cursor_shows_its_tooltip_after_a_quarter_second() {
    use glam::Mat4;
    use parkan_world::cockpit::{Cockpit, tooltip};
    use parkan_world::hud::{Layer, Pages, Space};
    use parkan_world::text::GameFont;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    play.units[bunker].clan = Some(play.player_clan);
    play.enter_command(bunker);
    let pages = Pages::open(&game).unwrap();
    let mut cockpit = Cockpit::open(&game, &pages, &play).unwrap();
    assert!(!cockpit.system_cursor, "the install's Iron_3D.ini forces the software cursor");
    let fonts = ["GAME_FONT", "MENU_FONT", "TOOL_FONT"].map(|n| GameFont::ui(&game, n).unwrap());
    let space = Space::new(640.0, 480.0);
    cockpit.update(&mut play, 0.0);
    let frame = |cockpit: &mut Cockpit, cursor: [f32; 2], clock: f64| {
        cockpit.commander.cursor = Some(cursor);
        cockpit.draw(&play, space, &fonts[0], &fonts[1], Mat4::IDENTITY);
        cockpit.tooltip(space, &fonts[2], clock)
    };
    // The chat button, disabled in single play, still hands its text: 5038 *Chat*.
    let chat = [20.0, 380.0];
    assert!(frame(&mut cockpit, chat, 1000.0).1.is_empty(), "the rest starts");
    assert!(frame(&mut cockpit, [21.0, 381.0], 1200.0).1.is_empty());
    let (batches, text) = frame(&mut cockpit, [21.0, 381.0], 1251.0);
    assert_eq!(text.len(), 1);
    assert_eq!(text[0].text, cockpit.strings[&5038]);
    assert!(!batches.is_empty() && batches.iter().all(|b| b.layer == Layer::Tip));
    // Three pixels off, the wait starts again.
    assert!(frame(&mut cockpit, [24.0, 381.0], 1300.0).1.is_empty());
    // The Game menu button: 1302 *Game menu*.
    let menu = [20.0, 440.0];
    frame(&mut cockpit, menu, 2000.0);
    let (_, text) = frame(&mut cockpit, menu, 2300.0);
    assert_eq!(text[0].text, cockpit.strings[&1302]);
    // Off every widget, nothing.
    frame(&mut cockpit, [320.0, 240.0], 3000.0);
    assert!(frame(&mut cockpit, [320.0, 240.0], 4000.0).1.is_empty());
    // The box is the text's width and the font's height, each plus 8, 16 under the cursor.
    let width = fonts[2].advance(&cockpit.strings[&1302]);
    let [x0, y0, x1, y1] = tooltip::place(space, menu, width, fonts[2].line_height, tooltip::BELOW);
    assert_eq!([x0, y0], [menu[0], menu[1] + 16.0]);
    assert_eq!([x1 - x0, y1 - y0], [width + 8.0, fonts[2].line_height + 8.0]);
    let _ = m;
}

/// Camouflage draws the machine in the material of the face its ground search holds
/// (docs/07, "What a camouflaged machine wears"): on the landscape the face's first material,
/// on a bridge's deck the batch's own.
#[test]
#[ignore = "needs the game install"]
fn a_camouflaged_hero_wears_the_grounds_first_material_and_a_bridges_deck_on_it() {
    use glam::Vec3;
    let (mut play, _) = mission_01_play();
    play_for(&mut play, 0.25, |_| {});
    assert_eq!(play.camouflage(), None, "off, the hero draws in its own skin");
    play.hero.pilot.switches.camouflage = true;

    // Where Mission 01 starts: a landscape face, and `Land1.wea` at its first byte.
    let hit = play.hero.walker.ground.expect("the hero stands on something");
    let face = hit.face.expect("a landscape face");
    let land = &play.ground.land;
    let first = land.layer1[usize::from(land.faces[face].tex1)].clone();
    assert_eq!(play.camouflage(), Some(first.as_str()));
    // A face with a second material laid over it still gives its first.
    let two = land
        .faces
        .iter()
        .position(|f| f.tex2 != parkan_formats::landmesh::NO_TEXTURE && f.tex1 != f.tex2 && f.normal[2] > 0.9)
        .expect("Tut_1 has two-material faces");
    let corner = |i: usize| Vec3::from_array(land.positions[usize::from(land.faces[two].vertices[i])]);
    let middle = (corner(0) + corner(1) + corner(2)) / 3.0;
    let (lower, upper) = (
        land.layer1[usize::from(land.faces[two].tex1)].clone(),
        land.layer1[usize::from(land.faces[two].tex2)].clone(),
    );
    put(&mut play.hero.walker, &play.ground, middle + Vec3::Z * 5.0);
    assert_eq!(play.hero.walker.ground.and_then(|h| h.face), Some(two));
    assert_eq!(play.camouflage(), Some(lower.as_str()));
    assert_ne!(lower, upper);

    // On the bridge over the gorge: the deck's batch, in the bridge's own wear.
    put(&mut play.hero.walker, &play.ground, Vec3::new(790.5, 690.0, 40.0));
    let deck = play.hero.walker.ground.expect("the deck holds the hero");
    let (bridge, _) = deck.solid.expect("a face of the bridge, not the gorge under it");
    let worn = play.camouflage().expect("the deck's batch has a material").to_owned();
    assert!(play.battle.wears[bridge].iter().flatten().any(|name| *name == worn), "{worn} is the bridge's");
    assert!(!play.ground.land.layer1.contains(&worn), "{worn} is not the landscape's");

    play.hero.pilot.switches.camouflage = false;
    assert_eq!(play.camouflage(), None);
}
