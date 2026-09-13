"""The MAT0 flags byte's route to a blend function."""

from __future__ import annotations

import struct

import pytest

from openparkan import materials


def material(blend: int) -> materials.Material:
    return materials.Material(name="X", entry_count=0, track_count=0, blend=blend)


@pytest.mark.parametrize(("flags", "mode", "function"), [
    (materials.BLEND_OPAQUE, 0, ("ONE", "ZERO", False)),
    (materials.BLEND_LIT, 0, ("ONE", "ZERO", False)),
    (materials.BLEND_ALPHA, 4, ("SRCALPHA", "INVSRCALPHA", True)),
    (materials.BLEND_ALPHA | materials.BLEND_BIT0, 4,
     ("SRCALPHA", "INVSRCALPHA", True)),
    (materials.BLEND_ADD, 2, ("SRCALPHA", "ONE", True)),
])
def test_the_flags_byte_reaches_a_blend_function(flags, mode, function):
    m = material(flags)
    assert m.blend_mode == mode
    assert m.blend_function == function


def test_the_engine_alpha_tests_whenever_it_blends():
    """Mode 0 is the only one with alpha testing off."""
    for mode, (_, _, test) in materials.BLEND_MODES.items():
        assert test == (mode != 0)


def test_an_index_past_the_translate_table_has_no_mode():
    """The table is five long; nothing shipped runs past it, but a wrong
    reading of the field would."""
    m = material(0xFF)
    assert m.blend_index == 0xF
    assert m.blend_mode is None
    assert m.blend_function is None


def test_the_header_carries_the_ground_a_unit_stands_on():
    header = struct.pack("<2HBBfI", 0, 0, 1, materials.UNSET, 1.0, 0x461C4000)
    m = materials.parse("WATER_BOT", header)
    assert m.surface == 1
    assert m.speed_factor == 1.0
    assert m.damage_rate == materials.LIQUID_BED_RATE


def test_a_zero_dword_is_no_damage():
    m = materials.parse("L02", struct.pack("<2HBBfI", 0, 0, 1, materials.UNSET, 1.0, 0))
    assert m.damage_rate == 0.0


def ground(names: list[str]) -> materials.Material:
    entries = [materials.MaterialEntry(texture=n) for n in names]
    tracks = [materials.Track(kind=0, param=0, keys=[materials.Key(entry=i, time=0)])
              for i in range(len(names))]
    return materials.Material(name="L20", entry_count=len(names),
                              track_count=len(names), entries=entries, tracks=tracks)


def test_the_landscape_layers_the_second_track_over_the_first():
    twin = ground(["L20.0", "L20M.0"])
    assert twin.entry_for_track(materials.GROUND_DETAIL_TRACK).texture == "L20M.0"
    assert twin.entry_for_track(0).texture == "L20.0"


def test_a_track_the_material_lacks_is_taken_as_track_0():
    """Both manager fetches clamp; the library's variant() does not."""
    single = ground(["L08.0"])
    assert single.variant(materials.GROUND_DETAIL_TRACK) is None
    assert single.entry_for_track(materials.GROUND_DETAIL_TRACK).texture == "L08.0"
    assert single.entry_for_track(-1).texture == "L08.0"
