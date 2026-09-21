# The engine's own configuration — five `.ini` files

Five text files sit outside the data, and **each belongs to exactly one
module** — the binary that carries the file's name is the same one that
carries its switches, and no other binary carries either:

| file | read by | what it is |
|---|---|---|
| `Comp.ini` | `World3D.dll` | the component registry |
| `Behavior.ini` | `Behavior.dll` | the AI's logging and debug switches |
| `ArealMap.ini` | `ArealMap.dll` | the navigation mesh's, the same shape |
| `Iron_3D.ini` | `iron3d.dll` | display, input, multiplayer, difficulty |
| `MISSIONS/dispatcher.ini` | `iron3d.dll` | which missions have been completed |

*Measured* — five of five names appear in exactly one of the installation's 21
binaries. Read by `openparkan.settings`, shown by `uv run openparkan settings`.

The last two are **written by the game**, not shipped. What they hold is one
player's settings and progress, so nothing here treats their contents as a
fact about the format; only their shape is checked.

## `Comp.ini` — the Component Address File

This is the interesting one. Eight rows, and the engine's own words for it
come out of `World3D.dll`: the reader at `0x10014790` names itself
`LoadComponentAddr` in an error string, and its other error calls the file the
*Component Address File*.

```
// Current constants for component identification:
//
//     CID_CLASSIC_LANDSCAPE   0
//     ...
//     CID_RESEARCH            7
//
// Format:
//     CID  DLL-Name  Function_Name   Comments...

0  terrain.dll  LoadLandscape      // comments...
3  animesh.dll  LoadAgent          // comments...
7  misload.dll  LoadResearch       // comments...
```

**How it is read** — format facts, out of the disassembly:

- the file is opened `"rt"`, a line at a time into a 200-byte buffer;
- a line **under five characters** is skipped, and so is one starting `//`;
- the rest goes through `sscanf` as **`"%d %s %s"`**, which is why a trailing
  comment needs no delimiter and why anything after the function name is free
  text;
- each row becomes a **16-byte record**: the id, the module handle from
  `LoadLibraryA`, and the entry point from `GetProcAddress`;
- the table's counter advances **only on success**, so a row naming a DLL or a
  function that is not there is silently dropped.

**All eight functions are real exports of the DLL beside them** — reproduce it
with `uv run --group analysis python analysis/registry.py`:

| id | name | | |
|---:|---|---|---|
| 0 | `CID_CLASSIC_LANDSCAPE` | `terrain.dll` | `LoadLandscape` `0x10016640` |
| 1 | `CID_CLASSIC_FORT` | `terrain.dll` | `LoadBuilding` `0x100552a0` |
| 2 | `CID_CLASSIC_CAMERA` | `terrain.dll` | `LoadCamera` `0x10083920` |
| 3 | `CID_CLASSIC_PUPPET` | `animesh.dll` | `LoadAgent` `0x10016a00` |
| 4 | `CID_CLASSIC_STATIC` | `animesh.dll` | `LoadAgent` `0x10016a00` |
| 5 | `CID_CLASSIC_ATMOSPHERIC` | `terrain.dll` | `CreateAtmosphere` `0x10069d60` |
| 6 | `CID_SHADER` | `terrain.dll` | `CreateShader` `0x1004b910` |
| 7 | `CID_RESEARCH` | `misload.dll` | `LoadResearch` `0x100025f0` |

Three DLLs, seven distinct entry points — a puppet and a static thing are the
same loader, which is the engine saying scenery is an agent that does not act.

The `CID_*` names appear in **no binary at all**. The engine dispatches on the
bare number; the names are in the comment header for whoever edits the file.
So this is one of the few places where a vocabulary survives *only* as
documentation, and losing the file would lose the names.

Two warnings about what this is not. These ids are **not** the `CICLS_*`
component classes of [14-controls.md](14-controls.md), which number differently
and mean a part of a machine rather than a kind of scene object. And they are
not the mission object kinds of [04-missions.md](04-missions.md) either: a
mission's kind 1 is a unit where `CID_CLASSIC_FORT` is 1. Three numberings,
three vocabularies.

`CID_SHADER`'s loader is a **singleton factory** — `CreateShader` returns a
kept pointer if it has one, else allocates 0xb8 bytes and constructs an object
holding two arrays of 200 elements, 8 and 12 bytes wide. What it shades is not
established, and it was followed here only far enough to rule it out as the
unfound caller of §2.2's material-track selector.

### What the loaders do, and who asks for them — *read*

**Every loader allocates its object, constructs it with the four arguments it
was given, and returns an interface of it** — a pointer some way into the
object, which `World3D.dll` then asks for interface 6, the game object:

| entry | allocates | constructor | returns |
|---|---:|---|---|
| `LoadLandscape` | 0x7d40 | `CLandscape::CLandscape` (`Terrain.dll:0x100166a0`) | the object |
| `LoadBuilding` | 0xfc | `CBuilding::CBuilding` (`0x10055310`) | `+0x8` |
| `LoadCamera` | 0x1a4 | `CCamera::CCamera` (`0x100839c0`) | `+0x134` |
| `LoadAgent` | 0x7bc | `AniMesh.dll:0x10001000`, then the load `0x10002ff0`; destroyed if that fails | `+0x130` |
| `CreateAtmosphere` | 0x1ac | `CAtmosphere::CAtmosphere` (`0x1006ec30`) | `+0x138` |
| `CreateShader` | 0xb8 | once, then the kept pointer | the object |
| `LoadResearch` | 0x138 | a 0x80-byte `.trf` reader at `+0x130` ([16-research.md](16-research.md)) | the object |

`LoadAgent` refuses a null library or member name. `CBuilding`'s constructor
reads the `FORT` record it is given and loads the record's first slot through
`LoadAgent` — a building is a fortification around an agent
([18-vocabulary.md](18-vocabulary.md)).

**The callers name an object *class*, not a registry id.** `World3D.dll`'s
`0x10007a50` — its error calls it `LoadObjectFromDisk: Illegal Class` — maps a
class from 1 to 11 onto a registry row, and passes the row's loader
`(library, member, 0, player)`:

| class | id | loader | asked for by |
|---:|---:|---|---|
| 1 | 0 | `LoadLandscape` | |
| 2, 4, 9 | 3 | `LoadAgent` | 4 a robot from `CreateObjectFromScheme`; 9 what a controller emits (`Control.dll`) |
| 3 | 1 | `LoadBuilding` | a building from `CreateObjectFromScheme` (`ArealMap.dll:0x10015541`), and `iron3d.dll` |
| 5 | 2 | `LoadCamera` | `iron3d.dll`, `Control.dll` |
| 7 | 5 | `CreateAtmosphere` | the mission's sky (`iron3d.dll:0x100a24b5`) |
| 10 | 4 | `LoadAgent` | mission scenery (`iron3d.dll:0x100a4334`) |
| 11 | 7 | `LoadResearch` | |

Classes 6 and 8 are illegal, and no class reaches id 6: the shader is loaded by
`World3D.dll`'s `LoadComponent` (`0x10014980`), which takes the id itself, and
`Terrain.dll` calls it with 6 from four constructors — `CPrimBuffer`'s
(`0x10032a10`), `CShade`'s (`0x10041370`), `CAtmosphere`'s and `CCamera`'s.
`CreateObjectFromScheme` picks class 3 or 4 by the top bit of the scheme's
type, which a building's carries. The puppet and the static being one loader
is the same thing seen from the other side: scenery is an agent of class 10.

## The two debug files

`Behavior.ini` (14 switches) and `ArealMap.ini` (10) share a **five-switch
logging preamble** — `LogFile`, `SaveLog`, `MaxErrorLevel`, `DefErrorLevel`,
`LookBugMode` — and nothing else. Two modules built on one framework.

The rest is each module's own, and reads as a developer's console:
`LockBehaviour`, `GiveDefaultOrder`, `DefaultOrderPhase`, `DeterminMode`,
`ImmortalHero` and `UseWizard` in `Behavior.dll`; `ShowAreals`,
`Areal_NoZBuffer`, `HallWay_NoZBuffer` and `EdgeUp` for the navigation mesh
([08-arealmap.md](08-arealmap.md)).

Worth noticing which module that is. `Behavior.dll` is the strategy layer;
the thing that actually loads and runs the `.scr` scripts is `ai.dll`
([15-behaviour.md](15-behaviour.md)), and **`ai.dll` has no configuration file
at all**. So the switch named `LockBehaviour` locks the layer that gives
orders, not the interpreter that carries them out.

`DefaultOrderPhase = 10` is the one number here that looks like it indexes
something, and it does not index anything (*read*). `Behavior.dll`'s reader
(`0x10003080`) keeps it at `0x1005d374` (1 by default) beside
`GiveDefaultOrder` at `0x10066bbc`. Its only reader is the behaviour's tick
(`0x10004c40`): when `GiveDefaultOrder` is on, the behaviour is not locked
(`LockBehaviour`, `0x10066bac`, off; two of the behaviour's own flags clear),
and **the behaviour's `+0xa00` equals `DefaultOrderPhase`**, it gives a battle
robot (`Type` `0x1008000`) order 13 and a transport (`0x1002000`,
`ROBOT_TRANSPORT`) order 6 —
`ORDER_ROBOT_RANDOMGO` and `ORDER_ROBOT_TRANSPORT` by
[31-packages.md](31-packages.md)'s numbering (`0x10004c80`). So it is a phase
number compared with a behaviour field, not an index. *Derived*: the only
write to `+0xa00` found is the constructor's 0 (`0x10003a68`; a search over
every base register, and the variable-by-id interface hands out no pointer to
it), so with the shipped 10 — or the default 1 — the default order never
fires even when `GiveDefaultOrder` is switched on.

`Iron_3D.ini` has a key the shipped file does not carry: `[CS]
FULL_RESEARCH_TREE`. `iron3d.dll:0x1008ac50` reads it as non-zero or not; it
silences the warning a research tree with debugging information raises
([16-research.md](16-research.md)), and four part-list builders of the panels
take its inverse as a flag (`0x10048292`, `0x100520ee`, `0x10052ac6`,
`0x10053321`). **It shows every part whether researched or not** (*read*): the
flag is the byte the catalogue collector tests at `0x1008a879`, and clear — the
key set — the part is taken with no state test at all
([38-designs.md](38-designs.md#the-catalogue--read-and-measured)).

## `Iron_3D.ini` and `dispatcher.ini` — the player's, not the game's

`Iron_3D.ini` is four sections and 33 keys: `[CS]` the display and input,
`[MULTIPLAYER]` a login, `[TEMP]` two ranges the interface normalises against
(`OFFENCE_MIN`/`MAX`, `DEFENCE_MIN`/`MAX`), and `[LEVEL_RATIO]` three
difficulty multipliers — 0.5, 0.7, 1.0 — that `GAME_LEVEL` picks between:
0 `EASY`, 1 `MEDIUM`, anything else `HARD` (*read*, `iron3d.dll:0x10076010`).
The ratio scales enemy warriors' hit points, shields and gun damage
([26-damage.md](26-damage.md#the-difficulty-ratio--read-and-measured)).

`dispatcher.ini` is one `[COMPLETE]` section with one key per mission
finished, value 1. The shell writes it when a won mission hands control back
([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
**The key is the mission's own directory path**, every
separator and dot flattened to an underscore, lowercased, trailing separator
kept:

```
MISSIONS\CAMPAIGN\CAMPAIGN.00\Mission.01\
missions_campaign_campaign_00_mission_01_
```

On the install this was written against, 21 of the 29 missions are marked: all
20 campaign missions and `Single.02`. The eight unmarked are the demo, the six
multiplayer maps and `Single.01` — which is what an install that has played
the campaign looks like, not a statement about the format.

## What this does not say

- ~~**What the loaders do.**~~ — **followed one level**, above: what each
  allocates, constructs and returns, and which object class reaches which.
  What the landscape, camera and atmosphere constructors read is theirs to
  document ([03-terrain.md](03-terrain.md), [10-sky.md](10-sky.md)).
- **What a shader component is.** `CID_SHADER` allocates and is never followed
  further; `CPrimBuffer`, `CShade`, `CAtmosphere` and `CCamera` load it.
- **Whether the engine accepts more component ids than eight.** The registry
  is a file, but `LoadObjectFromDisk` knows only the seven ids its classes map
  to, so a new id would be reachable through `LoadComponent` alone.
- ~~**`DefaultOrderPhase`**~~ — **read**, above: a phase compared with a
  behaviour field nothing but the constructor writes. What was meant to
  advance that field is not established.
- ~~**What `FULL_RESEARCH_TREE` does to the part lists.**~~ — **read**: it
  skips the catalogue's researched-and-in-tree test, above.

Everything above except the export addresses is re-derived by
`uv run openparkan verify`; those come from `analysis/registry.py`.
