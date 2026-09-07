"""Parser for ``Land.msh`` -- the terrain mesh of a Parkan map.

``Land.msh`` is an NRes archive whose members all share the name ``Land`` and
are distinguished only by their numeric type id, which acts as a stream
selector.  Every stream is a flat array indexed by vertex or by face.

    id  stride  indexed by  contents
    --  ------  ----------  ---------------------------------------------
     1     ---  ---         unresolved (small, mostly 0xFF)
     2      12  ---         bounding geometry: 8 bbox corners, then more
     3      12  vertex      position, float32 x/y/z  (z is up)
     4       4  vertex      normal, int8 x/y/z / 127, then one padding byte
     5       4  vertex      layer-1 UV, uint16 8.8 fixed point
    18       4  vertex      layer-2 UV, uint16 8.8 fixed point
    14       4  vertex      layer blend weight, float32 in 0..1
    11       4  face        (face index, flags)
    21      28  face        the face record, see FACE below

FACE, as 14 little-endian uint16::

     0  flags; 1544 (0x0608) marks water
     1  surface bitfield; bit 0x02 marks water
     2  lo byte = layer-1 texture index, hi byte = layer-2 (0xFF = none);
        both index the map's Land1.wea / Land2.wea name tables
     3  always 0xFFFF
     4  vertex 0
     5  vertex 1
     6  vertex 2
     7  adjacent face across edge 0 (0xFFFF = none)
     8  adjacent face across edge 1
     9  adjacent face across edge 2
    10  unresolved
    11  unresolved
    12  unresolved
    13  patch / sector id

See ``docs/03-terrain.md`` for how each of these was established.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field
from pathlib import Path

from .mesh import read_wea
from .nres import NResArchive

STREAM_BOUNDS = 2
STREAM_POSITION = 3
STREAM_NORMAL = 4
STREAM_UV1 = 5
STREAM_UV2 = 18
STREAM_BLEND = 14
STREAM_FACE_FLAGS = 11
STREAM_FACE = 21

FACE_STRIDE = 28
NO_NEIGHBOUR = 0xFFFF
NO_TEXTURE = 0xFF

#: Bit 1 of the face's surface word marks a water surface.  It is a *bitfield*,
#: not an enum: the observed values are 0, 2, 16 and 18, and testing ``== 2``
#: silently misses every water face that also carries bit 16.
SURFACE_WATER_BIT = 0x02

#: Face flags word carried by every water face, on every map that has water.
#: An independent corroboration of SURFACE_WATER_BIT.
FLAGS_WATER = 1544

#: UV values are 8.8 fixed point and the layer-1 mapping tiles every 50 world
#: units, which is how ``u == x / 50`` comes out as ``u16 == x * 5.12``.
UV_FIXED_POINT_SCALE = 256.0


@dataclass
class LandMesh:
    positions: list[tuple[float, float, float]]
    normals: list[tuple[float, float, float]]
    uv1: list[tuple[float, float]]
    uv2: list[tuple[float, float]]
    blend: list[float]
    faces: list[tuple[int, int, int]]
    adjacency: list[tuple[int, int, int]]
    face_flags: list[int]
    face_surface: list[int]
    face_tex1: list[int]
    face_tex2: list[int]
    face_patch: list[int]
    layer1_names: list[str] = field(default_factory=list)
    layer2_names: list[str] = field(default_factory=list)
    _grid: dict | None = field(default=None, repr=False, compare=False)

    @property
    def vertex_count(self) -> int:
        return len(self.positions)

    @property
    def face_count(self) -> int:
        return len(self.faces)

    def bounds(self) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
        xs = [p[0] for p in self.positions]
        ys = [p[1] for p in self.positions]
        zs = [p[2] for p in self.positions]
        return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))

    def is_water(self, face: int) -> bool:
        """Whether a face is part of a water surface."""
        return bool(self.face_surface[face] & SURFACE_WATER_BIT)

    def water_faces(self) -> list[int]:
        return [i for i in range(self.face_count) if self.is_water(i)]

    def water_level(self) -> float | None:
        """The z of the map's water plane, or None if the map has no water.

        Water is a single flat plane on all 11 maps that have any, so a lone
        value is expected; None also covers a map that breaks that assumption.
        """
        levels = {self.positions[v][2] for i in self.water_faces() for v in self.faces[i]}
        return levels.pop() if len(levels) == 1 else None

    def _build_index(self, cells: int = 64) -> None:
        """Bucket faces into a coarse XY grid so height_at is not O(faces)."""
        (minx, miny, _), (maxx, maxy, _) = self.bounds()
        self._grid_origin = (minx, miny)
        self._grid_step = ((maxx - minx) / cells, (maxy - miny) / cells)
        self._grid_cells = cells
        grid: dict[tuple[int, int], list[int]] = {}
        sx, sy = self._grid_step
        for fi, tri in enumerate(self.faces):
            xs = [self.positions[i][0] for i in tri]
            ys = [self.positions[i][1] for i in tri]
            for cx in range(int((min(xs) - minx) / sx), int((max(xs) - minx) / sx) + 1):
                for cy in range(int((min(ys) - miny) / sy), int((max(ys) - miny) / sy) + 1):
                    grid.setdefault((cx, cy), []).append(fi)
        self._grid = grid

    def height_at(self, x: float, y: float) -> float | None:
        """Terrain elevation at a world XY, or None if outside the mesh.

        Where water covers the ground the higher surface wins, which is what a
        thing standing on the map would rest on.
        """
        if getattr(self, "_grid", None) is None:
            self._build_index()
        sx, sy = self._grid_step
        ox, oy = self._grid_origin
        cell = (int((x - ox) / sx), int((y - oy) / sy))
        best = None
        for fi in self._grid.get(cell, ()):
            a, b, c = (self.positions[i] for i in self.faces[fi])
            den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
            if abs(den) < 1e-12:
                continue
            l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den
            l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den
            l3 = 1.0 - l1 - l2
            if l1 < -1e-6 or l2 < -1e-6 or l3 < -1e-6:
                continue
            z = l1 * a[2] + l2 * b[2] + l3 * c[2]
            if best is None or z > best:
                best = z
        return best

    def texture_name(self, layer: int, index: int) -> str | None:
        table = self.layer1_names if layer == 1 else self.layer2_names
        if index == NO_TEXTURE or index >= len(table):
            return None
        return table[index]


def _read_wea(path: Path) -> list[str]:
    """Read a ``.wea`` name table from disk; the same format objects use."""
    return read_wea(path.read_bytes()) if path.exists() else []


def load(path: str | Path) -> LandMesh:
    """Load a ``Land.msh``.  Sibling ``Land1.wea`` / ``Land2.wea`` are picked
    up automatically when present, giving the terrain texture names."""
    path = Path(path)
    archive = NResArchive.open(path)

    raw_pos = archive.one_of_type(STREAM_POSITION)
    nv = len(raw_pos) // 12
    positions = [struct.unpack_from("<3f", raw_pos, i * 12) for i in range(nv)]

    raw_n = archive.one_of_type(STREAM_NORMAL)
    normals = []
    for i in range(nv):
        b = raw_n[i * 4 : i * 4 + 3]
        normals.append(tuple((v - 256 if v > 127 else v) / 127.0 for v in b))

    def read_uv(stream: int) -> list[tuple[float, float]]:
        raw = archive.one_of_type(stream)
        return [
            (
                struct.unpack_from("<H", raw, i * 4)[0] / UV_FIXED_POINT_SCALE,
                struct.unpack_from("<H", raw, i * 4 + 2)[0] / UV_FIXED_POINT_SCALE,
            )
            for i in range(nv)
        ]

    uv1 = read_uv(STREAM_UV1)
    uv2 = read_uv(STREAM_UV2)

    raw_blend = archive.one_of_type(STREAM_BLEND)
    blend = [struct.unpack_from("<f", raw_blend, i * 4)[0] for i in range(nv)]

    raw_face = archive.one_of_type(STREAM_FACE)
    nf = len(raw_face) // FACE_STRIDE
    faces, adjacency = [], []
    flags, surface, tex1, tex2, patch = [], [], [], [], []
    for i in range(nf):
        r = struct.unpack_from("<14H", raw_face, i * FACE_STRIDE)
        faces.append((r[4], r[5], r[6]))
        adjacency.append((r[7], r[8], r[9]))
        flags.append(r[0])
        surface.append(r[1])
        tex1.append(r[2] & 0xFF)
        tex2.append(r[2] >> 8)
        patch.append(r[13])

    return LandMesh(
        positions=positions,
        normals=normals,
        uv1=uv1,
        uv2=uv2,
        blend=blend,
        faces=faces,
        adjacency=adjacency,
        face_flags=flags,
        face_surface=surface,
        face_tex1=tex1,
        face_tex2=tex2,
        face_patch=patch,
        layer1_names=_read_wea(path.parent / "Land1.wea"),
        layer2_names=_read_wea(path.parent / "Land2.wea"),
    )
