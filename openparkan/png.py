"""Minimal PNG writer (stdlib only, no Pillow).

Keeping the toolkit dependency-free matters here: the whole point is that a
contributor can clone the repo, point it at a Steam install and get pictures
out without a build step.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path


def _chunk(tag: bytes, payload: bytes) -> bytes:
    body = tag + payload
    return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def write_png(
    path: str | Path, width: int, height: int, pixels: bytes, alpha: bool = False
) -> None:
    """Write RGB (3 bytes/px) or RGBA (4 bytes/px) pixel data as a PNG."""
    stride = 4 if alpha else 3
    expected = width * height * stride
    if len(pixels) != expected:
        raise ValueError(f"expected {expected} bytes of pixel data, got {len(pixels)}")
    raw = bytearray()
    row = width * stride
    for y in range(height):
        raw.append(0)  # filter type 0 (None)
        raw += pixels[y * row : (y + 1) * row]
    header = struct.pack(">IIBBBBB", width, height, 8, 6 if alpha else 2, 0, 0, 0)
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", header)
        + _chunk(b"IDAT", zlib.compress(bytes(raw), 6))
        + _chunk(b"IEND", b"")
    )
    Path(path).write_bytes(blob)
