<!-- Phase R research note R4: Animation playback. Where this note and docs/ disagree, docs/ wins: docs/24-motion.md#playing-a-state--read-and-measured -->

> **Superseded since this note was written:**
>
> - The live cost's scaling is read: file cost × (1 + g_v + g_s), each gap the largest |destination box centre − source box minimum| over the source's switched-on axes (docs/24). Without it the hero's run ping-pongs into the backward states.
# R4 engine description: playing a controller's animation (Phase One, Mission 01)

Labels: **faithful** means read or measured, with its source in doc.md and
known.toml. **Stand-in** means the behaviour is unknown and we choose it; mark
it `// STAND-IN (R4)` in the code.

## Data (all from the install)

| what | where | reader |
|---|---|---|
| states | `.ctl` section 1: +0x04 mode, +0x0c/+0x10 pair A, +0x14/+0x18 pair B, +0x1c blend base, +0x20 length ms, +0x24.. boxes, +0x54 engine factor, +0x90 action group, +0x94 use count | `control` (patch) |
| transition costs | the `A x A` float table after the states; `cost[to][from]`; >= 1e6 no edge | `control` (patch) |
| channels | `.ctl` section 2: +0 node, +4/+8 frames, +0xc initial, +0x10/+0x14 points, +0x18 rate, +0x1c span, +0x20 flags | `control.Channel` (+ node) |
| pose keys | mesh stream 8 keys, stream 19 frame map, node anim_start / fallback_key | `mesh` |
| item factor | component record +0x24 (1.0 on every door and pod) | raw |

## The machine clock (faithful)

```
per machine: clock_ms, step_start_ms, step_ms, q_prev, q, queue: VecDeque<state>, cur
tick(t_ms):
    while t_ms >= clock_ms:
        if states[cur].mode & ANCHOR: plan()
        if queue.is_empty(): break
        cur = queue.pop_front(); run_action_group(states[cur].actions)
        state_step()
    pose_update(t_ms)                      # every rendered frame
```

`plan()` (faithful, `0x100051c0`):
```
if applies(cur): queue = path(cur -> cur)        # a whole cycle, or the diagonal
else: j = argmin over j != cur, states[j].mode & ANCHOR, applies(j): path_cost(cur -> j)
      if cost < 1e6: queue = path(cur -> j); if use[j] > 0: use[j] -= 1
path: Dijkstra rooted at the target with dist[k] = cost[target][k],
      relax dist[k] = dist[u] + cost[u][k]; next_hop[k] = u; stop when cur is settled;
      queue = next hops from cur to target (target included).
live cost = file cost * (1 + largest box gap)   # STAND-IN: use file cost x 1 until the gap
                                                # formula (0x10001790) is transcribed
```
`applies(s)` = velocity and spin inside the boxes switched on by state +0x00
bits 0–2 and 4–6, plus the conditions (docs/24). For the hero's
conditions: **stand-in**, treat as satisfied.

## A state step (faithful, `0x10005370`)

```
q_prev = q; q = 1.0
if mode & FIXED (0x100000): step = length/1000; clock += length; return   # no integration
speed = max(|v_local.x|, |v_local.y|, |v_local.z|)
step = length / 1000
if mode & BY_VELOCITY and step > 0 and speed * step > 5: step = 5 / speed
if step == 0 and speed > 1e-9:
    if !(mode & BY_VELOCITY):
        lo = min(max|box_min|, max|box_max|)          # velocity box
        p = clamp((speed - lo) / D, 0, 1)             # STAND-IN: D = largest (max - min) of the box
        q = p + (1 - p) * blend
    step = ((1 - q) * stride_a + q * stride_b) / speed
if mode & JITTER: step += step * 0.25 * (rand01() - 0.5)
step = clamp(step, 0.01, 5.0)
step_start = clock; step_ms = 1000 * step; clock += step_ms
attitude_integrate(step); velocity_integrate(step)            # docs/24, dt = step
if mode & BY_VELOCITY: position += velocity_world * step
else: position += rotate(body, (1 - q) * disp_a + q * disp_b)
```
Here `disp_a = R(A1) − R(A0)` and `disp_b = R(B1) − R(B0)`, where R(f) is node
0's translation at frame f (see the key lookup below). `stride_a = |disp_a|`
and `stride_b = |disp_b|`. Compute them once per state at load.

**Stand-in:** whether the body's integrated velocity (+0x1c8) is overwritten
by the step velocity is unread. Keep the integrated velocity for the box tests.

Rendering between steps (faithful, `0x10015a50`): s = clamp((t − step_start) ÷
step_ms, 0, 1). Draw position = lerp(pos_at_step_start, pos, s) and attitude =
slerp(prev, cur, s).

## What the mesh plays (faithful, `0x100059a0`, `AniMesh.dll`)

```
u = min(4 s, 1); w = (1 - u) * q_prev + u * q
fa = lerp(A0, A1, s); fb = lerp(B0, B1, s)
for node in mesh.nodes:
    if node has own segment (a channel): pose = key_at(node, lerp(first, last, v_node))
    else: pose = pose_at(node, fa, fb, w)
pose_at(n, fa, fb, w):
    use_a = w < 1 && fa >= 0;  use_b = w > 0 && fb >= 0
    a only -> key_at(n, fa); b only -> key_at(n, fb); both -> lerp translations, slerp rotations (short way) at w
key_at(n, f):
    if f < 0 or n not animated: return keys[n.fallback]
    k = round_half_even(f - 0.5); if k >= frame_count: return keys[n.fallback]
    i = frame_map[n.anim_start + k]; if i >= n.fallback: return keys[n.fallback]
    if f == keys[i].time: return keys[i]; if f == keys[i+1].time: return keys[i+1]
    t = (f - keys[i].time) / (keys[i+1].time - keys[i].time); lerp / slerp keys[i] -> keys[i+1]
```

**Hero numbers to test against** (measured):
- Stand is state 0, frame 2.
- The walk cycle is frames 5→13: 2.445 of body travel, so 0.49 s at 5 m/s.
- The run cycle is frames 18→34: 5.674, so 0.405 s at 14 m/s.
- It starts walking through frames 65→66 or 3→4, at 125 ms per half-frame
  step.
- 72 velocity-driven states all stride the way their box runs.

Test these in `parkan-sim::anim` with a synthetic mesh, and against the install
with `--ignored`.

**Consequence for M3:** position advances once per state step, not per
simulation tick. The step is 50 ms standing and 16–27 ms walking at 5 m/s
(strides 0.081–0.136). It is 33–35 ms running at 14 m/s (strides 0.46–0.49). The fixed 60 Hz tick calls
`tick(t_ms)`, which runs zero or more steps. Draw with s-interpolation.

## Channels: turret, barrels, doors, pods (faithful)

Loader:
- Each section-2 record becomes a channel on `node`.
- At init the node gets its own segment: A = (−1, −1), B = (first, last),
  weight 1.
- The value is `initial`, wrapped (flag 1) or clamped. Flag 2 plays 1 − v.

**Turret** (R3, `0x100289f0`): v moves toward its target by at most rate × dt.
The node plays `first + v × (last − first)`.

**Items (doors, pods)** (`0x10020900`, `0x10022120`, `0x1002d260`):
```
on step (when t >= end):
    progress += (switched_on ? +0.45 : -0.45) * factor;  clamp to [0,1]
    if progress hit 0 or 1: state_word = 0  (building sees "done")
    for each channel: from = its current value, to = progress
    end = start + max over channels of 1000 * |to - from| (short way if wrap) / (factor * rate)
    if end - start < 1: end = start + 100
between steps: v = from + (to - from) * clamp((t - start) / (end - start))
```
So a pod opens in 1 ÷ rate s and fires the capture at 0.9 ÷ rate s, plus up to
100 ms before the first step.

The rule above is the item's plain state (+0x50 & 0xc = 0). Two other item
states change what happens at the ends (read at `0x10020a52`):

- **0x4:** the progress wraps past 0 and 1 instead of stopping, so it runs
  continuously.
- **0x8:** the progress reverses direction at 0 and 1, so it ping-pongs.

Which objects use 0x4 and 0x8 was not measured. **Stand-in:** implement them
only if a Phase One object turns out to need them.

Pods and capture are outside Phase One; doors use the same item rule.

**Stand-in:** channel flags 0x10 (node +0x128) and 0x100 (node mask bit 0x20)
are unread; ignore them. The camera channel (flag 4) is placed by R3's eye
rule, not by frames.

## Faithful vs unknown summary

| piece | status |
|---|---|
| clock, queue, anchor planning, Dijkstra direction | faithful (read + measured) |
| step length rules, jitter, clamp, per-step integration | faithful (read) |
| body moves by velocity or by root stride; per-frame interpolation | faithful (read) |
| two-frame blend, weight easing, q from speed | faithful, except D (stand-in: box span) |
| key lookup and interpolation | faithful (read) |
| live cost scaling by box gap | stand-in (formula outlined, not transcribed) |
| state conditions (16-byte records) | stand-in (satisfied) |
| channel frames, item step timing, pod open time | faithful (read + measured) |
| which pod part is computer 0 on two-part buildings | guess: file order |
