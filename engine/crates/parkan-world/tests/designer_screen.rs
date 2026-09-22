//! The warbot designer screen against the install and the recording of Mission 02
//! (docs/37-designer.md, docs/38-designs.md).

use parkan_formats::{gamedir, mission};
use parkan_world::cockpit::designer::{self, ROW_STEP, ROW_TOP, Screen};
use parkan_world::designs::Tab;
use parkan_world::play::Play;

fn mission_02_play() -> Play {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_02).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.02").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 02 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    play
}

/// A layout point on row `k` of the panel at `x`.
fn row(x: f32, k: usize) -> [f32; 2] {
    [x + 60.0, ROW_TOP + ROW_STEP * k as f32 + 10.0]
}

/// A click 50 ms after the last.
fn click(
    play: &mut Play,
    screen: &mut Screen,
    at: [f32; 2],
    strings: &std::collections::BTreeMap<u32, String>,
) {
    play.hero.time_ms += 50.0;
    screen.click(play, at, strings);
}

fn double(
    play: &mut Play,
    screen: &mut Screen,
    at: [f32; 2],
    strings: &std::collections::BTreeMap<u32, String>,
) {
    click(play, screen, at, strings);
    click(play, screen, at, strings);
}

#[test]
#[ignore = "needs the game install"]
fn mission_02s_designer_builds_the_recordings_warbot_by_double_clicks_and_hands_it_to_the_factory() {
    let mut play = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let f =
        play.factories.iter().position(|f| f.logic_id == 0x8000_0001_u32 as i32).expect("the Large Factory");
    let t = play.factories[f].target;
    let mut screen = Screen::default();
    screen.open(&mut play, t, &strings).unwrap();
    {
        let s = screen.session.as_ref().unwrap();
        assert_eq!(s.prompt(), Some(designer::STRING_SELECT_CHASSIS));
        assert_eq!(s.enabled, [true, false, false, false, false, false], "only Chassis at the start");
        let offered: Vec<_> = s.source.rows.iter().filter_map(|r| r.part.clone()).collect();
        assert_eq!(offered, vec!["R_B_02"], "the L-2f");
        assert_eq!(s.source.rows[0].text, "Large Flying Chs (L-2f)");
    }
    // The chassis: a double click on the source's preview, as the recording's player does.
    double(&mut play, &mut screen, [90.0, 300.0], &strings);
    {
        let s = screen.session.as_ref().unwrap();
        assert!(s.design.is_some(), "the L-2f is in");
        assert_eq!((s.tab, s.prompt()), (Tab::Turrets, Some(designer::STRING_SELECT_TURRET)));
        assert_eq!(s.name, "LF?-X Unknown");
        assert!(s.enabled[1] && s.enabled[4] && !s.enabled[2], "Turrets and Internal systems");
        assert!(!s.enabled_button(designer::Button::Accept));
    }
    // A slow second click is no double click.
    click(&mut play, &mut screen, row(0.0, 0), &strings);
    play.hero.time_ms += 300.0;
    click(&mut play, &mut screen, row(0.0, 0), &strings);
    assert!(screen.session.as_ref().unwrap().design.as_ref().unwrap().turret().is_none());
    // The turret, then two rockets and a flame thrower into empty sockets.
    click(&mut play, &mut screen, row(0.0, 0), &strings);
    assert_eq!(screen.session.as_ref().unwrap().tab, Tab::Weapons);
    for gun in ["e_gun_bl_15", "e_gun_bl_15", "e_gun_bc_06"] {
        let s = screen.session.as_ref().unwrap();
        let k = s
            .destination
            .rows
            .iter()
            .position(|r| {
                r.part.is_none()
                    && r.place.as_ref().is_some_and(|p| s.designer.offers(p).iter().any(|o| o == gun))
            })
            .expect("an empty socket for the gun");
        click(&mut play, &mut screen, row(designer::DESTINATION_X, k), &strings);
        let s = screen.session.as_ref().unwrap();
        let j = s.source.rows.iter().position(|r| r.part.as_deref() == Some(gun)).unwrap();
        double(&mut play, &mut screen, row(0.0, j), &strings);
    }
    let lines = |screen: &Screen| {
        let s = screen.session.as_ref().unwrap();
        s.rating.unwrap().lines(s.designer.offence_range, s.designer.defence_range).to_vec()
    };
    assert_eq!(lines(&screen), ["51 / 14 t", "48 kph", "23 %", "17 %", "350 m"]);
    assert!(screen.session.as_ref().unwrap().enabled[5], "a weapon enables Ammo");
    // A destination double click takes a gun off; fitted back, the figures return.
    {
        let s = screen.session.as_ref().unwrap();
        let k = s.destination.rows.iter().position(|r| r.part.as_deref() == Some("e_gun_bc_06")).unwrap();
        double(&mut play, &mut screen, row(designer::DESTINATION_X, k), &strings);
    }
    assert_eq!(lines(&screen)[3], "9 %", "two rockets left");
    assert!(screen.session.as_mut().unwrap().fit_part("e_gun_bc_06", &mut play.assembly, &strings));
    // The recording's upgrades, each a double click from the source into its slot.
    for part in [
        "i_arm_b_02",
        "i_eng_b_01",
        "i_pws_b_01",
        "i_fsh_b_01",
        "i_dsh_b_01",
        "i_rps_b_01",
        "i_rdr_b_01",
        "i_def_b_01",
    ] {
        let s = screen.session.as_mut().unwrap();
        assert!(s.fit_part(part, &mut play.assembly, &strings), "{part}");
    }
    // docs/38's 126.5 s box: the design accepted at 155.5 s.
    assert_eq!(lines(&screen), ["59 / 6 t", "48 kph", "28 %", "17 %", "400 m"]);
    let s = screen.session.as_ref().unwrap();
    assert_eq!(s.name, "LFW-X Warrior");
    assert!(s.enabled_button(designer::Button::Accept));
    // Accept: the factory gets the project, and the designer closes.
    click(&mut play, &mut screen, [213.0, 462.0], &strings);
    assert!(!screen.is_open(), "accept closes the designer");
    let project = play.factories[f].shown().cloned().expect("the project is shown");
    assert_eq!((project.name.as_str(), project.chassis_size), ("LFW-X Warrior", 4));
    assert_eq!((project.ore, project.power), (411.0, 226.5));
    assert_eq!(project.lines[0], "59 / 6 t");
    assert!(project.sphere.is_some());
    // Its bytes, held in memory, load as a robot.
    let unit = play.spawn(&project, play.player_clan, glam::Vec3::new(393.75, 854.91, 154.05), 0.0);
    let unit = unit.expect("the design spawns as a robot");
    assert_eq!(play.units[unit].type_word, project.type_word);
}

#[test]
#[ignore = "needs the game install"]
fn the_designers_exit_closes_it_and_clear_empties_the_project() {
    let mut play = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let t = play.factories[0].target;
    let mut screen = Screen::default();
    screen.open(&mut play, t, &strings).unwrap();
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    assert!(screen.session.as_ref().unwrap().design.is_some());
    // Clear, at (231, 450).
    click(&mut play, &mut screen, [240.0, 460.0], &strings);
    let s = screen.session.as_ref().unwrap();
    assert!(
        s.design.is_none() && s.tab == Tab::Chassis && s.enabled == [true, false, false, false, false, false]
    );
    // Exit, at (413, 450).
    click(&mut play, &mut screen, [420.0, 460.0], &strings);
    assert!(!screen.is_open());
}

/// A tab the fit stays on steps its destination to the next slot, so a turret's sockets, the
/// armour, the systems and the clips fill one after the other and the source panel offers the
/// next slot's parts without another click (docs/37, "Fitting").
#[test]
#[ignore = "needs the game install"]
fn a_fit_steps_the_destination_to_the_next_slot_and_the_source_offers_what_that_one_takes() {
    let mut play = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let t = play.factories[0].target;
    let mut screen = Screen::default();
    screen.open(&mut play, t, &strings).unwrap();
    // The chassis turns the panels to Turrets, the turret to Weapons, both on their first row.
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    let s = screen.session.as_ref().unwrap();
    assert_eq!((s.tab, s.destination.selected), (Tab::Weapons, Some(0)));
    let sockets = s.destination.rows.len();
    assert!(sockets > 2, "the turret has {sockets} sockets");

    // A gun into the first socket: the destination steps to the second, and the source panel
    // is what that socket offers.
    let offered = |screen: &Screen| {
        let s = screen.session.as_ref().unwrap();
        s.designer.offers(s.destination.selected_row().unwrap().place.as_ref().unwrap())
    };
    let second = {
        let s = screen.session.as_ref().unwrap();
        s.designer.offers(s.destination.rows[1].place.as_ref().unwrap())
    };
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    let s = screen.session.as_ref().unwrap();
    assert!(s.destination.rows[0].part.is_some(), "the first socket is filled");
    assert_eq!(s.destination.selected, Some(1), "and the second is selected");
    assert_eq!(offered(&screen), second);
    assert_eq!(s.source.rows.iter().filter_map(|r| r.part.clone()).collect::<Vec<_>>(), second);

    // Round the last socket it comes back to the first.
    for _ in 1..sockets {
        double(&mut play, &mut screen, row(0.0, 0), &strings);
    }
    assert_eq!(screen.session.as_ref().unwrap().destination.selected, Some(0), "wrapped round");

    // The systems tab steps down its own rows the same way.
    let s = screen.session.as_mut().unwrap();
    assert!(s.select_tab(Tab::Internal, &mut play.assembly, &strings));
    assert_eq!(s.destination.selected, Some(0));
    let slots = s.destination.rows.len();
    assert!(slots > 1, "the L-2f has {slots} system slots");
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    assert_eq!(screen.session.as_ref().unwrap().destination.selected, Some(1));
}

/// Each tab keeps its own selected row, which it shows again when it turns back on; a fit
/// turns on only the tabs the part gives rows; taking a turret off turns Weapons and Ammo off,
/// and the tab a removal is made on steps to its next row (docs/37, "Which tab and row a fit
/// leaves").
#[test]
#[ignore = "needs the game install"]
fn each_tab_keeps_its_row_and_a_removal_turns_off_the_tabs_it_empties() {
    let mut play = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let t = play.factories[0].target;
    let mut screen = Screen::default();
    screen.open(&mut play, t, &strings).unwrap();
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    {
        let s = screen.session.as_ref().unwrap();
        // Turrets, Armour and Internal systems: the L-2f gives each a row.
        assert_eq!(s.enabled, [true, true, false, true, true, false]);
        assert_eq!((s.tab, s.destination.selected), (Tab::Turrets, Some(0)));
    }
    // Internal systems at its third row, then away and back: the third row again.
    let s = screen.session.as_mut().unwrap();
    assert!(s.select_tab(Tab::Internal, &mut play.assembly, &strings));
    assert_eq!(s.destination.selected, Some(0), "a tab starts at its first row");
    click(&mut play, &mut screen, row(designer::DESTINATION_X, 2), &strings);
    let s = screen.session.as_mut().unwrap();
    assert_eq!(s.destination.selected, Some(2));
    assert!(s.select_tab(Tab::Turrets, &mut play.assembly, &strings));
    assert!(s.select_tab(Tab::Internal, &mut play.assembly, &strings));
    assert_eq!(s.destination.selected, Some(2), "the row it was left on");
    assert!(s.select_tab(Tab::Turrets, &mut play.assembly, &strings));

    // The turret turns the panels to Weapons; two guns; then Weapons steps as a gun comes off.
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    assert_eq!(screen.session.as_ref().unwrap().tab, Tab::Weapons);
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    double(&mut play, &mut screen, row(0.0, 0), &strings);
    let s = screen.session.as_ref().unwrap();
    assert_eq!(s.destination.selected, Some(2), "two sockets filled, the third selected");
    assert!(s.enabled[5], "a gun turns Ammo on");
    double(&mut play, &mut screen, row(designer::DESTINATION_X, 0), &strings);
    let s = screen.session.as_ref().unwrap();
    assert!(s.destination.rows[0].part.is_none(), "the first gun is off");
    assert_eq!(s.destination.selected, Some(1), "and the removal stepped to the next socket");
    assert!(s.enabled[5], "a gun is left, so Ammo stays on");

    // The turret off: Weapons and Ammo go off with it.
    let s = screen.session.as_mut().unwrap();
    assert!(s.select_tab(Tab::Turrets, &mut play.assembly, &strings));
    double(&mut play, &mut screen, row(designer::DESTINATION_X, 0), &strings);
    let s = screen.session.as_ref().unwrap();
    assert!(s.design.as_ref().unwrap().turret().is_none());
    assert_eq!(s.enabled, [true, true, false, true, true, false], "Weapons and Ammo are off");
}

/// Save opens the name field; typed and ended with Enter, the design is kept under its name.
/// Load lists the saved designs but for the designer's own files and ones the factory cannot
/// build, and a row loads its design back, fitted again part by part; Esc drops the list, and
/// neither takes the mouse once the mission is decided (docs/37, "The buttons").
#[test]
#[ignore = "needs the game install"]
fn save_keeps_the_design_under_the_typed_name_and_load_fits_it_again() {
    use parkan_world::cockpit::designer::{FIELD, LoadList};

    let mut play = mission_02_play();
    let game = gamedir::find(None).unwrap();
    let strings = parkan_world::resources::game_strings(&game).unwrap();
    let font = parkan_world::text::GameFont::ui(&game, "GAME_FONT").unwrap();
    let f =
        play.factories.iter().position(|f| f.logic_id == 0x8000_0001_u32 as i32).expect("the Large Factory");
    let t = play.factories[f].target;
    let mut screen = Screen::default();
    screen.open(&mut play, t, &strings).unwrap();
    // Save is dim with no project; with one it opens the field, empty and typing.
    click(&mut play, &mut screen, [285.0, 460.0], &strings);
    assert!(screen.session.as_ref().unwrap().field.is_none(), "no project, no field");
    for part in ["R_B_02", "e_tur_bb_01", "e_gun_bl_15", "e_gun_bc_06", "i_eng_b_01", "i_arm_b_02"] {
        let s = screen.session.as_mut().unwrap();
        assert!(s.fit_part(part, &mut play.assembly, &strings), "{part}");
    }
    let file = |s: &designer::Session| {
        let design = s.design.as_ref().unwrap();
        s.designer.dat_bytes(design, s.designer.type_word(design))
    };
    let saved = file(screen.session.as_ref().unwrap());
    click(&mut play, &mut screen, [285.0, 460.0], &strings);
    let field = screen.session.as_ref().unwrap().field.clone().expect("the name field");
    assert!(field.text.is_empty() && field.active);
    // While it is up the buttons take no click: exit does nothing.
    click(&mut play, &mut screen, [420.0, 460.0], &strings);
    assert!(screen.is_open());
    for c in "Tester\r".chars() {
        screen.key(c, &font);
    }
    assert!(screen.session.as_ref().unwrap().field.is_none(), "Enter saves and the field goes");
    assert_eq!(screen.saved.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["Tester"]);
    // A second, clicked shut on the field itself; and one named as the designer's own files.
    click(&mut play, &mut screen, [285.0, 460.0], &strings);
    for c in "Second".chars() {
        screen.key(c, &font);
    }
    click(&mut play, &mut screen, [(FIELD[0] + FIELD[2]) / 2.0, 420.0], &strings);
    assert_eq!(screen.saved.len(), 2, "a click on the field ends it too");
    click(&mut play, &mut screen, [285.0, 460.0], &strings);
    for c in "my_temp_unit\r".chars() {
        screen.key(c, &font);
    }
    // Esc drops a field unsaved.
    click(&mut play, &mut screen, [285.0, 460.0], &strings);
    screen.key('Z', &font);
    screen.escape();
    assert!(screen.is_open() && screen.session.as_ref().unwrap().field.is_none());
    assert_eq!(screen.saved.len(), 3);
    // Clear, then load: two rows, the designer's own name left out, in name order.
    click(&mut play, &mut screen, [240.0, 460.0], &strings);
    assert!(screen.session.as_ref().unwrap().design.is_none());
    click(&mut play, &mut screen, [320.0, 460.0], &strings);
    let list = screen.session.as_ref().unwrap().list.clone().expect("the load list");
    assert_eq!(list.names, ["Second", "Tester"]);
    // Esc drops the list and leaves the designer up; load again, and take the second row.
    screen.escape();
    assert!(screen.is_open() && screen.session.as_ref().unwrap().list.is_none());
    click(&mut play, &mut screen, [320.0, 460.0], &strings);
    let [x0, y0, x1, y1] = LoadList::row(1);
    click(&mut play, &mut screen, [(x0 + x1) / 2.0, (y0 + y1) / 2.0], &strings);
    let s = screen.session.as_ref().unwrap();
    assert!(s.list.is_none(), "a row loads and the list goes");
    assert!(file(s) == saved, "the design back, part for part: the same file");
    assert_eq!(s.tab, Tab::Chassis, "both panels turned to Chassis");
    assert!(
        s.enabled[1] && s.enabled[2] && s.enabled[5],
        "Turrets, Weapons and Ammo on as the fits left them"
    );
    // Once the mission is decided the designer takes neither the mouse nor Esc.
    assert!(screen.takes_input(&play));
    play.progression.as_mut().unwrap().progress.outcome = Some(true);
    assert!(!screen.takes_input(&play));
}
