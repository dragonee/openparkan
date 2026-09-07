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
    uint32    flags, 1 throughout
    int32     attachment node in the parent's mesh, -1 on the root
    char[32]  display name
    uint32    class
    int32     child count
```

The class says what a part is and, with it, what it hangs off:

```
0  chassis     the root
1  turret      bolts to the chassis
2  armour      modelled, but only visible from inside
3  internal    engine, battery, shield, sensor, deflector -- likewise
4  gun         bolts to the turret
5  ammunition  a clip belonging to the gun above it
```

The component list is **a tree written depth first**: a record owns the next
`child_count` records' subtrees. That reading consumes all 458 shipped
assemblies exactly, and it is what makes the attachment field usable — a
child's node index is into *its own parent's* mesh, not the chassis's.

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

### How parts attach

A mesh carries **geometry-less `Base_*` nodes**: `Base_TM` and `Base_TL` on a
chassis, `Base_LU_01` / `Base_RU_01` / `Base_gun` on a turret, `Base_RDR` and
`Base_DF` for a radar and a deflector. They exist only to hold a pose, and a
component's attachment field is the index of one of them. Across all 458
assemblies, **946 of 946 guns and 468 of 468 turrets** land on a `Base_*`
node, the one exception being a chassis with no mesh at all.

The mount itself is the socket's **position**. Its rotation is not applied:
a socket carries the same rotation as the root node of the part that plugs
into it — on 1306 of the 1414 attachments — so composing the two turns the
part twice and guns come out pointing sideways. Cancelling it instead
(`socket ∘ root⁻¹`) fixes those but flips the other 108, whose socket and root
disagree by exactly 180°, upside down; `r_l_02`'s turret ends up underneath
the chassis. Taking the position and leaving the part in its own orientation
is right in both groups, and puts a bunker's turret at z 16.49 on a bunker
whose roof is at 16.49.

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
uint16   animation map start   into stream 19, 0xFFFF if not animated
uint16   fallback key     index into stream 8
uint16   slot_index[15]   addressed as [variant * 5 + lod], 0xFFFF = none
```

and stream 2 is a **140-byte header followed by 68-byte slots**:

```
uint16   first triangle, triangle count
uint16   first batch, batch count
float32  aabb min[3], aabb max[3]
float32  bounding sphere centre[3], radius
uint32   x5
```

`140 + 68 * count` accounts for stream 2 exactly on **all 434 meshes**.

The 15 slot indices are **three blocks of five**, and each block is one
variant: within a block the first four triangle counts fall monotonically on
**1157 of 1161** chains, and only 869 once the fifth slot is counted, so the
fifth is not a level. `s_stn_0_01` is the clearest case — one node, three
slots of 58, 22 and 14 triangles, which is a level ladder and not three parts.

So **one node draws one slot**: `slot_index[variant * 5 + lod]`. Reading the
five as parallel groups draws four levels of detail on top of one another,
which is what makes a model look like scrambled geometry — not the `i*` nodes.
Across the placed missions it inflated the drawn geometry from 229k triangles
to 381k.

Two things pin the reading down beyond the monotonicity:

- level 0 of variant 0 alone reproduces the authored bounding box **exactly on
  302 of 434 meshes**, against 231 when all five slots are drawn together;
- 28 nodes carry only the fifth slot of their block, so level 0 falls back to
  the coarsest slot present rather than drawing nothing. No node lacks
  geometry in variant 0 but has some in a later variant.

What the later variants are is not established. `fr_b_brige`'s `o02` node has
identical counts in variants 0 and 1 (12, 6, 2 both times), which reads like a
damage state.

### The stream 2 header is the model's authored extent

The 140 bytes before the first slot are 35 floats:

```
float32  box corner[8][3]        the model's bounding box, in posed space
float32  sphere centre[3], radius
float32  axis low[3], axis high[3], radius
```

The last seven are a bounding **cylinder**, and its radius is exact: 11.6771
on `R_B_02` is `sqrt(8.791² + 7.687²)`, 28.3412 on `fr_b_brige` is
`sqrt(25.6² + 12.16²)`, 66.927 on `fr_b_bunker` is `47.325 × sqrt(2)`.

The box is the useful part, because **it is stated in posed space**. That
makes it the oracle that decides whether a pose reading is right: with poses
applied the drawn geometry reaches it on 305 of 434 meshes, without them on
237.

### Node poses

Stream 8 is an array of 24-byte keys:

```
float32  translation[3]
float32  time, in frames
int16    rotation[4]      w, x, y, z over 32767
```

**34038 of the 34049 keys the game ships are unit quaternions** to within
0.1%; the other eleven are all zero or, in one case, unnormalised, and are
repaired on read. The game is left-handed — Z up, DirectX — so the matrix it
builds from a quaternion is the transpose of the right-handed one, and the
same four numbers denote the **conjugate** rotation here. Reading them
straight through instead costs a factor of ten in the box error (median 1.48%
of the model diagonal against 0.10%).

A node's pose composes down the parent chain: the parent's rotation turns the
child's translation, translations sum, rotations multiply. `fr_l_gener`'s four
corner pylons sit at (±20.74, ±20.74, 22.58) once composed, and on top of each
other without it.

Stream 19 is the **frame map**: `link_count` entries per animated node, each an
index into stream 8, starting at the node's `anim_start`. A node's
`fallback_key` is the **last** frame of its own run — true on all 817 animated
nodes — so using it as a rest pose leaves a turret swung round to wherever its
animation ended. Frame 0 of the run is the rest pose, and it keeps a model
inside its own authored box on 106 of 157 animated meshes against 81 for the
fallback.

### The ground datum: a model's own z = 0

A mission places an object by putting model **z = 0** at the placement height,
and that is all there is to it — no offset. Measured across the 864 placed
objects in the shipped missions, against the terrain height under each:

| kind | origin − terrain | lowest exterior vertex − terrain |
|---|---|---|
| building | **0.00** | −1.16 |
| unit | +1.54 | **−0.08** |
| vegetation | +2.25 | −2.79 |
| rock | +7.80 | −0.09 |

A building's origin lands on the ground; a unit's mission z is set so its
wheels or feet do. Both are the same rule seen from two ends, and both are
tight: 64% of units put their lowest vertex within one unit of the terrain.

This only reads that way **once node poses are applied**. Before that a
building's parts pile up on their own origins, its geometry comes out roughly
symmetric about z = 0, and it looks half-buried — which is what earlier drafts
of this document described, and why they recommended resting a model on
`-min(z)`. That heuristic lifts `fr_b_bunker` 23 units into the air.

Buildings whose exterior does go below z = 0 are the ones that should: bunkers
and towers are dug in, and a bridge spans a canyon. The ones that do not are
exactly the ones you would expect — `fr_b_plant` −0.00, `fr_b_store` 0.00,
`fr_l_angar` −0.00, and the turrets in `turrets.rlb` at +0.08.

### Units are authored at the same scale as everything else

An earlier draft recorded that chassis meshes were about 1/20 the scale of
buildings, from `r_h_02` fitting in a 0.7 × 0.8 × 1.26 box. That was measured
before poses were applied and is wrong: posed, `r_h_02` spans 1.5 units
vertically, `R_B_02` is 17.6 × 15.4 × 3.9, and `R_B_08` is 29.7 units tall,
against buildings of 40 to 215. The placement statistics above settle it
independently — if units were twenty times too small their bases would not
land on the terrain at all.

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

Vegetation and rock are the two kinds the ground datum fits least well:
`s_tree_59` spans z −127.1 to +2.1, almost entirely *below* its origin, and
the missions place it 125 units above the ground, which compensates almost
exactly. Read as "the mission z is the model origin" that is consistent — the
tree is authored hanging below its origin and placed high enough to make up
for it — but it means a mis-set placement is invisible in the data, and it is
why vegetation's residuals are the widest of the four kinds.
