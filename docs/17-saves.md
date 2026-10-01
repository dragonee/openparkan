# Save games — `.sav`

`SAVE/` holds seven slots: the six of the game menu's save page, all filled in
this installation, from 20 KB to 106 KB, and a seventh that is the quick
save's, empty here. A `saveslots.cfg` beside them indexes them
([below](#saveslotscfg-and-the-seven-slots--read-and-measured)).

**A save is a fixed sequence of sections, and all six parse to the last
byte.** Most of the bytes inside the sections are the classes' own memory —
string fields whose tails were never zeroed, so `LFW-7 Warrior` is followed
by `0f 00 00` rather than a terminator, and a few words of stack — which is
why the file looked like a heap dump for so long. But the order is fixed, and
every section either says how long it is or takes its count from a place the
reader can reach. The sequence below was *read* from the writer
(`iron3d.dll:0x100a1590`) and its loader (`iron3d.dll:0x100a2bd0`), and
*measured* against the six saves: `openparkan.save.read(path, game)` walks
all six to their final byte.

## The header — *read* and *measured*

```
char[4]   "SLOT"
uint8     version, 1 everywhere
uint8     difficulty: 0 EASY, 1 MEDIUM, 2 HARD
int32     length of the mission path
char      path[length]     'missions/campaign/campaign.05/mission.01/'
```

The writer emits it field for field through one primitive, `0x100b4b34`,
`fwrite(ptr, size, count, file)`; `mov dword ptr [esp + 0x4c], 0x544f4c53` at
`0x100a1637` is the only place `SLOT` is written, and `0x100a2ced` in the
loader compares against it.

**The version is a real version.** The loader reads each clan's leading word
(below) only when that byte is at least 1 (`0x100a30d8`), so a version-0 save
simply lacks it.

**The second byte is the difficulty, not campaign against single.** The writer
takes it from the settings object's `+0x150` (`0x100a1663`), the loader writes
it back there (`0x100a2d42`), and the level ratio at `iron3d.dll:0x10076010`
reads that same field as `EASY`, `MEDIUM` or `HARD`. Five saves hold 0 and the
one skirmish holds 1; an earlier draft read the correlation as the meaning.
The path's own directory is what says campaign or single.

## The sections — *read*, *measured* 6 of 6

```
header
int32     size, then the world                     World3D.dll, below
int32     clan count
int32     objective count, then per objective
              char[255] text, int32 state, int32 exempt
per clan  int32 a word of the clan's (version >= 1)
          int32 x minds    the clan's mind list: a unit id or -1
int32     count, then 24-byte records
int32     count, then per unit design
              uint32 0xF0F1, uint32 Type, 112-byte components depth first
int32     1, int32 id, int32 id
int32     clan count again
per clan  int32 size, then the clan's AI state     ai.dll, below
```

**The one count the file does not carry is a mind list's length.** The writer
loops over the clan record's `+0x20`, which is the clan's `minds` word from
`data.tma` ([04-missions.md](04-missions.md)), and writes no count; the loader
does the same. So the world can be read from the save alone, and everything
after it needs the mission. With it, all six saves end exactly: 216 objects,
12 objectives, 4 designs and 20 AI states.

What each section is:

| section | who writes it | what it holds |
|---|---|---|
| world | `World3D.dll`'s queue, slot 22 (`0x10009a90`) | every game object, below |
| objectives | `iron3d.dll:0x1006b180` | text, a state the script sets — 1 complete, −1 failed, 0 open — and a word that exempts the objective from the mission's completion test (`0x1006b130`) |
| clan word | `0x100a1770`, the clan record's `+0x10` | 1 to 57; **unknown** |
| mind list | the SuperAI's slot 17 per entry | a unit's id or −1, one per mind |
| 24-byte records | the game object's `+0x700` member, `0x10081990` | none in four saves, 2 and 4 in two; **unknown** |
| unit designs | `0x100569b0`, recursive | a whole `.dat` assembly in the `.dat`'s own layout ([07-objects.md](07-objects.md)): the 108-byte body, the child count, the children |
| `1, id, id` | `iron3d.dll:0x10063130` over the object at `+0x20` | `1, -1, -1` in all six; **unknown** |
| AI state | `ai.dll` SuperAI slot 19, `0x100020f0` | the clan script's state, below |

**An earlier draft said the body was length-prefixed blobs throughout**, from
walking `(int32 length, bytes)` twice before it failed. Only the world is
such a blob, and so is each clan's AI state at the end. The walk's "second
blob" was the clan count read as a length — `02 00 00 00`, taking two bytes of
the objective count with it — and the next "length" it rejected ran into the
first objective's text.

## The world — *read* and *measured*

The world is what `World3D.dll`'s object queue hands the writer from its slot
22: a 48-byte header — a 1, the queue's game time (`0x10032a38`, the value
`SetGameTime` sets), then ten words of stack — and one record per object,
parent before child (`0x10009bc0`):

```
uint32    id              top byte a class, the rest a serial
char[128] archive         'objects.rlb', or empty
char[128] member          'fr_l_gener', 'DATA\MAPS\KM_14\land'
int32     parent's id     0 for none
int32     parent's slot   -1 for none
uint32    bytes after this 276-byte head
int32     property 0x803 of an object whose type slot answers 3, else 0
uint32    n, then n chunk sizes, then the chunks
```

The archive and member names come from `World3D.dll`'s two name tables,
3000 × 128 bytes each, indexed by the id's low word. The 128-byte archive
field is what the reference scan below called the *wide* record.

**The id's top byte is the `objects.rlb` tag of the member it names**
(*measured*, 184 of 184 model objects):

| top byte | tag | what | objects |
|---|---|---|---:|
| `0x13` | `FORT` | a building | 40 |
| `0x14` | `BTLU` | a unit or creature | 60 |
| `0x19` | `BULL` | a round in flight | 47 |
| `0x1a` | `STAT` | scenery | 37 |
| `0x11` | — | the landscape, no chunks | 6 |
| `0x17` | — | the sky, one 4-byte chunk | 6 |
| `0x1b` | — | a research tree, 3 chunks | 20 |

**The chunks are the object's owners' own state.** The queue asks the object
for them through its slot 17 — `AniMesh.dll:0x10001c20` for every model — and
chunk 0 is a byte per owner saying how many chunks that owner wrote, in a
fixed order, absent owners skipped:

| owner | kept at | chunks it writes |
|---|---|---|
| the part list | `AniMesh` object `+0x6e4` | 1 if the object has parts, else 0 |
| the model | `+0x188`, from `AniMesh.dll:0x10016a70` | 1: the object's scale |
| the control system | `+0x18c`, from `Control.dll!LoadControlSystem` | 1: the placement, below |
| the wizard | `+0x194`, from `Wizard.dll!CreateWizard` | 2 |
| the behaviour | `+0x190`, from `Behavior.dll!CreateBehaviour` | 4 |

Units hold `01 01 01 02 04` or, without parts, `00 01 01 02 04`; scenery and
rounds `00 01 01`. The load side reads the bytes back the same way
(`AniMesh.dll:0x10001d60`, slot 18). Every unit, scenery piece and round holds
exactly 1 plus the sum of those bytes; **all 40 buildings hold one chunk more,
of 4 bytes**, which no owner in the table accounts for (*measured*).

### The 450-byte record was three chunks

The records the reference scan measured as "450 bytes plus a multiple of 8"
are scenery and rounds: a 276-byte head, the chunk table, a 3-byte chunk 0, a
12-byte scale and a **143-byte control chunk that grows in 8-byte steps**:

| control chunk | scenery | rounds |
|---:|---:|---:|
| 143 | 33 | 1 |
| 151 | 4 | 27 |
| 159 | 0 | 5 |
| 167 | 0 | 14 |

So the step that "separated the furniture from everything else" separates
scenery from **rounds in flight** — the `bf_b_01`, `bp_b_04`, `bb_b_02` names
are `BULL` records — and a name appearing with two step counts (`bp_b_03`,
`bp_b_04`, `s_stone_13`) is two objects of that name in different states.
What an 8-byte step holds is **not established**.

The `uint16` that "rose in file order" at `+0x1be` of a scenery record was,
on every 450-byte one, the serial of the next record's id — 450 bytes on,
four bytes before that record's archive name (33 of 33).

## The part list — *read* and *measured*

A model's part list is `AniMesh.dll`'s own array, handed over whole
(`0x10003660`) except for its first entry, which is the object itself:

```
char[32]  archive
char[32]  member
int32     +64  the id of the part this one hangs off; 0 for the object
int32     +68  the node of that part's mesh it bolts to, or its slot
int32     +72  this part's own id
```

`0x10003760` attaches a part: it gives it the lowest id not already in use,
looks its member up in `objects.rlb`, and turns `+68` into a place on the
machine by the record's tag — for an `EXTO` part it adds what the object at
`+0x13c` answers from slot 14 for `+64` (one less when `+64` is not 0), for an
`INTO` part what the object at `+0x164` answers from slot 17. That is the
node-or-slot split the `.dat` attachment field has
([07-objects.md](07-objects.md)), taken relative to the parent part.
`0x100036a0` reads the chunk back as `size / 76` records, each attached in
turn.

Measured over 906 part records in 94 lists:

- **`+72` is unique within every list**, 94 of 94. Ids are handed out
  lowest-free as parts come and go, which is why [18-vocabulary.md](18-vocabulary.md)
  saw the numbers run down as often as up.
- **`+64` names the object (0) or another part of the same list, 906 of 906.**
- **Every ammunition part hangs off a weapon (`WPN`) at attachment 0** — a
  clip in slot 0 of its gun, 69 of 69. Every weapon hangs off another part at
  a non-zero node, 154 of 154 (140 of them an `SHS` part, 14 a `BLD` one), and
  every `SHS` part off the object itself, 57 of 57.

The earlier "round count" reading of ammunition's `+64` was the gun's id.

## The placement — *measured*

The model's chunk is three floats, **the object's scale**. The control system's
chunk opens on a flags word — `0x10000ff0` on all 184 — then **the orientation
as a quaternion `(w, x, y, z)`** and **the position**, at `+4` and `+20`.

So the name and the position were always in the same record, and the question
of how to join them is answered by reading the record rather than by windows:

- **42 objects stand within 0.25 of where their mission placed them**, every
  one read at `+20` of its own control chunk — the same 42 the byte scan
  found before the record was known.
- **41 carry the placed name**: 11 scenery pieces their own, 30 units and
  buildings the member of their `.dat`'s root part (a mission places
  `gener01.dat`; the save names `fr_l_gener`). The 42nd was placed as a
  `splant01.dat` (`fr_l_plant`) and saved as `fr_m_plant`: a plant upgraded
  since.
- All 42 hold the placement's scale, and the 34 upright ones its angle `a`
  as `(cos a/2, 0, 0, −sin a/2)`.
- **Scenery keeps its placement but not exactly.** All 37 scenery records lie
  within 5.7 units of a placement of their own name; 7 stand on it upright
  and every one of the other 30 is tipped off the vertical. That is why
  `s_tree_55`'s saved (511.2, 163.0, 221.8) is not the mission's (509.5,
  164.2, 222.2). Trees and stones settled onto the ground is the obvious
  reading; it is a **guess**.

## The AI state — *read* and *measured*

Each clan's last section is what `ai.dll`'s SuperAI hands over from its slot
19 (`0x100020f0`): a 2000-byte block, 36 more bytes of counts and small
fields, four bytes for every script variable, and 28 for each open problem of
one kind and four for each entry of another. `varset.var` declares 231
variables ([15-behaviour.md](15-behaviour.md)), so a clan with nothing open
holds **2036 + 4 × 231 = 2960 bytes — 16 of the 20 clans exactly**; the other
four hold 2972 and 2988.

## What a save refers to

| save | mission | map | research trees | members |
|---|---|---|---|---:|
| `slot1` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 114 |
| `slot2` | `campaign.04/mission.01` | `C4M1` | `data.trf` | 81 |
| `slot3` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 18 |
| `slot4` | `campaign.03/mission.02` | `32` | `c3m2p`, `c3m2e`, `c3m2e2`, `data` | 387 |
| `slot5` | `campaign.05/mission.01` | `KM_14` | `data.trf` | 18 |
| `slot6` | `single.02` | `SC_1` | `scream.trf` | 540 |

Every one of those maps and research trees is installed. `slot4` naming
**four** trees is a mission with three opposing clans carrying a
[research tree](16-research.md) for each, plus the shared `data.trf`: each
tree is a world object of class `0x1b`.

## The member references

Before the sections were read, references were recovered by **scanning** for
an archive name followed by a member name 32 or 128 bytes later. The scan is
kept, because it checks the archive side independently: **1342 references,
and all 1342 resolve** into the archive they name, the game's own lookup
folding case (`objects.rlb` holds `r_l_03` where a save says `R_L_03`).

Every one of the 1342 now has a home in the parse: **184 record heads** (the
128-byte form), **906 part records**, **201 in control chunks** and **51
design components**. The narrow form's measured stride — 812 of 1152 gaps
exactly 76 — was the part lists, diluted by the other two populations.

## Three ways in that did not work

Written down because each is the obvious next idea.

**The pointers do not resolve to file offsets.** Taking every plausible
pointer value and every record offset, the best constant delta maps **3 of
540** records — noise. The records were written one at a time into a buffer,
so a pointer identifies an object only to the engine that wrote it. The parse
made the question moot: records carry ids, and a part's parent is an id.

**A part record carries no pointer identity.** No field is unique across a
save — the most varied holds 26 distinct values in 114 records. The identity
it does carry is local: `+72`, unique within its own list.

**Two saves of one mission differ mostly in rubbish.** `slot3` and `slot5` are
90.4% identical, and the regular difference — a word every 450 bytes, `+0x54`
holding `0x0019f4e8` — is the Windows main-thread **stack** range. The parse
says why it recurs at that stride: `+0x54` is inside a record head's 128-byte
archive field, and `World3D.dll:0x10009bc0` copies the two names into a stack
buffer and writes all 128 bytes of each, tail and all. A field holding
`0x0019xxxx` is stack junk; the world header's last ten words are another
example.

## Positions are off the four-byte grid

An earlier version said a save does not store where anything stands, on a scan
that stepped four bytes at a time and found 10 matching triples. Stepping
**one** byte finds **42**, at every alignment — the records sit wherever the
variable-length chunks before them end. **An alignment assumption is an
assumption**, and a negative resting on one is worth no more than it.

## `saveslots.cfg` and the seven slots — *read* and *measured*

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
`filename`, which is how the reader tells them apart. *Measured*: seven slots,
`slot1` to `slot7`, each naming `slot<n>.sav`; six not empty, each of those six
naming a `.sav` that is on disk; the seventh named `empty`, marked empty, with
no `slot7.sav`.

**One object keeps the index** (`iron3d.dll:0x1008c300`, vtable `0x100e6678`,
0x14 bytes): the parsed file at `+0x10` and a list of slot records at `+4`.

- **Reading** (`0x1008c710`): `saveslots`' `quantity`, then one record for each
  index below it, bound to the object `slot%d` of the index plus one. A record
  answers its `name` (`0x1008c060`), its `filename` (`0x1008c140`) and its
  `empty` (`0x1008c220`) out of the parsed file each time it is asked.
- **Writing.** Setting a slot's name (`0x1008c930`) or its empty flag
  (`0x1008cb00`) writes the whole file at once: `DeleteFileA` on
  `save/saveslots.cfg`, then the file's own writer (`0x100c5e60`). The
  constructor and the destructor (`0x1008c4c0`) do the same, so the file is
  written again every time the index is opened, and the banner's date is the
  last time it was. *Measured*: the banner reads `14/6/2026 21:38` and the
  file's own time is 14 June, 21:38.
- **Who makes one.** The game, each time it runs a mission (`0x1005c8be`, kept
  at its `+0x34`), and the shell's slot list, a temporary one each time it is
  filled (`0x1001e182`).
- **The path is relative**, `save/saveslots.cfg`, where a save's is the current
  directory, `/save/` and the slot's `filename` (`0x100c8390` asks
  `GetCurrentDirectoryA`).

**Slots 0 to 5 are the save page's, and slot 6 is the quick save's.** The save
page makes six slot widgets and its click tests six
([39-boarding.md](39-boarding.md#the-game-menu--read)); the quick save and the
quick load push the index 6 ([14-controls.md](14-controls.md#quick-save-and-quick-load--read-and-seen)).
So the seventh slot is on no save page, and only F7 writes it.

**The writer has three callers and no others** (`0x100a1590`, every `call` to it
in the module):

| caller | which slot | what it does to the index |
|---|---|---|
| the save page's *Save* (`0x10065767`) | the selected one, 0 to 5 | the typed name, then `empty` FALSE |
| Enter on the save page (`0x10065c88`) | the same | the same |
| the quick save (`0x100a50e4`) | 6 | `name` *"Quick Save"* (string 6245), then `empty` FALSE |

Each hands the writer the slot's `filename`, and the writer opens the current
directory + `/save/` + that name for `"wb"` (`0x100a15b9`–`0x100a1626`). All
three write the same file, so a quick save's header and sections are a save's.
**No `slot7.sav` is installed to measure it against**: this installation's
quick slot was never written.

**A save's name is what was typed after the name already there.** The slot
widget opens on the slot's `name`, a letter or a digit is appended while it is
under 16 characters, and Backspace takes one off the end
([39-boarding.md](39-boarding.md#the-game-menu--read)). *Measured*: the six
filled slots are named `emptyfde`, `emptyfdffff`, `emptygrfdfg`,
`emptydfsdfsdf`, `emptydsadsd` and `emptyfdd` — the seventh's untouched `empty`
with 3 to 8 letters typed after it, 8 to 13 characters, none over 16.

### Loading — *read*, and *seen*

**A load is a mission started with a slot index.** The parameter block the
executable hands the game ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured))
carries the slot at `+0x148` (−1 for none) and a fresh-start byte at `+0x154`.
With the byte clear:

1. `Run` takes the slot's `filename`, opens the file under `/save/` and reads
   its `SLOT`, its version, its difficulty and its mission path, which it copies
   into the block (`0x1005ddfd`–`0x1005df3b`);
2. the mission loads from that path as any does;
3. the level then calls the loader (`0x100a1e24` → `0x100a2bd0`), which opens the
   same file again and reads the sections above.

Two things set the block so:

| who | slot | how |
|---|---|---|
| the shell's load-game screen, a row chosen (`0x100130f5`–`0x10013143`) | the row's index, 0 to 6 | only if `slot%d.sav` of the index plus one is a file whose first four bytes are `SLOT` (`0x10013260`); then mode 1, `+0x148` the row, `+0x154` clear, no mission path, and the shell starts the game |
| the executable, on the game's exit code 4 (`iron_3d.exe:0x4012b0`) | 6 | the same block with 6, and the game runs again with no shell between |

- **The load-game screen lists all seven.** Its list (`0x1001e100`) makes a row
  for every slot of the index, named by the slot's `name`, and disables the row
  of a slot whose `empty` is TRUE (`0x1001e26b`, `0x100c9e10`). So the quick
  save is its seventh row, *"Quick Save"* once F7 has named it.
- **The screen checks the file by its number, the game opens it by its
  `filename`.** The two agree in the installed index, where slot *n* names
  `slot<n>.sav`.
- **Exit code 4 has one source**, the quick load
  ([14-controls.md](14-controls.md#quick-save-and-quick-load--read-and-seen)).

**A load takes the clans' clocks back with the world.** A clan's AI state
carries the seconds clock its takt steps, the SuperAI's `+0x854`
(`ai.dll:0x10002399` writes it, `0x10002510` reads it back), and the reader
re-bases the word beside it on the wall clock, `+0x858` = `timeGetTime` − clock
× 1000 (`0x10002516`–`0x1000253b`). So what a script times off that clock
([34-progression.md](34-progression.md#when-the-mission-handler-runs--read))
is as far along after a load as it was at the save, whatever was played since.

*Seen*, in the bonus part of *The Convoy* (`9SBZOCWv_vE`): a quick save at 6:56.7
and two quick loads of it, their *Exiting...* panels at 7:22.85 and 7:50.07 and
play back at 7:24.2 and 7:51.3. The two throw away 26.2 s and 25.9 s of play,
so every clan's clock is 52 s behind the video from there on (*derived*).

*Seen*: a save made in a bunker's command view comes back on foot. In the
recording of *The Convoy* (`-yNnsqudMzw`) the box *"from: System / Game
saved..."* stands over the command view from 12:56.5 to 13:20.0, longer than
one box's 20 seconds, so the player saved there at least twice; the load at
13:57.1 opens on the hero standing inside the bunker, and the player is back in
the command view by 14:01. So the interface's mode stack is not in the file
(*inferred*).

## Not established

- **What most chunks hold.** The part list is read, the scale and the control
  chunk's first 32 bytes measured; the rest of the control chunk, the wizard's two
  chunks, the behaviour's four, a building's extra 4-byte chunk and a research
  tree's three are not. The 8-byte step of a scenery piece's or round's
  control chunk is the smallest handle.
- **The clan word** before each mind list (`clan record + 0x10`, 1 to 57).
  The next handle is whoever writes that field.
- **The 24-byte records** of the game object's `+0x700` member and **the
  `1, id, id` triple** from the object at `+0x20`.
- **The AI state's layout** beyond its size: which 2000 bytes are fixed and
  what the 28- and 4-byte entries are. `ai.dll:0x100020f0` is the writer and
  its slot 20, `0x10002400`, the reader.
- **A quick save's own bytes.** The quick save goes through the one writer
  ([above](#saveslotscfg-and-the-seven-slots--read-and-measured)), so its file is
  a save like the other six; this installation has no `slot7.sav` to walk.
- **What a save keeps of the interface.** *Seen*, a save made in a command view
  loads on foot ([Loading](#loading--read-and-seen)). Whether the message box,
  its history or the mode stack is anywhere in the file is not read; the
  unknown chunks above are where they would be.
- **What a load does with a slot whose file is gone.** The shell's screen and
  the quick load both test the file first; `Run`'s own open (`0x1005de8e`) is
  not followed past a failed `fopen`.
- Whether a mind list's ids are the units' logical ids: `slot1`'s player list
  holds 12, 27, 28 and 29, and the mission places a hero with logical id 12,
  but the world records carry serials, not logical ids, so nothing in the
  file joins the two yet.
