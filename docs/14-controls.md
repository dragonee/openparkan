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
| 2, 3 | the chord: a modifier (`SCAN_NULL` on 104 of 116 rows) and the key |
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
| 5, 6 | the guns' `+0x174` × 1000 ÷ interval, 5 skipping guns in state 1 | | 14 | mean shield fill × Σ deflector values 0–5 × shield value 0 |
| 7 | the fight shield's mean sector fill | | 15 | Σ deflector values 0–5 × shield value 0 |
| 8 | the radar's value 3, its range | | 16 | deflector value 0 × shield value 0 |

Its callers ask for it after a `QueryInterface` for `0x204`: `Behavior.dll`
for 1, 2, 5, 6 and 7 (id 1 at `0x100180bd`), `ArealMap.dll`
for 6, `iron3d.dll` for 2, and a gun, on the round it fires, for 10–12
(`Control.dll:0x1002986c`). The ids are listed in `control.DEVICE_QUERIES`.

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

- `sensitivity` (`0x100234d0`) is `Iron_3D.ini`'s `MOUSE_SENS` × 0.01. The ini
  read is at `iron3d.dll:0x1002a64d`. That it is the same value that reaches
  `World3D`'s setter (`0x1000aa1d`) is a *guess*: the handoff was not traced.
- `invert` is ±1, from `SetInverseMotion` (`0x10010e10`). The two globals
  earlier read as sensitivities are these signs.
- A joystick axis adds counts × invert × `JOY_SENS` × 0.01 × magnitude.

**The hero's rows** (*measured*, `hero.tbl`):

- **Mouse X** turns the unit: `ANGLE_Z`, 0.15, wrapping, class 0.
- **Mouse Y** tilts the turret: `CICLS_TURRET` index 1, `ANGLE_Y`, 0.25,
  clamped.
- **The machine tables** `m1.tbl` and `m2.tbl` send mouse X to the turret
  instead (`ANGLE_X`).
- **The table is the chassis's.** The hero chassis record `r_h_02` names
  `hero.tbl`; `r_l_06` names `m2.tbl`.

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
