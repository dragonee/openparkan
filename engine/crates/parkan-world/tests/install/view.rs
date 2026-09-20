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
    // 20 s on the box is gone.
    play.hero.time_ms += 20_001.0;
    let later = cockpit.draw(&play, Space::new(640.0, 480.0), &font, &menu, view_proj);
    assert!(!later.text.iter().any(|r| r.text.starts_with("from:")), "the box lives 20 s");
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
    let r = play.outer_bound(None);
    assert!((r - 1.473).abs() < 2e-3, "the hero's chassis box's half-diagonal: {r}");
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
        assert_eq!((eye.forward, eye.fov_x, eye.near), (own.forward, 1.3, 0.5));
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
