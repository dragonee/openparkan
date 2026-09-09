"""Reader for ``Material.lib`` -- the ``MAT0`` material database.

905 materials, each naming one or more texture layers.  A model's wear (its
``.wea``) lists material names; a mesh batch picks one of them by index; the
material names the texture.  That completes the chain from a triangle to a
pixel::

    mesh stream 13 batch -> wear entry -> Material.lib MAT0 -> Textures.lib Texm

Terrain works the same way: ``Land1.wea`` and ``Land2.wea`` name *materials*,
not textures, which is why ``WATER``, ``B_S0`` and ``ENV_NLAVA`` cannot be
found in ``Textures.lib`` -- they are here, and they name ``WATER0.0``,
``B_FOUND.0`` and ``LAV00.0``.

A record opens with two uint16: the number of texture entries and the number
of **layers**.  The count divides by the layer count on all 905 records, and
the quotient is a frame count -- ``WATER_M`` is one layer of ten frames
(``WATER0.0`` .. ``WATER9.0``), while ``WATER_BOT`` is two layers of one
(``L20.0`` and ``L20M.0``).  860 materials have a single layer, 43 have two
and two have eight; animation is much the commoner reason for a material to
hold several textures.

The two-layer ones are the ground: a texture in RGB565 and its ``M`` twin in
XRGB8888, holding the same picture flattened towards neutral grey -- see
``docs/03-terrain.md``.  It is **not** a mask, which this reader used to call
it.  The two eight-layer ones, ``B_LBL_01`` and ``R_LBL_01``, are not eight
images at all: both name ``PG27.0`` eight times and ask for cells 0 to 7 of
it, so they are the blue and red team variants of one insignia sheet.

An entry begins 12 bytes in and runs **34** bytes to the next, with the
texture name in a 14-byte field at +20.  34 rather than 40: at 34 the opacity
byte lands on 100 in every entry of 904 of the 905 records, and every other
stride tried collapses to 531 -- which is exactly the number of records
holding a single entry, where a stride cannot be wrong.

Getting that stride right is what makes every material resolve.  The names
used to be extracted by pattern, and the pattern swallowed whatever
alphanumeric byte happened to sit in front of a name: it read ``qqds.7`` out
of ``B_MTP_04``, whose real texture is ``MTP_04.0``, and ``0FAIR.0`` ..
``7FAIR.0`` out of the ``FIRE_SMOKE`` animations, where those digits are the
cell bytes 48..55 of fourteen frames that all name ``FAIR.0``.  Read by
offset, **all 905 materials name a texture that is in Textures.lib**, against
891 by pattern.

At +6 sits a three-byte RGB **diffuse colour** that modulates the texture.  It
matters: ``WATER``'s texture is a neutral grey ripple and the blue is entirely
in its ``#4d6aff``, and lava is a dull red pattern tinted ``#b41e00``.  761
materials carry a colour other than white.

The byte at +5 ahead of it is **not** the constant marker an earlier reading
took it for.  It is an **opacity in percent** -- ``World3D.dll``'s parser
multiplies it by 0.01 -- and it looks constant only because every material but
one is fully opaque.  The exception gives it away: ``FIRESTORM``'s entries run
**0, 60, 80, 90, 95, 100** across its frames, which is a fade-in.  3138 of the
3143 entries are at 100.

The record is **versioned**, and the parser gates its tail on that: at version
2 it reads the two bytes at +4 and +5, at 3 a ``float32`` defaulting to 1.0,
at 4 a ``uint32`` defaulting to 0.  Below each it substitutes the default --
and for the two bytes the default is **0xFF**.  So 0xFF in those fields is the
engine's own "not set", which is what byte 5 holds on all 905 records and byte
4 on 376 of them.  The other 11 values of byte 4 sort the library by role:
0 to 4 hold **43 of the 45 multi-layer materials** and are almost all opaque
ground, 5 is 342 object materials of which every one carries alpha, 6 is the
87 ``TREE*`` and foliage, 7 is the two water materials, and 8 to 10 are three
smaller families.  It reads like a shader or blend mode, but nothing confirms
it.

The byte immediately before the name, at +19, selects a **sub-image**: ``0xFF``
means the whole texture, and anything else is a cell of a sprite sheet.  427
of the 905 materials take the whole texture; the rest index one.  The sky
materials are the clearest case -- ``SUN.0`` is a 2 x 2 sheet holding a sun
corona and a moon, and ``ENV_SUN`` asks for cell 0 while ``ENV_MOON`` asks for
cell 2, which is where the moon is.  ``SUN1.0`` holds four stars and a moon
and its three ``ENV_SUN_*`` materials name cells 0, 1 and 3 -- the three
stars.

The cell is **not** a grid index, which an earlier reading assumed from the
2 x 2 sheets.  It indexes the texture's own ``Page`` table -- a list of
sub-image rectangles appended to the Texm payload, see ``texm.parse_pages``.
That is why ``SUN.0`` behaves like a 2 x 2 grid (its four pages *are* the four
quadrants) while ``EFFECT6.0`` does not: its 26 pages are four 128 x 32
strips, eight 64 x 64 tiles, eight 30 x 30 discs and five 16 x 16 icons.  All
62 textures a material indexes carry a table, and every one of the **2513**
cells asked for -- across every entry of every material, not just the first --
is inside its own.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from .nres import NResArchive

MATERIAL_TAG = "MAT0"

#: An entry's diffuse colour, and the opacity that anchors the offset.
ENTRY_BASE = 12
#: One texture entry.  34, not 40: at 34 the marker byte lands on 100 in every
#: entry of 904 of the 905 records, against 531 at any other stride tried --
#: and 531 is just the number of single-entry records, where a stride cannot
#: be wrong.  The name field runs 14 bytes from +20 and the longest shipped
#: name is 12 characters.
ENTRY_STRIDE = 34
COLOUR_OFFSET = 6
#: An **opacity in percent**, which the engine multiplies by 0.01.  It reads
#: as a constant 100 because all but one material is fully opaque -- the
#: exception is ``FIRESTORM``, whose entries run 0, 60, 80, 90, 95, 100 across
#: its frames, which is a fade-in.  It is what anchors the entry offsets.
OPACITY_OFFSET = 5
OPAQUE = 100

#: The byte before an entry's texture name picks one of the texture's own
#: sub-images -- an index into its ``Page`` table; see ``texm.parse_pages``.
CELL_OFFSET = 19
#: ...or asks for the whole texture.
WHOLE_TEXTURE = 0xFF

#: Where an entry's texture name starts, and how long the field is.
NAME_OFFSET = 20
NAME_FIELD = ENTRY_STRIDE - NAME_OFFSET

#: A second colour sits ahead of the marker.  On 38 of the 43 two-layer
#: materials the two entries hold it and the diffuse the opposite way round --
#: entry 0 white diffuse and black here, entry 1 the reverse -- so the second
#: entry is marked as something other than an ordinary lit layer.  Which of
#: the two D3D slots each is has not been established.
TINT_OFFSET = 2


@dataclass(frozen=True)
class MaterialEntry:
    """One texture entry of a material."""

    texture: str
    #: Sub-image of that texture, or WHOLE_TEXTURE.
    cell: int = WHOLE_TEXTURE
    #: Diffuse colour, as ``(r, g, b)``.
    colour: tuple[int, int, int] = (255, 255, 255)
    #: The second colour slot, ahead of the opacity.
    tint: tuple[int, int, int] = (0, 0, 0)
    #: Opacity in 0..1; 1.0 on every entry but ``FIRESTORM``'s fade-in.
    opacity: float = 1.0


@dataclass
class Material:
    name: str
    #: Total texture entries: ``layers * frames``.
    entry_count: int
    layer_count: int
    #: Every entry in file order.
    entries: list[MaterialEntry] = field(default_factory=list)

    @property
    def colour(self) -> tuple[int, int, int]:
        """Diffuse colour of the first entry; white when there is none."""
        return self.entries[0].colour if self.entries else (255, 255, 255)

    @property
    def cell(self) -> int:
        """Sub-image the first entry asks for."""
        return self.entries[0].cell if self.entries else WHOLE_TEXTURE

    @property
    def textures(self) -> list[str]:
        """The entries' texture names, skipping the entries that name none."""
        return [e.texture for e in self.entries if e.texture]

    @property
    def whole_texture(self) -> bool:
        return self.cell == WHOLE_TEXTURE

    @property
    def texture(self) -> str | None:
        return self.textures[0] if self.textures else None

    @property
    def frame_count(self) -> int:
        """How many animation frames the base layer has."""
        if self.layer_count <= 1:
            return len(self.textures)
        return max(1, self.entry_count // self.layer_count)

    @property
    def frames(self) -> list[str]:
        """The base layer's texture per frame, in order.

        Single-layer materials -- 860 of the 905 -- store their frames
        consecutively, so the name list is the animation.  Multi-layer ones
        interleave in an order that is not established, so only the first
        texture is offered for those.
        """
        if self.layer_count > 1:
            return self.textures[:1]
        return self.textures


def parse_entries(data: bytes, count: int) -> list[MaterialEntry]:
    """The ``count`` texture entries of a MAT0 record."""
    out = []
    for i in range(count):
        at = ENTRY_BASE + i * ENTRY_STRIDE
        blk = data[at : at + ENTRY_STRIDE]
        if len(blk) < ENTRY_STRIDE:
            break
        out.append(
            MaterialEntry(
                texture=blk[NAME_OFFSET:].split(b"\0")[0].decode("latin-1"),
                cell=blk[CELL_OFFSET],
                colour=tuple(blk[COLOUR_OFFSET : COLOUR_OFFSET + 3]),
                tint=tuple(blk[TINT_OFFSET : TINT_OFFSET + 3]),
                opacity=blk[OPACITY_OFFSET] / OPAQUE,
            )
        )
    return out


class MaterialLibrary:
    """``Material.lib``, keyed by material name (case-insensitively)."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.materials: dict[str, Material] = {}
        for entry in self.archive:
            if entry.tag != MATERIAL_TAG:
                continue
            data = self.archive.read(entry)
            count, layers = struct.unpack_from("<2H", data, 0)
            self.materials[entry.name.upper()] = Material(
                entry.name, count, layers, parse_entries(data, count)
            )

    def get(self, name: str) -> Material | None:
        return self.materials.get(name.upper())

    def texture_for(self, name: str) -> str | None:
        """Base texture name for a material, or None if it has none."""
        material = self.get(name)
        return material.texture if material else None

    def colour_for(self, name: str) -> tuple[int, int, int]:
        """Diffuse colour of a material; white when it has none."""
        material = self.get(name)
        return material.colour if material else (255, 255, 255)

    def frames_for(self, name: str) -> list[str]:
        """Animation frames for a material, or an empty list if unknown."""
        material = self.get(name)
        return material.frames if material else []

    def __len__(self) -> int:
        return len(self.materials)
