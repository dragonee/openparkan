# Builders and transports

The two units that work instead of fighting. A **builder** puts up buildings
and upgrades them; a **transport** carries ore from a mine to a storage. Both
are ordinary assemblies with a different turret, and both are driven by
`Behavior.dll` tasks the engine names in its own log strings: `Task_Build`,
`Task_Upgrade`, `Task_Transport`.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured* is
re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## A unit's class word is its Type — *measured*

The second `uint32` of every `UNITS/**/*.dat` ([07-objects.md](07-objects.md#unitsdat--unit-and-building-assemblies)),
the class word, is the object's **Type** ([30-turrets.md](30-turrets.md)), the same number the
mission property `Type` carries and `varset.var` names:

| class word | varset name | assemblies |
|---|---|---|
| `0x01002000` | — (transport) | 13: the 9 in `TRANSPRT`, 4 in `AI` |
| `0x01004000` | `ROBOT_BUILDER` | 16: the 8 in `BUILDER`, 4 in `AI`, and two factory designs in `UNITS/`, each written twice |
| `0x01008000` | — (warrior) | 311 |
| `0x01010000` | — (HQ) | 15 |
| `0x01020000` | — (hero) | 19 |
| `0x20000000` | `CLASS_ANIMAL` | 8 |
| `0x800000xx`… | `BUILDING_MINE`, `BUILDING_BUNKER_SMALL`, … | every building; the mast is `0x80000080` and the little teleport `0x80000100`, which varset does not name |

**A builder is Type `0x1004000` and carries a builder beam; a transport is
`0x1002000` and carries a cargo turret** — all 16 and all 13, and no other
assembly carries either (*measured*; control: 311 warriors carry neither).

The `bld_unit_<n>.dat` and `view_unit_<n>.dat` files loose in `UNITS/` are not
building sites. `iron3d.dll` writes them (`%s\units\bld_unit_%d.dat`), and
`<n>` is a building's logic id (`-2147483604` is `0x8000002c`): they are the
designs a factory has queued, and what its panel shows.

## The builder — *measured*

| part | small | medium | large |
|---|---|---|---|
| turret, "Mobile builder" | `e_tur_lt_03` / `lb_03`, 1 battle slot, 3 energy 5 ore | `e_tur_mt_03` / `mb_03`, 1 slot, 8 / 25 | `e_tur_bt_08` / `bb_08`, 2 slots, 20 / 45 |
| beam module, "Mobile builder" | `e_gun_ls_10`, level 0, free | `e_gun_ms_12`, level 6, 20 / 30 | `e_gun_bs_01`, level 15, 30 / 60 |

The shipped builders are small and medium only; four of the medium ones also
fill a battle slot with a gun:

| assembly | chassis | turret | beam |
|---|---|---|---|
| `BUILDER/b_s_wel1`, `tut3_b`, `AI/sml_bldr`, `swelbld1–3` | S-31 wheel | `e_tur_lt_03` | `e_gun_ls_10` |
| `BUILDER/s_bld_t3` | S-42t track | `e_tur_lt_03` | `e_gun_ls_10` |
| `BUILDER/31bmtrk1`, `33mbtrk1`, `b_m_trk1` | M-42t track | `e_tur_mt_03` | `e_gun_ms_12` (+ a gun) |
| `BUILDER/bld_tst` | M-12 walker | `e_tur_mt_03` | `e_gun_ms_12` |
| `BUILDER/M7_M_BLD` | M-2f flyer | `e_tur_mb_03` | `e_gun_ms_12` (+ a gun) |

Builder turrets are numbered `_03` on small and medium and `_08` on large;
turret numbering is in [30-turrets.md](30-turrets.md).

### The beam is class 30 — *read*, and *measured*

Each builder module's controller carries **two components of type 30** and
nothing else, emitting `bld_l_01` (small and large modules) or `bld_m_01`
(medium): `BULL` records with one 1-hit-point node, a controller whose top
speed is 10,000, and a `builder_tail` trail. `bld_b_01` exists and nothing
fires it. No other controller in `guns.rlb` has a type 30 (*measured*).

`Control.dll`'s component factory builds type 30 with **the gun's own class**
(`0x100294c0`, `0x1002d672`), and treats 2 and 30 alike where it tests
(`0x1002c15d`, `0x1002e5ed`). The difference is in `Behavior.dll`: its device
lists file a type-30 part apart from the guns (`0x1001c137`), so a beam is
not a weapon to the behaviour code, and both builder tasks refuse to run
without an intact one — a type-30 device whose value `0x400` is above zero
(`0x10029420`, `0x10033250`).

## Building a building — *read*

`ORDER_ROBOT_BUILD` (7) makes `M_Task_Build` (vtable `0x10059c60`). Its target
is a building Type and either a place (`TARGET_BY_PLACE`, 0x202, the ground
height taken there) or a full placement matrix (0x206).

**What gets built.** `ArealMap.dll` registers the twelve `BuildDat.lst`
schemes by name, each for one building Type (`0x1001ce90`): `Bunker_Small`
`0x80010000` … `Tower_Large` `0x80200000`. **All twelve are read**; the
file's "There must be 11 schemes" is stale. A scheme's `.dat` list is its
**upgrade ladder**: a builder builds the first, and each upgrade moves the
building one entry on (below). *Measured*: every assembly in a scheme carries
that scheme's Type; the mast, the little teleport, bridges and ruins are in
no scheme, so no builder puts them up.

**What it costs.** The task loads the scheme's first `.dat` and walks its
parts, asking the mission's research tree for each: every part must be
researched, or the log says "Cannot build"; and **the building's ore cost is
the sum of its parts' build ore** (`0x10029810`, into `+0x120`; the parts'
build energy is summed beside it). The `Building_Cost` constant (100) is not
the price — it is a threshold, below.

By the parts catalogue's `BuildOreCost` (*measured*; a mission's `.trf` can
differ — [16-research.md](16-research.md)):

| scheme | first building | ore | its upgrades |
|---|---|---:|---|
| Generator | `gener01` | 110 | `gen_l_n1` 110 |
| MainTeleport | `mtp_m_n1` | 395 | — |
| Mine | `smine01` | 540 | `mmine01` 1,340, `lmine01` 3,460 |
| Institute | `sinst01` | 560 | 655, 800, 900 |
| Storage | `sstore01` | 670 | 1,015, 1,410 |
| Bunker_Small | `sbunk01` | 695 | 805, 840 |
| Hangar (the Outpost) | `shang01` | 720 | — |
| Plant | `splant01` | 770 | 1,215, 1,910 |
| Bunker_Medium | `mbunk01` | 1,080 | 1,210, 1,375 |
| Tower_Medium | `mtow01` | 1,100 | 1,195, 1,325 |
| Tower_Large | `ltow01` | 1,595 | 1,790, 1,880 |
| Bunker_Large | `lbunk01` | 1,646 | 1,751, 1,695 |

Every first building but the mine fits in the 2,000 a builder carries
(*measured*).

**The steps** (`0x10028b80`; state names from `0x100284f0`):

1. **Decide** (`0x10028ff0`). A **mine** sends the builder straight to the
   site. So does holding at least `Building_Cost`, 100 ore. Otherwise it looks
   for the nearest mine or storage of its clan holding more than 1.5 ×
   `Building_Cost`, 150 (`0x10029110`); with none, the task cannot run.
2. **GoToStorage** — to that holder's ore place, at the unit's speed ×
   `Transport_SpeedPercent`.
3. **OreOnBoard** — take ore at `Transport_OreOnBoardPerSecond`, 100 a second,
   until it holds the building's cost.
4. **GoToBuild** — to the site at the unit's speed × `Build_SpeedPercent`.
5. **Wait**, then **create the building** (`0x10029240`) through
   `CreateObjectFromScheme` (`0x1001d440`) with the scheme, the matrix and the
   builder's clan — refused if its bounding sphere hits another building —
   and take the cost out of the builder's ore. On failure the game is told
   "Builder … failed to build" (`0x1000c9d1`). A building made this way is
   created in build mode, and `CreateObjectFromScheme` gives it **order 18 with
   parameter 0** at once (`0x1001e007`): the construction sphere below. The
   builder itself does nothing more.

Three things follow from the code as written:

- **The wait is no wait.** It builds once `now − +0x128 ≥ 1000 ms`, and
  `+0x128` is only ever set to zero (`0x10028606`, `0x10028b1d`, `0x10028d69`),
  so the building appears the tick the builder arrives.
- **Holding 100 is enough to set off**, whatever the building costs: the
  decision only compares with `Building_Cost`, and creation takes the full sum.
- **So a builder's ore goes below zero** (*read*). The takt subtracts the cost
  through `MBehaviour`'s property setter (slot 18, `0x100092c0`). The setter
  stores the value into the property table as it is (`0x100269e0`), with no
  floor and no cap, and then only marks it changed for the network. A builder
  holding 150 that puts up a 1,080-ore bunker is left at −930. Its next build
  fetches ore until it holds that building's cost again, and the room it loads
  into is `MaximumOre` minus a negative number.
- **It pays even when the building is refused** (*read*). `CreateBuilding`
  (`0x10029240`) returns nothing to test. On a refusal it reports "failed to
  build", and the takt (`0x10028e9a`) takes the cost and ends the task
  regardless.
- *Measured*: builders start with 200 or 2,000 ore, and 11 of the 12 schemes'
  first buildings cost more than 200. So a builder placed with 200 goes into
  debt on any building but a generator.
- **A mine is never fetched for** — the builder goes straight to the site with
  whatever it holds — and the large mine, 3,460, is the one level no builder
  could carry.

Nothing in the three tasks asks the unit's size (a search for property
`0x201` in them finds nothing), so a small builder builds what a medium one
does.

**The speeds and the distance** (*read*). `Build_SpeedPercent` and
`Transport_SpeedPercent` are compiled as 1.0 (`0x10016250`; no file names them),
so a builder and a transport walk at their full speed, held as every walk is
([31-packages.md](31-packages.md#how-a-walks-speed-is-held--read)).
`Build_BuildDistance` (150) is **never read**. The constants block is reached
only through two getters, `0x10014650` and `0x10014660`, 83 calls in all.
Following each result finds reads of 34 of the block's fields and none of its
`+0x3c`.

**The menu's build and upgrade rows need an intact beam too**
(`iron3d.dll:0x10076da0`, [31-packages.md](31-packages.md#the-commanders-menus--measured-and-read)).

## Upgrading a building — *read*

`ORDER_ROBOT_UPGRADE` (24) makes `M_Task_Upgrade` (vtable `0x10059c20`).

- **Target** (`0x100332e0`): a building of the builder's own clan, by logic id,
  whose level (property `0x209`) + 1 is still inside its scheme — else "dead,
  enemy or fully upgraded building". It needs an intact beam.
- **GoToBuild**: a random point beside the building (`0x100338a0`).
- **On arrival** the builder becomes **invulnerable** (property 162) and the
  **old building** is given order 18 with parameter 1 (`0x100335a1`) — the
  construction sphere below.
- **50 seconds later** (`0x1003363f`) the old building is removed, the scheme's
  next `.dat` is created at the same matrix with level + 1, the old building's
  ore is carried over, and the new building is given order 18 with parameter 2
  (`0x10033790`).
- **Then the builder waits** while the new building's property `0x20c` reads 1,
  and when it stops, drops its invulnerability and is done. `0x20c` is 1 exactly
  while the building's current task is order 18 (`0x1000a82c`): "the sphere is
  still running" — which is also what a transport refuses a mine or storage for,
  and an upgrade a building.

So an upgrade walks a building up its scheme — `smine01` → `mmine01` →
`lmine01`, `sbunk01` → `sbunk02` → `sbunk03` — and **the task charges no ore**:
its only reads and writes of ore are the building's own, moved across.
**Nothing else charges for it either** (*read*, as a search). The menu's
Upgrade entry gives the order and tests no ore (`iron3d.dll:0x10078f60`,
`0x1007bbb0`). Across the install, the ore property `0x2000100` is pushed only
by `Behavior.dll`'s own tasks and systems:
- the distributor and the building place tick;
- the default capacities (`0x10008790`) and a mission property;
- the mine, research, construction, build, transport and upgrade tasks.

`iron3d.dll` writes it once, into an order for a mine. No other module names
it. None of these is on an upgrade's path but `M_Task_Upgrade`. If the
building changes hands on the way, or reads `0x20c` = 1 before the builder
arrives, the builder drops its invulnerability and stops.

## The construction sphere — *read*, and *measured*

Order 18 makes the hidden `ShowUpgrade` task (vtable `0x10059ae8`) **on the
building**. It is a list of timed phases picked by the order's parameter
(`0x10031150`); each phase has a code, flags, a "clear the area" switch and a
length in seconds, and the task steps to the next when the length runs out
(`0x10031680`):

| parameter | given | phases (code, seconds) | total |
|---|---|---|---:|
| 0 | a new building (`0x1001e007`) | 1 for 5 · — for 25 · — for 5 · 2 for 5 · 0 for 1 | 41 s |
| 1 | the building being upgraded (`0x100335a1`) | `0x309` for 25 · — for 1 · 8 for 90 | 116 s |
| 2 | the building an upgrade made (`0x10033790`) | 10 for 3 · 0 for 1 | 4 s |

What a phase does when it starts:

- **Its code goes to the building's controller** (IControl slot 19,
  `Control.dll:0x10004800`), except `0x309`, and is kept as behaviour property
  `0x205`. A building controller's states each carry a request code at `+0x98`;
  a state applies only when that is the current code or −1
  (`Control.dll:0x10001140`). Entering a state runs the state's **action group**
  (`+0x90`, section 5; `Control.dll:0x1000c37c`, interpreter `0x10002800`).
- **Clearing the area.** Every unit within the sphere's radius + 15 (on start)
  or + 20 (on a phase change) is ordered `ORDER_ROBOT_LEAVE` to radius + 20
  from the building, unless it is already leaving or upgrading. A leaving unit
  keeps going while the building's code is 1 or `0x309` or `0x20c` is 1
  (`0x1002c1ba`). This is the builder "escaping" — and, on an upgrade, the
  builder is exempt because it is on `ORDER_ROBOT_UPGRADE`.
- **The sphere as an obstacle.** A flag raises the sphere: the building's
  ground-plan obstacle becomes an octagon round the sphere, radius
  r / cos 22.5° + 20 (`0x1000a6f3`), re-registered on the areal map
  (`0x10006220`), and property `0x202` reads the sphere centre's height instead
  of 0 — which is what refuses "Go Inside Non-complete Building". When the
  task ends the obstacle goes back to the building's own outline.
- The last phase of parameters 0 and 2 carries a flag value 2 that the task's
  step does not test (it tests 1, 8 and 4); what reads it is *unknown*.

**What the building's controller does with the codes** — *measured*: every
one of the 30 `fortif.rlb` building controllers has 14 states, and the codes
6, 1, 2, 0, 8 and 10 each open exactly one of them; no state in any other
archive (1,270) has a code. The action groups (a plant's, the rest alike):

| code | action group |
|---|---|
| 1 | start effect 9002 — the **sign** (`B_Sphere_Sign`: glow, `build_sign.wav`) |
| 2 | its state kills inside the sphere; the state before it on the chain starts 9100 — the **dome** (`B_Sphere_Main`: four `NE_Shield3`, `build_sphere.wav`) — and 9001 — the **ray** (`B_Sphere_Start`: plasma, lightning, `build_ray.wav`), stops the sign, and **kills inside the sphere** |
| 8, 10 | **kill inside the sphere** |
| 0 | stop the ray |

Which state each code opens is *measured*; the path the controller takes
between them (its transition table) is a *guess*. Code 6 is in every
controller and in no phase.

The **kill** (action 21, `Control.dll:0x100033e6`) takes the building's
construction sphere, finds every world object of classes `0x4`, `0x10` and
`0x400` inside it, and kills each through its life system. That these classes
are units is a *guess*.

**The kill repeats every 250 ms while the code is held** (*read*, and
*measured*). The controller runs its action group each time it takes a state
off its queue ([24-motion.md](24-motion.md#playing-a-state--read-and-measured)).
Whenever the current state is an anchor that still applies, the planner
(`Control.dll:0x100051c0`) queues the way back to it, `0x10004f50` walking the
predecessors of the graph search rooted at that state (`0x100019d0`). That
search starts every state at its own edge into the target, the target's
self-edge included.
*Measured*, on all 30 `fortif.rlb` controllers:
- **The three kill states are anchors.** Each state asked for by code 2, 8 or 10
  is fixed at a 250 ms step, with its boxes switched off, so load does not
  scale its costs.
- **Each has a self-edge of cost 1**, and the planner's way back from it is
  that edge alone. The code-8 state has no other way out.

So while the building's code stays 2, 8 or 10, the state takes itself again
every step and kills inside the sphere four times a second:
- about 20 times in a new building's 5-second code-2 phase;
- for the rest of the upgrade on the old building, from 26 s until it is
  replaced at 50 s;
- about 12 times in the new building's 3-second code-10 phase.

The state before code 2's, which starts the dome and the ray, kills once more
as it passes.

**The sphere** is `CBuilding`'s construction sphere (`Terrain.dll:0x1005bd70`):
built round the building's outer contours ("Illegal placement" without them),
with 15 more radius on a mine (`0x1005c50c`).

So a **new building**, which appears the moment the builder arrives, shows the
sign for 5 s; for the next 30 s it sends everyone out, with the dome's obstacle
up for the last 5; then the dome, the ray and the kill come on for 5 s, the ray
stops, and a second later the task ends and the building is done — 41 s. An
**upgrade** sends everyone but the builder out of the old building's sphere for
25 s and then holds the dome with its kill (code 8); at 50 s the upgrade
replaces the building, and the new one kills once more (code 10) and finishes
in 4 s. *Measured*: 24 of 30 controllers name the three sphere effects; the 5
bunkers and towers use `B_Sphere_Start_BT` for the ray; the six without are the
ruins and main teleports, which nothing builds.

### The beam — *read*, and *measured*

Nothing in the build, upgrade or sphere tasks sets a unit's fire state (the
only such writes are in the attack code, `0x10024f99`–`0x10025b74`), so **a
builder never fires its beam to build**; the dome, ray and kill all belong to
the building. The beams' rounds, `bld_b_01`, `bld_l_01` and `bld_m_01`, have
1 hit point and **no explosion** — the only rounds of the 66 without one — so a
hit would do nothing (*measured*).

**Nothing fires it at all** (*read*, as a search):
- **The AI.** `Behavior.dll` sends a fight state (`0x200`, interface `0x202`
  slot 6) in two places. The component index comes from a gun record of a
  turret's gun list (`0x10024f99`), or from a network message repeating one
  ("Fireing", `0x1002536e`). A third `0x200` near them (`0x1002599c`) is a
  property *read* through the device manager. A type-30 part is filed in the
  turret's other list (`0x1001c137`), which only the beam checks read.
- **The player.** `World3D.dll`'s row handler sends a state to the components
  of the class a `.tbl` row names, found by an exact type match (the device
  manager's slot 17, `Control.dll:0x1002c3c0`). No `CICLS_` name is 30, so no
  row can reach a beam, and the selection it starts from takes only types 1, 2
  and 4 (`World3D.dll:0x1000ed20`).
- **The rest.** No other module sends `0x100` or `0x200` through a component
  slot but `Control.dll`'s own parts.

A player's builder cannot fire its beam either.

## Transporting ore — *read*, and *measured*

`ORDER_ROBOT_TRANSPORT` (6) makes `M_Task_Transport` (vtable `0x10059ca8`). **It
ignores its target** (`0x10031f70`): a transport chooses its own route.

| part | small | medium | large |
|---|---|---|---|
| turret, "Cargobot" | `e_tur_lt_04` / `lb_04`, 1 slot, 8 energy 6 ore | `e_tur_mt_04` / `mb_04`, 1 slot, 15 / 30 | `e_tur_bt_07` / `bb_07`, 2 slots, 18 / 35 |

Shipped: S-42t and S-31 with the small turret (`22strn1`, `s_trn_t3`,
`t_s_trk1`, `tut3_t`, `AI/sml_cargo`, `sweltrn1–3`), M-32 with the medium
(`41mtrn1`, `41mtrn1n`, `t_m_wel1`, `t_m_wlk1`), and one L-12w walker with the
large (`34tbwlk1`). Six of the nine in `TRANSPRT` fill a battle slot with a red
laser (`e_gun_lc_03`, `e_gun_mc_20`).

- **Capacity is one number.** Every placed transport and builder carries
  `MaximumOre` 2,000 — `Transport_MaxOre` — on small, medium and large chassis
  alike; transports start empty, builders with 200 or 2,000 (*measured*). No
  part of the assembly carries a cargo figure.
- **The route** (`0x10032870`): the **nearest mine of its clan whose loading
  place is free** (`0x10032a00`) and the **nearest storage** (`0x10032c50`). Only
  mines and storages: a transport never delivers to a factory or a research
  centre, which draw from the holders themselves
  ([23-economy.md](23-economy.md#how-ore-reaches-a-consumer--read-after-two-corrections)).
  With either missing the task cannot run. With ore aboard it heads for the
  storage, otherwise for the mine.
- **The places.** Each mine model has exactly one ground-level loading place
  (hall-way flag `0x8`) and each storage one unloading place (`0x10`); no other
  building has either (*measured*, [27-ownership.md](27-ownership.md)).
- **Loading** (`0x10032000`): 100 a second (`Transport_OreOnBoardPerSecond`),
  never more than the mine holds or the transport has room for. **Whichever
  runs out first ends it** — so a transport leaves a mine that has run dry
  part-loaded. *Derived*: a mine makes 50 a second
  ([23-economy.md](23-economy.md)) against the transport's 100, so a full mine
  of 500 empties in ten seconds and the transport leaves with about 1,000.
- **Unloading**: 100 a second into the storage, never more than it has room
  for. When the storage had less than 0.1 free as the tick began, the
  transport **steps aside** (state 5): it tries up to 100 random points within
  30 of the storage's unloading place, both ways on each axis, and walks to the
  first its walker takes at a quarter of its speed (`0x100322fd`). When a tick
  empties its cargo or fills the storage, it heads back to the mine (state 1).
- **Waiting at a full storage** (state 5, `0x10032681`, *read*). Each tick it
  looks again at its storage and its mine:
  - either one gone ends the task ("Task Ended");
  - either one belonging to another clan keeps it waiting;
  - once the storage has **more than 0.5 free**, it sets off for the mine
    (state 1). The go command (`0x100327b0`) picks the mine and the storage
    afresh.

  It still carries what it could not unload. At the mine it tops up to its
  capacity and returns (state 2, then 3), so a transport kept waiting goes on
  shuttling between a mine and a storage with room. The state names end at
  OreOffBoard, and the task's own code never sets state 6, whose handler
  would send it to the storage.

## Not established

- ~~Whether anything charges for an upgrade outside `M_Task_Upgrade`.~~ Nothing
  does.
- ~~What the ore setter does when a building's cost exceeds what the builder
  holds.~~ It stores the negative result.
- ~~Whether a builder ever fires its beam, and how often the sphere's kill
  repeats.~~ Nothing fires it; the kill repeats every 250 ms.
- ~~The values of `Build_SpeedPercent` and `Transport_SpeedPercent`, and any
  reader of `Build_BuildDistance`.~~ 1.0 and 1.0 (already in
  [24-motion.md](24-motion.md#how-the-ai-asks-for-speed--read)); nothing reads
  the distance.
- ~~What a transport does after waiting at a full storage.~~ Goes back to the
  mine once the storage has more than 0.5 free.
- What a mine's "ToMine", the amount of the lodes within 250, does to its
  output ([31-packages.md](31-packages.md#mineral-lodes--read-and-measured)).
