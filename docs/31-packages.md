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

| # | order | task built (vtable) | status line (*read*) |
|---:|---|---|---|
| 1 | `ORDER_ROBOT_STOP` | stop (`0x100596ac`) | stopping |
| 2 | `ORDER_ROBOT_GO` | go (`0x10059e74`) | moving |
| 3 | `ORDER_ROBOT_ATTACK` | `M_Task_Attack` (`0x10059d78`); **a go task if the target is a place** (`0x10033dab`) | attacking |
| 4 | `ORDER_ROBOT_PATROL` | patrol (`0x10059d38`) | patrolling |
| 5 | `ORDER_ROBOT_SEARCH` | search (`0x10059cf8`) | searching |
| 6 | `ORDER_ROBOT_TRANSPORT` | transport (`0x10059ca8`) | transporting |
| 7 | `ORDER_ROBOT_BUILD` | build (`0x10059c60`) | building |
| 8, 9 | `ORDER_ROBOT_RELOAD`, `_REPARE` | `M_Task_Reload` (`0x10059be0`), one case for both | refitting, repairing |
| 10 | `ORDER_BUILDING_MINE` | `M_Task_Mine` (`0x10059b60`) | unknown |
| 11 | `ORDER_BUILDING_CHARGE` | **none**: "Incorrect Order" | unknown |
| 12 | `ORDER_BUILDING_CONSTRUCT` | `M_Task_Construct` (`0x10059b24`) | unknown |
| 13 | `ORDER_ROBOT_RANDOMGO` | random go (`0x10059ba0`) | unknown |
| 14 | *(undeclared)* | `M_Task_Research` (`0x10059f58`): **one technology by its id**, from the player's research panel (below) | unknown |
| 15 | *(undeclared)* | migrate (`0x10059aac`), an animal's default order (`0x100088f0`) | unknown |
| 16 | *(undeclared)* | `M_Task_Research` again: **what a design needs, by its `.dat` name**, from the AI (below) | unknown |
| 17 | `ORDER_ROBOT_CAPTURE` | **the search task again**, restricted to building types `0x8017365e` (`0x100341a3`) | capturing |
| 18 | *(undeclared)* | a task logging "ShowUpgrade" (`0x10059ae8`): **the construction sphere**, given only by `Behavior.dll` itself ([32-builder.md](32-builder.md)) | unknown |
| 19 | `ORDER_ROBOT_SHUTDOWN` | shutdown (`0x10059eec`) | shutting down |
| 20 | `ORDER_ROBOT_LEAVE` | leave (`0x10059e38`, "Leave"): **the escape**, below | escaping |
| 21 | `ORDER_ROBOT_STAYGROUND` | stay ground (`0x10059eb0`) | standing |
| 22 | `ORDER_ROBOT_FOLLOW` | follow (`0x10059dfc`) | following |
| 23 | `ORDER_ROBOT_GETONBOARD` | get on board (`0x10059dc0`) | getting onboard |
| 24 | `ORDER_ROBOT_UPGRADE` | upgrade (`0x10059c20`) | upgrading |

An order carries a target, given as `TARGET_BY_LOGIC_ID` `0x201`, `_BY_PLACE`
`0x202`, `_BY_TYPE` `0x203`, `_NOT_DEFINED` `0x204` or `_BY_NAME` `0x205`. It
also carries an insert mode: to the end 1, to the start 2, or replace 3.

**Orders 14, 16 and 18 have no name anywhere**: `varset.var` does not declare
them, and none of the thirteen game binaries carries an `ORDER_` string at
all. The tasks' own log strings ("Research (%X)", "Migrate %d", "ShowUpgrade")
name the task, not the order. What gives them is *read*:

- **14** comes from the player's research panel (`iron3d.dll:0x100880b0`). For
  a technology whose state in the clan's tree reads 0 and which is not already
  queued (`0x10087c00`), it picks among the clan's research centres the living
  one whose sphere is not running (`0x20c` reads 0) and which has the fewest
  orders. It gives that centre order 14, target `0x204`, with the technology's
  id as the parameter, **to the end** of its queue. The task's `SetTarget`
  (`Behavior.dll:0x1002f470`) logs "Researching" for it.
- **16** comes from `ai.dll` (`0x10011100`), **replacing**: target `0x205` with
  a design's name. `SetTarget` loads that `.dat` ("Cannot Load DAT file", magic
  `0xf0f1`) and researches what the design needs.
- **18** is given by `CreateObjectFromScheme` and `M_Task_Upgrade` alone.

**The status line** is one of 18 strings, `iron3d.dll` 6180–6197 (*measured*):
no order, stopping, shutting down, standing, moving, escaping, following,
getting onboard, attacking, patrolling, searching, transporting, building,
refitting, repairing, capturing, upgrading, unknown. **The order at the head of
the unit's order list picks it** (`iron3d.dll:0x10076f90`, a switch over 0–24,
table `0x100770b8`; *read*). The unit's `IBehaviour` slot 4 gives the list's
length and slot 5 the list; a record's first word is its order.
- An empty list reads "no order".
- Every order the table does not name reads "unknown": 10 to 16, 18, and
  anything above 24.
- So a unit on the menu's Search and capture (order 5) reads "searching"; only
  a script's `ORDER_ROBOT_CAPTURE` reads "capturing".
- The line is `"%s [%s]"`, the unit's name and this string.

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
Attack, Capture building and Refit (7 rows).

**The second table is the wingman menu** (*read*, and *measured*).
`CMD_JAMES_WINGMAN_MENU` (740) — "Activate wingman menu" in `Command.dsc`, bound
to the tilde in `addition.man` and `ui_other.man` — reaches the game command
handler's case `0x100724fe`. In the first-person views, and while the player's
current unit record has its `+0xa2` set, that case calls the wingman selector
(`iron3d.dll:0x1006db40`). The drive switch below sets `+0xa2` when the player
takes a bot below auto-driver level 2. The selector toggles between off
(state 0) and on (state 2), gathering friendly units from a list as it switches
on; a Shift key changes how. While it is on, the first-person panel
(`0x100431a0`) opens the order menu with its flag set (`0x1007a8e0` stores it at
`+4`), and the menu builder (`0x1007aaa0`) then reads the second table for the
selector's units instead of the HQ table. *C01 Mission 3*'s tip, which names
"Stand By" and "Follow Me" as commands to the player's wingman, agrees.

### The wingman menu from first person — *read*, and *measured*

**Opening it** (`iron3d.dll:0x100724fe`, *read*). The tilde's command runs the
selector only when the key event is not a release (type 7), the view is in state
1 or 3, the driven unit's record (game `+0xaec`) has `+0xa2` set, and the view
has a selector (`+0x40`).

**Who can be a wingman** (*read*). The selector counts and reads the driven
unit's target list through `0x10091f20` and `0x10091f30`: the list's `+0x14`,
the records it keeps of the unit's **own clan**
([25-sensors.md](25-sensors.md#the-players-target--read-and-measured)). The
rebuild adds a listed contact there when its owner word is the driven unit's
and a record answers for its id (`0x10091de4`). So a wingman is **a unit of the
player's clan on the driven unit's radar**, in the radar's order. The hero
itself is never listed. With no such unit the tilde does nothing.

**The selector's three states** (`0x1006db40`, *read*). The selector keeps a
state at `+8` and the chosen units as a list of their records' `+0x28` at `+0xc`.
The tilde does this:

| state | tilde | then |
|---|---|---|
| 0, off, Shift up | every wingman is chosen (`0x1006dc40`) and the panel opens | 2 |
| 0, off, Shift held (DIK `0x2a` or `0x36`) | the choice is emptied and the panel opens | 1 |
| 1, picking | with at least one chosen, the order menu follows; with none, the panel closes | 2, or 0 |
| 2, ordering | the choice is emptied and the panel closes | 0 |

The panel opens and closes through the view's `+0x50` object, slot 8 and slot 7
(not followed).

**Keys** (`0x10070db0`, *read*; the case table *measured*). A character handler
switches on the character through a 127-byte index at `0x10071168`. `'1'`–`'9'`
share one case (`0x100710fa`). In view states 1 and 3 that case goes to the
selector:

- **Picking (state 1):** digit *n* toggles wingman *n* in the choice
  (`0x1006dd00` with *n* − 1). A number past the list does nothing.
- **Ordering (state 2):** digit *n* gives row *n* of the menu (`0x1006df80`).
- Either way the key is taken. In state 0 a digit falls through to the rest of
  the handler.

Escape (`0x1b`) goes elsewhere: only the tilde, or an order, closes the menu.

**The list the panel draws** (`0x100431a0`, `0x100432f0`, *read*). Every frame
the panel resets 16 line widgets and fills one per wingman, in list order:
the number *i* + 1, the record, and whether it is chosen (`0x1006df50`). A line
draws the number and two labels made from the record (`0x10077120`, not
followed). A chosen line is drawn highlighted; an unchosen one is grey, or
dimmed by half while picking. The lines stand 19 apart.

**The menu** (*read*). In state 2 the panel places the order menu at (200, 200)
(`0x1007b1a0`) and builds it for the chosen records with the wingman flag
(`0x1007a8e0`). The builder (`0x1007aaa0`) makes one line per row of the second
table, 19 high from y 50 at x 220 (`0x100670d0` with 0xdc, y, 0x17c, y + 19). It
closes the selector if no chosen unit is left (`0x1007ab59`, `0x1007ada1`). A
row is **enabled** when both of these tests pass (`0x1007acb4`–`0x1007ad7e`):

- **The target.** A row that needs one (Attack, Capture building) needs the
  driven unit's current target. The target must not be of the player's clan
  (its record's `+0x24` against game `+0xad0`). A row with pick mode 2 (Capture
  building) needs that target to be a building (object type 3).
- **The capturers.** Search and capture and Capture building need every chosen
  record's `+0x30` to be 1 or 2. Capture building also refuses a building of type
  `0x80000200`.

**No pick mode is entered from first person** (*read*). Row *n* is given at once
(`0x1006df80`): the row must be enabled (`0x1007a8d0`). The target is **the
driven unit's current target**, sorted into a unit or a building by its object
type. After the order the panel closes, the state goes to 0 and the choice is
emptied (`0x1006e2a9`).

**The orders** (*read*). Each row calls the order dispatcher (`0x10079230`) for
the chosen units with a kind of its own, directly (`0x10078b60`, `0x10078820`,
`0x100789c0`). The dispatcher builds a 316-byte order packet per unit
(`0x1007d000`) and hands it to the unit record's `+0x44` object, slot 3, with
insert mode 3, `INSERT_ORDER_REPLACE`. The packet's fields are the ones script
function 15 sets ([15-behaviour.md](15-behaviour.md#what-the-functions-do)):

| offset | field | default |
|---|---|---|
| +0 | the order | 0 |
| +4 | its parameter | 0 |
| +0xc | the target kind | `0x204`, `TARGET_NOT_DEFINED` |
| +0x10 | the target: a logic id, a type mask, or −1 | 0 |
| +0x12c | a place, x, y, z | 0 |
| +0x138 | a float | 1.0 |

| row | key | kind | order | parameter | target (kind, value) | when |
|---|---|---:|---|---:|---|---|
| Standby | 1 | 1 | `STAYGROUND` 0x15 | 0 | not defined | — |
| Follow me | 2 | 10 | `FOLLOW` 0x16 | 50 | by logic id: the driven unit's (record `+0x34`) | — |
| Search and capture | 3 | 3 | `SEARCH` 5 | 0 | by type, `0x8017365e` | every chosen `+0x30` is 1 or 2 |
| Seek and destroy | 4 | 4 | `SEARCH` 5 | 0 | not defined, −1 | — |
| Attack | 5 | 5 | `ATTACK` 3 | 0 | by logic id: the target unit's, else the target building's (`0x10078ce0`) | a target |
| Capture building | 6 | 6 | `SEARCH` 5 | 0 | by logic id: the target building's | a building target, capturers as above |
| Refit | 7 | 8 | `RELOAD` 8 | 0 | not defined | — |

The dispatcher's cases for these kinds are `0x100792ab`, `0x10079506`, `0x10079381`,
`0x100793d7`, `0x10079429`, `0x10079435` and `0x100794aa`, in row order.
Follow me is the only row the HQ executor (`0x1007b740`) has no case for: it is
the wingman's alone.

**The acknowledgement** (*read*, and *measured*). After giving the orders the
dispatcher takes the **last** chosen unit's record `+0x30`, picks a voice
(`0x1008e840`), and plays it through `0x10061ac0` with no speaker, so it queues
([34-progression.md](34-progression.md#messages--read-and-measured)). There are
three sets (registered `0x1005d3ca`–`0x1005d999`):

- `+0x30` of 1 or 2 takes the `_S` voices;
- 4 or 5 takes the `_B` voices;
- anything else takes the plain ones.

Each set has five voices, `VOICE_ACKNOWLEDGE`, `VOICE_AFFIRMATIVE`,
`VOICE_YES_SIR`, `VOICE_OK` and `VOICE_EXECUTE`. The pick is a 16-bit xorshift
that never repeats the last index (`0x1008e690`). All 15 are bound in
`ui/game_resources.cfg` to `voices.lib` (*measured*).

**Build and upgrade rows also need an intact beam** (*read*): the row test
(`0x1007bbb0`) refuses them while `0x10076da0` finds the builder dead, without a
type-30 component, or with that component's value `0x52` at 0 or below.

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
  **the guns still aim and fire through the unit's own fire control**, which the
  task sets to pick its own targets ([The fire control](#the-fire-control--read)).
  **It moves nothing.** Its start and every takt clear the walker
  (`0x10031bd0`, `0x10031ca0` → `0x1003c540`), emptying its three queues and
  giving the wizard no new points
  ([24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)).
- **Route — go.** It walks to the place at `Go_SpeedPercent` 1.0 of the unit's
  speed and ends when it stops there ("We are staying... task over",
  `0x1002b670`), or fails if the place is unreachable. It acts only when the
  walker is idle. Within 30 of the place it is over; 1.5 counts as arrival
  when the order names an object. Otherwise it calls `SetTarget` again.
  "Unreachable" is that call's refusal: out of the map, no areal map, or no
  global path
  ([24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)).
  The go task will not be
  interrupted by reasons 0–2 or 5, only 3 and 4 (`0x1002b390`).
  **Route does not chain waypoints** (*read*, as a search). The dispatcher's
  case for Route (`iron3d.dll:0x10079230`) first empties the unit record's list
  of points (`+0xc0`, 8-byte x/y records, `0x1007c0a0`), then gives one `GO` to
  the picked place, replacing. The unit record's constructor (`0x10074af0`)
  builds that list. Another path (`0x10079f40`) gives a `PATROL` of
  radius 300, replacing, on one of two units the record names or on the list's
  *first* point, and empties the list. Nothing fills the list: every
  store to an `+0xc4` field in `iron3d.dll`, the list's end, is one of these
  emptyings or belongs to a panel layout. The go task holds one place, and a
  later `GO` replaces it.
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
  compiled defaults by target, read where `SetTarget` (`0x1002d520`) and the
  start (`0x1002d7c0`) take them:

  | guarding | radius | new point every | speed |
  |---|---:|---|---:|
  | a unit | 60 | 5 + up to 10 s | 1.0 |
  | a building | 30 | 60 + up to 60 s | 80 |
  | a place | 60 | 20 + up to 10 s | 0.8 |

  **The building's 80 changes nothing a 1.0 would not** (*read*). A walk is
  asked for at the unit's speed (`+0x5fc`) × the task's figure, and
  `MWalker::SetTarget` (`0x1003bad0`) holds the request
  ([How a walk's speed is held](#how-a-walks-speed-is-held--read)):
  - no more than the unit's speed × `Movement_SpeedPercent` (1) ×
    `Speed_MaximumFactor`, the difficulty profile's;
  - no more than `Movement_MaxSpeed`, 600;
  - no less than the unit's second figure × `Movement_MinSpeedPercent` (1);
  - no less than 2.

  The unit's takt copies the same control record into both `+0x5fc` and the
  `+0x614` the cap reads (`0x1001bbe0`). So 80 × the speed is cut to the unit's
  full speed × `Speed_MaximumFactor`, exactly what a unit guard's 1.0 gets.
  Whether 80 was meant as 0.8 cannot be told from the code; in play it is full
  speed.

  **`Patrol_Attack_Range` (400) is never read** (*read*, as a search). The
  constants block is reached only through two getters (`0x10014650`,
  `0x10014660`, 83 calls), and following each result finds reads of 34 of its
  fields, every other patrol constant among them, but none of `+0x30`.
  `Build_BuildDistance` (`+0x3c`) is unread the same way. The range at which
  a guard fights is the fire control's 500
  ([The fire control](#the-fire-control--read)). The 400 a call for help uses
  is a separate constant ([Between orders](#between-orders--read)).

  **A small unit told to guard a building of another clan captures it first**
  (`0x1002da7a`): if the unit is of size class 2 or less, the patrol queues,
  behind itself, a search on that building (a capture), an escape from it and a
  patrol of it again, and ends. What the unit's `IBehaviour` slot `0x10` test
  it also makes is *unknown*.
- **Refit — reload.** `M_Task_Reload` walks to a ground-level dock and waits
  until life, charge and ammunition are at 98%
  ([27-ownership.md](27-ownership.md)). `ORDER_ROBOT_REPARE` builds the same
  task. **With no dock it fails at once** (`0x1002e800`). The start asks
  `0x10023b60` (not read) for a building. When the answer is −1, a walking unit
  logs "No Where to reX..." and the task fails. A flying unit first tries a
  second pick from where it is (task slot 15). If that is also −1, it logs
  "No Where to reX(for flyeing)..." and fails.
- **Follow me — follow.** It keeps near the logic id it is given, within a
  radius taken from the order's parameter (`0x1002ad80`, "FollowRadius").
  - **The radius is 20 unless the parameter lies strictly between 20 and 30**
    (`0x1002adab`). So the menu's 50 gives 20 — *read*.
  - **Each takt** (`0x1002ae20`) finds the leader through the system areal map.
    The task fails once the leader is gone or on another clan. It measures when
    the walker is idle or its check timer (`+0x60`) fires.
  - **It picks a new place** when the leader is more than radius + 20 away,
    or more than radius + 10 above or below (`0x1002aed7`). It does the same
    when close but `MBehaviour` `+0x140` is not −1 (a field not identified).
  - **Picking is gated** by a second timer (`+0x68`). It tries up to 77 random
    spots in the square ±radius about the leader, at the leader's height + 5
    (`0x1002b067`). It hands the first that `SetTarget` accepts over at the
    unit's full speed (`+0x5fc`, no percentage), the same near or far.
  - **When the leader stops**, the follower walks out its last trajectory to
    that spot and stays there. Neither timer's period was read.
- **Attack.** `M_Task_Attack` on a logic id. It cancels when the unit has no
  weapon, takes the nearest target when given none (`0x10026fd0`), and ends
  when the target is dead ("mission accomplished", `0x10027250`). In between it
  picks a point 50–100 short of the target and up to 80 to either side, walks
  there, and picks again on a timer. Within 200 it walks at 0.7–1.0 of its
  speed, re-picking every 8–16 s; beyond 200 it closes at full speed, re-picking
  every 8–16 s as well. [The attack, tick by tick](#the-attack-tick-by-tick--read)
  has the details, and which of its named constants are dead.
- **Search minerals — search by type `0x10001000`.** The minerals mode picks
  **the nearest mineral lode not yet found** (a lode's `+0xc` is 0,
  `0x1003075d`), from the whole map's list, not from what the clan has seen. It
  walks there; within 10 m it marks the lode found for everyone, logs "Resource
  Found" (`0x10030535`) and ends. **The lodes come from the mission file**
  ([Mineral lodes](#mineral-lodes--read-and-measured)).
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
- an escape from a named building is held while that building's variable
  `0x205` reads 1 or `0x309`, or its `0x20c` reads 1. **`0x205` is the
  construction sphere's phase code** (*read*): the variable getter returns
  `MBehaviour+0x9fc` for it (`0x1000a784`), which the sphere task writes at its
  start and at each phase change (`0x1003130d`, `0x10031711`). 1 is the sign
  and `0x309` the upgrade's clear-out ([32-builder.md](32-builder.md)), and
  `0x20c` reads 1 while the sphere task runs. So an escape from a building
  keeps going while that building's sphere clears its area.

**Nothing interrupts it**: its priority is 0 for every reason (`0x1002b9f0`),
so an escaping unit neither engages nor refits on the way. When it ends, the
task beneath it on the stack, if any, resumes.

**Who gives it** — five places, all in `Behavior.dll`:

| when | escape | inserted | read at |
|---|---|---|---|
| a unit **stands on a building** with **no order or a stop** — not a ruin, not a destroyed building, and not attached to a damaged node of the building (below) | no target | replacing | unit takt `0x10005408` |
| the behaviour's flag `0x10` is switched back on while the unit stands on a building: **the player lets go of the bot**, or raises its auto-driver level (below) | from that building | first | mode setter `0x10006f48` |
| a factory has just made the unit | no target | replacing | `0x1002aa6e`, "Adding Robot to game..." |
| a small unit guarding another clan's building, after capturing it | from that building | queued | patrol `0x1002da7a` |
| a building begins its construction sphere, for every unit within its radius + 15 not already escaping or upgrading | from that building | first | ShowUpgrade `0x10031571` |

So **a capture is followed by an escape when the capture task ends** —
Capture building, a script's capture by logic id, or the guard sequence — the
unit takt seeing an idle unit on the building. The menu's Search and capture
does not end at a capture, so that path does not run.

**The node test in the unit takt** (`0x1000537d`, *read*). The unit's parent
object is the building it stands on. `IGameObject` slot 3 returns the parent
and slot 16 the joint, the parent's node the unit is attached to (`+8` and
`+0xc`, as `CGameObject::SetParent` stores them: `AniMesh.dll:0x10017510`,
`0x10017570`, `0x10017ea0`). The building's `IAnimation`, which it hands on to
its `AniMesh` agent (`Terrain.dll:0x10057c20` passes interface `0xb` on), gives
in slot 16 that node's value, its `+0x1c` (`AniMesh.dll:0x100057f0`). A unit on
a node with a non-zero value is not given the escape. The same value makes a
building's place tick skip a hall-way place on that node (`Behavior.dll:0x10018b7e`).
The loader zeroes the value, and `IAnimation` slot 17 (`0x10005810`) sets it.
**Its one caller found is the node damage stage** (`Control.dll:0x100118c4`, in
`0x10011220`). As a node's life falls, the control system gives its animation
node the node's damage stage, held below the model's stage count. So **the value
is how damaged the node is**, 0 while it is whole. Two things follow:
- a unit attached to a damaged node of a building is not given the escape;
- a building's hall-way place on a damaged node is out of use.

The search that found it listed the slots called through each module's stored
`IAnimation` (`Control.dll`'s `+0x20` among them). A search for interface `0xb`
followed by a call at `+0x44` had found only the two slot-16 readers.

**Flag `0x10` is the player's hand** (*read*). Each unit's agent carries a
`Wizard.dll` object, and only it was found calling the mode setter
(`IBehaviour` slot 10) with a whole-behaviour argument. A scan of every call at
`+0x28` with −1 among its pushes, in all sixteen modules, finds
`Wizard.dll:0x10003904`. The other twelve hits pass −1 last and have a
different shape.
- **Its mode.** Message `(6, 7, p)` to the unit's agent
  (`AniMesh.dll:0x1000147c`) passes `p` to the wizard (`0x10001af0`): 0 is AI
  mode, 1 the player's mode, 2 off.
- **Its per-bit overrides** (slot 9, `0x10002070`): 0 follows the mode, 1 forces
  a bit on, anything else forces it off.
- **The flags** (`0x10003890`): bit 2, the behaviour's flag `0x10`, is on in AI
  mode unless forced off, and on in the player's mode only when forced on.
  Bits 4 and 8, flags `0x20` and `0x40`, work the same way from a second
  override. Mode 2 clears everything.
- **Taking a bot** (`iron3d.dll:0x10074ff0`, argument 1). The overrides follow
  the bot's **auto-driver level** (`+0x9c`), which `CMD_JAMES_AUTO_DRIVER` (744)
  steps 0 → 1 → 2 → 0 (`0x10075fc0`):

  | level | flag `0x10` | flags `0x20`, `0x40` | mode sent |
  |---:|---|---|---|
  | 0 | forced off | forced off | 1, the player's |
  | 1 | forced on | forced off | 1 |
  | 2 | forced on | forced on | 0, AI |

  The hero has every override forced off.
- **Letting go** (argument 0). It sends mode 0 and forces every override on,
  except on the hero, whose overrides all read 2, off.
- So **letting go of a bot that stands on a building turns flag `0x10` back on,
  and the mode setter gives the bot its escape**. Raising its auto-driver level
  from 0 to 1 while it stands there does the same.

*Against what the game looked like:* in play an escape follows every capture,
and follows leaving a driven bot inside a building or on its grounds. The code
now gives the second through the wizard and the mode setter. It gives the first
only where the capture task ends: after Capture building or a script's capture,
not after the menu's Search and capture, which replans at `0x100304b9`. What
gives a Search and capture unit its escape, if anything does, is *unknown*.

## Where a search looks — *read*, and *measured*

**What the clan knows.** Enemy and building candidates come from the clan's
**areal map**, not from the unit's own radar list (`0x10015260`,
`0x10015310`). That map keeps, for each areal, a snapshot of the units and
buildings in it: logic id, Type and clan. Two things refresh a snapshot from
the game's system map, and each stamps it with the time
(`ArealMap.dll:0x10001840`):
- **The clan's own tick refreshes every areal** (*read*). The clan's SuperAI
  creates the map (`ai.dll:0x10005cb0`, `CreateArealMap`) and sends it message 1
  on each of its takts (`0x10001780`). On message 1 the map
  (`ArealMap.dll:0x10001370`) runs a timer and, whenever it fires, refreshes all
  its areals in turn. The constructor (`0x10001010`) sets the timer's words to
  46 and 46 and its next time to 0 (`0x1002d570`: next = now + 46 × 64 ms + a
  random byte × 46 × 64 / 256). So the first tick refreshes everything, and
  then every **2.9 to 5.9 s**.
- Every 2.0–4.9 s each unit's radar module reports its position and radar range
  ([25-sensors.md](25-sensors.md)), and the map refreshes the areals within that
  range at once (`0x10001ec0`, `0x10001dd0`).

So **a clan's map holds the whole map from its first tick**, at most about
5.9 s old, and sooner near its radars. The collectors never read the stamp:
a search trusts any snapshot until the next refresh replaces it. An earlier
version of this page said a search sees only what the clan's radars have swept.
That reading missed the timed refresh.

**The plan runs in this order** (`0x100306f0`), and the first that finds
something is taken:

| mode | candidates | pick |
|---|---|---|
| minerals | the map's mineral lodes not yet found | nearest |
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

## Mineral lodes — *read*, and *measured*

**The lodes are the mission's own records**, the trailer's list that
[04-missions.md](04-missions.md) reads as per-clan viewpoints (*read*).
- **Loading.** `MisLoad.dll`'s reader (`0x10001b10`) reads the objects and the
  trailer. When the word before the object count (10 in every mission) is 5 or
  more, the trailer ends with a count and that many 28-byte records. The reader
  puts them at the mission's `+0x10`/`+0x14`, starting each at type
  `0x10001000`.
  `IMission` slots 12 and 13 (`0x10001520`, `0x10001530`) hand them out.
- **Into the map.** `iron3d.dll:0x10081880` copies them into the system areal
  map's list with `SetMineralLode` (`ArealMap.dll:0x10021db0`); a saved game
  restores the same list from its file (`0x10081750`).

A lode as the game keeps it, 24 bytes:

| offset | from the file record | what |
|---:|---|---|
| `+0x00`, `+0x04` | x, y | where it lies |
| `+0x08` | — (0) | the file's z is not copied |
| `+0x0c` | the first word | **found**: a search skips a lode whose word is set |
| `+0x10` | — (`0x10001000`) | the file's type word is not copied |
| `+0x14` | the third word, a float | **amount** |

The file's fourth word, a float, is not copied either.

What uses them (*read*):
- **Search minerals** walks to the nearest lode not found and, within 10, marks
  it found.
- **A mine** (`M_Task_Mine`'s `SetTarget`, `Behavior.dll:0x1002cd10`) adds up
  the amounts of every lode within 250 of it across the ground (`0x10020f70`, x and y only) as its "ToMine" (`MBehaviour+0x9e8`),
  and marks each found.

*Measured* over the 29 missions:
- **28 lodes in 13 missions.** 16 missions have none; the rest carry 1 to 4.
- **Every one of the 15 placed mines lies within 250 of a lode.** Control: 6 of
  the 95 other buildings in those missions do, generators mostly.
- **The amounts are powers of ten from 10⁴ to 10²⁰, or one less** (999,999
  once, 9,999,999 twice). The type word is `0x10001000` on 25; the other 3
  carry 0 and a fourth word of 100.
- **17 start found.** The other 11 are open to Search minerals until a mine
  within 250 marks them.

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

**Nothing reads these flags** (*read*, as a search with controls):
- **By offset.** The displacements `+0x834`–`+0x868` appear on no base register
  but the stack in `Behavior.dll`, `iron3d.dll` and `ai.dll`, nor does the `+0x820`
  block base outside its binder (`0x1000a2a0`) and getter (`0x10014680`). All 17
  calls of the getter read the ore and power fields at `+0x54`–`+0x68`
  ([26-damage.md](26-damage.md)).
- **By id.** The behaviour's variable getter (`0x1000a490`) returns no address
  in the block for any id.
- **By name.** No file in the install but `Behavior.dll`, which binds them, and
  `behpsp.res`, which sets them, names a `Task_*` flag (*measured*).

So a builder's 0 for Attack describes the design, and nothing enforces it. The
four medium builders that carry a gun engage like anyone else: the engagement
inserts an attack whatever the profile says, and the attack task cancels only
for want of a weapon (*derived*). What does gate packages, as read:

- the menu masks (transports alone get Transport minerals; builders alone get
  building);
- the size rule on the two capture packages;
- the attack task's "weapon absent" cancel.

## Between orders — *read*

**The six interrupt reasons** (*read*). A self-given task is inserted by
`0x100179c0` with a reason, and `0x10034510` builds its task from the reason:

| reason | the task | who asks |
|---:|---|---|
| 0 | `M_Task_Attack` on the best-scoring hostile contact | the engagement below (`0x10017e70`) |
| 1 | `M_Task_Attack` on a logic id | **retaliation**: a unit hit by an explosion attacks the object it came from (below) |
| 2 | `M_Task_Attack` on a logic id | nothing found |
| 3 | `M_Task_Reload`, a trip to a dock | refitting (`0x10017d50`) |
| 4 | none: the builder returns nothing | nothing found |
| 5 | `M_Task_Attack` on a **place** | **a call for help** (below) |

So a task's priority for reasons 0–2 and 5 is its willingness to be pulled into
a fight, for 3 into a refit. The only callers of `0x100179c0` are the four
named, so reasons 2 and 4 are tested by the priorities but never asked.

- **Retaliation and a call for help.** `MBehaviour::SendMsg` passes messages
  `0x19` and `0x1a` to slot 67 (`0x10005b01` → `0x100064b0`), which logs
  "Explode at [..] received from". Unless the unit is a building, it finds the
  object the explosion came from and asks for a reason-1 attack on its logic id
  (`0x10018060`). It then calls for help (`0x1000c260`): every warrior
  (`0x1008000`) of its own clan's areal-map snapshot within **400** of it is
  sent message `0x12d` with the victim's logic id. That message reaches slot 68
  (`0x10005aee` → `0x100065a0`), a reason-5 attack on the victim's place
  (`0x10018030`). The 400 is a constant of its own (`Behavior.dll:0x10059630`),
  not `Patrol_Attack_Range`.
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
  for minerals, building or transporting.** Engaging here means taking up an
  attack task. Picking a target to shoot at while the task goes on is the fire
  control's, below.
- **Refitting.** A unit whose life or charge is below half, or whose guns are
  mostly dry, sends itself to a dock as a reason-3 task
  ([27-ownership.md](27-ownership.md)). The default priority lets reason 3
  through only if a dock is reachable (`0x100018a0`).
- **With no order.** An animal's default order is 15, migrate; a mine's is to
  mine (`0x100088f0`). A unit placed inside a building gets a patrol inside it
  ("Give default patrol inside building order", `0x1000ac4c`). **A unit left
  idle or stopped on a building escapes** from it ([The escape](#the-escape--read)).
  For other units no default order was found.

### Migrate: an animal's pasture — *read*, and *measured*

Order 15's task (vtable `0x10059aac`) keeps an animal on its clan's pastures,
the clan's zones as the mission file gives them: a centre, an inner radius and
an outer one ([04-missions.md](04-missions.md)), which `IMission` slot 8 hands
the areal map as the clan's *migration areals* (slot 35, `0x100220e0`).

- **One pasture a clan.** The system areal map keeps a current pasture per
  clan (slot 47, `ArealMap.dll:0x10022230`). It answers the same one until the
  clan's timer runs out, then picks `rand() % count` — the same one again,
  possibly — and restarts the timer. The timer's words are 937 and 1875, ×64
  ms (`0x1002ab37`, `0x1002ab3d`): **60 s, plus up to 120 s**.
- **The start** (`Behavior.dll:0x1002c9d0`) asks for the clan's pasture into
  `+0x60`, starts its own timer at 60 + up to 120 s (`+0x58`), walks to a
  point, and starts a second timer at 5 + up to 10 s (`+0x64`). Both use the
  attack task's timer helper, (fixed, random) in seconds (`0x1004c4c0`).
- **The point** (`0x1002cba0`): the pasture's centre plus (*u* × inner,
  *v* × inner), where *u* and *v* are each `rand()` over 32767 held to at least
  0.2 (`0x10059770`, `0x100597e8`), and the centre's own height. **Both
  offsets are positive**, so the point always lies in the square to the +x, +y
  side of the centre, 0.2 to 1 inner radius along each axis. It tries up to 50
  points until the walker takes one.
- **Each tick** (`0x1002ca60`) asks the fire control for mode 0, no target of
  its own. When the long timer has run out it asks for the pasture again and
  walks to a new point. Otherwise, when the walker is idle and the short timer
  has run out, it walks to a new point; while the walker is busy it restarts
  the short timer.
- **What it lets through.** A migrating animal is the one animal that engages
  at all (`0x10017a1e`, [above](#between-orders--read)), and the task decides
  what:
  - **The score** (slot 13, `0x1002c910`): a hostile contact within the inner
    radius of the pasture's centre scores 1 / (*d* + 10), *d* its distance
    from the centre (`0x10059144`); anything else scores 0. The engagement
    takes only a contact scoring above 0 (`0x10017f75`).
  - **The priority** (slot 12, `0x1002c640`), by reason
    (`0x1002c8f8`): for an engagement, reasons 0, 2 and 5, it is 1 when the
    animal itself is within the outer radius of the centre and the contact
    within it too, and 0 otherwise. For **retaliation**, reason 1, it is always
    1. Reasons 3 and 4 get the default. The task also fills in two figures for
    the attack it lets through — 20, 10, 25 or 35 by where the two stand, and
    a circle about the centre of the outer radius plus 20, 80 or 100 — which
    `0x100179c0` merges into the new task (not followed further).
- **So** (*derived*): a grazing animal fires at nothing. It attacks a hostile
  unit that comes within the inner radius of its clan's current pasture while
  it is itself inside the outer radius, and anything that hurts it. The attack
  is then the animal's version of [the attack](#the-attack-tick-by-tick--read).

*Measured:* only nature clans carry zones — 12 of the 15, each with animals
of its own. One more places animals with no zone at all, where slot 47 has no
pasture to give (what the task does then is not read). Mission 02's two
pastures are in [34-progression.md](34-progression.md#mission-02-the-constructor-end-to-end--derived).

## The fire control — *read*

The object at `MBehaviour+0x35c` is the unit's **fire control** (constructor
`Behavior.dll:0x10023e80`, takt `0x10023ff0`). Its mode, `+0x28`, picks the
target the guns are pointed at (`0x100240a6`):

| mode | target |
|---:|---|
| 0 | none |
| 1 | the one it was given, by logic id (`+0x2c`) |
| 2 | the hostile contact nearest the unit, within 500 (`0x100254b0`) |
| 3, 4 | the two weighted pickers of [25-sensors.md](25-sensors.md) |

It starts in mode 2. **A task asks for a mode when it starts**, and some again
in their takt, through `0x10023f30` with {mode, logic id, 0.5}. The request is
kept unless `+0x5c` or `+0x60` is set (by `0x10023fd0` and `0x10025a00`, not
traced further). It also refreshes every gun's fire frequency with the difficulty
profile's `Fire_FreqFactor` (`0x1001b650`). What the 0.5 does is not read.

| mode | asked by |
|---:|---|
| 1 | attack (start, takt and manoeuvre) |
| 2 | stop, go, patrol, search, transport, random go, stay ground, follow, leave |
| 0 | shutdown, migrate |

So **the search task's start asks for mode 2** like every task that moves.
The fire control, run from the behaviour's takt with the unit's position
(`0x100050fc`), then aims at the nearest hostile contact within 500 whatever the
task's interrupt priority says. Standby asks for mode 2 as well, and a shut-down
unit or a migrating animal asks for none. The takt does nothing while the
behaviour's variable `0x208` is set. It sends the guns their fight state in the
same function (`0x10024f99`), once a gun's score clears the bar
([29-weapons.md](29-weapons.md#how-the-ai-fires--read)).

## The attack, tick by tick — *read*

`M_Task_Attack` (vtable `0x10059d78`) holds its target's logic id at `+0x58`
and the id it last handed the fire control at `+0x60`. It keeps two
randomised timers: `+0x68` while **fighting** and `+0x70` while **nearing**. The
flag `+0x64` says which phase it is in. Movement is the walker's
(`MWalker::SetTarget`, `0x1003bad0`, and the walk helper `0x10001960`). Here it
is only "walk to P at speed S".

**Start** (`0x10026fd0`):

- **No weapon:** it cancels ("CANCEL IRQ: Task_Attack: weapon absent") when the
  unit's device interface `0x204` has no property 5, or has it at 0. The test is
  skipped when the task's `+0x54` is 1 (not traced).
- **Timers**, in seconds, as (fixed, random):

  | unit | fighting `+0x68` | nearing `+0x70` |
  |---|---|---|
  | any, with `Behavior.ini`'s `DeterminMode` set | 10, 0 | 10, 0 |
  | an animal (type `0x20000000`) | 5, 5 | `Attack_Nearing_ChangeTrajectory` Min, Random: 4, 4 |
  | any other | 2 × `Attack_Fighting_…`: 8, 8 | 2 × `Attack_Nearing_…`: 8, 8 |

- **No target:** it takes the nearest (`0x10001120`) into `+0x58` and `+0x60`;
  still none, and the task cancels.

**Each tick** (`0x10027250`):

1. **Target gone:** if the target's position cannot be had, or is exactly
   (0, 0, 0), it logs "Target .. is dead... mission accomplished" and ends.
2. **Fire control:** while nearing it asks for {2, −1, 0.5}, the nearest
   hostile contact. Otherwise it asks for {1, `+0x60`, 0.5}, its own target
   ([The fire control](#the-fire-control--read)).
3. **Unstuck:** this runs only while fighting and not following the target
   (`+0x5c` clear). It needs `+0x78`, the time fighting began, to be set, and
   two times more than 10 s past it: now, and a time the behaviour keeps at
   `+0xb4` (`0x10027346`, not traced). It then tries up to 25 random points
   within ±27.5 of the target in x and y, at `0x10015400` × `Go_SpeedPercent`,
   restarts both timers and sets `+0x78` to now.
4. **Re-pick:** the task's slot 17 asks whether its timer has run out. If it
   has, slot 15, `MakeGoCommand`, re-picks. For a unit target it also
   re-picks whenever the walker's `0x1003ddc0` test holds while its
   `0x1003ddb0` answer is below 2. For a building target (id bit 31) it
   re-picks when the walker is idle (`0x1003dd80`) and the unit is within 5 of
   the point at behaviour `+0x170`. Those walker calls are the walker
   research's to name.

**Making a move** (`MakeGoCommand`, `0x10027580`):

- **A building target:** it asks the target's `IMesh2` for its sphere of
  radius *r* ("Behaviour panic: Target object does not support IMesh2"
  otherwise). It tries up to 77 random points between *r* − 20 and *r* − 5
  from the centre, at the unit's speed (`+0x5fc`). On the first it may walk
  to, it clears `+0x64` and `+0x78` and restarts timer `+0x70`
  (`0x10027783`). After 77 refusals the move fails.
- **A unit target:** the fight band comes from `0x1001cc50` (0.05 and 0.1 of a
  range it takes from the gun records). **Unless the band lies within 50..100,
  it becomes 50..100** (`0x100277be`, `0x100277cf`). The low end could be 50
  only with a range of exactly 1,000, so **the band is 50..100 whatever the
  guns** (*derived*).
- **The point** (`0x10027f80`) is the target's position, less the unit-to-target
  direction × (50 + up to 50), plus the perpendicular × (−1..1) ×
  `Attack_LeftRightRange` (80). An animal goes straight at the target on 30%
  of picks (`0x100281e9`). While the task follows its target (`+0x5c`), the
  point is the target's position itself.
- **The speed**, by the unit's distance to that point against
  `Attack_MaxFireDistance` (200, `0x10027897`):

  | distance | phase | speed | timer restarted |
  |---|---|---|---|
  | under 200 | fighting | the unit's speed × (`Attack_MinAttackSpeedPercent` 0.7 + up to `Attack_DelAttackSpeedPercent` 0.3) | `+0x68`; entering it stamps `+0x78` |
  | 200 or more | nearing | `0x10015400` × `Go_SpeedPercent` (1.0) | `+0x70` |

  The walker then caps either speed at the unit's top speed ×
  `Speed_MaximumFactor` ([How a walk's speed is held](#how-a-walks-speed-is-held--read)).
- **The walk.** `0x10028330` (not read) may give an object id. If it does, the
  task sets `+0x5c` and asks the walker to follow that object at the speed,
  with −1 and 5 (`0x10027a1f`). Then, unless the unit is an animal, it asks
  the fire control for mode 1 on the target. If the walker refuses, the task
  tries a random corner of the object's contour (variable `0x203`), and
  cancels ("contour unreacheble") when that is refused too. If there is no
  object id, the task clears `+0x5c`, asks for mode 1, and walks to the point
  (`0x10001960`). If that walk is refused, it lets both timers run out so the
  next tick re-picks (`0x10027ee1`).

**Named constants the attack never uses** (*derived* from the reads above):

- `Attack_MinFightDistance` (30) and `Attack_DelFightDistance` (20) are read
  (`0x1002760e`, `0x10027620`), but both branches overwrite the band before it
  is used.
- `Attack_MinNearingSpeedPercent` (0.8) and `Attack_DelNearingSpeedPercent`
  (0.2) are not read by any of the attack task's functions above. Nearing is at
  `Go_SpeedPercent`.

So **an AI attacker never holds still**. Within 200 it circles a point 50–100
short of its target and up to 80 aside, at 70–100% speed, picking a fresh
point every 8–16 s. The fire control shoots whenever the aim settles
([29-weapons.md](29-weapons.md#how-the-ai-fires--read)).

## What Mission 01 gives the AI — *measured*

- **The wingmen the hero can take.** The neutral clan `Ntrl` owns two bots:
  - `tut1_mf1.dat`, on `r_m_02`, a flying chassis of size class 3. It is
    **too big to capture**: Search and capture refuses it (`0x100301a9`).
  - `helic.dat`, on `r_t_02`, flying, size class 1. It **may capture**.

  The hostile `tut1_e1.dat` is `r_t_01`, walking, class 1.
- **Nothing to capture or dock at.** The mission's only buildings are the
  player's two halves of `m_bridge.dat`, and they have no pod and no dock.
  A capture skips bridges anyway (`0x10030859`). *Derived* from that:
  - Search and capture finds nothing and roams.
  - Capture building has no target.
  - A Refit fails at its start with "No Where to reX(for flyeing)...", since
    both wingmen fly.
  - Seek and destroy can pick only `tut1_e1`, the one unit of a clan hostile
    to the player (the other units are the neutral `Trgt` dummies), once the
    clan's areal map holds it within 3,000.

## How a walk's speed is held — *read*

A task asks its walker for a speed, the unit's speed (`+0x5fc`) × the task's
figure. `MWalker::SetTarget` (`Behavior.dll:0x1003bad0`, at `0x1003be4f`) then
holds it:

```
speed = min(speed, top × Movement_SpeedPercent × Speed_MaximumFactor)
speed = min(speed, Movement_MaxSpeed)                     # 600
speed = max(speed, low × Movement_MinSpeedPercent)
speed = max(speed, 2)
```

`top` is `+0x614` and `low` is `+0x618`. The unit takt copies one control
record (IControl query `0x12`) into both `+0x5fc`..`+0x610` and
`+0x614`..`+0x628` (`0x1001bbe0`), so `top` is the same speed the task
multiplied: record `+0x1c`, and `low` is its `+0x10`. `Movement_SpeedPercent`
and `Movement_MinSpeedPercent` are 1. `Speed_MaximumFactor` is the difficulty
profile's: 0.7 in `diff_slow`, 1 in the other four
([24-motion.md](24-motion.md)). **A figure above 1 is therefore the same as 1**, and
every task's walk is capped at the unit's speed × `Speed_MaximumFactor`.

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

- ~~Whether anything enforces a profile's task flags.~~ Nothing reads them
  ([Who may run which](#who-may-run-which--measured-and-read)).
- ~~What sets the mineral lodes.~~ The mission file's trailer records
  ([Mineral lodes](#mineral-lodes--read-and-measured)).
- ~~Whether a clan's areal map knows anything before its radars sweep, and how
  old a snapshot a search trusts.~~ It refreshes every areal on its first tick
  and every 2.9–5.9 s after; the stamp is never read
  ([Where a search looks](#where-a-search-looks--read-and-measured)).
- ~~What the search task's start does to the `+0x35c` controller.~~ It is the
  fire control; mode 2 aims at the nearest hostile contact within 500
  ([The fire control](#the-fire-control--read)).
- ~~What interrupt reasons 1, 2, 4 and 5 are.~~ 1 retaliation, 5 a call for
  help, 2 an attack and 4 nothing, neither of the last two ever asked
  ([Between orders](#between-orders--read)).
- ~~What orders 14, 16 and 18 are called.~~ Nothing in the install names them:
  `varset.var` skips them and no binary carries an `ORDER_` string. What gives
  them is read: research by id, research of a design by name, the construction
  sphere ([The orders](#the-orders--measured)).
- ~~Whether Route can chain waypoints.~~ It cannot: every `GO` the menus give
  replaces, and the unit record's point list is only ever emptied.
- ~~What reads `Patrol_Attack_Range`, and the building patrol speed of 80.~~
  Nothing reads the range; 80 is cut to full speed by the walker.
- ~~Which string the status line shows for which task.~~ By the head order.
- ~~Whether a wingman menu is what the second table is.~~ It is.
- ~~What the building's `IAnimation` test in the unit takt admits.~~ A unit on
  a damaged node of the building: the node's value is its damage stage
  ([The escape](#the-escape--read)). Whether other code than
  `Control.dll:0x100118c4` sets the value was not searched beyond the stored
  `IAnimation` pointers.
- ~~Who calls the behaviour's mode setter.~~ `Wizard.dll`, from the player's
  taking and letting go of a bot ([The escape](#the-escape--read)).
- ~~What a building's variable `0x205` holds.~~ The sphere's phase code.
- ~~What the attack task does each tick, and which of its constants it
  reads.~~ [The attack, tick by tick](#the-attack-tick-by-tick--read). The
  fight distance and nearing speed constants are dead.
- ~~What a Refit does with no dock.~~ It fails at its start.
- What `0x10028330` returns to `MakeGoCommand` and the goal point: an object
  id the walker follows. Which object it is (the target, or something between)
  was not read.
- The attack's other unread pieces:
  - the time at behaviour `+0xb4` that the unstuck test compares;
  - the task's `+0x54`, which skips the weapon test;
  - what `0x10023b60` picks as a Refit's dock, and from which clan's
    buildings.
- What a fire-control request's third value, 0.5, does, and what sets `+0x5c`
  and `+0x60` to lock a unit's fire mode.
- Who sends `MBehaviour` messages `0x19` and `0x1a`, the explosions that start a
  retaliation.
- What a unit record's `+0x30` is. The wingman menu lets only 1 or 2 capture, the
  same records speak `_S` voices, 4 and 5 speak `_B`, and boarding wants 4
  ([27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)).
- The two labels a wingman line draws beside its number (`iron3d.dll:0x10077120`),
  and what the view's `+0x50` object does on slots 7 and 8 as the panel opens and
  closes.
- Whether the wingman menu's lines take mouse clicks, as the HQ menu's do; string
  3044, "Use the mouse to click on the item", is not tied to either.
- Whether a digit that picks a wingman or an order also reaches `World3D.dll`'s
  input table, which toggles the hero's guns on the same keys. The character
  handler takes the key, but the table reads DirectInput separately.
- What the order packet's `+0x110`, `+0x120`, `+0x124` and `+0x138` (1.0) mean,
  and how the unit record's `+0x44` object inserts an order into the behaviour's
  list; the call reaches `Behavior.dll` through an interface not traced here.
