<!-- Phase R research note R1: How a round hits. Where this note and docs/ disagree, docs/ wins: docs/26-damage.md#the-hit-test--read-and-measured -->

> **Superseded since this note was written:**
>
> - What ends a round on a hit is closed: action 17 in its `+0x4e4` group kills it (docs/11, docs/26).
> - A round's owner is the whole robot, not the gun (docs/29, *The round's start*).
> - The sweep ends are read, not guessed: the agent sends message 1 to its collision object before its control system ticks and message 0x1c to it right after (AniMesh:0x10001370), so `start`/`end` are the sphere centre before and after the move (docs/26, *The hit test*).
> - A round's collision radius is its mesh's stream-2 header sphere, times the largest scale (AniMesh:0x1000a891, 0x10009510; docs/26).
> - Shield sector 0 (+y) is the front, 2 (−x) the left (docs/26, *Shields*).
# The hit test, for the Rust engine

Labels: **read** = recovered from the code at the address given; **measured** =
checked on the shipped data by `checks.py`; **guess** = a choice the code does
not settle; **unknown** = open. Addresses are `module:address`.

## 1. Where it runs in the frame (read, World3D:0x10006bf0)

One pass per world frame, no substeps:

1. Broadcast message 1 to every object. A round's controller ticks here
   (Control:0x1000cbb0): it moves, measures the distance moved against its
   remaining range, and clamps at the range end (0x1000cfd0).
2. Run the collision pass once (Control:0x1001c040). It fills each round's
   contact record and posts message 0x1b to every entry whose record flags
   are non-zero (0x1001c2de).
3. Broadcast message 0x1c. The round's post-tick fires its range-expired
   action group, `+0x4ec` (0x1000d390).
4. Deliver the posted queue. Message 0x1b reaches the round's collision
   response (Control:0x1000d0c0, §6).

A round therefore reacts to a contact in the same frame it was found, after
every object has moved.

## 2. What takes part (read)

Every agent gets one collision entry. It is created by AniMesh
(0x1000337b), and its kind comes from its `objects.rlb` tag
(AniMesh:0x1000317f):

| Tag | Kind | Carries a contact record |
|---|---|---|
| `BULL` | 9 (round) | yes |
| `BTLU` | 4 (unit) | yes |
| `WPNS` | 2 | no |
| `STAT` | 10 | no |
| anything else | the kind of the object it hangs on | — |

The contact record comes from Control:0x1001f33d. The tag counts are
measured: 67 BULL, 63 BTLU, 5 WPNS, 81 STAT.

Per entry the engine needs:

- `id`: the object;
- `owner`: the firing object's id, on a round;
- `flags`;
- `start`, `end`: the two centres of this frame's sphere sweep;
- `radius`: the world bounding sphere;
- the geometry to query, which is the object's mesh (§4) or, for the world
  entry, the landscape (§5).

Flags (Control:0x1001f670):

| Bit | Set from | Effect |
|---|---|---|
| 0 | disable/enable | the entry is skipped |
| 1 | type bit `0x20000` | rounds never query this entry's geometry |
| 2 | type bit `0x4000000` | on a round, its query also tests batches flagged `0x200` |

An entry whose radius is still −1, its initial value, is skipped (0x1001c164).

**The sweep ends.**

- read: message 1 sets `start = end = sphere centre` (0x1001fec0);
  message 0x1c sets `end = sphere centre` (0x10020010).
- unknown: how the two ends differ when the pass runs. Neither delivery
  order of message 1 gives different ends.
- guess, for the engine: `start` = the round's position before this
  tick's move, `end` = its position after. This is the range code's own
  measure (0x1000cf8c), and the only reading under which the map-edge
  clip (§3b) can fire.

## 3. The pass (read, Control:0x1001c040)

```
for each entry: record.flags = 0; record.best_d2 = FLT_MAX; record.bubbles.clear()
for i in entries (skip disabled or radius < 0):
    if i has a record:
        if i.kind == 9:  map_edge(i); segment_query(world, i)      # 3a, 3b
        else:            0x1001e650(i)                             # not read; not needed for rounds
    for j in entries[0 .. i) (skip disabled):
        if neither i nor j has a record: continue
        if swept_spheres(j, i):                                    # 3c
            resolve_pair(j, i)                                     # 3d
for each entry with record.flags != 0: post message 0x1b to entry.id
```

A world with its own sub-manager (entry `+0x40`) hands pairs to that manager
instead of step 3d. Mission 01 has no use for this; the rule is read, the
purpose unknown.

**3a. `segment_query(G, R)`** (0x1001d9d0). Run the round's segment
`R.start → R.end` through G's geometry (§4 or §5) with the round filter:

| Filter | Required | Excluded |
|---|---|---|
| node | none | none |
| batch | none | `8`, plus `0x200` unless R's flag bit 2 is set |
| triangle or face | none | `0x24` |

If there is a hit and `d2 < R.best_d2`:

- `R.best_d2 = d2`;
- `R.hit = {object id, node, batch, triangle in batch, triangle, world point}`;
- `R.flags |= 1`.

**3b. `map_edge(R)`** (0x1001e1e0). The world's box comes from world slot 10,
with its top z doubled.

- If `start` is outside the box, do nothing.
- If `end` is outside, intersect the segment with the six faces and take the
  nearest by squared distance from `start`.
- If that distance is below `R.best_d2`, store it and the point, and set
  `R.flags |= 2`.

**3c. `swept_spheres(A, B)`** (0x1001e9f0). Let `r = rA + rB` and
`d = B.start − A.start`.

- If `|d|² < r²`, they touch at once.
- Otherwise let `v = (B.end − B.start) − (A.end − A.start)`.
  - If `|v|² < 1e-6`, there is no contact.
  - Let `b = −d·v`. If `b < 0`, there is no contact.
  - Let `disc = b² − (|d|² − r²)·|v|²`. If `disc < 0`, there is no contact.
  - Let `t = (b − √disc) / |v|²`. There is a contact if `t ≤ 1`.

**3d. `resolve_pair`**, with a round `R` and another entry `O`
(0x1001d630). If neither entry is a round, this goes to 0x1001daf0, which
was not read.

1. If `O.kind == 4` and `O.id == R.owner`, stop. The shooter's own unit
   gives neither a bubble contact nor a face hit.
2. If `O.kind` is 3 or 4, run a bubble test. Kind 4's bubble is its own
   sphere; kind 3's comes from its `+0x34` sphere. Run `swept_spheres(bubble, R)`;
   on contact at `t`:
   - let `point = cR(t) + (cO(t) − cR(t)) · rR / (rR + rO)`, which is read
     only approximately;
   - insert `{O.id, |point − R.start|², point}` into `R.bubbles`, sorted
     ascending;
   - set `R.flags |= 4`.
   - One special case, whose meaning is unknown: when `O` is the context
     entry and the manager's interface 0x11 (slot 7) returns a value at or
     above the point's z, the contact is dropped.
3. Unless `O.flags` bit 1 is set, run `segment_query(O, R)`.

The broadphase gates the mesh query: a round's segment is tested against a
unit's mesh only after the two swept spheres touch.

No clan is consulted anywhere in the collision (read). Own-clan and own-object
hits are dropped later, by the hit queue (docs/26).

## 4. Against an object's mesh (read, AniMesh:0x10013ef0 → 0x10010a50 → 0x10008120)

Every node (part) is tested once, with no recursion. The top call passes
mode 1.

1. **Geometry.** Take `slot = node.slot_index[variant * 5 + 0]`, with
   `variant` the node's current one (AniMesh:0x100124d0). This is level 0.
   It is not the fifth slot. `0xFFFF` means the node has nothing to hit, so
   collision hulls, which only have a fifth slot, are never struck (measured:
   0 of the 28).
2. **Frame.** Transform the segment into the node's frame. A non-unit object
   scale rescales distances.
3. **Batches.** For each of the slot's batches, test the batch word `+0`
   against the filter. It is 0 on every shipped batch, so every batch passes.
4. **Triangles.** For each triangle (stream 7 flags word):
   1. Skip it if `flags & 0x24`. Measured: 1306 of 129,542 level-0 triangles,
      on 30 meshes — trees and the mines. Mission 01's targets have none.
   2. Build the plane from the stream-7 face normal (int16 ÷ 32767) through
      the triangle's first vertex.
   3. Run the one-sided segment–plane test (NGI32:0x10024410). With the
      normal `n`, offset `d` and `v = p1 − p0`, the segment hits when
      `n·v < 0`, `n·p0 + d ≥ 0` and `n·p1 + d < 0`. The point is
      `p0 + v · (n·p0 + d)/(−n·v)`.
      The test would retry reversed if batch word bit 1 were set; it is never
      set in the shipped data, so front faces only.
   4. Check the point lies in the triangle (0x10019324, not read — *guess*:
      an edge test).
   5. Take `d2 = |point − p0|²`. If `d2 ≤ best`, keep it.
      `best` starts at `|p1 − p0|²`.
5. **Result.** Node index, batch, triangle in batch, triangle index,
   `point` in world space and `d2`.
   - guess: the hit's node index equals the mesh node index. Part `i`'s
     `+8` names its node.

## 5. Against the terrain (read, Terrain:0x100205c0 → 0x1001dbe0)

`CLandscape::GetFirstIntersectedFace(start, end, filter)`:

1. **The landscape gate.** The landscape's own type mask must pass the
   filter's node masks, which are 0/0 for rounds. It always passes.
2. **The grid walk.** Walk the grid cells the segment's xy crosses, in order
   from `start`. The cell size is `+0x7a28 → +0x14`, with a floor per axis.
   Cells outside `[0, nx) × [0, ny)` are skipped.
3. **The faces in a cell.** For every face in the cell:
   1. The face's 32-bit mask must contain the required bits (0) and none of
      the excluded ones: `8` and `0x200` for a round.
   2. Run the segment–plane test from §4.4, one-sided. A face with mask bit 8
      is tried both ways, but a round excludes bit 8.
   3. Check the point lies in the triangle (0x1008a58e).
   4. Keep the nearest by squared distance.
4. **The first cell with a hit returns it:**
   `{landscape id, −1, face, 0, face, point, d2}`.

Water:

- read: faces with mask bit 8 are excluded;
- guess: mask bit 8 is water, via docs/03's compact field-1 bit `0x02`. So
  a round goes through water.
- measured: the shipped terrain surface word takes only compact bits `0x02`
  and `0x10`, so `0x200` never occurs on terrain.

Buildings in Terrain (`CBuilding`, vtable 0x1009b4ec slot 6) answer the same
query. Whether Mission 01's buildings go through it is unknown: they are
object meshes (§4) if they are AniMesh agents.

## 6. The round's response (read, Control:0x1000d0c0)

Precision: every comparison is squared distance from the segment start, in
32-bit floats.

### Bubble contacts (flag 4)

Walk `R.bubbles` in order. If flag 1 is also set, stop at the first contact
with `d2 ≥ R.best_d2`. For each contact:

1. Get the target's IDeviceManager. If the round's type has `0x2000000`,
   skip all of them.
2. Look up the sector at the point (slot 13). If there is none, go to the
   next contact.
3. Read the sector's strength (slot 14, docs/26).
4. If the round's life (`+0x590`) is greater than the strength:
   - notify the target (0x10, 0x19, shooter);
   - call slot 15 on the sector with 0; *guess*: that empties it;
   - take `strength` off the round's life (0x10010ba0);
   - continue with the next contact.
5. Otherwise:
   - the hit becomes `{target id, sector}`;
   - the round moves to the contact point;
   - it runs action group `+0x4e4`;
   - it takes its whole life off itself, and dies;
   - it returns.

A sector with nothing left has strength 0, so a live round passes it. The
contact is still made and the round loses nothing.

### Face or map edge (flags 1 or 2)

1. The round moves to `R.hit.point`.
2. It runs action group `+0x4e4` for a face hit (flag 1) or `+0x4e8` for the
   map edge alone.
3. Slot 23 does not kill the round here. unknown: which action in `+0x4e4`
   ends it and queues the `.exp`. docs/26 has a round die from full health.

### What the engine does

On a face hit:

- place the round at the point;
- fire its hit action group;
- apply its `.exp` as docs/26 describes, to `R.hit.object` and
  `R.hit.node`, or `−1` on terrain.

How the hit's object and node reach ILifeSystem slot 8 (Control:0x1000ebc0)
is not traced. The contact reference there has the same five-integer form:

| Field | Terrain | Mesh |
|---|---|---|
| `+0` | object id | object id |
| `+4` | −1 | node |
| `+8` | face | batch |
| `+0xc` | 0 | triangle in batch |
| `+0x10` | face | triangle |

No hit uses `{0, −2, −1, −1, −1}`. Slot 8's hit `+0x10` is compared with a
device's node (0x1000ec4a).

## 7. Beams and tunnelling

- A laser is an ordinary kind-9 round at 10,000 m/s, range 1,000. This is
  measured, for `bl_h_01`.
- read: every test is a segment or a swept sphere over the frame; nothing is
  sampled, so nothing tunnels within a frame.
- measured: 54 of 66 BULL records move more than their stream-2 radius even
  in 0.01 s. A point-sampled test would miss.

## 8. Mission 01 numbers (measured)

**Targets:**

| Mesh | Nodes | Nodes with level 0 | Level-0 triangles |
|---|---|---|---|
| `r_h_01` | 4 | 4 | 104 |
| `r_h_03` | 6 | 6 | 174 |
| `r_h_02` (the hero) | 10 | 9 | 230 |

None of the three has a hull, a fifth slot or a skipped triangle.

**Hero rounds:**

| Round | Speed (m/s) | Range | Radius |
|---|---|---|---|
| `bb_h_01` | 350 | 500 | 0.12 |
| `bp_h_01` | 150 | 150 | 0.21 |
| `bl_h_01` | 10,000 | 1,000 | 0.12 |
| `bm_h_01` | 70 | 350 | 0.40 |

The speed is the `.ctl` `+44` y value and the range is `+108`. That the
collision radius is the stream-2 header's sphere is a *guess*; AniMesh
returns a local sphere at `+0x110` (0x10014580).
