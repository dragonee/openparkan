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

### The four floats are two resources, charged twice — *settled*

An earlier draft of this section argued from the corpus that the four were
two pairs, one for researching an item and one for building it, with the
second member of each a **duration**, and offered `v1` as the research time.
**The pairing was right and the duration was wrong.** The game names these
fields itself, in [`objects.dlb`](19-descriptions.md), and none of them is a
time:

```
#7ResearchEnergyCost=18
#8ResearchOreCost=50
#9BuildEnergyCost=18
#ABuildOreCost=50
```

Joined on the short code, **304 of 329 items match all four exactly**; the
remainder are short codes the library reuses across entries, so the join
picks the wrong one rather than the values disagreeing. So a `TRF0` record's
floats are, in order:

| | |
|---|---|
| `values[0]` | `ResearchEnergyCost` |
| `values[1]` | `ResearchOreCost` |
| `values[2]` | `BuildEnergyCost` |
| `values[3]` | `BuildOreCost` |

Two resources — **energy and ore** — each charged once to research a thing
and once to build it. `Item.research_cost` and `Item.build_cost` return them.

That explains every shape the corpus showed. A pair reads zero exactly when
that half is free, which is why 277 items cost nothing to research: they are
what you start with. A factory's build costs tower over its research costs
(20, 120 against 20, 1250) because a building is expensive to put up and
cheap to think of. And the pairs track each other up an upgrade ladder
because a better part costs more of both.

**Nothing in the shipped data gives a research a duration.** If the game has
one it is computed rather than tabulated, and `varset.var`'s
`dTechnologyFactor` ([15-behaviour.md](15-behaviour.md)) is where to look
first.

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

- **Whether a research has a duration at all.** The four floats are costs and
  no shipped file tabulates a time, so if there is one it is computed.
- **`TRF6` and `TRFB`** — 395 part ids against 368 items, with one packed word
  each. The counts differ, so the mapping is not one to one.
- **`TRF9`**, which carries a description for only 150 of the 368, and
  **`TRFA`**'s template syntax (`@G@Weight  @B,weight,G,t,5,1@`).
- The `TRF0` record's **id**, its `(class << 16) | counter` word and its four
  packed bytes.
