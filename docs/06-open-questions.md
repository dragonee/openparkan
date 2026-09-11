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

- `MAT0`'s byte 4: eleven values plus an unset `0xFF` that sort materials into
  groups tracking their names (all six `TREE*` share 6), so it reads as a
  shader or blend-mode id, but nothing in the engine follows it. The
  directory's flags byte beside it is the sharper field; see
  [../TODO.md](../TODO.md) §2.4. The record's tail is the animation-track
  table and is now read, and the "second colour triple" is the entry's
  ambient — see [07-objects.md](07-objects.md#the-material-chain).
- Terrain face field 13 (0..62, ~57 distinct per map). **Not** a patch or
  sector id, which earlier drafts guessed: grouping faces by it gives regions
  that span the whole map, no tighter than a random subset of the same size,
  on every map tried. It is interleaved in face order, does not determine the
  texture pair or the surface word, and does not track elevation. Its groups
  are wildly uneven — 1, 2, 4 and 384 faces on SC_3.
- The surface word's bit `0x10`: set on about 95% of faces and clear on the
  rest in connected regions, tracking none of slope, elevation, material,
  level of detail, map edge, duplication between levels or coverage by the
  navigation mesh.
- The draw order's flags bit `0x80`, on 89 faces across the 33 maps. The rest
  of the face record is read: flags `0x004` is a second texture layer, `0x008`
  water and `0x2000` a liquid bed; fields 10..12 are the face's own normal;
  and the draw order's `0x10` opens a batch.

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
