# `sky.ske` — the atmosphere

Every mission directory has a `sky.ske` beside its `data.tma`. It is **not a
skybox**. `Terrain.dll` calls it an *atmosphere* file and names the tool that
wrote it:

```
SunDll panic : Old version ske file
 ReSave in SunEditor
```

alongside `CAtmosphere`, `CAtmData`, `CSun`, `CreateAtmosphereObject`,
`"Illegal atmosphere object type"`, and the settings `AtmSkyDetail`,
`AtmStarsOn`, `AtmCloudsOn`, `LensFlareOn`.

What it holds is a **day cycle**: a list of keyframes, each stamped with an
hour and a minute, carrying the colours and intensities the sky takes at that
moment. All 29 shipped files parse to the byte.

## Layout

```
file header, 124 bytes
    int32   -1                       magic
    int32   5                        version
    int32   1 or 2                   section count
    int32   1
    int32   keyframes in section 0
    ...                              time-of-day settings, see below
keyframe x count
for each further section:
    72 bytes                         a copy of file header bytes 52..123
    keyframes, to the end of the file
```

and one keyframe is

```
88 bytes    22 four-byte slots: BGRA colours, three of them float32
6 x string  the object's name in one slot, empty in the other five
float32[4]  intensities
int32 n
n x string  sound files
uint32[10]  a kind word, then the hour and the minute
```

A **string** is an `int32` length followed by that many bytes with no
terminator, which is what `MFile`'s string reader does — `Terrain.dll`'s
deserialiser is an unoptimised run of `fread(ptr, 4, 1, file)` calls, so the
field order reads straight off the disassembly.

## The kind word and the time

The keyframe's trailer opens with a kind word: **3** on 621 of the 656 shipped
keyframes, **1** on 6, and **0** on the 29 that close a section. The hour and
minute follow it — one word later when the kind is 3. Reading it that way
gives a valid time on every keyframe and leaves all 29 first sections sorted
by time; reading a fixed offset breaks on 18 of them.

`CAMPAIGN.04/Mission.01` is the clearest file: twelve keyframes at 00:20,
01:24, 06:48, 12:34, 13:58, 15:00, 15:48, 19:20, 22:39, 23:20, 23:59, 00:00.

## What the numbers are

The **third of the four float32** is the light. Across the 27 files whose
value varies at all, its low point falls within two hours of midnight; on
`CAMPAIGN.04/Mission.01` it runs 0.1 at 00:20 up to 5.0 at 12:34 and back
down. The other two are 2.2 and 2.0 almost everywhere.

The 88-byte block is **three groups of four colours** plus singles, stored
BGRA — the same DirectDraw convention as the textures, and confirmed by the
float slots decoding correctly little-endian. Slots 1–4 are the group that
tracks the day, going near-black at midnight. Reading them that way produces
coherent, art-directed skies, which is the real check:

| mission | brightest keyframe | slot 1 | slot 7 | slot 18 |
|---|---|---|---|---|
| CAMPAIGN.00 | 11:00 | `#afa5dc` | `#d7d7ff` | `#f0f5ff` |
| CAMPAIGN.01 / Mission.01 | 20:00 | `#962800` | `#ff6312` | `#821800` |
| CAMPAIGN.02 | 10:10 | `#7bab0f` | `#ebff77` | `#ffff00` |

— a pale violet daylight, a red sunset, and a toxic green world with a yellow
sun. The viewer draws slot 1 as the zenith, slot 7 as the horizon and fog, and
slot 18 as the sunlight, and scales the directional light by the third float.
**Which group is which is an interpretation**, not a proved fact: what is
proved is that the first group tracks the day cycle and that the second is
consistently the lighter of the two.

## Object types

`Terrain.dll`'s factory is a five-way switch — **SUN, SKY, RAIN, SNOW,
LIGHTNING** — allocating classes of 0x9e8, 0x560, 0xc0, 0xb8 bytes and one
more. A keyframe's name slot carries `sun`, `moon` or `env_lightning`, and the
counted string list carries sound files (`atm_rain1.wav` is the only one
shipped). Which field selects the type is **not established**: the shipped
keyframes are near-identical within a file, and the two record shapes that do
differ (a float where another has a colour, at slot 11) are not distinguished
by any word that has been located.

## The sibling `sky.wea` — the slot index is the role

Plain text in the same format model wears use, and **the position in the list
is what the entry means**. All 29 missions fill the same nine slots in the
same order:

| slot | role | example | constant across missions |
|---|---|---|---|
| 0 | nebula | `ENV_NEBULA_0` | four variants |
| 1 | stars | `ENV_STARS` | **yes** |
| 2 | clouds | `ENV_CLOUDS` | four variants |
| 3 | sun | `ENV_SUN_3` | five variants |
| 4 | moon | `ENV_MOON` | four variants |
| 5 | lens flare | `ENV_FLARE_00` | **yes** |
| 6 | lens flare | `ENV_FLARE_01` | **yes** |
| 7 | snow | `SNOWFLAKE` | two variants |
| 8 | rain | `RAIN_DROP` | **yes** |

Not one of those names is in `Textures.lib`. They are **material** names, and
they go through `Material.lib` exactly as the terrain's layers do — all
**261 slot names across the 29 missions** resolve that way.

## A material picks a sub-image as well as a texture

The byte immediately before a material entry's texture name is `0xFF` on 427
of the 905 materials and a small number on the rest. `0xFF` means *the whole
texture*; anything else is a cell of a sprite sheet, row-major over a 2 x 2
grid.

The sky is where this is unmistakable. `SUN.0` is one 256-pixel image holding
a sun corona in its top-left quarter and a rocky moon in its bottom-left, and
**`ENV_SUN` asks for cell 0 while `ENV_MOON` asks for cell 2** — which is
exactly where each one is. `SUN1.0` holds four bodies, three stars and a moon;
its three `ENV_SUN_*` materials name cells 0, 1 and 3, the three stars, and
leave cell 2, the moon, alone. `SUN3.0` holds a star and three planets, and
its three `ENV_MOON_*` materials name cells 1, 2 and 3 — the planets.

Two do not fit: `ENV_SUN_2` and `ENV_MOON_5` both name `SUN4.0` with cells 4
and 5, which are past the end of a 2 x 2 grid, and `SUN4.0`'s two sprites sit
in the cells a 2 x 2 would number 1 and 3. The reader falls back to the whole
texture for an index it cannot place rather than guess.

## The lens flare

`CSun::RenderFlare` is short and reads straight through. It strings **twelve
sprites along the line from the sun's position on screen through the centre of
the screen**, taking each one's place, size, colour and texture from four
parallel tables in `Terrain.dll`'s data:

| # | position | size | colour | texture |
|---:|---:|---:|---|---:|
| 0 | 1.2 | 0.2 | `#FFB090A3` | 0 |
| 1 | 0.7 | 0.3 | `#FF5A58BB` | 0 |
| 2 | 0.5 | 0.2 | `#9630BE52` | 1 |
| 3 | 0.2 | 0.1 | `#96C93432` | 1 |
| 4 | 0.0 | 0.1 | `#FF30BE52` | 0 |
| 5 | −0.2 | 0.3 | `#96969664` | 1 |
| 6 | −0.3 | 0.3 | `#FFB090A3` | 0 |
| 7 | −0.5 | 0.7 | `#FF7C6BC9` | 0 |
| 8 | −0.6 | 0.4 | `#96306452` | 1 |
| 9 | −0.8 | 1.0 | `#FF0B17B9` | 0 |
| 10 | −1.0 | 0.3 | `#FFB626B1` | 0 |
| 11 | −1.1 | 0.2 | `#FF7CC5C9` | 0 |

Position 1 is the sun itself and 0 the middle of the screen, so the chain
starts just past the sun and runs out the far side. A ghost's half-size is
`0.25 * (viewport width / 2) * size`, which makes the largest of them an
eighth of the screen across. The colours are `D3DCOLOR` constants; the engine
scales **only their alpha** by the flare's intensity and leaves the RGB alone,
which is visible in the helper that does it — it shifts the top byte out,
multiplies, and ORs the other three back unchanged. The texture column picks
between the two flare slots of `sky.wea`; which of the pair the engine calls 0
is not established.

Intensity has two gates, and both are now exact. The first: the flare is off
once the sun is more than **15°** off the view axis, ramps linearly to full
on-axis, and the ramp is then squared. The second ramps on **how high the body
stands** — see [below](#where-the-sun-stands-and-it-is-not-in-a-file). Both
sets of cosines are cached at load from constants of 15, 30 and 60 degrees.

The whole flare is skipped when the first gate falls below **0.1**, and the
composite intensity also brightens the sun's own billboard: the engine lerps
each of its three colour channels from `c` to `5c`, so a sun on the view axis
draws up to five times its own colour.

## The weather, and how the engine reads a keyframe

A keyframe carries names, and the names say which atmosphere object it acts
on. Four appear across the 656 shipped keyframes: **`sun`** (73) and
**`moon`** (65), which come in start/stop pairs, and **`atm_rain1.wav`** (9)
and **`env_lightning`** (9), which appear once in a section.

That is the weather switch. **14 of the 29 missions carry a marker — eight
name rain and eight name lightning — and not one names snow.** The rain
markers are all early morning: 02:40, 05:10, 05:40 and 07:00.

`Terrain.dll` says what the engine does with them.
`CAtmosphere::HandleEvents` walks a list of 20-byte events built by
`CAtmData::GetEvents`, and each event's second word is the object type it
passes to `CreateAtmosphereObject` — a five-way switch, allocating 0x9e8,
0x560, 0xc0, 0xb8 and one more. `GetEvents` itself dispatches on a ten-valued
opcode at the head of its keyframe record, and the ten branches pair up
exactly:

| opcode | object type | action |
|---:|---:|---|
| 0, 1 | 0 | start, stop — and branch 0 compares the name to `"sun"` |
| 2, 7 | — | nothing |
| 3, 4 | 2 | start, stop |
| 5, 6 | 3 | start, stop |
| 8, 9 | 4 | start, stop |

Even starts, odd stops. Object type **1** is never created from an event, and
the reason is visible one call site up: the sky is created directly, with a
hardcoded `1`, which is why every mission has one and no keyframe has to ask.

Which field of the *file* feeds that opcode is not established — the trailer
word this reader calls `kind` is 3 on 621 of the 656 keyframes, which does not
fit. So where a shower **stops** is unknown: the sun and moon come in pairs
and rain does not.

## What the viewer draws

- The **nebula** on the dome, multiplied by the keyframe's zenith-to-horizon
  gradient, so one draw gives "this sky at this hour".
- The **stars** over it, additive, fading in as the day's light drops.
- The **clouds** over that, tiled four times and tinted by the horizon colour.
- The **sun** and **moon** as billboards, each at its own fixed place, and
  only whichever one the keyframes say is up.
- **Rain**, when the mission's keyframes ask for it: the drops are its own
  `RAIN_DROP` sprite, cell 21 of `EFFECT6.0`, falling in a 900-unit box that
  rides with the camera.
- The **lens flare**, as a 2D overlay drawn after the scene — it is in the
  lens, not the world, so it takes no depth test. The twelve elements and
  their tables are the engine's, and so is the second gate — full for the sun,
  0.39 for the moon, nothing when neither is up. Only the first gate is
  adapted: its 15° cone is calibrated to the game's field of view and would
  almost never open against an orbiting camera that looks down at the terrain,
  so the same linear-then-squared ramp is driven by the sun's distance from
  the centre of the screen instead.

The **sun and moon stand still**, each at its own fixed place, and only one of
them is ever up — see the next section. The time-of-day control walks the
keyframes, and the scene's light points at whichever body is in the sky so the
shading and the sky agree.

## Where the sun stands, and it is not in a file

It was never going to be found in `sky.ske`, because it is not in any file:
**`CSun`'s two angles are constants in `Terrain.dll`**, and the only thing the
mission chooses is *when* the sun is up.

The chain is short. `CAtmData::GetEvents` walks the keyframes; on the
start opcode it compares the keyframe's name against the literal `"sun"` and
fills a four-`int32` block in the DLL's own data from that one test:

| field | `name == "sun"` | anything else | what `CSun` does with it |
|---|---:|---:|---|
| +0 | the sun's lifetime | the moon's | `× 1000`, kept as milliseconds |
| +4 | 90 | 0 | × π/180 → `this+0x30`, an azimuth |
| +8 | 30 | 50 | × π/180 → `this+0x34`, a tilt from the zenith |
| +C | 3 | 4 | the `sky.wea` slot to draw with |

The event record carries a pointer to that block at `+0x10`, and
`CreateAtmosphereObject` hands it to the constructor. Three things fall out.

**The fourth field is the slot table.** 3 and 4 are `sun` and `moon` in
`SLOT_ROLES` — read out of `sky.wea` quite separately, from the nine slots all
29 missions fill in the same order. The engine and the data agree on the
index without either having been derived from the other.

**The two angles are an azimuth and a tilt.** `CSun::Render` builds
`Rodrigues(axis = (cos A, sin A, 0), B)` and multiplies it by a rotation of
`A` about the vertical, which is `Rz(A) · Rx(B)`; the body's direction is that
matrix's third column, `(sin A sin B, −cos A sin B, cos B)`. So:

| | azimuth | tilt | direction (game axes, z up) | above the horizon |
|---|---:|---:|---|---:|
| sun | 90° | 30° | (0.5, 0, 0.866) | **60°** |
| moon | 0° | 50° | (0, −0.766, 0.643) | **40°** |

**And that is where the flare's second gate comes from.** It ramps on the
height of that same direction — the vector at `this+0x78`, whose third
component is the `+0x80` the gate negates and compares — between
`cos 60° = 0.5` and `cos 30° = 0.866`. The sun's height *is* `cos 30°`, to
the last bit, so the sun sits exactly on the top edge of the ramp and always
flares at full; the moon's 0.643 gives 0.390. The two gate constants were
chosen to bracket the two bodies, which is what identifies what the gate
measures.

Nothing ever rewrites the two angles: `Render` rebuilds the same matrix from
them every frame. The sun does not travel.

What the mission does choose is **when**, and the data backs the reading. A
keyframe naming a body toggles it, the engine's opcodes running even to start
and odd to stop, and **32 of the 35 shipped sections hold exactly one pair of
each** — the sun up from about 01:30 to 15:00, the moon from 16:20 to
midnight. **No section has them up at once**, on any of the 29 missions, which
is what makes two fixed positions only a quarter turn apart coherent: they are
never in the sky together. The three exceptions are two five-keyframe skies
that name the sun on every keyframe and never the moon, and one that names
each body once and stops neither.

## Not resolved

- **The sun's lifetime.** The block's first field is a duration in seconds
  that `GetEvents` computes by mapping the start and stop keyframes' clock
  times through a per-section scale and taking the difference, wrapping round
  the section. The scale comes from a virtual call whose meaning is not
  pinned down, so the number is not reproduced here — only what it is for.
- **Snow.** No shipped mission names it, so there is nothing to switch on.
  `SNOWFLAKE` resolves — it is cell 20 of `EFFECT6.0`, a 16 x 16 icon — and
  the viewer would draw it the same way it draws rain if a mission asked.
- **Where a shower stops.** The sun and moon come in start/stop pairs and rain
  does not, so the viewer runs the weather to the next keyframe that names
  anything. That is a reading, not a fact.
- **The keyframe count of a second section.** Six files have two; their
  72-byte section headers are byte-identical yet hold 27 and 20 keyframes, so
  the count is not in them. The reader takes the second section's keyframes to
  the end of the file, which consumes all six exactly.
- Which field selects the object type.
- The rest of the 124-byte file header. It holds `23, 59` where a time would
  go, `6939832` twice, and a couple of small counts.
- Slots 0, 5, 15–17, 19–21 of the colour block, and the trailer past the time.
