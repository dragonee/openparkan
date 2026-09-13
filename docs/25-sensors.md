# Sensors — radar, the detection shield, and what the AI sees

How a machine notices another: the radar that looks, the three things a target
gives away, the shield and camouflage that hide them, and what the behaviour
code does with what it sees. The looking is in `Control.dll`, the using in
`Behavior.dll`; the numbers are in the `.ctl` controllers and the unit
assemblies.

**Every claim is tagged.** *Measured* means `openparkan verify` re-derives it
from the shipped data. *Read* means it comes from the disassembly, with the
address given, and has no data-side check. *Guess* fits the evidence and is
not established. *Unknown* means exactly that.

## A radar is five values — *read*, and *measured*

A radar is a class-8 component, `CICLS_RADAR`. Its sixteen values
([13-control.md](13-control.md#the-64-bytes-are-the-components-values)) are
used five at a time:

| value | what it is | read at |
|---|---|---|
| 0, 1, 2 | sensitivities to a target's three signatures, taken as ids `0x300`–`0x302`, so each is scaled by the radar node's life and its power level | `Control.dll:0x10024620` |
| 3 | range, taken plain | `0x10024441`, `0x1002b6ec` |
| 4 | how long a scan stays good before the next, in the control clock's milliseconds | `0x100243e0`, `0x1002b712` |
| 5–15 | zero on all 76 | |

*Measured*, over all 76 class-8 components:

| where | sensitivities | range | period | power a second |
|---|---|---|---|---|
| the 12 radar parts, `intsys.rlb` `o_rdr_*` | 0.05, 0.7, 25 | 250–700 | 750 | 0.08–0.30 |
| the 55 robot turrets' radar slots, `turrets.rlb` | 0.5, 0.5, 0.5 | 500 small, 800 medium and large | 750 | 1 |
| the 3 bunker and tower radars, `parts.rlb` `o_bnt_rdr_*` | 0.5, 0.5, 0.5 | 500, 600, 750 | 750 | 0.01 |
| the 5 animals, and chassis `r_l_06` | 0.5, 0.5, 0.5 | 500 | 750 | 0 or 1 |

A radar part's range climbs with its mark — *measured*:

| size | MK1 (`_df`) | MK2 | MK3 | MK4 |
|---|---|---|---|---|
| small `l` | 250 | 300 | 400 | 600 |
| medium `m` | 300 | 350 | 450 | 650 |
| large `b` | 350 | 400 | 500 | 700 |

— the "Sensor range" the parts catalogue prints for them
([19-descriptions.md](19-descriptions.md)) is a computed field, `sensrange`,
not a number in the file. A radar draws on power channel 2, served third,
after engines and the miscellaneous channel
([23-economy.md](23-economy.md#a-power-shortage-lowers-efficiency-once-the-batteries-run-down--read-and-measured)).

## A scan is a sphere, a falloff and three tests — *read*

`IControl`'s radar query (`Control.dll:0x1002c400`) runs the radar's scan
(`0x10024390`):

1. **Nothing, if the radar is off or broken.** Its switch word at `+0x50` is
   zero, or its node's life is 0 (`0x10021820`).
2. **The last answer, if it is fresh**: until `value 4` has passed since the
   last scan. Every shipped radar says 750.
3. **Otherwise a sphere**: the world's objects within `value 3` of the radar
   (`0x1002446c`), less the radar's own object.
4. **Each is tested** (`0x10024620`). With `d` the distance and `R` the range,
   a target is detected when `d < R` and, for any of the three,

   ```
   sensitivity × life × level × signature × (1 − d²/R²)  ≥  1
   ```

   tried in the order mass, electronics, drive (below). A target whose word
   from its slot `0x2c` is `0xfffe` in the low 16 bits is never detected
   (`0x10024638`). It is the value a destroyed object's owner word is set to
   (`0x10011098`, [26-damage.md](26-damage.md#hit-points--read-and-measured)),
   so *guess*: the radar passes over wrecks.

So the distance at which one signature is seen is `R · √(1 − 1/(v·s))`: a
product `v·s` of 10 is seen at 95% of the range, 2 at 71%, and 1 or less not
at all. **Nothing in the test involves a third object**: no line of sight, no
terrain, no other unit's equipment. What a radar sees depends on the radar, the
target and the distance between them.

**Only one radar counts per control system** — *read*. The component factory
keeps a single radar pointer, overwritten by each class-8 record it builds
(`0x1002d5d2`), and the query asks that one. *Measured*: no `.ctl` carries two.
An assembled robot has two, though — the turret's radar slot and the radar part
fitted to it (below) — and which of the two ends up as the pointer is
*unknown*.

## What a target gives away — *read*

The three signatures are properties `0x45`, `0x43` and `0x44` of the target's
control system (`Control.dll:0x1000dcc0`):

| property | radar value | is | read at |
|---|---|---|---|
| `0x45` | 0 | the object's **mass** | `0x1000e1f0` |
| `0x43` | 1 | its **area** × the power it asks a second on channels 0, 2 and 5 | `0x1000e267` |
| `0x44` | 2 | its **volume** × the power it asks a second on channels 0, 3 and 4 | `0x1000e31c` |

each then multiplied down by every detection shield it carries (next section).

- **Mass, area and volume** are totals over the model's nodes
  (`0x1000fac0`). A node's area and volume come from the mesh
  (`AniMesh.dll:0x100051f0`, ids `0xf` and `0x10`); the one scales with two of
  the object's three scale factors and the other with all three, which is why
  they read as area and volume — the names are a *guess*, the scaling *read*.
  A node's mass is its volume times the **second float of its `.ndp` record**,
  and each part adds its mass, the float at `+0x1c` of its `.ctl` record — 0
  on all 24 armour parts and 100 to 40,000 kg on the other 104 internal ones
  (*measured*; [24-motion.md](24-motion.md#load--read-and-measured)).
- **The power figures** are what the control system's consumers ask for, per
  second, in its last power tick (`Control.dll:0x1002d47a`, `0x1002d496`):

  | channel | draws there |
  |---|---|
  | 0 | doors, computers, repair systems, and whatever else has no channel of its own — in both |
  | 2 | cameras, radars, class 17 |
  | 5 | fight shields, detection shields, deflectors, class 27 |
  | 3 | engines, a building's efficiency component, classes 16 and 20 |
  | 4 | turrets, guns, classes 22, 24 and 30 |

So `0x43` is an electronics signature and `0x44` a drive-and-weapons one —
the names are a *guess*. What follows is derived, not observed: **a machine that
stands still with its guns charged asks next to nothing on channels 3 and 4**,
since an engine at rest draws nothing and a gun draws only to refill
([23-economy.md](23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured)),
so its `0x44` falls to what channel 0 asks — and a radar part's sensitivity to
`0x44` is 500 times its sensitivity to mass. A shield recharging, or camouflage
running, raises `0x43`.

## The detection shield hides all three, and camouflage hides them again — *read*, and *measured*

A detection shield — `CICLS_DETECTSHIELD`, class 10, "Large detect.shld" — is
the game's stealth, and camouflage is one of its modes. There is no separate
camouflage part.

**What it does** — *read*, at `Control.dll:0x1002bd94`, `0x1002be95` and
`0x1002c088`. For each of the three signatures, each shield multiplies by

```
max(0, 1 − value i × life × level)                     i = 0, 1, 2
   × max(0, 1 − value 3)          only while camouflage is on
```

**Camouflage is a state** — *read*. The shield starts in state `0x2000`,
camouflage off (`0x10026470`); `0x1000` turns it on and `0x4000` toggles it
(`0x10026570`), which is what the input tables send as `CMD_CAMOUFLAGE_WEAR`,
`CIS_CHAMELEON_INV` ([14-controls.md](14-controls.md)). While it is on the
shield draws `value 4` a second on top of its power figure (`0x100264b0`), and
**it switches itself off once its power level no longer covers its plain draw**
(`0x10026510`): when `level × (power + value 4) < power`. For a small MK2
shield that is a level under 0.09 / 0.29, 31% — derived from the table below.
The cockpit says so: `iron3d.dll` watches the player's class-10
component for state `0x1000` (`0x10076e40`) and plays `VOICE_CHAMELEON_SYS_ON`
or `_OFF` when it changes. A behaviour's device manager can switch every
detection shield's camouflage on (`0x1000`) or off (`0x2000`)
(`Behavior.dll:0x10019a10`), and starting any task switches it off
(`0x10034930`), as it does the repair system
([26-damage.md](26-damage.md)).

*Measured*, the twelve parts in `intsys.rlb`:

| part | mass cut (0) | electronics cut (1) | drive cut (2) | camouflage (3) | camouflage's power (4) | power |
|---|---|---|---|---|---|---|
| small MK1–MK4 | 0.9 | 0.45 | 0.01 | 0, 0.84, 0.91, 0.933 | 0.1, 0.2, 0.25, 0.3 | 0.08–0.11 |
| medium MK1–MK4 | 0.9 | 0.62 | 0.01 | 0, 0.98, 0.988, 0.991 | 0.1, 0.3, 0.35, 0.4 | 0.18–0.21 |
| large MK1–MK4 | 0.98 | 0.982 | 0.05 | 0, 0.948, 0.972, 0.9795 | 0.2, 0.4, 0.45, 0.55 | 0.27–0.32 |

**A MK1 shield has no camouflage**: value 3 is 0 on exactly the three `_df`
parts, which are exactly the three whose catalogue entry prints "Supression
0.0%"; the others print 20, 35 and 50 by mark. The printed percentage is a
literal in the entry, not computed from the values. Camouflage's cut and its
running cost both climb with the mark.

What that means, derived: a small MK2 shield leaves a machine 10% of its mass
signature, 55% of its electronics and 99% of its drive; with camouflage on,
1.6%, 8.8% and 15.8%. Against a radar part, whose drive sensitivity is 25, a
drive signature has to fall below 0.04 before it goes unseen at any distance.

**There is no jammer** — *read*, as a search. The engine's class list has no
jamming class, no binary carries the word, and the detection test reads only
the target and the radar. What hides a machine is its own shield.

## Who carries what — *measured*

- **Every robot has one radar and one detection shield.** All 372 assemblies
  with a turret carry exactly one `i_rdr` part, fitted to the turret, and one
  `i_dsh` part, fitted to the chassis; none of the 86 without a turret carries
  either. The turret's controller declares a radar of its own under the part's
  label, with values (the second row of the first table), and the label's
  size letter is the size of radar part the turret takes — all 744 fitted
  radar and deflector parts match it ([30-turrets.md](30-turrets.md)); each chassis in
  `bases.rlb` declares a detection-shield slot the same way but empty — all 22
  of those records' values are zero.
- **Of the buildings, only bunkers and towers can see.** All 27 carry one
  "Bunker radar" or "Tower radar" (`e_gun_fs_*`, range 500–750); the other 49
  building assemblies — factories, power plants, mines, storages, research
  centres, hangars, teleports, bridges — carry no radar at all.
- **Animals** have a radar built into their controller.

## What the AI does with it — *read*

Every behaviour owns a **radar module** (`Behavior.dll:0x10023120`, at
`+0x4a8`) with two timers, each `a × 64 ms` plus a random share of `b × 64 ms`
(`0x1004c550`):

- **Every 0.45–1.9 s it re-reads the contacts** (`0x10023240`). It asks its
  machine's `IControl` for the radar's list, and for each contact looks up the
  owning clan and drops it if that clan's kind (the clan SuperAI's slot 10) is
  0 or 3, if it is a building, or if it is the machine itself. What is left
  goes into two lists by clan: **hostile** (`0x1000d460`) and **friendly**
  (`0x1000d4f0`). A clan is hostile when the relation table says 0 and friendly
  when it says 2; a clan of kind 3 is neither to anyone, and a machine of a
  kind-0 clan takes every other clan as hostile. Kinds 0 and 3 line up with
  the mission file's nature and neutral clans
  ([27-ownership.md](27-ownership.md#the-clan-word-is-a-type--measured-and-read)),
  which is a *guess* that the two are the same number.
- **Every 2.0–4.9 s it reports its position and radar range** to its clan's
  `IArealMap` (`0x1002355f`) — *guess*: marking the area the clan can see.

**Targets come from the hostile list.** The target selector (`0x10025410`),
refreshed on a timer, picks by mode: none, a given target, or one of three
pickers that walk only the hostile radar list and only within 500 — the contact
nearest a point (`0x100254b0`), and two that weigh distance against two figures
the clan's areal map keeps for each unit (`0x100255a0`, units only — a building
calling it logs a warning; `0x100256e0`). What those figures are was not read.
And the behaviour's own
tick scores the hostile contacts through its task stack and issues an order
against the best (`0x10017e70`) — *guess* that the order is an attack; its
kind was not read.

**What follows** — *read*:

- **A machine without a working radar picks no targets of its own.** With no
  radar the query returns nothing; with the radar's node destroyed the scan
  returns nothing; with its power short, its sensitivities fall with its level
  but its range does not.
- **Buildings are not in the lists**, so these pickers never choose one. An
  order to attack a building names it instead.
- **Nothing here makes an AI unit flee**; no code that reads the radar lists
  was found to do so. That is a search, not a proof.

## Not established

- Which of a robot's two radar components — the turret's slot (0.5, 0.5, 0.5,
  500 or 800) or the fitted part (0.05, 0.7, 25, 250–700) — the control system
  ends up using.
- How the mission file's 0/1 relation words become the runtime's 0 and 2.
- What `IArealMap` does with the radar report, and what the player's map and
  radar display show — `iron3d.dll`'s drawing was not read.
- What asks an AI machine's device manager to switch camouflage *on*: task
  start switches it off (`0x10034930`), and no caller that switches it on was
  traced.
- The third list the radar module fills, of every kept contact
  (`+0x3c`, read by the walker at `0x1003f760`): where it is emptied was not
  found.
- Class 17, found only on projectiles in `weapon.rlb`, draws on the sensor
  channel and takes the cosine of its value 0 (`Control.dll:0x100247a0`), so
  *guess*: a missile seeker's cone. It does not use the detection test above.
