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
It is set on 1811 of the 4737 emitters and what it controls is not known.

Types 7 and 10 allocate the same 0x48-byte object and install different
vtables, so they are sibling classes rather than unrelated ones.

## Inside a block

Only the sound emitter is read. Type 2 keeps a **near and far audible
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

**180 of the 441 four-byte slots across the ten block types are read as a
float.** Everything else the editor wrote and the engine never looks at:

| type | block | offsets it loads |
|---:|---:|---|
| 1 | 224 | 8, 12 · 28, 32, 36 · 52, 56, 60 · 80, 84, 88, 92 · 112, 116 |
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
**not one of the 180 offsets lands inside a block's `(archive, member)`
pair**, though the read map comes from the code and `RESOURCE_AT` came from
the data; an error on either side would collide somewhere.

The families fall out of it. **Types 3 and 9 read the same eighteen
offsets** — type 9's constructor installs type 3's vtable and then overrides
it, so it inherits the layout. **Types 7 and 10 read the same thirty-one**,
which is the sibling relationship the factory already suggested. Type 4 reads
the same as type 3 as well: its `+32` and `+36` are read through a second
pointer to the block, at `+0xfc` (`0x100108f9`), which an earlier walk did not
follow. Type 1 also reads `+8` and `+12`, its window, in an update the first
sweep did not decode (`0x1000f6e0`).

One field is identified by its shape rather than by the code: **types 1 and 2
keep a unit vector at `+52`** — unit length on 597 of 618 and 517 of 517
blocks, and exactly `(1, 0, 0)` on 542 and 517 of them. A direction with a +X
default. The drawing types keep something else there: on type 8 not one block
of 237 is unit length.

Type 3's `+40..+48` and `+52..+60` are a component-wise (low, high) pair of
`float32[3]` on 1427 of 1545 blocks, which is the shape a particle spread
takes — and the engine reads exactly those six, which is the data and the code
agreeing without either being derived from the other. Types 7 and 10 read a
`float32[3]` at `+56`, `+80`, `+104` and `+128`, a regular 24-byte stride, in
their own methods rather than through a helper.

**None of that names a field.** It says which of the 30 to 60 floats in a
block are live, which classes share a layout, and which two hold a direction.
What they mean is still open.

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

**All 98987 reads of the 180 live slots are a real float**: finite, and either
exactly zero or between 1e-6 and 1e6 in magnitude. The 123 dead slots manage
**92.3%** — 5395 of their reads are NaN, denormal or absurd, which is what an
editor's uncleared buffer looks like. Nothing in the recovery used the values.

Sorting the live slots by the shape of their values across the library gives:

| shape | slots |
|---|---:|
| signed | 37 |
| positive | 59 |
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
| +0x10 | u32 | flags: 1 jitter; **2 delete once *t* ≥ 1**; 8 a random offset (+0x18); 0x10 keep running when the attach point is hidden; 0x20 ping-pong; 0x40 start switched off; 0x80 / 0x100 hold *t* at 0 while the manager's bit 0 (messages 23, 24) is clear / set; 0x200 × linear progress; 0x800 tested in the draw (`0x10007d44`); 0x8000 skip the attach point's second test | explosions 0x16, rounds 0x10 |
| +0x14 | u32 | a number a global table must accept before the instance updates or draws (`0x1000ec40`); meaning *unknown* | 18 values, 0 to 787; 786 on the hero's guns |
| +0x18 | f32×3 | random offset amplitudes (flag 8) | |
| +0x24 | f32×3 | a point in the instance's frame (`0x10007eb5`); a bounds centre is a *guess* | (1, 0, 0) on impacts |
| +0x30 | f32×3 | **scale**; the size a controller or `.exp` asks for is multiplied by it | 0.1 on the hero's muzzle effects |

What 0x4 and 0x1000 do is *unknown*.

**Effect time *t*** runs 0 to 1 (`0x10005c60`, 18 modes):

| mode | *t* |
|---|---|
| 0 | a value set from outside (slot `0x1c`), **0 until set** |
| 1 | (now − start) / (end − start): once through |
| 2 | its fractional part: looping |
| 3 | 1 − mode 1 |
| 4 | the owner's value at a control point (action 14), through the owner's interface `0xb`, slot 9 |
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
update, or within 50 ms of a command to it.

## Emitter types — *read*, and *measured*

| type | class | window (block offsets) | what the code shows | what it is |
|---:|---|---|---|---|
| 1 | 0xf0, vtable `0x1001e78c` | +8..+12 | switches a handle from the owner's interface `0xe` on inside the window and off outside, then places it (`0x1000f6e0`) | a **light** — *guess*: +80..+88 is (0.5, 0.3, 0.01) on the cannon, (1, 0.2, 0.2) on the red laser, (0.2, 0.2, 1) on the plasma rifle; +112/+116 two light parameters (30, 3; the helm light 0.15, 0.35) |
| 2 | 0xa0, `0x1001f048` | trigger at +8 | plays once *t* crosses +8 (`0x10012f42`) | the **sound**; near/far at +64/+68 |
| 3 | 0xfc, `0x1001e770` | +32..+36 | a phase *a* + (*b* − *a*)·*x*^*g* from +8/+12/+16, *x* the progress through the window or seconds when +8 < 0 (`0x100105f0`); the (low, high) triples +40/+52 and +100/+112 shaped by per-axis powers; a 0..1 value +20..+24 to the power +28 and the phase's fractional part go to the draw (`0x100106c0`) | a **sprite** that moves (+40/+52) and grows (+100/+112) over its window — muzzle flashes, glows, bullets |
| 4 | 0x104, `0x1001e754` | +32..+36 | type 3's phase (`0x100108f0`) | a sprite variant |
| 5 | 0x54, `0x1001e360` | +12..+16 | a phase from +40, seconds when negative (`0x10002a20`) | the **laser** bolt `hero_laser_bullet` — a beam is a *guess* |
| 6 | 0x1c, `0x1001e738` | — | — | never shipped |
| 7 | 0x48, `0x1001e228` | +20..+24 | +0x24 × +0x28 particles with random start positions and velocities (`0x10001720`); each particle's age = (phase − its spawn) / +0x1c, one-shot flag, drag (`0x10001300`) | a **particle burst** — smoke, fire, splashes |
| 8 | 0xac, `0x1001e71c` | +16..+20 | windowed on *t* unless +0x8c (`0x100115c0`) | a **particle stream** — dust, missile smoke |
| 9 | 0x100, `0x1001e700` | +32..+36 | type 3 with its own draw | sprite |
| 10 | 0x48, `0x1001e24c` | +20..+24 | type 7 with its own start | particles (`NE_Gibs_Stn` debris) |

*Measured*: the window is an ordered span inside 0..1 on **4736 of 4737**
emitters; read four bytes early or late, 705.

A type-7 block with +4 = 1 draws its particles in sprite mode 3, any other
value in mode 0 (`0x100019d0`); that mode 0 faces the camera is a *guess*. Bit 8 of the type word goes to the
sprite draw (`0x10009930`); what it switches there is *unknown*.

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
  sounds; action 14 drives the three guns' effects from points 13, 9 and 22.
  The gun effects are time mode 4 — they play as that point's value runs from 0
  to 1 — which point value that is belongs to the firing chain.
- **The rounds** carry their flight effects: `hero_cannon_bullet`
  (a sprite and two glows), `hero_laser_bullet` (two type-5 bolts),
  `hero_prifle_bulletA/B`, and the missile's engine, smoke and launch.
- **A hit.** Each round's node explodes through its `.ndp` `.exp`: `bb_h_01`,
  `bl_h_01`, `bp_h_01` kind 2 and `bm_h_01` kind 3, all with placement 7 and
  eleven surface slots (`exp_H??_bul`, `_las`, `_pls`, `_mis`). The dummies'
  and hero's skins are all class 5, so a hit on them is the `mt` effect if the
  unit answers for its material — *unknown*, see below.
- **Range end** plays `bb_h_01_end`, `bl_h_01_end`, `pls_h_end` or the missile's
  own blast `exp_m_mis` in the air (slot 0).
- **The dummies** explode with `explode_aim_S` (`r_h_01`, radius 5.5) and
  `explode_aim_L` (`r_h_03`, 15), kind 1: `aim_exp_S/L`, 3 s, two fire bursts,
  two sprites, smoke, a sound and a light.

## Not resolved

- **Most of each emitter's floats.** The window, the phase and the sprite's
  moving and growing triples are read; a particle's colour and alpha over its
  life, the emission rate of type 8 and a type-5 bolt's length are not named.
- **What type 1 drives.** It switches something from the owner's interface
  `0xe` on and off; that it is a light rests on the colours.
- **What bit 8 does** in the sprite draw, and header flags 4 and 0x1000.
- **Whether a unit answers for its material** when a round strikes it, so
  whether a hit on a robot plays the `mt` slot or slot 0: the terrain answers
  (`Terrain.dll:0x1001a25a`), an agent's interface `0xd` was not read.
- **What the owner's value at a control point is** (time mode 4) — the gun's
  firing channel is a *guess* for R6.
- Snow and rain are **not** here. There is no FXID whose name mentions either,
  and `sky.wea`'s slots name the materials `SNOWFLAKE` and `RAIN_DROP`
  directly — see [10-sky.md](10-sky.md).
