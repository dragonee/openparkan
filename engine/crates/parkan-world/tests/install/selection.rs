//! The command-mode pick on the player's own path: the cursor's ray, what stops it and what
//! it takes, and what the Guard row's pick gives (docs/42).

use crate::common::{command_frames, mission_03_play, mission_04_aboard_the_hq, object_target, press_enter};
use parkan_sim::orders::{self, Target};
use parkan_world::pick::{Aim, GUARD_CLICK_RADIUS, GUARD_PICK_RADIUS, PickMode, cursor_state};
use parkan_world::play::{Mode, Play};

/// Mission 03 with the hero on the bunker's pod until the pod opens its command view.
fn mission_03_command_view() -> (Play, parkan_formats::mission::Mission, usize) {
    let (mut play, m) = mission_03_play();
    let bunker = object_target(&play, &m, "sbunk01.dat");
    assert!(play.stand_on_pod(bunker), "the bunker has a pod");
    for _ in 0..(20 * 60) {
        if play.mode() == Mode::Command(bunker) {
            break;
        }
        play.tick(1000.0 / 60.0, [0.0; 2]);
    }
    assert_eq!(play.mode(), Mode::Command(bunker), "the pod opens the bunker's command view");
    command_frames(&mut play, 1, |_| {});
    (play, m, bunker)
}

/// The command camera's ray at `p`.
fn at(play: &Play, p: glam::Vec3) -> Aim {
    let eye = play.eye().position;
    Aim::Ray { eye, direction: (p - eye).normalize() }
}

/// The robot of target `t`'s order.
fn order_of(play: &Play, t: usize) -> Option<orders::Order> {
    play.robots.iter().find(|(rt, _)| *rt == t).and_then(|(_, r)| r.order)
}

/// The hero on foot in a bunker's command view keeps its parent, so the world's object pick
/// takes it like any unit (`0x100361f6`): the player's own, kind 7 and the `PICK` cursor. A
/// click on it selects nothing and turns no page, since selecting refuses a hero
/// (`0x1007d0d4`); the Guard row's pick takes it, and its patrol has radius 300.
#[test]
#[ignore = "needs the game install"]
fn in_the_bunkers_command_view_the_cursor_takes_the_hero_on_its_pod_and_guard_patrols_300_about_it() {
    let (mut play, m, _) = mission_03_command_view();
    let builder = object_target(&play, &m, "tut3_b.dat");
    let hero = play.battle.combat.hero_index();
    let centre = play.battle.combat.hero.as_ref().unwrap().centre;
    let pick = play.pick(at(&play, centre));
    assert_eq!((pick.kind, pick.object), (7, Some(hero)), "{pick:?}");
    assert_eq!(cursor_state(pick.kind), 2, "PICK");
    // The click lets the bunker go and selects nothing.
    assert!(!play.selected.is_empty(), "the pod selected its bunker");
    assert_eq!(play.click_world(pick), None, "no page");
    assert!(play.selected.is_empty() && play.selected_units().is_empty());
    // The same on the satellite map, where the hero's record stands.
    let place = play.hero_place();
    let pick = play.pick(Aim::Map([place.x, place.y]));
    assert_eq!((pick.kind, pick.object), (7, Some(hero)), "{pick:?}");

    // The builder selected and Guard opened: the hero is a unit to guard, kind 9, GUARD.
    play.select_unit_alone(builder);
    let Some(parkan_sim::hq::Act::Guard) = play.hq_command(6) else { panic!("the Guard row") };
    play.open_pick(parkan_sim::hq::Act::Guard);
    assert_eq!(play.commander.pick_mode, PickMode::Guard);
    let pick = play.pick(at(&play, centre));
    assert_eq!((pick.kind, pick.object), (9, Some(hero)), "{pick:?}");
    assert_eq!(cursor_state(pick.kind), 5, "GUARD");
    play.click_world(pick);
    assert_eq!(play.commander.pick_mode, PickMode::Free);
    let order = order_of(&play, builder).expect("the builder is given the guard");
    assert_eq!(
        (order.code, order.parameter, order.target),
        (orders::PATROL, GUARD_PICK_RADIUS, Target::LogicId(play.hero_id))
    );
}

/// Aboard a bot the hero's object has no parent, and the pick passes it over: in Mission 04's
/// HQ command view, entered from aboard the HQ, a ray through where the hero rides takes no
/// hero (`0x100637fa`).
#[test]
#[ignore = "needs the game install"]
fn aboard_the_hq_in_its_command_view_the_cursor_passes_the_hero_over() {
    let (mut play, _, hq) = mission_04_aboard_the_hq();
    command_frames(&mut play, 60, |_| {});
    press_enter(&mut play);
    assert_eq!(play.mode(), Mode::HqCommand(hq));
    command_frames(&mut play, 1, |_| {});
    let hero = play.battle.combat.hero_index();
    let centre = play.battle.combat.hero.as_ref().unwrap().centre;
    let pick = play.pick(at(&play, centre));
    assert_ne!(pick.object, Some(hero), "{pick:?}");
}

/// The cursor's ray asks the world for the landscape and the buildings only (`0x10035e82`,
/// class mask `0xa`): through a unit it points at the ground beyond, and a building's shell
/// stops it. The Guard row's pick wants a unit or a place: a building answers kind 9 and the
/// click leaves the pick open; a place closes it with a patrol of radius 300. A guard clicked
/// on an own building outside it has radius 100.
#[test]
#[ignore = "needs the game install"]
fn the_cursors_ray_passes_a_unit_to_the_ground_stops_on_a_building_and_guard_takes_no_building() {
    let (mut play, m, _) = mission_03_command_view();
    let builder = object_target(&play, &m, "tut3_b.dat");
    let transport = object_target(&play, &m, "tut3_t.dat");
    let plant = object_target(&play, &m, "lplant01.dat");
    let eye = play.eye().position;
    // Through the builder's centre: the point lies beyond it, on the ground.
    let unit = play.battle.combat.targets[builder].centre;
    let direction = (unit - eye).normalize();
    let point = play.cursor_point(eye, direction).expect("the ground under the ray");
    assert!(point.distance(eye) > unit.distance(eye) + 1.0, "beyond the builder: {point} {unit}");
    let ground = play.ground.below(point.x, point.y, point.z + 1.0).unwrap().point;
    assert!((ground.z - point.z).abs() < 0.5, "on the ground: {point} over {ground}");
    // Through the plant's centre: its shell stops the ray short of the centre, above the ground.
    let building = play.battle.combat.targets[plant].centre;
    let point = play.cursor_point(eye, (building - eye).normalize()).expect("the plant");
    assert!(point.distance(eye) < building.distance(eye), "on its shell: {point} {building}");
    let under = play.ground.below(point.x, point.y, point.z + 1.0).map_or(f32::MIN, |h| h.point.z);
    assert!(point.z > under + 1.0, "above the ground: {point}, ground {under}");

    // Guard opened for the builder: over the plant the cursor is GUARD, and a click does nothing.
    play.select_unit_alone(builder);
    play.open_pick(parkan_sim::hq::Act::Guard);
    let pick = play.pick(at(&play, building));
    assert_eq!((pick.kind, pick.object), (9, Some(plant)), "{pick:?}");
    play.click_world(pick);
    assert_eq!(play.commander.pick_mode, PickMode::Guard, "the pick stays open");
    assert!(play.commander.pending.is_some());
    // A place beside the builder closes it: a patrol of radius 300 there.
    let beside = play.battle.combat.targets[builder].position + glam::Vec3::new(30.0, 0.0, 0.0);
    let pick = play.pick(at(&play, beside));
    assert_eq!((pick.kind, pick.object), (8, None), "{pick:?}");
    play.click_world(pick);
    assert_eq!(play.commander.pick_mode, PickMode::Free);
    let order = order_of(&play, builder).expect("the guard");
    assert_eq!(
        (order.code, order.parameter, order.target),
        (orders::PATROL, GUARD_PICK_RADIUS, Target::Place(pick.point.unwrap().to_array()))
    );

    // Outside the pick, the transport selected, a click on the plant guards it: radius 100.
    play.select_unit_alone(transport);
    let pick = play.pick(at(&play, building));
    assert_eq!((pick.kind, pick.object), (10, Some(plant)), "{pick:?}");
    play.click_world(pick);
    let order = order_of(&play, transport).expect("the guard");
    assert_eq!(
        (order.code, order.parameter, order.target),
        (orders::PATROL, GUARD_CLICK_RADIUS, Target::LogicId(play.units[plant].logical_id))
    );
}
