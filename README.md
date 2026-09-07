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
| **`data.tma` missions** | partial — clans, map link, object paths, property schema |
| **`Land.map` navigation** | not started |
| **Object meshes, scripts, gameplay** | not started |

## Quickstart

Requires [uv](https://docs.astral.sh/uv/). No other dependencies — the toolkit
runs on a bare interpreter by design.

```bash
uv sync
export PARKAN_DIR="/path/to/Parkan Iron Strategy"   # or pass --game

uv run openparkan info                       # summarise the install
uv run openparkan verify                     # re-derive every documented claim
uv run openparkan maps                       # list all 33 maps
uv run openparkan heightmap SC_3 --out sc3.png
uv run openparkan viewer --out terrain.html  # 3D viewer, all maps, ~11 MB
```

The game directory is found automatically if it sits next to this repo, is
named by `$PARKAN_DIR`, or lives in the default Steam location.

### More

```bash
uv run openparkan ls Textures.lib --type Texm
uv run openparkan extract sounds.lib --out /tmp/sounds     # RIFF/WAVE, playable as-is
uv run openparkan textures Textures.lib --out /tmp/tex     # 393 PNGs
uv run openparkan textures ui/minimap.lib --out /tmp/minimaps
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
PASS  NRes: inter-member padding is zero-filled      0 non-zero gaps, largest gap 7 bytes
PASS  Texm: declared format predicts the payload size 328/393 exact
PASS  Texm: every texture decodes to RGBA            393/393, formats {0,565,888,4444,8888}
PASS  Land.msh: all maps parse                       33 maps
PASS  Land.msh: face indices within the vertex array 33/33 maps
PASS  Land.msh: every vertex is referenced by a face 33/33 maps
PASS  Land.msh: face adjacency is mutual             33/33 maps
PASS  Land.msh: int8/127 normals are unit length     worst deviation 0.0133
PASS  Land.msh: layer-1 UV == world XY / 50          worst residual 0.004 texel units
PASS  Land.msh: texture indices resolve through Land1.wea
PASS  Land.msh: surface bit 0x02 marks exactly the water faces  33/33 maps, 3630 faces
PASS  Land.msh: face flags 1544 agree with the surface bit      33/33 maps
PASS  Land.msh: water is a single flat plane per map            11/11 maps with water
PASS  Terrain matches the game's own minimap art     SC_3 +0.883, Tut_1 +0.927,
                                                     ILKON +0.965, K1F +0.901
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

## Layout

```
openparkan/
  nres.py       container reader
  texm.py       texture decoder
  landmesh.py   terrain mesh parser
  gamedir.py    installation discovery
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
