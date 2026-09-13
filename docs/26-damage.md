# Damage, shields, armour and repair

Everything a hit does happens in `Control.dll`, in the same control system
that runs a building's power ([23-economy.md](23-economy.md)). The object hands
it out as **`ILifeSystem`** — interface id `0x16` in its `QueryInterface`
(`0x100076f0`), the sub-object at `+4`, vtable `0x1003b59c` — and
`Behavior.dll` refuses to run a unit without it ("Behaviour panic: Cannot
receive ILifeSystem"). Bots and buildings go through the same code.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## Hit points — *read*, and *measured*

Each node of a model has a life, built from its `.ndp` record
([07-objects.md](07-objects.md#ndp-is-a-damage-table-one-record-per-node)):

- **Maximum** = the `.ndp` hit points × the object's **volume scale** (its three
  scales multiplied, `+0x548`, `0x10009ee0`; 1 in every shipped mission) × the
  **level ratio** (`+0x660`, below). Changing either scale rescales every
  node's life and maximum in proportion (`0x10009f90`).
- **Damage** lowers a node's life, clamped at 0 (`0x10010f30`). A part's
  *condition* — the `0x100` bit on component values — is `life / max`.
- **A node at 0 is destroyed.** A node that is destroyed, or steps up a damage
  stage, destroys its child nodes (`0x10011130`). If it is node 0 or carries
  node flag bit 1, the object's owner word (`+0x550`) becomes `0xfffe`
  (`0x10011098`) and **the object dies** — unless its agent kind (`+0x50`, the
  kind `LoadControlSystem` is given) is 3, which is only marked (`0x100110ab`).
  A radar never detects an object whose word reads `0xfffe`
  ([25-sensors.md](25-sensors.md#a-scan-is-a-sphere-a-falloff-and-three-tests--read)),
  so that it passes over wrecks is a *guess* that joins the two. That kind 3 is a
  building is a *guess*; it fits the data: 12 of the 30 building tables in
  `fortif.rlb` give node 0 one hit point beside parts of 40,000–500,000, which
  would make those buildings die at the first scratch.
- **Damage stages.** A node with *N* damage states (the extra mesh blocks) is
  in stage `N − ceil(N × life / max)`; each step up plays the node's `.exp`
  explosion (`0x10011220`).

What the shipped tables give node 0 (*measured*):

| | node 0 hit points |
|---|---|
| chassis `r_t_*` | 90–120 |
| chassis `r_l_*` | 190–450, and one of 83,000 |
| chassis `r_h_*` | 380–1,400 |
| chassis `r_m_*` | 480–1,100 |
| chassis `r_b_*` | 1,500–4,500, and two of 10,000 and 80,000 |
| guns (`guns.rlb`, one node) | 120 small and medium, 500 big, up to 3,000 |
| internal parts (`intsys.rlb`, `o_*`) | 1 |
| bunkers, research centres, towers | 40,000–100,000 |
| bridges | 120,000–200,000 |
| mines, plants, stores, Outposts, generators, the medium `mtp` | 1 on node 0; parts 40,000–500,000 |

## The difficulty ratio — *read*, and *measured*

`Iron_3D.ini` carries `[LEVEL_RATIO] EASY=0.5 MEDIUM=0.7 HARD=1.0`, and
`GAME_LEVEL` picks one: `iron3d.dll` stores the setting at `+0x150`
(`0x10028e93`) and `0x10076010` reads it, 0 `EASY`, 1 `MEDIUM`, anything else
`HARD`. The shipped file says `GAME_LEVEL=1`.

That code gives the ratio, as object property 180, to **every warrior, HQ and
hero** (`Type` `0x1008000`, `0x1010000`, `0x1020000` — the `prof_war`,
`prof_hq` and `prof_hero` profiles) **of a clan not allied with the player's**:
when the mission has loaded (`0x1007d4b0`) and when one is created after. Not
builders, transports or explorers, and never the player's own. Property 180
does three things (`0x1000e980`, `0x1002ba30`):

- scales every node's life and maximum — **enemy combat units have half the
  hit points on easy**;
- scales the fight shield's sector maximum (`+0x11c`, `0x100255c0`);
- is copied to every class-2 gun (`+0x180`), and a gun hands it to each round
  it fires (`0x1002a3fb`) — which scales **the round's damage** (below).

## A hit, from the round to the node — *read*

**The `.exp` is the damage.** Its first word, which
[11-effects.md](11-effects.md) took for a count, is the **hit kind**
(`0x1000ebc0`):

| kind | shipped | what it does |
|---|---|---|
| 1 | 26 — animals, scenery, building and robot debris, `aim` | nothing but the effect |
| 2 | 54 — `bb_*_01` bullets, `bl_*` lasers, `bt_*`, `bp_h`, `bp_l`, `rg_b`; damage 1 on all | the whole damage to the one node struck |
| 3 | 64 — `ba`, `bf`, `bm`, `br`, the big `bb` and `bp`, `fm`, `fr`, `explode_rbr_bomb`; damage 1–800,000 | a blast: shields, then every node in reach |
| 4 | none | handled: shields only, never a node |

and the first two floats are the **damage** and the **radius** — absolute on a
round (agent kind 9), times the node's bounding radius otherwise. When a node
reaches a new damage stage it sends a hit whose damage is

    level ratio × .exp damage + the life the node lost since its last stage

(`0x10011794`). A round dies from full health, so **a round hits for
`ratio × (its .ndp hit points + the .exp damage)`**. The data bears that out
(*measured*): all 29 direct-hit rounds have an explosion damage of 1 and hit
points ending in 9 — 149, 899, 1,699 — so that the sum is round on 28 of them,
where the hit points alone are round on none. A light bullet `bb_l_01` hits
for 150, a big one `bb_b_01` for 900, lasers for 200–1,700.

The hit is queued on the object that exploded and applied on its tick
(`0x10012ce0`):

- **Kind 2** goes to the object and node the round struck.
- **Kind 3** hits the exploding object's own nodes, then **every object whose
  bounds reach the blast**, through `ILifeSystem` slot 8.
- A hit does nothing if the object that fired it no longer exists, if it is
  the target's own, or if the target is **invulnerable** (property 162, `+0x5b0`
  — the `[CS] INVULNERABILITY` debug key sets it on the hero, `0x1005e487`,
  and a unit is invulnerable from its arrival at a building it upgrades until
  the new level stands, `0x1003356f`, [32-builder.md](32-builder.md#upgrading-a-building--read)).

**A blast falls off with overlap** (`0x10010030`). For a node whose bounding
sphere has radius *r* and centre at distance *d* from a blast of radius *R*:

| | damage |
|---|---|
| `d ≥ R + r` | 0 |
| one sphere wholly inside the other | the whole blast |
| otherwise | `damage × ((R + r − d) / 2R)³` |

## The hit test — *read*, and *measured*

**One pass a frame.** The world's frame (`World3D.dll:0x10006bf0`) runs in four
steps, with no substeps:

1. It sends every object message 1, the tick; a round moves and spends its
   range here (`Control.dll:0x1000cbb0`).
2. It runs the collision pass once (`Control.dll:0x1001c040`).
3. It sends message `0x1c`.
4. It delivers what the pass posted: message `0x1b` to every object whose
   contact record was filled, which a round takes as its collision response
   (`0x1000d0c0`).

**Who takes part.** Every agent has a collision object: a kind, an owner, a
swept sphere — a start, an end and a radius — and the object's geometry. The
kind comes from the `objects.rlb` tag (`AniMesh.dll:0x1000317f`):

| tag | records | kind |
|---|---|---|
| `BULL` | 67 | 9, a round |
| `BTLU` | 63 | 4, a unit |
| `WPNS` | 5 | 2 |
| `STAT` | 81 | 10 |
| any other | — | the kind of the object it hangs on |

Only rounds and units carry a contact record. A collision object sets its
start and end to its bounding sphere's centre on message 1 (`0x1001fec0`) and
moves its end there again on message `0x1c` (`0x10020010`); how the two ends
differ when the pass runs between them is *not established*.

**The pass** does three things:

- **A round against the ground.** A round's segment is clipped to the map box,
  whose top is doubled (`0x1001e1e0`), and run through the landscape's
  `GetFirstIntersectedFace` (`Terrain.dll:0x100205c0`). That walks the grid
  cells along the segment from its start, tests each cell's faces with a
  one-sided segment–plane test and a point-in-triangle test, and returns the
  nearest hit in the first cell that has one (`0x1001dbe0`).
- **Every pair, once.** A pair goes further only if one side has a contact
  record and the two swept spheres touch within the frame (`0x1001e9f0`).
- **A round against an object** (`0x1001d630`):
  - **its own shooter is skipped** — a unit whose id is the round's owner gives
    no contact at all (`0x1001d6af`);
  - a unit's **bubble**, its bounding sphere, gives a contact, kept in order of
    distance;
  - then the round's segment is run through the object's mesh.

  No clan is consulted here; a hit on one's own side is dropped later, in the
  hit queue ([above](#a-hit-from-the-round-to-the-node--read)).

**The mesh test** (`AniMesh.dll:0x10013ef0`) takes every node once, and for
each node the triangles of **level 0 of its current variant**, in the node's
frame (`0x10010a50`): a plane from the stream-7 face normal, one-sided, and a
point in the triangle (`0x10011090`). The nearest hit by squared distance from
the segment's start wins, and the record keeps **the object, the node, the
batch, the triangle and the world point** — on the ground, the face, with the
node −1.

A round's query **passes through triangles flagged 4 or 32**, through batches
flagged 8, and through batches flagged `0x200` unless the round's type carries
`0x4000000` (`Control.dll:0x1001d9fa`). Flags 2 and 16 are struck. The fifth
mesh slot and the 28 collision hulls are never tested
([07-objects.md](07-objects.md#the-fifth-slot-is-collision-geometry)).

|  | *measured* |
|---|---|
| hulls a round can strike | none — 0 of 28 have a level 0 in any variant |
| level-0 triangles a round passes through | 1306 of 129542, on 30 meshes: trees and the mines |
| `r_h_01` / `r_h_03` / hero `r_h_02` | 4 / 6 / 9 nodes to hit, 104 / 174 / 230 triangles, none passed |

**What the round does** (`0x1000d0c0`):

- **Bubbles.** It walks its bubble contacts nearer than its face hit. Where its
  life exceeds the sector's strength
  ([below](#shields-a-generator-a-deflector-six-sectors--read-and-measured)) it
  passes through and loses that strength (`0x1000d1b4`). Otherwise it stops on
  the bubble, runs its hit action group (`+0x4e4`) and spends all its life.
- **A face or the map edge.** It moves to the recorded point and runs its
  `+0x4e4` group for a face (`0x1000d35b`) or `+0x4e8` for the edge
  (`0x1000d36e`).
- **What the groups do** (*measured* on all 66 rounds, the actions *read* in
  [13-control.md](13-control.md#the-section-5-record--read-and-measured)): the
  face group ends or replays its flight effect (19, 8 or 10) and **kills the
  round** — action 17 on 63, invulnerability off and `ILifeSystem` slot 7, so
  node 0's `.exp` plays and deals the hit; 15 on 3; the edge group always **removes** it (15), with no
  explosion. At the **end of its range** (`0x1000d069`, block entry 4,
  `0x1000d390`) 58 rounds explode node 0 with their own `*_end.exp` (action 27)
  — a puff in the air, or for a missile its full blast — and the other 8 are
  killed or removed.

**Beams are rounds.** A laser flies at 10,000 m/s, but every test is a segment
or a swept sphere over the frame, so nothing is sampled and nothing tunnels. 54
of 66 rounds move further than their own radius even in a 0.01 s tick
(*measured*).

## Shields: a generator, a deflector, six sectors — *read*, and *measured*

A shield is **two parts**: the fight shield (class 9, `i_fsh`) holds six
sectors, and a **deflector** (class 21, `i_def`) decides how much of each stops
damage. The bubble exists only while **both are present, switched on and not
destroyed** (`0x1002c500`).

**Which sector.** A hit on the object that names no node of its own carries
its sector with it — what a round striking the bubble looks like; any other
hit is given one if its sphere crosses the bubble, the object's bounding
sphere, from outside: `r < d < r + R` (`0x1000ff00`). A blast that goes off
inside the bubble meets no shield. The sector is the hit's
dominant axis after the object's matrix is applied (`0x1002c590`):

| sector | 0 | 1 | 2 | 3 | 4 | 5 |
|---|---|---|---|---|---|---|
| axis | +y | −y | −x | +x | +z, top | −z, bottom |

Which of ±x and ±y is the front is not established here.

**How much it stops.** A sector's effective strength is

    deflector value i × condition × power level  ×  shield value 0 × the sector's fill

(`0x1002ca30`). A hit takes `min(damage, effective strength)` out of the
sector and **the sector loses that divided by the deflector's coefficient**
(`0x1002ca80`) — so a 0.5 deflector spends two points of shield per point it
stops. What is left of a **blast** goes on to the nodes; what is left of a
**round that struck the bubble** is spent (`0x1000ebc0`). An unpowered or
destroyed deflector stops nothing.

**Recharge** (`0x100257b0`, on the power tick): the charge left after the
shield's idle draw buys `charge / value 2` points, at most `value 1 ×
condition` a second and never more than the six sectors lack; the points go
to each sector **in proportion to what it lacks** (`0x10025a90`). There is no
delay after a hit — no timer in any of the class's methods. **Short of power
the shield drains**: below the idle draw the charge is negative and the six
sectors lose in proportion to what they hold, at up to `power / value 2` a
second.

**A spent sector still meets a round.** The contact is made, but a strength of
0 is below any live round's life, so the round passes and loses nothing
(`0x1000d1b4`).

**The numbers** (*measured*): every fight shield is three values and zeros —
a sector maximum, a recharge a second and the charge a point costs — and every
deflector is six equal coefficients.

| | sector max | recharge /s | charge /point | deflector |
|---|---|---|---|---|
| chassis (`bases.rlb`, 22) | 100–1,000 | 10–100 | 2 | only `r_l_06`, 0.5 |
| `o_fsh_t` / `o_def_t` | 350–1,200 | 4–10 | 0.03 | 0.7–1.0 |
| `o_fsh_l` / `o_def_l` | 1,500–1,950 | 11–17 | 0.04 | 0.7–1.0 |
| `o_fsh_m` / `o_def_m` | 2,500–3,000 | 30–36 | 0.05 | 0.85–1.0 |
| `o_fsh_b` / `o_def_b` | 3,350–3,800 | 50–80 | 0.06 | 0.8–1.0 |
| `o_fsh_f` | 10,000–14,000 | 80–200 | 0.0003–0.0006 | — |
| buildings (`fortif.rlb`, 19) | 8,000 | 80 | 0.00015 | none |
| turrets (`turrets.rlb`, 55) | — | — | — | 0.5 |
| building defences (`u_*_def`, 7) | — | — | — | 0.36 |

The chassis rows are the slots' defaults: a fitted shield generator, deflector
and armour replace them on every shipped robot but the Small Tower, which has no
armour fitted ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
No building controller carries a deflector: a building's shield absorbs only
once one is fitted (the `u_*_def` parts).

## Armour — *read*, and *measured*

Class 27 (`i_arm`) is three numbers, kept when the part is created
(`0x1002d4b0`): a rating, a linear and a square factor. Every hit on a node
becomes (`0x10010030`)

    min(damage, linear × damage + square × damage²)

so armour takes most off small hits and nothing off a big one — a hit of
`(1 − linear) / square` or more goes through whole. The rating is handed out
as a property and plays no part in the sum; it is a weight per unit of area,
which the mass sum multiplies by every node's area (`0x1000fbac`,
[28-chassis.md](28-chassis.md#what-a-chassis-weighs--read-and-measured)). With more than one class-27 part
the last one created wins.

| (*measured*) | rating | keeps of a small hit | goes through whole from |
|---|---|---|---|
| chassis (22) | 0 | all | — |
| `o_arm_t` | 0.125–3.75 | 85–57% | 1,822–2,713 |
| `o_arm_l` | 2–7.5 | 74–45% | 2,373–2,892 |
| `o_arm_m` | 10–22.5 | 62–34% | 2,688–3,035 |
| `o_arm_b` | 17.5–30 | 51–22% | 2,464–2,790 |

## Repair: a unit's own repair unit, switched on and off — *read*, and *measured*

The **repair unit** a robot carries is its **repair system**, class 15
(`CICLS_REPAIRSYS`, the `i_rps` parts). The **repair mode** you toggle is that
system's switch. Both are about the unit's *own* hit points: nothing in the
code lets a repair unit mend another unit or a building (see "Nobody repairs
anybody else" below).

**It is a switch, and it ships off** — *read*, and *measured*.
- A component's state word is `+0x50`. The repair class's constructor sets it
  to 0 (`Control.dll:0x10022ae0`).
- The component record's `+0x18`, which [13-control.md](13-control.md) calls "an
  index", is really the **initial state**: the parser copies it over the
  constructor's value unless it is -1 (`0x10021d86`).
  - All 64 class-15 records leave it at -1, so **every repair system starts
    switched off**.
  - Other classes do use the field: 33 on the 4 class-24 turret records, 0 on
    the 24 class-25 building records, 9 on the 2 class-29 records.
- The state values are the input layer's own `CIS_` numbers:
  - 0 is `CIS_SWITCHOFF`, `0x20` is `CIS_SWITCHON`.
  - `0x40`, `CIS_SWITCH_INV`, toggles: the class turns `0x20` into 0 and
    anything else into `0x20` (`0x10022c90`).
- **The G key toggles it** — *measured*. `hero.tbl`, `m1.tbl` and `m2.tbl` each
  bind `SCAN_G` to `CICLS_REPAIRSYS MCMD_STATE CIS_SWITCH_INV`, which
  `Command.dsc` calls `CMD_REPAIRSYS_ON`, "Repair on/off".
- **The cockpit announces it.** `iron3d.dll` reads the player's unit's first
  class-15 component and counts `0x20` as on (`0x10076e10`). When that changes
  it plays `VOICE_REPAIR_SYS_ON` or `_OFF` (`0x100a5485`).

**What it does while on** — *read* (`0x10022b20`, `0x10022bb0`).
- **Off, or on a destroyed node:** it draws nothing and repairs nothing.
- **On:** each power tick it asks for its power figure (the *idle* draw) plus
  `value 1 ×` the hit points it would restore. It restores at most
  `value 0 ×` its node's condition a second, never more than the object lacks.
- **Short of power:** after the idle draw, the charge left buys
  `charge ÷ value 1` points.
- **Where the points go:** to the object's *own* nodes, in index order
  (`0x10010ba0` on the owner, `+0x3c`).
  - A unit's repair skips destroyed parts; a kind-3 object's restores them
    (`0x10022b00`).
- **Running cost:** left on at full health it costs only its idle figure.
  Being a class-15 part, it draws on power channel 0, served second.
- **No reach:** values 2–15 are zero on all 64 records (*measured*), so there
  is no range, no target and no radius. The catalogue agrees: all 16 `i_rps`
  entries in `objects.dlb` show a single **"Regeneration", in HP/s**.

**The numbers** — *measured*.

| part | Regeneration, HP/s by mark | charge a point | idle draw a second |
|---|---|---|---|
| `o_rps_l` small | 11 / 13 / 15 / 17 | 0.04 | 0.08–0.11 |
| `o_rps_m` medium | 30 / 32 / 34 / 36 | 0.05 | 0.18–0.21 |
| `o_rps_b` large | 50 / 60 / 70 / 80 | 0.06 | 0.27–0.31 |
| `o_rps_f` fortification | 80 / 110 / 150 / 200 | 0.0006 → 0.0003 | 0.01 |
| a chassis's own slot (`bases.rlb`, 22) | 1 | 1 | 1 |
| a building's own (`fortif.rlb`, 26) | 100 | 0.0002 | 0.01 |

- **Who carries one:** 372 robot assemblies, all but two, carry exactly one
  `i_rps` part. The two without are the target dummies `l_targ.dat` and
  `M_targ.dat`, on chassis `R_H_01` ("Hero target") and `R_H_03`. So do 70 building assemblies; the power mast, the four ruins and
  the small main teleport don't.
- **A unit uses the fitted part's values.** Every chassis's controller declares its
  class-15 slot under the part's label with 1, 1, 1, and the fitted repair unit is
  re-parsed into that slot, replacing them
  ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
  All 372 turreted robots fill it (*measured*).

What a full repair costs — *derived*, for a Small Wheel chassis (`r_l_03`, 2,621
hit points over 12 nodes) brought back from the edge with no part destroyed,
at full power and with the repair unit's own node intact:

| with | time | charge |
|---|---|---|
| a small MK1 repair unit | 238 s | 105 for the points + 19 idle ≈ 124, 4% of the small MK1 battery's 3,000 |
| a small MK4 | 154 s | 105 + 17 ≈ 122 |

**What the AI does with the switch** — *read*. The earlier note that nothing
reads `Decision_RepairOn`/`Off` was wrong.
- **Where the thresholds live.** A behaviour keeps its difficulty profile at
  `+0x8d4` (bound at `Behavior.dll:0x1000a2f4` by `0x10019bd0`, read through
  `0x100146a0`); `Decision_RepairOn` is its `+0x10`, `Decision_RepairOff` its
  `+0x14`.
- **The decision** (`0x10017c70`) reads the object's life fraction and battery
  charge, then:
  - **switches repair on** when the unit *needs service* — life under 0.5, or
    under 0.9 for a building (`0x1001c700`) — *or* its life is under
    `Decision_RepairOn`, **provided its charge is over 30%**;
  - **switches it off** when it does not need service and its life is over
    `Decision_RepairOff`, **or whenever its charge falls under 10%**.
- **How the switch is applied.** It goes through the behaviour's device
  manager (`+0x3d0`). That manager collects every class-15 component into one
  list (`0x100186b0`) and sends each `CIS_SWITCHON` or `CIS_SWITCHOFF`
  (`0x10019a80`). Its sibling sends the detection shield's camouflage on and
  off the same way (`0x10019a10`).
- **Starting any task turns repair off**, and camouflage too
  (`0x10034930`). The next decision turns repair back on if it is needed. The
  manager remembers the last state it sent (`+0xaf`) and sends nothing when
  asked for the same again, so a switch the player flipped by key in between
  is not seen by it.
- **When the decision runs.**
  - A building runs it on its takt timer (`0x100054a0`).
  - A unit runs it on its takt (`0x10005110`) unless that tick sent it to a
    dock or into an attack (`0x10017d50`, `0x10017e70`).
  - The unit takt needs bit `0x10` of the behaviour's flags, which the
    behaviour's mode setter derives from its argument (`0x100067b0`). Who
    clears it, for instance while the player drives the unit, is not traced.
  - Clans of type 3 skip the takt altogether (`0x10005070`).

| profile (*measured*) | `Decision_RepairOn` | `Decision_RepairOff` | a unit switches on below | and off above |
|---|---|---|---|---|
| `diff_strong` | 0.8 | 0.9 | 80% | 90% |
| `diff_normal`, `diff_slow`, `diff_stupid` | 0.5 | 0.8 | 50% | 80% |
| `diff_weak` | 0.1 | 0.3 | 50% (needing service overrides) | 50% |

The last two columns are *derived* from the rule above. A building needs
service below 90%, so under every profile it repairs below 90% while its charge
allows.

**Nobody repairs anybody else** — *read*, as a search. Everything in the
shipped code that raises a node's life:
1. **The repair system**, on its own object only (`Control.dll:0x10022c87`).
2. **Setting the life fraction, property `0x31`**, through `ILifeSystem`
   slot 6 or the control system's main interface, slot 14 (`0x1000e980` →
   `0x1000e9c2`, which also
   restores destroyed parts). Across all twelve modules, the only callers
   that pass `0x31` are:
   - `Behavior.dll:0x1001816a`, the **dock**, on the units standing in it
     ([27-ownership.md](27-ownership.md));
   - `0x1001c69b`, which fills the behaviour's **own** object to full — every
     takt while `Behavior.ini`'s `DeterminMode` is set or a demo records or
     plays back (`0x10004e1f`), and on one internal message (`0x1000934c`);
   - `Control.dll:0x1000b382`, an object property handler whose ids 0 and 1
     set invulnerability and life (what calls it is not traced; mission
     properties, by the look of it — *guess*).
3. **The other callers of the node update** (`0x10010ba0`) all pass damage:
   collision (`0x1000d212`, `0x1000d2f9`), the ground (`0x10012a7e`) and
   vital nodes (`0x10012c37`).
4. **A hit cannot heal**: the armoured damage is clamped at 0
   (`0x10010253`), and every one of the 144 `.exp` files does 0 or more
   (*measured*).

The orders say the same.
- **Orders 8 and 9 build the same task.** `ORDER_ROBOT_RELOAD` (8) and
  `ORDER_ROBOT_REPARE` (9) share one case in `MTaskStack::CreateTaskFromOrder`
  (`0x10033a80`, table `0x100344b0`): `M_Task_Reload`, a trip to a dock.
- **No repair-another order exists** among `varset.var`'s orders.
- **`Task_Repare` does nothing.** The behaviour-profile flag is bound with
  the other `Task_*` flags (`0x10022e20`, struct `+0x820`, `Task_Repare` at
  `+0x2c`), but nothing reads it. The struct has no direct reads, and all 17
  calls of its getter `0x10014680` read the ore and power fields at
  `+0x54`–`+0x68`.

What *can* restore someone else is a dock: a unit standing in one gains 10% of
its full hit points a second, destroyed parts included, plus 10% of its
shield, battery and ammunition (`Behavior.dll:0x10018100`, `0x10019372`,
`0x100181e0`).

## Not established

- Which of ±x, ±y is a model's front, so which sector is "front".
- How a collision object's start and end differ when the pass runs: message 1
  sets both to the sphere's centre, message `0x1c` moves the end, and the pass
  runs between the two.
- How the struck object and node reach `ILifeSystem` slot 8's hit. It carries
  the same five-integer reference the collision record does, but the copy was
  not found.
- Whether a round's owner id is the unit or the gun that fired it — which
  decides whether a turret's round can strike its own robot.
- What reads a node's fifth slot, if a round's hit test does not.
- Who clears the behaviour flag `0x10` that lets a unit's takt switch its
  repair (`Behavior.dll:0x100067b0`'s caller), and so whether the AI overrides
  the switch while the player drives.
- What `IControl` component query `0x77`, which the catalogue's Regeneration
  row reads (`iron3d.dll:0x1006f62c`), answers.
- The two 1.0 floats of an `.exp`. Its slots 1–11 are by ground surface
  ([11-effects.md](11-effects.md#what-an-explosion-plays--read-and-measured)).
- What agent kind 3 is, and what becomes of a kind-3 object whose node 0 is
  destroyed (it is marked `0xfffe` and not killed).
