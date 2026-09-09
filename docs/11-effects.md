# Effects: `effects.rlb` and the `.exp` explosions

Two small formats that together say what happens when something is destroyed.
They are the animated half of the renderer, and nothing in a static scene
draws them — what they give you is the whole graph, from a mesh node's damage
record through to the sprites and sounds it sets off.

Everything below is re-derived by `uv run openparkan verify`.

## `.exp` — what an explosion sets off

144 members across five archives — 119 in `weapon.rlb`, then `animals.rlb`
(12), `system.rlb` (8), `static.rlb` (4) and `turrets.rlb` (1). Each is:

```
int32    count
float32  a           0 to 500; unresolved
float32  magnitude   2 on every _l, 3 on every _m, 4 on every _b
float32  1.0
float32  1.0
int32    flags       0 on 81 records, 7 on 63
count x  char[32] archive, char[32] member    the FXID to play
```

`24 + count * 64` fits all 144 exactly, and **212 of the 213 names they carry
are real `FXID` members**. The one that is not, `exp_t_sn_mis`, sits beside
`exp_t_st_mis` and `exp_t_sw_mis` in the same archive and reads as a typo.

The **magnitude** is the field that gives the format away. It tracks the size
suffix in the record's own name across every family: `explode_frt_l` 2,
`explode_frt_m` 3, `explode_frt_b` 4, and the same for `explode_rbr_*`.

A record is a fixed **792-byte buffer written without being cleared**, so
everything past the last name pair is whatever an earlier edit left there.
`weapon.rlb/bb_b_02.exp` declares three effects and carries four names, the
fourth a duplicate of the first. Reading past the count gets you stale data
that no longer resolves — which is how the count was confirmed.

## `FXID` — one effect

923 members of `effects.rlb`, and every one is a **60-byte header followed by
that many typed emitter blocks**. A block opens with a `uint32` whose low byte
is its type; the type fixes the block's length and where inside it the
`(archive, member)` pair sits.

| type | length | resource at | names | blocks |
|---:|---:|---:|---|---:|
| 1 | 224 | — | nothing; parameters only | 618 |
| 2 | 148 | 84 | a sound in `sounds.lib` | 517 |
| 3 | 200 | 136 | a material | 1545 |
| 4 | 204 | 136 | a material | 202 |
| 5 | 112 | 48 | a material | 31 |
| 6 | 4 | — | — | 0 |
| 7 | 208 | 144 | a material | 1161 |
| 8 | 248 | 184 | a material | 237 |
| 9 | 208 | 136 | a material | 266 |
| 10 | 208 | 144 | a material | 160 |

That table walks **all 923 effects to the byte**, 4737 emitters in total.

It was arrived at twice. First it was fitted from the data: 338 records have a
resource pair in every block, which pins those lengths directly, and the rest
fell to a search for the table that walks the most records — 922 of 923 at the
first pass, then all 923 once type 4 was corrected from 200 to 204. The
independent confirmation was that on every record that walks, **every block
ends where a `(archive, member)` pair begins**, which a wrong stride breaks
immediately.

Then it was read off the engine, which agrees exactly. `Effect.dll`'s emitter
factory does:

```
mov edx, [edi]      ; the block's first dword
and edx, 0xff       ; the low byte is the type
lea eax, [edx - 1]  ; types are 1-based
cmp eax, 9
ja  <skip>
jmp [eax*4 + <table>]
```

and each of the ten branches allocates its class, stores the block pointer,
and advances `edi` by exactly the stride above — `add edi, 0xe0` for type 1,
`0x94` for type 2, and so on. Two things follow that the data could not give:
**type 6 is a real type with a 4-byte block**, which no shipped effect uses,
and **bit 8 is a flag rather than part of the type** — right after the switch
the factory computes `(word >> 8) & 1` and stores it as a byte on the emitter.
It is set on 1811 of the 4737 emitters and what it controls is not known.

Types 7 and 10 allocate the same 0x48-byte object and install different
vtables, so they are sibling classes rather than unrelated ones.

## Inside a block

Only the sound emitter is read. Type 2 keeps a **near and far audible
distance** as two `float32` at +64 and +68: the first is never greater than
the second on any of the 517 blocks, and the pairs are exactly what a 3D sound
wants — (3, 40) on 160 of them, (10, 100) on 120, then (3, 50), (10, 80),
(15, 300). `aim_exp_L`'s explosion is audible from 20 to 200 units. Two more
`float32` at +8 and +12 sit in 0..1, most often (0.0, 0.1), and +72 is 1.0 on
516 of the 517.

## Which floats are live

The other nine types keep only a pointer to their block — at `this+0x18` for
most of them, `+0x1c` for type 1, `+0x24` for the sound — so the fields that
matter are whatever each class's own virtual methods load through that
pointer. That set can be recovered: walk the class's vtable, taint the
register the block pointer lands in, and record every `fld` off it.

**176 of the 441 four-byte slots across the ten block types are read as a
float.** Everything else the editor wrote and the engine never looks at:

| type | block | offsets it loads |
|---:|---:|---|
| 1 | 224 | 28, 32, 36 · 52, 56, 60 · 80, 84, 88, 92 · 112, 116 |
| 2 | 148 | 8, 12 · 28, 32, 36 · 52, 56, 60 · **64, 68** · 72, 76 |
| 3 | 200 | 8, 12 · 24, 28, 32, 36 · 40–60 · 100–120 |
| 4 | 204 | 8, 12 · 24, 28 · 40–60 · 100–120 |
| 5 | 112 | 4, 8, 12, 16 · 24–44 |
| 6 | 4 | — |
| 7 | 208 | 12–32 · 44–140 |
| 8 | 248 | 8–32 · 52, 56, 60 · 88–120 · 136–168 |
| 9 | 208 | the same eighteen as 3 |
| 10 | 208 | the same thirty-one as 7 |

Two things check the map. **The sound emitter is the control**: its `+64` and
`+68` were read off the data long before this existed, and the map contains
them — along with the `+8`, `+12` and `+72` the data had also flagged. And
**not one of the 176 offsets lands inside a block's `(archive, member)`
pair**, though the read map comes from the code and `RESOURCE_AT` came from
the data; an error on either side would collide somewhere.

The families fall out of it. **Types 3 and 9 read the same eighteen
offsets** — type 9's constructor installs type 3's vtable and then overrides
it, so it inherits the layout. **Types 7 and 10 read the same thirty-one**,
which is the sibling relationship the factory already suggested. Type 4 reads
a strict subset of type 3's, short only `+32` and `+36`.

One field is identified by its shape rather than by the code: **types 1 and 2
keep a unit vector at `+52`** — unit length on 597 of 618 and 517 of 517
blocks, and exactly `(1, 0, 0)` on 542 and 517 of them. A direction with a +X
default. The drawing types keep something else there: on type 8 not one block
of 237 is unit length.

Type 3's `+40..+48` and `+52..+60` are a component-wise (low, high) pair of
`float32[3]` on 1427 of 1545 blocks, which is the shape a particle spread
takes — and the engine reads exactly those six, which is the data and the code
agreeing without either being derived from the other. Types 7 and 10 read a
`float32[3]` at `+56`, `+80`, `+104` and `+128`, a regular 24-byte stride, in
their own methods rather than through a helper.

**None of that names a field.** It says which of the 30 to 60 floats in a
block are live, which classes share a layout, and which two hold a direction.
What they mean is still open.

One negative result worth keeping: **an explosion's size is not in its
effect.** `exp_frt_l`, `exp_frt_m` and `exp_frt_b` share their emitter blocks
byte for byte; the 2, 3 and 4 that separate them are the magnitude in their
`.exp`, which scales the whole thing at run time.

**Every one of the 3577 material references resolves** through `Material.lib`,
and 516 of the 517 sounds are in `sounds.lib` — the exception is
`fortif_door_move.wav`, which was never shipped.

`aim_exp_L`, the large explosion an animal makes, is a good example of the
shape:

```
 0  type  7 flagged  material.lib/fire_smoke
 1  type  7 flagged  material.lib/fire_add
 2  type  4          material.lib/spittle_g_add
 3  type  4          material.lib/glow_eng
 4  type  7          material.lib/smoke_g_add
 5  type  2          sounds.lib/Exp_anl_06.wav
 6  type  1          -
```

Five sprite layers, a sound, and one parameter block.

## The chain, end to end

A mesh node carries a durability and an explosion in its
[`.ndp`](07-objects.md); the explosion names effects; an effect names
materials; a material names a texture. All of it holds:

**2189 of the 2203** `.ndp` explosion references walk the whole way to an
effect whose every material resolves. The 14 that do not are the references
to `exp_t_sn_mis` and its neighbours.

## Not resolved

- **What distinguishes one emitter type from another.** Ten types with fixed
  lengths, all but two naming a material, and nothing yet says which is a
  sprite burst, which a trail, which a light. The read map above narrows it —
  the families are visible, and type 1 and the sound share a direction the
  drawing types do not — but no type is named.
- **What the live floats mean**, except the sound emitter's distances and the
  direction at `+52`. 176 of them are read; the values look like colours,
  lifetimes, velocities and spreads, but nothing pins one down. The next
  handle is a class whose method does something recognisable with a value —
  feeds it to a matrix, compares it against a clock — rather than just copying
  it into the particle it builds.
- **The 60-byte effect header** and the `.exp`'s first float and flags word.
- **What bit 8 controls.** That it is a flag is settled; what it switches is
  not.
- Snow and rain are **not** here. There is no FXID whose name mentions either,
  and `sky.wea`'s slots name the materials `SNOWFLAKE` and `RAIN_DROP`
  directly — see [10-sky.md](10-sky.md).
