# openparkan

Clean-room readers for the data files of **Parkan: Iron Strategy** (Nikita,
1998), and a step toward an open reimplementation of its engine.

This repository contains **no game assets**. Point it at your own installation.

Everything here was derived by observing the shipped data files. No
disassembly of the game binaries was involved, and none of the original code is
reproduced.

## Status

| | |
|---|---|
| **NRes containers** | done — all 116 archives, 6697 members, round-trip-accurate model |
| **Texm textures** | done — all 393 textures decode, all 5 pixel formats |
| **`Land.msh` terrain** | done — all 33 maps, geometry, normals, UVs, materials, water |
| **3D terrain viewer** | done — self-contained HTML, no server |
| **`data.tma` missions** | done — all 29 parse to EOF, 864 objects placed |
| **`objects.rlb`, unit assemblies** | done — 590 records, 458 assemblies, 5708 components |
| **Object meshes** | geometry done — 68 meshes; per-face texture unresolved |
| **`Land.map` navigation** | not started |
| **Behaviour scripts, gameplay** | not started |

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
uv run openparkan mission CAMPAIGN/CAMPAIGN.02/Mission.03 --list
uv run openparkan viewer SC_3 Tut_1 ILKON --out three-maps.html
```

## Verification

Reverse-engineered format notes are easy to write and easy to get wrong, so
every factual claim in `docs/` is re-derived from the installed files by
`uv run openparkan verify`:

```
PASS  NRes: header size == file size                 116/116 archives
PASS  NRes: every member offset is 8-byte aligned    116/116 archives, 6697 members
PASS  NRes: no member ranges overlap                 0 overlaps
PASS  NRes: inter-member padding is zero-filled      0 non-zero gaps, largest gap 7 bytes (< 8 as expected)
PASS  Texm: declared format predicts the payload size 328/393 exact (rest have a truncated mip tail)
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
PASS  data.tma: parses exactly to end of file        29/29 missions, 864 objects
PASS  data.tma: the map it names exists              29/29 missions reference a real DATA/MAPS entry
PASS  data.tma: placed objects lie inside the map    864/864 objects within their map's XY extent
PASS  data.tma: buildings sit on the terrain surface median height above ground +0.000 over 167 buildings
PASS  data.tma: every object reference resolves      864/864 -- UNITS/*.dat on disk, scenery as STAT in objects.rlb
PASS  data.tma: ClanID is a 0-based index into the clan list 463/463 object ClanIDs in range
PASS  data.tma: objects belong to the clan whose base they sit at 123/125 (98.4%) on skirmish and multiplayer maps
PASS  objects.rlb: every record parses into slots    590 records, tags ['BTLU', 'BULL', 'EXTO', 'FORT', 'INTO', 'STAT', 'SUNO', 'WPNS']
PASS  objects.rlb: scenery resource slots resolve    405/405 slots across 81 STAT records
PASS  MESH: object meshes parse with consistent streams 68/68 meshes, 41663 triangles, all indices in range
PASS  MESH: int8/127 normals are unit length         worst deviation 0.0130 -- the same encoding as the terrain
PASS  UNITS/*.dat: assemblies parse on a 112-byte stride 458/458 files, 5708 components
PASS  UNITS/*.dat: components resolve in objects.rlb 5705/5708 resolve (3 do not; see docs/07-objects.md)
```

That last check is the important one. The game ships pre-rendered minimaps in
`ui/minimap.lib`; rasterising our parsed terrain and correlating it against
that art is an independent test of the whole chain, and it passes on every map
tried.

## Documentation

- [00-feasibility.md](docs/00-feasibility.md) — can this engine be rebuilt, and what would it take
- [01-nres.md](docs/01-nres.md) — the NRes container
- [02-texm.md](docs/02-texm.md) — the Texm texture format
- [03-terrain.md](docs/03-terrain.md) — `Land.msh`, and how each field was proved
- [04-missions.md](docs/04-missions.md) — the `MISSIONS` directory
- [05-engine.md](docs/05-engine.md) — the shipped DLLs and what they do
- [06-open-questions.md](docs/06-open-questions.md) — what is still unknown
- [07-objects.md](docs/07-objects.md) — `objects.rlb`, unit assemblies, object meshes

## Layout

```
openparkan/
  nres.py       container reader
  texm.py       texture decoder
  landmesh.py   terrain mesh parser
  gamedir.py    installation discovery
  mission.py    data.tma reader: clans, objects, routes
  objects.py    objects.rlb records and UNITS/*.dat assemblies
  mesh.py       object geometry (MESH members)
  viewer.py     self-contained HTML viewer generator
  verify.py     the checks quoted above
  png.py        dependency-free PNG writer
  cli.py        command line
docs/           format documentation
```

## Prior art

- [valentineus/fparkan](https://github.com/valentineus/fparkan) — a Rust
  monorepo covering NRes and RsLi archives, static MSH geometry, `Texm`,
  `WEAR`, `MAT0` and terrain, with a headless runtime and a partial Vulkan
  renderer. The furthest-along project by some distance.
- [AlexKimov/parkan-file-formats](https://github.com/AlexKimov/parkan-file-formats)
  — 010Editor templates and QuickBMS scripts, mostly for Parkan 1's `.lib`.

## Licence

MIT for this code. The game's data files are not covered and are not included;
you need your own copy of the game.
