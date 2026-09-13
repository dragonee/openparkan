# Motion — how a machine moves, and what moving costs

A machine's movement is one `Control.dll` control system (the 0x670-byte class
[23-economy.md](23-economy.md) describes) driving a motion body it embeds at
`+0x1b4`. The controller file supplies three things: an **animation state
graph** (section 1), a **parameter block** of speeds and limits (the frame),
and **components** (section 4) whose engines and masses feed the limits. What
follows is read from the code and checked against the 531 shipped controllers.

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured*
is re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *guess* fits the evidence and is not established.

## The pieces — *read*

| Where | What |
|---|---|
| `+0x46c` | pointer to the authored block, file +20 (`0x10008ee9`) |
| `+0x470`–`+0x4d8` | the live block: a copy of the authored one that the engine overwrites |
| `+0x100`–`+0x19f` | the current state, a copy of one section-1 record (`0x1000c36f`) |
| `+0x1b4` | the motion body (`0x10014260`); it keeps its own pointers, `+0x1ac` authored and `+0x1b0` live (`0x100143e0`) |
| `+0x1c8` | **velocity**, in the machine's own frame, y forward (body `+0x14`) |
| `+0x1d4` | spin: angular velocity, z is yaw (body `+0x20`) |
| `+0x354`, `+0x358` | the left and right running gear's mean life (`0x10012a40`) |
| `+0x538` | total mass, kg (`0x1000fac0`) |
| `+0x4d8` | spare payload, kg — the live copy of file +124 |
| `+0x1a8` | the ground's speed factor (`0x1001a450`) |

## Section 1 is the animation state graph — *read*, and *measured*

`counts[0]` states of `156 + 16 × counts[1]` bytes, then a `counts[0]²` table
of floats — transition costs, not the int32 [13-control.md](13-control.md)
once took them for. The loader copies each state whole and keeps its 16-byte
conditions behind a pointer (`0x10001730`).

| Offset | Field |
|---|---|
| +0x00 | flags: bits 0–2 switch on the velocity box per axis, bits 4–6 the spin box |
| +0x04 | flags: bit 0 an **anchor** the planner chooses among (`0x1000524c`); `0x10000` moves by velocity; `0x100000` a fixed step with no motion; `0x1000000` jitters the step ([Playing a state](#playing-a-state--read-and-measured)) |
| +0x0c, +0x10 | frame pair **A**: first and last frame |
| +0x14, +0x18 | frame pair **B**: first and last frame |
| +0x1c | the blend base toward B |
| +0x20 | the step's fixed length in ms; 0 lets speed set it |
| +0x24 / +0x30 | velocity box, min xyz / max xyz |
| +0x3c / +0x48 | spin box, min xyz / max xyz |
| **+0x54** | **the engine factor** |
| +0x94 | a use count, −1 for unlimited (`0x1000530f`) |
| 16 × B | conditions: bit 0x100 needs a byte of the i-th 0x5c-byte entry at `+0xc4` set, 0x200 needs it clear (`0x10001107`); what the entries are is not read |

A state applies while the machine's velocity and spin lie inside the boxes its
flags switch on (`0x10001000`). An anchor that stops applying queues the
cheapest path to an anchor that does (`0x100051c0`); the table's row is the
destination
([Playing a state](#playing-a-state--read-and-measured)).

*Measured:* 1,690 states in 207 controllers. The 4,323 bounds a flag bit
switches on all read min ≤ max; the 5,817 it leaves off are all exactly
−FLT_MAX..FLT_MAX. On `r_l_01` (Small Walking Chs) the boxes read like gaits:
standing within ±0.6 m/s, walking forward 0.6–12, running 12–25 and backward
the same negated, turning in place with yaw between ±0.1 and ±2.28 rad/s.

## Playing a state — *read*, and *measured*

A controller runs its states one at a time on a clock of its own (`+0xdc`, in
ms). Each tick, while that clock is not ahead of the game's time, the machine
tick (`0x1000bcf0`, loop at `0x1000c2a5`):

1. plans, if the current state is an anchor (below);
2. takes the next state off its queue, copies it to `+0x100` and runs its
   action group (`+0x90`);
3. runs one **state step** (`0x10005370`, at `0x1000c38c`), which moves the
   clock on by the step's length.

**How long a step lasts** (`0x10005370`):

| State | Step |
|---|---|
| `+0x04` bit `0x100000` | `+0x20` ms, and nothing is integrated: 420 building states in `fortif.rlb` and one in `system.rlb` (*measured*) |
| `+0x20` set | `+0x20` ms; a velocity-driven state is cut so that speed × step ≤ 5 (`0x1000550e`) |
| `+0x20` = 0 | ((1 − q) × stride A + q × stride B) ÷ speed (`0x100057c1`) |

- **Then** bit `0x1000000` jitters it by up to ±12.5% (30 states), and it is
  held to 0.01–5 s (`0x100057d6`).
- **Speed** is the largest component of the velocity in the machine's frame.
- **A stride** is how far node 0, the body, moves from a pair's first frame to
  its last. It is measured once per state through the mesh
  (`0x10019df0`), into `+0x58`–`+0x8c`; those bytes are zero in all 1,690
  states in the files.

**What moves the body.**

- A velocity-driven state (`0x10000`) moves it by velocity × step
  (`0x10015920`).
- Any other state moves it by its blended root stride, (1 − q) A + q B, turned
  into the world (`0x10015990`).
- The attitude and velocity integrators run once a step, with dt the step.
- Between steps the drawn body is interpolated by the phase s (`0x10015a50`).

**What the mesh plays** (`0x100059a0`, through `AniMesh.dll`'s interface
`0xb` at `+0x20`):

- **Phase:** s = (t − step start) ÷ step length, held to 0–1.
- **Frames:** A = A₀ + s (A₁ − A₀) and B = B₀ + s (B₁ − B₀)
  (`AniMesh.dll:0x10008b30`).
- **Weight:** w eases from the last step's q to this step's over the first
  quarter: u = min(4s, 1), w = (1 − u) q₋₁ + u q.
- **q** is 1, except on a state that is neither velocity-driven nor fixed in
  length. There q = p + (1 − p) × `+0x1c`, where p = (speed − lo) ÷ D held to
  0–1 (`0x100057a3`). lo is the smaller of the velocity box's largest |min| and
  largest |max|. D is a box extent; that it is the box's span is a *guess*.
- **Pose:** each node takes frame A's pose at w = 0, frame B's at w = 1, and a
  blend between ([07-objects.md](07-objects.md#how-the-engine-plays-it--read)).

So a walk plays one stride of animation per stride of ground covered, and the
feet do not slide.

**Which state comes next** (`0x100051c0`):

- **Only an anchor plans** (bit 0; 593 of the 1,690 states). The states
  between anchors play from the queue.
- **An anchor that still applies** queues the path back to itself: a whole
  cycle.
- **Otherwise** it queues the path to the cheapest other anchor that applies,
  and that anchor's use count `+0x94` goes down by one unless it is −1.
- **The path** is the cheapest by Dijkstra (`0x100019d0`).
  - The row is the destination: `table[to][from]` (*measured*: all 826
    zero-cost edges join a state whose B ends where the next one's starts; read
    the other way, none do).
  - A cost of 1,000,000 or more is no edge.
  - At load each cost is multiplied by one plus the largest gap between the two
    states' boxes (`0x10001790`).

**The hero** (*measured*, `r_h_02` against `R_H_02.msh`):

- **Standing** is state 0: frame 2, 50 ms steps, |vy| ≤ 2 m/s.
- **Starting to walk** plays frames 65→66 or 3→4 in two 125 ms halves (states
  1–2 into the cycle at 9, or 5–6 into it at 33).
- **The walk cycle** is frames 5→13, in third-of-a-frame steps (states 33–44,
  then 9–20). Backward plays the same frames reversed. The walk box is vy 2 to
  10.
- **The run cycle** is frames 18→34, in steps of 4/3 frame (states 73–96). It
  is entered through half-frame transitions, q 0.6, that blend toward pair A,
  frame 2. The run box is vy 6 to 14.
- **Strides:** the body node moves 0.08–0.14 a walk step and 0.46–0.49 a run
  step, and all 72 velocity-driven states stride the way their box runs. A walk
  cycle covers 2.445 and a run cycle 5.674.
- **So** a walk cycle at 5 m/s takes 0.49 s, and a run cycle at the hero's
  14 m/s takes 0.405 s (*derived*).

## The engine factor is the state's — *read*, and *measured*

The factor at `+0x154` in the engine draw (`0x100266e1`) is **state +0x54**,
copied in with the rest of the current state. So an engine draws

    power × (largest |velocity| component ÷ largest |authored top speed| component)
          × the current state's factor × dt

and a machine standing in a factor-0 pose draws nothing, whatever its engines.

| Factor | States | Where |
|---:|---:|---|
| 0 | 1,144 | idle poses; **every** state of the hero chassis (105) and of four of the five animals |
| 1 | 496 | walking and turning, and the single state of every one-state chassis — all the wheeled and tracked ones and most flyers |
| 1.5 | 29 | `r_l_01` (8) and the `a_a_l1` animal (21) |
| 2 | 21 | `r_l_01` (12), `r_m_01` (8), `a_a_l1` (1) |

So **the hero on foot spends no energy walking**, and neither do most animals.

## Speed is a target approached at a fixed acceleration — *read*

Each state step (`0x10005370`, dt = the step's length, held to 0.01–5 s:
[Playing a state](#playing-a-state--read-and-measured)) the
velocity integrator (`0x100153c0`) moves each axis of the velocity toward
*command × live top speed* by at most *live acceleration × dt*, stops at the
target, and clamps the result to the live top speed (`0x10016810`). The
command is the body's `+0x08` triple, `IControl` `+0x1bc`, and `SetTangAccel`
(`0x100043d0`) writes it — from the input table
([From input to motion](#from-input-to-motion--read-and-measured)) or the AI.
`0x10014610` does not write it: it clamps the velocity and turns it into the
world frame.

- **Triple 1 (+20) is the acceleration**, and the live copy is **twice** the
  authored value (`0x1000fe26`).
- **Triple 3 (+44) is the top speed** in m/s, y forward. The game's own stat
  panel shows `+48 × 3.6` as "Max speed" in km/h (`iron3d.dll:0x1006f3c7`).
- **Triple 4 (+56) is a turn rate**: multiplied by dt as the largest angular
  step (`0x100147dd`), and scaled by engines and load below.

## What sets the live limits — *read*

`0x1000fca0` recomputes the live block from the authored one. With

- **E** = Σ over the machine's engines of *value 0 × condition* (id 0x100,
  `0x1002c06d`), times *(left gear + right gear) ÷ 2*
- **r** = spare payload ÷ payload, 1 empty and 0 full
- **G** = the ground's speed factor

it writes

| Live | From authored |
|---|---|
| top speed | `top × G × E × (1 + r) ÷ 2`, **capped at the authored top on each axis** |
| turn | `turn × E × (1.5 r + 0.5)` — not capped |
| acceleration | `accel × 2` |

It runs at load, at reset, after any node takes damage (the flag at `+0x598`
set in `0x10010f30`, `0x1001102f`, acted on next tick by `0x10012a40`), and
when a part comes off (`0x10011920`).

What follows from it:

- **Damage slows a machine**: an engine's condition and the running gear's
  life both enter E.
- **A power shortage does not.** E uses the condition bit (0x100), not the
  power level bit (0x200); nothing in `Control.dll` asks an engine for 0x200
  (a sweep of every value-id query, not a proof). A starved engine's draw is
  what gets cut, not the machine's speed.
- **A robot has one engine, and its mark caps its speed.** The fitted engine replaces
  the chassis slot's drive of 1 ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)),
  so E is its drive, 0.7 on a Mk1 to 1.0 on a Mk4, times its condition and the gear.
  An empty robot on flat ground reaches at most that fraction of the chassis's
  authored top speed, and turns that much slower — *derived*.
- **Load halves top speed at most**: a machine carrying its full payload runs
  at half speed on one healthy engine.

## From input to motion — *read*, and *measured*

**Walking.** `W` sends `MCMD_WALK_F` 1 and `S` sends `MCMD_WALK_B` −1.

- Both set the command's y to ±1, so the hero walks at the full live top
  speed: 14 m/s, `r_h_02` triple 3 (*measured*).
- Release sends 0.
- `World3D.dll` tracks what is held in flags at row `+0x20`/`+0x24`, and in
  global `0x10795244`, "walking".

**Strafing** is `A`/`D`, `MCMD_LEFT`/`RIGHT`. It is not the command's x axis:

- The command's y goes to ±1, with the sign of the current direction.
- `SetStrafeAngle` gets ±π/2, or ±π/4 while already walking (globals
  `0x1079523c` left, `0x10795240` right).
- On release the angle returns to 0, and the command to 0 if nothing else is
  held.

The body applies the *change* in strafe angle as a turn of its own
(`0x10014cf0`: previous at `+0x17c`, delta at `+0x188`). How that turn divides
between the legs and the turret was not read; that the legs turn and the
turret holds its heading (see `0x100059a0` in
[30-turrets.md](30-turrets.md)) is a *guess*.

**Turning is a pending turn.** The angle triple `+0x1e0` (body `+0x2c`) holds a
rotation not yet made. Each tick the attitude integrator:

- reads it as (v − 0.5) × 2π radians per axis (`0x1001480f`);
- turns the body by at most the live turn rate × dt, scaling all three axes by
  the same factor so the largest one fits;
- adds the step back into the triple and wraps it (`0x100149d2`), so the
  triple returns to 0.5 when the turn is done.

So the mouse queues a turn and the hull pays it out. On the hero:

- mouse X is 0.15 × 0.006 × 2π = **0.00565 rad (0.32°) a filtered count**;
- the turn rate is **25.12 rad/s** about z (`r_h_02` triple 4, *measured*);
- so the hull keeps up with any hand.

The two other axes have 1.57 rad/s, and triple 6 is (0, 0, 6.28): the hero
turns only about z (*measured*). That triple 6 is the per-axis limit that
forbids pitch and roll is a *guess*; triple 6 stays open below.

**The keypad cruise** — *measured* rows, *unknown* effect:

- `*` sends `MCMD_FORWARD` 1 and `/` sends 0.
- `+` and `−` send ±1 with the table's ramp 0.05 over 1000.
- They reach the same handler as walking (`0x100101b2`). What the ramp does
  there was not read. That a held `+` adds 0.05 a second to the command is a
  *guess*.

## Running gear: legs, wheels and tracks by side — *read*, and *measured*

`.ndp` flag **0x20 is the left running gear and 0x40 the right**.
`0x10012a40` averages the life fraction of each side's nodes into `+0x354` and
`+0x358`; a machine with no such nodes keeps the constructor's 1.0 on both.

The spin integrator also takes the **difference**: yaw rate gains
`(right − left)` times either the forward speed ÷ the authored top forward
speed or the body's forward command, chosen by a flag at body `+0x1a8`
(`0x1001478e`, `0x10014b3e`), so a machine with one side shot up veers.

*Measured:* 15 `.ndp` tables carry the flags, 28 nodes a side — twelve
chassis and three animals. All 28 flagged 0x20 rest at
x < 0 and all 28 flagged 0x40 at x > 0, where the models' other off-centre
nodes split 74 and 75. The names agree where they have a side letter:
`LFdd`/`RFdd`, `WFLa`/`WFRa`, `TFL`/`TFR`. The twelve chassis are every
walking, wheeled and tracked chassis in the table below, the Transformer,
the Tiny Spider and one flyer, the S-2f, whose pair is `LWB`/`RWB`; the hero
and the other flyers carry none.

## Load — *read*, and *measured*

`0x1000fac0` weighs the machine:

- each node: `.ndp` float +8 × the volume of the node's level-0 geometry slot
  (the slot record's `+0x34`, its bounding box's volume; id 0x10, which
  `AniMesh.dll:0x100051f0` scales by all three of the object's scale factors)
  — a density, which the round body weights of the player chassis bear out
  ([28-chassis.md](28-chassis.md#what-a-chassis-weighs--read-and-measured))
- when the unit has armour, armour's first value × the node's area (`+0x30`,
  `0x1000fbac`)
- each component: its **mass at record +0x1c**, in kg, added to its node
  (ids 0x100 and 0x200, `0x1002bb40`)
- **spare payload** = file +124 + the mass of one node range the mesh reports
  (the chassis's own, by the look of it — *guess*) − the total, never below 0

The stat panel shows **file +124 × 0.001 as "Max payload" in t** and the total
mass × 0.001 as "Weight" (`iron3d.dll:0x1006f300`, IControl 136 and 124).

*Measured:* every internal part but armour carries a mass — 100 to 40,000 kg
over the 104 in `intsys.rlb`, against 0 on all 24 armour parts — and so does
every `o_cNN` ammunition clip, 6.25 to 6,000. The guns' own components and the
slots a chassis or a building declares for its parts carry none, bar one
detection-shield slot of 10. (This page once called the clips guns;
[29-weapons.md](29-weapons.md) sorts them out.) An internal engine's mass
rises with its value and draw:

| | df (0.7) | 01 (0.8) | 02 (0.9) | 03 (1.0) |
|---|---|---|---|---|
| `o_eng_l` | 450 kg, 0.75/s | 675, 1.2 | 900, 1.5 | 1,350, 1.85 |
| `o_eng_m` | 1,200, 2 | 1,800, 2.5 | 2,400, 3.1 | 3,000, 3.75 |
| `o_eng_b` | 1,600, 4.5 | 2,400, 6.8 | 3,700, 9 | 4,800, 11.2 |

and payload rises with chassis size without overlap: tiny 2.775–3.25 t, small
4.25–10 t, medium 24–36 t, large 55–500 t. Five carry 1,000 or 10,000 t, which
reads as no limit: the Small Tower, the L-7f, L-8f and S-7f flyers, and the
hero.

## Ground and slope — *read*

- **The ground is the material under the unit.** Each tick the ground contact
  (`0x1001a450`) finds the world face under the machine
  ([below](#finding-the-ground--read)) and asks that face's owner, through its
  interface 0xd, for **material manager slot 9 on the face's material id**
  (`0x1001aaf5`; the id is world face `+0x74`). What comes back is the loaded
  material's `+0x154` block: the `MAT0` class byte, the **surface id**; the
  float, **G**, into `+0x1a8`; and the dword, **a damage rate**, into
  `+0x1a4`. G scales top speed. An owner with no manager gives surface 10
  (`0x1001aace`). When the surface id changes, block group `+0x504[id]` runs
  (`0x1001ab3e`; an id above 10, such as `0xFF`, runs nothing).
- **Mode 2 brakes on slopes.** Only when file +104 is 2 does the velocity
  integrator compare the ground's tilt with the cone at +112 (`0x100157ac`):
  with `c` the cosine of the tilt, a factor
  `min(1, 2 (c − cos cone) ÷ (1 − cos cone))`, 0 past the cone, pulls the
  velocity toward that fraction of itself at 1.5 × the acceleration. It
  applies moving one way across the slope only — uphill, by the look of it
  (*guess*).

*Measured:* all 509 mode-0 controllers keep the default cone of 1.57079; all
16 mode-2 controllers carry 0.6 rad (34°); the six mode-3 controllers carry
0.6 or 1.52 and take a different branch, the one that adds gravity
([below](#gravity--read-and-measured)).
Among the chassis, mode 2 is every wheeled and tracked chassis, the small and
medium walkers, the Transformer, the Small Tower and the hero; the Large
Walking Chs, the Tiny Spider and every flyer are mode 0 and ignore slope.

## Ground and collision — *read*, and *measured*

### What the shipped surfaces carry — *measured*

| `MAT0` field | read into | on the 905 materials |
|---|---|---|
| +4 class byte | surface id `+0x1b0` | 0–10, or `0xFF` on 376 |
| +6 float | G `+0x1a8` | **1.0 on every one** |
| +10 dword, holding a float | rate `+0x1a4` | 10000 on `WATER_BOT` and `ENV_LAVA_BOT`, 1000 on `B_S0_DAM` and `B_DD1DK_DAM`, 0 on the rest |

- **The ground never changes a machine's speed on shipped data**: G is 1
  everywhere.
- **Lake and lava beds kill.** An agent of kind 4 loses `dt × rate` hit points
  through the node update each tick (`0x10012a66`). Kind 4 is every `BTLU`
  record in `objects.rlb` — the units, the hero among them — because an
  agent's kind comes from its object tag: `BTLU` 4, `BULL` 9, `WPNS` 2, `STAT`
  10 (`AniMesh.dll:0x1000317f`). So a unit touching a bed loses 10000 hit
  points a second. Every one of the 6102 bed faces is surface 1 with that rate.
- **The terrain uses five surface ids.** Counting the faces of both levels of
  detail, surface 1 covers 208406 (`L02`, `L33`, `L35` and most others), 2
  covers 54220 (`L00`, `L19`, `L20`, `L28`, `L32`), 0 covers 9626 (`L08`), 7
  the 1006 `WATER` faces, and `0xFF` the 2624 `ENV_NLAVA` lava surfaces.
- **Tut_1** has 5250 faces of `L02` (1), 2208 of `L00` (2), 478 of
  `WATER_BOT` (a bed, 1, at 10000) and 354 of `WATER` (7).

### The eleven surface groups switch the dust — *measured*

The 84-byte block after section 4 holds **21 section-5 group indices**. The
loader rebases them and copies them to `+0x4dc` (`0x100093ee`), so `+0x504` is
entries 10–20; entry 0 runs once at load (`0x10009408`). On all 531
controllers every entry is -1 or a valid group, and read two bytes early none
of the 273 blocks with an entry set still indexes groups.

Nine chassis set the surface entries: `r_b_03`, `r_b_04`, `r_b_05`, `r_l_01`,
`r_l_03`, `r_l_04`, `r_m_01`, `r_m_03` and `r_m_04`. On all nine, surfaces 0
and 2 share one group and 1 and 3–10 share another. The first group is all
action 11 and the second all action 10, and every effect id they name is one
of the chassis's `dust_*` emitters. Action 10 starts an effect with a mode;
action 11 calls the same interface's slot `0x30` (`0x10002fb5`), and what that
does is *unknown*. That it stops the dust — no dust on `L00` and `L08` — is a
*guess*. The footstep effects (`step_*`) are not in these groups; they are
named records of their own.

### Finding the ground — *read*

1. **Body sphere.** The object's bounding sphere gives the centre and radius.
   A radius under 20 is held to at most 7.5 (`0x1001a48e`); one of 20 or more
   is kept. The centre goes into `+0x98`.
2. **Keep the face.** If a face from the last tick is still held (`+0xa4`),
   the engine walks the mesh from the last ground point (`+0x8c`) to the new
   centre (`Terrain.dll` `CWorld::FindWorldFace`, slot 9, called at
   `0x1001a627`) with the limit 0.173648, cos 80°.
3. **Otherwise search.** An `IWorld` slot 10 query runs at the centre with 0.5,
   in two passes (register values 6, then 10; `0x1001a6cc`, `0x1001a77d`). A
   face is taken if its normal z is above 0.173648 (`0x1001a6fd`) and, on the
   first pass, the hit lies below the centre. **A face steeper than 80° is
   never ground** — 323 of the 173827 level-0 terrain faces across the maps
   (*measured*).
4. **Ground point.** The centre projected onto the face plane goes into
   `+0x70` and the face normal into `+0x7c`; `+0x8c` keeps the point
   (`0x1001a848`). With no face, the ground point is the centre.
5. **Touching** means `|ground point − centre|² ≤ 2r²` (`0x1001a9f9`), and the
   surface record is read only then. The exception is a face with world flag
   `0x400` — the liquid bed, the terrain's `0x2000`: its record is read when an
   `IWorld` slot 8 query for class `0x200`, the liquid surface, finds a gap
   under the radius (`0x1001aa10`). Which way that gap is measured is
   *unknown*.
6. **Contact points.** The same face search runs again for each of the current
   state's `counts[1]` contact points (`0x1001abcc`) — the feet, wheels or
   tracks, placed on their nodes.

### Gravity — *read*, and *measured*

`CWorld` sets gravity to **10.0** at `+0xc` when it is built
(`Terrain.dll:0x10024c1a`); slot 4 returns it and slot 5 sets it. Only a
mode-3 controller uses it: the velocity integrator adds −g along world z,
turned into the machine's frame (`0x10015879`). The six mode-3 controllers are
the four `bf_*_01` rounds in `weapon.rlb` and the two hero targets `r_h_01`
and `r_h_03`. **Modes 0 and 2 — every other machine, the hero `r_h_02`
included — have no gravity term.** What holds them to the ground is the
contact above; how the position is put back onto `+0x70` is *unknown*.

### Lakes in the areal map — *measured*

An areal whose third flag word has all of `0xF0` set (240 or 242 on the
shipped maps) covers **only water and bed faces**: 1845 beds and 1272 water
faces across the maps, counting the level-0 faces whose centre falls in
exactly one areal. Every other areal is ground apart from 12 shore faces.
Tut_1 has 5 lake areals among its 378. See
[08-arealmap.md](08-arealmap.md).

## The chassis, in the game's own units — *measured*

The 22 chassis with a stat panel in `objects.dlb`, each joined to the
same-stem controller in `bases.rlb`. Max speed is `+48 × 3.6`, payload
`+124 ÷ 1000`, acceleration the live `2 × +24`.

| Chassis | Code | Max speed km/h | Accel m/s² | Payload t | Slope | States (factors) |
|---|---|---:|---:|---:|---|---|
| Tiny Spider Chs | T-12w | 95 | 32 | 3.25 | — | 27 (0, 1) |
| Tiny Helicopter Chs | T-2 | 120 | 40 | 2.775 | — | 1 (1) |
| Small Walking Chs | S-12w | 100 | 34 | 6 | 34° | 111 (0, 1, 1.5, 2) |
| Small Wheel Chs | S-31 | 120 | 50 | 6.5 | 34° | 1 (1) |
| Small Track Chs | S-42t | 105 | 40 | 7.5 | 34° | 1 (1) |
| Small Flying Chs | S-2f | 160 | 50 | 4.25 | — | 1 (1) |
| Small Flying Chs | S-4f | 135 | 80 | 5.5 | — | 1 (1) |
| Small Flying Chs | S-6f | 126 | 50 | 10 | — | 1 (1) |
| Small Flying Chs | S-7f | 108 | 60 | 1,000 | — | 1 (1) |
| Medium Walking Chs | M-12 | 90 | 30 | 28 | 34° | 53 (0, 1, 2) |
| Medium Wheel Chs | M-32 | 110 | 40 | 32 | 34° | 1 (1) |
| Medium Track Chs | M-42t | 95 | 30 | 36 | 34° | 1 (1) |
| Medium Flying Chs | M-2f | 125 | 50 | 24 | — | 1 (1) |
| Large Walking Chs | L-12w | 85 | 20 | 65 | — | 92 (0, 1) |
| Large Wheel Chs | L-32 | 95 | 40 | 70 | 34° | 1 (1) |
| Large Track Chs | L-42t | 90 | 26 | 80 | 34° | 1 (1) |
| Large Flying Chs | L-2f | 110 | 44 | 55 | — | 1 (1) |
| Large Flying Chs | L-7f | 108 | 44 | 1,000 | — | 4 (0, 1) |
| Large Flying Chs | L-8f | 72 | 12 | 1,000 | — | 1 (1) |
| Transformer Chs | L-22w | 90 | 12 | 500 | 34° | 131 (0, 1) |
| Small Tower | L-22w | 0.72 | 2 | 1,000 | 34° | 1 (0) |
| Hero chasis | HERO | 50.4 | 140 | 10,000 | 34° | 105 (0) |

Every max speed that moves is a whole number of km/h except the hero's —
20 of 21, against 8 that are whole in m/s — so the speeds were typed in km/h.

## What a full-speed minute costs — *measured*, then *derived*

Every chassis controller declares an engine slot of power 20 and a battery slot of
10,000 at 250 a second (*measured*), but a robot runs on the parts fitted into them
([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
At top speed in a factor-1 state the fitted engine draws its own power figure — 0.75 on
a small Mk1 to 11.2 on a large Mk4 — so a minute at full speed costs 45 of a small Mk1
battery's 3,000 (1.5%) or 672 of a large Mk4's 31,000 (2.2%) — *derived*. Half speed is
half that and a factor-2 state doubles it. The hero's built-in engine asks 0.1 and its
factor is always 0: it walks for free.

Six chassis carry a **second store** with no slot (*measured*): the Transformer, the
L-7f, L-8f, S-6f and S-7f hold a further 1,000,000 at 1,000 a second, and the Small
Tower a generator; those stay beside the fitted battery.

## How the AI asks for speed — *read*

`MBehaviour`'s orders take a fraction of the unit's speed (`Behavior.dll`
`+0x5fc`, from an object record the unit reports): a go order at
`Go_SpeedPercent` (`0x1002b59b`). The compiled defaults (`0x10016250`) are 1
for going, building, transport and patrolling a unit; 0.8 for patrolling a
place and for pathfinding near a building; 80 for patrolling a building (sic);
`Attack_MinAttackSpeedPercent` 0.7 with `Del` 0.3, `MinNearing` 0.8 with
`Del` 0.2; `Movement_InsideBuilding_Speed` 3; `Movement_MaxSpeed` 600. `diff_slow.var` sets
`Speed_MaximumFactor` to 0.7 where the other four difficulties set 1; where
it is applied is not read. The mission property `MaxSpeedPercent` reads back
`Movement_SpeedPercent` and its setter does nothing (`0x1000b621`). `ai.dll`
asks for the live top speed (IControl 145) and compares it with 1
(`0x100091b0`).

## Not established

- Collision between objects: the shapes (sphere, cylinder, the fifth mesh
  slot, the `.bas` polygon, the areal obstacles) and the response
  (slide, stop or push).
- How the body is snapped to the ground point, and whether a mode-0 or mode-2
  machine ever falls.
- Jumping and `CanJump`, the map edge, bridges (`m_bridge`). That buildings and
  bridges are ground through the same world-face query
  (`CBuilding::GetFirstIntersectedFace`) is a *guess* from the names.
- What action 11 does; what register values 6 and 10 select in `IWorld` slot
  10; the sign of the liquid-surface gap.
- The exact divisor D in a blended state's weight (`0x100057a3`), and what
  state bit `0x40000` changes in the integrators.
- Whether the step velocity a state sets (`+0x200`, body `+0x4c`) replaces the
  integrated velocity `+0x1c8` the boxes are tested against.
- Where G would ever differ from 1: no shipped material sets it
  ([Ground and collision](#ground-and-collision--read-and-measured)).
- How the strafe angle's turn (`0x10014cf0`) is split between the hull and the
  turret, and what `MCMD_FORWARD`'s ramp does.
- Which node range the payload sum counts as the chassis.
- Triples 5 (+68) and 6 (+80): 6 clamps an attitude the spin integrator
  drives from a per-state selector (state +0x08), 5 is multiplied into it;
  what that attitude is on screen is not read.
- Whether any module calls `CWorld` slot 5 to change the 10.0 gravity.
- Where `Speed_MaximumFactor` is applied, and what the unit's `+0x5fc` speed
  base is.
- Whether a module outside `Control.dll` stops a machine whose engines are
  unpowered.
