"""The .ske day cycle, on headers this file builds itself."""

from __future__ import annotations

import math
import struct
from pathlib import Path

from openparkan import sky


def test_a_header_says_how_long_a_day_lasts():
    """Bytes 64 and 68 are hours then minutes of real time."""
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


def keyframe(colours: dict[int, tuple[int, int, int]], fog_end: float = 0.6) -> sky.Keyframe:
    slots = [bytes(4)] * sky.SLOT_COUNT
    for slot, (r, g, b) in colours.items():
        slots[slot] = bytes((b, g, r, 255))
    slots[sky.FOG_START_SLOT] = struct.pack("<f", 0.0)
    slots[sky.FOG_END_SLOT] = struct.pack("<f", fog_end)
    return sky.Keyframe(hour=12, minute=0, slots=slots, name="")


def test_the_fog_ends_at_seven_hundred_times_its_slot():
    k = keyframe({}, fog_end=0.6)
    assert k.fog_start == 0.0
    assert abs(k.fog_end - 420.0) < 1e-3


def test_the_fog_colour_is_the_horizon_the_camera_faces():
    north, east, south, west = (100, 0, 0), (0, 100, 0), (0, 0, 100), (50, 50, 50)
    k = keyframe(dict(zip(sky.HORIZON_SLOTS, (north, east, south, west), strict=True)))
    assert k.fog_colour(0.0) == (100, 0, 0, 255)
    assert k.fog_colour(math.pi / 4) == (50, 50, 0, 255)
    assert k.fog_colour(math.pi) == (0, 0, 100, 255)


def test_the_dome_is_a_cap_with_its_rim_at_eye_height():
    points = sky.dome()
    assert points[0] == (0.0, 0.0, sky.DOME_HEIGHT)
    assert len(points) == 1 + sky.DOME_RINGS * sky.DOME_SEGMENTS
    rim = points[-sky.DOME_SEGMENTS:]
    assert all(abs(z) < 1e-6 for _x, _y, z in rim)
    assert abs(math.hypot(rim[0][0], rim[0][1]) - 24142.1) < 0.1
