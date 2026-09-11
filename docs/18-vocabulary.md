# The object vocabulary, and what a save shows of it

A note rather than a format document. Reading the [save games](17-saves.md)
put a census of the engine's live objects in front of us, and the names in it
turn out to be systematic. This writes down what those names are, what the
data beside them holds, and — where there is nothing but a guess — says so.

**Every claim below is tagged.** *Measured* means `openparkan verify`
re-derives it. *Guess* means it fits the data and is not established.
*Unknown* means exactly that.

## The naming scheme

An `objects.rlb` member is `<family>_<size>_<index>`. All three slots read.

### The size letter — *measured*

| letter | size |
|---|---|
| `b` | large |
| `m` | medium |
| `l` | small |
| `t` | tiny |
| `f` | huge |

Checked against the display name each assembly gives its parts:
**140 of 142 agree**. The two that do not are `A_L_05`, whose label
`L Large Lobster` is a creature's and does not follow the scheme, and
`R_B_06`, labelled `Small Tower (L-22w)` where the letter says large.

This is why `R_B_04` is `Large Track Chs` and `R_L_04` is `Small Track Chs`
— the pair that first made the letters look arbitrary.

*Guess*: `b`/`m`/`l`/`t` read as big, medium, little, tiny. `f` for huge fits
no such pattern and is unexplained.

Two-letter forms (`bc`, `bt`, `mt`, `lt`, `fc`) take the size from the first
letter — every `f*` part is huge, every `m*` medium. *Guess*: the second
letter is a sub-type; nothing tests it.

### The tail — *guess*

Mostly `01`–`07`, a running index. **59 members end in `df`**, and every one
sits where a unit would carry its cheapest fitting, so `df` is almost
certainly *default*. Some tails are words instead: `brige`, `inst`, `ruin`,
`store`, `plant`, `mine`, `bunker`, `angar`.

## The component families

What a save actually holds. Across the six saves, **361 distinct members**,
347 from `objects.rlb` and 14 from `effects.rlb`.

| family | *guess* at meaning | evidence |
|---|---|---|
| `i_arm` | armour | labels are `ARMOUR SA.Mk4` etc. — **measured** |
| `i_eng` | engine | labels are `Large engine` — **measured** |
| `i_pws` | battery / power store | labels are `Large Battery` — **measured** |
| `i_fsh` | shield generator | labels are `Lrg Shld generator` — **measured** |
| `i_dsh` | detector shield | labels are `Large detect.shld` — **measured** |
| `i_rdr` | radar / sensor | label `sensor module`; matches `CICLS_RADAR` |
| `i_rps` | repair system | matches `CICLS_REPAIRSYS` |
| `i_def` | deflector | labels say `deflector` |
| `e_gun` | gun | drawn externally, class 4 in the assemblies |
| `e_tur` | turret | class 1, and guns bolt onto it |
| `e_tow`, `e_bnt` | tower, turret mount | *guess* from the names alone |
| `R_*` | chassis | labels are `Large Track Chs (L-42t)` — **measured** |
| `u_*` | emplacements | `u_bun_def_l_01` reads as bunker/defence |
| `r_shield_b/g/r` | shield effects in three colours | *guess* |
| `bb`, `bl`, `bm`, `bp`, `br`, `ba`, `bf`, `bt` | building types | *unknown* which is which |

The first five rows are not guesses at all: the `.dat` assemblies pair each
member with the display name the game's UI shows, which
[07-objects.md](07-objects.md) already uses.

## `i_cNN` is ammunition, and `NN` names the weapon — *measured*

Sixty-four members are named `i_c01` … `i_c18`. An earlier draft of this note
read the number as the engine's **`CICLS_` component class id**, because the
range lines up and `World3D.dll`'s resolver leaves 6, 7, 14 and 16–18
unnamed. **That was wrong**, and the assemblies say so plainly: every one of
these members is an *ammunition clip*, class 5 in the `.dat` taxonomy, with
labels like `Clip 75mm ammo` and `Winged pack I`. `i_c08` is a howitzer clip,
not a `CICLS_RADAR`. Two small sequential numbers happened to overlap.

What the number really is falls out of the assembly tree. A `.dat` is written
depth first, so a clip's parent is the gun it belongs to, and:

```
e_gun_<size><kind>_<NN>      the gun
i_c<NN>_<size>_<index>       its ammunition
```

**All 588 clips in the shipped assemblies hang off a gun, and all 588 match
that gun on both the number and the size letter.** No exceptions. So `NN` is a
**weapon-type id**, shared by a gun and the clips that feed it, and the pair
(size, `NN`) identifies the weapon: `i_c05_b_*` is 80mm ammunition for
`e_gun_bc_05` while `i_c05_l_*` is a missile pack for `e_gun_ll_05`.

The gun's second letter is its kind, and it splits cleanly — **measured**:

| letter | feeds | count |
|---|---|---:|
| `c` | shells and ammunition clips | 299 |
| `l` | rockets, missiles and winged packs | 289 |

*Guess*: `c` for cannon, `l` for launcher.

Fourteen `i_cNN` members are in `objects.rlb` but in no assembly, and they
split cleanly — **measured**. **Eight are in every research tree's part list**,
so they are buildable and simply never pre-fitted. The other **six are
`i_c06_l_01/02/df` and `i_c07_l_01/02/df`, and they appear nowhere else in the
installation at all** — not in a tree, not in a save, not in `objects.dlb`.
Every one of the 50 clips that *is* fitted is also in the part list, so those
six are the only ones outside the game's own catalogue. *Guess*: cut content.

## Two building sets — *measured*

`bu_` (34 members) and `fr_` (35) supply **the same 34 suffixes**: `bunker`,
`brige`, `inst`, `mine`, `plant`, `ruin`, `store`, `towH`, `towL`, `angar` and
the rest, each in its sizes. One function list, two sets of models.

*Guess*: two architectural styles, most likely the game's opposing sides.
Nothing in the shipped data attaches either prefix to a faction name, so which
is whose is **unknown**.

## The member record in a save

The engine writes "a thing from an archive" as two 32-byte string fields
followed by three `int32`. The strings are
[read](17-saves.md); the three integers are not.

| offset | *measured* | reading |
|---|---|---|
| `+64` | 0 (567), 7 (245), 1 (94), −1 (91), 6, 3 | **unknown.** Not the component class: tested against the class id the assemblies give the same member and it disagrees on 953 of 957. Ammunition records carry 10 and 12 here where internals carry 0, which would suit a quantity, but that is one weak pattern and not evidence. |
| `+68` | 0–7 mostly, with some values that are plainly uninitialised (`0x01010000`) | **unknown** |
| `+72` | 1098 of 1158 hold 1…32; 919 of 1157 adjacent records differ by exactly one | counts along a group of parts. It **runs down as often as up** — `23, 17, 16, 15` in one save, `17, 20, 19, 18` in another — so the obvious reading, a slot number counting from 1, is wrong. **Role unknown.** |

## A negative result worth recording

A save looked like it might hand us the engine's class layout: it is a raw
memory dump, so any vtable pointer in it would name a C++ class and its method
table. **It does not.** Of every word in the image's address range, only four
recur across all six saves, and the two that appear exactly once per save sit
at fixed header offsets — leftovers in a dumped struct, not an object's vtable.
The saves carry heap addresses, not class pointers, so they are no route into
the binaries' class layout.

## What this does not cover

The object graph itself. Units, buildings, positions, damage, resources and
mission progress are all in a `.sav` and none of it is decoded; see
[17-saves.md](17-saves.md). This note is about the *names* and the little that
sits beside them.
