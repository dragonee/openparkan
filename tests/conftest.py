"""Fixtures built in code.

The repository ships no game data, so every test here constructs the bytes it
needs.  That is the point: a contributor without a copy of the game can still
run these and find out whether a reader was broken.  The claims about what the
*shipped* files contain are checked separately by ``openparkan verify``, which
does need an installation.
"""

from __future__ import annotations

import struct

import pytest

from openparkan import control, nres


def build_nres(members: list[tuple[str, str, bytes]]) -> bytes:
    """Build an NRes archive from ``(tag, name, payload)`` triples."""
    payloads = bytearray()
    placed = []
    for tag, name, blob in members:
        placed.append((tag, name, len(blob), nres.HEADER_SIZE + len(payloads)))
        payloads += blob

    directory = bytearray()
    for index, (tag, name, size, offset) in enumerate(placed):
        rec = bytearray(nres.ENTRY_SIZE)
        rec[0:4] = tag.encode("latin-1").ljust(4, b" ")
        struct.pack_into("<I", rec, 4, 0)          # element count
        struct.pack_into("<I", rec, 8, 0)          # link count
        struct.pack_into("<I", rec, 12, size)
        rec[20:52] = name.encode("latin-1").ljust(32, b"\0")
        struct.pack_into("<I", rec, 56, offset)
        struct.pack_into("<I", rec, 60, index)
        directory += rec

    total = nres.HEADER_SIZE + len(payloads) + len(directory)
    head = struct.pack("<4sIII", nres.MAGIC, 1, len(placed), total)
    return bytes(head + payloads + directory)


@pytest.fixture
def nres_archive():
    """A factory for NRes archives, so a test can say what it needs."""
    return build_nres


def build_component(type_id: int, library: str = "", member: str = "",
                    entries: tuple[int, ...] = (), label: str = "",
                    index: int = -1, values: tuple[float, ...] = (),
                    power: float = 0.0) -> bytes:
    """One section-4 record, laid out as ``Control.dll`` reads it."""
    rec = bytearray(control.COMPONENT_FIXED)
    struct.pack_into("<i", rec, 0, type_id)
    struct.pack_into("<f", rec, control.COMPONENT_POWER_AT, power)
    struct.pack_into("<i", rec, control.COMPONENT_INDEX_AT, index)
    struct.pack_into(f"<{len(values)}f", rec, control.COMPONENT_VALUES_AT, *values)
    at = control.COMPONENT_NAME_AT
    rec[at:at + 32] = library.encode("latin-1").ljust(32, b"\0")
    rec[at + 32:at + 64] = member.encode("latin-1").ljust(32, b"\0")
    struct.pack_into("<i", rec, control.COMPONENT_COUNT_AT, len(entries))
    out = bytes(rec) + struct.pack(f"<{len(entries)}i", *entries)
    if label:
        out += struct.pack("<i", len(label)) + label.encode("latin-1") + b"\0"
    else:
        out += struct.pack("<i", 0)
    return out


def build_reference(library: str, member: str,
                    values: tuple[int, ...] = (0,) * 9) -> bytes:
    """One section-5 record: nine ints, then the name pair."""
    rec = bytearray(control.REFERENCE_STRIDE)
    struct.pack_into("<9i", rec, 0, *values)
    at = control.REFERENCE_NAME_AT
    rec[at:at + 32] = library.encode("latin-1").ljust(32, b"\0")
    rec[at + 32:at + 64] = member.encode("latin-1").ljust(32, b"\0")
    return bytes(rec)


def build_ctl(counts=(0, 0, 0, 0, 0), params=None, components=(), groups=(),
              block=None) -> bytes:
    """A whole controller: frame, sections, the block, the reference groups."""
    a, b, c, _d, _e = counts
    out = bytearray(struct.pack("<5i", *counts))
    values = params if params is not None else [0.0] * 27
    out += struct.pack("<27f", *values)
    out += bytes(a * (control.SECTION1_RECORD + control.SECTION1_PER_B * b))
    out += bytes(4 * a * a)
    out += bytes(c * control.SECTION2_RECORD)
    for part in components:
        out += part
    out += block if block is not None else bytes([control.UNSET]) * control.BLOCK_SIZE
    for group in groups:
        out += struct.pack("<i", len(group))
        for ref in group:
            out += ref
    return bytes(out)


@pytest.fixture
def ctl():
    return build_ctl


@pytest.fixture
def component():
    return build_component


@pytest.fixture
def reference():
    return build_reference
