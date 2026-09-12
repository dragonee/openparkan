# Save games — `.sav`

`SAVE/` holds up to seven slots, six of them filled in this installation, from
20 KB to 106 KB, beside a `saveslots.cfg` that indexes them.

**A save is not a designed file format.** It is the engine's live object graph
written to disk more or less as it sat in memory: the classes' own 32-byte
string fields, heap addresses left in place, and buffer tails that were never
zeroed, so a name is not reliably NUL-terminated — `LFW-7 Warrior` is followed
by `0f 00 00`, not a terminator. Reconstructing the graph would mean
reconstructing the classes.

So this document does something narrower and says so plainly: it reads the
header, which is a real header, and recovers **what a save refers to**, which
is the part worth having and the part the installation can check.

## The header

Parsed strictly, and it reads on all six:

```
char[4]   "SLOT"
uint8     version, 1 everywhere
uint8     0 on the five campaign saves, 1 on the single mission
int32     length of the mission path
char      path[length]     'missions/campaign/campaign.05/mission.01/'
```

The path is the one genuinely length-prefixed string in the file; everything
after it is fixed-size fields. All six paths resolve to an installed mission
directory.

## What a save refers to

| save | mission | map | research trees | members |
|---|---|---|---:|---:|
| `slot1` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 114 |
| `slot2` | `campaign.04/mission.01` | `C4M1` | `data.trf` | 81 |
| `slot3` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 18 |
| `slot4` | `campaign.03/mission.02` | `32` | `c3m2p`, `c3m2e`, `c3m2e2`, `data` | 387 |
| `slot5` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 18 |
| `slot6` | `single.02` | `SC_1` | `scream.trf` | 540 |

Every one of those maps and research trees is installed. `slot4` naming
**four** trees is the interesting row: a mission with three opposing clans
carries a [research tree](16-research.md) for each, plus the shared `data.trf`
— which is what the per-mission wiring in those archives is *for*.

The size of a save tracks how much world there is, not how far in you are:
`slot3` and `slot5` are the same mission minutes apart and both 20 KB, while
`slot6`'s skirmish holds 540 references.

## The member references

The engine's record for "a thing from an archive" is two 32-byte string
fields, the archive then the member:

```
+0    objects.rlb\0   ...pointer junk...
+32   i_arm_b_05\0    ...pointer junk...
```

**The pair is written two ways**, and an earlier draft of this section knew
only one of them. The member name sits 32 bytes after the archive name in the
common record and **128** in a second one. A scan that looks only at 32 does
not report the others as unresolved — it never counts them — so its hit rate
flattered its coverage by about a fifth.

Scanning for both shapes recovers **1342 references across the six saves, and
all 1342 resolve** into the archive they name. This is still a **scan, not a
parse**, and the reader says so; a perfect hit rate against real data is what
it has instead of a decode.

The two records are not one field written loosely. The wide one names objects
the mission itself places — 7, 8, 8, 3 and 8 of them on five of the six saves
— and **the narrow one names none, on any save**. So the wide record is a
placed world object and the narrow one is something else, most likely the
parts a machine is assembled from.

An earlier draft also explained four references that "do not resolve": they
paired `objects.rlb` with `R_L_03`, `R_L_04` and `R_L_05`, and were read as
part ids from the research tree's `TRF6` rather than archive members. **That
was a case-sensitivity bug and the explanation was invented to fit it.**
`objects.rlb` holds those members as `r_l_03`, `r_l_04` and `r_l_05`, and the
game's own lookup folds case — `NResArchive.find` says so. Nothing in a save
names a research part id where a member belongs.

So the member field holds an archive member, always.

## The two records have sizes

Both records repeat at a measurable stride, and the strides differ — which is
the strongest evidence yet that they are two kinds of thing.

**The part record is 76 bytes.** 812 of the 1152 gaps between consecutive
narrow records are exactly that, the two-string record
[18-vocabulary.md](18-vocabulary.md) already measures fields in. The rest are
88, 112 and larger, which is what runs separated by their owner's own data
look like.

**The world record is 450 bytes plus a multiple of 8.** Every one of the 82
gaps below 500 bytes between wide records is 450, 458, 466 or 474 — 82 of 82,
no other value — so a world object has a fixed part and a short variable one.

The step is not noise: **it separates the map's furniture from everything
else.**

| | records | steps taken |
|---|---:|---|
| `s_tree_*`, `s_stone_*` | 37 | **0** on 33, 1 on 4, never more |
| everything else | 45 | 1 on 26, 2 on 5, 3 on 14, **never 0** |

The names in the second row are `objects.rlb` members like `bb_b_02` and
`bp_b_04` — not `.dat` assemblies and not `objects.dlb` parts, so the step is
not a count of anything the model files carry.

Nor is it a constant of the model: **three of the 24 names appear with two
different counts** (`bp_b_03` and `bp_b_04` at 2 and 3, `s_stone_13` at 0 and
1), so the same object saved twice can take a different number of steps. That
makes it at least partly the instance's own state. A list of attached parts
remains the obvious reading and is still a **guess**; what is measured is the
split and the variation.

## Inside the world record

The fixed part carries a `uint16` at **+0x1be** that, on scenery, **only ever
increases down the file** — 7, 8, 9, 10, 12, 14, 22 in one save — with
`0xffff` where it holds nothing. Five of the six saves have scenery and all
five rise. So it is an identity assigned in order.

It is **not an index into the mission**: not into its object list, not into
its statics, not into its non-statics, under any shift. `slot3`'s counter 9 is
`s_tree_54` where the mission's ninth object is `s_tree_93`, and the offsets
between the two orders are not even constant.

And the same offset is **not the same field on every object**. On fourteen
records of `slot1` it holds the low half of `1.0f` instead. So the 450-byte
record is a size, not a layout: the classes inside it differ, which is what a
dump of live objects looks like and why this is hard.

## Inside the part record

A census of all 1158 part records puts real fields only in the last twelve
bytes. Everything before is the 32-byte archive field, the 32-byte member
field, and the uninitialised tails of both.

The three `int32` are **not three fields every part uses.** They split by what
the part is:

| kind | non-zero at `+64` | non-zero at `+68` |
|---|---:|---:|
| `AMM` ammunition | **69 of 69** | **0 of 69** |
| `WPN` weapons | 165 of 165 | 165 of 165 |
| `SHS` chassis | 8 of 65 | 65 of 65 |
| `BLD` buildings | 7 of 43 | 43 of 43 |
| `DVC` devices | 146 of 615 | 530 of 615 |

**Ammunition is the only kind that never uses `+64`'s neighbour**, and the
only kind that always uses `+64`, where it holds 10 to 20. A round count is
the obvious reading and `objects.dlb` cannot confirm it: its stat rows name
the fields a part displays (`Weight`, `Blast area`, `Damage`) without giving
values, so there is no capacity to compare against. **Guess**, marked as one.

`+72` is the ordinal [18-vocabulary.md](18-vocabulary.md) describes, still
unexplained.

## Three ways in that do not work

Written down because each is the obvious next idea, and each costs an
afternoon to rule out.

**The pointers do not resolve to file offsets.** A heap dump usually keeps its
pointers, and if the file were a contiguous image of a heap region then
`file offset = address - base` for one constant base. It is not. Taking every
plausible pointer value and every known record offset, and asking which delta
maps the most *distinct* records, the best reaches **3 of 540** — noise. The
records were written one at a time, not copied as a block, so a pointer
identifies an object only to the engine that wrote it.

**The part record has no identity.** If a record held its own address, the
graph could be rebuilt from the file alone. No field in the 76 bytes is unique
per record: across 114 records in one save the most varied field holds **26
distinct values**, and most hold fewer than ten. A part record is a *value* —
a kind and some state — not an entity something else can point at.

**Two saves of one mission differ mostly in rubbish.** `slot3` and `slot5` are
the same mission and **90.4% identical**. The differences are regular — one
two-byte change per world record, every 450 bytes — which looks like the
signal until you read it: the dword at `+0x54` is `0x0019f4e8` in `slot1`,
which is the Windows main-thread **stack** range. It is uninitialised buffer,
and it differs because the two saves were written with a different stack under
them. Differential analysis works mechanically; those two saves are simply
nearly the same state.

That last one is worth keeping as a **tool**: a field holding `0x0019xxxx` is
stack junk, and so is a field that differs between two saves of identical
state. Mapping the rubbish before decoding anything is the way to avoid
inventing meaning for it — which this project has already done once, with the
`R_L_03` "research part id" that was really a case-sensitivity bug.

## Positions are in a save, off the four-byte grid

An earlier version of this section said a save does not store where anything
stands, on the evidence that a scan found 10 matching `float32` triples across
all six saves. **That scan stepped four bytes at a time.** The records sit at
arbitrary byte offsets — the two-string fields are not aligned to anything —
so three quarters of the file was never looked at.

Stepping **one** byte finds **42** triples matching a position the mission
places, against 10 on the dword grid. `slot4` alone accounts for 20 of its 27
placed objects. The matches land at every alignment: 0, 1, 2 and 3.

Relative to the nearest world record they cluster at **`+0x143`** (scenery),
**`+0x161`** (`fr_b_ruin`) and **`+0x1b1`** (`fr_l_gener`) — three classes,
three offsets, which is the same "450 bytes is a size, not a layout" result
from the other side.

What is **not** established is the join between a record's name and the
position at its offset. Reading `+0x143` of every scenery record gives
plausible, stable coordinates — `s_tree_55` is (511.2, 163.0, 221.8) in all
three saves of that mission — but they are not the mission's placement of
*that* name. So the field is real and the attribution is not; a record's name
field and the position near it may belong to different objects, which the
450-byte window cannot resolve.

The lesson is the one this project keeps relearning: **an alignment assumption
is an assumption.** A negative that rests on one is worth no more than the
assumption.

## `saveslots.cfg`

Plain text in the engine's `OBJECT` / `END` form, tab-separated:

```
OBJECT saveslots
	quantity		=	7
END

OBJECT slot1
	name		=	"emptyfde"
	filename		=	"slot1.sav"
	empty		=	FALSE
END
```

The first object declares the slot count and is not itself a slot — it has no
`filename`, which is how the reader tells them apart. Seven slots, six not
empty, and each of those six names a `.sav` that is on disk; `slot7` is marked
empty and no `slot7.sav` exists.

## What is not read here

Everything else, which is most of the file:

- **The object graph.** Units, buildings, their components, positions, damage,
  the player's resources, mission progress — all present, none decoded. The
  76-byte two-string record is the one structure identified, and it appears in
  runs of a few to a few hundred, separated by the owning object's own data.
- **The heap addresses.** `0x10106b98` appears at the same place in all six
  saves, so some of what is written is a vtable pointer, meaningless off the
  machine that wrote it.
- Whether the version byte's second half really means campaign against single,
  or something else that happens to correlate. One of six saves has it set.
