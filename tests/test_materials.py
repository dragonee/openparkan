"""The MAT0 flags byte's route to a blend function."""

from __future__ import annotations

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
