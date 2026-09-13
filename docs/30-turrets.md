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

**The camera** (class 4, constructor `0x100233d0`) creates a view and hands it
its values 0, 1 and 2 (`0x100238b0`, *read*). By their numbers they are a near
plane, a far plane and a field of view of 1.3 rad (74°); the names are a
*guess*.

**Turn rates.** No turret authors a turn rate or a limit (*measured*): all 58
controllers in `turrets.rlb` carry the constructor's defaults on every frame
slot ([13-control.md](13-control.md)). The player's input aims the turret
through `CICLS_TURRET`, with `MCMD_ANGLE_X` (0.15, wrapping) and `MCMD_ANGLE_Y`
(0.25, not wrapping) ([14-controls.md](14-controls.md)): a turret turns all the
way round and pitches within limits. Where those limits come from was not
traced.

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
- **The radar and deflector parts don't use sockets.** They attach to the
  turret's named body nodes (`BTmn_m1o1`, `LTdef_m1o1`, `GP_m1o1`, …).

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
- Which axis the mounting bit mirrors, and where a turret's pitch limits and
  turn speed come from.
- What `IControl` does with the HQ bit, and what makes an HQ unit different in
  play beyond its profile and the orders it is given.
- The Large transport's second slot, and which Large builder socket takes the
  module.
- Camera values 3–5 (1, 150, 1), and the four class-24 components on the hero
  turret.
- What prevents the player from building the six free turrets.
