"""Reader for object geometry -- the ``MESH`` members of ``static.rlb`` & co.

An object mesh is itself an NRes archive nested inside an archive member, with
the same numeric-type-as-stream-selector convention as the terrain
(``landmesh.py``), and the same per-vertex encodings.  All 68 meshes in
``static.rlb`` carry the identical set of 14 streams:

    id  stride  indexed by  contents
    --  ------  ----------  ---------------------------------------------
     1      38  sub-object  per-sub-object header, contents unresolved
     2     ---  ---         floats; opens with bounding-box corners
     3      12  vertex      position, float32 x/y/z
     4       4  vertex      normal, int8 x/y/z / 127, then one padding byte
     5       4  vertex      UV, uint16 8.8 fixed point
     6       6  face        triangle, three uint16 vertex indices
     7      16  face        face record, contents unresolved
     8     ---  ---         unresolved; 24 bytes on most meshes, 96 on some
     9      32  sub-object  sub-object name ("Base_TM", "leaf1_m1o1")
    10       4  sub-object  one uint32, zero throughout the shipped data
    13      12  ---         unresolved
    15       8  vertex      unresolved
    17, 19   0  ---         always empty

Vertex positions, normals, UVs and triangles are established; which texture
each face uses is not -- see ``docs/06-open-questions.md``.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

from .nres import NResArchive

STREAM_SUBOBJECT_HEADER = 1
STREAM_BOUNDS = 2
STREAM_POSITION = 3
STREAM_NORMAL = 4
STREAM_UV = 5
STREAM_TRIANGLE = 6
STREAM_FACE = 7
STREAM_SUBOBJECT_NAME = 9

NAME_FIELD = 32
FACE_STRIDE = 16

#: UVs use the same 8.8 fixed point as the terrain.
UV_FIXED_POINT_SCALE = 256.0


@dataclass
class ObjectMesh:
    name: str
    positions: list[tuple[float, float, float]]
    normals: list[tuple[float, float, float]]
    uv: list[tuple[float, float]]
    triangles: list[tuple[int, int, int]]
    subobjects: list[str] = field(default_factory=list)
    texture_names: list[str] = field(default_factory=list)

    @property
    def vertex_count(self) -> int:
        return len(self.positions)

    @property
    def triangle_count(self) -> int:
        return len(self.triangles)

    def bounds(self) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
        xs = [p[0] for p in self.positions]
        ys = [p[1] for p in self.positions]
        zs = [p[2] for p in self.positions]
        return (min(xs), min(ys), min(zs)), (max(xs), max(ys), max(zs))


def parse(blob: bytes, name: str = "<mesh>", texture_names: list[str] | None = None) -> ObjectMesh:
    """Parse a ``MESH`` payload.  ``texture_names`` comes from the sibling
    ``.wea`` member and is carried through for callers that want it."""
    inner = NResArchive(blob, name)
    stream = {e.type_id: inner.read(e) for e in inner}

    raw_pos = stream[STREAM_POSITION]
    nv = len(raw_pos) // 12
    positions = [struct.unpack_from("<3f", raw_pos, i * 12) for i in range(nv)]

    raw_n = stream[STREAM_NORMAL]
    normals = [
        tuple((v - 256 if v > 127 else v) / 127.0 for v in raw_n[i * 4 : i * 4 + 3])
        for i in range(nv)
    ]

    raw_uv = stream[STREAM_UV]
    uv = [
        (
            struct.unpack_from("<H", raw_uv, i * 4)[0] / UV_FIXED_POINT_SCALE,
            struct.unpack_from("<H", raw_uv, i * 4 + 2)[0] / UV_FIXED_POINT_SCALE,
        )
        for i in range(nv)
    ]

    raw_tri = stream[STREAM_TRIANGLE]
    triangles = [
        struct.unpack_from("<3H", raw_tri, i * 6) for i in range(len(raw_tri) // 6)
    ]

    raw_names = stream.get(STREAM_SUBOBJECT_NAME, b"")
    subobjects = [
        raw_names[i * NAME_FIELD : (i + 1) * NAME_FIELD].split(b"\0")[0].decode("latin-1")
        for i in range(len(raw_names) // NAME_FIELD)
    ]

    return ObjectMesh(
        name=name,
        positions=positions,
        normals=normals,
        uv=uv,
        triangles=triangles,
        subobjects=subobjects,
        texture_names=texture_names or [],
    )


CONTROL_POINT_NUMERIC = 36
CONTROL_POINT_NAME = 32


@dataclass
class ControlPoint:
    """A named point on a model: an attachment, a light, an effect origin.

    The record is nine float32.  In ``static.rlb`` and ``turrets.rlb`` the
    first triple is zero, the second is a position inside the model's bounding
    box and the third a unit direction -- but that reading does not hold in
    every archive (``guns.rlb`` stores a scalar width in a vector slot), so the
    triples are exposed as they are and named ``a``, ``position`` and
    ``direction`` only as the best-supported interpretation.
    """

    name: str
    a: tuple[float, float, float]
    position: tuple[float, float, float]
    direction: tuple[float, float, float]


def parse_control_points(blob: bytes, source: str = "<cpt>") -> list[ControlPoint]:
    """Parse a ``CTPT`` payload.

    The layout is two parallel arrays, not an array of records::

        uint32  count
        count x float32[9]     numeric data
        count x char[32]       names

    which is why the total is always ``4 + count * 68``.
    """
    count = struct.unpack_from("<I", blob, 0)[0]
    expected = 4 + count * (CONTROL_POINT_NUMERIC + CONTROL_POINT_NAME)
    if expected != len(blob):
        raise ValueError(
            f"{source}: {count} control points implies {expected} bytes, have {len(blob)}"
        )
    names_at = 4 + count * CONTROL_POINT_NUMERIC
    out = []
    for i in range(count):
        v = struct.unpack_from("<9f", blob, 4 + i * CONTROL_POINT_NUMERIC)
        raw = blob[names_at + i * CONTROL_POINT_NAME : names_at + (i + 1) * CONTROL_POINT_NAME]
        out.append(ControlPoint(raw.split(b"\0")[0].decode("latin-1"), v[0:3], v[3:6], v[6:9]))
    return out


def read_wea(blob: bytes) -> list[str]:
    """Parse a ``.wea`` texture name table: a count, then ``index name`` pairs."""
    tokens = blob.decode("latin-1").split()
    if not tokens:
        return []
    count = int(tokens[0])
    names = [""] * count
    for i in range(1, len(tokens) - 1, 2):
        idx = int(tokens[i])
        if 0 <= idx < count:
            names[idx] = tokens[i + 1]
    return names
