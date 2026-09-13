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

## What the engine's own loader says

`Comp.ini` names the entry point — `CID_RESEARCH 7 misload.dll LoadResearch`
([22-settings.md](22-settings.md)) — so the tree has a front door, and going
through it settles some things the data could not.

`LoadResearch` builds a 0x138-byte object and hands the file to a 0x80-byte
reader whose loading method is at `MisLoad.dll:0x10002fe0`. That method takes
the streams in a **fixed order**, which is not the order the tags run in:

```
TRF0  TRF1  TRFB  TRF6  TRF7  TRF8  TRF9  TRFA  TRF2  TRF3  TRF4  TRF5
```

**Ten of the twelve are mandatory** — a missing tag returns false and the load
fails. Only `TRF3` and `TRF5` are skipped quietly, and *those two are the only
tags ever absent from a shipped archive*: three of the 29 lack them, and each
one lacks both. The code and the data draw the same line.

**The directory's second count is a version, and the loader checks it.**
Before reading a byte it asks for `TRF0`, fails unless that field holds **3**,
and it holds 3 on all 29. The same field over `TRF1` it keeps as a boolean —
0 on all 29, so nothing shipped turns that switch on and what it switches is
unknown.

### The count/pointer pairing, from the other side

For `TRF2` the loader allocates one `(count, pointer)` pair per item, fills
the counts from the stream, then walks `TRF3` handing each item with a
non-zero count the cursor and advancing it by `count * 4`. Then the same for
`TRF4` over `TRF5`.

That is exactly the structure this reader builds, and it was derived here from
the data before the loader was read — the second time in this project that the
binary has confirmed a layout rather than supplied it (the first was the
`.scr` node, [15-behaviour.md](15-behaviour.md)). The data closes the loop a
third way: `sum(TRF2) == TRF3`'s element count `==` its size in int32, on
**26 of 26** archives that carry it.

### `TRF1` is state, not a label

Every stream but one is used where it lies. `TRF1` is copied: the loader
allocates `element_count` bytes, zeroes them, and reads the stream into that
buffer. **A stream the engine takes a writable copy of is state**, so the
category byte is where an item *starts* rather than what it permanently is —
which is why 5 reads as *starting*, meaning already available.

Two more per-item arrays are allocated and never read from the file: 368
`int32` and 368 `int16`. Those are runtime, and there is nothing in the
archive to match them to.

### The interface over the record

An earlier draft of this section said nothing in `MisLoad.dll` indexes a
40-byte record. That was wrong, and wrong for a bad reason: the search covered
`imul` and `lea`+`shl` and missed the form the compiler actually used —
`lea eax, [eax + eax*4]` and then an `*8` in the addressing mode, ×5 then ×8
with no multiply instruction to find.

There are **55 vtable slots** over the loaded tree, and ten of them are
per-field getters over the record. Each bounds-checks the index against the
item count and returns −1 outside it:

| slot | reads | |
|---:|---|---|
| 36, 38, 42, 39 | `int32` at `+0x10`, `+0x14`, `+0x18`, `+0x1c` | the two text offsets, the id, the panel offset |
| 37, 35, 34, 41, 40, 53 | `byte` at `+0x22`…`+0x27` | six separate fields |

**That the last six bytes get a getter each is what settles their shape.**
They were read here as two packed words; they are six fields, and the data
says the same — each holds between 4 and 33 distinct values across all 29
archives, which the bytes of one packed number would not.

Three more slots matter: 32 and 33 return the prerequisite and unlock lists as
`(pointer, count)` over the pairs the loader built, and 44 is the mapping
below.

### `TRFB` is the part-to-item mapping

Slot 44 bounds its argument against **395**, reads a `uint16` from `TRFB` as a
byte offset into `TRF6`, copies the NUL-terminated part id out, and returns
the *second* `uint16` of the same entry. So a `TRFB` entry is two `uint16`:
**an offset into `TRF6`, and the index of the item that researches that part.**

The data confirms it four ways:

- all **11455** entries across the 29 archives land on a `TRF6` string start;
- all **11455** name an item in `0..367`;
- **11455 of 11455** land on the item whose `TRF8` display name is that part's
  own name in [`objects.dlb`](19-descriptions.md) — two files that share no
  bytes, agreeing on every entry;
- and the record's `uint16` at `+0x20` indexes the `TRFB` entry that names
  that same item, on **10672 of 10672** records, so the mapping is written
  both ways round.

Every one of the 368 items is named by at least one part, and none by more
than two. The 27 that take two are mounting pairs — `e_tur_bb_01` and
`e_tur_bt_01`, one turret researched once — which is exactly the 395 − 368
difference that made the counts look like a puzzle.

## What is not read here

- **Whether a research has a duration at all.** The four floats are costs and
  no shipped file tabulates a time, so if there is one it is computed.
- ~~`TRF6` and `TRFB`~~ — **closed**, above: `TRFB` maps each of the 395 parts
  onto the item that researches it, and 27 items take two parts each.
- **`TRF9`**, which carries a description for only 150 of the 368, and
  **`TRFA`**'s template syntax (`@G@Weight  @B,weight,G,t,5,1@`).
- The `TRF0` record's **id** at `+0x18` — 879 distinct values from 0 to 3896,
  equal to the item's own index on only 986 of 10672 records.
- **What three of the six bytes at `+0x22`..`+0x27` mean.** Their shape is
  settled and their ranges measured (1..7 with 255 for none, 8..12, 16..72,
  80..84 with 255 for none, 0..5, 0..3); each has its own getter. Three are
  now read (*measured*, all 10,672 records): `+0x22` is the role a unit part
  gives a unit — 1 a bunker or tower turret, 2–5 a battle, transport, builder
  or HQ turret, 6 the hero, 7 an animal, 255 anything else; `+0x26` is the
  size — 0 tiny, 1 small, 2 medium, 3 large, 4 for `H`, `A` and `N`, 5 for `E`;
  `+0x27` equals the part's UpgradeLevel ([30-turrets.md](30-turrets.md)).
  `+0x23`..`+0x25` are open.
- **What `TRF1`'s directory flag switches.** The loader keeps it; no shipped
  archive sets it.
