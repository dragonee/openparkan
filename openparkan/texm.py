"""Decoder for ``Texm``, the texture format inside Parkan's NRes archives.

Layout (see ``docs/02-texm.md``)::

    0x00  char[4]  'Texm'
    0x04  uint32   width
    0x08  uint32   height
    0x0C  uint32   mip level count
    0x10  uint32   flags       (32 on every mip-mapped texture, 0 otherwise)
    0x14  uint32   unknown     (always 0 in the shipped data)
    0x18  uint32   unknown     (varies; not needed to decode)
    0x1C  uint32   pixel format, spelled as a decimal channel-width literal:
                   8888, 888, 565, 4444, or 0 for 8-bit palettised
    0x20  ...      pixel data, mip 0 first, each level half the previous

Palettised textures put a 256 x BGRX palette immediately after the header and
before the index data.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

HEADER = struct.Struct("<4sIIIIIII")
HEADER_SIZE = HEADER.size  # 32
PALETTE_SIZE = 256 * 4

FMT_PALETTE8 = 0
FMT_RGB565 = 565
FMT_ARGB4444 = 4444
FMT_XRGB8888 = 888
FMT_ARGB8888 = 8888

_BYTES_PER_PIXEL = {
    FMT_PALETTE8: 1,
    FMT_RGB565: 2,
    FMT_ARGB4444: 2,
    FMT_XRGB8888: 4,
    FMT_ARGB8888: 4,
}


class UnsupportedTexture(ValueError):
    pass


@dataclass
class Texture:
    width: int
    height: int
    mips: int
    fmt: int
    flags: int
    rgba: bytes  # mip level 0, 4 bytes per pixel, R G B A

    @property
    def has_alpha(self) -> bool:
        return self.fmt in (FMT_ARGB8888, FMT_ARGB4444)


def mip_pyramid_pixels(width: int, height: int, levels: int) -> int:
    """Total pixel count across ``levels`` mip levels, halving and clamping at 1."""
    return sum(max(width >> i, 1) * max(height >> i, 1) for i in range(levels))


def parse_header(data: bytes) -> tuple:
    magic, w, h, mips, flags, u5, u6, fmt = HEADER.unpack_from(data, 0)
    if magic != b"Texm":
        raise UnsupportedTexture(f"not a Texm blob (magic {magic!r})")
    return w, h, mips, flags, fmt


def decode(data: bytes) -> Texture:
    """Decode mip level 0 of a Texm blob to straight RGBA8888.

    Only level 0 is decoded: it is the only level a renderer needs to import,
    and it sits at a known offset regardless of how the tail of the mip chain
    is padded.
    """
    w, h, mips, flags, fmt = parse_header(data)
    if fmt not in _BYTES_PER_PIXEL:
        raise UnsupportedTexture(f"unknown pixel format {fmt!r}")
    body = data[HEADER_SIZE:]

    palette = None
    if fmt == FMT_PALETTE8:
        palette = body[:PALETTE_SIZE]
        body = body[PALETTE_SIZE:]
        if len(palette) < PALETTE_SIZE:
            raise UnsupportedTexture("truncated palette")

    need = w * h * _BYTES_PER_PIXEL[fmt]
    if len(body) < need:
        raise UnsupportedTexture(
            f"truncated pixel data: need {need} bytes for {w}x{h}, have {len(body)}"
        )

    out = bytearray(w * h * 4)
    if fmt == FMT_RGB565:
        for i in range(w * h):
            v = body[i * 2] | (body[i * 2 + 1] << 8)
            r = (v >> 11) & 0x1F
            g = (v >> 5) & 0x3F
            b = v & 0x1F
            out[i * 4 : i * 4 + 4] = bytes(
                ((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255)
            )
    elif fmt == FMT_ARGB4444:
        for i in range(w * h):
            v = body[i * 2] | (body[i * 2 + 1] << 8)
            a = (v >> 12) & 0xF
            r = (v >> 8) & 0xF
            g = (v >> 4) & 0xF
            b = v & 0xF
            out[i * 4 : i * 4 + 4] = bytes((r * 17, g * 17, b * 17, a * 17))
    elif fmt in (FMT_XRGB8888, FMT_ARGB8888):
        # Stored little-endian ARGB, i.e. B G R A in memory order -- the
        # DirectDraw convention this engine was written against.
        for i in range(w * h):
            b, g, r, a = body[i * 4 : i * 4 + 4]
            out[i * 4 : i * 4 + 4] = bytes((r, g, b, 255 if fmt == FMT_XRGB8888 else a))
    else:  # palettised
        for i in range(w * h):
            j = body[i] * 4
            out[i * 4 : i * 4 + 4] = bytes((palette[j + 2], palette[j + 1], palette[j], 255))
    return Texture(w, h, mips, fmt, flags, bytes(out))


def to_rgb(tex: Texture, background: tuple[int, int, int] = (255, 0, 255)) -> bytes:
    """Flatten RGBA to RGB, compositing transparency over ``background``."""
    src = tex.rgba
    out = bytearray(tex.width * tex.height * 3)
    for i in range(tex.width * tex.height):
        r, g, b, a = src[i * 4 : i * 4 + 4]
        if a == 255:
            out[i * 3 : i * 3 + 3] = bytes((r, g, b))
        else:
            f = a / 255.0
            out[i * 3 : i * 3 + 3] = bytes(
                (
                    int(r * f + background[0] * (1 - f)),
                    int(g * f + background[1] * (1 - f)),
                    int(b * f + background[2] * (1 - f)),
                )
            )
    return bytes(out)
