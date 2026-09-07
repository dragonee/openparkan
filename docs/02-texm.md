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
0x14       4  uint32, zero in every shipped texture
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

## Usage

```
uv run openparkan textures Textures.lib --out /tmp/tex
uv run openparkan textures ui/minimap.lib --out /tmp/minimaps
```

## Verified by

`uv run openparkan verify`, checks 5–6: the declared format predicts the payload size
for 328/393 textures, and all 393 decode to RGBA at the declared dimensions.

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

So foliage transparency is ordinary 8888 alpha, and a renderer wants an alpha
**test** rather than blending for it: a tree is a pair of crossed planes, and
a cutout needs no depth sorting. Without it a tree draws as a solid slab.
