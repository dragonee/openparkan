# Open questions

Things observed but not resolved. Roughly in the order they block progress.

## data.tma leftovers

The format is parsed end to end (see [04-missions.md](04-missions.md)), but a
few fields are carried through without being understood:

- A property stores three words after its type tag. The first is the value;
  the other two are constant per property name across every mission, so they
  read as bounds or defaults, but nothing confirms which is which.
- The four words after an object's instance name (`0, -1, -1, 1` throughout).
- The word before the object count, always 10.
- The word after a clan's behaviour-tree path.
- The four words trailing each trailer viewpoint.
- Object `scale` is `1,1,1` in every shipped mission, so the axis order is
  unverified.

## Land.map / ArealMap — blocks pathfinding

One NRes member, type 12, named `ArealMap`; 132 KB on ILKON, 188 KB on SC_3,
750 KB on map 23. Handled by `ArealMap.dll` (226 KB). This is presumably the
navigation and territory map. Partly characterised, not solved.

**It is not a raster.** Bytes per world unit² varies more than twenty-fold
across the 33 maps (0.014 on Net_4_04 to 0.34 on map 41), so size tracks
content, not area. Two maps that share terrain — Net_2_01 and Net_4_02, with
identical vertex and face counts — have byte-identical sizes, which is a useful
consistency check for anyone continuing.

**The file opens with area polygons.** Each record runs:

```
float32  centre x, centre y
float32  0, 0
float32  polygon area          (38910 on a ~125-unit hexagon; 2.598 * 125^2 = 40600)
float32  0, 0, 1.0
uint32   1, 0, 1, 0
uint32   vertex count          (7 and 10 in the records read so far)
uint32   0
         count x { float32 x; float32 y; float32 0 }
uint32   a trailing block of varying length
```

The vertices trace a closed polygon on a lattice of about 125 units — on ILKON
the first record is a hexagon through (374.3, 374.3), (499.0, 374.3),
(623.8, 374.3), (748.5, 374.3), (623.8, 499.0), (499.0, 499.0), (374.3, 499.0).

**But only about ten of these records exist**, filling the first ~6.5 KB of a
132 KB file. The trailing block's length is not encoded anywhere found so far,
so the records cannot be walked reliably; they were located by scanning for the
header signature.

**The remaining 95% is something else**: uint16 values, mostly small, with long
verbatim runs — `(1, 29)` repeated for stretches at one offset, `(10, 1)` at
another. That shape suggests run-length encoding, but summing the pairs either
way gives totals in the hundreds of millions, far too large for any grid the
map could have, so plain RLE is ruled out in both orderings.

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

## Object mesh: which texture a face uses

Geometry reads fine (see [07-objects.md](07-objects.md)), but nothing found so
far picks between the textures a mesh's `.wea` lists — and meshes can list a
lot of them: `static.rlb` has models with a single sub-object and twelve or
fourteen textures.

Ruled out, so that the next person need not repeat it:

- **The face record (stream 7, 16 bytes per face).** Field 0 is zero on every
  face of every mesh in `static.rlb`. Fields 1–3 are the three edge
  neighbours: their high bytes take values 0..4 with 255 for "none", which is
  exactly what a face index into a 1200-face mesh looks like. No field has a
  value range matching the texture count.
- **Per sub-object.** Sub-object count equals texture count on only 9 of 68
  meshes; 14 meshes have one sub-object and two textures, 5 have one
  sub-object and twelve.
- **Stream 13 and stream 15**, scanned byte-column-wise for any field with as
  few distinct values as the texture count.

Untextured, rocks look right and vegetation looks like bare skeletons, because
the foliage is alpha-textured billboards — see [07-objects.md](07-objects.md).

## Object mesh: local origins and the .ctl controller

Meshes are not consistently based at z = 0, and adding a mesh's minimum z to
its placement does not make scenery sit flush on the terrain. Every `STAT`
record has a `.ctl` slot that has not been read; a transform there is the
likely explanation.

## How a unit's components are positioned

A `.dat` assembly lists its parts but not where they attach. The `.cpt` control
points are clearly the raw material — `TurretCenter`, `TurretDirect`, `foot_fl`
— but nothing yet says which point on a chassis a given part binds to. Until
that is worked out a unit can only be drawn as its first component.

## CTPT field roles outside static.rlb

The container is solid everywhere (all 284 members parse, all 3599 points
named), but the nine floats only read cleanly as
`(zero, position, unit direction)` in `static.rlb` and `turrets.rlb`.
`guns.rlb` and `parts.rlb` put scalars like `Width` in a vector slot.

## BASE — building footprints

`fortif.rlb` `.bas` members open with a count and then float triples at z ≈ 0
that trace a polygon, so they read as a building's ground footprint. The
header does not divide evenly into the payload, so the full record is not
mapped.

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
- Face field 0's other values (1536, 1540, 9728 on SC_3) and field 1's bit
  `0x10`. Only the water bit in each is understood.

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
