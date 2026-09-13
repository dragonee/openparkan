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
| Hangar | `shang01` | 720 | — |
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
   "Builder … failed to build" (`0x1000c9d1`).

Three things follow from the code as written:

- **The wait is no wait.** It builds once `now − +0x128 ≥ 1000 ms`, and
  `+0x128` is only ever set to zero (`0x10028606`, `0x10028b1d`, `0x10028d69`),
  so the building appears the tick the builder arrives.
- **Holding 100 is enough to set off**, whatever the building costs: the
  decision only compares with `Building_Cost`, and creation takes the full sum.
  What the ore setter does with a result below zero is not read.
- **A mine is never fetched for** — the builder goes straight to the site with
  whatever it holds — and the large mine, 3,460, is the one level no builder
  could carry.

Nothing in the three tasks asks the unit's size (a search for property
`0x201` in them finds nothing), so a small builder builds what a medium one
does. `Build_BuildDistance` (150) is bound by name and no read of it was found.

## Upgrading a building — *read*

`ORDER_ROBOT_UPGRADE` (24) makes `M_Task_Upgrade` (vtable `0x10059c20`).

- **Target** (`0x100332e0`): a building of the builder's own clan, by logic id,
  whose level (property `0x209`) + 1 is still inside its scheme — else "dead,
  enemy or fully upgraded building". It needs an intact beam.
- **GoToBuild**: a random point beside the building (`0x100338a0`).
- **On arrival** the builder becomes **invulnerable** (property 162) and an
  order 18 goes out — the hidden `ShowUpgrade` task (`0x10059ae8`), the
  construction sphere; that it goes to the building is a *guess*.
- **50 seconds later** (`0x1003363f`) the old building is removed, the scheme's
  next `.dat` is created at the same matrix with level + 1, the old building's
  ore is carried over, and the new building is given order 18.
- **Then the builder waits** while the new building's property `0x20c` reads 1,
  and when it stops, drops its invulnerability and is done. That `0x20c` = 1
  means "under construction" is a *guess* that fits all three places it is read:
  here, the transport refusing such a mine or storage, and an upgrade refusing
  such a building.

So an upgrade walks a building up its scheme — `smine01` → `mmine01` →
`lmine01`, `sbunk01` → `sbunk02` → `sbunk03` — and **the task charges no ore**:
its only reads and writes of ore are the building's own, moved across. Whether
something outside the task charges for an upgrade is not established. If the
building changes hands on the way, or reads `0x20c` = 1 before the builder
arrives, the builder drops its invulnerability and stops.

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
  for. When the storage has less than 0.1 free the transport **waits beside
  it** (state 5), trying up to 100 random nearby points (`0x100322fd`).

## Not established

- Whether anything charges for an upgrade outside `M_Task_Upgrade`.
- What the ore setter does when a building's cost exceeds what the builder holds.
- What the new building's creation mode (2 for a build, 4 for an upgrade, passed
  to `CreateObjectFromScheme`) changes, and whether a freshly built building runs
  the `ShowUpgrade` sphere too.
- When the beam is actually fired, and what hitting something with `bld_*_01`
  does: no firing from the three tasks was traced.
- Property `0x20c`: what sets it, and whether it is "under construction".
- The values of `Build_SpeedPercent` and `Transport_SpeedPercent`, and any reader
  of `Build_BuildDistance`.
- What a transport does after waiting at a full storage (state 5's handler).
