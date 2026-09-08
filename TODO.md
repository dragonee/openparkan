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
      LZSS members of `gamefont.rlb` do **not** decode and are refused; see
      §2.4. → [docs/12-rsli.md](docs/12-rsli.md)
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

Two things do draw as flat grey rather than as art, and cannot be fixed from
the shipped data: `B_MTP_04`, `B_MTP_04G` and `B_MTP_05` name `qqds.7` and
`ds.7`, which are in no archive and have no near match, and the five
`FIRE_SMOKE*` animations name `0FAIR.0` upward when only `FAIR.0` exists.
That costs 206 triangles on `fr_l_gener` and 153 on `fr_m_mtp`.

## 2. Missing fidelity

Correct as far as it goes, but not what the game showed.

### 2.1 The sky's weather layers

Eight of `sky.wea`'s nine slots are drawn: the nebula, stars and clouds on the
dome, the sun and moon as billboards, the lens flare as a 2D overlay, and rain
where a mission asks for it (see [docs/10-sky.md](docs/10-sky.md)). The ninth
is snow, and **no shipped mission names it**, so there is nothing to switch
on.

What is left of the weather is where a shower *stops*: the sun and moon come
in start/stop pairs and rain does not, so the viewer runs it to the next
keyframe that names anything — a reading, not a fact.

And **where the sun stands** is the renderer's own arc, not the game's:
`CSun::Render` builds its matrix from two angles at `this+0x30` and
`this+0x34` — a rotation of the second about the horizontal axis at the first,
which is a Rodrigues matrix read straight off the disassembly — but nothing
writes those two fields from a file that has been found, and no pair of floats
in a keyframe or in the 124-byte header varies the way an azimuth and an
elevation would.

The flare's second intensity gate is in the same position: the engine ramps it
between the cosines of 30° and 60° of a float the sun object keeps at `+0x80`,
and what that float is has not been established.

Two smaller unknowns sit in the same file: the keyframe count of a second
section (six missions have one), and most of the 124-byte file header. The
object type is now half-answered — `CAtmData::GetEvents` dispatches on a
ten-valued opcode whose branches pair up as start/stop per object type, and
the sky is created outside that switch with a hardcoded id — but which *file*
field feeds the opcode is not pinned down, which is why where a shower stops
is still unknown.

### 2.2 The second layer of a terrain material

42 ground textures ship as a `L20.0` / `L20M.0` pair and the material names
both. They are **not** detail or bump layers, which an earlier note assumed
from `Iron_3D.ini`'s `EMBM=1`: correlation between the two is 0.994 median
over all 42 pairs, so the `M` half is the same image in XRGB8888 rather than
RGB565, at an exposure authored per texture (ratio 0.54 to 2.03).

`Iron_3D.ini` carries `BITDEPTH` and `RENDER_QUALITY`, which is presumably
what chose between them; which index goes with which setting is not
established, so the reader takes layer 0. Two materials have **eight** layers
(`B_LBL_01`, `R_LBL_01`) and those are unexplained.

Byte 4 of a `MAT0` record sorts materials into twelve groups that track their
names — all six `TREE*` share value 6, the effects share `0xFF` — and reads
like a shader or blend-mode id, but nothing confirms it.

### 2.3 Effects

Both formats are [read](docs/11-effects.md) — 923 effects walk their emitter
blocks to the byte, 144 `.exp` records parse, and a destroyed node's damage
record reaches real sprites on 2189 of 2203 references. Nothing draws them:
an explosion is transient and a static scene has nowhere to put one.

The block table is now the engine's rather than a fit: `Effect.dll`'s emitter
factory masks the word to a byte, subtracts one, bounds it at 9 and jumps
through a ten-entry table whose branches advance the read pointer by exactly
those strides. That added **type 6**, a 4-byte block nothing uses, and settled
**bit 8** — the factory stores `(word >> 8) & 1` on the emitter, so it is a
flag, not part of the type.

Inside a block, the sound emitter is read: type 2 keeps a **near and far
audible distance** at +64 and +68, ordered on all 517 blocks, (3, 40) and
(10, 100) being the commonest. The rest is open — ten types and nothing yet
says which is a sprite burst, which a trail, which a light; 30 to 60 floats
per block that read as colours, lifetimes and velocities. The emitter object
keeps only a pointer to its block, so the field offsets live in each class's
update method behind its vtable, which is where this goes next.

Also open: the 60-byte effect header, the `.exp`'s first float and flags word,
and what bit 8 controls.

A negative result worth keeping: an explosion's size is **not** in its effect.
`exp_frt_l`, `_m` and `_b` share their emitter blocks byte for byte; the 2, 3
and 4 that separate them are the magnitude in their `.exp`.

### 2.4 RsLi's LZSS keeps the font shut

The archives open, but `gamefont.rlb`'s two members are `0x040` LZSS and do
not decode. The obvious 12-bit offset, 4-bit length shape reproduces
`ARIALTEX.TFT`'s `Tfnt` header and lands a `Texm` magic at 4116 — where
arithmetic says one belongs, 20532 − 4116 − 32 = 16384 = 128 × 128 — and then
emits maximum-length matches from the wrong place, so the "pixels" are the
bytes `Texm` repeated. A ring-buffer index, a pre-filled window, other
starting positions and other minimum lengths all fail the same way, and the
size check cannot catch it because the output is truncated to fit.

The engine's decompressor has not been found: it is not beside the loader in
`Ngi32.dll`, and neither a 4096 window constant nor a dispatch on the seven
storage flags turns up in that DLL. Until it does, the font and its palette
stay shut. See [docs/12-rsli.md](docs/12-rsli.md).

---

## 3. Renderer engineering, not format work

No reverse engineering needed; just work.

- [ ] **LOD switching.** All four levels of each variant are parsed and
  `slots_for_lod` takes the level; the viewer always asks for 0. Switching by
  screen size is a payload change, not a format question.
- [ ] **Terrain culling.** There is no patch id to cull by: face field 13 turned
  out not to be spatial (see docs/06-open-questions.md), so a renderer has to
  build its own grid, which is what `LandMesh._build_index` already does for
  height queries.
- [ ] **Alpha ordering.** Cutouts need none, which is why they are what the
  viewer uses, but the graded textures behind effects and the sky will.
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
- Terrain `Land.msh` stream 1 (mostly `0xFF`) and stream 2 (737 float3 on
  SC_3: eight bounding-box corners then 729 = 27³ entries that look like a
  spatial subdivision).
- Terrain stream 11's flags word — 72 on 4228 faces, 88 on 329; 88 correlates
  with water.
- `CTPT`'s nine floats read as `(zero, position, unit direction)` in
  `static.rlb` and `turrets.rlb` but not in `guns.rlb` or `parts.rlb`, which
  put scalars like `Width` in a vector slot.
- `BASE` (`.bas`) footprint records: a count then float triples, but the header
  does not divide evenly into the payload.
- 65 of 393 textures have a mip tail shorter than the declared level count.
  Harmless for level 0; a packer would need to reproduce it.
