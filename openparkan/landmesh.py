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
    11       4  face        the draw order: a face index, a flags byte
                             and a byte that is zero on 275566 of 275882
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

Each square names **two** cells, and they are the same ground at two levels
of detail: the first is the mesh as authored and the second a simplification
of it.  Each level covers the map on its own, level 1 is built from a subset
of level 0's vertices, and it has fewer faces in every one of the 7488 cell
pairs where it differs at all.  Level 0 is the contiguous face range
``[0, split)`` and level 1 the rest.  A renderer draws **one level per cell**;
drawing both puts two surfaces a fraction of a unit apart over the flat ground
and they z-fight.  See ``LandMesh.lod_faces``.

FACE, as 14 little-endian uint16::

     0  flags over a constant 0x600: bit 0x004 marks a face with a second
        texture layer, 0x008 water and 0x2000 the bed beneath a liquid
     1  surface bitfield; bit 0x02 marks water and bit 0x10 is clear on lava
     2  lo byte = layer-1 texture index, hi byte = layer-2 (0xFF = none);
        both index the map's Land1.wea / Land2.wea name tables
     3  always 0xFFFF
     4  vertex 0
     5  vertex 1
     6  vertex 2
     7  adjacent face across edge 0 (0xFFFF = none)
     8  adjacent face across edge 1
     9  adjacent face across edge 2
    10  face normal x, int16 over 32767
    11  face normal y
    12  face normal z
    13  three 2-bit codes, one per edge: which edge of the face across
         that edge is the shared one, or 3 where there is no neighbour.
         This is the mesh's winged-edge link -- see EDGE_TWIN_BITS

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
STREAM_DRAW_ORDER = 11
DRAW_ORDER_STRIDE = 4
#: Bit 4 of a draw-order entry's flags byte: this face opens a batch.  It is
#: the one bit of the byte the landscape's draw reads (``Terrain.dll:0x1004399a``).
DRAW_BATCH_START = 0x10
#: What the engine writes into the rest of the byte when it rebuilds a cell's
#: draw order after placing a building (``Terrain.dll:0x10060480`` and two
#: siblings): bit 3 set, bits 5-6 set to 2, bits 0-2 clear.  That is the
#: constant 0x48 every shipped entry carries.
DRAW_FLAGS_BUILT = 0x48
#: Bit 7, on 89 shipped entries of ``ILKON`` and ``SC_3``, has **no reader and
#: no writer** in ``Terrain.dll``: the draw tests bit 4 alone and the
#: rebuilders write every other bit through masks that leave bit 7 as it was.
#: So it changes nothing the engine does.
DRAW_FLAGS_UNREAD = 0x80
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
#: A square names one cell per **level of detail**.  The two records share a
#: box and split the faces into two contiguous runs: the authored mesh, then a
#: simplification of it built from a subset of the same vertices.
CELLS_PER_SQUARE = 2
LOD_COUNT = CELLS_PER_SQUARE

#: Stream 1 is the square table that indexes stream 2: one record per grid
#: square, four words of header then room for 15 cell indices terminated by
#: ``0xFFFF``.  Every one of the 7488 shipped squares uses exactly two.
SQUARE_WORDS = 19
SQUARE_HEADER = 4
NO_CELL = 0xFFFF
STREAM_SQUARES = 1

#: Bit 13 of the face's flags word marks the **bed beneath a liquid**: it is
#: set on exactly the 6102 faces whose layer-1 material is ``WATER_BOT`` or
#: ``ENV_LAVA_BOT``, and on no other face of any map.
FLAGS_LIQUID_BED_BIT = 0x2000

#: cos 80 degrees.  A face whose normal z is not above this passes neither of a
#: unit's ground searches (``Control.dll:0x1001a6fd``) and ends a mesh walk
#: (``Terrain.dll:0x10026630``); only the searches' last fallback can take one,
#: for a tick.
WALKABLE_NORMAL_Z = 0.173648
#: How many faces ``CWorld::FindWorldFace`` visits before giving up: the
#: counter's 25th pass returns failure (``Terrain.dll:0x10026519``).
WALK_FACES = 24

#: Bit 2 of the face's *flags* word marks a face that carries a second
#: texture layer: it is set on exactly the 32450 faces whose layer-2 index is
#: not 0xFF, and on no other, across all 33 maps.
FLAGS_LAYER2_BIT = 0x0004

#: The landscape keeps stream 21 as it reads it -- the pointer at its ``+0x6c``
#: and the element count beside it (``Terrain.dll:0x100176e6``), 28 bytes a
#: face, adjacency at ``+0xe`` (``0x1001e951``) -- and reads the record's first
#: **dword** as the face's flags (``0x10060530``).  So the landscape's 32-bit
#: face mask is the file's *flags* word in its low half and its *surface* word
#: in its high half, and a query's world-level masks are turned into it by
#: ``Terrain.dll:0x10022da0``.  Two identities check the reading: the
#: landscape's ``0x2000``, world flag ``0x400``, is the flags word's
#: ``FLAGS_LIQUID_BED_BIT``, on exactly the 6102 bed faces, and its
#: ``0x20000``, world flag ``0x200``, is the surface word's
#: ``SURFACE_WATER_BIT``, on exactly the 3630 water faces.
#:
#: A unit's ground search and its contact points' searches ask for faces with
#: **world flags 0x208 and class bit 8 excluded** (``Control.dll:0x1001a687``,
#: ``0x1001ad7f``).  Beside the liquid surface that is landscape ``0x20`` and
#: ``0x40000`` -- the flags word's ``0x20`` and the surface word's ``0x04``.
#: **No shipped face carries either**: 0 of 275882 across all 33 maps.  See
#: ``docs/24-motion.md``, "Finding the ground".
FLAGS_NOT_GROUND_BIT = 0x0020
SURFACE_NOT_GROUND_BIT = 0x0004

#: Bit 1 of the face's surface word marks a water surface.  It is a *bitfield*,
#: not an enum: the observed values are 0, 2, 16 and 18, and testing ``== 2``
#: silently misses every water face that also carries bit 16.
SURFACE_WATER_BIT = 0x02

#: Bit 4 of the surface word is **clear on lava and on its bed, and set on
#: everything else**.  On the 29 maps that set it anywhere, the faces with it
#: clear are *exactly* the faces whose layer-1 material names lava -- all 6711
#: of them across the library, surfaces and beds alike.  The other four map
#: files never set it, and they contain no lava; ``LandMesh.marks_lava`` says
#: which kind a map is, because on those four a clear bit means nothing.
#:
#: Pooling the maps is what hid this: the four that never set the bit
#: contribute 23439 clear faces with no lava under them, which buries the 6711
#: that carry the signal and leaves "about 95% set, in connected regions".
SURFACE_NOT_LAVA_BIT = 0x10

#: Face flags word carried by every water face, on every map that has water.
#: An independent corroboration of SURFACE_WATER_BIT.
FLAGS_WATER = 1544

#: UV values are 8.8 fixed point and the layer-1 mapping tiles every 50 world
#: units, which is how ``u == x / 50`` comes out as ``u16 == x * 5.12``.
UV_FIXED_POINT_SCALE = 256.0

#: Field 13 is three 2-bit codes packed low to high, one per edge.  Edge `e`
#: of a face reads `(field13 >> 2 * e) & 3`, and the code is **the index of
#: the matching edge back in the neighbouring face** -- so a walker crossing
#: an edge arrives knowing which edge it came in by, without searching the
#: neighbour's three.  It holds on **817150 of 817150** shared edges across the
#: 33 maps.  `EDGE_NONE` is the code where there is no neighbour, and it agrees
#: with the adjacency field on all **827646** edge slots.
#:
#: That is a winged-edge structure, and the engine names it: `Terrain.dll`
#: carries `CTerrain::FindFaceInWing`.  The field's range of 0..62 is the
#: giveaway -- 63 would be a face with all three edges free, and no map has
#: one.  Earlier notes read the packed byte as a single number and looked for
#: spatial structure in it, which there is none of.
EDGE_TWIN_BITS = 2
#: The per-edge code for an edge with no face across it.
EDGE_NONE = 3

#: A face's own normal is int16 over this, the same scale a mesh pose key uses
#: for its quaternion.
NORMAL_SCALE = 32767.0


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
    #: The face's own normal, from fields 10..12 as int16 over 32767.  Unit
    #: length on 275881 of the 275882 shipped faces and agreeing with the
    #: geometric normal on 275877, so a renderer that wants flat shading has
    #: it without a cross product.
    face_normal: list[tuple[float, float, float]] = field(default_factory=list)
    layer1_names: list[str] = field(default_factory=list)
    layer2_names: list[str] = field(default_factory=list)
    #: The map's own spatial index, from stream 2; empty if it has none.
    cells: list[Cell] = field(default_factory=list)
    #: One entry per grid square, from stream 1: the cells that cover it.
    squares: list[tuple[int, ...]] = field(default_factory=list)
    #: The landscape's own grid, cells across and down.  ``CLandscape`` takes the
    #: first from the square stream's **second count field** in the NRes directory
    #: and divides the element count by it for the second
    #: (``Terrain.dll:0x100178e6``-``0x1001794f``).
    grid: tuple[int, int] = (0, 0)
    #: Stream 11: the order the map was baked to draw in.  See
    #: ``parse_draw_order``; the viewer buckets by material itself and does
    #: not use it.
    draw_order: list[int] = field(default_factory=list)
    #: The flags byte beside each draw-order entry; ``DRAW_BATCH_START`` says
    #: the face opens a batch.
    draw_flags: list[int] = field(default_factory=list)
    _grid: dict | None = field(default=None, repr=False, compare=False)

    @property
    def grid_size(self) -> tuple[int, int]:
        """How many cells across and down, from the distinct cell corners.

        It agrees with ``grid``, which the file states outright, on all 33 maps.
        """
        if not self.cells:
            return (0, 0)
        return (len({round(c.minimum[0], 1) for c in self.cells}),
                len({round(c.minimum[1], 1) for c in self.cells}))

    def cell_size(self) -> tuple[float, float]:
        """The landscape's own cell, in world units on x and y.

        The map's extent over its :attr:`grid`.  ``CLandscape`` keeps the
        reciprocal and finds the cell under a point by ``floor((x - x0) / cell)``,
        the stream-2 header's corner 0 being the origin
        (``Terrain.dll:0x1001775c``, ``0x10017c3f``, ``0x100205d5``).  It is a
        **per-map** figure -- 49.90 world units on map 41 to 311.28 on ``SC_3``
        -- and never a constant.
        """
        (lo, hi) = self.bounds()
        return tuple((hi[a] - lo[a]) / self.grid[a] if self.grid[a] else 0.0
                     for a in (0, 1))

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

    def edge_twin(self, face: int, edge: int) -> int | None:
        """Which edge of the neighbour across ``edge`` is the shared one.

        ``None`` where the edge has no neighbour.  This is field 13 unpacked;
        it saves a walker searching the neighbour's three edges for the one it
        came in by.
        """
        code = (self.face_patch[face] >> (EDGE_TWIN_BITS * edge)) & 3
        return None if code == EDGE_NONE else code

    def _side(self, a: int, b: int, x: float, y: float) -> float:
        """Twice the signed area of (a, b, point): positive left of a->b."""
        (ax, ay, _), (bx, by, _) = self.positions[a], self.positions[b]
        return (bx - ax) * (y - ay) - (by - ay) * (x - ax)

    def contains_xy(self, face: int, x: float, y: float) -> bool:
        """Whether a point lies in a face seen from above, edges included.

        Faces wind counter-clockwise from above, so inside is left of all
        three edges (``Terrain.dll:0x100225e0``).
        """
        v = self.faces[face]
        return all(self._side(v[e], v[(e + 1) % 3], x, y) >= 0 for e in range(3))

    def walk(self, face: int, start: tuple[float, float], end: tuple[float, float],
             limit: float = WALKABLE_NORMAL_Z) -> int | None:
        """The face under ``end``, walked to across edges from ``face``.

        The rule ``CWorld::FindWorldFace`` follows (``Terrain.dll:0x10026340``)
        as a unit's ground contact calls it, from the last ground point to the
        new sphere centre:

        * ``start`` must lie in ``face``;
        * a face whose normal z is not above ``limit`` ends the walk;
        * a face holding ``end`` is the answer;
        * otherwise cross edge ``e`` -- vertices ``e`` and ``e + 1`` -- where
          vertex ``e`` lies right of the line start->end and vertex ``e + 1``
          left of it, to the neighbour across that edge;
        * no such edge, no neighbour, or a 25th face gives ``None``, and the
          caller falls back to a vertical search.
        """
        if not self.contains_xy(face, *start):
            return None
        dx, dy = end[0] - start[0], end[1] - start[1]

        def side(vertex: int) -> float:
            px, py, _ = self.positions[vertex]
            return dx * (py - start[1]) - dy * (px - start[0])

        for _ in range(WALK_FACES):
            if self.face_normal[face][2] <= limit:
                return None
            if self.contains_xy(face, *end):
                return face
            v = self.faces[face]
            for e in range(3):
                if side(v[e]) < 0 < side(v[(e + 1) % 3]):
                    face = self.adjacency[face][e]
                    break
            else:
                return None
            if face == NO_NEIGHBOUR:
                return None
        return None

    @property
    def marks_lava(self) -> bool:
        """Whether this map uses the surface word's lava bit at all.

        Four of the 33 files never set it.  On those a clear bit says nothing,
        so asking ``is_lava`` of them would call the whole map lava.
        """
        return any(v & SURFACE_NOT_LAVA_BIT for v in self.face_surface)

    def is_lava(self, face: int) -> bool:
        """Whether a face is lava or the bed beneath it.

        False throughout on a map that does not mark lava -- and none of the
        four such maps has any.
        """
        return self.marks_lava and not self.face_surface[face] & SURFACE_NOT_LAVA_BIT

    def is_water(self, face: int) -> bool:
        """Whether a face is part of a water surface."""
        return bool(self.face_surface[face] & SURFACE_WATER_BIT)

    def is_ground(self, face: int) -> bool:
        """Whether a unit's ground search would look at this face.

        The search's filter excludes world face flags ``0x208`` and class bit
        8 (``Control.dll:0x1001a687``), which in the file are the surface
        word's water bit, the flags word's ``FLAGS_NOT_GROUND_BIT`` and the
        surface word's ``SURFACE_NOT_GROUND_BIT``.  The last two are set on
        **no shipped face**, so on the install this is "not water".
        """
        return not (self.face_surface[face] & (SURFACE_WATER_BIT | SURFACE_NOT_GROUND_BIT)
                    or self.face_flags[face] & FLAGS_NOT_GROUND_BIT)

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
        """Bucket faces into a coarse XY grid so height_at is not O(faces).

        Level 0 only: the coarse level covers the same ground and would put a
        second, simplified surface under every query.
        """
        (minx, miny, _), (maxx, maxy, _) = self.bounds()
        self._grid_origin = (minx, miny)
        self._grid_step = ((maxx - minx) / cells, (maxy - miny) / cells)
        self._grid_cells = cells
        grid: dict[tuple[int, int], list[int]] = {}
        sx, sy = self._grid_step
        for fi in self.lod_faces():
            tri = self.faces[fi]
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

    @property
    def lod_split(self) -> int:
        """First face of the coarse level; ``face_count`` if there is only one.

        The cells' runs chain end to end and the levels do not interleave, so
        each level is a single slice of ``faces``: level 0 is ``[0, split)``
        and level 1 ``[split, face_count)``, on all 33 maps.
        """
        per = len(self.cells) // LOD_COUNT
        return self.cells[per].first if per and len(self.cells) > per else self.face_count

    def lod_faces(self, level: int = 0) -> list[int]:
        """The face indices of one level of detail, in file order.

        The map is stored **twice**: every grid square names one cell per
        level, level 0 as authored and level 1 a simplification of it drawn
        from the same vertices.  Each level covers the whole map on its own --
        a random point of any map lands on exactly one face of each -- and
        level 1 is coarser in 6387 of the 7488 cell pairs and never finer.

        Drawing both is what made the ground flicker.  Where the simplifier
        left a triangle alone the two copies are bit-identical (a fifth to two
        fifths of the faces), but where it did not the two surfaces sit a
        fraction of a unit apart over gently sloping ground and z-fight; the
        old duplicate filter caught only the identical half.  Draw one level.
        """
        split = self.lod_split
        return list(range(0, split) if level == 0 else range(split, self.face_count))

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


def parse_draw_order(raw: bytes | None) -> list[int]:
    """Stream 11: the order to draw a map's faces in.

    Four bytes a face -- a ``uint16`` face index, a flags byte and a byte that
    is zero on 275566 of the 275882 faces.  The indices are a **permutation**
    of the whole face list on all 33 maps, and it is not an arbitrary one: it
    reorders faces only *within* a cell, keeping all 14976 cells contiguous,
    and inside each one it sorts them by texture pair.  In this order **every
    one of the 14976 cells draws in the minimum number of batches** -- no
    texture pair appears twice in a cell's run -- against 11463 in file order.

    The flags byte says where a batch begins: **bit 0x10 is set on exactly
    the 27174 faces that open a run of one texture pair inside a cell**, and
    clear on every one of the other 248708 -- so walking the order and
    changing material wherever the bit is set draws the map.

    So it is the draw order the map was baked with, and a renderer that
    buckets faces by material itself, as this one does, does not need it.
    """
    if not raw:
        return []
    count = len(raw) // DRAW_ORDER_STRIDE
    return [struct.unpack_from("<H", raw, i * DRAW_ORDER_STRIDE)[0]
            for i in range(count)]


def parse_draw_flags(raw: bytes | None) -> list[int]:
    """The flags byte beside each entry of the draw order.

    ``DRAW_BATCH_START`` is the one that is read: it opens a batch.
    """
    if not raw:
        return []
    count = len(raw) // DRAW_ORDER_STRIDE
    return [raw[i * DRAW_ORDER_STRIDE + 2] for i in range(count)]


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

    raw_draw = (archive.one_of_type(STREAM_DRAW_ORDER)
                if archive.has_type(STREAM_DRAW_ORDER) else None)
    raw_face = archive.one_of_type(STREAM_FACE)
    nf = len(raw_face) // FACE_STRIDE
    faces, adjacency = [], []
    flags, surface, tex1, tex2, patch = [], [], [], [], []
    face_normal: list[tuple[float, float, float]] = []
    for i in range(nf):
        r = struct.unpack_from("<14H", raw_face, i * FACE_STRIDE)
        faces.append((r[4], r[5], r[6]))
        adjacency.append((r[7], r[8], r[9]))
        flags.append(r[0])
        surface.append(r[1])
        signed = struct.unpack_from("<3h", raw_face, i * FACE_STRIDE + 20)
        face_normal.append(tuple(v / NORMAL_SCALE for v in signed))
        tex1.append(r[2] & 0xFF)
        tex2.append(r[2] >> 8)
        patch.append(r[13])

    square_entry = next(e for e in archive.entries if e.type_id == STREAM_SQUARES)
    across = square_entry.link_count
    grid = (across, square_entry.element_count // across if across else 0)

    return LandMesh(
        cells=parse_cells(archive.one_of_type(STREAM_BOUNDS)),
        squares=parse_squares(archive.one_of_type(STREAM_SQUARES)),
        grid=grid,
        draw_order=parse_draw_order(raw_draw),
        draw_flags=parse_draw_flags(raw_draw),
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
        face_normal=face_normal,
        layer1_names=_read_wea(path.parent / "Land1.wea"),
        layer2_names=_read_wea(path.parent / "Land2.wea"),
    )
