# openparkan

Clean-room readers for the data files of **Parkan: Iron Strategy** (Nikita,
1998), and a step toward an open reimplementation of its engine.

This repository contains **no game assets**. Point it at your own installation.

Most of it was derived by observing the shipped data files. `Land.map` resisted
that and was recovered by disassembling `ArealMap.dll`; what that means in
practice — format facts yes, code no — is set out in
[09-method.md](docs/09-method.md). No code from the game is reproduced here.

## Status

| | |
|---|---|
| **NRes containers** | done — all 116 archives, 6697 members, round-trip-accurate model |
| **Texm textures** | done — all 393 textures decode, all 5 pixel formats, alpha included |
| **`Land.msh` terrain** | done — all 33 maps, geometry, normals, both texture layers and their blend, water |
| **3D terrain viewer** | done — self-contained HTML, no server |
| **`data.tma` missions** | done — all 29 parse to EOF, 864 objects placed |
| **`objects.rlb`, unit assemblies** | done — 590 records, 458 assemblies, 5708 components |
| **Object meshes** | done — geometry, materials, textures, node poses, LOD and interior/exterior selection |
| **Assembled units and buildings** | done — the `.dat` component tree and where each part bolts on |
| **Materials** | done — `Material.lib`, 905 materials, animation frames and diffuse colour; 99.4% of batches reach a texture |
| **Baked lighting** | done — `lightmap.lib`, 21 buildings, per-batch |
| **Building interiors** | done — path graph for 29 buildings |
| **Control points** | done — 284 members, 3599 named attachment points |
| **`Land.map` navigation mesh** | done — all 33 maps, 34662 areals, adjacency and grid |
| **Sky** | done — `sky.ske` day cycle and `sky.wea`'s nine texture slots, all 29 missions |
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
PASS  NRes: the element-count field equals size / stride 231/231 terrain streams across 33 maps
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
PASS  data.tma: buildings sit on the terrain surface median height above ground +0.000 over 167 buildings
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
PASS  Material.lib: batch -> wear -> MAT0 -> Texm resolves 15053/15138 batches reach a real texture (905 materials)
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
```

That last check is the important one. The game ships pre-rendered minimaps in
`ui/minimap.lib`; rasterising our parsed terrain and correlating it against
that art is an independent test of the whole chain, and it passes on every map
tried.

## What is left

Renderer work is triaged in [TODO.md](TODO.md), which records both what is
closed and what the answer turned out to be. Nothing is known to draw
*incorrectly* any more; what is missing is fidelity the game had. Worst-looking
first: the sky's weather layers (lens flares, snow and rain), the second layer
of a terrain material, effects, the two `NL` archives that hold the fonts and
2D sprites, and a handful of fields carried through the readers without being
understood.

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
  viewer.py     self-contained HTML viewer generator
  verify.py     the checks quoted above
  png.py        dependency-free PNG writer
  cli.py        command line
docs/           format documentation
analysis/       disassembly scaffolding (not part of the library)
```

## Prior art

- [valentineus/fparkan](https://github.com/valentineus/fparkan) — a Rust
  monorepo covering NRes and RsLi archives, static MSH geometry, `Texm`,
  `WEAR`, `MAT0` and terrain, with a headless runtime and a Vulkan renderer.
  The furthest-along project by some distance, and actively maintained.

  Its `docs/reference/msh.md` supplied two facts this project had failed to
  find on its own: that stream 2 is a 140-byte header followed by 68-byte
  geometry slots, and that a node's fifteen trailing words are
  `slot_index[lod * 5 + group]`. Both were checked against the shipped data
  before being used — the slot layout accounts for stream 2 exactly on all 434
  meshes. fparkan is GPL-2.0 and this project is MIT, so only its
  documentation was read, never its source; a file format is a fact, an
  implementation of one is not.
- [AlexKimov/parkan-file-formats](https://github.com/AlexKimov/parkan-file-formats)
  — 010Editor templates and QuickBMS scripts, mostly for Parkan 1's `.lib`.

## Licence

MIT for this code. The game's data files are not covered and are not included;
you need your own copy of the game.
