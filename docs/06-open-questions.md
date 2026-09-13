# Open questions

Things observed but not resolved. Roughly in the order they block progress.

## data.tma leftovers

The format is parsed end to end (see [04-missions.md](04-missions.md)), but a
few fields are carried through without being understood:

- ~~A property's two words after its value~~ — **answered: minimum and
  maximum.** Every instance keeps its value between them; `-1` on an int is
  no maximum, equal bounds lock the value, and `CurrentOre`'s maximum is its
  object's `MaximumOre`. See [04-missions.md](04-missions.md).
- ~~The four words after an object's instance name (`0, -1, -1, 1`
  throughout)~~ — **answered**, and they were not constant: a building's start
  flag (it marks one half of each bridge pair), the logical id of the building
  a unit starts inside and the hall-way vertex it stands at (13 units), and
  the property table's own leading word. `MisLoad.dll` names them all; see
  [04-missions.md](04-missions.md#what-the-objects-words-do--read-and-measured).
- ~~The word before the object count, always 10~~ — **answered: the object
  record's version**; the scale is read from 10 on
  ([04-missions.md](04-missions.md#the-scale)), and the "always 6" before the
  clans is the clan table's. The word after an object's path, "varies", is its
  clan index.
- ~~The word after a clan's behaviour-tree path.~~ **Answered**: the clan's
  mind count, how many bots it may field; see [04-missions.md](04-missions.md).
- ~~The four words trailing each trailer viewpoint~~ — **answered: the
  trailer's records are mineral lodes**, not viewpoints; the words are a found
  flag, the object type `0x10001000`, an amount and a word nothing reads.
  What reads the amount off `SetMineralLode`'s record (`+0x14`) is open.
- ~~Object `scale`'s axis order~~ — **answered**: x, y and z scale the
  model's own axes and the radius takes the largest; only vegetation and rock
  are scaled, and the engine applies it to them (218 placements carry a value
  other than 1, 0.2 to 21; see [04-missions.md](04-missions.md#the-scale)).
  Uniform on all 864, so the data still does not exercise the axis order.
- What reads a building's start flag back (`IBuilding` slot 13, `CBuilding
  +0xb8`), and the word after the map path, which `IMission` slot 11 returns to
  no caller found.

## Land.map leftovers

The navigation mesh is [solved](08-arealmap.md). Two fields are not:

- ~~An areal edge's second `int32`~~ — **answered: it is the twin edge**, the
  index of the same edge in the neighbouring areal, right on 193418 of 193418
  shared edges and `-1` on every boundary edge. See
  [08-arealmap.md](08-arealmap.md).
- ~~The sub-block list (`B` in the record)~~ — **answered: carried and never
  used.** Its layout is three extra edge pairs and a point list per sub-block,
  and the loader (`ArealMap.dll:0x10007640`) steps over both and keeps no
  field for B. None ships. See [08-arealmap.md](08-arealmap.md).

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
1 (`counts[0]` animation states of `156 + 16*counts[1]`, then a `counts[0]**2`
table of floats),
section 2 (`counts[2]` records of 36 bytes), section 4 (`counts[3]` component
records, one shape for all 30 type ids), an 84-byte block and section 5
(`counts[4]` groups of an int32 and that many 100-byte records). A component's
label names a family of internal parts -- all 57 of them prefix an `INTO`
record in `objects.rlb`.

What is open is the **meaning** of the fields rather than their extent: the
sixteen values of most component classes
([23-economy.md](23-economy.md)), section 1's conditions, triples 2 and 6, and
the nine ints of a section-5 record. The motion fields — acceleration, top
speed, turn rate, slope mode and cone, payload, a state's velocity and spin
boxes and engine factor — are [read](24-motion.md).

Section 1's frame pairs, step lengths, flags and transition table, and section
2's channels, are [read too](24-motion.md#playing-a-state--read-and-measured).

Its sibling `.ndp` is [solved and read](07-objects.md), the second `float32`
included: ~~what it is~~ — answered, it times the node's volume is the node's
mass ([24-motion.md](24-motion.md#load--read-and-measured)), a density. It
*falls* as a node grows, which once ruled out a mass; a density is what fits.

## CTPT field roles outside static.rlb

~~Answered.~~ The nine floats looked like `(zero, position, unit direction)` in
`static.rlb` and `turrets.rlb` but put scalars like `Width` in a vector slot in
`guns.rlb` and `parts.rlb`. They do not: every one of the 191 points named
`Width`, `Height` or `Size` has exactly one non-zero component — an axis times
the size. See [07-objects.md](07-objects.md#ctpt--control-points).

## BASE — the two int32 per corner

~~Answered.~~ They are two parallel arrays, as suspected, and they are a
back-reference: **which triangle of the building's own mesh each corner was
traced off, and which corner of it**. Only the inner ring carries them; the
outer ring is a clearance drawn around the building. See
[07-objects.md](07-objects.md).

## Object mesh leftovers

- ~~**The fifth slot of a variant**~~ — **answered: it is what the unit's own
  first-person view draws.** The turret's camera component registers its view
  with the unit's mesh, and the mesh draws slot `variant × 5 + 4` of every node
  to that view. The 28 "collision hulls" carrying only a fifth slot are the
  cockpit the view sits in — `CameraCenter` is on one on all 54 turret records
  that have one. A round's hit test takes level 0 and never strikes them. See
  [07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws).
- ~~**Pose key `time`**~~ — answered: a key's time is **the frame at which
  its run first names it**, which is what an animation player interpolates
  between. See [07-objects.md](07-objects.md).
- ~~25 of the 1414 attachments have a socket that turns the part by 120°~~ —
  **answered: the root's rotation never reaches the picture.** Mounting drops a
  part's root node and hangs its children on the socket
  (`AniMesh.dll:0x1000a7dd`), and all 1417 mounted parts' roots are empty. See
  [07-objects.md](07-objects.md#how-parts-attach).
- What reads object face flags 2 (the floors of the 29 walkable buildings) and
  16. The mesh visitor takes its triangle masks from the caller
  (`AniMesh.dll:0x10008120`), so the answer is in whoever queries a building's
  faces other than a round.
- What sets an object's material track (`ILifeSystem` slot 16,
  `Control.dll:0x10008810`; nothing found) and who calls IAnimation slot 27,
  which makes a mesh wear the material of a face it names
  (`AniMesh.dll:0x10005970`). See
  [07-objects.md](07-objects.md#who-picks-an-object-meshs-material-track--read).
- What IAnimation node mask bit `0x10` does — the ground contact sets it on a
  contact point's carrying node (`Control.dll:0x1001a3aa`).

## Unresolved terrain fields

- `MAT0`'s byte 4 is **answered: it is the ground's surface id**. Eleven
  values plus an unset `0xFF` sort materials into groups tracking their names
  (all six `TREE*` share 6), so it reads as a shader id. The loader copies it
  to the runtime material's `+0x154`, and the accessor for that field, slot 9
  of the material manager's vtable, is called by `Control.dll`'s ground
  contact on the material under a unit. An earlier answer, that no module
  calls it, searched only the modules that store a manager. The directory's
  flags byte beside it is the field a renderer wants. See
  [07-objects.md](07-objects.md#the-class-byte-is-the-grounds-surface-id)
  and [24-motion.md](24-motion.md#ground-and-collision--read-and-measured).
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
- ~~The draw order's flags bit `0x80`~~ — **answered: inert.** The landscape's
  draw reads bit `0x10` of the byte and nothing else (`Terrain.dll:0x1004399a`),
  and the engine's own rebuilders write the other bits as `0x48` through masks
  that keep bit 7 as it was; nothing reads or writes it. The 89 entries of
  `ILKON` (76) and `SC_3` (13) that carry it change nothing. See
  [03-terrain.md](03-terrain.md#what-the-engine-does-with-the-byte--read-and-measured).
- ~~Who asks a ground material for its second track~~ — **answered: the
  landscape's draw**, on every face that is not water, binding it as a second
  texture at render phase 9 (`Terrain.dll:0x1002b4b6`). See
  [03-terrain.md](03-terrain.md#who-asks-for-track-1--read-and-measured).

## Textures

- ~~Palettised textures and a colour key~~ — answered: **none is keyed**.
  `Ngi32.dll` uploads a palettised texture to an opaque surface unless its
  header asks for alpha, and no shipped header does. On an alpha surface it
  would clear index 0, which no palettised image draws. No module sets
  `COLORKEYENABLE`. See [02-texm.md](02-texm.md#what-the-loader-does-with-alpha--read-and-measured).
- ~~Who loads a texture opaque~~ — answered: `World3D.dll`'s material loader,
  for every lit skin (directory flags bit 1) unless `EMBOSS_BUMP` is on, so
  171 of the 279 alpha-format textures upload without alpha on this install.
  See [02-texm.md](02-texm.md#who-loads-a-texture-opaque--read-and-measured).
- **Header `+0x14` bit `0x4000000`** on 81 textures: no module reads it (a
  sweep with a positive control); its meaning is *unknown* and the engine can
  ignore it.

## Not looked at at all

`.scr` **node semantics** -- the structure is now [read](15-behaviour.md), all 58
files end to end, but what a node does is not -- and the network protocol.
The text layer around a mission is [read](21-briefing.md): the opening
flythrough, its subtitles and voices, and the in-mission messages, through the
[resource descriptors](20-resources.md) that bind them. Save
games in `SAVE/` are [partly read](17-saves.md): the header and what a save
refers to, but not the object graph, which is a raw heap dump. The `.trf` archives are [read](16-research.md): 368 research
items, their costs, descriptions, classification bytes and state, and their
prerequisite graph. The engine's own `.ini` files are now
[read](22-settings.md) — the component registry, the two debug files, the
display settings and the mission-progress dispatcher, each matched to the
module that reads it; the input tables beside them are [read](14-controls.md). `*.ctl` and `*.ndp` are identified above; `*.exp` and
`effects.rlb` are [read](11-effects.md), header and flags included, and so are
the light, bolt, stream and fade fields of the emitters; the rest of an
emitter's floats are not.

`sky.ske` is [solved](10-sky.md) -- day cycles of colour keyframes read the
way `Terrain.dll` reads them, all 29 files to the byte: each keyframe carries
its event opcode ahead of its slots, a file's two days play in turn, and the
clock starts at the file's closing time. What is not established is on the
engine's side of it: where the sun's two lights point, how the dome escapes
the far plane and the fog, and which way the camera's heading angle faces.

The ones that hold back a picture are triaged in [../TODO.md](../TODO.md).
