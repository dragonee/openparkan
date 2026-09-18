# Effects: `effects.rlb` and the `.exp` explosions

Two small formats that together say what happens when something is destroyed.
They are the animated half of the renderer, and nothing in a static scene
draws them — what they give you is the whole graph, from a mesh node's damage
record through to the sprites and sounds it sets off.

Everything below is re-derived by `uv run openparkan verify`.

## `.exp` — what an explosion sets off

144 members across five archives — 119 in `weapon.rlb`, then `animals.rlb`
(12), `system.rlb` (8), `static.rlb` (4) and `turrets.rlb` (1). An `.exp` is
**what a hit does**, and every one is 792 bytes:

```
int32    kind        1 nothing, 2 a direct hit, 3 an area blast (Control.dll:0x1000ebc0)
float32  damage      0 to 800,000
float32  radius      2 on every _l, 3 on every _m, 4 on every _b
float32  1.0
float32  1.0
int32    placement   7 on 63 records -- at the point of impact; 0 on 81
12 x     char[32] archive, char[32] member   slot 0 the effect; 1-11 by surface struck
```

The names fill 1 slot or all 12 (and none on two). On the 50 full records,
slots 1–11 carry the surface in one order — `sn st gr sw ic mt gr wt al an
sh`, 550 of 550, the flame family's catch-all `mn` standing in for 32 — and
690 of the 692 names are real `FXID` members. **Slot *s* + 1 is the effect for
ground surface *s***, the class byte of the material struck: `wt` is water
(class 7), `mt` the machine and building skins (5), `gr` the grass classes 2
and 6, `sh` the shields (10). See [What an explosion plays](#what-an-explosion-plays--read-and-measured).

**This page read the first word as a count until the damage code was read**
([26-damage.md](26-damage.md)). It matches the filled slots on only 26 of the
144, and the "stale names past the count" it explained are the surface slots.
The **radius** tracks the size suffix in the record's own name across every
family: `explode_frt_l` 2, `explode_frt_m` 3, `explode_frt_b` 4, and the same
for `explode_rbr_*` — a multiple of the exploding node's bounding radius.

## `FXID` — one effect

923 members of `effects.rlb`, and every one is a **60-byte header followed by
that many typed emitter blocks**. A block opens with a `uint32` whose low byte
is its type; the type fixes the block's length and where inside it the
`(archive, member)` pair sits.

| type | length | resource at | names | blocks |
|---:|---:|---:|---|---:|
| 1 | 224 | — | nothing; parameters only | 618 |
| 2 | 148 | 84 | a sound in `sounds.lib` | 517 |
| 3 | 200 | 136 | a material | 1545 |
| 4 | 204 | 136 | a material | 202 |
| 5 | 112 | 48 | a material | 31 |
| 6 | 4 | — | — | 0 |
| 7 | 208 | 144 | a material | 1161 |
| 8 | 248 | 184 | a material | 237 |
| 9 | 208 | 136 | a material | 266 |
| 10 | 208 | 144 | a material | 160 |

That table walks **all 923 effects to the byte**, 4737 emitters in total.

It was arrived at twice. First it was fitted from the data: 338 records have a
resource pair in every block, which pins those lengths directly, and the rest
fell to a search for the table that walks the most records — 922 of 923 at the
first pass, then all 923 once type 4 was corrected from 200 to 204. The
independent confirmation was that on every record that walks, **every block
ends where a `(archive, member)` pair begins**, which a wrong stride breaks
immediately.

Then it was read off the engine, which agrees exactly. `Effect.dll`'s emitter
factory does:

```
mov edx, [edi]      ; the block's first dword
and edx, 0xff       ; the low byte is the type
lea eax, [edx - 1]  ; types are 1-based
cmp eax, 9
ja  <skip>
jmp [eax*4 + <table>]
```

and each of the ten branches allocates its class, stores the block pointer,
and advances `edi` by exactly the stride above — `add edi, 0xe0` for type 1,
`0x94` for type 2, and so on. Two things follow that the data could not give:
**type 6 is a real type with a 4-byte block**, which no shipped effect uses,
and **bit 8 is a flag rather than part of the type** — right after the switch
the factory computes `(word >> 8) & 1` and stores it as a byte on the emitter.
It is set on 1811 of the 4737 emitters, and it draws the emitter over the scene
while the effect's point is in view — see [bit 8 and the tested
point](#bit-8-and-the-tested-point--read-and-measured).

Types 7 and 10 allocate the same 0x48-byte object and install different
vtables, so they are sibling classes rather than unrelated ones.

## Inside a block

The sound emitter was the first one read off the data. Type 2 keeps a **near and far audible
distance** as two `float32` at +64 and +68: the first is never greater than
the second on any of the 517 blocks, and the pairs are exactly what a 3D sound
wants — (3, 40) on 160 of them, (10, 100) on 120, then (3, 50), (10, 80),
(15, 300). `aim_exp_L`'s explosion is audible from 20 to 200 units. Two more
`float32` at +8 and +12 sit in 0..1, most often (0.0, 0.1), and +72 is 1.0 on
516 of the 517.

## Which floats are live

The other nine types keep only a pointer to their block — at `this+0x18` for
most of them, `+0x1c` for type 1, `+0x24` for the sound — so the fields that
matter are whatever each class's own virtual methods load through that
pointer. That set can be recovered: walk the class's vtable, taint the
register the block pointer lands in, and record every `fld` off it.

**181 of the 441 four-byte slots across the ten block types are loaded as a
float straight off the block pointer.** That is a **lower bound** on what is
live, not the whole of it. Reading the classes' code for this page found two
kinds of load the walk cannot see: a field **copied as a dword** before it is
used (type 1's start colour and position, the start of a particle's fade, a
bolt's sprite count), and a field read **through a pointer into the block** —
the per-axis exponent triples of types 3, 4 and 9 at +64..+72 and +124..+132
(`add esi, 0x40` and `add ebx, 0x7c` at `0x100106f6`, `0x10010784`), and type
8's at +124..+132 and +172..+180 (`0x10012185`). Of the 122 slots outside the
map, 103 hold a sane float on every block, though many of those are simply
zero throughout; see [the emitter types](#emitter-types--read-and-measured)
for what is named:

| type | block | offsets it loads |
|---:|---:|---|
| 1 | 224 | 8, 12 · 28, 32, 36 · 52, 56, 60 · 80, 84, 88, 92 · 112, 116, 120 |
| 2 | 148 | 8, 12 · 28, 32, 36 · 52, 56, 60 · **64, 68** · 72, 76 |
| 3 | 200 | 8, 12 · 24, 28, 32, 36 · 40–60 · 100–120 |
| 4 | 204 | the same eighteen as 3 |
| 5 | 112 | 4, 8, 12, 16 · 24–44 |
| 6 | 4 | — |
| 7 | 208 | 12–32 · 44–140 |
| 8 | 248 | 8–32 · 52, 56, 60 · 88–120 · 136–168 |
| 9 | 208 | the same eighteen as 3 |
| 10 | 208 | the same thirty-one as 7 |

Two things check the map. **The sound emitter is the control**: its `+64` and
`+68` were read off the data long before this existed, and the map contains
them — along with the `+8`, `+12` and `+72` the data had also flagged. And
**not one of the 181 offsets lands inside a block's `(archive, member)`
pair**, though the read map comes from the code and `RESOURCE_AT` came from
the data; an error on either side would collide somewhere.

The families fall out of it. **Types 3 and 9 read the same eighteen
offsets** — type 9's constructor installs type 3's vtable and then overrides
it, so it inherits the layout. **Types 7 and 10 read the same thirty-one**,
which is the sibling relationship the factory already suggested. Type 4 reads
the same as type 3 as well: its `+32` and `+36` are read through a second
pointer to the block, at `+0xfc` (`0x100108f9`), which an earlier walk did not
follow. Type 1 also reads `+8` and `+12`, its window, and `+120`, its range
jitter (`0x1000fa9a`), in an update the first sweep did not decode
(`0x1000f6e0`); the map missed `+120` until the light was read.

One field was identified by its shape before the code named it: **types 1 and
2 keep a unit vector at `+52`** — unit length on 597 of 618 and 517 of 517
blocks, and exactly `(1, 0, 0)` on 542 and 517 of them. A direction with a +X
default; on the light it is the end of the light's direction (below). The
drawing types keep something else there: on type 8 not one block of 237 is
unit length.

Type 3's `+40..+48` and `+52..+60` are a component-wise (low, high) pair of
`float32[3]` on 1427 of 1545 blocks, which is the shape a particle spread
takes — and the engine reads exactly those six, which is the data and the code
agreeing without either being derived from the other. Types 7 and 10 read a
`float32[3]` at `+56`, `+80`, `+104` and `+128`, in their own methods rather
than through a helper: those are the second and fourth triple of each of their
two channels, 12 bytes apart throughout
([below](#a-channel-is-a-low-high-jitter-exponent-run--read-and-measured)).

**None of that names a field.** It says which of the 30 to 60 floats in a
block are live, which classes share a layout, and which two hold a direction.
The types' own code names the light, the bolt, the stream's rate and the
fades — see [the emitter types](#emitter-types--read-and-measured); the runs of
four triples that place and size what a type draws are
[below](#a-channel-is-a-low-high-jitter-exponent-run--read-and-measured), and
most of the rest is still open.

One negative result worth keeping: **an explosion's size is not in its
effect.** `exp_frt_l`, `exp_frt_m` and `exp_frt_b` share their emitter blocks
byte for byte; the 2, 3 and 4 that separate them are the radius in their
`.exp`, which scales the whole thing at run time.

**Every one of the 3577 material references resolves** through `Material.lib`,
and 516 of the 517 sounds are in `sounds.lib` — the exception is
`fortif_door_move.wav`, which was never shipped.

`aim_exp_L`, the large explosion an animal makes, is a good example of the
shape:

```
 0  type  7 flagged  material.lib/fire_smoke
 1  type  7 flagged  material.lib/fire_add
 2  type  4          material.lib/spittle_g_add
 3  type  4          material.lib/glow_eng
 4  type  7          material.lib/smoke_g_add
 5  type  2          sounds.lib/Exp_anl_06.wav
 6  type  1          -
```

Five sprite layers, a sound, and one parameter block.

## A third witness to the read map, and what shape the floats have

The read map came out of `Effect.dll`'s vtables and knows nothing about the
data, which makes the data an independent test of it — and the sharpest form
of the test is simply whether a slot the engine loads as a float *holds* one.

**All 99605 reads of the 181 live slots are a real float**: finite, and either
exactly zero or between 1e-6 and 1e6 in magnitude. The 122 dead slots manage
**92.2%** — 5395 of their reads are NaN, denormal or absurd, which is what an
editor's uncleared buffer looks like. Nothing in the recovery used the values.

Sorting the live slots by the shape of their values across the library gives:

| shape | slots |
|---|---:|
| signed | 37 |
| positive | 60 |
| 0..1 | 36 |
| integral | 33 |
| always zero | 15 |

The 15 that are always zero are read by the engine and never set by the
editor — a parameter the artists left alone throughout.

### The motif is a component-wise (low, high) triple

Type 3's `+40..+48` against `+52..+60` was the first one found, and it is not
special. Searching every type for a pair of disjoint triples that is ordered
on nearly every block **and identical on a good share of them** — an emitter
that does not randomise a quantity writes the same value twice — turns up the
same shape elsewhere. The two strongest are in type 10: `+80..+88` against
`+128..+136`, ordered on 154 of 160 blocks and *identical* on 154, and
`+92..+100` against `+104..+112`, ordered on all 160 and identical on 158.
Type 4's `+100..+108` against `+112..+120` is ordered on all 202.

Where several triples happen to be ordered the partner is ambiguous, so this
is a motif rather than a field list. It says what the block *is*: a parameter
sheet of randomised ranges.

### Two handles followed, one of them dead

`Effect.dll` imports three things that would name a value if a float reached
them — `ngiGetClocks`, `ngiGetSinCos` and `g_FastProc`'s matrix routines.

**The clock is a dead end.** All 13 of its call sites are the same three
instructions — `call ngiGetClocks; mov [state], eax; ret` — seeding a
pseudo-random generator. Nothing in the DLL compares a block float against
elapsed time, so a lifetime cannot be found that way.

**`ngiGetSinCos` is called from exactly one place**, at `0x1000c293`, and that
routine is the emitter's **random direction**: it draws from the generator,
scales the integer by 1/65536, offsets it, feeds the result to sin/cos, scales
a vector by three separate factors and hands it to `g_FastProc`'s transform
with a matrix at the object's `+0xc0`. So the engine's emitters do randomise a
direction within a spread, and the values that shape it pass through here —
but the routine takes them as arguments rather than reading the block itself,
so the offsets are one call further out.

## The chain, end to end

A mesh node carries a durability and an explosion in its
[`.ndp`](07-objects.md); the explosion names effects; an effect names
materials; a material names a texture. All of it holds:

**2189 of the 2203** `.ndp` explosion references walk the whole way to an
effect whose every material resolves. The 14 that do not are the references
to `exp_t_sn_mis` and its neighbours.

## How an effect runs — *read*

**Who owns effects.** `CreateFxManager` (`0x100140c0`) builds a manager; every
agent makes one for itself (`AniMesh.dll:0x100033ba`) and so does the landscape
(`Terrain.dll:0x100172fa`). A controller reaches its agent's manager at `+0x3c`.
The manager loads an `FXID` once by `(archive, member)` and hands out a
**template** (slot `0x10`, `0x10004320`); an **instance** of a template is made
on an object and an attach point under a 32-bit **key** (slot `0x14`,
`0x10004590`). A controller's key is its part's high word × 0x10000 plus the
record's id.

**The 60-byte header** (`0x10007650`, `0x10005c60`, `0x10008120`):

| off | type | field | shipped |
|---|---|---|---|
| +0 | u32 | emitter count | 0 to 81 |
| +4 | u32 | **time mode**, below | 0 on 131, 1 on 540, 2 on 115, 4 on 47, 5 on 46, 14–17 on 44 (*measured*) |
| +8 | f32 | **duration**, s: end = start + 1000 × it, in ms | 0 to 5 (0.75 on the hero's guns, 1.5 on impacts, 3 on the dummies' explosions) |
| +0xc | f32 | spread of a random jitter on *t* (flag 1) | |
| +0x10 | u32 | **flags**, below | explosions 0x16, rounds 0x10 |
| +0x14 | u32 | **the settings switch** the effect belongs to; off, the instance neither updates nor draws ([below](#which-effects-run-the-settings-switch--read-and-measured)) | 18 of the 20 ids; `0x312` "Gun fire" on the hero's guns |
| +0x18 | f32×3 | random offset amplitudes (flag 8) | |
| +0x24 | f32×3 | **the tested point**, in the instance's frame: the camera's view of it decides bit 8 and flag 0x400 ([below](#bit-8-and-the-tested-point--read-and-measured)) | (0, 0, 0) on 462, (1, 0, 0) on 368, (0.5, 0, 0) on 93 |
| +0x30 | f32×3 | **scale**; the size a controller or `.exp` asks for is multiplied by it | 0.1 on the hero's muzzle effects |

**The flags** (`0x10006170`, `0x10007650`, `0x10007d10`, `0x10008120`; counts
*measured* over the 923):

| flag | effects | what it does |
|---:|---:|---|
| 0x1 | 58 | jitter *t* by +0xc |
| 0x2 | 403 | **delete once *t* ≥ 1** (`0x100062a6`) |
| 0x4 | 409 | **let go of the attach point**: after its first tick the instance bakes its world frame into its own and forgets the point (`0x10006324`), so an explosion stays where it went off; 402 of the 409 are time mode 1 |
| 0x8 | 57 | a random offset (+0x18) |
| 0x10 | 466 | keep running when the attach point is hidden |
| 0x20 | 50 | ping-pong |
| 0x40 | 8 | start switched off |
| 0x80 / 0x100 | 110 on 0x100 | hold *t* at 0 while the manager's bit 0 (messages 23, 24) is clear / set |
| 0x200 | — | × linear progress |
| 0x400 | 114 | **draw nothing while the tested point is hidden** (`0x10008016`): the building and robot beacon lights, `f_*light*`, `rb_*light*` |
| 0x800 | 198 | **drawn only by a draw call that passes its pass argument** (`0x10007d44`; the manager skips such calls outright when all its instances are 0x800, `0x10004061`); which caller passes it is not established. Lights, sounds, breath and beacons |
| 0x1000 | 14 | **hand the emitters the manager's target point** every manager tick (`0x10006349`, manager slot `0x44`); a type-5 bolt takes it for its start (`0x10003070`). Exactly the 14 effects with a bolt, all bolt-only |
| 0x2000 | 1 | passed to the renderer as effect draw flag 4 (`0x1001088c`), which reaches its texture choice by distance (`Terrain.dll:0x10028417`); `env_lightning` alone |
| 0x8000 | 5 | skip the attach point's second test |
| 0x10000 | 1 | *unknown* — no test found; `aim_tail_S` |

**Effect time *t*** runs 0 to 1 (`0x10005c60`, 18 modes):

| mode | *t* |
|---|---|
| 0 | a value set from outside (slot `0x1c`), **0 until set** |
| 1 | (now − start) / (end − start): once through |
| 2 | its fractional part: looping |
| 3 | 1 − mode 1 |
| 4 | **the animation value of an owner's mesh node** (action 14), through the owner's interface `0xb`, slot 9 — [below](#time-mode-4-is-a-nodes-animation-value--read-and-measured) |
| 5 | the owner's speed ÷ its top speed (properties `0x21` and `0x11`); 6–8 per axis |
| 9–12 | the same for spin (property `0x24`) |
| 13, 14 | 1 − an owner value (the attach point's; property `0x31`) |
| 15 | the larger of modes 5 and 9 |
| 16, 17 | mode 4 that only rises (or drops to 0) / only falls (or jumps to 1) |

then flag 0x200 multiplies it by mode 1, and flag 0x20 folds it
(*t* < 0.5 ? 2*t* : 2(1 − *t*)). With flag 1, *t* moves by a random amount up to
±half of +0xc before it is clamped to 0..1 (`0x10008317`).

**Every emitter has a window** of *t*, a `(low, high)` pair — the first thing
each class's update compares — and does nothing outside it (table below). So an
effect is a **timeline**: `hero_cannon` flashes two sprites over 0.01–0.5 and
two more over 0.51–1.

**Starting, ending, switching** — the manager's slots, and the controller
action that calls each ([13-control.md](13-control.md#the-section-5-record--read-and-measured)):

- **start** (slot `0x2c`, action 10, `0x10007b00`): start = now, end = now +
  duration, time mode = the record's v5.
- **restart** (slot `0x30`, action 11, `0x10007b40`): the next update after
  *end* starts it again in the header's own mode (`0x10008134`).
- **on / off** (slots `0x3c` / `0x40`, actions 18 / 19, `0x10007a30`): off runs
  every emitter once with *t* = −1.
- **delete** (slot `0x18`, action 8).
- an instance with flag 2 **deletes itself** once *t* ≥ 1 (`0x100062a6`): every
  impact and explosion carries it.

**The manager ticks** on message 28 with the time in milliseconds
(`0x10003d54`): an instance updates when 100 ms have passed since its last
update, or within 50 ms of a command to it. The emitters' own clock is
**seconds since the instance started** (`0x1000846c`), which is what a phase
"in seconds when negative" and a stream's interval count.

### Time mode 4 is a node's animation value — *read*, and *measured*

Action 14's v5 (`Control.dll:0x10003031`) is **a mesh node**, not a control
point. The controller adds the index of its part's first node, less one
(`AniMesh.dll` slot 14, `0x10005780`), and the manager keeps it for the
instance (slot `0x34`). Time mode 4 then asks the owner's interface `0xb` — the
`AniMesh` object itself — for slot 9 (`0x10005600`), which returns **that
node's animation value**, record `+0x114`. The value is what a channel writes
when it animates the node: the channel update (`Control.dll:0x10021c97`, and
`0x10009a7b` at load) hands `AniMesh` slot 10 (`0x10005630`) the channel's
value, wrapped or clamped and inverted by its flags, and the node plays frame
`+0x100` lerped toward `+0x104` by it.

*Measured* on the hero's turret, where all seven action-14 nodes are animated
by a channel:

| effect | node | animated by |
|---|---|---|
| `hero_cannon` | 13 `Gun02_m1o1` | channel 3, the cannon's barrel (gun component) |
| `hero_prifle` | 9 `Plz02_m1o1` | channel 8, the plasma rifle's barrel |
| `hero_redlaser` | 22 `Lz02_m1o1` | channel 22, the laser's barrel |
| `hero_cannon_sfx` | 11 `RVin_m1o1` | channel 13, a class-24 arm |
| `hero_prifle_sfx` | 7 `LVin_m1o1` | channel 16, an arm |
| `hero_redlaser_sfx` | 19 `Lz00_m1o1` | channel 19, an arm |
| `hero_missile_sfx` | 15 `Rk00_m1o1` | channel 23, an arm |

So a gun's effect **runs with its barrel's stroke**: the stroke takes the
channel 0 → 0.5 → 1 and snaps it back to 0
([29-weapons.md](29-weapons.md#the-guns-takt-a-stroke-then-the-interval)), and
`hero_cannon`'s flash, light and `H_fire_cannon.wav` (at 0.01) play across it
with no start command. The `_sfx` effects are **not shot sounds**: they follow
the arms, and `H_gh_cannon.wav` plays as the cannon's arm passes 0.15. Read as
channel numbers instead, the seven name a channel on that node twice.

## Emitter types — *read*, and *measured*

| type | class | window (block offsets) | what the code shows | what it is |
|---:|---|---|---|---|
| 1 | 0xf0, vtable `0x1001e78c` | +8..+12 | creates a light in the owner's light manager, interface `0xe` (`0x1000f4b0`), switches it on inside the window and off outside, and hands it a position, direction, colour, range and attenuation every update (`0x1000f6e0`) | a **light** — [below](#type-1-is-a-light--read-and-measured) |
| 2 | 0xa0, `0x1001f048` | trigger +8, window +8..+12 | +4 of 0: plays as *t* crosses +8 going up; +4 of 2 or 3: plays while *t* is inside the window (`0x10012eb0`) | the **sound**; near/far at +64/+68 — [below](#type-2-is-a-sound--read-and-measured) |
| 3 | 0xfc, `0x1001e770` | +32..+36 | a phase *a* + (*b* − *a*)·*x*^*g* from +8/+12/+16, *x* the progress through the window or seconds when +8 < 0 (`0x100105f0`); its position +40 → +52 shaped by the powers at +64 and its size +100 → +112 by those at +124; a 0..1 value +20..+24 to the power +28 and the phase's fractional part go to the draw (`0x100106c0`) | a **sprite** that moves (+40/+52) and grows (+100/+112) over its window — muzzle flashes, glows, bullets |
| 4 | 0x104, `0x1001e754` | +32..+36 | type 3's phase (`0x100108f0`) | a sprite variant |
| 5 | 0x54, `0x1001e360` | +12..+16 | a phase from +40, seconds when negative (`0x10002a20`); a start point, and sprites along the line from it to where the effect is now, each lerp(+24, +28) wide by the progress (`0x10002be0`) | a **bolt**: laser and shock tails, `hero_laser_bullet` — [below](#bolts-streams-and-fades--read-and-measured) |
| 6 | 0x1c, `0x1001e738` | — | — | never shipped |
| 7 | 0x48, `0x1001e228` | +20..+24 | +0x24 × +0x28 particles, each running +44 → +56 in place and +92 → +104 in size over its life, both jittered at the high end and shaped by the powers at +80 and +128 (`0x10001720`, `0x10001300`); its age = (phase − its spawn) / +0x1c, one-shot flag, drag, and a fade value from +8/+12/+16 | a **particle burst** — smoke, fire, splashes |
| 8 | 0xac, `0x1001e71c` | +16..+20 | windowed on *t* unless +0x8c; one particle every +24..+28 seconds into a ring of +36 (`0x100115c0`), each running +88 → +100 in place and +136 → +148 in size, shaped by the powers at +124 and +172 | a **particle stream** — dust, missile smoke |
| 9 | 0x100, `0x1001e700` | +32..+36 | type 3 with its own draw | sprite |
| 10 | 0x48, `0x1001e24c` | +20..+24 | type 7 with its own start | particles (`NE_Gibs_Stn` debris) |

*Measured*: the window is an ordered span inside 0..1 on **4736 of 4737**
emitters; read four bytes early or late, 705.

A type-7 block with +4 = 1 draws its particles in sprite mode 3, any other
value in mode 0 (`0x100019d0`); that mode 0 faces the camera is a *guess*.

### A channel is a (low, high, jitter, exponent) run — *read*, and *measured*

Every drawing emitter builds the same **particle**: a 0xcc-byte class whose vtable is
`Effect.dll:0x1001eb08` and whose constructor is `0x100092d0`. A type-3, 4 or 9 sprite
embeds one at its `+0x30` (`0x10009246`); types 7, 10 and 8 allocate an array of them; a
bolt's sprites are a 0x14c-byte derivation with the same slots (`0x1001eb2c`). The class
keeps **three lerped `float32[3]` channels** — a base, a delta and the value they make —
at `+0x60`/`+0x6c`/`+0x78`, `+0x84`/`+0x90`/`+0x9c` and `+0xa8`/`+0xb4`/`+0xc0`. Each has
a setter that takes one lerp parameter (slots 2, 4, 6: `0x1000d4b0`, `0x1000d510`,
`0x1000d580`) and one that takes **a parameter per axis** (slots 3, 5, 7: `0x1000d390`,
`0x1000d3f0`, `0x1000d450`).

The draw names them. The first channel's value is the particle's **position** — the
billboard is turned by the camera less it (`0x100093fc`) — the third is a **per-axis
scale** on its matrix (`0x1000d0c0`, called at `0x100098c0`), and the second is a
direction, used only where a sprite mode orients rather than faces (`0x1000d110` at
`0x1000986c`). **So the per-axis exponents shape position and size**, which this page had
as a guess.

A channel's floats are four consecutive triples — **low, high, jitter, exponents** — and
each type reads its two at:

| type | position: low, high, jitter, exponent | size: low, high, jitter, exponent | the pow calls |
|---:|---|---|---|
| 3, 4, 9 | +40, +52, —, **+64** | +100, +112, —, **+124** | `0x100106f6`, `0x10010784`; 4 and 9 tail into 3's draw at `0x10010a5a`, `0x10013920` |
| 7, 10 | +44, +56, +68, **+80** | +92, +104, +116, **+128** | `0x10001684`, `0x10001693` |
| 8 | +88, +100, +112, **+124** | +136, +148, +160, **+172** | `0x100121f8`, `0x10012276` |

Each axis is `low + (high − low) × x^e`, with *x* the progress through the window on a
sprite and a bolt and the particle's age on a burst and a stream; an exponent of exactly
1.0 is taken straight rather than through `pow` (`0x10011170`). Where a jitter triple
exists the spawn adds a uniform in ±half of it to the **high** end (`0x1000186a`,
`0x1000195b`, `0x10011ee1`, `0x10011f60`; the generator at `0x10002680` returns
`rand16 × v / 65536 − v / 2`). Types 3, 4 and 9 have a third channel at +76 → +88 with no
exponent, and the run ends exactly where the block's `(archive, member)` pair begins —
+136 for types 3, 4 and 9, +184 for type 8, which is `RESOURCE_AT` twice over.

*Measured* over the **3571** drawing blocks: the position triple is exactly (1, 1, 1) on
**3518** and the size triple on **3300**, so **324 of the 7142 channels** are bent —
**320** blocks over **208** effects, 28 of them a position and 182 a size. By type the
position is bent on 22 of the 1545 type 3, 25 of the 1161 type 7 and 6 of the 160 type 10,
and on none of type 4, 8 or 9; the size on 100, 135, 21 of type 9, 11 of type 8 and 4 of
type 4. **192 of those 7142 triples differ across their axes**, so the per-axis part earns
its name: (0.5, 1, 1) on 134 blocks, (1.5, 1.5, 0.5) on
13, `B_Sphere_Main`'s (1, 0.1, 0.1) on 3. The two ends of the range are worth naming:
**0** puts the value at its high end from the first update, since `x^0` is 1 — the size of
`aim_light_L` and `aim_light_S`, the position of `env_lightning` — and **2** holds it near
its low end almost to the end of the window, which is what `B_Sphere_Sign` and
`B_Sphere_Start` open with.

### Type 1 is a light — *read*, and *measured*

Every agent builds a `CLightManager` (`Terrain.dll:0x1007fa40`) and files it in
its interface table as **`0xe`**, beside its material manager as `0xd` and its
effect manager as `0x13` (`AniMesh.dll:0x1000358f`); the effect manager binds
`0xe` at message 4. A type-1 emitter asks that manager for a light record
(slot 12) and then drives it. The record is **Direct3D's `D3DLIGHT2` layout** —
type at +4, colour at +8, position +0x18, direction +0x24, range +0x30,
attenuation +0x38..+0x40, theta and phi +0x44/+0x48, the active bit in +0x4c —
followed by the manager's flags and 1 / range (slots 3, 4, 5, 6, 8, 9, 13;
*derived* from the offsets each slot writes).

Each quantity in the block is a **(start, end) pair lerped by the progress
through the window** (`0x1000f6e0`):

| block | field | measured over the 618 |
|---|---|---|
| +4 | kind: 1 point, flags `0x80000000`; 2 and 5 point; 3 directional; 4 parallel point; 6 point, `0xa0000000`; 7 point, `0x20000000` (`0x1000f649`) | 5 on 416, 6 on 175, 7 on 14, 1 on 12, 2 on 1 |
| +16 → +28 | position, in the effect's frame, then the owner's | (0, 0, 0) on 427 |
| +40 → +52 | direction | `(1, 0, 0)` at +52 on 542 |
| +64 → +80 | colour, RGBA | overbright starts: (3, 2, 0) on 140, (2, 1, 0) on 110 |
| +96 | colour jitter, ± half of each | 0 on 511 |
| +112 → +116 | range, clamped to at least 0.01, × a factor taken from the instance's frame (its scale is a *guess*) | 30 → 3 on the hero's cannon |
| +120 | range jitter (the read map missed it) | 0 on 560 |
| +124..+132 | the three attenuation terms, handed on unchanged | (0, 1, 0) on 447, (0, 1, 1) on 170, (0, 0, 1) on 1 — never a constant term |

### What a light does to a surface — *read*, and *measured*

A light record is **0x5c bytes** in the manager's array (`Terrain.dll:0x1002a1ca`), and
the manager's flags sit at **+0x50** (`0x100808b7`).

**`EmulatePointLights` (`Terrain.dll:0x1002a130`) reads six fields and no more**: the type
+4, and it skips anything but a point light (`0x1002a1da`); the flags +0x50, and it skips
a light with `0x20000000` (`0x1002a200`); the colour +8..+0x14 (`0x1002ad33`); the
position +0x18 (`0x1002a3f3`); and the range +0x30 (`0x1002a59c`, `0x1002a7d1`,
`0x1002a804`). **The three attenuation terms at +0x38..+0x40 are never read**, nor the
direction +0x24, nor the manager's 1 / range at +0x54. The control is in the same walk:
it does find +0x30 and +8, which the routine plainly uses.

**The range is a hard cut, twice.** A triangle is dropped when the light's distance to its
plane is past the range (`0x1002a59c`), and again when no vertex of it is within the range
of the light itself (`0x1002a7d1`).

What stands in for a falloff is **an extra additive pass**. The light is projected onto the
triangle's plane, and a **disc of radius sqrt(range² − d²)** (`0x1002a804`–`0x1002a817`) is
spanned by two in-plane axes of that length (`0x1002a994`), clipped to the triangle, and
queued as its own draw item. That item's material is the shade's light template —
`shade+0xc18`, copied to the item's `+0x70` at `0x1002ae5e` — with the light's colour
**divided by its own length** in the block's ambient rgb (`shade+0xc2c`, `0x1002ae12`) and
**that length × 0.25, held at 2**, in its ambient alpha (`shade+0xc38`, `0x1002ae35`). So
the shape of the falloff is a texture across the disc, the overbright is the colour's
length rather than its components, and 1 / (a₀ + a₁·d + a₂·d²) never happens.

**What switches it on.** A mesh batch takes emulated lights only when its batch word
carries `0x800` and the shade's `+0xcc4` is set (`0x10045d38`, `0x10045d47`). `+0xcc4` is
settings id 3, **`EmulatePointLight`** (`0x10046c7a`, the page's values at `0x100a6cac`),
which `Terrain.dll` registers with a default of 1 (`0x1005ec5a`); its neighbours are
`LightingOn` (1), `SpecularsOn` (1), `ForceSWFog` (0) and, at id 29, `UseDXLighting`
(default 2), which `CStridedPrimitive::RenderVB` tests before it will build a device
material at all (`0x1002fe50`). `Iron_3D.ini` sets none of the five, so every default
stands.

**The manager flags, every test found.** `0x80000000` at `0x10047a52` — the light is
skipped unless the object being drawn is its owner — at `0x10047c96`, where a second loop
skips such a light outright, and at `0x100808c1` / `0x10080914`, the manager's set-flags
slot, which re-registers the light when that bit changes. `0x20000000` only at
`0x1002a200`. Nothing else in `Terrain.dll` tests either constant.

*Measured*: the shipped attenuation triples are (0, 1, 0) on 447 blocks, (0, 1, 1) on 170
and (0, 0, 1) on 1 — and this path reads none of them, so **what a light's attenuation is
for is not established**. `Ngi32.dll` does carry a `SetLight` wrapper (`0x10008b50`, slot
29 of its render interface, vtable `0x10031600`) and a `LightEnable` beside it
(`0x10008b20`), while its exported `n3dSetLighting` is a stub (`ret 8`, `0x100025a0`);
which caller, if any, reaches slot 29 is not found.

### Type 2 is a sound — *read*, and *measured*

**At load** (slot 1, `0x10012d10`) the emitter takes the sound server from its
manager (`+0x18`), opens its sound (`0x100065a0`, a handle at `+0x98`, −1 when
the member does not load) and stops it at once (server slot 7), and sets its
**previous time** `+0x9c` to 0. Block **+4** picks one of two behaviours: 2 or 3
set the byte `+0x94` (`0x10012d3e`), anything else clears it.

**Each update** (slot 2, `0x10012eb0`), with *t* the effect time:

- **+4 not 2 or 3: a one-shot.** A *t* of exactly 1.0 is taken as 0
  (`0x10012f2d`, the constant at `0x1001e220`). The sound plays when
  previous ≤ *t*, previous ≤ trigger (+8) and trigger < *t*
  (`0x10012f42`–`0x10012f73`). A sound still playing is stopped first and
  started again (server slots 11, 7 and 6). Either way *t*, with 1.0 taken as
  0, becomes the previous time.
- **+4 of 2 or 3: a loop.** While low (+8) ≤ *t* ≤ high (+12) a sound that is
  not playing is started (`0x10012fca`–`0x10012fe8`), so it plays again each
  time it ends. Outside the window a playing sound is stopped
  (`0x10013008`).
- **While a sound plays** (`0x1001300b`) the update hands the server, as message
  `0x38e`, a position lerped from +16 to +28 across the window and carried into
  the owner's frame, a near distance +64 and a far distance +68, and a factor
  lerped from +72 to +76 (`0x10013050`). The lerps of +64 and +68 run from each
  value to itself: they do not change. The factor is the playback rate, not a
  volume ([below](#how-a-sound-is-heard--read-and-measured)).
- **Switched off** (slot 5, `0x10013170`), a playing sound is stopped.

So a one-shot **plays again on the way down** once it has sat at 1.0
(*derived*). A time of exactly 1.0 leaves the previous time at 0, so the first
update below 1.0 crosses the trigger from 0. A one-shot that falls from
anything short of 1.0 stays silent, and a mode-1 effect that ends at 1.0 plays
nothing more while it holds there.

*Measured* over the 517 sound emitters: +4 is 0 on 411 and 2 on 106, and 3 on
none. The 106 loops are what hums or breathes: the chassis engines
(`eng_rb_*`, `eng_rl_*`), the doors, the animals' sounds and breath, and the
hero's `hero_breath`, `H_breath.wav` looping from 1 to 4 m over the whole of
its 3-second time. Every `step_*` effect and all four hero arm sounds are
one-shots.

### How a sound is heard — *read*, and *measured*

**The server.** The emitter's server is a sound world of `Ngi32.dll`'s 3D sound
(vtable `0x100316f0`, made by the 3D sound's slot 4, `0x1000c7d0`; the landscape
keeps it at `Terrain.dll` `+0x7bf4`). Its slots take a sound handle: 4
(`0x1000dad0`) sets parameters, 6 (`0x1000de60`) marks it to play, 7
(`0x1000def0`) stops it, 11 (`0x1000df90`) answers whether it plays. Each sound
is a `0x88`-byte record whose `+8` is a Direct3D `DS3DBUFFER`.

**Message `0x38e` is a mask**, one bit a field (`0x1000dad0`): `2` the factor
(`+8` of the block → record `+0x5c`), `4` the position (`+0xc` → `+0xc`), `8` the
velocity (`+0x18`), `0x80` the near distance (`+0x3c` → `flMinDistance`), `0x100`
the far distance (`+0x40` → `flMaxDistance`) and `0x200` the mode (`+0x44` →
`dwMode`). The emitter's block, laid out from `0x10013048`: `+8` the factor, `+0xc`
the position, `+0x18` the owner's velocity (its interface `0x27`), `+0x3c` near,
`+0x40` far, `+0x44` the emitter's float +80. *Measured*: +72 and +76 are 1.0 on
516 of the 517 sound emitters (0.8 → 1.0 on the other), and +80 is 0 on all 517,
`DS3DMODE_NORMAL`.

**The factor is a rate.** The 3D sound's update (`0x1000e2c0`) multiplies the
sample's own frequency by record `+0x5c` and hands it to the buffer's
`SetFrequency`, held to the device's range (`0x1000e443`).

**Two paths, one taken.** The 3D sound object keeps at `+0x7c` bit 16 of the flags
it was made with (`0x1000c667`). With it set the update mixes by itself; with it
clear it hands each buffer its `DS3DBUFFER` whole through `SetAllParameters`
(`0x1000e74e`) and the listener its own each frame (`0x1000ca06`), and Direct3D
Sound places the sound. **The object is made once**, by whoever asks first
(`niCreate3DSound`, `0x1000c3e0`, returns the one there is):

- `services.dll`'s sound server makes it with flags **`0x120`**
  (`0x10011919`), from `iron3d.dll`'s `createSubsystems` (`0x1005b6d0`,
  `0x1005b835`), which the executable calls at start;
- `World3D.dll!stdInitGame` (`0x10013f5e`) would make it with its settings'
  flags, whose sound mode starts at `0x10000` (`0x10014c30`; setting 110,
  `0x1000a91f`, which nothing found sets), but it runs when a mission loads,
  after the object exists.

So the game takes **the Direct3D Sound path** (*derived*). Its distance law is
Direct3D Sound's own: whole within the minimum distance, and beyond it the gain
falls as *min* ÷ (*min* + *R* × (*d* − *min*)), 6 dB a doubling for a rolloff *R*
of 1, no further past the maximum distance (Microsoft's `DS3DBUFFER`
documentation). *R* is the listener's: the listener object (vtable `0x10031728`,
made by the 3D sound's slot 3, `0x1000c8c0`, for `Terrain.dll`'s camera at
`0x10083bc2`) starts its distance, rolloff and Doppler factors at 1.0
(`0x1000d123`–`0x1000d129`), and no call to its parameter slot (5, `0x1000d370`)
is found. The listener's position and orientation come from its camera's
matrix (`0x1000d4bc`), and both the listener and the buffers go to Direct3D
Sound with y and z swapped (`0x1000d5df`, `0x1000e4a7`).

**Past the far distance** a one-shot is stopped (`0x1000e51b`–`0x1000e54a`: the
distance above `flMaxDistance` sets `+0x37`, and a record not looping, `+0x7d`,
is cleared); a loop plays on.

**The other path**, for the record (`0x1000e615`–`0x1000e6ff`): silent past the
far distance, else `1000 × log2(min ÷ (min + R × (d − min)))` hundredths of a
decibel inside it, 0 within the near distance, held above −10000; and a pan of
−4000 times the sound's direction against the cross of the listener's front and
top.

*Measured* in Mission 01's recording, where the sound server's levels are known
(a voice, `T01_T01`, sits 11.8 dB under its file's own level): the hero's cannon
(`H_fire_cannon.wav`, within its near 10) is found at normalised correlation
0.64–0.74 at every shot from 126 s to 166 s, and the radar's ping every 1.51 s
from 106.8 s on.

**The hero's breath is not heard.** `hero_breath` is made by the turret's load
group (action 4 on points 20–22, node 3) and runs in time mode 2, its loop over
the whole window, some 0.6 m from the eye in the engine's rest pose, inside its
near distance of 1: read, it should sound whole. But `H_breath.wav` mixed into the recording's own audio at
the voices' level, once every 3.1 s from 100.5 s, is found at every one of its
starts (correlation 0.08–0.15), and the recording itself has no such run (none
above 0.062, and no 3-second rhythm) (*measured*). What silences it is not
established. One gate is read: before each tick an instance whose attach node
(`AniMesh` record `+0x14` flags 1 or 4, or a stage `+0x1c` not 0 without header
flag `0x8000`) counts as hidden is switched off unless its header has flag
`0x10` (`0x10006170`–`0x10006211`), and `hero_breath` has not; that the hero's
node 3 counts as hidden in the game's own view is not found.

### Bolts, streams and fades — *read*, and *measured*

**A type-5 bolt's length is not in its block.** The update keeps a **start
point** — the effect's position on its first tick, or the manager's target
point, which header flag 0x1000 hands every emitter each tick — and the
effect's current position (`0x10002a20`). The draw measures the distance
between the two and lays **floor(length / +36) sprites** along it, at least
one and at most +20 (`0x10002c53`); +4 → +8 is the fade value across the
window. *Measured* on all 31 bolts: +20 is 20, +36 is 50, 150, 200 or 250, +40
is −1 (a phase in seconds) and +4 → +8 is 1 → 0. The fade runs straight across
the window (`0x10002dd4`). Each sprite's texture runs along the beam, one repeat
every +32 (`0x10002e79`, `0x10009b90`); +32 is 5 on the hero's laser.

**A bolt's +24 and +28 are its width at the two ends of its window**, not at the two ends
of its beam. The load lays every sprite's size channel out with a base of (1, +24, +24) and
a delta of (0, +28 − +24, +28 − +24) (`0x10002944`–`0x100299be`), and the draw hands that
channel the progress through the window (`0x10002fb4`, the scalar setter `0x1000d580`) —
the same channel a particle's size uses ([above](#a-channel-is-a-low-high-jitter-exponent-run--read-and-measured)).
The 1 is along the beam, where each segment's own length sizes it, so y and z are the
width and the cross-section is square. *Measured* on all 31 bolts: **26 carry the same
value twice** — a width that does not change — and 5 halve: (1.5, 0.75), (3, 1.5),
(1.5, 0.5), (0.6, 0.3) and (0.9, 0.45). `hero_laser_bullet`'s two bolts are 0.4 and 0.1
wide and both are constant.

**The manager's target point is the muzzle.** The gun sets it as it makes the
round (slot `0x44`, `0x10004c50`): the shooter's node 0 and the muzzle point, which
the manager carries with that node on every tick. The 14 effects that take it
are the laser, taser and builder tails, and each of the 23 rounds that carries
one restarts it in time mode 1 as it stops. The beam then stands after the
round has stopped, 0.75 s on a laser
([29-weapons.md](29-weapons.md#a-beam-outlives-its-round--read-and-measured)).

**A type-8 stream emits by interval.** While (last emission + lerp(+24, +28,
window progress)) is before now, it emits one more particle, spaced along the
path the effect moved, into a ring of +36 (`0x10011a6c`, `0x10011230`).
A stream particle's age runs 0 to 1 over the ring, so **it lives +36
intervals** (`0x1001209e`). *Measured* on the 237 streams: intervals from 0.005
to 0.2 seconds — (0.08, 0.08) on 80, (0.05, 0.01) on 48 — rings of 6 to 40,
and lives of 0.05 to 4.5 s; the hero cannon's smoke puffs every 0.05 s falling
to 0.02, ten at a time, so each lives 0.5 s falling to 0.2.

**A fade value.** A burst particle hands the renderer
**+8 + (+12 − +8) × age^+16** (`0x100013c2`), a stream particle
+4 + (+8 − +4) × age^+12 (`0x10012322`), a sprite +20 + (+24 − +20) ×
progress^+28 (`0x10010881`). The renderer draws nothing at 0
(`Terrain.dll:0x1002887e`). *Measured*: 1197 of the 1321 burst blocks fall
from start to end, 118 hold and 6 rise; 233 of the 237 streams fall and 4
hold.

**The value stands in for the material's ambient alpha** (*derived*):

- The renderer copies the state the value arrives in (`Terrain.dll:0x100288e1`,
  `0x10029aa0`) into the draw item's material block (`0x10028287`). That block's
  `+0x20` is the entry's ambient alpha
  ([07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured)).
- The device material makes it its diffuse alpha (`0x100308af`), so the value
  scales the texture's alpha.
- An effect sprite's draw item carries draw flags 4 (`0x100282a3`), without the
  `0x10` that turns Direct3D's lighting on. `Ngi32.dll` draws such an item as
  pre-lit vertices, FVF `0x1e2` (`0x100075fb`), and their colour is the material's
  ambient with the ambient alpha as its alpha
  ([below](#how-an-effect-sprite-is-coloured--read-and-measured)).
- *Measured*: 233 of the 243 materials the effects draw carry a black diffuse and
  an ambient colour, the unlit glow of
  [07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured).
- *Seen*: the hero's laser is red and pink, while its `LASER.0` is grey and only
  its materials' ambient is red.
- **A blend adds to what the surface holds** (*derived*). Direct3D's fixed-function
  blend states work on the render target's stored values, with no conversion
  from display space. The game draws to a 16-bit surface
  ([05-engine.md](05-engine.md)), so an additive sprite adds its colour times its
  alpha to the display-space pixel. Over Mission 01's lavender sky that saturates
  red wherever the laser's texture alpha is above about a third. *Seen*: the
  recording's beam is a broad pink band, and yellow `NE_Laser_Y` makes its core
  white, not yellow.
- *Seen*, not explained: near the camera the recording's beam looks broader than a
  0.4 m strip whose alpha peaks in the middle quarter. The width is not the answer —
  `hero_laser_bullet`'s +24 and +28 are 0.4 and 0.4 (*measured*, above).

### How an effect sprite is coloured — *read*, and *measured*

**The device material is never built for one.** `CStridedPrimitive::RenderVB` gates both
the device material (`Terrain.dll:0x10030620`) and the software shade that stands in for it
on draw flag `0x10` (`0x1002fe3d`), and an effect sprite's item carries 4. With the flag
clear the draw goes straight to the unlit vertex paths chosen at `0x10030104`: a buffer of
FVF **`0x1c2`** — `XYZ | DIFFUSE | SPECULAR | TEX1`, 28 bytes a vertex — for one texture
stage (`0x1002f1e0`) and `0x2c2` for two (`0x1002f3a0`, `0x1002f480`), against `0x112`
(`XYZ | NORMAL | TEX1`) and `0x212` on the lit side. The expansion copies the item's colour
stream into each vertex's diffuse and its specular stream into the specular
(`0x1003534f`, `0x100355bd`). `Ngi32.dll` then sets the format from the same flags:
`0x1c4` for flag 8, `0x112` for flag `0x10` and **`0x1e2`** otherwise (`0x100075fb`).

**The item carries one colour, not a stream.** Building it (`0x10028260`), `CShade` copies
the material entry to the item's `+0x70`, sets the draw flags to 4 (`0x100282a3`) and then
**overrides what the caller passed**: it writes one `D3DCOLOR` at the item's `+0x19c` and
one at `+0x1a0` and points the diffuse and specular streams at them **with a stride of 0**
(`0x1002845b`–`0x100284a1`), so all four vertices of the quad carry the same pair. (What
the emitter passes instead is an all-zero descriptor — `Effect.dll:0x1001e860`, the
particle's slot 8 — so nothing of its own reaches the vertices.)

**Where the pair comes from.** `CShade::RenderEffect` (`Terrain.dll:0x10028840`, the
shade's interface slot 27, which `Effect.dll` calls at `0x100099df`) is handed the material
entry the manager gave the sprite, with the fade already written into the entry's **ambient
alpha `+0x20`** (`Effect.dll:0x100099a8`); it draws nothing at 0 or less (`0x1002887e`).
The item builder then asks the **Shader** component — `Comp.ini` names component 6
`terrain.dll!CreateShader` — through `shade+0xbd0` slot 4 (`0x10028220`, `0x1002824e`),
which is `0x1004f710`, with the entry, the distance to the eye (`0x1003c340`) and effect
draw flag 4. It reads three components at the block's `+0x14`, `+0x18` and `+0x1c`
(`0x1004f71c` and on) and the alpha at `+0x20` (`0x1004f82f`), which is the slot the fade
arrives in:

```
diffuse  = knee(ambient.r), knee(ambient.g), knee(ambient.b), ambient alpha   x 255
           knee(c) = c              c <= 1
                     c/6 + 5/6      1 < c <= 7
                     2              c > 7
specular = 0, 0, 0, fog             x 255
           fog = 1                              d^2 <= near^2   (0x1004bf20)
                 0                              d^2 >= far^2
                 1 - (d^2 - near^2) x k         between
```

so an effect sprite fogs **linearly in the squared distance**, which the engine does not
follow — its sprites take the same linear-in-distance fog as its other pipelines. Effect
draw flag 4 — header flag `0x2000`, which only `env_lightning` carries — forces the factor
to 1, so that effect never fogs. One branch is left over: with the shader's own `+0x3c`
bit 0 set the diffuse is replaced by green alone at `(r + g + b) × 0.33 × 7` held at 255 (`0x1004f900`),
a monochrome mode nothing found switches on.

**So a fade scales alpha, not colour** (*read*). The colour on every vertex is the
material's ambient — the unlit glow of
[07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured) — with
no scene colour added, and the fade is the alpha the texture stage multiplies the texture's
alpha by. *Measured* over the **243** materials the effects draw: **all 1598 of their
entries carry an ambient alpha of 1.0**, so the fade is the whole of a sprite's alpha, and
not one entry has an ambient component above 1, so the knee never fires on a shipped
effect; 1583 of the 1598 carry a black diffuse, which this path would not read anyway.

## Which effects run: the settings switch — *read*, and *measured*

Header +0x14 is a **settings id**. `Effect.dll` registers a settings page with
the game's settings (`InitializeSettings`, `0x10014090`, id `0x14`) whose
switches are named by the DLL's own string table under the very same ids
(`0x1000e9c0`); an instance whose switch is off neither updates nor draws
(`0x1000ec40` returns table[id & 0xff], `0x10007d59`, `0x1000825e`). The high
byte is the group the switch is listed under:

| group | switches |
|---|---|
| 0 | `0x000` Dust, `0x001` Smoke, `0x002` Engine fire, `0x003` Explode, `0x004` Shield, `0x005` Lights |
| 1 | `0x106` Dust, `0x107` Smoke, `0x108` Explode, `0x109` Shield, `0x10a` Lights, `0x10f` Lights |
| 2 | `0x20b` Smoke, `0x20c` Engine fire, `0x20d` Explode, `0x20e` Gun fire |
| 3 | `0x310` Smoke, `0x311` Explode, `0x312` Gun fire, `0x313` Lights |

Each group also carries a "LOD distribution" and a "High quality LOD" name
(`0x?f0`, `0x?f1`) and a float the particle emitters scale by a random number
(`0x1000ec50`). What the four groups stand for is *unknown*.

**The presets.** The page's slot 8 (`0x1000e7c0`) sets all twenty at once:
presets 1 and 2 turn every switch on; preset 3 turns off the dust and smoke of
group 0, the dust, smoke and lights of group 1, the smoke and engine fire of
group 2 and the smoke of group 3. `iron3d.dll` reads `Iron_3D.ini`'s
`RENDER_QUALITY` and asks every page for preset 3, 2 or 1 for the values 0, 1
and 2 (`0x100616f0`, through `World3D.dll:0x1000a600`); the page itself starts
on preset 1.

*Measured*: **every one of the 923 effects names one of the twenty** — 18 of
them in use, `0x311` "Explode" on 237 and `0x5` "Lights" on 120 — and the
install's `RENDER_QUALITY=2` leaves them all on. The hero's guns are `0x312`
"Gun fire". `hero_helm_light` is `0x107` "Smoke", so a switch's name is the
artists' filing rather than a rule.

## Bit 8 and the tested point — *read*, and *measured*

When the factory builds an instance it marks it for a **visibility test** if
any emitter has bit 8 or the header has flag 0x400 (`0x10007984`). At
intervals (`0x10007fa1`) the instance's draw carries header +0x24 into the
world through its frame and casts a ray to it from the camera through the
world (`0x10007eb5`, `0x10007f7f`): no hit, and the point is **in view**.

- **Flag 0x400** draws nothing while the point is hidden (`0x10008016`).
- **Bit 8**: while the point is in view the emitter's sprites are drawn with
  effect draw flag 1 (`0x10009930`), which makes the renderer turn
  **`ZENABLE` off** for them (`Terrain.dll:0x100282c6`, set at
  `0x1003e54c`) — a glow seen through what stands in front of its sprites
  once its centre is visible. Hidden, the sprites are depth-tested like any
  other. Every sprite that goes through that draw carries flag 2 as well,
  **`ZWRITEENABLE` off**.

*Measured*: bit 8 is on 1811 emitters, all of them drawn — 1088 of type 3, 660
of 7, 39 of 9, 20 of 8, 4 of 4 — and never on a light, a sound, a bolt or type
10. Header +0x24 lies on +x on all 923: (0, 0, 0) on 462, **(1, 0, 0) on 368**
and (0.5, 0, 0) on 93, and 359 of the 467 effects with a flagged emitter lift
it off their origin. An impact is aimed along the struck face's vector
(placement 7, below); if that vector becomes the effect's x axis, (1, 0, 0)
keeps the test point clear of the face the effect sits on — a *guess*.

### A beacon light's glow — *read* in part, and *measured*

The lamps buildings and robots carry are the other side of the test. *Measured*:
header flag 0x400 is on 114 effects, and **112 of them have no bit-8 emitter**;
110 of those carry `0xd00` or `0xd20` (0x800, 0x400, 0x100, and ping-pong on the
blinkers), the `f_*light*`, `rb_*`, `rl_*` and `rm_*` beacons. `f_signlight_g`,
two of which each of Mission 01's bridges starts, is the lamp's ring (`LAMP`,
`S15`, 1 wide), a ball (0.65), a spark (0.4), a **glow 9 across** (`GLOW_G`,
additive) and a light. Only `B_Sphere_Sign` and `f_build_sign` pair 0x400 with
bit 8.

So by what is read a beacon's sprites never take effect draw flag 1: while its
point is hidden it draws nothing, and while it is in view its sprites go through
the renderer depth-tested like any other. Two more gates stand in front of it
(*read*): the instance's draw skips a 0x800 effect when its pass argument is 0
(`0x10007d3a`), and the manager's draw returns at once for a pass argument of 0
unless its flag 2 is set (`0x10004061`). The manager starts with flags 8
(`0x10003b9a`); message 23 sets its bit 0 and message 24 clears it
(`0x10003f2e`, `0x10003f46`), which holds a 0x100 effect's *t* at 0.

*Seen*: drawn depth-tested in the scene pass, as the engine drew it, the 9 m glow
on the bridge's pylon top is cut by the pylon's own faces and shows only through
the gaps between them; a player's report on Mission 01 calls that wrong.

Not established: which call passes the pass argument, and so in which pass, and
with which depth state, the beacons draw; what sends messages 23 and 24.

## What an explosion plays — *read*, and *measured*

A node's damage stage plays its `.exp` (`Control.dll:0x10011220`):

1. **The surface.** It takes the exploding object's contact record. With a
   world face it asks the face's owner for its material manager and reads the
   face material's class byte (`Control.dll:0x100114fd`), the same query as the
   ground contact ([24-motion.md](24-motion.md#ground-and-collision--read-and-measured)).
   A record with no face gives **10** (`0x1001150b`); no record, or an owner
   with no manager, leaves it unset.
2. **The slot.** Surface 0..10 plays slot surface + 1; unset, or a slot that
   does not load, plays **slot 0** (`Control.dll:0x100117d0`, `0x100117f7`).
3. **Where.** At the node's bounding-sphere centre, with its axis picked by the
   placement word (`Control.dll:0x100115e3`): 0 the node's second axis, 1 and 2
   its first, 3 its third, 4–6 the object's axes, **7 the struck face's
   vector** (the face's `+8`), or the contact's own vector when it has no face
   — that this is the impact normal is a *guess*.
4. **How big.** Scale = the `.exp` radius on a round (agent kind 9), the radius
   × the node's bounding radius on anything else (`Control.dll:0x10011749`).

**A unit answers for its material** (*read*). A round's contact record keeps
the object, node, batch and triangle it struck; a contact with an object id
and no −1 among them goes to `CWorld::GetWorldFace` (`Control.dll:0x1001147e`,
`Terrain.dll:0x10024d70`). That asks the struck object for its geometry,
interface `0x18` — on a unit its `AniMesh` (`AniMesh.dll:0x10006e50`) — whose
slot 3 gives the triangle's record, with the **material id** at `+0x34`: the
node's wear base ORed with the struck batch's material byte
(`AniMesh.dll:0x100135c8`). The damage stage then asks the object for interface
`0xd`, and **a unit has one**: its loader keeps the material manager it loads
for its own model at `+0x14c` and files it in its interface table under `0xd`
(`AniMesh.dll:0x100032a4`, `0x1000358f`), which both its `QueryInterface`s
serve (`0x10001320` when the agent has no outer object; `0x100012d0` always).
Units have no outer object — `World3D.dll`'s loader passes 0
(`0x10007bc8`); only a `CBuilding` aggregates its agent. So a hit on a unit
plays the slot of the **struck batch's material class**.

*Measured*: the 1150 wear materials of the 63 `BTLU` units are class 5 on 992,
unset on 92 (slot 0), 6 on 30, 8 on 25, 9 on 8, 10 on 2 and 1 on 1; the Mission
01 dummies `r_h_01`, `r_h_03` and the hero's `r_h_02` are class 5 throughout,
so a hit on them plays the **`mt`** effect (slot 6).

*Measured*, which surface each slot is — the material classes carry the tags'
names:

| slot | tag | class | its materials |
|---:|---|---:|---|
| 1 | `sn` | 0 | `L08`, `L09` |
| 2 | `st` | 1 | `STONE00/01`, `B_FOUND`, `L02`… (the commonest ground) |
| 3 | `gr` | 2 | `L00`, `L01`, `L19`, `L20`, `L28`, `L32`, `L40` |
| 4 | `sw` | 3 | `L23`, `L26` |
| 5 | `ic` | 4 | `L18`, `L27`, `NE_GIBS_ICE` |
| 6 | `mt` | 5 | 342 machine and building skins |
| 7 | `gr` | 6 | trees, bark, grass, leaves |
| 8 | `wt` | 7 | `WATER`, `WATER_M` |
| 9 | `al` | 8 | `B_MTP_*`, the main teleports |
| 10 | `an` | 9 | `BIRD_*`, `R_PG9` |
| 11 | `sh` | 10 | `NE_SHIELD2/3`, `NE_MASLO` |

Read one slot either way, none of the seven name witnesses agrees.

## Mission 01's effects — *measured*

- **The hero's turret** (`o_tur_ht_02`) binds nine effects at load on
  control-point triples: `hero_cannon` (11, 12, 13), `hero_prifle` (15, 16, 17),
  `hero_redlaser` (4, 5, 6), `hero_helm_light`, `hero_breath` and four `_sfx`
  sounds; action 14 times the three guns' effects from their barrel nodes 13,
  9 and 22 and the sounds from the arm nodes 11, 7, 19 and 15. All seven are
  time mode 4, so they play as those nodes' channels run: the guns' effects
  through each barrel stroke, the `_sfx` as the arms unfold
  ([above](#time-mode-4-is-a-nodes-animation-value--read-and-measured)).
- **The rounds** carry their flight effects: `hero_cannon_bullet`
  (a sprite and two glows), `hero_laser_bullet` (two type-5 bolts from the
  muzzle, riding on the hero's body, to the round, run over 0.75 s once it
  stops), `hero_prifle_bulletA/B`, and the
  missile's engine, smoke and launch.
- **A hit.** Each round's node explodes through its `.ndp` `.exp`: `bb_h_01`,
  `bl_h_01`, `bp_h_01` kind 2 and `bm_h_01` kind 3, all with placement 7 and
  eleven surface slots (`exp_H??_bul`, `_las`, `_pls`, `_mis`). The dummies'
  and hero's skins are all class 5, and a unit answers for its material, so a
  hit on them is the **`mt`** effect.
- **Every switch is on**: the hero's guns are `0x312` "Gun fire", and the
  install's `RENDER_QUALITY=2` picks preset 1.
- **Range end** plays `bb_h_01_end`, `bl_h_01_end`, `pls_h_end` or the missile's
  own blast `exp_m_mis` in the air (slot 0).
- **The dummies** explode with `explode_aim_S` (`r_h_01`, radius 5.5) and
  `explode_aim_L` (`r_h_03`, 15), kind 1: `aim_exp_S/L`, 3 s, two fire bursts,
  two sprites, smoke, a sound and a light.

## Not resolved

- ~~**The sound server's slots**, what message `0x38e`'s +80 float is, and how
  near, far and volume become gain.~~ Answered: a sound world of `Ngi32.dll`'s
  3D sound; +80 is the `DS3DBUFFER` mode, the "volume" is a playback rate, and
  Direct3D Sound's distance law applies
  ([How a sound is heard](#how-a-sound-is-heard--read-and-measured)).
- **What silences the hero's breath** in its own view.
- **How Direct3D Sound pans** a sound about the listener.

- **The rest of each emitter's floats.** The window, the phase, the sprite's
  moving and growing triples, the light, the bolt's segments, the stream's
  interval and lifetime, and the fade values are read, and the fade value is the
  material's ambient alpha ([above](#bolts-streams-and-fades--read-and-measured)).
  ~~Still unnamed: what the (low, high) triples a particle's per-axis
  exponents shape are — position and size is a *guess* (types 7 and 10: +80
  and +128; type 8: +124 and +172, `0x10012030`) — and a bolt's widths +24/+28
  (a width at each end is a *guess*).~~ Answered: they shape **position and size**,
  each channel a (low, high, jitter, exponent) run the particle class lerps per axis,
  and a bolt's +24 and +28 are its width at the two ends of its *window*
  ([above](#a-channel-is-a-low-high-jitter-exponent-run--read-and-measured)).
- ~~**How an effect sprite's pre-lit vertices are coloured** (draw flags 4, FVF
  `0x1e2`): the unlit branch of `CStridedPrimitive::RenderVB` picks its vertex
  path at `0x10030104`, and none of them was followed.~~ Answered: one colour on
  every vertex of the quad — the material's ambient, with the ambient alpha, where
  the fade lands, as its alpha, and the fog factor in the specular's
  ([above](#how-an-effect-sprite-is-coloured--read-and-measured)). *Seen*: the laser
  is red, as its materials' ambient colours are.
- ~~**How the shade lights with a type-1 light** — the falloff over range and
  attenuation (`EmulatePointLights`, `Terrain.dll:0x1002a130`, and the Direct3D
  path), and what the manager flags `0x80000000` and `0x20000000` mean beyond
  the two tests found.~~ Answered in part: `EmulatePointLights` reads six fields of
  the record and **not the attenuation**, cuts hard at the range and draws the light
  as a textured disc of radius sqrt(range² − d²); every test of both flags is now
  enumerated ([above](#what-a-light-does-to-a-surface--read-and-measured)). Still open:
  **what a light's attenuation triple is for**, since no path found reads it, and
  whether anything reaches `Ngi32.dll`'s `SetLight` (`0x10008b50`).
- **Who passes the draw's pass argument** that flag 0x800 waits for (manager
  slot 3, `0x10004050`; the landscape's call at `Terrain.dll:0x1001f178` pushes
  one argument fewer than the slot takes), what draw flag 4 (header
  0x2000) changes in the texture choice, and header flag 0x10000.
  ~~Who sets the manager's target point that bolts start from.~~ Answered: the
  gun, as it makes the round: the muzzle on the shooter's node 0
  ([29-weapons.md](29-weapons.md#a-beam-outlives-its-round--read-and-measured)).
- **What the four settings groups are**, and the group floats `+0x1084` and
  `+0x1294` and the page's `+0x14a4` that the presets set.
- Snow and rain are **not** here. There is no FXID whose name mentions either,
  and `sky.wea`'s slots name the materials `SNOWFLAKE` and `RAIN_DROP`
  directly — see [10-sky.md](10-sky.md).
