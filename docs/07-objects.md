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
int32    triangle x count          where the corner was taken off the model
int32    corner   x count          which of that triangle's three it is
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
spans a median 1.93 units from its lowest sample to its highest — buildings
sit on flat ground — and the placement height is a median **0.04** above their
mean. That makes it the one independent check on object *placement* the data
offers, which is why the viewer draws it through the same transform as the
model rather than a baked one: an outline that agreed with the model no matter
what would test nothing. The `Footprints` toggle shows them.

### The block after a ring is where its corners came from

It looked like a header that would not divide evenly. It is a
**back-reference**: `count` int32 triangle indices, then `count` int32
corners, so a ring point is `mesh.triangles[triangle][corner]` of the
building's own mesh. The author traced the outline **on the model**.

It resolves exactly on **15 of the 30** records — 154 points, every one of
them — and on the other 14 no ring point sits on a mesh vertex at all, so
those outlines were traced on geometry the shipped mesh no longer carries.
(`fr_b_ruin` scores one hit out of 18, which is a coincidence rather than a
partial match.)

That also settles why only one of the two rings carries it. **Ring 0 is the
inner one on all 30** — the outline taken off the building — and it is the one
with the back-reference; the outer ring is a clearance the author drew around
the model rather than off it, and has none. What looked like "written only
when another ring follows" was "written only on the ring that was traced".

### `.ndp` is a damage table, one record per node

An `int32` count and then **76 bytes per record**. All 542 shipped members are
exactly `4 + n * 76` bytes, and 541 of them have `n` equal to the node count of
the mesh they belong to.

```
int32    flags        0 on most; 1 on scenery, 112 on projectiles, 32/64 on running gear
float32  durability   hit points; 1000000 where the node cannot be destroyed
float32  density      times the node's volume, its mass; 104 distinct values
char[32] archive      the explosion's library
char[32] member       the explosion, a .exp
```

**The first float is the node's hit points** — *read*. `Control.dll` builds one
node record per `.ndp` record (`0x1000b240`) and at reset sets each node's life
to that float times the object's volume scale and the difficulty's level
ratio (`0x1000f940`, [26-damage.md](26-damage.md#hit-points--read-and-measured));
damage lowers it, clamped at 0 (`0x10010f30`), and a node at 0 is marked
destroyed — and takes the whole object with it if it is node 0 or carries flag
bit 1. A component's powers are scaled by its node's `life / max`
([23-economy.md](23-economy.md#efficiency-is-a-buildings-size)). The size
correlation that stood here alone before agrees: the first float correlates
**+0.56** with the node's volume in log space, against **+0.19** for the
second.

**The second float times the node's volume is its mass** — *read*,
`0x1000fac0` ([24-motion.md](24-motion.md#load--read-and-measured)), which makes
it a density by the look of it. That fits what was measured of it before: it
*falls* as a node grows.

**Flags 0x20 and 0x40 mark a machine's left and right running gear** —
*measured*, 28 nodes each across 15 chassis and animal tables, every 0x20 node
at x < 0 and every 0x40 node at x > 0 — and the drive averages each side's life
([24-motion.md](24-motion.md#running-gear-legs-wheels-and-tracks-by-side--read-and-measured)).

2203 records name an explosion, and the names say plainly what the table is
for: `explode_tree.exp` and `explode_leaf.exp` on scenery, `explode_frt_b.exp`
and `explode_frt_m.exp` on fortifications, `explode_rbr_l.exp` on large robots,
`selfexp_anl_01b.exp` on animals. Parkan lets you shoot a building apart piece
by piece; this is the table that says what each piece costs and what it looks
like going up. What an `.exp` does to whatever it hits is in
[26-damage.md](26-damage.md).

## UNITS/**/*.dat — unit and building assemblies

458 files, all the same shape:

```
uint32   magic, always 0x0000F0F1
uint32   class word: the unit's Type (0x1008000 a warbot ... 0x8000xxxx a building)
components × N, 112 bytes each:
    char[32]  archive name, always "objects.rlb"
    char[32]  record name
    uint32    flags, 1 throughout
    int32     attachment: a node of the parent's mesh for a turret or gun, the
              index of the parent controller's slot for an internal part or
              clip; -1 on the root
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

The class word is the unit's **Type**, and a robot's turret decides it
([30-turrets.md](30-turrets.md#the-turret-decides-what-the-unit-is--measured-and-read)).
The part classes are not strict: **26 guns are written with class 2**, not 4
(11 in `AI`, 8 in `BATTLE`, 7 in `AUTO`), so find a unit's guns by their
`e_gun_` prefix.

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
`e_gun_bl_17` "Large Winged SSM" and its kin.

**They do not matter, because the engine throws the root away** — *read*.
Mounting an external part merges its mesh into the unit's (`AniMesh.dll`
`0x1000a460`, from the part attach at `0x100038db`): it takes one node fewer
than the part has (`0x1000a719`), skipping node 0, and hangs every node whose
parent was node 0 on the socket instead (`0x1000a7dd`). The root's own pose,
rotation and all, never reaches the picture, so wherever socket and root
disagree the socket wins — which is `socket ∘ root⁻¹` applied to the root's
children, the rule above. Nor could the root hide geometry: **1417 of the 1417**
external parts mounted in the shipped assemblies have a root node with no slot
at all (*measured*). Why those 25 roots were authored turned is a question
about the artists' tool, not about the game.

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
| 5 | 4 | vertex | UV, `uint16` over 1024 ([below](#how-a-material-reaches-the-device--read-and-measured)) |
| 6 | 6 | face | triangle, three `uint16` indices — **relative to the batch** |
| 9 | 32 | sub-object | sub-object name |
| 1 | 38 | node | flags, parent, `slot_index[lod * 5 + group]` |
| 7 | 16 | face | face record: flags, edge neighbours, normal, winged-edge link |
| 13 | 20 | batch | draw batch: material, index range, vertex range |
| 17 | 20 | node | building interior path graph — see below |
| 10 | — | node | a label a node: `uint32` length, the bytes and a NUL; empty throughout `static.rlb`, a socket's part prefix on units ([38-designs.md](38-designs.md#sockets-carry-the-part-prefix-they-take--measured-and-read)) |
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

**But the flag is exactly what a cutaway needs.** 21 of the 435 meshes carry
internal nodes, and the inside is most of the model: **29460 triangles of
interior against 10725 of shell**, with `fr_b_ruin` alone at 2478 against 450.
Hiding the `o*` nodes opens a building up — the bunker turns out to be an
octagonal chamber with corridors running off it — and the cross-check holds:
**all 21 of the meshes with an interior also carry a path graph** to walk it.
The 8 that carry a graph without internal nodes are the bridges, ruins and the
Outpost (`fr_l_angar`), which you cross rather than enter.

So `select(interior=True)` is a cutaway, `select()` is the building, and
`select(interior=False)` is a shell that is far too short to be one.

### Flag bit 5 marks the cockpit, which only the unit's own view draws

28 nodes across the shipped archives carry flag bit `0x0020`, and **every one
of them is named `CP_m1o1` (19) or `BTCP_m1o1` (9)**. Nothing else carries the
bit. Six further `CP_m1o1` nodes do not, and all six are empty — the bit goes
on the ones that have geometry. All 28 are in `turrets.rlb`.

They were written up here as collision hulls. They are the **cockpit**: the
geometry a unit's first-person view sees of the unit it sits in, and nobody
else sees at all — see [the fifth slot](#the-fifth-slot-is-what-the-units-own-view-draws).
The turret's `CameraCenter` control point sits on one on **54 of the 54**
turret records whose mesh has one (*measured*).

Drawn in a survey, a cockpit is a crude oversized box around the part it
belongs to, and it is conspicuous. On `o_bnt_la_01` the model's real geometry tops out at
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

`ObjectMesh.select()` and the viewer both skip them, as a third-person view
of the game does. 18402 triangles.

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
float32  area            what armour is weighed by (28-chassis.md)
float32  volume          the bounding box's; density x this is the node's mass
uint32   x3
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
- 28 nodes carry only the fifth slot of their block. No node lacks geometry
  in variant 0 but has some in a later variant.

### The fifth slot is what the unit's own view draws

Those 28 nodes are **exactly the 28 flagged `0x20`** — every one named `CP_*`
or `BTCP_*` — so a cockpit keeps its geometry in the fifth slot and nowhere
else. On the 288 ordinary nodes that carry both a level 0 and a fifth slot,
the fifth is **always a separate slot**, never one of the node's levels reused:
a same-sized copy of level 0 on 141, a coarser shape on 137 (closest to level
1 on 83 of those), and finer on 10 (*measured*). The meshes that carry fifth
slots are 54 guns, 28 turrets, 11 chassis, 5 parts and 5 buildings.

**What reads it** — *read*:

- **The view registers.** A turret's camera component creates the unit's
  first-person view (`Control.dll:0x100238b0`, `World3D.dll!CreateObject` type
  5) and hands that view to the unit's mesh through IAnimation slot 33
  (`0x1002399a`), a list at mesh `+0x1c8`; it takes it back through slot 35
  when the view goes (`0x100239cd`, and the component's destructor
  `0x1002349a`). The component's mesh is the whole unit's: the control system
  asks its aggregate for IAnimation (`Control.dll:0x1000791d`) and hands that
  to every component (`0x100079e2`).
- **The mesh asks.** Drawing, the mesh asks slot 34 whether the view drawing it
  is in that list (`AniMesh.dll:0x10014bdb`). If it is, the level is **4**
  (`0x10014be5`) instead of the one `CShade` picks by distance.
- **Every node draws slot `variant × 5 + 4`** (`0x100124d0`, no fallback), so a
  node with no fifth slot is **not drawn at all** in that view. A node flagged
  `0x20` is handed to `CShade`'s mesh draw with mode 2 and any other with mode
  1 (`0x10014e9a`), which that draw files under layers 10 and 9 where an
  ordinary surface gets 0, or 5 when see-through (`Terrain.dll:0x1004553b`),
  and in place of `CShade::SetClipStateForJoint` the mesh calls `CShade` slot
  15 with 1 (`0x10014f85`). What those layers are, and what slot 15 does, is
  [below](#a-layer-is-a-pass-and-the-cockpits-two-are-a-frustum-of-their-own--read).
- The per-part draw behind the mesh's interface `0x20` slot 6
  (`AniMesh.dll:0x100101d0`) takes the level as an argument and applies the
  same two modes at level 4 (mode 0 for an agent of kind 3).

So from inside, a unit is its fifth slots: the cockpit shell, and whatever of
the hull and guns the artist gave a fifth slot to be seen from there. A round's
hit test takes level 0 (`AniMesh.dll:0x10010c33` → `0x100124d0`), never this,
so a cockpit is never struck
([The hit test](26-damage.md#the-hit-test--read-and-measured)).

This also corrects why level 0 falls back to the fifth slot in this library:
the only nodes it happens to are cockpits, which a survey does not draw.
`ObjectMesh.select` and the viewer skip them; `Subobject.cockpit_slot` names
the slot.

### A layer is a pass, and the cockpit's two are a frustum of their own — *read*

A draw item is filed by a **group** and a **layer**, and the layer is the index
of the pass it joins inside that group's pass list
([10-sky.md](10-sky.md#the-dome)). The mesh draw picks both together
(`0x1004552a`–`0x100455bc`): mode bit 0 gives group 1, layer 9; mode bit 1
group 1, layer 10; and otherwise the see-through test gives group 1, layer 5 or
group 0, layer 0. Every pass overrides the camera's near plane, far plane and
viewport z range for its own length, so what the four choices mean is:

| what | group | layer | near | far | viewport z |
|---|---:|---:|---:|---:|---|
| an ordinary opaque surface | 0 | 0 | 0.5 | 700 | 0.1 – 0.99 |
| a see-through one | 1 | 5 | 0.5 | 700 | 0.1 – 0.99 |
| a fifth slot, ordinary node | 1 | 9 | **0.05** | **10** | **0.0 – 0.1** |
| a fifth slot, the cockpit | 1 | 10 | **0.05** | **10** | **0.0 – 0.1** |

So the fifth slots are **not** drawn with the scene. They get a near frustum of
their own — 0.05 to 10 units, against the world's 0.5 to 700 — mapped to the
front tenth of the depth buffer, while the world owns 0.1 to 0.99 and the sky
is pinned at 1.0. A cockpit shell closer to the eye than the world's own 0.5
near plane is neither clipped by it nor ever occluded by anything in the
world, and the two first-person layers depth-test against each other alone.
Group 1 runs after group 0, so they are drawn over the finished scene.

**`CShade` slot 15 is a one-byte setter** (`0x100437a0`, `ret 8`; the vtable
`0x1009b17c` is installed at `0x10041f94`): it writes its argument into
`CShade+0xca8`. The mesh draw reads that byte at `0x100455fe` and, **when it is
zero**, sets bit 0 of the draw item's flag word (`0x10045608`). So
`AniMesh`'s "slot 15 with 1" is exactly *clear bit 0 on this draw*, and it
stands in for `CShade::SetClipStateForJoint` (slot 14, `0x10043680`), which the
ordinary path calls to hand the joint's clip planes to the material manager.
What bit 0 then does is **not established**: the item renderer masks it off
again before passing the flags to the device (`and edx, 0xfffffffe`,
`0x1003056a`), an item copied for another draw has it cleared with bit `0x400`
(`0x1002c6ce`), and no test of it was found — against the control that the
neighbouring bits do turn up at once, bit 3 at `0x10030043`, bit 4 at
`0x1002fe3d` and bit `0x400` at `0x1003ecb9`.

### Every level is a simplification in place

The ladder is real by triangle count — **137445, 34843, 14033 and 5039**
across the four levels, over 434, 283, 282 and 197 meshes — and then slot 4
goes back *up* to 13763 over only 103, which is the other half of the argument
that the fifth is not a level.

The coarse levels sit exactly where level 0 sits. Posed, each level's centre
is within a median **0.000 to 0.011** of the model's size of level 0's, and
its extent within **0.001 to 0.037**. Against the model's own authored box:
level 0 fits on 434 of 434, level 1 on 257 of 283, level 2 on 232 of 282,
level 3 on 134 of 197 and slot 4 on 101 of 103. The stragglers are trees and
stones, where a simplification legitimately changes the silhouette.

That took two attempts, and the first one was wrong in a way worth recording.
`posed_positions(lod)` used to pose only the vertices *that level* reached,
which is fine if you draw the level you asked for — and a trap if you do not.
Reading levels 1 to 3 back out of `posed_positions(0)` returns them in raw
node-local space, which makes them look as though they live in a different
frame: level 1 appeared to fit the authored box on only 129 of 283, and
`o_bnt_rdr_l_01` appeared to have its level 1 centred on the origin where its
level 0 rests on the ground. Both were artefacts of the unposed read.

`posed_positions` now poses **every** slot, which is unambiguous: **no vertex
of any of the 435 meshes is reached by two slots whose nodes pose it
differently**, so there is never a choice to get wrong.

Each slot also states its own axis-aligned box, and that is what pinned the
decode down: a slot's declared box matches the vertices decoded for it exactly
on **1451, 1011, 952, 535 and 288 slots** — every slot of every level, with
the box stated in node-local space.

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

**Which block a damaged node draws** (*read*). The mesh draw picks
`slot_index[stage × 5 + level]`, the stage being the value the node's life
system handed the mesh (`AniMesh.dll:0x100124d0` with variant −1). The life
system counts a node's stages as its blocks in a row with a level-0 slot
(`0x10005840`, *measured*: 1 on 1598 nodes, 2 on 130, 3 on 15, none on 597)
and hands the mesh its stage held below that count. At its last stage, a
life of 0, the node is hidden from the draw unless its flags word carries
`0x100`, which only the 60 shell nodes of the `bu_*` building records do. See
[26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured).

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
[cockpit nodes](#flag-bit-5-marks-the-cockpit-which-only-the-units-own-view-draws)
are skipped it puts **157 of 157** animated meshes inside their own authored
box. So does the fallback key, and so does every other frame: the box does not
discriminate between frames, because an animation stays inside it. What the
box does discriminate is whether the cockpits are drawn — 145 of 157 with
them — which is how the twelve stragglers turned out not to be a pose question. The
earlier "106 of 157 against 81 for the fallback" was the same measurement taken
over all fifteen slot indices, before the slot index was understood; it was
comparing two piles of superimposed levels of detail.

### Animation: stream 19 over stream 8

**157 of the 435 meshes carry one.** Stream 8 is the pose keys and stream 19
the frame map: `frame_count` indices per animated node, laid out consecutively
from that node's `anim_start`, one entry per frame. `frame_count` is the
archive entry's own second count.

Three things make it playable, and each is checked over every animated mesh:

- **It is rigid, one node to a vertex.** Every one of the 296379 vertices those
  meshes hold is reached by exactly one node, so a bone per node at a single
  weight plays the whole thing and there is nothing to blend.
  (`ObjectMesh.node_of_vertex`.)
- **A key's `time` is the frame at which its run first names it** — on all
  **33020** keys. The map repeats a key to hold it, so the times are what a
  player interpolates between rather than a per-frame table to step through.
  A turret's nine frames are six keys with holds in between; stepping them
  jumps 90° at a time.
- **An animated node's rest pose is its own first frame**, on all 817 of them.
  That is what lets a bind pose be taken from the rest pose and the animation
  begin exactly on it.

The rigs read as rigs. `R_H_02`'s ten nodes are `B_Dn` (the body) with
`LL_Up`, `LL_Dn`, `FL_Up`, `FL_Dn` down one leg and `LR_*`, `FR_*` down the
other, and **only the body's key translates** — every limb bone's translation
is the same in all 124 frames, which is what a jointed skeleton looks like.
Nine of the 34 rigs move their root more than a unit; `R_H_02`'s lunges 5.7
forward and comes back to zero, so the loop closes. A few of the creatures'
do not. None of that travel is drawn: the engine clears a root's translation
([below](#how-the-engine-plays-it--read)), and the controller moves the body
by the same distance instead.

#### How the engine plays it — *read*

`AniMesh.dll` poses a node at a fractional frame f (`0x10012880`):

- **The key** is the run's entry at round(f − 0.5).
- **Past the run**, on a node that is not animated, or where the entry is at
  or beyond the node's fallback key, the **fallback key alone** is the pose
  (`0x10012ba2`), with no interpolation at all. That branch matters: the
  fallback key is the last key of the node's *own* run, so the key after it in
  stream 8 belongs to the next node, and interpolating on into it sends the
  node anywhere. A controller's channel reaches those frames often — the frame
  map repeats the last key to hold it, and the channel's frame range runs to
  the end of that hold on 27 of the 32 building models any mission places.
- **Between keys** it takes that key or the next one in stream 8 when f equals
  its time. Otherwise it interpolates between them by time: a lerp of the
  translation and a slerp of the rotation. The key is below the fallback key
  here, so the next one is still the node's own.

A controller hands the mesh **two frames and a weight**:

- **The frames:** frame A and frame B, each a lerp across its state's pair.
- **The weight** w (`0x10012560`): frame A alone at w = 0 or when B is
  negative, frame B alone at w = 1 or when A is negative, and a slerp and lerp
  between the two otherwise, the short way round.

**The root turns but does not travel** (*read*). The pose walk
(`0x10008b30`) poses each node into a 4×4 whose column 3 is the translation:
`0x10012560` stores the key's translation there (`0x1000b8e0`, column 3), and
`0x1000b600` reads a column back the same way, as elements 3, 7 and 11.

- **The root.** For the first node, when it has no parent, the walk zeroes
  elements 3, 7 and 11 (`0x10008d88`) before it copies the matrix to the
  node's model matrix at `+0x60`. The rotation stays.
- **Every other node.** Its model matrix is its parent's times its own
  (`0x10009002`).
- **The world.** A node's world matrix at `+0x20` is the object's placement
  (`+0x164`) times its model matrix (`0x100090a2`).
- **Handing it out.** IAnimation slot 4 (`0x10005320`) runs the walk, then
  hands out the model matrix for mode 0, the world matrix for mode 2 and the
  identity for mode 1.

So a machine's body node sways in the picture but never carries it forward:
its travel is the stride the controller moves the body by
([24-motion.md](24-motion.md#playing-a-state--read-and-measured)). Everything
hung below the root turns with it, a mounted turret and its camera included
([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)).

**A node can have its own segment.**

- **Where it comes from:** a controller channel gives its node one
  (`0x10005500` bit 3).
- **What it holds:** frames A = −1 and B = the channel's first and last, at
  weight 1.
- **How it plays:** the node plays first + v × (last − first) for the
  channel's value v, whatever state the body is in
  ([13-control.md](13-control.md)).

`ObjectMesh.track(node)` returns the key per frame; `animated` says whether
there is anything to play.

### The ground datum: a model's own z = 0

A mission places an object by putting model **z = 0** at the placement height,
and that is all there is to it — no offset. Measured across the 864 placed
objects in the shipped missions, against the terrain height under each:

| kind | origin − terrain | lowest exterior vertex − terrain |
|---|---|---|
| building | **+0.03** | −0.68 |
| unit | +1.56 | **−0.00** |

A building's origin lands on the ground; a unit's mission z is set so its
wheels or feet do. Both are the same rule seen from two ends, and both are
tight: 75% of units put their lowest vertex within one unit of the terrain,
and the median unit is **0.03** off it.

Vegetation and rock are the two kinds this fits least well, for reasons the
data itself gives — see [Local origins](#local-origins) below. More than half
of their placements are scaled, and z = 0 lands on the placement height only
after the scale is applied.

The figures moved slightly when the terrain gained its levels of detail:
`height_at` samples the fine level alone now, and the placement checks
improved with it — buildings within two units of the ground went from 96 of
167 to **100**, and units within one from 194 of 296 to **223**.

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
uint32  flags             the batch word: 2 struck from behind, 8 a portal
uint16  material          low byte indexes the wear; high byte 0xFF or 0x00
uint16  node              0xFFFF, or on a portal the room beyond it
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

**The first dword is the batch word every face query tests, and the `+6`
halfword names a portal's room** (*read*, and *measured*). `AniMesh.dll`'s face
source hands a face's batch record back through interface `0x18` slot 3
(`0x100134d0`): its first dword as the answer's first word, which the
collision's push-out keeps as the face's batch word, and its `+6` halfword,
plus the part's first node, as the node `CBuilding::PortalDrawNotify` reads.
Over the install's **15153** batches the word carries 2 on 1477, 8 on **633**
and `0x200` on none; the 633 are `fortif.rlb`'s `DEFAULT`, `PORTAL_001` and
`PORTAL_004` batches — the doorway and portal quads a mover passes — and the
`+6` halfword is a node on exactly those, `0xFFFF` on the other 14520
([24-motion.md](24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).

**The word a draw sees is the file's, with `0x20` alone or-ed in** (*read*, and
*measured*). Slot 3 answers a module static (`AniMesh.dll:0x100270f8`) into which it
stores the stream's dword whole (`0x10013538`) and or-s `0x20` on a portal batch
(`0x10013647`), and the stream is the archive's own bytes in a view mapped read-only
(`niOpenResFileEx(…, 4)`, `0x1000a52e`; `Ngi32.dll:0x10011c5e`), so nothing can write it.
Over the 15153 batches the word takes 14 values over nine bits — 1 on 183, 2 on 1477, 4 on
112, 8 on 633, `0x10` on 66, `0x40` on 170, `0x100` on 2953, `0x2000` on 972 and `0x4000`
on 307 — and **`0x800`, under which the shade's mesh draw would lay an effect's light on a
batch as a disc, on none**. What lights a batch is its vertices: the shipped setting sends
every lit item to the shade's own lighter and never to the device material's builder
below ([11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).

### Stream 7 is the per-face record

The last stream the reader carried unread. **One 16-byte record per triangle
on all 435 meshes**, 241887 in all, and it is the object mesh's version of the
terrain's face record:

```
+0   uint16   flags
+2   uint16   neighbour across edge 0   (0xFFFF = none)
+4   uint16   neighbour across edge 1
+6   uint16   neighbour across edge 2
+8   int16    normal x, over 32767
+10  int16    normal y
+12  int16    normal z
+14  uint16   the winged-edge link in its low six bits
```

**The normal is exact.** Read as `int16` over 32767 it is unit length on all
241887 faces and points the same way as the cross product of the triangle on
241879 — the same encoding, to the constant, that the terrain's fields 10 to
12 use.

**The adjacency is exact too**, and it is checked the strong way rather than
by mutuality: of the 674206 in-range neighbours, **674200 share two vertex
positions** with the face that names them. Six do not, all in `s_stn_0_13`,
and 362 share all three, which is a pair of coincident triangles. 51455 of the
725661 slots are `0xFFFF`, the open edges of the model.

**The last word is the winged-edge link**, exactly as the terrain's field 13
is ([03-terrain.md](03-terrain.md#field-13-is-the-winged-edge-link)): three
2-bit codes, edge *e* at `(word >> 2e) & 3`, each the index of the matching
edge back in the neighbour, 3 where there is none. Here it is checked by
geometry rather than mutuality — the code names the neighbour's edge with the
same two vertex positions on **674206 of 674206** in-range neighbours, and
reads 3 on all **51455** open edges (*measured*). 240500 faces hold nothing
above the six bits; the other 1387 do, and the extra bits recur at the same
record positions across unrelated models — faces 120, 123 and 124 of a node's
run on most trees carry `0x28c0`, `0x8800` and `0x4dc0` above their link —
which is what an exporter's uncleared buffer looks like (*guess*).
`ObjectMesh.edge_twin` reads it.

**The flags word** takes six values — 0 on 233714 faces, then 2, 4, 16, 32 and
34 (*measured*):

| flag | faces | where |
|---|---|---|
| `0x02` | 6166 | **the walkable surfaces of the buildings you walk through**: on exactly the 29 `fortif.rlb` meshes with a path graph and nowhere else |
| `0x04` | 1355 | foliage (`TF2`, 707) and see-through glass and teleports in buildings; 1033 name a see-through material |
| `0x10` | 384 | **the broad faces of door leaves**, on 20 `fortif.rlb` buildings; 320 of them `R_NP13` |
| `0x20` | 274 | building glass (`B_COMP_3G`, 234) and the bridge's additive `B_A_BRIGE` |

A round passes through faces flagged 4 or 32 and strikes 2 and 16 — *read*,
`Control.dll:0x1001d9fa` ([26-damage.md](26-damage.md#the-hit-test--read-and-measured)).
The collision push-out drops faces flagged 2 unless the mover's collision flags
carry 8, and its door test takes 16 — *read*,
`Control.dll:0x1001dbce`, `AniMesh.dll:0x1000dbba`
([24-motion.md](24-motion.md#collision-between-objects--read)). So a floor does
not push a walker standing on it. The collision's triangle mask is 4 where a
round's is `0x24`, and the shipped bridges need `0x20` passed there too, which
is a stand-in and not a read
([24-motion.md](24-motion.md#the-cap-where-two-halves-meet--measured-and-a-stand-in)).

**Flag 2 is a walkable surface, and a chosen one** (*measured*). All 6166 faces
lie in a **level-0** slot — 4562 in the first variant's, 1480 in the second's,
124 in the third's — and nowhere else. Posed into model space the normal's z is
above the engine's own cos-80° ground threshold `0.173648` on **6100 of 6166**,
of which 4258 are within 10° of level; 66 lie at or below zero. So it covers
**ramps and stairs**, not just flat floor. 3306 sit on exterior nodes and 2860
on interior ones, and the meshes that carry most are `fr_b_plant` (453),
`fr_m_plant` (443), `fr_l_plant` (423) and `fr_l_bunker` (398). And it is a
**subset somebody chose**: against the 4562 flagged faces of the first variant's
level-0 slot, **1802 more faces of that same slot** point within 10° of up and
carry nothing at all.

**Flag 16 is the leaf's face, not the leaf** (*measured*), which sharpens the
door test [24-motion.md](24-motion.md#a-shot-opens-a-door--read-and-seen)
already reads. All 384 are vertical once posed (|normal z| < 0.1, 384 of 384).
They sit on 52 nodes, **every one interior**, 50 of them animated; the two that
are not are `fr_l_gener`'s `i20_m1o1` and `i30_m1o1` — the same generator
docs/24 already flags as the one mesh whose flagged triangles sit on other nodes
than its doors. Each mesh takes them from a single wear entry: `R_NP13` on 18
meshes (320 faces), `B_GEN_05` on `fr_b_ruin` (40) and `B_MTP_01` on
`fr_l_gener` (24). Grouped by shared vertices they are back-to-back pairs of
thin planar slabs with opposite normals — `fr_m_tower`'s at y = ∓0.72, each
7.66 × 6.34; `fr_b_store`'s at x = −0.60 and +0.61. Within a door node's level-0
slot only **the two broad sides** are flagged — 8 triangles of 20 on 34 of the
52 nodes, 4 of 12 on twelve small ones, 10 of 16 on four and 12 of 36 on two —
never the slab's rim.

**What reads them** — *read*, and narrowed. Three places in the install, all in
`AniMesh.dll`: the triangle visitor `0x10008120`, which loads the flags word and
tests it against its filter's required and excluded masks (`+4` and `+8`); the
push-out's test of a gathered face's `+0x44` against the filter's `+0x18` and
`+0x1c` (`0x1000dbfb`, `0x1000dc0e`); and the door test's required `0x10`
(`0x1000dbba`). The visitor has exactly two callers, `0x100106a4` and
`0x10010c91`, both inside the segment hit test's node visitors, and it is **not
exported** — `AniMesh.dll` exports `LoadAgent` and `LoadAniMesh` and nothing
else. (Control: the same direct-call and whole-section dword scan does find the
data references for `0x1000ce90`, `0x1000dfe0`, `0x100106d0` and `0x10010dc0` as
single vtable entries, so it finds pointers where there are any.)

Two narrowings follow. **The walk-face query does not read flag 2**: the face
test docs/24 already reads (interface `0x25` slot 2, interface `0x18` slot 7 —
`0x1000d0ef`, `0x10013fe0`, `0x10015ca0`) selects ground by the normal's z
against `0.173648` with **no triangle-flag test at all**, so the flagged floors
are chosen for the collision **push-out**, not for the ground search. And the
six-word filter constructor `Control.dll:0x10013f60` has four call sites —
`0x10013481`, `0x1001db67`, `0x1001dbb8`, `0x1001dc03` — the last three the
collision pair query already read, the first writing 0 into both triangle masks.

Still open: whether anything **outside `Control.dll`** hands a face query a mask
carrying 2 or 16. The round's filter is built inline rather than through the
constructor, so enumerating the constructor's callers is not a complete
enumeration of filters.

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

All 15138 batches reach a real texture that way.

A `MAT0` record is a **14-byte header, then the entries, then a table of
animation tracks over them** — and all 905 parse to the byte with nothing left
over:

```
+0   uint16   entry count
+2   uint16   track count      the engine refuses more than 20
+4   uint8    class            \
+5   uint8    unused            |  version-gated
+6   float32                    |
+10  uint32                    /
+14  entry[]  34 bytes each
     track[]  a uint32, a uint16 key count, then 6 bytes per key
```

The version those last four fields are gated on **is not in the record**. It
is the archive directory entry's second count field, and the parser
substitutes a default below each threshold: `0xFF` for the two bytes below
version 2, 1.0 for the float below 3, 0 for the dword below 4. Every shipped
record declares **version 6**, so all four are present and the header is 14
bytes on every one — which is what fixes where the entries start. That
default is also why `0xFF` in the class byte means *not set* rather than a
twelfth class: it is exactly what the parser writes when the record is too
old. Byte 5 is `0xFF` on all 905 and byte 4 on 376.

An entry is a **`D3DMATERIAL7` written as bytes**, and that is why it is 34
long:

```
+0   uint8[3] ambient  rgb      +3  uint8 ambient  alpha, per cent
+4   uint8[3] diffuse  rgb      +7  uint8 diffuse  alpha, over 255
+8   uint8[3] specular rgb      +11 uint8 specular alpha, over 255
+12  uint8[3] emissive rgb      +15 uint8 emissive alpha, over 255
+16  uint8    specular power
+17  int8     sub-image, -1 for the whole texture
+18  char[16] texture name
```

The parser multiplies the **ambient** alpha byte by 0.01 and the other fifteen
colour and alpha bytes by 1/255 (`World3D.dll:0x100046ba` against
`0x10004628`; an earlier draft had all four alphas per cent). **Not one of the
12572 alpha bytes exceeds 100** — a test no wrong stride and no wrong header
length survives. Only the ambient alpha ever reaches the device, as the
diffuse alpha (below), so the other three scales change nothing on screen. The stride used to be
read as 40 with the entries starting at 12, which is why the names were
extracted by pattern and why eight materials looked like they named textures
nobody shipped. They do not: read by offset, all 905 name a texture in
`Textures.lib`.

The diffuse colour is not decoration: `WATER`'s texture is a neutral grey
ripple and all of the blue is in its `#4d6aff`, and lava is a dull pattern
tinted `#b41e00`. 761 materials carry a diffuse other than white. The
**ambient alpha** is the only one that ever varies — 3138 of 3143 entries are
at 100, and the exception gives it away: `FIRESTORM` runs **0, 60, 80, 90,
95, 100** across its frames, which is a fade-in.

### How a material draws is in the archive directory

Nothing inside the record says how a material blends. The **directory entry**
does. Its first count field — where every other archive keeps an element
count — is a flags byte here, and it is the one `World3D.dll`'s loader
branches on: bit 1 into one field of the loaded material, bits 2 to 5 into
another, bit 0 into a local flag one record sets and bit 6 into one none does.

It takes five values, and they sort the library by how the material is drawn:

| flags | n | what it holds |
|---|---|---|
| 0 | 54 | opaque and lit. 52 of the 54 name a texture with **no alpha channel at all**; the other two name no texture |
| 2 | 417 | the ordinary lit skin — **3140 of the 3143** references from a model's wear land here, and 261 carry a specular colour |
| 4 | 219 | see-through: smoke, dust and most of the sky. 175 of the 219 carry a black diffuse, so they draw unlit |
| 5 | 1 | `ENV_STARS`, which is 4 with bit 0 as well |
| 8 | 214 | **additive** |

The additive reading is the firmest, and it comes from four directions at
once. **44 of the 46 materials the artists themselves named `*_add`** carry
flags 8, and so does every `JET*`, `SHOOT*`, `LASER_*` and `SPLASH*`. **210 of
the 214** carry a black diffuse, so the scene light never reaches them — which
is what you want for a glow. **Not one** carries a specular colour, against
261 of the 417 flags-2 skins. And the population is right: the materials an
effect's emitters name are **2519 at flags 8 and 980 at flags 4** — the
additive glows and the smoke — against 24 ordinary skins.

Who names what agrees everywhere else too. The terrain's layer tables are 196
at flags 0 against 74 elsewhere; a model's wear is 3140 at flags 2, with the
140 at flags 4 being bunker portals and instrument glass and the 10 at flags 8
being `PI_LIGHT`, `PI_TELE` and the bridge glows; `sky.wea`'s slots are 4, 5
and 8 and never 0 or 2.

The word *additive* used to be read off the data rather than out of the
engine. It is now out of the engine, with every link from a binary:

| where | what |
|---|---|
| `World3D.dll:0x10004415` | `material[0x168] = (flags >> 2) & 0xF` |
| the manager's index 3 | returns `&material[0x164]`, so that field is the block's `+4` |
| `Terrain.dll:0x10028907` | uses the block's `+4` to index a five-entry table |
| `CShade::InitAlphaBlendModeTranslateTable`, `0x10046a60` | fills it with mode ids `0, 4, 2, 3, 5`, falling back to the last supported mode where the device refuses one |
| `Ngi32.dll:0x100346e0` | six 40-byte records of `{D3D render state, value}` — the modes |

Decoding the table gives the blend functions themselves:

| mode | SRCBLEND | DESTBLEND | alpha test |
|---|---|---|---|
| 0 | `ONE` | `ZERO` | off |
| 1 | `SRCALPHA` | `ZERO` | on |
| **2** | **`SRCALPHA`** | **`ONE`** | on |
| 3 | `ZERO` | `SRCCOLOR` | on |
| **4** | **`SRCALPHA`** | **`INVSRCALPHA`** | on |
| 5 | `DESTCOLOR` | `SRCCOLOR` | on |

So the flags byte resolves to exactly what the data said: **0 and 2 reach mode
0** and do not blend at all, **4 and 5 reach mode 4** and blend on alpha, and
**8 reaches mode 2 — `SRCALPHA/ONE`, which is additive.** The engine's own
name for mode 0 is in its assertion text: `BLEND_DISABLE`.

One thing the data could not have told us: **every mode but 0 turns alpha
testing on**, with `ALPHAFUNC = GREATEREQUAL`. The engine alpha-tests whenever
it blends, and never when it does not. `Material.blend_function` returns the
triple.

This is the field a renderer needs; the record's own class byte at +4, below,
is not.

### What a blended batch writes, and the alpha test's reference — *read*

The six mode records set five states each, `SRCBLEND`, `DESTBLEND`,
`ALPHAFUNC`, `ALPHATESTENABLE` and `ALPHABLENDENABLE` (render states `0x13`,
`0x14`, `0x19`, `0xf`, `0x1b`), through `Ngi32.dll`'s cached setter
(`0x10008680`, the cache at `+0x11c + state × 4`). None of them touches the
depth states or the reference, so those come from elsewhere:

- **The reference is 1.** `Ngi32.dll`'s device reset (`0x10006be0`) writes
  `ALPHAREF` (`+0x17c`) 1, `ALPHAFUNC` 7 (`GREATEREQUAL`), `ZENABLE` 1 and
  culling off into the cache, then pushes every state that differs to the
  device (`0x10006c52`). Its constructor set `ZWRITEENABLE` (`+0x154`) 1 as
  well, and `ALPHAREF` again (`0x100060de`, `0x1000611a`, `ebx` 1 from
  `0x10005e45`), so the cache entry's two writers agree. The six mode records
  are 30 `{state, value}` pairs over five states, and **neither `ALPHAREF`
  (24) nor `ZWRITEENABLE` (14) is among them** (*measured*), so **a blended
  draw drops exactly the texels whose alpha is 0** and keeps every other.
- **The depth states belong to the draw item.** `CShade`'s mesh draw gives each
  batch's item `ZENABLE` at `+0x12c` and `ZWRITEENABLE` at `+0x12d`
  (`Terrain.dll:0x10045b1e`): 1 and 1; 1 and **0** when the item is filed under
  layer 5; and 0 and 0 under the cockpit's mode 2. The primitive's render
  (`0x100302e1`) hands the two bytes to render states 7 and `0xe` before it
  draws, and a layer's render puts back the values it saved (`0x1003dc59`).
- **Layer 5 is translucency, not the blend mode.** A batch is filed
  see-through (`0x1004552a`) when its phase's ambient alpha is below 1.0
  (`0x10045567`), or its batch word carries 8 or `0x100`; the blend mode does
  not enter. A see-through item goes to layer 5 of the queue's second list
  (`0x10032f3c`, `+0x1c`), everything else to its layer of the first (`+0x14`),
  and the queue renders one list per call (`0x10032c60`).

*Measured*: the ambient alpha is below 1.0 on one material of 905, `FIRESTORM`
(its fade-in), and every foliage material — `FTREE1`, `HTREE1`, `GRASS`,
`ELKA`, `TF2` — carries 1.0. So **a tree's leaves are an ordinary item: they
blend `SRCALPHA/INVSRCALPHA`, drop their alpha-0 texels and write depth**, and a
leaf drawn first hides the leaves behind it where its texels are not clear.
`TF2`, the palm's texture, has an alpha of 0 on 30,413 of its 65,536 texels and
255 on 21,113; `FTREE1` and `HTREE1` are 4-bit, 0 on 38% and 50%.

Not established: ~~where a batch word's 8 and `0x100` come from (the same
unwritten word the push-out reads, [24-motion.md](24-motion.md#not-established))~~
— the push-out's word is **read** to be the batch record's own first dword
([Materials are per batch](#materials-are-per-batch-not-per-face)), which
carries 8 on 633 batches and `0x100` on 2953; ~~whether the word `CShade`'s mesh
draw tests is that same dword~~ — **read**: the draw asks the mesh's slot 3 for
the batch record and takes its first dword (`0x1004501e`–`0x1004502d`), whose 8
sends the batch to the portal fade
([24-motion.md](24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read)),
so the 2953 batches carrying either — every one of the 633 carries `0x100`
as well, and 1430 more are two-sided `0x102` ones — are the ones tested;
~~which sort type each of the queue's layers is created with (`CreatePrimLayer`,
`0x10031760`, takes 0 to 5, and type 3 is `CCamDistSortLayerVB`, whose render is
`0x1003e1d0`), and so whether a layer's items are drawn in distance order~~ —
**read** for the layers that matter here: the types are the descriptors'
([10-sky.md](10-sky.md#the-dome)), so group 1's layers 6 and 7 are type 3, and
every effect sprite is filed in layer 6
([11-effects.md](11-effects.md#effect-sprites-are-drawn-far-to-near--read-and-seen)),
whose items are drawn far to near; what a type-1, 2 or 5 layer does with its
items' order — the see-through surfaces' layer 5 among them — is not read.

### The class byte is the ground's surface id

The record's byte 4 sorts the library into eleven groups — the ground in 0 to
4, object skins in 5, foliage in 6, water in 7, and three smaller families in
8, 9 and 10, with `0xFF` on the other 376 meaning *not set* rather than a
twelfth group. It has the shape of a shader id, and it is not one: it is **the
surface id a unit reads from the ground it stands on**.

A loaded material is a 368-byte record in a global array in `World3D.dll`, and
the class byte lands at its `+0x154`:

```
+0x000  a flag, initialised to -1      +0x154  the record's class byte
+0x008  entry count                    +0x158  the record's byte 5
+0x00c  entries, 76 bytes each         +0x15c  the record's float, 1.0
+0x010  track count                    +0x160  the record's dword
+0x014  the tracks                     +0x164  the block index 3 hands out
```

An entry is 76 bytes because a `D3DMATERIAL7` is 68, plus the texture and the
cell — which is the same shape the file's 34-byte entry carries, written for
the API rather than for disk.

Three instructions in the whole of `World3D.dll` name the class field's
address: two loader stores, and one accessor that computes
`&material[id].class` and returns it. That accessor is **slot 9 of the
material manager's eleven-slot vtable**, a two-argument call, and
**`Control.dll` calls it** — in the ground contact (`0x1001aaf5`) and again
where a node's damage stage plays its explosion (`0x100114fd`). Each time it
asks the object that owns a face for interface 0xd — the terrain answers with
its manager field `+0x7be0` (`Terrain.dll:0x1001a25a`) — and passes the face's
material id. It takes the class, the float (a speed factor, 1.0 on every
material) and the dword (hit points lost per second: 10000 on the two liquid
beds, 1000 on two damaged bases); see
[24-motion.md](24-motion.md#ground-and-collision--read-and-measured).

An earlier search called the accessor unused, and how it missed is worth
keeping. It assumed only the modules that *store* a manager could call one —
`Terrain.dll`, in three object fields, and `AniMesh.dll` in one — and in those
it found calls through slots 1, 2, 3 and 6 and none through 9, even enumerating
**every** indirect call at that vtable offset (87). It never looked in
`Control.dll`, which gets a manager by `QueryInterface` into a stack local at
the moment it needs one, so a search of where the pointer is kept could not
reach it. See [../analysis/README.md](../analysis/README.md).

So a *renderer* that ignores the class byte loses nothing measurable — the
simulation does not — because the only distinction it draws that a renderer
would act on is which materials carry the
ground's second track, and **all 43 of those have a class below 5 while no
material above it has a second track at all** — the track count says it
already.

### Two signals, and both are used

The flags byte says how a material **composites**; the texture says what its
alpha **means**. They are not the same question, and a renderer needs both.

241 of the 393 textures carry an alpha channel and on most of them it is a
gloss map over solid machinery — alpha-testing that punches holes through a
building, which is why the reader drops the channel by default. The flags byte
is what says when not to. Of the 219 see-through materials, **166 name a
graded alpha** and 32 name a silhouette; of the 417 ordinary skins, only 4 are
silhouettes. So:

- **additive** (flags 8) blends additively whatever the alpha's shape — an
  explosion or a shield is a glow;
- **see-through** (flags 4) blends when the alpha is graded, but takes the
  alpha *test* when it is a silhouette. Those 32 are the foliage — `FTREE1`,
  `HTREE1`, `GRASS`, `ELKA` — and a tree wants a hard edge and a depth write,
  not sorting;
- everything else keeps the old cutout heuristic, and its alpha never leaves
  the reader.

### The second count is animation tracks, not layers

The second `uint16` counts **animation tracks**, not texture layers, which is
what an earlier draft called it. The engine caps it at 20 with the message
*"Too many animations for material."*, and each track is a flags word, a key
count, and one 6-byte key per keyframe: the entry to show, when to show it,
and a word that is zero on all 3147 shipped keys. Playing a track walks its
keys and interpolates the two entries that bracket the clock — which is why
an entry is a whole `D3DMATERIAL7` and not just a name.

So the counts read the other way round from the old guess. `WATER_M` is one
track of ten keys 200 apart, and `WATER_BOT` is **two tracks of one key
each**. 860 materials have a single track, 43 have two and two have eight.

Every one of the 102 tracks across the 45 materials that have more than one
holds a **single key, and track *i* names entry *i***. So the extra tracks are
not later frames — they are alternative renderings of the same surface, and
the caller picks one:

- the **43** two-track materials are the ground, and track 1 is the `M` twin
  the landscape lays over the base as a second texture stage — see
  [03-terrain.md](03-terrain.md#who-asks-for-track-1--read-and-measured);
- the **two** eight-track ones, `B_LBL_01` and `R_LBL_01`, name cells 0 to 7
  of one insignia sheet, blue and red. `R_LBL_01` is in the wear of 23
  turrets, 12 chassis and 7 static models, `B_LBL_01` in 19 buildings'
  (*measured*).

See `openparkan/materials.py`.

### Who picks an object mesh's material track — *read*

An object mesh does not fetch its materials itself; `CShade` does, and the
mesh tells it which track. At the start of a draw (`AniMesh.dll:0x10014b30`,
interface `0x18` slot 11) the mesh calls `CShade::StartMeshRender`
(`Terrain.dll:0x100437c0`, at `AniMesh.dll:0x10014e0a`) with a material
manager and a track; `CShade` keeps them at `+0xcb8` and `+0xcbc` and passes
the track to the manager's slot 5 (`Terrain.dll:0x100454e6`) or slot 3
(`0x10045521`) for every batch.

- **The track** is what the unit's `ILifeSystem` slot 15 answers
  (`AniMesh.dll:0x10014dee`, through the mesh's `+0x30`, `QueryInterface`
  0x16): the control system's `+0x554` (`Control.dll:0x10008800`). The
  constructor sets it to 0 (`0x100070d1`) and only slot 16 (`0x10008810`)
  writes it; no caller of slot 16 has been found, so as far as is read an
  object draws track 0 — cell 0 of an insignia sheet.
  - **Seen otherwise** in "Let's Play - Parkan: Iron Strategy, Part 6.5"
    (9SBZOCWv_vE, 37.4 s and 2:15). On C03 M02, Enemy 1's Medium Mine wears
    `B_LBL_01`'s track 1, cell 6 of `PG27` (a filled triangle over a bar), and
    the player's Small Bunker track 0, the arrow. So something writes the track
    per clan, and the clan's index, the sign a single-player game gives clan
    *i*, picks it. The engine draws a building so; what writes it is not read.
- **The manager** is the mesh's own (`+0x24`, `QueryInterface` 0xd, taken at
  `0x10007022`) — unless IAnimation slot 27 (`AniMesh.dll:0x10005970`) has
  given it another. Slot 27 takes a face reference, asks the object that owns
  the face for *its* manager (`0x100059e3`, into mesh `+0x204`) and keeps the
  face's material handle at `+0x208`. From then on the mesh draws every batch
  in that one material (interface `0x18` slot 3, `0x10013595`) with the owner's
  manager and track 0. Who calls slot 27 is not established; something that
  wants to wear the surface it hit is the obvious reader.

The landscape draws through the same call with its own manager and track 0
(`Terrain.dll:0x1001b95e`).

### How a material reaches the device — *read*, and *measured*

Mission 01's objective buoys (`s_tree_29`) are the worked example: the
beam is `HLP_RAY_R` (16 batches), the lamp `HLP_LAMP_R` (2) and the base ring
`HLP_PLACE_R` (6), from the model's wear (*measured*).

**Loading** (`World3D.dll:0x100045f7`..`0x100049b7`). The 34-byte file entry
becomes a 76-byte `D3DMATERIAL7` plus two fields:

| file | loaded | scale |
|---|---|---|
| +0..+2 ambient rgb | +0x10..+0x18 | ÷255 |
| +3 ambient alpha | +0x1c | ×0.01 |
| +4..+6 diffuse rgb, +7 alpha | +0x00..+0x0c | ÷255 |
| +8..+11 specular | +0x20..+0x2c | ÷255 |
| +12..+15 emissive | +0x30..+0x3c | ÷255 |
| +16 power | +0x40 | int |
| name | +0x44 | texture handle (`0x10004b10`); −1 when the name is empty |
| +17 cell | +0x48 | int8; −1 when the name is empty |

A track's word splits at `0x100049ff` into **mode = word & 7** and **lerp mask
= word >> 3**; each key keeps its entry as a dword and its time as the
`uint16`. Across the 962 tracks the modes are 821 × 0, 96 × 1, 44 × 2 and
1 × 3, and the masks 848 × 0, 106 × 1, 3 × 3, 3 × 7, 1 × 17 and 1 × 2
(*measured*).

**Playing a track.** The manager has two fetches:

- **slot 3** (`0x100031f0`) takes the world clock in milliseconds
  (`[0x10032a38]`, the low dword of `0x10029c88`) less the material's start
  stamp (its list record `+4`, set by slot 10 at `0x10003ae0`). The period is
  the **last key's time**, and the mode picks through the table at
  `0x10003668`: **0 loops** (time mod period), **1 ping-pongs**, **2 plays
  once** and holds the last key, **3 jumps** to `rand() % period`;
- **slot 5** (`0x10003680`) takes a fraction instead: outside 0..1 it becomes
  0.5, and the time is last-key-time × fraction.

Either way the brackets are the same (`0x10003758`): the key *i* is the
largest with `key[i-1].time <= t < key[i].time`, and *i* = 0 when none is. The
material shown is `lerp(key[i].entry, key[i+1 mod n].entry, f)` with
`f = (t - key[i-1].time) / (key[i].time - key[i-1].time)` and
`key[-1].time = 0`. So **a key's time is when its interval ends**, and the
next key's entry has arrived by then: the loop is continuous, and it wraps
back to key 0's entry at the period. A single-key track shows its entry as is.

The lerp (`0x10003030`) is per mask bit — **1 ambient rgb, 2 diffuse rgb, 4
specular rgb, 8 emissive rgb, 0x10 ambient alpha** — and every other field,
the power, the **texture and the cell**, comes from `key[i].entry` unchanged.
The texture and cell therefore **step**; only colours glide.

**The device material** is built per draw item (`Terrain.dll:0x10030819`,
under `CStridedPrimitive::RenderVB` at `0x1002ff27`) from the phase the mesh
draw copied to the item's `+0x70` (`0x10029aa0`):

```
diffuse.rgb  = entry diffuse          diffuse.a  = entry AMBIENT alpha
ambient      = 0, 0, 0, 1
specular.rgb = entry specular         specular.a = 1
emissive.rgb = scene colour + entry AMBIENT rgb
power        = 1 << entry power
```

The scene colour is the sky's property 16
([10-sky.md](10-sky.md#the-scene-colour-is-added-to-every-material)). **The
entry's own emissive is never read** — the `+0x14..+0x1c` of the item block is
the ambient colour. No module sets `D3DRS_AMBIENT` (no `push 0x8b` in
`Terrain.dll`, `Ngi32.dll` or `AniMesh.dll`), so the material's ambient term
multiplies zero anyway: **the ambient colour of a material is its self-light.**
Lighting itself is `D3DRS_LIGHTING = (draw flags >> 4) & 1`
(`Ngi32.dll:0x10007630`); a mesh batch draws with flags `0x404`, plus `0x10`
when its record's `[+0x20]+0x10` is 0 (`Terrain.dll:0x100455e4`). Flag 4
turns culling off (`0x10007662`), so every mesh batch is two-sided. A batch whose ambient alpha is below 1 is
queued as translucent (`0x10045567`).

So for a lit batch, with the texture stage modulating:

```
rgb   = texture.rgb × clamp(scene + ambient + Σ lights × diffuse + specular)
alpha = texture.a × ambient alpha
```

and the blend mode only says what is done with it: flags 0 and 2 write it
(`ONE/ZERO`), 4 and 5 blend it over (`SRCALPHA/INVSRCALPHA`, alpha-tested),
8 adds it (`SRCALPHA/ONE`, alpha-tested). A black diffuse carrying an ambient
colour is the unlit glow, and it is common: 210 at flags 8, 171 at 4, 112 at
2 and 1 at 5 (*measured*).

**The cell** is a UV rewrite, not a texture matrix. `Ngi32.dll`'s page setup
(`0x1000ff60`) keeps a rectangle per cell — record 0 the whole texture
`(0, 1, 0, 1)`, record *i*+1 `(x/W, w/W, y/H, h/H)` from the `Page` table —
and `SetCell`, texture slot 7 (`0x100101d0`), picks one; `RenderVB` calls it at
`0x100301f6`. Slot 13 (`0x100101c0`) hands the rectangle back and the draw
rewrites stage 0's coordinates in place, `u' = u0 + u × du`,
`v' = v0 + v × dv` (`0x100076d0`). The strided path asks both stages'
textures for their rectangles itself (`Terrain.dll:0x10038815`) and applies
them as it expands the streams.

**Object UVs are over 1024, not 256.** The strided expansion
(`Terrain.dll:0x10038400`) reads stream 5's `uint16` pair and computes
`u0 + uint16 × (du × K)` (`0x10038a01`), with `K = 1/1024` set by the static
initialiser at `0x10035070`. `AniMesh.dll` hands stream 5 over with a stride
of 4 (`0x100160cc`, from the slot filled at `0x10016035`), and its own hit
query decodes the same stream with its own 1/1024 (`0x10013d08`). The beam
shows why this matters: its raw UVs run 0..1024 × 0..1018, **exactly one
cell** at 1/1024, and four cells wide by two tall at 1/256 (*measured*). Of
the 3399 object batches whose material asks for a cell, 2626 reach no further
than 1024 (*measured*).

**The buoy.** `HLP_RAY_R` is flags 4 (blended), texture `SUN4.0` — 256 × 256
ARGB4444 whose cells 0 to 2 are the 64 × 128 strips at (0,0), (64,0) and
(0,128) — with a black diffuse, ambient alpha 1 and ambient
`#dc1414`, `#f00019` and `#ffb97d` on cells 0, 1 and 2. Its one track is mode
0, mask 1, keys (entry 0, 50), (1, 100), (2, 150), (1, 200) (*measured*). So
over a 200 ms loop the cell steps 0, 1, 2, 1 every 50 ms while the ambient
glides 0 → 1 → 2 → 1 → 0, and the beam draws
`SUN4.0 strip × clamp(scene + ambient)`, blended on the strip's own alpha:
at Mission 01's 40/255 grey that is about (1.0, 0.24, 0.24) — **a
translucent pinkish red, flickering towards a pale orange-white**, one strip
per face. `HLP_LAMP_R` (flags 2, `S12N2.0`, whole texture) and `HLP_PLACE_R`
(flags 2, `COMP_2.0`, whole texture) are the same track over ambients
`#dc0000` → `#ff3c19` → `#ffa587` and `#dc0000` → `#f08c19` → `#ffb97d`: opaque
surfaces that glow red and pulse five times a second (*derived*).

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

### How a lightmapped batch is drawn — *read*, and *measured*

**The lightmap replaces the scene's light on a lit batch.** `CShade`'s mesh
draw (`Terrain.dll`, the batch loop from `0x10044fcf`) takes each batch record
the mesh hands it (`+0xcb0` slot 3). Bit `0x2000` of the record's flags word
marks a lit batch (`0x10045098`), and for one the draw also expands the UV set
at stream-table `+0x28` (`0x10045963`). That is stream 18: `AniMesh.dll` puts
stream 18 there with a stride of 4 (`0x10016135`), next to stream 5 at `+0x20`.
Once the draw item is built, it asks the mesh's material manager for the
lightmap's texture and hands it to `0x1002c1a0` (`0x10045d64`–`0x10045d97`).

- **The texture.** The call is manager slot 8 (vtable `0x100209e4` + 0x20,
  `World3D.dll:0x100031a0`). It takes the record's `+8` as a key: the wear's
  index in the high word and a lightmap's index in its `LIGHTMAPS` list in the
  low word. It returns that lightmap's texture handle. The wear loader filled
  the list from the section's names (`0x10003e1c`), each loaded through
  `0x10004cb0` from `lightmap.lib` like any texture (`0x10003f24`). *Measured:*
  all 21 `LIGHTMAPS` sections hold exactly one page, so every lit batch takes
  index 0. That index 0 is the material word's high byte, with `0xFF` for
  none, is *derived*; the writer of the flag was not traced.
- **The material is rewritten** (`0x1002c1a0`, before anything else, on the
  draw item's copy of the entry): the diffuse rgb becomes the self-light rgb,
  the diffuse rgb becomes 0, and the power word becomes 0. In the terms of
  [How a material reaches the device](#how-a-material-reaches-the-device--read-and-measured),
  the device material gets:
  - diffuse 0, so no light reaches the batch;
  - emissive = scene colour + the entry's **diffuse**;
  - specular unchanged, at power 1;
  - diffuse alpha still the entry's ambient alpha.
- **Where the device can combine two textures in one pass**, which is where
  `IsPhaseSupported(3)` answered yes into `CShade+0x1918` (`0x100411d2`), the
  same item gets:
  - the lightmap as its **second texture** (`+0xbc`);
  - the whole page as its second cell (`+0xc4` = −1);
  - render phase **3** (`0x1002c219`);
  - blend mode `+0xbf4`, which is the translate table's entry 0, mode 0 `ONE/ZERO` (`0x100411f2`).

  Phase 3 (`Ngi32.dll`'s phase table, record 4) is:
  - stage 0 `MODULATE(TEXTURE, DIFFUSE)`, alpha `SELECTARG2`;
  - stage 1 `MODULATE(TEXTURE, CURRENT)`, alpha `SELECTARG2`;
  - no alpha test.
- **Otherwise** a second item draws the lightmap over the first, unlit
  (flags `0x404`, `0x1002c353`):
  - its only texture is the page, on stream 18's UVs;
  - its vertex colour is the constant `0xffffffff`;
  - its phase is `CShade+0x1920`: 7, `MODULATE(TEXTURE, DIFFUSE)`, where the device has it, else 1 (`0x1004112e`);
  - its blend is the table's entry 3, mode 3, `ZERO/SRCCOLOR`, which multiplies what is already drawn.

  The first item keeps its own phase and blend. The colour comes out the same
  (*derived*).

So, on the one-pass path:

```
rgb   = texture.rgb × lightmap.rgb × clamp(scene + material diffuse + specular)
alpha = material ambient alpha            (the texture's alpha is not used)
```

drawn opaque, with no alpha test, fogged like any batch (nothing on this path
touches the fog state, [10-sky.md](10-sky.md)). The page is `RGB565` decoded
as any texture: each channel 0..1, so white leaves a surface as its texture
has it and the rest darken and tint it. There is no ×2.

*Measured* over the 21 lightmapped meshes of `fortif.rlb`:
- **What the rule covers.** All 972 lit batches use flags-2 materials, the
  ordinary lit skin ([How a material draws](#how-a-material-draws-is-in-the-archive-directory)),
  so the opaque blend changes nothing any of them had.
- **Their diffuse** is `#ffffff` on 518 and `#cdcdcd` on 361 of the 972. A
  white one saturates the clamp whatever the scene colour, so it shows
  texture × lightmap at full strength, day or night.
- **The other batches.** The same buildings' 4,469 unlit batches (3,390 at
  flags 2, 1,079 at flags 4) draw as any model does
  ([How a material reaches the device](#how-a-material-reaches-the-device--read-and-measured)).
  The sun's lights reach them indoors as out. Nothing on this path tests where
  the camera is; that nothing else darkens an inside is a search, not a proof.
- **`fr_b_plant_00.0`**, the Large Factory's page, is 256 × 256. A third of
  its texels have a channel at 250 or more, its median brightness is 115 and
  its mean is (129, 121, 93): saturated green, orange and purple patches over
  white.

*Seen*, on *The Constructor*'s recording (95–108 s): the factory's inside is
dark grey metal under green lamps and lavender walls, as a surface
modulated by those patches is.

**The pod's green glass** is not a lightmap either. The pod's nodes 24 and 25
(`i16`, `i17`) draw 22 batches of `B_COMP_3G`:
- flags 4, blended `SRCALPHA/INVSRCALPHA`;
- a black diffuse, so unlit, over an ambient `#71ff71`;
- the texture `SUN4.0`.

It is a translucent green shell, and from inside it tints everything behind it.

**The phase-6 path never takes a lit batch.** It draws two UV sets over one
image, and needs `CShade`'s setting 25 (`+0xcc8`) and a device with phase 6.
The draw takes it only when the lightmap flag is clear (`0x1004569e`). Which
setting 25 is was not read.

**For an engine:**
1. Load each wear's `LIGHTMAPS` pages. Keep stream 18's UVs, over 1024.
2. For a batch whose material word's high byte is `0x00`, draw with the
   lightmap as a second texture on those UVs, and shade the vertex as
   `clamp(scene + material diffuse)`. Add the specular term if you draw one;
   no light's diffuse applies. Multiply the texture, the lightmap and that
   colour; alpha is the material's ambient alpha. Draw it opaque and fogged.
3. Draw every other batch of the building as any model's.

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

A node's first `uint32` is a **flag word**: some bits make the vertex a place —
a charging dock, the control pod a capturer walks to, a mine's loading place.
See [27-ownership.md](27-ownership.md).

The second `uint32` is **the node the vertex hangs on** (*measured*). A vertex's
position is in that node's frame, and posed through the node it lands on its
place: the pod places of the Large Factory and the Outpost land over their pods
([24-motion.md](24-motion.md#the-ground-inside-a-building--read-in-part-and-measured)).
Taken raw, in the model's frame, they sit tens of metres off.

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

The reading is `(zero, position, vector)`, and the first triple is **exactly
zero on 3432 of the 3599** points.

**A `_d`/`_w`/`_h` triple is one frame's three axes** (*measured*). 47 triples
across the install share a stem and end in `_d`, `_w` and `_h` — `rech_`,
`Smoke_`, `Mineglow_`, `Tele_`, and the energy bridge's `RayD_n`/`RayW_n`/`RayH_n`
— and on 45 of them the three directions lie on three distinct model axes. An
action-4 effect names them in that order on 43 of the 51 records that build a
frame from one, so a frame's first axis is the **depth** whatever hangs in it
travels and its second and third are its **width** and **height**
([11-effects.md](11-effects.md#a-control-point-frames-axes-are-depth-width-and-height--measured)).

**Where it is not zero, it is not floats.** Its second slot is an **int32: the
node the point sits on** — a node of the same-stem mesh on 2984 of the 2985
points that have one (*measured*, `ControlPoint.nodes`). The third slot holds
the same number on 3338 of the 3599. On the hero turret `CameraCenter` names
node 35 (`CP_m1o1`) and `TargetDirect` 34 (`GP_m1o1`).

**The two nodes do two jobs** — *read*:

- **The second slot places the point.** Loading a part's `.cpt`, the control
  system builds one 12-byte entry per point — the record, the part it came
  with, and a node — and the node is the second slot and only the second slot
  (`Control.dll:0x1000b22a`). A part's node 0 becomes its socket, and any other
  node is shifted to where the part's nodes landed in the merged mesh
  (`0x10008be8`). The point's world position and direction come from that node
  (`0x1001b4f0`).
- **The third slot is the node that carries it.** The ground contact reads it
  for each of a state's contact points: it sets node mask bit `0x10` on that
  node through IAnimation slot 8 (`0x1001a3aa`), and a contact point whose
  third-slot node is destroyed — node record `+0x28` bit `0x10`, set when its
  life reaches 0 (`0x10011071`) — no longer touches the ground (`0x1001ac0d`).
  What mask bit `0x10` does in the mesh is not read.

It fits the data (*measured*): on a chassis, where the two differ, the third
is a node below the one the point sits on on **69 of 78** points — every
`weel_*`, every `leg_*`, and most `dust_*` — so a wheel's contact point rides
on the body and dies with the wheel. The other nine name −1 or a node past the
mesh. On guns and rounds, where 156 more points differ, nothing in the ground
contact reads them; whether anything else does was not searched.

The third triple is a direction whose **length carries a magnitude**, which is
what an earlier draft missed when it said `guns.rlb` and `parts.rlb` "store
scalars such as `Width` in a vector slot". They do not. On a frame or an aim
point the vector is unit length — **553 of the 570** named `*_X`, `*_Y`,
`*_Z`, `*Direct` or `*Center` — and on a size it is an axis times that size:
**all 191 points named `Width`, `Height` or `Size` have exactly one non-zero
component**. `parts.rlb`'s `Width_1` at `(0, 0, 0.42)` is 0.42 across the
model's z; its `Dir_1` at `(0, 4.849, 0)` is a barrel axis 4.849 long. One
reading covers every archive.

## Local origins

Vegetation and rock are the two kinds the ground datum fits least well:
`s_tree_59` spans z −127.1 to +2.1, almost entirely *below* its origin, and
the missions place it 125 units above the ground, which compensates almost
exactly. Read as "the mission z is the model origin" that is consistent — the
tree is authored hanging below its origin and placed high enough to make up
for it — but it means a mis-set placement is invisible in the data, and it is
why vegetation's residuals are the widest of the four kinds.

The other half of the reason is **scale**. 216 of the 401 vegetation and rock
placements carry a uniform scale, 0.2 to 21, which the engine applies to the
whole object ([04-missions.md](04-missions.md#the-scale)).
Measured with the scale applied, only 2 of 134 scaled trees and 2 of 82 scaled
stones have their lowest vertex more than 0.25 above the ground. Measured at
scale 1, 63 and 44 do. `s_tree_59` itself is placed at scale 1 both times.
