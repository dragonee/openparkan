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
(`MResearchCenter::CalcSummCost`, `::CreateParentTechArray`), and 7 and 10 when
a research completes ([below](#completing--read)). `iron3d.dll` calls 3,
11–15, 17 and 18 through the tree it keeps at `0x1010c380`. That tree is the
player clan's: the level's setup installs it from the player's clan record
(`0x100a2610`, `0x1008a3e0`).

**Slot 3's record** (`MisLoad.dll:0x10002aa0`, *read*) is eight words: the
researched bit, the available bit and the in-tree bit, each as 0 or 1; the four
costs; and the part id.

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
`0x1002000`, builder `0x1004000`, HQ `0x1010000`, hero `0x1020000`, and a
battle robot `0x1008000` for anything else — `varset.var`'s `ROBOT_TRANSPORT`,
`ROBOT_BUILDER`, `ROBOT_HQ`, `ROBOT_HERO` and `ROBOT_BATTLEUNIT`. *Measured*: **164 of 167** placed
buildings carry the `Type` their root part derives; the other three are ruins,
placed as `0x80002000` where the derivation gives 0. Over the 395 parts of a
tree, **293 derive 0** and 102 name a building or a unit — the same split in
all 29 trees.

**Who reads the `Type`** (*read*, as an enumeration). `0x1008a590` has
**six call sites in three functions, all of them the warbot designer's**, and
each pair is the same idiom: derive from the design's base part (`+0x39c`), and
where that part's kind is 9 derive again from the design's second part
(`+0xd240`, the turret) and keep that instead.

| caller | what it does with it |
|---|---|
| `0x1004ec50` at `0x1004f338`, `0x1004f364` | the unit box's title, `"%s-%s %s"` (`0x100765e0`, [38-designs.md](38-designs.md)) |
| `0x100506d0` at `0x1005151b`, `0x10051541` | the takt's **Accept**: stores it in the designer's `+0x04` (`0x10051546`) |
| `0x100544b0` at `0x10054520`, `0x1005454c` | the unit writer's class word ([30-turrets.md](30-turrets.md)) |

Nothing else reaches it. `0x1008a590`, `0x1008a500` and `0x1008a690` are not
among `iron3d.dll`'s eight exports, and a raw scan of every module in the
install for their addresses as little-endian words finds **none of them in any
vtable or data table**. The control on that scan is that it does find the
addresses the docs say are installed in one — `Terrain.dll:0x1007ed40` at
`0x1009c19c` and `0x10080550` at `0x1009c214`, `CSun`'s object slot 3 and
`CLightManager`'s slot 9.

### `0x1008a690` is the designer's part category — *read*, and *measured*

The same record `0x1008a500` fills, read on different fields, gives a
**category 0–7**, and the number is a dispatch index and nothing else. The
function is a leaf — a chain of compares and returns, no calls — and it asks,
in order: kind 9 sub-kind 32
→ **0**; kind 9 sub-kind 33 → **1**; kind 8 by its *second* sub-kind — 81
`BLD` → **5**, 80 `TUR` → **1**, 84 `UPG` → **1**, 82 `DEF` → **1**, 83 `RDR`
→ **2**; kind 12 → **2**; kind 11 sub-kind 69 `BRN` → **6**; kind 10 → **4**;
kind 11 sub-kind 68 `ARM` → **7**; and then `sete`/`lea ecx, [ecx*4 - 1]`
(`0x1008a761`–`0x1008a770`), which is **3** for any other device and
**−1** for anything else at all.

So a building is filed by its branch, not its kind: a bunker's turret fits as a
turret and a tower's radar as a gun. Its two readers are both in the designer
and both use it as a jump table index:

- **fitting a part** (`0x100519e0`): `dec eax; cmp eax, 6; ja` and the table at
  `0x10051b88` — turret `0x10052570`, gun `0x10052fb0`, device `0x10052d10`,
  ammunition `0x10053510`, armour `0x100537b0`, building and brain nothing.
  Category 0 and −1 both fall past the bound, so **choosing a chassis once a
  project exists does nothing**; the chassis path is taken earlier, at
  `0x10051a5c`, on there being no preview object yet
  ([38-designs.md](38-designs.md#fitting--read-and-measured)).
- **taking one off** (`0x10053a50`): `cmp eax, 7; ja` and the table at
  `0x10053cdc`, entered at the category itself — 0 and 5 both go to
  `0x10053b00`, which throws the whole project away; 1 `0x10053df0`, 2
  `0x10054210`, 3 and 4 clear a slot of the design's `0x330`-stride array, 6
  nothing, 7 `0x10053c62`. Being unsigned, −1 falls past this bound too.

*Measured* over the shipped trees: all **29** carry the same 395 parts and the
same split — **27 chassis, 74 turrets, 67 guns, 104 devices, 58 clips, 34
buildings, 6 brains, 24 armours**, and exactly **one** part that falls through
to −1: `R_H_01`, *Hero target*, the only `SHS:TAR` record. The designer can
neither fit nor unfit it.

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

## Researching — *read*

A technology is researched by **a research centre carrying out order 14**. The
player's research panel gives the order
([41-commander.md](41-commander.md#the-research-page-4--read-and-seen)):
- it goes to the end of one centre's queue, as
  [31-packages.md](31-packages.md#the-orders--measured) reads;
- the centre runs its orders one at a time, so **each centre researches one
  technology at a time**;
- the panel spreads a queue over several centres by giving each order to the
  one with the fewest.

### The task

`M_Task_Research`'s table is at `Behavior.dll:0x10059f58`, 17 slots:

| slot | address | does |
|---:|---|---|
| 0 | `0x10036060` | returns 14, the order |
| 1, 2 | `0x10036070`, `0x10036050` | destroy and stop; both run `0x1002f400`: the ore request (property `0x2000100`) to 0, `SetPowerUsage(0)` |
| 3 | `0x1002f470` | `SetTarget` |
| 6 | `0x1002f710` | start: stamps the clock (`+0x50`) |
| 7 | `0x1002f730` | the takt, `OnBehaviourTakt` |
| 12 | `0x1002f460` | interrupt priority: **0 for every reason**, so nothing the centre sees interrupts it |
| 14 | `0x1002f440` | the label, `"Research (%X)"` |

- **`SetTarget`** with target `0x204` and a technology id appends the id to the
  task's own list (`+0x58`) and succeeds (`0x1002f51e`). Target `0x205`, a
  design's name, is the AI's order 16. Any other target logs
  *"Research: Incorrect Target"* and fails.
- **Which tree it reads.** The tree is the centre's `MBehaviour+0x50`. `Capture`
  (`0x10008e40`, at `0x10008fe3`), `SetClan` (`0x10009130`, at `0x10009226`) and
  `ReloadSuperAI` (`0x10008c70`, at `0x10008d6f`) each set it from slot 10 of the
  clan's SuperAI. So **a captured centre
  researches from its new clan's tree** (*derived*).

### The takt

Each takt (`0x1002f730`):
1. **The step.** dt is (now − last) × 0.001 s, and last becomes now.
2. **With no technology in hand** (`+0x6c` is −1), it walks its list for the
   first id whose slot-3 record reads *not researched* and *available*. It logs
   the others as *"is not ready for research"*. The first one found becomes
   current (`0x1002f8db`), and:
   - **Free.** If the centre's `FreeTechnoNum` (`MBehaviour+0x9f4`) is above
     zero, it is decremented and the ore and power costs are 0 (`0x1002f8f8`).
   - **Otherwise** the power cost is the item's research energy and the ore
     cost its research ore: record words 4 and 5, `values[0]` and `values[1]`.
   - **The time cost** is the centre's `FreeResearchTime` (`+0x9f8`,
     `0x1002f942`).
   - All three collected amounts start at 0. The centre's power use is set to
     its profile's `Use_Power`, and its ore request to the ore cost.
   - **If no id qualifies**, it logs *"cannot research more..."*, clears the
     request and the power use, and returns 0: **the task is over**
     (`0x1002fa37`).
3. **A current technology already researched** (by another centre, say) is
   dropped: *"allready researched"*, and the current id goes back to −1.
4. **Otherwise it accrues.** Take *old* as the smallest of three fractions:
   time collected over the time cost, ore over ore, and power over power. A
   zero cost's fraction is 1. Then:
   - **Ore.** The take is `KPD × Use_Ore × dt`, held to what the ore property
     (`0x2000100`) holds, and it is added to the ore collected. The property is
     then written **0**, whatever the take left, with the ore still missing as
     its most, which is the request: halved while more than 5 is missing, and 0
     once none is (`0x1002fbdf`–`0x1002fc27`;
     [below](#the-ore-a-centre-takes--read)).
   - **Power and time.** Power collected grows by `KPD × Use_Power × dt`, and
     time by dt.
   - **Progress.** *new* is the smallest of the three fractions again. The
     item's stored progress (slot 4) grows by *new* − *old*, held to 1, and is
     written back (slot 5, `0x1002fe0f`).
   - **Done.** When the stored progress + 0.001 reaches 1 (`0x1002fe16`), it
     logs *"researched"*. It then completes the technology through the centre
     (`0x1002febf`), sets its progress to 1, and makes −1 current again. It
     returns 1, so the next takt starts the next id in its list, or ends.

### How often a building's takt runs — *read*

**Every 64 to 127 ms, on the first frame past its timer.** The chain, from the
mission loop down:

1. `iron3d.dll`'s loop calls `stdCalculateGame` (`World3D.dll:0x100139a0`) once a
   pass, uncapped ([14-controls.md](14-controls.md#a-row-that-stays-down--read)).
2. That runs the queue's slot 4 (`0x10006bf0`), which sends every object in the
   registry message 6 with 1 and the clock (`0x10006c6f`).
3. A building's object hands message 6 on to the agent it wraps
   (`Terrain.dll:0x10057b55`), and the agent to its behaviour
   (`AniMesh.dll:0x100013a1`).
4. The behaviour's message 1 (`Behavior.dll:0x100059f5`) calls its slot 65
   (`0x10006350`), which, once the behaviour is initialised (flag 2), calls
   `MBehaviour::takt`, slot 55 (`0x10006386` → `0x10004c40`).
5. That calls a building's takt, slot 57, for any Type with the top bit
   (`0x100050a2` → `0x100054a0`), a neutral clan's excepted
   ([31-packages.md](31-packages.md#which-objects-run-a-behaviour--read)).
6. **The building's takt tests its timer `+0x930`** (`0x100054d4`), and only when
   that has run out does it run the repair decision and **the task stack's takt**
   (`0x100054fd` → `0x10034a30`), which calls the current task's slot 7
   (`0x10034a56`) — here `M_Task_Research`, a factory's `M_Task_Construct` or a
   mine's `M_Task_Mine` alike.

The stack's takt has four callers, all in `0x10004c40` and its two takts
(`0x10004db5`, `0x10005228`, `0x1000526d`, `0x100054fd`), and a building is on
one of them. Its timer's words are 1 and 1 (`0x10003995`–`0x1000399c`), so when it
runs out it is set to now + 64 ms + 64 ms × the low byte of a `rand()` ÷ 256
(`0x1004c550`): **64 to 127 ms**, tested once a frame, restarted from the frame
that saw it.

**What that bounds.** dt is measured, so no time is lost: the task's clock is
stamped as it starts (`0x1002f710`), and each takt takes all that has gone by.
A research's time budget is only ever met late, by less than one gap. With the
free 5 s, *progress + 0.001* reaches 1 at 4.995 s, so the takt that completes it
falls between **4.995 s and about 5.12 s** after the order, a frame more at the
most; and the check that says *"Research complete..."* runs once a pass of the
game loop ([41-commander.md](41-commander.md#the-research-page-4--read-and-seen)).

*Seen* in `training mission 4 teleport`, frame by frame at the recording's 30 a
second: the row's button turns to the stop cross at 303.633 s, and the box
*"Research complete... (Large Battle Turret)"* is up at 308.633 s: **5.00 s, to a
frame**. That is a check and not a proof: it fits the bound, and a takt every
frame would fit as well. What separates the two is the ore, next.

### The ore a centre takes — *read*

**Both, because the two are one thing.** Property `0x2000100` is a field of the
behaviour: slots 18 and 19 (`0x100092c0`, `0x100093e0`) reach a list at `+0x5e8`,
whose entries keep the value at `+0x14` and the most at `+8`, and the setter
writes what it is given, unclamped (`0x100269e0`). So the get in the takt
(`0x1002fbb4`) reads what the centre holds. But *what it holds* is only ever what
the distribution has brought it since its last takt, because that takt wrote 0
over it:

| who writes a centre's ore property | at | value | most |
|---|---|---:|---:|
| the distribution step, what it was given | `0x1001a8b7` | held + received | — |
| the same, with less than 0.01 wanted | `0x1001a26b` | its most, from no holder | — |
| the behaviour's default capacity | `0x100087cb` | 0 | 0 |
| the task's start of a technology | `0x1002f98d` | 0 | the ore cost |
| **each takt, after the take** | `0x1002fc27` | **0** | the request |
| the task's end, *"cannot research more..."* | `0x1002fa6d` | 0 | 0 |
| the task's stop or destruction | `0x1002f423` | 0 | 0 |

(*Read*, as a search: of the 68 pushes of `0x2000100` in `Behavior.dll`, the
research task's are five, `0x1002f41d`, `0x1002f987`, `0x1002fa67`, `0x1002fbae`
and `0x1002fc21`; the distributor's four that a building that holds no ore can
reach are `0x1001a1af`, `0x1001a265`, `0x1001a870` and `0x1001a8b1`.)

**The control is the factory**, which has the same want and the same 0: its build
moves `min(request, held)` out and then writes 0 with the request as its most
(`0x10015540`, `0x1002a604`; [23-economy.md](23-economy.md#ore-reaches-the-factory-at-one-a-second-from-each-holder--read)).
The two differ in what bounds the take. A factory takes up to what it asked for,
so nothing is lost. A centre takes up to `KPD × Use_Ore × dt`, and **what is over
that is burnt**: the 0 is written whether the take emptied the property or not.

**What the two timers make of that** (*derived*). A step comes every 192 to 255
ms, a takt every 64 to 127, so each step's delivery is met by exactly one takt
and the takts after it find 0. That takt's gap is what the take is bounded by, and a
step falling at random lands in a gap of 99 ms on average (the gaps' mean, 95.5,
plus their variance, 341, over it). So research collects at most

```
KPD × Use_Ore × 99 ms   each step of 223.5 ms   =   0.443 × KPD × Use_Ore a second
```

whatever the clan has, and it never sees the rest of what it paid for. Over the
four grades (`Use_Ore` is 1 on `prof_institute`):

| centre | `KPD` | the most it collects | from one small holder, 1 ore a second | from two, 2 a second |
|---|---:|---:|---:|---:|
| small, RC-17 | 1 | 0.44 a second | 0.44, and 0.56 burnt | 0.44, and 1.56 burnt |
| medium, RC-30 | 3 | 1.33 | 1, none burnt | 1.33, and 0.67 burnt |
| large, RC-47 | 5 | 2.2 | 1, none burnt | 2, none burnt |
| enhanced, RC-67 | 7 | 3.1 | 1, none burnt | 2, none burnt |

The request does not stop it: while more than 5 is missing it is half of what is
missing, far above what a step brings. So a small centre with a mine and a
warehouse pays about **4.5 ore for each one it collects**, and the 35 ore of a
Large Battle Turret takes 79 s there however full the warehouse stands.

### Completing — *read*

`0x10023aa0`, the research centre's completion:
1. It calls the tree's slot 7. That sets the researched and available bits,
   and gives the available bit to every item in the tree whose prerequisites
   are all researched ([above](#trf1-is-state-not-a-label)).
2. It walks the item's unlocks (slot 10). An unlock that is now in the tree,
   available and not researched, **and whose two research costs are both 0**,
   is completed the same way, recursively (`0x10023b34`).

**So a free technology is researched the moment it opens.** Nothing waits in a
queue for it. *Measured*, over all 29 trees:
- 1,318 free items start waiting;
- no tree starts with a free item already open.

**The message comes from `iron3d.dll`, not from here.** *"Research complete..."*
is posted by the game loop's check of the queued orders
([41-commander.md](41-commander.md#the-research-page-4--read-and-seen)).

### What follows — *derived*

- **No size gate.** Neither the order nor the takt reads a size or level byte of
  the item or the centre. A centre's grade gates research only through the
  tree's edges, which name the centres
  ([The spine](#the-spine)).
- **Cancelling keeps what was done.** An aborted order stops the task, which
  clears the request and the power use. Nothing refunds what was collected, and
  the item's stored progress stays. Started again, a research needs only the
  rest of its budgets. A free one takes another of the centre's
  `FreeTechnoNum`.
- **Without ore, only free research finishes.** An ore cost's fraction grows only
  by what the distribution brings, so on a map with no ore a paid technology
  never completes.
- **What sets the pace.** The time budget is fixed. `KPD` speeds only ore and
  power. Paid at an enhanced centre (`KPD` 7, `Use_Power` 3, `Use_Ore` 1), the
  Large Battle Turret's 12 energy would take 12 ÷ 21 = 0.57 s, against the default
  2 s of time, and its 35 ore 11 s at the most, the 3.1 a second
  [above](#the-ore-a-centre-takes--read), and 35 s from one small holder. There is
  no ore on hand to take at once: a centre holds what the last step brought it.
  (This line once gave 5 s, `KPD × Use_Ore × dt` with the ore on hand.)

## What an AI clan's tree gates — *read*, and *measured*

Every clan holds a tree, an enemy clan as much as the player: the `.trf` its
record in `data.tma` names ([04-missions.md](04-missions.md#clan)), loaded as
its own object with the state bytes that file ships. `iron3d.dll` hands each
tree to its clan's SuperAI as the mission's objects are made — the tree is
asked whose it is, `IResearch` slot 24, and that clan's SuperAI is given it
through its slot 21 (`iron3d.dll:0x100605e7`–`0x100605fc`,
`ai.dll:0x10001fa0`) — and the SuperAI's slot 10 hands the same pointer back
(`0x10001f90`), which is the `MBehaviour+0x50` a clan's buildings research
from ([The task](#the-task)). Nothing gives an AI clan a fuller tree than the
file's: its state changes only as its own research completes.

**Two things read its researched bit for an AI clan**, and both ask the same
question — every part of a design *researched* and *in the tree*, each part
found by its id through slot 2 and read through slot 3:

- **the design store**, which marks the designs the clan's build orders may
  pick, when it loads and each time the clan orders a research
  ([15-behaviour.md](15-behaviour.md#the-byte-at-0x104-is-the-clans-research--read));
- **the factory**, as it starts a bot that is not free
  ([36-factory.md](36-factory.md#what-the-start-refuses--read)).

A part the tree does not list at all fails both, slot 2 answering −1.

**What an AI researches is a design, not a technology.** Function 41 gives a
research centre order 16 with a design's file name (`ai.dll:0x10011100`), and
the design is the cheapest of those whose every part is *available* and in the
tree but not yet all researched, priced by
`ArealMap.dll:CalcFullResearchCost` (`0x10016280`;
[below](#what-function-41-does-for-a-clan--read-and-measured)). A design with a
part that is out of the tree, or one that is waiting on a prerequisite, is never
chosen.

*Measured*, the 59 warrior designs of `UNITS\UNITS\AI\` against the nine trees
of the clans whose scripts build by name:

| tree | every part researched | a part out of the tree (0) | a part waiting (4) | every part open, not all researched |
|---|---:|---:|---:|---:|
| `data` | 7 | 0 | 47 | 5 |
| `c1m4e` | 27 | 32 | 0 | 0 |
| `c2m1e` | 28 | 31 | 0 | 0 |
| `c2m3e` | 11 | 48 | 0 | 0 |
| `c3m1e` | 27 | 32 | 0 | 0 |
| `c3m2e`, `c3m2e2` | 44 | 15 | 0 | 0 |
| `c4m2e2` | 43 | 12 | 0 | 4 |
| `scream` | 7 | 0 | 47 | 5 |

(A design is counted once, by its worst part: out of the tree before waiting
before open.) All 59 name only parts each of the nine lists. **On the six trees
written for one enemy clan, a design is either researched whole or has a part
that is not in the tree**, so there is nothing for the clan to research, and
none of those six clans' scripts calls function 41. The two scripts that do
call it, `c4m2e2` and `scream`, are the two whose trees leave designs open: 4
and 5. `c1m3e`'s clan reads `data` too and never asks.

**When the clan asks again** (*read*, and *measured*). Both scripts raise
`PBM_MAKE_RESEARCH` 20 / 1 from `Problems0`, and its `_Start` is the same in
both, two calls of the stub function 0 in `scream`'s aside: it takes a
`BUILDING_INSTITUTE`, attaches it as the problem's critical
unit, calls function 41, and then files `ACTION_NOTHING_DOING` and
`ACTION_DESTROY` on the centre; the `_Continue` has no nodes. The clan's takt
tests those at its head
([15-behaviour.md](15-behaviour.md#what-ends-a-problem--read-and-measured)):
while the centre has its order 16 the problem stands, and the takt after its
order list is empty the problem is retired and the centre let go, so the next
raise asks function 41 for the next cheapest design. A centre captured or
destroyed under it ends the problem at the next takt as well — a critical unit
lost, and `PBM_MAKE_RESEARCH` is the one code whose problem ends on its units
gone whatever its drain (`ai.dll:0x100056c8`). The `ACTION_DESTROY` is dead
weight: a building's shell answers its id for good.

### Who marks an item researched — *read*, as a search

Three things set the researched bit, and no script is one of them:

- **the file**, `TRF1`'s 7s, granted as the mission loads;
- **a research centre's completion** (`0x10023aa0`), which has two callers, the
  takt at `0x1002febf` and itself at `0x10023b34`, and asks the tree's slot 7 at
  `0x10023ac7`;
- **a save's load**, slot 26 (`MisLoad.dll:0x100035f0`): it ORs the bit into every
  index the save lists and then calls slot 7 on each (`0x1000365f`, and
  `0x1000372f` for the other form of the file). Slot 26's one caller is the tree
  object's own forwarder (`0x1000289f`, through `+0x130`).

**The search.** 414 calls in the install go through `+0x1c`, and slot 7 can be
reached no other way: the routine's address is in the table at `0x1000e18c` and
nowhere else. Asked of each by what it calls on:
- in `Behavior.dll`, 74, of which **one** loads its object from `+0x50`, the
  behaviour's tree, and that is `0x10023ac7`;
- in `ai.dll`, 36, on the object lists at `+0x35c`, `+0x3d8` and `+0x3dc`, and none on
  the store's tree at `+0x20` or the pointer slot 10 hands out. So **no function of
  the script table marks anything**;
- in `iron3d.dll`, 116, with none on the player's tree at `0x1010c380`;
- in `MisLoad.dll`, 4: slot 26's two, and two on the archive a file is read through
  (`0x10003100`, `0x100041a6`);
- the 11 in `World3D.dll` that `analysis/slotcalls.py` flags, for sitting in a function
  that asks for interface `0x502`, are on an object being added, and push three
  words, a kind of 0, 1 or 2 and a value, where slot 7 takes an index
  (`0x10007f6e`, `AddObject`).

The same search for slot 3, whose callers are known, finds them
(`Behavior.dll:0x1002f7ed`, `ai.dll:0x10011629`). The engine loads no save, so of
the three it has two.

### What function 41 does for a clan — *read*, and *measured*

**The walk** (`ai.dll:0x10010f90`) passes over a record whose byte is set; leaves
clear one with a part out of the tree or not yet available; sets the byte of one
whose parts are all researched; and prices the rest. **The price is ore alone**:
`CalcFullResearchCost` answers two sums and the walk compares the first, `rfOreCost`
(`ArealMap.dll:0x10016728`, `0x100110a0`), the research ore of each part not
researched, a part counted once. The cheapest goes to the store's `+0xc`, the first
of an equal pair; and with none open `+0xc` keeps what it had, which is how the
clan stops: that record's byte is set by then, so `0x10011145` gives no order and
function 41 answers 0.

**The order** (`0x10011100`) is order 16, replacing, with target `0x205` and the
scheme's path. `SetTarget` reads the file itself (`Behavior.dll:0x1002f5c9`) and
hands the parts to `0x10023790`, which appends each one not researched to the
task's list, in the scheme's order, once. So the takt researches them one after
another, each with its own budgets and each taking a `FreeTechnoNum` if the centre
has one. Nothing but `ACTION_NOTHING_DOING` tells the clan it is over.

**The byte at `+0x25`, function 56**, says the walk found every byte set. It reads
0 on both trees for good, since 27 records of `c4m2e2`'s are out of the tree and 61
of `scream`'s wait on a parent no design asks for.

*Measured*, the 77 records in store order against the two trees, each design
researched whole before the next is priced:

| `c4m2e2`, 43 may be built | ore | energy | what the order lists | what joins |
|---:|---:|---:|---|---|
| `32_l2` | 43 | 16 | Large Howitzer | `32_l2`, `AI_L2`, `AI_LT_21` |
| `AI_LW_31` | 19.5 | 20 | Large Missile Lr | `AI_LW_31` |
| `AI_LW_21` | 50 | 25 | a Large HQ Turret | `AI_LW_21`, an HQ |
| `AI_LS_31` | 95 | 55 | a Large HQ Turret, Large Rail Gun | `AI_LS_31`, an HQ |
| `AI_LT_31` | 40 | 25 | Large Blue Laser | `AI_LT_31`, an HQ |
| | **247.5** | **141** | | **50 may be built, 27 never** |

| `scream`, 9 may be built | ore | energy | what the order lists | what joins |
|---:|---:|---:|---|---|
| `swelbld1` | 5 | 8 | Small Red Laser | `swelbld1`, `sweltrn1`: a builder and a transport |
| `23sfly1e` | 15 | 10 | Small Flying Chs | `23sfly1e`, `laser_c1`, `speed_c1` |
| `laser_c2` | 10 | 2.5 | Small engine, with the free Large and Medium engines | `laser_c2`, `speed_c2` |
| | **30** | **20.5** | | **16 may be built, 61 never** |

`data.trf` reads the same as `scream.trf` on every line, and `c1m3e`, which reads
it, never calls 41.

**Function 65 stays 0.** None of the four designs with 105,000 of guns and a medium
or large chassis (`lwing1`, `lwing2`, `mwelssm1`, `mwing1`) is among those 16, so
`dLargeResearched` never turns on and `scream`'s `dLargeResearched == d1` block is
dead on the shipped trees (*measured*). `scream` is the one caller.

**Which clans it changes.** The 4 clans that run a script calling 41:

| mission | clan | its centre | ore | from |
|---|---|---|---|---|
| C04 M02 | `Enm2`, `c4m2e2` | a small one, placed | a medium mine, no storage | `dTime3` on: 600 s on easy, 150 s on medium, 0 on hard |
| Single.01 | `Enemy`, `scream` | a small one, placed | a small mine and a small storage | its first takt |
| Single.02 | `enemy`, `enemy1`, `scream` | none placed | none placed | only once it has built them: a raise of `PBM_BUILDING_NEEDED` |

**What it does to what they build** (*measured*, by the strength formula over the
game's own floats). `scream` keeps its spread at 0, so its clans always take the
best of `SELECT_BEST_COMBAT`: `12spd1`, a tiny spider, until the second order's 20
ore in all, and `23sfly1e`, a Small Flyer, from the refresh after, with `laser_c1`
and `laser_c2` behind it and the spider fourth. `Enm2`'s best two, all a hard level's
spread reaches, are `AI_LS_21` and `AI_LT_10`; `32_l2` and `AI_L2` come in behind those
for the first order's 43 ore, and `AI_LW_31`, worth 14,878, goes to the head for
the second's 19.5, ahead of `AI_LS_21`. An order for a design does not build it: a
factory is told at the clan's next want, a free mind permitting.

The raise is at 0.52 and does not come first. The `_Start` pass stops at the first
handler that will not start
([15-behaviour.md](15-behaviour.md#the-planner-when-a-_start-runs-and-when-a-_continue--read-and-measured)),
so a heavier want with no unit to send, `Enm2`'s capture at 0.55 say, keeps the
research out of it until that want's life runs down: 25 takts, 175 s (*derived*).

## Mission 04's tree — *measured*, and *seen*

*"In the Research Center, research the missing components of the large flying
warbot and construct it in the Factory."*

**The clans.**
- The player's clan `Plr` researches from `tut4_pl.trf` and has **3 minds**.
- The neutral `Ntrl` reads `data.trf`.

**The starting state** of `tut4_pl.trf` is 368 items:
- **341 out of the tree.**
- **26 granted:**
  - the Large Flying chassis L-2f (`R_B_02`);
  - the large internals: engines 1–3, batteries, shield generators, detection
    shields, sensor modules, repair units, deflectors, and armour 1 and 2;
  - the large research centre RC-47 and the large factory FB-47L;
  - six brain modules.
  - **No gun of any kind.**
- **One open: the Large Battle Turret 4L1** (`e_tur_bb_01` hung, `e_tur_bt_01`
  upright).
  - It costs 12 energy and 35 ore to research.
  - It needs RC-47 and FB-47L, both granted.
  - It unlocks the Large Battle Turret 5L1, which is out of the tree: the panel
    names it as a descendant, but it can never be researched.

**The missing component is that one turret**, by the designer's pages
([37-designer.md](37-designer.md)):
- A large factory's chassis page offers `R_B_02` alone.
- The chassis's turret socket, `e_tur_bb`, offers nothing until 4L1 is
  researched. Then it offers `e_tur_bb_01`, which has no gun socket.
- With the default internals, the design's box reads **37 / 28 t, 58 kph,
  23 %, 0 %, 350 m**.

**The buildings.**
- **The research centre.** `einst01.dat` is the *Enh Research cntr RC-67*:
  - `KPD` 7;
  - `FreeTechnoNum` 5 and `FreeResearchTime` 5, the only centre in the install
    granted either ([23-economy.md](23-economy.md#the-four-grants-a-mission-gives-a-building--read-and-measured)).
- **The factory.** `lplant01.dat` has `FreeBotNum` 50.
- **No ore.** The map has no lode.

**So** (*derived*):
- **The research.** The turret costs nothing and takes 5 s: without the free
  technologies it could never finish.
- **The build.** The flyer costs no ore and 1 power, and 60 s on a large
  factory ([23-economy.md](23-economy.md#construction--read)).

**Seen** in the recording, `training mission 4 teleport`, at 960 × 720:
- **302.5 s — the screen.** The hero stands on the centre's pod and the
  *Research Center* screen is up. The box shows *Large Battle Turret*, under it
  *DESCENDANTS* and *Large Battle Turret (5L1)* (the font's 5 reads like an S),
  and the turret turning. One row reads *Large Battle Turret (4L1)* with the
  start icon.
- **303.5–303.75 s — the click.** The player clicks the batch button, whose
  tooltip *Mark all items to research* shows at 304 s. The row's icon becomes
  the red stop cross, and a fill grows from the row's left: red at 303.75 and
  304.25 s, olive by 304.5 s. The Energy row falls from 99% to 93%.
- **304.75 s — closed.** The screen has gone and the hero is still in the pod.
  The research goes on.
- **308.75 s — done.** *"from: System / Research complete... (Large Battle
  Turret)"*: 5.0–5.25 s after the click, which is the free 5 s.
- **336–346 s — the factory.**
  - The warbot constructor offers the *Large Flying Chs (L-2f)* alone.
  - *SELECT WEAPON* reads *NO ITEMS AVAILABLE*.
  - The box reads *LFW-X Warrior 37 / 28 t, 58 kph, 23 %, 0 %, 350 m*, as
    measured above. Tuned, it reads 45 / 20 t, 60 kph, 26 %, 0 %, 400 m.
  - Production starts at 346 s, with the free-bots icon and **1** free mind.
  - The player then has the helicopter and the captured HQ, so **the hero
    holds no mind** (*derived*).

### For an engine

1. **Order 14** puts a technology on a centre's queue. The centre runs one at a
   time, and a task's list holds the ids it was given.
2. **The start.** A research starts only on an item in the tree, available and
   not researched. It takes a free technology while the centre has one (costs
   0), else the item's research energy and ore. Its time is the centre's
   `FreeResearchTime`. Set the centre's power use to `Use_Power`.
3. **On each of the building's takts, every 64 to 127 ms, with dt the time since
   the last:**
   - add `min(KPD × Use_Ore × dt, ore held)` ore, `KPD × Use_Power × dt` power
     and dt time;
   - set the ore held to 0, whatever it was, and ask the next step for the
     request;
   - progress is the smallest of the three fractions, a zero cost counting as
     1;
   - keep the progress on the item, not on the task.
4. **At progress 1 − 0.001:**
   - mark the item researched, and open what it unlocks;
   - research at once every newly open item that costs nothing;
   - end the task when no queued id is left.
5. **Mission 04.** Its only research is the Large Battle Turret 4L1: free, 5 s,
   at the enhanced centre.

### Not established

- ~~**The ore take.** Whether property `0x2000100`'s get is the centre's ore held
  or what the distribution step has delivered.~~ **Read**: both, because each
  takt writes 0 over the property after its take, so what it holds is what the
  distribution brought since the last, and what is over the take is burnt: a
  small centre collects 0.44 ore a second at the most
  ([The ore a centre takes](#the-ore-a-centre-takes--read)).
- ~~**Who else completes a technology.** Only the takt and the routine itself call
  `0x10023aa0` (*read*, as a search). Calls to the tree's slot 7 made through
  its table, from a script or a save, were not searched.~~ **Read**, as a search:
  a save's load, and nothing else
  ([Who marks an item researched](#who-marks-an-item-researched--read-as-a-search)).
- **What happens to a research when its centre is captured, upgraded or
  destroyed** mid-way. Not followed, on the task's side. On the plan's, an AI
  clan's `PBM_MAKE_RESEARCH` lets go of a centre it has lost at its next takt
  ([What an AI clan's tree gates](#what-an-ai-clans-tree-gates--read-and-measured)).
- ~~**How often a building's takt runs**, which fixes how late the message can
  be.~~ **Read**: on the first frame past a timer of 64 to 127 ms, so the message
  comes at most about 0.13 s behind a research's time budget
  ([How often a building's takt runs](#how-often-a-buildings-takt-runs--read)).
- **What `ArealMap.dll:CalcFullResearchCost` adds for a part that is not yet
  available**: its parents, through `0x10015fe0`, not followed. A design the
  store's walk prices has every part available, so the price it reads is the
  parts' alone.

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
- ~~What `iron3d.dll:0x1008a690` does with the small number it derives from a
  part's bytes, and whether anything but the part lists reads a part's
  derived `Type`~~ — **closed**: the number is the **warbot designer's part
  category**, 0–7 with −1 for none, and it is a jump table index in the only
  two places that ask for it, the fit (`0x100519e0`) and unfit (`0x10053a50`)
  dispatches. The derived `Type` has six call sites in three functions, all
  three the designer's — the unit box title, Accept, and the unit writer —
  and neither function's address occurs in any vtable or data table in the
  install ([above](#0x1008a690-is-the-designers-part-category--read-and-measured)). Over
  the 395 parts of every one of the 29 trees the categories fall 27/74/67/
  104/58/34/6/24 with one part, `R_H_01`, uncategorised, and 293 of the 395
  derive `Type` 0.
- ~~Which tree an AI clan holds, with what state, and whether a clan's research
  changes what it may build~~ — **read**, and *measured*: its own `.trf` with
  the flags the file ships, handed to its SuperAI as the mission loads
  (`iron3d.dll:0x100605fc`); the design store marks a design buildable once
  every part of it is researched there and marks again each time the clan
  orders a research, and a factory asks the same of a paid bot. Over the nine
  trees of the clans that build by name, 7 to 44 of the 59 warrior designs are
  researched whole, and on six of the nine every other design has a part out of
  the tree ([What an AI clan's tree gates](#what-an-ai-clans-tree-gates--read-and-measured)).
