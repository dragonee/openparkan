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

Every member begins with the same frame: a **128-byte parameter block**, then
an 84-byte block that can be entirely unset. Six members are nothing else —
exactly 212 bytes with `0xFF` from +128 to the end, the same "not set" fill
the `MAT0` loader uses. Five of those six carry no sections at all.

```
0x00  int32[5]   section counts
0x14  float[3]   +20   triple 1
0x20  float[3]   +32   triple 2
0x2c  float[3]   +44   triple 3
0x38  float[3]   +56   triple 4   two-pi on 364
0x44  float[3]   +68   triple 5   1.0 on 418
0x50  float[3]   +80   triple 6   two-pi on 422
0x5c  int32      +92   0 on 324, else 1000, 2000, 5000
0x60  float[2]   +96   zero on 512
0x68  int32      +104  0, 2 or 3
0x6c  float      +108  -1.0 on 465
0x70  float      +112  pi/2 on 509
0x74  int32      +116  0 on 342, else 3, 4, 16
0x78  float      +120  -1.0 on 433
0x7c  float      +124  FLT_MAX on 502
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

**Which triple is which is not established** and this document does not guess.
Three members side by side:

| | `ctl_cam_fly` | `o_c01_l_01` (a gun) | `fr_b_plant` (a factory) |
|---|---|---|---|
| +20 | 20, 20, 20 | 2.5, 2.5, 2.5 | 1, 1, 1 |
| +44 | 100, 100, 100 | 0, 0, 0 | 0, 0, 0 |
| +56 | 6.28, **0.6**, 6.28 | 6.28, 6.28, 6.28 | 0, 0, 0 |
| +80 | 6.28, 6.28, 6.28 | 6.28, 6.28, 6.28 | 6.28, 6.28, 6.28 |
| +92 | 1000 | 0 | 1000 |

The flying camera is the legible one: 100 on triple 3 against 20 on triple 1
is a speed against an acceleration, and the **0.6 in the middle component of
triple 4** — 34°, where the other two axes are free — is the one place in 531
files where a single axis is clamped on its own. That is what a pitch limit on
a camera looks like. It is a reading, not a fact, and nothing else in the data
tests it.

## The reference record

The sections after the frame carry **100-byte records**: a 32-byte archive
name, a 32-byte member name, then nine `int32`.

```
0x00  char[32]   archive
0x20  char[32]   member
0x40  int32[9]   unresolved
```

**All 1651 of them resolve** to a member that exists, and they point into
exactly three archives:

| Into | Records | What it is |
|---|---:|---|
| `effects.rlb` | 1435 | a visual effect |
| `objects.rlb` | 158 | **every one a `BULL` record** — a projectile |
| `weapon.rlb` | 58 | another weapon |

That settles what the record is for: **a controller names what the thing
emits.** Only `guns.rlb`, `turrets.rlb`, `animals.rlb` and `bases.rlb` carry
an `objects.rlb` reference — the four archives that hold things which shoot —
and a gun's single reference is the bullet it fires. A building's controller
names only effects: `fr_b_plant` has 54, most of them `f_signlight_g`.

The nine ints are small and unresolved. The first three are zero on 1373,
1537 and 1474 records, the fourth is 3, 4 or 5 on 1230, and two of the rest
count upwards across a run — 100, 101, 102 beside 12, 13, 14 — which reads as
an index rather than a parameter. `-1` turns up in every slot, the same
sentinel the frame uses, but no record is `-1` throughout.

One caveat on the tail: on **41** of the 1651 the third int reads as ASCII
rather than a number, so the nine ints are not the same nine fields on every
record. The two names are sound on all 1651 — they resolve — but a reader that
depends on the ints should check them.

## What is not read

**The sections themselves.** The five counts at +0..+16 say how many of each
of five kinds a member carries, and the frame accounts for the file exactly
when all five are zero. Beyond that the sections are variable-length and
nest, so their sizes are not a function of the counts:

- The three kinds that ever appear alone give strides of **160**, **36** and
  **180** — and a count of 1 on the second slot costs **zero** bytes
  (`s_tree_a_80.ctl` is 212 bytes with counts `(0, 1, 0, 0, 0)`), so that slot
  is a flag, not an array length.
- Solved exactly over all 531 members, no assignment of five fixed strides
  fits: the best exact solution fails on **520** of them.

So this reader finds the reference records **by their shape** rather than by
walking the sections, anchored on the set of archive names that actually
exist. Without that anchor a record whose member field holds an uninitialised
tail gets picked up four bytes late, splitting `objects.rlb` into `cts.rlb`;
with it, all 1651 land on a real member and 1144 of 1322 consecutive pairs sit
exactly 100 bytes apart.

### Where to look next

`LoadControlSystem` copies six `(archive, member)` name pairs into a local
block and hands it to a message dispatch. The object is 0x668 or 0x670 bytes
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
blocks, each block a run of
`int32` terminated by `-1`. That is consistent with the `-1` fills throughout
these files, and it is the next thing to test.

Two smaller unknowns sit beside the sections: the 84-byte block at +128 — set
on 525 members, and identical between `ctl_cam_fly` and `fr_b_plant` (a 1.0,
a 250.0, then two `±FLT_MAX` boxes) but different in the guns — and the nine
ints of the reference record.
