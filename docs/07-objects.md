# Objects: `objects.rlb`, unit assemblies, and object meshes

Three small formats that together turn a name in a mission file into geometry.

```
mission data.tma  "s_tree_06"                    (scenery)
  -> objects.rlb  STAT record
     -> static.rlb  s_tree_0_06.msh   geometry
     -> static.rlb  s_tree_0_06.wea   texture names

mission data.tma  "UNITS\BUILDS\GENER\gener01.dat"   (building or unit)
  -> the .dat assembly, a list of components
     -> objects.rlb  FORT/INTO/EXTO/... record per component
        -> fortif.rlb / intsys.rlb / turrets.rlb / ...
```

## objects.rlb — resource reference records

590 records. Each is a run of **64-byte slots**, each slot a
`(archive, member)` pair in two 32-byte NUL-padded fields. The NRes tag says
what kind of thing the record describes:

| Tag | Size | Slots | Count | Meaning |
|---|---|---|---|---|
| `STAT` | 384 | 6 | 81 | scenery — `.msh` `.wea` `.cpt` `.ndp` `.ctl` |
| `INTO` | 320 | 5 | 192 | internal part (engines, armour, batteries) |
| `EXTO` | 320 | 5 | 146 | external part (turrets) |
| `BULL` | 320 | 5 | 67 | projectile |
| `BTLU` | 320/448 | 5/7 | 63 | creature |
| `FORT` | 128 | 2 | 34 | fortification / building |
| `WPNS` | 320 | 5 | 5 | weapon |
| `SUNO` | 128 | 2 | 2 | sun |

A scenery record reads:

```
slot 0  static.rlb  s_tree_0_06.msh
slot 1  static.rlb  s_tree_0_06.wea
slot 2  static.rlb  s_tree_0_06.cpt
slot 3  static.rlb  s_tree_0_06.ndp
slot 4  static.rlb  s_tree_a_06.ctl
slot 5  -           -
```

All 405 populated slots across the 81 `STAT` records resolve to real archive
members.

## UNITS/**/*.dat — unit and building assemblies

458 files, all the same shape:

```
uint32   magic, always 0x0000F0F1
uint32   class word
components × N, 112 bytes each:
    char[32]  archive name, always "objects.rlb"
    char[32]  record name
    uint32    varies
    int32     varies
    char[32]  display name
    uint32    varies
    int32     varies
```

**A unit is an assembly of parts**, which is the game's central mechanic
written straight into the data. `UNITS/UNITS/BATTLE/w_b_trk1.dat` is 18
components:

```
R_B_04       Large Track Chs (L-42t)
i_arm_b_02   ARMOUR LA.Mk3 (ARM 3)
i_eng_b_03   Large engine (LEng4)
i_pws_b_03   Large Battery (LBt4)
i_fsh_b_df   Lrg Shld generator (LSG1)
i_dsh_b_df   Large detect.shld (LDS1)
...
```

The display names are the strings the game's UI shows, so the whole parts
catalogue is readable without touching the executable.

All 458 files parse on the 112-byte stride, giving 5708 components. **5705
resolve** in `objects.rlb`; three do not — `fr_l_mast`, `fr_l_tele` and
`u_tel_def_a_03` — which look like content cut before release.

## MESH — object geometry

An object mesh is **an NRes archive nested inside an archive member**, using
the same numeric-type-as-stream-selector convention as the terrain and the
same per-vertex encodings. All 68 meshes in `static.rlb` carry the same 14
streams:

| Stream | Stride | Indexed by | Contents |
|---|---|---|---|
| 3 | 12 | vertex | position, `float32` x/y/z, **Z up** |
| 4 | 4 | vertex | normal, `int8` x/y/z ÷ 127, plus a padding byte |
| 5 | 4 | vertex | UV, `uint16` 8.8 fixed point |
| 6 | 6 | face | triangle, three `uint16` indices — **relative to the batch** |
| 9 | 32 | sub-object | sub-object name |
| 1 | 38 | node | flags, parent, `slot_index[lod * 5 + group]` |
| 7 | 16 | face | face record: fields 1–3 are edge neighbours; the rest unresolved |
| 13 | 20 | batch | draw batch: material, index range, vertex range |
| 17 | 20 | node | building interior path graph — see below |
| 10 | 4 | sub-object | one `uint32`, zero throughout |
| 2 | 68 | slot | 140-byte header, then triangle and batch ranges |
| 8, 15, 19 | — | — | animation keys and auxiliary streams, unresolved |

Streams 4 and 5 use exactly the encodings established for the terrain, which
is a useful independent confirmation of both: the normals come out unit-length
to within 0.013 across all 68 meshes.

### `o*` and `i*` are external and internal *components*, not two models

`fr_b_bunker` has nodes `o01`…`o04` and `i01`…`i14`, and flag bit 0 splits them
exactly along that naming (1564/1564 sub-objects across six archives). The
tempting reading — that `o*` is the outside and `i*` the walk-in interior — is
**wrong for rendering**, and acting on it makes buildings far too short:

| mesh | `o*` at LOD 0 | `i*` at LOD 0 |
|---|---|---|
| `fr_l_bunker` | −5.71 .. 12.54 | −8.25 .. **44.00** |
| `fr_l_plant` | −32.95 .. 32.95 | −35.74 .. **53.28** |

A building's tall structure lives in its `i*` nodes. The flag matches
`objects.rlb`'s `INTO` / `EXTO` tags, which distinguish a robot's internal
parts (engine, battery, armour) from its external ones (turrets, guns) — both
of which are drawn. So `ObjectMesh.select()` draws every node by default and
takes `interior` only as an optional filter.

Stream 1 carries one 38-byte header per sub-object:

```
uint16   flags        bit 0 set = interior
uint16   parent       sub-object index, 0xFFFF for a root
uint16   ...          a list of part indices into stream 2, 0xFFFF-terminated
```

**Flag bit 0 agrees with the `o*` / `i*` naming on all 1564 sub-objects across
six archives.** The parent field gives a hierarchy: on the bunker, the
interiors hang off `o01`, and `o03`, `o04`, `Base_TL` chain off `o02`.

This explains `objects.rlb`'s `INTO` and `EXTO` record tags — interior and
exterior objects — and why only buildings have a path graph.

### Nodes, slots and levels of detail

A node's 38-byte record is:

```
uint16   flags            bit 0 set = interior
uint16   parent           node index, 0xFFFF for a root
uint16   animation map start
uint16   fallback key     selects the node's static pose
uint16   slot_index[15]   addressed as [lod * 5 + group], 0xFFFF = none
```

and stream 2 is a **140-byte header followed by 68-byte slots**:

```
uint16   first triangle, triangle count
uint16   first batch, batch count
float32  aabb min[3], aabb max[3]
float32  bounding sphere centre[3], radius
uint32   x5
```

`140 + 68 * count` accounts for stream 2 exactly on **all 434 meshes**. This
is what a renderer needs: pick a level of detail, skip the interior nodes,
follow each node's slots to a run of triangles.

Selecting **LOD 0 across all nodes** is what a renderer wants: on `fr_l_bunker`
that is 3050 triangles of 4066, the remainder being LOD 1 duplicates. Drawing
every level of detail superimposed is what makes a building look like
scrambled geometry — not the `i*` nodes.

### Buildings must be rested on their base, not their origin

A mission places an object by putting model **z = 0** at the placement height,
and for units and scenery that is right — they are authored with their base at
the origin (measured base-minus-terrain: units +0.16, rocks 0.00).

Buildings are not. Their exterior is authored **symmetric about z = 0**:
`fr_m_bunker` spans −9.04..9.04, `fr_l_angar` −18.43..18.43, `fr_l_plant`
−32.95..32.95. Placing that origin at ground level buries half the building,
and measured across all 167 placed buildings the base sits a median of
**23.6 units underground** — about 44% of the model.

Two things say the minimum z really is the bottom of the structure rather than
a deep foundation:

- the interior path graph's lowest waypoint sits slightly *above* the exterior
  minimum on 28 of 29 buildings — a floor below ground level would make no
  sense;
- units and scenery, which are visibly correct, are authored the other way.

So a renderer should offset a model by `-min(z)` of the geometry it draws.
That is a no-op for units and rocks and lifts buildings out of the ground.

**This is a heuristic, not the engine's rule.** The real datum has not been
found. Ruled out: the `Root` control point (buildings have `P###` points and no
`Root`), the `.bas` footprint plane (its z range is symmetric, e.g.
−60.51..60.51 on `fr_b_bunker`, so it is not a ground plane as read), the slot
AABBs (they match the raw vertex bounds exactly, so they carry no transform),
and the root node's pose (identity on the buildings checked).

**Still missing: node poses.** Stream 8 holds 24-byte keys — a `float32[3]`
translation and a packed rotation — and a node's `fallback_key` selects one.
Poses compose down the parent chain, and composing the translations alone
produces offsets like (54.5, 77.1, 57.2) that fling geometry away, so the
rotation matters and its packing is not decoded. Identity reads as
`(0, 0, 32767, 0, 0, 0)` and a quarter turn as `(0, 0, 23220, 0, 0, 23119)` —
23220/32767 = 0.709, 23119/32767 = 0.706, so cos and sin of 45° are in there.

Until poses are applied, sub-assemblies render at their authored origin rather
than their assembled position: some parts of a building float, and
`fr_l_gener` comes out spiky.

### Units are authored at a different scale

A unit's chassis is tiny in model units — `r_h_02` is 390 triangles inside a
0.7 × 0.8 × 1.26 box, against a bunker 52 units tall. Something scales them up
and it is not the mission record, whose scale is `1,1,1` throughout. Not yet
investigated.

Sub-object names describe the model's construction: `s_tree_0_06` is
`Base_TM`, `leaf1_m1o1`, `leaf2_m1o1`, `leaf3_m1o1`.

### Vegetation is billboards

Rendering the decoded geometry untextured makes rocks look like rocks, but
trees look like sparse skeletons — because that is what they are. `s_tree_33`
is 24 vertices and **12 triangles**: a pair of crossed planes. `s_tree_05` has
ten sub-objects named `lf11_m1o1` through `lf33_m1o1` — leaf clusters. The
foliage is alpha-textured planes, so it needs the texture assignment to read
correctly.

### Materials are per batch, not per face

This is why the assignment resisted every search of the face record: **there
is no per-face material field.** Stream 13 groups the index buffer into draw
batches and names a material for each.

```
uint16  0, 0
uint16  material          low byte indexes the wear; high byte 0xFF or 0x00
uint16  0xFFFF
uint16  index count       3 x the triangles in this batch
uint16  first index       offset into stream 6
uint16  0
uint16  vertex count
uint16  first vertex
uint16  0
```

The batches tile the index buffer exactly — index counts sum to `3 × triangles`
and the offsets are contiguous — on all 435 meshes. `fr_b_bunker` has 224
batches over 39 materials and uses every one of them.

The element count comes from the stream's own NRes directory entry, which is
also how the 20-byte stride was pinned down; a hex dump alone suggested 12.

### Indices are batch-relative

Stream 6's indices are **relative to the covering batch's `first_vertex`**, the
DirectX `DrawIndexedPrimitive` convention: the real vertex is
`batch.first_vertex + index`.

This is easy to get wrong, and a naive range check does not catch it. Relative
indices are small — the largest on `fr_m_brige` is 243 in a 1458-vertex mesh —
so they *also* look like valid absolute indices, and "every index is inside the
vertex array" passes while the geometry is scrambled.

The decisive test is **vertex reachability**: a correct reading must use every
vertex the file stores. Resolved through the batches, the indices reach
**100% of vertices on all 435 meshes**. Read as absolute they reach 41% on
average and as little as 6% on `fr_l_gener` — a mesh does not store 5461
vertices in order to draw 330 of them. Two further confirmations: every index
is below **its own batch's** `vertex_count` (435/435), and the batch windows
end exactly at the vertex count (`max(first_vertex + vertex_count) == 1458` on
the bridge).

Edge-length statistics, by contrast, *prefer* the wrong answer — the absolute
reading produces tighter triangles because it keeps re-using one small corner
of the vertex array. Plausibility is not verification.

### The material chain

```
mesh stream 13 batch
  -> wear entry            the .wea palette, up to 46 material names
     -> Material.lib MAT0  905 materials, each naming its texture layers
        -> Textures.lib    a Texm, named exactly as the material spells it
```

**15053 of 15138 draw batches reach a real texture** this way. The 85 that do
not are animation frames (`0FAIR.0`, `1FAIR.0`, …) held outside `Textures.lib`.

`MAT0` is only partly mapped: the record opens with a layer count and carries
per-layer colour bytes, but the per-layer stride varies with layer type. The
texture names are extracted by pattern instead, which is safe because they are
distinctive — 3096 of 3139 resolve.

A wear is a **skin**, not just a texture list: `World3D.dll` calls them wears
and has `CMD_CAMOUFLAGE_WEAR` alongside "Illegal wear length".

### Buildings carry an interior path graph

Stream 17 is non-empty on **29 of the 30 meshes in `fortif.rlb` and nowhere
else** — only a building has an inside to walk around. The engine calls it a
*hall way*; `MHallWay::LoadFromResource` in `ArealMap.dll` reads it.

```
nodes × 20 bytes    float32 x, y, z; uint32 ×2
links × 40 bytes    uint32 start node, end node; uint32 ×8 (0xFFFFFFFF)
```

Both counts come from the directory entry: nodes from the element-count field
at +4, links from the field at +8. `size = 20*nodes + 40*links` holds on all
29. Every link joins two real nodes (1096/1096).

## CTPT — control points

Every record with geometry has a `.cpt` slot. The format is **two parallel
arrays, not an array of records**, which is why the total is always
`4 + count × 68`:

```
uint32   count
count × float32[9]     numeric data      (36 bytes each)
count × char[32]       names             (32 bytes each)
```

All 284 `CTPT` members across ten archives parse, giving **3599 control points,
every one of them named**. The names say what the format is for:

| Archive | Names | Meaning |
|---|---|---|
| `static.rlb` | `Root`, `Exp_X/Y/Z`, `Eff1_X/Y/Z`, `Up1Light` | origin, explosion and effect frames, light positions |
| `turrets.rlb` | `TurretCenter`, `TurretDirect` | turret mount and aim direction |
| `guns.rlb` | `Dir_1`, `Width_1` | barrel direction and spread |
| `bases.rlb` | `foot_fl`, `foot_fl_d` | walker foot positions |
| `fortif.rlb` | `P000`, `P001`, … | numbered points around a building |

A triple of points named `Exp_X`, `Exp_Y`, `Exp_Z` shares one position and
carries unit directions `(1,0,0)`, `(0,1,0)`, `(0,0,1)` — **three named points
describing one attachment frame**. That is the clearest evidence for what the
nine floats are.

The reading `(zero, position, unit direction)` holds strongly in `static.rlb`
(first triple exactly zero on 93% of points, third unit-length on 99%, second
inside the mesh bounds on 99%) and in `turrets.rlb` (third unit-length on
100%). It does **not** hold everywhere: `guns.rlb` and `parts.rlb` appear to
store scalars such as `Width` in a vector slot. The parser therefore exposes
the three triples as-is.

## Local origins

Object meshes are not consistently based at z = 0. `s_tree_0_06` spans
z −11.1 to +22.8; `s_tree_59` spans −127.1 to +2.1, almost entirely *below*
its origin — and the missions place it 125 units above the ground, which
compensates almost exactly. Adding the mesh's minimum z to its placement does
not make scenery sit flush in general (median −1.9, wide spread), so there is
probably a transform in the `.ctl` controller that has not been read yet.
Buildings, by contrast, sit at a median of +0.000 above the terrain.
