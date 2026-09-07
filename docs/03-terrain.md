# Land.msh — the terrain mesh

Each of the 33 directories under `DATA/MAPS/` holds one map:

```
DATA/MAPS/SC_3/
  Land.msh    NRes, 9 members  — the terrain mesh
  Land.map    NRes, 1 member   — 'ArealMap', unresolved (see 06-open-questions)
  Land1.wea   text             — texture-layer-1 name table
  Land2.wea   text             — texture-layer-2 name table
```

`Land.msh` is an NRes archive whose members all share the name `Land`; the
numeric **type id acts as a stream selector**. Every stream is a flat array
indexed by vertex or by face.

## Streams

| Type | Stride | Indexed by | Contents |
|---|---|---|---|
| 3 | 12 | vertex | position, `float32` x/y/z — **Z is up** |
| 4 | 4 | vertex | normal, `int8` x/y/z ÷ 127, plus one padding byte |
| 5 | 4 | vertex | layer-1 UV, `uint16` pair, 8.8 fixed point |
| 18 | 4 | vertex | layer-2 UV, same encoding |
| 14 | 4 | vertex | layer blend weight, `float32` in 0..1 |
| 21 | 28 | face | the face record, below |
| 11 | 4 | face | `(face index, flags)` |
| 2 | 12 | — | bounding geometry: 8 bbox corners, then more |
| 1 | — | — | unresolved; small, mostly `0xFF` |

## The face record (28 bytes, 14 × uint16)

| Field | Meaning |
|---|---|
| 0 | flags (4 distinct values across a map) |
| 1 | surface kind — **2 means water** |
| 2 | lo byte = layer-1 texture index, hi byte = layer-2 (`0xFF` = none) |
| 3 | always `0xFFFF` |
| 4, 5, 6 | vertex indices |
| 7, 8, 9 | adjacent face across each edge (`0xFFFF` = mesh boundary) |
| 10, 11, 12 | unresolved |
| 13 | patch / sector id (0..62 on SC_3) |

## The `.wea` name tables

A `.wea` file is a whitespace-delimited count followed by `index name` pairs:

```
5
0 B_S0
1 L04
2 L00
3 WATER_BOT
4 WATER
```

Face field 2's low byte indexes `Land1.wea` and its high byte indexes
`Land2.wea`. The same format is reused for mission skyboxes (`sky.wea` lists
`ENV_NEBULA_0`, `ENV_STARS`, `ENV_SUN_3`, `ENV_MOON`, …).

Most names resolve to entries in `Textures.lib` (`L04` → member `L04.0`).
Eight do not — `WATER`, `WATER_M`, `WATER_BOT`, `B_S0`, `B_MTP_01`,
`ENV_NLAVA`, `ENV_NLAVA_M`, `ENV_LAVA_BOT` — which are presumably animated or
procedural materials handled elsewhere.

## How each of these was established

Structural fields are easy to assert and hard to prove. These are the checks
that actually pin them down, all of them in `openparkan/verify.py`:

**Fields 4–6 are vertex indices.** Across all 33 maps every value lands inside
the vertex array, no triangle is degenerate, and the referenced set covers
100% of vertices with none left over.

**Fields 7–9 are face adjacency.** On SC_3, 13556 of 13803 slots hold a valid
face index and the other 247 are exactly `0xFFFF`; nothing else appears. The
decisive check is mutuality: **for all 33 maps, whenever face A names B as a
neighbour, B names A back.** A field that satisfies that is not a coincidence.

**Stream 5 is the layer-1 UV, in 8.8 fixed point, tiling every 50 world
units.** Correlation of the u component against world X is `+1.000` and of v
against world Y is `−1.000`, to three decimal places. The ratio `u16 / X` tops
out at exactly 5.12 = 256/50. Reconstructing `u = x/50` and `v = (maxY − y)/50`
leaves a worst-case residual of **0.004 texel units** over the whole map.

**Stream 4 is a normal, not a colour.** Reading the three bytes as *signed* and
dividing by 127 yields a unit vector at every vertex of every map — worst
deviation from unit length **0.0133**. Nothing but a normal does that.

**Field 1 = 2 means water.** The 88 faces so marked on SC_3 share a single Z
value: spread `0.000000`, a perfectly flat plane at z = 54.90. Their layer-1
texture index resolves through `Land1.wea` to the name `WATER`, and the faces
marked `WATER_BOT` are the lowest terrain on the map. The name table, the
geometry and the flag all agree.

## The strongest check: the game's own art

`ui/minimap.lib` contains a pre-rendered minimap for each map. Rasterising our
parsed terrain to a heightmap and correlating it against that shipped image
gives:

| Map | Pearson r |
|---|---|
| SC_3 | +0.883 |
| Tut_1 | +0.927 |
| ILKON | +0.965 |
| K1F | +0.901 |

For SC_3, the seven wrong flip/rotate orientations score +0.205 to +0.402, so
the match is unambiguous and it also fixes the convention: **minimaps are
stored top-down (screen order) while the world has +Y running north.**

## Usage

```
uv run openparkan maps
uv run openparkan heightmap SC_3 --out sc3.png
uv run openparkan viewer SC_3 Tut_1 --out viewer.html
```

## Scale

Maps are square, 798 to 2490 world units on a side, with 3000–10600 vertices
and 3100–9500 triangles. It is a coarse adaptive mesh — roughly one triangle
per 35 × 35 units on SC_3 — not a regular heightfield grid.
