# TODO — renderer

What is left to draw a Parkan scene correctly, and what has been closed.
Ordered by how much each item costs you in a picture, not by how interesting
it is.

Format questions that do not affect rendering (save games, the network
protocol, `.scr` semantics, leftover `data.tma` words) live in
[docs/06-open-questions.md](docs/06-open-questions.md).

Every figure below was measured against a real install and can be re-derived
with `uv run openparkan verify`.

---

## 0. Done

Closed, with what the answer turned out to be. A question with its answer
written down is a question nobody reopens.

- [x] **Node poses.** Stream 8 is 24-byte keys: a `float32[3]` translation, a
      `float32` frame time, and the rotation as `int16[4]` over 32767 in
      `(w, x, y, z)` order — **34038 of 34049** are unit length, and they read
      conjugated because the game is left-handed. Composing them reaches the
      model's own authored box on **305 of 434** meshes against 237 without.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **Unit scale.** Not a thing. The "chassis are 1/20 too small" reading
      came from measuring an *unposed* chassis; posed, they run 1.5 to 30
      units against buildings of 40 to 215.
- [x] **The vertical datum for buildings.** There is no datum. A mission puts
      model z = 0 at the given height and nothing else: over 864 placements a
      building's origin lands a median **0.00** from the terrain under it and
      a unit's lowest exterior vertex **0.08**. The "rest it on its base"
      heuristic this replaced lifted `fr_b_bunker` 23 units into the air.
- [x] **Component attachment.** A `.dat` is a tree written depth first —
      **458 of 458** assemblies consume their child counts exactly — and a
      component's second field is the node index in its *parent's* mesh.
      **946 of 946** guns and **468 of 468** turrets land on a geometry-less
      `Base_*` node. Units draw assembled instead of as a bare chassis.
- [x] **The slot index.** `[variant * 5 + lod]`, not `[lod * 5 + group]`.
      Level 0 alone reproduces the authored box exactly on **302 of 434**
      meshes against 231 for all five together. The viewer had been drawing
      four levels of detail on top of one another — 381k triangles where 229k
      was right.
- [x] **The terrain's second texture layer.** Stream 14 is the weight of
      layer 1: exactly 1.0 on **all 258046** vertices no layer-2 face touches,
      and below it on 19663 of the 41404 that one does. Ground transitions
      blend instead of ending at a polygon edge.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **Palettised transparency.** There is none — the palette's fourth byte
      is constant on all 15 palettised textures. Alpha lives in the 4444 and
      8888 formats, on **241 of 393** textures, and was being flattened away;
      foliage was drawing as solid slabs. → [docs/02-texm.md](docs/02-texm.md)
- [x] **Animated and special materials.** Terrain layer names are *material*
      names: **270 of 270** resolve through `Material.lib`, which is the whole
      of the "eight unresolvable special materials". `WATER_M` is one layer of
      ten frames, and water is blue because its material is `#4d6aff` — its
      texture is a neutral grey ripple.
- [x] **Lightmaps.** `lightmap.lib` is 25 atlases named by a `.wea`'s
      `LIGHTMAPS` section, and mesh stream 18 is their UV set — over **1024**,
      not 256, exact to the half-texel inset on all 21. Of the 435 object
      meshes, 21 carry both and **none carries one without the other**.
- [x] **Skyboxes.** `sky.ske` is not a skybox: it is an *atmosphere*, a day
      cycle of colour keyframes, and all **29 files parse to the byte** (656
      keyframes, every one with a valid time, every first section sorted).
      → [docs/10-sky.md](docs/10-sky.md)
- [x] **The sky textures.** `sky.wea`'s slot index is the role — all 29
      missions fill the same nine slots — and **261 of 261** slot names
      resolve through `Material.lib`. A material picks a **sub-image** as well
      as a texture: `ENV_SUN` and `ENV_MOON` are both `SUN.0`, at cells 0 and
      2, which is where each one sits.
- [x] **Backface culling.** Winding is consistent: **434 of 435** meshes wind
      over 95% of their triangles the same way as their own vertex normals.
      Everything draws front-face now except cutouts, which stay two-sided
      because a tree is a pair of crossed planes.
- [x] **The batch material's high byte.** It marks the lit batches: the 21
      meshes with a `0x00` batch are exactly the 21 with a lightmap, and
      **51324 of 51324** of their vertices carry a lightmap UV against 1268 of
      85347 under `0xFF`.
- [x] **Terrain patches** — a negative result. Face field 13 is *not* spatial:
      only 25 of 348 groups are even 20% tighter than a random subset of the
      same size, so there is nothing there to cull by.
- [x] **The rest pose.** Not a pose problem at all. The renderer was drawing
      the models' *collision hulls*: sub-object flag bit `0x20` sits on 28
      nodes and every one is named `CP_m1o1` or `BTCP_m1o1`, they are always
      leaves, and they carry **18402 triangles** of oversized box. Skip them
      and level 0 fits inside the extent the file itself states on **434 of
      434** models against 422 while they are drawn — 157 of 157 animated
      meshes against 145. The "51 animated meshes fall outside their box"
      figure this replaces was measured over all fifteen slot indices, which
      superimposes every level of detail; at the level the renderer draws, the
      residual was twelve, and all twelve were hulls.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **What a `.ctl` is** — a *movement* controller, not an animation one.
      `Control.dll`'s `LoadControlSystem` behind an `IControl` of
      `SetTangAccel` / `SetNormSpeed` / `SetStrafeAngle`; the file is that
      object written out, ∓FLT_MAX "no limit" triples and all. Its size
      correlates **+0.97** with its own leading count and **+0.40** with the
      node count of the mesh it belongs to, over 542 records, so it holds no
      per-node data. `.ndp`, the other unread `STAT` slot, is a hit-point
      float and an `(archive, member)` pair naming an `.exp` explosion.
- [x] **The socket's rotation.** It was being thrown away, and it was saying
      something real. A part mounts by making its root node take the socket's
      pose, so the transform is `socket ∘ root⁻¹`; every mounted part's root
      translation is exactly zero on **1414 of 1414** attachments, which is
      why the socket's position alone was indistinguishable from its full pose
      on the 1306 whose rotations agree. Of the 108 that disagree, **83 are
      exactly 180° and every one is on a chassis whose own name says Flying or
      Helicopter** — an aircraft's turret hangs under the belly. The socket
      even sits at the bottom of a flying hull and the top of a tracked one.
      Overlap with the hull falls on 22 of the 404 distinct mounts and rises
      on none. → [docs/07-objects.md](docs/07-objects.md)
- [x] **Damage variants.** The three five-slot blocks are damage states, and
      the renderer is right to draw only the first. 1479 nodes fill block 0,
      135 fill block 1, 15 fill block 2, and **1790 of 1790** fill them in
      order. A later block is the same part with pieces gone —
      `fr_l_gener`'s pylons run 88 / 66 / 14 triangles at z 22.70 / 9.31 /
      −16.88, the last sunk under the ground. The `.ndp` table settles the
      direction: **all 145** nodes with a second block name an explosion, and
      a tree's is `explode_tree.exp`. A tree is not built.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **`.ndp`, the damage table.** An `int32` count then 76 bytes per node:
      flags, durability, an unresolved float, and the `(archive, member)` pair
      naming the explosion that node plays. **542 of 542** members are exactly
      `4 + n*76` and 541 have one record per mesh node; 2203 records name an
      explosion. Read by `openparkan.objects.parse_damage`.
- [x] **The lens flare.** `CSun::RenderFlare` strings **twelve sprites** along
      the line from the sun's position on screen through the centre of the
      screen, and its four tables came out of `Terrain.dll` whole: positions
      from 1.2 (past the sun) to −1.1 (past the far side), sizes 0.1 to 1.0
      scaled by a quarter of the half-viewport, twelve `D3DCOLOR` constants of
      which the engine scales only the alpha, and a per-element pick between
      `sky.wea`'s two flare slots. The angular gate is exact too: off beyond
      15° from the view axis, linear to full on-axis, then squared.
      → [docs/10-sky.md](docs/10-sky.md)
- [x] **Effects.** `effects.rlb` is 923 effects, each a 60-byte header and
      then typed **emitter** blocks whose type fixes the block's length and
      where its `(archive, member)` pair sits. The table of nine types walks
      **all 923 to the byte** — 4737 emitters — and **3577 of 3577** material
      references resolve, 516 of 517 sounds. An `.exp` is a 24-byte header and
      one 64-byte name per effect: **144 of 144** parse, 212 of their 213
      names are real. End to end, **2189 of 2203** `.ndp` explosion references
      reach an effect whose every material resolves.
      → [docs/11-effects.md](docs/11-effects.md)
- [x] **The sprite-sheet cell.** Not a grid index, which an earlier reading
      assumed from `SUN.0`. A Texm may carry a **`Page` chunk** after its mip
      pyramid — the magic `'Page'`, a `uint32` count, then that many 8-byte
      rectangles of `(x, width, y, height)` — and the cell indexes *that*.
      `SUN.0`'s four pages are its quadrants, which is why the grid reading
      looked right; `EFFECT6.0`'s 26 are strips, tiles, discs and 16 x 16
      icons all at once. **All 61 indexed textures carry a table and all 478
      cells fall inside their own.** It was blocking the weather sprites and
      every effect sprite. → [docs/02-texm.md](docs/02-texm.md)
- [x] **The weather switch.** A keyframe names the atmosphere object it acts
      on: `sun` and `moon` in start/stop pairs, `atm_rain1.wav` and
      `env_lightning` once each. **14 of the 29 missions carry a marker** —
      eight name rain, eight lightning, none snow — and rain now draws.
      `CAtmData::GetEvents` dispatches on a ten-valued opcode whose branches
      pair up start/stop per object type, and the sky is created outside that
      switch with a hardcoded id. → [docs/10-sky.md](docs/10-sky.md)
- [x] **See-through buildings.** Reported against the alien plant on
      `Mission.02`, and it was every machine on every map. Carrying alpha and
      *being* transparent are different things: of the 241 textures with
      alpha, **only 23 are cut silhouettes**. The rest are continuous gloss
      maps — `S0A1.0` has not one pixel at 0 or 255 — and alpha-testing them
      discarded **30% of all object texture area**, 61% of `MTP_01.0`.
      `texm.is_cutout` separates them by asking for a real hole and a thin
      transition; a non-silhouette now takes its colour and ignores alpha
      rather than compositing it, which would wash the colour out.
      → [docs/02-texm.md](docs/02-texm.md)
- [x] **The placement rotation's sense.** Reported as bridges not meeting.
      A bridge is two halves back to back — nine pairs across seven missions,
      each pair's angles exactly π apart — and their roadways have to join.
      The angle applies **as it stands** about the viewer's Y: that joins the
      ends to within a unit on **9 of 9** pairs, against **0 of 9** negated,
      with gaps of 4.9 to 282.8 units. Every placed object was turned the
      wrong way; only the bridges could show it.
      → [docs/04-missions.md](docs/04-missions.md)
- [x] **The `NL` archives.** They are **RsLi**, and the reason they looked
      like noise is that the **entry table is encrypted** -- two bytes of
      state seeded from the header word at 0x14, `a = ((a<<1) ^ d)`,
      `d = (d>>1) ^ a`, running across the table without resetting. The
      keystream came out of `Ngi32.dll`'s loader; fparkan's reference named
      the format and gave the entry layout. Decrypted, **all 24 Deflate
      members of `sprites.lib` inflate to exactly the size they declare** and
      are ordinary `Texm`, so `openparkan textures sprites.lib` writes the
      cockpit, the interface, the cursors and the logo straight out. The two
      LZSS members of `gamefont.rlb` took a second attempt; see the entry
      above. → [docs/12-rsli.md](docs/12-rsli.md)
- [x] **What the `*M` ground texture is.** The 43 two-layer ground materials
      pair a texture in RGB565 with its `M` twin in XRGB8888, and the twin is
      the same picture **flattened towards neutral grey**: closer to grey on
      **40 of the 43** pairs, and moved furthest where the base started
      furthest away — `L25`, a near-white snow, goes from a mean distance of
      110 to 12. A per-channel fit of `M = a.base + b` leaves a residual of 2
      to 15 levels out of 255. So it is neither a mask, which this reader used
      to call it, nor a bump map, which an earlier draft guessed from `EMBM=1`,
      nor a plain bit-depth copy — a copy would not be pulled towards grey, and
      would not be pulled hardest exactly where the original is least grey.
      What the renderer *does* with it is a separate question; see §2.2.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **Levels of detail, and the trap that hid them.** The four levels are a
      ladder by triangle count — **137445, 34843, 14033, 5039** over 434, 283,
      282 and 197 meshes — and they are the same object with detail removed:
      posed, each level's centre sits within a median **0.000 to 0.011** of the
      model's size of level 0's, and its extent within **0.001 to 0.037**.
      Each slot's own declared box matches the vertices decoded for it
      **exactly, on every slot of every level** — 1451, 1011, 952, 535 and 288.
      Getting there took two attempts. `posed_positions(lod)` posed only the
      vertices that level reached, so reading levels 1 to 3 out of
      `posed_positions(0)` returned them in raw node-local space and made them
      look as though they lived in a different frame — level 1 appeared to fit
      the authored box on 129 of 283 rather than 257. It now poses **every**
      slot, which is unambiguous: no vertex of any of the 435 meshes is reached
      by two slots whose nodes pose it differently. The viewer packs all four
      levels and switches by apparent size, which at a camera framing the whole
      map draws **189804 triangles instead of 641530** across the 864 placed
      models — 30%, for 2.1 MB more payload.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **RsLi's LZSS, and with it the font.** The bit packing was right the
      first time; what was wrong is that the offset is an **absolute index
      into the ring buffer**, not a distance back, and the ring is pre-filled
      with **spaces** with the write position starting at **0xFEE** — the
      classic `N − F` of Okumura's LZSS. Miss any one of the three and a
      member still decodes for a few kilobytes before drifting, which is what
      made the first attempt so convincing. The routine came out of
      `Ngi32.dll`: `rsLoadFast` tests `flags & 0x1e0`, a **mask over the four
      packed bits**, and `rsLoad` switches on the masked value. The earlier
      sweep missed it because it looked for the seven storage constants one at
      a time and never for the mask over them.
      Both members now unpack exactly. **`ARIALTEX.TFT`** is a `Tfnt` — 256
      glyph records then a `Texm` at 4116 — and the metrics check out: on all
      **123** drawn glyphs the span is exactly `advance + 1`, and they sit in
      seven rows 18 pixels apart. Its atlas is **pixel format 2**, one byte per
      pixel indexing an external palette, which appears nowhere else.
      **`PAL.PAL`** is that palette plus a table tagged **`Ipol`**: 256 × 256
      bytes, **symmetric on all 65536 cells** with `table[i][i] == i` on 237 of
      256 — a colour mixer. Every lit font pixel is index 73, and 73 is white.
      → [docs/12-rsli.md](docs/12-rsli.md)
- [x] **The terrain's spatial index.** `Land.msh` stream 2 is not an
      unexplained blob: it is the map's own grid. Eight bounding-box corners,
      then a 68-byte record per cell holding `uint16 first`, `uint16 count`,
      the cell's box, its centre and a bounding-sphere radius. It **parses
      with nothing left over on all 33 maps**, the runs chain end to end, and
      their counts **sum exactly to the face count** — so faces are stored in
      cell order and a run indexes them directly. **All 275882 faces lie
      inside their own cell's box.** The grid is 16 x 16 on 28 maps and 8 x 8
      on five, following the mesh's vertex count rather than the world size.
      Each cell is listed twice, and the second record is where the coplanar
      copies live: **46261 of the 46283 duplicated faces are in it, against
      22 in the first**, which ties this to the duplicate-face finding.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The `MAT0` opacity, which was being read as a marker.** The byte at
      +5 of an entry is an **opacity in percent** — `World3D.dll`'s parser
      multiplies it by 0.01 — not the constant 100 an earlier reading called a
      marker. It looks constant because 3138 of the 3143 entries are fully
      opaque; the exception gives it away, `FIRESTORM` running **0, 60, 80,
      90, 95, 100** across its frames, which is a fade-in and is not something
      a marker can be. It still anchors the entry offsets, so nothing about
      the stride changes.
- [x] **The `MAT0` entry stride, and with it every texture name.** An entry is
      **34 bytes**, not 40: at 34 the marker byte lands on 100 in every entry
      of **904 of the 905** records, and every other stride tried collapses to
      531 — exactly the number of single-entry records, where a stride cannot
      be wrong. The names had been extracted by pattern instead, and the
      pattern swallowed whatever alphanumeric byte sat in front of a name: so
      `B_MTP_04`'s texture read as `qqds.7` when it is `MTP_04.0`, and the
      `FIRE_SMOKE` animations' as `0FAIR.0` .. `7FAIR.0` when all fourteen
      frames name `FAIR.0` and those digits are the cell bytes 48..55. Read by
      offset, **all 905 materials name a texture that is in Textures.lib**,
      against 891 by pattern — which empties section 1 — and the cell check
      now covers **2513** cells across every entry of every material rather
      than 478 first entries. → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The eight-layer materials.** Not eight images: `B_LBL_01` and
      `R_LBL_01` both name `PG27.0` eight times and ask for cells 0 to 7 of
      it, and differ only in their two colours. They are the blue and red team
      variants of one insignia sheet.
- [x] **Where the sun stands** — and it is in no file, which is why it was
      never found in one. `CSun`'s two angles are **constants in
      `Terrain.dll`**, filled by `CAtmData::GetEvents` from a single test on
      the keyframe's name: `(90°, 30°)` when it is exactly `sun`, `(0°, 50°)`
      otherwise. `CSun::Render` rebuilds `Rz(A)·Rx(B)` from them every frame
      and nothing ever changes them, so **the sun does not travel** — it
      stands 60° above the horizon and the moon 40°, a quarter turn apart.
      Three things confirm it. The same block's fourth field is 3 for the sun
      and 4 for the moon, which is `SLOT_ROLES` exactly — the engine and
      `sky.wea` agreeing on an index neither derived from the other. The
      flare's second gate, unidentified until now, ramps between `cos 60°` and
      `cos 30°` of the body's **height**, and the sun's height *is* `cos 30°`
      to the last bit: the two constants were chosen to bracket the two
      bodies, the sun flaring at full and the moon at 0.390. And the missions
      back the picture — **32 of 35 sections hold one start/stop pair of each
      body and none has them up at once**, the sun running about 01:30 to
      15:00 and the moon 16:20 to midnight, which is what makes two fixed
      positions a quarter turn apart coherent. The viewer's invented day arc
      is gone. → [docs/10-sky.md](docs/10-sky.md)
- [x] **Coplanar geometry.** Two causes, both fixed. The terrain's two ground
      layers now share a single pass — bucketing faces by the pair costs 5 to
      8 groups per map against 3 to 5 — and the file's own duplicated faces
      are filtered: **46283 of 275882** faces across the 33 maps repeat a
      triangle already in the list, and drawing both copies made the walkable
      ground flicker.

---

## 1. Wrong on screen today

Nothing known. What is left below is fidelity the game had and this does not,
and engineering.

The two materials that used to draw as flat grey are fixed; see the entry on
the `MAT0` entry stride in section 0. Nothing in `Material.lib` is
unresolvable any more — all **905** materials name a texture that exists.

## 2. Missing fidelity

Correct as far as it goes, but not what the game showed.

### 2.1 The sky's weather layers

All nine of `sky.wea`'s slots are accounted for and eight are drawn: the
nebula, stars and clouds on the dome, the sun and moon as billboards at their
own fixed places, the lens flare as a 2D overlay with both of the engine's
gates, and rain where a mission asks for it (see
[docs/10-sky.md](docs/10-sky.md)). The ninth is snow, and **no shipped mission
names it**, so there is nothing to switch on.

What is left of the weather is where a shower *stops*: the sun and moon come
in start/stop pairs and rain does not, so the viewer runs it to the next
keyframe that names anything — a reading, not a fact.

Two smaller unknowns sit in the same file: the keyframe count of a second
section (six missions have one), and most of the 124-byte file header. The
object type is now half-answered — `CAtmData::GetEvents` dispatches on a
ten-valued opcode whose branches pair up as start/stop per object type, and
the sky is created outside that switch with a hardcoded id — but which *file*
field feeds the opcode is not pinned down, which is why where a shower stops
is still unknown.

The sun's **lifetime** is the last piece of its own block: `GetEvents` maps
the start and stop keyframes' clock times through a per-section scale and
takes the difference, and that scale comes from a virtual call that has not
been followed. It changes nothing on screen — the start and stop keyframes
already say when the sun is up.

### 2.2 What the engine does with a material's second layer

The layer itself is [read](docs/03-terrain.md) and section 0 says what it is.
What no code path has been traced to is **which layer the renderer binds, and
how**.

Everything about the data says *modulation*: 128 is the value that changes
nothing, which is why the twin is the half stored at 8 bits per channel where
banding around the neutral point would show; and on 38 of the 43 the two
entries carry their two colour slots the opposite way round — entry 0 with a
white diffuse and black in the slot ahead of the marker, entry 1 the reverse —
so the second is flagged as something other than an ordinary lit layer.

None of that is confirmation. What was searched and came up empty:

- `BITDEPTH` and `RENDER_QUALITY` are in `iron3d.dll` and in no other binary.
  Both are read into a settings block built on the stack by the loader at
  `0x10061360`, and `BITDEPTH` is only ever *written* back to the ini. Nothing
  has been followed from that block to a material's layer index.
- The material manager is `World3D.dll` — `LoadMatManager`, and the MAT0
  parser at `0x10004634`. Its `GetMaterialPhase` takes a *table* and an index,
  but the table is the wear table, not the layer.
- `Ngi32.dll` exports `rsLoadMultiTexture`, which is the obvious name for it
  and is a **stub**: `xor eax, eax; ret 0x10`. That whole DLL's texture
  exports are stubs, so the real renderer is `iron3d.dll` and the multitexture
  path has to be found there.

The reader takes layer 0, which is the coloured one and the only one that
stands alone. If the engine does modulate, the ground is missing a contrast
boost at close range and nothing else.

### 2.3 Effects

Both formats are [read](docs/11-effects.md) — 923 effects walk their emitter
blocks to the byte, 144 `.exp` records parse, and a destroyed node's damage
record reaches real sprites on 2189 of 2203 references. Nothing draws them:
an explosion is transient and a static scene has nowhere to put one.

Which floats in a block are **live** is now settled. Each class keeps only a
pointer to its own block, so the fields that matter are whatever its virtual
methods load through it; walking the vtables of `Effect.dll` recovers the set.
**176 of the 441 four-byte slots across the ten block types are read as a
float** and the rest the editor wrote and the engine never looks at. Two
things check the map: the sound emitter's `+64` and `+68` were read off the
data long before it existed and the map contains them, and **not one of the
176 offsets lands inside a block's `(archive, member)` pair** even though the
map came from the code and `RESOURCE_AT` came from the data.

The class families fall out of it — types 3 and 9 read the same eighteen
offsets, 7 and 10 the same thirty-one, 4 a strict subset of 3's — and one more
field is named: **types 1 and 2 keep a unit vector at `+52`**, unit length on
597 of 618 and 517 of 517 blocks and exactly `(1, 0, 0)` on most. The drawing
types keep something else there; on type 8 not one block of 237 is a unit
vector.

What is still open is **what any of the live floats mean**. The values look
like colours, lifetimes, velocities and spreads. Type 3's `+40..+48` and
`+52..+60` are a component-wise (low, high) pair on 1427 of 1545 blocks and
the engine reads exactly those six, which is the data and the code agreeing;
types 7 and 10 read a `float32[3]` at `+56`, `+80`, `+104` and `+128`, a
24-byte stride. None of that names a field. The next handle is a method that
does something *recognisable* with a value — feeds it to a matrix, compares it
against a clock — rather than copying it into the particle it builds.

Also open: the 60-byte effect header, the `.exp`'s first float and flags word,
and what bit 8 controls.

A negative result worth keeping: an explosion's size is **not** in its effect.
`exp_frt_l`, `_m` and `_b` share their emitter blocks byte for byte; the 2, 3
and 4 that separate them are the magnitude in their `.exp`.

### 2.4 The `MAT0` class byte

Byte 4 sorts the library into eleven groups plus an unset value, and what it
*is* is still open. What is settled is how to read it.

`World3D.dll`'s parser gates the record's tail on a **version**: at 2 it reads
the bytes at +4 and +5, at 3 a `float32` defaulting to 1.0, at 4 a `uint32`
defaulting to 0 — and below each it substitutes the default. For the two bytes
that default is **`0xFF`**, so 0xFF is the engine's own *not set*, which is
what byte 5 holds on all 905 records and byte 4 on 376 of them. It is not a
twelfth group.

The other eleven sort the library by role, sharply: **0 to 4 hold 43 of the 45
multi-layer materials** and are almost all opaque ground, 5 is 342 object
materials of which every one carries alpha, 6 is the 87 `TREE*` and foliage,
7 is `WATER` and `WATER_M`, and 8, 9 and 10 are three smaller families of 24,
12 and 9. That is the shape of a shader or blend mode and it is the strongest
hint there is, but the field has exactly one consumer in the engine and
following it into `GetMaterialPhase`'s output struct did not name it.

---

## 3. Renderer engineering, not format work

- [x] **LOD switching** — done, once the geometry was validated; see section 0.
  All four levels are packed and the viewer picks one by apparent size, the
  thresholds being multiples of the model's own radius so a lamp post and a
  factory swap at the same size on screen rather than the same distance. The
  **Detail levels** button pins everything to level 0 for comparison.
- [x] **Alpha ordering** — done. The see-through layers now composite in a
  written-down order: the dome, the stars and the clouds behind everything,
  then the opaque world, then water, rain and the footprint overlay, and the
  lens flare last in its own overlay scene. Water no longer writes depth,
  which is safe because it never sorts against itself: **all 11 maps that
  carry water carry it as a single flat plane**, none at two heights.
- [ ] **Terrain culling** — the grid exists after all, and this entry used to
  say it did not. Face field 13 is not spatial, but **stream 2 is**: a 16 x 16
  or 8 x 8 grid of cells, each with a box, a centre, a radius and a run of
  faces, and every one of the 275882 faces lies inside its own cell. See
  section 0. Culling still buys nothing *here* — the worst map is **8400
  triangles** after de-duplication, median 7607, drawn in one call, and the
  survey camera frames the whole map — so the reader exposes the grid and the
  viewer does not use it. A game renderer would.
- [x] **Coplanar geometry** — done; see section 0. Nothing in the scene should
  be drawn twice at the same depth, and two things were: the terrain's two
  ground layers, now one pass, and the file's own duplicated faces, now
  filtered by `LandMesh.distinct_faces`.

## 4. Known-unknowns carried in the readers

Parsed and passed through without being understood. None affects a picture
today; each is a small trap for anyone extending the code.

- A batch's vertex range (fields 7 and 8): contiguous, but tiles the vertex
  array on only 69 of 435 meshes, so not a partition.
- Face record fields 10, 11, 12, and field 0 (near-constant per mesh).
- Terrain `Land.msh` stream 1 (mostly `0xFF`).
- Terrain stream 11's flags word — 72 on 4228 faces, 88 on 329; 88 correlates
  with water.
- `CTPT`'s nine floats read as `(zero, position, unit direction)` in
  `static.rlb` and `turrets.rlb` but not in `guns.rlb` or `parts.rlb`, which
  put scalars like `Width` in a vector slot.
- `BASE` (`.bas`) footprint records: a count then float triples, but the header
  does not divide evenly into the payload.
- 65 of 393 textures have a mip tail shorter than the declared level count.
  Harmless for level 0; a packer would need to reproduce it.
