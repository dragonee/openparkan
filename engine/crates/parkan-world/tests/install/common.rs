//! Fixtures the tests share: a mission opened from the install, and the moves made on it.

use parkan_formats::gamedir;

/// Mission 01's play with its progression, the hero standing still.
pub(crate) fn mission_01_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_01).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.01").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 01 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// Stand the hero `distance` from `target`'s placement, facing it, on the ground.
pub(crate) fn stand_facing(play: &mut parkan_world::play::Play, target: usize, distance: f32, around: f32) {
    assert!(play.stand_facing(target, distance, around), "somewhere level to stand");
}

/// Stand a machine at `at`'s xy on the ground below it.
pub(crate) fn put(w: &mut parkan_sim::machine::Walker, ground: &parkan_sim::ground::Ground, at: glam::Vec3) {
    w.body.position = at;
    w.follow_ground(ground);
    w.from = (w.body.position, w.body.yaw);
}

/// Mission 01 with both neutral warbots captured and a radar takt later on the hero's
/// radar: the play, and the targets of `tut1_mf1`, `helic` and the hostile `tut1_e1`.
pub(crate) fn mission_01_wingmen() -> (parkan_world::play::Play, [usize; 3]) {
    mission_01_captured(false)
}

/// [`mission_01_wingmen`], a captured bot given Standby when `standby`.
pub(crate) fn mission_01_captured(standby: bool) -> (parkan_world::play::Play, [usize; 3]) {
    let (mut play, m) = mission_01_play();
    play.capture_standby = standby;
    let tick = 1000.0 / 60.0;
    let target_of = |path: &str| {
        let object = m.objects.iter().position(|o| o.path.to_ascii_lowercase().ends_with(path)).unwrap();
        play.battle.objects.iter().position(|&o| o == object).unwrap()
    };
    let bots = [target_of("tut1_mf1.dat"), target_of("helic.dat"), target_of("tut1_e1.dat")];
    for bot in [bots[0], bots[1]] {
        stand_facing(&mut play, bot, 12.0, 3.5);
        play.tick(tick, [0.0; 2]);
        while play.targets.current != Some(bot) {
            play.targets.select_next();
        }
        assert!(play.enter());
    }
    for _ in 0..60 {
        play.tick(tick, [0.0; 2]);
    }
    (play, bots)
}

pub(crate) fn mission_02_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_02).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.02").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 02 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// The centre of the level-0 bounding sphere of `node` of a target's part, in the world.
pub(crate) fn door_centre(
    play: &parkan_world::play::Play,
    target: usize,
    part_index: usize,
    node: usize,
) -> glam::Vec3 {
    let part = &play.battle.combat.targets[target].parts[part_index];
    let slot = &part.mesh.slots[usize::from(part.mesh.nodes[node].slot_index[0])];
    let [cx, cy, cz, _] = slot.sphere;
    let c = part.nodes[node].apply([cx, cy, cz].map(|v| f64::from(v * part.scale)));
    glam::Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)
}

pub(crate) fn mission_03_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_03).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.03").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 03 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// The target of the mission object whose path ends in `name`.
pub(crate) fn object_target(
    play: &parkan_world::play::Play,
    m: &parkan_formats::mission::Mission,
    name: &str,
) -> usize {
    play.battle
        .objects
        .iter()
        .position(|&o| m.objects[o].path.to_ascii_lowercase().ends_with(name))
        .unwrap_or_else(|| panic!("no {name}"))
}

/// Play `seconds` at 60 ticks a second, calling `each` after every tick.
pub(crate) fn play_for(
    play: &mut parkan_world::play::Play,
    seconds: f32,
    mut each: impl FnMut(&mut parkan_world::play::Play),
) {
    for _ in 0..(seconds * 60.0) as usize {
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        each(play);
    }
}

/// Walk the hero along `route`, W held and turned toward each point in turn, for at most
/// `seconds`, W let go at the last point: the tick it reached the first point on the building's
/// floor (the first whose ground is a building's face), the tick it reached the last point, the
/// index of the point it was heading for, and the tick `done` first held.
pub(crate) fn walk_route(
    play: &mut parkan_world::play::Play,
    route: &[[f32; 2]],
    seconds: usize,
    done: impl Fn(&parkan_world::play::Play) -> bool,
) -> (Option<usize>, Option<usize>, usize, Option<usize>) {
    play.hero.key("SCAN_W", true);
    let (mut on_floor, mut arrived, mut finished, mut next) = (None, None, None, 0);
    for tick in 0..(60 * seconds) {
        let at = play.hero.walker.body.position;
        while next < route.len() && glam::Vec2::from_array(route[next]).distance(at.truncate()) < 1.2 {
            next += 1;
        }
        if on_floor.is_none() && play.hero.walker.ground.is_some_and(|g| g.solid.is_some()) {
            on_floor = Some(tick);
        }
        if next == route.len() {
            if arrived.is_none() {
                arrived = Some(tick);
                play.hero.key("SCAN_W", false);
            }
        } else {
            let to = glam::Vec2::from_array(route[next]) - at.truncate();
            play.hero.walker.body.yaw = (-to.x).atan2(to.y);
        }
        play.update_input();
        play.tick(1000.0 / 60.0, [0.0; 2]);
        if done(play) {
            finished = Some(tick + 1);
            break;
        }
    }
    (on_floor, arrived, next, finished)
}

pub(crate) fn mission_04_play() -> (parkan_world::play::Play, parkan_formats::mission::Mission) {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, gamedir::MISSION_04).unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), "Mission.04").unwrap();
    let mut play = Play::load(&game, &m).unwrap().expect("Mission 04 has a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    (play, m)
}

/// Enter, `CMD_ENTER_STATE`, from the view the player has.
pub(crate) fn press_enter(play: &mut parkan_world::play::Play) {
    let eye = play.eye();
    let view = parkan_world::play::View {
        eye: eye.position,
        look: eye.forward,
        view_proj: glam::Mat4::IDENTITY,
        shift: false,
    };
    play.command(parkan_formats::controls::CMD_ENTER_STATE, &view);
}

/// Mission 04 with its HQ targeted from 12 m and Enter pressed.
pub(crate) fn mission_04_aboard_the_hq() -> (parkan_world::play::Play, parkan_formats::mission::Mission, usize)
{
    let (mut play, m) = mission_04_play();
    let hq = object_target(&play, &m, "tut4_hq.dat");
    stand_facing(&mut play, hq, 12.0, 0.0);
    play.tick(1000.0 / 60.0, [0.0; 2]);
    for _ in 0..=play.targets.listed.len() {
        if play.targets.current == Some(hq) {
            break;
        }
        play.targets.select_next();
    }
    assert_eq!(play.targets.current, Some(hq), "the HQ is on the hero's list");
    press_enter(&mut play);
    (play, m, hq)
}

/// A frame of command mode's camera and a tick of play, `n` times.
pub(crate) fn command_frames(
    play: &mut parkan_world::play::Play,
    n: usize,
    mut each: impl FnMut(&parkan_world::play::Play),
) {
    for _ in 0..n {
        play.update_input();
        play.command_frame(play.hero.time_ms / 1000.0, parkan_world::command::Edges::default());
        play.tick(1000.0 / 60.0, [0.0; 2]);
        each(play);
    }
}

/// A campaign mission's progression on its own: its clans' scripts, its objects and its
/// messages, with no scene under it, so a test can run a whole mission's worth of takts.
pub(crate) fn campaign_progression(
    path: &str,
) -> (parkan_world::progress::Progression, parkan_formats::mission::Mission) {
    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    campaign_progression_at(path, parkan_world::settings::game_level(&game))
}

/// [`campaign_progression`] at game level `level`: 0 easy, 1 medium, 2 hard.
pub(crate) fn campaign_progression_at(
    path: &str,
    level: usize,
) -> (parkan_world::progress::Progression, parkan_formats::mission::Mission) {
    use parkan_formats::mission;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, path).unwrap();
    let name = path.rsplit('/').next().unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), name).unwrap();
    let hero = m.objects.iter().position(|o| parkan_world::hero::is_hero(&o.path)).expect("a hero");
    let p = parkan_world::progress::Progression::load(&game, &dir, &m, hero, level).unwrap();
    (p, m)
}

/// A campaign mission's play with its progression, the hero standing still.
pub(crate) fn campaign_play(path: &str) -> parkan_world::play::Play {
    campaign_play_at(path, None)
}

/// [`campaign_play`] at a game level of its own: 0 easy, 1 medium, 2 hard.
pub(crate) fn campaign_play_at(path: &str, level: Option<usize>) -> parkan_world::play::Play {
    use parkan_formats::mission;
    use parkan_world::play::Play;

    let game = gamedir::find(None).expect("a Parkan install: set PARKAN_DIR");
    let dir = gamedir::resolve(&game, path).unwrap();
    let name = path.rsplit('/').next().unwrap();
    let m = mission::parse(&std::fs::read(dir.join("data.tma")).unwrap(), name).unwrap();
    let mut play = Play::load_at(&game, &m, level).unwrap().expect("a hero");
    play.load_progression(&game, &dir, &m).unwrap();
    play
}
