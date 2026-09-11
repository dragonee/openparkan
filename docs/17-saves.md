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

Scanning for that shape — an archive name with a member name 32 bytes after it
— recovers **1158 references across the six saves, and 1154 of them resolve**
into the archive they name. This is a **scan, not a parse**, and the reader
says so; a good hit rate against real data is what it has instead of a decode.

The four that do not resolve are not failures of the scan. They pair
`objects.rlb` with `R_L_03`, `R_L_04` and `R_L_05`, which are not members of it
— they are part ids from the research tree's `TRF6`, and the save stores one
beside a display name (`Small Track Chs (S-42t)`). So the member field holds
either an archive member or a research part id, and nothing in the record
distinguishes them.

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
