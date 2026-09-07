# Can Parkan: Iron Strategy's engine be reimplemented?

**Yes.** As engine reimplementations go this is a friendly target — friendlier
than the OpenRA or OpenMW starting positions were. The data formats are simple,
several are self-describing, and one of them ships its own schema. What is
*not* cheap is the gameplay simulation, which is where the actual decade of
work lives.

This document is the summary. The detail is in the sibling files, and every
factual claim below is re-derived from the shipped files by
`uv run openparkan verify`.

## What makes it tractable

**One container format, and it is trivial.** All 116 archives — `.rlb`, `.lib`,
`.dlb`, `.res`, `.trf`, and the per-map files — are `NRes`: a 16-byte header
and a 64-byte-per-entry directory at the end of the file. It took about twenty
lines of Python to enumerate all 6697 members. See
[01-nres.md](01-nres.md).

**The texture format documents itself.** `Texm`'s pixel-format field literally
spells the channel widths as a decimal number: `8888`, `888`, `565`, `4444`, or
`0` for palettised. No guessing was needed; all 393 textures decode. See
[02-texm.md](02-texm.md).

**Much of the content needs no format work at all.** Sounds are plain
RIFF/WAVE. Music is already Ogg Vorbis, and the game ships
`libogg`/`libvorbis`. `mission.cfg`, `descr`, `*.wea` and `*.fml` are readable
text. One shipped comment even explains its own conventions: *"Names of
properties are unimportant, but object names are."*

**The missions carry their own schema.** `data.tma` is binary, but each object
is followed by its property *names* in plain text — `LogicalID`, `ClanID`,
`MaxSpeedPercent`, `MaximumOre`, `ChargeRadius`, `FreeConstructionTime`. The
gameplay data model came straight out of the mission files, with no
disassembly. See [04-missions.md](04-missions.md).

**The DLL split is free architecture documentation.** `Terrain`, `World3D`,
`ArealMap`, `Behavior`, `ai`, `Control`, `Effect`, `Net`, `MisLoad`, `Ngi32` —
that list maps almost one-to-one onto the modules a new engine needs. See
[05-engine.md](05-engine.md).

**There are no shaders to reproduce.** `Ngi32.dll` imports only `DDRAW` and
`DSOUND`; `World3D.dll` imports `DINPUT`. This is fixed-function DirectX 6/7.
The most exotic feature in `Iron_3D.ini` is `EMBM` — environment-mapped bump
mapping. All of it maps cleanly onto modern Metal, Vulkan or WebGPU.

**Terrain is already solved.** All 33 maps parse: geometry, normals, UVs,
two-layer materials, water planes and face adjacency. The parse is confirmed
against the game's own pre-rendered minimaps at Pearson r = +0.88 to +0.97.
See [03-terrain.md](03-terrain.md).

**One geometry format, used twice.** Object meshes turn out to be NRes
archives nested inside archive members, using the same stream convention and
the same `int8`/127 normals and 8.8 fixed-point UVs as the terrain. Cracking
the terrain therefore cracked most of the object format too. See
[07-objects.md](07-objects.md).

**The parts catalogue is in the data.** A unit is an assembly of components,
each naming an `objects.rlb` record and carrying the display name the game's
UI shows — "Large Track Chs (L-42t)", "ARMOUR LA.Mk3 (ARM 3)". All 458
assemblies and 5708 components read without touching the executable.

## What is genuinely hard

**The simulation, not the parsing.** Damage model, economy, capture rules,
pathfinding, and the FPS/RTS hybrid where the player drops into a mech. None of
that is in the data files. It is 3.59 MB of x86 code across the DLLs, with
`iron3d.dll` (929 KB) and `Terrain.dll` (623 KB) as the bulk.

**Behaviour graphs.** `.scr` files are compiled node graphs — they open with a
node name like `PBM_N_OPTIMAL_TRANSPORT_Start` followed by dense int32 index
arrays with `0xFFFFFFFF` as the null link. Parsing the graph structure is a
weekend. Making the nodes *behave* the same is the multi-month part, and it is
interpreted by `Behavior.dll` (357 KB) plus `ai.dll` (207 KB).

**Navigation.** `Land.map` / `ArealMap` is unresolved and is what pathfinding
and territory almost certainly ride on. See
[06-open-questions.md](06-open-questions.md).

**The hybrid partial-replacement trick probably will not help.** The engine is
split into DLLs, which suggests swapping one at a time on Windows. But most
export only 1–13 symbols — a factory returning a COM-style vtable — so this
needs those vtables reversed first, and it is Windows-only, which defeats the
point.

## The honest alternative

If the goal is just to *play it on a Mac*, a reimplementation is the expensive
path. This is a 32-bit DirectDraw title; CrossOver and Wine run those through
their 32-on-64 layers, and it is the fragile end of that spectrum, but it is
days of fiddling against person-years of engineering. Note also that
`Iron_3D.ini` already reads `DISPLAY_WIDTH=1920`, so modern resolutions are not
a reason to rebuild.

A reimplementation earns its keep for the other things: native Apple Silicon,
moddability, a map editor, deterministic multiplayer, and preservation.

## Roadmap

| Step | Scope | Status |
|---|---|---|
| 1 | Asset toolkit: NRes, textures, sounds | **done** |
| 2 | Terrain viewer: render a real map | **done** |
| 3 | Mission loader: place every object from `data.tma` | **done** |
| 4 | Static simulation: camera, selection, minimap, no AI | next |
| 5 | Gameplay: movement, combat, economy, then `.scr` behaviours | the long tail |

Steps 1–3 are a realistic solo side project. Step 5 is where OpenRA-class
projects spend a decade.

`data.tma` is now fully parsed: all 29 missions consume to the last byte,
864 objects are placed, and every reference they make resolves. Buildings land
at a median of +0.000 units above the terrain, which is the check that ties
mission space and terrain space together.

**Step 4 is the next thing**, and it is mostly engineering rather than reverse
engineering: a scene graph, camera, selection and a minimap over data that is
already readable. Object *geometry* now reads too, so a scene can be drawn
from real meshes rather than placeholders.

Every object a mission places — all 864 of them — now resolves to real
geometry, so a scene can be drawn from the game's own meshes.

What still blocks a playable build is `Land.map` (navigation), the per-face
texture assignment on object meshes, and how components bind to a chassis when
a unit is assembled — see [06-open-questions.md](06-open-questions.md).

## Legal position

Reimplementing an engine is clean as long as assets stay in the user's own
copy of the game — the OpenMW and OpenRA model. This repository contains no
game data, and everything in it was derived by observing data files rather
than by disassembling the binaries.

## Prior art

[valentineus/fparkan](https://github.com/valentineus/fparkan) is a Rust
monorepo, 528 commits, actively maintained: lossless NRes and RsLi handling,
validated static MSH geometry, `Texm`/`WEAR`/`MAT0`, `Land.msh` terrain, a
headless runtime, and a basic Vulkan renderer. It explicitly stops short of
engine parity. [AlexKimov/parkan-file-formats](https://github.com/AlexKimov/parkan-file-formats)
is thinner and mostly covers Parkan 1's `.lib`.

Nobody has done the gameplay simulation. That remains the whole project.
