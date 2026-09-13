<!-- Phase R research note R3: Controls, turret aim and the eye. Where this note and docs/ disagree, docs/ wins: docs/30-turrets.md#aiming-and-the-camera--read-and-measured -->

> **Superseded since this note was written:**
>
> - A channel's frames do play linearly in its value, on the node section 2's +0 names (docs/24, *Playing a state*).
> - The camera component's +16 point is `CameraCenter`: `Channel.origin`.
# R3 → engine: input, motion, aim and the first-person eye (Mission 01 hero)

Each line is tagged **R** (read in the binaries), **M** (measured in the data,
`checks.py`) or **G** (a guess the engine must mark as such). Addresses are in
`doc.md` and `known.toml`. Axes are the model's: **x right, y forward, z up**.

## 0. Data to load

| What | Where | Values (hero) |
|---|---|---|
| input table | `hero.tbl`, named by objects.rlb `r_h_02` (M) | rows below |
| chassis controller | `bases.rlb/r_h_02.ctl` triples (M) | accel (90,70,90) → live ×2; top (1,14,1) m/s; turn (1.57,1.57,25.12) rad/s; triple6 (0,0,6.28) |
| turret controller | `turrets.rlb/o_tur_ht_02.ctl` | class-1 entries (1,2); class-4 entries (0,), node 35 |
| channels | section-2 records of that `.ctl` (36 bytes: +4 first frame, +8 last, +0xc initial, +0x14 point, +0x18 rate/s, +0x1c span rad) (R+M) | yaw: 49–53, 0.5, `TurretDirect`, 100, 6.28; pitch: 55–57, 0.2727, `TargetDirect`, 0.75, 1.9199 |
| control points | `turrets.rlb/o_tur_ht_02.cpt`; int32 at +4 = node (M) | `CameraCenter` node 35 pos (0,0,0); `TargetDirect` node 34 dir (0,1,0) |
| turret mesh | `turrets.rlb/o_tur_ha_02.msh`, 62 frames (M) | 1 `Turn_m1o1`, 33 `Eye_m1o1`, 34 `GP_m1o1`, 35 `CP_m1o1` |
| settings | `Iron_3D.ini` `MOUSE_SENS=100`, `MOUSE_REV_Y=0` | sens = 1.0 |

## 1. Mouse → axis deltas (per input tick)

```
m_x = counts_x * sens * 0.95 + 0.05 * m_x_prev          (R)
m_y = counts_y * sens * 1.2 * 0.95 + 0.05 * m_y_prev    (R)
if counts_x == 0 and counts_y == 0: m_x = m_y = m_x_prev = m_y_prev = 0   (R)
delta(row) = clamp(m * 0.006, -1, 1) * invert_axis * row.magnitude        (R)
sens = MOUSE_SENS * 0.01 (R ini read; G that it is the same value)
invert_y = -1 if MOUSE_REV_Y else 1 (G: SetInverseMotion's caller not traced)
```

Row matching: a chord (modifier, key) and press/release select rows, and a
held `SCAN_LSHIFT` selects the Shift rows instead of the plain ones (M,
`hero.tbl`). How ties are resolved when both could match: G. Take the
modifier row when the modifier is held.

## 2. Rows → component values

Every angle target is a triple `v` in normalised units, where 0.5 is centre (R).

```
apply(row):
  v = target_triple(row.class, row.index)
  c = {ANGLE_X:0, ANGLE_Y:1, ANGLE_Z:2}[row.command]
  if row.device is MOUSE axis or JOY axis: v[c] += delta(row)   else: v[c] = row.magnitude
  if row.state == MAN_WRAP: v[c] = v[c] - floor(v[c])            (R)
  else:                     v[c] = clamp(v[c], 0, 1)             (R)
```

`class 0` means the unit's pending-turn triple; `CICLS_TURRET` index 1 means
the first turret; `CICLS_CAMERA` index 1 means the first camera. That index 1
is "the first" is G, supported by `-1` meaning "all" on the weapon rows.

Hero bindings (M):

| input | effect |
|---|---|
| mouse X | unit `turn[z] += Δ` (0.15, wrap) |
| mouse Y | turret `aim[y] += Δ` (0.25, clamp) |
| Shift+mouse X / Y | camera `look[x] += Δ` (0.1, wrap) / `look[y] += Δ` (0.15, clamp) |
| Shift+RMB press, Shift release | camera `look = (0.5, 0.5)` |
| W / S press | command.y = +1 / −1; release → 0 (R) |
| A / D press | strafe (§3); release → strafe 0 |
| keypad * / | | command.y = 1 / 0 (G: FORWARD shares the walk handler) |
| keypad + / − | ramp 0.05 per 1000 ms (G, unread) |

## 3. Motion (per simulation tick, dt = ms × 0.001, held 0.01–5 s)

**Velocity** (R, docs/24): each axis of the body-frame velocity moves toward
`command × live_top` by at most `live_accel × dt`, clamped to live top.
Hero live values: top (1, 14, 1) × G×E×(1+r)/2, capped; accel 2 × authored.
Use E = 1, r = 1, G = 1 for Phase One; ground factor G is from R2.

**Pending turn** (R):

```
e = (turn - 0.5) * 2π                      # radians still to turn, per axis
lim = live_turn * dt                         # (1.57, 1.57, 25.12) * dt on the hero
k = min(1, min_i(lim_i / |e_i|))             # one factor for all axes (R: uniform scale)
step = e * k
rotate hull by step (z = yaw)                # sign: G, test that mouse-right turns right
turn -= step / 2π ; turn = turn - floor(turn)   # returns to 0.5
```

Because triple 6 is (0, 0, 6.28), apply only z on the hero (M; G that it
is a limit).

**Strafe** (R for the values, G for the effect):

- On A/D press, set `strafe = ±π/2`, or `±π/4` if W or S is held.
- Set `command.y = ±1` with the current walking sign (+1 when idle).
- On release, set `strafe = 0` and `command.y = 0` unless W or S is held.
- **Engine** (G): move along the hull's forward direction rotated by `strafe`
  about z, and keep the view heading. The binary turns the body by the change
  in strafe angle (`0x10014cf0`); the split between legs and turret is not
  read.

## 4. Turret aim

State: `aim = (0.5, 1 − 0.2727, 0.5)`, the stored triple, which is mirrored
because the turret is upright, `0x4000000` (R). The channels start at their
initial values:

```
yaw_ch = 0.5
pitch_ch = 0.2727
```

Per tick (R):

```
target = (1 - aim.x, 1 - aim.y)              # upright mirror
yaw_ch   = step_toward(yaw_ch,   target.x, rate=100,  dt, wrap=True)
pitch_ch = step_toward(pitch_ch, target.y, rate=0.75, dt, wrap=False)

step_toward(cur, tgt, rate, dt, wrap):
  d = tgt - cur
  if wrap and |d| > 0.5: d = d - sign(d)     # the short way round
  s = rate * dt
  cur = tgt if |d| <= s else cur + sign(d) * s
  if wrap: cur = cur - floor(cur)
```

Mouse-Y sign (G): the stored `aim.y` is mirrored, so with invert = +1 a
mouse moving down (counts > 0) raises `aim.y`, lowers `pitch_ch` and tilts
the sight down. Verify in play.

**Pose from the channels** (M for the spans, G for linear playback):

- Evaluate each turret node's track at the frame of the channel that moves it:
  - `frame_yaw = 49 + 4 × yaw_ch` for the nodes the yaw frames move
    (`Turn_m1o1`, …);
  - `frame_pitch = 55 + 2 × pitch_ch` for the nodes the pitch frames move
    (`GP_m1o1`, the barrel mounts, …).
- Interpolate the rotation between the two neighbouring keys (slerp) and the
  translation linearly.
- **Which frame each node uses.** A node's track changes only inside its
  channel's frame range. The ranges measured on `o_tur_ha_02` overlap on a few
  nodes (for example `LVin_m1o1`: yaw and gun ranges), so pick per range.
- **Rest.** Nodes outside both ranges use frame 0 of their track.

Measured angles: the sight (`GP_m1o1` +y) sits at −30.5° at pitch 0, 0° at
0.2727, and +79.4° at 1. Yaw 0.5 faces forward.

## 5. First-person camera

- **Eye.** The eye position is the world position of `CameraCenter`: node 35
  `CP_m1o1`'s origin, composed through `Eye_m1o1`, `B_Up_m1o1`, `B_Md_m1o1`,
  `Turn_m1o1` and the turret root. The root sits on the chassis's `Base_TL`
  socket, then the unit transform. On the hero that is 0.876 over the turret
  root and 0.16 forward, and pitch does not move it (M).
- **View direction** is the world direction of `TargetDirect` (0,1,0) on node
  34 `GP_m1o1`, so the view pitches with the turret sight. That the camera
  looks along `TargetDirect` and stands at `CameraCenter` is G; the data
  strongly supports it.
- **Up and side axes.** Up is the node's +z; side is up × forward.
- **Free look** (R), on top of that frame. Build it in this order:

  ```
  qP = axis_angle(side_axis, (0.5 - look.y) * π)
  qR = identity
  qY = axis_angle(up_axis,   (0.5 - look.x) * 2π)
  view = base * (qP * qR * qY)
  ```

  The binary composes the pitch quaternion with the roll-0 and yaw ones about
  (0,1,0) and (0,0,1) in its own camera frame. The engine maps those to
  side/up as above (G for the axis naming).
- **Shake** (R, partly): `offset = shake_vector × 0.02` is added to the eye.
  Its oscillation constants are 3, 3 and 2.5, and the trigger is unknown.
  Phase One: none.
- **Projection.**
  - The horizontal field of view is 1.3 rad (R).
  - Vertical: `2·atan(tan(0.65)·H/W)`.
  - Near 0.1, far 1000 (camera values 0 and 1; G names).
  - Values 3–5 (1, 150, 1): unknown; ignore.
- **Aim point.** The screen centre looks along `TargetDirect`, the turret's
  sight, so a centred crosshair is the aim (derived). How the HUD draws it is
  not read. Barrels have their own points (`Mgun_d`, `Plaz_d`, `Laz_d`,
  `Roc_d1`/`d2`); see R6.
- **Third person.** None found for the hero. Phase One: first person only.

## 6. Open (mark as guesses in code)

- The yaw sign of `turn.z` against mouse direction, and the pitch sign.
- `MCMD_FORWARD` ramp semantics.
- The strafe body turn split (`0x10014cf0`).
- Whether frames play linearly in channel value.
- The two camera point roles (position vs direction).
