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
| 14 | 4 | vertex | weight of layer 1, `float32` in 0..1 |
| 21 | 28 | face | the face record, below |
| 11 | 4 | face | `(face index, flags)` |
| 2 | 68 | cell | the spatial index: 8 bbox corners, then a box and a face run per cell |
| 1 | 38 | square | the square table over those cells |

## The face record (28 bytes, 14 × uint16)

| Field | Meaning |
|---|---|
| 0 | flags; the value **1544** (`0x0608`) marks water |
| 1 | surface bitfield; bit **`0x02`** marks water |
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

**Water is marked by bit `0x02` of field 1 — and field 1 is a bitfield, not an
enum.** This is worth stating carefully because the obvious reading is wrong:
the observed values are 0, 2, 16 and 18, so testing `field1 == 2` silently
misses every water face that also carries bit 16, and holds on only 5 of the
33 maps. Testing `field1 & 2` matches the water faces **exactly on all 33
maps** — 3630 faces across the 11 maps that have any.

Three independent signals agree on every map, which is what makes this solid:

- `field1 & 0x02` is set,
- field 0 equals `1544` — an entirely separate marker, same face set,
- the layer-1 texture index resolves through `Land1.wea` to `WATER`,
  `WATER_M`, `ENV_NLAVA` or `ENV_NLAVA_M`.

And the geometry corroborates it: on all 11 maps with water, every water
vertex shares a single Z — a perfectly flat plane (z = 54.90 on SC_3, spread
`0.000000`). The faces textured `WATER_BOT` are the lowest terrain on the map,
sitting under it.

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

## The second texture layer

A face names two textures — the low and high bytes of face word 2 — and a
vertex carries UVs for both, in streams 5 and 18. Between 8% and 30% of a
map's faces carry a second layer; 32450 of 275882 across all 33 maps.

Stream 14 is **the weight of layer 1**, and the proof is a clean split: it is
exactly 1.0 on all 258046 vertices that no layer-2 face touches, and below 1.0
on 19663 of the 41404 that one does. Nothing else in the mesh separates so
cleanly on that boundary.

So the ground is `mix(layer2, layer1, blend)`. Without it every texture
boundary on the terrain is a hard polygon edge; the game's are gradients.

Draw it in **one pass**. Grouping faces by the pair `(layer 1, layer 2)`
rather than by layer 1 alone costs almost nothing — 5 to 8 groups per map
against 3 to 5 — and then one material can sample both. Drawing layer 2 as a
second, coplanar mesh instead makes the ground flicker: the two passes compile
to different shader programs, their interpolated depths come out a hair apart,
and the walkable areas z-fight as the camera moves.

## Terrain layers name materials, not textures

`Land1.wea` and `Land2.wea` hold **material** names, and they go through
`Material.lib` exactly as a model's wear does. That is what makes the eight
names an earlier draft listed as unresolvable resolve: `WATER` is not in
`Textures.lib`, but the material of that name points at `WATER0.0`, `B_S0`
points at `B_FOUND.0` and `ENV_NLAVA` at `LAV00.0`. All **270 layer names
across the 33 maps** reach a texture this way, against 193 by direct lookup.

Two things follow that a flat-colour stand-in was hiding:

- **Water is animated.** `WATER_M` is a ten-key animation, `WATER0.0`
  through `WATER9.0`, all palettised 64x64 ripple patterns.
- **Water is blue because its material is.** The texture is neutral grey; the
  colour is the material's `#4d6aff` diffuse, which multiplies it.

Water is drawn see-through here. The material declares no transparency, but
every map that has water also carries a `WATER_BOT` material on the ground
beneath it, and a lake bed nobody can see would not be worth authoring. The
exact figure is a renderer choice.

## Streams 1 and 2 are the map's own spatial index

Two streams, and together they are a **flat grid with a per-square list** —
not the quadtree the 4x size step between the small and large maps suggests.
There is no hierarchy in either.

### Stream 1: the square table

One 19-`uint16` record per grid square: four words of header, then room for
**15 cell indices** with `0xFFFF` for an empty slot. It divides exactly on all
33 maps — 64 records for the 8 x 8 maps, 256 for the 16 x 16.

Every square uses **exactly two** of its fifteen slots, all 7488 of them, and
they are its own two: square `i` names cells `i` and `squares + i`, and that
pair always shares a bounding box. The other thirteen slots are a capacity the
shipped data never needs.

The four header words are `0, 0xFFFF, 0, 0` on every one of the 7488 records,
so nothing in the data tells them apart. `0xFFFF` is this format's usual
"none", as it is for a face's neighbour and a node's slot.

### Stream 2: the cells

96 bytes of header — **the eight corners of the mesh's bounding box**,
exactly — then 44 bytes that are zero on every map, then one 68-byte record
per cell:

```
uint16   first        the first face of this cell's run
uint16   count        how many
float32  0
float32  min[3]       the cell's box
float32  max[3]
float32  centre[3]
float32  radius       of the sphere around the box
float32  0 x5
```

It **parses with nothing left over on all 33 maps**. The runs chain end to
end — `first + count` is the next record's `first` on every one — and their
counts **sum exactly to the face count**. Which means the faces are stored in
cell order and a run indexes `faces` directly, with no permutation table in
between.

The proof is total: **all 275882 faces across the 33 maps have every one of
their three vertices inside the box of the cell whose run holds them.** Not
most; all.

The grid is uniform and its resolution follows the mesh rather than the
world:
**16 x 16 on 28 maps and 8 x 8 on the other five**, which is why `SC_3` at
2490 units gets 8 x 8 while map 11 at 998 units gets 16 x 16 — SC_3 has 5692
vertices and map 11 has 6458.

Each cell is listed **twice**, and the two records are the same ground at two
**levels of detail** — see below. They share a box, and the second holds no
more faces than the first on all 7488 pairs.

## Face field 13 is not a patch id

An earlier draft read the last word of the face record as a patch or sector
id, on the strength of its range (0..62, about 57 distinct values per map).
It is not. Grouping faces by it gives regions that **span the whole map**:
only 25 of 348 groups across six maps are even 20% tighter than a random
subset of the same size, and the median group covers 100% of the map either
way.

It is also interleaved in face order rather than run-length, does not
determine the texture pair or the surface word, and does not track elevation.
Its groups are wildly uneven — 1, 2, 4 and 384 faces on SC_3. So field 13 is
not what a renderer would cull by — but the map does carry a grid, in
[stream 2](#stream-2-is-the-maps-own-spatial-index), which is.

## The `*M` twin is the material's second track, drawn unlit

43 ground materials name a texture and its `M` twin, and every pair is the
same size with the base in **RGB565** and the twin in **XRGB8888**. They hold
the same picture — correlation 0.94 to 1.00 — but not the same colours.

What the `M` half is becomes obvious once you measure the distance from
neutral grey rather than the brightness. **On 40 of the 43 pairs the `M`
texture sits closer to grey than the base**, and the ones that move furthest
are the ones that started furthest away: `L25`, a near-white snow, goes from a
mean distance of 110 to 12; `L23` from 98 to 15. The three exceptions differ
by about a point. Saturation drops on 36 of 42, and a per-channel least
squares fit of `M = a·base + b` lands a residual of only 2 to 15 levels out of
255 — so the twin is the base put through a per-texture brightness and
contrast, aimed at mid-grey.

**The two are not two layers of one draw.** A material's second `uint16`
counts *animation tracks*, not layers — see
[07-objects.md](07-objects.md#the-second-count-is-animation-tracks-not-layers)
— and on all 43 the two tracks hold a single key each, track 0 naming entry 0
and track 1 naming entry 1. A caller asking for track 1 gets one texture,
exactly as a caller asking for track 0 does. Nothing in the record binds them
together.

What separates them is the **lighting**. An entry is a `D3DMATERIAL7`, and on
38 of the 43 the first carries a **white diffuse over a black ambient** and
the second the reverse — a **black diffuse over a white ambient**. Under
fixed-function lighting that is the difference between a surface the scene
light multiplies and one that shows at full brightness whatever the lighting.
The base is lit; **the twin is drawn unlit**, which is exactly what a copy
flattened towards mid-grey is for: the contrast a light would have supplied is
already baked out of it.

They are not bump maps, which an earlier draft guessed from `Iron_3D.ini`'s
`EMBM=1`; not a mask, which the reader used to call them; and not a second
texture stage, which the draft before this one called them. Nor are they
simply the 16-bit and 32-bit variants of one texture: a bit-depth copy would
not be pulled towards grey, and would not be pulled hardest exactly where the
original is least grey.

The engine's multitexture path is real, and this is not it. `Ngi32.dll` keeps
a table of 14 render phases at `0x10036a30`, several of them two-texture, and
`CShade::ConfigureTextureAndAlphaBlendModes` asks the device which of them it
supports at start-up; where the two-texture `MODULATE` phase is missing the
engine falls back to a second pass. But a `MAT0` entry carries **one** texture
and one cell, and the material manager hands back one entry at a time — so no
material ever reaches that path. See
[05-engine.md](05-engine.md#the-render-phase-table).

**What is still open is who asks for track 1.** The selector is an argument to
the material manager's `GetMaterialPhase`, and no caller of it has been found;
the reader takes track 0, which is the lit half.

## The map is stored twice, at two levels of detail

Every grid square names two cells, and they are not two patches of ground:
they are the **same** ground, twice. Level 0 is the mesh as authored and level
1 a simplification of it, and a renderer draws one of them per cell.

Four things say so, and each is checked on all 33 maps:

- **The levels do not interleave.** The cells' runs chain end to end and level
  0's cells come first, so each level is a single slice of the face array:
  level 0 is `[0, split)` and level 1 the rest, where `split` is the first
  face of cell `squares`.
- **Each level covers the whole map on its own.** A random point of any map
  lands inside exactly one face of level 0 and exactly one of level 1. Their
  plan-view areas each come to 1.00 of the map's footprint.
- **Level 1 introduces no new vertices.** Every vertex it uses, level 0 uses
  too — which is what a mesh simplifier produces and what a second patch of
  ground would not.
- **Level 1 is coarser.** It holds no more faces than level 0 in any of the
  **7488** cell pairs, fewer in 6387 of them, and 102055 faces against 173827
  across the library.

### Drawing both is what made the ground flicker

Where the simplifier left a triangle alone the two copies are bit-identical —
**46186** of level 1's faces repeat a level-0 triangle exactly, between a
fifth and two fifths of a map. Those are easy to spot and an earlier draft
filtered them, calling them "duplicated faces" and guessing the engine "draws
one patch or the other". Half right: the other **55869** are the faces the
simplifier *did* change, and they are not identical to anything — they sit a
fraction of a unit from the surface they replace.

That residue is what a viewer sees as flicker. Over gently sloping ground the
two surfaces graze each other and z-fight along thin wandering seams; over
steep ground they part company by up to 70 units and the coarse level breaks
through the fine one as streaks along the ridges. The faces in the thin band
sit on a median slope of 14° to 33°, the ones that separate grossly on 24° to
49° — which is why the flat basins flickered and the mountains looked noisy
rather than doubled.

`LandMesh.lod_faces(0)` returns the fine level and the viewer draws that:
**173827 triangles instead of 275882**, one surface everywhere, and no seams.
Both levels are packed — the coarse one costs only its indices, since its
vertices are a subset of the fine one's — and the **Coarse ground** button
switches between them. A game renderer would switch per cell by distance; a
survey camera frames the whole map, so here it is a toggle.

An earlier deduplication pass — keep the first face of each set of triangles
with identical positions — is gone. It removed exactly the 46186 the
simplifier had not touched and kept every one that flickered.
