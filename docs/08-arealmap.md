# Land.map — the navigation mesh

Each map's `Land.map` is an NRes archive with a single member of type 12 named
`ArealMap`. It holds a convex decomposition of the world into *areals*, the
adjacency between them, and a uniform grid that indexes them for point lookup.

This is the pathfinding structure. It is also the first format here recovered
by **disassembling `ArealMap.dll`** rather than by inference from data —
see [09-method.md](09-method.md) for why, and what that changes.

```
uv run openparkan verify        # includes six ArealMap checks
```

## Layout

```
areals × count                       count comes from the NRes directory entry,
                                     not from the payload
uint32   cells across, cells down    128 × 128 on every shipped map
for x in range(across):              note: x is the OUTER loop
    for y in range(down):
        uint16   item count
        uint16 × item count          indices of areals covering that cell
```

One areal:

```
0x00  float32  centre x, centre y
0x08  float32  0, 0
0x10  float32  area of the polygon
0x14  float32  0, 0, 1.0
0x20  uint32   flags: word 0 is 0 or 1; word 2 has all of 0xF0 set (240,
               242) on a lake and 1..29 elsewhere; words 1 and 3 are 0
0x30  uint32   vertex count V
0x34  uint32   sub-block count B          zero on every shipped map
0x38  float32  V × [3]                    polygon vertices
      int32    (V + 3B) × [2]             per edge: neighbour areal index and a
                                          second index whose role is unclear
      B × { uint32 n; float32 n × [3] }
```

So an areal is `56 + V*20` bytes when B is zero — which is exactly the 196-byte
stride visible in a hex dump of a 7-vertex areal, and the reason a fixed-stride
guess almost worked and then fell apart.

**A lake is its own areal.** Every areal whose third flag word has all of
`0xF0` set covers only water and liquid-bed faces, and every other areal is
ground apart from 12 shore faces (*measured* at the level-0 faces whose centre
lies in exactly one areal; `Areal.lake`). Tut_1 has 5 such areals among
378. What the low bits mean is not established; see
[24-motion.md](24-motion.md#lakes-in-the-areal-map--measured).

**The areal count is not in the payload.** It lives in the NRes directory
entry's element-count field at offset +4 — see [01-nres.md](01-nres.md), which
this discovery corrected.

Edge field 0 is the index of the areal across that edge, or `-1` on the
boundary of the mesh. **Field 1 is the twin edge** — the index of this same
edge in the neighbour's own list — so a path walker crossing it arrives
knowing which edge it came in by. On **193418 of 193418** shared edges it
names the edge in the neighbour that points straight back, and it is `-1` on
all 4280 boundary edges.

It had been set aside because it "ranges beyond the areal count, so it is not
a second areal reference". That was right and stopped one step short: it is an
*edge* reference, and an areal has up to 285 of them. It is the same
winged-edge link the land mesh keeps in face field 13, where it is packed into
two bits because a triangle has three edges; see
[03-terrain.md](03-terrain.md).

## What it looks like

Areals are mostly triangles and quads. On SC_3: 292 triangles, 120 quads, 46
pentagons, and a tail up to 14 vertices. Big polygons cover open ground and the
decomposition refines sharply along cliffs, the river canyon and the coastline
— the shape of the terrain is visible in the mesh.

## Verified by

Six checks in `uv run openparkan verify`, over all 33 maps:

- **The payload is consumed exactly** — 33/33, 34662 areals. This is the same
  check the engine makes; it panics with *"SystemArealMap panic: Incorrect
  ArealMap"* if the cursor does not land on the end.
- **The cell grid is 128 × 128** on every map.
- **Adjacency is mutual** — 33/33 maps. Whenever areal A names B across an
  edge, B names A back, which is what establishes edge field 0.
- **The stored area matches the polygon** — 34301 of 34662 areals agree with
  their own shoelace area within 2%.
- **Areals span the same extent as the terrain**, to within a unit.
- **Areals tile the map** — the areas sum to the full square, to within 1%, on
  every map. The decomposition has no gaps and no overlaps.
- **Every cell index is a real areal.**

Two more sit with the ground checks: **the four flag words take only the
values above** — 34662 areals — and **a lake areal holds only water and
liquid-bed faces**, with a dozen shore faces the only exception the other way.

## How the layout was found

`ArealMap.dll` exports named functions, and `CreateSystemArealMap` calls a
loader that panics with *"Failed Loading ArealMap"*. Inside it:

- it looks up **chunk type 12** and strides the resource directory by **64
  bytes** — independently confirming the NRes model derived from data;
- it reads the areal count from directory offset **+4**;
- for each areal it calls a per-areal load that skips **0x30** bytes of header,
  reads two counts, and advances the cursor by `V*12 + (V+3B)*8` plus the
  sub-blocks — giving the record layout above;
- it then reads two `uint32` grid dimensions and walks x-outer, y-inner reading
  a `uint16` count and that many `uint16` items per cell;
- it packs each cell into one 32-bit word: **low 22 bits = start index, top 10
  bits = item count** — matching the debug strings `CellStartIndex` and
  `CellItemNum`;
- finally it compares the cursor against the chunk size and panics on a
  mismatch.

The engine's own strings name the concepts: `Areals`, `cells`, `BrokenAreal`,
`SubAreals`, `MigrationAreals`, and *"MHallWay panic: cannot load path graph"*.
