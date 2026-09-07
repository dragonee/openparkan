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

*(Everything that used to be in this section — node poses, unit scale, the
vertical datum, and component attachment — is done; see
[docs/07-objects.md](docs/07-objects.md). What is left here came out of doing
it.)*

### 1.1 A unit renders in its rest pose, not standing

A walking chassis's legs are animated, and the rest pose is frame 0 of each
node's run in the frame map. That is the right *static* choice — it keeps a
model inside its own authored box on 106 of 157 animated meshes against 81 for
the fallback key — but it is not necessarily the pose the game shows a parked
unit in, and 51 animated meshes still fall outside their box in it.

**Where to look:** the `.ctl` controller, which is the one slot of a `STAT`
record still unread, and stream 19's relationship to `Iron_3D.ini`'s animation
settings.

### 1.2 A socket's rotation is thrown away

A part is mounted at its socket's position with its own orientation. That is
right in the sense that both alternatives are visibly worse (see
docs/07-objects.md), but it means a turret can never be drawn turned, and the
108 attachments whose socket and root rotations disagree by 180 degrees are
telling us something that is not yet understood.

### 1.3 Damage variants are guessed at

Two of every three five-slot blocks are unused by the renderer. 135 nodes
populate them, and `fr_b_brige`'s `o02` carries identical triangle counts in
variants 0 and 1 — a destroyed state is the obvious reading, but nothing
confirms it, and if it is wrong then something is not being drawn.

## 2. Missing fidelity

Correct as far as it goes, but not what the game showed.

### 2.1 Animated and special materials

Eight terrain names never resolve in `Textures.lib` — `WATER`, `WATER_M`,
`WATER_BOT`, `B_S0`, `B_MTP_01`, `ENV_NLAVA`, `ENV_NLAVA_M`, `ENV_LAVA_BOT` —
and are drawn as flat colours. Separately, 85 of 15138 object draw batches
reach a texture name like `0FAIR.0`, `1FAIR.0`, `2FAIR.0`: numbered animation
frames held outside `Textures.lib`. `Material.lib` is the place to look; its
`MAT0` records carry a layer count and `World3D.dll` complains about "Too many
animations for material".

### 2.2 Multi-layer materials

`MAT0` declares a layer count — up to 29, and 377 of 905 materials have more
than one — and the reader takes only the first texture. The
per-layer stride varies with layer type and is not mapped, so the colour bytes
around each name are unread. Detail and bump layers are being dropped —
`Iron_3D.ini` has `EMBM=1`, so the game used environment-mapped bump mapping.

### 2.3 Palettised transparency

Format-`0` textures decode fully opaque. Whether a palette index acts as a
colour key is unknown, and the foliage textures strongly suggest one does —
vegetation is alpha-billboard geometry, so without this trees render as solid
slabs rather than leaves.

### 2.4 Lightmaps

`lightmap.lib` is 2.7 MB of NRes and has never been opened. Static lighting is
presumably in there; everything is currently lit by one directional light.

### 2.5 Skyboxes

`sky.ske` (binary) and `sky.wea` per mission name environment textures —
`ENV_NEBULA_0`, `ENV_STARS`, `ENV_SUN_3`, `ENV_MOON`, `ENV_FLARE_00`. The
`.wea` half is already readable with `mesh.read_wea`; `sky.ske` is not parsed.

### 2.6 Effects

`*.exp` explosion definitions (`system.rlb`) and `effects.rlb` are untouched.
Not needed for a static scene; needed for anything animated.

### 2.7 The `NL` archives block the UI

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
- **Alpha ordering.** Needed before 2.3 looks right.
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
