# The MISSIONS directory

```
MISSIONS/
  dispatcher.ini              campaign completion flags
  Single.01/  Single.02/      standalone scenarios
  Multi.01/ … Multi.06/       multiplayer scenarios
  Autodemo.00/                attract-mode demo
  CAMPAIGN/CAMPAIGN.00..05/Mission.01..04/
  SCRIPTS/                    shared behaviour scripts: *.scr *.fml *.trf
```

29 mission directories. Each holds:

| File | Format | Contents |
|---|---|---|
| `data.tma` | binary | clans, object placement, routes, mineral lodes — **fully parsed** |
| `mission.cfg` | text | resources, objectives, minimap |
| `descr` | text | one-line description shown in the menu |
| `sky.wea` | text | skybox texture name table |
| `sky.ske` | binary | skybox / celestial body definition |
| `Mistips.mis` | markup | briefing text with `<<tag>>` markup |
| `briefing.cfg`, `messages.cfg` | text | campaign missions only |

```
uv run openparkan missions              # every mission, its map and contents
uv run openparkan mission Single.01     # one mission in detail
uv run openparkan mission CAMPAIGN/CAMPAIGN.02/Mission.03 --list
```

## data.tma

Binary, little-endian. Strings are length-prefixed: a `uint32` byte count then
that many bytes, no NUL terminator.

```
uint32   version, always 1
uint32   route count
routes   { uint32 id; uint32 point count; float32[3] × count }
uint32   the clan table's version, 6
uint32   clan count
clans    × clan count
uint32   the object record's version, 10    the scale is read from 10 on
uint32   object count
objects  × object count
trailer
```

The two words once written up as "always 6" and "always 10" are **versions**,
and the loader branches on them (*read*, `MisLoad.dll`). The clan table
(`0x100015b0`) accepts 1 to 6 and, below 6, builds the alliance matrix from
the clan types instead of reading it. The object reader (`0x10003900`) takes
each field only from the version that introduced it: the clan index from 3,
the logical id from 4, the property table from 6, the instance name from 7,
the start flag from 8, the host building and its vertex from 9, and the scale
from 10 — an older record gets a scale of 1. Every shipped mission is 10
(*measured*, 29 of 29).

**The routes are polygons, not paths** (*read*). `IMission` slot 8
(`MisLoad.dll:0x10001380`) makes each one the system areal map's *tactical
areal* of its id. A unit's behaviour reports which of them it stands in, and a
script asks with function 32, which is how a campaign mission notices the
player walking into an area. The clan record's zones are handed over in the
same call as the clan's *migration* areals, and they are not the ones a script
tests. See [34-progression.md](34-progression.md).

### Clan

```
string   name                         "Player", "Clan I", "Plr", "Natur"
int32    always -1
float32  base x, base y
uint32   clan type: 0 nature, 1 player, 2 enemy, 3 neutral
string   AI script                    MISSIONS\SCRIPTS\scr_pl_1
uint32   zone count
zones    { uint32 kind; float32[3] centre; float32 inner; float32 outer }
string   behaviour tree               MISSIONS\SCRIPTS\scream.trf
uint32   minds: how many bots the clan may field, 2..17 -- see below
uint32   relation count
relations { string clan name; uint32 relation }
```

The word after the behaviour tree is **the clan's mind count** — how many bots
it may have at once, the game's "Available CPUs" (`iron3d.dll` string 3067),
*read*. `MisLoad.dll:0x10003ca0` loads it into the clan record, `iron3d.dll`
copies it (`0x10038ebd`) and fills the clan SuperAI's mind list with that many
free slots (`0x10039266`). A factory will not start a bot without a free slot
("No Free mind... cannot start constructing", `Behavior.dll:0x10029ba0`), a bot
under construction already holds one, and a bot's slot is given back when it
is destroyed or captured. When the player has none left the game says
`VOICE_NO_CPU`. Nothing found raises it during a mission. See
[23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured).

*Measured:* it runs 2..17 over the 101 shipped clans, 5 on 53 of them. No clan
is placed with more `UNITS\UNITS` robots than its minds — two sit exactly at
the limit, `CAMPAIGN.00/Mission.01`'s `Trgt` at 5 and
`CAMPAIGN.02/Mission.04`'s `Enemy` at 17 — while counting every owned object
instead, 18 clans exceed it. The two top clans of `Multi.01` to `Multi.04` get
8 each.

This word was open until the construction code was read. The earlier note here
measured what it is *not* — not the clan's type word, zone count or ally count, and
equal to the clan count only by the chance of the common value 5 — and guessed
a clan strength or AI level. It is a strength of a kind: a cap on the army.

The relation table is an **alliance matrix**: every clan lists every clan,
with 1 towards itself and its allies and 0 towards its enemies. Zones appear
only on campaign missions, with radius pairs like 20/40 and 10/30.

### Object

```
uint32   kind        0 building, 1 unit, 2 vegetation, 3 rock
uint32   always 0x80000002
string   path
uint32   clan index                   v3; the owner's 0-based clan, = ClanID
int32    logical id                   v4; also repeated as the LogicalID property
float32  x, y, z
float32  turns about x, y, z          radians; x and y are 0 throughout
float32  scale x, y, z                v10; uniform throughout, 1 on 646 of 864
string   instance name                v7; empty for scenery
uint32   start flag                   v8; a building's, see below
int32    host building                v9; a logical id, or -1
int32    hall-way vertex              v9; in the host, or -1
uint32   property table word          v6; 1, read and discarded
uint32   property count
properties × count
```

The six floats after the position are one rotation triple
(`MisLoad.dll:0x10003900`), and the placement matrix is built as Rz·Ry·Rx
(`0x10001d80`), so the two zero words are rotation x and y stored as 0.0.

`path` resolves two different ways depending on `kind`. Buildings and units
name a `.dat` definition file under `UNITS/`; vegetation and rock name a
`STAT` member of `objects.rlb`. **All 864 placed objects across all 29
missions resolve** — 463 files on disk, 401 archive members.

### What the object's words do — *read*, and *measured*

`iron3d.dll:0x100a3ea0` places a mission: it sorts the records by kind, creates
the buildings first and files each under its logical id, then the units, then
the scenery. `MisLoad.dll` hands it each record through `IMission` slot 10
(`0x10001440`).

- **The word after the path is the owning clan.** `iron3d.dll` indexes its clan
  records with it (`0x100a4097`), and it equals `ClanID` on **463 of 463**
  owned objects. Scenery carries leftovers (−1, 0, 3…) that nothing reads.
- **The two "always 0" words and `rotation` are one vector of three turns.**
  `MisLoad.dll:0x10001d80` builds the placement as Rz(z)·Ry(y)·Rx(x) with the
  position as its translation. The turns about x and y are 0 on all 864.
- **Only scenery is scaled.** Kinds 2 and 3 go to `World3D.dll`'s
  `AddNewObjectToGame` with the scale, which hands it to the model's interface
  0x18 slot 15 (`0x10008428`): `AniMesh.dll:0x10014770` multiplies the model's
  own x, y and z by the three and its bounding radius by the largest. Buildings
  and units go to `ArealMap.dll`'s `CreateObjectFromScheme`, which takes no
  scale. *Measured*: x = y = z on 864 of 864; 134 trees and 82 rocks are not 1,
  and two animals (`tushka.dat`, 1.5 and 1.25) carry a scale nothing applies.
  See [the scale](#the-scale).
- **A unit can start inside a building.** The host word is a building's logical
  id and the vertex word a vertex of that building's hall-way graph
  ([07-objects.md](07-objects.md)); `CreateObjectFromScheme` logs *"Placing
  Robot inside Building … into Vertex"* and takes the vertex's position
  (`ArealMap.dll:0x10015290`). While the vertex is set, `IMission` slot 10 hands
  out a zero position and zero turns (`0x100014b4`). *Measured*: 13 objects set
  the pair, all units, each naming a building placed in the same mission and a
  vertex inside its graph — 12 heroes in their own clan's bunker and one
  walker in another clan's store; the other 851 set both to −1.
- **The start flag is a building's, and it is the building's object state.**
  `iron3d.dll:0x10033cb0` turns it into bit 0 of `CreateObjectFromScheme`'s
  `dwCreateFlag` (bit 2 says a logical id follows), and that bit sets 2 on the
  building through `IBuilding` slot 12 (`Terrain.dll:0x10056cb0`,
  `CBuilding +0xb8`, 1 by default). A unit's and scenery's flag is not passed
  on. *Measured*: it marks **exactly one half of each of the nine bridge
  pairs** — always the half with the later logical id and the angle π further
  on — and 4 other buildings (two mines, a plant, a generator); 6 units and 4
  trees set it to no effect. What the field is called, and that it changes
  nothing, is [below](#the-start-flag-changes-nothing--read).
- **The property table's leading word** is read into a local and dropped
  (`0x10003ab0`); it is 1 on all 864.

### The start flag changes nothing — *read*

`IBuilding`'s vtable is `Terrain.dll:0x1009b52c`, 22 slots. Slot 12
(`0x10056cb0`, 2 arguments) writes the interface's `+0xac` and slot 13
(`0x10056cd0`, 1 argument) reads it back. The interface sits at
`CBuilding +0xc`, so `+0xac` is `CBuilding +0xb8` — which is the offset this
page already named.

**Slot 13 has no caller.** Enumerating every `QueryInterface(0x17)` site in the
install — `mov edx, 0x17` then `call [vtable]` with an out-pointer, **23 sites**
across `AniMesh`, `ArealMap`, `Behavior`, `Control`, `Terrain` and `iron3d` —
and every vtable offset then called on the answer, rejecting any call whose
argument run does not match the slot's own `ret n`, gives: slot 3 (2 arguments)
one caller, slot 12 (2) two, slot 14 (2) one, slot 15 (3) twelve, slot 16 (2)
six, and **slot 13 (1) none**. The control is the writer of that same field:
`ArealMap.dll:0x10015a0d` tests `dwCreateFlag` bit 0, asks for `0x17` and calls
slot 12 with the literal `2` — the path described above — and the pass finds
it. The arity check is what makes the pass trustworthy: without it the pass
also reported two hits whose receiver register had its definition killed by an
intervening call.

**But the field itself is read, and it has a name.** Every `+0xb8` memory
operand in `Terrain.dll`, mapped to its enclosing function and named by the
assertion string that function carries, gives five `CBuilding` sites and no
more: `0x100569b4`, `CBuilding::CBuilding()`, writes **1**; `0x10056cb9`, slot
12, writes its argument; `0x10056cd6`, slot 13, reads — no caller;
`0x10057868` and `0x1005797e`, both in `CBuilding::SendMsg()`, each
`cmp [this+0xb8], 0`; and `0x10058993`, `CBuilding::SetObjectState()`, writes
it from the **last** element of the object's state array, asserting that
element's type code is 4. `SetObjectState` names the field: **`CBuilding
+0xb8` is the building's object state**, 1 by construction and 2 when the
mission's flag is set.

And `SendMsg` only ever tests it against zero — in two cases of a four-way
switch on the per-item state at `[this+0x64] + i*0x6c + 0x50`, under message 1
— while a placed building is never zero. **So the mission's flag reaches
exactly one reader in the engine, and that reader cannot tell 1 from 2.** There
is no `CBuilding::GetObjectState`; the ten `CBuilding::` names in the binary do
not include one. What the mission file marks on nine bridge halves and four
other buildings, the game does not act on.

### The rotation's sense

The turn about z turns the object about the map's up axis, and the bridges settle
which way. A bridge is placed as **two halves back to back**: nine pairs
across seven missions, each pair's angles exactly π apart to four decimals —
`+0.0370` and `+3.1786` on Tut_1, `-0.0789` and `+3.0627` on KM_4. Their
roadways have to meet.

Taking the angle as it stands joins the two ends to within a unit on **all
nine pairs** — to 0.00 on eight of them. Negating it joins **none**, and
leaves gaps of 4.9 to 282.8 units. So a renderer that maps game
`(x, y, z)` to a Y-up `(x, z, -y)` applies the angle unchanged about its
Y: the axis swap carries the sense across as it is.

It is worth stating because nothing else in the shipped data tests it.
Buildings and units placed at an arbitrary heading simply face somewhere, and
a wrong sense looks like a design decision. Two objects that must interlock
are the only witness.

The loader now says the same (*read*): the placement matrix's turn about z is
`[[cos, −sin], [sin, cos]]` over x and y (`MisLoad.dll:0x10001d80`), a positive
angle taking +x towards +y.

### The scale

The three scale floats are **always equal**. **218 of 864** placements carry a
value other than 1, from 0.2 to 21 — shrunk as well as enlarged:

- 134 of 303 vegetation, 0.3 to 11;
- 82 of 98 rock, 0.2 to 21;
- two units, the `tushka` animals on CAMPAIGN.02/Mission.03, at 1.5 and 1.25.

No building is scaled. Mission 01 has 17: `s_tree_04` at 2, 2.5 and 3, and six
stones at 8 to 20.

**Only vegetation and rock are drawn at their scale.** MisLoad reads the scale
from record version 10 on (`MisLoad.dll:0x100039bf`). The placement matrix it
hands out is rotation and translation alone (`0x10001d80`), and the scale
travels beside the matrix (`GetObject`, `0x10001440`, out +0x44).

iron3d gives vegetation and rock to World3D's `AddNewObjectToGame` with the
scale in a parameter block (`iron3d.dll:0x100a4334`). A scale other than
(1, 1, 1) goes to the object's mesh interface `SetScale`
(`World3D.dll:0x100083f4`, `AniMesh.dll:0x10014770`). Units and buildings are
built from their `.dat` with the matrix alone (`iron3d.dll:0x10033cdb`), so
the two animals' scale is never applied.

`SetScale` scales **the object**, not only its drawing:

- every node's matrices are recomposed with the scale;
- the pose walk multiplies each node's translation column by it (`0x10008c8f`
  on the root, `0x10008f8a` on the rest), though the root's is then cleared
  (`0x10008d88`, [07-objects.md](07-objects.md#how-the-engine-plays-it--read));
- both bounding boxes, the bounding sphere and the cylinder are scaled;
- the node area and volume getters scale by two and three of the factors.

The mesh hit test works in each node's frame
([26-damage.md](26-damage.md)), so a round meets the scaled tree.

Measured against the terrain, taking the lowest level-0 vertex after poses:

| | placements | more than 0.25 above the ground |
|---|---|---|
| scaled trees, at their scale | 134 | **2** |
| the same, at scale 1 | 134 | 63 |
| scaled stones, at their scale | 82 | **2** |
| the same, at scale 1 | 82 | 44 |
| unscaled trees (control) | 169 | 10 |
| unscaled stones (control) | 16 | 2 |

Drawn at scale 1, nearly half the scaled scenery would hang in the air. At its
scale it is dug in, like the control.

**A scaled tree is also tougher** — *read*, and *measured*. Vegetation and rock
do carry node life: their `STAT` records name a `.ndp` and a `.ctl` (81 of 81),
`iron3d.dll` gives them to `AddNewObjectToGame` as **type 10**
(`0x100a4331`), and the agent loader hands every agent, whatever its tag, a
control system and its `ILifeSystem` (`AniMesh.dll:0x100032e7`, `0x1000330d`).
The control system re-reads its mesh's scale on every tick and rescales every
node's life and maximum by **the three factors multiplied**
(`Control.dll:0x10007ac6` → `0x10009ee0`), so a tree at scale 3 has 27 times
its table's hit points and Mission 02's `s_stone_10` at 21 holds 4,630,500,000.
See [26-damage.md](26-damage.md#vegetation-and-rock-carry-node-life--read-and-measured).

**Unknown:** whether anything calls `SetScale` on a unit while the game runs.

### Property

```
uint32   type        0 float32, 1 int32
uint32   value       the per-instance value
uint32   minimum
uint32   maximum     -1 on an int means none
string   name
```

The two words after the value are **its bounds**. Across every property
instance in the shipped missions but one family, the value sits between them,
and the bounds say what the property is: `0..1` for `Invulnerability`,
`Life state` and `MaxSpeedPercent`, `0..INT_MAX` for `FreeBotNum` and
`FreeTechnoNum`, `2..1000` for the construction and research times, `0..1e6`
for `MaximumOre`. Three conventions ride on it. `ClanID`'s maximum is `-1`,
meaning none. `LogicalID`, `Type` and `ChargeRadius` set both bounds to the
value itself, which locks it. And `CurrentOre`'s maximum is **the same object's `MaximumOre`** on
all 463 — the one bound that moves with the object — while its minimum was
never initialised: denormal floats like `6.45e-39` wherever it is not zero,
which is why it alone looked like it broke the rule.

An earlier note said both words were constant per property name. They are not
on four: `LogicalID` and `Type` are locked to each instance's own value, `CurrentOre`'s tracks
`MaximumOre`, and `NOT USED` has two pairs.

Each object carries its own property *names*, so the gameplay data model comes
straight out of the mission file: `Invulnerability`, `Life state`, `LogicalID`,
`ClanID`, `Type`, `MaxSpeedPercent`, `MaximumOre`, `CurrentOre`,
`ChargeRadius`, `FreeBotNum`, `FreeTechnoNum`, `FreeConstructionTime`,
`FreeResearchTime`, and six slots literally named `NOT USED`. The four
`Free…` properties are a factory's free bots, a research centre's free
technologies, and the research time — see
[23-economy.md](23-economy.md#the-four-grants-a-mission-gives-a-building--read-and-measured).

**`ClanID` is a 0-based index into the clan list** — not the clan's type word,
which this page once read as a 1-based index
([27-ownership.md](27-ownership.md#the-clan-word-is-a-type--measured-and-read)). All 463 owned objects have a ClanID in range, and on
skirmish and multiplayer maps 123 of 125 objects sit nearest the base of the
clan they are assigned to.

### Trailer

```
string   map path                     DATA\MAPS\SC_3\land
uint32   a word                       1 on the multiplayer maps and Single.01
string   description                  see the warning below
uint32   the lode table's word, 1
uint32   lode count
lodes    { float32[3] position; uint32 found; uint32 type; float32 amount; uint32 }
```

The map path is how a mission chooses its terrain — `mission.cfg` only names
the *minimap* image.

**The trailer's records are mineral lodes**, not a viewpoint per clan (*read*).
`MisLoad.dll` reads the description from object version 2 and the lodes from
5 (`0x10001b10`), 28 bytes each (`0x10004040`), and hands out 24 of them
through `IMission` slots 12 and 13. `iron3d.dll:0x10081880` builds one 24-byte
record per lode — x, y, a zero z, the found flag, the type `0x10001000` and
the amount — and gives the list to `ArealMap.dll`'s
`SetMineralLode` (`0x10021db0`); `0x10081c10` then draws `effects.rlb`'s
`env_mineral` on the ground under each. `0x10001000` is the type the builders'
minerals search looks for, and the search skips a lode whose found flag is set
and sets it when a unit gets within 10 m ([31-packages.md](31-packages.md)).

*Measured*: 28 lodes on 13 of the 29 missions; the count equals the clan count
on only 2. 17 start found. The type word is `0x10001000` on 25 and 0 on 3, but
`iron3d.dll` writes its own `0x10001000` whatever the file says; the file's z
is dropped too. The amount runs 1e4 to 1e20 — what reads it off the lode
(`+0x14`) is not found. The fourth word (0, or 100.0 as a float on the three
with type 0) never leaves `MisLoad.dll`.

**The word after the map path is read by nothing** — *read*, with a control.
It is `IMission` slot 11's: a 16-slot vtable at `MisLoad.dll:0x1000e0e8`
(slot 16 null), slot 11 at `0x10001510`, `__stdcall`, `ret 4`, returning the
object's `+0x28`. `MisLoad.dll` exports `CreateMissionData` and, over the
import tables of all fifteen DLLs and the executable, **`iron3d.dll` is its
only importer**. The three calls to the import thunk (`0x100cd06c`) are at
`0x100a2041`, `0x100a2e3b` and `0x100a37e1`, inside `0x100a1f90`,
`0x100a2bd0` and `0x100a36a0`, which hand the pointer on only to the placement
(`0x100a3ea0`) and the lode setup (`0x10081880`). Across all five frames the
pointer is **never stored to a global or an object field** — every store is
`mov [esp+k], reg` into the function's own frame — so a call on it has to sit
inside one of the five, and those five hold exactly two indirect calls at
`+0x2c`, `0x100a257d` and `0x100a3aa2`, both pushing three arguments at the
object in `[game + 0xae8]` rather than one at the mission. **Slot 11 is called
nowhere.** The control: the same one-argument interface-call scan
(`push R; call [V+disp]` where `V` came from `mov V,[R]`), run over every
module at `+0x30` and `+0x34`, returns `iron3d.dll:0x10081893` and
`0x1008192c` — the known slot 12 and slot 13 calls that feed `SetMineralLode`.
A slot-11 call would have had exactly that shape. For scale, the same scan at
`+0x2c` finds 152 calls across the install.

What the word *means* stays open, and deliberately so. **It is 1 on seven of
the 29 missions — `Multi.01`…`Multi.06` and `Single.01` — and 0 on the other
22** (*measured*), `Autodemo.00` and `Single.02` included. Two readings track
it and **both break once**. The six `Multi` maps are the only missions with
more than one clan of type `CLAN_PLAYER` (2, 2, 2, 2, 3 and 4), and every
mission with the word clear has one player clan or none — but `Single.01` has
one and carries the word, so that reading is right on 28 of 29. No campaign
mission carries it and no `Multi` lacks it — but `Single.02` is outside the
campaign too and carries 0, so that reading is right on 27 of 29. Routes do
not separate them either. With one break in each, no meaning is published
here. The one thing that could still settle it is whether `Single.01` appears
in the game's multiplayer map list, which lives in `iron3d.dll`'s menu code.

The lode table's word is passed to the lode reader, which ignores it. A save
keeps its own copy of the lodes, 24 bytes each, read back by `0x10081750`.

**The description field is damaged in the shipped data.** Its length word is
the *capacity* of a fixed-size buffer, and whatever followed the real text in
memory was written out with it: sometimes MSVC's `0xCD` debug fill, sometimes
fragments of other strings. `CAMPAIGN.01/Mission.02` stores
`'Cature the jeepnStrateg.tIL\Missionsr\Ms'` where the text is
`Cature the jeep`. There is no in-band way to find the real end, so use the
mission directory's plain-text `descr` file instead; `Mission.title` does.

## mission.cfg — plain text, and self-explaining

```
object	minimap
 library	= "ui\minimap.lib"
 minimap	= "sc3.tex"
end

object primary_objectives
  objective1  = "1. Destroy all enemy vehicles"
  objective2  = "2. Destroy or capture all enemy buildings"
end
```

CRLF, tab-separated, `#` comments. One shipped comment is a gift:
*"Names of properties are unimportant, but object names are."*

Some of its objects are not free-form at all. An object whose `desc` is
`"resource"` is a **resource descriptor** — a library, a type, and a list of
names bound to its members — and it is the same object in the same syntax that
`ui/*.cfg` and `DATA/TextRes.cfg` use. A mission carries between three and
five of them: its minimap, its ambient theme and variations, and, for a
campaign mission, its briefing and tutorial voices.
→ [20-resources.md](20-resources.md)

## Mistips.mis — briefing markup

```
<<interleave=150>>
<<color=255,0,0>>
Mission name:
<<br>>
```

`interleave` is a typewriter delay, `color` is RGB, `br` a newline.

## How the parse was validated

- **It closes.** All 29 missions consume to the last byte with nothing left
  over. A wrong field width anywhere derails the whole file, so this is the
  main structural proof.
- **The declared object count is right.** The word before the object list
  equals the number of objects the parse then finds, on every mission.
- **The loader agrees field by field.** `MisLoad.dll`'s readers take the same
  fields in the same order, and name what the parse had left as padding.
- **Everything it points at exists**: 29/29 map paths resolve to a real
  `DATA/MAPS` entry, 864/864 object references resolve.
- **Placement agrees with the terrain.** All 864 objects fall inside their
  map's XY extent, and buildings sit at a **median of +0.026 units** above the
  terrain surface sampled from `Land.msh`. Bridges float (median +12.8) and
  bunkers dig in (median −3.0), which is what those things should do.

## SCRIPTS

- **`.fml`** — plain text formula sets: `FUNCTION( , fTemp + 0.001, )`.
- **`.scr`** — the mission AI, now [read](15-behaviour.md): named handlers over
  flat node lists, all 58 files end to end.
- **`.trf`** — NRes archives of 12 streams tagged `TRF0`–`TRFB`, all named
  `ResTree`, nearly all exactly 77448 bytes.

Loaded by `ai.dll`. The graphs are [parsed](15-behaviour.md) and their
operands resolve by name against `varset.var`; reproducing what the nodes *do*
is the hard part of the project and is untouched.
