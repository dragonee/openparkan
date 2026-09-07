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
(``L20.0`` and its mask ``L20M.0``).  860 materials have a single layer, 43
have two and two have eight; animation is much the commoner reason for a
material to hold several textures.

An entry begins 12 bytes in and runs 40 bytes to the next, with the texture
name at +20.  At +6 sits a three-byte RGB **diffuse colour** that modulates
the texture, preceded by a constant 100 -- constant on 904 of the 905 records,
which is what makes the offset trustworthy.  It matters: ``WATER``'s texture
is a neutral grey ripple and the blue is entirely in its ``#4d6aff``, and lava
is a dull red pattern tinted ``#b41e00``.  761 materials carry a colour other
than white.

The texture names are still extracted by pattern rather than by offset,
because the record's tail is not a constant size -- most are
``12 + 40 * count + 8`` bytes but the two-layer ones are longer.  The names
are unambiguous enough for that to be safe: the number found equals the
declared count on 895 of the 905 records.
"""

from __future__ import annotations

import re
import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

MATERIAL_TAG = "MAT0"

#: An entry's diffuse colour, and the constant that anchors the offset.
ENTRY_BASE = 12
ENTRY_STRIDE = 40
COLOUR_OFFSET = 6
COLOUR_MARKER_OFFSET = 5
COLOUR_MARKER = 100

#: Texture references look like ``NAME.0`` -- the same form Textures.lib uses.
_TEXTURE_RE = re.compile(rb"[A-Za-z0-9_]{2,}\.\d+")


@dataclass
class Material:
    name: str
    #: Total texture entries: ``layers * frames``.
    entry_count: int
    layer_count: int
    #: Texture names in file order; the first is the base texture.
    textures: list[str]
    #: Diffuse colour multiplying the base texture, as ``(r, g, b)``.
    colour: tuple[int, int, int] = (255, 255, 255)

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


class MaterialLibrary:
    """``Material.lib``, keyed by material name (case-insensitively)."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.materials: dict[str, Material] = {}
        for entry in self.archive:
            if entry.tag != MATERIAL_TAG:
                continue
            data = self.archive.read(entry)
            entries, layers = struct.unpack_from("<2H", data, 0)
            names = [m.group().decode("latin-1") for m in _TEXTURE_RE.finditer(data)]
            colour = (255, 255, 255)
            at = ENTRY_BASE + COLOUR_OFFSET
            if len(data) >= at + 3 and data[at - 1] == COLOUR_MARKER:
                colour = tuple(data[at : at + 3])
            self.materials[entry.name.upper()] = Material(
                entry.name, entries, layers, names, colour
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
