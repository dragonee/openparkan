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
0x20  float[3]   +32   triple 2   not read in Control.dll; zero on 518; y the AI's speed floor
0x2c  float[3]   +44   triple 3   top speed, m/s
0x38  float[3]   +56   triple 4   turn rate; two-pi on 364
0x44  float[3]   +68   triple 5   how fast the hull rights itself; 1.0 on 418
0x50  float[3]   +80   triple 6   the most the body leans, rad; two-pi on 422
0x5c  int32      +92   ms a dead object lasts; 0 on 324, else 1000, 2000, 5000
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

**Four of the triples are now named** ([24-motion.md](24-motion.md)):
`Control.dll:0x1000fca0` makes the live limits from triple 1 (+20, the
acceleration, doubled), triple 3 (+44, the top speed in m/s) and triple 4
(+56, the turn rate). Triple 6 bounds the body's lean
([below](#the-lean-and-triple-6--read-and-measured)). Triple 5 is multiplied
into the spin integrator (`0x10014b15`): the share of the hull's tilt righted
each step ([24-motion.md](24-motion.md#the-hull-leans-and-rights-itself--read-and-measured)).
**Triple 2 is never read in `Control.dll`** (*read*): no code there reaches +32..+40 in the authored
block (the `+0x46c` pointer and the body's `+0x1ac`), the live copy
(`+0x47c`..`+0x484`, the body's `+0x1b0`) or the property interface, while
the same scan finds triples 1, 3, 4, 5 and 6. 518 of the 531 files leave it
zero (*measured*). Outside it, `Behavior.dll:0x1003bed0` reads its forward
component as the AI walker's speed floor
([24-motion.md](24-motion.md#how-the-ai-asks-for-speed--read)). Three members side by side:

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

### The lean, and triple 6 — *read*, and *measured*

A machine **leans**: the motion body keeps a per-axis tilt at `+0xa0`
(controller `+0x254`) and turns it into its tilt quaternion `+0x90`
(`0x10015251`). A state's bytes **+0x08, +0x09, +0x0a** say what leans it about
x, y and z (`0x10014e8d`; table `0x1001538c`):

| selector | source, about or along x, y, z |
|---:|---|
| 1, 2, 3 | the turn made this step ÷ (dt × authored triple 4) |
| 4, 5, 6 | the velocity ÷ authored triple 3 |
| 7, 0 | nothing |
| 8, 9, 10 | the step's change of velocity (body `+0x170`, `0x10015906`) ÷ authored triple 1 |
| + `0x80` | negated |

The lean heads for *source × live triple 6* at *dt × live turn rate* and is
held within ±triple 6 (`0x100151ae`). So **triple 6 is the most the body leans
about each axis, in radians**, reached at full speed, turn or acceleration.
A state with no selector does not lean, which is why the default whole turn
does no harm.

*Measured*: 28 states in 16 controllers select a lean, and every axis they
lean carries an authored triple-6 limit (32 of 32); z, which none leans, keeps
6.28 on all 16.

- **The flyers and the animal `a_a_l2`** (9 controllers, `0x86, 0x03`): about
  x from the negated vertical speed, and about y — **a bank** — from the yaw
  turn, up to 0.76 rad.
- **The six- and four-wheelers** (`r_*_03`, `r_*_04`, `0x89, 0x83`): about x
  from the negated forward acceleration and about y from the negated yaw turn,
  0.15–0.25 rad and 0.02–0.1.
- **`r_t_02`** (`0x05, 0x84`): about x from the forward speed, about y from the
  negated sideways speed.

**Which way a positive lean tips the model** is now traced through the
quaternion: a positive angle about x tips the **nose down**, about y the **top
to the left**, about z the **nose to the left**. The turn triple goes to three
axis-angle quaternions (`0x100141c0`, q = (cos a/2, sin a/2 x axis) from
`ngiGetSinCos`), and `g_FastProc` slot `+0x3c` (`Ngi32.dll:0x10014450`) builds
from them not R but **S . R . S with S = diag(1, 1, -1)**: `M02`, `M12`, `M20`
and `M21` carry the sign a right-handed R does not. So the x and y rotations
run left-handed and the z one right-handed
([24-motion.md](24-motion.md#the-hull-leans-and-rights-itself--read-and-measured)).
That is what the righting needs: the settle angles are added to the spin, so
they can only stand the hull up if a positive pitch tips the nose down.

The only other reader of the lean is a generic device's input
([below](#the-entries-are-channels-and-a-devices-inputs--read-and-measured)),
which no shipped device selects; `0x1001789a` and `0x10017d9b` only serialise
it.

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
| +92 | read, 3 sites: the death delay, ms, added to the clock when the object dies ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)) |
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
| section 2 | `counts[2]` = C | C records of 36 bytes: **channels** — the node at +0, frames, initial value, control points at +16 and +20, rate, span, flags ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)) |
| section 4 | `counts[3]` = D | D component records, each **type-dispatched** |
| the block | — | a fixed **84 bytes**, copied into the object: 21 section-5 group indices ([24-motion.md](24-motion.md#the-eleven-surface-groups-switch-the-dust--measured)) |
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
0x08  uint32     flags, per class: a gun's 0x2000000 fires every barrel at once;
                 a turret's 0x4000000 is its ground mounting, 0x8000000 an HQ
0x18  int32      the initial state (CIS_ switch values); -1 keeps the class's default
0x20  float      power: a consumer's draw a second, a battery's output
0x24  float[2]   a generic device's two input weights
0x2c  float[16]  the component's values, copied whole into the object
0x6c  char[32]   archive        what this part emits
0x8c  char[32]   member
0xac  int32      N -- how many 4-byte entries follow
0xb0  int32[N]   the entries: the section-2 channels it drives
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

**Where each class's values are read** (*measured*: which of the sixteen are
ever non-zero):

| class | values set | read in |
|---|---|---|
| 2 gun, 30 builder's beam | 0–3 | [29-weapons.md](29-weapons.md) |
| 4 camera | 0–5, the same six on all 61 | 0–2 in [30-turrets.md](30-turrets.md); 3–5 not read |
| 5 engine | 0 | [24-motion.md](24-motion.md) |
| 8 radar, 10 detect shield | 0–4 | [25-sensors.md](25-sensors.md) |
| 9 fight shield, 15 repair, 21 deflector, 27 armour | 0–2, 0–1, 0–5, 0–2 | [26-damage.md](26-damage.md) |
| 17 seeker | 0–2 | 0–1 in [29-weapons.md](29-weapons.md); 2 reaches `IDeviceManager` id 12 |
| 19 battery, 26 efficiency | 0 | [23-economy.md](23-economy.md) |
| 3 generic device | 0 (0.5 on 8 flyer records) | not read; its update (`0x10020900`) never takes a value ([28-chassis.md](28-chassis.md#what-a-devices-value-turns--read-and-measured)) |
| 24 the hero's arms | 1 and 4 | not read |
| 1, 12, 13, 25, 29 | none | — |

So the only values not yet read are class 3's value 0, the camera's 3–5 and
the arms' 1 and 4.

### The entries are channels, and a device's inputs — *read*, and *measured*

**A component's entries are channel indices.** The parser adds the part's
first section-2 channel to each and keeps it as a 20-byte channel link in the
component's list at `+0x1c` (`Control.dll:0x10021de7`): a gun's barrels, a
turret's yaw and pitch, a door's leaves, a wheel. *Measured*: all **885**
entries, on 474 components, index a channel of their own controller, and 861
of the 991 channels are driven by some component. No frame field feeds a
channel: a channel moves at its own rate toward what its component asks.

**The generic device takes its rate from the motion.** The factory builds
classes 3, 6, 7, 11–14, 16, 18, 20, 22–25, 28 and 29 as one device
(`0x1002d6ec`, `0x10020800`). Its update adds two inputs, selected by the flags
word's bytes 1 and 2 and weighted by the floats at `+0x24` and `+0x28`
(`0x10020985`); a non-zero byte 0 selects an input that sets the progress
outright. A selector *s* (`0x10020d90`, `0x10020ea0`):

| *s* | input |
|---:|---|
| 2, 3, 4 | spin about x, y, z |
| 5, 6, 7 | the same, negated |
| 8, 9, 10 | the lean ÷ authored triple 6 |
| 11, 12, 13 | the velocity ÷ authored triple 3 |
| 14 | the speed ÷ the authored top speed, as lengths |
| other | the default: 1 as the first input, 0 as the second, the progress unchanged as byte 0 |

*Measured*, over the 276 generic devices: selectors 12 (39), 4 (20), 7 (14)
and 1 (24), nothing else. **The running gear of the wheeled and tracked
chassis** (`r_*_03`, `r_*_04`) is 28 class-3 records whose rate is forward
speed ÷ top speed plus or minus half the yaw rate — (12, 7) on the records
whose flags' top byte is 1 and (12, 4) on those with 2, weights 1.0 and 0.5 on
all 28 — so the two sides turn at different speeds in a turn. Eleven records
on `r_b_02`, `r_l_02`, `r_l_05` and `r_m_02` set their progress from
selector 12, forward speed, and six on `r_l_03` and `r_m_03` from 4, the yaw
rate. This is the frame reaching a channel: through triple 3, never directly.

## The section-5 record — *read*, and *measured*

Nine `int32`, **then** the name pair:

```
0x00  int32      flags: 0x40000000 / 0x20000000 test the conditions,
                 0x80000000 opens a run, 0x10000000 closes it (below)
0x04  int32      the condition mask: one bit per condition byte
0x08  int32      the inversion: the masked bytes that count when clear
0x0c  int32      the action, 0..27 (Control.dll:0x10002800, table 0x10003590)
0x10  int32[4]   v4..v7, the action's arguments
0x20  int32      not read; 1.0 as a float on two records
0x24  char[32]   archive
0x44  char[32]   member
```

A group is a list of actions run in order: when a state is entered (its `+0x90`),
when a contact lands (its `+8`, [below](#section-1s-conditions-are-contacts--read-and-measured)),
from the 84-byte block (entry 0 at load, 2 to 4 on a round's hit, edge and range
end, 6 and 7 at critical damage, 9 on control message 12, 10 to 20 by the
ground's surface), by a component (a gun's `+0xc`; a generic device's `+0x10`
and `+0x14` as it starts and stops working, `0x10020951` and `0x10020931`), or
by a building's construction codes.
The actions (*read*; counts *measured* across the 2925 records):

| action | records | does |
|---:|---:|---|
| 0 | 202 | **stop the body**: its command, velocity, spin and step velocity go to zero (`0x10014400`); first in every round group |
| 1, 2 | 80, 30 | property `0x200` / `0x201` on the object |
| 3 | 217 | **an effect** by name on **node** v4, id v7 (`0x10002972`): v4 is rebased by the part's first node through `AniMesh` slot 14, as action 14's is (`0x100029bb`) |
| 4 | 1203 | **an effect** by name on three control points v4..v6, at their centroid, id v7 (`0x10002a8d`) |
| 5 | 72 | **an effect** by name in the world at the sphere `+0x38` gives — a building's construction sphere — scaled by its radius, id v7 (`0x10002e0e`); `fortif.rlb` only |
| 7 | 0 | node v4 takes its whole life as damage (`0x10003087`) |
| 8 | 22 | delete effect v4 (`0x10002fd4`) |
| 10 | 462 | start effect v4 in time mode v5 |
| 11 | 52 | restart effect v4 in its own mode once it has finished |
| 12, 13 | 14, 0 | `IControl` slots 3 / 4 with (v4, v5, v6) × 0.001 — the missiles' (0, 1, 0) |
| 14 | 11 | effect v4 takes its time from control point v5 (`0x10003031`) |
| 15 | 75 | **remove the object** with no explosion (`0x10003341`) |
| 17 | 69 | **kill it**: invulnerability off, then `ILifeSystem` slot 7, so its node 0 explodes (`0x100033d0`) |
| 18, 19 | 42, 196 | switch effect v4 on / off |
| 20 | 60 | **place the building**: on an agent of kind 3, the object's parent's `ITerrain` (interface 5) slot 3, `CLandscape::PlaceBuilding` (`0x1000352e`, `Terrain.dll:0x1000df10`); `fortif.rlb` only, in every building's construction states |
| 21 | 60 | kill every unit in the construction sphere ([32-builder.md](32-builder.md)) |
| 27 | 58 | **explode node v4** with the named `.exp` (`0x100030ce`) |

Only actions 3, 4 and 5 name an effect in `effects.rlb`, and 27 an `.exp` in
`weapon.rlb`; no other record carries a name. Codes 6, 9, 16 and 22–26 fall to
the default and carry nothing. The effects themselves are in
[11-effects.md](11-effects.md#how-an-effect-runs--read).

This reader had that the other way round until the sections were walked:
anchoring on the names put the ints where the names are, which is also where
the old note about "three of the nine reading as ASCII on 41 records" came
from. There are **2925** such records and **1432** of them are named.

Across both places a controller names **1769** resources and **every one
resolves**. The 219 on components are what the part emits — a gun's projectile
is named there, which is why all 158 `objects.rlb` references are `BULL`
records, carried only by the four archives that hold things which shoot.

### A record's condition, and runs — *read*, and *measured*

A controller keeps **sixteen condition bytes** at `+0x378`, and a record's
ints 1 and 2 test them (`0x100022c0`):

| byte | set when |
|---:|---|
| 0–10 | the machine stands on ground surface id *i* — one of them, rewritten whenever the id changes (`0x10002790`, `0x1001ab21`) |
| 7 | then overwritten with the face's world flag `0x400`, **the liquid bed** (`0x1001ab2d`) |
| 14 | the machine is critically damaged ([below](#critical-damage-block-entries-6-and-7--read-and-measured)) |
| 15 | a copy of `+0x618`, taken before each group runs (`0x1000286d`); control message 7 with argument 2 sets it (`0x10007be9`); with it set, node damage passed with its third argument set is added to the node's `+0x10` and the life put back (`0x10010fe6`) |
| 11–13 | never |

Condition *i* **holds** when bit *i* of the mask is set and byte *i* is set —
or clear, where bit *i* of the inversion is set. A record flagged
**`0x40000000` runs when any masked condition holds**, one flagged
`0x20000000` when all of them do, and a record with neither always runs.

Records group into **runs** (`0x100028b2`). `0x80000000` opens one; the next
`0x10000000` record closes it, and runs on its own condition **or when no
record since the opening one has run** — an *else*. So a run is a switch.

*Measured*: 37 records test conditions, all with `0x40000000` and a single
mask bit, on bytes 0, 1, 2, 5, 7, 8, 9 and 10; every inversion lies inside its
mask. Eight runs open and eight close, none left open. The hero's footstep
groups are the clearest (`r_h_02`, one per foot):

| surface (tag) | step effect |
|---|---|
| 5 (`mt`) | `step_hm` |
| 1 (`st`) | `step_hs` |
| 2 (`gr`) | `step_hg` |
| 8 (`al`) | `step_ha` |
| 10 (`sh`; also a face whose owner has no materials) | `step_hf` |
| 9, and the else | `step_hg` |

— metal, stone and grass by the surface's own tag, 8 of the 12 effect letters
against 2 with the surfaces shifted by one. The critically damaged hero
starts `smoke_fr_01` only while byte 7 is clear: no smoke over a liquid bed
(*derived*).

### Section 1's conditions are contacts — *read*, and *measured*

A state's `counts[1]` 16-byte conditions are its **contacts** — a foot, a
wheel, a leg — and the same points in every state (961 of 961, *measured*).
The engine keeps one 0x5c-byte entry per contact at `+0xc4`
([24-motion.md](24-motion.md#finding-the-ground--read)):

```
+0   int32   a control point of the object's .cpt; the node is its third slot
+4   uint32  flags
+8   int32   the section-5 group run when the point lands, or -1
+12  int32   0 in every file; the load writes the planted gap here
```

| flag | meaning |
|---|---|
| `0x100` | the state applies only while this point's node is **intact** (`0x10001107`) |
| `0x200` | … only while it is **destroyed** — a node at 0 life, status `0x10` (`0x1001106c`, `0x1001ac39`) |
| `0x4` | a destroyed point falls back to the next control point whose node is intact (`0x1001ac75`) |
| `0x1` | the point's ground gap, point and normal count toward the body's (`0x1001b00a`) |
| `0x2` | the node the point **carries** is laid along the ground under it, through `IAnimation` slot 31 and node mask `0x10` (`0x1001affd`, `0x1001a3af`) — authored on the tracked chassis's twelve belts, and nothing else ([28-chassis.md](28-chassis.md#the-belt-lies-along-the-ground--read-and-measured)) |
| `0x1000` | set on the states whose last pose holds the point within 0.1 of its rest height (`0x1001a328`) |
| `0x10` | derive `0x1` from that too (`0x1001a314`); **no shipped contact carries it**, 0 of 2634 |
| `0x20` | derive `0x2` from the point's axis standing within 0.05 of upright in that same pose (`0x1001a331`). **2410 of the 2634 carry it** — every walking chassis's feet — and the pose places 2217 ([24-motion.md](24-motion.md#a-walkers-feet-lie-flat-where-the-animation-lays-them--read-and-measured)) |

**A contact that lands runs its group**: an intact point in a `0x1000` state
that was not planted in the one before runs `+8` (`0x1001b08f`). Those are the
**footsteps**: 22 groups in `bases.rlb` and `animals.rlb` are reached from
nowhere else.

**The state test, then, is about damage**, not about the ground. *Measured*:
2634 contacts on 961 states; the 1363 on controllers with a same-stem `.cpt`
all index it (`LeftFoot`, `foot_fl`, `leg_fl`, `weel_fr`, `Placement`); `+12`
is zero on all 2634. 102 states need a destroyed contact — 81 on `r_b_05`, 11
on `r_l_01`, 9 on `r_m_01`, 1 on `r_b_01` — and every contact on them sits on a
running-gear node, 103 left (`.ndp` `0x20`) and 103 right (`0x40`): **the
walkers limp** when a leg is shot off.

### A footstep, end to end — *read*, and *measured*

**Which states plant a foot** is worked out as the machine **takes** the state
(`0x10019df0`, from `0x10007b99` and `0x10031982`, each right after the state's
record is copied into the current-state slot `+0x100`), in the same pass that
measures the state's stride (`0x1001a2d5`–`0x1001a328`). For each contact the
pass poses the mesh at the state's last pose — pair B's last frame, all of the
weight on B — and finds the contact point, its height carried back up by the
root's own height at that pose. When that height lies within 0.1 (`0x1003c03c`)
of the height the live contact record holds (`+0x14`), the contact gains
`0x1000`. Flag `0x10` then sets or clears `0x1` to match, and `0x20` sets or
clears `0x2` by whether the point's axis stands within 0.05 of upright. The
answer depends on nothing but the state, so working it out once per state, as
the engine does, comes to the same thing.

**A foot lands** in the ground contact's pass over the contacts
(`0x1001b081`–`0x1001b0be`), with no ground distance in the test:

1. a contact whose node is gone (`+0x59` clear) is passed over;
2. if its live **planted** byte `+0x5a` is clear and the current state's contact
   carries `0x1000`, its group `+8` runs (`0x10002800`) and the byte is set;
3. if the byte is set and the state's contact lacks `0x1000`, the byte is
   cleared.

So a step sounds once each time the machine enters a state that ends on that
foot after one that did not.

**What the group plays.** The ground's surface id sets condition bytes 0–10 one
at a time, byte *i* to (id = *i*) (`0x10002790`), when the id changes; byte 7 is
then the liquid bed's flag instead
([24-motion.md](24-motion.md#finding-the-ground--read)). `r_h_02`'s two
groups are a single run each: action 10 starts effect 101, 201, 301, 401 or
501 (+ the foot, 1 or 2) in time mode 1 for bytes 5, 1, 2, 8 and 10, and 301
for byte 9 or when nothing else ran. The load group (block entry 0) created
them with action 3 on nodes 4 and 8, the feet `LeftFoot` and `RightFoot` sit
on: `step_hm`, `step_hs`, `step_hg`, `step_ha` and `step_hf`. So the step plays
**at the foot's node** (*measured*).

**What a step is** (*measured*, all nine `step_*` effects): header mode 0 and
0.5 s, and one one-shot sound whose trigger is 0.1 — `step_h?.wav` from 1 to
50 m (2 to 60 for metal), `step_r?.wav` from 10 to 80, `step_rh.wav` from 15 to
350 on the transformer. Started in mode 1, its time crosses 0.1 about 50 ms
after the foot lands, within the effect manager's 100 ms updates
([11-effects.md](11-effects.md#how-an-effect-runs--read)).

*Measured* across the install:

- all 183 action-3 records of controllers with a same-stem mesh name a node of
  it, while 18 of them lie past the same-stem `.cpt`;
- the contact groups hold only action 10 in mode 1 (800 contacts) or actions 10
  and 11 in modes 0 and 1 (368).

*Derived* on `r_h_02`, with the rest pose's height as the live record's (see
below): at a run, whose cycle takes 12 states, the left foot lands entering
states 90 and 78 and the right entering 78; at a walk, 24 states, the right foot
lands entering 9 and 12 and the left entering 18. Most feet clear 0.1 by a
wide margin at a run (0.13–0.55), but the left foot at state 78 sits 0.0991
from rest, and at a walk the lifts are 0.1003 to 0.1218. So how often the
walk sounds depends on float rounding, and on which height the live record
holds.

### A building's load group — *read*, and *measured*

**A building runs its load group like any controller.** The controller
loader runs block entry 0's group once, when the file loads
(`0x10009408`). It does not ask what the owner is. A building's agent loads
its record's `.ctl` like any agent's; `CBuilding` walks that same control
system's items for its doors and pods
([27-ownership.md](27-ownership.md#capture--read)). So the section-5 effects in
a `fortif.rlb` controller's load group are created as the building is placed.
They hang on the building's own control points and nodes, as the hero's
turret's hang on its (*derived*, [29-weapons.md](29-weapons.md)).

*Measured*: 28 of the 30 `fortif.rlb` controllers have a load group. Their
most common effects are:
- the screens `f_pict_13` (81), `f_pict_08` (50), `f_pict_11` (28) and `f_pict_10` (23);
- the small lights `f_smalllight_r` (65), `_y` (48) and `_g` (40), and `f_signlight_g` (30);
- the door sounds `door_open_01` and `door_close_01` (52 each);
- the construction sphere `b_sphere_*`;
- `f_recharge_r` over docks (15), and `smoke_fr_02` (6);
- the bridges' `f_brige_ray` (5) and `f_brige_light_b` (3).

**The Large Factory's** (`fr_b_plant.ctl`, 55 records):

| action | effect | on | ids |
|---|---|---|---|
| 4 | `f_signlight_g` ×4, `f_blinklight_r` ×2, `f_smalllight_y` ×4, `f_smalllight_r` ×6, `f_blinklight_g` | control points 99–113, 120, 121 (`Sign_Entrance*`, `Sign_Type1`, …) | 11–27 |
| 4 | `f_pict_08`, `_10`, `_11`, `_13` ×22 | points 0–21, the consoles | 1000–1021 |
| 4 | `f_recharge_r` ×2 | points 114–116 and 117–119 (`Rech_*`), the dock | 2000, 2001 |
| 4 | `smoke_fr_02` ×3 | points 122–130 (`Smoke*`), the three chimneys | 3300–3302 |
| 5 | `b_sphere_start`, `b_sphere_sign`, `b_sphere_main` | the construction sphere | 9001, 9002, 9100 |
| 3 | `door_open_01`, `door_close_01` on each door | nodes 3, 16, 14 | 8000–8005 |
| 10 | start 3000, 3001 in mode 2 | — | no record of the group makes either id |

What those effects are ([11-effects.md](11-effects.md#how-an-effect-runs--read), *measured*):
- **The lights** are mode 1 (1 s) or, for the blinkers, mode 2 (2 s) with
  ping-pong. They carry flags `0x400` and `0x800`: drawn only while their
  tested point is in view, and only by the draw pass that passes its argument.
- **The screens and the smoke** are mode 0, whose time stays 0 until
  something sets it, so the emitters whose windows hold 0 run.
- **The door sounds** are modes 16 and 17: the door node's animation value
  while it only rises, or only falls. So the open sound plays as the door
  opens and the close sound as it closes.
- **Every one is switched on** at the default preset: Lights, Shield or Smoke
  ([11-effects.md](11-effects.md#which-effects-run-the-settings-switch--read-and-measured)).

*Seen*: in *The Constructor*'s recording the factory's chimneys smoke from the
briefing on (12 s), and its consoles glow inside (100 s).

**Where an action-3 effect stands** (*read*, and *measured*). The instance is
made on the owner and an attach point of (object, node)
(`Control.dll:0x100029ee`; the node is v4 plus the part's first node, less one),
and each tick it takes the node's matrix, property 2, as its own frame
(`Effect.dll:0x1000625a`). Every emitter of the four door effects sits at
(0, 0, 0) in it, so the sound plays at the **node's authored origin**.

That origin is often nowhere near the geometry the node draws. *Measured* over
the 112 action-3 door effects in `fortif.rlb`, as the distance from the node's
origin to the centre of its level-0 bounding sphere:

| buildings | origin to sphere |
|---|---:|
| the mines, stores, towers, ruins, `fr_l_bunker`, `fr_l_gener` | 0.0 to 1.5 m |
| `fr_b_plant` entrance | 8.1 m |
| `fr_l_plant` entrance | 19.4 m |
| `fr_m_plant` entrance, the bunkers | 20.0 to 21.2 m |
| the institutes | 14.6 to 15.0 m |
| the three factories' side doors | 30.8 m |

68 of the 112 stand more than 10 m out. A door sound is audible from 5 m to
60 m ([11-effects.md](11-effects.md#how-a-sound-is-heard--read-and-measured)),
so at 30.8 m it plays at a sixth of its gain — all but inaudible standing in
the doorway, while the Small Generator's, whose origins are on its doors, plays
whole. **STAND-IN**: openparkan stands an action-3 effect at the centre of the
node's level-0 bounding sphere instead, which puts every one of the 112 on its
door. Where the node's matrix itself comes from is not traced.

**For an engine:** when a building is placed, run its controller's load group
as the hero's turret's is run. That means the effects of actions 3 and 4 on
the building's nodes and control points, each placed through the building's
placement and node pose, and actions 10 and 11 by id. Leave action 5 to
construction.

### Critical damage: block entries 6 and 7 — *read*, and *measured*

After a node takes damage the node update (`0x10012a40`) decides whether the
machine is **critical**:

- **with running gear** (`.ndp` flags `0x70`, *n* nodes): when their life
  fractions sum to *n* / 2 − 1 or less (`0x10012ad6`) — on a machine with one
  gear node a side, both gone (*derived*);
- **without**: below 20% of its full hit points, 30% for an agent of kind 10
  (`0x10012bda`, `0x1000fa85`).

Becoming critical runs **block entry 6** and sets condition byte 14
(`0x10012bfb`); from then on the machine **burns**, losing 10% of the life it
had at that moment times the update's dt (`0x10012c1c`). Recovering runs
**entry 7** and clears the byte (`0x10012b12`).

*Measured*: 32 controllers set entry 6, all set entry 7, and every entry-6
group switches effects on (action 18) that the entry-7 group switches off
(19): `smoke_fr_01` on 21, flames on the trees, `aim_fire_S` on four.
Entry 9 runs on control message 12 (`0x10007cab`); entries 1, 5 and 8 are
read by nothing, and no shipped controller sets 1, 5, 8 or 9.

### What the loader is handed

`LoadControlSystem` takes **three `(archive, member)` pairs** from the agent's
`+0x80`, `+0xc0` and `+0x100` (`AniMesh.dll:0x100032e7`), plus a kind. The
message it sends is `0x80000020`, handled at `0x10007830`, which calls the
loader `0x10008b10`. The pairs are **the `.ctl`**, **the `.ndp`**, read at 76
bytes a record into the node records (`0x10008c6b`), and **the `.cpt`**, read
at 36 bytes into the control-point table at `+0x54` (`0x10008be1`), whose
entries keep the point's node (`0x1000b210`). The 36-byte stride this page
once matched against the reference record is the control point's.

The control system with that dispatcher installs six vtables
(`0x1000728d`): `0x1003b5e0` at +0 (`IControl`), `0x1003b59c` at +4
(`ILifeSystem`), `0x1003b544` at +8 (interface `0x202`), `0x1003b4fc` at +0xc
(`IDeviceManager`, `0x204`), `0x1003b4e8` and `0x1003b4e0`. The six this page
listed before (`0x1003d198` at +0x14, `0x100314da`) are a second class's, and
its +0xc vtable `0x1003d1b4` has the same slot 4. **The 16-way jump table once
taken for the controller's `MCMD_` dispatch is that slot 4**, a getter by id —
see [14-controls.md](14-controls.md#the-join-with-the-controller--read). Input
reaches the controller through `IControl`'s setters and the component
interface, and walking through `World3D.dll`'s own table, so no frame field
is wired to a message.

### Not established

- ~~Triple 5 (+68): multiplied into the spin integrator (`0x10014b15`); what it
  stands for~~ — **read**: how fast the hull rights itself
  ([24-motion.md](24-motion.md#the-hull-leans-and-rights-itself--read-and-measured)).
- Class 3's value 0 (0.5 on eight records), the camera's values 3–5, the
  hero's arms' values 1 and 4.
- The section-5 record's int 8 (`+0x20`): not read by the interpreter, the only
  code that walks the records; 1.0 as a float on the two `eng_rb_0?_snd`
  records.
- ~~Which way a positive lean tips the model on screen.~~ — **read**: a
  positive pitch tips the nose down, a positive roll the top to the left and a
  positive yaw the nose to the left, since the rotation the turn triple builds
  is S . R . S with S = diag(1, 1, -1)
  ([above](#the-lean-and-triple-6--read-and-measured)).
- What control message 7's arguments 0, 1 and 2 stand for, and so what byte 15
  and `+0x618` mean; no shipped record tests byte 15. One more reader is now
  known: while `+0x618` is set the ground contact does not move the body at all
  (`0x1001b3f7` skips the lift), so whatever the message means, it freezes the
  machine where it stands
  ([24-motion.md](24-motion.md#holding-the-body-on-the-ground--read-and-measured)).
- `IDeviceManager` ids 5 and 6: what the gun's `+0x174` (the round's property
  `0x35`) is.
- The height a live contact record holds at `+0x14` when the loader compares
  each state's last pose with it: the record is filled from the point's
  position at load (`0x1001a017`, copied in by `0x1001b6a0`), and which pose the
  mesh holds then is not read. The rest pose is assumed above.
