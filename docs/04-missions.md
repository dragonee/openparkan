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

29 mission directories in total. A mission directory holds:

| File | Format | Contents |
|---|---|---|
| `mission.cfg` | text | resources, objectives, minimap — fully readable |
| `data.tma` | binary | clans, object placement, per-object properties |
| `descr` | text | one-line description shown in the menu |
| `sky.wea` | text | skybox texture name table |
| `sky.ske` | binary | skybox / celestial body definition |
| `Mistips.mis` | markup | briefing text with `<<tag>>` markup |
| `briefing.cfg`, `messages.cfg` | text | campaign missions only |

## mission.cfg — plain text, and self-explaining

```
object	ambient_music_loop
 desc		= "resource"
 library	= "sounds.lib"
 libtype	= "multi"
 type		= 5
 THEME		= "atm_c1_lp.wav"
end

object	minimap
 library	= "ui\minimap.lib"
 minimap	= "sc3.tex"
end

object primary_objectives
  objective1  = "1. Destroy all enemy vehicles"
  objective2  = "2. Destroy or capture all enemy buildings"
end
```

CRLF line endings, tab-separated, `#` comments. One of the shipped comments is
a gift: *"Names of properties are unimportant, but object names are."*

## Mistips.mis — briefing markup

```
<<interleave=150>>
<<color=255,255,255>>
<<br>>
<<color=255,0,0>>
Mission name:
```

`interleave` is a typewriter-effect delay; `color` is RGB; `br` is a newline.

## data.tma — partially understood

Binary. Strings are **length-prefixed**: a `uint32` byte count followed by the
bytes, with no NUL terminator. `Single.01/data.tma` contains 521 such strings.

What is legible so far:

- **Clans come first.** `Single.01` opens with `Player` and `Enemy`;
  `Multi.01` with `Clan I`, `Clan II`, `Clan III`; a campaign mission with
  `Plr`, `Trgt`, `Enm`. Each is followed by `0xFFFFFFFF`, a pair of float32
  that read as map coordinates (630.1, 751.9 on Single.01 — inside that map's
  0..2490 extent), and paths to its AI scripts under `MISSIONS\SCRIPTS\`.
- **The map link lives here, not in `mission.cfg`.** Every `data.tma` contains
  a string like `DATA\MAPS\SC_3\land`. That is how a mission selects its
  terrain. `mission.cfg` only names the *minimap* image.
- **Object placement** references unit definitions by path:
  `UNITS\BUILDS\BUNKER\mbunk01.dat`, `UNITS\BUILDS\GENER\gener01.dat`.
- **The property schema is embedded.** Each object is followed by its own
  property name table: `Invulnerability`, `Life state`, `LogicalID`, `ClanID`,
  `Type`, `MaxSpeedPercent`, `MaximumOre`, `CurrentOre`, `ChargeRadius`,
  `FreeBotNum`, `FreeTechnoNum`, `FreeConstructionTime`, `FreeResearchTime`,
  and several slots literally named `NOT USED`.

That last point matters more than it looks: the missions carry their own
schema, so recovering the gameplay data model does not require reversing
`iron3d.dll` first.

**Not yet resolved:** the 16-byte header varies between missions
(`Single.01` starts `01 00 00 00 | 00 00 00 00 | 06 00 00 00 | 02 00 00 00`,
a campaign mission starts `01 | 05 | 01 | 08`), and the record framing between
strings has not been mapped. This is the next thing to crack — see the roadmap
in [00-feasibility.md](00-feasibility.md).

## SCRIPTS — behaviour graphs

- **`.fml`** — plain text formula sets:
  `FUNCTION( , fTemp + 0.001, )`, `FUNCTION( , 233,  )`. 58 files.
- **`.scr`** — compiled node graphs. Open with a node name such as
  `PBM_N_OPTIMAL_TRANSPORT_Start`, then dense arrays of int32 slots where
  `0xFFFFFFFF` is a null link. 58 files.
- **`.trf`** — NRes archives holding 12 streams tagged `TRF0`–`TRFB`, each
  named `ResTree`. 29 files, nearly all exactly 77448 bytes, which suggests a
  fixed-capacity table rather than a packed one.

These are interpreted by `Behavior.dll` (357 KB of code) and `ai.dll` (207 KB).
Parsing the graph structure is tractable; reproducing what the nodes *do* is
the hard part of the whole project.
