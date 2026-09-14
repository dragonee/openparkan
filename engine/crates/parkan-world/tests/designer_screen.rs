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
