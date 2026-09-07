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

### 2.1 The sky

The biggest thing still missing from every picture. `Terrain.dll` calls
`sky.ske` an **atmosphere** file, written by a tool it names SunEditor, and
the class names say what is in it: `CAtmosphere`, `CAtmData`, `CSun`,
`CreateAtmosphereObject`, "Illegal atmosphere object type", plus the settings
`AtmSkyDetail`, `AtmStarsOn`, `AtmCloudsOn`, `LensFlareOn`. So it is a list of
typed sky objects and timed events (`GetTimeDiffInSec`, `GetEvents`; the first
words of a file read as `23, 59` where a time would go). 999 to 8573 bytes,
different per mission.

The sibling `sky.wea` is already readable and names the textures:
`ENV_NEBULA_0`, `ENV_STARS`, `ENV_CLOUDS`, `ENV_SUN_3`, `ENV_MOON`,
`ENV_FLARE_00`, `ENV_FLARE_01`, `SNOWFLAKE`, `RAIN_DROP`. What is missing is
the placement, and guessing it would be inventing.

**Where to look:** `Terrain.dll`'s `CreateAtmosphereObject`, which has to
switch on the object type word.

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

- **LOD switching.** All four levels of each variant are parsed and
  `slots_for_lod` takes the level; the viewer always asks for 0. Switching by
  screen size is a payload change, not a format question.
- **Terrain culling.** There is no patch id to cull by: face field 13 turned
  out not to be spatial (see docs/06-open-questions.md), so a renderer has to
  build its own grid, which is what `LandMesh._build_index` already does for
  height queries.
- **Alpha ordering.** Cutouts need none, which is why they are what the
  viewer uses, but the graded textures behind effects and the sky will.

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
