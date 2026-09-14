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
| `+0x21c` | the settle angles: the pitch and roll that would stand the hull up (body `+0x68`, `0x1000c576`) |
| `+0x254` | the lean: pitch, roll and yaw offsets in radians (body `+0xa0`, `0x10014e5f`) |
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
| +0x04 | flags: bit 0 an **anchor** the planner chooses among (`0x1000524c`); `0x10000` moves by velocity; `0x100000` a fixed step with no motion; `0x1000000` jitters the step ([Playing a state](#playing-a-state--read-and-measured)); `0x10`–`0x80` and `0x400000` how the hull rights itself ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)) |
| +0x08..+0x0a | the lean: one selector byte per axis, what leans the body about x, y, z ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured), [13-control.md](13-control.md#the-lean-and-triple-6--read-and-measured)) |
| +0x0c, +0x10 | frame pair **A**: first and last frame |
| +0x14, +0x18 | frame pair **B**: first and last frame |
| +0x1c | the blend base toward B |
| +0x20 | the step's fixed length in ms; 0 lets speed set it |
| +0x24 / +0x30 | velocity box, min xyz / max xyz |
| +0x3c / +0x48 | spin box, min xyz / max xyz |
| **+0x54** | **the engine factor** |
| +0x94 | a use count, −1 for unlimited (`0x1000530f`) |
| 16 × B | the **contacts**, a foot, wheel or leg each: a control point, flags, the group run when it lands. `0x100` makes the state need that point's node intact, `0x200` destroyed (`0x10001107`) — a walker's limping states ([13-control.md](13-control.md#section-1s-conditions-are-contacts--read-and-measured)) |

A state applies while the machine's velocity and spin lie inside the boxes its
flags switch on, its contacts' nodes are intact or destroyed as they ask, and
its request code matches (`0x10001000`). An anchor that stops applying queues the
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
- The picture adds no second copy of the stride: the pose walk clears the
  root node's translation and keeps its rotation (`AniMesh.dll:0x10008d88`,
  [07-objects.md](07-objects.md#how-the-engine-plays-it--read)).

**The step velocity is a report, not a state.** The move writes the velocity
it used into `+0x200` (body `+0x4c`): a velocity-driven state the integrated
velocity clamped into its own box and turned into the world (`0x10001160`,
`0x10014610`, stored at `0x1001592b`); any other state its world stride ÷ the
step (`0x10015990`). Nothing copies it back:

- the planner tests the boxes against the integrated velocity `+0x1c8` and
  spin `+0x1d4` (`0x100051ef`);
- only the integrator and `SetTangSpeed` (`0x100044a0`) write `+0x1c8`;
- `+0x200` is read by the property getter alone — 0x28 and 0x67 its length,
  0x66 mass × it (`0x1000df1a`, `0x1000e145`, `0x1000e0e1`) — where property
  0x23 is the length of `+0x1c8` (`0x1000de5d`).

**State bit `0x40000` lifts the limits** (`0x100053f4`, passed to all three
body routines):

- no clamp of the velocity to the live top speed (`0x1001461e`, `0x100158c4`);
- no slope brake (`0x10015681`);
- no turn-rate clamp on the pending turn (`0x10014c02`).

*Measured:* 153 states carry it — 70 in `weapon.rlb`, 80 in `static.rlb` (the
trees and stones) and 3 in `system.rlb`. No chassis or animal state does.

**What the mesh plays** (`0x100059a0`, through `AniMesh.dll`'s interface
`0xb` at `+0x20`):

- **Phase:** s = (t − step start) ÷ step length, held to 0–1.
- **Frames:** A = A₀ + s (A₁ − A₀) and B = B₀ + s (B₁ − B₀)
  (`AniMesh.dll:0x10008b30`).
- **Weight:** w eases from the last step's q to this step's over the first
  quarter: u = min(4s, 1), w = (1 − u) q₋₁ + u q.
- **q** is 1, except on a state that is neither velocity-driven nor fixed in
  length, while the machine moves (speed above 1e-9, `0x1000553e`). There
  q = p + (1 − p) × `+0x1c`, where p = (speed − lo) ÷ D held to 0–1
  (`0x100057a3`). Both are taken over all three axes of the velocity box,
  switched on or not (`0x1000555b`–`0x1000576d`):
  - **lo** is the smaller of the largest |min| and the largest |max|;
  - **D** is the largest ||max| − |min|| — the absolute bounds, their
    difference and its absolute value (`0x100055d4`), then the largest
    component (`0x10001af0`). An axis left off gives 0, and a box that
    straddles zero gives less than its span. D = 0 leaves q at 1.

  *Measured:* 348 states take this branch. All 348 switch on all three
  velocity axes, so lo is finite. On 324 D equals the box's largest span; on
  the other 24 D is 0, and their blend base is 1 anyway.
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
  - A cost of 1,000,000 or more is no edge. The planner compares the scaled
    costs (`0x10001a0e`).
  - At load each cost is scaled (`0x10001790`, called from the loader at
    `0x10008f57`): it is multiplied by 1 + g_v + g_s.
    - g_v is the largest distance, over the velocity axes the **source** state
      switches on, from the centre of the destination's velocity box to the
      minimum of the source's.
    - g_s is the same over the spin box (`0x10001af0` takes the larger of
      three).
    - It is not the gap between the boxes. *Measured:* of the 3,522 edges
      under 1,000,000 in the 531 controllers, the factor changes 1,925
      non-zero costs, by up to ×46.5, and lifts none to 1,000,000. One plus
      the gap between the velocity boxes would give the same factor on only
      776.
    - It is what keeps a gait going (*measured*). On `r_h_02`, 8 of the
      velocity-driven anchors have exits running both ways, and the file ties
      every such exit at 1. Scaled, the exits that keep the direction are the
      cheapest on all 8. At the end of the forward run, state 78 goes on to 85
      at 5, not to the backward run's 79 at 17.

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
when a node reaches or leaves its last damage stage, taking its area, volume and mass
out of the totals or back (`0x10011920`).

What follows from it:

- **Damage slows a machine**: an engine's condition and the running gear's
  life both enter E.
- **A power shortage does not.** E uses the condition bit (0x100), not the
  power level bit (0x200); nothing in `Control.dll` asks an engine for 0x200
  (a sweep of every value-id query, not a proof). A starved engine's draw is
  what gets cut, not the machine's speed.
- **No other module stops it either** — *read*, as a search.
  - A device's power level is its property 1 (`0x1002bbb1`).
  - Outside `Control.dll`, that property is asked for once in the whole
    install: `Behavior.dll:0x100182ba`. That is a docked unit's guns being
    refilled, on class 2 only. The positive control is `Control.dll`'s own
    ask, at `0x1002e6dd`.
  - What the AI does watch is the live top speed. A unit's behaviour copies
    it every takt ([below](#how-the-ai-asks-for-speed--read)).
  - While it is at least 0.5 and above triple 2's forward component, the takt
    sets flag `0x800` at `+0xa04` and runs the walker
    (`Behavior.dll:0x100051b0`).
  - Once it is not, the takt clears the flag and, if the flag was set, clears
    the walker's three target queues (`0x1000528f`, `0x1003dc90`): the AI
    stops driving it.
  - Engines and running gear shot to nothing trip that. A flat battery does
    not, because it never lowers the top speed.
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

**The hull turns and the turret turns back** (*read*):

- **The hull.** The attitude integrator adds the *change* in strafe angle to
  the step's turn after the turn-rate clamp (`0x10014cf0`: previous angle at
  body `+0x17c`, the change at `+0x188`), so the hull swings by the whole
  change in one step.
- **The turret.** The control takt (`0x100059a0`, at `0x10005ab8`) hands the
  turret z = (1 − s) × change − angle, s being the fraction of the step
  gone. That is −(previous + s × change). The turret keeps it ÷ its yaw span
  at `+0xe8` (`0x10028990`), negated on a hung mounting.
- **The yaw channel.** Its first entry adds that offset before it wraps and
  inverts (`0x10021bd0`; slot 12, `0x100294b0`, returns it).
- **What it does.** Pressing `A` walks forward on a hull turned π/2 while the turret
  counter-turns, easing across the step, and the sight stays ahead. Only the
  hull's yaw carries the strafe.

A strafe to the left is +z on the hull. A right-handed frame (left gear at
−x, below) makes that a left turn, and so do the tables, whose
`OBJ_TURN_LEFT` key sends a +0.7 spin. The turret's frames make the matching
counter-turn a right one ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)).
That the two cancel on screen is *derived*.

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

The two other axes have 1.57 rad/s. Triple 6 is (0, 0, 6.28) on the hero
(*measured*), but it is not what keeps the hero from pitching or rolling. It
limits the lean, and every hero state leans on no axis
([below](#the-hull-leans-and-rights-itself--read-and-measured)). An earlier
*guess* on this page, that triple 6 was the per-axis turn limit, was wrong.

**The keypad cruise** — *measured* rows, *read* effect:

- `*` sends `MCMD_FORWARD` 1 and `/` sends 0.
- `+` and `−` send ±1 with the table's ramp 0.05 over 1000. They reach the
  walking handler (`0x100101b2`), which sets the command's y to what the
  row's axis function returns (`0x10010a50`).
- **A pressed key's row stays active**, and every active row is run again
  on each input update (`World3D.dll:0x1000f477`). The key coming up clears it
  ([14-controls.md](14-controls.md#a-row-that-stays-down--read)).
- **Each run moves y toward ±1** by 0.05 × min(1, held ms ÷ 1000), and never
  past it.
- **So the ramp is a step per update, not per second.** The step grows over
  the first second held and is 0.05 after that. At an update every 50 ms,
  a stopped cruise reaches full in 31 runs, about 1.5 s. Letting go leaves y
  where it got to.
- The rate of the input update itself was not read.

## The hull leans and rights itself — *read*, and *measured*

Triples 5 (+68) and 6 (+80) do not limit the turn. They shape how the hull
sits: **triple 6 is the most it leans** on each axis, and **triple 5 is how
fast it rights itself**. Both take effect in the spin integrator
(`0x10014780`), once a state step. Axis x is pitch and y is roll, since y
points forward.

**The lean follows the state's `+0x08` word** (`0x10014e46`).

- Byte 0 drives pitch, byte 1 roll and byte 2 yaw.
- In each byte the low seven bits pick a source (jump table `0x1001538c`), and
  bit `0x80` negates it:

  | selector | source, on that byte's axis of the machine |
  |---:|---|
  | 1, 2, 3 | this step's turn about x, y, z ÷ (authored turn rate × dt) |
  | 4, 5, 6 | velocity x, y, z ÷ the authored top speed on that axis |
  | 8, 9, 10 | this step's change of velocity on x, y, z (body `+0x170`, `0x10015906`) ÷ the authored acceleration |
  | 0, 7 | none: the lean on that axis is set to 0 |

- The target is that fraction × triple 6 on the axis (`0x10014f80`).
- The lean (`+0x254`) moves toward it by at most the live turn rate × dt, and
  is held within ± triple 6 (`0x100151ae`).
- It is turned into a rotation (body `+0x90`, the last step's kept at `+0x80`).
  The drawn body takes it, blended between the two by the step's phase
  (`0x10015a50`).

*Measured*: 28 of the 1,690 states set the word, in 16 controllers.

| word | on | pitch | roll |
|---|---|---|---|
| `0x0386` | every state of the other eight flyers, and all 10 of the `a_a_l2` animal's | −(vertical speed ÷ top) | the turn about z: banks into a turn |
| `0x8389` | the six wheeled and tracked chassis | −(forward speed gained ÷ acceleration): squats as it pulls away | −(the turn about z): leans out of a turn |
| `0x8405` | the Tiny Helicopter T-2 | forward speed ÷ top: nose down | −(sideways speed ÷ top) |

Byte 2 is 0 on all 1,690, so nothing leans in yaw and triple 6's z of 6.28 is
never used. Every walker, the hero and four of the five animals carry 0
everywhere: they never lean, whatever their triple 6. The flyers' triple 6
allows 0.1 to 0.76 rad; the wheeled and tracked chassis 0.15 to 0.25 rad of
pitch and 0.02 to 0.1 of roll.

**Righting follows the state's `+0x04` bits** (`0x1000c3a2`, in the machine
tick).

- **Bit `0x400000`** skips the step.
- **Bits `0x40` and `0x80`** aim at the world's up, (0, 0, 1), turned into the
  hull's frame by its matrix (`+0x264`).
- **Bits `0x10` and `0x20`** aim instead at the vector at `+0x348`. Nothing in
  `Control.dll` writes that offset by displacement; that it is the ground's
  normal is a *guess*.
- Pitch is taken from the target when bit `0x10` or `0x40` is set, and roll
  when `0x20` or `0x80` is; an axis not taken is left at 0 (`0x1000c4af`).
- The angles land in `+0x21c`. Each step the hull turns by triple 5 × those
  angles, on top of its spin (`0x10014b0b`).
- The same bits zero the spin about x and y (`0x10014af9`).

So triple 5 is the share of the hull's tilt taken back each step: 1 rights it
at once, 0.15 slowly, 0.01 (the L-8f) barely.

*Measured*, across the same 1,690 states:

- **808 aim at up** (`0xC0`): every flyer, the hero and its two targets, the
  small and medium walkers, the Transformer, the Small Tower, 14 of the Tiny
  Spider's 27 states and one of the Large Walking Chs's 92, three of the five
  animals, and 44 trees.
- **390 aim at `+0x348`** (`0x30`): the six wheeled and tracked chassis, the
  other 91 Large Walking and 13 Tiny Spider states, the other two animals, 15
  stones, 21 trees and one `system.rlb` controller.
- **421 skip it** (`0x400000`): the 420 building states and one in
  `system.rlb`. The 70 rounds set none of these bits, and the flying camera
  sets `0x40` alone.

That the wheeled and tracked chassis follow the slope and the rest stand
upright is *derived* from this, and the guess about `+0x348` above.

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
- **spare payload** = file +124 + the chassis's own body − the total, never
  below 0 (`0x1000fc51`)

**The chassis's own body is the root object's nodes** — *read*.

- **The range.** The sum asks the model, through `AniMesh.dll` interface slots
  14 and 15 with part id 0, for the first node carrying that id and how many
  do (`0x1000fafa`, `0x1000fb08`; `AniMesh.dll:0x10005780`, `0x100057c0`).
- **The node records.** A model node record (`0x130` bytes at model `+0x1a8`)
  begins with the id of the part that brought it (`AniMesh.dll:0x100123eb`).
- **Id 0 is the root.**
  - The agent enters its own object as part 0 when it is built
    (`AniMesh.dll:0x1000311f`), and merges that mesh with id 0
    (`0x10016a70`).
  - Every part attached after it takes the smallest id not yet used, so 1 or
    more (`0x10003775`). They are attached in the assembly's order
    ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
  - On a robot the root is the chassis.
- **What comes back.** Only those nodes' density × volume returns to spare
  payload (`0x1000fb6f`). Armour's weight on them does not, since it is added
  after (`0x1000fbac`). So a chassis's body never eats into its payload, and
  its armour does.

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
- **G comes from the file and nowhere else** — *read*, as a search of every
  write.
  - In `Control.dll`, only two instructions write `+0x1a8`: the constructor's
    1.0 (`0x10006ec9`) and the ground contact's copy (`0x1001aafd`). The
    contact keeps the last value while the machine touches no face.
  - In `World3D.dll`, only the material loader writes the float the
    contact copies, material `+0x15c`. It sets 1.0 first (`0x1000451a`), then
    the header's float when the record's version is 3 or more
    (`0x1000458b`).
  - No other module holds the manager's record array.
  - So G could differ from 1 only through a `MAT0` record, and none shipped
    has it ([below](#what-the-shipped-surfaces-carry--measured)).
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
  through the node update on each of its life updates (`0x10012a66`). Kind 4
  is every `BTLU` record in `objects.rlb` — the units, the hero among them — because an
  agent's kind comes from its object tag: `BTLU` 4, `BULL` 9, `WPNS` 2, `STAT`
  10 (`AniMesh.dll:0x1000317f`). So a unit touching a bed loses 10000 hit
  points a second. Every one of the 6102 bed faces is surface 1 with that rate.
  [Water and lava beds, in detail](#water-and-lava-beds-kill--read-and-measured)
  gives the cadence and how the loss is shared.
- **The terrain uses five surface ids.** Counting the faces of both levels of
  detail, surface 1 covers 208406 (`L02`, `L33`, `L35` and most others), 2
  covers 54220 (`L00`, `L19`, `L20`, `L28`, `L32`), 0 covers 9626 (`L08`), 7
  the 1006 `WATER` faces, and `0xFF` the 2624 `ENV_NLAVA` lava surfaces.
- **Tut_1** has 5250 faces of `L02` (1), 2208 of `L00` (2), 478 of
  `WATER_BOT` (a bed, 1, at 10000) and 354 of `WATER` (7).

### Water and lava beds kill — *read*, and *measured*

**When the rate is read** (*read*). The ground contact copies a face's
material record — the surface id, G and the rate at `+0x1a4` — only when the
body sphere touches the face, `|ground point − centre|² ≤ 2r²`
(`0x1001a9f9`). A liquid bed (world face flag `0x400`) is read instead when
the liquid surface above it lies less than r below the sphere's centre
(`0x1001aa99`), and the record's dword is loaded as a float (`0x1001ab06`). A
machine touching no face keeps the last rate it read, as it keeps G.

**When it is spent** (*read*). The machine's tick runs a **life update** once
250 ms have passed since the last, give or take 125: the wait is 250
(`0x10006364`) plus a 16-bit random times 250/65536, less 125
(`0x1000c774`–`0x1000c7c1`). The update's dt is the elapsed milliseconds ×
0.001 (`0x1000c7e6`), and it hands that to the node update and to the power
tick (`0x1000c7f5`, `0x1000c7fd`). The node update (`0x10012a40`):

- does nothing to an **invulnerable** object (`+0x5b0`, `0x10012a47`);
- when the rate is above 0 and the agent's kind is 4 (`+0x50`, `0x10012a66`),
  calls `0x10010ba0` with **−rate × dt** and both of its flags clear
  (`0x10012a7e`).

The update also runs on every tick while the counter `+0xd4` is above 0
(`0x1000c7ce`); the tick counts it down (`0x1000c843`). Who sets it is not
traced.

**How the loss is shared** (*read*, `0x10010ba0`). An object with `+0x618` set
takes nothing (`0x10010bb1`). A loss is held to the object's current total life
(`+0x590`, `0x10010c6f`). Then:

- **First flag clear**, as the node update passes it: every node loses **the
  same share of its own current life**. The share is the held loss over the
  total (`0x10010e22`), and each node is handed −share × its life through
  `0x10010f30` (`0x10010eb7`–`0x10010ede`). The stage update follows
  (`0x10010ef7`).
- **First flag set**: the loss is taken whole from the last node down to node 0
  (`0x10010d87`–`0x10010e07`).

So on ground damage no node reaches 0 until the loss reaches the total, and
then every node does in the same update. Node 0 goes with the rest, and the
object dies as any whose node 0 is destroyed
([26-damage.md](26-damage.md#hit-points--read-and-measured)). No shield and no
armour is asked.

**On Mission 01** (*measured*, then *derived*). The hero's two models carry
7355 hit points — `r_h_02` 2881 over 10 nodes, `e_tur_ht_02` 4474 over 36 —
and its seven fitted parts 1 each, 7362 in all. Whether the fitted parts count
in the unit's total is not traced, and the answer below holds either way. The
player's own hero is never given the difficulty ratio
([26-damage.md](26-damage.md#the-difficulty-ratio--read-and-measured)).

- **Time to die** (*derived*). A loss of 10000 × dt means the hero dies on the
  first life update that brings the time charged to 0.74 s. dt counts from the
  previous update, and that one may have been on dry ground, so up to one
  interval before the hero reached the bed is charged too. An even 250 ms
  cadence kills on the **third** update in the lake, 2500 each: 0.5–0.75 s after
  the feet are wet. With the ±125 ms jitter the span is about 0.4–1.1 s.
- **Wading** (*derived*). A bed reads wet once the water surface is less than
  r = 2.18 below the sphere's centre. Where the engine puts the hero's sphere,
  that is feet within about 0.9 of the water line. Walking onto a lake's bed is
  death within a second.
- **A flyer** (*derived*) whose sphere keeps more than r above the water never
  reads the bed. Whether a flying machine runs the ground contact at all is not
  traced.
- **The water does not slow a unit on its way in**: G is 1 on `WATER_BOT`, as
  on every material.
- **Leaving keeps the rate** (*derived* from the first paragraph). A unit that
  jumps off a bed and touches nothing still carries 10000 until it lands on a
  face whose rate is 0.

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
of the chassis's `dust_*` emitters. **The dust shows on surfaces 0 and 2
only** — `L08`, `L09` and `L00`, `L01`, `L19`, `L20`, `L28`, `L32`, `L40` —
*read* and *measured*:

- action 10 starts each dust effect in the record's time mode, which is **0** on
  all nine: effect time held at a set value that is 0 unless something sets it
  (`Effect.dll:0x100074a7`);
- every dust effect's own mode is 15, the unit's speed over its top speed, and
  every dust emitter's window starts at 0.2 (all five `dust_*` effects), so at
  time 0 none emits;
- action 11 (`0x10002fb5`, `Effect.dll:0x10004b40`) restarts the effect in its
  own mode once it has finished — dust has no duration, so at once — which
  gives it back to the unit's speed.

So on the commonest ground, stone class 1, a wheeled or tracked chassis raises
no dust, and on sand and grass it raises it above a fifth of its top speed
([11-effects.md](11-effects.md#how-an-effect-runs--read)). The footstep effects
(`step_*`) are not in these groups: a contact runs its own group when it lands,
and that group picks the step by surface
([13-control.md](13-control.md#a-records-condition-and-runs--read-and-measured)).

### Finding the ground — *read*

1. **Body sphere.** The object's bounding sphere gives the centre and radius.
   A radius under 20 is held to at most 7.5 (`0x1001a48e`); one of 20 or more
   is kept. The centre goes into `+0x98`.
   A second sphere, from the object's other interface (`+0x28`, `0x1001a518`),
   gives a radius r₂ that is held to 7.5 only on objects with flag
   `0x1000000`; it bounds the first search pass below.
2. **Keep the face.** If a face from the last tick is still held (`+0xa4`),
   the engine walks the mesh from the last ground point (`+0x8c`) to the new
   centre (`Terrain.dll` `CWorld::FindWorldFace`, slot 9, called at
   `0x1001a627`) with the limit 0.173648, cos 80°. Any failure clears the
   face and falls through to the search. The walk, in the plane
   (`Terrain.dll:0x10026340`):
   - the start point must lie in the held face, else it fails (code `0x10`);
   - a face whose normal z is not above the limit ends it (code 1);
   - a face holding the centre is the answer;
   - otherwise it crosses edge *e* — vertex *e* to vertex *e* + 1 — where
     vertex *e* lies right of the line start→centre and vertex *e* + 1 left of
     it (`0x100225e0`, twice the signed area), to the neighbour across that
     edge;
   - no such edge, no neighbour, or a 25th face fails (code 4).

   A face's inside is left of all three edges, so the walk assumes faces wind
   counter-clockwise from above and that adjacency slot *e* is the edge from
   vertex *e* to *e* + 1. *Measured:* both hold on every map — 817150 of 817150
   neighbours share those two vertex positions, and 272580 of 272580 walkable
   faces wind counter-clockwise — and on Tut_1 the rule walks from a face to
   the face under a point 7.6 or 10.3 away on 97 of 97 samples.
3. **Otherwise search.** `IWorld` slot 10 (`Terrain.dll:0x10026b20`) is a
   **walk-face query**: its register argument is an axis in the low two bits
   (2, z) and a direction, `4` for faces at or above the point and `8` for
   faces at or below it (`CLandscape::GetWalkFace`, `0x10021b84`,
   `0x10021ff6`, `0x1002201b`). So 6 looks up and 10 looks down. The ground
   contact runs (`0x1001a6cc`, `0x1001a77d`):
   1. up: a face is taken if its normal z is above 0.173648 (`0x1001a6fd`) and
      it is less than r₂ above the centre — the sphere has sunk into it;
   2. down: a face is taken if its normal z is above 0.173648;
   3. neither: the up query's face, whatever it was (`0x1001a7e1`).

   The query walks the scene from its root, recursing into children, and asks
   every node whose type is 1 or 3 (the filter's `0xa` against `1 << type`,
   `0x10026bc9`) — the landscape among them, since it is found — keeping the
   smallest gap. An object answers through
   interface `0x25` slot 2, with the 0.5, if it has one, else interface
   `0x18` slot 7 (`0x10026bdc`, `0x10026c83`). The landscape tests the faces
   of the grid cell under the point, in cell order, and returns the first
   whose triangle holds the point and whose plane lies the right way. An
   `AniMesh` object answers `0x25` at object `+0xc` (its interface request,
   `AniMesh.dll:0x10006e50`), and both its slot 2 (`0x1000ccb0`) and its slot 7
   (`0x10013fe0`) visit **level 0 of the current variant** (`0x10007edb`); a
   `CBuilding` hands slot 7 to its mesh (`Terrain.dll:0x10056c70`). The ground search's filter excludes
   world face bits `0x200` (the liquid surface) and `0x8` (`0x1001a687`).

   **Buildings are ground; scenery and units are not** (*read*). The type the
   filter tests is `IGameObject` slot 11. The collision pass reads the same
   number as the landscape's collision kind 1 (`Control.dll:0x1001c185`).
   `CBuilding` answers 3 (`Terrain.dll:0x10057da0`), and so does a building's
   own agent, which takes its parent's kind. The agent build reads a parent's
   kind through this same slot (`AniMesh.dll:0x10003174`). An agent's kind
   comes from its tag: 4 for a unit, 10 for a `STAT` tree or stone
   (`0x1000317f`). That its own slot 11 hands out that kind is *derived*. So the
   walk-face query asks the landscape and every building, the bridges among
   them, and never a tree, a stone or another unit (*derived*).

   **How an object's faces answer** (*read*):
   - **Interface `0x25` slot 2** (`AniMesh.dll:0x1000ccb0`) visits a node when
     the query point, in x and y, lies within its level-0 slot sphere's radius
     (times the largest scale) plus the 0.5 (`0x1000ce90`). For each of that
     slot's triangles, in the world (`0x1000cfa0`), it rejects a face whose
     normal z is below 0.173648, cos 80° (`0x1000d0ef`, `0x1002096c`). The
     point is then tested against the triangle in x and y (`0x1000d220`).
     Inside, the height is the plane's at the point. Outside, it is measured
     against the first edge it lies beyond: the nearer end's height past an end,
     otherwise the height along the edge, with the squared distance to it. A
     face farther than 0.5 is rejected (`0x1000d145`). Direction bit 8 rejects a
     face above the point, bit 4 one below (`0x1000d161`–`0x1000d18f`). The
     nearest remaining face by squared height gap wins.
   - **Interface `0x18` slot 7** (`0x10013fe0`) does the same with no margin.
     The point must lie inside the triangle, projected along the query's axis
     (`0x10015ca0`). It returns its last answer again when asked the same
     question twice.
   **A face steeper than 80° is ground for a tick at most** — 323 of the
   173827 level-0 terrain faces across the maps (*measured*): only fallback 3
   takes one, and the next tick's walk gives it up at its first step.
4. **Ground point.** The centre dropped **vertically** onto the face plane
   goes into `+0x70` — x and y kept, z solved from the plane through the
   face's first vertex (`0x1001bfc0`) — and the face normal into `+0x7c`;
   `+0x8c` keeps the point (`0x1001a848`). With no face, the ground point is
   the centre.
5. **Touching** means `|ground point − centre|² ≤ 2r²` (`0x1001a9f9`), and the
   surface record is read only then. The exception is a face with world flag
   `0x400` — the liquid bed, the terrain's `0x2000`: its record is read when
   `IWorld` slot 8, the same vertical query in both directions for faces
   with bit `0x200`, the liquid surface (`Terrain.dll:0x10025ba0`), finds one
   with **centre z − surface z < r** (`0x1001aa99`). A sphere is wet from r
   above the water down.
6. **Contact points.** The same face search runs again for each of the current
   state's `counts[1]` contact points (`0x1001abcc`) — the feet, wheels or
   tracks, placed on their nodes. Each 16-byte record in the state is one
   contact: `+0` a node, `+4` flags, `+8` a group. Flag 1 lets it lift the
   body and join the averaged normal (below). The live record's byte `+0x59`
   is set when the contact's node lacks flag `0x10` in its 44-byte life record
   (`+0x55c`, `0x1001ac39`) — whether the node still stands: status `0x10` is
   a node at 0 life (`0x1001106c`, *read* in
   [13-control.md](13-control.md#section-1s-conditions-are-contacts--read-and-measured))
   — and it is that byte a state's `0x100` and `0x200` conditions
   test (`0x10001107`). A contact whose node lacks it searches from the next
   node that has it when flag 4 is set, and is skipped otherwise
   (`0x1001ac3e`).
7. **Placement.** Unless state bit `0x8000000` is set, the object is placed on
   its held face through `IWorld` slot 11, `PlaceObjectOnWorldFace`
   (`0x1001b4c3`).
   - **It becomes a child of what it stands on** (*read*,
     `Terrain.dll:0x10025fc0`). The face record names an object and a part. If
     the machine already hangs on that object and part, nothing happens.
     Otherwise the machine's world matrix is kept (`0x100262c8`), its parent
     removes it (slot 5, `0x100262ee`), the face's object adds it at the face's
     part (slot 4, `0x10026314`), and the world matrix is put back
     (`0x1002632c`). On a bridge's deck the hero is the bridge's child; back on
     the landscape, the landscape's.
   - **Its collision object moves with it** (*read* in part). A collision
     object takes message 21 (`Control.dll:0x1001f548`, table
     `0x1001f638`). With sub-code 6 it leaves its manager and joins the first
     ancestor that answers `0x203` (`0x1001f585`, `0x1001fe40`); with 7 it only
     leaves. That the reparenting sends message 21 is a *guess*. The code that
     sends it was not found.

### Holding the body on the ground — *read*, and *measured*

After the search the ground contact moves the body, through
`0x10015d60` (`0x1001b446`), by a lift **straight up**:

- **State bit `0x4` clear** — flyers, and any state without contact points:
  the lift is (ground z − centre z) + r when the ground point lies less than r
  below the centre, and 0 otherwise (`0x1001b3c3`). The sphere is pushed out
  of the ground and never pulled down: **such a machine does not fall**.
- **State bit `0x4` set, with contact points:** the lift is the largest
  (ground z − contact z) over the contacts with flag 1 (`0x1001b000`), and the
  body **falls**. Each tick it tries a fall of (v − g dt ÷ 2) dt, with g the
  world's gravity (`IWorld` slot 4, 10.0) and v a fall speed at body `+0xac`
  that loses g dt (`0x10015d91`). If the fall stays above the lift it is
  taken; otherwise the body rises or sinks by the lift.
- Whenever the lift is taken, in either case, v returns to 0 and the averaged
  normal — the sphere's and each flag-1 contact's, over their count —
  becomes the body's ground normal `+0x194` (`0x10015e47`), which the slope
  brake reads.
- The move shifts the body's three matrices (`+0xb0`, `+0xf0`, `+0x130`) and
  notifies the object (`0x10015f0f`). The velocity is not touched.

*Measured:* bit `0x4` is set on exactly the 961 states of the controllers
that declare contact points — the walkers, wheeled and tracked chassis, the
hero, three animals, the trees and stones — and on no other. So everything
with legs, wheels or tracks falls under gravity 10, and every flyer holds its
height. A mode-0 or mode-2 machine falls exactly when its state has contact
points.

### Standing on a bridge — *read*, and *measured*

A bridge is a building (`Type` `0x80001000`), so its level-0 faces answer
the ground search as the landscape's do
([Finding the ground](#finding-the-ground--read)). A machine walks onto the
deck when the deck face is the nearest walkable face under its sphere, or a
walkable face above the centre by less than r₂. From then on the hero is the
bridge's child, until a landscape face is nearer again.

*Measured* on Mission 01. Its two `m_bridge.dat` halves (objects 23 and 24,
mesh `fr_m_brige`) are placed turned π apart. Each has 230 level-0 faces, 130
of them with a world normal z above 0.173648:

| along a half, from its land end | deck z | ground |
|---|---:|---|
| the end, 36 m out from the half's origin | 0.32 | dry bank at 0.00 |
| the half's origin | 9.39 | dry bank at 0.00 |
| 24 m past the origin, and on to where the halves meet | 11.52 | falls away to a dry bed 28.7 below the bank, under water at −1.73 |

- **The deck meets the bank.** It starts 0.32 above the dry ground and climbs
  11.2 over 56 m, a slope near 11°, well inside the 80° limit.
- **The halves meet level.** Object 23's deck and object 24's both stand at
  11.52 where they meet, near y 683.
- **Without the deck there is no crossing.** The only ground between the banks
  is the water at −1.73 over a bed that falls to −28.7.

### Collision between objects — *read*

**Who is in the pass** (*read*):
- **Every agent has a collision object.** The agent build makes one whatever
  its kind, once the world has a collision manager (`AniMesh.dll:0x10003382`).
  A tree, a stone, a building's agent and a unit alike.
- **Where it registers.** It joins the manager of the first ancestor above its
  owner that answers `0x203` (`Control.dll:0x1001fe40`), which for a placed
  object is the world's. A building's agent makes a manager of its own
  ([26-damage.md](26-damage.md)), so objects placed inside or on a building
  join that one instead.
- **Who has a contact record.** Only rounds and units do, and a unit's is what
  moves it.
- **What the pass skips.** It passes over an entry whose flags carry 1
  (collision object slot 7 sets it, `0x1001fe10`; slot 6 clears it, callers not
  traced), and one whose radius is still below 0 (`0x1001c15a`).
- **Which pairs go on.** It takes every pair j < i once. A pair goes on only
  when one side has a contact record (`0x1001c1e9`) and the swept spheres touch
  within the frame (`0x1001e9f0`). A unit against a tree, a stone, a building or
  another unit qualifies; two pieces of scenery never do.
- **Handlers.** A pair with a handler of its own (`+0x40` slot 7) goes to it.
  A pair with no round goes to `0x1001daf0`, with **the larger sphere as the
  obstacle A and the smaller as the mover B** (`0x1001d647`). A must answer
  interface `0x25`, or the pair ends (`0x1001db07`).
- **The landscape** is the manager's context entry, kind 1. A unit meets it
  only through the map box ([below](#the-map-edge--read)); the ground search
  holds it up.

**B's move.** B's segment runs from its sphere's start to its end, with its
end moved by B's accumulated push this pass less A's (`0x1001dcb2`). Every
contact record's push is zeroed as the pass starts (`0x1001c077`). A push P
starts at 0:

1. **Faces stop B.** B's segment runs through A's level-0 faces (interface
   `0x18` slot 6). Triangles flagged 4 are passed, and so are batches flagged 8
   and, unless B's collision flags carry 4, batches flagged `0x200`. The filter
   has the round query's shape with triangle mask 4 in place of `0x24`
   (`0x1001db5f`–`0x1001dbb8`). If B's move runs against the struck face's
   normal, B's end goes back to its start and P becomes start − end
   (`0x1001dd42`).
2. **Small faces stop large movers.** A second query takes only batches flagged
   8 (`0x1001dbeb`). When the face it strikes has a shortest edge whose square
   is under 235 and B's class is 4 or more, or under 160 and B's class is 3 or
   more, B's end goes back to its start the same way (`0x1001deb7`,
   `0x1001deea`). The class is the first dword of message `0x201` through B's
   interface `0x10` slot 26. `MBehaviour` answers variable `0x201` with its
   size class (`Behavior.dll:0x1000a533`, set at `0x10005e8f` from
   `0x1000cee0`): `t` 1, `l` and `h` 2, `m` 3, `b` 4. That this is the value the
   pass reads is a *guess*. If so, the hero, class 2, is never stopped this way.
3. **A's shape pushes B out** (`AniMesh.dll:0x1000d410`, *read*). B's sphere
   at its end goes to A's interface `0x25` slot 3, its radius held to 7.5 when
   B's flags carry `0x1000000` (`0x1001df8f`). The push it returns is added to P
   (`0x1001e007`):
   - **Gather.** The pose walk runs. Every node whose level-0 slot sphere
     overlaps B's sphere is visited (`0x1000dfe0`). Each triangle that
     `0x1000e900` accepts, given the triangle, its plane and the sphere, gives a
     direction to B's centre and a distance, turned into the world
     (`0x1000e0c0`). The faces are kept sorted by distance. `0x1000e900`
     (*read*) passes over a triangle whose plane lies behind the centre or more
     than r in front of it. It projects the centre into the plane and keeps
     that point when it is inside the edges; otherwise it takes the closest
     point on an edge or a corner (`0x1000eb70`). So a face pushes only from its
     front, from its closest point.
   - **Hidden faces go.** A face whose centroid is hidden from B's centre by
     another gathered face is dropped: the segment from the centre to the
     centroid meets that face inside its triangle, edges included
     (`0x1000d7a5`–`0x1000dac0`).
   - **Filter.** What is left is filtered by batch and triangle flags, as in
     step 1.
   - **Accumulate**, nearest first (`0x1000dd7d`–`0x1000def8`). Take a face
     with unit direction d and depth p = r − distance, and let s = P·d. If
     s < p, take d′ = d − (d·P)P ÷ |P|², the part of d square to the push so
     far (d′ = d while |P|² ≤ 1e-4). Then P += d′ × (p − s) ÷ (d′·d), or
     P += normalised d′ × (p − s) when |d′·d| ≤ 0.002. So P meets each face's
     depth without undoing the faces before it.
   - **Clamp.** P is held to 4r (`0x1000df50`, `0x10020970`).

**Sharing the push.** P under 1e-6 in squared length is no contact. Otherwise P
is shared by **mass squared**, the owner's property 0x7c copied to the
collision object on message `0x1c` (`0x10020038`). B moves by
P × m_A² ÷ (m_A² + m_B²) and A by −P × m_B² ÷ (m_A² + m_B²). A side with no
contact record does not move, and the other takes all of P (`0x1001e05f`). So
a unit meeting a tree, a stone or a building takes the whole push. Each moved
record gains flag 8.

**When it happens** (*read*,
[26-damage.md](26-damage.md#the-hit-test--read-and-measured)). The world's frame
sends every object message 1, and the control system's tick moves the machine.
Then the pass runs, then message `0x1c`, and then each record with flags set
gets message `0x1b`. So the push lands after this frame's move and ground
contact, and the next frame starts from it. That the ground contact runs inside
that tick is *derived*.

**The machine takes the push** on message `0x1b` (`0x1000c990`, slot 23),
for record flags 8 or `0x10`, by moving its position — the velocity is
kept:

- with state bit `0x4` the push is made **horizontal**: its z is dropped and
  x and y are scaled up to keep its length, at most ×4 (`0x1000ca44`) — unless
  the machine's parent is of type 3 and the push points down, when it is
  taken whole (`0x1000c9eb`);
- without bit `0x4` it is taken whole.

So units slide around each other's meshes, the lighter one giving way, and
no collision costs speed or life. Every geometry step reads level 0: no `.bas`
polygon, areal or fifth slot (the first-person view's geometry, once read as
collision hulls; see [07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws))
takes part in this pass.

**A straight-up push goes nowhere** (*derived*). A walker pressed up by a floor
under it has almost no x and y to keep, so even ×4 moves it next to nothing.
On a slope, though, the push keeps a sideways part. A machine standing on a
building is the building's child
([Finding the ground](#finding-the-ground--read), step 7), and its collision
object is then in the building's own manager, whose pass the world's runs: the
building's faces push it too
([Walking into a building](#walking-into-a-building--read-and-measured)).

*Measured*, the spheres on Mission 01 (the parts' header spheres joined as
`0x10009510` joins them, times the placement's scale):

| object | collision radius | level-0 triangles | flagged |
|---|---:|---:|---|
| the hero, `tut1_p` | 2.18 | 688 | none |
| `l_targ` / `M_targ` dummies | 5.22 / 15.45 | 104 / 174 | none |
| `tut1_e1`, `helic`, `tut1_mf1` | 2.72, 2.45, 5.98 | 756, 812, 1054 | none |
| `s_tree_29` | 3.43 | 178 | none |
| `s_tree_04`, scaled | 43.5–65.2 | 404 | 192 flagged 4, its leaves |
| `s_stone_05`–`_10`, scaled | 41.2–78.9 | 52–86 | none |
| `m_bridge` | 61.9 | 230 | 16 flagged 2, 18 flagged 4 |

The hero is the smaller sphere against every one of them, so it is always the
mover. Its sphere is pushed out of their triangles, and its segment is
stopped by their faces. A leaf never stops it.

### Walking into a building — *read*, and *measured*

Nothing enters a building by command. A unit walks in: the building's faces
are its floors and its walls, and its doors open for whoever stands on the
building near them. The capture that follows is
[27-ownership.md](27-ownership.md#capture--read)'s.

**A unit standing on a building is in the building's own collision manager**
(*read*):

1. Standing on a building's face makes the unit the building's child
   ([Finding the ground](#finding-the-ground--read), step 7). `CBuilding`
   passes adding and removing a child to its agent (`Terrain.dll:0x100569f0`,
   `0x10056a20`).
2. Adding a child tells the parent (event 2) and then the child (event 6,
   `AniMesh.dll:0x10017670`); removing one tells the parent 3 and the child 7
   (`0x10017716`). An agent passes each event on as message 21 to its mesh,
   its control system and its collision object (`IGameObject` slot 20,
   `0x10001ba0`).
3. On 6 the collision object leaves its manager and joins the first ancestor
   that answers `0x203`; on 7 it only leaves (`Control.dll:0x1001f585`). Its
   next landscape face adds it back to the landscape's.
4. A building's agent answers `0x203`. Its build makes the building a
   collision manager of its own (`AniMesh.dll:0x10003538`) and files it in the
   agent's interface table (`0x100035ef`), and `CBuilding` passes the request to
   the agent (`Terrain.dll:0x10057cf6`).

**The world's pass runs the building's** (*read*). When the building's own
collision object attaches its interfaces, it asks its owner for `0x203`, keeps
the manager as its entry's pair handler and makes itself that manager's context
entry (`Control.dll:0x1001f600`–`0x1001f616`). In the world's pass:

- a pair of the building's entry and a mover goes to the handler's slot 7
  (`0x1001c22f`), which only lists the mover as a visitor (`0x1001c500`);
- after the pairs, each entry with a handler has the handler's slot 3 run with
  the frame's time (`0x1001c2d3`): the building's own pass (`0x1001c040`),
  over its members and its visitors. Its context is the building, not the
  landscape, so every one of them goes to the ordinary pair against it
  (`0x1001c1b2`, `0x1001d630`). The building is the larger sphere, the
  obstacle, and its level-0 faces push the unit out
  ([above](#collision-between-objects--read)). Members and visitors are paired
  with each other the same way, and the visitors are forgotten at the end
  (`0x1001c2b4`).

So **a building's walls and floors push every unit on it or near it**, the one
it stands on included. A walker standing on a building takes a push that points
down whole ([above](#collision-between-objects--read), message `0x1b`), and a
bridge's deck pushes the hero crossing it.

**Doors** (*read*). `CBuilding` files each class-12 item as a door, with the
nodes its channels play (`Terrain.dll:0x100583a2`–`0x100584e8`). A door has a
state, the time it opened, a lock flag and value, and a hold.

- **It opens for a child that comes near.** A child that moves tells its
  parent (event 1: an object setting its matrix tells its parent,
  `AniMesh.dll:0x10017be1`). The building's notification (`CBuilding` slot 20,
  `0x10059f40`) first drops every door's hold. Then, for each door part, it
  takes the part's capsule from the building's mesh — two points and a radius
  (`0x1005a27f`) — and measures the child's bounding-sphere centre to the
  capsule's segment. Within the child's radius plus the capsule's, the door
  opens (`0x1005b480`) and that child holds it (`0x1005a5a0`). A door locked
  shut (the lock flag with value 1) is passed over. A child that leaves
  (event 3) drops the holds it made.
- **Its states** (`CBuilding::SendMsg`, `0x10057550`). Opening switches the
  item on. Once the item stops, the door is open and the time is kept; the
  building sees it stop when the state word clears, at 0.9 ÷ rate
  ([27-ownership.md](27-ownership.md#capture--read)). An open door that nothing
  holds and nothing locks open closes 5000 ms after it opened: the item is
  switched off, and the door is shut once it stops.
- **An open door lets units through.** The push-out asks the building, for a
  face on a door's node, whether that door is open (`IBuilding` slot 17,
  `0x1005b620`), and drops the face when it is
  (`AniMesh.dll:0x1000dd13`–`0x1000dd46`). A shut, opening or closing door
  pushes like a wall, from wherever its channel has moved its node.
- **Two more openers:** a hit struck on a door's node opens it
  (`Control.dll:0x1000ec7e`), and a hall-way link opens the doors listed on it
  (`ArealMap.dll:0x1000b170`), which is how a unit routed through a building
  gets through.

*Derived*: only the building's children are tested, so a door does not open for
a unit on the landscape outside it. It opens for a walker that stands on the
building's own floor near it, and a closed door it meets pushes it.

**What is drawn inside** (*read*, in part). Nothing is hidden by where the
camera is: the node flag the draw skips starts clear (`AniMesh.dll:0x10012407`)
and only a destroyed node sets it
([26-damage.md](26-damage.md#a-hit-from-the-round-to-the-node--read)). The
interior's coloured lamps are the building's lightmap on its lit batches
([07-objects.md](07-objects.md#baked-lighting)). How the lightmap combines with
the lit colour is not established.

**The Large Factory and the Outpost of *The Constructor*** (*measured*, in
model space with z up from the placement):

| | Large Factory, `fr_b_plant` (`lplant01.dat`) | Outpost, `fr_l_angar` (`shang01.dat`) |
|---|---|---|
| way in | door `i05` (node 3) across x −11…11 at y 87.5–89.2, z 0–15.4, behind a forecourt floor at z 0 from y 87.4 to 108.5 | no door: ramps from z 0 to 1.9 at both ends of its hall |
| doors | 3, on nodes 3, 16 and 14 (the entrance and two side doors at x ±26), rate 0.4: open in 2.5 s | none |
| floors | entrance hall `i01` at 0; the pod room at −12.4 | hall at 1.9; under the pod 0.48 |
| pod | node 25 (`i17`), radius 4.77, centre (0.06, −48.66, −9.67): zone 3.82 across | node 2 (`o03`), radius 6.37, centre (19.40, 8.79, 4.79): zone 5.10 across |
| capture fires | 4.5 s after the pod starts opening | 3 s |
| lightmap | on 100 of its 438 batches | none |

Mission 02 places the Large Factory 0.08 above the ground under it, so a unit
walking up to the door is the building's from the forecourt on.

**Against the recording** (*seen*, 30 fps). The hero crosses the forecourt at
92–94 s, is in the dark entrance at 94.5 s and under the hall's lamps at
96.0 s. It stands on the pod from 102 s at the latest, and at 106.3 s the
factory screen and "Building is captured" appear in the same frame. The
autocannon's count drops from 500 to 499 at 94.5 s, as the hero reaches the
door. A door that opens for the hero needs no shot, and the recording does not
show which opened it. Walking out, from 158 s to 166.5 s, the hero fires
nothing. Inside the Outpost, at 334–338 s, it is captured the same way.

**For an engine** — *derived*, in this order each frame:

1. A unit whose nearest walkable face is a building's is that building's
   child. The building keeps its children and their bounding-sphere centres.
2. For each child that moved: drop every door's hold. For each door not locked
   shut, for each of its nodes, if the child's centre is within
   (child radius + part radius) of the part's axis, open the door (switch its
   item on, unless already open or opening) and hold it. Until the mesh's
   capsule is read, a part's level-0 slot bounds stand in.
3. Tick every door: opening → open when its item stops (the time kept);
   open → closing when unheld, not locked open, and 5 s past opening; closing
   → shut when the item stops. A door's item steps like any other
   ([28-chassis.md](28-chassis.md)).
4. Collide: a building's faces push every unit on it or touching its sphere,
   the faces of a door that is open excepted; units on the same building push
   each other.
5. Tick the pod ([27-ownership.md](27-ownership.md#capture--read)): a child in
   the first computer's zone switches it on; when it has opened, and that child
   is still there, the capture callback runs once. For another clan it takes the
   building, shows string 5039 "Building is captured" as a System line with its
   voice, and at once opens the building for the player (a plant's page 5); for
   the player's own clan it only opens it.
6. Leaving is walking out: a door ahead opens as the unit nears it, and once a
   landscape face is nearer the unit is the landscape's again.

### What a buoy does to a walker — *read*, *measured*, and not established

The five buoys on Mission 01 — objects 25, 26, 27, 29 and 30 — are `s_tree_29`,
kind 2 at scale 1 (*measured*). Its model `s_tree_0_29` has 178 level-0
triangles at two dozen slopes. 36 are vertical, and 12 face out and up at
normal z 0.49, a horizontal part h of about 0.87. That the 0.49 faces are the
ones a walker meets head-on is *derived* from the creep measured below, which
matches that h.

**What the read steps do at it** (*read*):

- **No handler of its own.** The collision object's constructor
  (`0x1001f290`) zeroes the handler at `+0x40` (`0x1001f2c6`), and no other
  writer was found among the collision code's functions. So the pair takes the
  default path, `0x1001d630` → `0x1001daf0`.
- **The face stop is a point's segment.** Step 1's query goes through A's
  `+0x38` slot 6, `AniMesh.dll:0x10013ef0`, with two points and no radius
  (`0x1001dd24`). It stops B only when B's **centre** would cross a face.
- **The small-face stop needs class 3 or 4.** `0x1000cee0` gives a unit its
  class from the third letter of a name (`Behavior.dll:0x1000cfb1`). Which name
  it is handed is not traced. `tut1_p` gives `t`, 1, and `r_h_02` gives `h`,
  2, so the hero is 2 at most.
- **The push-out is flattened.** Step 3 pushes B's sphere out along each face
  from the face's closest point. The machine then drops the push's z and scales
  x and y back up to its length, at most ×4 (`0x1000ca44`), and keeps its
  velocity.

**So a sloped face gives way** (*derived*). A walker moves a step s straight
into a face whose normal has horizontal part h. The sphere sinks s × h into the
face's plane, and the push out has that length. Flattened, all of that length
points back along the face's horizontal normal. The walker is left s × (1 − h)
inside the face. A vertical face (h = 1) stops it dead. The buoy's cone
(h ≈ 0.87) lets it creep 13% of every step, and any glancing contact adds a
sideways part that slides it round. The centre meets a face, and step 1 stops
it, only after about 2.18 ÷ 0.06 ≈ 36 such steps.

**On openparkan's engine**, which follows the steps above (*measured*): the hero
walked head-on into buoy 26 moves in 0.06 for each 0.46 step. A sideways push
of about 0.05 a step grows until it slides past.

**Not established.** No read step stops a class-2 walker at a small sloped
object. Whether the original game lets the hero through a buoy has not been
seen here. What could stop it, all unconfirmed:

- a handler set at `+0x40` by code outside the functions searched;
- a class of 3 or more reaching message `0x201`;
- a stop that tests the swept sphere rather than the centre's segment.

### The map edge — *read*

A moving object that is not a round is kept inside the world's box
(`0x1001e650`, from `IWorld` slot 10's box query at `0x1001e664`):

- **hard:** past the box's x and y sides inset by the sphere's radius, or
  more than 20 above its top, it is pushed back to the boundary; nothing
  pushes it up from below;
- **soft:** within 80 of an inset side it is pushed back by
  3 × 0.0125 × min(depth, 80), and above the top by 3 × 0.05 × min(depth, 20)
  (`0x1001e7ba`).

Both add to the contact record's push with flag `0x10`, which the machine
takes like any other push. A round meets the same box with its top doubled
and is removed ([26-damage.md](26-damage.md#the-hit-test--read-and-measured)).

### Jumping — *read*

There is none. `CanJump` is bound from `chas_wlk.var` into `MBehaviour`
`+0x7d0` (`Behavior.dll:0x100176bf`) and never read: of the 27 calls to the
profile getter (`0x10014670`), the fields read after them are `CanFly` (+0xc)
and `WalkChassis` (+0x18), never +0x10, and nothing else addresses the
profile. No `MCMD_` command jumps, and iron3d's only `jump` is a camera-path
edge type.

### Gravity — *read*, and *measured*

`CWorld` sets gravity to **10.0** at `+0xc` when it is built
(`Terrain.dll:0x10024c1a`); slot 4 returns it and slot 5 sets it. Only a
mode-3 controller uses it: the velocity integrator adds −g along world z,
turned into the machine's frame (`0x10015879`). The six mode-3 controllers are
the four `bf_*_01` rounds in `weapon.rlb` and the two hero targets `r_h_01`
and `r_h_03`. **Modes 0 and 2 — every other machine, the hero `r_h_02`
included — have no gravity term in the integrator.** Gravity reaches them
through the ground contact instead: a state with contact points falls at the
same 10.0 until a contact lands
([Holding the body on the ground](#holding-the-body-on-the-ground--read-and-measured)).

**Nothing changes the 10.0** — *read*, as a search.

- **Who holds a world.** Terrain.dll's `GetWorld` is imported by
  `Control.dll`, `World3D.dll`, `iron3d.dll`, `AniMesh.dll`, `ai.dll`,
  `Effect.dll`, `ArealMap.dll` and `Behavior.dll`. `Behavior.dll` never
  calls its import.
- **The calls on it.** Where a module keeps the result, the calls through it
  were followed: `World3D.dll` only releases it, `ai.dll` calls slot 8, and
  `Control.dll`'s calls through a field at `+0x44` reach slots 3, 4 and 6 to
  11, never 5.
- **Slot 5.** It takes the world and a pointer to one float, and pops 8
  (`Terrain.dll:0x10026b00`). Every indirect call at `+0x14` with exactly two
  pushes was listed across all 16 modules: 117 sites. None is on a world
  pointer. The same scan at `+0x10` returns the one known gravity read,
  `Control.dll:0x10015887`, which is the control.

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
`+0x5fc`): a go order at `Go_SpeedPercent` (`0x1002b59b`). The compiled
defaults (`0x10016250`) are 1 for going, building, transport and patrolling a
unit; 0.8 for patrolling a place and for pathfinding near a building; 80 for
patrolling a building (sic); `Attack_MinAttackSpeedPercent` 0.7 with `Del`
0.3, `MinNearing` 0.8 with `Del` 0.2; `Movement_InsideBuilding_Speed` 3;
`Movement_SpeedPercent` and `Movement_MinSpeedPercent` 1;
`Movement_MaxSpeed` 600. The mission property `MaxSpeedPercent` reads back
`Movement_SpeedPercent` and its setter does nothing (`0x1000b621`). `ai.dll`
asks for the live top speed (IControl 145) and compares it with 1
(`0x100091b0`).

**`+0x5fc` is the live block's forward top speed.**

- **When it is taken.** Every unit takt starts by copying six live figures
  (`Behavior.dll:0x10005133` into `0x1001bbe0`). The source is the answer to
  IControl slot 13 with id `0x12`, which is the live parameter block at
  control `+0x470` (`Control.dll:0x1000dd6f`).
- **What is copied**, twice over (once at `+0x5fc`, again at `+0x614`):

  | behaviour | live block | what |
  |---|---|---|
  | `+0x5fc` | `+0x1c` (file +48) | top speed, forward |
  | `+0x600` | `+0x10` (file +36) | triple 2, forward |
  | `+0x604`, `+0x608` | `+0x18`, `+0x20` | top speed, x and z |
  | `+0x60c` | `+0x2c` (file +64) | turn rate about z |
  | `+0x610` | `+0x04` | acceleration, forward (twice the file's) |

- **So the base is the limited speed.** It is the authored top × G × E ×
  (1 + r) ÷ 2, not the authored top.

**`Speed_MaximumFactor` caps what the AI asks the walker for.**

- **Where.** An order hands its speed to `MWalker::SetTarget`
  (`Behavior.dll:0x1003bad0`, called from 15 places). It holds the speed in
  turn (the cap is at `0x1003be4f`):
  1. to at most the live forward top × `Movement_SpeedPercent` ×
     `Speed_MaximumFactor`;
  2. to at most `Movement_MaxSpeed`, 600;
  3. to at least triple 2's forward component × `Movement_MinSpeedPercent`
     (`0x1003bed0`);
  4. to at least 2 m/s (`0x1003bf05`).
- **Where the factor lives.** It is the first float of the difficulty profile
  at the behaviour's `+0x8d4` ([26-damage.md](26-damage.md)).
- **What it does.** `diff_slow.var` sets it to 0.7 where the other four
  difficulties set 1 (*measured*), so on that difficulty an AI unit is driven
  at no more than 70% of what its engines and load allow — *derived*.
- **Triple 2's forward component is the walker's floor.** It is 0.6 on the
  walkers, the Transformer and the hero, 0.49 on the Tiny Spider, 0.1 on the
  Small Tower and the two targets, and 0 on every wheeled, tracked and flying
  chassis. That is under the 2 m/s floor either way — *measured*.

## How the AI drives a machine — *read*, and *measured*

The AI presses no keys. A task gives `MWalker` (`MBehaviour` `+0xb8`) a place
and a speed. The walker plans and cuts the plan into timed **movement points**.
It hands them to the unit's interface `0x201`: the `Wizard.dll` object that
also decides who drives ([29-weapons.md](29-weapons.md)). The Wizard follows
the points and writes the machine's velocity and spin.

**The walker** (`Behavior.dll`):

- **The global path** (`0x1003fbf0`). Two places on the same map object
  (place `+0x1c`) and in the same areal (`+0x20`) need no path. Otherwise the
  clan areal map's slot 7 names the map object, and that object's interface
  `0x303` slot 14 returns the areals from start to goal. Each areal after the
  first goes into the global queue `+0xe8`. A count of 0 or less fails
  quietly. Places on different map objects log "Cannot Generate GlobalPath".
  The local queue `+0xfc` and the trajectory `+0x110` (records of `0x48`
  bytes) are built from it.
- **The search** is `ArealMap.dll`'s `MGraph`, not read. `MHallWay` answers
  `0x303` (`ArealMap.dll:0x1000aab0`), and `FormPath` walks back through a
  chain it guards against cycles (`0x10017265`).
- **Each takt** (`0x1003d280`) looks up the unit's areal ("is out of
  ArealMap" when there is none). It grows the trajectory to at least
  `PathFind_MinPointInTrajectory`, 3 points, in at most six tries.
- **The hand-over** (`0x1003d960`) sends the Wizard every record past the
  Wizard's own count, through slot 17. It panics past 300. A point holds a
  position, a velocity, a heading, a time in ms and flags. Bit 0 comes from
  the record's byte `+0x44`. **`0x3030` is added unless the chassis profile
  has `CanFly` and not `WalkChassis`** (`0x1003daab`).
- **A stop** (`0x1003d7f0`) goes through slot 19. It is the last point's
  position plus 0.5 × its velocity, at velocity 0, 1000 ms later, with flags
  `0x3031` or 1. From v to 0 over a second covering 0.5 v is an even
  deceleration, so the easing is in the points — *derived*.
- **A clear** (`0x1003c540`) empties the three queues and sets `+0xb4`.

**The point flags** apply to the velocity in the machine's frame, one nibble
per axis: x at bit 4, y at bit 8, z at bit 12 (`Wizard.dll:0x10003750`).

| nibble | the axis |
|---:|---|
| 1 | never negative |
| 2 | never positive |
| 3 | 0 |
| 5, 6 | + or − the live top speed on that axis |

So `0x3030` leaves only forward and back: a walker, wheel or track neither
side-slips nor climbs by its points. `0x3331` zeroes all three axes.

**The Wizard's takt** (`Wizard.dll:0x10001d50`):

1. **Inputs.** dt is IControl property 101 (ms) × 0.001. The live parameter
   block comes from property `0x12`, and the matrix from `IGameObject` slot 8.
2. **Who drives.** It follows only while `+0x1f7` is set. That is off when
   the unit's own group word `+0x200` gives it to the player: mode 1 with
   word 0, or word 3 (`0x10003c25`). With mode 1 and word 0 the refresh also
   empties the Wizard's points (`0x10003c12`).
3. **A new segment** (`0x10002c80`) starts when the last one is due. It drops
   the points whose time has passed. It fits a cubic from the machine's
   position and the last velocity to the next point's position and velocity,
   over the time between (the 3 and 2 at `0x100030cc`, `0x1000310d`). With no
   point left it heads for the stop point if there is one. Otherwise it holds:
   velocity 0 and flags `0x3331`, 1000 ms at a time (`0x10002f2d`).
4. **Sampling** (`0x100031a0`) takes the curve's velocity at t + dt/2 and its
   heading at t + dt. With flag bit 0 the heading is a straight blend.
5. **The writes** (`0x10003750`), only while `+0x1f8` is set (1 from the
   constructor; `0x10002b00` sets it):
   - **velocity**: the sampled velocity in the machine's frame, through the
     flags, into IControl slot 5, `SetTangSpeed` (`+0x1c8`,
     `Control.dll:0x100044a0`);
   - **spin**: (0, 0, s) into slot 4 (`+0x1d4`, `0x10004440`). s is the
     signed yaw angle from the machine's forward axis to the heading, ÷ (dt ×
     the live yaw turn rate), held to ±1 (`0x100034c0`).

   It never writes the command `+0x1bc`, the pending turn `+0x1e0` or the
   strafe angle.

**Flying** — *measured*, then *derived*. `CanFly` without `WalkChassis` is on
`chas_fly.var` alone, of the six chassis profiles. So only a flyer's points
keep x and z, and only a flyer strafes and climbs along its path. Mission 01's
two neutral warriors both fly:

| unit | chassis | top speed x, y, z (m/s) | yaw rate |
|---|---|---|---:|
| `helic.dat` | `r_t_02` Tiny Helicopter, `chas_fly` | 20, 33.3, 35 | 4.0 |
| `tut1_mf1.dat` | `r_m_02` Medium Flying, `chas_fly` | 4, 34.7, 20 | 3.8 |
| `tut1_e1.dat` | `r_t_01` Tiny Spider, `chas_wlk` | 10, 26.4, 1 | 3.5 |

All three controllers are mode 0, with no gravity term
([Gravity](#gravity--read-and-measured)). Moving a flyer to a point needs
nothing more than a walker does: timed points with flags 0 or 1, the cubic's
velocity in the machine's frame each tick, and the yaw spin. The height its
points are given is not traced. `Movement_FlyHeight` is 40 and
`FlyNearLandHeight` 15, by name only.

## Not established

- How the velocity integrator's pull toward *command × top speed*, with the
  command left at 0, combines with a velocity the Wizard writes every frame;
  and whether the Wizard's spin, held to ±1, is a rate or a fraction (a key
  sends +0.7). A stand-in takes the written velocity as the machine's own, and
  s × the live yaw rate as its turn.
- The areal search (`MGraph`: algorithm, costs, and what the land answers for
  `0x303`), the local path and its obstacle contours, the Wizard's heading
  curve (`0x10003d80`), and who reads `Movement_FlyHeight`. A stand-in
  searches the areal adjacency of [08-arealmap.md](08-arealmap.md) by A*, with
  straight lines between edge midpoints.
- Whether the walker's clear (`0x1003c540`) also empties the points the Wizard
  already holds. `ClearWizardPath` is logged at `0x10040e3b`.

- ~~How interface `0x25` slot 3 turns an object's level-0 triangles into a
  push, and what slot 2 does with its 0.5~~ — **read**: the push accumulates
  over the unhidden faces within the sphere, nearest first, and is held to 4r;
  the 0.5 is how far off a triangle, in x and y, a point may still find it
  ([Collision between objects](#collision-between-objects--read),
  [Finding the ground](#finding-the-ground--read)). `0x1000e900` is now read
  too: a face in front of the centre within r, from its closest point. Whether
  any `Terrain.dll` class answers `0x25` is not checked.
- ~~Which scene nodes are types 1 and 3~~ — **read** for buildings: a
  `CBuilding`'s slot 11 is 3, so a bridge is ground. That a unit's and a tree's
  slot 11 give their collision kinds 4 and 10, so they are not ground, is
  *derived*. Still open: what class message
  `0x201` returns through a mover's interface `0x10` (the size class is a
  *guess*; that class is read from a name's third letter, `Behavior.dll:0x1000cfb1`,
  but which name is not traced), the masses property `0x7c` gives a unit and a
  static object, and who sets a collision entry's skip flag 1.
- **What stops a walker at a small sloped object**, such as a buoy. No read
  step does
  ([What a buoy does to a walker](#what-a-buoy-does-to-a-walker--read-measured-and-not-established)).
  Nor is it known who, if anyone, sets a collision object's pair handler
  (`+0x40`).
- Who sets the machine's counter `+0xd4`, which runs the life update — and so
  the ground damage — on every tick
  ([Water and lava beds kill](#water-and-lava-beds-kill--read-and-measured)).
  Whether a flying machine runs the ground contact at all.
- ~~Whether `PlaceObjectOnWorldFace`'s reparenting sends a collision object its
  message 21, so that a machine on a building's deck leaves the world's
  collision manager for the building's; and so whether a bridge's own faces
  ever push a hero standing on it~~ — **read**: it does, and the world's pass
  runs the building's, so they do
  ([Walking into a building](#walking-into-a-building--read-and-measured)).
- How the building's mesh builds the capsule a door part is tested against
  (`Terrain.dll:0x1005a27f`), the node whose box bounds a pod's zone in height
  (`0x10058607`), and how a lightmap combines with a batch's lit colour.
- How a walker climbs a building's ramp while the ramp's own faces push its
  sphere back.
- Which `Land.msh` faces carry the world face bit `0x8` and class bit 8 that
  the ground search excludes; the landscape converts them to its own mask at
  `Terrain.dll:0x10022da0` (world `0x8` → `0x20`, `0x200` → `0x20000`,
  `0x400` → `0x2000`).
- The contact records' flag 2, which hands the contact to the object's
  interface slot `0x7c` (`0x1001affd`), and flag `0x20`. Flag `0x1000` runs the
  record's group once while its node stands (`0x1001b08f`); no shipped record
  sets it.
- ~~Where G would ever differ from 1~~ — answered: nowhere but a `MAT0`
  record; the constructor and the ground contact are its only writers
  ([Ground and slope](#ground-and-slope--read)).
- ~~How the strafe angle's turn is split between the hull and the turret, and
  what `MCMD_FORWARD`'s ramp does~~ — **read**: the hull takes the whole
  change and the turret an offset that undoes it across the step; the ramp is
  a growing step per input update
  ([From input to motion](#from-input-to-motion--read-and-measured)).
- How often `World3D.dll`'s input update (`0x1000f100`, the manager's slot 4)
  runs, which sets how fast the keypad cruise ramps.
- ~~Which node range the payload sum counts as the chassis~~ — answered: part
  0's nodes, the root object's ([Load](#load--read-and-measured)).
- ~~Triples 5 and 6~~ — answered: 6 is the most the hull leans
  ([13-control.md](13-control.md#the-lean-and-triple-6--read-and-measured))
  and 5 how fast it rights itself
  ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)).
  Still open there: who writes the vector at control `+0x348` that bits
  `0x10` and `0x20` right the hull toward.
- ~~Whether any module calls `CWorld` slot 5~~ — answered: none does
  ([Gravity](#gravity--read-and-measured)).
- ~~Where `Speed_MaximumFactor` is applied, and the `+0x5fc` speed base~~ —
  answered: it caps an order's speed in `MWalker::SetTarget`, and the base is
  the live forward top speed
  ([How the AI asks for speed](#how-the-ai-asks-for-speed--read)).
- ~~Whether a module outside `Control.dll` stops a machine whose engines are
  unpowered~~ — answered: none reads an engine's power level; the AI stops
  driving a unit whose live top speed falls to its floor
  ([What sets the live limits](#what-sets-the-live-limits--read)).
- What behaviour flag `0x800` (`+0xa04`) changes besides clearing the walker.
  Triple 2 is never read inside `Control.dll` — nothing reaches +32..+40 in
  either copy of the block ([13-control.md](13-control.md)) — and the AI reads
  its forward component as a floor.
