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

- **14** comes from the player's research panel (`iron3d.dll:0x100880b0`,
  [41-commander.md](41-commander.md#the-research-page-4--read-and-seen)). For
  a technology whose researched bit in the clan's tree reads 0 and which is not already
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

As it opens and closes, the selector calls the view's `+0x50` object, slot 8
and slot 7 (not followed). **These do not show or hide the lines**: the panel
draws a line for every wingman whatever the state (below).

**Keys** (`0x10070db0`, *read*; the case table *measured*). The key-down handler
([40-command-mode.md](40-command-mode.md#input--read-and-measured)) switches on the virtual-key code through a 127-byte index at `0x10071168`. `'1'`–`'9'`
share one case (`0x100710fa`). In view states 1 and 3 that case goes to the
selector:

- **Picking (state 1):** digit *n* toggles wingman *n* in the choice
  (`0x1006dd00` with *n* − 1). A number past the list does nothing.
- **Ordering (state 2):** digit *n* gives row *n* of the menu (`0x1006df80`).
- Either way the key is taken. In state 0 a digit falls through to the rest of
  the handler.

Escape (`0x1b`) closes the menu too: its case sends 740 while the menu is up
and the top mode is not 7 (`0x10070f3f`–`0x10070f86`), as the tilde or an
order does. An earlier reading here had only the tilde and an order close it.

**The list the panel draws** (`0x100431a0`, `0x100432f0`, *read*). The panel is
one of the cockpit's widgets and draws every frame unless the top `CState` mode
is 6, a building's screen.

- Each frame it resets 16 line widgets. It fills one per wingman in list order
  with the number *i* + 1, the record, and whether that wingman is chosen
  (`0x1009d970`, `0x1006df50`). It also sets each line's `+0x21` to whether the
  selector is picking (slot 1, `0x1009b700`).
- The count is the driven unit's wingmen now (`0x1006ddb0`, `0x10091f20`), not
  the selector's state.
- **So the lines show whenever the driven unit has a wingman**, with the selector
  off too. Line *i* is placed at (0, 19 *i*) and drawn (`0x1004321c`).

**A line** (`0x1009d990`, *read*) is built left to right with the pen
([35-hud.md](35-hud.md#the-weapons-list--read-and-measured)) from its top left:

| piece | primitive | width | shows |
|---|---|---:|---|
| `ending_text` | `0x10099a30`, variant 1 | 5 | |
| `body_text` | `0x10099f60` | 12 | the number, centred |
| a lamp | `0x1009a8f0` | 19 | by the line's look, below |
| the first icon | `0x1009a7a0` | 19 | by the unit's Type |
| the second icon | `0x1009a7a0` | 19 | by the unit's property `0x207` |
| `separator_left_text` | `0x10099c90`, variant 0 | 5 | |
| `ray_emitter` | `0x10099d80` | 10 | by the line's look |
| `ray_body`, the bar | `0x1009a380` | 135 | the unit's name (its record's `+0x44` object, slot 41), centred over a fill of its life percentage (`0x1007e980`) |
| `ray_ending` | `0x10099e70` | 6 | |

- **An icon piece** (`0x1009a7a0`) is square. Its side is the `body_text`
  piece's height, 19. The piece is drawn white and the icon inside it inset by
  2, 15 × 15, in the icon's tint, and the pen moves 19.
- **The icons and their tint** are the commander's unit box's (`0x10077120`,
  [41-commander.md](41-commander.md#the-box)).

| the line | lamp | emitter | icons' tint | text |
|---|---|---|---|---|
| chosen | green (3) | `_pressed` (2) | as given | white |
| not chosen, picking | yellow2 (2) | `_off` (0) | as given | `#808080` |
| not chosen, otherwise | black (4) | `_off` (0) | halved: (tint >> 1) & `0x7f7f7f`, opaque | `#808080` |

So the line is 230 wide. Its name bar runs from x 89 to 224.

**The menu** (*read*). In state 2 the panel places the order menu at (200, 200)
(`0x1007b1a0`) and builds it for the chosen records with the wingman flag
(`0x1007a8e0`). The builder (`0x1007aaa0`) makes one line per row of the second
table, 19 high from y 50 at x 220 (`0x100670d0` with 0xdc, y, 0x17c, y + 19). It
closes the selector if no chosen unit is left (`0x1007ab59`, `0x1007ada1`).

**The menu's rows** (`0x1007b1e0` with the wingman flag, `0x1009c830`, *read*):

- **No strip.** The flag skips the commander's *Orders* strip (`0x1007b206`).
- **Each row** is placed where the builder put it and drawn with its number.
  Left to right from (220, 50 + 19 *n*):

| piece | primitive | width | shows |
|---|---|---:|---|
| `ending_text` | `0x10099a30`, variant 1 | 5 | |
| `body_text` | `0x10099f60` | 12 | the row's key, *n* + 1, centred |
| `ray_emitter_normal` | `0x10099d80`, variant 1 | 10 | |
| `ray_body`, the bar | `0x1009a380` | 150 | the order's string, centred, with no fill |
| `ray_ending` | `0x10099e70` | 6 | |

- **Enabled or not.** The number and the string are white on an enabled row and
  `#808080` on a disabled one (`0x1009c849`–`0x1009c86d`).
- **Width.** A row is 183 wide and ends at x 403.
- **The commander's rows**, which get no number, are drawn differently:
  `ending_stub` (variant 0), `ray_emitter_normal`, a bar 247 wide that a pressed
  row fills for a second, and `ray_ending`.

A row is **enabled** when both of these tests pass (`0x1007acb4`–`0x1007ad7e`):

- **The target.** A row that needs one (Attack, Capture building) needs the
  driven unit's current target. The target must not be of the player's clan
  (its record's `+0x24` against game `+0xad0`). A row with pick mode 2 (Capture
  building) needs that target to be a building (object type 3).
- **The capturers.** Search and capture and Capture building need every chosen
  record's `+0x30` to be 1 or 2. Capture building also refuses a building of type
  `0x80000200`.

**Against the recording of Mission 01** (*seen*, 960 × 720, 1.5 × the layout):

- **Before any capture** (196–203 s) no line is drawn.
- **From 204 s** a line shows for *TFW-2 Warrior*, and from 205.5 s a second one
  for *MFW-1 Warrior*.
- **Unchosen with the selector off**, both lines are what the table reads: black
  lamps, dim icons, grey names.
- **At 208 s the menu is open.** Both lines are chosen, with green lamps, lit
  emitters, full icons and white names. The seven rows *Standby* … *Refit* stand
  under x 220, 19 apart, their numbers in dark boxes.
- **By 208.5 s an order has been given.** The menu is gone and the lines are
  unchosen again. They stay on screen to the recording's end.
- **Row by row** at the top left: the number box 0–17, the lamp 17–36, the two
  icons to 74, and the name bar from about 90 to 223.
- **The icons' tints.** The second line's icons (*TFW-2*, `+0x30` read as 1) are
  near white. The first line's (*MFW-1*, 3) are green.
- **Disabled rows.** The menu is open again from 228.7 s to 228.9 s, under the
  message box. *Search and capture*, *Attack* and *Capture building* are grey:
  the chosen *MFW-1* is no capturer, and the target, *MFW-1* itself, is of the
  player's clan. *Seek and destroy* and *Refit* are white.

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
  **It moves nothing.** Its start and every takt ask for fire mode 2 and clear
  the walker (`0x10031ca0`, `0x10031d00` → `0x1003c540`; `0x10031bd0`, once
  quoted here, is the shutdown task's takt), emptying its three queues and
  giving the wizard no new points
  ([24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)).
- **Route — go.** It walks to the place at `Go_SpeedPercent` 1.0 of the unit's
  speed and ends when it stops there ("We are staying... task over",
  `0x1002b670`), or fails if the place is unreachable. It acts only when the
  walker is idle. Within 30 of the place it is over; 1.5 counts as arrival
  when the order names an object. Both are measured across the ground: the length
  they are compared with (`0x1002b7d0`, `0x10020f70`) takes x and y alone.
  Otherwise it calls `SetTarget` again. **A go that ends at a place while it is the
  unit's only order** (`IBehaviour` slot 4 answers 1, `0x1002b83b`) first gives it
  `PATROL` of radius 150 about that place, to the end of its list
  (`0x1002b8a8`–`0x1002b8d7`), so a unit sent somewhere and given nothing after
  keeps patrolling there
  ([42-selection.md](42-selection.md#spreading-a-group--read)).
  "Unreachable" is that call's refusal: out of the map, no areal map, or no
  global path
  ([24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)).
  The go task will not be
  interrupted by reasons 0–2 or 5, only 3 and 4 (`0x1002b390`).
  **The Route row does chain waypoints** (*read*, in
  [42-selection.md](42-selection.md#an-order-row-leaves-a-pick-open--read)).
  This paragraph once said it does not, from a search that swept only the
  stores to an `+0xc4` field, the list's end, and so missed the pick's appends.
  The dispatcher's case for Route (`iron3d.dll:0x10079230`) first empties the
  unit record's list of points (`+0xc0`, 8-byte x/y records, `0x1007c0a0`),
  then gives one `GO` to the picked place, replacing. The unit record's
  constructor (`0x10074af0`) builds that list. The Guard row's pick
  (`0x10079f40`) gives a `PATROL` of radius 300, replacing, on one of two units
  the record names or on the list's *first* point, and empties the list. The
  list **is** filled: a place clicked in the Route or the Guard pick joins every
  selected unit's list (click kinds 12 and 8), and the Route pick
  (`0x10079750`) then gives the pending unit a `GO` to each point in turn, the
  first replacing its queue and the rest appending. The go task holds one place
  and the queue the rest.
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
  corner of the building's ground contour, then walks to the pod: the contour
  is the `.bas` outer ring, and a corner is walkable on an areal whose first
  flag word is set ([The capture, tick by tick](#the-capture-tick-by-tick--read-measured-and-seen)). If the walk to
  the pod is refused — a ruin has no pod — the plan falls through to roaming,
  and the next plan picks the same building again, so a capturer can hang about
  a ruin (*derived*). When the
  building turns the unit's own clan the task logs "Building [..] captured"
  (`0x10030474`) and, being a search by type, **plans the next one** at once
  (`0x100304b9`): it does not end, and the code read gives it no escape — it
  walks out of the building towards its next target (below).
  - **Rescan:** every 3 s plus up to 3 s, against 15 plus up to 15 s in the other
    modes (`0x100301b2`, `0x1003011b`); a plan also runs whenever the unit stops.
    The timer is only looked at while no building is picked: with one picked
    and still another clan's, the takt goes straight to the walker
    (`0x10030352`).
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
  patrol task walks a loop of random points around it and draws a fresh loop
  on a timer (`0x1002dd90`), the timer only while `Behavior.ini`'s
  `DeterminMode` is 0 (`0x1002d900`); [The patrol, tick by tick](#the-patrol-tick-by-tick--read)
  has it whole. The compiled defaults by target, read where `SetTarget`
  (`0x1002d520`) and the start (`0x1002d7c0`) take them:

  | guarding | radius | new loop every | speed |
  |---|---:|---|---:|
  | a unit | 60 | 5 + up to 10 s | 1.0 |
  | a building | 30 | 60 + up to 60 s | 80 |
  | a place | 60 | 20 + up to 10 s | 0.8 |

  **The building's 80 changes nothing a 1.0 would not** (*read*). A walk is
  asked for at the unit's speed (`+0x5fc`) × the task's figure, and
  `MWalker::SetTarget` (`0x1003bad0`) holds the request
  ([How a walk's speed is held](#how-a-walks-speed-is-held--read)):
  - no more than the unit's speed × `Movement_SpeedPercent` (1) ×
    `Speed_MaximumFactor`, the difficulty block's 1;
  - no more than `Movement_MaxSpeed`, 600;
  - no less than the unit's second figure × `Movement_MinSpeedPercent` (1);
  - no less than 2.

  The unit's takt copies the same control record into both `+0x5fc` and the
  `+0x614` the cap reads (`0x1001bbe0`). So 80 × the speed is cut to the unit's
  full speed, exactly what a unit guard's 1.0 gets.
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
    `SetTarget` (`0x1002b059`) accepts a spot only on a walkable areal, or
    any for a flyer, and only once the global path to it is found: it answers
    1 when the path does (`0x1003bfdf`), 0 otherwise
    ([24-motion.md](24-motion.md#the-global-path--read-and-measured)). Near a
    bridge most spots about a leader on it lie over the canyon, and are
    refused.
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

## The capture, tick by tick — *read*, *measured* and *seen*

What a small warbot does between its order and the building turning, as the
search task (vtable `0x10059cf8`) runs it. *Teleport*, the fourth training
mission, is the example: the player's Tiny Helicopter `tut4_f1` (`R_T_02`,
size `t`) takes the neutral Large Factory `lplant01` and Research Center
`einst01` from command mode.

### The orders that start it — *read*

- **A click on the building.** With every selected unit of size class 1 or 2,
  a left click on a building of another clan is pick kind 4. The cursor over it
  is `CAPTURE`. With no pick pending, the click is the dispatcher's case 6:
  `SEARCH`, target `0x201` with the building's logic id, parameter
  `0x8017365e` ([42-selection.md](42-selection.md#a-left-click-in-the-world--read)).
- **Search and capture** from the HQ menu is case 3: `SEARCH`, target `0x203`,
  parameter `0x8017365e`. The row is offered when the first selected unit's
  `+0x30` is 1 or 2 ([41-commander.md](41-commander.md#which-rows-it-offers)).
- **A script's** `ORDER_ROBOT_CAPTURE` builds the same task
  ([The orders](#the-orders--measured)).

Both menu paths read *searching* on the status line, since the head order is 5.

### Setting the target (slot 3, `0x10030110`) — *read*

It clears the four mode words: `+0x58` minerals, `+0x5c` capture, `+0x60` one
building, `+0x64` enemies. It sets the rescan timer `+0x74` to 15 s plus up to
15 s. Then it switches on the target's kind:

| target | what must hold | what it sets |
|---|---|---|
| `0x201`, a logic id | the id has the top bit, a building's; the unit's variable `0x201`, its size class, is at most 2; the system areal map (behaviour `+0x48`, slot 21) knows the id; the building's clan is not the unit's; its Type is not `0x80000200` | `+0x5c`, `+0x6c` the id, `+0x60` |
| `0x203`, by type, a mask with `0x10000000` | — | `+0x58` |
| `0x203`, a mask with the top bit | size class at most 2 | `+0x5c`, and the timer to 3 s plus up to 3 s |
| `0x204` | — | `+0x64` |

Anything else, or a failed test, refuses the order. The mask is not kept: the
capture plan pushes its own `0x8017365e` (`0x10030806`).

**The start** (slot 6, `0x10030280`) asks the fire control for mode 2 and sets
the **landing flag** `+0x7c`. The flag is 0 when the chassis profile has
`CanFly` (`0x10014670`, `+0xc`) and 1 otherwise. Then it plans.

### The plan (slot 15, `0x100306f0`) — *read*

1. **The building.** A search by type picks the nearest known building
   ([What each package does](#what-each-package-does--read)) into `+0x6c`. A
   single building keeps the one it was given.
2. **With a building, for a unit that cannot fly or whose landing flag is
   set** (`0x1003091b`), it walks in: `0x10001ab0` with the id, the place
   flag `0x40` and the unit's speed (`+0x5fc`) × `Go_SpeedPercent`.
   `MakeInsideDest` (`0x10001270`) makes the goal:
   - the building, by the clan areal map's slot 7, must be finished (its
     variable `0x202` not above 0, else *"Making Go Inside Non-complete
     Building"*);
   - the unit's size class must be at most 2 (*"TypedSizes missmached"*);
   - the building's hall way (interface `0x303`) must hold a vertex with bit
     `0x40`, the pod (slot 13, `GetBestVertexOfType`, `ArealMap.dll:0x1000a7e0`).

   The goal is that vertex's position carried into the world through its
   node (slot 5, `0x1000a760`). The walker is given the goal with −1 and 5
   (`MWalker::SetTarget`). If it accepts, the plan sets the landing flag and
   ends.
3. **For a flyer with the landing flag clear, or a flyer whose walk in was
   refused** (`0x1003096a`), it clears the flag and lands first:
   - of the building's **contour**, variable `0x203` (below), it takes the
     vertex nearest the unit across the ground (`0x10020f70`);
   - it counts only a vertex on an areal whose record's `+0x20` word is not 0.
     The areal is the system areal map's slot 7 at the point, and the record
     its slot 6. This is the same first flag word that a non-flyer's valid
     place needs ([42-selection.md](42-selection.md#a-valid-place--read-and-measured));
   - it goes there (`0x10001960`) at the same speed, with −1 and 5.
4. **Otherwise** the plan falls through to the retreat and to roaming
   ([Where a search looks](#where-a-search-looks--read-and-measured)).

**The contour** (`0x1000a611`, variable `0x203` of the building's behaviour) is:
- **a standing building's** (`+0x968` = 1): `IBasement` slot 3, the `.bas`
  **outer ring** placed in the world ([03-terrain.md](03-terrain.md)). It must
  be one ring: *"Behaviour panic: Building has %d contours"*. Interface `0x11`
  is `IBasement` (`Terrain.dll:0x10057cb9`, `CBuilding` + 4).
- **while its construction sphere runs** (`+0x968` = 2, which the sphere task
  writes at `0x10031342` and `0x10031746`, and back to 1 after): eight points,
  at angles 0, π/4, …, on a circle about the sphere that `IBuilding` slot 15
  gives. The radius is that sphere's ÷ cos(π/8) + 20. Only x and y are written.
- **nothing** for any other object.

### Each tick (slot 7, `0x10030300`) — *read*

1. **The building's clan.** With a building picked, it looks the building up
   by id in the system areal map. If the building is gone or has become the
   unit's clan's, a single building **ends the task** (`0x10030362`, returning
   0) and a search by type plans again. If it is still another clan's, it goes
   straight to step 3: no timed rescan.
2. **With nothing picked**, the rescan timer plans again when it fires.
3. **The walker busy**: *"We are moving..."*, and the task goes on.
4. **The walker idle, the landing flag clear** — a flyer at its corner: it
   sets the flag and plans (`0x10030412`). The next plan walks it in.
5. **The walker idle, the flag set**: if the building (clan areal map, slot 7)
   is now the unit's clan's, it logs *"Building [..] captured"*. A single
   building ends. A search by type forgets the building, resets the landing
   flag from `CanFly` and plans the next one. If the building is not yet the
   unit's clan's, it plans again.

   So a unit waiting on the pod for it to open plans again each tick, and each
   plan hands the walker the pod again.

### What fires it, and what follows — *read*, and *derived*

- **The pod.** The building's computer fires when the unit's bounding-sphere
  centre stands in its zone
  ([27-ownership.md](27-ownership.md#capture--read)).
- **A flyer is in the zone too.** The one state of `r_t_02`, `r_l_02` and
  `r_b_02` (mode `0x150c1`, *measured*) lacks `0x8000000`. So the ground
  contact places a flyer on the face under it, and makes it that face's
  object's child, as it does a walker
  ([24-motion.md](24-motion.md#finding-the-ground--read), step 7). That a
  flyer over the pod's floor is a building child the zone test sees is
  *derived*.
- **The capture.** The callback changes the owner. When the player's clan
  gains the building, the callback shows 5039 and plays the voice
  ([27-ownership.md](27-ownership.md#capture--read)). This happens for an AI
  unit of the player's clan as for the hero.
- **A single building ends the task.** The unit, idle on the building, gets
  the unit takt's escape: no target, replacing ([The escape](#the-escape--read)).
  The escape's start (`0x1002ba50`) never asks the chassis profile. A flyer's
  points must lie on usable areals, as a walker's do.
- **A search by type goes on.** It plans the next building from the pod,
  gets no escape, and roams once nothing is left.

### Mission 04 — *measured*

With the helicopter at its start (477.9, 530.6), on areal 355, whose flag word
is 0:

| building | across the ground | picked? |
|---|---:|---|
| Large Factory `lplant01` (`fr_b_plant`), neutral | 549.1 | **first** |
| Research Center `einst01` (`fr_e_inst`), neutral | 591.3 | second |
| main teleport `mtp_m_n1`, neutral | 998.6 | never: the plan skips it |
| generator `gener01` | 122.0 | never: the player's |

**The factory.**
- Its outer ring, placed at rotation −0.616, has 14 vertices, all on areals
  whose word is 1.
- The nearest to the start is vertex 9 at (734.3, 892.0), 443.1 away. Vertex 8,
  at 445.1, is next.
- Its pod is hall-way vertex 31, at (750.02, 950.70, 41.5).
- Three of its nine exits reach the pod along the hall way: 67 at
  (733.1, 1022.2) by 115.5, 68 by 161.4, and 69 by 170.3.

**The research centre**, planned from the factory's pod:
- Its ring has 9 vertices, all on areals whose word is 1.
- The nearest is vertex 8, the ring's last, at (881.8, 900.3), 141.1 away.
  Vertex 7, at (897.4, 922.7), is 150.0 away. (This page once counted 13 and 8
  vertices and gave vertex 7: its check dropped each ring's last corner twice,
  once as the closing repeat the reader already drops.)
- Its pod is vertex 0, at (950.71, 869.39, 65.65).
- Its three exits all reach the pod: 19 by 92.3, 21 by 108.9, and 23 by 108.8.
  Exit 23 is at (877.1, 920.9).
- Route 2 of the mission holds the research centre and vertex 8. Its script
  shows `T04_H02` once the helicopter (id 3) or the hero (id 2) is reported
  inside it (function 32, [15-behaviour.md](15-behaviour.md)).

### Against the recording — *seen*

The recording of *Teleport*, 960 × 720. Its map positions are read at one
layout unit of the commander's map, 6.6 m on `Tut_4` (side 1697), and are good
to about ±13 m.

- **84–86 s, the Battle units page.**
  - The HQ, *LWC-1 Comm. Center*, is offered Standby, Route, Seek and destroy,
    Guard and Refit.
  - The *TFW-2 Warrior* is offered the same, plus Search and capture.
  - At 86 s, after a click on that row, the helicopter's line reads
    *searching*.
  - The HQ reads *standing* at 89 s and *patrolling* at 96 s.
- **The flight** (the helicopter's mark on the commander's map).
  - It passes (638, 729) at 112 s, (683, 800) at 120 s and (736, 888) at 128 s.
  - At 136 s it is at (722, 888), the factory's vertex 9.
  - It passes (692, 986) at 144 s and (727, 1025) at 152 s, by exit 67.
  - At 156 s it is at (758, 994), inside.
  - It averaged about 10 m/s to the corner.
- **175.5 s, the factory turns.**
  - The message box reads *"from: System / Building is captured"*.
  - At 176.0 s, `T04_I03` follows.
  - Between 175.0 and 175.5 s the factory's map icon turns from grey to light
    blue.
- **The research centre.**
  - At 237 s, `T04_H02` shows: route 2.
  - At 256 s, *"Building is captured"*, then `T04_I04` at 258 s.
- **After the last capture**, the helicopter's cross is at (959, 995) at
  282 s and (787, 926) at 302 s. It then holds at about (715, 847) from 362 s to
  402 s.

So the plan's order (factory, then research centre) and the landing corner
agree with the recording.

### For an engine

1. **Orders.**
   - A click on another clan's building, with only small units selected: a
     search on that building.
   - Search and capture: a search by type.
2. **A flyer lands first.** Take the building's `.bas` outer ring in the world.
   Keep the vertices on areals whose first flag word is set. Fly to the one
   nearest across the ground.
3. **Then walk in.** Once there, or at once for a walker, go to the pod vertex
   (hall-way bit `0x40`) at speed × `Go_SpeedPercent`. The hero's ways
   ([24-motion.md](24-motion.md#walking-into-a-building--read-and-measured))
   are a stand-in for the path the walker plans.
4. **The capture.** The building fires as for the hero: change the clan, and
   show 5039 and play the voice when the player's clan gains it.
5. **After.** A single building ends and the unit escapes. A search by type
   takes the next building from where it stands, and roams once none is left.

### Not established

- How the walker's path joins the hall way (`MGraph`), and so which exit a
  capturer takes. The hall way's shortest way from the nearest exit is a
  stand-in.
- How high a flyer's points are put, and so whether it touches down at its
  corner or hovers there. ~~`Movement_FlyHeight` 40 and `FlyNearLandHeight` 15~~
  are ruled out: their only reader, `Behavior.dll:0x100153a0`, is never called
  ([24-motion.md](24-motion.md#how-the-ai-drives-a-machine--read-and-measured)).
- Why the helicopter covered its first 443 m at about 10 m/s, a third of its
  33.3 m/s forward top speed.
- What held it at (715, 847) from 362 s: a roaming search plans again whenever
  its walker stops.
- `IBuilding` slot 15's sphere, which the octagon contour is drawn about.

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
areal (`0x10030ed2`). A usable areal is a walkable one: the test reads its
record's first flag word (`0x10030fc4`,
[24-motion.md](24-motion.md#the-global-path--read-and-measured)). It moves at its speed × `Go_SpeedPercent`, 1.0, in every
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
  control's, below. Which contacts score, and how far the attack may take the
  unit, are the current task's to say:
  [The patrol, tick by tick](#the-patrol-tick-by-tick--read) reads a patrol's,
  and [A task's limits](#a-tasks-limits--read) the circle a task hands its
  attack.
- **Refitting.** A unit whose life or charge is below half, or whose guns are
  mostly dry, sends itself to a dock as a reason-3 task
  ([27-ownership.md](27-ownership.md)). The default priority lets reason 3
  through only if a dock is reachable (`0x100018a0`).
- **With no order.** An animal's default order is 15, migrate; a mine's is to
  mine (`0x100088f0`). A unit placed inside a building gets a patrol inside it
  ("Give default patrol inside building order", `0x1000ac4c`). **A unit left
  idle or stopped on a building escapes** from it ([The escape](#the-escape--read)).
  For other units no default order was found: an empty stack answers with an
  embedded stop task, and a go to a place that was a unit's only order leaves a
  patrol of 150 about it ([Route — go](#what-each-package-does--read)) ([Which objects run a behaviour](#which-objects-run-a-behaviour--read)).

### Migrate: an animal's pasture — *read*, and *measured*

Order 15's task (vtable `0x10059aac`) keeps an animal on its clan's pastures,
the clan's zones as the mission file gives them: a centre, an inner radius and
an outer one ([04-missions.md](04-missions.md)), which `IMission` slot 8 hands
the areal map as the clan's *migration areals* (slot 35, `0x100220e0`).

- **One pasture a clan.** The system areal map keeps a current pasture per
  clan (slot 47, `ArealMap.dll:0x10022230`). It answers the same one until the
  clan's timer runs out, then picks `rand() % count` — the same one again,
  possibly — and restarts the timer. The timer's words are 937 and 1875, ×64
  ms (`0x1002ab37`, `0x1002ab3d`): **60 s, plus up to 120 s**. The current one
  starts at −1, so the first question picks at once (`0x10022264`); a clan with
  no zones answers −1 and logs *"Migration Place Error: Clan … has no
  MigrationAreals"* (`0x10022294`). Only a question moves the clan on: the
  answer changes when an animal asks after the timer has run out.
- **The start** (`Behavior.dll:0x1002c9d0`) asks for the clan's pasture into
  `+0x60`, starts its own timer at 60 + up to 120 s (`+0x58`), walks to a
  point, and starts a second timer at 5 + up to 10 s (`+0x64`). Both use the
  attack task's timer helper, (fixed, random) in seconds (`0x1004c4c0`).
- **The point** (`0x1002cba0`): the pasture's centre plus (*u* × inner,
  *v* × inner), where *u* and *v* are each `rand()` over 32767 held to at least
  0.2 (`0x10059770`, `0x100597e8`), and the centre's own height. The walker does
  not keep that height: the walk to a point gives each trajectory point the
  ground under it and 15, and an animal's 30 and up to 50 more, 45 to 95 m over
  the ground ([24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)). **Both
  offsets are positive**, so the point always lies in the square to the +x, +y
  side of the centre, 0.2 to 1 inner radius along each axis. It tries up to 50
  points until the walker takes one (`0x10001960`, `0x1002cc92`), at the unit's
  speed × `Go_SpeedPercent` — the patrol block's `+0x34`, bound by that name at
  `0x10016598`. With no pasture it walks nowhere (`0x1002cbbc`).
- **Each tick** (`0x1002ca60`) asks the fire control for mode 0, no target of
  its own. When the long timer has run out it asks for the pasture again,
  starts the long timer again, walks to a new point and starts the short timer.
  Otherwise, when the walker is idle and the short timer has run out, it walks
  to a new point; while the walker is busy it restarts the short timer, so an
  animal stands 5 to 15 s at each point it reaches.
- **Every distance it measures is across the ground**: the score's and the
  priority's go through `0x10020f70`, which takes x and y alone.
- **What it lets through.** A migrating animal is the one animal that engages
  at all (`0x10017a1e`, [above](#between-orders--read)), and the task decides
  what:
  - **The score** (slot 13, `0x1002c910`): a hostile contact within the inner
    radius of the pasture's centre scores 1 / (*d* + 10), *d* its distance
    from the centre (`0x10059144`); anything else scores 0. The engagement
    takes only a contact scoring above 0 (`0x10017f75`).
  - **The priority** (slot 12, `0x1002c640`), by reason
    (`0x1002c8f8`): for an engagement, reasons 0, 2 and 5, it is 1 when the
    animal itself is strictly inside the outer radius of the centre and the
    contact within it (`0x1002c706`–`0x1002c722`), and 0 otherwise. For
    **retaliation**, reason 1, it is always
    1. Reasons 3 and 4 get the default. The task also hands the attack it lets
    through a time and a circle about the pasture's centre, which `0x100179c0`
    merges into the new task ([A hit pulls a unit in](#a-hit-pulls-a-unit-in--read)
    gives them).
- **So**: a grazing animal fires at nothing. It attacks a hostile unit that
  comes within the inner radius of its clan's current pasture while it is
  itself inside the outer radius (*derived*), and anything that hurts it
  (*read*, [below](#a-hit-pulls-a-unit-in--read)). The attack is then the
  animal's version of [the attack](#the-attack-tick-by-tick--read).

**With no pasture** (*read*) the task walks nowhere, nothing scores
(`0x1002c91a`), and its priority is the base's for every reason
(`0x1002c653` → `0x1002c8df` → `0x100018a0`): a hit is answered as a stopped unit
answers it, 1,000 about where it stands, and an engagement is scored by nothing.
An animal whose firer stands exactly on the outer circle is answered the same
way, the one case the table below leaves to the base.

*Measured:* only nature clans carry zones — 12 of the 15, each with animals
of its own. One more places animals with no zone at all, where slot 47 has no
pasture to give, so its animals graze on the spot. Mission 02's two
pastures are in [34-progression.md](34-progression.md#mission-02-the-constructor-end-to-end--derived).

### A hit pulls a unit in — *read*

**Every hit tells its victim who fired.** `ILifeSystem` slot 8
(`Control.dll:0x1000ebc0`), the hit a struck object takes, first sends its object
message `0x19` with the firer's object id (`IGameObject` slot 13 with class `0x10`,
`0x1000ebdf`). That is before the shields, the invulnerability test and any damage
is worked out, so a direct hit and every object a blast reaches (`0x10012ce0`) are
told, **whatever the hit does to them**. A round passing through a shield bubble
tells the shielded object too (`0x1000d1ed`), while its firer exists. The id is the
round's property 127 (`0x10011766`, setter `0x1000e9d2`), which the gun's fire
(`0x1002a3e8`) sets to the firing unit's id. **Message `0x1a` is never sent**: a
search of every binary finds the two `0x19` sends and nothing else.

`AniMesh.dll:0x10001370` routes class `0x10` to the agent's behaviour
(`MBehaviour::SendMsg`), and slot 67 (`Behavior.dll:0x100064b0`) takes it. It does
nothing for a building, for property `0x208` set (`+0xa64`), or in a network game
(`+0xa04 & 4`, from the game mode's bit 2). Otherwise:

1. It asks for a **reason-1 attack on the firer's logic id** (`0x10018060` →
   `0x100179c0`).
2. Unless the victim is a hero (`0x1020000`), it adds **0.004** to its clan
   SuperAI's decrease toward the firer's clan (`ai.dll:0x10001fe0`, the SuperAI's
   vtable slot 13; the `1.0` this call pushes at `0x1000657f` is the slot's
   third argument and it is never read). The clan's takt,
   every 7–8 s (`0x100017f0`), takes that off the attitude and reads the word from it
   again ([25-sensors.md](25-sensors.md#what-moves-an-attitude-being-shot-and-nothing-else--read)),
   so enough hits turn a neutral or allied clan hostile, both ways: **13** on an
   ally, **42** inside one takt on a neutral. **This is the only thing in the
   game that moves a relation**, and it only moves it down.
3. It **calls for help** (`0x1000c260`): message `0x12d` to every warrior
   (`0x1008000` and no other bit) in its clan's areal snapshot within **400**, in
   three dimensions. Each asks for a reason-5 attack (`0x10018030`) on the nearest
   hostile warrior, builder or transport (mask `0x100e000`) its clan knows within
   **3,000** of the victim, across the ground (`0x10001120`). **A hero is never
   picked**, so a call against a lone hero finds nothing.

**The gate** (`0x100179c0`) every interrupt passes, in order:

| | refused when |
|---|---|
| 1 | an animal (`0x20000000`) whose current order is not 15, migrate: **an attacking animal takes no other interrupt** |
| 2 | `DeterminMode` is set |
| 3 | the unit is a building |
| 4 | its clan's type is 3, neutral |
| 5 | its current order is 7, build |
| 6 | the running task's priority for the reason (slot 12) is 0.3 or less |
| 7 | the pause `+0x5e0` has not run out: **2 s plus up to 3 s** (`0x1000388e`), one for every reason, started again whenever the gate reaches it, even if what follows fails |
| 8 | a unit that is not a building with a speed cap (`+0x614`) below 0.5 or no mobile flag (`0x800`) (`0x10034510`) |
| 9 | the attack's target (`0x10026d40`) is missing, itself, **of its own clan**, or destroyed: **no relation, radar or distance is asked** |
| 10 | the unit has no weapon (the attack's start) |

The new task goes on top. Its limit is the running task's answer merged with the
running task's own, the tighter of each winning, and its start stamps the limit's
timer (`0x100349a8`). Slot 9 (`0x10001660`) ends the attack past its time or outside
its circle, in three dimensions, and the task beneath starts again.

**What each task answers a hit** (slot 12 with reason 1, and its limit):

| task | answer | the attack's limit |
|---|---|---|
| stop, and the tasks that embed it (seek and destroy, mine, construct) | 1 | 1,000 about where the unit stood (`0x100018a0`) |
| patrol (`0x1002d250`) | 1, wherever the firer is | a place: radius + 60 about it; a building: radius + 80; a unit guarded: radius + 60 about it; inside a building, none |
| follow (`0x1002acd0`) | 1 if armed | 2 × radius + 20 about the leader |
| an attack an interrupt made (`0x10026b10`) | the firer's score, 1000 ÷ (*d* + 10) across the ground, 0 past 700 or with no snapshot record; it switches only to a firer scoring above its own target | |
| an attack an order gave (`+0x54` 1) | 0 | |
| migrate (`0x1002c640`) | 1 | by where the animal and the firer stand, across the ground, below |
| standby, shutdown, go, transport, build, capture and capture building, search minerals, random go, boarding, research, the construction sphere | 0 | |

A migrating animal's attack, about its pasture's centre:

| the animal | the firer | time | circle |
|---|---|---:|---|
| beyond the outer radius | anywhere | 10 s | none |
| within the outer radius | beyond it | 20 s | outer + 20 |
| within the outer radius | between the inner and the outer | 25 s | outer + 80 |
| within the outer radius | within the inner | 35 s | outer + 100 |

For an engagement (reasons 0, 2, 5) a migrating animal answers 1 only when it and
the contact are both within the outer radius: 20 s and outer + 80 for a contact
within the inner radius, 10 s and outer + 20 otherwise.

**An animal's attack** is the ordinary one with the animal's timers (5 + 5 s and
4 + 4 s), going straight at the target on 30% of its picks. It fires on the target
(mode 1) from within 200 and on the nearest hostile contact (mode 2) beyond, the
target's place taken from the system map, not the radar. The fire control does
nothing for an animal whose walker is idle (`0x10024069`).

*Measured*, Mission 02's pastures: the western's outer radius 50, the eastern's 40,
both inner 20. A hero firing from off a pasture draws a 20 s attack held within 70 m
(west) or 60 m (east) of its centre.

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
kept unless `+0x5c` or `+0x60` is set. It also refreshes every gun's fire
frequency with the difficulty block's `Fire_FreqFactor` (`0x1001b650`), the
default 1 on every unit
([26-damage.md](26-damage.md#the-difficulty-block-every-behaviour-holds--read-and-measured)).

| mode | asked by |
|---:|---|
| 1 | attack (start, takt and manoeuvre) |
| 2 | stop, go, patrol, search, transport, random go, stay ground, follow, leave |
| 0 | shutdown, migrate |

**The 0.5 is the field's own value, and nothing reads it** (*read*). The
request copies its three words into `+0x28`, `+0x2c` and `+0x30`
(`0x10023f45`–`0x10023f55`), and the constructor has already put 2, −1 and
`0x3f000000` there (`0x10023ec2`–`0x10023ecc`). All **20** call sites of
`0x10023f30` in `Behavior.dll` push 0.5 as the third word, so every request
rewrites `+0x30` with what it already holds. Over the fire control's own code
(`0x10023e80`–`0x10025c60`) a sweep of every memory operand finds two readers
of `+0x28` (`0x100240a6`, `0x10025430`) and two of `+0x2c` (`0x100240b5`,
`0x1002543f`) — the mode and the id the takt picks by — and **none of
`+0x30`**; and no instruction in the module touches `MBehaviour+0x38c`, the
same field reached through the behaviour's base. That is the control for the
negative: the same sweep, over the same object, does find the neighbours.

**Neither lock can ever be set** (*read*). `+0x5c` has one writer,
`Behavior.dll:0x10023fd0`, which sets it to 1 and puts an id at `+0x64` — and
**nothing calls it**. There is no direct call anywhere in `Behavior.dll`, and
no vtable slot either: the constructor installs `0x1005956c` at the object's
`+0` (`0x10023f1b`), a 20-slot table the fire control inherits whole and
overrides nothing in. The only place the value `0x10023fd0` occurs as four
bytes in any section of any shipped module is `World3D.dll:0x1000c167`, which
is `push offset "CICLS_SIMPLE"` — the modules share an image base, so a string
address collides with a code one. `+0x60` is
set to 1 and cleared to 0 only inside `0x10025a00` (`0x10025b8b`,
`0x10025bda`), and `0x10025a00`'s one caller is the takt at `0x1002422a`,
which reaches it **only when `+0x5c` or `+0x60` is already set**
(`0x10023f36`'s mirror at `0x10024216`). `0x10025c40`, a two-line predicate
that answers whether either is set, has no caller either. So the guarded
branch at `0x10023f38`/`0x10023f3f` never takes: **0 of the 20 requests are
ever refused**, and a task always gets the fire mode it asks for. The control
is the same call scan over the same class, which finds the constructor's
caller (`0x10003723`), the takt's (`0x100050fc`), the request's twenty and
`0x10025a00`'s one. The twenty cover every task in the table above, so there is
no order in any shipped mission whose fire-control request could be turned
away, and no count to take over the 458 assemblies: the number is 0.

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
   ([The fire control](#the-fire-control--read)). Nearing is the flag at `+0x64`
   (`0x100272c9`), which only a unit target's move sets: **a building target is
   never neared**, since its move clears the flag (`0x1002777d`, below), so the
   fire control holds the building from the first pick on, at any distance.
   *Seen* on C03 M02 ("Let's Play - Parkan: Iron Strategy, Part 6",
   -yNnsqudMzw, 13:49.5–13:54.2): the raider ordered onto the player's Small
   Bunker lands a winged SSM on it while it still stands 292 m off. The
   building, which the radar never lists, is out of reach of mode 2.
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

## The patrol, tick by tick — *read*

Order 4's task (vtable `0x10059d38`; slot 0 answers 4) guards a place, a
building or a unit. Mission scripts give it more than any other order
([How the missions use them](#how-the-missions-use-them--measured)), and the
HQ menu's Guard gives it too. Its constants are `Behavior.dll`'s compiled
defaults (`0x10016250`), bound by name into a block at behaviour `+0x694`
(`0x10016480`) — *measured*, re-read by `openparkan verify`:

| block | constant | default |
|---|---|---:|
| `+0x00` | `Patrol_Place_SpeedPercent` | 0.8 |
| `+0x04` | `Patrol_Place_Radius` | 60 |
| `+0x08`, `+0x0c` | `Patrol_Place_ChangeTrajectoryMinDelay`, `…RandomDelay` | 20, 10 s |
| `+0x10` | `Patrol_Building_SpeedPercent` | 80 |
| `+0x14` | `Patrol_Building_Radius` | 30 |
| `+0x18`, `+0x1c` | `Patrol_Building_ChangeTrajectoryMinDelay`, `…RandomDelay` | 60, 60 s |
| `+0x20` | `Patrol_Unit_SpeedPercent` | 1.0 |
| `+0x24` | `Patrol_Unit_Radius` | 60 |
| `+0x28`, `+0x2c` | `Patrol_Unit_ChangeTrajectoryMinDelay`, `…RandomDelay` | 5, 10 s |
| `+0x30` | `Patrol_Attack_Range` | 400, never read |

### Setting the target (slot 3, `0x1002d520`)

- **A place** (`TARGET_BY_PLACE`, `0x202`) sets the place flag `+0x64` and
  copies the place, x, y and z, to `+0x88`. The radius (`+0x68`, and the same
  value at `+0x70`) is `Patrol_Place_Radius`, **unless the order's parameter is
  neither 0 nor −1**, when it is the parameter. The speed figure `+0x6c` is
  `Patrol_Place_SpeedPercent`.
- **A building** (a logic id with bit 31) sets `+0x58` and keeps the id at
  `+0x74`, with `Patrol_Building_SpeedPercent` and the same parameter rule for
  the radius. When two of the unit's own behaviour fields, `+0x18c` and
  `+0x140`, are both other than −1 (neither is named), it instead sets `+0x5c`
  and a speed figure of 20. This page calls that the patrol inside a building,
  a *guess* from what its takt does.
- **A unit** (any other logic id) sets `+0x60`, keeps the id at `+0x94`, with
  `Patrol_Unit_Radius` (or the parameter) and `Patrol_Unit_SpeedPercent`.
- Any other target kind logs *"\*\*\* Task_Patrol has incorrect target"* and is
  refused.

### The loop of points (`0x1002dd90`)

The start (slot 6, `0x1002d7c0`) arms three randomised timers, (fixed, random)
seconds: `+0xbc` from the building delays, `+0xc4` from the unit delays and
`+0xcc` from the place delays, or 20/0, 10/0 and 10/0 while `DeterminMode`
(`0x10066bc0`, `Behavior.ini`, 0 as shipped) is set. It then draws the loop,
and a draw that fails fails the start. Unless the patrol is inside a
building, it walks to the loop's first point at the unit's speed (`+0x5fc`) ×
the speed figure (`0x10001960`), and asks the fire control for mode 2, the
nearest hostile contact ([The fire control](#the-fire-control--read)).

The draw, by target, into a list of 12-byte points (`+0xa4`, index `+0xb8`
set to 0):

| target | how many points | where | timer restarted |
|---|---|---|---|
| a place | **15 + (16-bit random % 5)**, 15–19 (`0x1002de5f`) | the place ± radius on x and y, each `rand()`/32767 × 2 − 1 | `+0xcc` |
| a unit | **3 + (random % 5)**, 3–7 (`0x1002e246`) | the unit's position ± radius | `+0xc4` |
| a building | one per vertex of its contour (property `0x203`) | each vertex pushed out from the contour's centroid by `Patrol_Building_Radius`, 30 — the block's constant, not the order's radius (`0x1002e105`) | `+0xbc` |

- **A place or unit point** is tried up to 350 times (`0x1002df68`,
  `0x1002e354`). It must lie inside the map's box less 100 on every side
  (`0x100595bc`), and, unless the chassis profile's `CanFly` is set, on an areal
  the system areal map calls usable. The last try is kept even when none
  passes, so these draws never fail.
- **A unit patrol** fails, logging *"Unit has been destroyed... PatrolUnit
  failed"*, when the areal map no longer holds the unit or it is not of the
  patroller's own clan — **Guard works only on one's own clan's units**.
- **A building patrol** fails when the building is gone (*"Building has been
  destroyed... PatrolBuilding failed"*) or has no contour.
- The 16-bit random is the pair of words at `0x10066bfc`: x ← (2x) xor y, then
  y ← (y ≫ 1) xor x, and y is the result.

### Each tick (slot 7, `0x1002d900`)

1. **A small unit guarding another clan's building** captures it first
   ([What each package does](#what-each-package-does--read)): queue the
   capture, the escape and the patrol again, and end.
2. **Inside a building** (`+0x5c`): ask for fire mode 2, clear the walker, and
   stay.
3. **The timer** of the target's kind (`+0xc4`, `+0xcc` or `+0xbc`), while
   `DeterminMode` is 0: when it has run out, draw a new loop and walk to its
   first point (a failed draw ends the task).
4. **Otherwise, the next point**: a place or unit patrol moves on when the
   walker is idle (`0x1003dd80`); a building patrol when the walker's
   `0x1003ddc0` holds and `0x1003ddb0` is below 2. The index becomes
   (index + 1) % count (`0x1002dcdf`), and `MWalker::SetTarget` is handed the
   point at the unit's speed × the speed figure, with 0, −1 and 5.

**It never ends on its own.** Only the capture branch, a failed draw, or an
order that replaces it takes a patrol off the stack. The speed figures are held
by the walker ([How a walk's speed is held](#how-a-walks-speed-is-held--read)):
0.8 of top speed for a place, full speed for a unit, and full speed for a
building's 80.

### What it scores (slot 13, `0x1002d390`) and lets through (slot 12, `0x1002d250`)

**The score.** For each hostile contact the engagement asks the patrol
(`0x10017e70`). It scores 0 unless the contact lies **within the radius of the
patrol's centre**, across the ground and strictly inside (`0x1002d4a1`). The
centre is the place, the building's position, or the unit's position with the
distance × 0.7 (`0x10059968`). Inside, the score is the default's formula:

    (2 − (a + 1) ÷ (b + 1)) × (c × b + 10) ÷ (d + 10)

where *d* is that distance and *a*, *b*, *c* are the clan areal map's record
of the contact at `+0x20`, `+0x18` and `+0x1c` (slot 9, `ArealMap.dll:0x10001ab0`):
the life the contact has **left**, the life it would have **whole**, and its
guns' rate, all cached by the refresh at `0x10006fdf`
([15-behaviour.md](15-behaviour.md#what-a-strength-is--read-and-measured)). So
`2 − (a + 1) ÷ (b + 1)` is 1 against an untouched target and rises towards 2 as
it is shot apart, and `c × b + 10` is how much gun a whole one brings: a patrol
takes up the wounded and the dangerous first. The default score (`0x10001010`), which most tasks keep, measures
*d* from the unit itself and cuts off at 500. **So a patrolling unit takes up
an engagement only against what comes inside its patrol ground**: a place
patrol of radius 60 ignores a hostile 61 m from its place, however close to the
unit on its way there. Its fire control still shoots at the nearest hostile
within 500 all along.

**The priority** is 1 for every reason but refit (3) and 4, which take the
base's (`0x100018a0`). Beside it the patrol hands the new attack a **limit**
(jump table `0x1002d374`):

| patrol | reasons 0 (engage) and 1 (retaliate) | reasons 2 and 5 (call for help) |
|---|---|---|
| a place | a circle about the place, radius + 60 (`0x10059a68`) | the same |
| a building | within radius + 80 of the building (`0x10059a8c`) | the same |
| a unit | within radius + 60 of the unit | within radius × 1.3 + 78 of it (`0x10059a98`, `0x10059a94`) |
| inside a building | none | none |

### A task's limits — *read*

Every task keeps a 28-byte **limit** at `+0x2c`, which slot 4 (`0x100014c0`)
copies in: a time in seconds (`+0x2c`), a circle's centre and radius (`+0x30`,
`+0x3c`), and a unit's logic id and a radius (`+0x40`, `+0x44`). Slot 9
(`0x10001660`) tests it: past its seconds since the task's stamp `+0x48`
(milliseconds × 0.001; who sets the stamp was not traced) it logs *"Time Limit
expired"*; with the unit farther than
the radius from the centre, **in three dimensions**, *"Place Limit expired"*;
farther than `+0x44` from the named unit, *"Unit Limit expired"*. A zero time
or radius, or the id −1, tests nothing.

- **Who gives a limit.** `CreateTaskFromOrder` hands slot 4 the order packet's
  `+0x110` (`0x10034466`), and an engagement hands it the current task's answer
  merged with that task's own limit, keeping the tighter time, circle and unit
  radius where both give one (`0x100179c0`). An order from a script carries none: `ai.dll`'s packet
  (`0x10004330`, `0x100043a0`) starts with no time, no circle and unit −1.
- **The base priority's limit** (`0x100018a0`), which `Task_Stop` answers with
  (slot 12 `0x10031d90`), is 1000 about where the unit stands (`0x1000193e`).
- **What ends a task.** The stack's takt (`0x10034a30`) runs the task's takt,
  then slot 9; if either answers 0 it logs *"Task Ended"*, removes the task and
  starts the next one (`0x10034930`). **An attack that strays out of its
  patrol's circle is dropped, and the patrol beneath it starts again**, drawing
  a fresh loop.

### What a script's four floats do

Function 15's `fSuccess`, `fSurvive`, `fTime` and `fIndependence` (all 0.5 in
`varset.var`, *measured*) go into the packet at `+0x12c`, `+0x130`, `+0x134`
and `+0x138` (`ai.dll:0x100083a6`). `CreateTaskFromOrder` gives a task the
target (`+0xc`), the parameter, `+8` and the limit (`+0x110`) and nothing
beyond (`0x10034413`–`0x1003446f`), so **no task is handed them**; no reader of
them was found.

## Which objects run a behaviour — *read*

`MBehaviour::takt` (`0x10004c40`) runs for every object that has a behaviour,
of every clan, in the same order:

1. the default order and killed-flag bookkeeping, the live limits and the
   building's place tick (`0x10018ac0`);
2. **then it returns for a neutral clan** (type 3, `0x10005070`): a neutral
   object runs no radar module, no unit or building takt and no fire control. A
   neutral unit takes no order through its stack, and a neutral building's guns
   never fire;
3. the radar module (`0x100234e0`);
4. a building's takt (`+0xe4`), or, with flag `0x10`, a unit's (`+0xe0`),
   which runs the task stack and, every so often, the refit and engagement
   checks;
5. with flag `0x40`, unless the current task is order 18's construction sphere,
   the fire control (`0x10023ff0`).

**Every behaviour starts with flags `0x10`, `0x20` and `0x40` on**: the
constructor ORs `0xff8` into its flags (`0x10003c95`). An object that answers
no interface `0x201`, its machine at `+0x70` — a building — has `0x10` and
`0x20` taken off again as its behaviour attaches (`0x10005d59`), and keeps
`0x40`. Otherwise only the wizard turns them off, for a unit the player takes
([The escape](#the-escape--read)). So in single player **every enemy unit
thinks as the player's own do**, and a building of a non-neutral clan aims and
fires its guns through the same fire control. The radar module lists no
buildings among its contacts ([25-sensors.md](25-sensors.md)), so that fire
control shoots only at units.

- **With no order**, a unit's stack answers with its embedded task, a
  `Task_Stop` (vtable `0x100596ac`, made at `0x1000fa00`): fire mode 2, the
  default score within 500, and the base priority with its 1000 limit. **An
  idle unit engages whatever hostile unit comes within 500**, and its attack
  may take it 1000 from where it stood. A new warbot's escape ends that way
  ([36-factory.md](36-factory.md)).
- **Shut down** (order 19, vtable `0x10059eec`): its start resets the fight
  module and asks for fire mode 0; its takt asks again and clears the walker;
  its priority is 0 for every reason, retaliation included
  (`0x10031b10`). A shut-down unit neither moves, aims, fires nor answers fire.

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

## Mission 03's last battle — *measured*, *read*, *derived* and *seen*

*The Field Base* ends with a fight the scripts stage: three enemy flyers are sent
at the base once the player has built four warbots, and objective 5 completes
when the enemy clan has no robots. The scripts' tests and messages are
[34-progression.md](34-progression.md)'s; what the units do is this page's.

### What the scripts give — *measured*

- **The enemy starts shut down.** `tut3_en`'s `Init` gives logic ids 3, 4 and
  5 `ORDER_ROBOT_SHUTDOWN` (19), replacing. They are `Enm`'s three units on the
  plateau, `tut3_f1`–`f3` at about (1870, 1880), on ground 245 m up. A shut-down unit
  neither moves, aims, fires nor answers fire
  ([Which objects run a behaviour](#which-objects-run-a-behaviour--read)).
- **The neutral clan does nothing.** `tut3_nt` only reads its base, and `Ntrl`
  is clan type 3, so its Small Bunker and Small Generator run no radar, takt or
  fire control until they are captured.
- **The player's script sends the enemy.** When objective 4 completes,
  `tut3_pl2` gives `ORDER_ROBOT_PATROL` (4), `INSERT_ORDER_REPLACE`,
  `TARGET_BY_PLACE`, parameter `d0` = 0:

  | id | unit | place | from its start | ground under the place |
  |---:|---|---|---:|---:|
  | 3 | `tut3_f1` | (1124, 783) | 1,326 m | 84.1 m |
  | 4 | `tut3_f2` | (606, 993) | 1,549 m | 90.6 m |
  | 5 | `tut3_f3` | (1124, 783) | 1,331 m | 84.1 m |

  (1124, 783) lies 140 m from the Small Bunker; (606, 993) lies 74 m from the
  Small Generator and 679 m from the bunker. Function 15 finds a unit of any
  clan by logic id: the clan areal map's slot 7 (`ArealMap.dll:0x10001a40`)
  hands the id to the system map's (`0x10021020`) with no clan test.
- The four floats are `varset.var`'s 0.5 each, and no task reads them
  ([What a script's four floats do](#what-a-scripts-four-floats-do)).

### The two sides — *measured*

The enemy, all **S-2f** flying chassis (`R_L_02`, 160 km/h, 190 HP body, a
270 HP hung turret), each with a 300 m sensor module:

| id | unit | guns | per second |
|---:|---|---|---:|
| 3 | `tut3_f1` | Small Autocannon S75Can, 350 m, 400 rounds | 600 |
| 4 | `tut3_f2` | Small Red Laser SRLs, 1,000 m | 336 |
| 5 | `tut3_f3` | two Small Flame Throwers SFT, 140 m, 60 rounds each | 420 |

The player's side:

- **The Small Bunker** (`sbunk01.dat`, Type `0x80010000`), neutral until
  captured: a turret `e_bnt_lt_01` carrying two Huge Flame Throwers HFTB
  (250 m, 790 a round, 0.5 a second) and `e_gun_fs_12`, which despite its name
  is a sensor module of range 500 (`o_bnt_rdr_l_01.ctl`, class 8), with a shield
  generator and a repair unit on the building.
- **The prebuilt designs** ([34-progression.md](34-progression.md)):
  `tut3_p1`, an S-12w walker with two S75Can (1,200 a second, 350 m), and
  `tut3_p2`, an S-31 on wheels with two guided Small Missile Lr SML4S (544 a
  second, 350 m); both carry 300 m sensors.
- The builder `tut3_b` and the transport `tut3_t` carry no gun.

### What follows — *derived*

1. **The patrols.** Each flyer draws a loop of 15–19 points within 60 m of its
   place on x and y and flies to the first at 0.8 of its speed. That is its
   live top speed, 17.84, 20.03 and 18.62 m/s for `f1`, `f2` and `f3` once
   engine and load are counted
   ([24-motion.md](24-motion.md#what-shipped-units-get--measured-then-derived)),
   so 14.27, 16.02 and 14.90 m/s. **It flies 15 m over the ground**: every walk
   the patrol asks for builds its place with the word that has the walker raise
   each point ([24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)),
   the loop's own `SetTarget` (`0x1002dd74`) included. Over Tut_3 that way is
   longer than the straight line — 1,407 m for `f1`, 1,419 for `f3`, 1,716 for
   `f2`, 6.2 to 10.8% more, on 20 m cuts over the landscape (*measured*) — so the
   flight takes about 99, 95 and 107 s. (This page once took the chassis's
   authored 44.4 m/s, and 37 s and 44 s.) Every 20–30 s it draws a new loop about
   the same place, and it never stops patrolling.
2. **On the way it only shoots.** Its fire control, mode 2 from the patrol's
   start, aims at the nearest hostile unit its radar lists within 500 — within
   its 300 m sensor. It takes up no engagement: nothing scores unless it is
   inside 60 m of its place.
3. **Hit on the way, it barely turns.** Retaliation (and a call for help from a
   wingman within 400) inserts an attack with the patrol's circle, radius 120
   about the place — and a script's place stands at z 0, the limit testing in
   three dimensions. Anywhere on its way in the flyer is hundreds of metres
   outside that sphere, so the attack is dropped on its first stack takt and the
   patrol starts over with a fresh loop.
4. **At its place the limit holds, mostly** (*measured*). Flying 15 m over ground
   84.1 m and 90.6 m high, the flyer stands 99 m and 106 m above z 0 at the two
   places, inside the sphere out to about 68 m and 57 m across the ground. Of a
   1 m grid over the square its loop is drawn from, 13,464 of 14,641 points
   (92.0%) lie inside the limit about (1124, 783), where the ground falls away,
   the farthest 73.6 m out; about (606, 993), 7,601 (51.9%), and 3,059 (20.9%)
   lie over the Small Generator, whose faces put a point 115 m over their top
   (the ground routine's first query; its inner ring gives the same 3,059). So
   there it circles its loop and takes up an attack on a hostile unit inside 60 m
   of the place, and keeps it while it stays inside the sphere: over most of the
   bunker-side square, over half the generator-side one. The attack's own moves,
   50–100 short of the target and up to 80 to the side, are flown 15 m over the
   ground as well ([24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)),
   and one that carries it out is dropped.
5. **The defence.** A player warbot with no order engages any hostile unit
   within 500 (its radar permitting) and may chase it 1000 from where it stood;
   on Standby it only shoots; guarding, the patrol's rules apply about its own
   ground. A captured bunker aims at the nearest hostile unit within its 500 m
   sensor and fires the HFTB inside 250 m: (1124, 783) is in reach, (606, 993)
   is not.
6. **The end.** Objective 5 completes on the Mission handler run after clan 1's
   robot count reaches 0 ([34-progression.md](34-progression.md)).

### Seen in a recording

In the 960 × 720 recording of *The Field Base*:

- **The new warbots patrol.** SSW-4 reads *"[escaping]"* on the Battle units
  page from 264 s to 295 s and *"[patrolling]"* by 296 s, while the player
  points at the satellite map and the Small Generator's mark (292 s); SSW-5 to
  SSW-8 go the same way, and all five read *"[patrolling]"* by 360 s. No code
  read gives a new bot a patrol (the factory gives only the escape,
  `0x1002aa6e`), so these are the player's Guard orders, given on the map
  (*derived*; ordering from command mode is
  [40-command-mode.md](40-command-mode.md)'s).
- **The patrol comes in at the read speed** (*measured* off the satellite map).
  The map on the command panel is north up; fitted by least squares to five
  building icons (the generator, the mine, the warehouse, the bunker and the
  factory, residuals within 4.5 px), it draws 0.193 px a metre across and 0.199
  down, about 5 m a pixel. From 396 s it shows two groups of red flyer crosses:
  - **a pair**, 8–10 px apart, on the line from the plateau to (1124, 783) to
    within 19 m: 760 m along it at 396 s and 1,065 m at 420 s, 12.7 m/s across
    the ground, against 13.1 and 13.8 m/s for `f1` and `f3` at 0.8 of their live
    top speed along the ground + 15;
  - **one** on the line to (606, 993) to within 19 m: 931 m along at 404 s and
    1,158 m at 420 s, 14.2 m/s, against 14.4 m/s for `f2`.

  Both trail the prediction from the orders at 334 s by 58 to 81 m, 4 to 6 s of
  flight, which the start from a standstill would take up (*guess*). **At 418 s, when the fight starts, the pair is some 275 m short of its
  place** and about 195 m from the bunker, inside the HFTB's 250 m, and the
  single flyer about 355 m from the generator. So the patrol took 84 s to the
  fight because its flyers fly at 0.8 of their *live* top speed along the ground
  + 15 and met the base's defence well before their places; nothing is missing
  from the 85 s.
- **The fight.** At 420 s the cursor over the base's marks at the top of the
  map shows *"SFW-2 Warrior [patrolling]"* — an enemy flyer, reading its patrol.
  From 418 s to 426 s explosions and two burning flyers fill the lower left of
  the command camera's view over the bunker, which fits the place (1124, 783)
  140 m from it (*guess*). *"MISSION COMPLETE !"* is up at 429 s and the
  campaign menu at 432 s. Who fired cannot be told from the view.

### For an engine

- Run every unit's and building's behaviour, of every clan but a neutral one
  (clan type 3), which runs none; a building runs its radar, its building takt
  and its fire control, not a unit's takt.
- **Shutdown**: no movement, fire mode 0, ignore every interrupt.
- **Idle** (no order): fire mode 2; engage the best-scoring hostile unit within
  500; the attack's limit is 1000 about where the unit stood.
- **Patrol by place**: 15 + (0..4) points uniformly in the square of the radius
  (the parameter, or 60) about the place, inside the map less 100, a walker's on
  usable ground; walk them in turn at 0.8 × speed, advancing when the walker is
  idle, a flyer 15 m over the ground under each; draw a new loop every
  20 + U(0, 10) s. **By unit**: 3 + (0..4) points
  about the unit, own clan only, full speed, every 5 + U(0, 10) s. **By
  building**: its contour's vertices pushed out 30, full speed, every
  60 + U(0, 60) s.
- **A patrol's engagement**: score only contacts inside the radius of its centre
  (× 0.7 of the distance for a unit); limit the attack to radius + 60 about the
  place in three dimensions, with the place's z as given (0 from a script), or
  radius + 80 of a building, radius + 60 of a unit; drop an attack that leaves
  its limit and restart the task beneath it.
- **Fire control** (mode 2) runs whatever the task, units and buildings alike,
  on the radar's hostile units only.
- Mission 03: the three enemy flyers above, sent by `tut3_pl2`; the bunker's two
  HFTB and 500 m sensor once it is the player's.

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
block's, and every behaviour holds the block's default, 1: `diff_slow.var`'s 0.7
is never loaded
([26-damage.md](26-damage.md#the-difficulty-block-every-behaviour-holds--read-and-measured)).
**A figure above 1 is therefore the same as 1**, and every task's walk is capped
at the unit's own speed.

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
- ~~What a fire-control request's third value, 0.5, does, and what sets `+0x5c`
  and `+0x60` to lock a unit's fire mode.~~ Nothing, and nothing: 0.5 is the
  constructor's own `+0x30`, which every request rewrites and no reader ever
  reads, and neither lock has a reachable writer, so all 20 requests stand
  ([The fire control](#the-fire-control--read)).
- ~~Who sends `MBehaviour` messages `0x19` and `0x1a`, the explosions that start a
  retaliation.~~ Every hit's first step sends `0x19` with the firer's id, and so
  does a round passing through a shield; `0x1a` is never sent
  ([A hit pulls a unit in](#a-hit-pulls-a-unit-in--read)).
- Whether fire mode 1's target fetch (`0x100241d8`, from the system map) consults
  the radar when it aims or fires;
  ~~who writes the clan attitude's increase field~~ — **nobody**, in any of the
  sixteen modules, with the decrease field's own writer as the control
  ([25-sensors.md](25-sensors.md#what-moves-an-attitude-being-shot-and-nothing-else--read));
  what `IGameObject` slot 21, the test that the firer still exists, answers; ~~how
  high a flying medusa holds against its attack's three-dimensional circle~~ —
  **read** for its walk points: 45 to 95 m over the ground, each drawn afresh
  ([24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)); how
  near it comes to them is the motion's, and a grazing medusa's is its hover.
- ~~What a unit record's `+0x30` is. The wingman menu lets only 1 or 2 capture, the
  same records speak `_S` voices, 4 and 5 speak `_B`, and boarding wants 4~~ —
  **read**: the size class. One bind, `iron3d.dll:0x1007e3c0`, writes it for a
  unit's record and a building's alike, from what the object answers for `0x201`,
  `MBehaviour`'s `+0x960`, the size letter of its root name
  ([38-designs.md](38-designs.md#the-catalogue--read-and-measured),
  [27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)).
- ~~The two labels a wingman line draws beside its number (`iron3d.dll:0x10077120`).~~
  Answered: the unit's two icons and its name over its life
  ([The wingman menu from first person](#the-wingman-menu-from-first-person--read-and-measured)).
  Still open: what the view's `+0x50` object does on slots 7 and 8 as the selector
  opens and closes. It does not hide the lines.
- Whether the wingman menu's lines take mouse clicks, as the HQ menu's do; string
  3044, "Use the mouse to click on the item", is not tied to either.
- Whether a digit that picks a wingman or an order also reaches `World3D.dll`'s
  input table, which toggles the hero's guns on the same keys. The character
  handler takes the key, but the table reads DirectInput separately.
- ~~What the order packet's `+0x110`, `+0x120`, `+0x124` and `+0x138` (1.0)
  mean.~~ `+0x110` is the task's 28-byte limit: `+0x120` its circle's radius,
  `+0x124` its unit id; `+0x12c`–`+0x138` are a script's four floats, `+0x138` defaulting to 1.0,
  which no task is handed ([A task's limits](#a-tasks-limits--read)). How the
  unit record's `+0x44` object inserts an order into the behaviour's list is
  still not traced.
- ~~What a patrol does tick by tick, and what a script's success, survive, time
  and independence floats do.~~ [The patrol, tick by tick](#the-patrol-tick-by-tick--read).
- ~~Which units run their behaviour.~~ Every object of a non-neutral clan
  ([Which objects run a behaviour](#which-objects-run-a-behaviour--read)).
- ~~The clan areal map's contact record (slot 9, 40-byte records): what its
  `+0x18`, `+0x1c` and `+0x20`, which every engagement score weighs, are.~~ The
  life a contact would have whole, its guns' rate, and the life it has left —
  `ILifeSystem` properties 54 and 38 with device query 6 between them, cached
  by `ArealMap.dll:0x10006e40`
  ([15-behaviour.md](15-behaviour.md#what-a-strength-is--read-and-measured)).
- ~~How high a flyer on patrol flies, which decides whether a script patrol's
  circle about a place at z 0 ever holds its attack.~~ — **read**, and
  **measured**: 15 m over the ground under each point, as every walk the patrol
  asks for builds its place with the word that has the walker raise it
  ([24-motion.md](24-motion.md#a-flyers-walk-points--read-and-measured)); so on
  Mission 03 the attack holds near the place, over 92.0% of the loop's square
  about (1124, 783) and 51.9% about (606, 993), and is dropped anywhere on the
  way in ([What follows](#what-follows--derived)). ~~Why the recording's patrol
  took about 85 s to the fight~~: its flyers come in at 0.8 of their live top
  speed along the ground + 15, measured off the satellite map, and meet the
  base's defence some 275 m short of their place
  ([Seen in a recording](#seen-in-a-recording)).
- The two behaviour fields, `+0x18c` and `+0x140`, that make a building patrol
  hold inside a building, and the walker tests `0x1003ddc0` and `0x1003ddb0` a
  building patrol moves on by.
- What the factory's object slot 47 does with a new bot's logic id
  (`0x1002ab02`).
