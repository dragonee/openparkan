# Open questions

Things observed but not resolved. Roughly in the order they block progress.

## data.tma record framing — blocks mission loading

Strings are length-prefixed and the object graph is legible (see
[04-missions.md](04-missions.md)), but the 16-byte header differs between
missions and the binary framing between strings has not been mapped. Until
this is done, missions can be *read* but not *loaded*.

The property-name tables embedded next to each object are the lever here: they
give the field names, so it should be possible to work out the record layout by
aligning names against the bytes that follow them.

## Land.map / ArealMap — blocks pathfinding

One NRes member, type 12, named `ArealMap`, 188538 bytes on SC_3, starting with
float32 data. Not a plain square grid: 188538 divides to 434.2², 307.0² or
217.1² at 1, 2 and 4 bytes per cell, none of them whole. Handled by
`ArealMap.dll` (226 KB). Presumably the navigation and region map used for
pathfinding and territory.

## The `NL` archives

`gamefont.rlb` and `sprites.lib` are not NRes. They begin:

```
4E 4C 00 01   'NL', version 1
<u16> <u16>   two equal values (2,2 and 24,24 respectively)
00 00 00 00
BA AB         0xABBA marker
```

after which the payload is high-entropy — compressed or obfuscated. Only 2
files, holding fonts and 2D sprites, so this blocks UI work but nothing else.

## Mesh format for objects

`MESH` members in `static.rlb` and `system.rlb` hold the geometry for units and
buildings. Not yet examined here; `Land.msh` was the priority because terrain
is what a viewer needs first. The upstream Rust project
[fparkan](https://github.com/valentineus/fparkan) reports validated static MSH
geometry support and is the obvious place to start.

## Unresolved terrain fields

- `Land.msh` stream 1: 2432 bytes on SC_3, mostly `0xFF`. Not indexed by
  vertex or face count.
- `Land.msh` stream 2: 737 float3 on SC_3. The first 8 are the map's bounding
  box corners; the remaining 729 (= 27³) look like a spatial subdivision.
- Face record fields 10, 11, 12.
- Face record field 13 (0..62, 59 distinct on SC_3) — treated here as a patch
  or sector id, but not confirmed.
- Stream 11's flags word: 72 on 4228 faces, 88 on 329, then 328, 968, 984, 344.
  Bit flags of some kind; 88 correlates with water.

## Textures

- Palettised textures (format `0`) are decoded fully opaque. Whether a palette
  index is treated as a colour key for transparency is unknown; foliage
  textures suggest one probably is.
- 65 of 393 textures have a mip tail shorter than the declared level count
  implies. Harmless for level 0, but a packer would need to reproduce it.

## Special materials

Eight terrain texture names do not resolve in `Textures.lib`: `WATER`,
`WATER_M`, `WATER_BOT`, `B_S0`, `B_MTP_01`, `ENV_NLAVA`, `ENV_NLAVA_M`,
`ENV_LAVA_BOT`. Likely animated materials defined in `Material.lib` (206 KB,
NRes) — not yet inspected.

## Not looked at at all

`*.ctl` controllers, `*.exp` explosions, `sky.ske` skyboxes, `UNITS/**/*.dat`
unit definitions, `lightmap.lib`, `.scr` node semantics, the `.trf` streams,
save games in `SAVE/`, and the network protocol.
