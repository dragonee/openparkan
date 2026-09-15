"""The command mode's cursors: RIFF ACON animated cursors and ``ui/cursor.cfg``."""

from __future__ import annotations

import struct

import pytest

from openparkan import cursors


def cur(hotspot=(7, 7), size=32, bits=4) -> bytes:
    """A one-image .cur: its directory, then a BITMAPINFOHEADER and no pixels."""
    directory = struct.pack("<3H", 0, 2, 1) + struct.pack(
        "<4B2H2I", size, size, 16, 0, hotspot[0], hotspot[1], 40, 22)
    info = struct.pack("<IiiHHI", 40, size, size * 2, 1, bits, 0) + bytes(20)
    return directory + info


def chunk(tag: bytes, body: bytes) -> bytes:
    return tag + struct.pack("<I", len(body)) + body + (b"\0" if len(body) & 1 else b"")


def ani(frames: int, steps: int, jiffies=9, rate=None, seq=None, hotspot=(7, 7)) -> bytes:
    flags = 1 | (2 if seq else 0)
    body = b"ACON" + chunk(b"anih", struct.pack("<9I", 36, frames, steps, 0, 0, 4, 1,
                                                 jiffies, flags))
    if rate:
        body += chunk(b"rate", struct.pack(f"<{len(rate)}I", *rate))
    if seq:
        body += chunk(b"seq ", struct.pack(f"<{len(seq)}I", *seq))
    fram = b"fram" + b"".join(chunk(b"icon", cur(hotspot)) for _ in range(frames))
    body += chunk(b"LIST", fram)
    return b"RIFF" + struct.pack("<I", len(body)) + body


def test_four_frames_in_order_at_the_header_rate():
    a = cursors.parse_ani(ani(4, 4, hotspot=(1, 1)))
    assert (a.frame_count, a.step_count, a.flags) == (4, 4, 1)
    assert a.sequence == [0, 1, 2, 3]
    assert a.rates == [9, 9, 9, 9]
    assert a.step_ms(0) == pytest.approx(150.0)
    assert [f.hotspot for f in a.frames] == [(1, 1)] * 4
    assert a.frames[0].resource_type == 2 and a.frames[0].bit_count == 4
    assert (a.frames[0].width, a.frames[0].height) == (32, 32)


def test_three_frames_played_back_and_forth():
    a = cursors.parse_ani(ani(3, 4, rate=[9, 9, 9, 9], seq=[0, 1, 2, 1]))
    assert a.flags == 3
    assert a.sequence == [0, 1, 2, 1]
    assert len(a.frames) == 3


def test_a_frame_count_the_list_does_not_hold_is_refused():
    blob = bytearray(ani(4, 4))
    at = blob.index(b"anih") + 8 + 4
    blob[at:at + 4] = struct.pack("<I", 5)
    with pytest.raises(cursors.CursorFormatError):
        cursors.parse_ani(bytes(blob))


def test_not_an_animated_cursor():
    with pytest.raises(cursors.CursorFormatError):
        cursors.parse_ani(b"RIFF\0\0\0\0WAVE")


def test_cursor_cfg_reads_every_key(tmp_path):
    path = tmp_path / "cursor.cfg"
    path.write_bytes(
        b'object\tPLACE\r\n HARDWARE_CURSOR\t= "ui/place.ani"\r\n TEXTURE\t= "new_ui1"\r\n'
        b" OFFSET_X\t= 192\r\n OFFSET_Y\t= 16\r\n WIDTH\t= 16\r\n HEIGHT\t= 16\r\n"
        b" EXTENT_X\t= 8\r\n EXTENT_Y\t= 8\r\n PHASE_DELAY\t= 150\r\nend\r\n")
    (spec,) = cursors.cursor_cfg(path)
    assert spec.name == "PLACE" and spec.hardware == "ui/place.ani"
    assert (spec.texture, spec.offset, spec.size, spec.extent, spec.phase_delay_ms) == (
        "new_ui1", (192, 16), (16, 16), (8, 8), 150)
