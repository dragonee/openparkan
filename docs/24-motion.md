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
| +0x04 | flags; bit 0 lets a state take part in selection (`0x1000524c`) |
| +0x14, +0x18, +0x1c | by their values a first frame, a last frame and a rate — *guess* |
| +0x24 / +0x30 | velocity box, min xyz / max xyz |
| +0x3c / +0x48 | spin box, min xyz / max xyz |
| **+0x54** | **the engine factor** |
| +0x94 | a use count, −1 for unlimited (`0x1000530f`) |
| 16 × B | conditions: bit 0x100 needs a byte of the i-th 0x5c-byte entry at `+0xc4` set, 0x200 needs it clear (`0x10001107`); what the entries are is not read |

A state applies while the machine's velocity and spin lie inside the boxes its
flags switch on (`0x10001000`); the engine moves to the cheapest transition in
the table whose cost is below 1,000,000 and whose target applies (`0x100051c0`).

*Measured:* 1,690 states in 207 controllers. The 4,323 bounds a flag bit
switches on all read min ≤ max; the 5,817 it leaves off are all exactly
−FLT_MAX..FLT_MAX. On `r_l_01` (Small Walking Chs) the boxes read like gaits:
standing within ±0.6 m/s, walking forward 0.6–12, running 12–25 and backward
the same negated, turning in place with yaw between ±0.1 and ±2.28 rad/s.

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

Each tick (`0x10005370`, dt = elapsed ms × 0.001, held to 0.01–5 s) the
velocity integrator (`0x100153c0`) moves each axis of the velocity toward
*command × live top speed* by at most *live acceleration × dt*, stops at the
target, and clamps the result to the live top speed (`0x10016810`). The
command is the body's `+0x08` triple — what writes it (`0x10014610`, from the
state and the input) is not read.

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
- **Extra engines cannot raise top speed past the authored value**, but they
  make up for load, damage and ground — and they do raise the turn rate.
- **Load halves top speed at most**: a machine carrying its full payload runs
  at half speed on one healthy engine.

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

- each node: `.ndp` float +8 × the mesh node's volume (id 0x10, which
  `AniMesh.dll:0x100051f0` scales by all three of the object's scale factors,
  where id 0xf takes two — [25-sensors.md](25-sensors.md#what-a-target-gives-away--read));
  that the `.ndp` float is a density is a *guess*
- each component: its **mass at record +0x1c**, in kg, added to its node
  (ids 0x100 and 0x200, `0x1002bb40`)
- **spare payload** = file +124 + the mass of one node range the mesh reports
  (the chassis's own, by the look of it — *guess*) − the total, never below 0

The stat panel shows **file +124 × 0.001 as "Max payload" in t** and the total
mass × 0.001 as "Weight" (`iron3d.dll:0x1006f300`, IControl 136 and 124).

*Measured:* every internal part but armour carries a mass — 100 to 40,000 kg
over the 104 in `intsys.rlb`, against 0 on all 24 armour parts — and so does
every `o_cNN` gun, 6.25 to 6,000. The slots a gun mount, a chassis or a
building declares for its parts carry none, bar one detection-shield slot of 10. An internal engine's mass
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

- **The ground scales top speed.** On contact `0x1001a450` asks the object
  underneath for a surface record: +8 becomes `+0x1a8`, the factor G; +0xc
  becomes `+0x1a4`, a rate at which kind-4 agents lose hit points while on it
  (`0x10012a66`); +0 is a surface id 0–10 that picks one of eleven entries at
  `+0x504` for `0x10002800`. Which surfaces carry which values is not read.
- **Mode 2 brakes on slopes.** Only when file +104 is 2 does the velocity
  integrator compare the ground's tilt with the cone at +112 (`0x100157ac`):
  with `c` the cosine of the tilt, a factor
  `min(1, 2 (c − cos cone) ÷ (1 − cos cone))`, 0 past the cone, pulls the
  velocity toward that fraction of itself at 1.5 × the acceleration. It
  applies moving one way across the slope only — uphill, by the look of it
  (*guess*).

*Measured:* all 509 mode-0 controllers keep the default cone of 1.57079; all
16 mode-2 controllers carry 0.6 rad (34°); the six mode-3 controllers carry
0.6 or 1.52 and take a different branch (a height, `0x10015879`, unread).
Among the chassis, mode 2 is every wheeled and tracked chassis, the small and
medium walkers, the Transformer, the Small Tower and the hero; the Large
Walking Chs, the Tiny Spider and every flyer are mode 0 and ignore slope.

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

Every chassis above but the hero carries one engine of power 20, and all 22 a
10,000 battery giving 250 a second (*measured*). At top speed in a factor-1
state the engine draws 20 a second: **1,200 a minute, 12% of that battery**
(*derived*). Half speed is half that; a factor-2 state doubles it; each
internal engine adds its own power (0.75–11.2) on the same formula. The hero
chassis's engine asks 0.1 and its factor is always 0: it walks for free.

Six chassis carry a **second store** (*measured*): the Transformer, the L-7f,
L-8f, S-6f and S-7f hold a further 1,000,000 at 1,000 a second — for them a
minute at full speed is nothing — and the Small Tower a generator.

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

- Which ground surfaces carry which speed factor and damage rate — the record
  behind `0x1001a450`'s query — and whether water is one of them.
- What writes the command triple the velocity integrator multiplies by top
  speed (`0x10014610`).
- Which node range the payload sum counts as the chassis.
- Triples 5 (+68) and 6 (+80): 6 clamps an attitude the spin integrator
  drives from a per-state selector (state +0x08), 5 is multiplied into it;
  what that attitude is on screen is not read.
- The mode-3 height branch (`0x10015879`).
- Where `Speed_MaximumFactor` is applied, and what the unit's `+0x5fc` speed
  base is.
- Whether a module outside `Control.dll` stops a machine whose engines are
  unpowered.
