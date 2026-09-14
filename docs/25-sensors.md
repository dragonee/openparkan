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
An assembled robot's is the **fitted radar part**: the part is re-parsed into the
turret's radar slot, the device the pointer already names
([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
All 372 turreted robots fill that slot (*measured*), so on every shipped robot the
sensitivities are 0.05, 0.7 and 25 and the range 250–700; the turret slot's 0.5/0.5/0.5
at 500 or 800 is never used.

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
or `_OFF` when it changes.

**When the AI wears it** — *read*. A behaviour's device manager switches every
detection shield's camouflage on (`0x1000`) or off (`0x2000`)
(`Behavior.dll:0x10019a10`), sending only when the answer changes (`+0xae`).
Two things ask it:

- **The engagement check** of a unit's takt (`0x10017e70`), which runs when its
  timer comes round and the unit does not send itself to a dock. It asks for
  camouflage **on** when all three hold (`0x10017fdd`), and **off** otherwise:
  1. the radar module's hostile list is not empty;
  2. no hostile contact is within **150 m** of the unit across the ground
     (`0x10017f49`);
  3. at least **10 s** have passed since the unit's fire control last handed
     its turret a target (`0x10017fb8`; the time is stamped at `0x10025009`,
     right after interface `0x204` slot 16, `SetTarget`).

  So an AI machine hides while it has seen enemies that are not yet close and
  it has not been shooting, and drops the cloak when one comes within 150 m or
  it opens fire. The check is skipped altogether for a neutral clan's units
  and while `Behavior.ini`'s `DeterminMode` is set, and a building has no such
  takt.
- **Starting any task** switches it off (`0x10034930`), as it does the repair
  system ([26-damage.md](26-damage.md)).

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
  centres, Outposts, teleports, bridges — carry no radar at all.
- **Animals** have a radar built into their controller.

## What the AI does with it — *read*

Every behaviour owns a **radar module** (`Behavior.dll:0x10023120`, at
`+0x4a8`) with two timers, each `a × 64 ms` plus a random share of `b × 64 ms`
(`0x1004c550`):

- **Every 0.45–1.9 s it re-reads the contacts** (`0x10023240`). It asks its
  machine's `IControl` for the radar's list, and for each contact looks up the
  owning clan and drops it if that clan's type is 0 or 3, if it is a building,
  or if it is the machine itself. What is left goes into two lists by clan:
  **hostile** (`0x1000d460`) and **friendly** (`0x1000d4f0`). A clan is
  hostile when its relation says 0 and friendly when it says 2
  ([below](#clan-relations-the-files-words-straight-through--read-and-measured));
  a clan of type 3 is neither to anyone, and a machine of a type-0 clan takes
  every other clan as hostile.
- **The type is the mission file's clan word** — *read*. The behaviour asks the
  system areal map, which it keeps at `+0x48` (`Behavior.dll:0x10005db1`), for
  `GetClanType` (slot 10, `ArealMap.dll:0x10020a10`). `iron3d.dll` fills that
  same map from each clan record's `+0xc`, the file's word, through
  `SetClanType` (slot 11, `iron3d.dll:0x100602cd`); both reach the map as the
  landscape's interface `0x302` (`ArealMap.dll:0x10014580`,
  `iron3d.dll:0x100601a5`). So types 0 and 3 are the file's nature and neutral
  clans ([27-ownership.md](27-ownership.md#the-clan-word-is-a-type--measured-and-read)).
  An earlier note here named the clan SuperAI's slot 10; that slot takes no
  clan and is not what the list consults.
- **Every scan also appends each kept contact to a third list** (`+0x3c`,
  `0x10023446`) — hostile, friendly or neither — and that list is **never
  emptied** — *read*, as a search. The scan clears only the hostile and
  friendly lists (`0x10023283`, `0x10023290`); the same search finds those two
  clears and nothing for `+0x3c` or the behaviour's `+0x4e4`, and only the
  behaviour's constructor and destructor touch it otherwise. Its one reader,
  the walker at `0x1003f760`, **is called by nothing**: no call, jump or
  pointer anywhere in `Behavior.dll` names it, where the same byte search finds
  the scan's one caller and the three of its neighbour `0x1003f3a0`. So the
  list grows by a scan's worth of ids every second or two for the unit's life
  and nothing uses it.
- **Every 2.0–4.9 s it reports its position and radar range** to its clan's
  `IArealMap` (`0x1002355f`). The clan map refreshes its snapshot of every
  areal the report reaches — the one the unit stands in, and each neighbour
  whose shared edge starts within the radar's range, spreading outwards — from
  the system map's lists of the units and buildings there, and stamps it with
  the time (`ArealMap.dll:0x10001ec0`, `0x10001dd0`, `0x10001840`;
  [31-packages.md](31-packages.md#where-a-search-looks--read-and-measured)).
  **The refresh applies no detection test**: every object the system map lists
  in those areals is copied in, whatever its signatures, shield or camouflage.
  So a camouflaged machine drops off the enemy's radar lists but not off an
  enemy clan's areal map, where its searches look.

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
- **A missile's seeker ignores all of this.** Class 17 follows the target its
  gun hands it while it is within range and inside its cone
  ([29-weapons.md](29-weapons.md#guided-rounds-differ-in-how-hard-they-steer--read-and-measured));
  its tick (`Control.dll:0x100247c0`) reads the target's node position and
  life and no signature, so camouflage does not throw off a round already
  flying — *read*.

## Clan relations: the file's words, straight through — *read*, and *measured*

The relation words in `data.tma` ([04-missions.md](04-missions.md)) are **0
hostile, 1 neutral and 2 allied**, and they are what each clan's SuperAI
holds. Nothing maps a 0/1 file onto a 0/2 runtime.

1. **The loader files them by name** (`MisLoad.dll:0x100015b0`). Each clan
   gets an array with one word per clan; each relation record lands under the
   clan its name matches, ignoring case (`lstrcmpiA`, `0x10001903`); a clan
   named by no record stays 0. Then a neutral clan's whole row becomes 1
   (`0x1000195c`), every entry towards a neutral clan becomes 1, and **every
   clan's word towards itself becomes 2** (`0x100019b2`). The clan info hands
   the array out at `+0x18` (`0x1000136d`).
2. **`iron3d.dll` gives every word to the clan's SuperAI** (`0x100a2773`):
   slot 7 with 2 towards itself (`0x100a27e3`) and the array's word towards the
   others (`0x100a2800`), or 2 when a clan has no array.
3. **The SuperAI keeps the word and an attitude** (`ai.dll:0x10005e80`): the
   word itself, which slot 8 returns (`0x10005f10`) and which the behaviour's
   hostile and friendly tests read, and a float attitude of 1/6, 1/2 or 5/6
   for a word of 0, 1 or 2. Setting clan *i*'s word towards *j* also writes it
   into *j*'s SuperAI towards *i*.
4. **Each clan-brain takt re-reads the word from the attitude**
   (`0x10005f30`): below 1/3 it is 0, above 2/3 it is 2, otherwise 1. The
   attitude then moves 0.0033 a takt, never across the band's edge: a hostile
   one up to 0.2833, a neutral one back to 0.49995, an allied one down to
   0.7166. So a relation stays where the mission put it. What would
   move an attitude across a band — the two change fields the takt adds and
   subtracts — was not found written (a search for their offsets).

*Measured*, over the 101 clans of the 29 shipped missions: towards other clans
the words are 0 on 135, 1 on 137 and 2 on 28, and every clan that names itself
writes 1, which the loader turns into 2. No name misses its clan, and every word
to or from a neutral clan is already 1. After the loader's rules every pair of
clans holds the **same word both ways**: 69 pairs hostile, 69 neutral and 14
allied. On Mission 01 the player (`Plr`) and the target dummies (`Trgt`) are
neutral to each other, as are `Trgt` and `Enm`.

What follows for sensors: a clan's units put only the machines of clans at 0
in their hostile lists and only those at 2 in their friendly ones — a neutral
clan's machine is in neither, and an allied clan's is friendly.

## The player's target — *read*, and *measured*

`iron3d.dll` gives every unit record a **target list** (`+0x38`, made by
`0x100909b0` at `0x1007e570`). The player's target is the current target of the
list on the unit the player drives:

| offset | what |
|---|---|
| +0 | the unit's record |
| +4 | **the current target**, a record, or none |
| +8 | the listed contacts, 8 bytes each: an object id and its owner word |
| +0x14 | the listed records of the unit's own clan |
| +0x20 | a byte: the last rebuild listed a hostile contact |

**What is listed** (`0x10091b90`):

- **The source.** The list is rebuilt from the unit's radar contacts each time
  its record's takt (`0x10075680`) runs the list's takt (`0x10090bb0`, called at
  `0x1007572d`). Life-system slot 9 (`Control.dll:0x1000ef50`) runs the radar
  query ([above](#a-scan-is-a-sphere-a-falloff-and-three-tests--read)) and
  pairs each id with its owner word.
- **Left out** (`0x10091c80`): a `ROBOT_HERO` (`0x1020000`) of the player's
  clan (game `+0xad0`), and any `BUILDING_BRIDGE` (`0x80001000`) or
  `BUILDING_RUINE` (`0x80002000`).
- **Also left out:** a hostile unit whose first order is `0x13`, the value
  `varset.var` names `ORDER_ROBOT_SHUTDOWN` (`0x10077410`, `0x10091d15`). A
  building is never left out this way.
- **Hostile** here means the clan's relation word is 0, from a clan other than
  the unit's own whose type is not 0 (`0x10039440`, `0x100394b0`;
  [below](#clan-relations-the-files-words-straight-through--read-and-measured)).
- **A friend** is only a unit of the unit's own clan (`0x10091070` compares
  owner words). An allied clan's unit is not one.
- **The enemy voice.** When a rebuild lists a hostile contact after one that
  listed none, the driven unit plays `VOICE_ENEMY_DETECTED` (`0x10091ea6`).

**Each takt, on the unit the player drives** (the record at game `+0xaec` in
view states 1 and 3, `+0xaf0` in state 6):

1. **The target is dropped** (`0x10090c28`–`0x10090cb9`) in three cases:
   - its object has left the world tree (no parent);
   - its owner word is `0xfffe`;
   - it is farther across the ground (x and y) than the unit's **sensor range**.

   The sensor range is life-system property `0x50`, the number the stat panel
   prints after "Sensor range:" (string 5073). A machine's control system
   answers it with the radar's value 3 (`Control.dll:0x1000e7fc`, through the
   device getter's id 8, `0x1002b6ec`). The base class has no case for it
   (`0x1000dcc0`). A unit without a radar gets 1 m.
2. **An empty list leaves no target** (`0x10090cc4`).
3. **With no target, one is picked:**
   - the nearest hostile across the ground (`0x10090e30`);
   - failing that, the nearest unit of its own clan (`0x10091070`);
   - failing that, the nearest listed object that has a record (`0x10090d13`).

So **the player has a target whenever anything is on its radar** (*derived*).

**The keys** (`iron3d.dll`'s command handler `0x10071cd0`, cases `0x10072187`,
`0x100721c2`, `0x100721fd` and `0x100723ed`; the bindings are *measured*):

| command | key | what it does |
|---|---|---|
| `CMD_JAMES_SELECT_TARGET` 732, "Target selection" | Tab | the listed entry after the target, wrapping; the first when the target is not listed (`0x10090dc0`) |
| `CMD_JAMES_SELECT_ENEMY` 733, "Select nearest enemy" | E | with a hostile target, the next hostile in list order, wrapping; otherwise the nearest hostile across the ground (`0x10090e30`) |
| `CMD_JAMES_SELECT_FRIEND` 734, "Select nearest friend" | T (F in `ui_other_d.man`) | the same over the unit's own clan (`0x10091070`) |
| `CMD_JAMES_AIM_TARGET` 750, "Aim target" | right mouse button | a pick along the middle of the view (`0x100911f0`) |

A pick that finds nothing leaves the target as it was, except the right button,
which clears it.

**The right button** (`0x100911f0`):

- **The ray** runs from the camera through the middle of its view, the mean of
  its four corner points (`0x100912d7`). It starts at the unit's `+0x98` along
  that line and ends at the sensor range. When the range does not reach past
  the start, nothing happens (`0x100914e2`).
- **The candidates** are the listed objects, less the current target, whose
  bounding spheres lie inside the camera's six frustum planes (`0x10091610`).
- **First choice** (`0x10091780`): the candidate nearest the ray's start that
  meets three conditions:
  - the ray passes through its sphere, taken at 0.4 of its radius for a
    building (kind 3) and 0.6 for anything else;
  - its centre lies ahead, within the ray's length;
  - its centre is farther from the unit than that radius plus the unit's
    `+0x94`.
- **Otherwise** the candidate whose centre projects nearest the middle of the
  screen (`0x100919a0`).
- **The result becomes the target**, or none. The current target is never a
  candidate, so pressing again picks the next object along the line
  (*derived*).

**Setting a target** (`0x10090a70`) plays `TARGET_SELECTED`, `i_trg_sel.wav`,
when it changes on the driven unit's list. It then hands the target to the
unit's guided guns
([29-weapons.md](29-weapons.md#the-players-target-reaches-the-turret--read)).
When an object leaves the game, every list aimed at it is set to none
(`0x1007d610`, `0x1007d690`).

**A vacant vehicle selects itself** (`0x100757ad`–`0x100758a9`):

- **When.** A unit of a type-3 clan does this once in its life (`+0x134`):
  when the player's hero comes within the hero's sensor range across the
  ground, unless the view is in state 5.
- **What happens.** The unit becomes the hero's target. The game shows string
  3040, "Vacant vehicle detected...", and plays `VOICE_UNIT_DETECTED`,
  `vc_u_det.wav`.
- **Why it matters.** Enter captures that target
  ([27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)).

*Measured*:

- The four commands are bound to Tab, E, T and the right button in
  `ui_other.man` and `addition.man`; `ui_other_d.man` puts the friend on F.
- The three sounds are named in `ui/game_resources.cfg`.
- The constants are `varset.var`'s.
- **On Mission 01** only `Enm` is hostile to `Plr`, and its one unit is
  `tut1_e1`. E finds nothing else. The target dummies (`Trgt`) are neutral to
  the player. Tab, the right button or the nearest-listed rule picks them.

## How the game colours what it marks — *read*, and *measured*

**One rule gives every mark its colour** (`iron3d.dll:0x10065440`, *read*). It
takes the viewing clan and the marked object's clan and returns a Direct3D
colour, `0xAARRGGBB`. Every one of its six callers passes the player's clan
(game `+0xad0`) as the viewer. The tests run in this order, and the first that
holds decides:

| the marked object's clan | colour | r, g, b |
|---|---|---|
| the viewer's own | `0xff8080ff`, light blue | 128, 128, 255 |
| type 0, nature (`0x100394b0`) | `0xffffff00`, yellow | 255, 255, 0 |
| type 3, neutral (`0x100394a0`) | `0xffa0a0a0`, grey | 160, 160, 160 |
| its word towards the viewer is 1, neutral (`0x10039460`) | `0xffff00ff`, magenta | 255, 0, 255 |
| its word is 2, allied (`0x10039480`) | `0xff00ffff`, cyan | 0, 255, 255 |
| its word is 0, hostile (`0x10039440`) | `0xffff0000`, red | 255, 0, 0 |
| any other word | `0xffffff00`, yellow | 255, 255, 0 |

- **The type comes first.** A neutral clan is grey whatever its words, and a
  nature clan yellow.
- **The word is the marked clan's.** The tests ask the object's clan record's
  SuperAI (slot 8) for its word towards the viewer. After loading, every pair
  of clans holds the same word both ways
  ([Clan relations](#clan-relations-the-files-words-straight-through--read-and-measured)),
  so the direction does not change a shipped mission's colours.

**On Mission 01** (*measured*), as the player (`Plr`) sees them:

- the five target dummies (`Trgt`, type 2, word 1) are **magenta**;
- `tut1_e1` (`Enm`, type 2, word 0) is **red**;
- `helic` and `tut1_mf1` (`Ntrl`, type 3) are **grey**;
- the hero and the bridge (`Plr`), and the two bots once captured, are
  **light blue**.

**Where the rule is used** (*read*: its six callers):

- **The cockpit radar** (`0x1003fb90`, one of the HUD widgets `0x10043b20`
  draws). Each entry of the driven unit's target list is placed at its bearing
  and distance, as a small mark in the colour of its owner word's clan
  (`0x100402bd`). The current target's mark also gets an outline in the same
  colour (`0x100403d4`, `0x100404d1`). A flyer's mark is a cross and any other
  unit's a square (`0x10075f70`,
  [35-hud.md](35-hud.md#the-radar--read-and-seen)).
- **The target panel** (`0x10040f30`). The widget `0x10040940` hands it the
  driven unit's current target, list `+4`.
  - **Where.** A 150 × 174 panel at the bottom left of the HUD's 640 × 480,
    (0, 306) to (150, 480). It shows the target through a camera in
    (9, 315)–(137, 443).
  - **The frame.** The target is framed by a square outline in its colour
    (`0x100415a8`). Over the first second after the panel's clock (`+0x2c4`)
    restarts, the frame eases in from the middle of the screen, (320, 240).
    After that it is a square of half-side 200 × a scale about the target's
    projection; a scale below 0.025 is made 0.1, one above 1.1 held to 1.1.
    The whole panel, the frame's scale included, is read in
    [35-hud.md](35-hud.md#the-target-panel-and-the-players-own-unit).
  - **The sister widget** (`0x10040a40`) shows the driven unit itself, with no
    frame, in the panel at the bottom right, (490, 306)–(640, 480).
- **The unit marker** (`0x10077d80` for units, over every unit record by
  `0x1007d5e0`; `0x10034900` for buildings). In the rule's colour
  (`0x10077e71`) it draws:
  - **Brackets.** page9's bracket, the 14 × 28 at (55, 0) of `ui/ui.lib`'s
    `ui_tex9.tex`, cut by the HUD's loader `0x100433a0`. One stands to the left
    of the unit's projected centre and one, mirrored, to its right, 28 tall
    about the centre. The gap either side is 44 × a figure the record answers
    for the camera distance (`0x1007dff0`, its slot 5, not read).
  - **Signs**, for a clan of type 1 or 2: a sprite the clan record's `+0x14`
    indexes, and a class icon by `Type` (transport `0x1002000`, builder
    `0x1004000`, anything else).
  - **Bars** below: page9's 27 × 16 bar frame tinted blue, then a green
    (30, 180, 80) and an orange (255, 130, 50) bar. A unit of the player's
    clan is also named in green.

  **Only two units get one.**
  - A unit **selected** in the commander's view: its record's `+0x80` is 1.
  - The unit under the **mouse cursor**, the cursor object at game `+0x24`,
    which `0x1008da40` fills. In view states 1, 3, 4 and 6, the cockpit's, the
    cursor picks nothing (`0x1008daa4`–`0x1008dac8`).

  It skips the player's own hero. It also skips a unit neither of the player's
  clan nor in the list at the player clan record's `+0x54` (`0x1007e660`,
  `0x10039370`).
- **The map** (`0x10077690` for units, `0x100347f0` for buildings): each mark
  in the rule's colour. A selected unit is outlined in white, and a hero in
  green.

**In the cockpit the target is not bracketed in the world** (*read*, as a
search). The screen projection (`0x100cd1a0`) is reached from:

- the target panel (`0x100414b1`);
- the unit marker (`0x1007e03f`);
- the right button's pick (`0x10091a20`);
- a commander's screen (`0x1004f75b`);
- the guided lock (`0x1009ce77`).

Its four other calls come from two functions beside it (`0x100cd260`,
`0x100cd2a0`) that no call reaches and no dword in the image points at.

**The guided lock** (`0x1009cd30`, run for each weapon slot) draws four 9 × 9
corners. They close from the HUD's (30, 30)–(630, 450) onto the target as the
lock counts. Their colour is green, (20, 205 + 50 × the lock's share, 20). It
plays `TARGET_ZOOM` and `TARGET_READY` (`0x1009d0bc`, `0x1009d15b`).

So **a first-person target shows three ways**: as its radar mark's outline, as
the target panel's frame, and, for a guided gun, as the lock's corners. The
first two take the rule's colour.

## Not established

- ~~How the mission file's 0/1 relation words become the runtime's 0 and 2.~~
  Answered: the file holds 0, 1 and 2, and they pass straight through
  ([Clan relations](#clan-relations-the-files-words-straight-through--read-and-measured)).
- ~~How the HUD marks the player's target, and what plays `TARGET_READY` and
  `TARGET_ZOOM`.~~ Answered: the cockpit radar outlines its mark and the target
  panel frames it, in one colour rule by clan; the guided lock's corners play
  both sounds
  ([How the game colours what it marks](#how-the-game-colours-what-it-marks--read-and-measured)).
  Still open there:
  - ~~the radar mark's shape (`0x10075f70`)~~, answered in
    [35-hud.md](35-hud.md#the-radar--read-and-seen);
  - ~~the target panel frame's scale~~, answered in [35-hud.md](35-hud.md);
  - the unit marker's gap (the record's slot 5);
  - what fills the clan record's list at `+0x54`, which decides which other
    clans' units get a marker.
- What the unit record's `+0x94` and `+0x98` are: the right button's margin
  and its ray's start.
- What the player's map and radar display show. `IArealMap`'s side of the
  radar report is answered ([above](#what-the-ai-does-with-it--read)), but
  `iron3d.dll`'s drawing was not read. One negative, as a search:
  `iron3d.dll` never makes the two radar queries the behaviour makes —
  `IControl` slot 10 with id `0x10` for the contact list, slot 4 with id 8 for
  the range — where the same search finds both in `Behavior.dll`
  (`0x1002336b`, `0x10023243`); so the cockpit does not draw the radar
  component's scan in that form. ~~The cockpit radar~~ is answered in
  [35-hud.md](35-hud.md#the-radar--read-and-seen): it draws the driven unit's
  target list about the camera's heading, and its `RADAR` string
  (`0x1004011d`) is a sound's name, not a label. Still open: the map,
  `0x10073550`, which loads `minimap` and `map_compass_icon`, marks units and
  buildings in the same colours
  ([How the game colours what it marks](#how-the-game-colours-what-it-marks--read-and-measured));
  the rest of it is not read.
- ~~What asks an AI machine's device manager to switch camouflage *on*.~~
  Answered: the unit takt's engagement check
  ([When the AI wears it](#the-detection-shield-hides-all-three-and-camouflage-hides-them-again--read-and-measured)).
- ~~Where the radar module's list of every kept contact (`+0x3c`) is
  emptied.~~ Answered: nowhere, and its only reader is dead code
  ([above](#what-the-ai-does-with-it--read)).
- ~~Class 17 on projectiles.~~ Answered in
  [29-weapons.md](29-weapons.md#guided-rounds-differ-in-how-hard-they-steer--read-and-measured):
  a missile's seeker, `cos(value 0)` its cone. It does not use the detection
  test above.
- What would move a SuperAI's attitude from one relation band to another:
  nothing was found writing the two change fields its takt applies
  (`ai.dll:0x10005f46`, `0x10005f60`).
