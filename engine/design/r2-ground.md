<!-- Phase R research note R2: Walking into the world. Where this note and docs/ disagree, docs/ wins: docs/24-motion.md#ground-and-collision--read-and-measured -->

> **Superseded since this note was written:**
>
> - Dust is the other way round from the guess here: it shows on surfaces 0 and 2 only, above a fifth of top speed (docs/24, docs/11).
> - R1 did not settle collision between objects; it is still unknown, and so still a stand-in.
# R2 engine description: ground following and collision (Phase One, Mission 01 / Tut_1)

Labels: **faithful** means read or measured, with its source in doc.md and
known.toml. **Stand-in** means the behaviour is unknown and we choose it; mark
it in the code with `// STAND-IN (R2)`.

## Inputs (all from the install)

| input | where | reader |
|---|---|---|
| terrain faces, vertices, face normals, adjacency | `DATA/MAPS/Tut_1/Land.msh` streams 3, 21 | `landmesh` |
| layer-1 material per face | face field 2 low byte → `Land1.wea` name | `landmesh.layer1_names` |
| surface id, G, damage rate per material | `Material.lib` MAT0 header: +4 u8, +6 f32, +10 f32 (0 when the dword is 0) | `materials` (patch: `library.patch.md`) |
| machine sphere radius | object bounding sphere (see docs/07) | `mesh` |
| controller mode, cone, block groups 10–20 | `.ctl` +104, +112, the 84-byte block | `control` (patch) |
| lake areals | `Land.map` areal word 2, all of 0xF0 set | `arealmap` |
| gravity | 10.0 (a `Terrain.dll` constant) | constant |

## Per tick, per machine (after the velocity integrator from docs/24)

```
r = sphere.radius;  if r < 20: r = min(r, 7.5)            # faithful
c = sphere.centre (world)

# 1. find the ground face
if held_face is valid:
    face = walk_mesh(from=last_ground_point, to=c, held_face,
                     walkable = normal.z > 0.173648)       # faithful: cos 80° limit
    # the walk: step across the edge the segment leaves by (face field 13 twin),
    # stop at the face containing c's xy. STAND-IN: the walk order inside FindWorldFace is unread.
if face is None:
    face = vertical_query(c)       # the face under c (then above): first pass needs hit.z < c.z
    if face and face.normal.z <= 0.173648: face = None     # faithful
# 2. ground point
if face:
    g = project(c onto face plane); n = face.normal          # faithful
    last_ground_point = g; held_face = face
else:
    g = c; n = (0, 0, 1)                                      # STAND-IN for the default normal constant
touching = |g - c|^2 <= 2 r^2                                 # faithful
```

### Surface record (faithful)

```
if touching, or (face is a liquid bed [flags 0x2000] and the water surface is within r):
    m = material(face.layer1)
    surface = m.class (0xFF → none);  G = m.float;  rate = m.dword_as_float
    if surface != last_surface:
        last_surface = surface
        run block group (surface <= 10 ? ctl.block[10 + surface] : none)
if agent_kind == 4 and rate > 0:  hp -= rate * dt            # faithful: 10000/s on beds
live_top_speed uses G (always 1.0 on shipped data)
```

On Tut_1 this gives: `L02` → 1, `L00` → 2, `WATER_BOT` → 1 with rate 10000,
and `WATER` → 7. The lake is the 5 areals marked 240. **Driving into the lake
kills the unit in well under a second** once it touches the bed. The "within r"
test for a bed is a **stand-in**: use `water_level − c.z < r`, because the sign
is unread.

### Holding the machine on the ground

- Mode 0 and mode 2 machines, which include the hero `r_h_02`, have **no
  gravity** in the integrator (faithful). **Stand-in:** after integration set
  `position.z` so that the sphere centre sits at `g.z + r` along `n`, which
  keeps `|g − c| = r` and so touching is true. Use the contact points (below)
  for pitch and roll.
- Mode 3 controllers (the `bf_*_01` rounds, `r_h_01`, `r_h_03`) add
  `velocity += (0, 0, −10) · dt` in world space (faithful), then collide with
  the ground as above.
- Slope: mode 2 brakes past the cone (docs/24, faithful). Faces steeper than
  80° are not ground at all (faithful), so treat them as walls.
  **Stand-in response:** remove the velocity component into the wall's normal.

### Contact points (attitude)

The face search runs again for each of the current state's `counts[1]` contact
points at their nodes (faithful that it happens). **Stand-in:** fit pitch and
roll to the ground points of the front, back, left and right contacts, or use
the terrain normal `n` at `c` until R4 reads the contact records.

### Map edge, bridges, buildings, other units

- **Unknown.** R1 (`0x1001c040`) covers the collision pass. Until it lands:
  - **Stand-in:** clamp the position to the `Land.msh` bounding box inset by `r`.
  - **Stand-in:** units against units and objects are spheres. Push the two
    centres apart along xy by the overlap, and cancel the approaching velocity.
    There is no damage, because collision damage (`0x1000d212`) is from hits.
  - **Stand-in:** `.bas` buildings are their mesh faces added to the walk-face
    set, so a bridge deck is ground when its normal z > 0.1736 and a wall otherwise.

## Faithful vs unknown summary

| piece | status |
|---|---|
| surface record source, G, damage rate, Tut_1 surfaces | faithful (measured) |
| 80° walkable limit, touching test, radius clamp | faithful (read) |
| gravity 10, only mode 3 | faithful (read + measured) |
| surface dust groups (10 start, 11 = slot 0x30) | partly: action 11 unknown |
| mesh walk order, default normal, snap to ground | stand-in |
| object collision, map edge, bridges, jumping | unknown: R1 / later |
