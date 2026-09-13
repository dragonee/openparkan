"""The words of a mission object and a lode that MisLoad.dll's reader names."""

from __future__ import annotations

import struct

from openparkan import mission


def placed(unknown, rotation=0.5, kind=mission.KIND_UNIT):
    return mission.MissionObject(
        path="UNITS\\UNITS\\HERO\\hero_3.dat", name="hero_3", logical_id=1,
        position=(1.0, 2.0, 3.0), rotation=rotation, scale=(1.0, 1.0, 1.0),
        kind=kind, unknown=unknown,
    )


def float_word(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


def test_the_word_after_the_path_is_the_clan():
    assert placed((2, (0, 0), (0, -1, -1, 1))).clan_index == 2


def test_the_padding_words_are_turns_about_x_and_y():
    o = placed((0, (float_word(0.25), 0), (0, -1, -1, 1)), rotation=1.5)
    assert o.angles == (0.25, 0.0, 1.5)


def test_a_unit_outside_every_building_has_no_host():
    o = placed((0, (0, 0), (0, mission.NOT_INSIDE, mission.NOT_INSIDE, 1)))
    assert o.host is None and o.vertex is None and o.start_flag == 0


def test_a_unit_inside_names_the_building_and_a_vertex():
    o = placed((0, (0, 0), (0, -2147483647, 7, 1)))
    assert o.host == -2147483647 and o.vertex == 7


def test_a_building_carries_its_start_flag():
    assert placed((1, (0, 0), (1, -1, -1, 1)), kind=mission.KIND_BUILDING).start_flag == 1


def test_a_lode_reads_its_flag_type_and_amount():
    lode = mission.Lode((10.0, 20.0, 0.0), (1, mission.MINERAL_LODE, float_word(1e8), 0))
    assert lode.found and lode.object_type == mission.MINERAL_LODE
    assert lode.amount == 1e8
