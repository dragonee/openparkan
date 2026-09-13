"""The .exp explosion: a hit kind, its numbers, and twelve name slots."""

from __future__ import annotations

import struct

import pytest

from openparkan import effects


def build_explosion(kind=effects.HIT_DIRECT, damage=1.0, radius=2.0, placement=7,
                    names=()) -> bytes:
    blob = bytearray(effects.EXPLOSION_SIZE)
    struct.pack_into("<i4fi", blob, 0, kind, damage, radius, 1.0, 1.0, placement)
    for i, (library, member) in enumerate(names):
        at = effects.EXPLOSION_HEADER + i * effects.EXPLOSION_STRIDE
        blob[at:at + 32] = library.encode("latin-1").ljust(32, b"\0")
        blob[at + 32:at + 64] = member.encode("latin-1").ljust(32, b"\0")
    return bytes(blob)


def test_the_header_is_kind_damage_radius_and_placement():
    got = effects.parse_explosion(build_explosion(effects.HIT_AREA, 800.0, 4.0, 0))
    assert (got.kind, got.damage, got.radius, got.placement) == (effects.HIT_AREA, 800.0, 4.0, 0)
    assert got.values == (1.0, 1.0)


def test_slot_zero_is_the_effect_and_the_rest_are_by_surface():
    names = [("effects.rlb", "exp_b_mn_bul")] + [
        ("effects.rlb", f"exp_b_{tag}_bul") for tag in effects.SURFACE_TAGS]
    got = effects.parse_explosion(build_explosion(names=names))
    assert got.effect.member == "exp_b_mn_bul"
    assert [r.member[6:8] for r in got.by_surface] == list(effects.SURFACE_TAGS)
    assert len(got.effects) == effects.EXPLOSION_SLOTS


def test_blank_slots_are_left_out_of_the_effects():
    got = effects.parse_explosion(build_explosion(names=[("effects.rlb", "aim_exp_L")]))
    assert [str(r) for r in got.effects] == ["effects.rlb/aim_exp_L"]
    assert not any(got.by_surface)


def test_the_first_word_is_not_a_count():
    # kind 3 with one name: read as a count it would ask for three
    got = effects.parse_explosion(build_explosion(effects.HIT_AREA, names=[("a.rlb", "b")]))
    assert len(got.effects) == 1


def test_anything_but_792_bytes_is_refused():
    with pytest.raises(effects.EffectFormatError, match="not the 792"):
        effects.parse_explosion(build_explosion()[:-64])
