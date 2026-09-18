# Open questions still to research

Compiled on 2026-09-14 against `a5e0188` from every doc's *Not established*,
*Not resolved* and *What is not read here* section, [TODO.md](TODO.md),
[docs/06-open-questions.md](docs/06-open-questions.md) and the unknowns in
[engine/README.md](engine/README.md#stand-ins). Each doc stays the source of
truth; this is the queue.

**[M1]–[M5]** marks a question the engine currently answers with a stand-in,
by milestone. Those come first. Left out: behaviour already read and waiting
only on the engine (the README's *Read since the stand-in was written*), and
engineering such as terrain culling or drawing the sky's textures.

A round of twelve was worked on **2026-09-18**, four areas of three. Seven
closed outright, four are answered as far as the shipped files allow — each
with a control showing the search that found nothing would have found
something — and one narrowed. A thirteenth, contact flag 2, closed as a side
effect. The engine changed in two places: a tracked warbot's belts now lie
along the ground under them, and seven of the formula evaluator's thirteen
operators were wrong and are fixed. Three doc corrections came out of it —
the `.fml` operator table's address, the Large builder's module socket, and
the eight `b?` prefixes, which are rounds and never were buildings — and two
premises in this file were wrong and are rewritten above: the six turrets'
gate is measured over 17 player trees, not 11, and the "five labels and the
label inside a block" is five plus one, the one being no anomaly.

A second round of twelve, the same four areas of three, was worked the same
day. **All twelve closed**, one of them because an earlier round had already
answered it and this file was never ticked. Three are negatives carrying their
controls: no module outside `Terrain.dll` reads `ForceSWFog`, not one of the
275882 shipped land faces carries either bit the ground search excludes, and
nothing reads an `.exp`'s two 1.0 floats. The engine changed in five places,
and elsewhere it was already right, which the docs now record as read rather
than assumed: a tree and a stone carry node life like any other
agent and are built at their placement scale, so a stone scaled 21 holds
billions rather than thousands; a vital node's death now takes node 0 with it;
a mine's lode and its total meet half way, so a lode of 600 yields 350; a chord
falls through to the plain row by the read rule rather than by luck; and a
walker's feet are laid along the ground by the pose its state ends in.

That last one **corrects a premise of the round before it**, and both entries
are rewritten where they stand. `CONTACT_PLACE` is not only the authored flag 2 on twelve
belt contacts: flag `0x20` makes the engine work the flag out afresh each time
a machine takes a state, from the contact's own axis in the state's end pose,
and it comes out set on **2229 of the install's 2634 contacts** rather than
twelve. Every walker's feet conform to the terrain, not just the tracked
chassis' belts. Two smaller corrections came out of the round: the contact pass
runs at each state, not at load ([13-control](docs/13-control.md)), and the
derivation behind "a clan orders a bot it has no mind for" was reading `op5` as
`==` when it is `!=` ([23-economy](docs/23-economy.md)).

## Sky and rendering

[10-sky](docs/10-sky.md#not-resolved), [02-texm](docs/02-texm.md),
[07-objects](docs/07-objects.md), [06](docs/06-open-questions.md)

- [ ] [M1] Where the sun object's two lights point: nothing writes the light record's `+0x24`.
- [ ] What `CSun` does with a body's lifetime, for a body that started before the clock's start.
- [ ] [M1] How the dome escapes the far plane and the fog (render layer 1's projection, fog defaults at `0x1003d9f2`), and what lies below its rim.
- [x] ~~[M1] Whether `ForceSWFog` does anything outside `Terrain.dll`~~ — closed 2026-09-18: **no
  module reads it**, and it is entry 0 of `Terrain.dll`'s 36-setting page. A setting is read as
  `[index * 4 + 0x100a6cac]`, an address the loader relocates, so the readers are countable rather than
  searchable: 41 relocation sites name the page, all 41 carry a constant index, none is 0, and between
  them they cover 32 of the 36 — entries 1 `LightingOn` and 2 `SpecularsOn` either side of it are the
  control. Across modules the page is interface `0x1e` of `World3D.dll`'s settings registry, keyed
  `(index << 16) | id`, and the game makes exactly one cross-module settings read in its life:
  `RobotBestLOD`, from `AniMesh.dll`. `Ngi32.dll`, which would have to implement a software fog, does
  not import the registry at all. The engine's per-fragment fog stands ([10-sky](docs/10-sky.md)).
- [ ] [M1] The fog heading's world axis: that the camera matrix's first column is the view direction is a guess.
- [x] ~~[M1] Whether water draws see-through~~ — closed 2026-09-18: **opaque**, and the engine was
  already right. A mesh batch takes its blend mode from its material's flags byte through `CShade`'s
  translate table, but the ground does not: the ground surface takes `CShade+0xbf4`, which its init
  fills from translate index 0 — mode 0, `ONE`/`ZERO`, no blend and no alpha test — and the
  `REFLECTION_SHIFTED` water surface writes mode 0 outright. Measured over all 275882 faces of the 33
  maps, `WATER` (1006 faces on 7 maps), `WATER_BOT` (2015) and every `L*` ground material carry flags
  byte 0. The control is the 2624 `ENV_NLAVA` faces, whose flags byte 4 *does* ask for
  `SRCALPHA`/`INVSRCALPHA`: they draw opaque anyway, because the ground draw never looks
  ([03-terrain](docs/03-terrain.md)).
- [x] ~~[M2] Whether a blended material writes depth, and the alpha test's reference value~~ — this
  line was **stale**: an earlier round answered it (`27172f2`), a blended batch drops only alpha 0 and
  writes depth, and the engine implements it. Re-read independently 2026-09-18 and it holds:
  `ALPHAREF` is 1 and `ALPHAFUNC` is `GREATEREQUAL`, written by two agreeing writers, and the six
  blend-mode records are 30 `{state, value}` pairs over five states, among which neither `ALPHAREF`
  nor `ZWRITEENABLE` appears ([07-objects](docs/07-objects.md)).
- [ ] [M3] What draw layers 10 and 9, which a fifth slot is filed under, do (`Terrain.dll:0x1004553b`); and `CShade` slot 15.
- [ ] The sun sprite's extent unit, camera slot 27, and shader slot 5's colour filter and flag bit 0.
- [ ] Texture header bit `0x4000000`: which batch of exports it marks, and whether the exporter is
  identifiable. (~~and load flag `0x200000`'s effect~~ — closed 2026-09-18: it is the texture stage.
  `Ngi32.dll`'s surface description fills `DDSURFACEDESC2 +0x78`, `dwTextureStage`, from bit 21 of the
  load flags, shifted rather than masked, which is why an immediate scan never found it; `ENV_STARS` is
  the one material that sets it, so exactly one surface in the game is made for stage 1
  ([02-texm](docs/02-texm.md)). The header bit is **not a property of the picture**: format, mips, alpha
  and wearer all fail to separate the 81, and the directory being in offset order they are members 66 to
  154 of 393 — one run of insertions, eight unmarked inside it and none outside. `+0x14` is the
  exporter's flags word and the loader reads only bits 24 and 25.)
- [ ] Who sets an object's material track (`ILifeSystem` slot 16), and who calls IAnimation slot 27.
- [ ] Whether any caller besides the round's hit test and the collision pass hands a face query a
  triangle mask carrying 2 or 16 — the round builds its filter inline, so enumerating the filter
  constructor's call sites is not a complete enumeration. (~~What reads object face flags 2 and 16~~ —
  narrowed 2026-09-18: a face flag can be read in only three places, all `AniMesh.dll`, whose visitor is
  not exported and has two callers; and the walk-face query reads no triangle flag at all, so the flagged
  floors are the push-out's and not the ground search's. What the two flags **are** is measured through:
  2 is the walkable surface, all 6166 in a level-0 slot and 6100 above the engine's own cos-80° threshold,
  a chosen subset; 16 is the broad face of a door leaf, all 384 vertical on 52 interior nodes
  ([07-objects](docs/07-objects.md)).)
- [x] ~~What IAnimation node mask bit `0x10` does~~ — closed 2026-09-18: it lays the node along the
  ground under it. Slot 8 mirrors the bit into the node record's byte `+0x113` — nothing tests the mask
  word against `0x10`, so the byte is the whole path — and the pose walk then turns the node's world
  matrix by the rotation slot 31 left on it and writes the translation back, so the node tilts where it
  stands. Set by the ground contact on a `CONTACT_PLACE` contact's carrier. ~~Set on twelve nodes in the
  game: the belts of the three tracked chassis.~~ **Corrected later the same day**: `CONTACT_PLACE` is
  worked out from the state's end pose as well as read from the file, so it reaches 2229 of the 2634
  contacts and every walker's feet, not twelve belts (see *Contact record flag `0x20`* below).
  Implemented ([28-chassis](docs/28-chassis.md), [24-motion](docs/24-motion.md)).

## Effects and sound

[11-effects](docs/11-effects.md#not-resolved)

- [ ] [M4] The rest of the emitter floats:
  - [ ] whether a fade scales alpha or colour
  - [ ] the per-axis exponent triples
  - [ ] the sprite powers at `+64` and `+124`
  - [ ] a bolt's widths at `+24` and `+28`
- [ ] [M4] How the shade lights a type-1 light (falloff, attenuation) and the manager flags `0x80000000` and `0x20000000`.
- [ ] [M4] Header flag `0x800` and the rest of the draw path:
  - [ ] who passes the draw-pass argument flag `0x800` waits for
  - [ ] who sets the target point a bolt starts from
  - [ ] draw flag 4 (header `0x2000`)
  - [ ] header flag `0x10000`
- [ ] The four settings groups, the group floats `+0x1084` and `+0x1294`, and the page's `+0x14a4`.
- [ ] [M4] The effect manager's random generator and jitter, the owner values of time modes 5–15, and a phase's animated frames.
- [ ] [M4] When a stream emits its first particle, and where burst and stream particles go.
- [ ] [M4] How often an effect tests its point's view, and what that ray meets.
- [ ] [M4] What a building answers for a strike's material, and a node's wear base.
- [ ] [M5] How a sound falls off between its near and far distances, and how it is panned.

## Motion, ground and controls

[24-motion](docs/24-motion.md#not-established), [13-control](docs/13-control.md#not-established),
[14-controls](docs/14-controls.md)

- [ ] [M3] How interface `0x25` slot 3 turns level-0 triangles into a push, and what slot 2 does with its 0.5.
- [ ] [M3] Which scene nodes are types 1 and 3, which decides whether bridges and buildings are ground; plus a machine's type-3 parent and what message `0x201` returns.
- [ ] [M3] The ground contact's timing and dt, the pose its contact points use, the second sphere's radius r₂, and what lifts a sphere with no face under it.
- [x] ~~Which `Land.msh` faces carry the world bit `0x8` and class bit 8 that the ground search
  excludes~~ — closed 2026-09-18: **none of them do**. The missing step was naming the fields. The
  ground search excludes world `0x208` and class bit 8; `Terrain.dll` folds each pair into one
  landscape mask, and the landscape's mask is the file's face record unconverted — the flags word low,
  the surface word high. So the two bits are the flags word's `0x20` and the surface word's `0x04`, and
  **0 of 275882 faces across all 33 maps** carry either, at both levels of detail. The reading is
  cross-checked on two bits of the same word that faces *do* carry: landscape `0x2000` is the flags
  word's `0x2000` on exactly the 6102 bed faces, and landscape `0x20000` the surface word's `0x02` on
  exactly the 3630 water faces — which is also the control. The engine's ground index was already
  exactly this filter. What would ever set them is not established
  ([24-motion](docs/24-motion.md)).
- [x] ~~Contact record flag `0x20`~~ — closed 2026-09-18, and it **corrects the flag-2 entry below**.
  `0x20` does not mean "place": taking a state poses the object at the state's end, asks each contact
  point for its own axis, and then *sets* `CONTACT_PLACE` where that axis stands up — z above 0 and
  1 − z below 0.05 — and clears it otherwise. So the flag is worked out afresh **each time a machine
  takes a state**, not read from the file. Measured over all 2634 contacts: 2410 carry `0x20` and every
  one of them is a **foot** — nothing with a wheel or a belt has it, and none of the 2410 is authored
  with flag 2. Posed at each state's last frame the axis stands up on 2217 of the 2410, so **2229
  contacts in the game lay their node along the ground**, not the twelve this file had counted. A
  walker's feet conform to the terrain in every state whose animation ends with the foot flat (20 of the
  hero's 210 states withhold it); the tracked belts are simply the case with nothing to work out. All
  2410 axes are unit length, which is why the game's unnormalised compare against 1 is a cosine. Read,
  measured, implemented and tested ([28-chassis](docs/28-chassis.md), [24-motion](docs/24-motion.md)).
  (~~flags 2 (slot `0x7c`)~~ — closed 2026-09-18: flag 2 is `CONTACT_PLACE` as authored, on 12 of the
  2634 contacts, four each on the three tracked chassis, every contact of the wheeled chassis being
  `0x5`. Right about the flag; wrong that those twelve were all that place.)
- [ ] [M3] How often `World3D.dll`'s input update runs (it paces the cruise ramp), and which screen states set the 0.5 mouse sensitivity.
- [ ] [M3] What handlers do when an active row runs again each update, and who calls the second walk/turn ramp's setter (slot 11).
- [ ] [M3] A state's use count `+0x94`, the state a machine starts in, the game's jitter random source, and a controller's request code before any is sent.
- [ ] [M3] The vector that righting bits `0x30` stand the hull toward (`+0x348`, no writer found).
- [ ] [M3] Which way across a slope the mode-2 brake acts, and which way a positive lean tips the model.
- [x] ~~[M3] A chord with no row of its own, such as Shift+W~~ — closed 2026-09-18: **the plain row**,
  unless another row of the same key claims that modifier. There is no best match and no search. A pass
  after loading appends each modifier's scan code to every plain row of any table with the same key; a
  key event then activates a modified row only while its modifier is held, and a plain row only while
  **none** of its listed modifiers is. Measured over the three tables' 116 rows: `SCAN_LSHIFT` is the
  only modifier any of them uses, on 4 rows each, and the pass writes exactly two exclusion entries per
  table — the plain mouse X and mouse Y rows. Every other plain row's list is empty, so Shift+W walks.
  The engine's stand-in agreed with the rule on all 116 shipped rows; it now applies the rule
  ([14-controls](docs/14-controls.md)).
- [ ] What behaviour flag `0x800` changes besides clearing the walker.
- [ ] [M14] The walker's local path and its obstacle contours: how it goes round a tree's or a stone's hole, whether it widens it by the unit's size, how a walker in one walks out, and what it does with a goal in one; the sub-areals' shapes and whether the search measures one from its centre; whether every scenery object reaches the areal map, and the box of a mesh of several parts; how it drops the points a unit has passed (`MWalker::ClearMoverReachedPoint`); how a unit's place comes onto a building's map object and which vertex the search starts from; who calls `MHallWay` slot 11; a hall-way vertex's size gate (the unit's `+0x960`, the record's `+0x28`); the link flags `0x10000` and `0x20000`; how a walker goes to the point it finds off a non-walkable areal, and what it does when its search fails; and how a walk to a door gets past the building's own walls, which cut no areal (the engine: a door more than 20 over the ground under it is passed over, and a straight line into a wall goes round the building's ground contour) ([24-motion](docs/24-motion.md#not-established)).
- [ ] The remaining `.ctl` values:
  - [ ] class 3's value 0, the camera's values 3–5, and the hero's arms' values 1 and 4
  - [ ] the section-5 record's int 8
  - [ ] control message 7's arguments (byte 15, `+0x618`)
  - [ ] `IDeviceManager` ids 5 and 6

## Turrets, weapons and camera

[29-weapons](docs/29-weapons.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[28-chassis](docs/28-chassis.md#not-established), [25-sensors](docs/25-sensors.md#not-established)

- [ ] [M4] Whether the landscape is among the objects the sight ray walks.
- [ ] [M4] What a falling round's mount solves for with no target, which way its lift turns on a hung turret, and whether a player's turret is in `CIS_MANUALCONTROL`.
- [ ] [M4] Whether an AI-set target survives the player taking over, and what sets the hero's target field (`+0x38`).
- [ ] [M4] How a gun's capacitor refills (the power tick).
- [ ] [M3] How the camera builds its frame when its up is parallel to the look.
- [ ] [M5] How the HUD draws the aim point, the guns and the player's target; what plays `TARGET_READY` and `TARGET_ZOOM`; and the unit record's `+0x94` and `+0x98`.
- [x] ~~`e_gun_bl_03` and `e_gun_tl_02` carry a follower and no gun: are they never fitted, or does it
  pair with a later gun?~~ — closed 2026-09-18: never fitted. Both are on 0 of the 458 assemblies and are
  two of the four `e_gun_*` absent from the 395-entry part list every `.trf` carries and from
  `objects.dlb`; the install names them nowhere but `objects.rlb`. Their controllers hold one channel and
  no components, and their `.cpt` is empty, so there is no muzzle either. Over all 458 assemblies no
  follower is left without a gun ([29-weapons](docs/29-weapons.md)).
- [ ] The AI fight module:
  - [ ] its two aim factors
  - [ ] which gun lends the lead speed
  - [ ] tasks 2, 3 and 5
  - [ ] `MBehaviour+0x614`
  - [ ] the id nibble that frees the winged SSMs
- [ ] An AI turret's state word while it fights (`0x200` or `0x400`).
- [ ] What writes the design row's node field, inside the fits' loop over a part's nodes, and what the
  row's `+0x00` and `+0x24` mean. (~~The Large transport's second slot and the Large builder's module
  socket~~ — closed 2026-09-18: a socket's kind is its **stream-10 label**, not its node name, and the
  gun page is the label's last two letters after `e_gun_`. The Large transport has one socket, a cannon
  one; the Large builder's module is `Base_LU_02` and its cannon `Base_LU_01`, reversing the rule the
  small and medium builders gave. `objects.dlb`'s hanger rows agree on 48 of 55 turrets; its free text
  does not and was the source of "two battle slots". The slot records themselves are the destination
  panel's 64-row lists, a row keeping a mesh node and a research item index
  ([30-turrets](docs/30-turrets.md), [37-designer](docs/37-designer.md)).)
- [x] ~~What stops the player building the six free turrets, and which state bits the design screen
  tests~~ — closed 2026-09-18: the screen tests `RESEARCHED` and `IN_TREE` and **not** `AVAILABLE`, which
  it parses and never reads. Five of the six are out of the tree in all **17** trees a player clan loads —
  not 11; `full.trf` is loaded by nobody and `data.trf` is the player's in nine missions. The hero's
  turret passes both bits in six of the 17 and is stopped one level further out: its page never opens,
  because `r_h` is not a chassis-page prefix at any factory grade. Zero cost is not the gate
  ([30-turrets](docs/30-turrets.md), [38-designs](docs/38-designs.md)).
- [ ] Where the unit constructor's page item names (`+0xc4`) come from.

## Damage, sensors and ownership

[26-damage](docs/26-damage.md#not-established), [25-sensors](docs/25-sensors.md#not-established),
[27-ownership](docs/27-ownership.md#not-established)

- [ ] [M4] The hit test's point-in-triangle test (`0x10011090`) and the landscape's own cell size.
- [x] ~~[M4] Which node flag makes a node vital (is AniMesh query `0xe` the mesh node's flags?)~~ —
  closed 2026-09-18: **yes, and the bit is `0x200`**, which is what the engine already had. Query `0xe`
  reaches the mesh node's own stream-1 record — pointer at the runtime record's `+0x12c`, indexed by its
  `+8` at a stride of 38, the first `uint16` — where query `0xa`, four ids along, answers the *runtime*
  word instead; the life loader tests bit 9 of the former. Measured: 90 of 1845 mesh nodes on 9 of the
  435 meshes, and a coherent 90 — every one a geometry-bearing segment of an articulated limb or body
  chain (the hero's body and legs, the animals' necks, heads, tails and wings, the hero turret's two
  arms), 0 of `fortif.rlb`'s 273, and none on a wheeled or tracked chassis. Named against its neighbour
  as asked: mask `0x10` is broad, 861 of 1845 over all nine archives, where `0x200` is narrow; they
  coincide on 8, each the leaf of a flagged chain. Why the bit falls on those nine models and not on the
  other walkers' legs is not established ([26-damage](docs/26-damage.md)).
- [x] ~~[M4] Whether vegetation and rock carry node life~~ — closed 2026-09-18: **they do**, and the
  engine was wrong. A placed tree or stone goes into the game as agent type 10, and the agent build
  gives every agent a control system and queries its `ILifeSystem` before any branch on the kind; the
  life loader then branches on kind 10 to set the low-life mark at 0.3 of the total rather than 0.2,
  which is only reachable if scenery has a life system at all. Measured: all 81 `STAT` records name a
  `.ndp` and a `.ctl`, 123 node rows, 1 to 1500000 hit points, **none 0**, with `explode_tree` and
  `explode_stone` to play. The control is the same query by tag: 63 of 63 `BTLU`, 146 of 146 `EXTO`,
  **0 of 34 `FORT`**. And scenery is the one thing built at its placement scale, the tick rescaling
  every node's life by the three factors multiplied, so Mission 01's `s_tree_04` at 3 has a trunk of
  81000 rather than 3000 and Mission 02's `s_stone_10` at 21 holds 4630500000. Fixed and tested
  ([26-damage](docs/26-damage.md), [04-missions](docs/04-missions.md)).
- [ ] [M4] Whether a round's ground test strikes the water surface.
- [ ] [M4] What a dead unit leaves (wreck, damage stages), and what `iron3d.dll` does with owner word `0xfffe` (37 compares).
- [x] ~~The `.exp` record's two 1.0 floats, for which no reader was found~~ — closed 2026-09-18 as a
  **negative with its control**: the thread the doc named ends, and nothing reads them. The record is
  792 bytes on all 144, exactly `0x18 + 12 × 64`, so nothing is unparsed. Its bytes reach one pointer,
  the `.exp` cache, whose only fetch is the known one — and no module in the install holds the string
  `.exp` besides. That pointer is stored at offset 0 of the hit record, and the hit's `+0` is
  dereferenced in four functions, each reading the first dword only, the kind. The control is that the
  same enumeration finds four of the six scalars and all twelve names at named addresses: it is not
  blind to two adjacent floats between the radius and the placement word. What they are *for* is left
  as a guess the layout supports and the code does not settle ([26-damage](docs/26-damage.md)).
- [ ] What the player's map and radar display show (`iron3d.dll:0x1003fb90` and `0x10073550`).
- [ ] What moves a SuperAI's attitude from one relation band to another.
- [ ] The game view's states 1, 3, 4, 5 and 6, and what pods open on a generator, mine, storage or Outpost; also what `0x10033e40` refuses on a tower.
- [ ] The other four writers of a unit record's `+0xa2`.

## AI, scripts, packages and economy

[15-behaviour](docs/15-behaviour.md#what-is-not-read-here), [31-packages](docs/31-packages.md#not-established),
[23-economy](docs/23-economy.md#not-established), [32-builder](docs/32-builder.md#not-established)

- [ ] What the two floats an object's strength is made of are (`IControl` property `0x36` and interface `0x204`'s `+4`), and so what a `TAKE_BY_HITS` amount is worth; the problem's action record, and SuperAI `+0x40c`. The strength formula and the helper that sums it over a radius — once read as a distance — are now read.
- [ ] A problem's two raise numbers, which handler runs when, and who writes `dCurrentProblem` and `dCurrentSender`.
- [x] ~~Channel 2 of the message callback (function 57), and the count function 69 stores~~ — closed
  2026-09-18. **Channel 2 runs a line of the mission's own `object script` block** in `mission.cfg`:
  the callback takes the channel first, function 57 passes 2, and that arm formats its one value into
  the key `script%d`, looks it up and dispatches the first token against a ten-entry table which is
  **the game's debug console**, names and help text intact — `truth kill bkill cls summon ? create
  delete bcreate death`. Measured: 6 of the 29 shipped `mission.cfg` files carry a non-empty block, 20
  lines between them, three of the ten names used; 14 of 14 calls of function 57 name a line their own
  mission declares and 0 of 14 one it has not got. Control: 244 calls on channel 0. **Function 69's
  count is the AI design pick's spread**: the pick scores every design, sorts it best first, and then
  takes not the best but the candidate at a random index within that many of the top. All 7 call sites
  set it in `Init` from `fDifficulty` — an easy game makes the enemy clan build worse designs. The
  engine answers 0 to both, which is indistinguishable here: 0 of the 21 call sites names a destination
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~Whether any script depends on a constant landing inside a false block~~ — closed 2026-09-18: no,
  and the executor says why — the constant arm is the only one of the eight that does not test the
  condition byte first. All 63 in-block constants write one of five scratch variables; over the 121
  (constant, enclosing `if`) pairs, 88 are overwritten before any read on every path and the other 33
  reach the end of their handler, where every engine entry point writes before reading. None of the five
  names appears in `ai.dll`, so the engine cannot see them either
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~Five labels no goto aims at, and the label that sits inside a block~~ — closed 2026-09-18: five
  plus one, and **the one is not an anomaly**. The five are one label five times over —
  `PBM_BUILDING_PROTECT_Continue` node 19 of 20 — left behind because those five handlers write both
  exits as `return` and hold no goto at all; the same handler ships in two other scripts at 19 nodes with
  no label. The sixth, `c5m1p:PBM_BASE_DEFENCE_Start:9`, **is** aimed at, by the goto at node 3, and sits
  inside a block only because the handler's whole body is one `if`
  ([15-behaviour](docs/15-behaviour.md)).
- [x] ~~The `.fml` operators the corpus never uses~~ — closed 2026-09-18: the corpus uses **3 of 13**,
  `+`, `-` and `*`. All thirteen arms are now read rather than named, and seven were implemented wrongly
  here: `/` answers 0 when the divisor is 0 rather than dividing, `&`/`|`/`!` truncate through `_ftol`,
  and `N`/`S`/`B` are a clamped ramp, the positive part and a strictly-positive test. Fixed. The doc's
  table address was the dead copy; the live one is `0x10047c70` with its count at `0x10047c68`
  ([15-behaviour](docs/15-behaviour.md)).
- [ ] A fire-control request's 0.5, and what sets `+0x5c` and `+0x60` to lock a unit's fire mode.
- [ ] Who sends `MBehaviour` messages `0x19` and `0x1a` (the retaliations).
- [x] ~~Whether a clan's AI re-orders a build refused for want of a mind~~ — closed 2026-09-18: **it
  drops it, at every level**, and the page's derived claim was backwards. `op5` is `!=`, not `==`, so
  all 9 `ORDER_BUILDING_CONSTRUCT` sites — one per script, every one in `PBM_ROBOT_NEEDED_Start` — mark
  the problem *solved* and return when the order was **not** taken, and nothing in the handler orders
  again. 8 of the 9 never try without a mind at all, opening on `if dFreeMindNumber <= 0`. Because the
  problem is marked solved rather than left standing, the duplicate test no longer blocks it, and the
  108 raises of `PBM_ROBOT_NEEDED` across 13 scripts bring it back when something wants a robot. The
  engine's factory already refuses correctly and queues nothing ([23-economy](docs/23-economy.md)).
- [x] ~~What a mine's ToMine does to its output~~ — closed 2026-09-18: it **bounds** the output and
  never scales it — the rate is `Mine_OrePerSecond × KPD` whatever the lode holds — but **the bound
  bites at half**. A takt digs, takes `ToMine − total` before either changes, then adds the dig to the
  total *and* takes the same dig off `ToMine`, ending when that pre-dig gap was no more than the dig. So
  the two meet half way and a lode of 600 yields 350. Measured over all 29 missions: 15 placed mines in
  8 missions, each with exactly one lode within 250, none carrying less than twice the 500 a mine may
  hold — so no shipped mine is ever bounded by its lode and the fix is fidelity, not behaviour. Control:
  6 of the 95 other buildings in those missions also stand within 250 of a lode. Fixed, with the final
  dig now banked on the ending takt ([23-economy](docs/23-economy.md)).

## Mission progression

[34-progression](docs/34-progression.md#not-established)

- [ ] The clock unit that times route reports.
- [ ] How a destroyed or captured unit leaves function 31's list.
- [ ] Where message text is drawn, and for how long.
- [ ] What `info_system` changes on screen.
- [ ] What follows `MISSION_COMPLETE`, and what leads to the next mission.
- [ ] Whether the hero reports its route from inside a boarded bot.
- [ ] What the behaviour does with the message 6 it sends itself.
- [ ] The ambient variations' schedule.
- [ ] A failure on the hero's death.

## Files and formats

[17-saves](docs/17-saves.md#not-established), [16-research](docs/16-research.md#what-is-not-read-here),
[19-descriptions](docs/19-descriptions.md#what-is-not-read-here), [18-vocabulary](docs/18-vocabulary.md),
[21-briefing](docs/21-briefing.md#not-established), [06](docs/06-open-questions.md), [22-settings](docs/22-settings.md)

- [ ] Saves:
  - [ ] most chunks' contents (the control chunk past `+32`, the wizard, behaviour, building and tree chunks)
  - [ ] the clan word before each mind list
  - [ ] the 24-byte records and the `1, id, id` triple
  - [ ] the AI state's layout
  - [ ] whether a mind list's ids are logical ids
- [ ] What `iron3d.dll:0x1008a690` does with a part's derived number, and what else reads a part's `Type`.
- [ ] What the words behind `objects.dlb`'s `A` and `N` are, and why twelve clip-less guns carry `A4`
  and `A5`. (~~what the `A` and `N` size letters stand for, and the 37 exceptions~~ — partly answered
  2026-09-18: the **referents** are pinned. `N` is the five `ANM` animals, which the research tree marks
  role 7 and no others; `A` is 27 fortification fittings, 21 of them fitted only under an `fr_*` root, and
  every line in the file that says "fortification" belongs to one. The words are **not recoverable**: the
  classification vocabulary reaches no shipped binary, with the `objects.dlb` token itself as the control.
  Of the 37 exceptions, 25 fall out of a rule — a weapon's `A<n>` is the size of the **round it fires** —
  7 more fire `f`-lettered rounds the rule cannot grade, one is a mobile builder, and 12 stay unexplained
  ([19-descriptions](docs/19-descriptions.md)).)
- [x] ~~Which of the `bb`/`bl`/`bm`/`bp`/`br`/`ba`/`bf`/`bt` building prefixes is which~~ — closed
  2026-09-18, and **they are not buildings**. All 61 records under those prefixes are `BULL` records —
  rounds — with their meshes in `weapon.rlb`, and none is placed in any mission. `b` is the tag's own
  letter and the second letter is the weapon family: bullet, laser, missile, plasma rifle, rocket, animal,
  flamer, taser, each settled by what fires it, by the trail effect its controller plays and by its
  flight ([18-vocabulary](docs/18-vocabulary.md)).
- [ ] Briefings: what game mode 4 is, how a briefing is skipped (`WaitForClick`), and the spline's curve.
- [ ] What the word after `data.tma`'s map path was for — it marks the free-play maps, but `Single.02`
  disagrees with every reading. (~~what reads a building's start flag back, and the word after the map
  path~~ — closed 2026-09-18, both negatively and with controls. **Nothing reads either.** `IMission`
  slot 11 returns the map word and no module calls it; the mission pointer is never stored outside the
  five frames that hold it. `IBuilding`'s start-flag getter has no caller either, and the field it reads —
  `CBuilding +0xb8`, the building's object state, 1 by construction and 2 when the flag is set — is only
  ever tested against zero, which a placed building never is. So the flag changes nothing
  ([04-missions](docs/04-missions.md)).)
- [ ] What `TRF1`'s directory flag does beyond the debug warning.
- [ ] What the landscape, camera and atmosphere component constructors read.

## Mission 02, *The Constructor*

Added on 2026-09-15 against `4f3a16e`, after the Outpost island's landing was fixed in M14.
[34-progression](docs/34-progression.md#not-established), [36-factory](docs/36-factory.md#not-established),
[37-designer](docs/37-designer.md#not-established), [38-designs](docs/38-designs.md#not-established),
[39-boarding](docs/39-boarding.md#not-established), [24-motion](docs/24-motion.md#not-established),
[31-packages](docs/31-packages.md#not-established)

**Where the engine is weaker than the game**

- [ ] The Large Factory's front door: its hall way does not lead to the pod, so the engine's way in is the west side door, which the recording's hero takes.
- [x] ~~The chimney smoke is orange where the recording's plumes are black~~ — closed in M14: an effect sprite takes its material entry's cell as a mesh batch does, and plays its track from its own start, so a puff leaves the chimney on `fire_smoke`'s first, orange cells and is on its later, black ones a quarter of a second on. Its plume's size went with it: a stream's particle walks and grows in metres, which the recording's 29 m over the chimney and 31 m across measure, where the control points' 2.6-long axes had made it 130 and 78. What a sprite's material starts from, and that the effect draw takes the entry's cell at all, are still stand-ins ([07-objects](docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured)).
- [ ] [M12] The medusas stand still. An animal's migration over its clan's pastures is not modelled, so the fight the recording shows from 195 to 218 s, two medusas high over the ground and green acid landing round the hero, never happens. Also not read: what their attack does with the migrate task's figures and circle, and how a flyer's migration point, at the pasture centre's height, meets its flying height.
- [ ] [M14] When `AniMesh.dll` works an agent's sphere and its node sphere out again (`0x10009510`), and at which pose. The engine works both out once, at rest, as the unit is made; they set how high a landed flyer stands, and so where the hero can get out.
- [ ] The minds: the recording's factory shows the hero holding one of the player's two, the same question as Mission 04's which unit holds none.

**The factory screen and production**

- [ ] What puts the capture and the factory screen on one frame.
- [ ] [M11] How the cursor is shown in view mode 5 (the engine: the system's cursor), and the fill colour the resource rows hand their bar (the engine: the weapons list's).
- [ ] Whether the designer pauses the world (the level's flag bit 8), and whether the `Mission` handler runs while a building's screen or the designer is up (the pause byte `+0xe8`).
- [ ] What handing the hero back does to it while the screen is up, and whether the player's keys still move it.
- [ ] The heading the escape leaves a new bot with, and whether a flyer climbs on its way out. (~~[M11] which areals the escape's random points must be on~~ — closed in M14: walkable ones, as the roam's test reads.)
- [ ] What commander pages 1–4 and 6–8 show from first person.

**The warbot designer**

- [ ] [M11] That the factory record's `+0x30`, the grade the chassis page is taken over, is the building's size class.
- [ ] [M11] The part box: `Epower`, the properties behind `regener`, `capacity`, `throughput`, `shotnum` and `blast`, and the formatter that prints one decimal whatever the template asks.
- [ ] [M11] What the turret fit does to guns on a turret it replaces, and the gun fit to a clip on a gun it replaces.
- [ ] [M11] Which destination row a tab selects as it turns on, which tab the panels turn to after a fit (only *seen*), and when fitting a chassis enables Armour (`0x10052491`).
- [ ] [M11] The previews' camera (which way it looks, the axis of its −0.5 rad pitch, which side its 60° field spans, its lights' colours), and how a scan band's green specular lights its strip.
- [ ] What the driven unit's property flag `0x8` gates while the designer is open; how the font's colour slot turns a row's colours into text; who fills the recent projects' count (`+0xb8e0`); the destination panel's own draw and takt; the tooltip's box and timing.

**Boarding, flying and getting out**

- [ ] [M11] The heading the hero is given on leaving, read as (F.x, −F.y) under an assumed matrix layout; the engine turns the hero to face the bot, as the recording shows.
- [ ] [M11] Which of a turret's nodes the boarding test's property `0x52` reads the life of.
- [ ] [M11] The name a bot's gun takes in the weapons list (`0x1008a470`, not followed).
- [ ] Why the recording's bot came to the hero between 236 and 249 s: an order the player gave, or its own behaviour after production.
- [ ] Whether a detached hero stays in the collision manager's or the areal map's lists, and what game messages `0x3f1` and `0x3f2` carry.

**Walking into the Large Factory**

- [ ] [M11] What `CBuilding` does to a door's or a pod's switch word as it files the item, and the capsule a door is measured against (`Terrain.dll:0x1005a27f`).
- [ ] [M11] Where a gathered face's batch word comes from (the engine lets movers **and rounds** through the `DEFAULT` and `PORTAL` materials' faces).
- [ ] [M14] How the basement is triangulated between its two rings, and which of the band's faces the first builder (`Terrain.dll:0x1000bdb0`) makes with the cut landscape face's own texture pair rather than the foundation. What the second builder writes on every face it makes — layer-1 slot 0, no second layer, flags `0x300`, a UV over the world at 0.066 a unit — is read (docs/03, "What a basement face wears"), and it is what the engine lays down.
- [ ] [M11] Whether the slope brake reads a building's stair faces, and who sets a collision object's flags, so which movers keep the floors in their push-out.
- [ ] [M14] How a portal face reaches `CBuilding::PortalDrawNotify` (`Terrain.dll:0x1005a5d0`, an interface slot nothing in the install is found to call) and which node its record names, so which cells a building draws. The lists and the render are read (docs/24, "A building is drawn cell by cell through its portals"); the engine draws every cell and only drops the portal quads, which costs frame time and shows nothing extra.
- [ ] [M14] Where the node matrix an action-3 effect takes as its frame (`Effect.dll:0x1000625a`, property 2) comes from. Its translation is the node's authored origin, which on 68 of `fortif.rlb`'s 112 door sounds stands more than 10 m from the door — 30.8 m on the three factories' side doors — so the sound is all but inaudible in the doorway. The engine stands the effect at the node's level-0 sphere centre instead.

## Mission 03, *The Field Base*

Added on 2026-09-15 against `950af7e`, after M12 made the mission winnable end to end.
[40-command-mode](docs/40-command-mode.md#not-established), [41-commander](docs/41-commander.md#not-established),
[42-selection](docs/42-selection.md#not-established), [32-builder](docs/32-builder.md#not-established),
[23-economy](docs/23-economy.md#not-established), [31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established), [37-designer](docs/37-designer.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M12] The construction sphere's look. Its three effects' records give time mode 0, a value set from outside (slot `0x1c`), and what sets it is not read: they loop on their durations and take the sphere's radius as their size, and the dome only partly shows.
- [ ] [M12] The commander's satellite map (`0x10073830`): its title bar and exit icon beyond their place, and the `+0x230` rectangle the column's click tests first. (Its marks by type are read and drawn since M13.)
- [ ] The warbot designer's save name field and load list: not built, and how they work is not established.
- [x] ~~A Mission 03 transport walks about 33 m/s where docs/23 gives 24~~ — closed in M13: the live limits now come from the unit's engine and load, and it walks 23.98. The recording's round being 13 s longer than two full-speed walks is still open.
- [ ] Why the patrol took about 85 s to reach the base in the recording, when a straight flight is 37–44 s; and how high a patrolling flyer flies, which decides whether its 3D attack limit ever holds.
- [ ] Which fight-module bar a building's guns must clear. With the walker's 0.85 the Small Bunker's flamers only fire at a unit close to its own ground level, never at hovering flyers, so the warbots do the fighting.

**Command mode and the panel**

- [ ] [M12] Whether the character handler sees Esc before its binding leaves command mode (the engine peels the map and the page back first).
- [ ] What view state 4 and the second camera at `+0x68` are for; cursors 7 and 8; the globals `0x1010bf7c`–`0x1010bf80`; the display's slot 12, which picks the system's cursor over the software one.
- [ ] What interface `0x201` slot 9 with (`0x20`, 1) and message (6, 7, 0) do to a bunker left for another view or for telepresence.
- [ ] [M12] What telepresence's auto-driver levels 1 and 2 hand to the AI. (What mode 2 does when its unit dies is read: the removal table rolls modes 1, 2, 5 and 7 back.)
- [ ] [M12] A unit record's `+0x30` and property `0x207`, and a building's `+0x30`: they pick and tint the panel's icons.
- [ ] [M12] What slot 7 of a unit's object does 0.6 s after *Explode!*; the chat overlay; the game menu's screen (mode 7); tooltips.
- [ ] [M12] What `0x10034230` accepts for an Upgrade row, and what Type `0x80000200` is. (The engine: the upgrade task's own target test — a live building of the Type whose scheme has an entry above its own — and, *derived* from Mission 03's recording offering no Upgrade Warehouse over a Small Warehouse whose Medium is unresearched, that the entry above is researched whole.)
- [x] ~~The research panel's contents and controls (page 4)~~ — read and built in M13 ([41-commander](docs/41-commander.md)).
- [ ] [M12] The routine that names a building (strings 6031–6098): the engine picks by Type and the root record's size letter.

**Selecting and ordering**

- [x] ~~[M12] An areal's first flag word (`+0x20`), which decides where a walker may be sent~~ — read in M14: it marks a walkable areal, the only kind the areal map links, and the engine's walker now keeps to them ([24-motion](docs/24-motion.md#the-global-path--read-and-measured)).
- [ ] Why the recording shows `PLACE`, not `GUARD`, over the bunker's roof at 190.5 s.
- [ ] [M12] Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule (`0x100361a0`).
- [ ] Whether the band is filled or only outlined; double clicks; whether a group sent to one place spreads out.
- [ ] The pending picks not traced: kind 2 (attack-target mode) and the orders kinds 2–5 give.
- [ ] [M12] A unit marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand.

**Placing and building**

- [ ] [M12] The site test's path search and its hall-way areal test (interface `0x303`, vertex bit 1). The basement's triangulation is [above](#walking-into-the-large-factory).
- [ ] What the pick's query record (first word `0xa`) asks the world for, so which objects stop the cursor's ray.
- [ ] What the game's `+0xe4` byte is, under which a site within 400 of one of the level's records turns red; whether holding `,` or `.` keeps turning the ghost.
- [ ] What an unfinished building looks like before the dome.
- [ ] [M12] Which state each sphere code opens, where an action-5 effect is placed, and which classes the sphere's kill takes.
- [ ] Which call sends SuperAI event 2 for a building a builder puts up (the recording counts the mine at 196 s, and it stands at 236 s).

**Economy**

- [ ] The clan's minds in the recording: the CPU figure reads 4 before any build, yet five warbots (SSW-4 to SSW-8) were built.
- [ ] The direction of the ore a mine's and a storage's loading places move by themselves (`0x10019482`, `0x100195b8`), and unit property `0x208`.
- [ ] [M12] Batteries and efficiency read from a building's root controller only; the economy timers' random source.

**Mission 03's script and recording**

- [ ] Why `T03_H03` and `T03_H02` never show in the recording.
- [ ] Which of the generator's exits the recording's hero used (the south one is inferred), and whether a shot door opens sooner than an approach.
- [ ] What component property `0x200` is (taken to be the door's node: all 56 door components' nodes match their channels').

## Mission 04, *Teleport*

Added on 2026-09-15 against `f26ffd8`, after M13 made the mission winnable end to end along the
briefing's route. [40-command-mode](docs/40-command-mode.md#not-established),
[16-research](docs/16-research.md#not-established), [27-ownership](docs/27-ownership.md#not-established),
[31-packages](docs/31-packages.md#not-established), [34-progression](docs/34-progression.md#not-established),
[24-motion](docs/24-motion.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[35-hud](docs/35-hud.md), [41-commander](docs/41-commander.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M13] Which of Mission 04's hero, helicopter and HQ holds no mind. The recording's factory shows one free of three once all three are the player's, and Mission 02's shows the hero holding one; the engine lets the HQ taken by Enter hold none, or the build could never start. Mission 03's "4 free, five built" may be the same question.
- [ ] [M3] The ground contact runs once a frame (message `0x1c`, read), but the engine runs it at state steps, and moving it waits on walking from the held ground face. Until then a hero pressed against Mission 03's Small Bunker door as it sinks is pushed down between steps; the bunker walk passes, but only just.
- [ ] The helicopter's capture pace: the recording takes 90 s to the factory and 81 s on to the research centre, the engine 54 s and 42 s at its live 14 m/s. And what held it still near (715, 847) from 362 to 402 s.
- [ ] The briefing's route is checked only by the engine's own test: the recording's player walked to the Teleport, so the large flyer's build, its flight to the HQ, boarding it, flying up to the plateau and setting down there were never compared with the game.
- [ ] Turret pitch and altitude as the user saw it. Measured: pitch moves neither a flyer nor its eye. Explained as rising ground lifting a low flyer, which never sinks back without F (read, `0x1001b3c3`); not confirmed in a window.
- [ ] Tooltips (the research rows' 6251/6252, the batch button's 6243), the maps' route lines and the selected unit's white outline: not drawn.
- [ ] [M13] The research box's name colour and its clip 5; the part preview draws larger than the game's.
- [x] ~~Mission 02's leaving places moved with the joined sphere~~ — closed in M14: the ground contact holds a machine about its node sphere's centre with the joined sphere's radius (read), so a landed L-2f stands 9.67 over flat ground and the whole island lets the hero out. The recording's 11 on landing and the hero's 3 on the same ground fit it ([24-motion](docs/24-motion.md#finding-the-ground--read), [39-boarding](docs/39-boarding.md#against-the-recording--seen)). Open still: see [Mission 02](#mission-02-the-constructor).

**The HQ's command view**

- [ ] [M13] Which bound a unit record's `+0x98` is: 8 × it is how far the camera rides behind the HQ. The engine takes the chassis sphere, 61 m, which the recording favours over the whole bound's 104 m and the cylinder's 38.7 m.
- [ ] [M13] What the game does when an HQ is lost in its own mode 3 (nothing read pops it), and how the stack reads after Enter in telepresence aboard an HQ.
- [ ] Why the HQ reads "[no order]" in its cockpit at 160 s after "[standing]" at 94 s. The engine's capture-standby departure gives it Standby.

**Capture and the maps**

- [ ] [M13] How a walker's path joins the hall way (`MGraph`), so which exit a capturer takes (the engine: the shortest whole way); whether a flyer touches down or hovers at its landing corner; and a flyer against a building's walls outside the hall way (the leg to exit 67 skirts the factory's west side).
- [ ] [M13] The escape's damaged-node test, and which paths a unit leaves a building by (the engine: back along the hall way).
- [x] ~~[M13] A contour vertex's areal flag word, which the engine has no areals for~~ — closed in M14: the engine reads the areal map, and a vertex counts on a walkable areal.
- [ ] [M13] The maps' contact list: the engine marks what lies in a player unit's radar range, not the list the run loop empties and each unit's takt refills.
- [ ] The sphere behind `IBuilding` slot 15.
- [ ] Why the Teleport's map icon looks white at about 412 s, as if selected, when a main teleport's capture selects nothing.

**Research**

- [ ] [M13] Whether a centre's ore take reads the ore it holds or the ore delivered to it.
- [ ] [M13] How often a building's takt runs (the engine: every tick), which bounds how late "Research complete" can come.
- [ ] What becomes of a research when its centre is captured, upgraded or destroyed mid-way (the engine keeps the queue with the building).
- [ ] Whether anything besides the task marks an item researched (scripts, saves), and the AI clans' own research orders (order 16), which are not modelled.

**The Main Teleport**

- [ ] What the teleport's class-25 parts do with the power byte's state `0x20`, and its class-29 parts with 1.
- [ ] What building interface `0xb` slot 16 asks about a place's node; what property `0x208` is (a network mirror flag is a guess); whether anything sets `pTeleFunc`.
- [ ] Whether a player-driven small unit can take the Teleport's pod.
- [ ] [M14] The places besides a dock's and a main teleport's (loading places) do not tick by the place rule yet; the place timer's random source; whether a destroyed generator stays in `World3D.dll`'s queue 3.
- [ ] [M14] What the game drives a dock's `f_recharge_*` glow with, whose own time mode is 0, a value set from outside.
- [ ] [M14] The AI's camouflage in play. Its repair decision and its trip to a dock are in (the engine: `diff_strong`'s 0.8 and 0.9 for `Decision_RepairOn` and `_Off`, since the profile a unit holds is not read; the nearest dock its size class fits, since the pick `0x10023b60` is not read, and never a building's own repair system).
- [ ] Why the hero's panel dims in the pod room, and why the chamber's glow reads greyer than the recording's (the dawn scene colour is the guess).
- [ ] Message 16, the helicopter in route 2, has no test.

**Movement**

- [ ] [M13] Whether the AI's aiming reaches the turret lock's lead, and what spin a unit let go keeps until the Wizard writes one.
- [ ] [M13] What a node reaching its last damage stage takes out of the load (the engine: its own weight and armour).

## Not looked at at all

- [ ] The network protocol.
