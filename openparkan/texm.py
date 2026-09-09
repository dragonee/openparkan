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

**A ``Page`` chunk may follow the pixel data**: the magic ``'Page'``, a
``uint32`` count, and then that many 8-byte rectangles of four ``uint16`` in
the order ``(x, width, y, height)``.  It is the texture's own list of
sub-images, and a material's cell byte indexes it.  ``SUN.0``'s four entries
are the four quadrants of a 256 x 256 sheet in the order top-left, top-right,
bottom-left, bottom-right, which is why ``ENV_SUN`` asks for cell 0 and
``ENV_MOON`` for cell 2.  The rectangles are not a grid: ``EFFECT6.0`` names
four 128 x 32 strips, eight 64 x 64 tiles, eight 30 x 30 discs and five 16 x 16
icons in one table of 26.

All 61 textures a material indexes carry one, and every cell asked for is
inside its table.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

HEADER = struct.Struct("<4sIIIIIII")
HEADER_SIZE = HEADER.size  # 32
PALETTE_SIZE = 256 * 4

#: The sub-image table that may follow the pixel data.
PAGE_MAGIC = b"Page"
PAGE_HEADER = 8
PAGE_STRIDE = 8

FMT_PALETTE8 = 0
#: 8-bit indices into an **external** palette rather than an embedded one.
#: The only texture that uses it is the font atlas inside ``ARIALTEX.TFT``,
#: whose palette is ``gamefont.rlb``'s ``PAL.PAL``.
FMT_INDEX8 = 2
FMT_RGB565 = 565
FMT_ARGB4444 = 4444
FMT_XRGB8888 = 888
FMT_ARGB8888 = 8888

_BYTES_PER_PIXEL = {
    FMT_PALETTE8: 1,
    FMT_INDEX8: 1,
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

    #: The texture's own sub-images, from its ``Page`` chunk, as
    #: ``(x, y, width, height)`` in pixels.  Empty when it has none.
    pages: list[tuple[int, int, int, int]] = field(default_factory=list)

    @property
    def has_alpha(self) -> bool:
        return self.fmt in (FMT_ARGB8888, FMT_ARGB4444)

    def page_uv(self, cell: int) -> tuple[float, float, float, float] | None:
        """``(u0, v0, u1, v1)`` of one sub-image, or None if there is no such cell."""
        if not 0 <= cell < len(self.pages):
            return None
        x, y, w, h = self.pages[cell]
        return (x / self.width, y / self.height,
                (x + w) / self.width, (y + h) / self.height)


def mip_pyramid_pixels(width: int, height: int, levels: int) -> int:
    """Total pixel count across ``levels`` mip levels, halving and clamping at 1."""
    return sum(max(width >> i, 1) * max(height >> i, 1) for i in range(levels))


def parse_pages(data: bytes) -> list[tuple[int, int, int, int]]:
    """The sub-images a Texm declares, as ``(x, y, width, height)``.

    The chunk sits after the mip pyramid, so finding it means knowing how long
    that is; a texture with a short mip tail simply has no chunk to find.
    """
    w, h, mips, _flags, fmt = parse_header(data)
    if fmt not in _BYTES_PER_PIXEL:
        return []
    end = HEADER_SIZE + mip_pyramid_pixels(w, h, mips) * _BYTES_PER_PIXEL[fmt]
    if fmt == FMT_PALETTE8:
        end += PALETTE_SIZE
    if end + PAGE_HEADER > len(data) or data[end : end + 4] != PAGE_MAGIC:
        return []
    count = struct.unpack_from("<I", data, end + 4)[0]
    if end + PAGE_HEADER + count * PAGE_STRIDE > len(data):
        return []
    out = []
    for i in range(count):
        x, width, y, height = struct.unpack_from(
            "<4H", data, end + PAGE_HEADER + i * PAGE_STRIDE
        )
        out.append((x, y, width, height))
    return out


def parse_header(data: bytes) -> tuple:
    magic, w, h, mips, flags, u5, u6, fmt = HEADER.unpack_from(data, 0)
    if magic != b"Texm":
        raise UnsupportedTexture(f"not a Texm blob (magic {magic!r})")
    return w, h, mips, flags, fmt


def decode(data: bytes, palette: bytes | None = None) -> Texture:
    """Decode mip level 0 of a Texm blob to straight RGBA8888.

    Only level 0 is decoded: it is the only level a renderer needs to import,
    and it sits at a known offset regardless of how the tail of the mip chain
    is padded.

    ``palette`` supplies the 1024-byte table for ``FMT_INDEX8``, which carries
    none of its own; without one those textures come out as a grey ramp.
    """
    w, h, mips, flags, fmt = parse_header(data)
    if fmt not in _BYTES_PER_PIXEL:
        raise UnsupportedTexture(f"unknown pixel format {fmt!r}")
    body = data[HEADER_SIZE:]

    if fmt == FMT_PALETTE8:
        palette = body[:PALETTE_SIZE]
        body = body[PALETTE_SIZE:]
        if len(palette) < PALETTE_SIZE:
            raise UnsupportedTexture("truncated palette")
    elif fmt == FMT_INDEX8 and palette is None:
        palette = bytes(v for i in range(256) for v in (i, i, i, 0))
    elif fmt != FMT_INDEX8:
        palette = None

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
    else:  # palettised, embedded (format 0) or external (format 2)
        for i in range(w * h):
            j = body[i] * 4
            out[i * 4 : i * 4 + 4] = bytes((palette[j + 2], palette[j + 1], palette[j], 255))
    return Texture(w, h, mips, fmt, flags, bytes(out), parse_pages(data))


def drop_alpha(tex: Texture) -> bytes:
    """The colour channels alone, with the alpha channel ignored.

    Most of the alpha in this game is not transparency.  201 of the 237
    ARGB8888 textures carry a *continuous* alpha field -- ``S0A1.0`` has not a
    single pixel at 0 or 255 -- which is a gloss or self-illumination map, and
    compositing it would wash the colour out.  See ``is_cutout``.
    """
    src = tex.rgba
    out = bytearray(tex.width * tex.height * 3)
    for i in range(tex.width * tex.height):
        out[i * 3 : i * 3 + 3] = src[i * 4 : i * 4 + 3]
    return bytes(out)


#: A silhouette needs a real hole, and its transition band is thin.
CUTOUT_MIN_TRANSPARENT = 0.05
CUTOUT_MAX_SOFT = 0.25


def is_cutout(tex: Texture) -> bool:
    """Whether this texture's alpha is a cut silhouette rather than a map.

    A silhouette is nearly binary: a real fully-transparent region and only a
    thin band between.  ``FTREE1.0`` is 38% transparent with a 4.8% band -- a
    white blob on black, the crown of a tree.  A gloss map is a continuous
    greyscale image of the surface: ``MTP_01.0`` has 75% of its pixels between
    the extremes and ``S0A1.0`` has 100%, and alpha-testing either punches
    holes through solid machinery.
    """
    alpha = tex.rgba[3::4]
    n = len(alpha) or 1
    transparent = sum(1 for v in alpha if v == 0) / n
    soft = sum(1 for v in alpha if 8 < v < 247) / n
    return transparent > CUTOUT_MIN_TRANSPARENT and soft < CUTOUT_MAX_SOFT


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
