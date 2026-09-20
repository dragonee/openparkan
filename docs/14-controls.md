# The input layer — `.tbl`, `.man`, `.dsc`

Seventeen files in the installation root are not archives, not compressed and
not obscure. They are tables the developers wrote in plain text and commented
in English, and together they are the whole path from a key to a command.
Nothing in this document was recovered from a disassembly. It was read.

```
ScanCode.dsc   SCAN_A                                   a key, and its label
Command.dsc    CMD_OBJ_MOVE_LEFT   Move object left     an action, described
hero.man       CMD_OBJ_MOVE_LEFT SCAN_NULL SCAN_A       which key runs it
hero.tbl       KEY SCAN_NULL SCAN_A 1 CICLS_UNKNOWN MCMD_LEFT 1.0 0 0 0.0 0
                                                        what it sends, where
```

Everything below is re-derived by `uv run openparkan verify`.

## The descriptors

`ScanCode.dsc` is **174** keys, each an identifier and the label the game
prints for it — `SCAN_ESC` / `Esc`, `SCAN_W_1` / `1`. `SCAN_MOUSE_X`,
`SCAN_LMOUSE` and the numeric-keypad `SCAN_G_*` family are in the same list, so
a mouse axis is a "key" as far as the tables are concerned.

`Command.dsc` is **72** actions, each with a sentence: `CMD_JAMES_HQ_MOVE_LEFT`
/ *Move HQ-camera left*. The families name the subsystems — `CMD_OBJ_*` for
the machine, `CMD_TURRET_*`, `CMD_CAMERA_*`, `CMD_FIRE_*`, `CMD_SELECT_*`,
`CMD_CAMOUFLAGE_*`, `CMD_REPAIRSYS_*`, and `CMD_JAMES_*` for the commander's
own view.

## The bindings

A `.man` is three fields a line: an action, a modifier key and a key. There
are **12** of them and **275** bindings, and **every one names a command that
exists in `Command.dsc` and keys that exist in `ScanCode.dsc`** — nothing
dangles in either direction. 65 of the 72 commands are bound somewhere; seven
are not bound at all.

The files come in pairs. `ui_hq.man` and `ui_hq_d.man` are byte-identical;
`ui_bots`, `ui_hero` and `ui_other` differ from their `_d` twins by a handful
of keys. The `_d` file is the default and the other is what the player is
using — `CMD_JAMES_SELECT_FRIEND` sits on `SCAN_F` by default and on `SCAN_T`
as shipped.

Four of `iron3d.dll`'s commands pick the player's target (*measured*): Tab
for `CMD_JAMES_SELECT_TARGET` (732), E for `_SELECT_ENEMY` (733), T or F for
`_SELECT_FRIEND` (734), and the right mouse button for `CMD_JAMES_AIM_TARGET`
(750). Shift with the right button is `CMD_CAMERA_CENTER` instead. What
each does is in
[25-sensors.md](25-sensors.md#the-players-target--read-and-measured).

## The table

A `.tbl` row is eleven fields and then a trailing text:

```
KEY   SCAN_NULL SCAN_A  1  CICLS_UNKNOWN  MCMD_LEFT   1.0  0  0            0.0  0     // OBJ_MOVE_LEFT
MOUSE SCAN_LSHIFT SCAN_MOUSE_X 1 CICLS_CAMERA MCMD_ANGLE_X 0.1 1 MAN_WRAP  0.0  0     // CAMERA_LEFT
KEY   SCAN_NULL SCAN_G_PLUS 1 CICLS_UNKNOWN MCMD_FORWARD   1.0 0 0         0.05 1000  // OBJ_SPEED_MORE
```

| Field | Meaning |
|---|---|
| 1 | `KEY` or `MOUSE` |
| 2, 3 | the chord: a modifier (`SCAN_NULL` on 104 of 116 rows) and the key; a chord with no row falls through to the plain row ([below](#a-chord-with-no-row-of-its-own--read-and-measured)) |
| 4 | 1 on the press, 0 on the release |
| 5 | `CICLS_*` — the class of component the command is aimed at |
| 6 | `MCMD_*` — the command itself |
| 7 | its magnitude |
| 8 | an index — a weapon number, or a channel |
| 9 | `0`, a `MAN_*` wrap flag, or a `CIS_*` state |
| 10, 11 | a ramp and its time, set on six rows only |

**116 rows** across three tables, every one with its eleven fields, and all 31
distinct scan names in them are in `ScanCode.dsc`.

The press/release convention is the whole of the input model: a key going down
sends its command with magnitude 1.0 (or -1.0 for the opposite direction, 0.7
for a turn), and the key coming up sends **the same command with 0.0**. That
holds on 21 of the 27 release rows; the six that do not are the camera-centre
rows, which send 0.5 on release rather than cancelling.

On a press row the trailing text is usually the `Command.dsc` identifier
without its prefix — 57 of 89 — so it is not a comment but the row's identity,
and it is what ties a table row to the `.man` binding that rebinds its key. The
rest are prose (`stop robot to left`) or parameterised placeholders that are
never bound literally: `SELECT_WEAPON_(n)`, `CAMERA_CENTER(x)`.

### The command set

Fourteen `MCMD_*` commands appear across the three tables:

| Command | Rows | | Command | Rows |
|---|---:|---|---|---:|
| `MCMD_SELECT` | 27 | | `MCMD_LEFT` / `MCMD_RIGHT` | 6 each |
| `MCMD_STATE` | 15 | | `MCMD_WALK_F` / `MCMD_WALK_B` | 6 each |
| `MCMD_ANGLE_Y` | 12 | | `MCMD_UP` / `MCMD_DOWN` | 2 each |
| `MCMD_FORWARD` | 12 | | `MCMD_LOCK` | 2 |
| `MCMD_ANGLE_X` | 11 | | `MCMD_ANGLE_Z` | 1 |
| `MCMD_ROTATE_Z` | 8 | | | |

and six target classes: `CICLS_UNKNOWN` (49 rows), `CICLS_MULTIGUN` (33),
`CICLS_CAMERA` (21), `CICLS_TURRET` (7), `CICLS_DETECTSHIELD` and
`CICLS_REPAIRSYS` (3 each). So a single mouse axis means "turn the turret" or
"turn the camera" depending only on whether shift is held — the row names the
class, and the engine finds the component.

This is the input half of the [movement controller](13-control.md), and the
next two sections join them.

## The numbers behind the names

The engine parses these tables as text, so it must carry a resolver for every
family of names in them — and it does. `World3D.dll` holds one chain of string
compares per family, each case returning a constant, each chain ending in a
branchless idiom (`neg eax; sbb eax, eax; and eax, M; add eax, K`) that yields
the constant on a match and the family's default otherwise: **-1** for most,
**0** for the component classes. The names are the files'; the numbers are the
engine's.

Five of the six families are `World3D.dll`'s alone. The sixth, the `CMD_`
commands, is split with `iron3d.dll`, and that split is worth a section of its
own below. `analysis/names.py` extracts all of them; swept across the whole
installation, those **two binaries are the only ones that carry a resolver
chain at all**.

**Every one of the 174 keys in `ScanCode.dsc` has a value, and there are no
others** — the two sets are equal. They are the real **IBM PC set-1 scan
codes**: `SCAN_ESC` is 1, `SCAN_A` is 30, `SCAN_LSHIFT` is 42, `SCAN_F1` is
59, and the first 56 entries of the descriptor are listed in code order. The
mouse and joystick continue the numbering past the keyboard — `SCAN_MOUSE_X`
is 510, `SCAN_JOY_X` 590.

The commands:

| | | | | | |
|---|---:|---|---:|---|---:|
| `MCMD_DUMMY` | 0 | `MCMD_ANGLE_Y` | 6 | `MCMD_SELECT_NEXT` | 14 |
| `MCMD_STATE` | 1 | `MCMD_FORWARD` | 7 | `MCMD_TABLE` | 15 |
| `MCMD_ROTATE_X` | 2 | `MCMD_BACK` | 8 | `MCMD_ANGLE_Z` | 16 |
| `MCMD_ROTATE_Y` | 3 | `MCMD_LEFT` | 9 | `MCMD_MISSILE` | 17 |
| `MCMD_ROTATE_Z` | 4 | `MCMD_RIGHT` | 10 | `MCMD_FIRE_ALL` | 18 |
| `MCMD_ANGLE_X` | 5 | `MCMD_UP` | 11 | `MCMD_WALK_F` | 19 |
| | | `MCMD_DOWN` | 12 | `MCMD_WALK_B` | 20 |
| | | `MCMD_SELECT` | 13 | `MCMD_LOCK` | 21 |

and the classes, which are plain ids rather than bits: `CICLS_TURRET` 1,
`MULTIGUN` 2, `SIMPLE` 3, `CAMERA` 4, `ENGINE` 5, `RADAR` 8, `FIGHTSHIELD` 9,
`DETECTSHIELD` 10, `ELEVATOR` 11, `DOOR` 12, `COMPUTER` 13, `REPAIRSYS` 15,
`POWERSTOR` 19. 6, 7, 14 and 16–18 name nothing. `CICLS_UNKNOWN` is not in the
chain at all, so it is whatever the resolver returns for a name it does not
know: **0**.

The `CIS_` states *are* bits, and the same bit means different things to
different classes — **256** is `CIS_CONTINUEFIGHT` to a gun,
`CIS_TURRETCONTROL` to a turret and `CIS_ANGLETRACE` to whatever traces. So a
state word cannot be read without knowing the class it is aimed at, which is
why the row carries both.

### The commands, and the two binaries that answer for them

`Command.dsc` and the `.man` files speak a sixth family, `CMD_`, and this one
is not resolved in `World3D.dll` alone. **Two** binaries carry a chain, and
between them they cover every one of the descriptor's 72 names:

| binary | names | values | what it answers for |
|---|---:|---|---|
| `World3D.dll` | 43 | 1–66 | the object you are controlling |
| `iron3d.dll` | 31 | 723–754 | the game itself |

The object commands are **banded by subsystem**, and the bands are the
engine's own — nothing here is grouping by name:

| band | | commands |
|---|---:|---|
| the hull | 1–14 | `CMD_OBJ_MOVE_*` 1–6, `CMD_OBJ_TURN_*` 7–10, `CMD_OBJ_SPEED_MAX` 11, `_MORE` 12, `_LESS` 13, `CMD_OBJ_STOP` 14 |
| the turret | 20–24 | `CMD_TURRET_LEFT` 20, `RIGHT` 21, `UP` 22, `DOWN` 23, `CENTER` 24 |
| the camera | 30–35 | `CMD_CAMERA_LEFT` 30, `RIGHT` 31, `UP` 32, `DOWN` 33, `CENTER` 34, `INFRARED` 35 |
| the weapons | 40–49 | `CMD_SELECT_ALL_WEAPON` 40, then `CMD_SELECT_WEAPON_1`–`_9` 41–49 |
| firing | 50–51 | `CMD_FIRE_SELECTED_CONT` 50, `CMD_FIRE_SELECTED` 51 |
| the rest | 60–66 | `CMD_CAMOUFLAGE_WEAR` 60, `CMD_REPAIRSYS_ON` 61, `CMD_CHANGE_TABLE` 63, `CMD_SPOTLIGHT` 64, `CMD_FIRE_MISSILE` 65, `CMD_FIRE_ALL` 66 |

The game commands are a flat run instead — `CMD_JAMES_HQ_MOVE_LEFT` is 723 and
`CMD_QUICK_LOAD` 754, with only **743** and **745** unused. They are the
commander's camera, target selection, the wingman menu, the pager, the chat
terminal, entering and leaving a warbot, the map's alpha, the game menu, help,
and quick save and load.

**The split is a fact about the files, not a reading of the names.** Ten of
the twelve `.man` files draw on one binary only — `hero.man` and both
`table_*.man` from `World3D.dll`, `addition.man` and both `ui_hq*.man` from
`iron3d.dll` — and no `.tbl` row anywhere names an `iron3d.dll` command. Only
`ui_other.man` and its `_d` twin mix, which is what a file of that name should
do.

Two loose ends, both small and both checked:

- **One name is in both chains**: `CMD_CAMERA_INFRARED`, and the two resolvers
  **agree** that it is 35. It is a component command the game shell also wants
  to reach.
- **One name is in neither file**: `CMD_FIRE_SELECTED` (51) is resolved by
  `World3D.dll` but appears in no `Command.dsc` line and no `.man` binding.
  Its neighbour `CMD_FIRE_SELECTED_CONT` (50) is the one the descriptor
  exposes, so this looks like the single-shot half of a pair that shipped
  unbound. The union is 73 names against the descriptor's 72, and that is the
  whole of the difference.

Of the 73, **65** are actually bound by the 275 `.man` lines.

## The join with the controller — *read*

This section once closed a gap by counting. `Control.dll` has a 16-way jump
table — `lea eax, [edx - 1]; cmp eax, 0xf; ja …; jmp [eax*4 + 0x1002b9e8]` —
and `MCMD_` 1 to 16 is that range, so the table was read as the controller's
movement dispatch. **It is not a message dispatch.** The function it sits in,
`0x1002b410`, is slot 4 of the control system's `IDeviceManager` (interface
`0x204`, the sub-object at `+0xc`, vtable `0x1003b4fc`), a getter that answers
a figure by id; `edx` is the id and the argument is where the answer goes.
Its offsets are relative to `+0xc`, which is why the slots it reads looked like
`+0x5c8` and `+0x5cc` when they are the factory's radar (`+0x5d4`) and seeker
(`+0x5d8`).

| id | answers | | id | answers |
|---:|---|---|---:|---|
| 1 | the batteries' fill | | 9 | the radar's value 4, its period |
| 2 | the batteries' capacity | | 10, 11, 12 | the seeker's values 1, 0, 2 |
| 3, 4 | channel 0/2/5 and 0/3/4 demand a second | | 13 | whether the turret is an HQ turret |
| 5, 6 | **damage a second** ([below](#ids-5-and-6-are-damage-a-second--read-and-measured)) | | 14 | mean shield fill × Σ deflector values 0–5 × shield value 0 |
| 7 | the fight shield's mean sector fill | | 15 | Σ deflector values 0–5 × shield value 0 |
| 8 | the radar's value 3, its range | | 16 | deflector value 0 × shield value 0 |

Its callers ask for it after a `QueryInterface` for `0x204`: `Behavior.dll`
for 1, 2, 5, 6 and 7 (id 1 at `0x100180bd`), `ArealMap.dll`
for 6, `iron3d.dll` for 2, and a gun, on the round it fires, for 10–12
(`Control.dll:0x1002986c`). The ids are listed in `control.DEVICE_QUERIES`.

### Ids 5 and 6 are damage a second — *read*, and *measured*

Both walk the control system's components (`+0x5b0`, `+0x5b4`) and take every
one whose class word `[+0x48]` is **2**, a gun — id 5 passing over any whose
slot 2 answers 1 (`0x1002b5c1`). For each it adds

    the gun's +0x174  ×  1000 ÷ max(1, the gun's value 3)

(`0x1002b5d1`–`0x1002b613` and `0x1002b652`–`0x1002b69e`; the 1.0 is
`0x1003b188` and the 1000.0 `0x1003cc8c`). Value 3 is the gun's **interval**
in ms, so the second factor is exactly the stat panel's shots a second.

**`+0x174` is the damage of the last round the gun made.** It has two writers
in the whole module: the constructor zeroes it (`0x100295e8`), and the routine
that creates a round — `World3D.dll!CreateObject` type 9, `0x100296f0` — reads
the new round's property `0x35` and stores it (`0x10029816`). Property `0x35`
falls to case 15 of the control's own property table (index table
`0x1000e5e8`, jump table `0x1000e554`), which is `0x1000e00c`: a call of
`0x10013620` with a level ratio of 1 — **the object's nodes' hit points plus
their explosions' damage**, the same sum `weapons.Gun.round.damage` derives
and the stat panel shows. (Property `0xa5` is its neighbour, `0x100136c0`.)

Two things follow. **A gun that has not fired yet answers 0**, since nothing
else ever writes `+0x174`; and the sum counts **one round a shot**, so a salvo
gun's several barrels do not multiply it, where
`units.Unit.firepower` does. What asks: `Behavior.dll` and `ArealMap.dll`,
which is how the AI and the builder weigh a unit up.

**The rows travel another way** (*read*,
[below](#from-a-row-to-a-command--read-and-measured)). A `.tbl` row is
interpreted in `World3D.dll` (`0x1000fb40`), whose 21-entry table covers the
whole `MCMD` space, and reaches `Control.dll` through `IControl`'s setters and
the component interface at object `+8`. So no field of the `.ctl` frame is
wired to a message: the command a row sets is multiplied by the live top
speed (triple 3) and approached at the live acceleration (triple 1), a queued
turn is paid out at the turn rate (triple 4)
([24-motion.md](24-motion.md#speed-is-a-target-approached-at-a-fixed-acceleration--read)),
a turret's channels move at their own section-2 rates, and a section-2
channel is reached by a component that lists it
([13-control.md](13-control.md#the-entries-are-channels-and-a-devices-inputs--read-and-measured)).

### Messages 20 and 21 are the agent's, not `MCMD_WALK_B` and `MCMD_LOCK`

The same numbering coincidence was drawn a second time, and it does not hold
either. `Control.dll`'s own message dispatcher (`0x10007830`, `IControl` slot
2) takes 1, 4 (attach), 7, 12, the `0x80000020` load and a 9-way table over
**20 to 28** (`lea eax, [edx - 0x14]; cmp eax, 8; jmp [eax*4 + 0x10007d8c]`):

- **20 re-initialises** (`0x10007d58`): `0x10009420` with the argument, then the
  node reset, the weights and the speed limits — the "reset" the live limits
  are recomputed at.
- **21 carries a sub-code** (`0x10007d0f`): with 5, and the time it is given
  matching, it runs the control takt (`0x100059a0`).
- 22–26 do nothing, 27 and 28 call vtable slots 23 and 24.

`AniMesh.dll`'s agent dispatcher (`0x10006fc0`) matches: its 20 (`0x10007277`)
re-initialises and then makes the same two calls as its control-system load
(`0x10007248`), and its 21 switches on a sub-code 2..9 (`0x1000729b`, table
`0x100073a8`); the forwarder at `0x10001320` passes on everything but 21.
**These are the engine's agent messages**, which share small numbers with
`MCMD_`; walking and the lock are the row handler's
([below](#from-a-row-to-a-command--read-and-measured)), which is where
`MCMD_WALK_F`'s handler was found.

**`MCMD_WALK_F` (19) is handled, and this entry used to say it was not.** The
claim was that no `cmp edx, 0x13` exists in any of six modules and that every
range reaching the walk messages starts at 20. Both halves were true and the
conclusion was still wrong, which is worth keeping as a caution: the search
looked for a *constant* in a *dispatcher*, and this dispatch has neither.

`World3D.dll` carries the **whole `MCMD` space in one table**:

```
0x1000fcac   lea eax, [ebp - 1]
             cmp eax, 0x14
             ja  default
             jmp dword ptr [eax*4 + 0x100109f8]
```

Twenty-one entries, messages 1 to 21, and message 19 is entry 18. So there is
no comparison against 19 to find — a range table handles every member of its
span without naming any of them — and the message id is in `ebp`, not `edx`.

**Entry 19 shares its handler with 7, 8 and 20.** `MCMD_FORWARD`,
`MCMD_BACK`, `MCMD_WALK_F` and `MCMD_WALK_B` all jump to `0x100101b2`, which
re-reads the message and separates them itself — `cmp ebp, 0x13` for the
forward walk, `cmp ebp, 0x14` for the back one, the remainder for the two
drive messages. Walking and driving are one routine with a mode, which is why
no dedicated handler existed to find. `MCMD_UP` and `MCMD_DOWN` share
`0x1001059b` the same way.

So the sharpest loose end the numbering exposed was an artefact of how it was
looked for.

The shipped rows show what the sharing is for. A paired message **carries its
sign in its name**: `MCMD_WALK_F` and `MCMD_UP` send only positive magnitudes,
`MCMD_WALK_B` and `MCMD_DOWN` only negative, and each pair is one handler that
reads the message to know which half it is. `MCMD_FORWARD` does the opposite —
it carries −1, 0 and 1 by itself, and **`MCMD_BACK` is sent by no shipped
row at all**. So driving reverses by sign on one message and walking by a
second message, through the same code.

### Three schemes

`hero.tbl`, `m1.tbl` and `m2.tbl` are the pilot on foot and two machine
schemes, and `hero.man`, `table_1.man` and `table_2.man` sit beside them by
name. The names pair them; the contents do not settle it, since all three
tables draw on the same command families and `hero.tbl`'s named actions match
all three `.man` files equally well (17 each).

## From a row to a command — *read*, and *measured*

`World3D.dll`'s row handler (`0x1000fb40`) turns a row into one of three calls:

| row | call | what it holds |
|---|---|---|
| `MCMD_WALK_F` / `WALK_B` / `FORWARD`, class 0 | `IControl` `SetTangAccel` (`Control.dll:0x100043d0`) | the command triple `+0x1bc`, y forward, in top speeds |
| `MCMD_LEFT` / `RIGHT`, class 0 | `SetTangAccel` and `SetStrafeAngle` (`0x10004560`, `+0x1f4`) | walk, and the strafe angle |
| `MCMD_ANGLE_X/Y/Z`, class 0 | `SetNormAngle` (`0x10004500`) | the unit's normalised angle triple `+0x1e0` |
| `MCMD_ANGLE_X/Y/Z`, class *c*, index *i* | the component interface (object `+8`, slot 14, `0x1002eb70`) | that component's triple: a turret's `+0x9c`, a camera's `+0x94` |

The weapon rows go to the guns, not to `IControl`. `MCMD_STATE` with index −1
reaches every **selected** gun, and `MCMD_SELECT` toggles which guns are
selected ([29-weapons.md](29-weapons.md#the-button-reaches-the-selected-guns)).

**An angle row edits one component of a triple.** `ANGLE_X`, `_Y` and `_Z` are
components 0, 1 and 2 (`0x1000fee1`, `0x1000ffcc`, `0x100100b7`). The handler
first reads the current triple.

- **Mouse and joystick rows add** to the component (scan codes `0x1fe`,
  `0x1ff` and `0x24e`–`0x253`).
- **Every other row sets it.** So a key row's 0.5 is a position, not a step.
- The state field then applies:
  - `MAN_WRAP` wraps the value into [0, 1);
  - `MAN_NOTWRAP` clamps it to [0, 1].

**What a mouse row adds** (`0x10010a50`, filter at `0x1000f24b`):

    m  = counts × sensitivity × 0.95 + 0.05 × m_previous      (Y also × 1.2)
    Δv = clamp(m × 0.006, −1, 1) × invert × magnitude

- `sensitivity` (`0x100234d0`) is `Iron_3D.ini`'s `MOUSE_SENS` × 0.01
  (*read*, [below](#the-ini-reaches-world3d--read)). The shipped ini says
  100, so it is 1.0 (*measured*).
- `invert` is ±1: mouse Y's (`0x100234f0`) follows `MOUSE_REV_Y`. Mouse X's
  (`0x100234ec`) is +1 in the shipped game. `SetInverseMotion`
  (`0x10010e10`) and setting `0x69` could change it, but no module imports
  the export and `iron3d.dll` never sends the setting. The two globals earlier
  read as sensitivities are these signs.
- A joystick axis adds counts × invert × `JOY_SENS` × 0.01 × magnitude.

### The ini reaches World3D — *read*

`World3D.dll`'s `CreateGameSettings` object keeps a table of handlers by
group (slot 9, `0x1000a640`). Slot 2 (`0x1000a3b0`) passes a setting on to
its group's handler. The id is a dword, the group in its low word and the
setting in its high one. `World3D.dll` registers group 10 for itself
(`0x10014cca`), and that handler's slot 2 (`0x1000a820`) switches on the
setting (`0x5a`–`0x6e`, table `0x1000aae0`).

`iron3d.dll` reads `Iron_3D.ini` at load (`0x10061310`) and sends it on:

| ini key | sent as | lands in (`World3D.dll`) |
|---|---|---|
| `MOUSE_SENS` × 0.01 | group 10, `0x66` | `0x1000aa1d` → `0x100234d0`, the mouse filter's multiplier |
| `JOY_SENS` × 0.01 | group 10, `0x67` | `0x1000aa31` → `0x100234d4`, the joystick's |
| `MOUSE_REV_Y` ≠ 0 | group 10, `0x6a` | `0x1000aa6a` → mouse Y's sign `0x100234f0` = −1, **and** joystick X's `0x100234f4`: the case falls through into `0x6b`'s |
| `JOY_REV_Y` ≠ 0 | group 10, `0x6c` | `0x1000aaa9` → joystick Y's sign `0x100234f8` |
| `EMBOSS_BUMP` | group 10, `0x6d` (and group 30, not `World3D.dll`'s) | `0x100234e8` |
| 1 | group 10, `0x65` | `0x100234e0` |

The handler's slot 4 (`0x1000adf0`) answers 0.5 for both sensitivities, and
slot 5 answers 2.0: a slider's range, by the look of it (*guess*).
`iron3d.dll:0x10061a50` sets the mouse sensitivity to that 0.5 while the view
is zoomed and puts the ini's value back otherwise: `0x100a4fc0` hands it the
zoom of the view state `+0x710` names, the command camera's in state 2 and the
unit or building record's in states 1 and 6, and none in any other
([30-turrets.md](30-turrets.md#the-zoom--read-and-measured)).
`iron3d.dll:0x1002a64d` is another `MOUSE_SENS` reader, into an options
object's `+0x724`.

### A row that stays down — *read*

The table reader (`0x1000b601` pushes its format, `'%s %s %d %s %s %f %d %s %f %d'`)
keeps each row as a 0x88-byte record in one of three tables:

| Offset | Field |
|---|---|
| +0x04 | the modifier's scan code |
| +0x5c | the key's |
| +0x60 | 1 on the press row, 0 on the release row |
| +0x64, +0x68 | class, `MCMD_` command |
| +0x6c, +0x70, +0x74 | magnitude, index, state |
| +0x78, +0x7c | ramp, ramp time in ms |
| +0x80, +0x84 | active, and the game time it became so |

**A key event activates its key's matching row and clears the other**
(`0x1000f5a4`). A going-down event sets the press row active, stamped with
the game clock (`SetGameTime`'s `0x10032a38`), and clears the release row.
Coming up does the opposite.

**Every active row is handed to the row handler on each update**
(`0x1000f477`–`0x1000f514`), the key events after that (`0x1000f5a4`), so a
keyboard row an event makes active runs on the next update. A mouse button's
row runs as it goes active too (`0x1000f774`). **Then the handler clears a row
with no ramp time** (`0x100109e7`): its last step takes the row's ramp time
(the context's walk or turn ramp's in its place for a walk or `MCMD_ROTATE_Z`
press row while that ramp is on), and a 0 sets the row's `+0x80` to 0. So a
row without a ramp time runs once each time its key goes down or comes up, and
only a ramp row runs again while it is held. What it does with a key row's
value is the axis function's (`0x10010a50`):

- **no ramp time**: the row's magnitude;
- **a ramp time**: the current value moved toward the magnitude by
  ramp × min(1, held ms ÷ ramp time), never past it. A release row clears
  itself once it arrives; a press row stays active while its key is held.

So the keypad's `+` and `−` steer the cruise command by a step per update
([24-motion.md](24-motion.md#from-input-to-motion--read-and-measured)).

**The update runs once a game frame** (*read*). It is slot 4 of the manual
manager, and the manager's own message handler (slot 2, `0x1000ec80`) is what
calls it, on **message 1** and while `+0x32` is set (`0x1000ecd5`). The
message reaches it down one chain: `iron3d.dll`'s mission loop calls
`stdCalculateGame` (`World3D.dll:0x100139a0`) once a pass; that runs the game
object queue's slot 4 (`0x10006bf0`), which reads `timeGetTime` into the game
clock `0x10032a38` (`0x10006c44`) and sends every object
`send(6, 1, clock)` (`0x10006c6f`); the agent's id-6 arm forwards message 1 to
its five sub-objects (`AniMesh.dll:0x100013a1`), the Wizard at `+0x178` among
them; and the Wizard's message 1 hands it on to the manual manager it keeps at
`+0x64` (`Wizard.dll:0x10001cc5`), while its mode `+0x1fc` is 1 and `+0x1f6`
is clear. **The mission loop is not capped** (`iron3d.dll:0x1005e713` to
`0x1005ef9e`): each pass clears the event list, pumps the window's messages,
calculates and renders, and ends with `Sleep(0)`; it sleeps 100 ms *instead*
of rendering when the render flag is clear (`0x1005ed55`), and there is no
timer, no frame count and no `iron_3d.ini` setting anywhere in it. So the
input update runs **once a rendered frame, at whatever rate the machine
draws**, and the keypad ramp is that much faster on a faster machine.

**Two gates sit on it.** The manager keeps the clock of its last update at
`+0x18` and passes the whole update over when the message brings the same
value (`0x1000ec90`), so it never runs twice in one millisecond; and it keeps
the count of `stdCalculateGame` calls at `+0x1c` (the counter `0x107951a8`,
advanced at `0x100139c4`) and, when more than one has gone by since, clears
its state and calls `stdClearKeyboard` (`0x10011830`, `0x1000ecb7`) — so a
machine that misses a frame's message 1 lets go of every key.

The context object holds a second ramp for each of walking and turning
(`+0x3d`/`+0x44`/`+0x4c`/`+0x50` and `+0x3e`/`+0x40`/`+0x48`/`+0x54`). The
walk messages (19, 20) and `MCMD_ROTATE_Z` use it in place of the row's while
it is switched on. It is set through the manager's slot 11 (`0x1000b380`),
whose kind of 1 turns the walking ramp on and 2 the turning one.

**Nothing calls it** (*read*, with a control). Slot 11 is at `+0x2c` of the
manager's own vtable (`0x10020b14`, fourteen slots, installed at
`0x1000b299`), and it takes five stack words — `this`, the kind and the three
values — from its `ret 0x14`. Its body has no reference anywhere in
`World3D.dll` outside the vtable, so every call must go through `+0x2c`. Of
the **345** indirect calls at `+0x2c` across the whole install, **none** has a
receiver that could be this object: no site loads its object from a field a
manual manager is kept in, and the one site that comes close
(`iron3d.dll:0x10039254`) calls `+0x38` on the same receiver, a slot a
fourteen-slot vtable does not have.

**The control** is the same enumeration run at other offsets of the same
vtable. At `+0x8`, slot 2, it finds all three places a manual manager is sent
a message — `AniMesh.dll:0x1000178a` and `0x10001b4c`, the agent forwarding to
its own at `+0x184`, and `Wizard.dll:0x10001cc5` at the Wizard's `+0x64`; at
`+0x14`, slot 5, it finds `Wizard.dll:0x10003aa6` and `0x10003beb`, the AI
poking keys in; at `+0x10`, slot 4, it finds `World3D.dll:0x1000ecd5`, the
manager running its own update. The search sees calls on this object wherever
one is held, and at `+0x2c` there are none.

The flags agree. `+0x3d` and `+0x3e` are written in exactly three places in
`World3D.dll`: the constructor clears both (`0x1000b2d1`, `0x1000b2d4`), the
table load clears them again (`0x1000b47a`, `0x1000b47d`, inside slot 3
`0x1000b3e0`, the reader that defaults to `M1.TBL`), and slot 11 sets one of
them. They are read at `0x100109a2` and `0x100109c4`, where the handler picks
the ramp time, and at `0x10010bda` and `0x10010c21` in the axis function.

**So every walk and every turn takes the row's own ramp time from the
`.tbl`** — which on all three shipped tables is 1000 ms on the keypad's `+`
and `−` and 0 everywhere else — and the second ramp is dead code in the
shipped game.

**The hero's rows** (*measured*, `hero.tbl`):

- **Mouse X** turns the unit: `ANGLE_Z`, 0.15, wrapping, class 0.
- **Mouse Y** tilts the turret: `CICLS_TURRET` index 1, `ANGLE_Y`, 0.25,
  clamped.
- **The machine tables** `m1.tbl` and `m2.tbl` send mouse X to the turret
  instead (`ANGLE_X`).
- **The table is the chassis's.** The hero chassis record `r_h_02` names
  `hero.tbl`; `r_l_06` names `m2.tbl`.

### A chord with no row of its own — *read*, and *measured*

Shift+W has no row in any shipped table. **It walks**: the lookup is an exact
match on the chord, and a row with no modifier answers for every chord *except*
the ones that another row of the same key claims.

**What the load builds** (*read*). The manager keeps, beside its three row
tables, one list of every modifier any row uses: the count at `+0x8830`, the
scan codes at `+0x8834`, each one's **held flag** at `+0x8884`, the rows that
use it at `+0x88d4` (200 to a modifier) and their counts at `+0xc754`. A row
with a modifier is registered there as it is read (`0x1000c080`). Then, once
the file is read, a pass over that list (`0x1000bd31`–`0x1000beb9`) takes each
modifier, walks the rows that use it, and for every **plain** row of any of the
three tables **whose key is the same** appends that modifier's scan code to the
plain row's own list at `+0x08`, counted at `+0x58`. So a plain row ends up
carrying the modifiers that would steal its key.

**What a key event does** (`0x1000f5ee`–`0x1000f6d9`). For each row of the
table the event's kind selects:

1. row `+0x5c`, the key, must be the event's code, else the row is passed over;
2. row `+0x60`, press or release, must be the event's, else the row's active
   flag `+0x80` is **cleared** and the row passed over;
3. then the modifier:
   - **a row with one** is made active only while that modifier's held flag is
     set (`0x1000f634`);
   - **a plain row** is made active only while **none** of the modifiers in its
     own list is held (`0x1000f682`–`0x1000f6d9`).

Making a row active is `+0x80 = 1` and `+0x84 =` the game clock, as
[above](#a-row-that-stays-down--read). The same exclusion gates the input
update's pass over the rows already active (`0x1000f2f0`, `0x1000f39e`,
`0x1000f3e5`), so a plain row does not keep running once its key is claimed.

**And a modifier going down re-decides the rows that use it** (`0x1000f950`,
called from the input update at `0x1000f2be`). For each queued event the pass
looks the code up in the modifier list; on a hit it writes the event's pressed
word into that modifier's held flag (`0x1000f9c3`) and then, for each row that
uses the modifier, matches the row's key and press field against the **key's own
held byte** (`0x1002a490 + code`) and sets or clears the row's active flag
(`0x1000fa12`, `0x1000fa39`). So pressing Shift while the mouse is already
moving hands the movement to the Shift row without waiting for a fresh key
event.

**So there is no best match and no search.** A chord whose modifier no row of
that key uses falls through to the plain row; a chord whose modifier some row
of that key does use fires that row, and the plain row is silent; and a
modifier that a *different* key's row uses changes nothing.

*Measured*, over the three shipped tables (116 rows):

- **`SCAN_LSHIFT` is the only modifier any of them uses**, on 4 rows of each
  table — mouse X, mouse Y and the right button twice.
- The load's pass therefore writes **two entries per table**: `SCAN_LSHIFT`
  onto the plain mouse X row and the plain mouse Y row. Every other plain row's
  list is empty.
- `SCAN_RMOUSE` has a Shift row and no plain row, so there is nothing to
  exclude; the plain right button is `iron3d.dll`'s, out of `ui_other.man`.

So in the shipped game the rule shows itself in exactly one place: holding
Shift moves the camera with the mouse instead of the hull and the turret
([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)).
Shift with any key of the keyboard reaches the plain row, W included.

### Leaving the window lets every key up — *read*

A key that goes down in the game and comes up while another program has the
keyboard sends the game no key-up. The game does not wait for one: switched
away from, it lets go of everything held.

- **The message.** The window procedure (`iron3d.dll:0x100a0bf0`) sends
  `WM_ACTIVATEAPP` (0x1c) through its jump table to `0x100a0d60`. With no
  shell (`getIShell`, `0x100074a0`) and a game (`getIGame`, `0x1005b580`),
  that hands the message's active flag to `World3D.dll`'s
  `stdSetApplicationState` (`0x100a0e1f`, through the import thunk
  `0x100cd186`). A mission always has no shell: `iron_3d.exe`'s WinMain
  deletes it (`0x40128f`, `deleteShell` clearing `0x1010b5fc` at
  `iron3d.dll:0x1005b621`) before it creates the game.
- **Only a change acts** (`World3D.dll:0x100145f0`). An unchanged state
  returns at once. A changed one is stored at `0x10023500`, and the DirectInput
  device `stdInitDIMouse` reads (`0x10795138`) is acquired or unacquired
  (`0x10014621`, `0x10014629`).
- **Every held key is queued up** (`0x10014688`–`0x100146d0`). Each code below
  700 whose held byte (`0x1002a490` + code) is set has the byte cleared and
  gets an event with a pressed word of 0. It is the same three words the key
  handler queues (`0x1001111b`–`0x10011133`): the pressed word, set from the
  held byte, then the code, then a kind (1 for codes 0x1fe–0x203, 2 for
  0x204–0x28a, otherwise 0).
- **What it does to the hero** (*derived*). The input update takes each such
  event as a key coming up (`0x1000f5a4`, above), so each held key's release
  rows run:
  - the walk stops;
  - the fire button's release turns the selected guns off;
  - Shift's two release rows set the camera's free look back to 0.5, so the
    view centres on the sight again
    ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)).

## `BuildDat.lst`

Not input, but the same kind of file and read here for it: the schemes the
behaviour system builds fortifications from, each a role and the assemblies
that fill it at each size.

```
Bunker_Small 3
  "UNITS\BUILDS\BUNKER\sbunk01.dat"
  ...
```

**All 32 assemblies it names exist on disk.** Its own header says *"There must
be 11 schemes"* — and it ships **12**: the eleven, plus `Tower_Large`. The
engine reads all twelve: `ArealMap.dll:0x1001ce90` registers each name with a
building Type, `Tower_Large` included, so the header's count is stale. A
scheme's list is that Type's upgrade ladder ([32-builder.md](32-builder.md)).

## The configuration beside them

The other text files in the root are the engine's own configuration —
`Comp.ini`'s component registry, `Behavior.ini`'s and `ArealMap.ini`'s debug
switches, `Iron_3D.ini`'s display settings — and they are now
[read](22-settings.md). They configure the engine rather than describing data,
so nothing in this toolkit acts on them; it reads them and says which module
owns each.
