"""Readers for the command mode's cursors: ``ui/cursor.cfg`` and its ``.ani`` files.

``ui/cursor.cfg`` names eight cursors, ``ARROW`` to ``UPGRADE``.  Each gives a
``HARDWARE_CURSOR`` -- a Windows animated cursor ``iron3d.dll`` loads with
``LoadCursorFromFileA`` -- and the same cursor as a software strip: four phases
``WIDTH`` wide stepping right from ``OFFSET_X``, ``OFFSET_Y`` on ``TEXTURE``,
with ``EXTENT_X``, ``EXTENT_Y`` as the hot spot and ``PHASE_DELAY`` milliseconds
a phase.  See ``docs/42-selection.md``.

An ``.ani`` is a RIFF ``ACON`` file::

    'anih'  36 bytes: size, frames, steps, width, height, bit count, planes,
            jiffies a step (1/60 s), flags (1 icon frames, 2 a 'seq ' chunk)
    'rate'  one jiffy count a step            (optional)
    'seq '  one frame index a step            (optional)
    LIST 'fram'  one 'icon' chunk a frame, each a whole .ico/.cur file

A ``.cur`` is an ``.ico`` of type 2 whose directory entry carries the hot spot
where an icon keeps its planes and bit count.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from . import mission


class CursorFormatError(ValueError):
    """The bytes are not an animated cursor this reader understands."""


@dataclass
class Frame:
    """One frame: its size, hot spot and bit depth, from its ``.cur`` file."""

    width: int
    height: int
    hotspot: tuple[int, int]
    bit_count: int
    #: 2 for a cursor, 1 for an icon.
    resource_type: int


@dataclass
class AnimatedCursor:
    """An ``.ani``: its header, its per-step timing and order, and its frames."""

    frame_count: int
    step_count: int
    #: The header's default jiffies (1/60 s) a step.
    jiffies: int
    flags: int
    frames: list[Frame]
    #: Jiffies for each step, from ``rate``; the header's value when absent.
    rates: list[int] = field(default_factory=list)
    #: The frame shown at each step, from ``seq ``; frames in order when absent.
    sequence: list[int] = field(default_factory=list)

    def step_ms(self, step: int) -> float:
        """How long a step lasts, in milliseconds."""
        return self.rates[step] * 1000.0 / 60.0


def _cur(blob: bytes) -> Frame:
    if len(blob) < 22:
        raise CursorFormatError("icon chunk shorter than its directory")
    reserved, kind, count = struct.unpack_from("<3H", blob, 0)
    if reserved != 0 or count < 1:
        raise CursorFormatError("icon chunk is not an .ico/.cur")
    w, h, _colours, _res, hx, hy, _size, offset = struct.unpack_from("<4B2H2I", blob, 6)
    bit_count = 0
    if offset + 16 <= len(blob):
        bit_count = struct.unpack_from("<H", blob, offset + 14)[0]
    return Frame(w or 256, h or 256, (hx, hy), bit_count, kind)


def parse_ani(data: bytes) -> AnimatedCursor:
    """Read an ``.ani`` animated cursor."""
    if len(data) < 12 or data[:4] != b"RIFF" or data[8:12] != b"ACON":
        raise CursorFormatError("not a RIFF ACON file")
    end = min(len(data), 8 + struct.unpack_from("<I", data, 4)[0])
    header = None
    rates: list[int] = []
    sequence: list[int] = []
    frames: list[Frame] = []

    def walk(pos: int, stop: int) -> None:
        nonlocal header, rates, sequence
        while pos + 8 <= stop:
            tag = data[pos:pos + 4]
            size = struct.unpack_from("<I", data, pos + 4)[0]
            body = pos + 8
            if body + size > stop:
                raise CursorFormatError(f"chunk {tag!r} runs past its parent")
            if tag == b"LIST":
                walk(body + 4, body + size)
            elif tag == b"anih":
                if size < 36:
                    raise CursorFormatError("anih shorter than 36 bytes")
                header = struct.unpack_from("<9I", data, body)
            elif tag == b"rate":
                rates = list(struct.unpack_from(f"<{size // 4}I", data, body))
            elif tag == b"seq ":
                sequence = list(struct.unpack_from(f"<{size // 4}I", data, body))
            elif tag == b"icon":
                frames.append(_cur(data[body:body + size]))
            pos = body + size + (size & 1)

    walk(12, end)
    if header is None:
        raise CursorFormatError("no anih chunk")
    _size, frame_count, step_count, _w, _h, _bits, _planes, jiffies, flags = header
    if len(frames) != frame_count:
        raise CursorFormatError(f"anih says {frame_count} frames, found {len(frames)}")
    return AnimatedCursor(
        frame_count=frame_count, step_count=step_count, jiffies=jiffies, flags=flags,
        frames=frames,
        rates=rates or [jiffies] * step_count,
        sequence=sequence or list(range(step_count)))


@dataclass
class CursorSpec:
    """One object of ``ui/cursor.cfg``."""

    name: str
    hardware: str
    texture: str
    offset: tuple[int, int]
    size: tuple[int, int]
    extent: tuple[int, int]
    phase_delay_ms: int


def cursor_cfg(path: str | Path) -> list[CursorSpec]:
    """``ui/cursor.cfg``'s objects in file order."""
    out = []
    for name, props in mission.load_cfg(path).items():
        p = {k.upper(): v for k, v in props.items()}
        out.append(CursorSpec(
            name=name,
            hardware=p.get("HARDWARE_CURSOR", ""),
            texture=p.get("TEXTURE", ""),
            offset=(int(p.get("OFFSET_X", 0)), int(p.get("OFFSET_Y", 0))),
            size=(int(p.get("WIDTH", 0)), int(p.get("HEIGHT", 0))),
            extent=(int(p.get("EXTENT_X", 0)), int(p.get("EXTENT_Y", 0))),
            phase_delay_ms=int(p.get("PHASE_DELAY", 250))))
    return out
