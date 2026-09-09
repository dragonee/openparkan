"""Parser for ``Land.msh`` -- the terrain mesh of a Parkan map.

``Land.msh`` is an NRes archive whose members all share the name ``Land`` and
are distinguished only by their numeric type id, which acts as a stream
selector.  Every stream is a flat array indexed by vertex or by face.

    id  stride  indexed by  contents
    --  ------  ----------  ---------------------------------------------
     1      38  square      the square table over the cells of stream 2
     2      68  cell        the spatial index: 8 bbox corners, then the cells
     3      12  vertex      position, float32 x/y/z  (z is up)
     4       4  vertex      normal, int8 x/y/z / 127, then one padding byte
     5       4  vertex      layer-1 UV, uint16 8.8 fixed point
    18       4  vertex      layer-2 UV, uint16 8.8 fixed point
    14       4  vertex      layer blend weight, float32 in 0..1
    11       4  face        (face index, flags)
    21      28  face        the face record, see FACE below

Streams 1 and 2 are the map's own **spatial index**, and it is a flat grid
with a list per square rather than a tree.  Stream 2 holds one 68-byte record
per cell -- a box, a centre, a bounding-sphere radius and a run of faces --
and the faces are stored in cell order, so a run indexes ``faces`` directly:
all 275882 faces across the 33 maps lie inside their own cell's box.  Stream 1
holds one 38-byte record per grid square naming the cells that cover it, two
of a possible fifteen on every one of the 7488 squares.  The grid is 16 x 16
on 28 maps and 8 x 8 on five, following the vertex count rather than the world
size.

Each square's two cells share a box and split its faces in two, and the second
block is where the duplicated geometry lives: between a fifth and two fifths
of a map's faces are stored twice at identical positions, and 46261 of those
46283 are in a second block.  See ``LandMesh.distinct_faces``.

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
    13  0..62, ~57 distinct values.  Not a spatial patch: a value's faces
         span the whole map, indistinguishable from a random subset of the
         same size.  Not a material key either.  Unresolved.

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

#: Stream 2 is the map's own spatial index: the eight corners of the mesh's
#: bounding box, then one record per cell of a uniform grid.
BOX_CORNERS = 8
BOX_HEADER = BOX_CORNERS * 12
#: A record: ``uint16 first``, ``uint16 count``, a zero, then the cell's box,
#: centre and bounding-sphere radius, then five more zeros.
CELL_STRIDE = 68
#: The first record does not start at the corners' end; 44 bytes of the header
#: come first and are zero on all 33 maps.
CELL_START = 44
#: A cell is listed twice.  The two records share a box and split the faces
#: into two blocks: 46261 of the 46283 duplicated faces are in the second.
CELLS_PER_SQUARE = 2

#: Stream 1 is the square table that indexes stream 2: one record per grid
#: square, four words of header then room for 15 cell indices terminated by
#: ``0xFFFF``.  Every one of the 7488 shipped squares uses exactly two.
SQUARE_WORDS = 19
SQUARE_HEADER = 4
NO_CELL = 0xFFFF
STREAM_SQUARES = 1

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


@dataclass(frozen=True)
class Cell:
    """One cell of the terrain's grid: a box and a run of faces."""

    #: The first face of the run, and how many.  Faces are stored in cell
    #: order, so this indexes ``LandMesh.faces`` directly.
    first: int
    count: int
    minimum: tuple[float, float, float]
    maximum: tuple[float, float, float]
    centre: tuple[float, float, float]
    #: Radius of the sphere around the box.
    radius: float

    @property
    def faces(self) -> range:
        return range(self.first, self.first + self.count)


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
    #: The map's own spatial index, from stream 2; empty if it has none.
    cells: list[Cell] = field(default_factory=list)
    #: One entry per grid square, from stream 1: the cells that cover it.
    squares: list[tuple[int, ...]] = field(default_factory=list)
    _grid: dict | None = field(default=None, repr=False, compare=False)

    @property
    def grid_size(self) -> tuple[int, int]:
        """How many cells across and down, from the distinct cell corners."""
        if not self.cells:
            return (0, 0)
        return (len({round(c.minimum[0], 1) for c in self.cells}),
                len({round(c.minimum[1], 1) for c in self.cells}))

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

    def distinct_faces(self) -> list[int]:
        """One face per set of triangles that occupy the same three points.

        Between a fifth and two fifths of a map's faces are stored **twice**,
        at bit-identical positions -- 84% of the flat ``L32`` ground on map 23
        alone.  The two copies carry the same layer-1 texture, the same UVs,
        the same normals and the same winding, and agree on whether they have
        a second layer; they differ only in incidental per-vertex layer-2 data
        and, on about an eighth of them, the face's patch word.  The engine
        presumably draws one patch or the other and never both.

        A renderer that draws the file as it stands draws those triangles
        twice at the same depth, and they z-fight -- which is what makes the
        walkable ground flicker.  This returns the first face of each set, in
        file order, so each surface is drawn once.
        """
        seen: dict[tuple, int] = {}
        keep = []
        for i, tri in enumerate(self.faces):
            key = tuple(sorted(self.positions[v] for v in tri))
            if key in seen:
                continue
            seen[key] = i
            keep.append(i)
        return keep

    def texture_name(self, layer: int, index: int) -> str | None:
        table = self.layer1_names if layer == 1 else self.layer2_names
        if index == NO_TEXTURE or index >= len(table):
            return None
        return table[index]


def _read_wea(path: Path) -> list[str]:
    """Read a ``.wea`` name table from disk; the same format objects use."""
    return read_wea(path.read_bytes()) if path.exists() else []


def parse_cells(raw: bytes | None) -> list[Cell]:
    """The grid of stream 2.

    Eight box corners, 44 bytes that are zero on every map, then one 68-byte
    record per cell.  It parses with nothing left over on all 33 maps, the
    runs chain end to end, and their counts sum to the face count -- so the
    faces are stored in cell order and a run indexes them directly.  Every one
    of the 275882 faces lies inside its own cell's box.
    """
    if not raw or len(raw) < BOX_HEADER + CELL_START:
        return []
    body = raw[BOX_HEADER:]
    out = []
    at = CELL_START
    while at + CELL_STRIDE <= len(body):
        first, count = struct.unpack_from("<2H", body, at)
        out.append(
            Cell(
                first=first,
                count=count,
                minimum=struct.unpack_from("<3f", body, at + 8),
                maximum=struct.unpack_from("<3f", body, at + 20),
                centre=struct.unpack_from("<3f", body, at + 32),
                radius=struct.unpack_from("<f", body, at + 44)[0],
            )
        )
        at += CELL_STRIDE
    return out


def parse_squares(raw: bytes | None) -> list[tuple[int, ...]]:
    """The square table of stream 1.

    A record is 19 ``uint16``: four of header -- 0, ``0xFFFF``, 0, 0 on all
    7488 shipped squares, so nothing tells them apart -- then room for 15 cell
    indices, ``0xFFFF`` for an empty slot.  Every square uses exactly two, and
    they are its own two: the pair always shares a bounding box.
    """
    if not raw:
        return []
    count = len(raw) // 2 // SQUARE_WORDS
    out = []
    for i in range(count):
        words = struct.unpack_from(f"<{SQUARE_WORDS}H", raw, i * SQUARE_WORDS * 2)
        out.append(tuple(v for v in words[SQUARE_HEADER:] if v != NO_CELL))
    return out


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
        cells=parse_cells(archive.one_of_type(STREAM_BOUNDS)),
        squares=parse_squares(archive.one_of_type(STREAM_SQUARES)),
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
