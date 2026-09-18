# Texm — the texture format

`Textures.lib` (57 MB, 393 textures) plus the smaller `ui/*.lib` archives store
textures as `Texm` blobs inside NRes members.

## Header (32 bytes)

```
offset  size  field
0x00       4  magic 'Texm'
0x04       4  uint32 width
0x08       4  uint32 height
0x0C       4  uint32 mip level count
0x10       4  uint32 flags — 32 on mip-mapped textures, 0 otherwise
0x14       4  uint32 the exporter's flags, of which the loader reads bits 24
              and 25 — 0 on 312 of Textures.lib's 393 and 0x4000000 on 81
0x18       4  uint32, varies; not needed to decode
0x1C       4  uint32 pixel format
0x20     ...  pixel data, mip 0 first, each level half the previous
```

## The pixel format field is self-documenting

It spells the channel widths as a decimal literal. This is the single most
useful thing about the format — no guessing was required:

| Value | Meaning | Bytes/px | Count in `Textures.lib` |
|---|---|---|---|
| `8888` | ARGB, little-endian, so **B G R A** in memory order | 4 | 237 |
| `888` | XRGB — same layout, 4th byte always 0 | 4 | 52 |
| `565` | RGB565, red in the high bits | 2 | 47 |
| `4444` | ARGB4444 | 2 | 42 |
| `0` | 8-bit palettised | 1 | 15 |

Palettised textures place a 256 × BGRX palette (1024 bytes) immediately after
the header, *before* the index data.

**Channel order was confirmed by eye**, not assumed: decoding a contact sheet
both ways gives warm orange explosion sprites and green foliage one way, and
cyan explosions with blue foliage the other. B-G-R-A in memory order is the
DirectDraw convention the engine was written against, and it is the one that
produces sane colours.

## Mip chains

The declared level count runs the usual halving pyramid, clamping at 1 pixel.
For 328 of 393 textures the total
`sum(max(w>>i,1) * max(h>>i,1) for i in range(mips)) * bytes_per_pixel`
(plus 1024 for a palette) equals the payload size exactly. The remaining 65
have a slightly short tail — the smallest levels are truncated or padded
differently.

Because of that, `openparkan.texm.decode` reads **mip level 0 only**. Level 0
sits at a known offset regardless of how the tail is stored, so decoding is
robust for all 393 textures.

## The `Page` chunk — a texture's own sub-images

Some of the 65 textures whose payload does not match the pyramid are not
truncated at all: they carry an extra chunk after it.

```
char[4]  'Page'
uint32   count
count x  uint16 x, uint16 width, uint16 y, uint16 height
```

Note the field order — **x, width, y, height**, not the x/y/w/h you would
guess. It is the texture's list of sub-images, and it is what a material's
cell byte indexes (see [07-objects.md](07-objects.md) for the material chain).

`SUN.0` declares four:

| cell | x | width | y | height | what it is |
|---:|---:|---:|---:|---:|---|
| 0 | 0 | 128 | 0 | 128 | the sun's corona |
| 1 | 128 | 128 | 0 | 128 | a soft glow |
| 2 | 0 | 128 | 128 | 128 | the moon |
| 3 | 128 | 128 | 128 | 128 | another glow |

which is why `ENV_SUN` asks for cell 0 and `ENV_MOON` for cell 2. That sheet
happens to be a 2 x 2 grid, and an earlier reading took the cell for a grid
index on the strength of it. **It is not a grid.** `EFFECT6.0`'s 26 pages are
four 128 x 32 strips, eight 64 x 64 tiles, eight 30 x 30 discs and five
16 x 16 icons, in one table; `RAIN_DROP` is cell 21, a 16 x 16 icon at
(16, 224), and no square grid puts anything but a fragment there.

**All 61 textures a material indexes carry a table, and all 478 cells asked
for fall inside their own** — which is what makes the reading safe, since a
wrong stride would run off the end almost immediately.

### How a cell is drawn — *read*

A cell is a rewrite of the texture coordinates, not a texture matrix.
`Ngi32.dll` turns the table into rectangles when it uploads the texture
(`0x1000ff60`): record 0 is the whole texture, `u0 = 0, du = 1, v0 = 0,
dv = 1`, and record *i* + 1 is page *i* as `u0 = x / W, du = width / W,
v0 = y / H, dv = height / H`. The texture's slot 7 (`0x100101d0`) selects one
and slot 13 (`0x100101c0`) returns it; the draw then rewrites every vertex,
`u' = u0 + u × du` and `v' = v0 + v × dv` (`0x100076d0`). With the object
mesh's `uint16` coordinates the engine folds the 1/1024 into the scale,
`u' = u0 + raw × du / 1024` (`Terrain.dll:0x10038a01`). Wrapping still
applies, so coordinates beyond 0..1 run out of the cell into its neighbours.
See [07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured).

## Usage

```
uv run openparkan textures Textures.lib --out /tmp/tex
uv run openparkan textures ui/minimap.lib --out /tmp/minimaps
```

## Verified by

`uv run openparkan verify`: the declared format predicts the payload size for
328/393 textures, all 393 decode to RGBA at the declared dimensions, and
478/478 material cells fall inside the `Page` table of the texture they name.

## Transparency

**241 of the 393 shipped textures carry alpha**, and 237 of those are graded
rather than a hard cut — antialiased edges, not a one-bit mask. They are all
`4444` or `8888`; the other three formats have no alpha channel at all.

The palettised format has none either, which is worth stating because a colour
key would be the obvious 1998 answer. Its palette is BGR**X**, and the fourth
byte is **constant across all 256 entries on every one of the 15 palettised
textures** — so it is padding. Palette index 0 is unused in the image data of
all fifteen, which rules out the other common convention too. Of those
fifteen, four are trees and ten are `WATER0`..`WATER9`, an animation.

### Most of that alpha is not transparency

Carrying alpha and *being* transparent are different things, and conflating
them punches holes through solid machinery. Look at the channel:

- `FTREE1.0` is **38% at exactly 0, 57% at exactly 255, and 4.8% in between** —
  a white blob on black. That is a cut silhouette, the crown of a tree.
- `MTP_01.0` has **75% of its pixels strictly between** the extremes, and
  `S0A1.0` has **100%** — not one pixel at 0 or 255. Rendered as an image the
  channel is a continuous greyscale picture of the surface's own detail. That
  is a gloss or self-illumination map, and there is no silhouette in it.

The second kind is much the commoner: of the 241 textures with alpha, **only
23 are silhouettes**. `openparkan.texm.is_cutout` separates them by asking for
a real fully-transparent region (over 5% at 0) and a thin transition (under
25% in between), which puts all three tree textures on one side and every
building and machine texture on the other.

It matters a great deal on screen. Treating any alpha as a cutout and
alpha-testing it discards **30% of all object texture area** — 61% of
`MTP_01.0`, 59% of `NP05.0`, 50% of `GEN_05.0` — and buildings come out as
skeletons you can see through. Nor can the channel simply be composited away:
flattening a gloss map over a background washes the colour out, so a
non-silhouette texture takes its colour channels and ignores alpha entirely
(`texm.drop_alpha`).

So foliage transparency is ordinary 8888 alpha, and a renderer wants an alpha
**test** rather than blending for it: a tree is a pair of crossed planes, and
a cutout needs no depth sorting. Without it a tree draws as a solid slab —
and with it applied to everything, a power plant draws as a wireframe.

### What the loader does with alpha — *read*, and *measured*

`Ngi32.dll` picks each texture's surface when it uploads it (`0x1000fb30`). It
copies the 32-byte header whole, so the word it tests is header `+0x14`
(`0x1000fc39`), and **only four things get an alpha surface**
(`0x1000fdf6`):

- format `4444`;
- format `8888`;
- header `+0x14` bit `0x1000000`;
- header `+0x14` bit `0x2000000`.

Everything else — `565`, `888` and the palettised textures — goes to an
opaque surface. There is one override: a caller's load flag `0x80000` sends
even an alpha format to an opaque surface (`0x1000fe18`) — see [below](#who-loads-a-texture-opaque--read-and-measured).

**A palette gets alpha only on an alpha surface** (`0x1000f620`):

- On an alpha surface, **index 0 is cleared to alpha 0** and indices 1–255
  are opaque. That is a colour key on index 0.
- With header bit `0x2000000` the palette becomes a 32-step fade instead. It
  takes the colour of the index named in the flags' low byte, at alpha
  7, 15, … 255.

Neither case is reached on shipped data (*measured*):

- no texture sets either bit, since `+0x14` is only 0 or `0x4000000`;
- no palettised texture draws index 0 anyway.

**So a palettised texture draws fully opaque**, and nothing is colour-keyed.
No module ever sets `D3DRENDERSTATE_COLORKEYENABLE` either: a sweep of
`Terrain.dll`, `World3D.dll`, `AniMesh.dll`, `Effect.dll`, `Ngi32.dll` and
`iron3d.dll` finds no push of state 41. The same sweep does find every fog
state site.

### Who loads a texture opaque — *read*, and *measured*

A texture is created through the 3D render interface's slot 14
(`Ngi32.dll:0x10007e10`), which keeps the caller's **load flags** ORed with the
resource index in the texture object's `+0x14` (`0x1000f3d0`); the upload masks
out the flags with `0x7ff80000`. It is a different word from the header's
`+0x14`.

**`World3D.dll`'s material loader is the caller that sets `0x80000`**
(`0x10004310`). It branches on the material's archive directory flags
([07-objects.md](07-objects.md#how-a-material-draws-is-in-the-archive-directory)):

- **bit 1 — the lit skins, flags 2 — gets `0x80000`** (`0x10004441`) unless
  both of two things hold: the render setting 109 is on (`0x1000aa04`, global
  `0x100234e8`) and the device reports capability 6 (render slot 31,
  `0x100140ad`);
- bit 0 (`ENV_STARS` alone) gets `0x200000`, which **names the texture stage**
  ([below](#load-flag-0x200000-is-the-texture-stage--read-and-measured)).

It then asks for the texture by name with those flags, the texture-format
setting and `0x400000` (`0x10004b10`, the call at `0x10004c88`).

**Setting 109 is `EMBOSS_BUMP`.** `iron3d.dll` reads `Iron_3D.ini`'s
`EMBOSS_BUMP` and hands it to the settings page `World3D.dll` registers as
`0xa` (`iron3d.dll:0x100613fd`, `0x1006176f`; `World3D.dll:0x10014cca`). This
install's `Iron_3D.ini` has `EMBOSS_BUMP=0`, so **every lit skin uploads
opaque**, whatever its format: the continuous greyscale in their alpha
channels, [above](#most-of-that-alpha-is-not-transparency), is not there to
draw with unless emboss bump mapping is on. That it is the emboss bump map
(`CShade::EmbossBumpMap`, `Terrain.dll:0x1002ce40`) is a *guess* from the
setting's name; that shade function is not read.

*Measured* over `Material.lib` and `Textures.lib`:

- **171 of the 279** `4444` and `8888` textures are named by a lit skin, and so
  load opaque;
- of the **23** cut-outs, three — `AIM_02`, `PI_CSPG3`, `S7` — are lit skins'
  and draw solid; the trees (`FTREE1`, `HTREE1`, `NTREE1`) are flags 4 and keep
  their alpha;
- **78 of the 81** textures with header bit `0x4000000` are lit skins'; the
  other three (`S14N1..3`) are named by no material.

**Header bit `0x4000000` is read by nothing.** A sweep of `Ngi32.dll`,
`Terrain.dll`, `World3D.dll`, `AniMesh.dll`, `Effect.dll` and `iron3d.dll` for
a test or mask of `0x4000000`, for a shift by 26 and for a byte test of the
header copy finds no read of the header word. The control: the same sweep finds
the loader's own tests of the header's `0x1000000` and `0x2000000`
(`0x1000fe39`, `0x1000fe23`, and the palette's `0x1000f64d`). Its hits for
`0x4000000` are the load flag passed to `rsLoadFast` (`0x1000fbcb`), a device
capability word (`0x1000649f`), a light's flags (`Terrain.dll:0x10047a74`) and
the C runtime's file-mode parsing. Nor does it separate the lit skins: the 81
and the 92 other lit-skin 8888 textures are alike in format, mip flags and
alpha statistics. What does separate them is not in the picture at all, and is
[below](#0x14-is-the-exporters-flags-word--measured).

### `+0x14` is the exporter's flags word — *measured*

The bit is not a property of the picture, which is why nothing in the picture
sorts the 81 that carry it. They are all `8888`, mip-mapped (`+0x10` = 32) and
have `+0x18` = 0 — but so are 156, 280 and 265 of the other 312, and the three
conditions together still admit **156** unmarked textures. Alpha does not sort
them either: 79 of the 81 are more than a tenth graded, and so are 119 of the
156 unmarked `8888`s. Nor does the wearer — the 81 are worn by nine archives in
26 combinations, one of which is none at all: 14 of them are worn by nothing.

**What sorts them is position in the file.** `Textures.lib`'s directory is in
offset order (verified, not assumed), and the 81 are members **66 to 154 of
393**: one run of 89, with no marked member outside it and only eight unmarked
inside it — `PG05.0`, `STONE00.0`, `STONE01.0`, `MTP_06.0`, `BIRD_B.0`,
`BIRD_T.0`, `BIRD_U.0` and `BIRD_W.0`. The names in the run are not
alphabetical, so the order is the order they were added in. The bit marks **one
batch of exports**, not a kind of texture.

`lightmap.lib` says the same from the other side: a **different** `+0x14` bit,
`0x800000`, on exactly its three `_01` lightmaps — `fr_l_bunker_01.0`,
`fr_m_bunker_01.0`, `fr_b_bunker_01.0` — and nowhere else, while
`ui/minimap.lib` (53), `ui/ui.lib` (15) and `ui/ui_back.lib` (32) are 0
throughout. (The control on the sweep is that per-archive pass itself: reporting
0 on the three `ui/*.lib`, it reports 81 in `Textures.lib` and 3 in
`lightmap.lib`.)

So `+0x14` is **an exporter flags word of which the loader reads only bits 24
and 25** — the alpha-surface tests above. Whatever the tool meant by bits 23 and
26, the engine never asks.

### Load flag `0x200000` is the texture stage — *read*, and *measured*

`Ngi32.dll:0x100101f0` fills a `DDSURFACEDESC2` for the surface its caller
creates through `IDirectDraw7::CreateSurface` (`0x1000fe93`). It zeroes `0x7c`
bytes, writes `dwSize` = `0x7c` and `dwFlags` = `0x121007`, the height, the
width and the mip count, a 32-byte pixel format copied out of the table at
`0x1003449c` with stride `0x44`, and `ddsCaps.dwCaps` = `0x401008` with its
`dwCaps2`. In the middle of that (`0x10010249`–`0x10010284`) it reads the
texture object's `+0x14` — the caller's load flags ORed with the resource index
— does `shr 0x15` and `and 1`, so **bit 21**, and writes the result into both
the texture object's `+0x18` and `[desc + 0x78]`. Offset `0x78` of a
`DDSURFACEDESC2` is **`dwTextureStage`**.

The control on the reading is why the flag was missed before: a plain immediate
scan over the texture code finds every other load flag — `0x80000` at
`0x1000fe18`, `0x400000` at `0x1000fb78`, `0x8000000` at `0x1000fba2` and
`0x1000fbc3`, `0x4000000` at `0x1000fbcb` — and no `0x200000`, because the
compiler isolated bit 21 with a shift instead of a mask. Sweeping every module
for a shift or bit test by `0x15` returns seven sites, six of them the same
statically linked CRT routine in six modules; the seventh is `0x10010269`.

*Measured*: exactly one of `Material.lib`'s 905 materials carries archive
directory bit 0 — `ENV_STARS`, flags 5
([07-objects.md](07-objects.md#how-a-material-draws-is-in-the-archive-directory))
— and it is that bit `World3D.dll:0x100043e3`–`0x100043f1` turns into
`0x200000`. `ENV_STARS` names one texture, `STAR0.0` (256 × 256, `8888`, 7 mips,
header `+0x14` = 0), and no other material names it. **So exactly one surface in
the shipped game is created on texture stage 1.**
