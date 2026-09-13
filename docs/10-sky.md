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

Bytes **64 and 68** of the header are two `uint32` holding **how long one
in-game day lasts in real time**, as hours then minutes. They sit inside the
72-byte block repeated ahead of every further section, so each section
declares its own. 21 of the 29 missions run a day in a quarter of an hour:

| hours, minutes | seconds | files |
|---|---|---|
| 0, 15 | 900 | 21 |
| 0, 40 | 2400 | 3 |
| 0, 20 | 1200 | 2 |
| 0, 9 | 540 | 1 |
| 24, 0 | 86400 | 2 |

The engine keeps `hours * 3600 + minutes * 60` per entry and maps a keyframe's
24-hour clock stamp onto it linearly — `clock_seconds * that / 86400` — so on
a 15-minute day noon falls 450 seconds in. `CAtmosphere::CAtmosphere` runs the
whole span through `CAtmData::GetTimeDiffInSec` and keeps the answer in
milliseconds.

The two files declaring a full **24 hours** are the giveaway, and they are the
check: a sky that keeps real time never visibly moves, and those two carry
**5 keyframes against a minimum of 12** everywhere else. They have nothing to
animate. The chain is `CAtmosphere::CAtmosphere` (`Terrain.dll:0x1006ec30`) ->
`0x1006fab0` -> `CAtmData` slot 3 (`0x1006a5d0`), which returns an array the
constructor fills from the reader's `0x100695b0`. That the array's source is
*this* pair of header words is the reading rather than a traced byte: it is
the only varying time-shaped pair in the header, it is inside the repeated
section block, and it sorts the static skies out exactly.

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
shipped).

**The five are numbered**, which an earlier draft of this section did not
know. `CAtmosphere`'s event handler at `0x1006fbb0` dispatches a type through
a jump table at `0x10070024`, and each case copies that type's name into a
buffer for its log line:

| | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| | `SUN` | `SKY` | `RAIN` | `SNOW` | `LIGHTNING` |

### The ten opcodes, decoded

`CAtmData::GetEvents` dispatches on a ten-valued opcode, and each of its cases
**writes** a phase and a type into a 20-byte event record — `{phase, type,
two words of time, one more}`. Read off those writes rather than guessed, the
table is:

| opcode | | opcode | | opcode | |
|---:|---|---:|---|---:|---|
| 0 | start `SUN` | 1 | stop `SUN` | 2 | *nothing* |
| 3 | start `RAIN` | 4 | stop `RAIN` | | |
| 5 | start `SNOW` | 6 | stop `SNOW` | 7 | *nothing* |
| 8 | start `LIGHTNING` | 9 | stop `LIGHTNING` | | |

Two things fall out of it. **2 and 7 share the switch's default and do
nothing** — which is what "cases 2 and 7 share the out-of-range target" in the
earlier note actually meant. And **`SKY` has no case at all**, which agrees
with the separate finding that the sky is created outside the switch with a
hardcoded id: a sky is never started or stopped because it is always there.

The pairing is not the arithmetic one. `type * 2 + phase` would put `RAIN` at
4 and 5; the cases say 3 and 4. The gaps at 2 and 7 are where the enum's
author left room, and the only way to get the table right is to read what each
case stores.

Phase **0 is the create side**: the handler that takes it resolves the type
name, looks the object up, and warns *"Atmosphere object already exists - %s"*.

### Which field carries the opcode — still open, and one candidate is dead

The opcode is assembled in memory. The collector fills a 0x98-byte record from
a **240-byte runtime keyframe** (the filler is at `0x100692d0`): hour at
`+0x14`, minute at `+0x18`, the opcode at `+0x28`, and six `c_str()` pointers
at `+0x64`…`+0x78` — the file's six name slots, so the runtime keyframe is the
file's keyframe expanded.

The trailer's last word is the only field in the file that spans 0 to 9, and
it is **ruled out**, for a sharper reason than the first attempt had. Now that
7 is known to mean *nothing happens*, the objection is exact: that word puts
**119 of the 140 named keyframes** — all 59 `moon` and 60 of the 75 `sun` — on
a do-nothing case. Bodies that must rise and set cannot all be no-ops. Six
keyframes, the last of six files, carry an uninitialised `6939832` there as
well.

So the type vocabulary is closed and the field that selects it is not.

Two candidates were tested and both failed, which is worth writing down so
they are not tried again. The first is the trailer word above. The second was
the **second section**: the collector takes a section number, six files carry
a second section, and it looked like the event list to the day cycle's
colours. It is not. Both sections of all six run **00h to 24h**, and in five of
them the second reuses **16 of the first's 17 distinct colour blocks** at
different times with a flatter light curve, and carries `sun` and `moon` names
of its own. It is a *second complete day cycle* — a weather variant of the
same day is the obvious reading and a **guess**; what is measured is that both
are whole cycles over the same span. `Single.01`'s pair share only 5 blocks,
so the variant can be a wholly different day. What selects between them is
open.

That also answers, in passing, the smaller unknown recorded here as "the
keyframe count of a second section": the count is not the question, because
the section is not a tail of the first.

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
`CAtmData::GetEvents` (`0x1006dc10`), and each event's second word is the
object type it passes to `CreateAtmosphereObject` — a five-way switch,
allocating 0x9e8, 0x560, 0xc0, 0xb8 and one more. `GetEvents` dispatches on a
ten-valued opcode at the head of its keyframe record through a jump table at
`0x1006e829`, and the ten branches pair up exactly — cases 2 and 7 share the
*same* target as the out-of-range default, so they do nothing at all:

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

### The opcode is not a field of the file

`GetEvents` gets its records from `0x1006d740`, which splits the query at
midnight and calls `0x1006d460` once or twice. That is the collector: it asks
the atmosphere data object for a section's keyframe count and then for each
keyframe in turn, receiving a **0x98-byte record** whose layout is the
engine's, not the file's — `+0x00` the opcode, `+0x20` the hour, `+0x24` the
minute, `+0x64` a **pointer** to the name that branch 0 compares against
`"sun"`.

So the opcode is assembled in memory, which is why looking for it in the file
comes up empty, and the data says the same. None of the 22 four-byte slots
carries a value in 0..9 on all 656 keyframes — slot 5 is the only small one
and it is 0 throughout. The trailer's last word does span 0..9, but it puts
**438 of the 656** keyframes on case 7, the no-op, and among them 60 named
`sun` and 59 named `moon` — bodies that must start and stop. It is not the
opcode.

**So where a shower stops is still unknown**, and the reason is now clear:
nothing in `sky.ske` says. The sun and moon come in start/stop pairs of
*named* keyframes; rain and lightning appear once in a section.

### How a keyframe's clock becomes an event time

The same collector shows the conversion the sun's lifetime needed:

```
t = (hour * 3600 + minute * 60) * scale / 86400
```

— the seconds since midnight, scaled by a per-section value the collector
fetches through the data object's vtable and divided by a day. What that scale
*is* is the last piece: it comes from a virtual call that has not been
followed, and it is what turns two clock times into a duration.

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

- **The sun's lifetime** is now computable. `GetEvents` maps the start and
  stop keyframes' clock times through the scale and takes the difference,
  wrapping round the section — and the scale is the declared day length
  above, so the duration is `(stop - start) * day_seconds / 86400`. What is
  left is only that no shipped file has been walked end to end against a
  running game to confirm the wrap.
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
- Which field selects the object type, and which carries the opcode. The
  runtime side is pinned: the filler at `Terrain.dll:0x100692d0` copies the
  240-byte keyframe's `+0x28` to the event record's `+0x00` (at `0x100694bd`),
  with the hour at `+0x14` and the minute at `+0x18`. Two file candidates are
  dead. The trailer's last word spans 0..9 but puts 119 of the 140 named
  keyframes on a do-nothing case; and so does **the word a contiguous copy
  would predict** — five dwords past the trailer's time, which is a *shifting*
  index because the time sits at 4 when the kind word is 3 and at 3 otherwise.
  That second test exists because the first used a fixed index and would have
  missed a moved field. It did not move. The copy is not contiguous anyway:
  the filler swaps `+0x34`/`+0x30` into `+0x04`/`+0x08` and `+0x3c`/`+0x38`
  into `+0x0c`/`+0x10`, so the file-to-memory order has to be read off the
  deserialiser rather than predicted.
- The rest of the 124-byte file header. Bytes 64 and 68 are the day length,
  above. It still holds `23, 59` where a time would go — the same
  hours-and-minutes shape, constant on all 29 — `6939832` twice, and a couple
  of small counts.
- Slots 0, 5, 15–17, 19–21 of the colour block, and the trailer past the time.
