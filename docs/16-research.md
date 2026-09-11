# The research tree — `.trf`

`MISSIONS/SCRIPTS/` holds 29 `.trf` archives beside the behaviour scripts.
Every member of every one is named **`ResTree`**, and that is exactly what it
is: the technology tree the player researches through. `Behavior.dll` owns it —
not the scripts, which are [`ai.dll`'s](15-behaviour.md).

All 29 read: **368 items** each, and in the 26 that carry edges, 8009
prerequisites in total.

## Twelve streams, one table in columns

A `.trf` is an NRes archive of twelve members tagged `TRF0`–`TRFB`. Four of
them are exactly 368 records wide, which is what gives the item count away:

| stream | shape | what it holds |
|---|---|---|
| `TRF0` | 368 × 40 bytes | the item record |
| `TRF1` | 368 × 1 byte | a category |
| `TRF2` | 368 × int32 | how many prerequisites this item has |
| `TRF3` | flat int32 | the prerequisites, run together in item order |
| `TRF4` | 368 × int32 | how many items it unlocks |
| `TRF5` | flat int32 | the unlocks, run together in item order |
| `TRF6` | text | 395 part ids — `e_gun_bc_05` |
| `TRF7` | text | short codes — `L80mmRG` |
| `TRF8` | text | display names — `Large Rail Gun`; **exactly 368** |
| `TRF9` | text | descriptions, on 150 items only |
| `TRFA` | text | a stat template per item, for the UI panel |
| `TRFB` | 395 × int32 | one packed word per `TRF6` entry |

`TRF2`/`TRF3` and `TRF4`/`TRF5` are a counted list each: the count stream says
how many entries the flat stream gives that item, and `sum(TRF2)` equals the
length of `TRF3` in every archive — 332 in most, 168 in `42.trf`.

A `TRF0` record is:

```
float32 x4     two pairs; the second of each is the larger
int32          byte offset into TRF7 — this item's short code
int32          byte offset into TRF8 — its display name
int32          an id, which is not the item's own index
int32          byte offset into TRFA — its stat template
uint32         (class << 16) | counter
uint32         four packed bytes
```

The three offsets all land inside their streams on all 368 items in all 29
archives.

### The four floats, and whether one of them is research time

Nothing in the data labels them. `TRFA`'s templates carry units (`t`, `HP`,
`m`, `MWt`, `kmph`) but they describe the *component* — weight, damage, range —
and none of the 27 distinct stat rows is a time. So what follows is a reading
of the corpus, and the reader does not encode it.

The four are **two pairs**, and each pair is zero *together*: `(v0,v1)` is zero
on 277 of 368 items and `(v2,v3)` on 50. The pairs behave differently:

- **All 91 items with a non-zero `(v0,v1)` are required by something**, with no
  exceptions. That pair only ever appears on an item that is researched.
- On an upgrade ladder both pairs rise monotonically, and `(v0,v1)` sits just
  *above* `(v2,v3)`:

  | | v0, v1 | v2, v3 |
  |---|---|---|
  | `ARMOUR SA.Mk2` | 6, 9 | 5.5, 8 |
  | `ARMOUR SA.Mk4` | 18, 24 | 16.5, 21 |
  | `ARMOUR SA.Mk6` | 31, 35 | 29.5, 32.5 |

- For a **building** the second pair's second element explodes where the first
  pair's does not:

  | | v0, v1 | v2, v3 |
  |---|---|---|
  | `Large Factory FB-47L` | 20, 120 | 20, **1250** |
  | `Large Core mine MS-47L` | 20, 150 | 20, **1500** |
  | `Sml Research cntr RC-17` | 0, 0 | 8, **250** |

  A component's two pairs stay close; two engine grades have them exactly
  equal.

That is the shape of **one pair for researching a thing and one for building
it**, with the second element of each the duration: a factory is quick to
research and slow to build, a laser is much the same either way, and a starting
item costs nothing to research because it is not researched. Up the research
centres `v1` runs 0, 50, 60, 90.

So **yes, `v1` is the best candidate for how long a research takes** — and `v3`
for how long the thing then takes to build. What is *not* settled is which
member of a pair is the duration and which the cost: `v0` and `v2` are the
smaller and steadier of each pair, which is what a price would look like, but
nothing in the shipped data distinguishes the two. The reader exposes
`Item.values` as four unnamed floats for that reason.

## Which way the edges point

`TRF3` and `TRF5` are **the same graph written twice** — once as in-edges and
once as out-edges. Across all 26 archives that carry them they are **exact
transposes of each other**, and that is what fixes the direction rather than
any guess about the names. Reading them out:

```
Huge Cannon              requires  Large Cannon
Large Rail Gun           requires  Large Cannon, Enh Research cntr RC-67
Tiny Laser               requires  nothing
Lrg Research cntr RC-47  unlocks   33 items
```

## The spine

Two chains gate almost everything, and they interlock:

```
Sml Research cntr RC-17  →  Med Research cntr RC-30  →  Lrg Research cntr RC-47  →  Enh Research cntr RC-67
Small Factory FB-17S     →  Medium Factory FB-30M    →  Large Factory FB-47L
```

A factory needs the research centre of its own grade as well as the factory
below it, so the two ladders are climbed together. They are also the tree's
biggest gates by a distance:

| item | unlocks |
|---|---:|
| `Lrg Research cntr RC-47` | 33 |
| `Med Research cntr RC-30` | 28 |
| `Large Factory FB-47L` | 19 |
| `Medium Factory FB-30M` | 18 |
| `Sml Research cntr RC-17` | 12 |
| `Enh Research cntr RC-67` | 11 |
| `Small Factory FB-17S` | 10 |

Nothing else in the tree unlocks more than five.

Off the spine the tree is mostly short ladders of the same component in
rising grades — `SEng2 → SEng3 → SEng4` for engines, `ARM 2 → ARM 3 → ARM 4 →
ARM 5 → ARM 6` for armour, each grade also opening its Large, Medium and Tiny
variants — and weapon lines that widen as they go: a cannon leads to a
howitzer and a rail gun, each of those to its ammunition clips.

## It is a per-mission tree

The 29 archives are not 29 copies. The item table is identical in shape —
368 items everywhere — but **the wiring is per mission**: 11 distinct
prerequisite totals from 0 to 341, with the commonest shared by 13 archives.

- **3 archives carry no `TRF3`/`TRF5` at all**, so nothing has a prerequisite.
- In **20 of the 26** that do, the four research centres form the chain above.
- The other **6 rewire it**. In `42.trf` the large centre needs two chassis
  instead of the medium centre, and the medium centre needs nothing.

So a mission can hand the player a different technology ladder, and several do.

## What is not read here

- **Which member of each float pair is the duration** and which the cost. The
  pairing itself, and which pair is research and which is building, is argued
  above; the split inside a pair is not.
- **`TRF6` and `TRFB`** — 395 part ids against 368 items, with one packed word
  each. The counts differ, so the mapping is not one to one.
- **`TRF9`**, which carries a description for only 150 of the 368, and
  **`TRFA`**'s template syntax (`@G@Weight  @B,weight,G,t,5,1@`).
- The `TRF0` record's **id**, its `(class << 16) | counter` word and its four
  packed bytes.
