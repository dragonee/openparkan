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
- [x] **Coplanar geometry.** Two causes, both fixed. The terrain's two ground
      layers now share a single pass — bucketing faces by the pair costs 5 to
      8 groups per map against 3 to 5 — and the file's own duplicated faces
      are filtered: **46283 of 275882** faces across the 33 maps repeat a
      triangle already in the list, and drawing both copies made the walkable
      ground flicker.

---

## 1. Wrong on screen today

These produce visibly incorrect output. Fix in this order. Both came out of
doing the pose work above.

### 1.1 A socket's rotation is thrown away

A part is mounted at its socket's position with its own orientation. That is
right in the sense that both alternatives are visibly worse (see
docs/07-objects.md), but it means a turret can never be drawn turned, and the
108 attachments whose socket and root rotations disagree by 180 degrees are
telling us something that is not yet understood.

### 1.2 Damage variants are guessed at

Two of every three five-slot blocks are unused by the renderer. 135 nodes
populate them, and `fr_b_brige`'s `o02` carries identical triangle counts in
variants 0 and 1 — a destroyed state is the obvious reading, but nothing
confirms it, and if it is wrong then something is not being drawn.

## 2. Missing fidelity

Correct as far as it goes, but not what the game showed.

### 2.1 The sky's weather layers

The dome now carries the mission's own nebula, stars and clouds, and its sun
and moon as billboards — `sky.wea`'s nine slots are a fixed role table and all
261 slot names across the 29 missions resolve (see
[docs/10-sky.md](docs/10-sky.md)). Four of the nine are not drawn:

- **The lens flares** (slots 5 and 6, `ENV_FLARE_00` / `ENV_FLARE_01`) need
  the sun's screen position and a chain of sprites down the view axis.
- **Snow and rain** (slots 7 and 8) are particle systems, which is
  `effects.rlb` — see 2.3.

And **where the sun stands** is the renderer's own arc, not the game's:
`CSun::Render` builds its matrix from two angles at `this+0x30` and
`this+0x34`, and no pair of floats in a keyframe varies with time the way an
azimuth and an elevation would.

Three smaller unknowns sit in the same file: the keyframe count of a second
section (six missions have one), which field selects the object type between
SUN, SKY, RAIN, SNOW and LIGHTNING, and most of the 124-byte file header.

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

`*.exp` explosion definitions (`system.rlb`) and `effects.rlb` are untouched.
Not needed for a static scene; needed for anything animated.

### 2.4 The `NL` archives block the UI

`gamefont.rlb` and `sprites.lib` are not NRes — `'NL'`, version 1, an `0xABBA`
marker, then high-entropy payload. Two files, holding fonts and 2D sprites.
Blocks any in-engine interface work and nothing else.

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
