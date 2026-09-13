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
letter — every `f*` part is huge, every `m*` medium. The second letter is now
read: on a turret, `t` is a ground mounting and `b` the same turret hung under
a flyer ([30-turrets.md](30-turrets.md)); on a gun it is the kind — `c` a gun,
`l` a launcher, `s` a module, the mobile builders and the bunker and tower
radars ([29-weapons.md](29-weapons.md)).

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

*Guess*: `c` for cannon, `l` for launcher. The gun records bear it out and add
two letters: `c` covers every gun, flamer, laser and taser — lasers and
tasers take no clip at all — `l` the rocket and missile launchers, `s` the
mobile builders, and `f` the huge guns ([29-weapons.md](29-weapons.md)).

Fourteen `i_cNN` members are in `objects.rlb` but in no assembly, and they
split cleanly — **measured**. **Eight are in every research tree's part list**,
so they are buildable and simply never pre-fitted. The other **six are
`i_c06_l_01/02/df` and `i_c07_l_01/02/df`, and they appear nowhere else in the
installation at all** — not in a tree, not in a save, not in `objects.dlb`.
Every one of the 50 clips that *is* fitted is also in the part list, so those
six are the only ones outside the game's own catalogue. *Guess*: cut content.

## `fr_` and `bu_`: a building and its body — *measured* and *read*

`bu_` (34 members) and `fr_` (35) supply **the same 34 suffixes**: `bunker`,
`brige`, `inst`, `mine`, `plant`, `ruin`, `store`, `towH`, `towL`, `angar` and
the rest, each in its sizes. An earlier draft read that as two sets of models
and guessed two architectural styles, one per side. **It is not two sides and
not two sets of models**; the records' tags say what they are.

Every `fr_` building is a `FORT` record of two slots, and the first names the
`bu_` of the same suffix in `objects.rlb`; the second is the building's `.bas`
ground plan. Every `bu_` is a `BTLU` record — the record kind a creature or a
robot body has — naming five `fr_` resources in `fortif.rlb`: the `.msh`,
`.wea`, `.cpt`, `.ndp` and `.ctl` of the model the building's `.bas` names.
*Measured*: 34 of 34 `FORT` records point at their own `bu_`, all 34 `bu_`
records are pointed at, and all 34 draw on their building's model — 28 of the
same suffix, and the six towers (`towH`, `towL` in three size letters) sharing
`fr_b_tower` and `fr_m_tower`. The 35th `fr_` member, `fr_l_01`, is a `BULL`
round.

The code does the same (*read*): `LoadBuilding`, the `CID_CLASSIC_FORT` loader
([22-settings.md](22-settings.md)), constructs a `CBuilding` that reads the
`FORT` record's four strings and hands its first slot to `AniMesh.dll`'s
`LoadAgent` (`Terrain.dll:0x10055e95`) — so a building is a fortification
wrapped around an ordinary agent, and the `bu_` record is that agent. There is
one set of building models, and no faction in the names.

## The component classes the resolver leaves unnamed — *read* and *measured*

`World3D.dll`'s name resolver knows 13 `CICLS_` classes and leaves 6, 7, 14
and 16–18 without a name ([14-controls.md](14-controls.md)). The `i_cNN`
ammunition was once taken for them and is not (above). What the rest of the
install says about the six:

- **17 is the round's seeker.** 20 components carry it, all on rounds in
  `weapon.rlb`, and `Control.dll`'s factory builds it its own 0xa0-byte
  class (`0x10024760`; [29-weapons.md](29-weapons.md)).
- **6, 7, 14, 16 and 18 are in no shipped controller**: 1066 components over
  20 classes, none of these five.
- **The factory gives them nothing of their own.** `Control.dll:0x1002d4b0`
  switches through a 30-byte index table and sends 3, 6, 7, 11–14, 16, 18,
  20, 22–25, 28 and 29 to one default: the generic 0xa4-byte device
  (`0x1002d6ec`, constructor `0x10020800`).
- **No module asks for them by class.** The `IControl` query that walks a
  controller's components of one class (interface 0x202, slot 9) is called
  with a constant class in `Behavior.dll` (1, 10, 15, 25, 26, 29, 30),
  `iron3d.dll` (1–4, 30), `World3D.dll` (2, 24) and `Effect.dll` (2) — the
  building binder's 26, 25, 29, 10 and 15 are the positive control — and never
  with 6, 7, 14, 16 or 18.
- What the engine does hold for them is a power channel each
  (`Control.dll:0x1003ccc8`): 14 on the batteries' channel, 16 on the engines',
  and 6, 7 and 18 on channel 0.

So the five are ids the engine reserves and nothing ships. *Guess*: cut
component kinds, like the six clips outside the catalogue.

## The member record in a save

The engine writes "a thing from an archive" as two 32-byte string fields
followed by three `int32`. **The 1158 narrow records a scan finds are three
populations**, now that the save is [parsed](17-saves.md): 906 entries of a
model's part list, 201 records inside the control system's chunk, and 51
components of saved unit designs in `.dat` layout. The table that follows
this paragraph used to average the three; the part list's fields are *read*
(`AniMesh.dll:0x10003760`) and *measured*:

| offset | part list, *read* | *measured* |
|---|---|---|
| `+64` | the **id of the part this one hangs off**, 0 for the object itself | 906 of 906 name 0 or another part in the same list; every ammunition part hangs off its gun |
| `+68` | the **node of the parent's mesh** for an `EXTO` part, the **slot of its controller** for an `INTO` one — the `.dat` attachment field, relative to the parent | 0 on all 69 ammunition parts: slot 0 of the gun |
| `+72` | the part's **own id**, the lowest not in use when it was attached | unique within all 94 lists |

That settles what this table used to say. The ammunition "quantity" at `+64`
was the gun's id. The `−1` (91) and the "plainly uninitialised" values were
the other two populations — a design component's `+64` is its flags word, 1.
And `+72` ran down as often as up because ids are handed out lowest-free as
parts come and go; the old "counts along a group" was only ever an
approximate description of a list written in attachment order.

## A negative result worth recording

A save looked like it might hand us the engine's class layout: it is a raw
memory dump, so any vtable pointer in it would name a C++ class and its method
table. **It does not.** Of every word in the image's address range, only four
recur across all six saves, and the two that appear exactly once per save sit
at fixed header offsets — leftovers in a dumped struct, not an object's vtable.
The parse shows what such leftovers are: the world header ends in ten words of
stack, two of which are `iron3d.dll` addresses in every save — `0x10106b98`,
and `0x100a16fb`, the save writer's own return address after an `fwrite`.
Objects are named by ids, not class pointers, so a save is no route into the
binaries' class layout.

## What this does not cover

Most of the object graph. The save's sections, its object records, part lists
and placements are now read ([17-saves.md](17-saves.md)); damage, resources
and most of each owner's chunk are not. This note is about the *names* and the
little that sits beside them.
