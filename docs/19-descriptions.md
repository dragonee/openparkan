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
| `//G<n>:L<n>` | a group and a level. *Unknown* what either indexes |
| `//<size>:<kind>:<sub>:<mark>:<a>` | the classification, below |
| `#1` | short code — `L152mmC`. On 356 of 395 |
| `#2` | display name — the string the UI shows. On all 395 |
| `#3`, `#4` | free text, and a member may carry several |
| `#5` | for ammunition, the weapon it feeds |
| `#6`…`#A` | five `key=value` numbers, on all 395 |
| `@G@…@B,…@` | one row of the part's stat panel |

The classification's first slot is the **same size letter as a part id**
([18-vocabulary.md](18-vocabulary.md)): `B` large (144), `M` medium (96),
`L` small (92), `T` tiny (26), `H` huge (2), plus `A` (27), `N` and `E`.
The kind slot is six values: `DVC` device (134), `SHS` chassis (78),
`WPN` weapon (62), `BLD` building (58), `AMM` ammunition (58), `ANM`
creature (5). The sub-kind is a three-letter mnemonic — `TUR`, `GUN`, `ARM`,
`LAS`, `ROC`, `MIS`, `FSH`, `DSH`, `BAT`, `REP`, `DEF`, `FLM`, `BUN` — and
the mark runs `MK1`…`MK8`.

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
it is not, and that section now says so. **No shipped file gives a research a
time.**

**297 of the 395 parts cost nothing to research** — both research fields zero —
which is the stock you begin with; the other 98 have to be paid for.

And `#5` states the gun-to-clip link a second time, independently of the
assembly tree that [18-vocabulary.md](18-vocabulary.md) uses: **all 58
ammunition members name the weapon they feed**, 45 of them ending in a short
code that is itself a member and the rest naming it in words
(`9-slt Rocket Launcher`).

## What is not read here

- **`//G<n>:L<n>`.** The group runs 1–4 and the level takes a dozen values;
  neither is tied to anything else in the data. *Unknown.*
- **The fifth slot of the classification line** (`A3`, `A1`), and the `A`, `N`
  and `E` size letters, which no part id uses.
- **The stat *values*.** This file names the fields and their units; what a
  given part weighs is not here.
