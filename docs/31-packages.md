# Packages — the orders a commander gives

What a unit does is a stack of **tasks** in `Behavior.dll`, and a task comes
from an **order**. The player gives orders through two menus in `iron3d.dll`.
The game calls the menu entries *orders* (string 5084), and the manual calls
them *packages*. Mission scripts give the same orders through `fn15` and `fn28`
([15-behaviour.md](15-behaviour.md)). This page maps each menu entry to its
order and task, and says what each task does and who may run it.
`openparkan.packages` holds the orders, the packages and `packages_for`.

**Every claim is tagged**, as in [23-economy.md](23-economy.md). *Measured*
is re-derived by `openparkan verify`. *Read* comes from the disassembly at the
address given. *Guess* fits the evidence but is not established.

## The orders — *measured*

`varset.var` declares them "should ident with constants in Visual C++". The
task factory `MTaskStack::CreateTaskFromOrder` (`Behavior.dll:0x10033a80`) is a
switch on exactly these numbers (*read*):

| # | order | task built (vtable) | status text |
|---:|---|---|---|
| 1 | `ORDER_ROBOT_STOP` | stop (`0x100596ac`) | stopping |
| 2 | `ORDER_ROBOT_GO` | go (`0x10059e74`) | moving |
| 3 | `ORDER_ROBOT_ATTACK` | `M_Task_Attack` (`0x10059d78`); **a go task if the target is a place** (`0x10033dab`) | attacking |
| 4 | `ORDER_ROBOT_PATROL` | patrol (`0x10059d38`) | patrolling |
| 5 | `ORDER_ROBOT_SEARCH` | search (`0x10059cf8`) | searching |
| 6 | `ORDER_ROBOT_TRANSPORT` | transport (`0x10059ca8`) | transporting |
| 7 | `ORDER_ROBOT_BUILD` | build (`0x10059c60`) | building |
| 8, 9 | `ORDER_ROBOT_RELOAD`, `_REPARE` | `M_Task_Reload` (`0x10059be0`), one case for both | refitting, repairing |
| 10 | `ORDER_BUILDING_MINE` | `M_Task_Mine` (`0x10059b60`) | |
| 11 | `ORDER_BUILDING_CHARGE` | **none**: "Incorrect Order" | |
| 12 | `ORDER_BUILDING_CONSTRUCT` | `M_Task_Construct` (`0x10059b24`) | |
| 13 | `ORDER_ROBOT_RANDOMGO` | random go (`0x10059ba0`) | |
| 14, 16 | *(undeclared)* | `M_Task_Research` (`0x10059f58`) | |
| 15 | *(undeclared)* | migrate (`0x10059aac`), an animal's default order (`0x100088f0`) | |
| 17 | `ORDER_ROBOT_CAPTURE` | **the search task again**, restricted to building types `0x8017365e` (`0x100341a3`) | capturing |
| 18 | *(undeclared)* | a task logging "ShowUpgrade" (`0x10059ae8`) | |
| 19 | `ORDER_ROBOT_SHUTDOWN` | shutdown (`0x10059eec`) | shutting down |
| 20 | `ORDER_ROBOT_LEAVE` | leave (`0x10059e38`, "Leave"): **the escape**, below | escaping |
| 21 | `ORDER_ROBOT_STAYGROUND` | stay ground (`0x10059eb0`) | standing |
| 22 | `ORDER_ROBOT_FOLLOW` | follow (`0x10059dfc`) | following |
| 23 | `ORDER_ROBOT_GETONBOARD` | get on board (`0x10059dc0`) | getting onboard |
| 24 | `ORDER_ROBOT_UPGRADE` | upgrade (`0x10059c20`) | upgrading |

An order carries a target, given as `TARGET_BY_LOGIC_ID` `0x201`, `_BY_PLACE`
`0x202`, `_BY_TYPE` `0x203`, `_NOT_DEFINED` `0x204` or `_BY_NAME` `0x205`. It
also carries an insert mode: to the end 1, to the start 2, or replace 3.

A unit's status line is one of 18 strings, `iron3d.dll` 6180–6197 (*measured*):
no order, stopping, shutting down, standing, moving, escaping, following,
getting onboard, attacking, patrolling, searching, transporting, building,
refitting, repairing, capturing, upgrading, unknown. The text column above
pairs them with orders by name. That pairing is a *guess*; which code picks the
string was not read.

## The commander's menus — *measured*, and *read*

`iron3d.dll` holds the entries as two tables of 20-byte rows (`0x10104f98`,
`0x10105150`). Each row is a command id, a robot-type mask, a needs-a-target
flag, a pick mode and a string id. A row appears for a selection only when its
mask holds every selected unit's type (`0x1007aaa0`). Build and upgrade rows
are further gated by research (`0x1007bbb0`).

`0x1007b740` executes a command. Commands that need no target go straight to
the dispatcher `0x10079230`, which gives every selected unit its order with
`INSERT_ORDER_REPLACE`. The others first enter a pick mode.

| id | package | string | offered to | order it gives (*read*) |
|---:|---|---:|---|---|
| 0 | **Standby** | 5000 | every robot | `STAYGROUND`, no target |
| 1 | **Route** | 5001 | every robot | pick mode 4, then `GO` to the picked place |
| 2 | **Search and capture** | 5002 | every robot | `SEARCH` by type, building mask `0x8017365e` |
| 3 | **Seek and destroy** | 5003 | every robot | `SEARCH`, no target, parameter −1 |
| 4 | **Attack** | 5004 | wingman menu | pick mode 3, then `ATTACK` the unit by logic id (`0x10078ce0`) |
| 5 | **Capture building** | 5005 | wingman menu | pick mode 2, then `SEARCH` on that building's logic id |
| 6 | **Guard** | 5006 | every robot | pick mode 3, then `PATROL` a unit or building by logic id, or a place (`0x10078df0`) |
| 7 | **Refit** | 5007 | every robot | `RELOAD`, no target |
| 8 | **Transport minerals** | 5020 | transports | `TRANSPORT`, no target, parameter −1 |
| 9 | **Search minerals** | 5030 | builders | `SEARCH` by type `0x10001000` |
| 10–16 | **Build** Mine, Warehouse, Factory, Outpost, Res. Center, Light Tower, Heavy Tower | 5031–5036, 5046 | builders | pick mode 4 (placement), then the builder's order |
| 17–23 | **Upgrade** the same seven | 1008, 1029–1033, 5047 | builders | `UPGRADE` by logic id, with a building type (`0x10078f60`) |
| 24 | **Follow me** | 5008 | wingman menu | `FOLLOW`, parameter 50, on the commander's logic id |

The HQ table holds rows 0–3 and 6–23 (22 rows). The masks are `0x0103e000`
(every robot type), `0x01002000` (transports) and `0x01004000` (builders). The
second table holds Standby, Follow me, Search and capture, Seek and destroy,
Attack, Capture building and Refit (7 rows). That it is the **wingman menu**
(`CMD_JAMES_WINGMAN_MENU`) is a *guess*, backed by *C01 Mission 3*'s tip, which
names "Stand By" and "Follow Me" as commands to the player's wingman.

**The Outpost is the building the files call a hangar** (*read*). The upgrade case for "Upgrade Outpost"
pushes building type `0x80000040`, which `varset.var` calls
`BUILDING_HANGAR` (`iron3d.dll:0x1007baf7`). The other six cases push mine
`…04`, storage `…08`, plant `…10`, institute `…400`, medium tower `0x80100000`
and large tower `0x80200000`.

## What each package does — *read*

Each task class answers the same vtable slots. Slot 3 sets the target, 6 starts
the task, 7 is its tick, 12 is its **interrupt priority** and 14 is a debug
label. The numbers quoted are the behaviour constants' compiled defaults
(`Behavior.dll:0x10016250`, bound by name at `0x10016480`; no shipped file
overrides them). They are attached to a task **by their names**. Where their
readers were not traced, that is marked.

- **Standby — stay ground.** The unit stays where it is. Its interrupt
  priority is **0 for every reason** (`0x10031c80`), so nothing it sees makes
  the behaviour pull it off station. That fits the tip's "fixed firing point":
  the guns still aim and fire through the unit's own fire control. That the
  fire control runs independently of the task is a *guess*; it was not traced.
- **Route — go.** It walks to the place at `Go_SpeedPercent` 1.0 of the unit's
  speed and ends when it stops there ("We are staying... task over",
  `0x1002b670`), or fails if the place is unreachable. The go task will not be
  interrupted by reasons 0–2 or 5, only 3 and 4 (`0x1002b390`). Whether the
  menu can chain waypoints was not established: it issues one place.
- **Seek and destroy — search, no target.** `SetTarget` (`0x10030110`) sets
  the task's enemy mode for a target of `0x204`. Each plan picks **the nearest
  hostile warrior, builder or transport the clan knows of, within 3,000**, and
  walks to where it was last seen (below). The task does not attack anything
  itself: its interrupt priority lets an engagement through at 1 (`0x10030027`),
  so the behaviour's own radar engagement ([Between orders](#between-orders--read))
  does the fighting on the way. It never ends on its own; with no enemy known it
  roams the map.
- **Search and capture — search by building type.** A unit of size class ≤ 2 —
  tiny or small, which includes every hero — or `SetTarget` fails
  (`0x100301a9`). Each plan picks **the nearest building the clan knows of that
  is not its own**, by these rules (`0x100306f0`):
  - **Types:** the mask `0x8017365e` holds every building type but three — the
    power mast (`0x80000080`), the little teleport (`0x80000100`) and the heavy
    tower (`0x80200000`) — and the plan skips main teleports and bridges as well
    (`0x10030859`). *Measured:* every type it can pick has a pod on every model
    but the ruins, which have none; the heavy towers have pods, so only an
    explicit Capture building takes one.
  - **Whose:** any clan but the unit's own — enemy, neutral or ally alike
    (`0x100308d0`).
  - **Distance:** across the ground, and **a generator counts at half its
    distance** (`0x100308ac`), so a capturer prefers a generator up to twice as
    far as anything else.
  - **Not while it is going up:** a building in its construction-sphere phase is
    skipped (`0x10015260`, variable `0x202`).
  - **Nothing else:** no test of what the building is worth, of who defends it,
    or of whether another unit is already on its way.

  It walks to the building's pod at the unit's speed × `Go_SpeedPercent`
  (`0x1003094c`). A **flying** capturer first lands at the nearest walkable
  corner of the building's ground contour, then walks to the pod. If the walk to
  the pod is refused — a ruin has no pod — the plan falls through to roaming,
  and the next plan picks the same building again, so a capturer can hang about
  a ruin (*derived*). When the
  building turns the unit's own clan the task logs "Building [..] captured"
  (`0x10030474`) and, being a search by type, **plans the next one** at once
  (`0x100304b9`): it does not end, and the code read gives it no escape — it
  walks out of the building towards its next target (below).
  - **Rescan:** every 3 s plus up to 3 s, against 15 plus up to 15 s in the other
    modes (`0x100301b2`, `0x1003011b`); a plan also runs whenever the unit stops.
  - **No fighting on the way:** its interrupt priority is 0 for engagements
    (reasons 0–2 and 5), and for a refit unless its life is at most 0.2 or its
    charge at most 0.3 (`0x10030000`).
- **Capture building — search on one building.** The same class with the picked
  building's logic id, under the same size rule, and `SetTarget` also refuses
  one of the unit's own clan or a main teleport. It ends — successfully — when
  the building is the unit's clan's (`0x100304a4`: the single-target flag
  `+0x60` that `SetTarget` sets for a logic id). A mission script's
  `ORDER_ROBOT_CAPTURE` by logic id takes the same path. With the task gone
  and the unit standing in the building, **the unit takt gives it the escape**
  (below).
- **Guard — patrol.** Pick a unit, a building or a place (`0x1002d520`). The
  patrol task moves to a new random point around it on a timer (`0x1002dd90`),
  and only while `Behavior.ini`'s `DeterminMode` is 0 (`0x1002d900`). The
  compiled defaults by target, by name (the building speed reads as a typo for
  0.8 — *guess*):

  | guarding | radius | new point every | speed |
  |---|---:|---|---:|
  | a unit | 60 | 5 + up to 10 s | 1.0 |
  | a building | 30 | 60 + up to 60 s | 80 |
  | a place | 60 | 20 + up to 10 s | 0.8 |

  `Patrol_Attack_Range` is 400. That it is the range at which a guard engages
  is a *guess*; its reader was not found.

  **A small unit told to guard a building of another clan captures it first**
  (`0x1002da7a`): if the unit is of size class 2 or less, the patrol queues,
  behind itself, a search on that building (a capture), an escape from it and a
  patrol of it again, and ends. What the unit's `IBehaviour` slot `0x10` test
  it also makes is *unknown*.
- **Refit — reload.** `M_Task_Reload` walks to a ground-level dock and waits
  until life, charge and ammunition are at 98%
  ([27-ownership.md](27-ownership.md)). `ORDER_ROBOT_REPARE` builds the same
  task.
- **Follow me — follow.** It keeps within a radius taken from the order's
  parameter: the menu passes 50 (`0x1002ad80`, "FollowRadius").
- **Attack.** `M_Task_Attack` on a logic id. It cancels when the unit has no
  weapon, takes the nearest target when given none (`0x10026fd0`), and ends
  when the target is dead ("mission accomplished", `0x10027250`). By its
  constants' names (readers not traced):
  - it holds a fight distance of 30 + up to 20, strafes 80 left or right, and
    fires from at most 200;
  - it closes at 0.8 + up to 0.2 of its speed and fights at 0.7 + up to 0.3;
  - it changes course every 4 + up to 4 s.
- **Search minerals — search by type `0x10001000`.** The minerals mode picks
  **the nearest mineral lode nobody has found yet**, from the whole map's list
  (`ArealMap.dll`'s `SetMineralLode`, `0x10021db0`), not from what the clan has
  seen. It walks there; within 10 m it marks the lode found for everyone, logs
  "Resource Found" (`0x10030535`) and ends. What sets the lodes was not traced.
- **Transport minerals, Build, Upgrade.** These are the transport, build and
  upgrade tasks; [32-builder.md](32-builder.md) covers what they do.

## The escape — *read*

`ORDER_ROBOT_LEAVE` is the **escape**: a task that takes a unit off a building
and out onto open ground. The game gives it: no menu entry offers it and no
shipped mission script names it ([How the missions use them](#how-the-missions-use-them--measured)).

**Where it goes** (start, `0x1002ba50`):

- **With no target** it tries random points around the unit, each inside the
  map by at least 100 and on a usable areal: 400 tries within 150 of it along
  each axis, then 300 within 300, 200 within 400 and 200 within 1,000 — the log
  names which ("Leave found 150m. place" …). With none it fails ("Cannot
  Leave").
- **Away from a building** (by its logic id) it takes a random point 20 to 70
  beyond the building's bounding sphere, at a random angle, 200 tries
  ("LeaveOut found").

It walks there at the unit's speed × `Go_SpeedPercent`.

**When it ends** (takt, `0x1002c100`): when the unit's walker has nothing
left to do — no path, no step, no wait (`0x1003dd80`), which is when it has
reached its point. Nothing in the takt tests the ground under the unit at the
end: that the point, on a usable areal, is always off the building is a
*guess* the 20-second check below backs. Two refinements:

- an escape with no target that still finds the unit on a building after 20 s
  stops it and routes it out through the building's own paths ("LEAVE IS TOO
  !!!");
- an escape from a named building is held while that building's state
  (variable `0x205`) reads 1 or `0x309`, or its `0x20c` reads 1 — that this
  keeps a unit clear while the building goes up is a *guess*
  ([32-builder.md](32-builder.md)).

**Nothing interrupts it**: its priority is 0 for every reason (`0x1002b9f0`),
so an escaping unit neither engages nor refits on the way. When it ends, the
task beneath it on the stack, if any, resumes.

**Who gives it** — five places, all in `Behavior.dll`:

| when | escape | inserted | read at |
|---|---|---|---|
| a unit **stands on a building** with **no order or a stop** — not a ruin, not a destroyed building, and not passing a test through the building's `IAnimation` (interface `0xb`, slot 16) whose meaning is *unknown* | no target | replacing | unit takt `0x10005408` |
| the behaviour's control flag `0x10` is switched back on while the unit stands on a building | from that building | first | mode setter `0x10006f48` |
| a factory has just made the unit | no target | replacing | `0x1002aa6e`, "Adding Robot to game..." |
| a small unit guarding another clan's building, after capturing it | from that building | queued | patrol `0x1002da7a` |
| a building begins its construction sphere, for every unit within its radius + 15 not already escaping or upgrading | from that building | first | ShowUpgrade `0x10031571` |

So **a capture is followed by an escape when the capture task ends** —
Capture building, a script's capture by logic id, or the guard sequence — the
unit takt seeing an idle unit on the building. The menu's Search and capture
does not end at a capture, so that path does not run. That the control flag
`0x10` is the AI's, off while the player drives a unit — which would make the
second row "the player lets go of a unit on a building" — is a *guess*: it fits
the flag gating the unit's own takt, but the callers of the mode setter were
not traced.

*Against what the game looked like:* in play an escape follows every capture,
and follows leaving a driven bot inside a building or on its grounds. The code
read gives the second through the mode setter (if flag `0x10` is the player's
hand, a *guess*) and the first only where the capture task ends: after Capture
building or a script's capture, not after the menu's Search and capture, which
replans at `0x100304b9`. What gives a Search and capture unit its escape, if
anything does, is *unknown*.

## Where a search looks — *read*, and *measured*

**What the clan knows.** Enemy and building candidates come from the clan's
**areal map**, not from the whole world and not from the unit's own radar list
(`0x10015260`, `0x10015310`). That map keeps, for each areal, a snapshot of the
units and buildings in it. Every 2.0–4.9 s each unit's radar module reports its
position and radar range ([25-sensors.md](25-sensors.md)). The clan map then
refreshes the snapshot of every areal within that range from the game's
system map, and stamps it with the time (`ArealMap.dll:0x10001ec0`,
`0x10001dd0`, `0x10001840`). So a search picks from **what the clan's radars
have swept, as last seen there**. A building or unit in an areal no radar of
the clan has covered is not a candidate. Whether a clan's map starts out
knowing anything was not traced.

**The plan runs in this order** (`0x100306f0`), and the first that finds
something is taken:

| mode | candidates | pick |
|---|---|---|
| minerals | the map's undiscovered mineral lodes | nearest |
| capture | the clan's known buildings of the mask, not its own, not under construction, no main teleport or bridge | nearest across the ground, a generator at half distance |
| capture, nothing found | the clan's known hostile warriors, builders and transports within 300 | a point away from them (below) |
| enemies | the clan's known hostile warriors, builders and transports within 3,000 | nearest the walker accepts |
| nothing found in any | — | a random point on the map |

**Hostile** is the behaviour's own test (`0x1000d460`): not the unit's clan; for
a neutral clan's unit nothing is hostile, for a nature clan's everything is;
otherwise the clans' relation ([25-sensors.md](25-sensors.md)). **HQs and heroes
are never hunted**: the mask `0x100e000` holds transports, builders and
warriors only (*measured* against `varset.var`'s robot types).

**Roaming.** With nothing to go for, the unit tries up to 150 random points at
least 100 inside the map's bounds, and walks to the first that lies in a usable
areal (`0x10030ed2`). It moves at its speed × `Go_SpeedPercent`, 1.0, in every
mode.

**The capturer's retreat, as read.** A capturer with no building to take and
hostile robots within 300 sums the directions from each of them to itself. It
then looks for a usable point 50, 53, 56 … up to 300 along that direction
(`0x10030c86`). The point is **not added to the unit's position**. *Measured:*
every map's navigation mesh starts at (0, 0), 33 of 33. So the point lies within
300 of the map's corner, or off the map, where no areal is usable and the unit
roams instead. That this is a slip in the game's code rather than a design is a
*guess*.

## Who may run which — *measured*, and *read*

A unit's **Type** picks its behaviour profile. `Behavior.dll:0x10008a80`
switches on the unit's Type (`MBehaviour+0xafc`) and loads that profile from
`behpsp.res`. Each robot profile's own `Type` variable is its `varset.var`
robot type:

| Type | varset.var | profile | placed from folder |
|---|---|---|---|
| `0x01002000` | `ROBOT_TRANSPORT` | `prof_trn.var` | `TRANSPRT` (11) |
| `0x01004000` | `ROBOT_BUILDER` | `prof_bld.var` | `BUILDER` (17) |
| `0x01008000` | `ROBOT_BATTLEUNIT` | `prof_war.var` | `BATTLE` (187), `AutoDEMO` (4) |
| `0x01010000` | `ROBOT_HQ` | `prof_hq.var` | `HQ` (8), and 3 in `BATTLE` |
| `0x01020000` | `ROBOT_HERO` | `prof_hero.var` | `HERO` (37) |
| `0x20000000` | `CLASS_ANIMAL` | `prof_animal.var` | `ANIMAL` (29) |
| `0x01001000` | — | `prof_exp.var` | never placed; the binary never names the file |

So a placed unit's role is its mission `Type` property, which follows its
`UNITS/UNITS` folder on 293 of 296. Buildings' profiles all carry
`0x7fffffff`. `prof_universal.var` is never named either: 15 of the 19
`prof_*.var` are loaded.

**Each profile carries fourteen task flags** (`MBehaviour+0x834`–`+0x868`,
bound at `0x10022e20`):

| profile | Stop | Go | Attack | Search | Patrol | Reload | Repare | Transport | Build | RandomGo | Charge | Mine | Construct | Research |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| war, trn, hq, hero, animal, universal | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| bld | 1 | 1 | 0 | 0 | 0 | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |
| exp | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 0 | 0 | 1 | 1 | 1 | 1 | 1 |
| generator | 1 | | | | | | | | | | 1 | | | |
| mine | 1 | | | | | | | | | | | 1 | | |
| plant | 1 | | | | | | | | | | | | 1 | |
| every other building | 1 | | | | | | | | | | | | | |

**Nothing was found that reads these flags.** The search covered the
displacements `+0x834`–`+0x868` on any base register, the `+0x820` block base,
and indexed forms in `Behavior.dll`, `iron3d.dll` and `ai.dll`. So a builder's
0 for Attack describes the design; whether the engine enforces it is *unknown*.
What does gate packages, as read:

- the menu masks (transports alone get Transport minerals; builders alone get
  building);
- the size rule on the two capture packages;
- the attack task's "weapon absent" cancel.

## Between orders — *read*

- **Engaging.** Every behaviour tick, `0x10017e70` scores the radar module's
  hostile contacts ([25-sensors.md](25-sensors.md)) through the current task.
  By default a contact scores 0 beyond 500 (`0x10001010`). `0x100179c0` then
  inserts a reason-0 task against the best contact, unless any of these holds:
  - `DeterminMode` is non-zero;
  - the unit's own type has bit 31, a building;
  - its clan is neutral (type 3);
  - its current order is `BUILD`;
  - it is an animal that is not migrating;
  - the current task's interrupt priority is below 0.3.

  Standby, shutdown, go, build and transport all answer 0 to reason 0, and so do
  Search and capture, Capture building and Search minerals; Seek and destroy
  answers 1. **So a
  unit engages on its own while stopped, guarding, seeking and destroying or
  attacking, but not while standing by, moving on a route, capturing, searching
  for minerals, building or transporting.**
- **Refitting.** A unit whose life or charge is below half, or whose guns are
  mostly dry, sends itself to a dock as a reason-3 task
  ([27-ownership.md](27-ownership.md)). The default priority lets reason 3
  through only if a dock is reachable (`0x100018a0`).
- **With no order.** An animal's default order is 15, migrate; a mine's is to
  mine (`0x100088f0`). A unit placed inside a building gets a patrol inside it
  ("Give default patrol inside building order", `0x1000ac4c`). **A unit left
  idle or stopped on a building escapes** from it ([The escape](#the-escape--read)).
  For other units no default order was found.

## How the missions use them — *measured*

The 58 shipped scripts name orders 318 times, mostly through `fn15` (236) and
`fn28` (85):

| order | uses | scripts |
|---|---:|---:|
| `PATROL` | 140 | 24 |
| `ATTACK` | 75 | 15 |
| `CAPTURE` | 48 | 14 |
| `GO` | 35 | 5 |
| `SHUTDOWN` | 28 | 12 |
| `BUILD` | 11 | 2 |
| `BUILDING_CONSTRUCT` | 9 | 9 |
| `RELOAD` | 7 | 2 |
| `TRANSPORT`, `UPGRADE` | 5 each | 4, 2 |
| `SEARCH` | 3 | 3 |
| `STOP` | 2 | 2 |
| `BUILDING_MINE`, `RANDOMGO`, `FOLLOW`, `GETONBOARD` | 1 each | 1 |

`REPARE`, `LEAVE`, `STAYGROUND` and `BUILDING_CHARGE` are never used: the last
has no task at all. Patrols are by logic id 75 times and by place 58, and
captures by logic id 34 times.

## Not established

- Whether anything enforces a profile's task flags.
- What sets the mineral lodes the minerals search picks from.
- Whether a clan's areal map knows anything before its units' radars have swept
  it, and how old a snapshot may be before a search still trusts it.
- What the search task's start does to the unit's `+0x35c` controller (it asks
  for mode 2, as other tasks do).
- What reasons 1, 2, 4 and 5 of the interrupt priority are. 0 is an engagement
  and 3 a refit.
- What orders 14, 16 and 18 are called. 14 and 16 build the research task.
- Whether Route can chain waypoints.
- What reads `Patrol_Attack_Range`, and the building patrol speed of 80.
- Which string the status line shows for which task.
- Whether a wingman menu is what the second table is.
- What the building's `IAnimation` test in the unit takt admits (interface
  `0xb`, slot 16, given the unit's `IGameObject` slot-16 value), which keeps some
  units on a building from escaping.
- Who calls the behaviour's mode setter, and so whether flag `0x10` is the
  player's hand on a unit.
- What a building's variable `0x205` holds, which an escape from it waits out.
