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
| 3 | the outer camera, `+8` ([30-turrets.md](30-turrets.md#the-outer-camera--read-and-measured)) | `CMD_JAMES_OUTER_CAMERA` |
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
| `+0x38` | the distance it follows an HQ unit at: 30 when made, 0 when a follow starts ([below](#an-hqs-command-mode-mode-3--read-and-seen)) |
| `+0x44` | an HQ unit it follows (mode 3) |
| `+0x48` | **the building it is held around** |
| `+0x4c` | the tilt: 0 straight down, π/2 level |
| `+0x50`, `+0x54`, `+0x58` | velocity sideways, forward and up |
| `+0x5c`, `+0x60` | tilt rate and yaw rate |
| `+0x64`–`+0x74` | per axis, the time of the last update |
| `+0x78`–`+0x88` | per axis, the time its key or edge last changed |
| `+0x8c`–`+0x91` | the six keys held: left, right, forward, backward, up, down |
| `+0x92`, `+0x93` | shorten and lengthen the follow distance; only the constructor writes them, 0 (`0x10036f8c`) |
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
3. follows an HQ unit in mode 3 (`+0x44`,
   [below](#an-hqs-command-mode-mode-3--read-and-seen));
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

**Esc** has a second reader, and it reads the key first (*read*). The window
message routine (`0x100a0e30`) hands a message to the input listeners before
the bindings, in every view state but 0 (the level's `+0x710`, `0x100a0e71`):
a key-down, `0x100`, goes to the listener chain's slot 0 (`0x100a0eb8`), which
asks each listener in turn and stops at the first to answer 1
(`0x100709f0`); only when none does is the same message passed on to
`0x10071c10` (`0x100a0fe5` → `0x100a0fe7`), whose lookup gives 735. The game's
own listener (vtable `0x100e6490`, registered at `0x10070d5c`) has at slot 0
the handler `0x10070db0`, which this page and its neighbours have called the
character handler. **It is the key-down handler**: it takes two arguments,
the key-down's `wParam` and `lParam` (`ret 8`), and switches on the virtual-key
code, `0x13` (`VK_PAUSE`) to `0x91` through its index at `0x10071168`.
`VK_ESCAPE` is `0x1b` and the digit keys are their characters, which is why the
name fitted. The character itself, `WM_CHAR` (`0x102`), goes to slot 2,
`0x100711f0`, a different handler. So in command mode the key-down handler
answers Esc in this order, taking the key at the first that applies (from
`0x10070dea`), and Esc leaves only when none applies:
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

Otherwise the key is not taken (`0x10071088` answers 0) and the binding has it.
Steps 2, 4, 5, 7 and 8 are skipped while the game menu, mode 7, is on the stack's
front (the `0x10044190` tests). So **Esc in command mode peels those back one
press at a time before 735 leaves** (*read*), and **Mission 04's recording shows
the same** (*seen*): from an HQ's command view, one Esc a second closes the map,
then the page, then leaves ([below](#leaving)). The objectives screen, a
building being placed and a message box on screen come off first, in that
order, in every view.

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

**What each level hands the AI** (*read*). Taking a bot (`0x10074ff0` with 1,
`0x10075027`–`0x100750c9`) writes the unit's `Wizard.dll` words through its
slot 9 (`0x10002070`: mask bit 0 → `+0x200`, 1 → `+0x204`, 4 → `+0x208`,
5 → `+0x20c`, 6 → `+0x210`, 7 → `+0x214`, 8 → `+0x218`, 9 → `+0x21c`), each
3 for the player, 1 for the AI and 0 to follow the mode message 7 sets. The
Wizard (`0x10003890`) then gives each side its bits: the behaviour's mode word
bit 2 from `+0x200`, which the mode set makes flag `0x10`, movement; bits 4 and
8 from `+0x20c`, flags `0x20` and `0x40`, the fight module; bit 1 from `+0x204`
(not followed); each component by its power group (interface `0x204`
slot 9, [29-weapons.md](29-weapons.md#who-may-drive-a-units-guns--read)) — 4,
the turret, guns, arms and builder, from `+0x20c`; 5, shields and armour, from
`+0x208`; 2, camera, radar and seeker, from `+0x210`; 0 from `+0x214`; 3, the
engines, from `+0x218`; 1 from `+0x21c`; and the unit's own rows (component −1)
their input bit from `+0x200` and their fire bit from `+0x204`
(`0x10003a67`–`0x10003a9a`).

| level | `+0x200`: the unit's rows, movement | `+0x20c`: turret and guns, fight | `+0x208`: shields | `+0x210`, `+0x214` | mode (message 7) | `+0xa2` | turret lock (179) |
|---:|---|---|---|---|---|---|---|
| 0 | player | player | player | player | 1, player | 1 | 1 |
| 1 | **AI** | player | player | player | 1, player | 1 | 0 |
| 2 | **AI** | **AI** | **AI** | not written | **0, AI** | 0 | 0 |

So **level 1 gives the AI the walk**: the behaviour's flag `0x10` is on, so its
unit takt runs the unit's orders and walker, and the player's movement rows
take no input; the turret, guns, shields, sensors and engines stay the
player's, the fight module off. **Level 2 gives the AI the unit**: movement,
turret and guns with the fight module, shields and armour, and by message 7
with 0 every word left to follow the mode: `+0x204`, the engines' and group
1's. The two words level 2 does not write, `+0x210` (camera, radar, seeker) and
`+0x214`, keep what was last written: 1 after a letting-go, which writes 1 into
every word (`0x10075131`), and 3 when the key steps a driven unit from 1 to 2,
so the sensors then stay the player's. A unit's words before its first take
are its Wizard's constructor's (not read). The player rides along in the
unit's cockpit. The hero ignores the level: its take gives every word 3
(`0x10075018`). `CMD_JAMES_AUTO_DRIVER` (744) steps the level 0 → 1 → 2 → 0 and
takes the unit again at the new one (`0x10075fc0`).

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

## An HQ's command mode: mode 3 — *read*, and *seen*

*Teleport* (`CAMPAIGN.00/Mission.04`) gives the player a mobile command centre.
Its help line T04_H01 (message 15) says command mode is "Enter" inside the
warbot and "Escape" out, and that "the mobile command center's camera only
moves in sync with the warbot". This section reads how. *Seen* here is the
recording `training mission 4   teleport [xoPlGMdKVSY].mkv` (960 × 720);
*measured* is `check_hq_command_mode`.

**The HQ** (*measured*):
- Mission 04's unit with logical id 1, `tut4_hq.dat` at (422.5, 518.2),
  belongs to `Ntrl`, a clan of type 3, neutral. Its Type is `0x1010000`.
- Its chassis `R_B_03` is wheeled and of size letter `b` (class 4), and drives
  with `m1.tbl`.
- Its turret `e_tur_bt_04` is its class-1 component. The turret word
  `0xc000000` carries the HQ bit `0x8000000` beside the upright mounting.
- The hero starts 65.7 m from it.
- Of the campaign's HQ units, a neutral clan owns two: this one and
  `CAMPAIGN.02/Mission.02`'s `23lhq1.dat`.

### Getting in: one Enter takes it, a second opens the view

- **The first Enter captures and boards it.** This HQ passes every test of the
  hero's Enter ([39-boarding.md](39-boarding.md#boarding--read),
  [27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)):
  its Type is within `0x103e000`, its size class is 4, and it has a class-1
  turret. So Enter within 20 m captures it for the player's clan and pushes
  mode 1 with it (*derived*).
  - Driving it is driving any wheeled bot: `m1.tbl`, the turret's camera and
    its cockpit.
  - Nothing in mode 1 asks `IsHQ` (`0x10076f50`) but Enter. Its seven calls
    (*measured*, as a call scan) are the push's refusal (`0x10062c2e`), the
    three handlers into mode 3 (`0x10063a52`, `0x1006480c`, `0x10064934`),
    Enter in telepresence (`0x10071f4e`), and the unit page's D button, drawn
    (`0x10085b5e`) and clicked (`0x100847d5`).
- **The second Enter pushes mode 3.** `CMD_ENTER_STATE`'s case
  (`0x10071f08`) does nothing in view state 4.
  - With mode 1 at the front, it pushes mode 3 with the driven unit (`+0xaec`,
    `0x100720f2`–`0x10072104`).
  - With mode 2 at the front, it does the same if the driven unit passes
    `IsHQ` (`0x10071f43`–`0x10071f55`).
  - The push refuses a unit that fails `IsHQ` (`0x10062c2e`), so Enter aboard
    any other bot does nothing.
- **The other way in** is the unit page's D button, *Strategic control*, shown
  for an HQ ([41-commander.md](41-commander.md#the-pages--read)). It calls
  `0x10062bc0(3, 0, unit)` and turns to page 0 (`0x100847ef`), from mode 4 or
  from another HQ's mode 3.
- **There is no handler for 0 → 3**: the table's entry is 0, so the hero on
  foot cannot open an HQ's view.

**The handlers** (*measured*, table `0x10104b18` at *front* × 8 + *new*):

| front → new | handler | reached by |
|---|---|---|
| 1 → 3 | `0x10063a20` | Enter aboard the HQ |
| 3 → 1 | `0x10063ad0` | Esc |
| 3 → 2 | `0x10063b40` | a unit page's A, B and C buttons |
| 2 → 3 | `0x10063bf0` | Esc from telepresence; Enter in telepresence aboard an HQ |
| 3 → 3 | `0x10064900` | D on another HQ |
| 4 → 3, 3 → 4 | `0x100647e0`, `0x10064880` | D from a bunker's view; the bunkers page, or Esc back to the bunker |
| 3 → 6, 6 → 3 | `0x10064210`, `0x10064280` | a tower's *Manual*; Esc back |
| 3 → 7 | `0x10064650` | the game menu |
| 0 → 3, 3 → 0 | none | |

**Mode 1 → 3** (`0x10063a20`), in order:
1. turns the outer camera off, and clears the four globals
   `0x1010bf7c`–`0x1010bf80`;
2. stops there for a unit that fails `IsHQ`;
3. **places the camera at the HQ's record position** (`+4`, `+8`, `+0xc`) with
   roll 0, pitch 0 and yaw π/2 (`0x10036ae0`). The tilt and the zoom are kept;
4. **sets the camera to follow the HQ** (`0x10037d70`): `+0x44` = the HQ and
   `+0x48` = 0, so no building box. It also clears the up and down keys, the
   velocities and the follow distance `+0x38`;
5. keeps the HQ as the driven unit (`+0xaec`), clears the view's building, and
   **sets view state 2**;
6. **lets the HQ go** (`0x10074ff0` with 0). It goes back to its AI with every
   override on ([31-packages.md](31-packages.md#the-escape--read));
7. turns an open page to 0 (`0x10084d80`).

**What 1 → 3 leaves alone** (*derived*):
- The selection is not touched. The HQ was selected alone when it was boarded,
  so it stays selected and marked.
- The hero stays out of the world, as it was aboard.
- Unlike 0 → 4, the keyboard is not cleared.

### The camera rides with the HQ

Each frame, while `+0x44` is set, the command camera's update (`0x10037a50`)
runs two steps before its ordinary ones ([The camera](#the-camera--read)).

**1. The target** (`0x10037e00`). The terms:
- θ is the yaw, τ the tilt and d the follow distance;
- (X, Y, Z) is the HQ object's position, the last column of its matrix
  (interface `+0x3c`, slot 8, kind 2).

The target is

  target = (X − d sin τ cos θ, Y − d sin τ sin θ, Z + d cos τ).

That is the point d back along the view's look from the HQ (*derived*: the
look is (cos θ sin τ, sin θ sin τ, −cos τ)).

For each axis, the camera's distance from the target is compared with a limit:
**3 m** for x and y, **2 m** for z (`0x100e5c14`, `0x100e5c0c`).
- **Within the limit**, the coordinate is **set to the target's**, its velocity
  cleared and both its keys let up.
- **Beyond it**, the key toward the target is pressed and the other let up:
  right for a target at greater x and left for less, forward for greater y and
  backward for less, up or down for z.

The keys' velocities are then worked out as for the player's keys
(`0x10037130`): S × min(2t, 1), with S = 125.

**2. The move** (`0x100380e0`):
- x += dt·v<sub>x</sub> and y += dt·v<sub>y</sub>, **along the world's axes,
  not the yaw's**. Each new value is taken only while above tan(½ field) × z
  when the velocity is negative, and below the map's side − tan(½ field) × z
  when it is positive.
- z += dt·v<sub>z</sub>, and yaw += dt × the yaw rate.
- z is raised to at least **2.5** above the highest landscape or building top
  at x, y (`0x100a14d0`, `0x100e5d04`).

**The follow distance** is then stepped:
- d −= 1 when d > 8 r, where r is the HQ record's `+0x98` (`0x100382e2`; the 8
  is at `0x100e5d00`);
- d −= 1 when a building is held and d > 200, which never happens in mode 3;
- d += 1 when d < 200.

So d grows by 1 a frame from 0 until it passes 8 r, then stays (*derived*).
**The camera pulls back out of the HQ to its orbit over the first frames**,
faster at a higher frame rate. The update also reads `+0x92` and `+0x93`, which
would shorten or lengthen d by 1 a frame, but nothing sets them (*read*, as a
search: their only writes are the constructor's). The move then hands the
frame to the view, as the ordinary move does.

**The ordinary update then runs as in mode 4**: edge flags, velocities from
the keys, and the move (`0x100375b0`).
- Its dt for each axis is the time since the follow's move stamped that axis,
  so it moves the position by next to nothing (*derived*).
- Its tilt step still applies, and so does its **h + 36 … h + 236 height band**.
- The box does not apply, since no building is held.

**What follows** (*derived*):
- **The camera stays on the HQ.** A unit at 95 km/h moves 0.44 m in a 60 Hz
  frame, well inside 3 m. So the camera sits on the target every frame and
  moves with the HQ, as T04_H01 says.
- **The player's camera keys do nothing.** They set the same key flags, but the
  follow resets every flag each frame before the velocities are read. The
  arrows, PageUp and PageDown are lost; Z still zooms.
- **The cursor at an edge swings the camera round the HQ.** The yaw and tilt
  turn as in mode 4, and the target is worked out from them afresh each frame,
  so the camera circles the HQ at distance d, and rises or sinks over it.
- **The height band wins over the orbit.** Where the ground under the camera is
  higher than Z + d cos τ − 36, the camera is lifted, and the HQ sits below the
  middle of the view.

**r, the HQ record's `+0x98`.** It is the seventh float that the unit's
interface `0x18` slot 12 fills in mode 2 (`0x1007e5c8`–`0x1007e5d6`,
`AniMesh.dll:0x100146a0`).
- **That slot returns the mesh object's bound** at `+0x134`: its two axis
  points put through the object's matrix, and a radius.
- **Where the bound comes from.** A single-part mesh copies it from its
  stream-2 header's cylinder (`AniMesh.dll:0x1000a899`,
  [07-objects.md](07-objects.md#the-stream-2-header-is-the-models-authored-extent)).
  A multi-part mesh works it out from its box (`0x10009d10`–`0x10009e47`, not
  followed). Which of the two a unit's object is was not traced.
- **Measured** on Mission 04's HQ: `r_b_03.msh`'s cylinder radius is 4.832 and
  its sphere's radius 7.653, so 8 r is 38.7 m or 61.2 m.
- **Seen**, the recording favours the second. At 80.5 s the HQ is about 200
  pixels across on the 960-pixel frame. With a field of 1.04 rad, a body
  13–15 m across would span about 300 pixels at 39 m.

### What the player has in mode 3

- **The same screen as a bunker's.** Modes 3 and 4 share their draw
  (`0x1008d51c`, [What command mode draws](#what-command-mode-draws--read)).
  The panel's buttons act in either mode
  ([41-commander.md](41-commander.md#what-a-click-on-the-column-does)). A page
  is enabled by what the clan holds, not by the view. *Seen* from 83.5 s: the
  Battle units page lists *LWC-1 Comm. Center* and *TFB-2 Warrior*.
- **The HQ carries out its own order.** It was let go on entry, so its AI
  drives it, and the player orders it from its page like any unit. *Seen* at
  90–94 s: *LWC-1 Comm. Center [no order]* becomes *[standing]* after a click
  on Standby. A Route would carry the camera along with it (*derived*).
- **Telepresence works from mode 3.**
  - A unit page's A, B and C buttons push mode 2 (3 → 2, `0x10063b40`). The
    handler clears the selection and selects the unit, lets the HQ go (with 0)
    and takes the unit (with 1), and makes the unit the driven unit. It lets the
    camera go, sets view state 1, clears the `CState`'s `+0x31` and stamps its
    `+0x2c`, and clears the keyboard.
  - **Esc comes back to mode 3** (2 → 3, `0x10063bf0`). The handler lets the
    unit go and **places the camera on the HQ again, facing north, following it
    from distance 0**. It makes the HQ the driven unit, sets view state 2,
    clears the keyboard and redraws the page.
  - Unlike 2 → 4, which leaves the camera where it was, 2 → 3 pulls the camera
    back out of the HQ once more.
- **Enter does nothing** in mode 3. The front is neither 1 nor 2, and the
  hero's test wants view state 1 or 3 (*derived*, as in mode 4).
- **The hero button** rolls the stack back to mode 0 (`0x10062ce0` with 0,
  [41-commander.md](41-commander.md#what-a-click-on-the-column-does)). That goes
  through 3 → 1, then 1 → 0, which puts the hero down beside the HQ or refuses
  with *Risk area!* ([39-boarding.md](39-boarding.md#leaving--read))
  (*derived*).

### Leaving

**Esc** (735) rolls the stack back. From mode 3 the record below is the HQ's
mode 1. **Mode 3 → 1** (`0x10063ad0`), in order:
1. turns the outer camera off;
2. selects the HQ (`0x1007d0a0` with 1);
3. makes it the driven unit, clears the view's building, and **sets view state 1**;
4. lets the camera go (`0x10037dd0`);
5. **sets the HQ's auto-driver level `+0x9c` to 0 and takes it**
   (`0x10074ff0` with 1);
6. clears the keyboard (`stdClearKeyboard`, its tail jump).

So **Esc returns to the HQ's cockpit**, with the player driving it, and a
second Esc puts the hero down beside it (*derived*). Before either, Esc's
key-down handler closes an open satellite map and turns a page to 0, first
([Input](#input--read-and-measured)).

*Seen*, 157–162 s, four cuts about a second apart:
- 158.0 s: the satellite map closes;
- 159.0 s: the Battle units page closes;
- 160.0 s: the HQ's cockpit, with two struts, target *Small Generator*, own
  panel *LWC-1 Comm. Center [no order]*, and weapons *LFT, LFT*;
- 162.0 s: the hero on foot beside the HQ, *Human*.

So **the map and the page do peel back one Esc at a time before 735 leaves.**

**When the HQ is lost in mode 3.**
- The unit record's removal (`0x100751a0`) rolls the stack back only when the
  lost unit is the driven one and the front is mode 1, 2, 5 or 7. Its case
  table at `0x1007563c` gives modes 3, 4 and 6 nothing (*measured*).
- The takt's lost-building flag (`0x10062950`) reads only a record's building.
- **The game frame does, for an HQ the hero boarded** (*read*). Every frame it
  tests the `CState`'s `+0x28`, the bot boarded from foot, with the component
  test, and once that refuses — the HQ gone, or its turret's body shot to
  nothing — rolls the stack back to mode 0 (`iron3d.dll:0x1005eab3`–`0x1005eacf`,
  [39-boarding.md](39-boarding.md#when-the-driven-bot-is-lost--read)): 3 → 1,
  then 1 → 0, which puts the hero at (x − 1, y − 1) beside a broken HQ,
  untested (*derived*). That is Mission 04's way in, Enter aboard the HQ.
- An HQ whose view was reached otherwise — from a bunker's view (4 → 3), or by
  Enter in telepresence begun there — is not the `CState`'s `+0x28`, and
  nothing read pops mode 3 when it dies: the camera's `+0x44` and the driven
  unit still name it. What the game does then was not followed.

### Against the recording — *seen*

| time (s) | what |
|---|---|
| 76.5–77.5 | the hero walks to the HQ; target *LWC-1 Comm. Center*, 5 m |
| 78.0 | **a cut into the HQ's cockpit**: two struts against the sky; own panel *LWC-1 Comm. Center [no order]*; weapons *LFT* ×2; target *TFB-2 Warrior [escaping]*. The box still shows *"Good job, Cadet!…"* (T04_I01) |
| 78.4 | **a cut to the command view**: the icon column, *Ore* and *Energy 5%*, the message box at the bottom right. The camera is low at the HQ, looking north at a rock face, with blue and yellow at the bottom edge |
| 78.8–79.2 | **the HQ rises into view from the bottom edge and settles** in the lower middle, bracketed and named in green, as the camera backs away |
| 79.5 | the box changes to T04_H01, from the Information assistant |
| 81.5 | the satellite map opens |
| 82.5–83.0 | the view swings off the HQ toward the rock: an edge turn |
| 83.5– | the Battle units page |
| 84–90 | the TFB-2 Warrior is ordered to *Search and capture* |
| 90–94 | the HQ is ordered to *Standby* |
| 157–162 | leaving, above |

**The pull-back takes about 0.8 s** from the cut, 78.4 to 79.2 s. At 1 m a
frame that is about 75 frames a second to reach 61 m, or about 48 to reach
39 m. The recording's own frame rate is not known, so this does not settle r
(*derived*).

### For an engine

1. **Board** Mission 04's HQ with Enter as any large bot: capture it if it is
   neutral, push mode 1, and drive it with `m1.tbl`.
2. **Enter aboard an HQ** (a unit whose turret carries `0x8000000`), or
   aboard one in telepresence, pushes mode 3. Enter in any other bot does
   nothing. Mode 3:
   - the camera at the HQ's position, yaw π/2, tilt and zoom kept, following
     the HQ from distance 0, with no building box;
   - the HQ handed back to its AI, keeping its order;
   - view state 2 with the commander panel on page 0;
   - the HQ still selected.
3. **Each frame in mode 3**, before the ordinary camera update:
   - target = HQ − d (sin τ cos θ, sin τ sin θ, −cos τ);
   - per axis: within 3 m (x and y) or 2 m (z), snap to the target and stop;
     otherwise drive toward it at the ramped key speed S × min(2t, 1), along
     the world axis;
   - z at least 2.5 above the ground or building top;
   - d += 1 a frame (not a second) until it passes 8 r, and d ≤ 200;
   - then the ordinary update: the edges turn yaw and tilt, the height band is
     h + 36 … h + 236, and there is no box. The player's move keys are
     overridden; Z zooms.

   For r, 8 r ≈ 61 m on Mission 04's HQ fits the recording. Until `+0x98` is
   traced, use the assembly's bounding sphere radius as a STAND-IN.
4. **Panel and orders** work as in mode 4. The HQ may be ordered like any unit,
   and the camera rides with it.
5. **Telepresence** works from mode 3 as from mode 4. Esc returns to mode 3
   with the camera placed back on the HQ and pulled out again.
6. **Esc** peels back the map, then the page, then leaves 3 → 1: the HQ is
   selected and taken at auto-driver level 0, the view is its cockpit, and held
   keys are dropped. A further Esc leaves the HQ as it would any bot.
7. **The hero button** goes to mode 0 through mode 1.
8. **An HQ the hero boarded, lost in mode 3** or its turret's body shot off,
   rolls the stack back to mode 0 through mode 1, and the hero is put down at
   (x − 1, y − 1) beside it. An HQ reached from a bunker's view is not handled
   by anything read; rolling back to the view below is the nearest thing
   (*guess*).

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
   selection, back to the cockpit, drop held keys. Its key-down first puts away,
   one press each, an objectives screen, a building being placed, a message box
   on screen, an open satellite map, then a page other than 0; only then does
   the binding leave.
8. **Telepresence**: a unit page's three buttons set the unit's auto-driver
   level 0, 1 or 2 and push mode 2 (take the unit, cockpit view). At 0 the
   player drives it whole; at 1 its AI walks it on its orders and the player
   has the turret and guns; at 2 its AI has it whole, fire included, and the
   player rides along. Esc returns to the command view, the camera where it was.

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
- ~~The following camera's distance logic in mode 3~~ — **read**, in
  [An HQ's command mode](#an-hqs-command-mode-mode-3--read-and-seen). Still
  open: the two attached positions (`+0x3c` 30 above, `+0x40` 90 above) that
  the move routine takes first.
- ~~Whether the key-down binding 735 or the character handler's Esc comes
  first~~ — **read**: the handler, which is the game listener's key-down handler
  (slot 0, `0x10070db0`), is asked first, and the key-down reaches the bindings
  only when no listener takes it (`0x100a0eb8`, `0x100a0fe5`), in
  [Input](#input--read-and-measured). Mission 04's recording peels the map and
  the page back one Esc at a time (*seen*).
- ~~What telepresence's auto-driver levels 1 and 2 hand to the AI~~ — **read**:
  level 1 the walk, level 2 the whole unit, fire and shields included
  (`0x10074ff0`, `Wizard.dll:0x10003890`), in
  [Telepresence](#telepresence-mode-2--read).
- ~~What mode 2 does when its unit dies~~ — **read**: the unit record's removal
  rolls the stack back in modes 1, 2, 5 and 7 (`0x100755a9`, table
  `0x1007563c`), so telepresence ends with its unit, back to the command view.
- **r, the unit record's `+0x98`**, which sets an HQ camera's follow distance at
  8 r: the bound's seventh float, but whether a unit's object keeps its
  chassis mesh's cylinder or a bound worked out over its parts was not traced.
  The outer camera stands off by the same r
  ([30-turrets.md](30-turrets.md#the-outer-camera--read-and-measured)).
  - Mission 01's recording puts the hero's at about 1.3–1.5 m. That is near its
    chassis mesh's box half-diagonal, 1.47, and far from its chassis sphere,
    1.15, or its joined sphere, 2.18.
  - Here the HQ recording favours the chassis sphere, 7.65, over the cylinder,
    4.83. The chassis box's half-diagonal, 8.14, would put the camera at 65 m
    and fits that recording nearly as well.
  - The multi-part branch of the bound (`AniMesh.dll:0x10009d0f`) takes the
    half-diagonal of a box (*seen*, *read* in part).
- ~~**What happens when an HQ is lost in its own mode 3**: nothing read rolls the
  stack back.~~ **Read** for an HQ the hero boarded: the game frame's test of
  the boarded bot rolls the stack back to mode 0 once the HQ is gone or its
  turret's body is shot off (`iron3d.dll:0x1005eacf`,
  [When the HQ is lost in mode 3](#leaving)). Still open for an HQ whose
  view was reached from a bunker's: nothing read rolls the stack back.
- Why *LWC-1 Comm. Center* reads *[no order]* in its cockpit at 160 s, after
  *[standing]* in its command view at 94 s: whether taking a unit at level 0
  clears its order, or the player ordered it again unseen.
- The recording's camera height and turn rates, beyond the tilt's timing:
  nothing in view gives a scale.
