# Turrets — two mountings, their sockets, and the role they give a unit

A robot is a chassis with one turret on it, and the turret carries everything
that points: the guns, the radar, the deflector and the camera. It also decides
what the robot *is*. A warbot, a transport, a builder and an HQ unit differ in
their turret and in nothing else the engine looks at.

**Every claim is tagged**, as in [23-economy.md](23-economy.md). *Measured*
means `openparkan verify` re-derives it. *Read* means it comes from the
disassembly at the address given. *Guess* fits the evidence but is not
established.

## Twenty-eight turrets, each built twice — *measured*, and *read*

`objects.rlb` holds 55 turret records, `e_tur_<size><mounting>_<nn>`. The size
letter is the usual one ([18-vocabulary.md](18-vocabulary.md)): `t` tiny, `l`
small, `m` medium, `b` large and `h` hero. The second letter is `t` or `b`.
Every turret but the hero's comes in both, 27 pairs in all. **The two letters
are two mountings of one turret**:

- **Shared data.** The pair shares its `objects.dlb` entry (name, code, costs),
  its mesh, its `.wea` and its `.ndp`. Both name the same `o_tur_<size>a_<nn>`
  files.
- **The controllers differ in one bit.** Each has its own `.cpt` and `.ctl`, and
  on all 27 pairs the controllers differ only in the turret component's flags
  word at record `+8` ([13-control.md](13-control.md#the-component-record)).
  Bit `0x4000000` is set on every `t` and clear on every `b`.
- **Research.** The research tree names both parts under one item, so
  researching a turret opens both mountings (docs/16-research.md's "pair of
  mounting variants").

**`b` hangs under a flyer; `t` stands on the ground** (*measured*):

- All 83 `b` turrets in the shipped assemblies sit on a chassis whose catalogue
  name says Flying or Helicopter.
- All 289 `t` turrets sit on one that doesn't.
- That is the upside-down turret socket
  [07-objects.md](07-objects.md#how-parts-attach) found on the flying chassis.

The bit is what the turret code reads (*read*). The turret class (constructor
`Control.dll:0x10027050`, vtable `0x1003caf8`, kept at control-system `+0x5c8`)
flips its aim on it:

- **Set (upright).** All three components of the aim triple are kept as 1 − v,
  at init (`0x100271c7`) and again when read (`0x100276ed`).
- **Clear (hung).** The strafe offset handed to the turret is negated
  (`0x100289b5`, [Aiming and the camera](#aiming-and-the-camera--read-and-measured)).

So the channels of a hung turret see the triple as stored, and the channels
of an upright one see 1 − v. The node is upside down under a flyer, which is
what makes the two agree on screen (*derived*, below).

A second bit, **`0x8000000`, marks the HQ turrets** (*measured*). It is set on
all 14 HQ records, both mountings. Of the rest, only `o_tur_lb_06` carries it,
and its `t` twin doesn't: a slip in the data, by the look of it (*guess*).

**What reads it** (*read*):

- **The control system.** It answers it as bool query 13 of its interface
  `0x204`: slot 4 (`0x1002b410`) tests the turret's record flags
  (`0x1002b7bb`).
- **The component interface.** `0x202` turns that into value 117 (`0x1002e8db`).
- **`iron3d.dll`.** Its `IsHQ(unit)` (`0x10076f50`) asks value 117 of the unit's
  first class-1 component, and seven places call it. What they gate is in
  [An HQ unit in play](#an-hq-unit-in-play--read).

## A turret is a turret, a radar, a camera and a deflector — *measured*, and *read*

**Components.** All 28 robot turret controllers open with the same four components:

| class | what | values |
|---|---|---|
| 1 | the turret | all zero |
| 8 | a radar slot, labelled `i_rdr_<size>` | 0.5, 0.5, 0.5, range 500 (small) or 800 (medium, large), period 750 |
| 4 | the camera | 0.1, 1000, 1.3, 1, 150, 1 on every one |
| 21 | a deflector slot, labelled `i_def_<size>` | six times 0.5 |

**The labels name the part size the turret takes** (*measured*):

- All 744 radar and deflector parts fitted to a turret in the shipped assemblies
  match the size in the label on their turret's radar or deflector component.
- Mostly that is the turret's own size. The medium transport turret takes small
  (`l`) parts. The tiny turret takes a small radar and a tiny deflector.
- The part's own values are in `intsys.rlb` ([25-sensors.md](25-sensors.md),
  [26-damage.md](26-damage.md)).

**The camera** (class 4, constructor `0x100233d0`) creates a view with
`World3D.dll!CreateObject` type 5 and hands it its values 0, 1 and 2
(`0x100238b0`, *read*). By their numbers they are a near plane, a far plane
and a field of view of 1.3 rad; the names are a *guess*.

**Values 3–5 (1, 150, 1) have no reader found.** The camera class never
touches its record past value 2. A scan of five modules for a value query
with a literal id of 3, 4 or 5 finds seven, all in `Control.dll` and all on
other classes (`0x100243dd`, `0x10024445`, …); that is the positive control.

**The field of view is horizontal** (*read*, in `Terrain.dll`, whose
`LoadCamera` `World3D.dll` re-exports):

- The camera keeps tan of half its angle (`0x100415a1`).
- It scales screen x by that alone, and y by it × height ÷ width
  (`0x1007f190`, `0x10077982`, `0x1008c1d0`).
- At 4:3 that is 74° across and 59° down.

Two links in this are *guesses*: that type 5 is that camera, and that value 2
is the angle it keeps. See
[Aiming and the camera](#aiming-and-the-camera--read-and-measured).

**Turn rates and limits are not in the frame.** All 58 controllers in
`turrets.rlb` carry the constructor's defaults on every frame slot
([13-control.md](13-control.md)). A turret's rates and limits are its two
**channels**, section-2 records
([Aiming and the camera](#aiming-and-the-camera--read-and-measured)).

**Hit points** (*measured*). The turret, its radar and its deflector sit on
three separate nodes, each with its own `.ndp` hit points:

| turret | body | radar node | deflector node |
|---|---:|---:|---:|
| tiny | 120 | 100 | 75 |
| small | 270 | 190 | 130 |
| medium | 720 | 240 | 190 |
| large battle, HQ, transport | 3,000 | 600 (HQ 800) | 800 |
| large builder | 1,500 | 350 | 350 |
| small HE | 80 | 190 | 130 |
| Transformer | 15,000 | 2,500 | 2,000 |
| Small tower | 3,500 | 1,500 | 1,500 |
| monsters C4M1 / C4M2 / C5M1 | 66,000 / 10,000 / 80,000 | 28,000 / 2,500 / 20,000 | 22,000 / 2,000 / 20,000 |

"Body" is the node under the class-1 component. Shooting the radar node blinds
the unit ([25-sensors.md](25-sensors.md)); shooting the deflector node drops the
shield bubble ([26-damage.md](26-damage.md)).

## Aiming and the camera — *read*, and *measured*

**A section-2 record is a channel** (*read*, turret code
`0x10027170`/`0x100289f0`; *measured* on all 58 turrets):

| Offset | Field |
|---|---|
| +0 | the **node** the channel animates |
| +4, +8 | first and last animation frame |
| +0xc | initial value, 0–1 |
| +0x10 | a second control point, or −1: the camera channel's `CameraCenter` |
| +0x14 | a **control point** in the same-stem `.cpt` |
| +0x18 | rate: value per second |
| +0x1c | span: radians from value 0 to 1 |
| +0x20 | flags; 3 on every yaw channel |

**The node is +0** (*read*, loader `0x10008fb9`; *measured*). All 991 channels
of the controllers that `objects.rlb` pairs with a mesh name a node of that
mesh. On 588 of the 593 that span frames, those frames move the node, posed as
the engine poses it. As a control, the next node moves on 149, and +0x14 read
as a node on 87. On the
hero turret the four channels animate `CP_m1o1`, `Turn_m1o1` (49–53),
`GP_m1o1` (55–57) and the barrel `Gun02_m1o1` (58–60).

**The flags** (`+0x20`, *read* `0x10009950`, `0x10021a30`):

| Flag | Meaning |
|---|---|
| 1 | wraps |
| 2 | inverts, 1 − v |
| 4 | is not driven by the component update (the camera) |
| 8 | joins the turret's list (the gun mounts that follow pitch) |
| `0x40` | takes the previous channel's value (`AR_*`, `AL_*`) |

**A turret's two component entries are its yaw and pitch channels.**

- **Pointing** (*measured*): on all 58 turrets the yaw channel points at
  `TurretDirect` and the pitch channel at `TargetDirect`.
- **Yaw** spans 6.28 over four frames on every turret.
- **Pitch** spans two frames: π/2 on 57 turrets, 1.92 rad (110°) on the
  hero's.
- **Where it starts**: the pitch channel starts at 0.111 on 52 turrets, 0.167
  on 5 and 0.273 on the hero's. **The start is level on every robot turret**
  (*measured*, all 55 `e_tur` records, within a degree). So the limits are
  −10°…+80° on 52, −15°…+75° on the Small tower's two and −30.5°…+79.4° on the
  hero's.
- **Rates**: yaw 0.5–0.85 turns a second (100 on the hero and two others),
  pitch 0.3–0.75 spans a second.

**The value is an animation frame** (*measured* on `o_tur_ha_02.msh`):

- The yaw frames 49, 51 and 53 turn `Turn_m1o1` to 180°, 0° and 180°, so 0.5
  looks ahead.
- The pitch frames 55, 56 and 57 tilt `GP_m1o1`'s sight to −30.5°, +24.4° and
  +79.4°. That is a 1.919-rad sweep against the channel's 1.920, and the
  initial 0.2727 lands within 0.6° of level.
- The engine plays frame = first + v × (last − first) on the node's own
  segment (*read*: `0x10009950` hands the node the pair and its value;
  `AniMesh.dll:0x10008b30` lerps across it).

**The aim is linear in the value** (*read*, and *measured*):

- **Between keys.** `AniMesh.dll:0x10012880` poses a fractional frame from the
  key at or before it and the next. It lerps the position and hands the two
  rotations to `Ngi32.dll`'s `g_FastProc` slot `+0x44`. That slot's generic
  build (`Ngi32.dll:0x10014630`, set at `0x10003bc6`) is a shortest-arc slerp:
  its weights come from the acos of the dot product, and it goes linear when
  the dot is within 1e-5 of 1. The CPU-specific builds (`0x10017f50`, and the
  one installed at `0x100040c6`) were not read.
- **The keys are even.** On all 55 robot turrets the yaw frames face 180°,
  −90°, 0°, +90° and 180° in turn: a quarter turn a frame, so 0.25 looks to −x
  and 0.75 to +x. Every pitch channel rises in two equal steps. A slerp over
  even keys about one axis turns at a constant rate, so the angle is linear in
  v (*derived*).

**How the aim moves** (*read*):

- **Storing the target.** The component interface stores the target triple at
  `+0x9c` (`0x1002eb70`).
- **The mounting.** An upright (`t`) turret keeps it as 1 − v, at init
  (`0x100271c7`) and again when read (`0x100276ed`). So the channel sees v.
- **The step.** Each tick, with dt = elapsed ms × 0.001 (`0x10027765`), each
  channel moves toward its target by at most rate × dt (`0x100289f0`). A
  wrapping channel takes the short way round.
- **The pitch clamp.** Mouse Y is clamped to [0, 1] by the row, so the
  channel's span is the pitch limit.
- **The hero** tilts 0.25 × 0.006 × 1.2 × 1.92 = 0.0035 rad a filtered count,
  at most 0.75 × 1.92 = 1.44 rad/s. Its yaw channel is never sent a value: the
  hull turns instead.
- **The strafe offset.** The turret's first entry also carries −(the strafe
  angle, eased across the step) ÷ the yaw span, negated when hung
  ([24-motion.md](24-motion.md#from-input-to-motion--read-and-measured)).
- **A machine's turret leads its hull.** A change to the turret's yaw is also
  added into control-system `+0x1ec` (body `+0x38`, `0x1002eb70`). `+0x65c` is
  set on a unit (agent kind 4) once a value turns it on (`0x1000ea66`,
  `0x10031a38`). With it set, and no normalised turn pending (`+0x35d`, which
  `SetNormAngle` sets and the spin setter clears), the attitude integrator
  (`0x10014a96`):
  - turns the hull toward the turret: spin z = clamp(−(`+0x38` − 0.5) × 2π
    ÷ (turn rate × dt), ±0.7);
  - pays that turn back out of `+0x38`, and keeps the step in turns at `+0x3c`
    (`+0x1f0`).
  - The takt then re-aims the turret: yaw = `+0x1ec` − (1 − s) × `+0x1f0`
    (`0x10005b13`). So the turret holds its heading while the hull comes round
    under it.

**Which way a count turns** (*read* chain, *derived* sign):

- **Counts.** DirectInput's relative mouse state is copied as it comes
  (`World3D.dll:0x10013a1c`), and in that API x grows to the right and y
  toward the user.
- **Pitch.** Mouse down adds to the stored y. An upright turret's channel
  sees 1 − y, and pitch channels carry no invert flag (0 on all 58). So the
  value falls, the frame falls toward `first`, and the sight lowers: every
  pitch channel's elevation rises with its frame (*measured*). Mouse down looks
  down unless `MOUSE_REV_Y` is set.
- **A machine's yaw.** Mouse right adds to the stored x. The upright 1 − v
  and the yaw channel's invert flag (3 on all 58) cancel, so the value rises
  toward 0.75, which faces +x. The running gear puts the left side at −x
  ([24-motion.md](24-motion.md#running-gear-legs-wheels-and-tracks-by-side--read-and-measured)),
  so +x is right. Mouse right turns right.
- **The hero's yaw.** Mouse right adds to the pending turn's z, and the
  integrator turns the hull by −(z − 0.5) × 2π (`0x10014858` negates it). The
  turret-led drive above uses the same form to bring a machine's hull round
  to a turret turned toward +x. For that to follow the turret, −z must be a
  right turn. So mouse right turns the hero right too.
- **The developers' labels agree** (*measured*). In `m1.tbl` and `m2.tbl`
  `OBJ_TURN_LEFT` sends `MCMD_ROTATE_Z` +0.7 and `OBJ_TURN_RIGHT` −0.7.
  That value is the spin's z (`World3D.dll:0x1000fe77` → `IControl` slot 4,
  `Control.dll:0x10004440`). So +z turns left, and a mouse count's −z turns
  right.
- **Three more consistencies.** `A` sets the strafe to +π/2, and it moves left
  only if +z turns left. The turret's counter-turn toward +x cancels that. A
  machine whose left gear is weaker gains +z spin, and veers toward its
  damaged side.
- **A hung turret** is the same on screen. Its channel sees the stored value,
  not 1 − v, and its node is upside down.

**The guns' sight** (*read* `0x10028130`, `0x1002a610`; *measured*):

- **What the turret hands each gun.** Its target, the yaw channel's second
  point and the pitch channel's point. On all 59 turret components those
  points are `TurretCenter` and `TargetDirect`.
- **The ray.** A gun aims its round at the first thing a ray from
  `TurretCenter` along `TargetDirect` meets, and at no nearer than 100 m. The
  camera looks along the same `TargetDirect`, so the shot goes where the
  crosshair is (*derived*).
- **Follower channels.** A channel flagged 8 joins the turret's list
  (`0x10009120`). Each tick the turret aims its gun's mount with it and sets
  that gun's ready byte (`0x10027f51`). How it decides is not read. On the hero
  there is one per gun: `Gun01`, `Plz01`, `Lz01` and `Rk01`.

**The first-person eye** (*read* `0x100234c0`; *measured* on the hero):

- **The camera's channel** points at `TargetDirect`, with `CameraCenter` the
  point just before it. This holds on all 58 turrets.
- **Which point is which** (*read*). `Control.dll:0x1001b4f0` returns a
  point's position (record `+0xc`) and vector (`+0x18`), scaled by the object
  and carried into the world through its node.
  - **The eye** is `CameraCenter`'s position, plus the shake
    (`0x10023603`; `0x1002368b`).
  - **The look** is `TargetDirect`'s vector (`0x10023618`).
  - **The up** is `CameraCenter`'s own vector. The camera crosses it with the
    look to get the side (`0x100236bc`), and the look with the side to get the
    up (`0x100236eb`).
  - **The frame** is a matrix whose columns are look, side, up and eye
    (`0x10023741`). If the two vectors are parallel, a look-only frame is used
    instead (`0x10023769`).
- **The data agrees** (*measured*): `TargetDirect` is +y on all 58 cameras.
  `CameraCenter`'s vector is +z on all 31 upright ones and −z on 25 of the 27
  hung ones. The two hung exceptions are the Transformer's and the Small
  tower's, which no mission hangs.
- **A point's node.** A `.cpt` point's second float is an int32, the node it
  sits on ([07-objects.md](07-objects.md#ctpt--control-points)).
  - On the hero, `CameraCenter` sits at the origin of node 35, `CP_m1o1`: not
    animated, flag `0x20`, a child of `Eye_m1o1`.
  - `TargetDirect` is +y on node 34, `GP_m1o1`, which the pitch frames tilt.
  - The eye is 0.876 above the turret root and 0.16 forward, and it does not
    move with pitch.
  - The camera component's own node is the eye's on only 7 of 58 turrets, so
    the point is what places it.

**Free look** (`0x10023788`, *read*; rows *measured*):

- **Mode.** In mode `0x200`, the constructor's, the camera turns its view by
  its own triple `+0x94`.
- **Pitch** is (0.5 − y) × π about the side axis. Clamped, that is ±90°.
- **Yaw** is (0.5 − x) × 2π about the up axis, wrapping. Roll is 0.
- **Rows.** Shift + mouse X adds 0.1 a step, Shift + mouse Y 0.15. Releasing
  Shift, or Shift + right button, sets both back to 0.5 at once.
- **Other cameras.** No second camera component exists on the hero's turret.
  A third-person view was not found.
- **Its signs** (*derived*). The side axis is up × look, which points left
  (−x for an up of +z and a look of +y), and both angles go through the rotation routines the hull's
  attitude uses, applied the same way round (`g_FastProc` `+0x3c`, then `+0x10`
  on the frame). Shift + mouse right gives a negative yaw about up: right, like
  the hull. Shift + mouse down gives a negative angle about the left axis,
  which tips the look up. So of the two free-look axes exactly one runs
  opposite to the main controls, and on these readings it is the vertical.
  Checking this in the game is cheap and has not been done.

**The camera shake** (*read*):

- **What feeds it.** Each machine tick hands every camera component the
  machine's change in velocity over the step, (previous − current) ÷ the step
  in seconds (`0x1000c6e7`). The velocity comes from two successive pose
  matrices; the previous one is kept at `+0x664`.
- **A jolt** (`0x10023ab0`) whose squared size reaches 0.3 starts a blend.
  The offset then runs from where it was (`+0xc8`) toward half the jolt
  (`+0xb0`) at (t − start) ÷ `+0xac` (2.5 s); the start is stamped at `+0xa0`
  and `+0xd4` is set.
- **A smaller jolt** during a blend ends it. The offset reached becomes the
  amplitude, and it rings down as amplitude × cos(π/2 × `+0xa8` × t) ÷
  (t + 1)^`+0xa4`, with the constructor's 3 and 3 (`0x10023566`–`0x100235c3`).
  A smaller jolt at any other time does nothing.
- **The eye moves** by the offset clamped to unit length, times 0.02
  (`0x10023646`, `0x10023677`): 2 cm at most.

## Gun sockets are the mesh's `Base_*` nodes — *measured*

Every `Base_*` node of a turret mesh, except the one it mounts by (`Base_TM` or
`Base_TL`), is a gun socket. The names say where: `LU`, `RU` and `MU` (left,
right and middle, upper); `C`, `CL`, `CR`; `LD`, `RD` (lower); `FC` and `BC` on
the big HQ and the Transformer; `Base_gun` on the Small tower.

**The sockets are the catalogue's slots.** On 22 of the 28 turrets the socket
count equals the number in the turret's `objects.dlb` text ("4-slot large
battle", "HQ turret with 5 battle slots"), once a builder's module socket is
counted apart. The six that differ are all special:

- the **Large transport** has one socket where its text says two battle slots;
- the **Large builder** has two sockets for "two battle slots" and a module, so
  one of them must be the module's (never assembled, so it can't be told which);
- the **three monsters** and the **hero turret** have no socket at all. Each
  carries **four guns built into its own controller** (class 2): `pcannon`
  effects on the monsters; `hero_cannon`, `hero_prifle`, `hero_redlaser` and
  `hero_missile` on the hero. The monsters' "1 battle slot" is those guns.

**What goes in a socket** (*measured*, over the 372 turret mounts):

- **Sizes match.** A turret's size letter is its chassis' on all 372, and a
  gun's is its turret's on 848 of 865. The other 17 are fortification guns
  (`e_gun_fc_*`, `e_gun_fl_*`), all on the Small tower.
- **Gun kinds.** The gun's second letter is its kind: `c` gun, `l` launcher,
  `s` a module ([18-vocabulary.md](18-vocabulary.md), [29-weapons.md](29-weapons.md)).
- **The builder module.** A **builder turret carries its module on
  `Base_LU_01`**: all 16 `e_gun_ls_10` / `e_gun_ms_12` "Mobile builder modules"
  do, on the small and medium builders, and no other turret mounts one. What
  the module does is in [32-builder.md](32-builder.md).
- **The radar and deflector parts don't use sockets.** Their attach field is not a
  node: it is the index of the turret controller's radar slot (1) and deflector slot
  (3), which the parts are re-parsed into
  ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).

## The turret decides what the unit is — *measured*, and *read*

A `.dat` assembly's class word ([07-objects.md](07-objects.md)) is the unit's
**Type**: the value a mission's `Type` property holds and `Behavior.dll` keeps
at `+0xafc`. Across all 458 assemblies the turret decides it, without
exception:

| turret | Type | behaviour profile | research role byte |
|---|---|---|---:|
| battle (tiny `01`; small and medium `01`–`02`; large `01`–`03`; small HE `06`; Transformer, Small tower, the three monsters) | `0x1008000` | `prof_war.var` | 2 |
| transport (`04` small/medium, `07` large) | `0x1002000` | `prof_trn.var` | 3 |
| builder (`03` small/medium, `08` large) | `0x1004000` | `prof_bld.var` | 4 |
| HQ (`05` small, `05`–`07` medium, `04`–`06` large) | `0x1010000` | `prof_hq.var` | 5 |
| hero | `0x1020000` | `prof_hero.var` | 6 |

- **Placed units agree.** Every unit placed in a mission carries the same Type
  as its assembly.
- **The profile follows the Type** (*read*). `Behavior.dll:0x10008a80` picks
  the behaviour profile from it, and the same switch gives buildings theirs.
- **So does the research tree** (*measured*). A research item's first tail byte
  is the same role, for every turret item in all 29 trees: 1 for a bunker or
  tower turret, 6 on the hero chassis, 7 on animals, 255 on anything else.

**A designed unit's class word comes from its turret's research item**
(*read*, and *measured*). The unit writer `iron3d.dll:0x100544b0` writes the
`0xf0f1` magic at `0x10054f5f`, and the class word after it is decided at
`0x10054505`–`0x10054551`:

- **An animal.** A chassis whose name starts with `a` is `0x20000000`.
- **Otherwise the base part.** `0x1008a500` fills a record for the design's
  base part (design `+0x39c`) through the research tree's reader. It takes
  slot 12 (`+0x23`), slot 11 (`+0x24`), slot 18 (`+0x25`), slot 14 (`+0x22`,
  the role) and slot 17 (`+0x26`, the size): `MisLoad.dll:0x10002d50`,
  `0x10002d80`, `0x10002de0`, `0x10002e10`, `0x10002db0`.
- **A chassis passes the question on.** If the base part's `+0x23` is 9, the
  record is filled again for the design's second part (`+0xd240`), the
  turret.
- **The Type** (`0x1008a590`):

| `+0x23` | `+0x24` | Type |
|---:|---:|---|
| 9 | 33 | by the role: 3 `0x1002000`, 4 `0x1004000`, 5 `0x1010000`, 6 `0x1020000`, any other `0x1008000` |
| 8 | 17 | by the size: 1 `0x80010000`, 2 `0x80020000`, 3 `0x80040000` |
| 8 | 16, 18–21, 24–26, 29, 30 | `0x80000040`, `0x80000400`, `0x80000004`, `0x80000010`, `0x80000008`, `0x80000200`, `0x80001000`, `0x80000002`, `0x80100000`, `0x80200000` |
| anything else | | 0 |

**The data bears it out** (*measured*):

- **The codes.** Across the 10,672 item records of all 29 trees, `+0x23` is
  the part's catalogue kind: 8 `BLD`, 9 `SHS` and `ANM`, 10 `AMM`, 11 `DVC`,
  12 `WPN`. `+0x24` is 33 exactly on the 812 `SHS:TUR` records.
- **Robots.** The function over each robot's turret item gives its `.dat`
  class word on all 372 robots with a turret.
- **Buildings.** Over each building's base part it gives the class word on 70
  of 74. The other four are ruins, sub-kind 28, which the function leaves at 0
  while their files say `0x80002000`.
- **A control.** Every building Type it gives is one `ArealMap.dll` registers
  for a `BuildDat.lst` scheme, but for the bridge's `0x80001000`.

So **the research role byte is what makes a warbot, a transport, a builder or
an HQ unit** when the player designs one.

**The Type decides the commander's orders** (*read*). `iron3d.dll` keeps a table
at `0x10104f98`: each record is an order, a Type mask and a string. It offers an
order when `mask & Type == Type` (`0x1007ae52`):

| orders | offered to |
|---|---|
| Standby, Route, Search and capture, Seek and destroy, Guard, Refit | every robot (mask `0x103e000`) |
| Transport minerals | transports |
| Search minerals; Build Mine, Warehouse, Factory, Outpost, Res. Center, Light Tower, Heavy Tower; Upgrade the same seven | builders |

A second table at `0x10105150` lists Standby, Follow me, Search and capture,
Seek and destroy, Attack, Capture building and Refit, for every robot. Where
each table is used, and what each order does, is in
[31-packages.md](31-packages.md).

## An HQ unit in play — *read*

Beyond its profile and its orders, **an HQ unit is one whose turret carries
bit `0x8000000`**, and `iron3d.dll` asks that through `IsHQ` (`0x10076f50`) in
seven places:

- **The unit panel's button.** For a unit whose Type (`+0x2c`) is `0x1010000`, the button
  at panel `+0x588`–`+0x598` is drawn white when the bit is set and grey
  (`0xff808080`) when not (`0x10085b5e`). Clicking it checks the bit again
  (`0x100847d5`) before it enters mode 3.
- **Mode 3.** `0x10062bc0` enters a `CState` mode; mode 3 is
  `SELECT_GUARD_TARGET_MODE` (the name switch at `0x1005a5cc`). It enters it
  only if the object handed in passes `IsHQ` (`0x10062c2e`).
- **Mode 3's handlers.** Three of the state table's handlers for mode 3
  (`0x10063a20`, `0x100647e0`, `0x10064900`, table `0x10104b18`) test it again
  before they hand the unit's position and π/2 on.
- **Command 730, `CMD_ENTER_STATE`.** Its case tests it on the controlled unit
  (`0x10071f4e`).

So the bit unlocks a guard-target selection that other units' panels grey
out. What the guard target then does for an HQ is the packages' side
([31-packages.md](31-packages.md)), not read here.

## Every turret

Build cost is energy / ore, from `objects.dlb`; "free" means all four costs are
zero. "Assembled on" names the chassis in the shipped `.dat` files, the `t` and
`b` mountings together.

| turret | code | role | gun sockets | body HP | build cost | assembled on |
|---|---|---|---|---:|---|---|
| Tiny Battle `tt/tb_01` | 2T1 | battle | 2 | 120 | 2.5 / 5 | `R_T_01`, `R_T_02` |
| Small Battle `lt/lb_01` | 2s1 | battle | 2 | 270 | 6 / 15 | `R_L_01`–`05` |
| Small Battle `lt/lb_02` | 3s2 | battle | 3 | 270 | 6 / 18 | `R_L_01`–`05` |
| Small Builder `lt/lb_03` | Bs1 | builder | 1 + module | 270 | 3 / 5 | `R_L_03`–`05` |
| Small Transport `lt/lb_04` | Ts1 | transport | 1 | 270 | 8 / 6 | `R_L_03`, `R_L_04` |
| Small HQ `lt/lb_05` | HQs1 | HQ | 3 | 270 | 4 / 7 | never assembled |
| Small HE `lt/lb_06` | Se1 | battle | 1 | 80 | 45 / 85 (research 35 / 60) | never assembled |
| Monstr C4M1 `lt/lb_07` | M1 | battle | 0, 4 built-in | 66,000 | free | `R_L_07` |
| Medium Battle `mt/mb_01` | 3m1 | battle | 3 | 720 | 9 / 22 | `R_M_01`–`04` |
| Medium Battle `mt/mb_02` | 4m1 | battle | 4 | 720 | 10 / 25 (research 10 / 35) | `R_M_01`–`04` |
| Medium Builder `mt/mb_03` | Bm1 | builder | 1 + module | 720 | 8 / 25 | `R_M_01`, `02`, `04` |
| Medium Transport `mt/mb_04` | Tm1 | transport | 1 | 720 | 15 / 30 | `R_M_03` |
| Medium HQ `mt/mb_05`–`07` | HQm1–3 | HQ | 3 / 4 / 5 | 720 | 7 / 14, 10 / 17, 12 / 20 | never assembled |
| Large Battle `bt/bb_01`–`03` | 4L1, 5L1, 6L1 | battle | 4 / 5 / 6 | 3,000 | 12 / 35, 15 / 40, 17 / 45 | `R_B_01`–`04` |
| Large HQ `bt/bb_04`–`06` | HQL1–3 | HQ | 4 / 5 / 6 | 3,000 | 20 / 40, 25 / 45, 30 / 50 | `R_B_01`, `03`, `04` |
| Large Transport `bt/bb_07` | TL1 | transport | 1 (text: 2) | 3,000 | 18 / 35 | `R_B_01` |
| Large Builder `bt/bb_08` | BL1 | builder | 2 (text: 2 + module) | 1,500 | 20 / 45 | never assembled |
| Transformer `bt/bb_09` | TR1 | battle | 8 | 15,000 | free | `R_B_05` |
| Small tower `bt/bb_10` | TT1 | battle | 1, fortification guns | 3,500 | free | `R_B_06` |
| Monstr C4M2 `bt/bb_11` | M2 | battle | 0, 4 built-in | 10,000 | free | `R_B_07` |
| Monstr C5M1 `bt/bb_12` | M3 | battle | 0, 4 built-in | 80,000 | free | `R_B_08` |
| HERO TURRET MK2 `ht_02` | HERO 2 | hero | 0, 4 built-in | — | free | `R_H_02` |

The bunker and tower turrets (`e_bnt_*`, `e_tow_*`) are buildings' parts
(catalogue group 1), not robots'. Their controllers are just a turret and a
camera, and they carry fortification guns and a bunker or tower radar
([25-sensors.md](25-sensors.md)).

## The turrets the player never builds — *measured*

**Six turrets cost nothing** to research or build: the Transformer, the Small
tower, the three monsters and the hero's. Their chassis (`R_B_05`–`08`,
`R_L_07`, `R_H_02`) are free too. They are the special ones:

- **Missions give them only to the enemy.**
  - The Transformer is placed for enemy clans 3 times and a neutral one once.
  - The Small tower is placed for enemies 28 times.
  - Each monster is placed once, for an enemy.
  - The hero turret is placed 37 times, always for a player.
  - For comparison, the paid battle turrets are placed for player clans 28
    times.
- **None of the six is in `UNITS/UNITS/AI`**, the folder of 77 assemblies
  whose name suggests the computer players' designs (*guess*).
- **They don't use sockets.** The monsters and the hero carry built-in guns and
  have no socket. The Small tower is the only robot turret that takes
  fortification guns.

Every turret, these six included, is in the part list of all 29 research trees,
so a tree's part list doesn't gate them. **A tree's state for them does.**

- **The state is bits** (*read*). Each item's category byte is the tree's
  starting state ([16-research.md](16-research.md)). `MisLoad.dll`'s reader hands it out as
  bits (slot 3, `0x10002aa0`): 2 the item has been researched, 1 it can be,
  4 it is in this tree.
- **Researching sets them.** Slot 7 (`0x10002c10`) sets 1 and 2 on an item
  that has 4. It then sets 1 on every present item whose prerequisites all
  have 4 and 2. The shipped states are only 0, 2, 4, 5 and 7 (*measured*).
- **What the trees say** (*measured*, all 29):
  - In the 11 player trees (`*p.trf`, `tut*_pl.trf`) the Transformer, Small
    tower and three monster turrets are never present.
  - The monsters are present and researched (7) in the enemy trees (`*e.trf`)
    and in `auto`.
  - The Transformer and Small tower are present only in `full`.
  - The hero turret is present in 9 trees, `c2m4p` and `c3m2p` among them.
    In every one of those its chassis `r_h_02` reads 2, researched but not in
    the tree.

So the missions keep five of the six out of the player's trees, and the
sixth's chassis. That a designer offers only parts with bit 4 is a *guess*.
`iron3d.dll` reads item records through wrappers at `0x1008a480`–`0x1008a4f0`,
used by the design screens from `0x10048925`, and which bits those screens
test was not read. That a zero cost marks them unbuildable is still a *guess*.

## Not established

- ~~Which code sets a newly designed unit's class word, and whether it reads
  the turret's research role byte~~ — **read**: `iron3d.dll:0x1008a590` over
  the turret item's kind, sub-kind and role
  ([The turret decides what the unit is](#the-turret-decides-what-the-unit-is--measured-and-read)).
- ~~Which way a mouse count tilts an upright turret on screen~~ — **derived**:
  mouse down lowers the sight and mouse right turns right, on the hero and on a
  machine ([Aiming and the camera](#aiming-and-the-camera--read-and-measured)).
  That Shift's free look runs vertically opposite is *derived* too, and not
  checked in the game.
- ~~What `IControl` does with the HQ bit~~ — **read**: query 13, value 117,
  `iron3d.dll`'s `IsHQ`. What an HQ's guard target does
  ([An HQ unit in play](#an-hq-unit-in-play--read)) is still open.
- The Large transport's second slot, and which Large builder socket takes the
  module. **No module names the `Base_*` nodes.** A search of every DLL for
  the string, and for `Base`/`BASE`/`ase_` as a four-byte constant, finds
  nothing. The unit writer takes each part's node from the design's slot
  records instead: three arrays of 0x330-byte records at design `+0xd220`,
  `+0x1a0c4` and `+0x26f68`, the node at `+4` and the item at `+0x20`
  (`0x10054767`, `0x10054877`). What fills them was not read.
- ~~The camera shake's constants and what triggers it~~ — **read**: a jolt in
  the machine's velocity, 2.5 s blend, cos(1.5π t) ÷ (t + 1)³ ring-down
  ([The camera shake](#aiming-and-the-camera--read-and-measured)).
  **Camera values 3–5 (1, 150, 1)** have no reader found. The four class-24
  components on the hero turret are its weapon arms
  ([29-weapons.md](29-weapons.md#the-button-reaches-the-selected-guns)).
- ~~Whether the engine plays a channel's frames linearly in its value~~ —
  **read** and **measured**: slerp between even keys. **How the HUD draws the
  aim point** was not traced. No crosshair object appears in `ui/hq.cfg` or
  `ui/cursor.cfg` (whose `TARGET` is a hardware cursor, `ui/target_5.ani`).
- What prevents the player from building the six free turrets — **narrowed**:
  five are absent from every player tree and the hero's chassis from every tree
  that holds its turret ([The turrets the player never builds](#the-turrets-the-player-never-builds--measured)).
  Which state bits the design screen tests is the next handle.
- How often the input update runs, and the 0.5 mouse sensitivity
  `iron3d.dll:0x10061a50` sets in some screen states
  ([14-controls.md](14-controls.md#the-ini-reaches-world3d--read)).
