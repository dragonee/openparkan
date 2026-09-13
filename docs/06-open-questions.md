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
- The word after a clan's behaviour-tree path: 2..17, 5 on 53 of 101 clans,
  equal between the matched opponents of `Multi.01`..`04`, climbing for the
  campaign's main enemy, and tracking no count in the record or its tree. A
  strength or AI level is the guess; see [04-missions.md](04-missions.md).
- The four words trailing each trailer viewpoint.
- Object `scale` is `1,1,1` in every shipped mission, so the axis order is
  unverified.

## Land.map leftovers

The navigation mesh is [solved](08-arealmap.md). Two fields are not:

- ~~An areal edge's second `int32`~~ — **answered: it is the twin edge**, the
  index of the same edge in the neighbouring areal, right on 193418 of 193418
  shared edges and `-1` on every boundary edge. See
  [08-arealmap.md](08-arealmap.md).
- The sub-block list (`B` in the record) is zero on every shipped map, so its
  contents are unexercised.

`MHallWay`'s path graph turned out to be mesh stream 17, carried by buildings
— see [07-objects.md](07-objects.md).

## The .ctl controller

[Read as far as its frame](13-control.md). The 212-byte parameter block at the
head of all 531 members is parsed and verified, the `IControl` interface is
recovered by name from `Terrain.dll`'s stub table, and the 100-byte reference
record — which is what a controller *emits*, an effect or a projectile — is
found and resolves on all 1651. The vertical datum, the component attachment
and the rest pose that this file was once expected to explain all turned out
to live in the mesh.

The **sections** are read too, and all 531 members now walk end to end: section
1 (`counts[0]` records of `156 + 16*counts[1]`, then `counts[0]**2` int32),
section 2 (`counts[2]` records of 36 bytes), section 4 (`counts[3]` component
records, one shape for all 30 type ids), an 84-byte block and section 5
(`counts[4]` groups of an int32 and that many 100-byte records). A component's
label names a family of internal parts -- all 57 of them prefix an `INTO`
record in `objects.rlb`.

What is open is the **meaning** of the fields rather than their extent: the
component record's 64-byte block at +0x2c, section 1's and section 2's record
contents, the 84-byte block, and the nine ints of a section-5 record. And
**which field feeds which setter** is unmapped: the stub table names the
interface but never receives an argument, so the mapping has to come from
`AniMesh.dll`, the only module that calls `LoadControlSystem`.

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

- `MAT0`'s byte 4 is **answered, and the answer is that nothing reads it**.
  Eleven values plus an unset `0xFF` sort materials into groups tracking their
  names (all six `TREE*` share 6), so it reads as a shader id — but the loader
  copies it to the runtime material's `+0x154` and the only code that names
  that field's address is an accessor, slot 9 of the material manager's
  vtable, which no module calls. The directory's flags byte beside it is the
  field a renderer wants, and the one distinction the class byte draws that a
  renderer would act on — the ground's second track — is in the track count
  already. See [07-objects.md](07-objects.md#the-class-byte-is-loaded-and-never-read)
  and [../analysis/README.md](../analysis/README.md).
- Terrain face field 13 is **answered: it is the winged-edge link**. Three
  2-bit codes, one per edge, each naming the matching edge back in the
  neighbouring face, with 3 for no neighbour — right on **817150 of 817150**
  shared edges. Reading the packed byte as one number is what made it look
  like a meaningless 0..62 with no spatial structure; 63 never appears because
  that would be a face with all three edges free.
  See [03-terrain.md](03-terrain.md).
- The surface word's bit `0x10` is **answered: it is clear on lava**. On the
  29 of 33 maps that set it anywhere, the faces with it clear are exactly the
  faces whose layer-1 material names lava — 6711 across the library, surfaces
  and beds alike. The four that never set it have no lava. The old reading,
  "about 95% set, in connected regions, tracking nothing", came of pooling the
  maps: those four files contribute 23439 clear faces with nothing under them.
  See [03-terrain.md](03-terrain.md).
- The draw order's flags bit `0x80`. The byte takes **exactly four values**
  across the library — `0x48`, `0x58`, `0xc8`, `0xd8` — so `0x08` and `0x40`
  are set on every one of the 275882 entries and say nothing, `0x10` opens a
  batch, and `0x80` is left. It is on **89 entries of two maps only**, `ILKON`
  (76) and `SC_3` (13), in short runs mostly of length 2. The faces it marks
  are ordinary: median area, free-edge count and level of detail all match the
  rest of their map, and they are not degenerate or duplicated. The rest of
  the face record is read: flags `0x004` is a second texture layer, `0x008`
  water and `0x2000` a liquid bed, and fields 10..12 are the face's own
  normal.

## Textures

- Palettised textures (format `0`) are decoded fully opaque. Whether a palette
  index is treated as a colour key for transparency is unknown; foliage
  textures suggest one probably is.

## Not looked at at all

`.scr` **node semantics** -- the structure is now [read](15-behaviour.md), all 58
files end to end, but what a node does is not -- and the network protocol.
The text layer around a mission is [read](21-briefing.md): the opening
flythrough, its subtitles and voices, and the in-mission messages, through the
[resource descriptors](20-resources.md) that bind them. Save
games in `SAVE/` are [partly read](17-saves.md): the header and what a save
refers to, but not the object graph, which is a raw heap dump. The `.trf` archives are [read](16-research.md): 368 research
items and their prerequisite graph, though four floats and three of the twelve
streams inside them are not. The engine's own `.ini` files are now
[read](22-settings.md) — the component registry, the two debug files, the
display settings and the mission-progress dispatcher, each matched to the
module that reads it; the input tables beside them are [read](14-controls.md). `*.ctl` and `*.ndp` are identified above; `*.exp` and
`effects.rlb` are [read](11-effects.md), though what the floats inside an
emitter mean is not.

`sky.ske` is [solved](10-sky.md) -- a day cycle of colour keyframes, all 29
files to the byte. Three things in it are not: the keyframe count of a second
section, which field selects the object type (SUN, SKY, RAIN, SNOW,
LIGHTNING), and most of the 124-byte file header.

The ones that hold back a picture are triaged in [../TODO.md](../TODO.md).
