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
What the 8-byte step counts is open; a list of attached parts is the obvious
guess and nothing here tests it.

## What a save does not contain

**Where anything stands.** Searching an entire save on four-byte alignment for
a `float32` triple matching any position its mission places finds **1 of 22**
on `slot3` and **4 of 27** on `slot4` — chance, against the 96 plausible
triples a file that size holds. No axis order and no sign flip does better,
and `float64` finds none at all. Ten such matches across all six saves against
123 placed positions.

So a save is not a snapshot of the world's geometry. Either placement is kept
in a form unlike the mission's, or the save is a delta over the mission it
names and the untouched scenery is simply reloaded. This is the first thing
anyone decoding the graph will try, so it is written down as tried.

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
