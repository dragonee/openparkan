"""behpsp.res behaviour profiles, on bytes this file builds itself."""

from __future__ import annotations

import struct

import pytest

from openparkan import profiles


def record(name: str, typ: int, value, default=None, low=None, high=None) -> bytes:
    raw = name.encode()
    fmt = "<4f" if typ == profiles.TYPE_FLOAT else "<4i"
    default = value if default is None else default
    low = (-3.4e38 if typ == profiles.TYPE_FLOAT else -(2**31)) if low is None else low
    high = (3.4e38 if typ == profiles.TYPE_FLOAT else 2**31 - 1) if high is None else high
    return (struct.pack("<4I", profiles.VALUE_SIZE, typ, 0, len(raw)) + raw
            + struct.pack("<I", 0) + struct.pack(fmt, value, default, low, high))


def build(*records: bytes) -> bytes:
    return struct.pack("<I", len(records)) + b"".join(records)


def test_a_profile_reads_its_variables_in_order():
    blob = build(record("Type", profiles.TYPE_DWORD, 7),
                 record("CanMove", profiles.TYPE_BOOL, 1),
                 record(profiles.POWER_OUT, profiles.TYPE_FLOAT, 10.0))
    got = profiles.parse(blob)
    assert list(got) == ["Type", "CanMove", profiles.POWER_OUT]
    assert got[profiles.POWER_OUT].value == 10.0
    assert got["Type"].type == profiles.TYPE_DWORD


def test_value_default_and_bounds_are_kept_apart():
    blob = build(record(profiles.ORE_MAXIMUM, profiles.TYPE_FLOAT, 4000.0,
                        1000.0, 0.0, 1e6))
    v = profiles.parse(blob)[profiles.ORE_MAXIMUM]
    assert (v.value, v.default, v.minimum, v.maximum) == (4000.0, 1000.0, 0.0, 1e6)


def test_an_unknown_type_is_refused():
    with pytest.raises(profiles.ProfileFormatError, match="record"):
        profiles.parse(build(record("X", 9, 1)))


def test_trailing_bytes_are_refused():
    with pytest.raises(profiles.ProfileFormatError, match="left over"):
        profiles.parse(build(record("X", profiles.TYPE_BOOL, 1)) + b"\0")


def test_a_record_running_past_the_end_is_refused():
    blob = build(record("Use_Power", profiles.TYPE_FLOAT, 3.0))
    with pytest.raises(profiles.ProfileFormatError, match="past the end"):
        profiles.parse(blob[:-6])
