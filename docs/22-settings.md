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
something. The `.scr` scripts number nothing 0–13 that it could index, and the
14 AI problems are only alphabetically ordered by this project, not by the
engine — so it is **unknown**, not a problem id.

## `Iron_3D.ini` and `dispatcher.ini` — the player's, not the game's

`Iron_3D.ini` is four sections and 33 keys: `[CS]` the display and input,
`[MULTIPLAYER]` a login, `[TEMP]` two ranges the interface normalises against
(`OFFENCE_MIN`/`MAX`, `DEFENCE_MIN`/`MAX`), and `[LEVEL_RATIO]` three
difficulty multipliers — 0.5, 0.7, 1.0 — that `GAME_LEVEL` picks between:
0 `EASY`, 1 `MEDIUM`, anything else `HARD` (*read*, `iron3d.dll:0x10076010`).
The ratio scales enemy warriors' hit points, shields and gun damage
([26-damage.md](26-damage.md#the-difficulty-ratio--read-and-measured)).

`dispatcher.ini` is one `[COMPLETE]` section with one key per mission
finished, value 1. **The key is the mission's own directory path**, every
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

- **What the loaders do.** Eight entry points are named; only their addresses
  are established here.
- **What a shader component is.** `CID_SHADER` allocates and is never followed
  further.
- **Whether the engine accepts more component ids than eight.** The registry
  is a file, so presumably yes; nothing was tested.
- **`DefaultOrderPhase`**, as above.

Everything above except the export addresses is re-derived by
`uv run openparkan verify`; those come from `analysis/registry.py`.
