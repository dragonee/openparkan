"""Reader for ``Land.map`` -- the navigation mesh, called an *areal map*.

A map's ``Land.map`` is an NRes archive holding a single member of type 12
named ``ArealMap``.  It carries a convex-polygon decomposition of the walkable
world (the *areals*), the adjacency between those polygons, and a uniform
128 x 128 grid that indexes them for point lookup.

The layout was recovered from ``ArealMap.dll`` rather than by inference; see
``docs/08-arealmap.md``.  The number of areals is not in the payload at all --
it lives in the NRes directory entry's element-count field.

Payload layout::

    areals x count            (count from the directory entry)
    uint32   cells across, cells down        (128 x 128 on every shipped map)
    for x in range(across):                  note: x is the outer loop
        for y in range(down):
            uint16 item count
            uint16 x item count              areal indices covering that cell

and one areal is::

    float32  centre x, centre y
    float32  0, 0
    float32  area of the polygon
    float32  0, 0, 1.0
    uint32   1, 0, 1, 0
    uint32   vertex count V
    uint32   sub-block count B               (zero on every shipped map)
    float32  V x [3]                         polygon vertices, counter-clockwise
    int32    (V + 3B) x [2]                  per-edge: neighbour areal, and the
                                             index of the same edge in that
                                             neighbour (-1 on the boundary)
    B x { uint32 n; float32 n x [3] }

so an areal occupies ``56 + V*20`` bytes when B is zero.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

from .nres import NResArchive

AREAL_MAP_TYPE = 12
HEADER_SIZE = 0x38

#: Neighbour index used on an edge that has no areal on the far side.
NO_NEIGHBOUR = -1


@dataclass
class Areal:
    """One convex cell of the navigation mesh."""

    centre: tuple[float, float]
    area: float
    vertices: list[tuple[float, float, float]]
    #: One per vertex: ``(neighbour areal, twin edge)``.  The twin is the
    #: index of this same edge in the neighbour's own list, so a path walker
    #: arrives knowing which edge it crossed -- the winged-edge link the land
    #: mesh keeps in face field 13, in a whole ``int32`` here.  Both are
    #: ``NO_NEIGHBOUR`` on the outside of the mesh.
    edges: list[tuple[int, int]]
    flags: tuple[int, int, int, int]

    def twin(self, edge: int) -> tuple[int, int] | None:
        """``(neighbour areal, its edge)`` across ``edge``, or None outside."""
        neighbour, back = self.edges[edge]
        return None if neighbour == NO_NEIGHBOUR else (neighbour, back)

    @property
    def neighbours(self) -> list[int]:
        return [n for n, _ in self.edges if n != NO_NEIGHBOUR]

    def polygon_area(self) -> float:
        """Shoelace area of the stored polygon, for checking against ``area``."""
        total = 0.0
        n = len(self.vertices)
        for i in range(n):
            x1, y1, _ = self.vertices[i]
            x2, y2, _ = self.vertices[(i + 1) % n]
            total += x1 * y2 - x2 * y1
        return abs(total) / 2.0


@dataclass
class ArealMap:
    source: Path
    areals: list[Areal]
    cells_across: int
    cells_down: int
    #: ``(x, y) -> tuple of areal indices`` for every grid cell.
    cells: dict[tuple[int, int], tuple[int, ...]]

    @property
    def areal_count(self) -> int:
        return len(self.areals)

    def bounds(self) -> tuple[tuple[float, float], tuple[float, float]]:
        xs = [v[0] for a in self.areals for v in a.vertices]
        ys = [v[1] for a in self.areals for v in a.vertices]
        return (min(xs), min(ys)), (max(xs), max(ys))

    def cell_of(self, x: float, y: float) -> tuple[int, int] | None:
        """Grid cell containing a world point, or None if outside."""
        (minx, miny), (maxx, maxy) = self.bounds()
        if not (minx <= x <= maxx and miny <= y <= maxy):
            return None
        cx = min(int((x - minx) / (maxx - minx) * self.cells_across), self.cells_across - 1)
        cy = min(int((y - miny) / (maxy - miny) * self.cells_down), self.cells_down - 1)
        return cx, cy

    def areals_at(self, x: float, y: float) -> tuple[int, ...]:
        """Candidate areal indices for a world point, via the grid."""
        cell = self.cell_of(x, y)
        return self.cells.get(cell, ()) if cell else ()


class ArealMapFormatError(ValueError):
    pass


def _read_areal(data: bytes, pos: int) -> tuple[Areal, int]:
    centre = struct.unpack_from("<2f", data, pos)
    area = struct.unpack_from("<f", data, pos + 0x10)[0]
    flags = struct.unpack_from("<4I", data, pos + 0x20)
    nverts, nblocks = struct.unpack_from("<2I", data, pos + 0x30)
    p = pos + HEADER_SIZE

    vertices = [struct.unpack_from("<3f", data, p + i * 12) for i in range(nverts)]
    p += nverts * 12

    edge_count = nverts + 3 * nblocks
    edges = [struct.unpack_from("<2i", data, p + i * 8) for i in range(edge_count)]
    p += edge_count * 8

    for _ in range(nblocks):
        n = struct.unpack_from("<I", data, p)[0]
        p += 4 + n * 12

    return Areal(centre, area, vertices, edges, flags), p


def load(path: str | Path) -> ArealMap:
    """Parse a ``Land.map``.

    Raises ArealMapFormatError unless the payload is consumed exactly, which is
    the same consistency check the engine makes.
    """
    path = Path(path)
    archive = NResArchive.open(path)
    matches = [e for e in archive if e.type_id == AREAL_MAP_TYPE]
    if len(matches) != 1:
        raise ArealMapFormatError(f"{path}: expected one ArealMap member, found {len(matches)}")
    entry = matches[0]
    data = archive.read(entry)

    areals = []
    pos = 0
    for _ in range(entry.element_count):
        areal, pos = _read_areal(data, pos)
        areals.append(areal)

    across, down = struct.unpack_from("<2I", data, pos)
    pos += 8
    if not across or not down:
        raise ArealMapFormatError(f"{path}: empty cell grid {across}x{down}")

    cells: dict[tuple[int, int], tuple[int, ...]] = {}
    for x in range(across):
        for y in range(down):
            count = struct.unpack_from("<H", data, pos)[0]
            pos += 2
            cells[(x, y)] = struct.unpack_from(f"<{count}H", data, pos)
            pos += count * 2

    if pos != len(data):
        raise ArealMapFormatError(
            f"{path}: parsed {pos} bytes of a {len(data)}-byte ArealMap"
        )
    return ArealMap(path, areals, across, down, cells)
