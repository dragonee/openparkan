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


def test_a_surface_plays_its_own_slot_and_falls_back_to_slot_zero():
    names = [("effects.rlb", "exp_b_mn_bul")] + [
        ("effects.rlb", f"exp_b_{tag}_bul") for tag in effects.SURFACE_TAGS[:7]]
    got = effects.parse_explosion(build_explosion(names=names))
    assert got.slot_for(7 - 1).member == "exp_b_gr_bul"
    assert got.slot_for(7).member == "exp_b_mn_bul"
    assert got.slot_for(None).member == "exp_b_mn_bul"
    assert got.slot_for(0xFF).member == "exp_b_mn_bul"


def build_effect(mode, duration, flags, emitters=()):
    header = bytearray(effects.HEADER_SIZE)
    struct.pack_into("<IIff I", header, 0, len(emitters), mode, duration, 0.0, flags)
    struct.pack_into("<3f", header, effects.HEADER_SCALE_AT, 0.1, 0.1, 0.1)
    return bytes(header) + b"".join(emitters)


def test_an_effect_header_names_its_time_mode_duration_and_flags():
    fx = effects.parse_effect(build_effect(effects.TIME_ONCE, 1.5,
                                           effects.FX_DELETE_AT_END | effects.FX_KEEP_WHEN_HIDDEN))
    assert (fx.mode, fx.duration) == (effects.TIME_ONCE, 1.5)
    assert fx.flags & effects.FX_DELETE_AT_END
    assert fx.scale == pytest.approx((0.1, 0.1, 0.1))


def test_an_emitter_is_active_over_its_window():
    block = bytearray(effects.EMITTER_SIZE[3])
    struct.pack_into("<I", block, 0, 3)
    struct.pack_into("<2f", block, effects.WINDOW_AT[3], 0.01, 0.5)
    fx = effects.parse_effect(build_effect(effects.TIME_POINT, 0.75, 0, [bytes(block)]))
    assert fx.emitters[0].window == pytest.approx((0.01, 0.5))
