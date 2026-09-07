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

`gamefont.rlb` and `sprites.lib` are not NRes. They begin:

```
4E 4C 00 01   'NL', version 1
<u16> <u16>   two equal values (2,2 and 24,24 respectively)
00 00 00 00
BA AB         0xABBA marker
```

after which the payload is high-entropy — compressed or obfuscated. Only 2
files, holding fonts and 2D sprites, so this blocks UI work but nothing else.

## Object mesh: the .ctl controller

Every `STAT` record has a `.ctl` slot that has not been read. The vertical
datum and the component attachment it was expected to explain both turned out
to live in the mesh itself — see [07-objects.md](07-objects.md) — so what a
controller carries is still open.

## CTPT field roles outside static.rlb

The container is solid everywhere (all 284 members parse, all 3599 points
named), but the nine floats only read cleanly as
`(zero, position, unit direction)` in `static.rlb` and `turrets.rlb`.
`guns.rlb` and `parts.rlb` put scalars like `Width` in a vector slot.

## BASE — building footprints

`fortif.rlb` `.bas` members open with a count and then float triples at z ≈ 0
that trace a polygon, so they read as a building's ground footprint. The
header does not divide evenly into the payload, so the full record is not
mapped.

## Object mesh leftovers

- **The fifth slot of a variant.** Four of every five slot indices are a level
  of detail ladder; the fifth is not (it breaks the monotonic triangle count
  on 292 chains). 316 of the 1845 nodes carry one; of the 288 that carry both
  a level 0 and a fifth slot, the two use the same materials on 197 and
  different ones on 91.
- **What the later slot variants are.** Two of the three blocks of five are
  used by 135 nodes. `fr_b_brige`'s `o02` has identical counts in variants 0
  and 1, which reads like a damage state.
- **Pose key `time`.** The `float32` at offset 12 of a stream-8 key is an
  integer frame number in every one of the 34049 keys, ranging 0..473. It is
  not used by a static renderer.
- The 180-degree disagreement between a socket's rotation and the root
  rotation of the part that mounts on it, on 108 of 1414 attachments.

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

`*.ctl` controllers, `*.exp` explosions, `sky.ske` skyboxes, `lightmap.lib`,
`.scr` node semantics, the `.trf` streams, save games in `SAVE/`, and the
network protocol.

The ones that hold back a picture are triaged in [../TODO.md](../TODO.md).
