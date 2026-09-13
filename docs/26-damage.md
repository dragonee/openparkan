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
| mines, plants, stores, hangars, generators, the medium `mtp` | 1 on node 0; parts 40,000–500,000 |

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
  and a unit is invulnerable while it upgrades a building, `0x1003356f`).

**A blast falls off with overlap** (`0x10010030`). For a node whose bounding
sphere has radius *r* and centre at distance *d* from a blast of radius *R*:

| | damage |
|---|---|
| `d ≥ R + r` | 0 |
| one sphere wholly inside the other | the whole blast |
| otherwise | `damage × ((R + r − d) / 2R)³` |

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

No building controller carries a deflector: a building's shield absorbs only
once something that does is fitted — which parts join a building's control
system is not established here.

## Armour — *read*, and *measured*

Class 27 (`i_arm`) is three numbers, kept when the part is created
(`0x1002d4b0`): a rating, a linear and a square factor. Every hit on a node
becomes (`0x10010030`)

    min(damage, linear × damage + square × damage²)

so armour takes most off small hits and nothing off a big one — a hit of
`(1 − linear) / square` or more goes through whole. The rating is handed out
as a property and plays no part in the sum. With more than one class-27 part
the last one created wins.

| (*measured*) | rating | keeps of a small hit | goes through whole from |
|---|---|---|---|
| chassis (22) | 0 | all | — |
| `o_arm_t` | 0.125–3.75 | 85–57% | 1,822–2,713 |
| `o_arm_l` | 2–7.5 | 74–45% | 2,373–2,892 |
| `o_arm_m` | 10–22.5 | 62–34% | 2,688–3,035 |
| `o_arm_b` | 17.5–30 | 51–22% | 2,464–2,790 |

## Repair — *read*, and *measured*

**Every object repairs itself** with its class-15 part (`0x10022bb0`): the
charge left after its idle draw buys `charge / value 1` points, at most
`value 0 × condition` a second, never negative. They go to the nodes **in
index order**, each topped up before the next (`0x10010ba0`). A unit's repair
skips destroyed parts; a kind-3 object's restores them too (`0x10022b00`,
`0x10010b10`).

| (*measured*) | points /s | charge /point |
|---|---|---|
| chassis (22) | 1 | 1 |
| `o_rps_l` / `_m` / `_b` | 11–17 / 30–36 / 50–80 | 0.04 / 0.05 / 0.06 |
| `o_rps_f` | 80–200 | 0.0003–0.0006 |
| buildings (26) | 100 | 0.0002 |

**A docked unit** — one inside a building that services it — gains **10% of
its full hit points a second, destroyed parts included**, 10% of its shield,
10% of its battery (`Behavior.dll:0x10018100`, `0x10019372`), and each gun
10% of its ammunition, at least one round (`0x100181e0`). `Behavior.dll`
answers "needs service" (`0x1001c700`) when an object's life is below 0.5 —
0.9 for a building — its property `0x32` below 0.6, or a component's value
`0x400` below 0.2; which of its six callers act on that is not traced.

## Not established

- Which of ±x, ±y is a model's front, so which sector is "front".
- Whether a round still collides with a sector that has nothing left.
- What `Decision_RepairOn` / `Decision_RepairOff` (0.1–0.8 / 0.3–0.9 by
  difficulty) switch: `Behavior.dll:0x10019bd0` binds them and nothing found
  reads their offsets.
- Whether any unit repairs another: the only code found that heals someone
  else's nodes is the dock. `Task_Repare` is a profile flag whose task class
  was not identified.
- The surface index behind an `.exp`'s slots 1–11, and the two 1.0 floats.
- What agent kind 3 is, and what becomes of a kind-3 object whose node 0 is
  destroyed (it is marked `0xfffe` and not killed).
- Which parts join a building's or a bot's control system, so which deflector
  and armour a finished unit ends up with.
