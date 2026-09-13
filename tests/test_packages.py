"""What the commander's menus offer a unit."""

from __future__ import annotations

from openparkan import mission, objects, packages


def labels(unit_type: int, size: int) -> set[str]:
    return {p.label for p in packages.packages_for(unit_type, size)}


def test_a_small_builder_gets_the_common_packages_and_building():
    got = labels(objects.TYPE_BUILDER, 2)
    assert {"Standby", "Route", "Search and capture", "Refit", "Build", "Upgrade",
            "Search minerals"} <= got
    assert "Transport minerals" not in got


def test_a_large_warrior_cannot_capture():
    got = labels(objects.TYPE_WARRIOR, 4)
    assert "Seek and destroy" in got
    assert not got & {"Search and capture", "Capture building"}


def test_a_building_is_offered_nothing():
    assert labels(0x80000010, 3) == set()


def test_every_package_order_is_declared():
    declared = set(packages.ORDERS.values())
    assert all(p.order is None or p.order in declared for p in packages.PACKAGES)


def test_the_status_line_follows_the_head_order():
    assert packages.status([]) == "no order"
    assert packages.status([packages.SEARCH, packages.CAPTURE]) == "searching"
    assert packages.status([packages.CAPTURE]) == "capturing"
    assert packages.status([packages.ORDERS["ORDER_ROBOT_REPARE"]]) == "repairing"
    for undeclared in (10, 11, 12, 13, 14, 15, 16, 18, 25):
        assert packages.status([undeclared]) == "unknown"
    assert len(set(packages.STATUS_BY_ORDER.values())) == len(packages.STATUS) - 2


def test_a_figure_above_one_walks_at_full_speed():
    top = 12.0
    building = packages.PATROL_SPEED_PERCENT["building"]
    unit = packages.PATROL_SPEED_PERCENT["unit"]
    place = packages.PATROL_SPEED_PERCENT["place"]
    assert packages.walk_speed(top * building, top) == packages.walk_speed(top * unit, top) == top
    assert packages.walk_speed(top * place, top) == top * place
    slow = packages.walk_speed(top * building, top, maximum_factor=0.7)
    assert abs(slow - top * 0.7) < 1e-9
    assert packages.walk_speed(0.5, 12.0) == packages.MIN_WALK_SPEED
    assert packages.walk_speed(5000.0, 5000.0) == packages.MOVEMENT_MAX_SPEED


def test_the_unit_takt_runs_unless_the_player_drives_at_level_zero():
    assert packages.unit_takt_runs(driven=False)
    assert not packages.unit_takt_runs(driven=True, auto_driver=0)
    assert packages.unit_takt_runs(driven=True, auto_driver=1)
    assert packages.unit_takt_runs(driven=True, auto_driver=2)
    assert not packages.unit_takt_runs(driven=False, hero=True)


def test_every_moving_task_lets_the_fire_control_pick():
    modes = packages.FIRE_MODE_BY_ORDER
    assert modes[packages.ATTACK] == packages.FIRE_GIVEN
    assert modes[packages.STAYGROUND] == modes[packages.SEARCH] == packages.FIRE_NEAREST
    assert modes[packages.SHUTDOWN] == packages.FIRE_NONE
    assert packages.BUILD not in modes


def test_lodes_come_from_the_trailer_records():
    lode = mission.Viewpoint((10.0, 20.0, 5.0), (1, packages.MINERALS, 0x461C4000, 0x42C80000))
    m = mission.Mission(source=None, version=1, routes=[], clans=[], objects=[],
                        map_path="", description="", viewpoints=[lode],
                        unknown_pre_objects=10)
    (got,) = packages.mineral_lodes(m)
    assert (got.x, got.y, got.found, got.type_word) == (10.0, 20.0, True, packages.MINERALS)
    assert got.amount == 10000.0 and got.last == 100.0
