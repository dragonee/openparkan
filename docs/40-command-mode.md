# Command mode — the bunker's camera and the commander's view

What the player gets by taking a bunker: a free camera over the base, the
commander panel with its icon column, and from there every unit and building.
*The Field Base* (`CAMPAIGN.00/Mission.03`) teaches it. Its tip T03_H062 says
the camera "can move in a limited radius around the Bunker", turns when the
cursor is at a side of the screen, moves on the arrows and climbs on PageUp and
PageDown. This page reads how. Capture itself is
[27-ownership.md](27-ownership.md#capture--read)'s. The panel's pages are
[41-commander.md](41-commander.md)'s. Selecting units and giving orders with the
cursor, in the world and on the map, is
[42-selection.md](42-selection.md)'s. Placing a building is
[32-builder.md](32-builder.md)'s.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured* is
re-derived by `openparkan verify` (`check_command_mode`), *read* comes from the
disassembly at the address given (all `iron3d.dll`), *derived* follows from
those, *seen* is the recording
`training mission 3   The field base [DW8XuX10y0U].mkv` (960 × 720), and
*guess* fits the evidence without being established.

## Three words: the mode, the view state and the camera — *read*

**The mode stack** is the interface's `CState`
([39-boarding.md](39-boarding.md#the-game-view-keeps-a-stack-of-modes--read)).
Command mode is two of its modes, and a third sits on top of them:

| mode | what | its record holds |
|---:|---|---|
| 2 | **telepresence**: driving a unit from command mode ([below](#telepresence-mode-2--read)) | the unit |
| 3 | an HQ unit's command view ([30-turrets.md](30-turrets.md#an-hq-unit-in-play--read)) | the HQ unit |
| 4 | **a bunker's command view** | the bunker |

**The level's view state word** (`+0x710`) picks the view the world is drawn
from (`0x100a1c30`, table `0x100a1c7c`):

| view state | view | who sets it |
|---:|---|---|
| 1 | the driven unit's camera (`0x1007e6a0` on `+0xaec`) | on foot, in a bot, telepresence |
| **2** | **the command camera**, the level's `+0x104` (the camera object at `+0x100`) | modes 3 and 4 |
| 3 | the outer camera, `+8` | `CMD_JAMES_OUTER_CAMERA` |
| 4 | a second camera of the same class, `+0x6c` (at `+0x68`) | not read |
| 5 | the briefing's | [21-briefing.md](21-briefing.md) |
| 6 | the building's camera (`0x1007e6a0` on `+0xaf0`) | a tower's screen |

**The command camera** is a `+0x100` member of the level, one class
(`0x10036d60`–`0x10037dd0`) with two instances: `+0x100` for state 2, `+0x68`
for state 4. Its fields:

| field | what |
|---|---|
| `+0x08` | its view (World3D object type 5, interface 8) |
| `+0x0c`, `+0x10` | the field of view now, and its widest |
| `+0x1c`–`+0x24` | position x, y, z |
| `+0x28`, `+0x2c`, `+0x30` | roll, pitch and yaw handed to the view |
| `+0x34` | zoom on |
| `+0x44` | an HQ unit it follows (mode 3) |
| `+0x48` | **the building it is held around** |
| `+0x4c` | the tilt: 0 straight down, π/2 level |
| `+0x50`, `+0x54`, `+0x58` | velocity sideways, forward and up |
| `+0x5c`, `+0x60` | tilt rate and yaw rate |
| `+0x64`–`+0x74` | per axis, the time of the last update |
| `+0x78`–`+0x88` | per axis, the time its key or edge last changed |
| `+0x8c`–`+0x91` | the six keys held: left, right, forward, backward, up, down |
| `+0x94`–`+0x97` | the cursor at the right, left, bottom and top edge |

## Entering — *read*

**A bunker's pod opens it.** The pod's same-clan callback
([27-ownership.md](27-ownership.md#capture--read), `0x10062630`) selects the
building (`0x1007db80`), closes the wingman menu, and sends the three bunker
Types, `0x80010000`, `0x80020000` and `0x80040000`, to
`0x10062bc0(4, building, 0)` (`0x10062779`–`0x10062797`). Because a capture
always ends in that callback, **taking a neutral bunker from its pod opens
command mode at once** (*derived*).

**Nothing else pushes mode 4 from on foot** (*measured*: the twelve calls of
`0x10062bc0` and the mode each pushes). The one other push of 4 is the
commander panel's bunkers page while in mode 3, an HQ unit's view
(`0x100862b3`–`0x100862d6`). Enter, `CMD_ENTER_STATE` (730), does nothing in
view state 2 (`0x10071f08`: it acts only in view states 1 and 3, or from mode 1
or 2). So **the way back into command mode after leaving is the pod again**,
which fires only after its occupant has left and 5 s have passed
([27-ownership.md](27-ownership.md#capture--read)) (*derived*).

**A mode already on the stack is not pushed twice.** Before pushing,
`0x10062a40` looks through the stack for a record of the same mode (the same
building for 4, the same unit for 2 and 3). If it finds one it rolls the stack
back to it (`0x10062ce0`) and pushes nothing.

**Mode 0 → 4** (`0x10063ca0`, table `0x10104b18` entry 4), in order:

1. finds the player's hero record (Type mask `0x1020000`);
2. turns the outer camera off (`0x10038ad0`);
3. clears the keyboard: `World3D.dll`'s `stdClearKeyboard` drops every pending
   key and mouse message ([39-boarding.md](39-boarding.md#boarding--read));
4. clears four globals (`0x1010bf7c`–`0x1010bf80`, not followed);
5. **lets the hero go** (`0x10074ff0` with 0): its `+0xa2` cleared and its
   overrides all off, so it stands where it is, with no AI
   ([31-packages.md](31-packages.md#the-escape--read));
6. **holds the camera around the building** (`0x10037da0`: `+0x48` = the
   bunker, the velocities and the up and down keys cleared);
7. clears the driven unit (`+0xaec`) and makes the bunker the view's building
   (`+0xaf0`);
8. **sets view state 2**;
9. **places the camera at the bunker's record position** (`+4`, `+8`, `+0xc`)
   with roll 0, pitch 0 and **yaw π/2**, facing +y, north, whatever way the
   bunker faces (`0x10036ae0`). The tilt `+0x4c` and the zoom are not touched;
10. turns the commander panel to page 0 if it is on another (`0x10084d80`).

**The world is not paused**: nothing in the transition pauses it (*derived*).
The recording's builder walks while the camera watches.

## The camera — *read*

**Each frame** the level updates the camera of its view state
(`0x100a55c0`): state 2 runs `0x10037a50` on `+0x100`. That update:

1. works out the edge turn rates from the edge flags (`0x100373d0`);
2. steps the zoom;
3. follows an HQ unit in mode 3 (`+0x44`; not Mission 03's);
4. sets the four edge flags from the cursor;
5. works out the velocities from the keys (`0x10037130`);
6. moves, turns, clamps and hands the frame to the view (`0x100375b0`).

**Time** is the services timer's: slot 2 the time now in ms, slot 3 the seconds
since a stamp ([35-hud.md](35-hud.md#the-message-box--read-and-measured)).
Every rate below is per real second.

### It starts where it was made

The level's set-up makes both cameras (`0x10036d60`, from `0x100a29c8` and
`0x100a29da`):
- the view: **field of view 1.04 rad, near 3, far 700**
  (`0x100364a0`; the parameter block's `+0xc`, `+0x10`, `+0x14`), so `+0x0c`
  and `+0x10` are 1.04;
- **the tilt `+0x4c` = 1.0**, or 1.25 while the game's byte `+0xe5` is set.
  That byte also makes Esc quit the game (`0x10070e03`), so it is clear in
  ordinary play (*derived*);
- yaw π/2, everything else 0, and the follow distance `+0x38` 30.

### Keys set velocities, which ramp up and die away

`CMD_JAMES_HQ_MOVE_*` reach the game's command handler (`0x10071cd0`). In view
state 2 each sets its flag on the camera at `+0x100` (in state 4, on `+0x68`),
and the key's release clears it (`0x10072740`):

| command | key (`addition.man`) | flag | velocity |
|---|---|---|---|
| 723 `MOVE_LEFT` | keypad ← | `+0x8c` | sideways −S |
| 724 `MOVE_RIGHT` | keypad → | `+0x8d` | sideways +S |
| 727 `MOVE_FORWARD` | keypad ↑ | `+0x8e` | forward +S |
| 728 `MOVE_BACKWARD` | keypad ↓ | `+0x8f` | forward −S |
| 725 `MOVE_UP` | PageUp | `+0x90` | up +S/2 |
| 726 `MOVE_DOWN` | PageDown | `+0x91` | up −S/2 |

- **S is 125** (m/s), or 80 while `+0xe5` is set (`0x10037141`, `0x1003714b`).
- **A flag stamps its time when it changes.** While held, with t the seconds
  since the key went down, the speed is 2t × S until t passes 0.5 s, then S.
  Up and down reach S/2 the same way: t × S for the first half second, then
  S/2. Left wins over right, forward over backward and up over down.
- **With neither key of a pair held**, a velocity that is not 0 is multiplied
  each update by (0.5 − dt), dt the seconds since that axis last moved the
  camera, and set to 0 once dt reaches 0.5. At 60 updates a second that is
  × 0.48 an update (*derived*): the camera coasts to a stop within a few
  frames, faster at a higher frame rate.

**The move** (`0x100375b0`), with θ the yaw `+0x30` and dt the seconds since
that axis last moved:
- sideways: x += dt·v sin θ, y −= dt·v cos θ;
- forward: y += dt·v sin θ, x += dt·v cos θ;
- up: z += dt·v.

So at yaw π/2 forward is +y, north, and right is +x, east (*derived*).

**Each coordinate is kept above a margin.** A new x (or y) is taken only while
it is **above tan(½ field) × z** and **below the map's side** (the land
record's `+8`, [35-hud.md](35-hud.md)); otherwise that coordinate stays. The
lower bound grows with the camera's height z, not its height above ground
(*read*; at z 120 and field 1.04 it is 69 m, *derived*).

### The cursor at an edge turns and tilts it

The cursor's position (`0x1010414c`, `0x10104150`) is divided by the display's
scale (`services.dll` `IDisplay` slots 4 and 5) into the 640 × 480 layout
(`0x10037c5f`):

| cursor | flag | rate |
|---|---|---|
| x ≤ 6 | `+0x95`, left | yaw +R |
| x ≥ 634 | `+0x94`, right | yaw −R |
| y ≤ 6 | `+0x97`, top | tilt +R, up toward level |
| y ≥ 474 | `+0x96`, bottom | tilt −R, down |

- **R is 1.5 rad/s**, 0.5 while `+0xe5` is set, and **0.2 while zoomed**
  (`0x100373d4`).
- The rate ramps and dies away as the keys' velocities do: 2t × R for the
  first half second, then R, and × (0.5 − dt) once the cursor leaves the edge.
- The edge flags are not updated while cursor 7 is up (`0x10104148`,
  [42-selection.md](42-selection.md)).
- **The yaw is not limited.** **The tilt is kept strictly between 0 and π/2**:
  a step that would reach either end is not taken (`0x10037835`).

### Height: 36 to 236 over what is below

After moving, with h the highest landscape or building surface at the camera's
x, y (`0x100a14d0`, mask 2 | 8; in view state 4 scenery and units too):
- **z below h + 36** is raised to h + 36;
- **z above h + 236** is lowered to h + 236.

So the height follows the ground as the camera moves, and a move over a roof
lifts it (*derived*). PageUp and PageDown move it within that band.

### The box around the bunker

With a building held (`+0x48`), **x and y are kept within 200 of the
building's record position**: an axis further than 200 (its difference rounded
to an integer) is set to the building's ± 200 on its side
(`0x100378a6`). So "a limited radius" is a square 400 m across
(*read*), and each axis is held separately.

*Measured*, on Mission 03: the Small Bunker (`sbunk01.dat`, Type `0x80010000`,
clan 2, neutral) stands at (1260.9, 813.9). The mission's one lode is
(−234.8, +128.8) from it, 35 m west of the box's edge, so the camera can look at
the lode but not stand over it (*derived*). The builder `tut3_b` at
(1106.6, 914.4) is inside the box.

### Zoom

`CMD_JAMES_ZOOM_MODE` (738, Z) in view state 2 toggles `+0x34`, but only while
the field is at one end: at most 0.2, or at least its widest
(`0x1007244a`). Each update then steps the field by 0.1 toward
0.2 when zoomed, or back toward the widest, stopping once past the end
(`0x10037b84`), and hands it to the view (interface `0x12` slot
10). It is per update, not per second.

### The frame

The pitch handed to the view is **π/2 − tilt** (`0x10037943`). The matrix
(`0x10037943`, as `0x10036ae0` builds it) is R_z(yaw) · R_y(pitch)
· R_x(roll) with the position in its last column, and it goes to the view's
slot 7. Its first column, (cos yaw cos pitch, sin yaw cos pitch, −sin pitch),
is the look, as in the unit cameras
([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured))
(*derived*). So:
- tilt 1.0 looks 32.7° below the horizon, tilt 1.25 18.4° below (*derived*);
- taking the field across the frame, as the briefing's is
  ([21-briefing.md](21-briefing.md)), a 4 : 3 frame's vertical half-field is
  23.2°, so the first view shows from 9.5° to 56° below the horizon and no sky
  (*derived*; the recording agrees, below).

## What command mode draws — *read*

The screens' draw (`0x1008d200`) switches on the mode; **modes 3 and 4 share
`0x1008d51c`**:

1. **The system cursor is shown** (`ShowCursor` until it counts up), when the
   display's slot 12 answers.
2. If the warbot designer is open, only the designer is drawn
   ([37-designer.md](37-designer.md)).
3. Otherwise:
   - the markers over the selected units (`0x1007d5e0`) and buildings
     (`0x1007db00`), [42-selection.md](42-selection.md);
   - **the commander panel with its icon column** (`0x100836f0` with 0 and 0):
     the resource rows, the column, the page header and the page
     ([41-commander.md](41-commander.md));
   - the satellite map in its commander's variant, (374, 63)–(640, 329) with
     its title bar at (374, 43), marking the camera in yellow
     ([35-hud.md](35-hud.md#the-satellite-map));
   - the message box, which **in view state 2 moves to (374, 352), 266 wide**
     ([35-hud.md](35-hud.md#the-message-box--read-and-measured));
   - the game's `+0x30` overlay;
   - the objectives screen, dimmed to 80% in modes 3–5
     ([35-hud.md](35-hud.md#the-objectives-screen)).

**No cockpit HUD** is drawn: that is the path of modes 0–2 and 6
(`0x1008d37f`). While cursor 8 is up, only the resource rows, the map, the box
and the objectives are drawn (`0x1008d326`).

## Input — *read*, and *measured*

**In play the game's commands come from `addition.man`.** The game loads four
binding files into its `+0x28` table, in order `table_1.man`, `table_2.man`,
`hero.man` and `addition.man` (from `0x1005ce90`); each load returns
its group's index (`0x1003ac00`). A window message the interface's handlers do
not take ([39-boarding.md](39-boarding.md)) goes to `0x10071c10`, which looks
its key up in **group 3**, `addition.man` (`0x10071c5a`): a key-up (`0x101`,
`0x105`, `0x202`, `0x205`, or `0x484`–`0x4a3`) goes to the release handler
`0x10072740`, anything else to the command handler `0x10071cd0`. It does
nothing during a briefing or while `+0xe5` is set.

*Measured*: **`ui_hq.man`'s eight rows are all in `addition.man`** (31 rows):
the six camera moves and `CMD_JAMES_BASE_ROTLEFT` / `_ROTRIGHT` on comma and
dot. `ui_hq.man` is named only where the four `ui_*` files are loaded together
(`0x10004434`, `0x10004af3`, `0x10068dd9`; not followed), not on the path of
the play dispatch (*read*).

**What the keys do in view state 2** (the command handler's cases):

| command | in view state 2 |
|---|---|
| 723–728 camera moves | as above |
| 738 `ZOOM_MODE` (Z) | zoom, as above |
| 35 `CMD_CAMERA_INFRARED` (N) | the command camera's infrared, in states 2 and 3 |
| 741, 742 `BASE_ROTLEFT` / `_ROTRIGHT` (, .) | turn the building being placed by 0.05 rad, in the commander's place modes 4 and 6 ([32-builder.md](32-builder.md)) |
| 739 `SATELLITE_MAP` (M), 731 `MISSION_OBJ` (F12), 729 `PAGER` (F2), 749 `HELP` (F1), 748 `GAME_MENU` (F3) | as elsewhere |
| 730 `ENTER_STATE` (Enter) | nothing |
| 735 `ROLLBACK_STATE` (Esc) | **rolls the mode back** ([below](#leaving--read)) |

**The hero's own table does not move it**: it has been let go (step 5 above)
(*derived*).

**Esc** has a second reader. The interface's character handler (`0x10070db0`),
one of the handlers a message meets before the bindings, answers it in this
order, taking the key at the first that applies (from `0x10070dea`):
1. the pause; quitting with `+0xe5` set or once the mission is over
   ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured));
   the help screen; the briefing;
2. an open objectives screen closes;
3. cursor 8 is put away (`0x1008fb00`);
4. a shown message box is hidden (729);
5. the wingman menu closes (740);
6. in mode 0, the game menu opens (748);
7. **in view state 2, an open satellite map closes** (`0x10071027`);
8. **in view state 2, a panel on a page other than 0 turns to page 0**
   (`0x1007104b`).

Otherwise the key is not taken. If this handler sees Esc's key-down, Esc in
command mode peels those back one at a time before 735 leaves; if it sees only
the character that follows the key-down, 735 has already left. Which is not
established ([39-boarding.md](39-boarding.md#not-established)).

## Leaving — *read*

**Mode 4 → 0** (`0x10063d60`, on 735's rollback `0x10062ff0`):

1. finds the hero record and turns the outer camera off;
2. **takes the hero back** (`0x10074ff0` with 1);
3. clears the selection (`0x1007d270`);
4. makes the hero the driven unit, clears the view's building, **sets view
   state 1**;
5. lets the camera go (`0x10037dd0`: the building, velocities and up and down
   keys cleared);
6. clears the `CState`'s `+0x31`, stamps its `+0x2c` with the time, and clears
   the keyboard.

**The hero is where it was, in the bunker's pod**, as after the factory screen
([36-factory.md](36-factory.md#what-the-controls-do--read)) (*derived*). The
camera keeps its tilt and zoom for next time; the next entry puts it back over
the bunker facing north.

**Between command views.** 4 → 4 (another bunker, `0x10063df0`) selects the new
bunker, re-places the camera over it and holds it there. 3 → 4 (`0x10064880`)
and 4 → 3 (`0x100647e0`) do the same between a bunker and an HQ unit. 4 → 4 and
4 → 2 also send the bunker being left interface `0x201` slot 9 with (`0x20`, 1)
and its object message (6, 7, 0), which [31-packages.md](31-packages.md#the-escape--read)
reads as the Wizard's AI mode (*guess* that this hands the bunker's own guns
back to its AI).

## Telepresence: mode 2 — *read*

T03_H07 calls it "manual control of a warbot". **A unit page's three buttons
push mode 2** (`0x100848b0`–`0x10084966`), each for a unit that passes the
boardable test (`0x10076d30`) and whose current order is not 24,
`ORDER_ROBOT_UPGRADE`:

| button rectangle | auto-driver level `+0x9c` |
|---|---|
| `+0x534` | 0: movement, turret and guns the player's |
| `+0x550` | 1: movement to the AI |
| `+0x56c` | 2: movement, weapons and shields to the AI |

The levels are [31-packages.md](31-packages.md#the-escape--read)'s. The
button's layout and art are [41-commander.md](41-commander.md)'s. A fourth
button (`+0x588`) is shown for an HQ unit and pushes mode 3.

**Mode 4 → 2** (`0x10063e90`): clears the selection and selects the unit; sends
the bunker the (`0x20`, 1) and (6, 7, 0) above; **takes the unit**
(`0x10074ff0` with 1); lets the camera go; makes the unit the driven unit,
clears the view's building, **sets view state 1**; clears the `CState`'s
`+0x31`, stamps `+0x2c` and clears the keyboard. The screens then draw mode 2
as on foot: the cockpit HUD for the unit.

**Mode 2 → 4** (`0x10063f30`, on Esc's rollback): turns the outer camera off,
clears the selection and the four globals, clears the keyboard, **lets the unit
go** (with 0), clears the driven unit, makes the bunker the view's building,
**holds the camera around it again without moving it** (`0x10037da0`), **sets
view state 2** and redraws the panel's page. So the camera is where the player
left it.

**There is no mode 2 → 0 or 2 → 1** in the table: telepresence always returns
to its command view. What happens when the unit dies is not read. Nor is there
a 1 → 4: its table entry is 0, and `0x10062bc0` calls the entry without a test,
so the game does not expect a driven bot in a bunker's pod (*derived*; large
bots are not routed to pods, [27-ownership.md](27-ownership.md#capture--read)).

## Selecting and ordering in the world

Clicks in the world and on the satellite map, the cursors, the markers and the
pick modes are [42-selection.md](42-selection.md)'s.

## Against the recording — *seen*

| time (s) | what |
|---|---|
| 166 | the hero shoots the bunker's door and walks in |
| 172–177 | the pod's room; the view goes green as the pod closes |
| 178.20 → 178.23 | **a hard cut, frame to frame, from the pod to the command camera**, on the frame *"from: System / Building is captured"* appears: over the bunker's roof edge, looking down at grass with no horizon; the icon column at the left, the resource rows at the top right with Energy counting up from 1%, the message box at the bottom right, the arrow cursor. The camera does not move in the next half second |
| 180 | the Builders page ([41-commander.md](41-commander.md)) |
| 184–186 | the red, then green, mine outline ([32-builder.md](32-builder.md)) |
| 187.0–188.0 | **the cursor at the bottom edge**: the view tilts from the grass to nearly straight down onto the bunker's roof in about a second, and stays |
| 191.0–192.0 | **the cursor at the top edge**: the view tilts back up to the horizon with the lode's plume |
| 194– | the camera has moved west over the base toward the builder |

The tilt down and back each take about a second: 1 rad at 1.5 rad/s after the
half-second ramp is 0.92 s (*derived*). The recording never leaves command
mode.

## For an engine

1. **Modes.** Keep the stack of [39-boarding.md](39-boarding.md) with mode 4
   (a bunker, its record) and mode 2 (telepresence, its unit). A push of a mode
   whose building or unit is already stacked rolls back to it.
2. **Enter** when the player's hero fires the pod of a bunker of its own clan,
   including the capture's own firing: select the bunker, let the
   hero go (it stands, no AI, not driven), clear held keys, hold the camera
   around the bunker, place it at the bunker's position facing north (yaw π/2),
   show the system cursor, turn the panel to page 0, hide the cockpit HUD, move
   the message box to (374, 352) × 266. Do not pause the world.
3. **Camera state:** position; yaw; tilt (0 down .. π/2 level), 1.0 at the
   mission's start and kept between visits; field 1.04 rad (near 3, far 700),
   zoomed toward 0.2 by 0.1 an update; velocities per axis with key-down and
   last-move times.
4. **Each frame**, in real seconds:
   - keys: pressing sets the axis's start time. Held, v = S × min(2t, 1) for the
     plane (S = 125) and S/2 × min(2t, 1) for up and down; released, v ←
     v × (0.5 − dt), 0 once dt ≥ 0.5;
   - edges on the 640 × 480 layout (x ≤ 6, ≥ 634; y ≤ 6, ≥ 474): yaw rate and
     tilt rate ±R × min(2t, 1), R = 1.5 (0.2 zoomed), dying away the same way;
   - move: sideways (sin θ, −cos θ), forward (cos θ, sin θ), each new x or y
     taken only if above tan(field/2) × z and below the map's side;
   - yaw += rate × dt; tilt += rate × dt unless that leaves (0, π/2);
   - z held to [h + 36, h + 236], h the highest landscape or building top at x, y;
   - x and y each held to the bunker's ± 200;
   - view matrix Rz(θ) Ry(π/2 − tilt), position last; look along its first column.
5. **Draw** the world from that camera; the selection markers, the commander
   panel with its column, the commander's satellite map, the message box and the
   objectives screen (80% dim) over it; the system cursor.
6. **Keys** in command mode, from `addition.man`: keypad arrows, PageUp and
   PageDown, Z, N, M, F12, F2, F1, F3; comma and dot turn a building being
   placed by 0.05 rad. Enter does nothing.
7. **Esc** leaves (mode 4 → 0): take the hero back where it stands, clear the
   selection, back to the cockpit, drop held keys. The character handler would
   first close an open satellite map, then turn a page back to 0; whether it
   sees the key before the binding leaves is not established, and peeling them
   back first is the gentler choice (*guess*).
8. **Telepresence**: a unit page's three buttons set the unit's auto-driver
   level 0, 1 or 2 and push mode 2 (take the unit, cockpit view). Esc returns to
   the command view, the camera where it was.

## Not established

- What view state 4 and the second camera at `+0x68` are for: its update runs
  only in a network game or with the level's `+0xaf5` set, and nothing read
  here sets that state.
- The four globals `0x1010bf7c`–`0x1010bf80` the transitions clear, and cursors
  7 and 8 (`0x10104148`), which stop the edge turns and cut the draw.
- What interface `0x201` slot 9 with (`0x20`, 1) does to the bunker left for
  another view or for telepresence, and whether its guns fire on their own
  while command mode is up.
- The display's slot 12 that decides whether the system cursor is shown.
- The following camera's distance logic in mode 3 (`+0x38`, `+0x92`, `+0x93`,
  `0x10037e00`, `0x100380e0`) and the two attached positions (`+0x3c` 30 above,
  `+0x40` 90 above) the move routine takes first.
- Whether the key-down binding 735 or the character handler's Esc comes first.
- What mode 2 does when its unit dies.
- The recording's camera height and turn rates, beyond the tilt's timing:
  nothing in view gives a scale.
