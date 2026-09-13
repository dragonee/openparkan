# The parts database — `objects.dlb`

An NRes archive in the installation root, **395 `DSCR` members**, one per part
id — and they are the same 395 ids in the same order as the research tree's
[`TRF6`](16-research.md). Each member is plain text with tagged lines, and it
is the authority on what a part is, because the developers wrote their own
field names into it.

```
//G3:L14
//B:WPN:GUN:MK2:A3
#1L152mmC
#2Large Cannon
#6UpgradeLevel=2
#7ResearchEnergyCost=18
#8ResearchOreCost=50
#9BuildEnergyCost=18
#ABuildOreCost=50
@G@Weight       @B,weight,G,t,5,1@
@G@Rate of fire @B,Frate,G,1/s,5,@
@G@Damage       @B,damage,G,HP,5,@
```

## The lines

| line | what it is |
|---|---|
| `//G<n>:L<n>` | the catalogue group, and a tech level. Both read below |
| `//<size>:<kind>:<sub>:<mark>:<a>` | the classification, below |
| `#1` | short code — `L152mmC`. On 356 of 395 |
| `#2` | display name — the string the UI shows. On all 395 |
| `#3`, `#4` | free text, and a member may carry several |
| `#5` | for ammunition, the weapon it feeds |
| `#6`…`#A` | five `key=value` numbers, on all 395 |
| `@G@…@B,…@` | one row of the part's stat panel |

### The group and the level

`//G<n>:L<n>` is not opaque after all.

**The group is the catalogue's top-level tab** — *measured*. Each of the four
holds one kind, or the two kinds that belong together:

| group | | kinds | parts |
|---:|---|---|---:|
| `G1` | buildings | `BLD` | 58 |
| `G2` | chassis | `SHS`, `ANM` | 82 |
| `G3` | armament | `WPN`, `AMM` | 117 |
| `G4` | devices | `DVC` | 134 |

**391 of 395** sit in the group their kind belongs to; the four that do not
are named in the check. Armament being one tab for both the guns and the
clips they take is the same pairing `#5` states line by line.

**The level is a tech level, and it rises with `UpgradeLevel`** — *measured*.
The bands rise and touch only at their edges:

| `UpgradeLevel` | tech levels |
|---:|---|
| 0 | 0–1 |
| 1 | 2–8 |
| 2 | 9–16 |
| 3 | 14–21 |

**393 of 395** sit in their band, counting level 0 as the free stock; the two
that do not are `e_gun_fc_07` and `i_pws_b_02`. The level also tracks depth in
the [research tree](16-research.md) — a part with no prerequisites is level 0,
and the deepest reach 16–21 — but loosely enough that it is a designer's
grading rather than a computed depth.

**The game does not read it** (*read*, below): nothing at run time gates a part
on its level. What it graded for the designers is not recorded.

### The rest of the classification line

The line is **variable length**: `//B:BLD:BUN:TUR:MK1:A3` has six slots where
`//B:WPN:GUN:MK2:A3` has five, because a sub-kind can be a chain. The mark is
whichever token matches `MK<n>`; everything between the kind and it is the
chain, and one `A<n>` token follows.

The first slot is the **same size letter as a part id**
([18-vocabulary.md](18-vocabulary.md)): `B` large (144), `M` medium (96),
`L` small (92), `T` tiny (26), `H` huge (2), plus three no part id uses — `A`
(27: the brain modules, the `f`-sized battery, shield and repair units, and
the building upgrades and deflectors), `N` (the five creatures) and `E` (the
three enhanced buildings, `fr_e_*`). The research tree files `H`, `A` and `N`
as size 4 and `E` as 5 ([16-research.md](16-research.md)); what the letters
stand for is not written anywhere — `E` for *enhanced* fits the display names,
and `A` and `N` are *unknown*.
The kind slot is six values: `DVC` device (134), `SHS` chassis (78),
`WPN` weapon (62), `BLD` building (58), `AMM` ammunition (58), `ANM`
creature (5). The sub-kind is a three-letter mnemonic — `TUR`, `GUN`, `ARM`,
`LAS`, `ROC`, `MIS`, `FSH`, `DSH`, `BAT`, `REP`, `DEF`, `FLM`, `BUN` — and
the mark runs `MK1` to `MK20`.

**The closing `A<n>` token is a size grade** — *measured*. It follows the size
letter on **358 of 395** parts: `T` → `A0`, `L` and `N` → `A1`, `M` → `A2`,
`B` → `A3`, `E` → `A4`, `A` and `H` → `A5`. The 37 that differ are all weapons
(20) and ammunition (17): rocket and missile launchers and their packs,
mostly a grade or two below their letter (`e_gun_bl_14`, a large missile
launcher, is `A1`; two small packs, `i_c09_m` and `i_c10_b`, sit above), and
the large guns of tech level 0 (`e_gun_bc_25`…`29`, `e_gun_bl_30`…`35`), all
but one `A4` or `A5`. Why those differ is not established.

### What the game takes from this file — *read*

**Nothing but its member names.** `objects.dlb` is named once in the whole
installation, in `iron3d.dll`, and the one function that opens it
(`0x10048220`, `0x100487a4`) builds a list of up to 64 parts: for each it asks
the library whether a member of that part id exists, and if so labels the
entry *"name (code)"* from the research tree's `TRF8` and `TRF7`. It never
reads a member's text.

What the game needs of a part's line it has from the research tree instead:
`TRF0`'s bytes `+0x23`..`+0x26` are the kind, the sub-kinds and the size as
numbers, and `TRFA` holds the stat rows ([16-research.md](16-research.md)).
The tree has no field for the group, the level, the mark or the grade.

A stat row is `@G@<label> @B,<field>,G,<unit>,<width>,<decimals>@`: the label
the panel prints, the field behind it, and its unit. Twenty-seven distinct
rows exist across the library — `weight` in `t`, `damage` in `HP`, `range` in
`m`, `maxspeed` in `kmph`, `Frate` in `1/s`. The same templates are copied
into the research tree's `TRFA` stream.

## What this settles

**The four float32 in a research-tree record are these four costs.** Joined on
the short code, **304 of 329 items match all four exactly** — the rest are
codes the library reuses, so the join picks the wrong entry rather than the
values disagreeing.

```
values[0]  ResearchEnergyCost
values[1]  ResearchOreCost
values[2]  BuildEnergyCost
values[3]  BuildOreCost
```

Two resources, energy and ore, each charged once to research a thing and once
to build it. [16-research.md](16-research.md) previously argued from the
shape of the corpus that the second member of each pair was a **duration**;
it is not, and that section now says so. A research's time budget is the
research centre's `FreeResearchTime`, a mission property
([23-economy.md](23-economy.md#research--read)), not anything in this file.

**297 of the 395 parts cost nothing to research** — both research fields zero —
which is the stock you begin with; the other 98 have to be paid for.

And `#5` states the gun-to-clip link a second time, independently of the
assembly tree that [18-vocabulary.md](18-vocabulary.md) uses: **all 58
ammunition members name the weapon they feed**, 45 of them ending in a short
code that is itself a member and the rest naming it in words
(`9-slt Rocket Launcher`).

## What is not read here

- ~~**The `A<n>` token**~~ — **narrowed**: a size grade that follows the size
  letter on 358 of 395; why 37 launchers, packs and level-0 guns differ is open,
  and the game never reads it.
- **What the `A` and `N` size letters stand for.** Which parts carry them is
  measured above, and the tree files them as size 4; the words are unknown.
- ~~**What the tech level gates exactly**~~ — **closed, negatively**: nothing.
  The game never reads this file's text, and the research tree it does read has
  no level field.
- **The stat *values*.** This file names the fields and their units; the values
  are computed at run time from the part's controller — a weapon's in
  [29-weapons.md](29-weapons.md#what-the-stat-panel-shows--read), a chassis's in
  [24-motion.md](24-motion.md).
