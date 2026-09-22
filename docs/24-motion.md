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
| +0x94 | a **use count**: how many times the planner may still choose this state, −1 for unlimited ([below](#a-states-use-count-and-its-request-code--read-and-measured)) |
| +0x98 | the **request code** the state waits for, −1 for any ([below](#a-states-use-count-and-its-request-code--read-and-measured)) |
| 16 × B | the **contacts**, a foot, wheel or leg each: a control point, flags, the group run when it lands. `0x100` makes the state need that point's node intact, `0x200` destroyed (`0x10001107`) — a walker's limping states; `0x2` lays the point's carrier node on the ground ([13-control.md](13-control.md#section-1s-conditions-are-contacts--read-and-measured)) |

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
  held to 0.01–5 s (`0x100057d6`). The step gains
  *step × 0.25 × (r ÷ 65536 − 0.5)*, with r the draw
  ([below](#the-jitter-draws-from-a-pair-of-16-bit-words--read)).
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
  and that anchor's use count `+0x94` goes down by one unless it is −1 or
  already 0 ([below](#a-states-use-count-and-its-request-code--read-and-measured)).
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

### The state a machine starts in — *read*

The constructor (`0x10006bf0`) puts the state **index** at 0 (`0x10006d19`) and
zeroes the current-state copy at `+0x100`, except that it writes `0x10001` into
its `+0x104` — anchor, and moves by velocity (`0x10006d43`, `0x10006d49`). So
the record the machine begins with is not a state of the file at all, and its
own use count, zero, stops it applying ([below](#a-states-use-count-and-its-request-code--read-and-measured)).
On the first tick the planner therefore plans from index **0** to the cheapest
anchor that applies, and the machine plays the path to it. The loader writes
neither field. *Read*, enumerating every access to the two displacements in the
module: `+0xfc` is written by the constructor and by the pop off the queue
(`0x1000c317`, `0x1000cc1a`), and `+0x100` by the constructor alone. The one
other write to each (`0x10029584`, `0x10029562`) is in another class's
constructor, whose vtable is `0x1003cbd8`.

So "the machine starts in state 0" is right as the graph's **source**, and
wrong as the state it plays: the first state it plays is whichever anchor the
plan reaches.

### A state's use count and its request code — *read*, and *measured*

The last two dwords of the 156-byte record are the two things that can stop a
state applying, and the applicability test reads them one after the other
(`0x1000112c`–`0x10001147`):

- **`+0x94`, the use count.** A state whose count is **0 does not apply**, ever
  again. The planner spends one each time it picks a state as its destination
  anchor: −1 is left alone, 0 is left alone, anything else goes down by one
  (`0x100052fd`–`0x1000530f`). Nothing else writes the field, so a finite count
  is a state the machine may take that many times in its life.
- **`+0x98`, the request code.** A state applies when its code equals the
  controller's current one, or when the state's own is −1
  (`0x10001136`–`0x10001147`). The controller's is at `+0x1ac`, `IControl`
  slot 19 sets it (`0x10004800`), and **the constructor starts it at 0**
  (`0x10006ecf`, where `ebx` is the zero the constructor clears at
  `0x10006bfc`) — not at −1.

*Measured*, over the install's 1,690 states:

| `+0x94` | states | where |
|---:|---:|---|
| −1 | 1,607 | everything else |
| 30 | 80 | 15 `s_stn_a_*` stones and 65 `s_tree_a_*` trees |
| 20 | 2 | `bm_b_04` and `bm_m_04` in `weapon.rlb` |
| 10 | 1 | `mtcheck` in `system.rlb` |

All 83 finite counts sit on **anchors**, which is the only place the planner
spends one, and every one of them is its controller's state 0. The 80 in
`static.rlb` are the whole of a **one-state** controller, each a 500 ms step, so
the count is a lifetime: 30 steps, 15 seconds, and then the only anchor there is
no longer applies and the machine stops planning. `mtcheck` has two states and
the two `bm_*_04` rounds three.

*Measured* for `+0x98`: 1,510 states ask for no code; the other 180 are the
`fortif.rlb` building controllers' — 30 controllers of 14 states each, with
codes 0, 1, 2, 6, 8 and 10 opening exactly one state apiece
([32-builder.md](32-builder.md#the-construction-sphere--read-and-measured)).
Since the controller starts at 0, **the state a finished building applies from
the start is the code-0 one** — the state that stops the construction ray — and
the other five wait for the construction task to send their code.

### The jitter draws from a pair of 16-bit words — *read*

The generator bit `0x1000000` draws from is inlined at the jitter itself
(`0x100057de`–`0x1000582f`), over two 16-bit words laid side by side in one
dword at `0x10042230`:

    s₀ ← (s₀ << 1) xor s₁        then        s₁ ← (s₁ >> 1) xor s₀

and the draw r is the new s₁, an integer 0–65535, scaled by 1/65536 (the float
at `0x1003b374`; the 0.25 at `0x1003b378` is the ±12.5%). It is **not**
`rand()` — no call is made at all — and not an xorshift over 32 bits.

**It is seeded, once, at load.** A static initialiser at `0x10006330` writes
the whole dword from `ngiGetClocks` (`NGI32.dll` ordinal 52, `0x100045b0`:
`QueryPerformanceCounter`, or `rdtsc` when the module's flag `0x20000` is set),
and it is reached through the module's `_initterm` table — the pointer sits at
`0x1003e160`, inside the range `0x1003e000`–`0x1003ed78` that `0x10034166`
walks. That matters: **all-zero is a fixed point of the recurrence**, so
without the seed every jittering step would come out at exactly −12.5%.

The same generator is inlined a second time, with its own words at
`0x10043228` and its own seeder at `0x1000dc20`, in the machine tick
(`0x1000c74a`) — there it jitters the interval between life updates, not a
step.

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
    stops driving it — and stops ordering it at all
    ([below](#flag-0x800-is-the-units-leave-to-be-ordered--read-and-measured)).
  - Engines and running gear shot to nothing trip that. A flat battery does
    not, because it never lowers the top speed.
- **A robot has one engine, and its mark caps its speed.** The fitted engine replaces
  the chassis slot's drive of 1 ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)),
  so E is its drive, 0.7 on a Mk1 to 1.0 on a Mk4, times its condition and the gear.
  An empty robot on flat ground reaches at most that fraction of the chassis's
  authored top speed, and turns that much slower — *derived*.
- **Load halves top speed at most**: a machine carrying its full payload runs
  at half speed on one healthy engine.
- **The body's damage is the engine's.** An engine's condition is the life
  left in its model node, over its maximum
  ([23-economy.md](23-economy.md), "What a value id is"). **Every chassis's
  engine sits on node 0** (*measured*, 24 of 24 `bases.rlb` robot controllers),
  and a fitted engine keeps the slot's node
  ([28-chassis.md](28-chassis.md#a-fitted-part-takes-over-its-slot--read-and-measured)).
  So a robot whose body is down to half its hit points has half its E: it
  runs at half speed and turns at half its rate. The hero too, whose built-in
  engine sits on its body node (*derived*).

### Flag `0x800` is the unit's leave to be ordered — *read*, and *measured*

Three routines in the whole install touch bit `0x800` of the behaviour's flag
word `+0xa04` — the takt that keeps it, and two that read it — and the
enumeration is the answer.
**Every access to `+0xa04` anywhere in the fifteen modules is in
`Behavior.dll`** — 51 of them, all at that one displacement; the only other
hit in the install is a `lea` of a log buffer in `Effect.dll`. Bit `0x800` is
at **five** of the 51:

| where | what |
|---|---|
| `0x100051e9` (`or ch, 8`), `MBehaviour` vtable slot 56, the unit takt `0x10005110` | **sets** it while the copied live forward top speed `+0x614` is at least 0.5 (`0x100595c0`) and above triple 2's forward `+0x600` |
| `0x10005295` (`test ah, 8`), `0x100052b0` (`and ah, 0xf7`), the same takt | **clears** it once that fails, and, only if it had been set, empties the walker's three queues (`0x1000528f`, `0x1003dc90`) |
| `0x10004b25` (`test ah, 8`), **`MBehaviour::AddOrder`**, vtable slot 3 (`0x10004a90`, vtable `0x10059250` installed at `0x10003bdd`) | without it the order is **refused**: `AddOrder` returns 0 and never reaches `MBehaviour::MakeNewOrder` (slot 28, `0x10004280`, which names itself in its own log) |
| `0x10034579` (`test ch, 8`), the self-given task factory `0x10034510` | without it **no task is built**: the six-way switch on the reason is never reached, so no `M_Task_Attack` and no `M_Task_Reload` ([31-packages.md](31-packages.md#between-orders--read)) |

**Both readers let a building through.** Each falls back on the object's Type,
`MBehaviour` `+0xafc` through slot 14 (`0x10008c50`): a Type carrying
`0x80000000`, `CLASS_BUILDING`, passes with the flag clear (`0x10004b37`,
`0x1003458b`). The task factory tests the live top speed against 0.5 itself
first (`0x10034539`), with the same escape.

So the flag is not a detail of the walker: **it is the unit's leave to be
ordered at all.** A unit that has lost it refuses every order `ai.dll`'s
script function 15 gives it — which the scripts read as *refused* and mark the
problem solved ([23-economy.md](23-economy.md)) — and never engages,
retaliates, answers a call for help or goes to refit.

**The control** is the same sweep at the word's other bits, which finds their
readers: `0x1` and `0x2`, the initialisation flags whose tests log
*"GMSG_INTERNAL_INIT … twise"* and *"Invalid initialization messages"*
(7 and 12 tests, set at `0x10005e51` and `0x10005fcb`); `0x4` (11 tests);
`0x10` (3 tests and 2 sign-extracts, `shl 0x1b; sar 0x1f`); `0x20`; `0x40`;
and `0x1000` (2 tests, set at `0x10004d79` and `0x10008cce`). No module
outside `Behavior.dll` reaches the word, through the plain displacement or
through the second interface's (`+0xc`, so `+0x9f8`, which is a float field of
the object's own).

*Measured*, over the 531 controllers: the live forward top is the authored
top × G × E × (1 + r) ÷ 2 **capped at the authored top**, so a controller
whose authored forward top is at or below max(0.5, triple 2's forward) can
never carry the flag. **94 of the 531 can; 437 cannot** — and of those 437,
30 are `fortif.rlb` buildings, which the Type escape covers, and the rest are
guns, internal parts and scenery with no behaviour of their own. Of the 24
`bases.rlb` chassis, **21 can and three cannot**: `r_h_01` and `r_h_03`, the
two shooting-range targets, and `r_b_06`, the Small Tower, all three authored
at a forward top of 0.2 against a floor of 0.1.

- **Those three are fielded.** 18 of the shipped assemblies sit on them, and
  the missions place **33**: `l_targ` ×3 and `m_targ` ×2 on *Line of Fire*,
  and 28 Small Towers across C01 M03, C02 M02, C03 M01/M03/M04, C04 M02 and
  `Multi.05`. Their Type is `0x01008000`, a warrior, not a building, so the
  escape does not apply: **a Small Tower can never be given an order and never
  takes a task of its own.** What it does to a passing hero is its guns' free
  fire, which is not on this path ([29-weapons.md](29-weapons.md)).
- **For everything that moves the flag is on.** A chassis loses it only once
  E × G × (1 + r) ÷ 2 falls to between 0.0113 (`r_l_02`) and 0.0429
  (`r_h_02`, the hero) — under 5% of its authored top speed, which takes both
  sides of the running gear or the engine gone. So on a healthy machine the
  flag changes nothing a player would see; on a wreck it is what stops the AI
  giving it anything more to do.

### What shipped units get — *measured*, then *derived*

The table puts the tutorial missions' units through the formula: the fitted
engine's drive, the load of [below](#load--read-and-measured), undamaged, on
ground factor 1. The top speed is forward, in m/s.

| unit | chassis | E | r | top speed, authored → live | yaw rate, authored → live |
|---|---|---:|---:|---|---|
| the hero, every mission | `r_h_02` | 1.0 | 0.9997 | 14.0 → 13.998 | 25.12 → 50.23 |
| Mission 01's `tut1_e1` | `r_t_01` | 0.7 | 0.188 | 26.39 → 10.97 | 3.5 → 1.92 |
| Mission 01's `helic` | `r_t_02` | 0.8 | 0.059 | 33.33 → 14.13 | 4.0 → 1.89 |
| Mission 01's `tut1_mf1` | `r_m_02` | 1.0 | 0.096 | 34.72 → 19.03 | 3.8 → 2.45 |
| Mission 03's enemy flyers `tut3_f1`, `f2`, `f3` | `r_l_02` | 0.8 | 0.003, 0.127, 0.047 | 44.44 → 17.84, 20.03, 18.62 | 6.5 → 2.63, 3.59, 2.97 |
| Mission 03's builder `tut3_b` | `r_l_03` | 1.0 | 0.367 | 33.33 → 22.79 | 6.0 → 6.30 |
| Mission 03's transport `tut3_t` | `r_l_03` | 1.0 | 0.439 | 33.33 → 23.98 | 6.0 → 6.95 |
| Mission 04's HQ `tut4_hq` | `r_b_03` | 1.0 | 0.114 | 26.39 → 14.70 | 2.28 → 1.53 |
| Mission 04's helicopter `tut4_f1` | `r_t_02` | 0.8 | 0.059 | 33.33 → 14.13 | 4.0 → 1.89 |

So most warbots run at under half their chassis's "Max speed". The 24 m/s of
Mission 03's transport is the one [23-economy.md](23-economy.md) times its
round with. A driven hull follows its turret at 0.7 × the live yaw rate
([30-turrets.md](30-turrets.md#the-hull-follows-the-turret--read-and-measured)).

## From input to motion — *read*, and *measured*

**Walking.** `W` sends `MCMD_WALK_F` 1 and `S` sends `MCMD_WALK_B` −1.

- Both set the command's y to ±1, so the hero walks at the full live top
  speed: 14 m/s, `r_h_02` triple 3 (*measured*).
- Release sends 0.
- `World3D.dll` tracks what is held in flags at the input context's
  `+0x20`/`+0x24` (W, S), and in global `0x10795244`, "walking".

**Strafing** is `A`/`D`, `MCMD_LEFT`/`RIGHT` (`0x10010350`, `0x10010457`). It
is not the command's x axis. Each strafe key keeps a held flag (globals
`0x1079523c` left, `0x10795240` right).

- **Going down** sets the command's y to ±1 by the sign of the y it finds, +1
  from standing, and hands `SetStrafeAngle` f × π/2 for left and −f × π/2 for
  right (`0x10020b60`, `0x10020b58`). f is 1 standing, and while walking ½
  with the sign of that y: +½ walking forward, **−½ backing up**, and 1 again
  within ±0.001 of 0 (`0x10020b5c`, `0x10020250`).
- **Coming up** sets the angle to the other strafe key's, worked out the same
  way, or to 0 when it is up too, and the command's y to 0 when neither strafe
  key is held and nothing walks.
- **So backing up mirrors the angle.** `S` then `A` turns the hull π/4 to the
  right and walks it backwards: back and to the left, 225° clockwise from the
  heading. `S` then `D` goes back and to the right, 135°. The player sees the
  same in the game (*seen*, `user-feedback` on Mission 01).

**A walk key while strafing** (`0x100101b2`, `0x100102b9`). The handler first
keeps W's and S's flags. A walk key coming up while the other is still held
sets y to that key's ±1 and does nothing more (`0x10010215`, `0x10010250`).
Otherwise "walking" becomes |magnitude| ≥ 1e-4 (`0x10020b68`), and:

- with no strafe key held, y takes the row's value, as above;
- with one held and walking, y becomes the sign of the magnitude and the angle
  ±π/4 by it, negated for right: `A` and then `S` also turns the hull right;
- with one held and not walking, the walk key's release, the angle becomes
  ±π/2 by the sign of the y left standing, negated for right, and y is left
  alone, so the strafe keeps going: `W` coming up under `A` strafes left at
  full speed, and `S` coming up under `A` backs the hull, turned right, to the
  left.

Both handlers run a key's row once (docs/14, [A row that stays
down](14-controls.md#a-row-that-stays-down--read)), so what is held is worked
out only as each key goes down or comes up.

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
- **A pressed key's row with a ramp time stays active**, and every active row
  is run again on each input update (`World3D.dll:0x1000f477`). The key coming
  up clears it ([14-controls.md](14-controls.md#a-row-that-stays-down--read)).
  With a strafe key held the run sets y to the sign of the magnitude and the
  strafe angle to ±π/4, as a walk key does.
- **Each run moves y toward ±1** by 0.05 × min(1, held ms ÷ 1000), and never
  past it.
- **So the ramp is a step per update, not per second.** The step grows over
  the first second held and is 0.05 after that. A stopped cruise reaches full
  in 31 runs. Letting go leaves y where it got to.
- **The update runs once a game frame, uncapped** (*read*,
  [14-controls.md](14-controls.md#a-row-that-stays-down--read)). The mission
  loop calculates and renders once a pass with nothing but a `Sleep(0)` at its
  end; `stdCalculateGame` sends every object message 1 with the game clock,
  and the Wizard hands that on to the manual manager, whose slot 2 runs the
  update. So the 31 runs take 31 frames: about half a second at 60 a second
  and a second at 30, and **the keypad ramps faster the faster the game
  draws**. The manager passes the update over while the clock stands on the
  same whole millisecond (`World3D.dll:0x1000ec90`), which is the only cap on
  it.

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
| `0x8405` | the Tiny Helicopter T-2 | forward speed ÷ top: nose down | −(sideways speed ÷ top): banks the way it slides |

The three readings in that column are confirmed by the turn's sense, which is
*read* [below](#the-hull-leans-and-rights-itself--read-and-measured): a
positive pitch tips the nose **down**, a positive roll the top to the **left**,
a positive yaw the nose to the **left**. A flyer's `0x03` roll is therefore a
left turn's positive yaw tipping the top left — into the turn — and a wheeled
chassis's negated `0x83` tips the top the other way, out of it.

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
- **Bits `0x10` and `0x20`** aim instead at the vector at `+0x348`
  (`0x1000c439`). **That vector is the averaged ground normal** (*read*). The
  reason no writer turned up is arithmetic: the motion body sits at control
  `+0x1b4`, so control `+0x348` is body **`+0x194`**, and the lift writes it
  through the body's own `this` (`0x10015e55`, in `0x10015d60`, called with
  `lea ecx, [esi+0x1b4]` at `0x1001b440`). The body's `+0x194` has exactly two
  writers — the body constructor (`0x1001432c`) and that lift — and two
  readers, the mode-2 slope brake (`0x100156c6`) and this righting. The same
  arithmetic already underlies two rows of
  [The pieces](#the-pieces--read): `+0x21c` is body `+0x68` and `+0x254` is
  body `+0xa0`.
  - So a machine with bits `0x30` **stands its hull along the ground it last
    landed on**: the face under the body sphere and each flag-1 contact's,
    averaged over their count plus one, refreshed every frame the lift is
    taken ([Holding the body](#holding-the-body-on-the-ground--read-and-measured)).
    With nothing under it the average is the default (0, 0, 1), so the hull
    stands upright.
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

So the wheeled and tracked chassis follow the slope and the rest stand upright
— *read* now that `+0x348` is the averaged ground normal, where this page once
called it *derived* from a guess.

**Which way a positive angle turns the hull** (*read*). The step's turn triple
— spin × dt plus triple 5 × the settle angles, added (`0x10014b83`) — is built
into three axis-angle quaternions about y, x and z (`0x100141c0`, axes pushed
at `0x10014d80`, `0x10014da1`, `0x10014dc0`), each q = (cos a/2, sin a/2 ×
axis) from `ngiGetSinCos`. `g_FastProc` slot `+0x3c` turns that into a matrix
(`Ngi32.dll:0x10014450`) and it is applied row by row (slot `+0x1c`,
`0x1001f790`). Reading the matrix out, it is **S · R · S with S = diag(1, 1,
−1)**: `M01 = 2xy − 2wz` and `M10 = 2xy + 2wz` as a right-handed R has them,
but `M02`, `M12`, `M20` and `M21` all carry the opposite sign
(`0x100144e9`, `0x1001450c`, `0x10014517`, `0x10014522`). So

| a positive angle | about | turns |
|---|---|---|
| pitch | x | the **nose down** |
| roll | y | the **top to the left** |
| yaw | z | the **nose to the left** |

The righting is what fixes it and is the check on it: the settle angles are
asin of the target in the hull's frame — pitch from its y, roll from its x
negated (`0x1000c4b4`, `0x1000c562`) — and they are **added** to the spin, so
they can only right the hull if a positive pitch tips the nose down. Two other
readings agree without being used to get there: a positive yaw turning left
matches the camera's ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)),
and the running gear's `(right − left)` added to the yaw turns a machine toward
its own damaged side ([below](#running-gear-legs-wheels-and-tracks-by-side--read-and-measured)).

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
  with `c` the cosine of the tilt — the ground normal dotted with world up,
  which is the global (0, 0, 1) at `0x10043b40` (`0x10016730`) — a factor
  `min(1, 2 (c − cos cone) ÷ (1 − cos cone))`, 0 past the cone, pulls the
  velocity toward that fraction of itself at 1.5 × the acceleration.
- **It acts going uphill** (*read*). Before the cone test the integrator builds
  two horizontal cross products and takes their dot
  (`0x100156ae`–`0x10015799`): **C** = up × N, with N the averaged ground
  normal at body `+0x194`, and **D** = W × up, with W the velocity turned into
  the world by the body's matrix `+0xb0`. A negative D · C skips the brake. In
  components that dot is −(W·N) over x and y alone, and (Nx, Ny) points
  **downhill**, so the brake is skipped exactly when the machine is moving
  downhill and acts when it is moving uphill — or exactly along the contour,
  where the dot is 0 and the test passes. The sign does not depend on which way
  the global up points: flipping it flips both cross products and leaves their
  dot alone.

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
  reads the bed. It does run the ground contact — the pass's only gate is an
  agent of kind 4 (`0x1000cb90`) — but state bit `0x4` keeps it from falling.
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

1. **Body sphere.** Two spheres of the agent give the contact its figures
   (*read*). The control keeps its agent's interface `0x18` at `+0x24` and
   interface `0x20` at `+0x28` (`0x10007922`–`0x1000793d`).
   - **The radius r** is interface `0x18` slot 9's, asked with 2 into a
     four-float answer (`0x1001a487`): the **agent's sphere**, the chassis's
     and every hung part's header spheres joined (`AniMesh.dll:0x10014580`
     reads it from the object's `+0x114`/`+0x120`, which `0x10009510` works
     out; [26-damage.md](26-damage.md)). A radius under 20 is held to at most
     7.5 (`0x1001a48e`); one of 20 or more is kept.
   - **The centre** goes into `+0x98` from the other ask: interface `0x20`
     slot 3 (`0x1001a518`, `AniMesh.dll:0x1000f3b0`), with 2 and a request
     record that is all zero (`0x10046328`). Its answer's centre is copied to
     `+0x98` (`0x1001a591`–`0x1001a5c3`), and its radius is r₂, held to 7.5
     only when it is under 20 **and** the object's flags carry `0x1000000`
     (`0x1001a51b`–`0x1001a58a`, message `0x10` then slot 14; which objects
     carry that flag is not read); r₂ bounds the first search pass below.
   - **r₂ is the node sphere's radius, and it is not r** (*measured*). r comes
     from the parts' header spheres and r₂ from the nodes' boxes, and over the
     **148** unit models the campaign places they differ on all 148 — r₂ ÷ r
     runs from 0.42 to 2.34. r₂ is the **larger** on 26 of them, 25 of which are
     the big machines whose parts' sphere is over 7.5 and held there (the L-2f
     at 11.84 against 7.5, the Transformer at 14.68, `M_targ` at 17.57), plus
     `42_mons`, whose parts' sphere clears 20 and is kept: 34.19 against 27.90.
     On the other 122 r₂ is the smaller. So reading r₂ as r changes the up pass
     on every unit in the game.
   - **What slot 3 answers.** A request equal to the default one (all zero,
     `0x10026ad0`) with no node gets the object's **node sphere**, `+0x124`
     and `+0x130`, through the object's matrix (`0x1000f5c8`); any other gets
     the agent's sphere (`0x1000f62e`).
   - **The node sphere** (`0x10009e0a`–`0x1000a147`) joins, as the parts'
     spheres are joined, a sphere for every node of the merged model whose
     flags pass two masks set once to 0 and 1 (`0x1000c7f0`): every exterior
     node. A node's sphere is its level-0 slot's box (the current variant,
     `0x100124d0`) put through the node's matrix, its centre the middle of the
     box's diagonal and its radius half the diagonal (`0x1000f694`); a node
     with no level-0 slot is a point at its origin. Node 0 is asked as the whole
     object while the sums are still empty, so it adds nothing.
   - **When the two are worked out** (*read*). Both sums are the one routine
     `0x10009510`, which has three call sites and no pointer to it in
     `AniMesh.dll`, all in the agent mesh's message handler (`0x10006fc0`,
     slot 2 of the vtable at `0x1002057c`):
     - message 4 with a non-zero argument, the attach's second pass
       (`0x100070bd`);
     - the `0x80000020` part load, after the merge `0x1000a460`, when the load
       record's `+0x40` is set (`0x10007254`–`0x10007261`), as the unit's
       parts are assembled
       ([28-chassis.md](28-chassis.md#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured));
     - message 20 (`0x10007285`, after `0x1000aa70`), which the agent's part
       removal (`0x10003c40`) sends with the part's id once it has dropped the
       part from its list (76-byte records at `+0x6e4`, the id at `+0x48`). The
       removal has one caller, the agent's `IGameObject` slot 13
       (`0x10001370`, at `0x10020208` in the vtable at `0x100201d4`), on
       message 6 with sub-code 20 (`0x100013ae` → `0x1000155d`); nothing else
       branches there. **Only the designer sends it**, below.

     It first runs the pose walk (`0x10008b30`), so the nodes' boxes are taken
     at the frames the mesh holds then. The mesh's constructor (`0x100068a0`)
     leaves both frames and the blend between them at 0 (`+0x1e4`, `+0x1e8`,
     `+0x1f4`) and sets the two bytes the walk tests (`+0x1fc`, `+0x1fd`), so
     at the attach and through the assembly the nodes stand at **frame 0**,
     the rest pose. No frame's own step calls it.

     **Who takes a part out of an agent** (*read*). Every call through
     `IGameObject` slot 13 (`+0x34`) in the 18 shipped binaries was read for
     its message and sub-code, the pushed arguments recovered past the calls
     nested among them: 376 calls. **Five** pass message 6 with 20, and all
     five are `iron3d.dll`'s designer taking a part off its own project, the
     model at its `+0xbc90`'s `+0x74`: the removal `0x10053a50` (`0x10053bab`,
     `0x10053c2e`, `0x10053c94`), a turret off `0x10053df0` (`0x100540ff`) and
     a gun off `0x10054210` (`0x100543ba`)
     ([38-designs.md](38-designs.md#fitting--read-and-measured)). The control
     is the part load beside it: the same sweep finds the **seven** sends of
     message 6 with `0x80000020` — the designer's five, the unit build in play
     (`Behavior.dll:0x1001ce5b`,
     [28-chassis.md](28-chassis.md#the-order-parts-load-in-and-what-a-slot-keeps--read-and-measured))
     and `ArealMap.dll:0x10014cab`, beside its "SubItem has … child items" log —
     and 21 of message 6 with 7, the mode setter
     ([31-packages.md](31-packages.md#the-escape--read)).
     - **The calls that forward a message or sub-code they were handed** were
       each followed. `Terrain.dll:0x1007d7e0` hands every object of two kinds
       its caller's sub-code, which is 23 or 24 (`0x1007e796`). `World3D.dll`'s
       queue takt sends 1 and `0x1c` (`0x10006c6f`, `0x10006ce1`), and its
       consumer (slot 8, `0x10006460`) hands a record of any type but 9 on as
       message, sub-code and argument (`0x10006a80`). A record reaches that
       consumer by the queue's post (slot 7, `0x10006090`), its two sends
       (slot 15 to an object, slot 18 to the host), a direct delivery or the
       network. The same sweep over those four slots, in every binary, finds
       no call that names type 6: the deliveries `World3D.dll` makes itself
       carry 9 (13 of them) or pass on a record as it came, and nothing calls
       the post with a type it names. The network packet builder is called with 9 or with the
       type of a record it relays (`World3D.dll:0x10007690`, from
       `0x100062ef`, `0x10006f6b`, `0x10006af6`).
     - **So no unit in play has a part taken out of its list.** A part shot to
       nothing is knocked off and hidden where it hangs, and the agent keeps
       its record
       ([26-damage.md](26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured)).
       The two spheres are worked out as the unit is attached and as each part
       loads, both at frame 0, and never again in play: the L-2f whose turret
       body `e_tur_bb_01` is shot off keeps the spheres it was built with. Only
       the designer's project has them worked out again, at whatever pose its
       model then holds.

   So **the contact holds the body by the agent's sphere's radius about the
   node sphere's centre** (*read*). An earlier reading took both from the
   agent's sphere, from the slot 9 ask alone; it put the L-2f's centre 2.75
   below its origin and left the flat ground of Mission 02's island a
   "Risk area!".
   - **Mission 04's helicopter:** r 2.45, about a node centre 0.60 below the
     origin; resting, its origin stands 3.04 over flat ground (*measured*).
   - **The L-2f:** the agent's sphere 12.26, held to 7.5, about a node centre
     2.17 below; resting, its origin stands **9.67** over flat ground. Its node
     sphere's radius is 11.84 (*measured*, the parts' and nodes' boxes and
     spheres as the two routines join them).
   - **The recording agrees** (*seen*). Mission 02's warbot, resting on the
     island's flat ground by the Outpost at 151.67, reads altitude 11 over
     Tut_2's water at z 150 (161.34), and the hero it lets out reads 3
     ([39-boarding.md](39-boarding.md#against-the-recording--seen)). The
     agent's sphere alone would read 12 there (161.92) and refuse every place;
     the chassis's own sphere would read 9.
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
   `CBuilding` hands slot 7 to its mesh (`Terrain.dll:0x10056c70`).

   **What the filter excludes** (*read*, and *measured*). The ground search
   builds its query with a required mask of nothing and an excluded one of
   **world face flags `0x208` and class bit 8** (`0x1001a687`; the contact
   points' searches use the same, `0x1001ad7f`). The record is eight dwords —
   the node-type mask, then a required and an excluded pair of a world flags
   word and a class word (`0x1001bd50`) — and the landscape turns each pair
   into one mask of its own (`Terrain.dll:0x10022da0`), which it tests as
   *all of the required bits present, none of the excluded*
   (`0x10021d33`–`0x10021d4f`). World flag `0x8` becomes landscape `0x20`,
   world `0x200` becomes `0x20000`, world `0x400` becomes `0x2000`, and class
   bit 8 becomes `0x40000`.
   - **The landscape's mask is the file's face record.** `CLandscape` keeps
     `Land.msh` stream 21 as it reads it — the pointer at `+0x6c` and the
     element count beside it (`0x100176e6`), 28 bytes a face, adjacency read at
     `+0xe` (`0x1001e951`), the texture bytes at `+4` and `+5` — and reads the
     record's first **dword** as the flags (`0x10060530`). So the landscape's
     32-bit mask is the file's **flags word** in its low half and its
     **surface word** in its high half. Two identities this document already
     holds confirm it: landscape `0x2000`, the liquid bed, is the flags word's
     `0x2000`, set on exactly the 6102 bed faces, and landscape `0x20000`, the
     liquid surface, is the surface word's `0x02`, set on exactly the 3630
     water faces ([03-terrain.md](03-terrain.md#the-face-records-last-unread-fields)).
   - So the two unnamed bits are the **flags word's `0x20`** and the **surface
     word's `0x04`**, and **no shipped face carries either**: 0 of 275882
     across all 33 maps, at both levels of detail. The control is the same scan
     over the same two fields for the other two bits the same filter names —
     3630 and 6102, as above. `LandMesh.is_ground` is the filter, and on the
     install it comes to "not water".
   - **What would set either is not established.** Enumerating every masked
     access to the face array in `Terrain.dll` — each `imul r, r, 0x1c`
     followed by a mask — returns thirteen sites and all of them are reads;
     the building insertion's own range was scanned for `or` and masked writes
     to no effect ([03-terrain.md](03-terrain.md#placing-a-building-cuts-the-landscape--read-in-outline-and-measured)).
     A plain immediate scan for `0x20` does not discriminate: `Terrain.dll`
     carries the world-to-landscape mask converter inlined at a dozen sites and
     the constant is in every one of them.
   - The other readers are the **draw-order rebuild**: the builder that lays
     out a cell's batches skips a face carrying `0x20` (`0x10060530`, beside
     `0x800`), and a later builder gathers the faces that do carry it and gives
     each one a batch of its own (`0x100644dc`). So a face with the bit would
     still be drawn, one batch apiece, and not walked on.

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
   body and join the averaged normal (below); **flag 2 lays the node the point
   carries along the ground it has just found** (`0x1001affd`,
   [28-chassis.md](28-chassis.md#the-belt-lies-along-the-ground--read-and-measured))
   — the tracked chassis's belts, twelve contacts in the game. The live
   record's byte `+0x59`
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
  normal — the sphere's and each flag-1 contact's, over **their count plus
  one** (`0x1001b0ea`) — becomes the body's ground normal `+0x194`
  (`0x10015e47`), which the slope brake reads and the righting stands the hull
  toward ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)).
- **Whether the fall is tried at all** is a fifth argument the pass works out:
  the number of flag-1 contacts it walked, above 0 (`0x1001b401`). With bit
  `0x4` set but no flag-1 contact the lift vector the loop fills is left at
  (0, 0, 0) (`0x1001ab5d`) and nothing moves.
- The move shifts the body's three matrices (`+0xb0`, `+0xf0`, `+0x130`) and
  notifies the object (`0x10015f0f`). The velocity is not touched.
- **The whole move is skipped** while the machine's byte `+0x618` is set
  (`0x1001b3f7`), which is control message 7's
  ([13-control.md](13-control.md#not-established)).

**A sphere with no face under it is lifted by its whole radius** (*read*). The
search's failure path does not leave the ground point empty: it copies the
**centre** into `+0x70` and the constant (0, 0, 1) at `0x10046428`
(`0x1001bfa0`) into the normal (`0x1001a862`–`0x1001a8a4`). So with bit `0x4`
clear the gap (ground z − centre z) is 0, which is above −r, and the lift is
**r**, every frame, for as long as the search finds nothing. A contact point
with no face under it is handled the same way (`0x1001af72`) and so lifts by 0,
and its default normal joins the average like any other.

**When it runs, and with what dt** (*read*). The pass is message `0x1c`, slot
24 of the control system (`0x10007d03`), and its only caller is
`0x1000cb80` → `0x1001a450` at `0x1000cb98`, for an agent of kind 4. So it runs
**once a frame**, after the machine tick and the collision pass and before the
push. Its dt is the frame's: the machine tick keeps the milliseconds since its
last at `+0xe8` (`0x1000bcfd`–`0x1000bd03`), and the fall reads that × 0.001
(`0x1001b41b`, the float at `0x1003c018`). It is not the state step's length,
and a frame in which no state step falls due still runs the contact.

**Which pose the contact points are read at** (*read*). The run-time pass does
**not** pose the object: no slot-25 call is made anywhere in it (every virtual
call in `0x1001a450` enumerated). It asks each point's position and axis
through the node the point sits on, at whatever pose the mesh currently holds
(`0x1001b4f0` mode 2 → interface `0x20` slot 4, the node's matrix). And the
machine tick has just set that pose: after its state-step loop it plays the
mesh at the frame's own time, for an agent of kind 4 (`0x1000c728`–`0x1000c737`,
`0x100059a0` with `+0xe4`). **So the contacts are placed at the frame's
interpolated pose, phase s of the step being played** — not at the step's last
frames. Only the once-per-state pass poses at the state's **end**, and it does
so to work `CONTACT_PLACE` out
([A walker's feet](#a-walkers-feet-lie-flat-where-the-animation-lays-them--read-and-measured)),
not to place anything.

*Measured:* bit `0x4` is set on exactly the 961 states of the controllers
that declare contact points — the walkers, wheeled and tracked chassis, the
hero, three animals, the trees and stones — and on no other. So everything
with legs, wheels or tracks falls under gravity 10, and every flyer holds its
height. A mode-0 or mode-2 machine falls exactly when its state has contact
points.

#### A contact's up pass must not reach as far as r — *measured*

The lift is the largest rise over the flag-1 contacts, **whatever its height**,
and a contact's own search takes the face above it before the one below when
that face is within its bound. Give that bound the agent sphere's r and a
machine nosing over the lip of a ramp is hoisted back up it: its leading
contacts find the ramp below, its trailing ones still stand under the floor
they have just left, and the **max** takes the floor.

*Measured* on C02 Mission 03, an SWW-X Warrior (chassis `R_L_03`, r 3.215,
r₂ 2.592) the player drives into the Large Factory, one metre past its door:

| contact z | its ground | rise |
|---:|---:|---:|
| 46.43 | 49.34 | **+2.91** |
| 46.43 | 49.11 | +2.68 |
| 46.43 | 43.63 | −2.80 |
| 46.43 | 43.63 | −2.80 |

The body sphere's own search finds the ramp at 42.93, 4.5 below it. The lift
takes +2.91, the ground contact carries the machine back up into the structure
over the ramp, and the collision pass — whose segment test puts a move that
runs against a face back at its start — undoes the whole move. The next tick
does the same. At its full 17.9 m/s the machine does not move a millimetre
again.

At r₂ the two rises of +2.91 and +2.68 both exceed the bound, those contacts
fall through to the face below, and the machine drives down to the pod. r₂ is
the **smaller** on 122 of the 148 unit models the campaign places.

*Measured*, the cost: over 18 captures on that mission, every small chassis
against every enemy building, **15** are taken with the bound at r and **14**
with it at r₂ — the Large Factory 5 s faster for a wheeled bot and 15 s faster
for a walker, and the Small Bunker lost for a walker, which stalls 34 m short
of its pod with no lift over 0.5 and no segment stop anywhere near it, so by
something other than this.

### A contact point sits on one node and dies with another — *measured*

The contact names a control point, and the point's first triple carries **two**
node numbers ([07-objects.md](07-objects.md#the-control-points)): the second
slot is the node the point **sits on**, the third the node it **dies with**,
which is what the state's `0x100` and `0x200` conditions test and what
[13-control.md](13-control.md#section-1s-conditions-are-contacts--read-and-measured)
means by "the node is its third slot". The third slot is a damage reference,
not a frame. Placing the point in it is wrong wherever the two differ.

*Measured* over the campaign's 15 chassis, they differ on 8:

- **The walkers** — `A_L_04`, `A_L_05`, `R_L_01`, `R_M_01`, `R_B_01`, `A_L_01`,
  `R_B_05` — name the same node twice, so the foot rides its own leg either
  way.
- **`R_T_01` and `R_M_03`** name a node and one below it: `leg_fl` sits on 14
  and dies with 16, `weel_fr` sits on 3 and dies with 5.
- **Every other wheeled chassis** — `R_B_03`, `R_B_04`, `R_L_03`, `R_L_04`,
  `R_B_06` — authors its `weel_*` points on **node 0, the body**, and names the
  wheel in the third slot. That is docs/07's "a wheel's contact point rides on
  the body and dies with the wheel", read exactly.

So a wheel's contact is authored in the model's own frame, at the tyre's
contact patch. `R_B_03`'s `weel_fr` is at (3.02, 3.22, −2.80): its x and y are
the front-right wheel node's own translation, (3.02, 3.19), and its z sits
0.04 above the model's lowest vertex, −2.84. Taken that way every wheeled
chassis's lowest contact lands within 0.03 of its tyres, and the bot sits on
the ground.

Posed in the wheel's frame instead, the same point lands at (6.24, 0.17,
−5.05) — outside a hull 8.02 wide, and 2.23 under the tyres. Since the lift
plants the contacts, the bot then rides 2.23 clear of the ground, about three
quarters of its own 3.10 wheel, and visibly hovers. openparkan did this until
it was measured; the mistake is easy to make and hard to see, because it is
invisible on the walkers and on anything with four small wheels.

**What is left is terrain.** The lift is the **largest** rise over the flag-1
contacts, so a unit is held at the highest ground under any one wheel and the
rest of it hangs. That is a real gap and the game has it: measured across the
campaign, `R_B_03` rides up to 0.67 clear where the ground under its six
wheels is broken, and `R_B_01` up to 0.98.

### A walker's feet lie flat where the animation lays them — *read*, and *measured*

A contact's flag `0x2`, `CONTACT_PLACE`, lays the node the contact carries
along the ground under it. Twelve contacts in the game are authored with it,
the tracked chassis's belts
([28-chassis.md](28-chassis.md#the-belt-lies-along-the-ground--read-and-measured)).
**Flag `0x20` asks for the same flag to be worked out from the state's own
pose**, and 2410 of the install's 2634 contacts carry it.

**When it is worked out** (*read*). Taking a state copies that state's
section-1 record into the current-state slot `+0x100` and then runs one pass
over the machine's contacts (`0x10019df0`, from `0x10007b99` and
`0x10031982`) — the pass that also measures the state's stride. It poses the
object at the state's end, `IAnimation` slot 25 with 1.0 and 1.0
(`0x1001a1fd`), and for each contact asks slot 4 for the point's position and
its **own vector**, both through the node the point sits on
(`0x1001b4f0`). Then, per contact (`0x1001a311`–`0x1001a370`):

- `0x1000`, `CONTACT_PLANTED`, is set where the point's height there is within
  0.1 of the height the live record holds (`0x1003c03c`, `0x1001a328`);
- `0x10` sets `0x1` to match that and clears it otherwise (`0x1001a314`);
- **`0x20` sets `0x2` where the vector's z is above 0 and 1 − z is below 0.05
  (`0x1003c038`, `0x1001a364`), and clears it otherwise** (`0x1001a36d`).

The vector is the posed axis in the model's frame, so the test is *this
contact's own up, within about 18° of the model's up*. It is unnormalised, and
every shipped control point's vector is unit length (2410 of 2410, *measured*),
so the comparison against 1 is a cosine.

**Who asks, and what the answer is** (*measured*, over every `.ctl` in the
install):

| controller | contacts with `0x20` | of them, placed |
|---|---:|---:|
| `a_a_l4`, `a_a_l5` (animals) | 484 each | 484 each |
| `r_b_01` | 364 | 364 |
| `a_a_l1` (an animal) | 218 | 114 |
| `r_h_02` (the hero) | 210 | 190 |
| `r_l_01` | 210 | 173 |
| `r_b_05` | 180 | 180 |
| `r_t_01` (the Tiny Spider) | 162 | 148 |
| `r_m_01` | 96 | 78 |
| `r_h_01`, `r_h_03` | 1 each | 1 each |
| **total** | **2410** | **2217** |

- **Every one of them is a foot** — `LeftFoot`, `RightFoot`, `foot_fl`,
  `leg_fl` — on the eight walking chassis that declare contacts and the three
  animals. Their flags are `0x21`, `0x25` and `0x125`: support, `0x20`, and the
  fallback and intact conditions. **No wheeled or tracked contact carries it**,
  and none of the 2410 is authored with `0x2`.
- **No shipped contact carries `0x10`**: 0 of 2634. A walker's support flag is
  authored, not derived.
- So **2229 contacts in the game lay their node along the ground** — the twelve
  belts and 2217 feet — against the twelve this document counted before.

**What it looks like.** A walker's foot node is turned onto the patch of ground
under it and put back where it stood, so the foot conforms to the slope and the
leg above it does not move. The states it is withheld from are the ones whose
last frame has the foot on its side: 20 of the hero's 210, 37 of `r_l_01`'s
210. The Large Walking Chs and `r_b_05` keep their feet flat throughout and
place in every state.

The engine does it in `place_by_pose`, once per state as the controller loads,
since the answer depends on nothing but the state; `Feet::stands_up` is the
test and `Walker::lay_belts` the turn, the same path the belts take.

### A flyer's height — *read*, and *measured*

**Only the command's z climbs or sinks a flyer** (*read*).

- **The key.** R and F set the command's z to 1 and −1, and letting go sets
  0 ([39-boarding.md](39-boarding.md#driving--read-and-measured)).
- **The speed.** The velocity integrator moves the vertical velocity toward
  command z × the live top z by at most the live acceleration z × dt, as it
  does on every axis
  ([Speed is a target](#speed-is-a-target-approached-at-a-fixed-acceleration--read)).
- **The frame.** `0x10014610` holds the velocity to the live top speed. It
  then turns the velocity by the matrix at body `+0xf0` (`0x10014726`, through
  `g_FastProc` `+0x1c`): the hull's own attitude, which the spin integrator
  builds from its quaternion (`+0x58`, `0x10014dfb`–`0x10014e43`).
- **What the frame leaves out.**
  - *The turret's pitch.* The turret is a component, and its target goes to
    its own channels. Of the triple, the component setter passes only the
    change in x to the body (`0x1002ec23`,
    [30-turrets.md](30-turrets.md#the-hull-follows-the-turret--read-and-measured)).
  - *The lean.* It is a rotation of its own (`+0x90`), and only the drawn body
    takes it ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)).
- **The hull stays level.** Every flyer state sets the righting bits `0xC0`
  (*measured*, above), which zero the spin about x and y (`0x10014af9`) and
  right the hull toward world up. So a flyer's velocity frame turns only about
  z (*derived*): W flies level along the hull, and the height changes only
  with R and F.

**Nothing holds a height above the ground** (*read*). A flyer's states have no
contact points, so the ground contact only lifts it out of the ground and
never pulls it down
([above](#holding-the-body-on-the-ground--read-and-measured)). Flying into
rising ground raises it, and it stays up when the ground falls away again
(*derived*). **The ceiling** is the world's box: a flyer more than 20 above its
top is pushed back ([The map edge](#the-map-edge--read)).

**What a player sees, then** (*measured* on openparkan's engine, which follows
these reads). Turret pitch reaches nothing that moves the body, and the eye
does not ride the pitch.

- **The pitch.** Over a full sweep of the pitch channel, the body's height
  and the eye's height stay within 0.01 m on the L-2f (hung `e_tur_bb_01`),
  on Mission 04's helicopter (hung `e_tur_tb_01`) and on its HQ (upright
  `e_tur_bt_04`).
- **No other path.** The only mouse-Y row in `m2.tbl` is the turret's
  `ANGLE_Y` (*measured*).
- **The ground does move a low flyer.** Mission 04's helicopter, boarded
  where it stands, holds 3.2 m over the valley floor. Held on W for 12 s
  toward the west slope, it rides 57 m up at its sphere's bottom, its eye
  1.2 m or more over the ground (0.4 m under it on the chassis's own sphere).
  It keeps that height where the ground falls away. The run is the same with
  the sight pitched down or up.
- **So** a player skimming the ground sees the altitude figure climb as the
  terrain rises, and never fall back without F (*derived*).

**The nine flying chassis' climb**, authored (*measured*, `bases.rlb`).
Acceleration is live, twice the file's; the live top speed can only be lower
than authored:

| chassis | top vertical m/s | vertical accel m/s² | to full climb |
|---|---:|---:|---:|
| Tiny Helicopter T-2 `r_t_02` | 35 | 30 | 1.17 s |
| Small Flying `r_l_02`, `r_l_06` | 20 | 30 | 0.67 s |
| Small Flying `r_l_05` | 35 | 50 | 0.70 s |
| Small Flying `r_l_07` | 15 | 30 | 0.50 s |
| Medium Flying M-2f `r_m_02` | 20 | 30 | 0.67 s |
| Large Flying L-2f `r_b_02` | 15 | 20 | 0.75 s |
| Large Flying `r_b_07` | 15 | 30 | 0.50 s |
| Large Flying `r_b_08` | 6 | 4 | 1.50 s |

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

#### The cap where two halves meet — *measured*, and a stand-in

Every bridge is placed this way. Across all 29 missions there are **9 bridge
pairs**, and on all nine the second half stands π from the first, straight on
along the span, and exactly **twice the deck's length** away — so the two decks
**abut**, to within a unit (`placement: a bridge's two halves meet`).

That means each half's far end stands square in the way of anything crossing.
All four `fortif.rlb` bridge meshes carry a cap there: **six** level-0 faces at
the deck's own far end, every one of them facing the model's +y. Three of the
four flag those six faces **4**, the bit a round passes and the collision's own
segment query passes too. The fourth, `fr_e_brige` — C02 Mission 04's *Enh
Bridge BS-52/30* — flags them **`0x20`**, which is the bit its additive
`B_A_BRIGE` material carries throughout the mesh: its deck is flagged `34`,
floor and energy at once. A round passes both, the round query's triangle mask
being `0x24` ([07-objects.md](07-objects.md#the-flags-word)).

**STAND-IN.** The read of the collision's own filters gives them a triangle
mask of **4** — `0x1001dbad` for the segment and `0x1001dbce` for the push-out,
[below](#collision-between-objects--read) — and nothing that drops `0x20`. Taken
as read, the energy bridge's cap is a wall: the hero walks 183.8 m of the 185.5
to the join and stops dead, half way over the gorge, while Mission 01's
`m_bridge` lets it straight through. The engine passes `0x20` here as it passes
4, so that all four bridges cross. What the game does instead — a batch flag the
mesh's batch word does not carry, or a filter this page has not found — is not
established.

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
- **An entry is the collision object plus 0x20.** The object keeps two vtables,
  `0x1003c2fc` and `0x1003c2f4` (`Control.dll:0x1001f2f4`), and joins its
  manager by handing it `this + 0x20` (`0x1001f611`). So every offset below,
  and the flags and mass the pass reads, are that much past the object's own:
  entry `+0x10` is object `+0x30`, the flags; `+0x14` is `+0x34`, the mass;
  `+0x18` is `+0x38`, the sphere; `+0x38` and `+0x3c` are `+0x58` and `+0x5c`,
  the face source (interface `0x18`) and the push-out (`0x25`); `+0x40` is
  `+0x60`, the manager itself (interface `0x203`). All seven are written by the
  attach, message 4 with a zero argument (`0x1001f598`–`0x1001f607`).
- **What the pass skips.** It passes over an entry whose flags carry 1
  (`0x1001c1e3`), and one whose radius is still below 0 (`0x1001c15a`).
  **Nothing sets that flag** (*read*, with a control). The flag is object
  `+0x30` bit 0; the constructor clears it (`0x1001f2c0`), the object's own
  slot 4 sets it (`0x1001fe10`) and slot 3 clears it (`0x1001fe00`), and slot 3
  is called once, by the object itself, at the end of message 4 with a non-zero
  argument — the detach (`0x1001f62d`). Neither body is referenced anywhere in
  `Control.dll` outside the vtable, and of every indirect call at `+0x10` in
  the install that passes no argument, **none** has a receiver that could be a
  collision object. The control is the same enumeration at `+0x8`, slot 2,
  which finds `AniMesh.dll:0x10001850` sending a message to the collision
  object the agent keeps at `+0x154`. So **no entry is ever skipped** in the
  shipped game.
- **Which pairs go on.** It takes every pair j < i once. A pair goes on only
  when one side has a contact record (`0x1001c1e9`) and the swept spheres touch
  within the frame (`0x1001e9f0`). A unit against a tree, a stone, a building or
  another unit qualifies; two pieces of scenery never do.
- **Handlers.** A pair with a handler of its own (`+0x40` slot 7) goes to it;
  `+0x40` is the **manager the object joined**, which the attach fills from
  interface `0x203` (`0x1001f600`) and without which it would not have
  registered at all, so the pass always has one to hand the pair to
  (`0x1001c222`). What that manager's slot 7 then does is not read.
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
   `0x1001deea`). **The class is the size class** (*read*).
   It is the first dword of message `0x201` through B's interface `0x10` slot
   26; interface `0x10` is the `MBehaviour` the agent build puts at `+0x17c`
   and files under that id (`AniMesh.dll:0x100034bd`, `0x1000361f`), its slot
   26 is `Behavior.dll:0x1000a490`, and variable `0x201` there answers
   `&[this + 0x960]` (`0x1000a533`) — the field set once at `0x10005e8f` from
   the machine's own slot 54, `0x1000cee0`. So the hero, class 2, is never
   stopped this way, and a class-3 or class-4 machine is.
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
     - **Which edge** (*read*, `0x1000e9a7`–`0x1000eb1e`). The edges are taken
       in the order a→b, b→c, c→a, each as (e × n)·(q − start) with e the edge
       made unit and q the projection. The **first** edge that gives more than
       0 decides alone; the other two are not looked at.
     - **That edge's point** (`0x1000eb70`–`0x1000ed4e`). The projection must lie
       within √(r² − h²) of the edge's line, h the height over the plane, or the
       triangle is passed over. Then t = (q − start)·e: below 0 the point is the
       edge's start and above its length its end, either of which must lie
       within r of the centre; between, the point is the foot on the edge at a
       distance √(s² + h²). So near a corner the first edge's end stands in for
       a nearer point through the next edge (*derived*).
   - **Hidden faces go** (*read*, `0x1000d7a5`–`0x1000dac0`). A face whose
     centroid, (a + b + c) ÷ 3, is hidden from B's centre by another gathered
     face is dropped.
     - **The order.** The faces are tested from the last, the farthest, to the
       first. A hidden face is taken out of both lists at once
       (`0x1000dade`–`0x1000db57`), so it hides nothing after. Each is tested
       against every other face still in the list, from the last.
     - **The crossing** is `Ngi32.dll`'s `g_FastProc` slot `+0xb4`, whose
       generic build is `0x1001fa00` (set at `0x10003cca`): with p₀ in front of
       the face's plane or on it, it answers only a segment that runs against
       the normal and reaches the plane before p₁
       (`0x1001fa2a`–`0x1001fa66`). The test runs it from the centre to the
       centroid (`0x1000d8c5`). When the hider's batch word, record `+0x40`,
       carries 2, it runs it both ways, centroid to centre first
       (`0x1000d86c`–`0x1000d8a4`).
     - **Inside the hider** (`0x1000d8d3`–`0x1000dab8`). The crossing is
       measured in the plane that drops the normal's largest axis, x before y
       and either before z on a tie. Each edge in the order c→a, a→b, b→c gives
       the cross product's component on that axis times the normal's. The
       first edge within 1e-5 of 0 (the float at `0x1002097c`) hides the face
       at once, whatever the others give; one below 0 does not hide it; three
       above 0 hide it.
     - **Floors hide too.** The test reads no triangle flag, so a floor, flagged
       2, hides a face behind it before the filter below takes the floor out.
     - **The record** (`0x1000d61b`–`0x1000d765`, 0x48 bytes): the three
       corners, the normal twice, the plane's −n·a, the batch word `+0x40` and
       the triangle word `+0x44`.
   - **Filter.** What is left is filtered by batch and triangle flags. The
     batches go as in step 1. The triangles go by a third filter of the pair's:
     flagged 4, and **flagged 2 unless B's collision flags carry 8**
     (`Control.dll:0x1001db2b`, `0x1001dbce`). The pair passes that filter as the
     push-out's fourth argument (`0x1001dfdb`), and the push-out tests every face
     against it (`AniMesh.dll:0x1000db93`). The step-1 filter goes in as the
     third argument, and where the push-out uses it is not traced.
   - **Accumulate**, nearest first (`0x1000dd7d`–`0x1000def8`). Take a face
     with unit direction d and depth p = r − distance, and let s = P·d. If
     s < p, take d′ = d − (d·P)P ÷ |P|², the part of d square to the push so
     far (d′ = d while |P|² ≤ 1e-4). Then P += d′ × (p − s) ÷ (d′·d), or
     P += normalised d′ × (p − s) when |d′·d| ≤ 0.002. So P meets each face's
     depth without undoing the faces before it.
   - **Clamp.** P is held to 4r (`0x1000df50`, `0x10020970`).

**Sharing the push.** P under 1e-6 in squared length is no contact. Otherwise P
is shared by **mass squared** (`0x1001e0db`), the owner's property 0x7c copied
to the collision object on message `0x1c` (`0x10020038`). B moves by
P × m_A² ÷ (m_A² + m_B²) and A by −P × m_B² ÷ (m_A² + m_B²). A side with no
contact record does not move, and the other takes all of P (`0x1001e05f`). So
a unit meeting a tree, a stone or a building takes the whole push. Each moved
record gains flag 8.

**The mass is what the machine weighs** (*read*, and *measured*). Property
`0x7c` is one of the 37 the control system's own property interface implements
([13-control.md](13-control.md#the-property-interface-is-not-where-the-names-are));
all the addresses here are `Control.dll`'s. `id − 1` indexes the byte table at
`0x1000e5e8` into the jump table at `0x1000e554`, and 124 shares its case with
104 and returns `&control[+0x538]` (`0x1000e0ac`). `+0x538` is where the weigh
routine accumulates ([Load](#load--read-and-measured)): it is zeroed at
`0x1000fadb`, each node's density × volume and armour's weight over its area
are added at `0x1000fbba` and `0x1000fc3f`, and the spare payload is the
authored payload plus the chassis's body less it (`0x1000fc57`). It is the
"Weight" the stat panel shows × 0.001 in t, and it is **not** the file's +124:
that is the authored payload, which property `0x88` hands out from
`[+0x46c] + 0x68` (`0x1000e17a`).

- *Measured*, over the shipped assemblies: **all 382** units under `UNITS`
  weigh something, from **271 kg** (`s_arah`, an `A_L_04` animal) to
  **4,804,877 kg** (`m7_tow`, the Small Tower on `R_B_06`), and by size class
  2,523–3,501 (class 1), 271–76,155 (2), 16,510–40,511 (3) and
  43,367–4,804,877 (4).
- **A static object answers nothing.** The mass field starts at 0
  (`0x1001f2c3`) and only message `0x1c` writes it, from the owner's property
  `0x7c`; a tree, a stone or a bridge carries no control system to answer it.
  It never matters: neither has a contact record, so it takes none of the push
  anyway.
- *Derived*, on Mission 01: the hero weighs **3,300 kg** and `tut1_e1`
  **3,239**, so a push between them is shared almost evenly — 0.491 of it to
  the hero. The neutral flyer `tut1_mf1` weighs **25,699**, so a push between
  the two gives the hero **0.983** and the flyer 0.017: the hero is shoved
  aside and the flyer hardly moves.

**When it happens** (*read*,
[26-damage.md](26-damage.md#the-hit-test--read-and-measured)). The world's frame
sends every object message 1, and the control system's tick moves the machine.
Then the pass runs, then message `0x1c`, and then each record with flags set
gets message `0x1b`. So the push lands after this frame's move and ground
contact, and the next frame starts from it.

**The ground contact is message `0x1c`** (*read*). The control system's message
switch (`Control.dll:0x10007cc2`, messages `0x14` to `0x1c`) sends `0x1b` to
slot 23 (`0x10007cda`, the push below) and `0x1c` to slot 24 (`0x10007d03`).
Slot 24 (`0x1000cb80`) runs the ground contact (`0x1001a450`) for an agent of
kind 4, a unit, at `0x1000cb98`, the ground contact's only caller. So:

- **it runs once a frame, not once a state step:** after the move and the
  collision pass, before the push is taken;
- **its dt is the frame's:** the machine tick keeps the milliseconds since its
  last at `+0xe8` (`0x1000bcfd`–`0x1000bd03`), and the lift's fall reads that
  × 0.001 (the float at `0x1003c018`, `0x1001b41b`).

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
it stands on included. The exception is a floor flagged 2, which pushes only a
mover whose collision flags carry 8
([above](#collision-between-objects--read)). A walker standing on a building
takes a push that points down whole (message `0x1b`). A bridge's deck pushes
the hero crossing it, except where its faces are flagged 2.

**Doors** (*read*). `CBuilding` files each class-12 item as a door, with the
nodes its channels play (`Terrain.dll:0x100583a2`–`0x100584e8`), and each
class-13 item as a computer. A door has a state, the time it opened, a lock
flag and value, and a hold.

- **Filed, it is shut** (*read*). Once every item is filed, the filing hands
  each door's item, and the **first** computer's, switch word 2 through
  `IItemManager` (interface `0x202`, `CBuilding +0x3c`) slot 6
  (`0x10058532`–`0x1005854f`, `0x100585a1`–`0x100585be`; `Control.dll:0x1002ed30`
  sets the item's property `0x600`, its switch word). That is the word the
  timed close sends (`0x1005766d`). It then puts the door's state at 2, closing,
  and its lock flags and hold at 0; the computer's state at 0. A word of 2 runs
  the progress back from its 0, which is an end at once, so the item holds 0
  and clears its word
  ([28-chassis.md](28-chassis.md#what-a-devices-value-turns--read-and-measured)):
  every door and pod starts shut and still, whatever the constructor's 5.
- **A second computer is left running** (*read*, and *measured*). The filing
  switches index 0 of the computer list alone (`0x1005858e`), and
  `CBuilding::SendMsg` drives that one alone (`0x100577ad`–`0x100577ed`). **18 of
  `fortif.rlb`'s 30 controllers carry two class-13 records**, each second one
  on a single node beside the first's: the Outpost's `o04` beside its pod `o03`,
  the Small Bunker's `i14` beside `i13`. The second keeps the constructor's word
  5, open and wrapping, so its node plays round for ever; 10 of those 18
  channels wrap.
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
- **The capsule is the part's box stood on its longest side** (*read*). The
  call is `IJointMesh` (interface `0x20`, `CBuilding +0x38`) slot 5
  (`AniMesh.dll:0x1000fd60`), asked in world space (2) about the door part's
  node. It asks its own slot 4 for that node's level-0 box — the current
  variant's, its eight corners through the node's matrix (`0x1000f760`) — and
  takes the diagonal *E* = corner 7 − corner 0 in the world. The corners are
  laid out as `0x10011850` writes them: 0 the minimum, 7 the maximum, and 1, 2,
  3, 4, 5, 6 taking the maximum on x; x and y; y; y and z; z; x and z.
  - Along **z** when *E*'s z is at least its x and its y (`0x1000fdd3`): from
    the middle of corners 5 and 7 to the middle of 0 and 2, the centres of the
    box's top and bottom faces.
  - Otherwise along **x** when its x is at least its y (`0x1000ff32`): from the
    middle of 1 and 7 to the middle of 0 and 4.
  - Otherwise along **y** (`0x1001007e`): from the middle of 3 and 7 to the
    middle of 0 and 6.
  - The radius is the distance from the first end to corner 7 (`0x100101b1`),
    half the diagonal of the end face.

  *Measured* over `fortif.rlb`'s **56** door parts at rest: 19 run along x,
  25 along y and 12 along z, and the capsule is narrower than the node's
  level-0 sphere on **56 of 56**. The Large Factory's front door `i05` is a
  capsule 22.1 long and 7.73 wide across the doorway at mid-height, against a
  sphere of 13.48; its side doors `i19` and `i21` are 11.5 long and 3.08 wide,
  against 6.53; the Small Bunker's `i03` 9.8 and 2.92, against 5.70.
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
  - **A door that sinks pushes down** (*derived*, and *measured* on openparkan's
    engine). The Small Bunker's door `i03` lowers into the floor for 3.6 s. A
    hero pressed against it meanwhile keeps meeting its upper triangles, while
    the floor in front, at 74.63, hides the lower ones whose centroids have gone
    under it. The upper triangles' ends lie above the hero's centre, so their
    push points down and back, and a walker on a building takes a downward push
    whole. The ground contact after the next frame's pass lifts it again. An
    engine that runs the ground contact only at state steps lets those pushes
    add up between steps: the hero sank 1.2 m there, and on one build fell
    through the floor.
- **Two more openers:** a hit struck on a door's node opens it
  (`Control.dll:0x1000ec7e`, [below](#a-shot-opens-a-door--read-and-seen)),
  and a hall-way link opens the doors listed on it
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
([07-objects.md](07-objects.md#baked-lighting)): the lightmap replaces the
scene's light on them, times their texture
([How a lightmapped batch is drawn](07-objects.md#how-a-lightmapped-batch-is-drawn--read-and-measured)).
Its consoles, signs, lamps, chimney smoke and dock glow are its controller's
load group ([13-control.md](13-control.md#a-buildings-load-group--read-and-measured)).

**The Large Factory and the Outpost of *The Constructor*** (*measured*, in
model space with z up from the placement):

| | Large Factory, `fr_b_plant` (`lplant01.dat`) | Outpost, `fr_l_angar` (`shang01.dat`) |
|---|---|---|
| way in | to the pod: the west side door `i21` (node 16) at x −26, up an 8° ramp to 0.96 ([below](#the-way-to-the-pod--measured-and-seen)); the front door `i05` (node 3) across x −11…11 at y 87.5–89.2, z 0–15.4, behind a forecourt floor at z 0 from y 87.4 to 108.5, into the entrance hall | no door: ramps from z 0 to 1.9 at both ends of its hall |
| doors | 3, on nodes 3, 16 and 14 (the entrance and two side doors at x ±26), rate 0.4: open in 2.5 s | none |
| floors | entrance hall `i01` at 0; the pod room at −12.4 | hall at 1.9; under the pod 0.48 |
| pod | node 25 (`i17`), radius 4.77, centre (0.06, −48.66, −9.67): zone 3.82 across | node 2 (`o03`), radius 6.37, centre (19.40, 8.79, 4.79): zone 5.10 across |
| capture fires | 4.5 s after the pod starts opening | 3 s |
| lightmap | on 100 of its 438 batches | none |

Mission 02 places the Large Factory 0.08 above the ground under it, so a unit
walking up to a door is the building's from the forecourt, or the floor before
a side door, on.

**Against the recording** (*seen*, 30 fps). The hero comes up to a side door,
the factory's west one ([below](#the-way-to-the-pod--measured-and-seen)), not
the forecourt, at 92–94 s. It is in the dark doorway at
94.5 s and under the cross corridor's lamps at 96.5 s. It stands on the pod from
102 s at the latest, and at 106.3 s the
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
   (child radius + capsule radius) of the part's capsule's segment, open the
   door (switch its item on, unless already open or opening) and hold it. The
   capsule is the node's level-0 box stood on its longest world axis, from one
   end face's centre to the other's, as wide as half an end face's diagonal
   ([above](#walking-into-a-building--read-and-measured)).
3. Tick every door: opening → open when its item stops (the time kept);
   open → closing when unheld, not locked open, and 5 s past opening; closing
   → shut when the item stops. A door's item steps like any other
   ([28-chassis.md](28-chassis.md)).
4. Collide: a building's faces push every unit on it or touching its sphere,
   the faces of a door that is open excepted; units on the same building push
   each other. Work the pushes out, run each unit's ground contact, and only
   then move the units by their pushes
   ([When it happens](#collision-between-objects--read)).
5. Tick the pod ([27-ownership.md](27-ownership.md#capture--read)): a child in
   the first computer's zone switches it on; when it has opened, and that child
   is still there, the capture callback runs once. For another clan it takes the
   building, shows string 5039 "Building is captured" as a System line with its
   voice, and at once opens the building for the player (a plant's page 5); for
   the player's own clan it only opens it.
6. Leaving is walking out: a door ahead opens as the unit nears it, and once a
   landscape face is nearer the unit is the landscape's again.

#### A shot opens a door — *read*, and *seen*

**The hit's opener** (`Control.dll:0x1000ebc0`, `ILifeSystem` slot 8, *read*)
runs before the hit's kind is looked at
([26-damage.md](26-damage.md#a-hit-from-the-round-to-the-node--read)):

1. **The object hit must be a building.** Its agent kind (`+0x50`) must be 3
   (`0x1000ebf7`), and it must have a device list (`+0x38`).
2. **The hit must name a node.** It needs an object (`+0xc`), a node that is
   not a shield sector's −2, and a batch (`+0x14`) that is not −1
   (`0x1000ec01`–`0x1000ec11`).
3. **It looks for a door on that node.** It walks the components from the last
   to the first (`0x1000ec13`–`0x1000ec54`). For each of class 12 it asks
   property `0x200` and stops at the first whose answer is the hit's node.
   Every door component's node is the node its channel plays: 56 of 56 in
   `fortif.rlb` (*measured*).
4. **It opens that door** through `IBuilding` slot 14 (`0x1000ec7e` →
   `Terrain.dll:0x1005b5d0`). Slot 10 turns the component into its door, and
   `0x1005b480` runs. A door that is shut (state 0) or closing (2) has its item
   switched on and becomes opening (3). An opening or open door is left as it
   is.

The hit then does its damage like any other.

*Derived* from those steps:
- **Nothing else is asked.** No clan, no hold, no lock: a shot opens any
  building's door, whoever fires. Nothing holds a door a shot opened, so it
  closes 5 s after it has opened unless a child near it holds it
  ([above](#walking-into-a-building--read-and-measured)).
- **Any hit that names a node** opens a door: a direct hit, and a blast's,
  which carries the struck face's node. A blast's hit reaches every building
  in range with the same node number, and only the number is compared. A
  nearby building with a door on that node index would open too; this is not
  seen.

**No door is locked in play** (*read*, as a search). `IBuilding` slot 6
(`0x1005b2f0`) locks a door: bit 1 of its `+0x58` and a value at `+0x60`, where
a value of 0 opens it at once. It is the only code that sets that bit. It has
no direct caller. Within 0x80 bytes after each of the 25 `IBuilding` queries
(interface `0x17`) in the install's DLLs and executable, no call goes through slot 6.
So the proximity opener's lock test (`0x1005a1ca`) never passes a door over.
That opener tests no clan either (`0x10059f40`): a neutral building's door
opens for a hero on its floor as the hero's own does.

**Mission 03's Small Bunker**, `fr_l_bunker` (*measured*, in model space):
- **The door.** One door, `i03`, node 9, component 3, with rate 0.25. It opens in
  4 s, and the building sees it open at 3.6 s. Its channel lowers the node
  from z 2.87 to −2.89, the door's own 5.76 m, into the floor.
- **The ramp.** The door (y −2.5 to 0.3) stands at the foot of the building's
  own ramp. The ramp falls from z 3.47 at y −35 to −5.56 at y −3, and lies
  below the landscape under it from about y −18 on. So a walker coming down
  it stands on the bunker's faces, the bunker's child.
- **The reach.** The door's capsule runs across the doorway along x, from
  (4.58, −2.07, −2.83) to (−5.03, −0.16, −2.83), 2.92 wide
  ([above](#walking-into-a-building--read-and-measured)). On the ramp at y −6,
  a hero's centre on the doorway's middle line is 4.8 from its segment, inside
  2.01 + 2.92. So walking down should open the door with no shot (*derived*).

**Against the recording** (*seen*, 5 fps from 164.4 s):
- 164.4–166.6 s: the hero walks down the ramp toward the closed door, which is
  dark with hazard stripes along its foot.
- 166.8 s: the battle laser's beam strikes the middle of the door.
- 167.2 s: a lit gap opens under the door's top edge, and widens as the edge
  drops.
- 171.5 s: the hero is in the corridor inside.

The shot and the approach come together, so the recording cannot tell which
opened the door. Both fit: the door began to move within 0.4 s of the beam, and
the hero was through inside 4.7 s, as a 4 s door allows.

**For an engine**: when a hit on a building names a node, open the first door on
that node as proximity would, with no hold. Gate neither opener by clan. The
round has to reach the door first: the black doorway quad stands a step in front
of every door that has one, and a round passes it as a mover does
([The doorways are portal quads](#the-doorways-are-portal-quads--measured-read-and-seen)).

### The ground inside a building — *read* in part, and *measured*

**The landscape is gone from under a building** (*derived*). Placing it deletes
every landscape face inside or across its `.bas` outer contour, and stitches in
new faces only outside the inner one
([03-terrain.md](03-terrain.md#placing-a-building-cuts-the-landscape--read-in-outline-and-measured)).
So inside the building the walk-face query
([Finding the ground](#finding-the-ground--read)) meets only the building's
level-0 faces. Its down pass takes the nearest walkable face under the body's
centre, and its up pass one less than r₂ above it. A unit can go down to the
Large Factory's pod floor at −12.4, although Tut_2's ground under the building
lies only 0.08 below its base. An engine that keeps the landscape draws sand
through the floor, and can hold a walker up on ground that is not there.

**What the collision query lets through** (*read*). The pair's face query
(`Control.dll:0x1001daf0`) builds its filter from the mover's collision flags
(`+0x10`, `0x1001db1e`):
- It excludes batches flagged `8` always, and `0x200` unless the mover's flags
  carry 4 (`0x1001db25`–`0x1001db48`, into the filter's `+0x14`).
- For the segment, it excludes triangles flagged 4 (`0x1001dbad`).
- For the push-out, it excludes triangles flagged 4, and flagged **2** unless
  the mover's flags carry 8 (`0x1001db2b`, `0x1001dbce`–`0x1001dbd1`, into a
  third filter's `+0x1c`, passed at `0x1001dfdb`).

`AniMesh.dll`'s push-out tests each gathered face against the filter: its
batch word at `+0x40` against required and excluded masks `+0x10`/`+0x14`, and
its triangle word at `+0x44` against `+0x18`/`+0x1c` (`0x1000dbd7`–`0x1000dc19`).
A face whose triangle word carries **`0x10`** goes on to the door test
(`0x1000dbba`, `0x1000dc5b`–`0x1000dd46`): the building's `IsDoorOpen` for the
face's node, and the face is dropped while it is open.

*Measured*: triangle flag `0x10` is the **door face**. On 17 of the 20 meshes
that carry it, the flagged triangles lie exactly on the controller's class-12
door nodes. On two institutes they lie on all their door nodes but one. On the
generator they lie on other nodes than its doors.

**The batch word is the file's** (*read*, and *measured*). The gathered face's
batch word is the first word of what the face source's slot 3 returns
(`AniMesh.dll:0x1000d71f`), and that slot, interface `0x18`'s
(`0x100134d0`), looks the batch record up in the node's mesh — stream 13, 20
bytes a record — and copies its **first dword** into its answer's first word
(`0x10013534`–`0x10013538`). The ray walker keeps the same dword for its own
batch masks (`0x100081ca`,
[26-damage.md](26-damage.md#the-query-record-and-what-a-round-excludes--read-and-measured)).
So bits 8 and `0x200` are the
file's, and so is the 2 that makes a hider hide both ways (`0x1000d865`).
Across the install's **15153** batches: **8 is on 633**, every one a
`DEFAULT`, `PORTAL_001` or `PORTAL_004` batch of `fortif.rlb` — 567 of its 594
`DEFAULT` batches and all 66 `PORTAL_*` ones, the 27 others lying in
lower levels of detail and damage variants, none at level 0 of the intact
variant — and **`0x200` is on none**. Each of the 633 also names a node in
its record's `+6` halfword, where the other 14520 hold `0xFFFF`
([below](#a-building-is-drawn-cell-by-cell-through-its-portals--read)).

**Who sets a mover's collision flags** (*read*, and *measured*). The flags the
query reads (collision object `+0x30`) are written by the object's slot 5
(`Control.dll:0x1001f670`): 2 from the word's `0x20000`, 4 from its
`0x4000000`, and **8 unless the word carries 4**. Its only callers in the
install are the two control dispatchers, at the end of message 4 with a
non-zero argument (`0x10007bd0`, `0x100319c5`), passing the current state's
flag word (`+0x104`, the state's `+0x04` copied to `+0x100`), to the object
the control keeps as interface `0x28` (asked for at its attach, `0x10007966`)
— the id the agent's build asks its new collision object for
(`AniMesh.dll:0x10003389`). A sweep of the sixteen modules for calls through `+0x14` that
push no argument and load `edx` finds these two and no other whose receiver is
a control's interface `0x28`. *Measured* over
the install's **206** controllers with states: **none mixes** states with and
without bit 4. **99 carry it in every state** — the 15 chassis of `bases.rlb`
that do not fly, the hero's `r_h_02` among them with 105 of 105; three
animals; the 80 trees and stones of `static.rlb`; and one of `system.rlb` —
and so lack 8; **107 carry it in none**: the nine flying chassis (`r_t_02`, `r_l_02`, `_05`, `_06`, `_07`,
`r_m_02`, `r_b_02`, `_07`, `_08`), the animals `a_a_l2` and `a_a_l3`, the 30
buildings and 66 rounds, and so carry 8. `0x20000` is on no state; `0x4000000`
on 31, all rounds, which pass batches flagged `0x200` (none is).

**The doorways are portal quads** (*measured*, *read*, and *seen*). `fr_b_plant`
carries 87 triangles of the material `DEFAULT`, all with triangle flags 0. 80
are at level 0, and 70 of those are on interior nodes. 8 of the 10 on outer
nodes have a coincident twin on an interior node, facing the other way. At the
entrance they are triangles 118 and 119 on node 1 (`o01`) and triangle 2294 on
node 2 (`i06`), all at y 89.3 across x −11…11, z 0…15.4. That is the whole
doorway, a step outside the door `i05`, with a third quad at y 87.4 on the hall
`i01` a step behind it.
- `DEFAULT` is `Material.lib`'s flags-4 material: blend mode 4, a black diffuse,
  the ambient alpha the device sees at 1, and the texture `DEFAULT.0`, 16 × 16,
  black and opaque. `World3D.dll` also falls back to it for a material it cannot
  find (`0x10004354`). `PORTAL_001` and `PORTAL_004` wear `PG23.0` and a bright
  green ambient.
- **How each is drawn is its batch word's**
  ([below](#a-building-is-drawn-cell-by-cell-through-its-portals--read)).
  One carrying `0x40` is never seen: 170 `DEFAULT` quads. A `DEFAULT` doorway
  without it, 397, is unseen within 75 of the camera and fades to black by 95,
  and beyond that the room behind it is not drawn at all. The 66 `PORTAL_001`
  and `PORTAL_004` quads are **signs**, green pictograms that fade in from 4 to
  36.
- *Seen*: Mission 03's Small Bunker carries a `DEFAULT` quad on its outer node
  `o01` at y −3.4…−1.5, in front of its door `i03` at y −2.5…0.3, and the hero
  comes down the ramp from −y. In the recording at 164.4–166.6 s that door is
  plainly in view, dark with hazard stripes along its foot, and the lit interior
  shows through the windows beside it. That quad's word is `0x148`, never seen.
- The Large Factory's entrance still reads black from outside at 88–93 s because
  the hall behind it is unlit, not because the quad covers it: its quads on
  `o01` are `0x148` too.
- It does not stop a walker either (*read*, and *measured*): every level-0
  doorway and portal batch carries 8 in its batch word, which the query
  excludes for every mover ([above](#the-ground-inside-a-building--read-in-part-and-measured)).
  **A round passes it too**: the round's query builds its filter the same way
  (`Control.dll:0x1001d9fa`), and with the quad solid a shot at the Large
  Factory's entrance strikes node 1, `o01`, a metre in front of the door, so no
  shot could ever open a door that has a doorway quad (*measured* on
  openparkan's engine).

#### A building is drawn cell by cell through its portals — *read*

`CBuilding` splits its mesh into **cells** and draws only the ones the camera
can see. The lists are built once, as the items are filed
(`Terrain.dll:0x100580b0`):

- node count into `+0xa0`, a byte per node into `+0x98` (drawn this frame) and
  a dword per node into `+0x9c` (the node's stream-1 flags, property `0xe`);
- node 0 always goes into the **exterior** list at `+0x80`;
- every other node whose flags carry **4** is a cell: with flag **1**, the
  interior bit, it joins the **room** list at `+0x74`, otherwise the exterior
  list.

*Measured* on `fr_b_plant`: 0x4 marks `o01`, `o03` and every `i*` room; the
three door nodes `i05`, `i19`, `i21` carry `0xd1` — no 4 — so each is drawn
with the room it hangs under, and `i16`, `i17` (`0x451`) with `i15`.

`CBuilding::Render` (`0x10058b90`) then:

1. culls the building's bounding sphere against the six frustum planes and
   hands the draw to its agent when it is out;
2. clears the drawn bytes and empties the portal queue at `+0x8c`/`+0x90`;
3. **camera outside the building's sphere**: draws the exterior list's entry 0
   and marks every exterior node drawn;
4. **camera inside it**: walks the room list, turns the camera into each room's
   frame (node matrix, property 2) and keeps the rooms whose box it lies in,
   then draws each;
5. drains the queue `CBuilding::PortalDrawNotify` (`0x1005a5d0`) fills as those
   cells draw: a portal names a node, and a node not yet drawn is queued — an
   interior one marks itself drawn, an exterior one marks the whole exterior
   list. Each queued node draws as a room if its flags carry 1, else as the
   exterior.

~~`PortalNearDist` and `PortalFarDist` are registered as variables
(`0x1005f24d`, `0x1005f26f`) and nothing reads them~~ — they are the render
settings' entries 27 and 28, 75 and 95 by default
([10-sky.md](10-sky.md#the-render-settings)), and `CShade` copies them to
`+0x1660` and `+0x1664` (`0x10046d77`, `0x10046d8e`) for the portal fade below.
`Ngi32.dll` exports `n3dGetPortalClipRect` and no module imports it: a portal
does not clip what is drawn through it.

**What a portal names** (*read*, and *measured*). `PortalDrawNotify` is slot 4
of the interface at `CBuilding +0x10` (vtable `0x1009b4ec`, whose slot 11 is
`Render`), and takes one argument, a face handle. It hands that to the
building's `IMesh2`, interface `0x18` (`CBuilding +0x2c`, so named by its
constructor's panic at `0x10055efa`; the notify reads it as its own `+0x1c`),
slot 3 (`AniMesh.dll:0x100134d0`), and reads the node
from the answer's `+0x18` (`0x1005a5ed`–`0x1005a5fc`). That slot fills `+0x18`
with the face's **batch record's `+6` halfword** plus its part's first node
(`0x1001353e`–`0x10013550`), and the notify refuses a node past the count
(*"Illegal joint #"*, `0x1005a60d`). *Measured*: that halfword is `0xFFFF` on
14520 of the install's 15153 batches and a node index on the other 633 — the
batches flagged 8, every doorway and portal quad — and on **625 of the 625** of
those that lie in a level-0 slot it is another node than the batch's own: the
room beyond. On `fr_b_plant` the entrance quads on `o01` name `i06`, `i06`'s
name `o01` and `i01`, and the `PORTAL_001` pair at the ramp's foot names `i15`
from `i13` and `i13` from `i15`. ~~**Not read**: who calls the notify with a
portal's face — no call through that slot is found — and so when during a
cell's draw it happens.~~ — **read**: `CShade`'s portal fade, as each portal
batch is drawn (below).

**Who calls the notify** (*read*). Interface `0x18` is `IMesh2`'s, and
`CBuilding` answers it with this interface at `+0x10` (its inner
`QueryInterface`, `0x10057c20`, jump table `0x10057d13`): a stand-in for its
real mesh at `+0x2c` whose slots pass on to the mesh's (slot 3, `0x10056c10`)
except slot 4, the notify, slot 6 (`GetFirstIntersectedFace`, `0x1005a6c0`) and
slot 11, `Render`. `AniMesh.dll`'s own `IMesh2` slot 4 is a bare `ret 4`
(`0x10007e80`), so for every mesh but a building's the notify does nothing.
1. `Render` hands that interface to `CShade` as the mesh it is drawing:
   `StartMeshRender`, `CShade` slot 5 (`0x100437c0`, called at `0x1005991f`),
   keeps it at `CShade +0xcb0`.
2. `CShade`'s mesh draw, slot 21 (`0x10044ea0`), asks that mesh for each
   batch's record (slot 3, `0x10045016`) and reads its first word. A batch
   carrying 8 and `0x20` is skipped whole (`0x10045044`–`0x1004505a`); one carrying 8 or
   `0x100` is filed translucent ([07-objects.md](07-objects.md#what-a-blended-batch-writes-and-the-alpha-tests-reference--read)).
3. Once the batch's primitive is built, a batch carrying 8 goes to
   `0x1002c4d0` (`0x10045d30`, its only caller), which sets the primitive's
   alpha and calls the mesh's slot 4 with the batch's face (`0x1002c65e`) —
   or does not.

**How a portal quad is drawn** (*read*). `0x1002c4d0` takes `d`, the distance
from the camera, `CShade +0xb98` (column 3 of the camera's matrix,
`0x1004651f`), to the primitive's first vertex, the first corner of its first
triangle (the primitive's slot 5, `0x1003c340`, a length through `Ngi32.dll`'s
reciprocal square root, `g_FastProc` entry 11). `f` is the camera's field of
view across, in radians (`ICamera` slot 7's `+0x14`, the angle slot 6 sets,
`0x100848a0`). By the batch word:
- **`0x10`**: the alpha runs from 0 where √d is 2.6 ÷ f to 1 where it is 7.8 ÷ f
  (`0x1009a8ec`, `0x1009a8f4`), and the room beyond is always drawn;
- **`0x40`**: the alpha stays 0 and the room beyond is always drawn;
- **neither**: from `0x1002c410`, near = 1.3 × `PortalNearDist` ÷ f and
  far = 1.3 × `PortalFarDist` ÷ f (`0x1009a908`), **75 and 95 at the game's 1.3
  rad**; the alpha is 0 within near, 1 beyond far and runs between, and **beyond
  far the notify is not called**, so the room beyond is not queued.

The alpha is written to the primitive's `+0x90` (`0x1002c63a`), its copy of the
material's ambient alpha (the material block at `+0x70`, `0x10029aa0`, whose
`+0x20` the draw also tests against 1 at `0x10045567`), which is the alpha the
device takes ([07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured)):
0 is not drawn at all, the alpha test dropping it, and 1 is opaque. So a
`DEFAULT` doorway is gone near by and a black wall far away, the black standing
in front of the room left undrawn behind it; and a sign is gone within 4 of the
camera and whole from 36 on, at 1.3 rad.

*Measured*, over the 633 batches flagged 8: **397** carry neither `0x10` nor
`0x40`, every one `DEFAULT`; **66** carry `0x10`, all 36 `PORTAL_001` and all 30
`PORTAL_004`, and no other batch in the install carries `0x10`; **170** carry
`0x40`, all `DEFAULT`. None carries `0x20`; all carry `0x100`, and 307 `0x4000`.
`fr_b_plant`'s level 0 holds 32 of the first, 4 signs and 9 open quads, the two
at the entrance on `o01` among them (`0x148`). `PG23.0` is an atlas of arrows
and pictograms, which the `PORTAL_*` materials' green ambient lights: these are
the signs *"follow the signs to the main engine room"* speaks of.

**So a building draws**: from outside its sphere, its exterior; from inside, the
rooms the camera stands in; and then every room a drawn portal names, as the
portal's batch is drawn — always through a sign or an open portal, and through a
`DEFAULT` doorway only while the camera is within far of it.

*Seen*: in the recording at 98–99 s, at the foot of the Large Factory's stairs,
green arrow signs stand on both walls of `i13` and a green band at the far end
of its ramp, and a green translucent sheet fills the left of the frame.
openparkan drew none of them before the signs were read, and draws them now from
the same place.

**Not implemented**: openparkan still draws every cell of a building at once. It
draws each portal quad by the fade above, so beyond far a doorway is the black
wall the game draws there, and what it hides — the room the game leaves out —
is drawn and never seen. The cost is frame time.

**The hall way's second word is the vertex's node** (*measured*). Posed through
the node it names, a vertex lands where it belongs:
- the Outpost's pod place on node 1 lands 0.3 from its pod's centre across the
  ground;
- the Large Factory's pod place (node `0x17`) lands at (0.02, −48.66, −11.02),
  over the pod floor.

Raw, the same vertices sit tens of metres off.

**The Large Factory's hall way is three groups with no link between them**
(*measured*): the forecourt and entrance, 9 vertices on nodes 1 and 4; the
interior with the pod, 62 vertices; and the rear, 13. Counting all 85 links,
the four gated ones among them
([The hall-way gates](#the-hall-way-gates-in-the-shipped-buildings--read-and-measured)),
the groups stay apart, with three lone vertices (18, 19, 20) beside them. The
front group never reaches the pod. The interior group does, from its three
side exits (flag 1): vertex 67 on the west at z 2.3, 68 above it at 9.4, and 69
on the east. Along the links their ways to the pod are 115.5, 161.4 and
170.3 m.

**For a walker only the low west exit leads in** (*measured*). The four gated
links are all a flyer's (`0x10000`) and all inside the interior group: 69–60
and 60–46 on the east, 68–64 and 64–41 on the west. With them out of the graph
68, 69, 60 and 64 stand alone, and the interior is 58 vertices whose one exit
is 67. So the only way to the pod the search gives a walker is 67's, through
the west side door `i21`, which the recording's hero takes; 68 and 69 are a
flyer's.

**What joins the groups is the areal map** (*read*). An exit is linked to the
walkable areal under it at a cost of 1 (`ArealMap.dll:0x1002363f`,
[The global path](#the-global-path--read-and-measured)), and each group has
its own: the front group's 2, 78 and 79 (flag `0x10000001`, any size), the
rear group's 70, 73 and 76, and the interior's 67. On Tut_2 all nine exits
stand over areals whose word is 1 (*measured*, the placement turning them into
the world as the table below does: 67 lands at (337.4, 790.0)). So a walker
sent to the pod from the forecourt is led off the front group, over the areals
round the building and in at 67 (*derived*).

From the front there is a way on the faces alone (*measured*). A flood over
the walkable level-0 faces on a half-metre grid, from the forecourt at 0,
reaches the pod floor only when a step may rise more than 0.3 m. With steps of
up to 0.6 m it gets there in about 240 m, over a gallery at 7.1. The recording
does not take it.

#### The way to the pod — *measured*, and *seen*

**The hero goes in by the west side door.** The hall way's shortest way from
the west exit runs through the side door `i21` (node 16) and down to the pod.
Its vertices stand about 1.4 above the floor, and the floor under each is the
walkable level-0 face nearest that height.
- *Model* is model space, z up from the placement.
- *World* is Tut_2's, from the placement (392.42, 788.73, 151.75) turned by
  −0.0246 rad.
- *Along* is the distance along the links.

| vertex | node | model (x, y, z) | floor under it | world (x, y, z) | along |
|---|---|---|---|---|---:|
| 67, the exit | `o03` | (−55.08, −0.04, 2.30) | none: the landscape | (337.36, 790.04, 154.05) | 0 |
| 66 | `o03` | (−39.66, −0.04, 2.30) | none: the landscape | (352.77, 789.66, 154.05) | 15.4 |
| 65 | `o03` | (−28.68, −0.04, 2.96) | 0.70, 8° | (363.75, 789.39, 154.71) | 26.4 |
| 58 | `i10` | (−22.56, −0.04, 2.96) | 0.96, flat | (369.87, 789.24, 154.71) | 32.5 |
| 59 | `i10` | (−22.20, 4.40, 2.96) | 0.96, flat | (370.33, 793.67, 154.71) | 37.0 |
| 37 | `i12` | (−16.86, 4.56, 2.90) | 0.71, 30° | (375.68, 793.70, 154.65) | 42.3 |
| 36 | `i12` | (−6.18, 4.56, −3.58) | −5.52, 30° | (386.35, 793.44, 148.17) | 54.8 |
| 6 | `i13` | (0.00, 4.26, −3.58) | −5.76, flat | (392.52, 792.99, 148.17) | 61.0 |
| 7 | `i13` | (0.00, −4.80, −3.58) | −5.76, flat | (392.30, 783.93, 148.17) | 70.1 |
| 25 | `i13` | (0.02, −10.26, −3.58) | −5.76, flat | (392.19, 778.47, 148.17) | 75.5 |
| 35 | `i13` | (0.02, −32.88, −12.04) | −13.85, 21° | (391.63, 755.86, 139.71) | 99.7 |
| 28 | `i15` | (0.02, −39.24, −12.04) | −14.02, 5° | (391.47, 749.50, 139.71) | 106.0 |
| 31, the pod | `i15` | (0.02, −48.66, −11.02) | −12.40, flat | (391.24, 740.08, 140.73) | 115.5 |

Between the vertices, on half-metre samples:
1. **66 → 58.** A floor at 0 and an 8° ramp up to 0.70 on the outer shell
   `o03`. Then the side door `i21` at x −26 and the cross corridor `i10` at 0.96.
   The doorway's `DEFAULT` quads stand at x −26.9 on `o03` and −25.0 on `i20`.
2. **59 → 36.** The stairs `i12`. Their collision faces are a smooth 30° ramp,
   with 64° side pieces, down to −5.52.
3. **6 → 25.** The corridor `i13` at −5.76.
4. **25 → 35.** A 21° ramp, 24 m long, down to −13.85.
5. **35 → 28.** The pod room `i15`, at 5°, between −13.85 and −14.32.
6. **28 → 31.** A 28° rise of 1.6 onto the pod's floor at −12.4, over the pod
   `i16`/`i17`.

All 191 samples lie on faces flagged 2, and the slopes are 0, 5, 8, 21, 28 and
30°. The floor runs from −14.32 to 0.96 and never changes by more than 0.30
between neighbouring samples. So the way has no step, only ramps.

*Seen*, against the recording:

| time | where the hero is |
|---|---|
| 94.5 s | the dark doorway |
| 96.5 s | the cross corridor's green lamps |
| 98.0 s | `i13`'s arches |
| 99.0 s | `i13`'s ramp, sloping down |
| 100.5 s | the pod room |
| by 102 s | on the pod |

*Derived*: 89 m from the door to the pod in about 7.5 s is 12 m/s on average,
with the door's opening on the way. The recording draws the interior lavender
and white where openparkan's engine draws it red; that is not looked at here.

**Portal quads stand between the rooms** (*measured*). The passages between
the interior's rooms hold pairs of see-through quads with triangle flags 0, like
the doorways. They are `Material.lib` flags-4 materials: `DEFAULT`,
`PORTAL_001` and `PORTAL_004`.
- On the way, four `PORTAL_001` triangles on `i13` cross the ramp's foot at
  y −34.1, before the pod room.
- Across `fortif.rlb`'s level-0 faces, the see-through faces with no flag are
  `DEFAULT` 1240, `PORTAL_001` 140, `PORTAL_004` 112 and the `NE_S…` family 121.
  What the `NE_S…` ones are was not looked at.
- Every see-through face's flags lie within 0, 2, 4 and 32.

*Measured on openparkan's engine*, the hero walking the table's waypoints:
- It waits 2.3 s at the side door for the door to open.
- Its stand-ins pass `DEFAULT` and let walkable faces not push. With those, it
  walks down the stairs and the ramp and stops at `PORTAL_001`.
- Passing `PORTAL_*` too, it reaches the pod, and the capture fires 4.4 s after
  it arrives.
- With the read rule in place of the walkable stand-in (faces flagged 2 and 4
  pass, the rest push), and `DEFAULT` and `PORTAL_*` passing, it walks the whole
  way, the stairs' side pieces included. It takes 7.8 s from vertex 65, before
  the door, to the pod (each vertex counted within 2.5), against the recording's
  7.5 s.

**What a gathered face carries** (*read*). The push-out copies each gathered
face into a 72-byte record (18 words, `AniMesh.dll:0x1000d75d`):
- its batch word `+0x40` is the first word of what the face source's slot 3
  returns (`0x1000d71f`, stored at `0x1000d724`);
- its triangle word `+0x44` is the first word of the triangle record that the
  source's slot 5 returns for (2, 3) (`0x1000d668`, stored at `0x1000d738`).

No instruction in `AniMesh.dll` writes 8, `0x200` or `0x2000` into a `+0x40`,
because nothing needs to: the word is the batch record's own first dword
([above](#the-ground-inside-a-building--read-in-part-and-measured)).

The pair's query builds its filter with a constructor of six words
(`Control.dll:0x10013f60`). Its first word is the OR of five of the mask
globals at `0x1003c1ac` (2, 4, 8, `0x10`, `0x400`): `0x41e` (`0x1001db14`).
What that word selects is not read.

**How the machine takes a building's push** (*read*, `0x1000c990`). With state
bit 4:
- The push is taken whole when the machine's parent answers 3 in slot 11 (a
  building) and the push's z is 0 or less (`0x1000ca08`, against the 0.0 at
  `0x1003b18c`).
- Otherwise its z is dropped, and x and y are scaled by |P| ÷ |P_xy|, at most
  4.0 (`0x1000ca8b`). A push with no x or y part moves nothing.

So a ceiling's push, downward, is taken whole. A flat floor's push, straight up,
moves nothing, and a sloped floor's becomes a push down the slope.

**The floors do not push a walker: flag 2 is the exemption** (*read*,
[Collision between objects](#collision-between-objects--read)). The push-out's
filter drops triangles flagged 2 unless the mover's collision flags carry 8.
Every floor on the way to the pod carries 2, so none of them pushes a mover
without 8. The walls, the stairs' 64° side pieces and the portal quads carry no
2, and they still push.

Without that rule the ramps would push the hero back (*derived*). The sphere is
the agent's joined sphere
([26-damage.md](26-damage.md), `AniMesh.dll:0x10009510`): r 2.18 for the hero.
Its centre stands about h = 1.37 above the feet (*measured* on openparkan's
engine, which poses the same parts). A floor at slope θ lies h cos θ from the
centre, less than r, so it is always in reach. Its push has depth p = r − h cos θ, a
horizontal part p sin θ, and after the rule the lesser of p and 4 p sin θ:

| floor on the way | depth p | horizontal part | pushed down the slope |
|---|---:|---:|---:|
| flat | 0.81 | 0 | 0 |
| 5°, the pod room | 0.82 | 0.07 | 0.28 |
| 8°, the side ramp | 0.82 | 0.11 | 0.46 |
| 21°, `i13`'s ramp | 0.90 | 0.32 | 0.90 |
| 28°, onto the pod | 0.97 | 0.46 | 0.97 |
| 30°, the stairs | 0.99 | 0.50 | 0.99 |

*Measured on openparkan's engine*, walking the way:
- with r 2.18 the largest horizontal part is 0.32, on `i13`'s ramp, as the
  table has it;
- with the ground search's own sphere, r 1.15, no walkable face comes in reach
  anywhere on the way.

Such a push would land each time the pass runs, and shove the hero down every
ramp, and back against every one on the way out. The recording shows the hero
walk in at about 12 m/s and out again from 158 to 166.5 s. So the hero's
collision flags lack 8, which was *derived* here first and is now **read**:
every state of the hero's controller carries bit 4, and a walker's flags take
8 only from a state that lacks it
([above](#the-ground-inside-a-building--read-in-part-and-measured)). A flyer's
states all lack it, so a building's floors push a flyer. Triangle flag 2 marks
exactly the walk-through floors
([07-objects.md](07-objects.md#stream-7-is-the-per-face-record)).

**The slope brake reads a building's floors too** (*read*). The mode-2 brake
tests the body's ground normal `+0x194` (`0x100156c6`), whose only writers are
the body's constructor and the lift (`0x10015e47`), which averages the normals
of whatever faces the sphere and the flag-1 contacts stood on, a building's or
the landscape's; nothing in between asks whose they are
([Holding the body](#holding-the-body-on-the-ground--read-and-measured)). So
the 30° stairs brake a hero climbing out, to 2 (cos 30° − cos 0.6) ÷
(1 − cos 0.6) = 0.47 of its speed (*derived*). The recording's hero walks out
from 158 to 166.5 s, a second slower than in, which a braked climb up 12.5 m of
stairs would account for. But openparkan's engine, with the brake on a
building's faces, never gets its hero up Mission 04's teleport chamber to the
field, which the recording's hero reaches in 4.6 s. What reconciles those is
not read, and the engine leaves the brake out on a building's faces.

**Steps and lift** (*read*,
[Finding the ground](#finding-the-ground--read) and
[Holding the body on the ground](#holding-the-body-on-the-ground--read-and-measured)).
- A contact takes the nearest walkable face below it, or one less than r₂ above
  the centre.
- The lift is the largest rise over the flag-1 contacts, whatever its height.
- No step limit is read.

On this way the floor never rises more than 0.30 between half-metre samples, so
neither limit comes into play.

**For an engine**, walking into a building:

1. Cut the landscape under every building
   ([03-terrain.md](03-terrain.md#for-an-engine)): inside the outer ring the
   building's faces are the only ground and the only thing drawn.
2. Find the ground inside by the walk-face query over the building's level-0
   faces (normal z above cos 80°): the nearest below the centre, or one less
   than r₂ above it. Lift the body onto it.
3. Collide against the building's faces, the one stood on included, except:
   - triangles flagged 4;
   - door faces (flag `0x10`) while their door is open;
   - faces whose batch's flags dword carries 8: the `DEFAULT`, `PORTAL_001` and
     `PORTAL_004` quads, and only they.
4. In the push-out, drop faces flagged 2 unless the mover's collision flags
   carry 8: a walker's, every one of whose states carries bit 4, never do, and
   a flyer's do. The floors and ramps are then climbed by the lift alone. The
   stairs' 64° side pieces carry no 2 and still push, and the hero still walks
   the stairs (*measured* on openparkan's engine). STAND-IN in openparkan: no
   robot keeps the floors; with them, Mission 02's flyer made at the Large
   Factory's creation vertex is pushed 31 m up off its floor and over the shut
   front door, and how a flyer's height meets this push is not read.
5. Take the push as the machine does: with state bit 4, whole when the parent
   is a building and z ≤ 0; otherwise flattened, lengthened to |P| and at
   most ×4.
6. The way to the Large Factory's pod is the hall way's, over the links the
   unit may cross: for a walker, from the west exit alone, vertices 67, 66, 65,
   58, 59, 37, 36, 6, 7, 25, 35, 28 and 31, in the world as the table gives
   them; a flyer may come in at 68 or 69 as well. The side door opens as the
   walker nears it (step 2 of
   [Walking into a building](#walking-into-a-building--read-and-measured)),
   and takes 2.5 s to open at rate 0.4.
7. Draw no portal quad: the `DEFAULT`, `PORTAL_001` and `PORTAL_004` faces are
   the openings between the building's cells, and a drawn one hides the door a
   step behind it and the next room beyond it
   ([above](#a-building-is-drawn-cell-by-cell-through-its-portals--read)).
8. Pass them with a round as well as with a mover, by the same batch word, so
   a shot reaches the door behind the doorway
   ([A shot opens a door](#a-shot-opens-a-door--read-and-seen)).

#### The ways into Mission 03's Small Generator and Small Bunker — *measured*, and *seen*

*The Field Base* sends the hero into two buildings for their pods: the Small
Generator (`gener01.dat`, `fr_l_gener`) and the Small Bunker (`sbunk01.dat`,
`fr_l_bunker`). **Both ways in are their hall ways' shortest ways from an exit
(flag 1) to the pod (flag `0x40`)**, as the Large Factory's is, and each is
walkable on the rules above with nothing more. The columns are the table's
above; *world* is Tut_3's, from the placements (651.79, 1051.17, 92.51) turned
by −0.0631 rad and (1260.93, 813.89, 80.35) turned by −1.6016 rad.

**The Small Generator** is two mirrored halves: 57 vertices, 56 links, the pod
vertex 16. Of its eleven exits, five reach the pod: 46 and 49 to the north
(`o06`, both also ground places, `0x10000000`), and 51, 53 and 54 to the south
(`o07`), 112.5 to 119.7 along. The recording's hero comes from the south,
past the warehouse, and takes the south way:

| vertex | node | model (x, y, z) | floor under it | world (x, y, z) | along |
|---|---|---|---|---|---:|
| 51, the exit | `o07` | (0.00, −64.24, 2.72) | none: the landscape | (647.73, 987.06, 95.23) | 0 |
| 55 | `o07` | (0.00, −55.93, 2.72) | none: the landscape | (648.26, 995.35, 95.23) | 8.3 |
| 52 | `o07` | (0.00, −51.32, 2.95) | −0.30, 14° | (648.55, 999.96, 95.46) | 12.9 |
| 56 | `o07` | (0.00, −32.20, −2.21) | −5.14, 14° | (649.76, 1019.04, 90.30) | 32.7 |
| 35 | `i12` | (0.00, −26.50, −3.51) | −6.27, 13° | (650.12, 1024.73, 89.01) | 38.6 |
| 30 | `i12` | (0.00, −15.99, −5.82) | −7.68, flat | (650.78, 1035.22, 86.69) | 49.3 |
| 33 | `i12` | (8.67, −13.79, −5.14) | −7.68, flat | (659.57, 1036.86, 87.37) | 58.3 |
| 32 | `i12` | (13.30, −17.60, −5.85) | −8.17, 9° | (663.95, 1032.77, 86.66) | 64.4 |
| 15 | `i04` | (18.21, −20.11, −6.15) | −8.88, 5° | (668.69, 1029.95, 86.36) | 69.9 |
| 8 | `i04` | (23.54, −19.59, −7.27) | −9.58, 17° | (674.04, 1030.13, 85.24) | 75.3 |
| 7 | `i04` | (27.72, −14.65, −7.80) | −10.55, 6° | (678.53, 1034.81, 84.71) | 81.8 |
| 9 | `i04` | (27.72, −8.01, −7.80) | −10.56, flat | (678.95, 1041.43, 84.71) | 88.5 |
| 6 | `i04` | (20.88, 0.00, −8.20) | −11.97, 10° | (672.62, 1049.86, 84.31) | 99.0 |
| 17 | `i05` | (13.50, 0.00, −10.24) | −12.48, flat | (665.26, 1050.32, 82.27) | 106.7 |
| 16, the pod | `i05` | (7.68, 0.00, −10.12) | −12.48, flat | (659.45, 1050.69, 82.39) | 112.5 |

Along it, on half-metre samples:
1. **51 → 52.** No face of the building under the first 12 m: the way starts
   on the landscape, crosses the inner ring between 55 (between the rings) and
   52 (inside), and the building's floor starts under it at model y −52.3, at
   −0.04.
2. **52 → 56.** A 14° ramp down, 19 m long. Half-way to 35 it passes the
   entrance's `DEFAULT` quads (`o07`, `i30`, `i12`) and the sliding door `i32`
   (node 13, rate 0.7, sliding 5.18 across): its faces carry triangle flags 0,
   not the door face's `0x10`, and its lower edge gives the only two samples
   off a floor, a 51° face at −3.80.
3. **35 → 6.** Corridors and ramps of up to 17°, with `DEFAULT` quads between
   `i12` and `i04`.
4. **6 → 16.** `PORTAL_001` quads between `i04` and the pod room `i05`, and the
   pod's floor at −12.48.

203 of the 205 samples lie on faces flagged 2, from −12.48 to −0.04.

**The Small Bunker** is 44 vertices and 51 links with **one exit, 43, and its
pod, 6**, 149.8 along. The way comes down the sunk ramp
([A shot opens a door](#a-shot-opens-a-door--read-and-seen)) and through the
bunker's one door:

| vertex | node | model (x, y, z) | floor under it | world (x, y, z) | along |
|---|---|---|---|---|---:|
| 43, the exit | `o01` | (−13.66, −63.24, 6.06) | none: the landscape | (1198.14, 829.49, 86.40) | 0 |
| 42 | `o01` | (−10.30, −51.88, 6.06) | none: the landscape | (1209.39, 825.78, 86.40) | 11.8 |
| 41, the ramp's top | `o01` | (−7.26, −35.24, 6.82) | 3.47, flat | (1225.93, 822.23, 87.17) | 28.8 |
| 40 | `o01` | (−3.58, −17.80, 4.22) | 0.38, 23° | (1243.24, 818.02, 84.56) | 46.8 |
| 39, before the door | `o01` | (−0.86, −3.88, −3.33) | −5.18, 20° | (1257.07, 814.87, 77.01) | 62.9 |
| 37 | `i02` | (1.06, 6.20, −3.33) | −5.75, flat | (1267.09, 812.64, 77.01) | 73.1 |
| 36 | `i02` | (9.70, 4.60, −3.33) | −6.03, 24° | (1265.22, 804.06, 77.01) | 81.9 |
| 25 | `i09` | (21.70, 2.52, −9.09) | −11.35, 23° | (1262.78, 792.13, 71.26) | 95.4 |
| 27 | `i09` | (27.17, 1.43, −9.09) | −11.48, flat | (1261.52, 786.69, 71.26) | 101.0 |
| 28 | `i09` | (28.16, 5.52, −9.09) | −11.50, 15° | (1265.58, 785.58, 71.26) | 105.2 |
| 23 | `i09` | (29.93, 14.34, −11.17) | −13.91, 15° | (1274.34, 783.54, 69.17) | 114.4 |
| 22 | `i09` | (31.45, 22.95, −11.17) | −13.96, flat | (1282.90, 781.75, 69.17) | 123.1 |
| 8 | `i12` | (16.79, 26.03, −13.00) | −13.74, 5° | (1286.42, 796.31, 67.35) | 138.2 |
| 6, the pod | `i12` | (5.66, 28.16, −10.79) | −12.04, flat | (1288.90, 807.37, 69.55) | 149.8 |

Along it:
1. **43 → 41.** No face of the building under the first 17 m: the way crosses
   the inner ring between 42 (between the rings) and 41 (inside), and the
   building's floor starts under it at model y −45.3, at 3.02, and runs flat at
   3.47 to the ramp's top.
2. **41 → 39.** The ramp, 20 to 23°, down to −5.18.
3. **39 → 37.** The door `i03` (node 9, rate 0.25, lowering 5.76): its faces
   carry `0x10`, and `DEFAULT` quads stand on `o01`, `i01` and `i02` at it.
   The two samples at its threshold have no floor.
4. **37 → 22.** The corridors `i02` and `i09`, ramps of up to 24°, `DEFAULT`
   quads between them.
5. **22 → 6.** `PORTAL_001` quads between `i09` and the pod room `i12`, and a
   see-through `B_COMP_3G` face flagged `0x20` before the pod.

All 258 floored samples lie on faces flagged 2, from −13.96 to 3.70.

**Off the way the ramp's mouth has walls** (*measured*). A walker heading east
along world y 815, 9 m south of the way, meets the building at model
(0.18, −41.94): `o01`'s vertical `B_GEN_08` face (normal −x) and a 32°
`B_RL_06` face stand within 0.4 of (0.18, −41.94, 4.45), where a hero's centre
is at world z 84.8.
They carry no flag 2, so they push. *Measured on openparkan's engine*, a hero
started there at (1212, 815) facing east slides along them and walks round the
building.

*Measured on openparkan's engine*, the hero walking each table's vertices with W
held from the exit, each vertex counted within 1.2:
- **The Small Generator.** On the building's floor 1.15 s in, on the pod at
  8.32 s, and captured at 9.37 s: 8.2 s from the floor to the capture. Its step
  from the landscape onto the floor lifts it 2.8, from 89.50 to 92.28. The door
  `i32` opens as it comes, and objective 1 completes at the next Mission handler.
- **The Small Bunker.** On the building's floor 1.60 s in, the ramp's top at about
  2.5 s, at the closed door from 4.75 to 8.0 s, on the pod at 13.72 s, and in
  command mode at 17.07 s: 14.6 s from the ramp's top. Its step onto the floor
  lifts it 1.6, from 81.98 to 83.55.

*Seen*, the recording at 2 fps:

| time (s) | the Small Generator |
|---|---|
| 104.0 | walking north between the pillars |
| 108.0 | the grey apron under the hero |
| 109.5 | down the dark ramp |
| 110.5 | the laser strikes the door ahead |
| 111.5 | through the doorway |
| 112.0–114.0 | the green corridors |
| 116.0 | on the pod |
| 116.5 | *"from: System / Building is captured"* |

| time (s) | the Small Bunker |
|---|---|
| 162.5 | the forecourt's grey under the hero |
| 164.0 | the ramp's top, the tower ahead |
| 166.5 | the laser strikes the door |
| 167.5 | the corridor |
| 178.2 | command mode ([40-command-mode.md](40-command-mode.md#against-the-recording--seen)) |

*Derived*: the generator's apron to its capture takes 8.5 s in the recording
and 8.2 on the engine; the bunker's ramp top to command mode 14.2 s and 14.6.
Both of the recording's heroes shoot the door on the way, and the engine's
doors open as the hero comes, so the shots change nothing here.

**For an engine**: walk Mission 03's buildings by their hall ways, as step 6
above does the Large Factory's: the Small Generator's from 51 (or 46, 49, 53,
54), the Small Bunker's from 43, in the world as the tables give them. The rules
above let a walker through; a walker off the bunker's way meets the ramp's
walls.

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

- **The global path** is `0x10036730`, which `MWalker::SetTarget` calls at
  `0x1003bfda` and `AppendTarget` at `0x1003c3b3`. It searches the walker's
  `MWorldGraph` (`+0x128`) and puts a waypoint into the global queue `+0xe8`
  for each step ([The global path](#the-global-path--read-and-measured)). The
  local queue `+0xfc` and the trajectory `+0x110` (records of `0x48` bytes)
  are built from it.
- ~~`0x1003fbf0` is the global path~~: nothing calls it, and no pointer to it
  lies anywhere in `Behavior.dll`; it is dead code. The interface `0x303` slot
  14 it asks for is `MHallWay`'s `0x1000a930`, which always answers 0; the
  system areal map's query (`ArealMap.dll:0x10020c60`) answers only 0, `0x302`
  and `0x305`.
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
`FlyNearLandHeight` 15, and they reach no point: the profile binds them at
`+0xa8` and `+0xac`, and their one reader, `Behavior.dll:0x100153a0`, which sets a
point's z to the ground under it plus one of the two, is never called — no call or
jump reaches it, its address stands in no shipped DLL as a pointer, and it is not
exported, where the same scan finds all 7 calls of the ground routine
`0x100146b0` it calls ([36-factory.md](36-factory.md#not-established)).

### The global path — *read*, and *measured*

**The areal map links only walkable areals** (`ArealMap.dll:0x10023240`,
run at load at `0x1001e657` and again as a static object is added or
removed). An areal whose record's first flag word (`+0x20`,
[08-arealmap.md](08-arealmap.md)) is 0 gets no links at all
(`0x100232ab`). Across each edge, a neighbour whose word is 0 is skipped
(`0x100232f4`). The engine's own strings call such an areal
*Non-Walkable*. So the word is the one command mode tests before it sends a
non-flyer anywhere ([42-selection.md](42-selection.md#a-valid-place--read-and-measured)).

**What the links cost:**

| link | cost | where |
|---|---|---|
| areal to areal across an edge | the distance between the two records' centres, across the ground, + 1 | `0x1002342b` |
| a walkable areal and a building's exit over it | 1 | `0x1002363f` |
| hall-way vertex to vertex | 3 × the distance + 5; 10 when either vertex has flag 2 | `0x1000a1dd` |
| a vertex with flag 4 to another building's with flag 4, closer than 50 | 0.1 | `0x1000b663`, `0x1000b6bc` |

An exit is a hall-way vertex with flag 1 over a walkable areal. Buildings do
not cut areals: only a tree or a stone (object kind 10) cuts the areals it
stands on
([below](#a-tree-or-a-stone-cuts-the-areals-it-stands-on--read-and-measured)).

**A bridge's halves meet at their flag-4 vertices.** *Measured*: each of the
four half-span `fr_*_brige` hall ways has three or five exits at its landward
end and one vertex with flag 4 toward the joint. On Tut_1 and KM_4 every exit stands on a
walkable areal, and the two halves' flag-4 vertices are 6.96 and 8.24 apart.
So the only link across either canyon runs over the deck.

**The search** (`MWorldGraph`, `Behavior.dll:0x10042c10`):

1. It is A\*, its open list ordered by the cost so far plus the distance from
   a node's centre to the goal's (`0x100431c0`); an areal's centre is its
   record's `+0`..`+8`, at height 0.
2. Each link's new cost is (the cost so far + the link's) × (1 + `rand()` ÷
   32767 × 0.7). The 0.7 is the graph's `+0x38` (`0x1003642b`, `0x10042e0b`).
   The factor compounds along the path.
3. It stops as soon as it reaches the goal (`0x10043011`, "Reached"). It
   fails after taking 2048 nodes off its open list (`0x100207df`).
4. A flag `0x20000` on a link is never crossed; `0x10000` only with `CanFly`
   (`0x10036934`). Links between areals carry `0xffff`.
5. A hall-way vertex also gates the unit by size (`0x10042d08`): flag
   `0x10000000` passes; `0x20000000` passes when the building's property
   `0x201` is at least the unit's `+0x960`; otherwise only a unit whose
   `+0x960` is at most 2.
   `+0x960` is the **size class** — T 1, S 2, M 3, L 4 — which the behaviour
   takes from the root component's member name, a unit's third letter and a
   building's fourth ([below](#not-established)), so the gate measures a unit
   against the building it is walking into.

The function names itself: both its bug lines read
*"\*\*\* Bug!!!: MWorldGraph::AddNeighbourToFront()"* (`0x10042d7d`,
`0x10042d84`).

#### The hall-way gates, in the shipped buildings — *read*, and *measured*

**A vertex's gate is its own flag word**, the one the file carries at +0xc of
its 20-byte record. The loader keeps it at the live vertex's `+0x10`
(`ArealMap.dll:0x1000a4fb`–`0x1000a534`, 92 bytes a vertex), and the search
reads it back through `MHallWay` slot 4. *Measured* over the **29 hall ways,
1056 vertices and 1096 links** the install ships, all of them in `fortif.rlb`:

| gate | vertices | where |
|---|---:|---|
| `0x10000000`, any size | 165 | every bridge vertex (9, 9, 9, 10), the mines' and stores' yards, the hangar, the Small Generator, the factories' approaches |
| `0x20000000`, the building's own size | 7 | the three factories alone — `fr_b_plant` 2, `fr_l_plant` 3, `fr_m_plant` 2 |
| neither, size class 2 or less | 884 | everything else |

So the gate says what the capture rule says. **All 21 control pods are
"size 2 or less"** vertices, which is the same bound the capture order puts on
a capturer ([27-ownership.md](27-ownership.md#capture--read)) — the path graph
refuses a medium or large bot the pod a second time. Of the 124 exits (flag
1), 66 are ground-level and 58 are not. The missions place **190 units and
buildings of size class 3 or 4** against 222 of size 1 or 2, so the gate is
shut to nearly half of what is on a map.

**The link flags come from the link's own tail.** The loader builds a 56-byte
link and copies the file's forty bytes — start, end and eight further words —
to its front, the eight as four pairs at `+8`..`+0x24`
(`ArealMap.dll:0x1000a568`–`0x1000a5a0`). The neighbour the search is handed
takes `0x20000` when **both** the link's `+0xc` and `+0x1c` are zero and
`0x10000` when the first is zero and the second is not (`0x1000a274`,
`0x1000a294`, and the same pair for the other direction at `0x1000a2d4`,
`0x1000a2f4`) — so they are the file's tail words 1 and 5.

*Measured*, over all 1096 links: **1078 carry neither, 18 carry `0x10000` and
none carries `0x20000`.** The eighteen sit on the three mines (2 each) and the
three factories (4 each), and are the only links in the shipped game a walker
may not cross; a flyer may, and nothing in the install is shut to everything.
The tail takes three shapes: all −1 on 1034, (−1, 0, −1, 0, −1, −1, −1, −1) on
the eighteen, and a node index with 1 beside it, four times over, on 44. This
page's reader had the tail down as "0xFFFFFFFF throughout"; it is not.

*Not established* beside it: the door list a link opens
(`ArealMap.dll:0x1000b170`, [above](#walking-into-a-building--read-and-measured))
is four dwords at the live link's `+0x28`, which is past the forty bytes the
load copies, so whether the 44 tails above are where it comes from is not read.
The opener takes each of the four that is not −1, asks the building for that
child's class and, on class 12, `CICLS_DOOR`, opens it.

#### Who relinks a hall way — *read*

`MHallWay` slot 11 (`0x1000b390`; the vtable is `0x10039420`, installed by
`CreateHallWay` at `0x10009749`) fetches the system areal map and walks the
hall way's own vertex array at `+0x28`. **Two sites in the whole install call
it**, both in `ArealMap.dll` and both at the end of the same pass: `OnAddStatic`
(`0x10022cae`) and `OnRemoveStatic` (`0x10023168`). Each asks the world for
its scene objects of kind 3 — the buildings — and, for every one that answers
interface `0x303`, runs slot 11 on its hall way. So a building's exits are
joined to the areals under them **when the map is built and again whenever a
tree or a stone is added or taken away**
([below](#a-tree-or-a-stone-cuts-the-areals-it-stands-on--read-and-measured)),
right after the broken areals themselves have been relinked by `0x10023240`.

**Where a walker may be sent.** `MWalker::SetTarget` first empties the three
queues, and answers 1 only once the global path is found (`0x1003bfdf`).
It then looks up the areal under the goal (system areal map slots 7
and 6). It refuses the goal when there is none, or when the areal's word is 0
and the chassis profile lacks `CanFly` (`0x1003bd5d`). A flyer is not
searched at all on open ground: `SetTarget` copies `CanFly` into walker
`+0x28` (`0x1003bb94`) and queues the goal straight (`0x10036915`). It clears
`+0x28` when either place is on a building (`0x1003bfcc`).

**The waypoints** (`0x10036a80`). Each step from one areal into the next puts
a point on an edge the two share. Where the straight line to the goal crosses
that edge, the point is the crossing. Otherwise it is one of the edge's ends
moved 3 along it (`0x10036cc5`): the one that makes the way through it,
from the last point to the goal, shortest.

**Off a non-walkable areal** (`0x1003dde0`). A walker whose search cannot
start because the areal under it has no links logs "Cannot leave Non-Walkable
Areal" (`0x1003e2dc`). It then tries 50 random points in a square about
itself, of half-width 30 (`0x1003e2f4`). Each round adds 3 to the half-width,
while it stays under 500 (`0x1003e400`). It takes the first point on a
walkable areal, empties its queues and goes there. With none it logs
"Warbot is absolutely in non-walkable". The square doubles its half-width on
every try for a unit whose slot 14 answers `0x20000000`.

**The areals are not all convex** (*measured*). 10,441 of the 34,662 areals
turn back by more than 0.6° at some corner, many by a right angle. A
straight line between two points of one of them can leave it.

**KM_4's canyon** (*measured*). *The Iron Monster* (C02 M01) has a canyon
across its valley. Its floor under the bridge lies at 18 m, the plateaus
either side at about 50 m. Every areal of the canyon floor and walls has the word 0,
and the plateaus either side have 1. So the walker finds no way down into
it, and none across but over the bridge.

### Dropping the points a unit has passed — *read*

`MWalker::ClearMoverReachedPoint` (`Behavior.dll:0x1003cfd0`, which names
itself in its own log line at `0x1003d16e`) is the walker's book-keeping when
the mover has eaten part of the trajectory. The trajectory is the list at
`+0x11c`, `0x48` bytes a record, `+0x114` records long; the mover's own count
sits at `MBehaviour` `+0x250`.

1. **It gives up the place it was holding.** If the walker's `+0x88` is a
   building id rather than −1, it finds that object, asks its hall way
   (interface `0x303`) who holds vertex `+0x8c` (slot 9) and, if that is this
   unit's own id (`MBehaviour` slot 12), sets the vertex free — slot 10 with
   −1 (`0x1003d059`).
2. **Nothing more happens while the mover is not behind** — `+0x250` at least
   `+0x114` returns at once (`0x1003d064`).
3. Otherwise *n* = `+0x114` − `+0x250` records come off the **front** of the
   list. Before they go, the last of them becomes the walker's own place: its
   position into `+0x6c`, and the record's `+0x28`, `+0x2c`, `+0x30` and
   `+0x34` into `+0x78`, `+0x7c`, `+0x88` and `+0x8c` — so the building and
   vertex the walker now counts itself at are the last passed point's.
4. **Every place the dropped stretch had booked is handed back**, record by
   record, by the same three calls as step 1 on each whose `+0x30` is a
   building id (`0x1003d0c5`–`0x1003d14c`).
5. The list then drops its first *n* (`0x1003d161`), and what is left, if
   anything, gives the walker its next point at `+0x90`.

So a unit that walks on does not hold the hall-way places behind it, and a
walker replanned part way along keeps the place it has reached rather than the
one it set out from.

### A tree or a stone cuts the areals it stands on — *read*, and *measured*

**Which objects.** The system areal map's slot 3 (`ArealMap.dll:0x1001f660`)
hears of each static object attached or detached. One of kind 10, a tree or
a stone, goes on to `OnAddStatic` (`0x10022580`) as it is attached
(`0x1001f68e`), and to `OnRemoveStatic` (`0x10022d80`) as it is detached. No
size, flag or margin keeps one out.

**Its footprint** (`0x1000f660`). The object's interface `0x18` answers slot
10 with mode 2. A mesh (`AniMesh.dll:0x10014620`) gives the 8 corners of its
header's box, each through its world matrix, and the areal map keeps the
first 4. *Measured*: on all 68 meshes of `static.rlb` those 4 are the box's
bottom face, (lo, lo), (hi, lo), (hi, hi), (lo, hi), counter-clockwise. So
the footprint is the model's whole box across the ground, turned and scaled
as it is placed: a tree's canopy, not its trunk. Height plays no part. Some
boxes are huge: `s_tree_0_87`'s is 92 × 113 m, `s_stn_0_13`'s 64 × 139 m.

**Which areals** (`0x10022580`). A walkable areal is broken when its box
overlaps the footprint's and its polygon meets it (`0x10018150`: a corner of
either inside the other, or two edges crossing). Each broken areal is built
again (`0x10011900`). It, its walkable neighbours and the areals under
hall-way vertices are then linked again (`0x10023240`). An areal left with
no static is made whole (`0x10007920`).

**Divide** (`MBrokenAreal::Divide`, `0x10010b10`) runs once per static,
starting from the whole areal:

1. The footprint joins the areal's holes. One inside a hole becomes that
   hole; one crossing a hole becomes their union.
2. A sub-areal the footprint crosses is replaced by what is left of it
   outside the footprint, traced as one or more polygons (`0x1000fe00`). An
   edge along the footprint gets no neighbour. A sub-areal the footprint lies
   wholly inside is kept whole, the footprint a hole in it. So **a stone in
   the middle of an areal leaves the links as they were**. A sub-areal inside
   the footprint is dropped.
3. On a crossing, the footprint's contour with each corner moved 1.5 out is
   kept (`0x10013190`). It only classifies points.

The footprint lies in no sub-areal, so it is not walkable.

**Links.** A broken areal gets no links of its own. A sub-areal's edge links
only where it lies on one of the areal's own edges. It links to the walkable
areal across, or, if that areal is broken, to its sub-areal whose edge ends
match within 2.0. The cost is the distance between the centres + 1, and a
sub-areal's centre is its vertices' average. A point on a broken areal
resolves to its sub-areal, else to a hole, else to an inflated contour
(`0x10024110`).

**Waypoints from a sub-areal** (`Behavior.dll:0x100377d6`). Where the line to
the goal crosses the edge, the point is the crossing. Otherwise it is an end
moved 0.2 × the edge's length in. A second point is pushed into the next
node, 0.2 at a time up to 2.

**The local path.** Its generator takes a sub-areal's polygon as the outline
and every hole of the broken areal as an obstacle (`0x10039002`–`0x10039089`).
An unbroken areal gives it none. It logs a start or a finish inside an
obstacle contour (`0x10039365`, `0x10039337`). A walker whose place is in a
hole or an inflated contour logs "Leave Obstacle !!!!!!!!" and fetches that
contour to walk out of (`0x1003e81d`).

***Ballen's Crossing*'s central stone** (*measured*). On C02 M02,
`s_stone_07` stands at (1026, 1031). Its box, 68.6 × 56.3 m, reaches off to
the south-west over ground the areal map leaves walkable. The laser walker's
patrol runs past it.

## Not established

- How the velocity integrator's pull toward *command × top speed*, with the
  command left at 0, combines with a velocity the Wizard writes every frame.
  A stand-in takes the written velocity as the machine's own. ~~Whether the
  Wizard's spin, held to ±1, is a rate or a fraction (a key sends +0.7)~~ —
  **read** for the spin integrator: the spin triple is a fraction, and a step
  turns the hull by spin × the live turn rate × dt (`0x10014b56`,
  [30-turrets.md](30-turrets.md#the-hull-follows-the-turret--read-and-measured)).
  That the Wizard writes the same triple is not traced.
- ~~The areal search (`MGraph`: algorithm, costs, and what the land answers for
  `0x303`)~~ — **read**
  ([The global path](#the-global-path--read-and-measured)), and how a tree or a
  stone cuts the areals it stands on
  ([read](#a-tree-or-a-stone-cuts-the-areals-it-stands-on--read-and-measured)).
  Still open: how the local path goes round its obstacle contours, and whether
  it widens them by the unit's size (a straight leg across an areal that is
  not convex can leave the walkable areals); how a walker in a hole walks out
  of it, and what it does with a goal in one; whether every scenery object
  reaches the areal map's slot 3, and a box for a mesh of several parts;
  whether the search measures a sub-areal from its centre; ~~how the walker drops the points a
  unit has passed (`MWalker::ClearMoverReachedPoint`, `0x1003cfd0`)~~ — **read**
  ([below](#dropping-the-points-a-unit-has-passed--read)); how the walker goes to the point
  it finds off a non-walkable areal, how a unit's place comes to be on a
  building's map object and which vertex the search starts from; ~~who calls
  `MHallWay` slot 11 (`0x1000b390`, which links a building's exits)~~ —
  **read**: `OnAddStatic` and `OnRemoveStatic` alone, on every building
  ([Who relinks a hall way](#who-relinks-a-hall-way--read)); ~~what a
  vertex's size gate reads from the unit (`+0x960`) and its record (`+0x28`),
  what the link flags `0x10000` and `0x20000` mean~~ — **read**, and
  **measured**: the unit's size class against the vertex's own flag word, and
  the link's tail words 1 and 5
  ([The hall-way gates](#the-hall-way-gates-in-the-shipped-buildings--read-and-measured));
  the Wizard's heading curve
  (`0x10003d80`); ~~who reads `Movement_FlyHeight`~~ — **read**: only
  `Behavior.dll:0x100153a0`, which nothing calls
  ([How the AI drives a machine](#how-the-ai-drives-a-machine--read-and-measured)).
- Whether the walker's clear (`0x1003c540`) also empties the points the Wizard
  already holds. `ClearWizardPath` is logged at `0x10040e3b`.
- **What lets a mover past a face flagged `0x20`.** The collision's own two
  filters take a triangle mask of 4 (`0x1001dbad`, `0x1001dbce`), and a round's
  takes `0x24`. The shipped data needs `0x20` passed: `fr_e_brige` flags its
  join cap with it where the other three bridges flag theirs 4, and its halves
  abut like theirs, so as read the energy bridge cannot be crossed
  ([The cap where two halves meet](#the-cap-where-two-halves-meet--measured-and-a-stand-in)).
  The batch word is the obvious other candidate — the query excludes batches
  flagged 8, and the energy batches carry `0x100` and no 8 — so either a third
  filter or ~~a flag set on the loaded batch~~ is doing it. The batch word is
  now read to be the file's own dword, set by nothing at load
  ([The ground inside a building](#the-ground-inside-a-building--read-in-part-and-measured)),
  so it is not the word. The engine passes `0x20` as a stand-in.

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
  *derived*. ~~Still open: what class message
  `0x201` returns through a mover's interface `0x10` (the size class is a
  *guess*; that class is read from a name's third letter, `Behavior.dll:0x1000cfb1`,
  but which name is not traced), the masses property `0x7c` gives a unit and a
  static object, and who sets a collision entry's skip flag 1.~~ — all three
  **read**, and the first two **measured**. Message `0x201` is the size class
  (`MBehaviour` `+0x960`), taken from the **root component's member name** the
  object's source 0 gives (`AniMesh.dll:0x100025e0`) — a unit's third letter
  and a **building's fourth**, which is the kind the node's slot 11 answers,
  4 or 3 (`Behavior.dll:0x1000cee0`); property `0x7c` is control `+0x538`,
  what the machine weighs in kg, and a static object answers nothing; and
  **nothing sets the skip flag**
  ([Collision between objects](#collision-between-objects--read)).
- **What stops a walker at a small sloped object**, such as a buoy. No read
  step does
  ([What a buoy does to a walker](#what-a-buoy-does-to-a-walker--read-measured-and-not-established)).
  ~~Nor is it known who, if anyone, sets a collision object's pair handler
  (`+0x40`).~~ — **read**: the attach does, from interface `0x203`, so it is
  the manager the object registered with
  ([Collision between objects](#collision-between-objects--read)). What that
  manager's slot 7 does with a pair is still open.
- Who sets the machine's counter `+0xd4`, which runs the life update — and so
  the ground damage — on every tick
  ([Water and lava beds kill](#water-and-lava-beds-kill--read-and-measured)).
  ~~Whether a flying machine runs the ground contact at all.~~ — **read**: it
  does. The pass's only gate is that the agent's kind is 4, a unit
  (`0x1000cb90`), which a flyer's is; it runs on message `0x1c`, once a frame,
  and state bit `0x4` is what decides whether the machine falls
  ([Holding the body](#holding-the-body-on-the-ground--read-and-measured)).
  Still open beside it: which objects carry the flag `0x1000000` that holds r₂
  to 7.5, and what the **contact points'** own up pass tests against — the body
  sphere's is r₂, but each contact's compares with a triple the pass builds from
  control `+0x2ec`, `+0x2fc` and `+0x30c` (`0x1001aba7`, `0x1001ae12`), which is
  not read. **What a contact's bound may not be is r**, the agent sphere's
  ([below](#a-contacts-up-pass-must-not-reach-as-far-as-r--measured)); the
  engine takes r₂ until the triple is read.
- ~~Whether `PlaceObjectOnWorldFace`'s reparenting sends a collision object its
  message 21, so that a machine on a building's deck leaves the world's
  collision manager for the building's; and so whether a bridge's own faces
  ever push a hero standing on it~~ — **read**: it does, and the world's pass
  runs the building's, so they do
  ([Walking into a building](#walking-into-a-building--read-and-measured)).
- ~~How the building's mesh builds the capsule a door part is tested against
  (`Terrain.dll:0x1005a27f`).~~ — **read**: `IJointMesh` slot 5
  (`AniMesh.dll:0x1000fd60`) stands the node's level-0 box on its longest world
  axis, from one end face's centre to the other's, as wide as half an end
  face's diagonal; over the 56 shipped door parts it is narrower than the
  node's sphere on 56
  ([Walking into a building](#walking-into-a-building--read-and-measured)).
  Also **read** there: the filing leaves every door and the first computer shut,
  and a building's second computer running. ~~The node whose box bounds a pod's zone in
  height (`0x10058607`)~~ — **read**: the pod node's parent, its level-0 box
  in world space
  ([27-ownership.md](27-ownership.md#the-zones-height-is-the-pod-nodes-parents-box--read-and-measured)).
  ~~How a lightmap combines with a batch's lit colour~~ —
  **read**: [07-objects.md](07-objects.md#how-a-lightmapped-batch-is-drawn--read-and-measured).
- ~~How a walker climbs a building's ramp while the ramp's faces push its
  sphere back; which way the hero takes to the Large Factory's pod~~ —
  **read** and **measured**. The push-out drops triangles flagged 2 unless the
  mover's collision flags carry 8, and every floor on the way carries 2. The way
  runs in by the west side door, down 30° stairs and a 21° ramp
  ([The way to the pod](#the-way-to-the-pod--measured-and-seen)). ~~Still open:
  who sets a collision object's flags (`+0x10`), and which movers carry 8 and 4.
  That the hero lacks 8 is *derived* from the recording.~~ — **read**, and
  **measured**: the control, through the collision object's slot 5
  (`Control.dll:0x1001f670`), from its state's word — 8 unless the word
  carries 4, 4 from `0x4000000`. None of the 206 controllers mixes; every
  walker lacks 8 and every flyer carries it, and 4 is on 31 rounds alone
  ([The ground inside a building](#the-ground-inside-a-building--read-in-part-and-measured)).
  ~~Whether the slope brake reads a building's stair faces~~ — **read**: it
  does, since its normal is whatever faces the lift stood on; how the
  recordings' heroes climb at a walk anyway is not read (same section).
- ~~Where a gathered face's batch word comes from. It is the first word of what
  the face source's slot 3 returns (`AniMesh.dll:0x1000d71f`), and the 8 and
  `0x200` the collision query excludes are not written by `AniMesh.dll`. Also
  open: whether the `DEFAULT` and `PORTAL_*` quads carry that word~~ — **read**,
  and **measured**: the batch record's own first dword (`0x100134d0`); 8 is on
  633 of the 15153 batches, exactly the doorway and portal quads, and `0x200`
  on none
  ([The ground inside a building](#the-ground-inside-a-building--read-in-part-and-measured)).
  Still open: what the filter's first word `0x41e` selects, where the
  push-out uses its third argument (the step-1 filter), and what else reads
  triangle flag 2.
- ~~What joins the Large Factory's three hall-way groups; the front group never
  reaches the pod.~~ — **read**: the areal map, each group's exits linked to
  the walkable areal under them at a cost of 1, and the front group still never
  reaches the pod, with all 85 links counted. For a walker only the west exit
  67 does: the east and upper west exits join the interior across four links
  only a flyer crosses
  ([A building is drawn cell by cell](#a-building-is-drawn-cell-by-cell-through-its-portals--read),
  where the hall way's groups are measured).
- ~~Who calls `CBuilding::PortalDrawNotify` with a portal's face.~~ — **read**:
  `CShade`'s portal fade (`Terrain.dll:0x1002c4d0`, from its mesh draw at
  `0x10045d30`), through the building's own `IMesh2` slot 4, as each portal
  batch is drawn: always for a sign (`0x10`) or an open portal (`0x40`), and for
  a `DEFAULT` doorway only within 1.3 × `PortalFarDist` ÷ the field of view of
  the camera, 95 at the game's 1.3 rad, beyond which the doorway is drawn black
  and its room not at all. ~~Which node a portal names~~ — **measured**: its
  batch record's `+6` halfword, the room beyond, on all 625 level-0 portal
  batches
  ([A building is drawn cell by cell](#a-building-is-drawn-cell-by-cell-through-its-portals--read)).
- What the camera's field of view is in play, which scales a portal's fade: the
  outer camera's 1.3 is a *guess* ([30-turrets.md](30-turrets.md)), and the
  buffering camera starts at 1.7 ([03-terrain.md](03-terrain.md#the-reflection-camera--read)).
- The 57 batches that wear `DEFAULT` with no portal bit — three trees, four
  internal systems, three turrets and some buildings' lower levels of detail —
  which nothing in the draw skips, so the game draws them black; openparkan
  leaves them out.
- ~~Which `Land.msh` faces carry the world face bit `0x8` and class bit 8 that
  the ground search excludes; the landscape converts them to its own mask at
  `Terrain.dll:0x10022da0` (world `0x8` → `0x20`, `0x200` → `0x20000`,
  `0x400` → `0x2000`)~~ — **read**, and **measured**: **none do**. The
  landscape's mask is the file's face record read as one dword, flags low and
  surface high, so the two are the flags word's `0x20` and the surface word's
  `0x04`, and 0 of the 275882 faces across the 33 maps carry either — against
  3630 and 6102 for the same filter's other two bits
  ([What the filter excludes](#finding-the-ground--read)). Still open: what
  would ever set them. Every masked access to the face array in `Terrain.dll`
  is a read, and the only readers beside the query filters are the draw-order
  rebuilders, one of which gives such a face a batch of its own.
- ~~The contact records' flag 2, which hands the contact to the object's
  interface slot `0x7c` (`0x1001affd`)~~ — **read**, and **measured**: it lays
  the node the point carries along the ground under it, through `IAnimation`
  slot 31 and node mask `0x10`, and the twelve contacts that ask for it are the
  tracked chassis's belts
  ([28-chassis.md](28-chassis.md#the-belt-lies-along-the-ground--read-and-measured)).
  ~~Still open: flag `0x20`~~ — **read**, and **measured**: it asks for `0x2`
  to be worked out from the state's own last pose, and it is on 2410 of the
  2634 contacts — every walker's feet — of which 2217 stand up and are placed
  ([A walker's feet lie flat where the animation lays them](#a-walkers-feet-lie-flat-where-the-animation-lays-them--read-and-measured)).
  Flag `0x1000` runs the
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
- ~~How often `World3D.dll`'s input update (`0x1000f100`, the manager's slot 4)
  runs, which sets how fast the keypad cruise ramps.~~ — **read**: once a game
  frame, which the mission loop runs uncapped, so the ramp is that much faster
  on a faster machine
  ([From input to motion](#from-input-to-motion--read-and-measured),
  [14-controls.md](14-controls.md#a-row-that-stays-down--read)).
- ~~Which node range the payload sum counts as the chassis~~ — answered: part
  0's nodes, the root object's ([Load](#load--read-and-measured)).
- ~~Triples 5 and 6~~ — answered: 6 is the most the hull leans
  ([13-control.md](13-control.md#the-lean-and-triple-6--read-and-measured))
  and 5 how fast it rights itself
  ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)).
  ~~Still open there: who writes the vector at control `+0x348` that bits
  `0x10` and `0x20` right the hull toward.~~ — **read**: it is the motion
  body's `+0x194`, the averaged ground normal the lift writes (`0x10015e47`),
  since the body sits at control `+0x1b4`; and the turn's own sense is read
  too, a positive pitch tipping the nose down, roll the top left and yaw the
  nose left
  ([The hull leans](#the-hull-leans-and-rights-itself--read-and-measured)).
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
- ~~What behaviour flag `0x800` (`+0xa04`) changes besides clearing the
  walker.~~ — **read**, and **measured**: it is read in two more places, both
  of which refuse the unit unless it is a building — `MBehaviour::AddOrder`
  and the self-given task factory — so it is the unit's leave to be ordered at
  all, and the three chassis that can never carry it are the Small Tower and
  the two shooting-range targets
  ([Flag `0x800`](#flag-0x800-is-the-units-leave-to-be-ordered--read-and-measured)).
  Triple 2 is never read inside `Control.dll` — nothing reaches +32..+40 in
  either copy of the block ([13-control.md](13-control.md)) — and the AI reads
  its forward component as a floor.
