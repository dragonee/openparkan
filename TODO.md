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
      model z = 0 at the given height and nothing else: a building's origin
      lands a median **+0.03** above the terrain under it and a unit's lowest
      exterior vertex within **0.03** of it. The "rest it on its base"
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
      of the "eight unresolvable special materials". `WATER_M` is a ten-key
      animation, and water is blue because its material is `#4d6aff` — its
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
- [x] **The `.ctl` controller, as far as its frame.** A *movement*
      controller, not an animation one, and now [read](docs/13-control.md).
      All **531** members open with the same **212-byte frame** — a 128-byte
      parameter block whose 24 float slots are finite on all **12744** reads,
      six triples whose components agree on 2889 of 3186, and the engine's
      own defaults: a whole turn, `pi/2`, `FLT_MAX`, `-1`. Six members are
      the frame and nothing else. `Terrain.dll` carries a stub table at
      `0x100191a0` that names the whole of `IControl` — tangential, normal
      and world speeds, two accelerations, two angles and a calculation
      mode — which is a better account of a triple than `(x, y, z)`.
      The 100-byte **reference record** inside it says what the thing
      *emits*: all **1769** resolve, and every one of the 158 that names
      `objects.rlb` is a `BULL` — a projectile — carried only by the four
      archives that hold things which shoot. The old note here said "a body
      of 156-byte records": that was section 1's record with the second
      count zero, right about the number and wrong about what it counted.
      Its size still correlates
      **+0.97** with its own leading count and **+0.40** with the node count
      of the mesh it belongs to, over 542 records, so it holds no per-node
      data. `.ndp`, the other unread `STAT` slot, is a hit-point float and
      an `(archive, member)` pair naming an `.exp` explosion.
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
- [x] **What the `*M` ground texture is.** The 43 two-track ground materials
      pair a texture in RGB565 with its `M` twin in XRGB8888, and the twin is
      the same picture **flattened towards neutral grey**: closer to grey on
      **40 of the 43** pairs, and moved furthest where the base started
      furthest away — `L25`, a near-white snow, goes from a mean distance of
      110 to 12. A per-channel fit of `M = a.base + b` leaves a residual of 2
      to 15 levels out of 255. So it is neither a mask, which this reader used
      to call it, nor a bump map, which an earlier draft guessed from `EMBM=1`,
      nor a plain bit-depth copy — a copy would not be pulled towards grey, and
      would not be pulled hardest exactly where the original is least grey.
      What the renderer does with it is settled too, below: it is the
      material's second track, drawn unlit.
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
- [x] **The terrain's square table.** `Land.msh` stream 1 is the other half of
      the index: one 19-`uint16` record per grid square, four words of header
      then room for **15 cell indices** with `0xFFFF` for empty. It divides
      exactly on all 33 maps, and every one of the **7488 squares uses exactly
      two** slots — its own two, since square `i` names cells `i` and
      `squares + i` and that pair always shares a bounding box. The four
      header words are `0, 0xFFFF, 0, 0` throughout, so nothing distinguishes
      them. It is **not** a quadtree, which the 4x step between the small and
      large maps had suggested — there is no hierarchy in either stream, just
      a flat grid and a list per square. With this the terrain format is read
      end to end. → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The terrain's spatial index.** `Land.msh` stream 2 is not an
      unexplained blob: it is the map's own grid. Eight bounding-box corners,
      then a 68-byte record per cell holding `uint16 first`, `uint16 count`,
      the cell's box, its centre and a bounding-sphere radius. It **parses
      with nothing left over on all 33 maps**, the runs chain end to end, and
      their counts **sum exactly to the face count** — so faces are stored in
      cell order and a run indexes them directly. **All 275882 faces lie
      inside their own cell's box.** The grid is 16 x 16 on 28 maps and 8 x 8
      on five, following the mesh's vertex count rather than the world size.
      Each cell is listed twice, and the two records are the same ground at
      two levels of detail; see the entry below.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The whole `MAT0` record, end to end.** A 14-byte header, then the
      entries at **34 bytes** each, then a table of animation tracks over
      them — and all **905 records parse to the byte with nothing left
      over**. An entry is a **`D3DMATERIAL7` written as bytes**: ambient,
      diffuse, specular and emissive as three colour bytes over 255 and an
      alpha in per cent, then a specular power, a signed cell byte and a
      16-byte name. That is what makes 34 the stride, and it is checkable —
      **not one of the 12572 alpha bytes exceeds 100**, which the best of
      twelve other (base, stride) pairs cannot match. The version the header's
      last four fields are gated on is **not in the record**: it is the
      archive directory entry's second count, 6 on all 905, which is why the
      header is 14 bytes rather than the 6 an older record would have.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **The `MAT0` opacity, which was being read as a marker.** It is the
      entry's **ambient alpha**, in per cent — `World3D.dll`'s parser
      multiplies all four alpha bytes by 0.01. It looks constant because 3138
      of the 3143 entries are fully opaque; the exception gives it away,
      `FIRESTORM` running **0, 60, 80, 90, 95, 100** across its frames, which
      is a fade-in and is not something a marker can be.
- [x] **Every texture name.** The names had been extracted by pattern, and the
      pattern swallowed whatever alphanumeric byte sat in front of a name: so
      `B_MTP_04`'s texture read as `qqds.7` when it is `MTP_04.0`, and the
      `FIRE_SMOKE` animations' as `0FAIR.0` .. `7FAIR.0` when all fourteen
      frames name `FAIR.0` and those digits are cell bytes. Read by offset,
      **all 905 materials name a texture that is in Textures.lib**, against
      891 by pattern — which empties section 1 — and the cell check now covers
      **2513** cells across every entry of every material rather than 478
      first entries. → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **What a material's second "layer" is — and it is not a layer.** The
      second `uint16` counts **animation tracks**, not texture layers; the
      engine caps it at 20 with *"Too many animations for material."* and each
      track is a list of 6-byte keys naming an entry and a time. `WATER_M` is
      one track of ten keys 200 apart, not one layer of ten frames. Every one
      of the **102 tracks across the 45 materials that have more than one
      holds a single key, and track *i* names entry *i***, so the extra tracks
      are alternative renderings of one surface: the 43 ground twins, and the
      two eight-track insignia sheets. → [docs/07-objects.md](docs/07-objects.md)
- [x] **What the engine does with the ground's `M` twin.** Nothing
      multitextured. On all 43 pairs the twin is track 1's entry, and what
      separates it from track 0 is the **lighting**: on 38 of the 43 the first
      entry carries a white diffuse over a black ambient and the second the
      reverse, so the base is lit by the scene and **the twin is drawn
      unlit** — which is what a copy flattened towards mid-grey is for. The
      engine's multitexture path exists and this is not it: `Ngi32.dll` keeps
      14 render phases at `0x10036a30`, `SetPhase(mode, tex0, tex1)` binds
      argument 0 to stage 0 and argument 1 to stage 1 on **every** two-texture
      phase, and `CShade::ConfigureTextureAndAlphaBlendModes` picks a phase
      per device and falls back to a second pass where the hardware is short.
      But a `MAT0` entry carries one texture and one cell, so no material ever
      reaches it. → [docs/03-terrain.md](docs/03-terrain.md),
      [docs/05-engine.md](docs/05-engine.md)
- [x] **How a material draws** — and it is not in the record. The archive
      directory's first count field is a **flags byte**, the one `World3D.dll`
      branches on, and it sorts the library by blend mode: **0** opaque and
      lit (52 of the 54 name a texture with no alpha at all), **2** the
      ordinary lit skin (**3140 of the 3143** model-wear references, 261 with
      specular), **4** see-through (smoke, dust, most of the sky; 175 of 219
      unlit), **8** **additive**. The additive reading holds from four
      directions: **44 of the 46** materials the artists named `*_add` carry
      it, along with every `JET*`, `SHOOT*`, `LASER_*` and `SPLASH*`; **210 of
      214** carry a black diffuse; **not one** carries a specular against
      261/417 of the flags-2 skins; and an effect's emitters name **2519 at 8
      and 980 at 4** against 24 ordinary skins. `Material.blend` carries it,
      and the viewer uses it with the texture rather than instead of it: the
      flags say how a material *composites*, the texture says what its alpha
      *means*. Additive blends whatever the alpha's shape; see-through blends
      a graded alpha (**166** of the 219) but alpha-tests a silhouette (32 —
      the foliage, `FTREE1`, `HTREE1`, `GRASS`, `ELKA`); everything else keeps
      the old heuristic and has its alpha dropped in the reader, which is what
      stops a gloss map punching holes through a building.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **Animation playback.** **157 of the 435 meshes carry one**, and three
      facts make it playable as a bone per node. It is **rigid**: every one of
      the 296379 vertices those meshes hold is reached by exactly one node, so
      a single weight plays it and there is nothing to blend. **A key's time
      is the frame at which its run first names it**, on all **33020** keys --
      the frame map repeats a key to hold it, so the times are what to
      interpolate between; a turret's nine frames are six keys, and stepping
      them jumps 90 degrees at a time. And **an animated node's rest pose is
      its own first frame**, on all 817, which is what lets the bind pose come
      from the rest pose. The rigs read as rigs: `R_H_02` is a body with four
      bones down each leg, and only the body's key translates.
      A mounted part now hangs from the **bone** of the node it attaches to
      rather than from a fixed pose, so a walking chassis carries its guns.
      An animated model is quantised into a cube rather than its own box,
      because the skin runs in the geometry's space and a rotation does not
      survive a non-uniform scale.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **The `.bas` block that "did not divide evenly".** It is not a header:
      a ring the author **traced on the model** carries a back-reference per
      corner — `count` int32 triangle indices, then `count` int32 corners — so
      the point is `mesh.triangles[triangle][corner]`. On the **16 of 30**
      records whose outline still sits on the shipped mesh it resolves on
      **154 of 154** points, all-or-nothing per record; on the other 14 no
      ring point lands on a vertex at all, so those were traced on geometry
      the mesh no longer carries. And it explains why only one ring has it:
      **ring 0 is the inner one on all 30**, the outline taken off the
      building, while the outer ring is a clearance the author drew.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **What `Land.msh` stream 11 is.** The draw order. Four bytes a face — a
      `uint16` index, a flags byte, and a byte that is zero on 275566 of
      275882 — and the indices are a **permutation of the whole face list on
      all 33 maps** that reorders faces only *inside* a cell: all **14976**
      cells stay contiguous. Within a cell it sorts by texture pair, and
      optimally — in this order **every one of the 14976 cells draws in the
      minimum number of batches**, no pair twice, against 11463 in file order.
      A renderer that buckets by material itself does not need it.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The "truncated mip tails", which were never truncated.** 65 of the 393
      textures have bytes left over after the mip pyramid, and an earlier
      reading called the tail short. Every declared level is present on all
      393; the leftover is the **`Page` sub-image table**, and it accounts for
      the payload to the byte on all 65 — `Page` magic, an 8-byte header and
      8 bytes a page. So `Textures.lib` is read with nothing unexplained.
- [x] **`CTPT`'s nine floats**, and the exception that was not one. The record
      is `(zero, position, vector)` — the first triple is exactly zero on
      **3432 of 3599** — and the third is a direction whose **length is a
      magnitude**. An earlier draft said `guns.rlb` and `parts.rlb` put
      scalars like `Width` in a vector slot; they do not. **All 191** points
      named `Width`, `Height` or `Size` have exactly one non-zero component,
      so `Width_1` at `(0, 0, 0.42)` is 0.42 across the model's z, and
      **553 of 570** frame and aim points are unit length. One reading covers
      every archive. → [docs/07-objects.md](docs/07-objects.md)
- [x] **A batch's vertex range**, which was never a partition. It is D3D's
      `(BaseVertexIndex, NumVertices)`: `vertex_count` is **exactly the largest
      relative index plus one on all 15153 batches of all 435 meshes**, so it
      is the span the driver has to transform, and two batches are free to
      reach the same vertices. That is why it tiled the array on only 69 of
      435 — it was never trying to.
- [x] **Terrain face fields 10, 11, 12 and field 0's bits.** 10..12 are the
      **face's own normal**, `int16` over 32767: unit length on **275881 of
      275882** and agreeing with the triangle's cross product on **275877**.
      Field 0 is a bitfield over a constant `0x600`, and `0x004` marks a face
      with a **second texture layer** — set on exactly the **32450** that have
      one and on none of the other 243432 — beside the known `0x008` for
      water. `0x2000` is still unread.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **The last two terrain bits that mattered.** The draw order's flags byte
      says **where a batch begins**: bit `0x10` is set on exactly the **27174**
      faces that open a run of one texture pair inside a cell and clear on all
      248708 others, agreeing on every one of the 275882 — so walking the
      order and changing material where the bit is set draws the map. And face
      flags bit `0x2000` marks the **bed beneath a liquid**: exactly the
      **6102** faces whose layer-1 material is `WATER_BOT` or `ENV_LAVA_BOT`,
      and no other. → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **Object mesh stream 7**, the last per-face record either format
      carried unread. **One 16-byte record per triangle on all 435 meshes** —
      241887 — and it mirrors the terrain's: a flags word, **three edge
      neighbours** (`0xFFFF` for none) and **the face's own normal** as
      `int16` over 32767. Both halves check exactly: the normal is unit length
      on all 241887 and agrees with the triangle's cross product on 241879,
      and **674200 of the 674206** in-range neighbours share two vertex
      positions with the face that names them — the six that do not are all in
      one mesh. The flags word takes six values and the trailing field has the
      shape of the terrain's six-bit class; neither is named.
      → [docs/07-objects.md](docs/07-objects.md)
- [x] **The renderer's interface.** `IDirect3DDevice7`, pinned by the vtable
      offsets the engine calls through — `SetRenderState` at 20,
      `DrawPrimitive` at 25, `SetTexture` at 35, `SetTextureStageState` at 37,
      `ApplyStateBlock` at 39, with nothing left over — and by six declared
      vertex formats whose FVF codes and strides check each other to the byte.
      Three layouts, each with a one-texture form and its two-texture twin.
      `BITDEPTH` and `RENDER_QUALITY` were a dead end: the real switch is the
      registry value `Disable MultiTexturing` under
      `HKCU\Software\Nikita\NgiTool`. → [docs/05-engine.md](docs/05-engine.md)
- [x] **The eight-layer materials.** Not eight images and not eight layers:
      `B_LBL_01` and `R_LBL_01` are eight *tracks* of one key, all naming
      `PG27.0` and asking for cells 0 to 7 of it. They are the blue and red
      team variants of one insignia sheet.
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
      8 groups per map against 3 to 5 — and the map is no longer drawn twice;
      see the level-of-detail entry below.
- [x] **The terrain flicker, and it was never a duplicate-face problem.**
      `Land.msh` stores the map **twice**, at two levels of detail: every grid
      square names one cell per level, level 0 as authored and level 1 a
      simplification of it. Four things say so on all 33 maps — the levels are
      two contiguous slices of the face array; **each covers the whole map on
      its own**, a random point landing inside exactly one face of each;
      level 1 **introduces no vertex level 0 does not**; and level 1 is no
      finer in any of the **7488** cell pairs. An earlier draft saw only the
      **46186** faces the simplifier had left alone, called them duplicates
      and filtered them — but the other **55869** are the ones it changed, and
      those sit a fraction of a unit from the surface they replace. That
      residue was the flicker: thin wandering seams over gentle ground (median
      slope 14–33°) and streaks along the ridges where the two levels part by
      up to 70 units. `LandMesh.lod_faces(0)` draws the fine level alone —
      **173827 triangles instead of 275882**.
      → [docs/03-terrain.md](docs/03-terrain.md)
- [x] **Depth precision at range.** The survey runs from a 1.5-unit chassis to
      a 26000-unit sky dome, and a linear depth buffer over `near = 1` resolves
      about **half a unit at 3 km** — enough to make a bunker's base and the
      ground it rests on swap places as the camera orbits. The renderer now
      asks for a **logarithmic depth buffer**, which holds about 0.002 units at
      the same range.

---

- [x] **The input layer.** The game ships it as commented plain text and
      nothing here read it. Now [read](docs/14-controls.md): `ScanCode.dsc`
      (174 keys), `Command.dsc` (72 actions with an English sentence each),
      12 `.man` files (**275** bindings, every one naming a command and keys
      that exist), and the three `.tbl` control tables (**116** rows, all
      eleven fields, all 31 scan names known). A row names the target class
      (`CICLS_TURRET`, `CICLS_CAMERA`) and the command (`MCMD_ANGLE_X`), and
      a key coming up re-sends the same command with 0.0 on 21 of 27 release
      rows. `BuildDat.lst` reads too: all 32 assemblies exist, and it holds
      **12** schemes where its own header says there must be 11.

- [x] **What the controller's messages are.** Settled, and it came from the
      text layer rather than the disassembly. `World3D.dll` carries a
      name resolver for every family the tables use, so the engine's own
      numbers are recoverable: all **174** scan codes (the real IBM PC set-1
      codes — `SCAN_A` is 30, `SCAN_ESC` 1, `SCAN_F1` 59), 22 `MCMD_`
      commands, 13 `CICLS_` classes and 15 `CIS_` state bits. The
      controller's dispatch is a 16-way jump table, and **`MCMD_` 1 to 16 is
      exactly that range** — the join the previous entry was missing.
      `MCMD_WALK_F` (19), `MCMD_WALK_B` (20) and `MCMD_LOCK` (21) fall
      outside it on 14 of the 116 rows, so walking is not the movement
      controller's job. → [docs/14-controls.md](docs/14-controls.md)

- [x] **The `.ctl` frame is the controller object.** Not a serialisation
      format with a layout of its own: `Control.dll`'s initialiser at
      `0x10006689` writes a compiled-in default into every field of the
      controller it builds, and that default is the **commonest value in
      the shipped files on all 27 slots** — 2.5 for the first triple, 6.28
      for the two angular ones, 1.0, -1.0, `FLT_MAX`, 1.57079. So object
      offset = file offset + `0x45c`, and a controller that leaves a slot
      alone carries the engine's own value rather than an artist's. The
      engine's "2π" is **6.28** and its "π/2" is **1.57079** — typed
      decimals, not the constants — and the reader now keeps those bits.
      → [docs/13-control.md](docs/13-control.md)

- [x] **The `.ctl` sections, all but one.** The loader at `0x10008b10`
      reads the five counts one at a time and then walks the body:
      **section 1** is `A` records of `156 + 16*B` followed by `A*A` int32
      (the engine computes the same span twice, once per record and once as
      `A*(A + 4*B + 39)*4`, which checked the arithmetic before a file was
      opened), **section 2** is `C` records of 36 bytes, **section 4** is
      `D` type-dispatched component records, then a fixed **84-byte block**
      the loader copies, then **section 5**, `E` groups of an int32 and that
      many 100-byte records. All **136** members with no component records
      are consumed to the byte, and all **395** that have them start section
      4 with an id in 1..30 — the range the factory at `0x1002d4b0`
      dispatches. The 84-byte "trailer" was never padding: it is copied into
      the object, and it only looked adjacent to the frame because the six
      smallest members have nothing in between.
      → [docs/13-control.md](docs/13-control.md)

- [x] **The `.ctl` component records — the format is finished.** A section-4
      record is one shape for all 30 type ids: thirteen of the factory's
      fourteen classes parse it with the same code at `0x10021d50`, and the
      fourteenth calls that code first. It is `0xb0` fixed bytes, then `N`
      int32 where `N` is at +0xac, then a length and that many bytes of
      label. **All 531 members walk end to end**, 1066 component records
      among them. A component's label names a family of internal parts:
      **all 57 distinct labels prefix an `objects.rlb` member and every one
      of the 186 they reach is an `INTO` record**. A section-5 record turned
      out to be nine int32 **then** the name pair, not the other way round —
      anchoring on names had put the ints where the names are. A controller
      names **1769** resources and every one resolves.
      → [docs/13-control.md](docs/13-control.md)

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

What is left of the weather is where a shower *stops* — and this round showed
why it was never going to be found in the file.

`CAtmData::GetEvents` (`0x1006dc10`) dispatches through a jump table at
`0x1006e829`; cases 2 and 7 share the out-of-range target and do nothing,
and the other eight pair up as start/stop per object type. Its records come
from `0x1006d740`, which splits the query at midnight and calls the collector
at `0x1006d460`. The collector asks the atmosphere data object for a section's
keyframes one at a time and receives a **0x98-byte record whose layout is the
engine's, not the file's**: `+0x00` the opcode, `+0x20` the hour, `+0x24` the
minute, `+0x64` a *pointer* to the name. **The opcode is assembled in memory.**

The data agrees. No slot of the 22 carries 0..9 across the 656 keyframes —
slot 5 is the only small one and it is 0 throughout. The trailer's last word
does span 0..9, but it puts **438 of 656** keyframes on case 7, the no-op,
including 60 named `sun` and 59 named `moon` — bodies that must start and
stop. So it is not the opcode, and the field hunt is closed as a dead end. The
next handle is the data object's own vtable, one hop further out.

**The sun's lifetime is half answered.** The collector's conversion is
`t = (hour * 3600 + minute * 60) * scale / 86400` — seconds since midnight,
through a per-section scale, over a day. What the scale *is* comes from a
virtual call on the same object and is the last piece. It changes nothing on
screen: the start and stop keyframes already say when the sun is up.

Two smaller unknowns sit in the same file: the keyframe count of a second
section (six missions have one), and most of the 124-byte header.

### 2.2 Who asks a material for its second track

Everything else about the ground's `M` twin is closed and sits in section 0.
What is left is the **selector**, and this round narrowed it a long way
without closing it.

The material manager's vtable is at `0x100209e4` — an earlier note had the
base wrong by two slots, which is why the first search looked at the wrong
offset. `GetMaterialPhase` is **index 5**, at `+0x14`, and it is a plain
stdcall:

```
GetMaterialPhase(self, handle, track, time, &out)      ret 0x14
```

`handle` packs `(table << 16) | index`; `track` is clamped to 0 when it falls
outside the material's track count; `out` receives a 0x50-byte descriptor —
a `D3DMATERIAL7`, one texture and one cell — written into a **static** at
`0x1013eab0`, and the function *returns* a pointer to `material + 0x164`, the
block the directory flags byte fills. There is a **four-argument sibling at
index 3** (`0x100031f0`) that returns the same pointer without a track index.

**No caller was found.** `LoadMatManager` is imported only by `Terrain.dll`,
which calls it three times and then reads the stored pointer twice, for vtable
index 6 and index 1 — never index 3 or 5. Neither entry point is the target of
a direct call or a jump anywhere in `World3D.dll`, and a scan of
`Terrain.dll`, `iron3d.dll`, `AniMesh.dll`, `ArealMap.dll`, `Effect.dll`,
`Control.dll`, `MisLoad.dll` and `Wizard.dll` for a call through `+0x14` with
five stack arguments, an out-pointer `lea`d from a local and a float in the
window turns up nothing that fits.

That is not proof of dead code — the pointer could reach a caller by a route
the scan does not model — but it does mean the reader's choice of track 0 is
not merely a default: nothing found asks for another.

### 2.3 Effects

Both formats are [read](docs/11-effects.md) — 923 effects walk their emitter
blocks to the byte, 144 `.exp` records parse, and a destroyed node's damage
record reaches real sprites on 2189 of 2203 references. Nothing draws them: an
explosion is transient and a static scene has nowhere to put one.

Which floats in a block are **live** is settled — **176 of the 441** four-byte
slots across the ten block types — and the map now has a **third witness, from
the data**. A slot the engine loads as a float should hold one, and every one
of the **97347** reads of the 176 is finite and either exactly zero or between
1e-6 and 1e6. The 127 dead slots manage **92.4%**: 5395 of their reads are
NaN, denormal or absurd, which is what an editor's uncleared buffer looks
like. The map came out of the vtables and used none of the values.

The slots are now **typed**, if not named: 37 signed, 59 positive, 32 in
0..1, 33 integral, and **15 that are always zero** — read by the engine and
never set by the artists. And type 3's (low, high) triple is a **motif**, not
a special case: type 10's `+80..+88` against `+128..+136` is ordered on 154 of
160 blocks and *identical* on 154, `+92..+100` against `+104..+112` ordered on
all 160 and identical on 158, and type 4's `+100..+108` against `+112..+120`
ordered on all 202. A block is a parameter sheet of randomised ranges.

**Two handles were followed and one is dead.** `Effect.dll` imports
`ngiGetClocks`, `ngiGetSinCos` and `g_FastProc`, which are the only things in
it that would *name* a value. The clock is spent: all 13 of its call sites are
`call ngiGetClocks; mov [state], eax; ret`, seeding a random generator, and
nothing compares a block float against elapsed time — so a lifetime cannot be
found that way, which the previous entry here assumed it could.
`ngiGetSinCos` has **exactly one** call site, at `0x1000c293`, inside the
emitter's random-direction routine: it draws from the generator, scales by
1/65536, feeds sin/cos, scales a vector by three factors and hands it to
`g_FastProc`'s transform against a matrix at the object's `+0xc0`. The
routine takes those factors as *arguments*, so the offsets that feed it are
one call further out — and that caller is the next handle.

Also open: the 60-byte effect header, the `.exp`'s first float and flags word,
and what bit 8 controls.

A negative result worth keeping: an explosion's size is **not** in its effect.
`exp_frt_l`, `_m` and `_b` share their emitter blocks byte for byte; the 2, 3
and 4 that separate them are the magnitude in their `.exp`.

### 2.4 The `MAT0` class byte

The *useful* half of this is closed and sits in section 0: the archive
directory's flags byte is the blend mode, and a renderer can read it. What is
left is the record's own **byte 4**, which sorts the library into eleven
groups plus an unset `0xFF` — and nothing in the engine follows it.

The record's tail is gated on a **version** that is not in the record either:
it is the directory entry's second count field, 6 on all 905. At 2 the parser
reads the bytes at +4 and +5, at 3 a `float32` defaulting to 1.0, at 4 a
`uint32` defaulting to 0, substituting the default below each. For the two
bytes that default is `0xFF`, so 0xFF is the engine's own *not set* — which is
what byte 5 holds on all 905 records and byte 4 on 376. It is not a twelfth
group.

The other eleven sort the library by role: **0 to 4 hold all 43 two-track
ground materials**, 5 is 342 object materials, 6 the 87 `TREE*` and foliage, 7
`WATER` and `WATER_M`, and 8, 9 and 10 three smaller families of 24, 12 and 9.
That is the shape of a shader id, but the loader copies it into the material
and **nothing reads it back**: the constant that addresses it, `0x10066b44`,
appears three times in `World3D.dll` and every one is in the loader or in the
tiny accessor that hands out its address. Whoever calls that accessor is
unfound — the same gap as §2.2.

Two smaller unknowns sit beside it. The version-3 `float32` is 1.0 on all 905
records, and the version-4 `uint32` is 0 on 901 and a *float* on the other
four — 1000.0 twice and 9999.0 twice — so it is a distance or a range the
engine reads as a dword.

---

## 3. Renderer engineering, not format work

- [x] **LOD switching** — done, once the geometry was validated; see section 0.
  All four levels are packed and the viewer picks one by apparent size, the
  thresholds being multiples of the model's own radius so a lamp post and a
  factory swap at the same size on screen rather than the same distance. The
  **Detail levels** button pins everything to level 0 for comparison.
- [x] **Cutaway** — done. 21 of the 435 meshes carry nodes the file marks
  internal, and the inside is most of the model: **29460 triangles against
  10725** of shell. The **Cut away** button hides the `o*` shell on the models
  that have one, which costs no rebuild — a group's `count` goes to zero and
  the geometry is shared by every placement, so one edit opens every building
  at once. A model with nothing inside is left alone, since cutting a tree
  open would only make it vanish.
- [x] **Animation** — done; see section 0. Every placement of a model shares
  one skeleton and they animate in step, which is what a survey wants; the
  **Animate** button pins everything to frame 0 for comparison. What is *not*
  read is what would choose an animation: a mesh carries exactly one, so a
  walk cycle and a turret sweep are the same track, and what triggers which is
  gameplay.
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
  section 0. Culling still buys nothing *here* — the worst map is **6259
  triangles** once only the fine level is drawn, median 5852, in one call, and
  the survey camera frames the whole map — so the reader exposes the grid and
  the viewer does not use it. A game renderer would, and it would switch
  levels **per cell**; the viewer packs both and offers the coarse one as a
  toggle instead, which costs only its indices because its vertices are a
  subset of the fine level's.
- [x] **Coplanar geometry** — done; see section 0. Nothing in the scene should
  be drawn twice at the same depth, and two things were: the terrain's two
  ground layers, now one pass, and the map's second level of detail, now not
  drawn at all (`LandMesh.lod_faces`). What is left is honest depth precision,
  and the renderer now asks for a logarithmic depth buffer to get it.

## 4. Known-unknowns carried in the readers

Parsed and passed through without being understood. None affects a picture
today; each is a small trap for anyone extending the code.

- Terrain **surface word bit `0x10`**: set on 244714 faces and clear on 30250,
  in connected regions, and it tracks *none* of slope, elevation, material,
  level of detail, map edge, duplication between levels or coverage by the
  navigation mesh. fparkan's notes say the word is a 16-bit compaction of a
  32-bit engine mask and that this bit is the engine's `0x00001000`, but do
  not name it either.
- Terrain **face field 13**, now known to be *six flags* rather than an id —
  fparkan documents a six-bit surface class packed from the same mask, and
  every one of the 275882 faces holds a value below 64. Which flag is which
  is open; none of the six tracks slope, height or face size.
- Terrain **draw-order flags bit `0x80`**, on 89 faces across the 33 maps. The
  rest of that byte is read: `0x10` opens a batch and `0x48` is constant.
- Object mesh **face flags** (0 on 233714 faces, then 2, 4, 16, 32, 34) and the
  **class** beside them, which sits below 64 on 240500 of 241887.
- The `.ctl` fields' **meaning**, now that their extent is settled: the
  component record's 64-byte block at +0x2c and its 4-byte entries, section
  1's and section 2's record contents, the 84-byte block, and the nine ints
  of a section-5 record.
- Which `.ctl` field feeds which channel. Two of the three legs are now
  closed. The message side: the controller dispatches `MCMD_` 1..16. The
  storage side: the frame from +20 on **is** the live object's parameter
  block at `file + 0x45c`, proved by the initialiser at `0x10006689`
  writing a default into all 27 slots that is the commonest value in the
  shipped files, 27 for 27. What is left is only the wiring between them —
  the handlers for messages 8..12 reach indexed channels on two sub-objects
  (`+0x5c8`, a 0xf4-byte class with 0x1c-stride channels defaulting to 0.5;
  `+0x5cc`, a 0x120-byte class with two arrays of six floats), and nothing
  yet says which triple either was loaded from. The read/write scan narrows
  it: **17 of the 32 slots are never touched in `Control.dll` outside the
  initialiser**, `+124` turns out to be run-time state rather than a
  parameter, `+116` is a bitfield whose bit 0 the engine tests, and
  `+44`/`+48`/`+52`/`+56` are compared rather than multiplied in.
- What handles `MCMD_WALK_F`. `MCMD_WALK_B` (20) and `MCMD_LOCK` (21) are
  answered — `Control.dll` has a second control class whose table covers
  messages 20 to 28, and `AniMesh.dll`'s agent takes both as well, keeping
  `MCMD_LOCK` for itself where it forwards everything else. `MCMD_LOCK`
  turns out to carry a sub-command rather than a magnitude. But **19 has no
  handler in any of the six modules scanned**, and six shipped rows send it:
  the `W` key, down and up, in all three tables. → [docs/14-controls.md](docs/14-controls.md)
