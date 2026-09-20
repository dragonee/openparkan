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
0x18       4  uint32 a colour the exporter recorded, read by nothing — a
              palette index on the palettised textures, 0 on 346 of 393
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
The total
`sum(max(w>>i,1) * max(h>>i,1) for i in range(mips)) * bytes_per_pixel`
(plus 1024 for a palette) equals the payload size exactly on 328 of the 393,
and **on the other 65 the leftover is the `Page` chunk** and nothing else, to
the byte. So every declared level is present in all 393 — and in all 518 `Texm`
members of the install, counting `lightmap.lib` and the three `ui/*.lib`
([below](#the-page-chunk--a-textures-own-sub-images)). An earlier reading
called those 65 a truncated tail; they are not truncated at all.

`openparkan.texm.decode` still reads **mip level 0 only**, because that is
what a renderer imports, but the deeper levels are there and are worth reading
for what they say about the tool that wrote them
([below](#the-mip-chain-is-one-tools-box-filter--measured)).

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

**Every texture a material indexes carries a table, and every cell asked for
falls inside its own** — 2513 of 2513 cells over 62 textures, counting every
entry of every material and not just the first, with cell numbers up to 63.
That is what makes the reading safe: a wrong stride would run off the end
almost immediately.

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

`uv run openparkan verify`: the header accounts for every byte of every
payload — 328 of the 393 end on the last mip level and the other 65 carry a
`Page` table after it — all 393 decode to RGBA at the declared dimensions,
2513/2513 material cells fall inside the `Page` table of the texture they name,
all 81 marked textures' mip chains are the box filter
([below](#the-mip-chain-is-one-tools-box-filter--measured)) while 89 unmarked
ones are not, and `+0x18` is non-zero only on members below 60.

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

Neither case is reached **in `Textures.lib`** (*measured*):

- none of its 393 sets either bit, since their `+0x14` is only 0 (312) or
  `0x4000000` (81);
- no palettised texture in it draws index 0 anyway.

**So a palettised texture out of `Textures.lib` draws fully opaque**, and
nothing there is colour-keyed. *Corrected 2026-09-20*: this paragraph used to
say "no texture sets either bit", which is false of the install as a whole —
**all eleven font atlases set `0x1000000`**, the alpha-surface bit, and every
one of them is palettised (format 0). On them the first case above *is*
reached, index 0 is cleared to alpha 0, and that colour key is exactly how a
glyph's background is cut out
([12-rsli.md](12-rsli.md#how-the-text-is-coloured)). The measurement was
right for the archive it was taken over and was stated of all textures.
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

### Which batch the bit marks — *measured*

The run is members **66 to 154**, and it is a coherent thing: **all 89 are
`ARGB8888`, all 89 are mip-mapped** (`+0x10` = 32, chains of 4 to 7 levels),
all 89 have `+0x18` = 0, and all 89 have the same mip generator
([below](#the-mip-chain-is-one-tools-box-filter--measured)). The names are the
skins of machines and buildings — 19 `S*`, 14 `RL_*`, 13 `PG*` and `PG_*`, 7
`GEN_*`, 5 `MTP_*`, 4 `DD*`, 4 `BIRD_*`, 3 `RU*`, 3 `PLT_*`, 2 `MN_*`, 2
`KORA*`, 2 `STONE0*`, then `APKORA`, `SKIN02`, `COMP_2`, `HNG_01`, `RB_04`,
`RBW_3`, `RLW_4`, `P26` and `AIM_02` — which is why 78 of the 81 are worn by a
lit material. The members on either side are a different kind of thing: 60 to 65 are
`BLUEPG0*`, `LAUSE*`, `LAULEG1` and `SUN5`, and 155 onwards `B_FOUND`, the four
`NEBULA_*` and the `SUN*` sprites.

**Nothing but the bit tells the 89 apart.** The eight unmarked inside the run —
`PG05.0`, `STONE00.0`, `STONE01.0`, `MTP_06.0`, `BIRD_B.0`, `BIRD_T.0`,
`BIRD_U.0`, `BIRD_W.0` — are `ARGB8888`, mip-mapped, `+0x18` = 0 and
box-filtered like the other 81; they span the same 64–256 sizes and the same
name families, and `PG03`, `PG04`, `PG06` and `PG07` are marked while `PG05` is
not, `MTP_01`, `MTP_03`, `MTP_04` and `MTP_05` while `MTP_06` is not. One more
axis, the `Page` table, is on 7 of the 81 and 0 of the 8, which is what eight
draws from a 7-in-89 rate would give anyway. So the bit is **bookkeeping inside
one batch**, not a property the batch shares and the eight lack.

### The mip chain is one tool's box filter — *measured*

Every mip level *k* of a shipped texture is the **truncated mean of the
matching 2<sup>*k*</sup> × 2<sup>*k*</sup> block of level 0** — in the
components the file stores, alpha included, `sum >> 2k` with no rounding. Not
the mean of the level above it: that rounds differently, and comparing level 2
against level 1 fails on textures the direct test passes.

- **81 of the 81** marked textures pass, every level, no exceptions.
- The **control** is that the same test *fails*: 89 of the 256 unmarked
  mip-mapped non-palettised textures do not pass it at level 1, and they are a
  block of their own — `L02.0`..`L24.0` (42 `RGB565` members, 177–218) and
  `L00M.0`..`L24M.0` (42 `XRGB8888` members, 219–260), plus `SLD_GL`, `S_05`,
  `SUN3`, `SUN1` and `NE_SHIELD`. The `M` maps are far off (mean error 3.3 to
  18.5 per component), the 565 ones barely (0.09 to 0.50, which is what
  quantising a higher-precision average to 5 and 6 bits costs).
- But it passes on **167** unmarked textures too, so it names the *tool*, not
  the batch. `lightmap.lib` and the three `ui/*.lib` carry no mip chains at all
  — all 125 of their members declare one level.

So `Textures.lib` is at least three blocks laid down in turn, each with its own
signature: members **0–59**, the only ones with `+0x18` ≠ 0 and mostly without
mip chains; members **66–154**, the `0x4000000` batch; and members
**177–260**, the `L*` lightmap-style set whose mips another tool made.

### `+0x18` is a colour, and it belongs to the first block — *measured*

`+0x18` is non-zero on **47 of `Textures.lib`'s 393**, and every one of them is
a member below 60 — so it and the `0x4000000` run never appear on one texture.
It is also non-zero on all 15 of `ui/ui.lib` and on 2 of `ui/ui_back.lib`
(`Crdt_uu`, `Crdt_dd`), and on none of `lightmap.lib` or `ui/minimap.lib`.

What it holds is **a colour**, which the 15 palettised textures settle: there
it is a palette index, and the entry it names is one of the nearest in the
whole 256-entry palette to the image's own mean colour. `WATER0.0`..`WATER9.0`
all say 115, whose colour is (181, 181, 189) against a mean of (181.5, 180.6,
183.2); `TREE01.0` says 57, (76, 100, 40) against (74.9, 93.2, 39.3). By
distance to the mean the named entry ranks **1st on ten of the fifteen and
inside the nearest twenty on all fifteen** — a uniformly chosen index would be
that close 7.8% of the time, so fifteen of them by chance is 10<sup>−17</sup>.

On the 49 `4444` textures that carry it the value is 12 or 13 bits and its top
two nibbles track the mean's red and green, but the low nibble does not track
blue — `LAVA0*.0` say `0x0c3c` against a mean of (12.1, 3.1, 1.1) in nibbles,
and `ui_tex9.tex` says `0x0ccc` against (12, 12, 12). **The packing on the
16-bit formats is not established.** Whatever it is, no module reads `+0x18`
either: the sweep that found the loader's tests of `+0x14` bits 24 and 25 finds
no read of the header's `+0x18`.

### The exporter is not identifiable from the install — *read*, a negative

**The only code in the shipped game that writes a `Texm` header is the
engine's own**, and it is not the exporter. A byte sweep of all 22 `.dll` and
`.exe` files in the install for the literal `Texm` returns exactly one hit,
`Ngi32.dll` file offset `0x7e80`, which is the immediate of
`mov dword ptr [esp], 0x6d786554` at `0x10007e7c`, inside the function that
starts at `0x10007e50`. That function builds a 32-byte header on the stack for
a texture it is about to create from nothing: width and height both from its
argument masked with `0xfe0`, **mip count 1**, `+0x10` = 0, **`+0x14` = 0**,
**`+0x18` = 0**, and a pixel format chosen from its caller's flags — `565` for
bit 27, `4444` for bit 29, `88` for bit 23 and `556` for bit 24, each gated on
a global being non-zero — and hands it to the texture constructor at
`0x10010310` after `0x1002a8b0` allocates `0x88` bytes.

That is the control the negative needs: **the search can find a writer of this
header**, and the one writer it finds sets both exporter fields to zero and one
mip level. So the tool that wrote `0x4000000`, `0x800000` and `+0x18` was never
shipped, and nothing in the install names it: the same sweep over the same 22
binaries finds no `.tga`, `.bmp`, `.pcx`, `.psd`, `3ds`, `3D Studio`,
`Photoshop` or `exporter` anywhere — the only `Convert` strings are the three
modules' scan-code converters and a Windows ACM import. What the shipped data
does identify is a **tool behaviour**:
the box filter above, shared by 248 of `Textures.lib`'s 341 mip-mapped members
and absent from the `L*` block. That is as far as the archaeology goes.

## Not established

- **What `+0x14` bit 26 and bit 23 meant to the tool that set them.** Their
  effect is a settled negative — nothing reads them — and their extent is
  measured, but the 81 and the 8 unmarked members inside the same run are alike
  on every axis the archive carries.
- **How `+0x18` packs a colour on the 16-bit formats.** The palettised reading
  is measured; the `4444` one is not.
- **`Ngi32.dll:0x10007e50`'s pixel formats `88` and `556`.** They are read off
  the immediates at `0x10007ef1` and `0x10007f0c`; no shipped texture declares
  either, so nothing checks the reading.

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
