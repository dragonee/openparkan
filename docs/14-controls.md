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

## The join with the controller

This closes something the [controller](13-control.md) left open. Its message
dispatch is a 16-way jump table — `lea eax, [edx - 1]; cmp eax, 0xf; ja …;
jmp [eax*4 + 0x1002b9e8]` — and **`MCMD_` 1 to 16 is exactly that range**, in
order. Entry by entry:

| Msg | Command | What the handler touches |
|---:|---|---|
| 1 | `MCMD_STATE` | `+0x5b4` |
| 2–4 | `MCMD_ROTATE_X/Y/Z` | `+0x5b4`, then locals |
| 5–6 | `MCMD_ANGLE_X/Y` | locals |
| 7 | `MCMD_FORWARD` | `+0x5c0` |
| 8, 9 | `MCMD_BACK`, `MCMD_LEFT` | `+0x5c8`, indices 3 and 4 |
| 10, 11, 12 | `MCMD_RIGHT`, `UP`, `DOWN` | `+0x5cc`, indices 1, 0, 2 |
| 13 | `MCMD_SELECT` | `+0x5bc` |
| 14, 15, 16 | `SELECT_NEXT`, `TABLE`, `ANGLE_Z` | `+0x5c0` |

The two slots hold different classes: `+0x5c8` is a 0xf4-byte object whose
constructor lays out channels of stride 0x1c defaulting to 0.5, `+0x5cc` a
0x120-byte one with two arrays of six floats defaulting to 1.0 and 0.0.

**The three commands the shipped tables use that fall outside 1..16 are
`MCMD_WALK_F` (19), `MCMD_WALK_B` (20) and `MCMD_LOCK` (21)** — 14 of the 116
rows. The movement controller cannot be what handles them, so walking a
machine is somebody else's job. That is the strongest thing the numbers say,
and where the next dig starts.

What is still not established is the step past this: which **field of the
`.ctl` frame** feeds which of those channels. Knowing that message 11 is
`MCMD_UP` and reaches index 0 of the object at `+0x5cc` does not say which of
the frame's six triples that object was loaded from.

### Three schemes

`hero.tbl`, `m1.tbl` and `m2.tbl` are the pilot on foot and two machine
schemes, and `hero.man`, `table_1.man` and `table_2.man` sit beside them by
name. The names pair them; the contents do not settle it, since all three
tables draw on the same command families and `hero.tbl`'s named actions match
all three `.man` files equally well (17 each).

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
be 11 schemes"* — and it ships **12**: the eleven, plus `Tower_Large`. Whether
the engine reads the twelfth is not established here, and the discrepancy is
recorded rather than resolved.

## What is not read here

The other configuration files in the root are text of the same kind and are
not parsed: `Comp.ini` (the component registry — `CID_CLASSIC_LANDSCAPE 0
terrain.dll LoadLandscape` through `CID_RESEARCH 7`), `Behavior.ini` and
`ArealMap.ini` (logging and debug switches, including `ImmortalHero`,
`DeterminMode` and `SaveLog`), and `Iron_3D.ini`. They configure the engine
rather than describing data, and nothing in this toolkit acts on them yet.
