# TODO — renderer

What is left to draw a Parkan scene correctly. Ordered by how much each item
costs you in a picture, not by how interesting it is.

Format questions that do not affect rendering (save games, the network
protocol, `.scr` semantics, leftover `data.tma` words) live in
[docs/06-open-questions.md](docs/06-open-questions.md).

Status of what already works is in the [README](README.md); every claim below
was measured against a real install and can be re-derived with
`uv run openparkan verify`.

---

## 1. Wrong on screen today

These produce visibly incorrect output. Fix in this order.

### 1.1 Node poses — parts float, `fr_l_gener` is spiky

**Blocks:** every multi-part model — buildings with sub-assemblies, turrets,
guns, and any assembled robot.

A mesh's stream 8 holds 24-byte keys: a `float32[3]` translation followed by 12
bytes of packed rotation. A node's `fallback_key` selects one, and poses
compose down the parent chain — parent rotation turns the child translation,
translations sum, rotations multiply.

Composing the **translations alone** yields offsets like (54.5, 77.1, 57.2) on
`fr_l_bunker`, which throws geometry off the map, so the rotation is load
bearing and its packing is not decoded.

What is known about those 12 bytes:

| key | bytes 12–23 as `int16[6]` |
|---|---|
| identity | `(0, 0, 32767, 0, 0, 0)` |
| quarter turn | `(0, 0, 23220, 0, 0, 23119)` |

23220/32767 = 0.7087 and 23119/32767 = 0.7056 — cos and sin of 45°, so a
half-angle representation is in there. Some keys read as a `float32` 1.0 at
offset 12, which argues against a plain `int16[6]`.

**Where to look:** fparkan documents `node38_fallback_pose`, type-8 keys and
the type-19 frame map in `docs/reference/msh.md` — read the docs, not the
source (see [docs/09-method.md](docs/09-method.md) on why). Otherwise
`Terrain.dll`'s `LoadBuilding` is unoptimised and readable.

**Test it with:** 3599 real keys across ten archives, plus the visible check
that `fr_l_gener` stops being a star and building sub-parts stop floating.

### 1.2 Unit scale — robots render about 1/20 too small

`r_h_02.msh` is 390 triangles inside a 0.7 × 0.8 × 1.26 box, against a bunker
52 units tall. Chassis meshes in `bases.rlb` are all authored at this scale.
The mission record's `scale` is `1,1,1` in every shipped mission, so the factor
comes from elsewhere — most likely the `.dat` component words or the `.ctl`
controller.

Probably falls out of 1.1, since both are transform data.

### 1.3 The vertical datum for buildings is a heuristic

The viewer rests every model on its own base (`-min(z)` of the drawn
geometry). That is right for units and scenery, which are authored
base-at-origin, and it stops buildings sinking — their exteriors are authored
symmetric about z = 0. But the engine must have a real rule and this is not it.

Ruled out already: `Root` control points (buildings carry `P###` and no
`Root`), the `.bas` footprint plane (symmetric z range, so not a ground
plane as read), the slot AABBs (identical to raw vertex bounds), and the root
node pose (identity on every building checked). See
[docs/07-objects.md](docs/07-objects.md).

### 1.4 A unit draws as its chassis only

A `.dat` assembly lists its parts but not where they attach. The `.cpt` control
points are plainly the raw material — `TurretCenter`, `TurretDirect`,
`foot_fl`, `Dir_1` — but nothing yet says which point on a chassis a given part
binds to. Needs 1.1 first: without poses there is nothing to attach *to*.

---

## 2. Missing fidelity

Correct as far as it goes, but not what the game showed.

### 2.1 Terrain draws one texture layer of two

Terrain carries a full second layer that is parsed and then ignored: stream 18
(layer-2 UV), stream 14 (per-vertex blend weight, `float32` 0..1), and the
`Land2.wea` name table. The viewer uses layer 1 only, so ground transitions are
hard instead of blended. Everything needed is already in `LandMesh`.

### 2.2 Animated and special materials

Eight terrain names never resolve in `Textures.lib` — `WATER`, `WATER_M`,
`WATER_BOT`, `B_S0`, `B_MTP_01`, `ENV_NLAVA`, `ENV_NLAVA_M`, `ENV_LAVA_BOT` —
and are drawn as flat colours. Separately, 85 of 15138 object draw batches
reach a texture name like `0FAIR.0`, `1FAIR.0`, `2FAIR.0`: numbered animation
frames held outside `Textures.lib`. `Material.lib` is the place to look; its
`MAT0` records carry a layer count and `World3D.dll` complains about "Too many
animations for material".

### 2.3 Multi-layer materials

`MAT0` declares a layer count — up to 29, and 377 of 905 materials have more
than one — and the reader takes only the first texture. The
per-layer stride varies with layer type and is not mapped, so the colour bytes
around each name are unread. Detail and bump layers are being dropped —
`Iron_3D.ini` has `EMBM=1`, so the game used environment-mapped bump mapping.

### 2.4 Palettised transparency

Format-`0` textures decode fully opaque. Whether a palette index acts as a
colour key is unknown, and the foliage textures strongly suggest one does —
vegetation is alpha-billboard geometry, so without this trees render as solid
slabs rather than leaves.

### 2.5 Lightmaps

`lightmap.lib` is 2.7 MB of NRes and has never been opened. Static lighting is
presumably in there; everything is currently lit by one directional light.

### 2.6 Skyboxes

`sky.ske` (binary) and `sky.wea` per mission name environment textures —
`ENV_NEBULA_0`, `ENV_STARS`, `ENV_SUN_3`, `ENV_MOON`, `ENV_FLARE_00`. The
`.wea` half is already readable with `mesh.read_wea`; `sky.ske` is not parsed.

### 2.7 Effects

`*.exp` explosion definitions (`system.rlb`) and `effects.rlb` are untouched.
Not needed for a static scene; needed for anything animated.

### 2.8 The `NL` archives block the UI

`gamefont.rlb` and `sprites.lib` are not NRes — `'NL'`, version 1, an `0xABBA`
marker, then high-entropy payload. Two files, holding fonts and 2D sprites.
Blocks any in-engine interface work and nothing else.

---

## 3. Renderer engineering, not format work

No reverse engineering needed; just work.

- **Backface culling.** Everything draws `DoubleSide`. Measured: interior faces
  are *not* uniformly back-facing, so culling is not a shortcut for hiding
  them, but front-face culling is still the correct default for closed shells.
- **LOD switching.** All three levels are parsed; the viewer always draws
  LOD 0. Switch by screen size.
- **Alpha ordering.** Needed before 2.4 looks right.
- **Terrain patches.** Face field 13 (0..62 on SC_3) reads as a patch or sector
  id and would give cheap frustum culling, if confirmed.
- **Batch material high byte.** 0xFF on 14166 batches, 0x00 on 972. Not a
  primitive type — both groups have index counts divisible by three. A blend or
  two-sided flag is the obvious guess and it is one afternoon to test.

---

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
