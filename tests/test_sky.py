"""The .ske day cycle, on files this module builds itself."""

from __future__ import annotations

import math
import struct
from pathlib import Path

import pytest

from openparkan import sky


def time(hour: int, minute: int, section: int = 0, tail: tuple[int, int] = (0, 0)) -> bytes:
    return struct.pack("<8I", section, 0, 0, hour, minute, 0, *tail)


def string(text: str) -> bytes:
    return struct.pack("<I", len(text)) + text.encode("latin-1")


def keyframe(hour: int, minute: int, opcode: int = sky.NOTHING, name: str = "",
             effects: tuple[str, ...] = (), light: float = 1.0, weather: float = 0.0,
             version: int = sky.KEYFRAME_VERSION, slots: list[bytes] | None = None) -> bytes:
    stored = sky.SLOT_COUNT if version >= 2 else sky.SLOT_COUNT - 2
    if slots is None:
        slots = [bytes((i, i, i, 255)) for i in range(stored)]
    out = struct.pack("<I", version) + time(hour, minute) + struct.pack("<I", opcode)
    out += b"".join(slots[:stored])
    out += string(name) + b"".join(string("") for _ in range(sky.NAME_SLOTS - 1))
    out += struct.pack("<4f", 2.2, 2.0, light, weather)
    if version == 3:
        out += struct.pack("<I", len(effects)) + b"".join(string(e) for e in effects)
    return out


def section(day: tuple[int, int], frames: list[bytes]) -> bytes:
    return (struct.pack("<2I", sky.SECTION_VERSION, len(frames))
            + time(*sky.SECTION_END, tail=(1, 0xDEADBEEF))
            + time(*day, tail=(1, 0xDEADBEEF)) + b"".join(frames))


def ske(sections: list[bytes], start: tuple[int, int, int] = (0, 1, 30), flag: int = 1) -> bytes:
    section_no, hour, minute = start
    return (struct.pack("<3I", sky.MAGIC, sky.VERSION, len(sections)) + b"".join(sections)
            + time(hour, minute, section_no) + struct.pack("<2I", 0, flag))


DAY = [
    keyframe(0, 0),
    keyframe(0, 30, 0, "sun", light=0.2),
    keyframe(4, 40, 3, effects=("atm_rain1.wav", "", "", ""), light=1.5, weather=0.8),
    keyframe(9, 0, 4, light=1.5),
    keyframe(14, 30, 1, "sun", light=0.2),
    keyframe(15, 30, 0, "moon", light=0.2),
    keyframe(23, 30, 1, "moon", light=0.2),
    keyframe(24, 0),
]


def test_the_layout_the_deserialiser_reads():
    data = ske([section((0, 15), DAY)])
    a = sky.parse(data)
    assert a.sections == 1 and len(a) == len(DAY)
    assert data[: sky.HEADER_SIZE] == a.header
    assert a.day == (0, 15) and a.day_seconds == 900
    # Bytes 64 and 68 are still the first day's length.
    assert struct.unpack_from("<2I", data, sky.DAY_LENGTH_AT) == (0, 15)
    assert a.section_headers[0].end.hour == 23 and a.section_headers[0].end.minute == 59
    assert (a.start.section, a.start.hour, a.start.minute) == (0, 1, 30)
    assert a.sky_flag == 1 and a.trailer_word == 0
    sun = a.keyframes[1]
    assert (sun.hour, sun.minute, sun.opcode, sun.name) == (0, 30, 0, "sun")
    assert sun.body == "sun" and sun.event == (0, 0)
    rain = a.keyframes[2]
    assert rain.weather == "rain" and rain.sounds == ["atm_rain1.wav"]
    assert a.keyframes[3].stops == "RAIN"


def test_a_file_that_is_not_consumed_exactly_is_refused():
    data = ske([section((0, 15), DAY)])
    with pytest.raises(sky.SkyFormatError):
        sky.parse(data + b"\0\0\0\0")
    with pytest.raises(sky.SkyFormatError):
        sky.parse(data[:-4])


def test_older_keyframe_versions():
    two = keyframe(6, 0, version=2)
    b, g, r = 100, 51, 7
    slots = [bytes((0, 0, 0, 255))] * 19 + [bytes((b, g, r, 255))]
    one = keyframe(7, 0, version=1, slots=slots)
    a = sky.parse(ske([section((0, 15), [two, one])]))
    assert a.keyframes[0].effects == [] and len(a.keyframes[0].slots) == sky.SLOT_COUNT
    made = a.keyframes[1].slots[20]
    assert a.keyframes[1].slots[21] == made
    assert made == bytes((30, 15, 2, 0))


def test_a_header_says_how_long_a_day_lasts():
    header = bytearray(sky.HEADER_SIZE)
    struct.pack_into("<2I", header, sky.DAY_LENGTH_AT, 0, 15)
    a = sky.Atmosphere(Path("x.ske"), sky.VERSION, 1, [], bytes(header))
    assert a.day == (0, 15)
    assert a.day_seconds == 900
    # noon falls halfway through a quarter-hour day
    assert a.real_seconds(12, 0) == 450


def test_a_static_sky_declares_a_full_day():
    header = bytearray(sky.HEADER_SIZE)
    struct.pack_into("<2I", header, sky.DAY_LENGTH_AT, *sky.STATIC_DAY)
    a = sky.Atmosphere(Path("x.ske"), sky.VERSION, 1, [], bytes(header))
    assert a.day_seconds == sky.CLOCK_DAY
    # keeping real time, a clock stamp maps to itself
    assert a.real_seconds(12, 0) == 12 * 3600


def test_a_header_too_short_to_hold_it_reads_zero():
    a = sky.Atmosphere(Path("x.ske"), sky.VERSION, 1, [], b"")
    assert a.day == (0, 0) and a.day_seconds == 0


def test_the_clock_starts_at_the_closing_time_and_runs_the_sections_in_turn():
    second = [keyframe(0, 0), keyframe(12, 0, 5), keyframe(18, 0, 6), keyframe(24, 0)]
    a = sky.parse(ske([section((0, 15), DAY), section((0, 20), second)]))
    assert a.cycle_seconds == 900 + 1200
    # 01:30 of a 900-second day, in whole seconds as the engine keeps it
    assert a.start_position == (0, 56)
    assert a.start_offset == 56
    assert a.clock(0) == (0, 1, 29)
    # past the end of day 0 the clock is in day 1, and after that day 0 again
    assert a.position(900 - 56 + 10) == (1, 10)
    assert a.position(900 + 1200 - 56 + 5)[0] == 0
    # the difference runs forward and wraps round the whole cycle
    assert a.between((1, 1000), (0, 100)) == 200 + 100
    assert a.between((0, 800), (1, 50)) == 100 + 50


def test_events_fire_in_a_half_open_window_split_at_the_section_end():
    a = sky.parse(ske([section((0, 15), DAY)]))
    fired = a.events_between((0, 0), a.start_position)
    # at load the clock runs from 00:00 to the start: the sun's start is in it
    assert [e.kind for e in fired] == ["sun"]
    wrapped = a.events_between((0, 890), (0, 20))
    assert [e.keyframe.minutes for e in wrapped] == [30]


def test_a_body_lives_until_the_next_stop_and_the_weather_until_its_own():
    a = sky.parse(ske([section((0, 15), DAY)]))
    starts = [e for e in a.events() if e.phase == 0 and e.type == 0]
    sun, moon = starts
    assert a.lifetime(sun) == a.real_seconds(14, 30) - a.real_seconds(0, 30)
    assert a.lifetime(moon) == a.real_seconds(23, 30) - a.real_seconds(15, 30)
    windows = [(kind, s.minutes, t.minutes if t else None) for kind, s, t in a.windows()]
    assert windows == [("sun", 30, 870), ("rain", 280, 540), ("moon", 930, 1410)]


def test_a_stop_at_the_end_of_the_cycle_does_not_count():
    frames = [keyframe(0, 0, 0, "sun"), keyframe(20, 0, 1, "sun"),
              keyframe(22, 0, 0, "sun"), keyframe(24, 0, 1, "sun")]
    a = sky.parse(ske([section((24, 0), frames)]))
    first, late = [e for e in a.events() if e.phase == 0]
    assert a.lifetime(first) == 20 * 3600
    assert a.lifetime(late) is None


def colour_keyframe(colours: dict[int, tuple[int, int, int]], fog_end: float = 0.6) -> sky.Keyframe:
    slots = [bytes(4)] * sky.SLOT_COUNT
    for slot, (r, g, b) in colours.items():
        slots[slot] = bytes((b, g, r, 255))
    slots[sky.FOG_START_SLOT] = struct.pack("<f", 0.0)
    slots[sky.FOG_END_SLOT] = struct.pack("<f", fog_end)
    return sky.Keyframe(hour=12, minute=0, slots=slots, name="")


def test_the_fog_ends_at_seven_hundred_times_its_slot():
    k = colour_keyframe({}, fog_end=0.6)
    assert k.fog_start == 0.0
    assert abs(k.fog_end - 420.0) < 1e-3


def test_the_fog_colour_is_the_horizon_the_camera_faces():
    north, east, south, west = (100, 0, 0), (0, 100, 0), (0, 0, 100), (50, 50, 50)
    k = colour_keyframe(dict(zip(sky.HORIZON_SLOTS, (north, east, south, west), strict=True)))
    assert k.fog_colour(0.0) == (100, 0, 0, 255)
    assert k.fog_colour(math.pi / 4) == (50, 50, 0, 255)
    assert k.fog_colour(math.pi) == (0, 0, 100, 255)


def test_the_cloud_colour_and_the_sun_light():
    k = colour_keyframe({sky.CLOUD_SLOT: (10, 20, 30), sky.SUN_LIGHT_SLOT: (255, 0, 51)})
    k.intensity = (2.2, 2.0, 2.0, 0.0)
    assert k.cloud_colour == (10, 20, 30, 255)
    assert k.sun_light == (2.0, 0.0, 0.4)


def test_the_dome_is_a_cap_with_its_rim_at_eye_height():
    points = sky.dome()
    assert points[0] == (0.0, 0.0, sky.DOME_HEIGHT)
    assert len(points) == 1 + sky.DOME_RINGS * sky.DOME_SEGMENTS
    rim = points[-sky.DOME_SEGMENTS:]
    assert all(abs(z) < 1e-6 for _x, _y, z in rim)
    assert abs(math.hypot(rim[0][0], rim[0][1]) - 24142.1) < 0.1
