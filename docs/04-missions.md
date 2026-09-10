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
| `data.tma` | binary | clans, object placement, routes — **fully parsed** |
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
uint32   always 6
uint32   clan count
clans    × clan count
uint32   always 10
uint32   object count
objects  × object count
trailer
```

### Clan

```
string   name                         "Player", "Clan I", "Plr", "Natur"
int32    always -1
float32  base x, base y
uint32   clan index, 1-based
string   AI script                    MISSIONS\SCRIPTS\scr_pl_1
uint32   zone count
zones    { uint32 kind; float32[3] centre; float32 inner; float32 outer }
string   behaviour tree               MISSIONS\SCRIPTS\scream.trf
uint32   varies
uint32   relation count
relations { string clan name; uint32 relation }
```

The relation table is an **alliance matrix**: every clan lists every clan,
with 1 towards itself and its allies and 0 towards its enemies. Zones appear
only on campaign missions, with radius pairs like 20/40 and 10/30.

### Object

```
uint32   kind        0 building, 1 unit, 2 vegetation, 3 rock
uint32   always 0x80000002
string   path
uint32   varies
int32    logical id                   also repeated as the LogicalID property
float32  x, y, z
uint32   two words, always 0
float32  rotation, radians
float32  scale x, y, z                1,1,1 throughout the shipped data
string   instance name                empty for scenery
uint32   four words: 0, -1, -1, 1
uint32   property count
properties × count
```

`path` resolves two different ways depending on `kind`. Buildings and units
name a `.dat` definition file under `UNITS/`; vegetation and rock name a
`STAT` member of `objects.rlb`. **All 864 placed objects across all 29
missions resolve** — 463 files on disk, 401 archive members.

### The rotation's sense

`rotation` turns the object about the map's up axis, and the bridges settle
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

### Property

```
uint32   type        0 float32, 1 int32
uint32   value       the per-instance value
uint32   two further words, roles not established
string   name
```

Each object carries its own property *names*, so the gameplay data model comes
straight out of the mission file: `Invulnerability`, `Life state`, `LogicalID`,
`ClanID`, `Type`, `MaxSpeedPercent`, `MaximumOre`, `CurrentOre`,
`ChargeRadius`, `FreeBotNum`, `FreeTechnoNum`, `FreeConstructionTime`,
`FreeResearchTime`, and six slots literally named `NOT USED`.

**`ClanID` is a 0-based index into the clan list** — not the clan's own `index`
field, which is 1-based. All 463 owned objects have a ClanID in range, and on
skirmish and multiplayer maps 123 of 125 objects sit nearest the base of the
clan they are assigned to.

### Trailer

```
string   map path                     DATA\MAPS\SC_3\land
uint32   varies
string   description                  see the warning below
uint32   varies
uint32   viewpoint count
viewpoints { float32[3] position; uint32 × 4 }
```

The map path is how a mission chooses its terrain — `mission.cfg` only names
the *minimap* image. There is one viewpoint per clan.

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
- **Everything it points at exists**: 29/29 map paths resolve to a real
  `DATA/MAPS` entry, 864/864 object references resolve.
- **Placement agrees with the terrain.** All 864 objects fall inside their
  map's XY extent, and buildings sit at a **median of +0.026 units** above the
  terrain surface sampled from `Land.msh`. Bridges float (median +12.8) and
  bunkers dig in (median −3.0), which is what those things should do.

## SCRIPTS — still opaque

- **`.fml`** — plain text formula sets: `FUNCTION( , fTemp + 0.001, )`.
- **`.scr`** — compiled node graphs, opening with a name such as
  `PBM_N_OPTIMAL_TRANSPORT_Start` then int32 slots with `0xFFFFFFFF` as null.
- **`.trf`** — NRes archives of 12 streams tagged `TRF0`–`TRFB`, all named
  `ResTree`, nearly all exactly 77448 bytes.

Interpreted by `Behavior.dll` and `ai.dll`. Parsing the graphs is tractable;
reproducing what the nodes do is the hard part of the project.
