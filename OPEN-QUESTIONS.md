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

- [ ] What lies below the dome's rim, and which views carry mode 1. *Narrowed 2026-09-20* out of the
  closed line on the sky's pass descriptor
  ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#sky-and-rendering)): the sky's fourth draw is **read** — four pre-transformed screen vertices from the viewport
  rectangle (`0x1007a1a6`), flags `0xc`, depth test and write both off, and a material block (sky
  `+0x484`) whose only written fields in the whole module are the ambient alpha at `+0x20`, two zero
  texture fields and a format id, so the only colour it can carry is the **scene colour**. It is
  **skipped whenever the view's mode (slot 24, `0x10083010`) is 1** (`0x1007a325`), and which views
  carry which mode is not read; that the world's view is mode 1 is *inferred*, not established. What
  lies below the rim was again not reached ([10-sky](docs/10-sky.md#the-dome)).
- [ ] What the colour filter's slot 5 computes. *Narrowed 2026-09-20*: it is **not the shader**, as this
  page said. The object reaches the bodies from `CAtmosphere`'s `+0x16c` (`0x1006fde6`) and the item
  renderer from the shader component's **interface 4** (`0x10032a75`); its slot 7 returns a block whose
  `+8` bit 0 is the flag, tested at `0x10079786`, `0x1007d923` and `0x1002fe79`, and its slot 5 maps one
  `D3DCOLOR`. The `0xff00ff00` mask names it — the camera's **infrared** (`CMD_CAMERA_INFRARED`,
  `CIS_INFRARED_ON/OFF/INV`, `NightVisionOn`), which is also what the outer view's flag `0x20` turned out
  to be. What slot 5 computes is still unread: the object's vtable is installed outside `Terrain.dll` and
  its class was not found ([10-sky](docs/10-sky.md)).
- [ ] Who sets an object's material track (`ILifeSystem` slot 16), and who calls IAnimation slot 27.
  *Narrowed 2026-10-01* from checking C03 M02 against a recording ("Let's Play - Parkan: Iron
  Strategy, Part 6.5", 9SBZOCWv_vE, 37.4 s and 2:15). *Seen*: the insignia follows the building's
  owner. Enemy 1's Medium Mine wears `B_LBL_01`'s track 1 (cell 6 of `PG27`), and the player's
  Small Bunker wears track 0 (cell 0, the arrow). So something writes the control system's
  `+0x554` for each clan. Not read: what writes it, whether it takes the clan's index or its sign
  (the clan record's `+0x14`, which a single-player game makes the same), and whether a capture
  changes it. The engine draws a building on its owner's track and a unit on track 0, a stand-in
  ([07-objects](docs/07-objects.md)).
- [ ] Whether any caller besides the round's hit test and the collision pass hands a face query a
  triangle mask carrying 2 or 16 — the round builds its filter inline, so enumerating the filter
  constructor's call sites is not a complete enumeration. (~~What reads object face flags 2 and 16~~ —
  narrowed 2026-09-18: a face flag can be read in only three places, all `AniMesh.dll`, whose visitor is
  not exported and has two callers; and the walk-face query reads no triangle flag at all, so the flagged
  floors are the push-out's and not the ground search's. What the two flags **are** is measured through:
  2 is the walkable surface, all 6166 in a level-0 slot and 6100 above the engine's own cos-80° threshold,
  a chosen subset; 16 is the broad face of a door leaf, all 384 vertical on 52 interior nodes
  ([07-objects](docs/07-objects.md)).)
- [ ] What a type-1, 2 or 5 pass does with its items' order. The effect sprites' layer 6 is a type-3
  `CCamDistSortLayerVB`, read to draw far to near (`Terrain.dll:0x1003e090`, `0x1003e1d0`;
  [11-effects](docs/11-effects.md#effect-sprites-are-drawn-far-to-near--read-and-seen)); the see-through
  surfaces' layer 5 is type 1, and group 1's layers 0–4, 8, 10 and 13 are type 2 and layer 12 type 5
  ([10-sky](docs/10-sky.md#the-dome)), whose render slots are not read. Narrowed 2026-09-30 out of
  [07-objects](docs/07-objects.md)'s note on the queue's sort types.
- [ ] **How the weather is drawn.** Raised 2026-09-30 from checking C03 M01 against a recording
  ("Let's Play - Parkan: Iron Strategy, Part 5", PfAg6zSe-yM). The keyframes' opcodes start and stop
  rain, snow and lightning, and `Terrain.dll`'s factory makes a RAIN, a SNOW and a LIGHTNING object
  for them (*read*); this map snows all day with `DUST_ADD` (`DUST.0`, cell 0, blend 2) in its snow
  slot, and lightning runs 00:01–23:58. The recording shows red dust falling through every frame of
  the briefing and of play (0:55 on) and a lightning bolt at 1:55. Not read: what the three objects
  draw — how many particles, in what box about the camera, how fast and how large they fall, and how
  a bolt is made, where and how often. The engine draws no weather ([10-sky](docs/10-sky.md)).

## Effects and sound

[11-effects](docs/11-effects.md#not-resolved)

- [ ] [M4] When a stream emits its first particle, and where burst and stream particles go.
- [ ] **What slows an ambient stream.** Raised 2026-09-30 from checking C03 M01 against a recording
  ("Let's Play - Parkan: Iron Strategy, Part 5", PfAg6zSe-yM, 4:19.4–4:24.4). A stream's clock is
  read as the seconds since its instance started (`Effect.dll:0x1000846c`, the context's first word),
  it emits every interval and catches up (`0x10011bf5`–`0x1001201c`, `last += interval`), and a
  particle ages one ring slot an emission (`0x1001209e`): `tree_light_33a`'s `fire_smoke`, a ring of
  20 at 0.08 s, lives 1.6 s and rises 100 m. The recording's volcano smoke rises at about 0.63 of
  that against its own puffs' width, and its flame flickers at 5.3 Hz with almost nothing at the
  interval's 12.5 Hz. Not read: who hands the manager its time, and whether an ambient effect — a
  load group's on a building, tree or stone, or a lode's plume — is updated on a slower clock. The
  engine runs those streams at half pace and every other at the read one (engine README,
  *Stand-ins*). Seen alongside: the recording's plume is distinct billows — the spawn's read
  jitter of each particle's far ends, which the engine now draws — over a flame about twice
  the engine's, whose brightness varies a third as much (not explained).
- [ ] How Direct3D Sound places a sound between the speakers. (~~[M5] How a sound falls off between
  its near and far distances~~ — this half was **stale**: the game takes the Direct3D Sound path
  rather than its own mixer, so the law is DirectSound's — whole within the near distance, then
  *min* ÷ (*min* + *R* × (*d* − *min*)) for the listener's rolloff *R* of 1, and no further past the
  far distance. Re-checked by the coordinator 2026-09-18: `services.dll:0x10011914` pushes flags
  `0x120` into `niCreate3DSound`, so bit 16 is clear and each buffer goes to DirectSound whole
  through `SetAllParameters`
  ([11-effects](docs/11-effects.md#how-a-sound-is-heard--read-and-measured)). What is left is only
  how DirectSound itself pans.)
- [ ] **Which draw reaches a unit's own manager**, the one an agent makes at `AniMesh.dll:0x100033ba`
  and a controller drives at its `+0x3c`. The remainder, narrowed, of the header flag `0x800` line
  closed 2026-09-19 ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#effects-and-sound),
  [11-effects](docs/11-effects.md)).

## Motion, ground and controls

[24-motion](docs/24-motion.md#not-established), [13-control](docs/13-control.md#not-established),
[14-controls](docs/14-controls.md)

- [ ] [M14] The walker's local path and its obstacle contours: how it goes round a tree's or a stone's hole, whether it widens it by the unit's size, how a walker in one walks out, and what it does with a goal in one; the sub-areals' shapes and whether the search measures one from its centre; whether every scenery object reaches the areal map, and the box of a mesh of several parts; how it drops the points a unit has passed (`MWalker::ClearMoverReachedPoint`); how a unit's place comes onto a building's map object and which vertex the search starts from; who calls `MHallWay` slot 11; a hall-way vertex's size gate (the unit's `+0x960`, the record's `+0x28`); the link flags `0x10000` and `0x20000`; how a walker goes to the point it finds off a non-walkable areal, and what it does when its search fails; and how a walk to a door gets past the building's own walls, which cut no areal (the engine: a door more than 20 over the ground under it is passed over, and a straight line into a wall goes round the building's ground contour) ([24-motion](docs/24-motion.md#not-established)).
- [ ] **C02 M03's Small Bunker (`l_bunk1`) stops a small walker short of its pod.** Raised
  2026-09-20; its wheeled half closed 2026-09-30 (the hull's righting, in
  [COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#motion-ground-and-controls)). With the up-pass bound
  at r a walker took it in 20 s; at r₂, now read, `21swlk1` stalls 37.8 m short of the pod and
  `22swlk1` never comes within 90 m, while the SSW-X walker takes it in 25 s (*measured*
  2026-09-30, each sent from 60 m north with the bunker's guns off). It is **not** the contact's
  mechanism — no lift over 0.5 and no segment stop at the spot, so it is the AI's own legs — and
  walkers right toward the world's up, so the hull's tilt does not reach it. It wants the inside of
  a building looked at as a whole: the push-out keeps any face under a mover that is not flagged 2,
  and docs/07 measures **1802 faces** of the first variant's level-0 slots pointing within 10° of up
  and carrying no flag at all ([24-motion](docs/24-motion.md#collision-between-objects--read),
  [07-objects](docs/07-objects.md#the-flags-word)).
- [ ] **What keeps a walker upright on a building's floor under a low ceiling.** Raised
  2026-09-30 from checking C02 M03 against a recording. C02 M03's Small Warehouse is 6.3–7.5 m
  high inside, and the medium walker `22mwlk1` placed on its floor has a 5.87 m agent sphere
  0.41 above its origin: the ceiling's faces (interior node `i09`, 42.88–44.08) press it 2.85 m
  down. *Read*: the sphere the pair hands the push-out is that agent sphere (interface `0x18`
  slot 9 with 2, `Control.dll:0x1001fef5`, into `+0x38`–`+0x50` at `0x1001ff36`); a machine
  standing on a building takes a down push whole (`0x1000c9eb`); the push lands after the
  ground contact. So drawn after the push the walker stands with its hull at the floor, and the
  recording shows it upright in **every** frame at 60 fps ("Let's Play - Parkan: Iron Strategy,
  Part 4", 1:43, 15:13–15:15). Not read: where the frame is drawn among move, pass, contact
  and push, which message runs the sphere's refresh (`0x1001fec0`, case 0 of `0x1001f548`), and
  whether anything keeps the pair from pushing. The engine draws the body where the contact
  last held it and keeps the push in the simulation, which three install tests need to carry
  units down a ramp into a building ([24-motion](docs/24-motion.md#not-established)).
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
- [ ] **What the eye the sprite draw is handed has been transformed by.** Raised 2026-09-21.
  The emitter loop's context carries the instance's own matrix at `+8` and a point at `+0x48`
  (`Effect.dll:0x10008050`, `0x100080aa`), and the draw takes that point for the eye
  (`0x100093fc`). The sprite's position channel is in the frame's space, so the eye must be
  too, and `[0x1001e0bc]` slot `0x68` — called just before with the matrix in `ecx` and the
  point in `edx` (`0x100080da`) — is where it would be put there. That slot lives in the maths
  interface, outside `Effect.dll`, and is not read; the engine assumes the transform
  ([11-effects](docs/11-effects.md#a-sprite-is-drawn-through-its-frame--read)).
- [ ] **What separates control message 7's argument 0 from 1**, which `Control.dll` treats alike;
  argument 2 is read, the object simulated elsewhere. No shipped section-5 record tests condition
  byte 15, so nothing an artist wrote turns on it. The remainder of the `.ctl` values line closed
  2026-09-20 ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#motion-ground-and-controls),
  [13-control](docs/13-control.md#control-message-7-says-who-simulates-the-object--read)).

## Turrets, weapons and camera

[29-weapons](docs/29-weapons.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[28-chassis](docs/28-chassis.md#not-established), [25-sensors](docs/25-sensors.md#not-established)

- [ ] [M5] How the HUD draws the aim point, the guns and the player's target; what plays `TARGET_READY` and `TARGET_ZOOM`; and the unit record's `+0x98`. ~~Its `+0x94`~~ — **read**, noticed 2026-09-22 by the coordinator: docs/39 had it all along as the radius the sphere interface `0x20` slot 3 answers as the record is bound, the unit's node sphere (`0x1007e5f2`–`0x1007e60e`), 11.84 on the L-2f, which leaving and the game frame read. The right button's pick still takes it as 0, filed under the README's "Read since" ([25-sensors](docs/25-sensors.md#not-established)).
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
- [ ] **What killed the hero in C02 M04's valley Light Tower.** Raised 2026-09-30 from checking
  C02 M04 against a recording ("Let's Play - Parkan: Iron Strategy, Part 4", Qqs8_i9IeUU, 21:53.5).
  The hero, its own panel's life arcs full, walks up to the pod of `mtow01` at (591, 1502), whose
  turret it has shot off — the target panel shows the turret red from 21:27 and the mast sunk by
  21:36 — and whose shield ring is red. One white flash, the view left on a white glare, and the
  mission fails; on the reload the same tower is captured whole (26:04). *Seen*, not read: whether
  the tower died then and its death killed who stood inside, or another gun reached the room.
  Nothing read kills a unit inside a building — a building whose node 0 dies becomes a shell with
  its model standing ([26-damage](docs/26-damage.md)) — and the engine leaves a hero standing
  there unharmed. *Seen a second time* 2026-10-01 on C03 M02 ("Let's Play - Parkan: Iron
  Strategy, Part 6", -yNnsqudMzw, 13:54.2). The player is at the Small Bunker's guns, the hero in
  its pod room, when the raider's winged SSM lands on the bunker (60,000 a round, a 45 m blast;
  in the engine's run it burst 17 m from the pod). One white flash, and the mission fails over
  the fallen hero's camera. So a blast at least that large reaches the room under a building.
  Still not read: the blast's own test against a hero indoors.
- [ ] **Whether a blast spares the object that fired it.** The remainder of the winged-SSM line
  closed 2026-10-01 ([COMPLETED-QUESTIONS](COMPLETED-QUESTIONS.md#turrets-weapons-and-camera)).
  [26-damage](docs/26-damage.md) reads that a hit does nothing if it is "the target's own", with no
  address for the test, and the engine's blast reaches the round's owner as it does any object
  whose bounds meet it. On C03 M02 that matters once the raider closes on the bunker: a winged SSM
  it fires from about 25 m lands within its own 45 m blast and kills it. The recording ends that
  raid sooner, on the first missile. Not read: the test, which sits somewhere between the
  damage stage that builds the hits (`Control.dll:0x100113e0`–`0x10011900`) and the queue's
  application (`0x10012ce0`).

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

- [ ] [M12] The game menu's save page — its draw (`iron3d.dll:0x10066d50`), its six slots' records and what the level's `0x100a1590` writes under `/save/` — and the shell that *Load game* and *Quit game* hand the mission to (exit codes 1 and 3). The engine takes *Save game* and *Load game* and does nothing, and *Quit game* closes the window; Mission 03 disables the first two ([39-boarding](docs/39-boarding.md#the-game-menu--read)).
- [ ] What the quick save and the quick load do: `CMD_QUICK_SAVE` on F7 and `CMD_QUICK_LOAD` on F8
  (`ui_other.man`, `addition.man`). Raised 2026-09-30 from checking C03 M01 against a recording
  ("Let's Play - Parkan: Iron Strategy, Part 5", PfAg6zSe-yM, 3:09): in play the message box shows
  *"from: System / Game saved..."* (`iron3d.dll` string 6246, beside 6245 *"Quick Save"*) and play
  goes on. Not read: where the save goes — one of the save page's six slots or a slot of its own, and
  its file under `/save/` — and what F8 loads. The engine binds neither
  ([14-controls](docs/14-controls.md), [17-saves](docs/17-saves.md)).
- [ ] A *Tiny Tower*'s battery of capacity −1: `Power::load` drops a negative battery, so the engine's battery arc follows the tower's fitted 31,000, where the game's id 1 answers 1 whenever any capacity is negative (`Control.dll:0x1002b42b`). How the power tick spends a −1 battery on a unit is not read ([41-commander](docs/41-commander.md#the-box), [23-economy](docs/23-economy.md)).
- [ ] What a building going up is still reached by: whether a blast's hit, which hurts every object whose bounds reach it and then each node by its sphere (`0x10010030`), passes a node hidden by action 1, which is not a destroyed node; and whether Behavior's hall-way searches pass over a building in the second between its showing at 40 s and its sphere's end at 41 s ([26-damage](docs/26-damage.md#not-established)).
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
- [ ] Why the hero's panel dims in the pod room, and why the chamber's glow reads greyer than the recording's (the dawn scene colour is the guess).
- [ ] Message 16, the helicopter in route 2, has no test.

**Movement**

- [ ] [M13] Whether the AI's aiming reaches the turret lock's lead, and what spin a unit let go keeps until the Wizard writes one.
- [ ] [M13] What a node reaching its last damage stage takes out of the load (the engine: its own weight and armour).

## Not looked at at all

- [ ] **The 61 stand-ins refiled on 2026-09-21.** From Engine M6 on, `engine/README.md` filed new stand-ins under *Read since the stand-in was written*, and this file's rule leaves those rows out as waiting only on the engine, so most never reached it: of a sample of 24 of their subjects, **19 have no line here**. They are back in the *Stand-ins* table (`78e745f`), and each wants either a line in the area it belongs to or a check that it is answered.
- [ ] The network protocol.
