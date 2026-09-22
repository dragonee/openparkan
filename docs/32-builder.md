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

## Placing a building — *read*, *measured* and *seen*

The player puts a building down from the command view: a Build row of the
builders' page ([31-packages.md](31-packages.md#the-commanders-menus--measured-and-read)),
then a full-size model under the cursor, red or green, turned with `<` and `>`
(T03_H08), and a click. What the view, its camera and the cursor otherwise do is
[40-command-mode.md](40-command-mode.md)'s; the page is
[41-commander.md](41-commander.md)'s.

### The Build row starts a pick on the first builder

The HQ executor (`iron3d.dll:0x1007b740`) gathers the selected records of the
player's clan (`0x10076e70`, which answers nothing once the game's state word
`+8` is not 4). For rows 10–16 it writes a pending pick into the **first**
record only (`0x1007ba09`–`0x1007bad5`), a small block at the record's `+0xa8`:

| offset | holds |
|---|---|
| `+0xa8` | pending, 1 |
| `+0xac` | the pick's kind: 1 Route, 3 build, 4 Guard, 5 Capture building |
| `+0xb0` | the building Type: `…04` mine, `…08` storage, `…10` plant, `…40` Outpost, `…400` institute, `0x80100000` light tower, `0x80200000` heavy tower |
| `+0xb8` | 0 (the stage) |
| `+0xbc` | the record itself |

The record's takt runs a pending block (`0x10075d1b` → `0x10079700`, a switch
on the kind); the build kind is `0x10079c40`, in two stages:

1. **Start** (`0x10079e57`). The game's pick mode `0x1010c388` becomes **6 for a
   mine and 4 for any other building**; the cursor kind `0x10104148` becomes 8,
   the model (`0x100571a0`), under which the commander panel draws only its
   resource rows ([41-commander.md](41-commander.md)); the model is made for the
   Type (`0x10057f00`); the record's placed byte `+0x131` is cleared.
2. **Waiting** for `+0x131`. When it is set (the click, below) the model goes, the
   cursor kind becomes 2, and the builder is given its order: `ORDER_ROBOT_BUILD`
   (7), the Type as its parameter, target kind `0x206` with the 4 × 4 placement
   matrix copied from the record's `+0xec` (`0x10079d45`–`0x10079d8d`), through the
   record's `+0x44` slot 3 with **insert mode 3, replacing** (`0x10079d95`). A
   player's unit then acknowledges with a voice as the wingman menu's orders do
   ([31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)).

### The model under the cursor

**Which model** (`0x10033830`). The Type picks a `FORT` record of `objects.rlb`,
and `World3D.dll!CreateObject` class 3, a building, loads it
(`0x10057f68`): `fr_l_mine`, `fr_l_store`, `fr_l_plant`, `fr_l_angar`,
`fr_l_inst`, `fr_l_towL`, `fr_l_towH`, and for the bunkers `fr_l_bunker`,
`fr_m_bunker`, `fr_b_bunker`. *Measured*: each is the root part of its scheme's
first `.dat` in `BuildDat.lst` (`smine01.dat`'s root is `fr_l_mine`, and so on),
a `FORT` record whose first slot is the building's agent and whose second is its
`.bas`. So the model is the bare building the builder will put up, without its
batteries, shields and guns. The object is told message `0x101` as it is made,
and `0x103` before it is deleted (below).

**Where it stands** (`0x10058020`, run by the cursor's draw each frame while the
cursor kind is 8, `0x100585b0`):
- **The pick.** A ray from the camera through the cursor
  (`0x10035e40`) goes into `Terrain.dll!GetWorld`'s segment query, slot 7
  ([29-weapons.md](29-weapons.md#where-the-round-leaves-and-which-way)), with a
  query record of its own whose first word is `0xa` (`0x10035e82`); what that
  record asks for is not established. The hit counts only strictly inside the
  map, a margin of 0.001 of its size in from each side.
- **A pick that misses** leaves the model where it was. The first miss after a
  hit says `VOICE_POINT_LAND` (`vc_003.wav`, *measured*) when the sound server's
  slot 10 answers true (`0x10058340`), and a miss as the model is made says it
  too.
- **A hit** places the model at the hit point, turned about z by its yaw (`+0x54`
  of the model's object `0x1010b540`): the matrix is `Rz(yaw)` with the hit as its
  translation (`0x1005810d`), counter-clockwise from +x for a positive yaw.
- **The yaw starts at 0** each time a model is made (`0x10057f33`).

**Its colour.** With a selection, the model object's behaviour is asked message
`0x102` with the first selected record's logic id (`0x10033d10`, below), and the
answer colours it (`0x10058239`–`0x10058264`):

| answer | colour | placeable (`0x1010b598`) |
|---|---|---|
| 0 | `0xffff0000`, red | no |
| 1 | `0xff00ff00`, green | yes |
| 2 | `0xffffff00`, yellow | yes |
| no selection | `0xff000000`, black | no |

In pick mode 6 a **mine off a lode** is red whatever the answer
(`0x10058268`, below). The query returns only 0 or 1 (below), so the model is
red or green. When the game's `+0xe4` byte is set (`0x10033d36`, not followed),
an answer of 1 or 2 also turns to 0 within 400 across the ground of any of the
level's `+0x728` records (`0x10033d80`).

**How it is drawn** (`0x10035f50`): in one flat colour, its red, green and blue
each 1 or 0 by the colour word's bytes, with alpha 1. The camera is put in mode
2 and handed the colour through slot 30, as the HUD panels draw their unit
([35-hud.md](35-hud.md#the-unit-in-the-middle--read-and-seen)), and the model is
drawn through its interface `0x18` slot 11 with flags `0x5f0`. For the draw fog
is off and the depth test is off (`D3DRS_FOGENABLE` 0, `ZENABLE` 0,
`ZWRITEENABLE` 1, `ZFUNC` 8, always; the four put back after). So it shows whole
over whatever stands in front of it. *Seen*, at
183–184 s in the recording of *The Field Base*: a solid red silhouette of the
mine, then a solid green one.

### The test: `IsPlacementValid` — *read*

Message `0x102` is `MBehaviour`'s `IAgent` slot 38 (`Behavior.dll:0x1000b9e0`,
vtable `0x100592e8`), which logs itself as `IsPlacementValid()`. Message `0x101`
starts `CLandscape::StartCheckMaxBasementAngle` for the model and `0x103`
stops it (`0x1000ba17`, `0x1000ba45`). For the query:
1. **The slope limit** is 0.88 when a builder is named and 0.8 without one
   (`0x1000ba63`, `0x1000ba6d`).
2. **The sphere.** The model's `IBuilding` slot 16 (`Terrain.dll:0x1005b680`)
   gives the middle of its outer contour's box and the contour's farthest
   point from there + 5; the query adds 5 more (`0x1000bdd9`).
3. **The path.** With a builder named, unless its chassis type (variable
   `0x207`) is 1, a flyer (`0x1000baf4`), the builder's position and the model's
   are looked up (`0x100366b0`, `0x10043220`) and a path is searched between them
   (`0x10020910`). A search that fails or comes back empty: *"BAD PATH"*, 0.
4. **The map.** The sphere must lie strictly inside the world box
   (`MBehaviour+0x690`) in x and y: else *"Intersects with Boundary"*, 0.
5. **Other buildings.** For every other building (the world's class-3 objects),
   the distance between the two spheres' centres across the ground must be at
   least the model's radius plus the other's (slot 16 again): else *"Intersects
   with …"*, 0 (`0x1000be44`–`0x1000bf21`).
6. **Its vertices.** Each vertex of the model's interface `0x303` whose flag word
   has bit 1 must fall on an areal of `GetSystemArealMap`'s map whose record's
   `+0x20` is set: else *"HallVertex …"*, 0 (`0x1000bf4a`–`0x1000bfa9`). That
   `0x303` is the hall way, and what bit 1 marks, are *guesses*.
7. **The basement** (`CLandscape::CheckMaxBasementAngle`, `Terrain.dll:0x10014c60`,
   through `+0x40` slot 8 at `0x1000c128`). Each outer-contour vertex is dropped
   onto the landscape; every inner-contour vertex is set to the mean of those
   heights; and every face of the basement triangulated between the two rings
   (`StartCheckMaxBasementAngle`'s) has its normal taken
   (`0x1000da20`). The smallest normal z must be at least the slope limit: else
   *"Ugly Basement"*, 0. So no basement face may be steeper than 28.4° with a
   builder (acos 0.88), 36.9° without. The basement here is the constrained
   Delaunay triangulation of the ring between the two `.bas` rings as they
   stand, the outer one's own corners only (`StartCheckMaxBasementAngle`,
   `0x100150f0`: a box, the outer ring's edges labelled 1 inside and 2 outside,
   the inner ring's 2 inside and 1 outside, and `FindMinNormalZProc()` taking the
   triangles labelled 1), the same triangulation the insertion lets in
   ([03-terrain.md](03-terrain.md#the-pieces-are-triangles-of-a-constrained-delaunay-triangulation--read-and-measured)).
8. Otherwise *"Place OK"*, 1.

**The query is the model's alone** (*read*, with a byte search). Its only
callers are `iron3d.dll`'s wrapper `0x10033d10` (`0x10033d29`, `0x10033d66`),
which only the model's class calls. `ai.dll`, `ArealMap.dll`, `Behavior.dll` and
`World3D.dll` make no call through `+0x98` at all. So an AI builder's site is not
put to this test, and `CreateObjectFromScheme` refuses only a sphere that meets
another building's ([Building a building](#building-a-building--read)).

### A mine must stand on a lode

In pick mode 6 the model's position, rounded to whole units, goes to
`0x10072f00`, which asks the lode list (`0x10081ba0`) for **a lode already found
within 20 across the ground**, strictly nearer (`0x10072f07`, `0x10081bee`). With
none the mine is red. The lode's amount, and whether the Search minerals order has
found it, are
[31-packages.md](31-packages.md#mineral-lodes--read-and-measured)'s.

**The plume** (`env_mineral`, [04-missions.md](04-missions.md)) shows only while
no building stands within 80 of its lode: `0x10081c10` asks the level's building
list (`0x100728e0`, any clan, strictly nearer) before drawing each
(`0x10081cd1`). *Seen*: the plume over *The Field Base*'s lode is there at 194.5 s
and gone at 195.0 s, as the mine appears beside it (below).

### Turning it

`CMD_JAMES_BASE_ROTLEFT` (741, `,`) adds **0.05 rad** to the yaw and
`CMD_JAMES_BASE_ROTRIGHT` (742, `.`) takes 0.05 away (`0x100725a7`,
`0x100725dc`, the float at `0x100e50a4`), but only while the view state word
(`+0x710`) is 2 and the pick mode is 4 or 6
([40-command-mode.md](40-command-mode.md) reads the same, at `0x100725b2`).
Whether a held key repeats the turn is not established. Seen from above, a
positive yaw turns the model anticlockwise.

### The click, and cancelling

**The left button going down** (the input listener's slot 8, `0x100714d0` →
`0x1008d690`) goes straight to the placement while the cursor kind is 8, over the
panel or not (`0x1008d70b`). The placement (`0x1008fe80`):
- **on a placeable site** sets `+0x131` on every selected record and copies the
  model's matrix into each one's `+0xec`, and ends the pick mode
  (`0x1008ff2d`–`0x1008ff66`). Only the record holding the pending build acts on
  it, so **only the first selected builder builds**;
- **on a red site** does nothing: the model stays under the cursor.

**Cancelling.** The right button's handler (`0x1008fb00`), in pick mode 4 or 6,
deletes the model, sets the cursor kind to 1, ends the pick mode and shows string
6207, *"Building was cancelled by user"* (*measured*), as a System line
(`0x1008fdaf`–`0x1008fe33`,
[35-hud.md](35-hud.md#the-message-box--read-and-measured)). It is called:
- by **the right button going down** (listener slot 10, `0x100716b0`), outside
  the first-person views and a building's screen;
- by **Esc** in the character handler (`0x10070ed1`), while the cursor is the
  model, after the objectives screen and the like have had it;
- by **`CMD_ROLLBACK_STATE`** (735, bound to Esc) while the cursor is the model,
  before it rolls the view mode back (`0x10062ff7`).

The pending block keeps waiting after a cancel with nothing to set its `+0x131`;
the next Build row writes a new one (*derived*).

### For an engine

1. A Build row puts the first selected builder into a placement for the row's
   Type. Pick mode 6 for a mine, 4 for the others.
2. Load the Type's `fr_*` `FORT` record (the scheme's first `.dat` root) as the
   model. Yaw 0.
3. Each frame: cast the cursor ray into the world. On a hit inside the map, place
   the model at the hit, turned by the yaw. Colour it by the test: green when
   `IsPlacementValid` passes for the first selected builder and, for a mine, a
   found lode lies within 20; red otherwise. Draw it in that flat colour, unlit,
   unfogged and over everything.
4. `,` and `.` turn it by ±0.05 rad a key-down.
5. A left click on a green site gives the builder `ORDER_ROBOT_BUILD` with the
   Type and the model's matrix, replacing its orders, and ends the placement. A
   click on red does nothing.
6. The right button, or Esc, cancels: *"Building was cancelled by user"*.
7. Hide a lode's plume while any building stands within 80 of it.

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

### Building a building, tick by tick — *read*, and *seen*

For the order a placement gives, target kind `0x206`:

1. **The target** (`SetTarget`, `0x100285f0`) is the placement matrix, copied to
   the task's `+0xe0`. A `0x202` place becomes an unturned matrix at the place,
   at the ground's height there (`0x100146b0`).
2. **The start** (`0x10028800`) reads the Type's first scheme `.dat` and sums its
   parts' ore. An unresearched part or an unreadable file ends the task with
   *"Cannot build"*, telling the behaviour's world object message `0x102` (its
   slot 51, `0x10028a03`). Otherwise the chooser (`0x10028ff0`) picks the first
   state, and the go command sets off.
3. **Each takt** (`0x10028b80`) first needs an intact beam (`0x10029420`); without
   one the task ends. Then, by state (`+0x5c`):
   - **3, GoToBuild.** The walk is to the matrix's translation, the building's
     own origin (`0x10029318`), at the unit's top speed × `Build_SpeedPercent`,
     1. When the walker reports its walk done (`0x1003dd80`), the state becomes 4.
   - **4, Wait.** On the next takt, `now − +0x128 ≥ 1000 ms` holds
     (`0x10028e89`), and the building is created (`0x10029240`):
     `CreateObjectFromScheme` with the scheme, the matrix and the builder's clan.
     A refusal tells the world object message `0x103` and logs *"CreateBuilding()
     failed..."*. Either way the cost comes off the builder's ore, the log reads
     *"BuildTask is over"*, and the takt answers 0: **the task ends there**.
4. **The builder is left with no order**, standing inside the building's site.
   The building's order 18 sends it out when the sphere's second phase clears the
   area ([The construction sphere](#the-construction-sphere--read-and-measured)).

**Mission 03's mine** (*measured*, and *derived*). `tut3_b`, an SWB-2 on the
S-31 wheel chassis, carries 200 of its 2,000 ore and stands 85.3 from the
mission's one lode, (1026.1, 942.7), which starts found. No building stands
within 80 of that lode, so its plume shows. A mine goes straight to its site
(`0x10028ff4`), and by the catalogue a small mine costs 540, so the builder is
left owing 340.

**Seen**, in the recording of *The Field Base* (960 × 720, 30 frames a second,
frames taken every half second):

| s | what shows |
|---:|---|
| 183–184 | the mine model, solid red, then green, under the cursor |
| 185.0 | *SWB-2 Builder \[building\]* |
| 194.5 | the lode's plume still up |
| 195.0 | the plume gone |
| 195.5 | a cyan glow at the site, the sign |
| 196.0 | *SWB-2 Builder \[no order\]* |
| 200.0 | *SWB-2 Builder \[escaping\]* |
| 230.5 | *\[no order\]*; rings over the site |
| 231.5–234.5 | the ray, with lightning, and the dome rising |
| 235.0–236.5 | the blue dome over the site, fading at the end |
| 237.0 | the Small Mine standing; the Ore row at 1% |

Against the read sequence
([above](#the-construction-sphere--read-and-measured)), with the building made
between 194.5 and 195.0 s: the sign for 5 s, the clearing about 200 s, the
dome's obstacle and then the ray, the dome and the kill from about 230 s, the ray
stopped at about 235 s and the task done at about 236 s — each within the
half-second sampling. The builder's own task ends as the building appears, and it
walks out once the sign's phase ends. What the site shows of the unfinished mine
before the dome was not made out: the recording's views of it are distant or
behind the dome.

**For an engine.**
1. On a build order, walk the builder to the placement's origin at its top speed.
2. On the takt after it arrives, create the scheme's first building at the
   placement matrix for the builder's clan, take its ore cost from the builder
   (going below zero), and end the builder's task.
3. Give the building order 18, parameter 0: the 41 s sign, clearing, dome, ray
   and kill of [the construction sphere](#the-construction-sphere--read-and-measured).
   The builder goes out with everyone else when the clearing phase starts.
4. Hide the lode's plume from the moment the building exists
   ([A mine must stand on a lode](#a-mine-must-stand-on-a-lode)).

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
  (`Control.dll:0x10001140`). The controller's own code starts at **0**, the
  constructor's (`0x10006ecf`), so a building applies its code-0 state — the one
  that stops the ray — before any phase has sent it anything
  ([24-motion.md](24-motion.md#a-states-use-count-and-its-request-code--read-and-measured)). Entering a state runs the state's **action group**
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
  runs out first ends it**. ~~*Derived*: a mine makes 50 a second against the
  transport's 100, so a full mine of 500 empties in ten seconds and the
  transport leaves with about 1,000.~~ A mine's task writes its running total
  back over what the mine holds every takt, so a mine that has dug its 500 is
  full again after each draw. The transport fills its 2,000, and it is the
  transport's room that ends the loading
  ([23-economy.md](23-economy.md#a-mine-digs-to-500-and-then-a-draw-does-not-empty-it--read);
  *seen*: two loads of 2,000 on *The Field Base*). Only a mine that has not yet
  dug 500 runs dry.
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
- ~~What a mine's "ToMine", the amount of the lodes within 250, does to its
  output ([31-packages.md](31-packages.md#mineral-lodes--read-and-measured)).~~
  It bounds the digging and never scales it: the task ends, "All Ore mined...",
  when what is left is no more than a takt's dig — and since a takt raises the
  total and lowers `ToMine` by the same dig, that bound bites at **half**, so a
  mine yields about `ToMine / 2` or 500, whichever is less. Only the ore dug
  comes off `ToMine`, and all 15 placed mines sit on 999,999 or more, so no
  shipped lode runs out
  ([23-economy.md](23-economy.md#a-mine-digs-to-500-and-then-a-draw-does-not-empty-it--read)).
- What the pick's query record (first word `0xa`, `iron3d.dll:0x10035e82`) asks
  the world's segment query for, so which objects stop the cursor's ray.
- That interface `0x303` is the hall way and what its vertex bit 1 marks, which
  `IsPlacementValid` tests against the system areal map's `+0x20`.
- What the game's `+0xe4` byte is (`0x10033d36`), under which a placement within
  400 of one of the level's `+0x728` records turns red.
- Whether holding `,` or `.` turns the model again on the key's repeats.
- What the site shows of an unfinished building before the dome: the recording's
  views of the mine are distant or behind it.
- ~~How `StartCheckMaxBasementAngle` triangulates the basement between its
  rings.~~ — **read**: as the constrained Delaunay triangulation of the ring
  between the two `.bas` rings, their edges its only constraints, the faces
  between them labelled 1 and measured (`0x100150f0`,
  [The test](#the-test-isplacementvalid--read)).
