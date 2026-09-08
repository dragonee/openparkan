# Open questions

Things observed but not resolved. Roughly in the order they block progress.

## data.tma leftovers

The format is parsed end to end (see [04-missions.md](04-missions.md)), but a
few fields are carried through without being understood:

- A property stores three words after its type tag. The first is the value;
  the other two are constant per property name across every mission, so they
  read as bounds or defaults, but nothing confirms which is which.
- The four words after an object's instance name (`0, -1, -1, 1` throughout).
- The word before the object count, always 10.
- The word after a clan's behaviour-tree path.
- The four words trailing each trailer viewpoint.
- Object `scale` is `1,1,1` in every shipped mission, so the axis order is
  unverified.

## Land.map leftovers

The navigation mesh is [solved](08-arealmap.md). Two fields are not:

- An areal edge's second `int32`. It ranges beyond the areal count, so it is
  not a second areal reference.
- The sub-block list (`B` in the record) is zero on every shipped map, so its
  contents are unexercised.

`MHallWay`'s path graph turned out to be mesh stream 17, carried by buildings
— see [07-objects.md](07-objects.md).

## The `NL` archives

`gamefont.rlb` and `sprites.lib` are not NRes. The 32-byte header is settled:

```
0x00  char[2]  'NL'
0x02  uint16   1            version
0x04  uint16   count        2 in gamefont.rlb, 24 in sprites.lib
0x06  uint16   count        the same value again
0x08  6 bytes  zero
0x0E  uint16   0xABBA       marker
0x10  uint32   unpacked     87096 and 1573632
0x14  uint32   packed       25991 and 99633
0x18  uint32   zero, twice
```

`packed` is exactly `file size − 96` on the one and `file size − 776` on the
other, so the stream sits at the end and a block of 64 or 744 bytes stands
between it and the header. That block is high-entropy on both, so it is not a
plain directory. `sprites.lib`'s `unpacked` reads as **24 × (65536 + 32)** —
24 sprites of 256 × 256 with a 32-byte header apiece — which is exactly
1573632.

The payload is **LZSS**, and `gamefont.rlb` all but falls out: a flag byte,
eight items, least significant bit first; a set bit is a literal; a clear bit
is a two-byte match with a 12-bit offset and a length of four bits plus three.
That gives **87057 bytes of a declared 87096** — 0.04% short — and the output
is unmistakably a font: forty zero bytes and then a glyph table stepping by
four, `04 02 04 00`, `08 06 08 00`, `0c 09 0c 00`.

Two things are missing. Where the last 39 bytes of `gamefont.rlb` come from,
and why `sprites.lib` does not decode the same way — from offset 776 it yields
352625 of its declared 1573632, and a search over offsets, over the 12/4,
4/12, 11/5, 10/6 and 8/8 splits, over minimum lengths 1 to 4 and both bit
orders finds nothing exact. No shipped binary contains the `'NL'` magic, the
`0xABBA` marker or either file's name, so the loader has not been found
either.

The probe is `analysis/nl.py`. Nothing is in the library, because a
decompressor that is 39 bytes short is a decompressor that does not work.

## The .ctl controller

[Identified but not parsed](07-objects.md): a *movement* controller —
`Control.dll`'s `LoadControlSystem`, driven through an `IControl` of speeds,
accelerations and angle limits — with a fixed parameter header and a body of
156-byte records. It does not affect a static picture, which is why it is not
read. The vertical datum, the component attachment and the rest pose that it
was in turn expected to explain all turned out to live in the mesh.

Its sibling `.ndp` is [solved and read](07-objects.md). One field of it is not:
the second `float32` of a record, 1000 on 549 of them and then 0, 10, 1, 300
and 500.

## CTPT field roles outside static.rlb

The container is solid everywhere (all 284 members parse, all 3599 points
named), but the nine floats only read cleanly as
`(zero, position, unit direction)` in `static.rlb` and `turrets.rlb`.
`guns.rlb` and `parts.rlb` put scalars like `Width` in a vector slot.

## BASE — the two int32 per corner

`.bas` is [solved](07-objects.md): two closed rings, a building's outline and
a clearance around it, all 30 records to the byte. Each corner carries two
`int32` that are not read — `fr_e_ruin`'s first ring gives 308, 306, 309, 304
and then 2, 1, 1, 1, which reads as two parallel arrays rather than pairs.

## Object mesh leftovers

- **The fifth slot of a variant.** Four of every five slot indices are a level
  of detail ladder; the fifth is not (it breaks the monotonic triangle count
  on 292 chains). 316 of the 1845 nodes carry one; of the 288 that carry both
  a level 0 and a fifth slot, the two use the same materials on 197 and
  different ones on 91.
- **Pose key `time`.** The `float32` at offset 12 of a stream-8 key is an
  integer frame number in every one of the 34049 keys, ranging 0..473. It is
  not used by a static renderer.
- 25 of the 1414 attachments have a socket that turns the part by 120° about
  (1, 1, 1) — a cyclic axis permutation — or by 126°. They are missile packs
  and shell clips on the winged SSM launchers. The other 83 disagreements are
  [solved](07-objects.md): a turret hung under a flying chassis.

## Unresolved terrain fields

- `Land.msh` stream 1: 2432 bytes on SC_3, mostly `0xFF`. Not indexed by
  vertex or face count.
- `Land.msh` stream 2: 737 float3 on SC_3. The first 8 are the map's bounding
  box corners; the remaining 729 (= 27³) look like a spatial subdivision.
- Face record fields 10, 11, 12, and field 0 (near-constant per mesh).
- `MAT0`'s byte 4: twelve values that sort materials into groups tracking
  their names (all six `TREE*` share 6, the effects share 0xFF), so it reads
  as a shader or blend-mode id. Also the record's variable-length tail, and
  the second colour triple after the diffuse.
- A batch's vertex range (fields 7 and 8). The ranges are contiguous but only
  tile the vertex array on 69 of 435 meshes, so they are not a partition.
- Terrain face field 13 (0..62, ~57 distinct per map). **Not** a patch or
  sector id, which earlier drafts guessed: grouping faces by it gives regions
  that span the whole map, no tighter than a random subset of the same size,
  on every map tried. It is interleaved in face order, does not determine the
  texture pair or the surface word, and does not track elevation. Its groups
  are wildly uneven — 1, 2, 4 and 384 faces on SC_3.
- Stream 11's flags word: 72 on 4228 faces, 88 on 329, then 328, 968, 984, 344.
  Bit flags of some kind; 88 correlates with water.
- Face field 0's other values (1536, 1540, 9728 on SC_3) and field 1's bit
  `0x10`. Only the water bit in each is understood.

## Textures

- Palettised textures (format `0`) are decoded fully opaque. Whether a palette
  index is treated as a colour key for transparency is unknown; foliage
  textures suggest one probably is.
- 65 of 393 textures have a mip tail shorter than the declared level count
  implies. Harmless for level 0, but a packer would need to reproduce it.

## Dangling texture references

Eight of the 905 materials name a texture that was never shipped: the five
`FIRE_SMOKE*` animations ask for `0FAIR.0` upward when only `FAIR.0` exists,
and `B_MTP_04`, `B_MTP_04G` and `B_MTP_05` ask for `qqds.7` and `ds.7` when
every member of `Textures.lib` ends in `.0`. Cut content, or frames the
engine generated at run time.

## Not looked at at all

`.scr` node semantics, the `.trf` streams, save games in `SAVE/`, and the
network protocol. `*.ctl` and `*.ndp` are identified above; `*.exp` and
`effects.rlb` are [read](11-effects.md), though what the floats inside an
emitter mean is not.

`sky.ske` is [solved](10-sky.md) -- a day cycle of colour keyframes, all 29
files to the byte. Three things in it are not: the keyframe count of a second
section, which field selects the object type (SUN, SKY, RAIN, SNOW,
LIGHTNING), and most of the 124-byte file header.

The ones that hold back a picture are triaged in [../TODO.md](../TODO.md).
