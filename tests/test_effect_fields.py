"""An effect's settings switch, test point, light, bolt, stream and fades."""

from __future__ import annotations

import struct

import pytest

from openparkan import effects


def header(setting=0x312, point=(1.0, 0.0, 0.0), flags=0, count=0):
    blob = bytearray(effects.HEADER_SIZE)
    struct.pack_into("<IIffI", blob, 0, count, effects.TIME_POINT, 0.0, 0.0, flags)
    struct.pack_into("<I", blob, effects.HEADER_SETTING_AT, setting)
    struct.pack_into("<3f", blob, effects.HEADER_POINT_AT, *point)
    return bytes(blob)


def block(kind, **floats):
    body = bytearray(effects.EMITTER_SIZE[kind])
    struct.pack_into("<I", body, 0, kind)
    for at, value in floats.items():
        off = int(at[1:])
        if isinstance(value, int):
            struct.pack_into("<I", body, off, value)
        else:
            struct.pack_into(f"<{len(value)}f", body, off, *value)
    return bytes(body)


def test_the_settings_id_names_a_switch_and_presets_turn_them_off():
    fx = effects.parse_effect(header(0x312))
    assert fx.setting == "Gun fire"
    assert fx.test_point == (1.0, 0.0, 0.0)
    assert effects.setting_enabled(0x000, 1)
    assert not effects.setting_enabled(0x000, 3)
    assert effects.setting_enabled(0x10F, 3)
    assert effects.RENDER_QUALITY_PRESET[2] == 1
    assert len(effects.EFFECT_SETTINGS) == 20
    assert {s & 0xFF for s in effects.EFFECT_SETTINGS} == set(range(20))


def test_a_type_one_block_is_a_light():
    body = block(1, o4=6, o16=(0.0, 0.0, 0.0), o28=(1.0, 0.0, 0.0),
                 o64=(3.0, 2.0, 0.0, 0.0), o80=(0.5, 0.3, 0.01, 0.0),
                 o112=(30.0, 3.0), o124=(0.0, 1.0, 0.0))
    fx = effects.parse_effect(header(count=1) + body)
    light = fx.emitters[0].light
    assert light.type == effects.LIGHT_POINT and light.flags == 0xA0000000
    assert light.colour[1] == pytest.approx((0.5, 0.3, 0.01, 0.0))
    assert light.range == (30.0, 3.0)
    assert light.attenuation == (0.0, 1.0, 0.0)
    assert light.position[1] == (1.0, 0.0, 0.0)


def test_a_bolt_draws_one_sprite_a_segment_within_its_count():
    fx = effects.parse_effect(header(count=1) + block(5, o20=20, o36=(250.0,)))
    bolt = fx.emitters[0]
    assert bolt.bolt_segments(10.0) == 1
    assert bolt.bolt_segments(600.0) == 2
    assert bolt.bolt_segments(1e6) == 20
    assert bolt.light is None


def test_a_stream_and_a_particle_carry_their_rate_and_fade():
    fx = effects.parse_effect(header(count=3) + block(8, o4=(0.5, 0.0, 1.0),
                                                       o24=(0.05, 0.02), o36=10)
                              + block(7, o8=(1.0, 0.0, 4.0)) + block(2))
    stream, burst, sound = fx.emitters
    assert stream.emission_interval == pytest.approx((0.05, 0.02))
    assert stream.particle_lifetime == pytest.approx((0.5, 0.2))
    assert stream.fade == (0.5, 0.0, 1.0)
    assert burst.fade == (1.0, 0.0, 4.0)
    assert sound.fade is None and burst.particle_lifetime is None


def test_the_header_names_a_spread_for_flag_1_and_an_amplitude_for_flag_8():
    blob = bytearray(header(flags=effects.FX_JITTER | effects.FX_RANDOM_OFFSET))
    struct.pack_into("<f", blob, effects.HEADER_JITTER_AT, 0.2)
    struct.pack_into("<3f", blob, effects.HEADER_OFFSET_AT, 1.0, 2.0, 3.0)
    fx = effects.parse_effect(bytes(blob))
    assert fx.flags & effects.FX_JITTER
    assert struct.unpack_from("<f", fx.header, effects.HEADER_JITTER_AT)[0] == pytest.approx(0.2)
    assert struct.unpack_from("<3f", fx.header, effects.HEADER_OFFSET_AT) == (1.0, 2.0, 3.0)
    # The two never overlap, and neither touches the tested point.
    assert effects.HEADER_JITTER_AT + 4 == effects.HEADER_FLAGS_AT
    assert effects.HEADER_OFFSET_AT + 12 == effects.HEADER_POINT_AT


def test_a_sprites_phase_sits_where_its_fade_and_window_do_not():
    fx = effects.parse_effect(header(count=2) + block(3, o8=(-1.0, 9.0, 1.0))
                              + block(7, o32=(0.6,)))
    sprite, burst = fx.emitters
    start, end, power = effects.PHASE_AT[3]
    assert struct.unpack_from("<3f", sprite.body, start) == (-1.0, 9.0, 1.0)
    assert (start, end, power) == (8, 12, 16)
    # A sprite's phase runs before its fade (+20..+28) and its window (+32..+36),
    # and a burst keeps an animation rate at +32 instead of a phase.
    assert end < effects.SPRITE_FADE_AT[0] < effects.WINDOW_AT[3]
    assert 3 not in effects.ANIMATION_RATE_TYPES
    assert set(effects.ANIMATION_RATE_TYPES) == {7, 8, 10}
    rate = struct.unpack_from("<f", burst.body, effects.ANIMATION_RATE_AT)[0]
    assert rate == pytest.approx(0.6)


def test_the_time_modes_name_the_owner_properties_they_read():
    assert effects.TIME_PROPERTY_VELOCITY == 0x21
    assert effects.TIME_PROPERTY_LIMITS == 0x11
    assert effects.TIME_PROPERTY_SPIN == 0x24
    assert effects.TIME_PROPERTY_LIFE == 0x31
    assert effects.TIME_PROPERTY_INVERSE == 14
    assert effects.TIME_POINT_INVERSE == 13
    assert effects.TIME_MOTION == 15
    fx = effects.parse_effect(header())
    assert fx.mode == effects.TIME_POINT
