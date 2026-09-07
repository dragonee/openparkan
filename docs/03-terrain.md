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
| 2 | 12 | — | bounding geometry: 8 bbox corners, then more |
| 1 | — | — | unresolved; small, mostly `0xFF` |

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

- **Water is animated.** `WATER_M` is one layer of ten frames, `WATER0.0`
  through `WATER9.0`, all palettised 64x64 ripple patterns.
- **Water is blue because its material is.** The texture is neutral grey; the
  colour is the material's `#4d6aff` diffuse, which multiplies it.

Water is drawn see-through here. The material declares no transparency, but
every map that has water also carries a `WATER_BOT` material on the ground
beneath it, and a lake bed nobody can see would not be worth authoring. The
exact figure is a renderer choice.

## Face field 13 is not a patch id

An earlier draft read the last word of the face record as a patch or sector
id, on the strength of its range (0..62, about 57 distinct values per map).
It is not. Grouping faces by it gives regions that **span the whole map**:
only 25 of 348 groups across six maps are even 20% tighter than a random
subset of the same size, and the median group covers 100% of the map either
way.

It is also interleaved in face order rather than run-length, does not
determine the texture pair or the surface word, and does not track elevation.
Its groups are wildly uneven — 1, 2, 4 and 384 faces on SC_3. A renderer that
wants to cull terrain has to build its own grid.

## The `*M` textures are a second copy, not a bump map

42 of the ground textures ship as a pair: `L20.0` in RGB565 and `L20M.0` in
XRGB8888, and the material for `L20` names both as its two layers. The `M`
half is **the same image**: correlation between the two is 0.994 median, 0.897
at worst, over all 42 pairs. What differs is colour depth and exposure — the
brightness ratio runs from 0.54 to 2.03 and was clearly authored per texture,
not applied as a gain.

So they are not detail layers and not bump maps, which is what an earlier
draft assumed from `Iron_3D.ini`'s `EMBM=1`. They read as the 16-bit and
32-bit variants of one texture, and `Iron_3D.ini` carries both `BITDEPTH` and
`RENDER_QUALITY`, which is presumably what chose between them. Which index
goes with which setting is not established, so the reader takes layer 0.

## A large minority of faces are stored twice

**46283 of the 275882 faces across the 33 maps repeat a triangle already in
the list** — bit-identical positions, in 46215 coincident sets, almost all of
them pairs. Per map it runs from a sixth to a fifth of the faces, and it is
not spread evenly: on map 23, **84% of the flat `L32` ground is duplicated**
against 20% of the surrounding `L33`, and duplicated faces are flatter than
the rest (median height range 11.8 against 22.6). Those flat areas are the
walkable ground.

The two copies are the same surface. Layer-1 UVs match on **all 46215** sets,
normals on 46126, winding on every one, and they always agree on whether the
face has a second layer. What differs is incidental: the layer-2 UVs on about
60% of pairs, the per-vertex blend on about 18%, and the face's patch word on
about an eighth.

Why the file is like this is not established — the engine presumably has each
copy in a different patch and draws one or the other, never both. What matters
for a renderer is that **drawing the list as it stands draws those triangles
twice at the same depth**, and they z-fight: the walkable ground flickers as
the camera moves. `LandMesh.distinct_faces()` returns the first face of each
set, in file order, and the viewer draws that.
