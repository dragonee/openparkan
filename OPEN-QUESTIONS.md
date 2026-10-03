# Open questions still to research

Compiled on 2026-09-14 against `a5e0188` from every doc's *Not established*,
*Not resolved* and *What is not read here* section, [TODO.md](TODO.md),
[docs/06-open-questions.md](docs/06-open-questions.md) and the unknowns in
[engine/README.md](engine/README.md#stand-ins). Each doc stays the source of
truth; this is the queue.

**[M1]–[M5]** marks a question the engine currently answers with a stand-in,
by milestone. Those come first. Left out: behaviour already read and waiting
only on the engine (the README's *Read since the stand-in was written*), and
engineering such as terrain culling or drawing the sky's textures. **That rule
misfired** until 2026-09-21: from Engine M6 on, the README filed its new
stand-ins under *Read since*, so this file left out 61 open stand-ins as read.
They are refiled, and compiling them is a line of its own under *Not looked at
at all*.

A line stays here until it is researched enough to close. A closed line moves to
[COMPLETED-QUESTIONS.md](COMPLETED-QUESTIONS.md) with its answer, under the same
section, and the record of the rounds that closed lines goes there too; a
remainder a closed line names stays here as a line of its own.

## Sky and rendering

[10-sky](docs/10-sky.md#not-resolved), [02-texm](docs/02-texm.md),
[07-objects](docs/07-objects.md), [06](docs/06-open-questions.md)

- [ ] **[M10] How the recordings' machine draws a liquid**: C03 M01's lava, and the lake's
  brightness with it. *Narrowed 2026-10-03* ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#the-rounds)).
  The recording's lava is (208, 7, 4) and shows its own texture and the mirrored plant together
  ("Let's Play - Parkan: Iron Strategy, Part 5", PfAg6zSe-yM, 1:25, briefing time 38 s); the
  engine, on `REFLECTION_SHIFTED` as this install's ini has it, draws (136, 17, 20). Read since:
  the drawing camera's type does reach the shade (`CShade +0xbcc`, set at
  `Terrain.dll:0x10085365`), a water face is filed in group 1 at layer 12 with `UseReflections`
  on and `UseEMBMReflections` off and at layer 1 otherwise (`0x1004452f`, `0x1004453b`), and
  with its under-water flag clear the reflection camera draws the water faces too
  (`0x10043ccc`–`0x10043d11`) — so under the plain `REFLECTION` it is the mirrored camera that
  draws the liquid as its own material into the frame the mirrored scene is in, the one read
  path on which texture and reflection share a picture. **The number does not come out**: the
  lava's lit colour is the scene-colour floor over its ambient, (0.65, 0.19, 0.25), and
  `LAV00.0`'s mean (174, 23, 0) gives (113, 4, 0) alone or (155, 1, 0) doubled through its
  twin's `LAV07.0`. Left: how a type-5 pass's liquid and the mirrored scene behind it share the
  picture (the liquid's plane is the nearer to the mirrored eye), which mode the recording's
  machine ran — no device test stands between `Iron_3D.ini` and the mode, and the compiled
  default is `REFLECTION` — and the lake's (162, 248, 253)
  ([03-terrain](docs/03-terrain.md#not-established)).
- [ ] Who calls `CCamera`'s `+4` slot 14 (`Terrain.dll:0x100852e0`), the routine that hands a
  camera's type — 2 `REFLECTION_SHIFTED`, 1 `REFLECTION`, 0 otherwise, made of its name at
  `0x10083a54` — to the buffering camera's slot 23 at `0x10085365`. Reached by the sky's and the
  light's branches independently on 2026-10-03; its caller was found by neither
  ([10-sky](docs/10-sky.md#the-skys-first-draw-is-a-screen-wide-quad--read),
  [03-terrain](docs/03-terrain.md#not-established)).
- [ ] [M1] Whether slot 17's alpha reaches a body's sprite. The sun's and the moon's sprite
  carries slot 17's colour on its four vertices (`Terrain.dll:0x1007cd98`), alpha and all, and
  the alpha is 50 of 255 or less on 249 of the 656 keyframes; what the device does with it is
  not checked against a frame, and the engine draws a body in its texture's own colours
  ([10-sky](docs/10-sky.md#the-sun-and-the-moon-are-drawn)).
- [ ] [M5] The line a room's mesh is asked along. A building's room test is the room's box and
  then a count of crossings by its mesh (`AniMesh.dll:0x10010550`) along the direction at the
  mesh's `+0x150`, whose writer is not read; the engine takes the room's own z. It decides
  whether the rain and the snow are drawn ([10-sky](docs/10-sky.md#the-weather--read-and-seen)).
- [ ] What sets camera flag 8. A camera whose flag word carries it draws neither the sky's pass
  nor the world (`Terrain.dll:0x100845eb`, `0x10084701`) ([10-sky](docs/10-sky.md#not-resolved)).
- [ ] [M1] Which shipped frames carry a scale. The shade puts a directional light into an
  item's frame by the transpose of the frame's 3 × 3 and normalises nothing
  (`Ngi32.dll:0x10024260`, `0x1001ba50`; three other builds of `g_FastProc`'s `+0x8c` were not
  read), so an item drawn at scale *s* takes *s* times the light; the engine takes a unit
  direction whatever the scale ([10-sky](docs/10-sky.md#the-lit-colour-is-the-games-own--read-and-measured)).
- [ ] Which light manager the gather passes over under bit 1 of the draw's mode: one whose
  registered object answers class 7 (`Terrain.dll:0x10047633`–`0x10047653`), for an agent
  attached to a parent's node whose record carries bit 0 at `+0xc`, or a draw whose argument
  carries `0x400` (`AniMesh.dll:0x10014d9d`). What answers class 7 and which nodes carry the
  bit are not read; if it is the sun's, a part hung inside a building takes no sunlight
  ([11-effects](docs/11-effects.md#not-resolved)).
- [ ] C03 M02's Large Factories' roofs, and the mine's glow. *Seen 2026-10-03 and not
  investigated*: in the briefing frames that settled the masts ("Part 6", -yNnsqudMzw, briefing
  time 49–62 s) each Large Factory's roof is violet-blue under its antenna in the recording and
  not in the engine, and `mineglow` on Enemy 1's Medium Mine is stronger there than here
  ([11-effects](docs/11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).
- [ ] **[M9] Camouflage's remainders**, left when the line closed 2026-10-03 for the hero
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#sky-and-rendering)): a dark patch with a white
  highlight where the hero's backpack lights sit, in every camouflaged frame of the recordings
  (its 164 batches carry no lightmap byte, and 37 carry the see-through word `0x100`, which the
  swap leaves alone — whether the patch is one of those is not established); what puts it out
  in Part 1 at 13:32–13:36, at the bunker's pod, where neither firing nor a hit does (725 of
  725 samples of "Tricky Tellus mission 2"); a camouflaged machine seen from outside on a floor
  or a bridge, and any enemy in the ground's texture, neither found in a frame; and, in the
  engine, a boarded bot and the AI's machines, which stay in their own skin, and camouflage's
  cut of the three signatures, which is not applied
  ([07-objects](docs/07-objects.md#what-a-camouflaged-machine-wears--read-and-seen)).
- [ ] **A loaded game's emblems.** Raised 2026-10-03: after a load every clan's sign is 0, the
  arrow — the loop that gives clan *i* sign *i* (`iron3d.dll:0x100a2420`) runs only on a fresh
  start, and the save's loader stores no sign — and Part 2 shows it, the enemy's Medium Factory
  wearing cell 6 before the player's load at 6:32.5 and the arrow after. **The engine keeps a
  clan's index as its sign across a load**, deliberately left: matching the game would turn
  every emblem into the arrow after a quick load, and whether to is the player's call. Not
  re-checked for a load before it: docs/07's own sighting of a captured warbot's arrow (Part 6,
  7:30) ([07-objects](docs/07-objects.md#in-a-network-game-and-after-a-load--read-and-seen)).
- [ ] The face filters the enumeration could not see. Of 159 non-stack arguments at the six
  face-query slots, 138 are static addresses in six `Behavior.dll` setting readers, one is a
  resource field, and **20 are values loaded from a field**, not read one by one; and the
  landscape's two translating wrappers (`Terrain.dll:0x1001e280`, `0x1001e400`) have no caller
  found. Left when the triangle-mask line closed 2026-10-03
  ([07-objects](docs/07-objects.md#who-fills-a-face-querys-filter--read)).

## Effects and sound

[11-effects](docs/11-effects.md#not-resolved)

- [ ] [M4] What a burst's fade below 0 draws. Type-10 debris is not dropped at an age of 1
  (its load clears the flag that ends a group there, `Effect.dll:0x10001ab1`), and 118 of its
  160 blocks fade to 0 at that age and below it after; the renderer draws nothing **at** 0
  (`Terrain.dll:0x1002887e`), and what it does under it is not read. The engine draws nothing
  at or under 0, and steps a group's clock for its fall by each update where the game adds it
  as the group is drawn. Left when the stream's and the burst's line closed 2026-10-03
  ([11-effects](docs/11-effects.md#a-burst-is-groups-that-leave-one-after-another--read-and-measured)).
- [ ] [M4] An instance's level, its remainders: a **moving** stream's share of its instance's
  sphere is read only in outline, a light's sphere is not read, and neither is the
  atmosphere's owner class, so the engine's lightning takes the robots' thresholds. Left when
  the level's line closed 2026-10-03
  ([11-effects](docs/11-effects.md#an-instances-level--read-and-measured)).
- [ ] The scale a stream particle's own matrix carries. By the read the matrix is built from
  the way the effect moved and the frame's third axis, and carries that axis's length on two
  of its axes (`Effect.dll:0x10011daf`–`0x10011e73`), which would make a chimney's plume 2.6
  times its block; the Large Factory's recording shows the block's own size, so the engine
  keeps its stand-in and the read is *not established*
  ([11-effects](docs/11-effects.md#bolts-streams-and-fades--read-and-measured)).
- [ ] [M4] How fast a lamp shimmers. Raised 2026-10-03 by a contradiction between two branches
  of one round: a light's jitter is redrawn on every update of its instance, and an instance
  that is being drawn updates on **every pass**, its draw's flag `0x40` opening the gate
  (`Effect.dll:0x1000817a`, `0x10007de5`, the 50 at `0x1001e4e8` and the 100 at `0x1001e4e4`
  re-read by the coordinator), where the page had every light stepping ten times a second.
  The engine still holds each draw for 100 ms, and no recording has been measured for a lamp's
  flicker ([11-effects](docs/11-effects.md#a-lights-jitter-is-five-draws-an-update--read-and-measured)).
- [ ] [M4] What *seen* is for a unit's own effect manager. A mesh draws its agent's manager
  with the pass 1 only while its sphere is inside the camera's six planes and the draw's flag 1
  is clear, and the landscape draws an object of no seen cell with `flags | 1`
  (`Terrain.dll:0x1001c8d7`), so the 198 effects carrying `0x800` — 147 of the 618 lights and
  29 looped sounds among them — stop with their owner unseen. Not read: which cells the
  landscape walks, the mesh's sphere, which flags `CBuilding::Render` (`0x10058b90`) adds, and
  whether a gun hung on a turret counts the camera as its own. The engine draws every effect
  whether or not its owner is seen, and files the camera's own unit's sprites with the world's
  where the game puts them in layer 7, under the cockpit ([M3], waiting on the cockpit's own
  pass) ([11-effects](docs/11-effects.md#which-draw-reaches-a-units-own-manager--read-and-measured)).
- [ ] [M14] What silences the hero's breath in its own view. Not a head-relative mode — no
  buffer's mode is ever set — and not, as far as it is read, the pass its flag `0x800` waits
  for, which a unit's own mesh passes while the unit is seen. Queued 2026-10-03 from the doc's
  own list ([11-effects](docs/11-effects.md#how-a-sound-is-heard--read-and-measured)).
- [ ] The volcano's flame. The feedback's line — the crater's flame about twice as large in
  the recording, its brightness varying a third as much, beating at 5.3 Hz with almost nothing
  at the interval's 12.5 Hz (Part 5, 4:19–4:24) — was queued with the stream's pace, which
  closed 2026-10-03 without it: the recording's bright glow measures 39–46 px, about 8–9 m,
  which is `tree_light_33b`'s 8–10 m sprites, and the beat was not measured again. Whether the
  engine's flame still differs is not looked at
  ([11-effects](docs/11-effects.md#who-hands-a-manager-its-time--read-and-seen)).

## Motion, ground and controls

[24-motion](docs/24-motion.md#not-established), [13-control](docs/13-control.md#not-established),
[14-controls](docs/14-controls.md)

- [ ] [M14] The walker's local path and its obstacle contours: how it goes round a tree's or a stone's hole, whether it widens it by the unit's size, how a walker in one walks out, and what it does with a goal in one; the sub-areals' shapes and whether the search measures one from its centre; whether every scenery object reaches the areal map, and the box of a mesh of several parts; how it drops the points a unit has passed (`MWalker::ClearMoverReachedPoint`); how a unit's place comes onto a building's map object and which vertex the search starts from; who calls `MHallWay` slot 11; a hall-way vertex's size gate (the unit's `+0x960`, the record's `+0x28`); the link flags `0x10000` and `0x20000`; how a walker goes to the point it finds off a non-walkable areal, and what it does when its search fails; and how a walk to a door gets past the building's own walls, which cut no areal (the engine: a door more than 20 over the ground under it is passed over, and a straight line into a wall goes round the building's ground contour) ([24-motion](docs/24-motion.md#not-established)).
- [ ] **Where the frame is drawn among move, pass, contact and push.** *Narrowed 2026-10-01* out of
  the low-ceiling line ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#motion-ground-and-controls)).
  The pair pushes out a unit's node sphere, not its agent sphere, so C02 M03's medium walker
  `22mwlk1` under the Small Warehouse's ceiling is pressed **0.28 m**, not 2.85: its node sphere is
  4.26 about a centre 0.67 below its origin and tops out 6.63 over the floor. The recording shows it
  upright in every frame ("Let's Play - Parkan: Iron Strategy, Part 4", 1:43, 15:13–15:15). Still not
  read: where the frame is drawn in the takt. The engine draws the body where the contact last held
  it ([24-motion](docs/24-motion.md#not-established)).
- [ ] **What a building's second sphere is.** A collision object of kind 3 or 4 keeps two spheres;
  a round is swept against a unit's first, its agent sphere, but against a building's second
  (`Control.dll:0x1001d6cd`), which comes from its device manager's slot 12 (`0x1002c500`). What that
  slot answers is not read ([24-motion](docs/24-motion.md#collision-between-objects--read),
  [26-damage](docs/26-damage.md)).
- [ ] **Whether the walk-face query reads a door's faces.** *Seen on the engine, not read*: a large
  walker that reaches the Large Factory's front leaf while it is still sinking is lifted 5.6 m onto
  the leaf's top by its feet's up pass, whose bound is 7.5, and rides it down for about a second
  ([24-motion](docs/24-motion.md#not-established)).
- [ ] **What lets a mover past a face flagged `0x20`.** Raised 2026-09-20 from play on C02 M04,
  *The Last Bastion*. The collision's own two filters take a triangle mask of **4**
  (`Control.dll:0x1001dbad` for the segment, `0x1001dbce` for the push-out) where a round's takes
  `0x24`; nothing read drops `0x20`. The shipped data needs it dropped. *Measured*: all **9** bridge
  pairs the 29 missions place stand π apart with their decks abutting, so each half's join cap — six
  level-0 faces at the deck's far end, all facing the span — is square in the way; three of the four
  `fortif.rlb` bridge meshes flag that cap **4** and `fr_e_brige` flags it **`0x20`**, the bit its
  additive `B_A_BRIGE` material carries throughout. Taken as read the hero walks 183.8 m of the 185.5
  to the join and stops. The **batch word** is the obvious other candidate — the query excludes
  batches flagged 8, and the energy batches carry `0x100` and no 8 — so either a third filter or a
  flag set on the loaded batch does it. The engine passes `0x20` as a stand-in
  ([24-motion](docs/24-motion.md#the-cap-where-two-halves-meet--measured-and-a-stand-in)).
- [ ] **What stops a tower's gun mast at the top.** Raised 2026-09-21 from play on C02 M04,
  *The Last Bastion*. Both `fortif.rlb` tower controllers carry a **class-29** component whose
  channels raise the mast out of the ground — `fr_m_tower`'s node 13 by 10.4 m over mesh frames
  1 to 4, `fr_b_tower`'s node 12 by 19.4 m over 0 to 3 — and each sets its switch word at
  `+0x18` to **9**, open and *bouncing*. Bit 8 is read to hold the progress at the end and swap
  the low bits (`Control.dll:0x10020ae4`), so as the file stands a tower raises its mast over
  five seconds and stows it over the next five, for ever. *Measured*: those two are the only
  records of the install's 1066 whose word bounces; the other 28 that set the word at all set 0
  (class 25) or 33 (the hero's arms). Nothing found switches it off — the component factory
  files every class in the controller's timed list (`0x1002d70a`), `CBuilding` looks for classes
  12 and 13 and no other (`Terrain.dll:0x100583a2`), and neither record names a section-5 group
  at `+0x10` or `+0x14`, which is what an item runs on its first step and its first skip. In the
  game the mast comes up as the tower is built and stays up, so the engine starts a bouncing
  item as one that opens and stops. What is left to find is the switch: the likely places are
  the fire control that owns a tower's turret and whatever handles a building finishing
  construction ([28-chassis](docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured)).
  *Seen* 2026-09-30 in "Let's Play - Parkan: Iron Strategy, Part 4" (Qqs8_i9IeUU): the valley's
  Light Tower keeps its mast up through the 40 s of the hero's attack from 20:42, its target
  silhouette sampled every 3 s, so the game's mast does not bounce on a ten-second period; its
  turret's node shows red from 21:27, and the mast sinks to a stub by 21:36. So the mast stays up
  while the turret lives and is stowed once it is lost — what switches it is still to find, and
  the engine keeps it up either way.
- [ ] **Whether a building's collision mesh follows the nodes its own items turn.** Raised
  2026-09-21 alongside the above. Nine class-26 records turn nodes that never stop — the three
  mines' rotors, the Main Teleport's twenty-eight rings, `fr_e_brige`'s hub — and the engine
  poses them every tick but rebuilds the collision solid only for a door's channels, since
  rebuilding it for a hub turning for ever would rebuild it every tick on every one of them.
  Whether the game's own `IGeometry` follows an animated node, and at what cost, is not read
  ([28-chassis](docs/28-chassis.md#every-component-is-stepped-not-only-a-device--read-and-measured)).
- [ ] **What separates control message 7's argument 0 from 1**, which `Control.dll` treats alike;
  argument 2 is read, the object simulated elsewhere. No shipped section-5 record tests condition
  byte 15, so nothing an artist wrote turns on it. The remainder of the `.ctl` values line closed
  2026-09-20 ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#motion-ground-and-controls),
  [13-control](docs/13-control.md#control-message-7-says-who-simulates-the-object--read)).

## Turrets, weapons and camera

[29-weapons](docs/29-weapons.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[28-chassis](docs/28-chassis.md#not-established), [25-sensors](docs/25-sensors.md#not-established)

- [ ] [M5] How the HUD draws the aim point, the guns and the player's target; what plays `TARGET_READY` and `TARGET_ZOOM`; and the unit record's `+0x98`. ~~Its `+0x94`~~ — **read**, noticed 2026-09-22 by the coordinator: docs/39 had it all along as the radius the sphere interface `0x20` slot 3 answers as the record is bound, the unit's node sphere (`0x1007e5f2`–`0x1007e60e`), 11.84 on the L-2f, which leaving and the game frame read. The right button's pick still takes it as 0, filed under the README's "Read since" ([25-sensors](docs/25-sensors.md#not-established)).
- [ ] **What lights a unit of more than one part in a panel's view.** Left 2026-10-01 by the mode-2
  read ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#turrets-weapons-and-camera)). A mesh of agent
  kind 2, 3 or 4 makes two directional lights at its set-up (`AniMesh.dll:0x100070c9`–`0x1000723b`),
  and a panel's view is lit by the shown unit's own lights alone. The one-part dummy matches that in
  the recordings; the hero, a chassis and a turret, is 2.06, 1.98 and 2.03 times as far over its
  floor, and Mission 01's spider and a driven warbot look the same. What makes the second pair was
  not found. The engine takes two pairs for a unit of more than one part, a stand-in. Also not read:
  that the sky's light manager is of kind 7, which is what keeps the sun off the placement ghost
  (*inferred* from the recordings), and what the phase at the shade's `+0x1928` sets
  ([35-hud](docs/35-hud.md)).
- [ ] Whether the designer's two preview lights are turned twice: the mesh gather turns every
  light's stored direction through its manager's owner's placement (`Terrain.dll:0x100802f0` with
  space 2), and the designer already sets its two lights in space 2 each draw. Noticed 2026-10-01,
  not followed up ([37-designer](docs/37-designer.md)).
- [ ] Which objects answer world class 2, the one class the outer camera's line drops. *Narrowed
  2026-09-20* with three controls, all negative. A class is slot 11 of the object interface and the
  query's first word is anded with `[class*4 + 0x1009a5f0]`, sixteen dwords whose entry *k* is `1 << k`,
  so the camera's `0x41a` does drop class 2 from a round's `0x41e`. **Class 1 is the landscape**, named
  by `CLightning::Init`'s own panic. (a) Every `call [reg+0x2c]` in all 21 modules followed within 8
  instructions by a comparison with a constant < 16 gives **68 sites** naming 1, 3, 4, 5, 6, 7, 9, 10
  and 11 — **never 2**. (b) Of the **71** `return N < 16` stubs in the install, the **7** that sit at a
  vtable's slot 11 answer 1 (three times), 3, 5, 7 and 11; four stubs do return 2, none at a slot 11.
  (c) The game-message switch on the id-class nibble gives 1, 3, 4, 7 and 11 a case and drops 2 to the
  default — weaker, since it drops 10 too. A class computed at runtime would still escape all three
  ([30-turrets](docs/30-turrets.md)).
- [ ] What the design state's `+0x370` is — the string all three readers of a design's `Type` test
  before anything else, answering `0x20000000`, an animal, when it begins with lower-case `a`
  (`iron3d.dll:0x1004f318`, `0x100514fb`, `0x100544f9`, each `cmp byte ptr [ecx], 0x61`; the boundary
  confirmed by five start offsets converging on it). *Raised 2026-09-20* out of the formats agent's
  part-category work, **and it corrects a claim in [30-turrets](docs/30-turrets.md)**, which read that
  string as the chassis's name ("a chassis whose name starts with `a`"). It is not: *measured over all
  **29** trees, which carry an identical set of **27** chassis part ids, **all 27 upper case** —
  `A_L_01`..`A_L_05` being the five animal ones — so **0 of the 783** examined begin with lower-case
  `a`*, and the test could never fire on a chassis id. The likely candidate is the designer's own kind
  prefix, spelled `r_`, `fr_` and `a_` and handed to the layout at `0x1004dff0`, but **no store into
  `+0x370` was found in `iron3d.dll`**, so that is *inferred*, not read — and that missing store is the
  next place to look. It makes no difference to a robot design, where the base part is a chassis and the
  `Type` comes from the turret either way ([38-designs](docs/38-designs.md#not-established),
  [30-turrets](docs/30-turrets.md)).
- [ ] Where the unit constructor's page item names (`+0xc4`) come from.
- [ ] The fire line's remainders, left when the AI's clear line closed 2026-10-01
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#turrets-weapons-and-camera)): whether a building's mesh
  agent answers the building's id, so that leaving the target's id out of `IWorld` slot 12's walk
  leaves its faces out (the engine leaves the whole target out); whether the landscape's slot-4
  record `+0x3c`, compacted into the word the 20 m exception reads, is the file's face dword
  (*inferred*; if it is, the exception never fires on ground, 0 of 275882 faces carrying `0x80`);
  and the 25 to 50 m between the engine's 414 m launch on C03 M02 and the recording's 365–390. The
  turret's property `0xf00` and the gun's report codes stay the README's M8 stand-in
  ([29-weapons](docs/29-weapons.md#not-established)).
- [ ] **Which gun killed the hero in C02 M04's valley Light Tower.** *Narrowed 2026-10-01*
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#turrets-weapons-and-camera)). A blast is stopped by
  nothing but distance, so one landing on a building reaches a hero in its pod room, as C03 M02's
  raid shows; and the tower's own death is ruled out, since every node of a building names a kind-1
  explosion, an effect alone. Which gun's blast it was at 21:53.5 of "Let's Play - Parkan: Iron
  Strategy, Part 4" is still not established.
- [ ] The blast's remainders: whether a round in the air hangs in the landscape's object grid, whose
  class the blast's mask `0x61c` carries; what a unit's behaviour does when message `0x19` names
  itself, which a firer inside its own blast is sent before the test that spares it; and a
  non-round node's blast — node 1 of `o_tur_la_06`, `explode_rbr_bomb`, 20,000 in 43.4 m, which no
  shipped design carries and the engine plays as an effect alone
  ([26-damage](docs/26-damage.md#not-established)).

## AI, scripts, packages and economy

[15-behaviour](docs/15-behaviour.md#what-is-not-read-here), [31-packages](docs/31-packages.md#not-established),
[23-economy](docs/23-economy.md#not-established), [32-builder](docs/32-builder.md#not-established)

- [ ] **What the engine below a SuperAI does with a problem's action record** (`+0x34`, function 27).
  Raised 2026-09-21 by M18. `varset.var` names the five *"what to do with expression"*, *"When
  building capture"*, *"When all units in group killed"* and *"When all units in group nothing to
  do"*, and nothing else ends a problem a handler left `ST_SOLVING`: `PBM_ROBOT_NEEDED_Start` files
  `ACTION_NOTHING_DOING` on the factory it ordered and its `_Continue` has no nodes, so without them a
  clan builds one warbot and never another. The engine retires a problem whose action has come true,
  tested at the head of each clan takt. What the game actually does with the record — retire, re-raise,
  or something else — and where it is tested, is the question
  ([15-behaviour](docs/15-behaviour.md#what-the-functions-do)).
- [ ] **What `fn8(ST_SOLVED)` leaves behind, and what the repeated `_Start` pass excludes.** Raised
  2026-09-21 by M18. The state setter is read only as far as releasing the problem's units, but a
  solved record that keeps its slot would be matched by the raise's duplicate test and block the next
  want for as long as it stood — which [23-economy](docs/23-economy.md) says does not happen. And the
  pass is read to run again whenever a handler leaves its problem neither solved nor solving, which
  `PBM_ROBOT_NEEDED_Start` does when the clan has no factory: as read, that is an endless loop. The
  engine retires a solved problem and does not offer a problem twice in one takt
  ([15-behaviour](docs/15-behaviour.md#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured)).
- [ ] **A `PBM_PLACE_PROTECT` place that is not on walkable ground strands the clan's warbots.**
  Raised 2026-09-21 from play on C02 M03. `c2m3e`'s `Problems0` names three places; (265, 1540) has
  **0 of 400** points sampled within its 150 m radius usable, its centre included, so the patrol's 350
  tries all fail and it takes the last point tried ([24-motion](docs/24-motion.md#not-established)'s
  stand-in). The clan's nearest capturer was standing off the areal map at (1341, 997) by the time the
  timed `PBM_BUILDING_CAPTURE` fired; from there the route is one escape leg, walked straight into the
  gorge wall, and it stands at 0 m/s for the rest of the mission — while from where the mission places
  it there is a 31-leg route over the bridge to the player's factory. What the game does with a patrol
  place off its own areal map, and what its walker does once stranded, are both unread.
- [ ] **Why Enemy 1's first builds on C03 M02 are medium tracked.** Left 2026-10-01 by the design
  store's line ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#ai-scripts-packages-and-economy)). In
  "Let's Play - Parkan: Iron Strategy, Part 6" Enemy 1's builds are named MTW-3, MTW-4, MTW-5, then
  LTW-6, and in Part 6.5 its first is MTW-3 again. As read, `SELECT_BEST_COMBAT` with the easy
  level's spread of 6 draws from seven designs — four large tracked, two large wheeled and one
  medium tracked, `m_stopper.dat` — so three in a row is one design of seven drawn three times.
  Later names are not explained either: MWW-21 to -23, where `23_m2`, the Medium Wheel, ranks eighth,
  one past the spread, and LTW-34 and -35. Not read: whether the draw `(rand() + timeGetTime()) %
  (n + 1)` is as free as it looks, and whether the level ratio reaches the object the store's fill
  builds ([15-behaviour](docs/15-behaviour.md)).
- [ ] Function 41, a clan's research order, in the engine: the design store's `+0x104` byte is
  marked again each time function 41 orders a research (`ai.dll:0x10010f90`), and only `c4m2e2` and
  `scream` call it. The engine leaves 41 unanswered, so a store is marked once, at load
  ([15-behaviour](docs/15-behaviour.md), [16-research](docs/16-research.md)).

## Mission progression

[34-progression](docs/34-progression.md#not-established)

- [ ] What the behaviour does with the message 6 it sends itself.

## Files and formats

[17-saves](docs/17-saves.md#not-established), [16-research](docs/16-research.md#what-is-not-read-here),
[19-descriptions](docs/19-descriptions.md#what-is-not-read-here), [18-vocabulary](docs/18-vocabulary.md),
[21-briefing](docs/21-briefing.md#not-established), [06](docs/06-open-questions.md), [22-settings](docs/22-settings.md),
[12-rsli](docs/12-rsli.md#not-resolved)

- [ ] Saves:
  - [ ] most chunks' contents (the control chunk past `+32`, the wizard, behaviour, building and tree chunks)
  - [ ] the clan word before each mind list
  - [ ] the 24-byte records and the `1, id, id` triple
  - [ ] the AI state's layout
  - [ ] whether a mind list's ids are logical ids
- [ ] What the words behind `objects.dlb`'s `A` and `N` are, and why twelve clip-less guns carry `A4`
  and `A5`. (~~what the `A` and `N` size letters stand for, and the 37 exceptions~~ — partly answered
  2026-09-18: the **referents** are pinned. `N` is the five `ANM` animals, which the research tree marks
  role 7 and no others; `A` is 27 fortification fittings, 21 of them fitted only under an `fr_*` root, and
  every line in the file that says "fortification" belongs to one. The words are **not recoverable**: the
  classification vocabulary reaches no shipped binary, with the `objects.dlb` token itself as the control.
  Of the 37 exceptions, 25 fall out of a rule — a weapon's `A<n>` is the size of the **round it fires** —
  7 more fire `f`-lettered rounds the rule cannot grade, one is a mobile builder, and 12 stay unexplained
  ([19-descriptions](docs/19-descriptions.md)).)
- [ ] What the word after `data.tma`'s map path was for — it marks the free-play maps, but `Single.02`
  disagrees with every reading. (~~what reads a building's start flag back, and the word after the map
  path~~ — closed 2026-09-18, both negatively and with controls: *nothing reads either*. `IMission`
  slot 11 returns the map word and no module calls it; the mission pointer is never stored outside the
  five frames that hold it. **The start flag's half was
  wrong**, corrected 2026-09-30: the landscape insertion calls `IBuilding` slot 13 through an answer it
  keeps on the stack, and the flag keeps a building at its file height
  ([04-missions](docs/04-missions.md#the-start-flag-keeps-a-building-at-its-file-height--read-and-measured)).)
- [ ] What the landscape, camera and atmosphere component constructors read.

## Mission 02, *The Constructor*

[34-progression](docs/34-progression.md#not-established), [36-factory](docs/36-factory.md#not-established),
[37-designer](docs/37-designer.md#not-established), [38-designs](docs/38-designs.md#not-established),
[39-boarding](docs/39-boarding.md#not-established), [24-motion](docs/24-motion.md#not-established),
[31-packages](docs/31-packages.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M12] ~~The medusas stand still; an animal's migration is not modelled; what their attack does with the migrate task's figures and circle; which difficulty profile an animal holds~~ — **partly answered** 2026-09-21 and 2026-09-22. The attack's half was stale, and the migration is modelled as read ([31-packages](docs/31-packages.md#migrate-an-animals-pasture--read-and-measured)). **No unit holds a difficulty profile** (*read*, a negative with its control): the one way into `+0x8d4` is `MBehaviour` slot 25 with kind 5 (`Behavior.dll:0x1000a1a0`), and a byte search of the whole install finds `diff_` only in `behpsp.res` itself, where the same search finds `prof_war` in `Behavior.dll` and `chas_fly` in `objects.rlb`; every behaviour keeps the block's compiled defaults (`0x10019b90`), `Speed_MaximumFactor` 1 ([26-damage](docs/26-damage.md#the-difficulty-block-every-behaviour-holds--read-and-measured)). An animal's walk points stand 45 to 95 m over the ground (`0x10040f20`: 30 and up to 50 more), and when no anchor state fits the planner queues the path back to the anchor it is in (`Control.dll:0x1000531a`). **The player's account settles the target** (2026-09-22): the game's medusas *"usually stay in one place until provoked by attacking them"*, and are not remembered flying — so a grazing medusa's hover is right, and **the walker's read heights are what diverge**: driven as read, a grazing medusa creeps 5.6 m up in two minutes and a fought one ends 30 to 50 m up, grazing on in the air because the Wizard's descent fits no moving anchor either. The engine now keeps an animal's points at the walk's own height, at least 15 over the ground — a stand-in pinned by a Mission 02 install test that fails with it off — and the medusas graze within 2 m of where they hover and fight no more than 10 m up. **What is left: what holds the game's medusas low** when the walker stands their points 30 to 80 m over a flyer's, and whether the recording's *"flying high"* from 195 s, read before the player's account, holds up frame by frame ([34-progression](docs/34-progression.md#the-medusas--read-and-measured), [24-motion](docs/24-motion.md#a-flyers-walk-points--read-and-measured)).
- [ ] [M12] Why a medusa off its clan's pasture flies there once a fight is over, and not before: on C01 M02, *The Arrival*, a medusa about 500 m from the pasture its clan picked holds its hover while grazing (5 m in 60 s), and once its 10 s attack on the hero ends it flies off towards the pasture at about 7 m/s, low, 221 m in 40 s. Before the eighth round (`0715e11`) it stayed after the fight too (3.4 m in 40 s), so something in `9d6784a` — the heading curve or the flyer's leg cut every 20 m — lets a post-fight walk find a moving state the grazing walk does not; the height stand-in above does not change it. Which the game does is not established; the player's account has medusas staying in one place ([34-progression](docs/34-progression.md#the-medusas--read-and-measured)).

**The warbot designer**

- [ ] [M11] What a fit does to the panels' previews: the destination's takt rebuilds a preview only for a picked row that holds a part, and what the fits themselves do to either preview is not traced. The engine shows each row's own part ([37-designer](docs/37-designer.md#the-rows--read-and-seen)).

**Boarding, flying and getting out**

- [ ] **What else reads a boarded hero's object place**, other clans' sensors among them: the game
  frame keeps the object on the bot, and the engine keeps the hero's own body where it boarded. The
  remainder of the boarded hero's route line closed 2026-09-22
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#mission-02-the-constructor),
  [39-boarding](docs/39-boarding.md#boarding--read)).

**Walking into the Large Factory**

- [ ] [M14] Why 4 placed buildings whose start flag is clear stand 0.03 to 0.14 off their cut contour's mean, where the insertion sets such a building down: C01 Mission 01's bunker (0.14), C03 Mission 01's generator (0.14) and bunker (−0.07), and C04 Mission 02's generator (0.03). The other 12 of the 16 off their mean carry the flag, which keeps them at their file height ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#mission-02-the-constructor)). The engine keeps every placed building at its mission height, sets down one the console's `bcreate` makes, and cuts the landscape by a mask with a buried apron rather than re-triangulating it ([03-terrain](docs/03-terrain.md#a-building-is-set-down-on-the-mean-of-its-contour--read-and-measured)).
- [ ] [M11] ~~What reconciles two reads with the recordings: the slope brake on a building's faces, which kept the hero off Mission 04's teleport chamber~~ — **answered** 2026-09-22 for the brake: it acts only while body `+0x1aa` is set (`Control.dll:0x10015690`), and the ground contact clears that on any held face whose class carries 2 (`0x1001a9b4`–`0x1001a9be`, re-read by the coordinator): a building face's triangle word, where 2 marks a walk-through floor, and the landscape's class from flags `0x8000`, on **0 of 275,882** landscape faces where `0x2000` is on 6,102 and `0x4` on 32,450. Of `fortif.rlb`'s 15,562 walkable level-0 faces 4,534 are floors, and the stairs and the chamber are among them: the engine's hero now wins Mission 04 6.0 s after the landing, the recording's 4.6. **What is left: what keeps a flyer's sphere off a building's floor it is made on.** With the read walk-point heights Mission 02's L-2f climbs clear of the factory rather than onto its roof, but kept, the floors still push it 33 m up in its first frame, so no robot keeps them ([24-motion](docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).
- [ ] [M8] What holds a machine the AI drives on a slope, when the game never brakes one: the brake is passed over while a velocity the Wizard wrote stands (body `+0x1a8`, set by `SetTangSpeed`, `Control.dll:0x100044f1`, cleared by the command's setter, `0x1000442b`, tested at `0x1001566b`). Without the brake the engine's AI units walk up 40° faces and a small warbot's refit never reaches Mission 03's bunker dock, so the engine keeps braking them, filed under the README's "Read since the stand-in was written"; what does the holding in the game — the local path, the areals, the planner's anchor boxes — is not established ([24-motion](docs/24-motion.md#ground-and-slope--read)).
- [ ] [M14] What the engine still draws that the portal fade would leave out, and two figures under it: every cell is drawn, so beyond far a black doorway covers a room the game skips; the field of view in play is taken as 1.3 rad, not read; and 57 non-portal `DEFAULT` batches — on trees, internal systems, turrets and lower detail levels — which the game draws black and openparkan leaves out ([24-motion](docs/24-motion.md#not-established)).

## Mission 03, *The Field Base*

[40-command-mode](docs/40-command-mode.md#not-established), [41-commander](docs/41-commander.md#not-established),
[42-selection](docs/42-selection.md#not-established), [32-builder](docs/32-builder.md#not-established),
[23-economy](docs/23-economy.md#not-established), [31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established), [37-designer](docs/37-designer.md#not-established)

**Where the engine is weaker than the game**

- [ ] **Why the recording's transport round is 13 s longer than two full-speed walks**, now that a
  Mission 03 transport walks 23.98 m/s as docs/23 gives it (the speed line, closed in M13,
  [COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#mission-03-the-field-base),
  [23-economy](docs/23-economy.md)).

**Left by the tenth round** (2026-09-28)

- [ ] Why the game spares a clan whose slot a joining player has taken from the network rule that keeps a new building 400 from another clan's base point (`0x10033de4`), and whether players already in the session as the game starts count as joining. **Narrowed** 2026-09-29, *read*: nothing else reads the byte — one instruction in `iron3d.dll` uses displacement `0x788`, the writer, and of 98 pointers into the clan array and 51 loops stepping 0x68 the only read is the walk's (the control: the sweeps find both). It is set by the network callback's message 5, which also clears the net manager's computer flag for the slot and sets its player flag; `World3D.dll` raises 5 when DirectPlay reports a new player who matches no departed one by name and password (`0x10005a50`, `0x100070ca`), and a returning (6) or leaving (7) player hands the clan's SuperAI 1 or 2 and never clears it. Network play only; the engine needs nothing ([32-builder](docs/32-builder.md#not-established)).

**Left by the eleventh round** (2026-09-29)

- [ ] [M12] A save's own bytes, left when the save page and the quick save closed 2026-10-01
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#mission-03-the-field-base)): a quick save's file, since
  no `slot7.sav` is installed to walk; what a save keeps of the interface — *seen*, one made in a
  bunker's command view loads on foot in the bunker, and what a load does with a hero saved aboard
  a bot is not; and what `Run` does when a slot's file is gone. The engine's quick save is the play
  kept in memory, in no slot, and the game menu's save page is not built
  ([17-saves](docs/17-saves.md#not-established), [14-controls](docs/14-controls.md)).
- [ ] A *Tiny Tower*'s battery of capacity −1: `Power::load` drops a negative battery, so the engine's battery arc follows the tower's fitted 31,000, where the game's id 1 answers 1 whenever any capacity is negative (`Control.dll:0x1002b42b`). How the power tick spends a −1 battery on a unit is not read ([41-commander](docs/41-commander.md#the-box), [23-economy](docs/23-economy.md)).
- [ ] What a building going up is still reached by: what interface `0x20` slot 3 answers for a node hidden by action 1, which is not a destroyed node — a blast takes each node by that sphere (`0x10010030`), and nothing else in its walk asks whether the node is shown (narrowed 2026-10-01); and whether Behavior's hall-way searches pass over a building in the second between its showing at 40 s and its sphere's end at 41 s ([26-damage](docs/26-damage.md#not-established)).
- [ ] Whether the HQ camera's ride holds while an HQ patrols. Since a lone go leaves a patrol of 150, Mission 03's HQ ride test measures only while the go runs: once over, the patrolling HQ put the engine's camera 14 m behind the ride's figure, which nothing now checks ([40-command-mode](docs/40-command-mode.md#an-hqs-command-mode-mode-3--read-and-seen)).
- [ ] What Alt+F toggles under `Iron_3D.ini`'s `DEBUG_KEYS_ON` (`0x10059d30`, `0x100717e3`), which the install does not set ([40-command-mode](docs/40-command-mode.md#not-established)).

## Mission 04, *Teleport*

[40-command-mode](docs/40-command-mode.md#not-established),
[16-research](docs/16-research.md#not-established),
[27-ownership](docs/27-ownership.md#not-established),
[31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established),
[24-motion](docs/24-motion.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[35-hud](docs/35-hud.md), [41-commander](docs/41-commander.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M3] The ground contact runs once a frame (message `0x1c`, read), but the engine runs it at state steps, and moving it waits on walking from the held ground face. Until then a hero pressed against Mission 03's Small Bunker door as it sinks is pushed down between steps; the bunker walk passes, but only just.
- [ ] The helicopter's capture pace: the recording takes 90 s to the factory and 81 s on to the research centre, the engine 54 s and 42 s at its live 14 m/s. And what held it still near (715, 847) from 362 to 402 s.
- [ ] The briefing's route is checked only by the engine's own test: the recording's player walked to the Teleport, so the large flyer's build, its flight to the HQ, boarding it, flying up to the plateau and setting down there were never compared with the game.
- [ ] Turret pitch and altitude as the user saw it. Measured: pitch moves neither a flyer nor its eye. Explained as rising ground lifting a low flyer, which never sinks back without F (read, `0x1001b3c3`); not confirmed in a window.
- [ ] Tooltips (the research rows' 6251/6252, the batch button's 6243), the maps' route lines and the selected unit's white outline: not drawn.
- [ ] [M13] The research box's name colour and its clip 5; the part preview draws larger than the game's.

**The HQ's command view**

- [ ] [M13] Which bound a unit record's `+0x98` is: 8 × it is how far the camera rides behind the HQ. The engine takes the chassis sphere, 61 m, which the recording favours over the whole bound's 104 m and the cylinder's 38.7 m.
- [ ] [M13] What the game does when an HQ reached from a bunker's command view is lost in its mode 3 (nothing read pops it), and how the stack reads after Enter in telepresence aboard an HQ. ~~An HQ the hero boarded~~ — **read** 2026-09-22: the game frame tests the boarded bot every frame and rolls the stack back to mode 0 once the component test refuses it (`iron3d.dll:0x1005eacf`), through any view standing on it ([40-command-mode](docs/40-command-mode.md#not-established)).
- [ ] Why the HQ reads "[no order]" in its cockpit at 160 s after "[standing]" at 94 s. The engine's capture-standby departure gives it Standby.

**Capture and the maps**

- [ ] [M13] How a walker's path joins the hall way (`MGraph`), so which exit a capturer takes (the engine: the shortest whole way); whether a flyer touches down or hovers at its landing corner; and a flyer against a building's walls outside the hall way (the leg to exit 67 skirts the factory's west side).
- [ ] [M13] The escape's damaged-node test, and which paths a unit leaves a building by (the engine: back along the hall way).
- [ ] [M13] The maps' contact list: the engine marks what lies in a player unit's radar range, not the list the run loop empties and each unit's takt refills.
- [ ] The sphere behind `IBuilding` slot 15.
- [ ] Why the Teleport's map icon looks white at about 412 s, as if selected, when a main teleport's capture selects nothing.

**Research**

- [ ] [M13] Whether a centre's ore take reads the ore it holds or the ore delivered to it.
- [ ] [M13] How often a building's takt runs (the engine: every tick), which bounds how late "Research complete" can come.
- [ ] What becomes of a research when its centre is captured, upgraded or destroyed mid-way (the engine keeps the queue with the building).
- [ ] Whether anything besides the task marks an item researched (scripts, saves), and the AI clans' own research orders (order 16), which are not modelled.

**The Main Teleport**

- [ ] What the teleport's class-25 parts do with the power byte's state `0x20`, and its class-29 parts with 1. *Seen*
  2026-09-30 in C02 M04's briefing ("Let's Play - Parkan: Iron Strategy, Part 4", Qqs8_i9IeUU,
  16:18.5, briefing time 49.5 s), the enemy holding the teleport and both generators: the tower's
  mast is closed, a blue-violet shaft under a blue cylinder, and the arch's opening a pale
  blue-white face, where the engine spreads three red vanes on a grey shaft and puts an orange glow
  at the arch's foot. Which of those the power byte drives is not read.
- [ ] What building interface `0xb` slot 16 asks about a place's node; what property `0x208` is (a network mirror flag is a guess); whether anything sets `pTeleFunc`.
- [ ] Whether a player-driven small unit can take the Teleport's pod.
- [ ] [M14] The places besides a dock's and a main teleport's (loading places) do not tick by the place rule yet; the place timer's random source; whether a destroyed generator stays in `World3D.dll`'s queue 3.
- [ ] [M14] What the game drives a dock's `f_recharge_*` glow with, whose own time mode is 0, a value set from outside.
- [ ] [M14] The AI's camouflage in play. Its repair decision and its trip to a dock are in (the engine: ~~`diff_strong`'s 0.8 and 0.9 for `Decision_RepairOn` and `_Off`, since the profile a unit holds is not read~~ — **read** 2026-09-22: no unit holds a profile, and every behaviour keeps the block's compiled 0.5 and 1 ([26-damage](docs/26-damage.md#the-difficulty-block-every-behaviour-holds--read-and-measured)); the nearest dock its size class fits, since the pick `0x10023b60` is not read, and never a building's own repair system).
- [ ] Why the chamber's glow reads greyer than the recording's. ~~Why the hero's panel dims in the pod room~~ — **narrowed** 2026-10-01: a panel's figure is its node colour held up to the scene colour ([35-hud](docs/35-hud.md#what-mode-2-does-with-the-colour--read-measured-and-seen)), so it changes with the clock, not the room; Mission 03's sky takes the scene colour from (104, 57, 65) at 02:40 to (40, 40, 40) at 03:40, about 80 s into play, and whether Mission 04's is the same was not checked. The glow's half waits on the world's lit colour taking the scene colour as a floor, where the engine adds it.
- [ ] Message 16, the helicopter in route 2, has no test.

**Movement**

- [ ] [M13] Whether the AI's aiming reaches the turret lock's lead, and what spin a unit let go keeps until the Wizard writes one.
- [ ] [M13] What a node reaching its last damage stage takes out of the load (the engine: its own weight and armour).

## C03 Mission 02, *The Convoy*

[playthrough-feedback](playthrough-feedback/feedback.md), [14-controls](docs/14-controls.md),
[29-weapons](docs/29-weapons.md), [31-packages](docs/31-packages.md), [26-damage](docs/26-damage.md),
[11-effects](docs/11-effects.md), [32-builder](docs/32-builder.md)

What was left on 2026-10-01 of the list made by setting the mission against "Let's Play - Parkan:
Iron Strategy, Part 6" (-yNnsqudMzw) and its bonus "Part 6.5" (9SBZOCWv_vE). Lines of that list
already queued in their own areas: Enemy 1's first builds and function 41 (*AI, scripts, packages
and economy*); the raid's launch, 414 m in the engine against the recording's 365–390, with the
fire line's remainders, and a unit of more than one part in a panel's view (*Turrets, weapons and
camera*); a save's own bytes (*Mission 03*); C03 M01's lava, the lit colour's remainders with the
Outpost's lavender face, and a captured building's emblem (*Sky and rendering*); the Large
Factory's masts (*Effects and sound*).

**Where the engine is weaker than the game**

- [ ] **The quick save and load in the window.** F7 keeps the play in memory and F8 puts it back,
  over the *MISSION FAILED* panel too. The play's side is pinned by three install tests and a
  headless run; what the window puts back beside it — the placed objects, the own view, the
  cockpit — has only been compiled, never run. To check by hand: save, let a factory make a unit,
  load, and look for a stale model or a stale name on a panel
  ([14-controls](docs/14-controls.md#quick-save-and-quick-load--read-and-seen)).
- [ ] **A unit that stands still on its way to a refit.** *Seen once on the engine*, 2026-10-01:
  Enemy 2's SWW-5, sent to refit at Enemy 1's dock across the map (the Large Factory at (1064,
  670)), stops at (727, 950) and stands there for minutes with its task still walking. Not looked
  into: whether its route fails, or it waits on a dock another unit holds
  ([31-packages](docs/31-packages.md)).

**Seen in the recordings and not yet set against the engine**

- [ ] **The second raid's warning in Part 6.5.** Part 6 shows I02 at 22:32; Part 6.5 never does. A
  dead unit is given no order, so the line stays silent if unit 14, the LWW-3, is lost before its
  timer; but no frame of Part 6.5 names an LWW-3 (the target panel's name bar, read every 2 s), so
  whether it was lost first is not found.
- [ ] **The Light Tower's shield under fire.** A large translucent green dome stands over the
  tower's top while it is attacked (48:30–49:20). `mtow02` carries a shield generator, 11,000 × 6,
  and the engine plays its flash, a type-9 dome whose pole now points at the hit. The two were not
  set side by side: two tries at standing the hero before Enemy 2's tower (`--face mtow02.dat,90
  --hold SCAN_LMOUSE`, seen with `--look`) showed no flash, and whether its shots were landing was
  not checked ([26-damage](docs/26-damage.md#what-a-shield-hit-draws--read-and-measured)).
- [ ] **The sparkles about a gun and what it hits.** White four-point stars about the muzzle and
  the unit being shot (48:05, 48:40): the gun flashes' and hits' own sprites, `DUST.0`'s star
  cells, worn by `splash5`, `plas_shoot`, `taser_shoot` and five explosions' sprites — not the
  weather, as the first pass had it. Not set against the engine's
  ([11-effects](docs/11-effects.md)).
- [ ] **The effect light on the ground.** The recording's orange pool about a warbot as it is hit
  (11:14) was not rendered from the same place in the engine; the 100 to 200 m reach of a large
  blast's light is by the read, not by a frame ([11-effects](docs/11-effects.md)).
- [ ] **The builder's upgrade from close by.** The dome's texture and shape match (33:32, 33:50);
  an upgrade itself — the old building under its dome from 26 to 31 s in, and the new one coming
  up under the dome played backward at 50 s — was not run in the engine beside 33:30–33:50
  ([32-builder](docs/32-builder.md#upgrading-a-building--read)).
- [ ] **Campaign 02's Outpost facade.** Bright yellow-green in the recording ("Let's Play -
  Parkan: Iron Strategy, Part 3", UfyUzq8k2kY, 1:22) and a dim olive with an orange block in the
  engine; its emblem matches. Not looked at again since the frame's colours were put right on
  2026-10-01.

## Not looked at at all

- [ ] **The 61 stand-ins refiled on 2026-09-21.** From Engine M6 on, `engine/README.md` filed new stand-ins under *Read since the stand-in was written*, and this file's rule leaves those rows out as waiting only on the engine, so most never reached it: of a sample of 24 of their subjects, **19 have no line here**. They are back in the *Stand-ins* table (`78e745f`), and each wants either a line in the area it belongs to or a check that it is answered.
- [ ] The network protocol.
