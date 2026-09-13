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
mirrors its aim on it: `0x100271c7` and `0x100276ed` when set, `0x100289b5` when
clear. So a hanging turret turns the same way on screen as a standing one. Which
axis is mirrored was not traced.

A second bit, **`0x8000000`, marks the HQ turrets** (*measured*). It is set on
all 14 HQ records, both mountings. Of the rest, only `o_tur_lb_06` carries it,
and its `t` twin doesn't: a slip in the data, by the look of it (*guess*).
`IControl`'s getter tests the bit (`Control.dll:0x1002b7bb`); what for was not
traced.

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
  on 5 and 0.273 on the hero's. On the hero that start is level (below).
  If it is level on the rest too, the limits are −10°…+80° and −15°…+75°
  (*guess*).
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
- **A machine's turret yaw** is also added into object `+0x1ec`
  (`0x1002eb70`). The control takt sets the turret's yaw from it
  (`0x100059a0`, when `+0x65c` is set and no turn is pending). What `+0x65c`
  and `+0x1f0` are was not read.

**The first-person eye** (*read* `0x100234c0`; *measured* on the hero):

- **The camera's channel** points at `TargetDirect`, with `CameraCenter` the
  point just before it. This holds on all 58 turrets.
- **Position and direction.** The camera takes a position and direction from
  one point and a direction from the other (`0x10023603`, `0x10023618`), then
  adds a shake offset × 0.02.
- **Which point is which** is a *guess*, by the data: position from
  `CameraCenter`, look from `TargetDirect`.
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

Which code writes the class word when the player designs a unit was not traced.
The unit writer `iron3d.dll:0x100544b0` writes the `0xf0f1` magic at
`0x10054f5f` and the class word after it.

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
so a tree's part list doesn't gate them. What stops a factory building one was
not traced. That a zero cost marks them unbuildable is a *guess*.

## Not established

- Which code sets a newly designed unit's class word, and whether it reads the
  turret's research role byte.
- Which way a mouse count tilts an upright turret on screen: the sign the 1 − v
  stored at init and read back gives it.
- What `IControl` does with the HQ bit, and what makes an HQ unit different in
  play beyond its profile and the orders it is given.
- The Large transport's second slot, and which Large builder socket takes the
  module.
- Camera values 3–5 (1, 150, 1), the camera shake's constants (`+0xa4` 3,
  `+0xa8` 3, `+0xac` 2.5) and what triggers it, and the four class-24
  components on the hero turret.
- Whether the engine plays a channel's frames linearly in its value, and how the
  HUD draws the aim point.
- What prevents the player from building the six free turrets.
