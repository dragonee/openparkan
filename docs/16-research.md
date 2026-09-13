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
| `TRF1` | 368 × 1 byte | the item's starting state, three bits |
| `TRF2` | 368 × int32 | how many prerequisites this item has |
| `TRF3` | flat int32 | the prerequisites, run together in item order |
| `TRF4` | 368 × int32 | how many items it unlocks |
| `TRF5` | flat int32 | the unlocks, run together in item order |
| `TRF6` | text | 395 part ids — `e_gun_bc_05` |
| `TRF7` | text | short codes — `L80mmRG` |
| `TRF8` | text | display names — `Large Rail Gun`; **exactly 368** |
| `TRF9` | text | descriptions; 150 items point at text, the rest at an empty string |
| `TRFA` | text | a stat template per item, for the UI panel |
| `TRFB` | 395 × 2 uint16 | a `TRF6` offset and the item that researches that part |

`TRF2`/`TRF3` and `TRF4`/`TRF5` are a counted list each: the count stream says
how many entries the flat stream gives that item, and `sum(TRF2)` equals the
length of `TRF3` in every archive — 332 in most, 168 in `42.trf`.

A `TRF0` record is:

```
float32 x4     the research energy and ore cost, then the build pair
int32          byte offset into TRF7 — this item's short code
int32          byte offset into TRF8 — its display name
int32          byte offset into TRF9 — its description
int32          byte offset into TRFA — its stat template
uint16         this item's own TRFB entry
byte x6        role, kind, sub-kind, branch, size, upgrade level
```

The four offsets all land inside their streams on all 368 items in all 29
archives. The third was read as "an id, not the item's own index" — 879
distinct values from 0 to 3896 — until the getter showed it added to `TRF9`'s
base (below). `TRF9` is 3,897 bytes in every archive, **all 10,672 offsets
land on a string start**, and 150 per archive land on text: the items that
have a description. The rest point at an empty string.

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

**Nothing in the tree gives a research a duration**, and none is needed: the
time budget is the research centre's own `FreeResearchTime`, the same for
every technology it researches — 2 seconds unless a mission grants otherwise
([23-economy.md](23-economy.md#research--read)). What sets a research's pace
is its ore and energy.

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
and it holds 3 on all 29. The same field over `TRF1` it keeps as a boolean at
`+0x7c`, which `IResearch` slot 29 hands out (`0x10003780`).

**That flag says the tree carries debugging information** (*read*). When
`iron3d.dll` gives a clan its tree (`0x1005fa50`) it asks the tree for the flag
(`0x1008ac40`) and, if it is set, shows *"Research tree contains debugging
information. Do not use in release mode"* (`0x100605d6`) — unless
`Iron_3D.ini`'s `[CS]` section sets `FULL_RESEARCH_TREE` to non-zero
(`0x1008ac50`; [22-settings.md](22-settings.md)). It is 0 on all 29. What
else the flag changes is not found: the warning is its only reader.

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
category byte is where an item *starts* rather than what it permanently is.

**It is three bits** (*read*):

| mask | | |
|---:|---|---|
| 4 | in the tree | the item belongs to this mission's tree |
| 2 | researched | `IResearch` slot 3 reports it as `b_res` |
| 1 | available | every prerequisite is researched; slot 3's `b_avail` |

Completing a research (`IResearch` slot 7, `0x10002c10`) needs mask 4, sets
1 and 2, then walks the tree and gives 1 to every item that is in it, not yet
available, and whose prerequisites all carry 4 and 2. Slot 3
(`0x10002aa0`) hands the three bits out with the four costs and the part name;
slots 25 and 26 export the researched items as a list of indices, with every
item's progress, and take them back (`0x100034b0`, `0x100035f0`).

*Measured*, all 10,672 records: the archives use five values — 0 out of the
tree (4581), 2 researched but out of it (70: the wildlife and the hero), 4
waiting (2044), 5 open to research (177) and 7 granted (3800). **Every 5 has
all its prerequisites researched (177 of 177) and every 4 lacks one (2044 of
2044)**, which is the rule the engine applies. A 7 is granted whatever its
prerequisites say.

Two more per-item arrays are allocated and never read from the file. The
`float32` per item at `+0x38` is research progress: `IResearch` slots 4 and 5
get and set it, and `M_Task_Research::OnBehaviourTakt` adds each tick's share
to it, caps it at 1 and completes the research there (`Behavior.dll:0x1002fdde`,
`0x1002fe0f`). The other is runtime bookkeeping with nothing in the archive to
match.

### The interface over the record

An earlier draft of this section said nothing in `MisLoad.dll` indexes a
40-byte record. That was wrong, and wrong for a bad reason: the search covered
`imul` and `lea`+`shl` and missed the form the compiler actually used —
`lea eax, [eax + eax*4]` and then an `*8` in the addressing mode, ×5 then ×8
with no multiply instruction to find.

**The getters sit on `IResearch`**, which is not the tree object's own table.
`LoadResearch` returns a 0x138-byte game object whose vtable (`0x1000e130`)
has 23 slots; asking it for interface `0x502` (`0x100027a0`) returns the
0x80-byte reader at its `+0x130`, whose table (`0x1000e18c`) follows
immediately and has **32 slots**. An earlier draft read the two as one 55-slot
table, so its slot numbers were 23 too high. By `IResearch` slot, ten are
per-field getters over the record; each bounds-checks the index against the
item count and returns −1 (a text getter, 0) outside it:

| slot | reads | |
|---:|---|---|
| 13, 15, 19, 16 | `int32` at `+0x10`, `+0x14`, `+0x18`, `+0x1c`, added to the stream's base | the short code, the name, the description, the stat template |
| 14, 12, 11, 18, 17, 30 | `byte` at `+0x22`…`+0x27` | six separate fields |

**That the last six bytes get a getter each is what settles their shape.**
They were read here as two packed words; they are six fields, and the data
says the same — each holds between 4 and 33 distinct values across all 29
archives, which the bytes of one packed number would not.

More slots matter: 9 and 10 return the prerequisite and unlock lists as
`(pointer, count)` over the pairs the loader built, 21 is the mapping below,
and 2 finds an item by part id. `Behavior.dll`'s research centre calls 3 and 9
(`MResearchCenter::CalcSummCost`, `::CreateParentTechArray`), and
`iron3d.dll` 3, 11–15, 17 and 18 through the tree it keeps at `0x1010c380`.

### The six bytes: a role, `objects.dlb`'s classification line, a size and a level — *read* and *measured*

`iron3d.dll:0x1008a500` collects five of them for a part — `+0x23`, `+0x24`,
`+0x25`, `+0x26` and `+0x22`, through slots 12, 11, 18, 17 and 14 — and
`0x1008a590` turns them into the object `Type` the part gives what it is built
into. Joined against [`objects.dlb`](19-descriptions.md), **the middle three
are that file's classification line as numbers**: over all 11,455 part
entries, every value stands for exactly one token.

| byte | | values |
|---|---|---|
| `+0x22` | role a unit part gives a unit | 1 a bunker or tower turret, 2–5 a battle, transport, builder or HQ turret, 6 the hero, 7 an animal, 255 anything else |
| `+0x23` | kind | 8 `BLD`, 9 `SHS` (and the creatures' `ANM`), 10 `AMM`, 11 `DVC`, 12 `WPN` |
| `+0x24` | sub-kind | 16 `HNG`, 17 `BUN`, 18 `INT`, 19 `MIN`, 20 `PLT`, 21 `STR`, 22 `TEL`, 23 `TOW`, 24 `TMP`, 25 `BRD`, 26 `GEN`, 28 `RUN`, 29 `TWL`, 30 `TWH`; 32 `SHS`, 33 `TUR`, 34 `TAR`; 49 `GUN`, 50 `FLM`, 51 `MIS`, 52 `ROC`, 53 `LAS`, 55 `DVC`, 56 `TAS`; 64 `DEF`, 65 `RDR`, 66 `REP`, 67 `FSH`, 68 `ARM`, 69 `BRN`, 70 `DSH`, 71 `ENG`, 72 `BAT` |
| `+0x25` | a building part's second sub-kind | 80 `TUR`, 81 `BLD`, 82 `DEF`, 83 `RDR`, 84 `UPG`; 255 on everything else |
| `+0x26` | size | 0 tiny, 1 small, 2 medium, 3 large, 4 `H`, `A` or `N`, 5 `E` |
| `+0x27` | upgrade level | `objects.dlb`'s `UpgradeLevel` ([30-turrets.md](30-turrets.md)) |

A weapon and its ammunition share a sub-kind: a clip is filed under the gun it
feeds. The sub-kinds fall in blocks of 16 — buildings from 16, chassis from
32, armament from 48, devices from 64 — with 27, 31, 35–48, 54 and 57–63
unused.

**What the engine makes of them** (*read*, `0x1008a590`): a building part
(kind 8) gives the building `Type` of its sub-kind — hangar `0x80000040`,
institute `…400`, mine `…04`, plant `…10`, storage `…08`, main teleport
`…200`, bridge `0x80001000`, generator `…02`, the two towers `0x80100000` and
`0x80200000` — and a bunker (17) the small, medium or large bunker `Type` by
its size; teleports (22), tower parts (23) and ruins (28) give 0. A turret
(kind 9, sub-kind 33) gives a unit `Type` by its role: transport
`0x1004000`, builder `0x1010000`, HQ `0x1020000`, hero `0x1002000`, and a
battle robot `0x1008000` for anything else. *Measured*: **164 of 167** placed
buildings carry the `Type` their root part derives; the other three are ruins,
placed as `0x80002000` where the derivation gives 0. `0x1008a690` maps the
same five bytes onto a small number 0–7 whose use is not traced.

### `TRFB` is the part-to-item mapping

Slot 21 bounds its argument against **395**, reads a `uint16` from `TRFB` as a
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

- ~~**Whether a research has a duration at all**~~ — **closed**: it has one,
  and it is not in the tree. The research centre's `FreeResearchTime` is the
  time budget of every research it runs
  ([23-economy.md](23-economy.md#research--read)).
- ~~`TRF6` and `TRFB`~~ — **closed**, above: `TRFB` maps each of the 395 parts
  onto the item that researches it, and 27 items take two parts each.
- ~~**`TRF9`**~~ — **closed**: the record's `+0x18` is an offset into it, and
  the 218 items without a description point at an empty string.
  **`TRFA`**'s template syntax is [read](19-descriptions.md).
- ~~The `TRF0` record's **id** at `+0x18`~~ — **closed**: not an id, the
  `TRF9` offset.
- ~~**What three of the six bytes at `+0x22`..`+0x27` mean**~~ — **closed**:
  `+0x23`..`+0x25` are `objects.dlb`'s kind, sub-kind and a building's second
  sub-kind, and `iron3d.dll` derives an object `Type` from them.
- ~~**What `TRF1`'s directory flag switches**~~ — **narrowed**: it marks a tree
  as carrying debugging information, and its one reader found is the warning
  `FULL_RESEARCH_TREE` silences. No shipped archive sets it.
- What `iron3d.dll:0x1008a690` does with the small number it derives from a
  part's bytes, and whether anything but the part lists reads a part's
  derived `Type`.
