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
| 7 | 208 | 144 | a material | 1161 |
| 8 | 248 | 184 | a material | 237 |
| 9 | 208 | 136 | a material | 266 |
| 10 | 208 | 144 | a material | 160 |

That table walks **all 923 effects to the byte**, 4737 emitters in total. Type
6 does not occur, and neither does 0.

The table was not guessed. It was fitted: 338 of the records have a resource
pair in every block, which pins each type's length directly, and the rest were
settled by a search over the remaining lengths for the table that walks the
most records — 922 of 923 at the first pass, then all 923 once type 4 was
corrected from 200 to 204. The confirmation is independent of the fit: on
every record that walks, **every block ends where a `(archive, member)` pair
begins**, which a wrong stride would break immediately.

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

- **What distinguishes one emitter type from another.** Nine types with fixed
  lengths, all but one naming a material, and nothing yet says which is a
  sprite burst, which a trail, which a light.
- **The floats inside a block.** Each carries 30 to 60 of them — colours,
  lifetimes, velocities and spreads, by the look of the values — and none is
  identified.
- **The 60-byte effect header** and the `.exp`'s first float and flags word.
- **Bit 8 of the type word**, set on 1811 of the 4737 emitters.
- Snow and rain are **not** here. There is no FXID whose name mentions either,
  and `sky.wea`'s slots name the materials `SNOWFLAKE` and `RAIN_DROP`
  directly — see [10-sky.md](10-sky.md).
