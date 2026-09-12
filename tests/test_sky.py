"""The .ske day cycle, on headers this file builds itself."""

from __future__ import annotations

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
