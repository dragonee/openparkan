"""Readers for ``gamefont.rlb`` -- the game's font and its palette.

The archive holds two members, both ``0x040`` LZSS, and until that was
decoded neither could be read at all; see ``openparkan.rsli``.

**``ARIALTEX.TFT``** is a ``Tfnt``: a 20-byte header, then 256 glyph records
of 16 bytes, then an ordinary ``Texm`` at offset 4116::

    +0   char[4]  'Tfnt'
    +4   int32[4] unread
    +20  256 x    float32 u0, float32 u1, float32 v0, int32 advance
    4116 Texm     128 x 128, pixel format 2

A record gives a glyph's left and right edge and its top edge as texture
coordinates, and how far the pen moves after drawing it.  The three agree:
on **123 of the 256** records the span ``(u1 - u0) * 128`` is exactly
``advance + 1``, one pixel of bearing -- on all 123 of them, not merely most.
The other 133 are the placeholder, a span of 0.00078 (a fifth of a pixel) with
an advance of 8, for a code point the font does not draw.  Every one of the
256 records has ``u0 <= u1``.

The glyphs sit in **seven rows 18 pixels apart**, at v = 0, 18, 36 ... 108,
which is what a 128-pixel atlas holding Latin and Cyrillic needs.

**Pixel format 2** appears nowhere else.  It is one byte per pixel -- 128 x
128 is exactly the 16384 bytes between the Texm header and the end of the
member -- and it indexes an *external* palette rather than carrying one, which
is what the other member is for.

**``PAL.PAL``** is that palette and a blend table::

    +0     uint8[1024]  256 BGRA entries, the fourth byte always zero
    +1024  char[4]      'Ipol'
    +1028  uint8[65536] a 256 x 256 table

241 of the 256 palette entries are non-zero and the fourth byte is zero on all
256, which is the convention the palettised ``Texm`` format uses too.  The
font's glyph pixels are all index 73, and index 73 is ``(255, 255, 255)`` --
the glyphs are white, and the engine tints them.

The table earns its name.  It is **symmetric on all 65536 cells** --
``table[a][b] == table[b][a]`` -- and ``table[i][i] == i`` on 237 of the 256
diagonal cells, the other 19 being indices the palette never uses.  That is an
interpolation table: given two palette indices it returns the index of their
mixture, which is how an 8-bit renderer blends.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass

MAGIC = b"Tfnt"
HEADER_SIZE = 20
GLYPH_COUNT = 256
GLYPH_STRIDE = 16
#: Where the atlas begins: ``HEADER_SIZE + GLYPH_COUNT * GLYPH_STRIDE``.
ATLAS_AT = HEADER_SIZE + GLYPH_COUNT * GLYPH_STRIDE

#: ``PAL.PAL``: a 1024-byte palette, a tag, then the table.
PALETTE_SIZE = 1024
PALETTE_TAG = b"Ipol"
TABLE_SIDE = 256
PAL_SIZE = PALETTE_SIZE + len(PALETTE_TAG) + TABLE_SIDE * TABLE_SIDE

#: The advance a record carries when it draws nothing.
BLANK_ADVANCE = 8

#: A placeholder record is not quite empty -- it spans 0.00078, a fifth of a
#: pixel -- while the narrowest real glyph spans 0.015625, two pixels.  Half a
#: pixel separates them cleanly.
MIN_SPAN = 1.0 / 256


class FontFormatError(ValueError):
    pass


@dataclass(frozen=True)
class Glyph:
    """One code point's place in the atlas, and its advance."""

    u0: float
    u1: float
    v0: float
    #: How far the pen moves, in pixels.
    advance: int

    @property
    def drawn(self) -> bool:
        return self.u1 - self.u0 > MIN_SPAN

    def width(self, atlas: int) -> int:
        """The glyph's span in pixels, given the atlas width."""
        return round((self.u1 - self.u0) * atlas)


@dataclass
class Font:
    """``ARIALTEX.TFT``: 256 glyph records and the atlas they index."""

    header: bytes
    glyphs: list[Glyph]
    #: The embedded ``Texm`` blob, ready for ``texm.decode``.
    atlas: bytes

    @property
    def drawn(self) -> list[int]:
        """Code points the font actually draws."""
        return [i for i, g in enumerate(self.glyphs) if g.drawn]

    @property
    def rows(self) -> list[float]:
        """The distinct row tops, in texture coordinates."""
        return sorted({g.v0 for g in self.glyphs if g.drawn})


@dataclass
class Palette:
    """``PAL.PAL``: 256 colours and the table that blends them."""

    #: ``(r, g, b)`` per index.
    colours: list[tuple[int, int, int]]
    #: The raw 1024 bytes, for handing to ``texm.decode``.
    raw: bytes
    #: ``blend[a * 256 + b]`` is the index of the mixture of ``a`` and ``b``.
    blend: bytes

    def mix(self, a: int, b: int) -> int:
        return self.blend[a * TABLE_SIDE + b]


def parse_font(blob: bytes) -> Font:
    """Parse a ``Tfnt``.  Raises unless the atlas is where the records end."""
    if len(blob) < ATLAS_AT + 4 or blob[:4] != MAGIC:
        raise FontFormatError(f"not a Tfnt blob (magic {blob[:4]!r})")
    glyphs = [
        Glyph(*struct.unpack_from("<3fi", blob, HEADER_SIZE + i * GLYPH_STRIDE))
        for i in range(GLYPH_COUNT)
    ]
    if blob[ATLAS_AT : ATLAS_AT + 4] != b"Texm":
        raise FontFormatError(
            f"no Texm at {ATLAS_AT}, found {blob[ATLAS_AT:ATLAS_AT + 4]!r}"
        )
    return Font(blob[:HEADER_SIZE], glyphs, blob[ATLAS_AT:])


def parse_palette(blob: bytes) -> Palette:
    """Parse a ``PAL.PAL``.  Raises unless the ``Ipol`` tag is in place."""
    if len(blob) < PAL_SIZE:
        raise FontFormatError(f"palette is {len(blob)} bytes, want {PAL_SIZE}")
    at = PALETTE_SIZE
    if blob[at : at + 4] != PALETTE_TAG:
        raise FontFormatError(f"no {PALETTE_TAG!r} tag, found {blob[at:at + 4]!r}")
    raw = blob[:PALETTE_SIZE]
    colours = [(raw[i * 4 + 2], raw[i * 4 + 1], raw[i * 4]) for i in range(256)]
    return Palette(colours, raw, blob[at + 4 :])
