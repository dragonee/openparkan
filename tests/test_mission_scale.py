"""A placement's scale applies to scenery only (docs/04-missions.md "The scale")."""

from __future__ import annotations

from openparkan import mission


def placed(kind: int, scale: float) -> mission.MissionObject:
    return mission.MissionObject(
        path="x", name="", logical_id=0, position=(0.0, 0.0, 0.0), rotation=0.0,
        scale=(scale, scale, scale), kind=kind,
    )


def test_vegetation_and_rock_are_built_at_their_scale():
    assert placed(mission.KIND_VEGETATION, 2.5).placed_scale == 2.5
    assert placed(mission.KIND_ROCK, 0.2).placed_scale == 0.2


def test_units_and_buildings_ignore_the_record():
    assert placed(mission.KIND_UNIT, 1.5).placed_scale == 1.0
    assert placed(mission.KIND_BUILDING, 3.0).placed_scale == 1.0
