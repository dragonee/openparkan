# Land.msh — the terrain mesh

Each of the 33 directories under `DATA/MAPS/` holds one map:

```
DATA/MAPS/SC_3/
  Land.msh    NRes, 9 members  — the terrain mesh
  Land.map    NRes, 1 member   — 'ArealMap', unresolved (see 06-open-questions)
  Land1.wea   text             — the materials a face names
  Land2.wea   text             — index for index, the materials their microtextures come from
```

`Land.msh` is an NRes archive whose members all share the name `Land`; the
numeric **type id acts as a stream selector**. Every stream is a flat array
indexed by vertex or by face.

## Streams

| Type | Stride | Indexed by | Contents |
|---|---|---|---|
| 3 | 12 | vertex | position, `float32` x/y/z — **Z is up** |
| 4 | 4 | vertex | normal, `int8` x/y/z ÷ 127, plus one padding byte |
| 5 | 4 | vertex | the material's UV, `uint16` pair, 1024 to one turn of the texture |
| 18 | 4 | vertex | the microtexture's UV, same encoding |
| 14 | 4 | vertex | alpha of a face's second material, `float32` in 0..1 |
| 21 | 28 | face | the face record, below |
| 11 | 4 | face | `(face index, flags)` |
| 2 | 68 | cell | the spatial index: 8 bbox corners, then a box and a face run per cell |
| 1 | 38 | square | the square table over those cells |

## The face record (28 bytes, 14 × uint16)

| Field | Meaning |
|---|---|
| 0 | flags over a constant `0x600`; `0x004` a second layer, `0x008` water (**1544**, `0x0608`), `0x2000` a liquid bed |
| 1 | surface bitfield; bit **`0x02`** marks water, bit `0x10` is clear on lava |
| 2 | lo byte = the face's material, hi byte = a second material drawn over it (`0xFF` = none); both index `Land1.wea` |
| 3 | always `0xFFFF` |
| 4, 5, 6 | vertex indices |
| 7, 8, 9 | adjacent face across each edge (`0xFFFF` = mesh boundary) |
| 10, 11, 12 | the face's own normal, `int16` over 32767 |
| 13 | the winged-edge link: three 2-bit edge codes |

Each row is established in the sections below; the table used to call 10–12
unresolved and 13 a patch id.

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

**Both bytes of face field 2 index `Land1.wea`.** `Land2.wea` is the same
length on all 33 maps and names, index for index, the material each
`Land1.wea` material takes its **microtexture** from
([below](#the-microtexture-is-land2weas--read-and-measured)); an earlier
reading here had the high byte index it, which
[the faces themselves refute](#a-faces-second-material--read-and-measured).
The same format is reused for mission skyboxes (`sky.wea` lists
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

**Stream 5 is the material's UV, laid flat over the world.** Correlation of
the u component against world X is `+1.000` and of v against world Y is
`−1.000`, to three decimal places, and on SC_3 the ratio `u16 / X` tops out at
exactly 5.12. That was first read as 8.8 fixed point, 256/50, a texture
tiling every 50 world units; the data cannot tell 256 from any other unit,
and the binary's is **1024**
([The UV unit is 1024](#the-uv-unit-is-1024--read-and-measured)): 5.12 is
1024/200, and SC_3's ground texture turns once in **200** world units.
Reconstructing `u = x/200` and `v = (maxY − y)/200` leaves a worst-case
residual of 0.001 of a turn over the whole map.

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

<a id="the-second-texture-layer"></a>

## The UV unit is 1024 — *read*, and *measured*

`Terrain.dll` keeps one constant for every `uint16` texture coordinate it
expands, 1 ÷ 1024 (`0x10035070` stores it at `0x100a5c6c` from the 1024.0 at
`0x1009a950`), and each of the eight strided expansions scales the cell's
width and height by it before it multiplies a coordinate
(`0x10038876`–`0x1003889d` and its seven likes). The landscape's own face
query makes its float coordinates the same way (`0x1001ae50`, the constant at
`0x100a5a04` from `0x100227cf`). An object mesh's are over 1024 too
([07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured)).

*Measured* on the 33 maps, the gradient of each stream across a face in
`uint16` a world unit:

- **stream 18 is 51.2 on every map** (51.19 to 51.20). At 1024 to the turn
  that is 0.05 a world unit, one turn in **20 units** — and 0.05 is the
  compiled value of the setting `MicroTexScale`, which nothing reads at run
  time ([10-sky.md](10-sky.md#the-render-settings)): the scale is baked into
  the stream. The values wrap at 65536, which is 64 whole turns, so the wrap
  does not show;
- **stream 5 varies by map**, one turn in 70 to 513 units: 4.0 on `KM_6bis`
  (256 units), 5.12 on `SC_3` and `KM_4` (200), 6.04 on maps 31 and 32 (170).

So a ground material's own texture is a **large** one — a 256-pixel image
across a couple of hundred units, about a texel a unit — and the fine grain
comes from the microtexture over it. The engine and the notes here read both
at 256 until this was read, which tiled the ground's texture sixteen times
too often and made of it a fine, even pattern; the recordings show broad
shapes ([below](#how-the-landscape-is-lit-and-fogged--read-and-measured)).

## A face's second material — *read*, and *measured*

A face names two materials — the low and high bytes of face word 2 — and
between 8% and 30% of a map's faces carry the second; 32450 of 275882 across
all 33 maps.

**The second is drawn over the first, on stream 14's alpha.** `CShade`'s cell
draw builds one item for the face's first material and, where the second byte
is not `0xFF` (`0x100445d1`–`0x100445e4`), hands it to `0x1002c000`
(`0x10043aef`), which makes a copy of the item — the same vertices, normals
and both coordinate streams (`0x1002c02d`–`0x1002c044`) — with:

- the material at the **second byte's index in the same manager**, track 0
  (`0x1002c094`–`0x1002c0ba`; the manager is the one the landscape's own draw
  hands the shade, loaded from `Land1.wea`, `0x1001b937`);
- blend mode `CShade+0xbf0`, which is translate index 1, mode 4:
  `SRCALPHA`/`INVSRCALPHA` (`0x1002c0d7`, [below](#the-ground-draws-opaque--read-and-measured));
- alpha mode 2 (`0x1002c0f2`), in which the shade makes a vertex's alpha the
  material's ambient alpha times the item's alpha stream
  ([10-sky.md](10-sky.md#the-lit-colour-is-the-games-own--read-and-measured)),
  and the alpha stream is **stream 14** (the record's pair at `+0x58`,
  `0x100449e5`–`0x100449fa`);
- and a place in the queue's layer 2 (`0x1002c170`), after layer 0 where the
  first item goes, whose own alpha stream is then taken away (`0x1002c182`).

So the ground is `mix(first, second, stream 14)`: the first material opaque,
the second over it. This page used to say the reverse — stream 14 "the weight
of layer 1" and the second name looked up in `Land2.wea` — from the one thing
the data shows without the binary, that the stream is exactly 1.0 on all
258046 vertices no two-material face touches.

*Measured*, and it settles both points without the binary: take every edge a
two-material face shares with a one-material face, at the fine level of all
33 maps — **15011** of them. On **7554** both ends carry 1.0 and the
neighbour's material is the face's **second** byte; on **7436** both carry
0.0 and the neighbour's material is its **first**; 21 are neither. So 1.0 is
the second material whole, where the face meets ground that wears it, and the
second byte names the same table the first does: all **17641** second bytes
of the fine level lie inside `Land1.wea`. (A vertex on such an edge is stored
twice, once for each side, which is how "1.0 on every vertex no two-material
face touches" and "0.0 at a patch's edge" are both true.)

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

**Water is opaque**, and so is every other layer:
[The ground draws opaque](#the-ground-draws-opaque--read-and-measured). The
`WATER_BOT` bed beneath it is not a hint that a lake is see-through; it is
what the camera sees from under the water, and what the reflection would show
if the lake were drawn from below ([Water reflects](#water-reflects--read-and-measured)).
The HTML viewer draws its lakes see-through anyway, which is a viewer's
licence rather than a reading.

**The layer-1 material is also the ground a unit feels.** Its `MAT0` class
byte is the surface id and its dword a damage rate. `WATER_BOT` and
`ENV_LAVA_BOT` are surface 1 at 10000 hit points a second, so every liquid-bed
face (flag `0x2000`) is lethal ground; `WATER` is surface 7 and `ENV_NLAVA` is
unset. A face whose normal z is not above 0.173648 — steeper than 80° — is
never taken as ground. See
[24-motion.md](24-motion.md#ground-and-collision--read-and-measured).

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

### The file states the grid, and the cell size falls out of it — *read*, and *measured*

The resolution is not something a reader has to infer from the corners: it is
in the **NRes directory**. `CLandscape`'s constructor reads stream 1's entry
and takes **cells across from the entry's second count field** — the one
[01-nres.md](01-nres.md) calls the link count — and **cells down from its
element count divided by that** (`Terrain.dll:0x100178e6`–`0x1001794f`). It
then takes **cell 0's box** out of stream 2, keeps its x and y sizes, and
stores their reciprocals (`0x10017bf4`–`0x10017c78`), the box's corner 0 being
the grid origin (`0x10017849`). Every face lookup is then
`floor((x − x₀) × inv)` on each axis (`0x100205d5`, and the same in
`GetWalkFace`).

So **the landscape's cell size is the map's extent over its own grid**, and it
is per-map. Measured over all 33 maps (`openparkan verify`):

- the stated grid equals the one the distinct cell corners give, **33 of 33** —
  `(16, 16)` on 28 and `(8, 8)` on 5;
- cell 0's box is exactly the extent over that grid, **33 of 33**;
- the cell runs from **49.90 world units on map 41** to **311.28 on `SC_3`**,
  22 distinct values, and the x and y cells are equal on every map because
  every map is square.

`LandMesh.grid` and `LandMesh.cell_size()` carry it, and the engine's ground
index is built on it rather than on a constant of its own.

Each cell is listed **twice**, and the two records are the same ground at two
**levels of detail** — see below. They share a box, and the second holds no
more faces than the first on all 7488 pairs.

## The face record's last unread fields

**Fields 10, 11 and 12 are the face's own normal**, `int16` over 32767 — the
same fixed point a mesh pose key uses for its quaternion. They are unit length
on **275881 of the 275882** faces and point the same way as the cross product
of the triangle on **275877**, so flat shading needs no cross product and the
winding is confirmed a third time. `LandMesh.face_normal` carries them.

**Field 0 is a bitfield over a constant `0x600`**, and two of its bits are
now named. `0x008` marks water, which was known. `0x004` marks a face that
carries a **second texture layer**: it is set on exactly the **32450** faces
whose layer-2 index is not `0xFF` and on none of the other 243432. So a face
says twice that it has a second layer — in its flags and in its texture word —
and the values that puzzled an earlier draft fall out: `1536` is the base,
`1540` adds the second layer, `1544` adds water.

`0x2000` marks the **bed beneath a liquid**: it is set on exactly the **6102**
faces whose layer-1 material is `WATER_BOT` or `ENV_LAVA_BOT`, and on no other
face of any map. So the ground under a lake says so itself — the third way a
map marks its liquids, beside the surface bit and the flags value 1544.

### Field 13 is the winged-edge link

Three 2-bit codes packed low to high, one per edge. Edge `e` reads
`(field13 >> 2e) & 3`, and the code is **the index of the matching edge back
in the neighbouring face** — so a walker crossing an edge arrives knowing
which edge it came in by, without searching the neighbour's three. Code 3
means there is no neighbour.

It holds on **817150 of 817150** shared edges across the 33 maps, and the
no-neighbour code agrees with the adjacency field on all **827646** edge
slots. The field's range is the confirmation: 63 would be a face with all
three edges free, and **no map has one** — 265635 faces have no free edge,
9998 have one and 249 have two.

That is a winged-edge structure, and the engine names it — `Terrain.dll`
carries `CTerrain::FindFaceInWing`. The code was found in the building-
insertion path: at `Terrain.dll:0x1000c309` the engine sets an edge's
adjacency to `0xFFFF` and immediately writes 3 into that edge's slot, through
three branches that mask `0xFC`, `0xF3` and `0xCF` — a byte written as three
2-bit fields, indexed by the same 0/1/2 that indexes adjacency.

Earlier notes read the packed byte as a single number and hunted for spatial
structure in it: "0..62, ~57 distinct values, not a spatial patch, a value's
faces span the whole map, indistinguishable from a random subset of the same
size, groups wildly uneven — 1, 2, 4 and 384 faces on SC_3". Every one of
those observations is true and none of them means anything, because the
number is three numbers.

Field 1, the surface word, has two bits in use, and the second one is
**lava**. Bit `0x10` is clear on lava and on the bed beneath it, and set on
everything else: on the **29 of 33 maps that set the bit anywhere**, the faces
carrying it clear are *exactly* the faces whose layer-1 material names lava —
all **6711** of them across the library, surfaces and beds alike, with no
exception on any map. The other four files never set it and contain no lava,
so `LandMesh.marks_lava` asks first; on those a clear bit means nothing.

An earlier reading of this field said "set on about 95% of faces and clear on
the rest, in connected regions, tracking none of slope, elevation, material,
level of detail, map edge, duplication between levels, or coverage by the
navigation mesh". The 95% was real and the conclusion was not, and the reason
is worth keeping: **the maps were pooled.** The four files that never set the
bit contribute 23439 clear faces with no lava under them, which buries the
6711 that carry the signal. Per map the correspondence is exact and obvious.
It was also read as "clear on every lava bed and on patches of ordinary
ground" — the ordinary ground was those four maps, and two of them,
`SC_1` and `Net_4_01`, are the same file under two names.

## Stream 11 is the order to draw the faces in

Four bytes a face: a `uint16` face index, a flags byte and a byte that is zero
on 275566 of the 275882. The old note called the word a flag and puzzled over
its values; splitting it into two bytes settles most of that, and the *index*
turns out to be the interesting half.

**It is a permutation of the whole face list on all 33 maps** — every face
once, none twice — and not an arbitrary one. It reorders faces only *inside* a
cell: all **14976** cells stay contiguous. And within a cell it sorts them by
texture pair, optimally: in this order **every one of the 14976 cells draws in
the minimum number of batches**, with no texture pair appearing twice in its
run, against 11463 in file order.

So stream 11 is the draw order the map was baked with — the engine walks a
cell and gets its faces already batched. A renderer that buckets faces by
material itself, as this one does, has no use for it, which is why the viewer
reads it and does not draw with it.

The flags byte beside each entry says **where a batch begins**. Bit `0x10` is
set on exactly the **27174** faces that open a run of one texture pair inside
a cell, and clear on all 248708 others — agreeing on every one of the 275882.
Walk the order and change material wherever the bit is set, and the map draws.

The rest of the byte is a constant `0x48` — bits 3 and 6 on every face — plus
bit 7, which 89 faces carry.

### What the engine does with the byte — *read*, and *measured*

**The draw reads bit 4 and nothing else.** The landscape hands a cell's run of
draw-order entries to `CShade`'s cell draw (`Terrain.dll:0x100438c0`, from the
landscape render at `0x1001c35f`), which takes each entry's face index and
tests `byte +2 >> 4 & 1` to know where a batch begins (`0x1004399a`).

**The engine writes the byte itself.** Placing a building rebuilds draw
order (`0x10064770`, called from the placement at `0x1000e430`) through three
builders (`0x10060480`, `0x10062eb0`, `0x10064490`). The one at `0x10060480`
walks a run and takes every face not yet taken whose texture pair is the one
asked for and whose flags carry neither `0x20` nor `0x800` — one batch per
pair, the same minimum-batch order the shipped files carry — and writes each
entry the same way: bits 0–2 clear, bit 3 set, bits 5–6 set to 2, bit 4 on the
first entry it writes. That is `0x48` plus the batch start, and **275882 of
275882** shipped entries are exactly that once bit 7 is set aside
(*measured*).

**Bit 7 is inert.** Nothing in `Terrain.dll` tests it, sets it or clears it —
searched as a shift by 7, a mask or test of `0x80`, a sign test after a byte
load, and a test of `0x800000` on the whole entry: the builders write every
other bit through masks (`0xfc`, `0xef`, `0xfb`, `0x9f`) that keep it as it
was, and the only read of the byte is the bit-4 test. The same scan run for
bit 4 finds its reader and its five writes, so the silence is the binary's.
The 89 entries that carry it — **76 on `ILKON` and 13 on `SC_3`** — change
nothing the game draws. Where they came from is not established; a tool that
built the order with these masks over a reused buffer would leave exactly
this.

## Placing a building cuts the landscape — *read* in outline, and *measured*

A building does not stand on the landscape: it is let into it. What its
placement changes decides what a unit walking inside it finds underfoot and
what the camera sees through its floor
([24-motion.md](24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).

**The placement retries by turning** (*read*). `CLandscape::PlaceBuilding`
(`Terrain.dll:0x1000df10`) takes the building's matrix and calls the insertion
(`0x1000e430`, at `0x1000e3ce`) with the matrix turned about z by an angle
that starts at 0. If the insertion fails it adds 0.01 rad (`0x1009a208`) and
tries again, up to 2π (`0x1009a218`), and only then logs *"Building insertion
finally failed!"*. A building that fits goes in at its own angle.

**The two rings of its `.bas` are the two contours** (*read*). `CBuilding`'s
loader (`0x10056040`) reads the member as two ring sets. First comes a count,
then that many rings, each of a point count, its points closed by the first
again, and one int array of triangles and one of corners. This set is kept at
the building's `+0x50` (`0x10056106`). Then comes a second count and its rings,
which have no arrays; that set is kept at `+0x44` (`0x100564f5`). So the
"marker 1" of [07-objects.md](07-objects.md#bas-is-a-buildings-ground-plan) is a
ring count, and every shipped `.bas` holds one ring of each kind (*measured*).
`IBasement`, the interface at `CBuilding + 4` (vtable `0x1009b57c`), hands out
each set in world space: slot 4 (`0x10057ed0`) the traced **inner** ring, and
slot 3 (`0x10057db0`) the clearance, **outer** ring. The insertion asks for both
(`0x1000e642`, `0x1000e664`).

**The insertion deletes the landscape inside the contour and stitches in new
faces** (*read* in outline: what follows is the function's own log strings
placed against the code that prints them; the geometry between them is not
transcribed).
- It seeks the landscape face under each contour vertex (*"Seeking face with
  vertex (%f, %f)"*, `CTerrain::FindFaceInWing`), and fails with *"Contour of
  building ran out of landscape"* or *"Building has 0 vertices in outer
  contours"*.
- It sorts the landscape faces it meets into state 1, **inside** the contour,
  and state 3, **cut** by it (`0x100104de`). It lists each as a deleted face,
  the pair (face, −1), and logs *"Face deleted(INSIDE) #%d"* (`0x10010611`)
  and *"Face deleted(ISECTED) #%d"* (`0x10010742`).
- It triangulates what is left of the cut faces outside the contour (*"There
  were %d outside triangles"*, *"New faces in patch qty = %d"*) and writes them
  over the deleted faces' slots (*"Replacing face #%d"*, `0x100133d4`), normals
  included. Each cut face's triangles reach exactly to the outer contour
  ([below](#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).
- It builds the **basement**, the faces between the outer contour and the
  inner one (*"New basement faces qty = %d"*, `0x10011a22`; *"Final outer
  contour and inner contour intersect"* when the two cross): the constrained
  Delaunay triangulation of the ring between them
  ([below](#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).
- It sets the building down on the mean height of its cut outer contour
  ([below](#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured)).
- An edge of a landscape face that now borders the basement takes
  `0x8000 | n` as its neighbour (*"OUTER LINK!!!!, #%d"*, `0x1000c8e7`), so a
  walk across the mesh can step from the landscape onto the basement.
- The cell's draw order is then rebuilt ([above](#what-the-engine-does-with-the-byte--read-and-measured)).
  That builder leaves out every face whose flags carry `0x20` or `0x800`
  (`0x10060530`, `0x10060547`). Landscape `0x20` is world face bit `0x8`, which
  the ground search also excludes. The landscape's flags are the file's face
  record read as one dword — the flags word low, the surface word high — so
  that bit is **the flags word's `0x20`**, and the class bit 8 the ground
  search excludes beside it is **the surface word's `0x04`**; 0 of the 275882
  faces carry either
  ([24-motion.md](24-motion.md#finding-the-ground--read)). No writer of either
  bit was found in the insertion (a scan of its range for `or` and masked
  writes).

**Nothing of the landscape is left inside the inner contour** (*derived*). The
faces there are deleted, and neither the patch nor the basement reaches past
the inner ring. The building's own faces are the only ground there, and the only
thing drawn.

*Measured*, on Tut_2 at level 0 (the rings placed by the mission's position and
angle; both at the building's own z = 0):

| | Large Factory (`fr_b_plant`) | Outpost (`fr_l_angar`) |
|---|---|---|
| inner ring | 20 corners, x −38.4…38.4, y −96.0…108.5, 13389 m² | 26 corners, x −26.4…34.5, y −37.3…32.7, 2874 m² |
| outer ring | 14 corners, x −49.0…49.0, y −105.6…118.1, 19249 m² | 12 corners, x −37.4…46.0, y −48.4…43.8, 6140 m² |
| landscape faces wholly inside the outer ring | 0 | 0 |
| landscape faces it cuts | 13 | 5 |

Tut_2's ground is a few large faces where the buildings stand, so every face
under them is a cut face: the pod room's floor at −12.4 lies under faces that
the placement deletes.

### What a basement face wears — *read*, and *measured*

The insertion builds its new faces in one array and fills it from two places.
The first (`0x1000bdb0`, called once per cut face from `0x10011cc5`) copies
that landscape face's flags and **both texture bytes** into a template
(`0x10011bbf` into `0x100a5964`–`0x100a5968`) and stamps them onto each face it
makes (`0x1000ccdc`, `0x1000ccf7`, `0x1000cd18`), so those faces keep the
ground's own pair. The second (`0x1000cd40`, called once from `0x10011ce9`)
writes every face it makes the same way instead (`0x1000d9c4`):

| field | value |
|---|---|
| tex1 (`+4`) | **0** |
| tex2 (`+5`) | `0xFF`, no second layer |
| flags (`+0`) | `0x300`, written as a **dword**, so the surface word at `+2` is zeroed with it |
| field 3 (`+6`) | 0, over the `0xFFFF` every shipped face carries there |
| normal (`+0x14`) | its own, from its corners, as int16 over 32767 |

The surface word is not written on its own: `0x1000d9d8` stores the dword
`0x300` across fields 0 and 1 together, and the `word` store four instructions
later (`0x1000d9e4`) goes to `+6`, which is [the face record's](#the-face-record-28-bytes-14--uint16)
always-`0xFFFF` field 3 and not the surface. The flags then take a conditional
**`0x8000`** (`0x1000d9fa`–`0x1000da0b`): a local float is compared against
zero and the bit is set unless it is negative. Which float that is, and so
which of the band's faces wear the bit, is **not established**.

and each corner takes a layer-1 UV from **its own world x and y times 0.066**
(`0x1009a214`, packed at 1024 to the UV unit at `0x1000d4be`), a blend of 1
(`0x1000d1fd`) and a **zero vertex normal** (`0x1000d106` into the packer at
`0x10015fd0`). The UV unit is 1024 (`0x100227c0`, `1.0/1024`,
[above](#the-uv-unit-is-1024--read-and-measured)),
so the foundation tiles **every 15.15 world units**, against the landscape's
170 to 256. A basement face's flags are `0x300`, without the `0x400` the
microtexture is drawn under, and its vertex normal is zero, so no light
reaches it: it shows its texture at the scene colour
([How the landscape is lit](#how-the-landscape-is-lit-and-fogged--read-and-measured)).

**The two builders are callbacks over a walk** (*read*). The insertion passes
each, with an object, to `0x10007f00` (362 bytes, 8 callers: six in the
insertion and one each in `StartCheckMaxBasementAngle` and
`CheckMaxBasementAngle`), which loops over the object and calls the callback
for each piece it holds (`0x10007f7d`, `0x10007fe7`). The first is handed one
object per landscape face of state 3, a **cut** face, from a list at the
insertion's `[ebp−0x74c]` (`0x10011b84`–`0x10011cc5`), after that face's own
flags and texture pair are copied into the template. The second, which names
itself `AddBasementFaceProc()` in its assertion, is handed one object for the
whole insertion (`[ebp−0x24]`, `0x10011ce9`). The count the insertion logs as
*"New basement faces qty"* (`0x10011a22`) is printed before either runs, so the
game calls both sets basement faces. ~~**Not established**: what a piece is and
where the band is triangulated — in the builders, which are 3972 and 3285
bytes, or where the objects are built — and so which area each cut face's
object covers, the part of the cut face outside the outer ring or more.~~ —
**read**: an object is a constrained Delaunay triangulation and a piece one of
its triangles; the band is triangulated where its object is built, and a cut
face's own triangles cover exactly the part of it outside the outer ring
([below](#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).

**Layer-1 slot 0 is the footing material, and nothing else uses it**
(*measured*). `Land1.wea` names slot 0 `B_S0` on 32 of the 33 maps and
`B_MTP_01` on FINAL; `B_S0`'s texture is `B_FOUND`, a grey foundation slab. Of
the **275882 faces across all 33 maps, not one names layer-1 slot 0** — at
either level of detail. The slot is reserved for the faces the engine makes,
which is why a building in the game stands on a band of stone.

### The pieces are triangles of a constrained Delaunay triangulation — *read*, and *measured*

**An object is a quad-edge subdivision** (*read*). The walk at `0x10007f00`
runs down the object's list of edges (`+8`), and an edge record is four
sub-edges of `0x18` bytes, `0x60` in all: each sub-edge carries its rotation
index in its first byte, a mark in its second, its next edge round its origin at
`+4`, its origin at `+8` and a label at `+0xc`, and the record carries one more
word at `+0x60`. For each edge and its reverse that is not marked, the walk asks
whether the face on its left is a triangle turning counter-clockwise
(`0x10003510`: the next edge round the origin and the next edge round the face
share their far end, and `0x10002ef0`, an orientation test, holds), hands that
edge and its record's `+0x60` to the callback, and marks the face's three edges
by going round it (`0x10007e20`, which gives up after `0x2710` steps). **A piece
is one triangle**, handed over once.

**The triangulation is Delaunay, and a constraint is never swapped** (*read*).
A site goes in with a tolerance of 0.001 (`0x10003cc0`, `0x3a83126f`); every edge
round it is then tested and swapped by `0x10003660`, which returns at once when
the edge's record carries `+0x60` and otherwise swaps the edge (`0x10001fe0`)
when the in-circle test holds (`0x10002ae0`: four squared lengths, each against
a triangle's area, summed and compared with 0) and recurses into the two edges
beyond. `0x10004eb0` lays an edge between two points in as a constraint: it puts
both points in as sites, forces the edge through, sets the record's `+0x60` to 1
(`0x100057e5` and three more sites) and ORs its two label arguments into the
edge's `+0xc` and its reverse's (`0x10004e60`). So each triangulation is the
constrained Delaunay triangulation of its sites and constraints, and each side
of a constraint is labelled. A label floods across the unconstrained edges
(`0x1000b710`, which sets an edge's label and its reverse's from the first labelled
edge of the face, and is run again until no edge is left at 0 — at most 100
times on a cut face, `0x1001079a`, or it gives up).

**A cut face keeps its triangles outside the outer ring** (*read*). Each
landscape face of state 3 gets its own triangulation, started from its three
corners (`0x10001a10`, `0x1000f7b0`). The outer contour is walked through the
landscape face by face (the face under its first corner at `0x1000ee99`, then
`0x10008880` per face, `0x1000f052` and `0x1000f59d`, which clips the contour's
segment to the face), and each piece of it inside a face goes into
that face's triangulation as a constraint labelled **1 on its left and 2 on its
right** (`0x1000f8e6`–`0x1000f907`). Every `.bas` ring winds anticlockwise
(*measured*, all 60), so 1 is inside the contour and 2 outside. After the flood,
`CountOutsideTrianglesProc()` (`0x1000b870`) counts the triangles whose three
edges carry 2 — *"There were %d outside triangles"* — and panics on a triangle
of mixed labels (*"Illegal state values"*), and the first builder (`0x1000bdb0`)
makes a face only of a triangle labelled 2 (`0x1000bdd9`). **So a cut face's
own triangles, in the ground's own texture pair, reach exactly to the outer
contour and no further**, and nothing of the landscape's pair is left between
the contours.

**The band is one triangulation of the ring between the contours** (*read*). Its
object (`0x10001c30`, four corners, `0x10010be2`) starts as a box round both
rings, the box's size again beyond them on every side, with sites along its
sides every 160 units (`0xa0` at `0x10010b20`; a failed pass halves that, down
to 5, `0x1001172e`). Into it go, as constraints:
- **the final outer contour**, labelled 1 on its left and 2 on its right
  (`0x10010ee9`–`0x10011117`): the outer ring with a corner added wherever it
  crosses a landscape edge, each corner on the ground. It is the list the clip
  above leaves behind, the start of each clipped piece and the last end
  (`0x1000fac7`, `0x1000ff22`, kept at `[ebp−0x14]`);
- **each inner ring**, labelled **2 on its left and 1 on its right**
  (`0x10011178`–`0x100113ac`), checked against the final outer contour on the
  way (`0x1000ada0`, *"Final outer contour and inner contour intersect"*).

So 1 is the ring between the contours and 2 everything else. `0x1000bad0` counts
the triangles labelled 1 into *"New basement faces qty"* and throws on a mixed
one, and `AddBasementFaceProc()` (`0x1000cd40`) makes a face only of a triangle
labelled 1 (`0x1000cd69`). Since the contours are constraints, no site outside
the outer contour can reach across it into the band, and **the basement is the
constrained Delaunay triangulation of the ring between the final outer contour
and the inner ring**. Every one of its faces is the foundation's; none keeps the
ground's pair.

The inner ring's corners stand at the mean height of the final outer contour
when `IBuilding` slot 13 (`0x10056cd0`, the dword at `CBuilding + 0xb8`) returns 1
(`0x10011147`), and at their own height otherwise. `CBuilding`'s constructor sets
that dword to 1 (`0x100569b4`) and a restored game state sets it from its last
block (`0x10058993`, `CBuilding::SetObjectState`); its setter, slot 12
(`0x10056cb0`), was not traced to a caller.

`StartCheckMaxBasementAngle` (`0x100150f0`) builds the placement test's band the
same way: a box (`0x1001539e`), the outer ring's edges labelled (1, 2)
(`0x1001556e`) and the inner ring's (2, 1) (`0x1001575d`), then the flood; and
`FindMinNormalZProc()` (`0x1000da20`) takes the triangles labelled 1
([32-builder.md](32-builder.md#the-test-isplacementvalid--read)).

*Measured*, with openparkan's own triangulation of the same rings (`engine`,
`cdt.rs`): for **all 167** buildings the install's missions place, the band's
faces tile the ring between the final outer contour and the inner ring — their
areas across the ground sum to the one's less the other's, every face turning
anticlockwise. The Large Factory's final outer contour on Tut_2 has **29**
corners, its own 14 and 15 crossings; the Outpost's 24 and the generator's 40.

### A building is set down on the mean of its contour — *read*, and *measured*

After the band is labelled the insertion takes the difference between the first
inner corner's height and the final outer contour's mean (`0x1001166d`), and
when `IBuilding` slot 13 returns 1 it takes that from the building matrix's
z translation, element 11 (`0x100147cc`–`0x100147e0`), and hands the matrix to
the building's control and to the object (`0x1001482c`, `0x1001485c`). The mean
runs over the contour's corners, the closing repeat left out
(`0x10010f18`–`0x10011132`). **So a building's base ends up at the mean height of
the ground along its cut outer contour.**

*Measured*: **151 of the 167** placed buildings' bases already stand there to a
centimetre, 145 to a millimetre, so the missions were saved with the drop made
(the contour over the level-0 faces that are not water, as the ground search
takes them; counting the water faces too, 150 and 143). The mean of the ring's
own corners alone matches only **40** to a millimetre, and Tut_2's generator,
on ground from 314.1 to 344.4, stands on the one mean to 0.000 and 1.87 off the
other. The 16 that do not are **8 of the 19 bridges** — off by 0.2 to 5.2 — and
8 buildings of four campaign missions: `CAMPAIGN.02/Mission.03`'s two mines,
factory and generator (4.70, 0.33, 0.85, 1.02), `CAMPAIGN.01/Mission.01`'s
bunker (0.14), `CAMPAIGN.03/Mission.01`'s generator and bunker (0.14, −0.07) and
`CAMPAIGN.04/Mission.02`'s generator (0.03). One more, `CAMPAIGN.05/Mission.02`'s
bridge, is on its mean only with the lake's own surface faces left out (0.0004
against 0.024). ~~What the game makes of the 16, whether the flag is 1 for them
or the landscape under them has changed since they were placed, is not
established.~~ **Twelve of the 16 are the buildings whose start flag is set**:
the flag makes slot 13 answer 2, and the insertion leaves them at their file
height — the 8 bridges and Campaign 2 Mission 03's four
([04-missions.md](04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured)).
The other four, all within 0.14 of their mean, carry no flag, and why they stand
off it is **not established**. openparkan keeps every placed building at its
mission height, which the drop leaves within 0.14 of where it would put it, and
sets down a building the console's `bcreate` makes
([15-behaviour.md](15-behaviour.md#what-the-consoles-create-bcreate-and-death-do--read-and-measured)).

### For an engine

1. Take the building's two `.bas` rings, the first inner and the second outer,
   placed by its matrix.
2. Delete every landscape face inside the outer ring or crossing it, from the
   draw and from every ground and collision query.
3. Fill the ring between the outer contour and the landscape faces that remain
   with new faces on the landscape's heights, and the ring between the outer
   and inner contours with basement faces wearing layer-1 slot 0 and no second
   layer, their UV laid over the world at 0.066 a unit
   ([above](#what-a-basement-face-wears--read-and-measured)).
   The outer contour is cut at every landscape edge it crosses and each corner
   dropped onto the landscape; the inner ring holds the building's base, which
   the insertion has set down on that contour's mean
   ([above](#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured)).
   The band is the constrained Delaunay triangulation of the ring between the
   two, the two rings its only constraints; a cut face keeps exactly its part
   outside the outer contour, in its own texture pair
   ([above](#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).
   `CheckMaxBasementAngle`
   ([32-builder.md](32-builder.md#the-test-isplacementvalid--read)) triangulates
   the rings the same way to measure the slope.
4. Inside the inner ring, the building's level-0 faces are the ground.

## What fparkan's notes add, and what they do not

[fparkan](https://fparkan.popov.link/) publishes its own reverse-engineering
of the same engine. Its *documentation* — not its source; see
[09-method.md](09-method.md) — was read for the two bits still open here, and
it does not name either. What it does give is a **frame**, and everything
below was re-checked against the install before it was written down.

- The surface word is a **16-bit compaction of a 32-bit engine mask**, with a
  documented bit-for-bit mapping. Our unexplained `0x10` is the engine's full
  mask bit `0x00001000`, and its `0x02` is full `0x00000008`. Neither is
  named there either. The compact word takes exactly two bits across the
  library: 0 on 27538 faces, 2 on 2712, 16 on 244714 and 18 on 918.
- Beside it the engine packs a **six-bit class** from six more mask bits —
  and **field 13 is exactly six bits wide**: every one of the 275882 faces
  holds a value below 64, the largest 62. The six bits turned out to be
  three 2-bit edge codes, the [winged-edge link](#field-13-is-the-winged-edge-link),
  not six flags as this bullet once concluded; either way its groups have no
  reason to be spatial or to track a material.

Two places where the reading here goes further. fparkan leaves the face
record's last eight bytes uninterpreted; six of them are the face's own
normal, checked above. And it describes stream 11 only as "cell accelerator
data" with no semantics; it is the draw order, and its `0x10` opens a batch —
both checked on all 275882 faces.

## Face field 13 is not a patch id

An earlier draft read the last word of the face record as a patch or sector
id, on the strength of its range (0..62, about 57 distinct values per map).
It is not — it is the winged-edge link, above. Grouping faces by it gives
regions that **span the whole map**:
only 25 of 348 groups across six maps are even 20% tighter than a random
subset of the same size, and the median group covers 100% of the map either
way.

It is also interleaved in face order rather than run-length, does not
determine the texture pair or the surface word, and does not track elevation.
Its groups are wildly uneven — 1, 2, 4 and 384 faces on SC_3. So field 13 is
not what a renderer would cull by — but the map does carry a grid, in
[stream 2](#streams-1-and-2-are-the-maps-own-spatial-index), which is.

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

**The record does not make them two layers.** A material's second `uint16`
counts *animation tracks*, not layers — see
[07-objects.md](07-objects.md#the-second-count-is-animation-tracks-not-layers)
— and on all 43 the two tracks hold a single key each, track 0 naming entry 0
and track 1 naming entry 1. A caller asking for track 1 gets one texture,
exactly as a caller asking for track 0 does. Nothing in the record binds them
together.

Their entries differ in **lighting**. An entry is a `D3DMATERIAL7`, and on 38
of the 43 the first carries a **white diffuse over a black ambient** and the
second the reverse — a **black diffuse over a white ambient**, which reads as
unlit. An earlier draft concluded the twin is drawn unlit; the draw below takes
only its texture, so those colours are not what shows.

They are not bump maps, which an earlier draft guessed from `Iron_3D.ini`'s
`EMBM=1`; not a mask, which the reader used to call them. Nor are they simply
the 16-bit and 32-bit variants of one texture: a bit-depth copy would not be
pulled towards grey, and would not be pulled hardest exactly where the
original is least grey. They **are** a second texture stage — which a draft
before this one said, and the one after it denied.

### Who asks for track 1 — *read*, and *measured*

**The landscape does, on every face that is not water.** `CShade`'s cell draw
(`Terrain.dll:0x100438c0`) flushes each batch through a lighting helper
(`0x1002b470`, called at `0x10043a3d` and for layer 2 from `0x1002c132`). When
the device supports render phase 9 — `CShade::ConfigureTextureAndAlphaBlendModes`
keeps that answer at `+0xbd8` (`0x10041274`) — and the batch's face is not
water (its face word 1 bit `0x02`), the helper asks the material manager's
slot 3 for **track 1** of a material (`0x1002b4b6`; which one is
[below](#the-microtexture-is-land2weas--read-and-measured)) and makes it the
surface's second stage: track 1's texture as the second texture, its whole
image as the second cell, and phase **9** — `tex0 · tex1 · 2 · diffuse`,
`MODULATE2X`, whose identity is mid-grey
([05-engine.md](05-engine.md#the-render-phase-table)). It then returns no
extra pass; on any other device, or on water, it goes on to build one where
it needs to, textured with track 0 (`0x1002b96f`), for the lights it tests
each vertex against.

The helper runs only while the setting `MicroTexturingOn` is on (`CShade
+0xcc0`, the fifth of the 36 settings, default 1, and no `shade.cfg` ships
to change it), the face carries flag `0x400` (all 173827 level-0 faces do, in the
constant `0x600`), and a per-frame camera flag at `+0xbcc` is clear.

<a id="the-microtexture-is-land2weas--read-and-measured"></a>

**The material it asks is not the face's own: it is `Land2.wea`'s at the same
index** — *read*, and *measured*. The landscape loads `Land1.wea` into its
material manager and then `Land2.wea` into the same manager as a second wear
(`0x100171e1`, then the manager's slot 6 at `0x10017215`, `World3D.dll`'s
wear loader `0x10003b10`, which answers the new wear's index), and keeps that
index for its face records (`0x10018ce9`–`0x10018cef`). A material handle
carries a wear in its high word, and the cell draw builds the microtexture's
handle as **the face's material index plus that wear's index shifted 16**
(`0x100445bd`–`0x100445ce`, and `0x100445fe` for the second material), which
is what the helper asks track 1 of (`0x1002b4a3`–`0x1002b4b6`). The second
stage's coordinates are the item's second pair, **stream 18** (`0x10044931`–
`0x10044946`; the landscape puts chunk 5 in the first pair and chunk `0x12`
in the second, `0x100175b4`, `0x10017641`). So a face draws

```
its Land1.wea material's texture, on stream 5      (a turn in 170 to 256 units)
  × track 1 of Land2.wea's material, on stream 18  (a turn in 20 units)  × 2
  × the vertex's lit colour
```

and its second material, where it has one, the same way with its own pair.
On `KM_6bis`, *The Last Bastion*: `Land1.wea` is `B_S0, L33, L00, L32` and
`Land2.wea` `DEFAULT, L05, L01, L32`, so the mossy rock `L33.0` is drawn under
the gravel `L05M.0`, and the grass `L00.0` under `L01M.0`.

What that reaches (*measured*, level 0 of all 33 maps): **169688 of the 172012
faces that are not water** have, at their material's index in `Land2.wea`, a
material with a second track — all but `L32`, whose own entry `Land2.wea`
names beside it — and 17263 of their 17641 second materials do. None of
the 1815 water faces does, and the draw never asks them. A material with one
track answers with track 0: both of the manager's fetches take a track and
clamp one outside the material's count to 0 (`World3D.dll:0x1000322f`), so
`L32` is doubled over itself. The pairs most worn are `L02` under `L03`
(30256 faces), `L33` under `L05` (25843), `L35` and `L22` under `L13`
(14961, 13070) and `L00` under `L01` (10169): eight materials — `L01`,
`L03`, `L05`, `L07`, `L09`, `L13`, `L17` and `L31` — are the detail of
eighteen others, and four are named beside themselves, `L28`, `L32` and the
two liquid beds.

So on the default settings the ground is the material times a microtexture
times two: where the microtexture is mid-grey it changes nothing, which is
*derived* from the phase and explains why the `M` images are flattened
towards grey. The engine's own names say the rest — the setting
`MicroTexturingOn`, `MicroTexScale`'s 0.05 in stream 18, and the panic when
chunk `0x12` is missing, *"Unable to find microtexture mapping chunk"*.

**An earlier reading here had the twin of the face's own material**, on the
same coordinates, which is the base against a flattened copy of itself and
changes little. It is another material's, sixteen times as fine.

**Two things this entry used to say were wrong.** Slot 3 is not track-less:
it takes `(handle, track, &out)` and uses the global clock, where slot 5 takes
a time as well. And there *is* a five-argument call through slot 5
(`Terrain.dll:0x100454e6`), in `CShade`'s mesh draw on a manager handed to
`CShade::StartMeshRender` (`0x100437c0`) rather than stored where the earlier
search looked. An object mesh passes its track there too — see
[07-objects.md](07-objects.md#who-picks-an-object-meshs-material-track--read).

## How the landscape is lit and fogged — *read*, and *measured*

**A landscape cell is a lit item like a mesh batch.** The cell draw files its
item with flags `0x414` (`0x10044627`), and `0x10` is what
`CStridedPrimitive::RenderVB` hands to the game's own shade
([10-sky.md](10-sky.md#the-lit-colour-is-the-games-own--read-and-measured)).
The item carries the face's material entry whole — diffuse, ambient,
specular and power, copied at `0x1004465e`–`0x10044756` — the cell's light
list (`0x1004482a`), stream 3 as positions and stream 4 as normals
(`0x100448e1`–`0x10044910`), and no colour stream, so the shade makes its
colours. Each vertex is then

```
lit      = max(ambient + Σ light colour × diffuse × cos θ,  scene colour)     kneed past 1
frame    = texture × microtexture × 2 × min(lit, 1)  +  0.8 × what lit has over 1
           fogged by 1 − (d² − start²) ÷ (end² − start²) toward the horizon ahead
```

on the values the textures and the frame store. Three things follow, each of
which the engine had otherwise:

- **the scene colour is a floor, not an addend.** A slope the lights do not
  reach is the scene colour and no brighter, and a lit one is the light alone:
  the engine added the scene colour to every vertex, which paled the whole
  ground by it;
- **the fog is linear in the squared distance**, so the middle distance keeps
  far more of itself than a fog linear in the distance leaves it;
- of the 24 materials any face wears, 22 carry a white diffuse over a black
  ambient — the other two are the liquids, `WATER` and `ENV_NLAVA` — and none
  a specular colour (*measured*), so the ground's lit colour is the lights'
  own, and a light past 1 whitens it.

A basement corner's normal is zero, so it takes no directional light and
stands at the scene colour; liquid surfaces are in
[Water reflects](#water-reflects--read-and-measured).

*Measured* on *The Last Bastion*'s briefing at 43.5 s ("Let's Play - Parkan:
Iron Strategy, Part 4", Qqs8_i9IeUU, 16:12.5), mean colours over the same
areas, the recording converted by the video's own matrix:

| area | recording | engine before | engine after |
|---|---|---|---|
| near hill, lit face | (76, 115, 26) | (90, 129, 30) | (78, 118, 28) |
| the hill behind it, unlit | (35, 57, 11) | (91, 136, 27) | (37, 61, 13) |
| far hill, half fogged | (77, 123, 20) | (99, 160, 24) | (79, 126, 21) |

and across the whole picture in an 8 × 10 grid the two now agree to within
about ten levels in every cell of the ground: the same dark valleys and the same patches
of moss, because the rock's texture lies across the hill once and not
sixteen times. Before, the engine's ground was a fine pale pattern washed
toward the fog.

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

## Water reflects — *read*, and *measured*

A lake is not drawn with its own texture, and not with a fixed environment
map or the dome's colours. **It is the scene drawn a second time, through a camera
standing at the eye mirrored in the water plane** (`Terrain.dll`). There are
two ways of doing that, and the shipped `Iron_3D.ini` picks the second:

- **`REFLECTION`**: the mirrored scene is drawn into the frame, and the water
  faces are left out, so the lake is a hole in the ground it shows through.
- **`REFLECTION_SHIFTED`**: the mirrored scene is drawn into a small texture,
  and each water face draws that texture through an environment-mapped bump
  map, tinted by its lit colour.

### The settings — *read*

`Terrain.dll`'s 36 render settings are named by `0x1005eb10`, each through
`0x1005f390` (a name, a type — 0 float, 1 int, 2 bool — and a group), and
kept at `0x100a6cac` + 4 × index. The defaults are set at `0x1005fa80`, and
no `shade.cfg` ships to change them ([10-sky.md](10-sky.md#the-render-settings)).
The water's:

| index | setting | type | default | set by |
|---:|---|---|---|---|
| 25 | `UseEmbossBump` | bool | 1 | `EMBOSS_BUMP` (`iron3d.dll:0x1006177b`) |
| 26 | `UseReflections` | bool | 1 | `REFLECTIONS` (`iron3d.dll:0x10061736`) |
| 30 | `UseEMBMReflections` | bool | 0 | `EMBM` (`iron3d.dll:0x10061795`) |
| 31 | `EMBMCoeff00` | float | 0.01 | — |
| 32 | `EMBMCoeff11` | float | 0.01 | — |
| 33 | `EMBMMaxVal` | int | 64 | — |
| 34 | `EMBMBumpTile` | float | 100 | — |
| 35 | `EMBMBumpMove` | int | 10000 | — |

`iron3d.dll` reads `Iron_3D.ini` at load (`0x10061310`) and sends each key,
as 0 or 1, through the game-settings object to group 30, the page
`Terrain.dll` registers as `0x1e`: `REFLECTIONS` as setting 26, `EMBOSS_BUMP`
as 25 (besides `World3D.dll`'s `0x6d`, [02-texm.md](02-texm.md)), `EMBM` as
30. **The install's ini says `REFLECTIONS=1` and `EMBM=1`, so the game runs
the `REFLECTION_SHIFTED` way** (*measured*). `CShade` copies settings 26 and
30 when it is built (`+0x1668`, `+0x166c`; `0x10046da5`).

### The reflection camera — *read*

`CLandscape::Initialize` makes it only while `UseReflections` is on
(`0x1001fde4`): a `CCamera` (`0x100839c0`) named `REFLECTION_SHIFTED` when
`UseEMBMReflections` is on (`0x1001fe75`) and `REFLECTION` when it is not
(`0x1001fec9`). The name is its mode, `+0x1a0`: 2 or 1, and 0 on every other
camera (`0x10083a54`). Its buffering camera (`CBufferingCamera`,
`0x100813e0`) is told where to draw (slot 7, `0x10081e80`):

- `REFLECTION` draws into the frame, over the whole screen, with a
  perspective projection;
- `REFLECTION_SHIFTED` draws into a texture `CShade` makes for it
  (`0x10041370`) and copies itself there when its draw ends (`0x10081d02`):
  square, the largest power of two not above the screen's smaller side but
  at most 256 (`0x100423a0`) — **256 at 640 × 480 and at every size above** —
  with a window projection (mode 2, `0x10081f20`).

Both start with a field of view of 1.7 (slot 10, `0x10081f60`; read back by
slot 17, `0x10082120`) and flags 1. A buffering camera's projection (slot
29, `0x10081a40`, set up each time its draw begins) passes a near plane of
**5** and a far plane of **700** (`0x10081a76`). A camera's transform is a
matrix whose columns are its forward axis, its left, its up and its position:
`Ngi32.dll` builds the view from it with forward as depth, left negated as x
and up as y (`0x10009450`).

### When it draws — *read*

`CCamera::Render` (`0x100844c0`), for a camera of mode 0, runs:

1. with `UseReflections` and `UseEMBMReflections` both on: **the reflection
   draw**, then a clear of the device's depth buffer (render slot 10, `Clear`
   with `D3DCLEAR_ZBUFFER` and z 1, `Ngi32.dll:0x10007ca0`);
2. the world's `0x800` pass, and the end of that render;
3. with `UseReflections` on alone: the reflection draw, and the depth clear;
4. the world, and the `0x1000` pass.

The reflection camera itself hands the world the pass flags `0x120`
(`0x10084762`) and never reflects.

**The reflection draw** (`0x10083e20`) does nothing when the map has no
water — `ITerrain` slot 11 answers −1 (`0x10083e7d`) — or when no water face
was met in the frame before: the cell draw sets `CShade +0x1671` when it
meets one (`0x10044488`), the end of each frame moves it to `+0x1670`
(`0x10046b40`), and the draw asks for that (shade slot 25, `0x10042910`).
Otherwise, with *h* the water level and (*x*, *y*, *z*) the eye, it gives the
reflection camera the scene camera's field of view and flags, and then:

- **`REFLECTION`**: the scene camera's transform reflected in the plane
  *z* = *h* — the identity with −1 for z and 2*h* in z's translation
  (`0x100842f8`, multiplied at `0x100843ce`).
- **`REFLECTION_SHIFTED`**: a transform at the mirrored eye (*x*, *y*,
  2*h* − *z*) whose forward axis is world +z, left +y and up +x
  (`0x1008409c`): **it looks straight up out of the water**, a mirrored frame.
  Its window (slot 27, `0x10081f90`) is the water box below, seen from that
  transform at depth *d* = *z* − *h* (`0x100842e2`). `Ngi32.dll` projects a
  window (left *l*, top *t*, width *w*, height *v*, depth *d*) as a
  perspective one (`0x100070d3`): x′ = 2*d*/*w* · x + (2*l* + *w*)/*w* · z,
  y′ = 2*d*/*v* · y − (2*t* + *v*)/*v* · z, w′ = z, with depth
  far/(far − near) · (z − near). So the texture holds the whole water box as
  the mirrored eye sees it: every point of the water plane lands on its own
  texel, whichever way the player looks, which is what a planar mirror needs
  (*derived*).
- Then it sets clip plane 0 to (0, 0, 1, 0.5 − *h*) (render slot 19,
  `SetClipPlane`, `Ngi32.dll:0x10008aa0`), enables it (render state 152,
  `CLIPPLANEENABLE`), draws the reflection camera, and disables it
  (`0x10084426`–`0x10084483`): **nothing below *h* − 0.5 reaches the
  reflection**.

### The water box — *read*, and *measured*

At load the landscape widens an empty box — every corner ±`FLT_MAX`
(`0x1001d5d0`) — by the vertices of every face whose first word, field 0 and
field 1 read as one dword, carries `0x20000`: field 1's bit `0x02`, the water
faces (`0x10017e6e`). Its interface `0x22` (`+0x144`, `0x1001a182`) hands
out, from slot 3 (`0x10022630`), the box's four top corners at the water
level: (min x, min y), (max x, min y), (max x, max y), (min x, max y).

*Measured* on the 11 maps with water: Tut_1's box runs from x 385.5 to 1480.5
and y 255.8 to 1531.7 at *h* −1.7255, 1095 × 1276, **4.28 × 4.98 units a texel**
of the 256 texture. The boxes run from 119 × 663 (0.47 × 2.59 a texel) to
`Net_2_07`'s whole 3793-unit map (14.8).

### How a water face draws — *read*

`CShade`'s cell draw (`0x100438c0`), for a camera above the water:

- **a liquid-bed face** (field 0 `0x2000`) **is not drawn** (`0x10043c43`,
  `0x10043c9f`), whatever the settings;
- **a water face**, with `UseReflections` on, marks the frame (above), and:
  - under `REFLECTION` **is not drawn** (`0x1004449b`);
  - under `REFLECTION_SHIFTED` its batch draws as a surface of its own
    (`0x1002ca80`, called at `0x10043b39` and `0x100443cb`): the batch's
    vertices and colours, with two new UV sets, two textures and render
    phase 10.
- With `UseReflections` off a water face draws its material like the ground.

**The `REFLECTION_SHIFTED` surface** (`0x1002ca80`), for a vertex at
(*x*, *y*) and the box from (*x*₀, *y*₀) to (*x*₁, *y*₁):

| | |
|---|---|
| texture 0 | a 32 × 32 bump map `CShade` makes (`0x100425cd`, `0x100491e0`) |
| texture 1 | the reflection |
| UV set 1 | u = 1 − (*y* − *y*₀)/(*y*₁ − *y*₀), v = 1 − (*x* − *x*₀)/(*x*₁ − *x*₀) |
| UV set 0 | `EMBMBumpTile` × set 1 + *t* on both, *t* = (clock ms mod `EMBMBumpMove`) ÷ `EMBMBumpMove`: the bump map repeats 100 times across the box and drifts one tile along its diagonal every 10 s |
| bump matrix | 00 `EMBMCoeff00`, 01 and 10 zero, 11 `EMBMCoeff11`; luminance scale 1, offset 0 |
| phase | 10 (`0x1002cdc3`) |

**Phase 10** is `Ngi32.dll`'s record 16 (table at `0x10036a30`): stage 0 is
`BUMPENVMAP` of texture 0, stage 1 is texture 1 × diffuse, and the alpha is
the diffuse's; both stages filter by point, and the record turns on the alpha
test (`GREATEREQUAL`). So the water shows **the reflection, displaced by the
bump map, times the face's lit colour** — `WATER_M`'s is `(99, 212, 255)`,
`WATER`'s `(77, 106, 255)` — and neither water texture is drawn at all.

**The bump map** (`0x100491e0`): the texel in column *i* and row *j* holds
du = round(`EMBMMaxVal` × cos 4π(*i*/32 + *j*/32 − 1)) and dv the same with
sin, as two signed bytes (cos `0x1008e160`, sin `0x1008e820`; π is taken as
3.14159) — a wave of constant length 64 turning along the diagonal, two
turns across a tile.

**Under the water it turns round.** The frame's flag `+0xbb0` (`0x100466d5`)
is set when `IWorld` slot 8's vertical search for class-2 faces — the liquid
surface Control.dll asks for too ([24-motion.md](24-motion.md)) — finds one at
the camera's position (`+0xb98`, column 3 of the camera's matrix) not below
the camera. Then, with `UseReflections` on, only bed faces that are not water
draw; with it off, beds and water. The camera it tests is the one drawing, so
the reflection camera, under the water by construction, sets it whenever its
eye is over a lake (*derived*).

### The ground draws opaque — *read*, and *measured*

A model's batch takes its blend mode from its material's flags byte, through
`CShade`'s five-entry translate table at `+0xbfc`
([07-objects.md](07-objects.md#how-a-material-draws-is-in-the-archive-directory)).
**The ground does not.** Fifteen instructions in `Terrain.dll` name that table
(sixteen byte matches, one of them a coincidence inside an unrelated
immediate), and they sort into four groups: five are
`InitAlphaBlendModeTranslateTable` filling it (`0x10046a3c` to `0x10046b25`);
seven read it at a **constant** index, all inside one `CShade` init
(`0x100411f2` to `0x10041347`); one is an accessor at `0x10042832` that no call
reaches; and **two** read it at a material's own blend field — `0x10028907`
and `0x1004565f`, the two mesh draws, the only ones whose SIB scale is 4 on a
register the caller filled. None of the four groups is the ground. What the
ground's surfaces take instead is one of the fields that init fills:

| field | filled from | mode | who takes it |
|---|---|---|---|
| `+0xbf4` | index 0, or index 3 where the device has no render phase 3 (`0x100411e4`, `0x1004121f`) | 0 (`ONE`/`ZERO`) or 3 (`ZERO`/`SRCCOLOR`) | **the ground surface** (`0x1002c226`→`0x1002c235`, `0x1002c298`→`0x1002c2a7`) |
| `+0xbe0`, `+0xbf0`, `+0xbf8` | index 1 (`0x10041305`, `0x10041326`, `0x10041347`) | 4 (`SRCALPHA`/`INVSRCALPHA`) | the two surfaces built over a ground one (`0x1002b505`, `0x1002c0da`→`0x1002c0e9`) |
| `+0xbe8`, `+0x1930` | index 1 | 4 | the surface that writes no depth (`0x1002b9fb`, `ZENABLE` 1 and `ZWRITEENABLE` 0) |

`InitAlphaBlendModeTranslateTable` fills the table with mode ids `0, 4, 2, 3,
5`, so index 0 is **mode 0: `ONE`/`ZERO`, no blend and no alpha test**. The
draw item keeps the mode at `+0xcc`, which its render hands to the device
(`0x100302d0`) before the two depth bytes.

The water is the same answer written a second way: the
`REFLECTION_SHIFTED` surface sets its own blend mode, and sets it to **0**,
literally, at `0x1002cdd0` — three instructions after the phase 10 at
`0x1002cdc3`. So a lake is opaque whether or not reflections are on, and what
lies under it is never mixed in.

*Measured* over `Material.lib`'s 905 materials and the layer-1 material of
every one of the 275882 faces on the 33 maps: **273258 faces wear a material
whose flags byte is 0, and 2624 wear one whose flags byte is 4.** `WATER`
(1006 faces on 7 maps) and `WATER_BOT` (2015) are among the first, with
`ENV_LAVA_BOT` and all 20 `L*` ground materials that any face wears; `WATER_M`,
named on the same 7 maps but worn by no layer-1 face, is 0 too, and so is
`B_S0` at flags 2, which is still index 0.

The 2624 are `ENV_NLAVA`, the lava surface of 4 maps, and that is the control:
the same measurement over the same field does find a liquid whose material asks
to be blended — flags 4, index 1, mode 4, `SRCALPHA`/`INVSRCALPHA` with the
alpha test on. It is drawn opaque anyway, because the ground draw never looks.
A 0 on the water is therefore a reading, not a default.

Phase 10's own record turns the alpha test on (`GREATEREQUAL`, against the
device's `ALPHAREF` of 1) while the blend stays off; the alpha it tests is the
face's diffuse, which is opaque, so nothing is dropped.

### What the recording shows — *measured*

Mission 01's recording (960 × 720):

- **At 150 s a smoke column mirrors in the lake.** Down the column at x 370–395,
  the smoke is dark from y 335 to 400, above a strip of sand (400–420) and
  the shore line (425); its image is dark from 430 to about 485 — the same
  column, mirrored about y ≈ 412, soft-edged and a little shorter.
- The open lake beside it is **(162, 248, 253)**, against a horizon sky of
  (174, 167, 215) at the top left: cyan like `WATER_M`'s diffuse, and brighter
  in green than the sky it would reflect.
- At 240 s the water under the island's cliff is a dark teal band, **smeared
  in vertical streaks** (mean (34, 55, 53) over x 40–300, y 440–470) — the
  cliff's image, blurred and displaced.
- In the briefing at 1:31 the lake before the island is a greenish, darker
  image of its hillside.

A planar mirror, soft and wavering: what a 256 texture over a 1095 × 1276
box, displaced by the bump map, would give (*derived*). The colours are not
yet reproduced (below).

### Not established

- **The lake's brightness.** Phase 10 multiplies the reflection by the lit
  colour, so the water can be no brighter than what it reflects but for what
  the shade moves into the specular: a lit channel past 1 adds up to 0.8 of
  itself after the texture
  ([10-sky.md](10-sky.md#the-lit-colour-is-the-games-own--read-and-measured)),
  which is one way a lake under a lifted sun comes out brighter than its sky
  (*derived*, and not checked against the recording's (162, 248, 253)). Or
  that machine did not draw phase 10 (its record needs capability `0x1`).
- **How the recordings' machine draws a liquid.** C03 M01's lava at briefing
  time 38 s ("Let's Play - Parkan: Iron Strategy, Part 5", PfAg6zSe-yM, 1:25)
  shows **both** the lava's own texture, bright — (187, 4, 3) to (225, 8, 5),
  `LAV00.0`'s cells plain in it — **and** the mirrored image of the plant
  standing over it. That is neither way as read: `REFLECTION` leaves the
  liquid's faces out, and `REFLECTION_SHIFTED` draws the reflection times the
  lit colour with no texture of the liquid's, which is what the engine draws,
  a flat dim red. The lava's faces carry water's flags exactly (`0x608`,
  surface 2, on all 2624), so nothing in the file tells them apart; what that
  machine's device ran in place of phase 10 is not read.
- **The microtexture on a liquid drawn without its reflection.** A water face
  skips the one-pass second stage (`0x1002b49d`) and takes the microtexture
  in passes of their own, within 260 units of the eye over the field of view
  and at an alpha of at most 0.3 (`0x1002b512` on, the 0.3 at `0x1004f065`);
  those passes were not followed, and the engine draws none.
- **What the pass flags `0x120` leave out of the reflection** — the world
  draw passes them on to what it draws (`0x1001c8a8`), and their tests were
  not followed.
- **Culling in a mirrored frame.** Both reflection transforms have a
  determinant of −1, which turns every triangle's winding round on screen;
  whether a cull mode is changed for the reflection was not found.
- ~~**How the water blends.**~~ — it does not: the `REFLECTION_SHIFTED`
  surface sets blend mode 0 outright at `0x1002cdd0`, and the ordinary ground
  takes mode 0 from `CShade+0xbf4`, so nothing under a lake shows through
  ([The ground draws opaque](#the-ground-draws-opaque--read-and-measured)).
- **How far the bump displaces.** `D3DFMT_V8U8`-style values are signed and
  stand for −1 to 1 on the device, which would make the largest displacement
  0.01 × 64 ÷ 127 ≈ 0.005 of the box, about 5.5 units on Tut_1 (*derived*
  from Direct3D's convention, not from this binary).
- **`CShade +0xbcc`.** Every water test above also asks that it be 0; it is
  the buffering camera's `+0x280` (slot 24, `0x10083010`), and no call to
  its setter (slot 23, `0x10082fd0`) was found — searched as every call
  through slot `0x5c` in `Terrain.dll` — so it is taken as always 0.
- ~~**What a basement builder's piece is, and where the band is
  triangulated.**~~ — **read**: a piece is one triangle of a quad-edge
  constrained Delaunay triangulation; the band is the triangulation of the ring
  between the outer contour, cut at every landscape edge it crosses, and the
  inner ring, and a cut face keeps exactly its part outside the outer contour
  in its own texture pair
  ([The pieces are triangles](#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).
- **The 16 placed buildings not on their contour's mean.** The insertion sets a
  building down on the mean height of its cut outer contour while `IBuilding`
  slot 13 answers 1, which `CBuilding`'s constructor sets; 151 of 167 already
  stand there, and 8 bridges and 8 buildings do not. Whether the game moves
  them, and what calls the flag's setter (slot 12, `0x10056cb0`), is not read
  ([A building is set down](#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured)).
