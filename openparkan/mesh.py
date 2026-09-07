"""Reader for object geometry -- the ``MESH`` members of ``static.rlb`` & co.

An object mesh is itself an NRes archive nested inside an archive member, with
the same numeric-type-as-stream-selector convention as the terrain
(``landmesh.py``), and the same per-vertex encodings.  All 68 meshes in
``static.rlb`` carry the identical set of 14 streams:

    id  stride  indexed by  contents
    --  ------  ----------  ---------------------------------------------
     1      38  sub-object  flags, parent, and a list of part indices
     2     ---  ---         floats; opens with bounding-box corners
     3      12  vertex      position, float32 x/y/z
     4       4  vertex      normal, int8 x/y/z / 127, then one padding byte
     5       4  vertex      UV, uint16 8.8 fixed point
     6       6  face        triangle, three uint16 indices *relative to the
                             first_vertex of the batch that covers them*
     7      16  face        face record, contents unresolved
     8     ---  ---         unresolved; 24 bytes on most meshes, 96 on some
     9      32  sub-object  sub-object name ("Base_TM", "leaf1_m1o1")
    10       4  sub-object  one uint32, zero throughout the shipped data
    13      20  batch       draw batch: material, index range, vertex range
    15       8  vertex      unresolved
    17      20  node        building interior path graph, see parse_path_graph
    19     ---  ---         unresolved; present on many meshes, empty on some

Materials are assigned per *batch*, not per face: stream 13 groups runs of the
index buffer and names a material for each, which is why no field of the face
record ever held a texture index.

Indices in stream 6 are **relative to the batch that covers them**, the
DirectX ``DrawIndexedPrimitive`` convention: the real vertex is
``batch.first_vertex + index``.  ``ObjectMesh.triangles`` has this already
applied; ``raw_triangles`` keeps the file's own values.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

from .nres import NResArchive

STREAM_BOUNDS = 2
STREAM_POSITION = 3
STREAM_NORMAL = 4
STREAM_UV = 5
STREAM_TRIANGLE = 6
STREAM_FACE = 7
STREAM_SUBOBJECT_NAME = 9
STREAM_SUBOBJECT_HEADER = 1
STREAM_BATCH = 13
STREAM_PATH_GRAPH = 17

SUBOBJECT_HEADER_SIZE = 38
#: Bit 0 of a sub-object's flags marks interior geometry.
SUBOBJECT_INTERIOR = 0x0001
NO_PARENT = 0xFFFF

BATCH_SIZE = 20
#: High byte of a batch's material word; 0xFF on most batches, 0x00 on some.
BATCH_MATERIAL_MASK = 0xFF

PATH_NODE_SIZE = 20
PATH_LINK_SIZE = 40
NO_LINK = 0xFFFFFFFF

NAME_FIELD = 32
FACE_STRIDE = 16

#: UVs use the same 8.8 fixed point as the terrain.
UV_FIXED_POINT_SCALE = 256.0


@dataclass
class Subobject:
    """One named part of a model.

    Buildings carry their inside and their outside in the same mesh -- Parkan
    lets you walk into them -- so a model's parts are split between the two.
    Names say which (``o01_0_m1o1`` outside, ``i03_0_m1o1`` inside) and so does
    ``flags`` bit 0, which agrees with the naming on all 1564 sub-objects
    across six archives.
    """

    name: str
    flags: int
    parent: int
    #: Indices into stream 2's part list; how those reach triangles is not
    #: yet known, so interior geometry cannot be filtered out yet.
    parts: list[int]

    @property
    def is_interior(self) -> bool:
        return bool(self.flags & SUBOBJECT_INTERIOR)


@dataclass
class Batch:
    """A run of the index buffer drawn with one material.

    ``material`` indexes the model's wear (its ``.wea`` palette).
    """

    material: int
    flag: int
    first_index: int
    index_count: int
    first_vertex: int
    vertex_count: int

    @property
    def triangles(self) -> tuple[int, int]:
        """``(first triangle, triangle count)`` into the mesh's triangle list."""
        return self.first_index // 3, self.index_count // 3


@dataclass
class ObjectMesh:
    name: str
    positions: list[tuple[float, float, float]]
    normals: list[tuple[float, float, float]]
    uv: list[tuple[float, float]]
    #: Absolute vertex indices, with each batch's first_vertex already added.
    triangles: list[tuple[int, int, int]]
    #: The file's own batch-relative indices.
    raw_triangles: list[tuple[int, int, int]] = field(default_factory=list)
    subobjects: list[str] = field(default_factory=list)
    texture_names: list[str] = field(default_factory=list)
    batches: list[Batch] = field(default_factory=list)
    parts: list[Subobject] = field(default_factory=list)

    @property
    def has_interior(self) -> bool:
        return any(p.is_interior for p in self.parts)

    def material_of_triangle(self, index: int) -> int | None:
        """Material index for a triangle, via the batch that covers it."""
        for b in self.batches:
            first, count = b.triangles
            if first <= index < first + count:
                return b.material
        return None

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
    entries = {e.type_id: e for e in inner}

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
    raw_triangles = [
        struct.unpack_from("<3H", raw_tri, i * 6) for i in range(len(raw_tri) // 6)
    ]

    raw_names = stream.get(STREAM_SUBOBJECT_NAME, b"")
    subobjects = [
        raw_names[i * NAME_FIELD : (i + 1) * NAME_FIELD].split(b"\0")[0].decode("latin-1")
        for i in range(len(raw_names) // NAME_FIELD)
    ]

    headers = stream.get(STREAM_SUBOBJECT_HEADER, b"")
    parts = []
    header_entry = entries.get(STREAM_SUBOBJECT_HEADER)
    n_parts = header_entry.element_count if header_entry else 0
    if len(headers) == n_parts * SUBOBJECT_HEADER_SIZE:
        for i in range(n_parts):
            words = struct.unpack_from("<19H", headers, i * SUBOBJECT_HEADER_SIZE)
            listed = [w for w in words[3:] if w != 0xFFFF]
            parts.append(
                Subobject(
                    name=subobjects[i] if i < len(subobjects) else "",
                    flags=words[0],
                    parent=words[1],
                    parts=listed,
                )
            )

    batches = []
    raw_batch = stream.get(STREAM_BATCH, b"")
    for i in range(entries[STREAM_BATCH].element_count if STREAM_BATCH in entries else 0):
        f = struct.unpack_from("<10H", raw_batch, i * BATCH_SIZE)
        batches.append(
            Batch(
                material=f[2] & BATCH_MATERIAL_MASK,
                flag=f[2] >> 8,
                index_count=f[4],
                first_index=f[5],
                vertex_count=f[7],
                first_vertex=f[8],
            )
        )

    # Resolve batch-relative indices to absolute ones.  A mesh without batches
    # has nothing to resolve against, so its indices are taken as they are.
    triangles = list(raw_triangles)
    for b in batches:
        first, count = b.triangles
        for t in range(first, min(first + count, len(triangles))):
            a, bb, c = raw_triangles[t]
            triangles[t] = (a + b.first_vertex, bb + b.first_vertex, c + b.first_vertex)

    return ObjectMesh(
        name=name,
        positions=positions,
        normals=normals,
        uv=uv,
        triangles=triangles,
        raw_triangles=raw_triangles,
        subobjects=subobjects,
        texture_names=texture_names or [],
        batches=batches,
        parts=parts,
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


@dataclass
class PathNode:
    """A waypoint inside a building."""

    position: tuple[float, float, float]
    a: int
    b: int


@dataclass
class PathLink:
    """A traversable connection between two waypoints."""

    start: int
    end: int
    #: Eight further slots, ``0xFFFFFFFF`` throughout the shipped data.
    extra: tuple[int, ...]


@dataclass
class PathGraph:
    nodes: list[PathNode]
    links: list[PathLink]


def parse_path_graph(blob: bytes, node_count: int, link_count: int) -> PathGraph:
    """Parse stream 17, the interior path graph the engine calls a *hall way*.

    The two counts are not in the payload; they are the element-count and the
    following field of the stream's own NRes directory entry.  Only buildings
    carry one -- 29 of the 30 meshes in ``fortif.rlb``, and nothing anywhere
    else, because only a building has an inside to walk around.
    """
    expected = node_count * PATH_NODE_SIZE + link_count * PATH_LINK_SIZE
    if len(blob) != expected:
        raise ValueError(
            f"path graph: {node_count} nodes and {link_count} links imply "
            f"{expected} bytes, have {len(blob)}"
        )
    nodes = []
    for i in range(node_count):
        x, y, z, a, b = struct.unpack_from("<3f2I", blob, i * PATH_NODE_SIZE)
        nodes.append(PathNode((x, y, z), a, b))
    base = node_count * PATH_NODE_SIZE
    links = []
    for i in range(link_count):
        values = struct.unpack_from("<10I", blob, base + i * PATH_LINK_SIZE)
        links.append(PathLink(values[0], values[1], values[2:]))
    return PathGraph(nodes, links)


def read_path_graph(archive: NResArchive) -> PathGraph | None:
    """Read the path graph out of an already-opened object mesh, if it has one."""
    for entry in archive:
        if entry.type_id == STREAM_PATH_GRAPH and entry.size:
            return parse_path_graph(
                archive.read(entry), entry.element_count, entry.link_count
            )
    return None


def read_wea(blob: bytes) -> list[str]:
    """Parse a ``.wea`` -- a *wear*, the material palette of a model.

    A count, then ``index name`` pairs.  Some wears carry further keyword
    sections such as ``LIGHTMAPS`` after the palette; parsing stops there.
    """
    tokens = blob.decode("latin-1").split()
    if not tokens or not tokens[0].lstrip("-").isdigit():
        return []
    count = int(tokens[0])
    names = [""] * count
    i = 1
    while i + 1 < len(tokens):
        if not tokens[i].lstrip("-").isdigit():
            break
        idx = int(tokens[i])
        if 0 <= idx < count:
            names[idx] = tokens[i + 1]
        i += 2
    return names
