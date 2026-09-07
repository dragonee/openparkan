"""Reader for ``Material.lib`` -- the ``MAT0`` material database.

905 materials, each naming one or more texture layers.  A model's wear (its
``.wea``) lists material names; a mesh batch picks one of them by index; the
material names the texture.  That completes the chain from a triangle to a
pixel::

    mesh stream 13 batch -> wear entry -> Material.lib MAT0 -> Textures.lib Texm

Only part of ``MAT0`` is mapped.  The record opens with a layer count and
carries per-layer colour bytes, and each layer names its texture in the same
form ``Textures.lib`` uses for member names (``L04.0``, ``STONE00.0``).  The
texture names are extracted by pattern rather than by offset, because the
per-layer stride varies with the layer type and has not been pinned down; the
names are unambiguous enough for that to be safe -- 3096 of 3139 resolve, and
the 43 that do not are animation frames held elsewhere.
"""

from __future__ import annotations

import re
import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

MATERIAL_TAG = "MAT0"

#: Texture references look like ``NAME.0`` -- the same form Textures.lib uses.
_TEXTURE_RE = re.compile(rb"[A-Za-z0-9_]{2,}\.\d+")


@dataclass
class Material:
    name: str
    layer_count: int
    #: Texture names in layer order; the first is the base texture.
    textures: list[str]

    @property
    def texture(self) -> str | None:
        return self.textures[0] if self.textures else None


class MaterialLibrary:
    """``Material.lib``, keyed by material name (case-insensitively)."""

    def __init__(self, path: str | Path):
        self.archive = NResArchive.open(path)
        self.materials: dict[str, Material] = {}
        for entry in self.archive:
            if entry.tag != MATERIAL_TAG:
                continue
            data = self.archive.read(entry)
            layers = struct.unpack_from("<H", data, 0)[0]
            names = [m.group().decode("latin-1") for m in _TEXTURE_RE.finditer(data)]
            self.materials[entry.name.upper()] = Material(entry.name, layers, names)

    def get(self, name: str) -> Material | None:
        return self.materials.get(name.upper())

    def texture_for(self, name: str) -> str | None:
        """Base texture name for a material, or None if it has none."""
        material = self.get(name)
        return material.texture if material else None

    def __len__(self) -> int:
        return len(self.materials)
