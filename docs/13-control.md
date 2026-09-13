# The controller — `.ctl`

The fifth slot of a `STAT` record, and the file that says how a thing *moves*.
`Control.dll` exports `LoadControlSystem`, and `AniMesh.dll` — the animated
mesh, the engine's "agent" — is the only module that imports it. Everything a
static picture needed lived in the mesh, which is why this file went unread
until the renderer was finished.

**531 members** carry the tag `CTLD`, spread over ten archives:

| Archive | Members | | Archive | Members |
|---|---:|---|---|---:|
| `intsys.rlb` | 128 | | `fortif.rlb` | 30 |
| `guns.rlb` | 124 | | `bases.rlb` | 24 |
| `static.rlb` | 81 | | `parts.rlb` | 13 |
| `weapon.rlb` | 66 | | `animals.rlb` | 5 |
| `turrets.rlb` | 58 | | `system.rlb` | 2 |

Everything below is re-derived by `uv run openparkan verify`.

## The 212-byte frame

Every member begins with a **128-byte frame**: five counts, then a 27-dword
parameter block. An 84-byte block follows it *in the file only when nothing
lies between them* — sections 1, 2 and 4 sit in the gap. Six members have
nothing in the gap and are exactly 212 bytes, `0xFF` from +128 to the end,
the same "not set" fill the `MAT0` loader uses. That is why 212 is the floor,
and why the 84-byte block read as a trailer until the layout was recovered.

```
0x00  int32[5]   section counts
0x14  float[3]   +20   triple 1   acceleration
0x20  float[3]   +32   triple 2
0x2c  float[3]   +44   triple 3   top speed, m/s
0x38  float[3]   +56   triple 4   turn rate; two-pi on 364
0x44  float[3]   +68   triple 5   1.0 on 418
0x50  float[3]   +80   triple 6   two-pi on 422
0x5c  int32      +92   0 on 324, else 1000, 2000, 5000
0x60  float[2]   +96   zero on 512
0x68  int32      +104  0, 2 or 3; 2 brakes on slopes
0x6c  float      +108  -1.0 on 465
0x70  float      +112  the slope cone; pi/2 on 509
0x74  int32      +116  0 on 342, else 3, 4, 16
0x78  float      +120  -1.0 on 433
0x7c  float      +124  payload, kg; FLT_MAX on 502
0x80  ...        +128  the 84-byte block, or 0xFF
```

**The block is floats where it says it is.** All **12744** reads across its 24
float slots, over all 531 members, are finite — the same witness the effect
blocks were held to, and the reason the slot map above can be trusted without
a single named field.

**The six triples are per-axis.** Their three components hold the same value
on 440, 521, 438, 484, 504 and 502 members respectively — 2889 of 3186 — which
is what a default written across `x`, `y` and `z` looks like.

**The defaults are the engine's own.** A whole turn (the file's `6.28`) on all
three components of triple 6 on 422 members; `pi/2` at +112 on 509; `FLT_MAX`
at +124 on 502; `-1.0` at +108 and +120 on 465 and 433. A limit of a whole
turn is the same as no limit, which is why it reads as a default rather than
as a rate.

## `IControl`, named by the people who wrote it

`Terrain.dll` carries a table of stub implementations at `0x100191a0`, one
every 32 bytes, each of which does nothing but log its own name. That table is
the interface, in order, with its arity given by each stub's `ret`:

| Method | Stack args | | Method | Stack args |
|---|---:|---|---|---:|
| `SetTangAccel` | 8 | | `SetTargetObject` | 12 |
| `SetNormSpeed` | 8 | | `GetControlPlace` | 12 |
| `SetTangSpeed` | 8 | | `SetControlPlace` | 12 |
| `SetWorldSpeed` | 8 | | `SetBaseProperty` | — |
| `SetWorldAccel` | 8 | | `SetNetSynchroMode` | — |
| `SetControlCalculationMode` | 8 | | `MakeMovementCorrection` | — |
| `SetWorldDirect` | 8 | | `SetObjectTransformation` | — |
| `SetStrafeAngle` | 8 | | `SetNormAngle` | 4 |

So the engine's own vocabulary for movement is **tangential**, **normal** and
**world** — a speed in each of the three (`SetTangSpeed`, `SetNormSpeed`,
`SetWorldSpeed`), an acceleration in two of them, two angles, and a
calculation mode. That is a better description of the file's shape than
`(x, y, z)`: a triple whose components are usually equal fits *tang, norm,
world* exactly as well as it fits three axes, and the interface names the
former.

Two matches are worth stating and neither is proved. `SetControlCalculationMode`
is a mode, and the int at **+104** takes exactly three values across the 531
members — 0 on 509, 2 on 16, 3 on 6. And the two triples that default to a
whole turn are the file's only angular quantities, against the interface's two
angle setters, `SetNormAngle` and `SetStrafeAngle`.

The stubs are dead ends for the mapping itself: each is a "not implemented"
log, so no argument ever reaches one. Whoever calls them with values out of a
`.ctl` is in `AniMesh.dll`, and that is where a field-by-field mapping has to
come from.

### What the numbers look like

**Three of the triples are now named** ([24-motion.md](24-motion.md)):
`Control.dll:0x1000fca0` makes the live limits from triple 1 (+20, the
acceleration, doubled), triple 3 (+44, the top speed in m/s) and triple 4
(+56, the turn rate). Triple 5 is multiplied into the spin integrator
(`0x10014b15`); triples 2 and 6 are not named. Three members side by side:

| | `ctl_cam_fly` | `o_c01_l_01` (a gun) | `fr_b_plant` (a factory) |
|---|---|---|---|
| +20 | 20, 20, 20 | 2.5, 2.5, 2.5 | 1, 1, 1 |
| +44 | 100, 100, 100 | 0, 0, 0 | 0, 0, 0 |
| +56 | 6.28, **0.6**, 6.28 | 6.28, 6.28, 6.28 | 0, 0, 0 |
| +80 | 6.28, 6.28, 6.28 | 6.28, 6.28, 6.28 | 6.28, 6.28, 6.28 |
| +92 | 1000 | 0 | 1000 |

The flying camera is the legible one: 100 on triple 3 against 20 on triple 1
is a speed against an acceleration, and the **0.6 in the middle component of
triple 4** is the one place in 531 files where a single axis's turn rate is
set on its own. This page once read it as a pitch limit; triple 4 is a rate,
so it is a slow turn about one axis.

## The frame is the live object's parameter block

The earlier note that "the file is that object written out" is now exact.
`Control.dll`'s initialiser at `0x10006689` writes a default into every field
of the controller it builds, and **the value it writes is the commonest value
in the shipped files, on all 27 slots**:

| File | Object | Default written | Files carrying it |
|---|---|---|---:|
| +20, +24, +28 | `+0x470`–`+0x478` | 2.5 | 324 |
| +32, +36, +40 | `+0x47c`–`+0x484` | 0 | 518–527 |
| +44, +48, +52 | `+0x488`–`+0x490` | 0 | 434–505 |
| +56, +60, +64 | `+0x494`–`+0x49c` | 6.28 | 364–365 |
| +68, +72, +76 | `+0x4a0`–`+0x4a8` | 1.0 | 418–444 |
| +80, +84, +88 | `+0x4ac`–`+0x4b4` | 6.28 | 422–451 |
| +92 | `+0x4b8` | 0 | 324 |
| +96, +100 | `+0x4bc`, `+0x4c0` | 0 | 512 |
| +104 | `+0x4c4` | 0 | 509 |
| +108 | `+0x4c8` | -1.0 | 465 |
| +112 | `+0x4cc` | 1.57079 | 509 |
| +116 | `+0x4d0` | 0 | 342 |
| +120 | `+0x4d4` | -1.0 | 433 |
| +124 | `+0x4d8` | FLT_MAX | 502 |

So **object offset = file offset + 0x45c**, and a shipped controller that
leaves a slot alone is carrying the engine's own compiled-in value rather than
anything an artist chose. The five counts below +20 are not part of the block:
the object keeps pointers in those slots (`+0x45c` holds one the constructor
sets, `+0x46c` another that later code follows).

Two of the constants are worth noticing, because they are not the mathematics
they look like. The "whole turn" is **6.28**, and the "half cone" is
**1.57079** — decimals somebody typed, not `2*pi` and `pi/2`. The reader keeps
the engine's bits rather than the real values, since the point is to match the
file.

This does not name the fields — [24-motion.md](24-motion.md) names the ones
the motion code uses. What it does settle is that there is nothing else to find about the
*shape*: the frame has no hidden structure, it is one C++ object's floats, and
anything further has to come from what reads them.

### How the block is loaded, in the engine's own order

The copy is at `0x10008ea0`. It reads **five `int32`** from the record one at a
time, then:

```
lea eax, [ebx + 0x470]        ; the object's parameter block
mov ecx, 0x1b                 ; 27 dwords -- 108 bytes
mov esi, ebp                  ; the record, now at +20
mov [ebx + 0x46c], ebp        ; keep a pointer to the source
rep movsd
...
add ebp, 0x6c                 ; on to the next record, 128 bytes on
```

Five ints and 27 dwords is 128 bytes, read in exactly the order this document
gives them. That is the frame confirmed from the read side as well as from the
defaults.

Note what it keeps at `+0x46c`: a pointer to the **source** record at file +20,
not to the copy. Thirteen sites later follow it, so an offset `[ptr + N]` in
that code is file offset `20 + N`.

### What reads the block

Every access to the block in `Control.dll` outside the initialiser, by file
offset:

| File offset | How it is used |
|---|---|
| +0 | written, and its address taken |
| +4, +8 | read, 5 sites |
| +16 | the source pointer — 13 reads, 1 write |
| +20 | address taken (the copy's destination) |
| +44, +48, +52 | `fcomp`, and multiplied into the live top speed (`0x1000fd5d`); +48's address is also taken |
| +56 | `fcomp` |
| +88, +104 | read as dwords |
| +92 | read, 3 sites |
| +108 | read, 5 sites |
| +116 | `test byte ptr [ptr + 0x60], 1` — a **bitfield**, bit 0 |
| +124 | 8 reads and **2 writes** in the integrator at `0x1000f412` |

Two things follow. **`+124` is the payload**, in kg. This page once called it
not a parameter because it is written at run time; the writes are to the live
copy at `+0x4d8`, which holds the payload still spare
([24-motion.md](24-motion.md#load--read-and-measured)). And **`+116` is a bitfield**: every one of the 531 values is below
32, bits 0 to 4 are used, and bit 0 — the one the engine tests — is set on 111.

**The list of slots "never touched" that stood here was wrong.** It followed
only the `+0x46c` pointer and plain displacements. The motion body keeps two
pointers of its own (`+0x1ac` authored, `+0x1b0` live), and code steps through
them with pointer arithmetic: +24, +28, +60 and +64 are read at `0x1000fca0`,
+68 to +76 at `0x10014b15`, +112 at `0x100157ac`. No untouched list is claimed
now.

### The property interface is not where the names are

The object carries a property interface — `[esp+0x1c] - 1`, `cmp eax, 0xb3`,
so **ids 1 to 180**, dispatched at `0x1000dcc0` through a byte index table at
`0x1000e5e8` into a jump table at `0x1000e554`. It looked like the place field
names would come from. It is not, and this is the count rather than the spot
check that stood here before.

There are **37 distinct cases**. **143 of the 180 ids fall to the default at
`0x1000e002`**, which returns zero — so the class implements 37 properties out
of an interface-wide id space of 180. Of those 37:

- **six reach the parameter block**, and they are three fields through three
  get/set pairs: **17 and 18** → file **+20**, **144 and 145** → file **+48**,
  **136 and 137** → file **+124**. One of each pair goes through the source
  pointer at `+0x46c`, the other through the copy at `+0x470`, and both land on
  the same field.
- **twelve hand out something else in the object** — `+0x2c`, `+0xf0`,
  `+0x1a0`, `+0x1bc`, `+0x1c8`, `+0x1d4`, `+0x1e0`, `+0x1f4`, `+0x200`,
  `+0x538` (twice) and `+0x660`.
- **nineteen are not a plain pointer** at all: they compute or convert rather
  than hand out a field.

So the interface exposes **three** of the block's thirty-two slots; the
motion code reads the rest directly.

## The sections

The loader at `0x10008b10` reads the five counts one at a time, copies the
parameter block, and then walks the body. That walk is the layout:

| Order | Governed by | Size |
|---|---|---|
| section 1 | `counts[0]` = A, `counts[1]` = B | A states of `156 + 16*B`, then an `A*A` table of floats |
| section 2 | `counts[2]` = C | C records of 36 bytes; +20 a node, where a gun's barrels sit |
| section 4 | `counts[3]` = D | D component records, each **type-dispatched** |
| the block | — | a fixed **84 bytes**, copied into the object |
| section 5 | `counts[4]` = E | E groups: an int32 `n`, then `n` records of 100 bytes |

Section 1's span is worth showing because the engine writes it twice. The
parse walks A records of `16*B + 156` and then adds `A*A*4`; the skip path
computes `A * (A + 4*B + 39) * 4` in one go. Those are the same number, which
is how the arithmetic was confirmed before any file was opened.

**This settles two old misreadings.** The "body of 156-byte records" in the
original note was section 1 with B = 0 — right about the number, wrong about
what it counted. And the "84-byte trailer" is not padding: the loader `rep
movsd`s it into the object and then reads section 5 after it.

Checked against the data: **all 531 members are consumed to the byte**, once
the component record below is added.

## The component record

A section-4 record is one shape for every type. The factory at `0x1002d4b0`
dispatches its first `int32` — **a type id from 1 to 30** — through a byte
index table at `0x1002d864` into fourteen cases, which build objects of
fourteen different sizes. But thirteen of the fourteen classes parse their
record with the *same* code at `0x10021d50`, and the fourteenth (type 2, which
type 30 shares) calls that code first and then does more with the object. So
the record's extent is common to all of them:

```
0x00  int32      type id, 1..30
0x04  int32      the model node it sits on, an index into the object's .ndp
0x08  uint32     flags: on a gun 0x2000000 fires every barrel at once
0x18  int32      the initial state (CIS_ switch values); -1 keeps the class's default
0x20  float      power: a consumer's draw a second, a battery's output
0x2c  float[16]  the component's values, copied whole into the object
0x6c  char[32]   archive        what this part emits
0x8c  char[32]   member
0xac  int32      N -- how many 4-byte entries follow
0xb0  int32[N]   the entries
      int32      L
      char[L+1]  a label, where L is not zero
```

**All 531 members now walk end to end**, 1066 component records among them,
every one carrying an id in 1..30 — 20 of the 30 ids are used.

**The type id is the engine's `CICLS_` class.** Every label family below sits
on exactly one id, and each family the input tables have a name for sits on
that name's number: `i_pws` on 19 (`POWERSTOR`), `i_fsh` on 9, `i_dsh` on 10,
`i_eng` on 5, `i_rdr` on 8, `i_rps` on 15, and every `i_cNN` gun on 2
(`MULTIGUN`). The float at `+0x20` is the class's **power figure** — zero on
all 64 turrets, 152 guns and every door and computer, 0.01 on each building's
efficiency component, 5 to 1000 on batteries — and each class draws on one of
six power channels served in a fixed order. The int at `+4` is the part's
**node**: all 781 components beside a same-named `.ndp` index inside it, and
the part's powers scale with that node's remaining life. An engine's draw
divides its speed by the largest component of the frame's **third triple**,
the per-axis top speed — the flying camera's 100. See
[23-economy.md](23-economy.md#a-power-shortage-lowers-efficiency-once-the-batteries-run-down--read-and-measured)
and [24-motion.md](24-motion.md).

The label is the good part. **All 57 distinct labels are a prefix of an
`objects.rlb` member, and every one of the 186 members they reach is an
`INTO` record** — an internal part. `i_pws_f` is a power supply, `i_rdr_b` a
radar, `i_eng_l` an engine, `i_fsh_f` a fight shield. So section 4 is the
controller's **parts list**, and a component's label says which family of
internal part it stands for. 395 of the 1066 records carry one. On a chassis
the six `i_*_<size>` labels are its **slots**: all 1,889 internal parts fitted
to shipped robots start with one of their chassis's labels
([28-chassis.md](28-chassis.md#a-chassis-declares-its-slots--measured-and-read)).

### The 64 bytes are the component's values

**Sixteen floats**, and all 17056 of them across the 1066 records are finite.
Every component class answers a *value id* through its vtable slot 4,
`Control.dll:0x10021d00`, which takes the id's low byte as an index into these
sixteen, multiplies by the owner's per-component figure when bit `0x100` is set,
and by a run-time level at `+0x4c` when bit `0x200` is. So id `0x300` is value 0
with both factors applied, and `0x300`–`0x305` are the first six. Which value
means what depends on the class. Class **26**, a building's efficiency, is
read end to end in [23-economy.md](23-economy.md); class **8**, the radar
(values 0–2 sensitivities to a target's mass, electronics and drive, value 3
its range, value 4 its rescan period), and class **10**, the detection shield
and its camouflage, in [25-sensors.md](25-sensors.md).

## The section-5 record

Nine `int32`, **then** the name pair:

```
0x00  int32[9]   unresolved
0x24  char[32]   archive
0x44  char[32]   member
```

This reader had that the other way round until the sections were walked:
anchoring on the names put the ints where the names are, which is also where
the old note about "three of the nine reading as ASCII on 41 records" came
from. There are **2925** such records and **1432** of them are named.

Across both places a controller names **1769** resources and **every one
resolves**. The 219 on components are what the part emits — a gun's projectile
is named there, which is why all 158 `objects.rlb` references are `BULL`
records, carried only by the four archives that hold things which shoot.
### What is still not read

The meaning of the fields rather than their extent: what most classes'
sixteen values mean, a component's 4-byte entries, section 1's conditions and
transition table, section 2's record contents, the 84-byte block's contents,
and the nine ints of a section-5 record. What section 1's states, the motion
triples and a component's mass (`+0x1c`) do is in [24-motion.md](24-motion.md).

### Where to look next

`LoadControlSystem` takes **three `(archive, member)` pairs** — six name
strings — copies them into a local block and hands it to a message dispatch.
Its only caller, at `AniMesh.dll:0x100032e7`, pushes them from three fields of
the agent at `+0x80`, `+0xc0` and `+0x100`, each an archive name followed by a
member name 0x20 later, plus a kind argument the callee compares against 9. The object is 0x668 or 0x670 bytes
with **six vtables** (`0x1003d298`, `0x1003d254`, `0x1003d1fc`, `0x1003d1b4`,
`0x1003d1a0`, `0x1003d198`) and the interface it returns is the last of them,
at `+0x14`. The message it sends is `0x80000020`, whose only handler is the function
at `0x10007830` (the branch at `0x10007890`). It calls **`0x10008b10`** — the
loader proper — which resolves a name through the resource manager at
`0x10042700`, then reads a
`count` dword and walks records at a **36-byte** stride (`add esi, 0x24` at
`0x10008be1`), which is the same 36 bytes the reference record ends with.

A sibling handler at `0x100314f0` — its `0x80000023` branch — shows the idiom
the sections are likely to follow: a **flags byte** whose bits gate optional
blocks, each block a run of `int32` terminated by `-1`. That is consistent with
the `-1` fills throughout these files, and it is the next thing to test.

**What the controller's messages are is now settled.** Its dispatch is a
16-way jump table on the message number, and that range is exactly `MCMD_` 1
to 16 — the movement commands the input tables send, whose numbers are
recovered in [14-controls.md](14-controls.md). Three commands the shipped
tables use fall outside it: `MCMD_WALK_F` (19), `MCMD_WALK_B` (20) and
`MCMD_LOCK` (21), so walking a machine is handled somewhere else entirely.
What remains open is narrower than it was: not *what the messages are*, but
which **field of the frame** feeds the channels those handlers reach —
`+0x5c8`, a 0xf4-byte object with 0x1c-stride channels defaulting to 0.5, and
`+0x5cc`, a 0x120-byte one with two arrays of six floats.

Two smaller unknowns sit beside the sections: the 84-byte block at +128 — set
on 525 members, and identical between `ctl_cam_fly` and `fr_b_plant` (a 1.0,
a 250.0, then two `±FLT_MAX` boxes) but different in the guns — and the nine
ints of the reference record.
