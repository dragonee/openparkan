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
(27), `N` (5) and `E` (the three enhanced buildings, `fr_e_*`). The research
tree files `H`, `A` and `N` as size 4 and `E` as 5
([16-research.md](16-research.md)). `E` for *enhanced* fits the display names;
what `A` and `N` refer to is settled [below](#the-a-and-n-size-letters), and
the words themselves are not recoverable.
The kind slot is six values: `DVC` device (134), `SHS` chassis (78),
`WPN` weapon (62), `BLD` building (58), `AMM` ammunition (58), `ANM`
creature (5). The sub-kind is a three-letter mnemonic — `TUR`, `GUN`, `ARM`,
`LAS`, `ROC`, `MIS`, `FSH`, `DSH`, `BAT`, `REP`, `DEF`, `FLM`, `BUN` — and
the mark runs `MK1` to `MK20`.

### The `A` and `N` size letters

Neither is a word we can recover, and both are a referent we can.

**`N` is an animal** — *measured*. Exactly five parts carry it, `A_L_01`
through `A_L_05`, and they are also the only five of kind `ANM`. The research
tree says the same thing independently of this file: **the same five and no
others** carry role byte `+0x22` = 7, the byte
[30-turrets.md](30-turrets.md) already reads as *an animal*, and the agreement
holds both ways round on all 29 trees. Their line is `//N:ANM:SHS:MK<n>:A1` —
grade `A1`, the small grade, which is what their models are — while the tree
files them as size 4 beside `H` and `A`.

**`A` is a fortification's fittings** — *measured*. Twenty-seven parts carry
it, all at tech level 0, all `UpgradeLevel=0`, all graded `A5`: six brain
modules `i_brn_a_01`…`06`, four fortification shields `i_fsh_f_01/02/03/df`,
four fortification batteries `i_pws_f_*`, four fortification repairs
`i_rps_f_*`, six building deflectors `u_{ang,ins,min,mtp,pln,str}_def_a_01`
and three mine upgrades `u_min_upg_a_01/02/03`. Over all 458
`UNITS/**/*.dat`, **21 of the 27 appear in an assembly, and all 31 of the
distinct assembly roots that take one are an `fr_*` building** — a bunker, a
mine, a plant, a store, a tower, a bridge; never a robot.

The file names the referent itself for twelve of them. **Every line in
`objects.dlb` that mentions a fortification belongs to an `A` part**, 12 of
12: `#4Fortification shield` on the four `i_fsh_f`, `#4Fortifications PS` on
the four `i_pws_f`, `#4Fortifications unit M1`–`M4` on the four `i_rps_f`.
Nothing else in the file uses the word. That same free-text slot is what names
`H` too — `e_tur_ht_02` carries `#4HERO`.

The six brain modules are the stranger part of the `A` set: they have **no
`objects.rlb` record at all** (26 of the 32 `A` and `N` parts do), sit in no
assembly and no save, and yet **all 29 research trees list them**, kind 11
`DVC`, sub-kind 69 `BRN`, size 4.

**The words themselves are not recoverable, and there is a control for
that** — *measured*. The token `objects.dlb` is present in `iron3d.dll`, so a
byte search over every shipped `.dll` and the executable does find a string
that is there; the tokens `DSCR`, `ANM`, `DVC` and `ResearchEnergyCost` are in
**no** shipped binary.
The classification line's vocabulary never reaches the code, so there is no
resolver chain to pull the spelling out of, and no shipped text writes `A` or
`N` out in words. They are referents, not words.

### The closing `A<n>` is a size grade, and on a weapon it is the round's

**The `A<n>` token is a size grade** — *measured*. It follows the part's own
size letter on **358 of 395**: `T` → `A0`, `L` and `N` → `A1`, `M` → `A2`,
`B` → `A3`, `E` → `A4`, `A` and `H` → `A5`. All **275** parts that are neither
`WPN` nor `AMM` carry the grade their letter implies, 275 of 275. The 37 that
differ are all armament, and a second rule accounts for 25 of them:

> **The `A<n>` on a weapon is the size of the round it fires, not the size of
> the weapon.**

All 120 `WPN` and `AMM` parts name a round through their controller's firing
component ([29-weapons.md](29-weapons.md)). **100 of the 120 carry the grade
of that round's size letter**; 83 carry the grade of their own; 75 agree on
both. Cross-tabulated: **75 fit both, 8 fit their own letter only, 25 fit
their round only, 12 fit neither** — 75 + 8 + 25 + 12 = 120.

- **25 of the 37 carry the round's grade**: all 17 ammunition packs, and eight
  weapons — `e_gun_bl_12`, `bl_13`, `bl_14`, `bl_15`, `bl_18`, `e_gun_fl_03`,
  `e_gun_ml_10` and `e_gun_ml_11`. These are large or medium launchers loaded
  with medium or small missiles. `e_gun_bl_14` is a large launcher firing
  `bm_l_01`, a small missile, and is graded `A1`; `i_c14_b_df`, the large pack
  that feeds it, is graded `A1` too.
- **8 are exceptions to nothing**, and all eight keep their own `A3`. Seven of
  them, `e_gun_fc_01/02/04/05/06/07/08`, fire a round whose size letter is
  `f` — `bb_f_01`, `bl_f_01/02/03/04`, `bb_f_02`, `bf_f_01`; `f` is not one of
  `t/l/m/b`, so a round of that size grades nothing. The eighth is
  `e_gun_bs_01`, a mobile builder, whose beam `bld_l_01` would say `A1` and
  does not get a hearing.
- **12 fit neither and stay unexplained**: `e_gun_bc_25`–`29`,
  `e_gun_bl_30`–`35` and `e_gun_fl_09`. All twelve are **clip-less** — the
  magazine reads `UNLIMITED` and the class-2 slot is unlabelled (only 1 of the
  25 round-graded ones is). Ten of the eleven at tech level 0 fire **exactly
  the round the clip-fed twin twenty marks earlier fires** — `bc_25`↔`bc_05`,
  `bc_26`↔`bc_06`, `bc_27`↔`bc_07`, `bc_28`↔`bc_08`, `bl_30`↔`bl_10`,
  `bl_31`↔`bl_11`, `bl_32`↔`bl_12`, `bl_33`↔`bl_13`, `bl_34`↔`bl_14`,
  `bl_35`↔`bl_15` — and only `bc_29` differs. They carry `A4` or `A5`
  (`bl_35` `A2`, `fl_09` `A1`) where both readings say `A1`–`A3`. So they are
  the built-in, ammunition-free versions of guns that already exist; **why
  they are graded as they are is not established.**

The control that the letters mean a size in this data at all is
`verify.py`'s existing *a name's size letter is the model's size*, which
re-derives chassis `R_T_/R_L_/R_M_/R_B_` → `T-/S-/M-/L-` and buildings
`fr_l/m/b/e_` → `-17/-30/-47/-67`. The control on the weapon-to-round join is
that, run over all 67 `BULL` records rather than the 61 in question, it
reproduces three families the docs already name: `rg_b_01` under
`e_gun_bc_05`, the Large Rail Gun; the `bld_*` beams under the mobile
builders; and `e_tur_ht_02`'s four guns as `bb_h_01`, `bp_h_01`, `bl_h_01`
and `bm_h_01`, in the gun order [29-weapons.md](29-weapons.md) gives.

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

- ~~**The `A<n>` token**~~ — **answered**: a size grade, the part's own letter
  on 358 of 395 and **the size of the round it fires** on a weapon, which
  accounts for 25 of the 37 that differ; 8 more keep their own letter, seven
  of them because the round they fire is `f`-sized and no grade covers that.
  The 12 clip-less built-in guns are still unexplained. The game never reads
  any of it.
- ~~**What the `A` and `N` size letters stand for**~~ — **answered as far as
  the data can answer it**: `N` is an animal and `A` a fortification's
  fittings, both settled twice over above. The *words* are not recoverable —
  the classification line's vocabulary reaches no shipped binary, and the
  control says a byte search would have found it if it did.
- ~~**What the tech level gates exactly**~~ — **closed, negatively**: nothing.
  The game never reads this file's text, and the research tree it does read has
  no level field.
- **The stat *values*.** This file names the fields and their units; the values
  are computed at run time from the part's controller — a weapon's in
  [29-weapons.md](29-weapons.md#what-the-stat-panel-shows--read), a chassis's in
  [24-motion.md](24-motion.md).
