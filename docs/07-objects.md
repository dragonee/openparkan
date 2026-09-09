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

### `.ctl` is a movement controller, not an animation one

The `.ctl` slot was the last one never opened, and the obvious place to look
for the pose a parked unit stands in. It is the wrong place.

`Control.dll` exports `LoadControlSystem`, which allocates a 0x668- or
0x670-byte object; the interface the rest of the engine drives it through is
`IControl`, and `Terrain.dll` names its methods in full when a call fails:
`SetTangAccel`, `SetNormSpeed`, `SetTangSpeed`, `SetWorldSpeed`,
`SetWorldAccel`, `SetNormAngle`, `SetStrafeAngle`, `MakeMovementCorrection`,
`SetControlCalculationMode`. Speeds, accelerations and angles — steering.

The file is that object written out. Its constructor fills four `float32[3]`
slots with ∓FLT_MAX as "no limit", and those exact bit patterns
(`ff ff 7f ff` / `ff ff 7f 7f`) appear in the same order in every controller.
The rest of the fixed header is the same vocabulary: 6.2831 (2π) and 1.5708
(π/2) as angle limits, 0.1, 0.5, 1.0, 2.5, 250, 500, 1000, 10000.

Two measurements say it is not per-node data. A controller's size correlates
**+0.97** with its own leading count field and only **+0.40** with the number
of nodes in the mesh it belongs to, over the 542 records that have both. And
the largest controllers belong to the animals — `a_a_l3.ctl` is 222496 bytes
for a 13-node model — which is behaviour, not geometry. Its body is a run of
156-byte records carrying consecutive intervals (22.875, 23.75, 24.625 …) and
∓ bound triples.

The remaining question the `.ctl` was carrying is
[still open](06-open-questions.md), but it is a question about how a unit
*moves*, not about how it stands.

### `.bas` is a building's ground plan

30 members of `fortif.rlb`, and every one is a run of blocks:

```
int32    1            a marker; constant on all 60 rings
int32    count        corners
float32  x, y, z  x (count + 1)    the ring, closed by repeating the first
int32    ...      x (2 * count)    two per corner; written only when another
                                   ring follows, so the last ring ends here
```

**All 30 parse exactly and all hold exactly two rings**, of 3 to 26 corners,
and **all 60 wind anticlockwise**. The z is constant across a ring on 46 of
the 60.

The two rings are the building and a clearance around it. The inner one traces
the model — its XY extent *is* the mesh's own bounding box on 18 of the 30,
`fr_b_bunker` at −47.3..47.3 on both axes, `fr_b_inst` at −55.2..55.2 and
−47.8..63.6 — and the outer one stands off from it, 1.32 to 2.35 times the
area with a median of 1.61. The exceptions are the bridges, whose model is
half a span: `fr_b_brige`'s ring covers y −37.1..0.0 of a mesh that runs to
119.1, which is the half that touches the ground.

Placed and turned by the mission's angle it lands where the building does.
Over the **167 placed buildings** that have one, the terrain under the outline
spans a median 2.24 units from its lowest sample to its highest — buildings
sit on flat ground — and the placement height is a median **0.00** above their
mean. That makes it the one independent check on object *placement* the data
offers, which is why the viewer draws it through the same transform as the
model rather than a baked one: an outline that agreed with the model no matter
what would test nothing. The `Footprints` toggle shows them.

The two `int32` per corner are not read.

### `.ndp` is a damage table, one record per node

An `int32` count and then **76 bytes per record**. All 542 shipped members are
exactly `4 + n * 76` bytes, and 541 of them have `n` equal to the node count of
the mesh they belong to.

```
int32    flags        0 throughout, except 1 on scenery and 112 on projectiles
float32  durability   1000000 where the node cannot be destroyed
float32  ...          unresolved; 1000 on 549 records, then 0, 10, 1, 300, 500
char[32] archive      the explosion's library
char[32] member       the explosion, a .exp
```

Which float is the hit points is settled only by weight of evidence: the first
is the one that scales with the size of the part, correlating **+0.56** with
the node's volume in log space against **+0.19** for the second.

2203 records name an explosion, and the names say plainly what the table is
for: `explode_tree.exp` and `explode_leaf.exp` on scenery, `explode_frt_b.exp`
and `explode_frt_m.exp` on fortifications, `explode_rbr_l.exp` on large robots,
`selfexp_anl_01b.exp` on animals. Parkan lets you shoot a building apart piece
by piece; this is the table that says what each piece costs and what it looks
like going up. The `.exp` files themselves are still
[unread](../TODO.md) — `openparkan.objects.parse_damage` gets you as far as
their names.

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

**Mounting makes the part's root node take the socket's pose**, so the
transform that places a part is `socket ∘ root⁻¹`. Two facts make that read
cleanly. Every mounted part's root node sits at its own origin — a translation
of exactly zero on all 1414 attachments — so the socket's position and its
rotation are independent contributions. And a socket usually carries the same
rotation as the root of the part that plugs into it, on 1306 of the 1414, so
`socket ∘ root⁻¹` reduces to the socket's position there and composing the two
naively would turn the part twice.

The 108 sockets that say something the part does not are the interesting ones,
and they are not noise. **83 of them are exactly 180°, and every single one is
on a chassis whose own name says so**: `R_L_02` "Small Flying Chs (S-2f)",
`R_T_02` "Tiny Helicopter Chs (T-2)", `R_B_02` "Large Flying Chs (L-2f)",
`R_M_02` "Medium Flying Chs (M-2f)". Not one tracked or walking chassis is
among them. An aircraft's turret hangs under the belly, upside down, and the
socket is where that is written down.

The geometry agrees. On a tracked chassis the turret socket sits at the top of
the hull — `R_L_01` puts `Base_TL` at z 0.12 under a roof at 0.20, `R_B_01` at
1.28 under 1.48. On a flying one it sits at the *bottom*: `R_L_02` at −0.43
against a belly at −0.53, `R_B_02` at −2.00 against −2.08. Taking only the
socket's position buries the turret whole inside the hull — on `R_M_02` all
1.38 of its height overlaps — while the socket's own pose hangs it below with
**zero** overlap, its top at −0.84 meeting the belly at −0.85.

Across the 404 distinct `(chassis, socket, part)` mounts, applying the
socket's rotation lowers a part's overlap with its hull on 22 and **raises it
on none**; the other 382 are unchanged, bit for bit, because the two rotations
agree there. A bunker's turret still lands at z 16.49 on a bunker whose roof
is at 16.49.

The remaining 25 disagreements are 120° about (1, 1, 1) — a cyclic axis
permutation — and two at 126°, all of them missile packs and shell clips on
`e_gun_bl_17` "Large Winged SSM" and its kin. They are not understood, but
they are no worse under this rule than the last.

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

### Flag bit 5 marks a collision hull, which must not be drawn

28 nodes across the shipped archives carry flag bit `0x0020`, and **every one
of them is named `CP_m1o1` (19) or `BTCP_m1o1` (9)**. Nothing else carries the
bit. Six further `CP_m1o1` nodes do not, and all six are empty — the bit goes
on the hulls that have geometry.

A hull is a crude oversized box around the part it belongs to, and drawing it
is conspicuous. On `o_bnt_la_01` the model's real geometry tops out at
z = 1.60, exactly the top of the authored box; its `CP_m1o1` reaches 3.55 —
more than twice the building's height, wrapped around it as a translucent
slab. On the twelve worst meshes the model's *bottom* matches the box's bottom
to 0.00 and only the top overshoots, which is the signature of one extra node
rather than a wrong pose.

The box settles it. Level 0 with poses applied fits inside the extent the file
itself states on **434 of 434** models once these nodes are skipped, against
**422** while they are drawn — and on the animated meshes alone, 157 of 157
against 145. Twelve models is also exactly the set that no choice of animation
frame could fix, which is how the hulls were found: the residual was never a
pose problem.

`ObjectMesh.select()` and the viewer both skip them. 18402 triangles.

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

### But only level 0 is in the model's own frame

The ladder is real by triangle count — **137445, 34843, 14033, 5039** across
the four levels, over 434, 283, 282 and 197 meshes, and then slot 4 goes back
*up* to 13763 over only 103, which is the other half of the argument that the
fifth is not a level.

What does not hold is where the coarse levels sit. Level 0 fits the model's
own authored box on **434 of 434** meshes; level 1 on **129 of 283**, level 2
on 124 of 282, level 3 on 66 of 197, with overruns up to half the model's
size. It is not a posing problem: the nodes that own the coarse slots carry
identity poses, so posed and unposed positions give the same answer on all
four.

`o_bnt_rdr_l_01` shows what is going on. Its level 0 runs z 0.1 to 2.3 — a
2.2-unit mast resting on the ground — and its level 1 runs −1.1 to 1.1: the
same height, **centred on the origin instead of standing on it**. 85 of the
283 level-1 slots are centred that way where level 0 is not. But that is not
the whole story either: only 134 of 283 match level 0's extent to within 5%,
and the mismatch grows with the level.

So the coarse slots are real geometry in a frame that has not been
established, and drawing them would put objects in the wrong place on more
than half the models. The reader parses all fifteen and `slots_for_lod` takes
a level; the viewer asks for 0 only.

### The later blocks are damage states

1479 nodes fill block 0, **135 fill block 1 and 15 fill block 2**, and no node
ever fills a later block without the earlier ones (1790 of 1790). They are
concentrated where you would expect: 97 of the 135 are in `fortif.rlb`, the
buildings.

A later block is the same part with pieces gone. `s_tree_0_04`'s crown drops
from 212 triangles topping out at z 22.12 to 104 at 10.39. `fr_b_bunker`'s
`o01` goes 396 → 282 and 10.59 → 7.22. Where a third block exists the sequence
continues: `fr_l_gener`'s four pylons run 88 / 66 / 14 triangles at z 22.70 /
9.31 / **−16.88**, the last sunk below the ground. Triangle counts fall on 121
of the 145 nodes that have both, and the materials are identical on all of
them — the damage is modelled, not textured.

The `.ndp` damage table settles which direction the sequence runs. **Every one
of the 145 nodes that carries a second block names an explosion**, and the
names are not ambiguous: `s_tree_0_04`'s tree node has 3000 durability and
`static.rlb/explode_tree.exp`; `s_tree_0_06`'s three leaf nodes have 1000 and
`explode_leaf.exp`. A tree is not built, so this is destruction, not
construction.

The geometry is authored separately rather than derived: a later block's
vertices are wholly disjoint from block 0's on all 135, never a subset. A
renderer that draws intact scenery wants block 0 and nothing else, which is
what `slots_for_lod` takes by default; nothing is missing from the picture.

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
index into stream 8, starting at the node's `anim_start`. The runs are laid out
node-major — consecutive animated nodes' `anim_start` differ by exactly
`link_count` — so frame *f* of a node is `anim_start + f`. A node's
`fallback_key` is the **last** frame of its own run — true on all 817 animated
nodes — so using it as a rest pose leaves a turret swung round to wherever its
animation ended.

Frame 0 of the run is the rest pose, and once the
[collision hulls](#flag-bit-5-marks-a-collision-hull-which-must-not-be-drawn)
are skipped it puts **157 of 157** animated meshes inside their own authored
box. So does the fallback key, and so does every other frame: the box does not
discriminate between frames, because an animation stays inside it. What the
box does discriminate is whether the hulls are drawn — 145 of 157 with them —
which is how the twelve stragglers turned out not to be a pose question. The
earlier "106 of 157 against 81 for the fallback" was the same measurement taken
over all fifteen slot indices, before the slot index was understood; it was
comparing two piles of superimposed levels of detail.

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

A batch names an index into the model's wear; the wear names a material; the
material names a texture:

```
mesh stream 13 batch -> .wea entry -> Material.lib MAT0 -> Textures.lib Texm
```

15053 of 15138 batches reach a real texture that way.

A `MAT0` record opens with two `uint16` — the number of texture entries and
the number of **layers** — and the count divides by the layers on all 905
records. The quotient is a frame count: `WATER_M` is one layer of ten frames
(`WATER0.0` .. `WATER9.0`) and `WATER_BOT` is two layers of one (`L20.0` and
its mask `L20M.0`). 860 materials have a single layer, 43 have two, two have
eight; **animation is much the commoner reason for a material to hold several
textures**, which an earlier draft of this document had backwards.

Entries start 12 bytes in on a 40-byte stride, with the texture name at +20
and a three-byte RGB **diffuse colour** at +6, behind a constant 100 that
holds on 904 of the 905 records. The colour is not decoration: `WATER`'s
texture is a neutral grey ripple and all of the blue is in its `#4d6aff`, and
lava is a dull pattern tinted `#b41e00`. 760 materials carry a colour other
than white.

An entry runs **34** bytes, not 40 — a stride that used to be wrong, which is
why the names were read by pattern and why eight materials looked like they
named textures nobody shipped. They do not: read by offset, all 905 name a
texture that is in `Textures.lib`. See `openparkan/materials.py`.

### Baked lighting

`lightmap.lib` is 25 `Texm` pages named after buildings, 128 or 256 pixels
square, RGB565. A `.wea` names one in a second keyword section:

```
39
0 B_PG4
...
38 R_NP13

LIGHTMAPS
1
0 fr_l_bunker_00.0
```

and **mesh stream 18 is the UV set that addresses it**. The correlation is
total: of the 435 object meshes, 21 carry both a `LIGHTMAPS` section and
stream 18, and **none carries one without the other**.

The UVs are the same `uint16` as stream 5 but over **1024**, not 256, because
they address one atlas page and never leave 0..1. Every one of the 21 tops out
at exactly `round((1 - 0.5 / width) * 1024)` for the width of its own page --
1022 for a 256-pixel lightmap, 1020 for a 128 -- which is the half-texel inset
an atlas is authored with, and which pins the divisor and the pairing at the
same time.

The pages are not soft grey shading: they are dense mosaics of per-triangle
patches in saturated greens, oranges and purples, which is what a building
lit from inside by coloured lamps bakes down to.

### The batch material's high byte marks the lit batches

A draw batch's material word carries the material index in its low byte and
either `0x00` or `0xFF` in its high byte -- 972 batches against 14181, an open
question in earlier drafts. It says whether the batch takes the model's
lightmap:

- the **21** meshes with a `0x00` batch are exactly the 21 with a lightmap;
- every vertex a `0x00` batch reaches carries a non-zero lightmap UV,
  **51324 of 51324**, against 1268 of 85347 under `0xFF`.

So a building's lit surfaces and its unlit ones are separate batches, and a
renderer binds the lightmap per batch rather than per model.

### Winding is consistent, so culling is safe

On **434 of the 435** object meshes, over 95% of triangles wind the same way
as their own vertex normals (median 100%, worst mesh 92.9%); the terrain is at
99.4%. Front-face culling is therefore correct for closed shells. Foliage
still needs two-sided drawing -- a tree is a pair of crossed planes, and half
of each plane faces away.

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
