# openparkan

A reimplementation of the **Parkan: Iron Strategy** (Nikita, 1998) game
engine, built on clean-room readers for the game's own data files.

This repository contains **no game assets**. Point it at your own installation.

Most of it was derived by observing the shipped data files. `Land.map` resisted
that and was recovered by disassembling `ArealMap.dll`; what that means in
practice — format facts yes, code no — is set out in
[09-method.md](docs/09-method.md). No code from the game is reproduced here.

## Status

| | |
|---|---|
| **NRes containers** | done — all 116 archives, 6697 members, round-trip-accurate model |
| **Texm textures** | done — all 393 textures decode, all 5 pixel formats, alpha and the `Page` sub-image table |
| **`Land.msh` terrain** | done — all 33 maps, geometry, normals, both texture layers and their blend, water, the spatial index (cell grid and square table), and the two levels of detail the map is stored at |
| **3D terrain viewer** | done — self-contained HTML, no server |
| **`data.tma` missions** | done — all 29 parse to EOF, 864 objects placed |
| **`objects.rlb`, unit assemblies** | done — 590 records, 458 assemblies, 5708 components |
| **Object meshes** | done — geometry, materials, textures, node poses, LOD, damage states, interior/exterior and collision selection, per-face normals and adjacency, and the animation 157 of them carry |
| **Assembled units and buildings** | done — the `.dat` component tree, and each part mounted at its socket's full pose |
| **Damage tables** | done — `.ndp`, 542 tables, one record per node, 2203 explosions named |
| **Building footprints** | done — `.bas`, 30 ground plans; the one independent check on object placement |
| **Effects** | done — `effects.rlb`, 923 effects and 4737 emitters; 144 `.exp` explosions; and at least 181 of a block's floats the engine reads; how an effect is timed, switched and chosen by surface; lights, bolts, streams, the settings switches and the depth-test flag |
| **Materials** | done — `Material.lib`, all 905 records end to end: a `D3DMATERIAL7` per entry, the animation tracks over them, and every batch reaching a texture |
| **Baked lighting** | done — `lightmap.lib`, 21 buildings, per-batch |
| **Building interiors** | done — the internal nodes are 29460 triangles behind 10725 of shell, and the viewer cuts buildings open; path graph for 29 |
| **Control points** | done — 284 members, 3599 named attachment points |
| **Movement controllers** | done — `.ctl`, all 531 members end to end: the parameter frame, the component records with their part labels, and 1769 resource references that all resolve |
| **Input layer** | done — `ScanCode.dsc`, `Command.dsc`, 12 `.man` binding files and the three `.tbl` control tables; 275 bindings and 116 rows, all resolving, and the engine's own numbers for every key, command, class and state, including all 72 `CMD_` commands out of the two binaries that resolve them |
| **`Land.map` navigation mesh** | done — all 33 maps, 34662 areals, adjacency and grid |
| **Sky** | done — `sky.ske` day cycle, `sky.wea`'s nine texture slots, the engine's twelve-element lens flare and both its intensity gates, the weather events and the sky clock, and where the sun and moon stand (fixed, and in no file), all 29 missions |
| **RsLi archives** | done — encrypted entry table read, deflate and LZSS both unpacked; all 24 `sprites.lib` members out, and `gamefont.rlb`'s font, glyph metrics and `Ipol` palette |
| **Behaviour scripts** | structure done — all 58 `.scr` end to end, 677 handlers and 6065 nodes, and `varset.var`'s 231 declarations, against which every one of the 9239 operands resolves by name; the nine universal event handlers, the 14 AI problems and their `Start`/`Continue` pairing, and which node slot reads and which writes |
| **Parts database** | done — `objects.dlb`, all 395 `DSCR` entries: the classification, the short code and display name, the stat panel rows, and the four costs that name the research tree's floats |
| **Research tree** | done — all 29 `.trf`, 368 items each with names, codes, categories and both edge lists; the prerequisite graph, its research-centre spine, and the per-mission rewiring. The engine's own loader confirms the count/pointer pairing, names the directory field it gates the load on, and shows `TRF1` to be state rather than a label. `TRFB` is the part-to-item mapping: 395 parts onto 368 items, agreeing with `objects.dlb` on 11455 of 11455 entries |
| **Engine configuration** | done — the five `.ini` files, each matched to the one module that reads it; `Comp.ini`'s eight-row component registry, how `World3D.dll` parses it and that all eight entry points are real exports; and the dispatcher's mission-progress keys, which round-trip against the directory tree |
| **Mission briefings** | done — `briefing.cfg` on all 20 campaign missions, 378 waypoints with all 24 fields; camera and target inside their own map and above the terrain on 378 of 378, 165 of 165 voices bound and 275 of 278 subtitles, the three missing being a gap in the shipped finale |
| **Resource bindings** | done — the `desc = "resource"` descriptor, 132 objects across 32 `.cfg` files binding 751 names into seven libraries, all resolving; and `TextRes.dll`'s string table, the game's own 173 lines of dialogue, read without a PE dependency |
| **Save games** | partial — all six saves parse to the last byte as the fixed sequence of sections read off `iron3d.dll`'s writer and loader (`0x100a1590`, `0x100a2bd0`): the header and its difficulty byte, every object's world record, part list and placement, the clans, objectives, mind lists and unit designs. `openparkan saves` lists what a save holds in the game's own words. Most of each owner's own chunk, and the AI state's layout, are the engine's memory and are not decoded |
| **Behaviour scripts, semantics** | read one call deep — `ai.dll`'s executor gives every node kind its behaviour (statement, `if`, label, goto, switch to a handler, return, constant); the function table has 73 slots and every call maps to its handler, each named as far as its code says; the trailer is a formula index into the script's `.fml`. What a handler asks of the engine below it is mostly not followed |
| **Engine research (Phase R)** | done — the hit test, the ground and gravity, the player's controls and eye, animation playback, effects, firing and the sky's fog, each in `docs/` with `verify` checks; [`engine/`](engine/README.md) lists the unknowns the engine must stand in for. No engine code yet |

## Quickstart

Requires [uv](https://docs.astral.sh/uv/). No other dependencies — the toolkit
runs on a bare interpreter by design.

```bash
uv sync
export PARKAN_DIR="/path/to/Parkan Iron Strategy"   # or pass --game

uv run openparkan info                       # summarise the install
uv run openparkan verify                     # re-derive every documented claim
uv run openparkan maps                       # list all 33 maps
uv run openparkan missions                   # list all 29 missions
uv run openparkan mission Single.01          # one mission in detail
uv run openparkan heightmap SC_3 --out sc3.png
uv run openparkan viewer --out terrain.html  # 3D viewer: maps + placed objects
```

The game directory is found automatically if it sits next to this repo, is
named by `$PARKAN_DIR`, or lives in the default Steam location.

### More

```bash
uv run openparkan ls Textures.lib --type Texm
uv run openparkan extract sounds.lib --out /tmp/sounds     # RIFF/WAVE, playable as-is
uv run openparkan textures Textures.lib --out /tmp/tex     # 393 PNGs
uv run openparkan textures ui/minimap.lib --out /tmp/minimaps
uv run openparkan textures sprites.lib --out /tmp/ui --alpha   # the 2D interface
uv run openparkan ls sprites.lib                               # an RsLi archive
uv run openparkan font --out font.png                          # the game font
uv run openparkan mission CAMPAIGN/CAMPAIGN.02/Mission.03 --list
uv run openparkan sky Single.01 --frames                    # the day cycle
uv run openparkan effects aim_exp_L                         # one effect's emitters
uv run openparkan explosions                                # every .exp
uv run openparkan unit battle/w_b_trk1                      # a robot, whole
uv run openparkan viewer SC_3 Tut_1 ILKON --out three-maps.html
```

## Tests

Two different things, and they answer different questions.

```
uv run pytest          # does the code still work?     no game needed
uv run openparkan verify   # is what the docs claim still true?  needs a game
```

`pytest` builds the bytes it reads — an NRes archive, a `.ctl` controller, a
`.tbl` table — so a contributor without a copy of the game can still find out
whether a refactor broke a reader. Nothing under `tests/` is game data, and
the repository ships none.

`verify` is the other half: it re-derives every factual claim in `docs/` from a
real installation. A reader can pass the tests and still be wrong about the
game; only `verify` can catch that.

## Verification

Reverse-engineered format notes are easy to write and easy to get wrong, so
every factual claim in `docs/` is re-derived from the installed files by
`uv run openparkan verify`:

```
PASS  NRes: header size == file size                 116/116 archives
PASS  NRes: every member offset is 8-byte aligned    116/116 archives, 6697 members
PASS  NRes: no member ranges overlap                 0 overlaps
PASS  NRes: inter-member padding is zero-filled      0 non-zero gaps, largest gap 7 bytes (< 8 as expected)
PASS  NRes: the element-count field equals size / stride 231/231 terrain streams across 33 maps
PASS  Texm: the header accounts for every byte of the payload 328/393 end exactly on the last mip level and the other 65 carry a Page table after it
PASS  Texm: every texture decodes to RGBA            393/393, formats {0: 15, 565: 47, 888: 52, 4444: 42, 8888: 237}
PASS  Land.msh: all maps parse                       33 maps
PASS  Land.msh: face indices within the vertex array 33/33 maps
PASS  Land.msh: every vertex is referenced by a face 33/33 maps
PASS  Land.msh: face adjacency is mutual             33/33 maps -- proves fields 7..9 are neighbours
PASS  Land.msh: int8/127 normals are unit length     worst deviation 0.0133 across all maps
PASS  Land.msh: layer-1 UV == world XY / 50          SC_3 worst residual 0.004 texel units
PASS  Land.msh: texture indices resolve through Land1.wea SC_3 layer-1 names in use: ['L00', 'L04', 'WATER', 'WATER_BOT']
PASS  Land.msh: surface bit 0x02 marks exactly the water faces 33/33 maps, 3630 water faces on 11 maps
PASS  Land.msh: face flags 1544 agree with the surface bit 33/33 maps -- an independent second marker
PASS  Land.msh: water is a single flat plane per map 11/11 maps with water
PASS  Terrain matches the game's own minimap art     SC_3 r=+0.883, Tut_1 r=+0.927, ILKON r=+0.965, K1F r=+0.901
PASS  Land.map: payload is consumed exactly          33/33 maps, 34662 areals
PASS  Land.map: every map uses the same cell grid    (128, 128)
PASS  Land.map: areal adjacency is mutual            33/33 maps -- proves edge field 0 is the neighbour
PASS  Land.map: the stored area matches the polygon  34301/34662 areals agree with their shoelace area within 2%
PASS  Land.map: areals span the same extent as the terrain 33/33 maps
PASS  Land.map: areals tile the map without gaps or overlap 33/33 maps -- areas sum to the full square
PASS  Land.map: cell grid indexes real areals        1034301 cell entries across 33 maps
PASS  data.tma: parses exactly to end of file        29/29 missions, 864 objects
PASS  data.tma: the map it names exists              29/29 missions reference a real DATA/MAPS entry
PASS  data.tma: placed objects lie inside the map    864/864 objects within their map's XY extent
PASS  data.tma: buildings sit on the terrain surface median height above ground +0.026 over 167 buildings
PASS  MESH: building exteriors are authored about their centre 20/30 building meshes are near-symmetric about z=0 -- they must be rested on their base, not their origin
PASS  data.tma: every object reference resolves      864/864 -- UNITS/*.dat on disk, scenery as STAT in objects.rlb
PASS  data.tma: ClanID is a 0-based index into the clan list 463/463 object ClanIDs in range
PASS  data.tma: objects belong to the clan whose base they sit at 123/125 (98.4%) on skirmish and multiplayer maps
PASS  objects.rlb: every record parses into slots    590 records, tags ['BTLU', 'BULL', 'EXTO', 'FORT', 'INTO', 'STAT', 'SUNO', 'WPNS']
PASS  objects.rlb: scenery resource slots resolve    405/405 slots across 81 STAT records
PASS  MESH: object meshes parse with consistent streams 68/68 meshes, 41663 triangles, all indices in range
PASS  MESH: int8/127 normals are unit length         worst deviation 0.0130 -- the same encoding as the terrain
PASS  MESH: the format is the same in every archive  435 meshes, 241887 triangles across 10 archives
PASS  CTPT: control points parse as two parallel arrays 284 members, 3599 points, 3599 of them named
PASS  MESH: draw batches tile the index buffer       435/435 meshes -- index counts sum to 3 x triangles
PASS  MESH: batch index ranges are contiguous        435/435 meshes
PASS  MESH: a batch's material indexes the model's wear 434/434 meshes with a wear -- this is where the texture assignment lives
PASS  Material.lib: batch -> wear -> MAT0 -> Texm resolves 15138/15138 batches reach a real texture (905 materials)
PASS  MESH: batch indices are relative to the batch's first vertex 435/435 meshes -- every index is below its own batch's vertex count
PASS  MESH: resolved indices reference every vertex  435/435 meshes reach 100% of their vertices (reading the indices as absolute reaches 41%)
PASS  MESH: node slot indices address real slots     434/434 meshes
PASS  MESH: slot ranges lie inside the triangle and batch lists 434/434 meshes
PASS  MESH: every model yields LOD 0 geometry        434/434 meshes -- what a renderer should draw
PASS  MESH: sub-object flag bit 0 marks interior geometry 1845/1845 sub-objects agree with the o*/i* naming
PASS  MESH: buildings carry an interior path graph   29 of 30 fortif.rlb meshes, 1056 nodes, 1096 links
PASS  MESH: path graph links join real nodes         1096/1096 links
PASS  UNITS/*.dat: assemblies parse on a 112-byte stride 458/458 files, 5708 components
PASS  UNITS/*.dat: components resolve in objects.rlb 5705/5708 resolve (3 do not; see docs/07-objects.md)
PASS  every placed mission object reaches geometry   864/864 objects resolve to a .msh through objects.rlb
PASS  MESH: flag bit 0x20 marks exactly the CP_* nodes 28 nodes carry the bit and every one is named CP_* or BTCP_*; 28/28 of those names that have geometry carry it, and the 6 that do not are empty. 18402 triangles that only the unit's own view draws
PASS  MESH: level 0 fits inside the authored box once cockpits are dropped 434/434 models fit inside their own box against 422/434 while the cockpit nodes are drawn
PASS  UNITS/*.dat: a socket that disagrees is a turret hung underneath 1306/1414 sockets carry the part's own rotation; of the 108 that do not, 83 are exactly 180 degrees and all 83 of those sit on a chassis whose own name says Flying or Helicopter
PASS  objects.rlb: a .ndp is one damage record per node 542/542 tables are exactly 4 + n*76 bytes and 541 of them have one record per mesh node; 2203 records name an explosion
PASS  MESH: a node with a second block is one that can be destroyed all 145 nodes that carry a second five-slot block name an explosion in their .ndp, and the block holds the same part with pieces gone: fewer triangles on 121/145
```

The minimap check above is the important one. The game ships pre-rendered
minimaps in `ui/minimap.lib`; rasterising our parsed terrain and correlating it
against that art is an independent test of the whole chain, and it passes on
every map tried.

## What is left

Renderer work is triaged in [TODO.md](TODO.md), which records both what is
closed and what the answer turned out to be. Nothing is known to draw
*incorrectly* any more; what is missing is fidelity the game had. Worst-looking
first: what the floats inside an effect's emitter mean, which of a ground
material's two tracks the engine asks for, what would *choose* an animation
(a mesh carries exactly one, and what triggers it is gameplay), and a handful
of fields carried through the readers without being understood.

## Documentation

- [00-feasibility.md](docs/00-feasibility.md) — can this engine be rebuilt, and what would it take
- [01-nres.md](docs/01-nres.md) — the NRes container
- [02-texm.md](docs/02-texm.md) — the Texm texture format
- [03-terrain.md](docs/03-terrain.md) — `Land.msh`, and how each field was proved
- [04-missions.md](docs/04-missions.md) — the `MISSIONS` directory
- [05-engine.md](docs/05-engine.md) — the shipped DLLs and what they do
- [06-open-questions.md](docs/06-open-questions.md) — what is still unknown
- [07-objects.md](docs/07-objects.md) — `objects.rlb`, unit assemblies, object meshes
- [08-arealmap.md](docs/08-arealmap.md) — `Land.map`, the navigation mesh
- [09-method.md](docs/09-method.md) — how this was done, and the clean-room line
- [10-sky.md](docs/10-sky.md) — `sky.ske`, the day cycle, and the lens flare
- [11-effects.md](docs/11-effects.md) — `effects.rlb` and the `.exp` explosions
- [12-rsli.md](docs/12-rsli.md) — the two archives that are not NRes, and their cipher
- [13-control.md](docs/13-control.md) — `.ctl`, the movement controller and `IControl`
- [14-controls.md](docs/14-controls.md) — the input layer the game ships as text
- [15-behaviour.md](docs/15-behaviour.md) — the `.scr` mission AI scripts and `varset.var`
- [16-research.md](docs/16-research.md) — the `.trf` research tree
- [17-saves.md](docs/17-saves.md) — what a `.sav` refers to
- [18-vocabulary.md](docs/18-vocabulary.md) — the object naming scheme, and what is guesswork
- [19-descriptions.md](docs/19-descriptions.md) — `objects.dlb`, the parts database
- [20-resources.md](docs/20-resources.md) — the `.cfg` resource descriptor and `TextRes.dll`
- [21-briefing.md](docs/21-briefing.md) — `briefing.cfg`, the opening flythrough
- [22-settings.md](docs/22-settings.md) — the engine's five `.ini` files
- [23-economy.md](docs/23-economy.md) — ore and power: who makes, carries, shares and spends them
- [24-motion.md](docs/24-motion.md) — how a machine moves: states, speed limits, running gear, load, and what moving costs
- [25-sensors.md](docs/25-sensors.md) — radar, the detection shield and camouflage, and what the AI sees
- [26-damage.md](docs/26-damage.md) — hits, explosions, shields and deflectors, armour, hit points and repair
- [27-ownership.md](docs/27-ownership.md) — charging docks, control pods and capture, and the clan type
- [28-chassis.md](docs/28-chassis.md) — chassis families, the slots they declare, weight, costs and research
- [29-weapons.md](docs/29-weapons.md) — guns, ammunition clips and rounds: energy or cartridges, damage, rate and range
- [30-turrets.md](docs/30-turrets.md) — turrets: two mountings, gun sockets, and the role a turret gives a unit
- [31-packages.md](docs/31-packages.md) — the commander's packages: orders, tasks, and who may run what
- [32-builder.md](docs/32-builder.md) — builders and transports: the beam, building, upgrading and carrying ore
- [33-units.md](docs/33-units.md) — a whole robot on one sheet: `openparkan unit`
- [34-progression.md](docs/34-progression.md) — a mission's progression: routes as trigger areas, the Mission handler, messages, objectives and the win
- [35-hud.md](docs/35-hud.md) — the cockpit HUD: the weapons list, the message box, the radar and the target and own-unit panels
- [36-factory.md](docs/36-factory.md) — the factory screen: its pieces and controls, projects, and production to the bot outside
- [37-designer.md](docs/37-designer.md) — the warbot designer as a screen: its panels, tabs and rows, the project view, the buttons, the turning previews
- [40-command-mode.md](docs/40-command-mode.md) — command mode: a bunker's pod, the command camera over the base, what it draws, its keys, leaving, and telepresence
- [41-commander.md](docs/41-commander.md) — the commander panel of command mode: the icon column, the unit and building pages, the order rows a selection is offered
- [42-selection.md](docs/42-selection.md) — selecting and ordering in command mode: the selection, the band, the pick under the cursor, the cursors, clicks and pending picks

## Layout

```
openparkan/
  nres.py       container reader
  texm.py       texture decoder
  landmesh.py   terrain mesh parser
  gamedir.py    installation discovery
  mission.py    data.tma reader: clans, objects, routes
  objects.py    objects.rlb records and UNITS/*.dat assemblies
  mesh.py       object geometry and control points
  arealmap.py   Land.map navigation mesh
  materials.py  Material.lib (MAT0)
  control.py    .ctl movement controllers
  controls.py   the input tables, and the engine's own numbers
  behaviour.py  .scr mission AI scripts, varset.var, and a pseudo-code view
  research.py   .trf research tree
  descriptions.py objects.dlb parts database
  weapons.py    guns, ammunition clips and rounds
  packages.py   orders, the commander's packages, and who may run them
  cursors.py    ui/cursor.cfg and its RIFF ACON animated cursors
  units.py      a unit assembly described whole
  resources.py  .cfg resource descriptors and the PE string table
  briefing.py   briefing.cfg flythroughs and messages.cfg
  settings.py   the engine's .ini files and mission progress
  save.py       SAVE/*.sav headers and references
  viewer.py     self-contained HTML viewer generator
  verify.py     the checks quoted above
  png.py        dependency-free PNG writer
  cli.py        command line
docs/           format documentation
tests/          unit tests, no game data needed
analysis/       disassembly scaffolding (not part of the library)
```

## Prior art

- [valentineus/fparkan](https://github.com/valentineus/fparkan) — a Rust
  monorepo covering NRes and RsLi archives, static MSH geometry, `Texm`,
  `WEAR`, `MAT0` and terrain, with a headless runtime and a Vulkan renderer.
  The furthest-along project by some distance, and actively maintained.

  Its format reference has supplied facts this project failed to find on its
  own, twice. `msh.md` gave the 140-byte stream-2 header with 68-byte geometry
  slots and a node's fifteen trailing words as `slot_index[lod * 5 + group]`;
  `rsli.md` gave the `NL` archives their name and their shape — an encrypted
  entry table, and members packed one at a time by any of seven methods, which
  is why reading them as a single stream could never have worked. The cipher
  itself is not in their reference; that came out of `Ngi32.dll`. Everything
  borrowed is checked against the shipped data before it is used: the slot
  layout accounts for stream 2 exactly on all 434 meshes, and every one of the
  26 RsLi members unpacks to the size its decrypted entry declares. fparkan is
  GPL-2.0 and this project is GPL-3.0, so only its documentation was read,
  never its source; a file format is a fact, an implementation of one is not.
- [AlexKimov/parkan-file-formats](https://github.com/AlexKimov/parkan-file-formats)
  — 010Editor templates and QuickBMS scripts, mostly for Parkan 1's `.lib`.

## Licence

Copyright (C) 2026 Michał Moroz. This program is free software: you can
redistribute it and/or modify it under the terms of the GNU General Public
License as published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version — the terms are in
[LICENSE](LICENSE). It is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.

The game's data files are not covered and are not included; you need your own
copy of the game.
