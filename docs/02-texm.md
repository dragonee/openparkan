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
0x14       4  uint32 flags — 0 on 312, 0x4000000 on 81 (all of them 8888)
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
even an alpha format to an opaque surface. Which callers pass it is not
traced.

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
state site. What bit `0x4000000` does — set on 81 `8888` building textures
(`PG*`, `GEN_*`) — is *unknown*; the loader does not test it.
