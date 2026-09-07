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

     0  flags
     1  surface kind (2 == water)
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
SURFACE_WATER = 2

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

    def texture_name(self, layer: int, index: int) -> str | None:
        table = self.layer1_names if layer == 1 else self.layer2_names
        if index == NO_TEXTURE or index >= len(table):
            return None
        return table[index]


def _read_wea(path: Path) -> list[str]:
    """Read a ``.wea`` name table: a count, then ``index name`` pairs."""
    if not path.exists():
        return []
    tokens = path.read_bytes().decode("latin-1").split()
    if not tokens:
        return []
    count = int(tokens[0])
    names = [""] * count
    for i in range(1, len(tokens) - 1, 2):
        idx = int(tokens[i])
        if 0 <= idx < count:
            names[idx] = tokens[i + 1]
    return names


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
