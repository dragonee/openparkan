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

What it holds is one or two **day cycles**: lists of keyframes, each stamped
with an hour and a minute, carrying the colours and intensities the sky takes
at that moment and the weather event that fires there. All 29 shipped files
parse to the byte.

## Layout — *read*

The deserialiser is an unoptimised run of `MFile` reads, so the order reads
straight off it: the file at `Terrain.dll:0x100672d0`, a section at
`0x100660c0`, a keyframe at `0x10066230`, and the 32-byte time they all use
at `0x10086570`.

```
int32   -1                          magic
int32   5                           version
int32   section count, 1 or 2
for each section:
    int32   1                       section version
    int32   keyframe count
    time    23:59                   read, never asked for
    time    the day length          hours and minutes of real time
    keyframe x count
time    the clock's start           section, hour and minute
int32   0                           read, never asked for
int32   0 or 1                      the sky's sixth parameter, never read
```

and one keyframe is

```
int32   3                           keyframe version
time    the keyframe's clock stamp
int32   the event opcode            0..9
88 bytes                            22 slots: BGRA colours, two float32
6 x string                          the object's name in the first
float32[4]                          the sun's two extents, the light, the weather
int32 n, n x string                 up to four effect names
```

A **time** is six `uint32` and eight more bytes. Three of its fields are ever
asked for: the section at `+0`, the hour at `+0xc` and the minute at `+0x10`.
A **string** is an `int32` length followed by that many bytes with no
terminator.

The keyframe reader also takes **version 2**, which has no effect list, and
**version 1**, which stores twenty slots and makes slots 20 and 21 from slot
19 at 0.3 a channel; both print *"Warning! Atmosphere file version is not up
to date"*. Every shipped keyframe is version 3 (*measured*, 656 of 656).

After loading, `0x10067500` bubble-sorts each section by `hour * 60 +
minute`. All 35 shipped sections are stored in order already (*measured*).

### An earlier reading was one keyframe out

This page used to describe a 124-byte file header, a 72-byte block "copied"
ahead of a second section, and a ten-word *trailer* after each keyframe whose
first word was a "kind" of 3, 1 or 0. That reading consumed every file to the
byte, which is why it survived, and it was wrong in one way that mattered:
**each keyframe was stamped with the time and opcode of the keyframe after
it.** The 40 bytes ahead of a keyframe's slots are its own version, time and
opcode; the old reader filed them under the keyframe before.

The other fields of the old reading fall out of the same shift:

| old reading | what it is |
|---|---|
| "kind word" 3, on 621 keyframes | the next keyframe's version |
| "kind word" 1, on 6 | the second section's version, 1 |
| "kind word" 0, on 29 | the section field of the file's closing time |
| the 72-byte "copy of the header" | the second section's day-length time and its first keyframe's preamble |
| "a second section's count is in neither header" | it is in its own header: 27 or 20 |

Every time-of-day claim the old reading supported moved by one keyframe, and
they are restated below. The colours and intensities belonged to the right
keyframes all along.

## How long a day lasts — *read*, and *measured*

The second time in each section header is **how long that section's day
lasts in real time**. For the first section its hour and minute sit at file
bytes **64 and 68**. 21 of the 29 missions run a day in a quarter of an hour:

| hours, minutes | seconds | files |
|---|---|---|
| 0, 15 | 900 | 21 |
| 0, 40 | 2400 | 3 |
| 0, 20 | 1200 | 2 |
| 0, 9 | 540 | 1 |
| 24, 0 | 86400 | 2 |

The chain is now traced to the byte. The section reader keeps the time at
section `+0x40` (`0x10066111`); the reader interface's slot 4 (`0x100695b0`)
returns its hour and minute from `+0x4c` and `+0x50`; `CAtmData::CAtmData`
(`0x1006a070`) keeps `hours * 3600 + minutes * 60` per section and hands the
array out as its slot 3 (`0x1006a5d0`).

A keyframe's 24-hour stamp is mapped onto its section's day linearly, in
integers — `clock_seconds * day_seconds / 86400` (`0x1006d460`) — so on a
15-minute day noon falls 450 seconds in.

The two files declaring a full **24 hours** are the check: a sky that keeps
real time never visibly moves, and those two carry **5 keyframes against a
minimum of 12** everywhere else.

## The clock: sections in turn, and where it starts — *read*

### A file's sections are played one after another

Six files carry a second section, and nothing chooses between them: **the
atmosphere plays section 0's day, then section 1's, then section 0's again.**

- `CAtmosphere::CAtmosphere` sums every section's day length into one cycle,
  in milliseconds, at `+0x150` (`0x1006efcd`).
- A position on the clock is a pair, *(section, seconds into its day)*.
  `CAtmData::GetTimeDiffInSec` (`0x1006a850`) measures from one position to
  another forward only: out of the first section, through every section
  between, into the second, wrapping round the whole cycle.
- Each takt (`CAtmosphere::SendMsg`, message 6 subcode 1, `0x10070040`) takes
  the time since the atmosphere's epoch modulo the whole cycle, and walks it
  through the sections from section 0.

So a two-section file is a **two-day cycle**. Both sections of all six run
00h to 24h, and in five of them the second reuses 16 of the first's 17
distinct colour blocks at different times (*measured*). The difference is
the weather: on five files the second day rains from 04:40 to 09:00.

### Where the clock starts

The time that closes the file is the start. `0x1006fab0` asks `CAtmData`
(slot 7, `0x1006ec00`) for it, which asks the reader (slot 5, `0x10069630`)
for its section, hour and minute. It scales the stamp by that section's day
length and measures from *(0, 0)* to it; `CAtmosphere::CAtmosphere` keeps the
answer × 1000 at `+0x14c`. The component's slot 6 (`0x10070330`), given the
game time, sets the epoch that far back, and resets the last event position
to *(0, 0)*.

*Measured*, all 29 files start in section 0:

| start | files |
|---|---:|
| 01:30 | 10 |
| 00:30 | 6 |
| 01:00 | 6 |
| 01:20 | 5 |
| 09:55, 09:45 | 1 each, the two 24-hour skies |

Every one of them is inside the sun's window, so **every mission opens with
the sun up**. Mission 01 starts at 01:30 of a 900-second day: 56 seconds in.

Because the last event position starts at *(0, 0)*, the first takt fires every
event stamped before the start — the sun's start at 00:30 included — so the
bodies and weather due by then exist as the mission begins.

Two other ways to move the clock are in the code and neither is traced
further: `CAtmosphere::SetObjectState` (`0x10070520`) writes `+0x14c` from a
four-byte state record, and the interface at `+0x130` has a slot 9
(`0x100705a0`) that jumps the clock to a given *(section, seconds)* and runs a
takt. Who calls either is not established.

## Events — *read*, and *measured*

### The ten opcodes

`Terrain.dll`'s factory is a five-way switch — **SUN, SKY, RAIN, SNOW,
LIGHTNING** — allocating classes of 0x9e8, 0x560, 0xc0, 0xb8 bytes and one
more. `CAtmosphere`'s event handler at `0x1006fbb0` dispatches a type through
a jump table at `0x10070024`, and each case copies that type's name into a
buffer for its log line:

| | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| | `SUN` | `SKY` | `RAIN` | `SNOW` | `LIGHTNING` |

`CAtmData::GetEvents` (`0x1006dc10`) dispatches on a ten-valued opcode through
the jump table at `0x1006e829`, and each case **writes** a phase and a type
into a 20-byte event record — `{phase, type, two words of position, a pointer
to a parameter block}`:

| opcode | | opcode | | opcode | |
|---:|---|---:|---|---:|---|
| 0 | start `SUN` | 1 | stop `SUN` | 2 | *nothing* |
| 3 | start `RAIN` | 4 | stop `RAIN` | | |
| 5 | start `SNOW` | 6 | stop `SNOW` | 7 | *nothing* |
| 8 | start `LIGHTNING` | 9 | stop `LIGHTNING` | | |

**2 and 7 share the switch's default and do nothing**, and **`SKY` has no
case**: the sky is created directly by `CAtmosphere`, with a hardcoded `1`,
so every mission has one and no keyframe has to ask. Phase **0 is the create
side**: the handler that takes it resolves the type name, looks the object
up, and warns *"Atmosphere object already exists - %s"*.

What each start case hands its object:

| start | parameter block | from the keyframe |
|---|---|---|
| `SUN` | lifetime, azimuth, tilt, `sky.wea` slot | the name, compared with `"sun"` |
| `RAIN` | **8** and a sound name | the first effect name, or *"Rain background sound not specified"* |
| `SNOW` | **7** | nothing |
| `LIGHTNING` | an effect name | the first effect name, or *"Lightning effect not specified"* |

The two constants are *read*; that they are `sky.wea` slots is *derived*:
they are the rain and snow entries of `SLOT_ROLES`, as the sun's 3 and 4 are
its sun and moon.

### The opcode is the word ahead of slot 0

The collector (`0x1006d460`) fills a 0x98-byte event record from each
240-byte runtime keyframe (the filler is `0x100692d0`); the runtime keyframe
is the file's keyframe as the reader lays it out, time at `+0x8`, the
opcode word at `+0x28`, slot 0 at `+0x2c`. The filler copies `+0x28` to the
record's `+0x00` (`0x100694bd`), and that is the word `GetEvents` switches on.

Three file candidates were once tested and all three failed — the trailer's
last word, the word five past the trailer's time, and the second section. The
first two are the same word on every version-3 keyframe, and it **was** the
opcode: it was being tested against the keyframe before its own. Read against the right keyframe (*measured*):

- the sun and the moon are named on 140 keyframes, and **138 carry opcode 0
  or 1**; the other two are the middle keyframes of the two 24-hour skies,
  which name the sun on all five keyframes. Control: the same word read the
  old way, off the next keyframe, gives 11;
- all **9 rain starts** name `atm_rain1.wav` first and all **8 lightning
  starts** name `env_lightning` first — the two things `GetEvents` refuses to
  run without;
- **every start is stopped later in its own section**: sun 37 of 37, moon 33
  of 33, rain 9 of 9, snow 9 of 9, lightning 8 of 8.

`env_lightning` is an effect name, not an object name: it sits in the counted
list with the rain sound, never in the six name strings.

### Where a shower stops, and snow

**A shower stops at its stop opcode** — 4 for rain, 6 for snow, 9 for
lightning — on a keyframe that names nothing, which is why a stop was never
found by name. *Measured*:

| weather | missions | spells |
|---|---:|---|
| rain | 8 | 04:40–09:00 on seven (the second day on five of them), and Single.01's 05:40–11:00 and 01:30–22:50 |
| snow | 7 | 00:00–23:59 on six, and three spells on `CAMPAIGN.05/Mission.01` |
| lightning | 8 | 00:01–23:58 on six, 05:10–08:30 and 02:40–21:10 |

**Snow is shipped.** An earlier note said no mission asks for it because no
keyframe *names* it; snow needs no name. The six missions that snow all day —
`CAMPAIGN.03`'s four, `Multi.01` and `Multi.03` — are exactly the six whose
`sky.wea` puts `DUST_ADD` rather than `SNOWFLAKE` in its snow slot: a dust
storm the length of the day, with lightning through it. `CAMPAIGN.05/Mission.01`
snows `SNOWFLAKE` three times.

The **fourth float** is what the weather runs at: rain and snow take it with a
colour made from slot 19, lightning takes it alone (`0x1006ce00`,
`0x1006cd81`, `0x1006cef8`). It is non-zero on 137 keyframes, all inside a
spell or on its stop, and 0 on all 495 outside one (*measured*).

### How a keyframe's clock becomes an event time

```
t = (hour * 3600 + minute * 60) * day_seconds / 86400
```

in integers, where `day_seconds` is **the keyframe's own section's declared
day length** (`CAtmData` slot 3, `0x1006a5d0`, called by the collector at
`0x1006d510`). That settles what an earlier version of this section called
an unfollowed virtual call.

A keyframe fires when the clock passes it: `GetEvents` takes the keyframes
with `from <= t < to` between the last position and the current one; a span
that leaves its section is split at the section's end and continued from the
start of the next (`0x1006d740`). A takt only asks when at least a second has
passed (`0x1007026c`). A keyframe stamped 24:00 has `t` equal to the day
length and never fires: 27 of the 29 carry opcode 7 anyway, and the other two
are the 24-hour skies' closing sun stops.

### A body's lifetime

On a `SUN` start (`0x1006dcb7`), `GetEvents` looks for the body's end
itself, testing each keyframe's record for opcode 1 (`0x1006de7b`): the
**first stop-`SUN` keyframe at or after the start**, in the start's section
and then the later ones, and before the end of the cycle — the last
section's full day. It **does not wrap back to section 0**, and a
stop stamped 24:00 lies on that end and does not count. The lifetime is the
forward distance between the two positions. When nothing is found the block
keeps whatever the previous search left, which happens on the two 24-hour
skies' second sun start.

Mission 01's sun is given 525 seconds (00:30 to 14:30 of a 900-second day)
and its moon 300 (15:30 to 23:30). What `CSun` does with the lifetime once
it has it (`× 1000` at `+0x2c`) is not traced.

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
| 7 | snow | `SNOWFLAKE`, `DUST_ADD` | two variants |
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

## What the numbers are

The 88-byte block is **colours stored BGRA** — the same DirectDraw convention
as the textures — with slots 5 and 6 read as floats. Every slot but two now
has a *read* destination; see
[the table below](#the-dome-the-fog-and-the-scene-colour--read-and-measured).
Reading them that way produces coherent, art-directed skies:

| mission | brightest keyframe | slot 1 | slot 7 | slot 18, the clouds |
|---|---|---|---|---|
| CAMPAIGN.00 | 07:00 | `#afa5dc` | `#d7d7ff` | `#f0f5ff` |
| CAMPAIGN.01 / Mission.01 | all five equal | `#962800` | `#ff6312` | `#821800` |
| CAMPAIGN.02 | 06:00 | `#7bab0f` | `#ebff77` | `#ffff00` |

— a pale violet daylight, a red sunset, and a toxic green world under yellow
cloud.

The four floats (*read*, `0x1006ac9a`, `0x1007dfdf`, `0x1006ce00`):

| float | what | shipped |
|---:|---|---|
| 1 | the sun sprite's extent across | 2.2 almost everywhere |
| 2 | the sun sprite's extent up | 2.0 almost everywhere |
| 3 | the light: the sun object's main light is slot 19 × this | 0.0 to 5.0 |
| 4 | the running weather's intensity | 0 unless a spell runs |

**The light fades out as a body sets.** On all 66 sun and moon windows with
keyframes inside, the third float at the start and at the stop is no higher
than anywhere between — 0.0 to 0.2, against up to 5.0 (*measured*; control,
read the old way, 0). On `CAMPAIGN.04/Mission.01` it runs 0.1 at the sun's
start at 00:20, 5.0 at 06:48, and 0.1 again at its stop at 13:58. Between a
body's stop and the next one's start it stands at 1.0 on many files, where
no body is up to use it.

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

The whole flare is skipped when the first gate falls below **0.1**. The same
gates also **brighten the sun's main light**, not its sprite, as this page
used to say: the colour handed to the light manager (`0x1007eb9e`) is
`lerp(c, 5c, gate1² × gate2 × slot 17's alpha / 255)` (`0x1007ea14`), so a
sun on the view axis lights the scene at up to five times its colour.

## Where the sun stands, and it is not in a file

It was never going to be found in `sky.ske`, because it is not in any file:
**`CSun`'s two angles are constants in `Terrain.dll`**, and the only thing the
mission chooses is *when* the sun is up.

On the start-`SUN` opcode `GetEvents` compares the keyframe's first name
against the literal `"sun"` and fills a four-`int32` block in the DLL's own
data from that one test:

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

What the mission does choose is **when**, and the data backs the reading.
**33 of the 35 shipped sections hold exactly one sun window and one moon
window**, opcode 0 to opcode 1 — the sun up from about 00:30 to 14:30, the
moon from 15:30 to 23:30. **No section has them up at once**, which is what
makes two fixed positions only a quarter turn apart coherent: they are never
in the sky together. The other two sections are the 24-hour skies, which run
the sun twice and never the moon.

## The dome, the fog and the scene colour — *read*, and *measured*

### How a keyframe reaches the sky

Each takt, the atmosphere works through three steps:

1. It interpolates the two keyframes around the clock for each object's type
   (`CAtmData`, `0x1006a970`). Colours are lerped channel by channel, floats
   linearly.
2. It hands the values to the object as numbered properties (`0x10070a20`).
3. The object stores them. Its property interface is the one at object `+4`
   (`QueryInterface` 7); the sky's setter is `0x1007bd70`, the sun's
   `0x1007ef10`. Their field offsets are relative to that interface, so the
   object's own are four more.

Following the file's slots through the reader (`0x10066230`), the record
filler (`0x100692d0`) and the sky case (`0x1006b2bc`) gives:

| property | file slots | what the sky does with it |
|---|---|---|
| 0–3 | 2, 3, 1, 4 | the **horizon**: ring 4 of the dome, and the fog colour |
| 4–7 | 7, 10, 8, 9 | ring 3 |
| 8–11 | 11, 14, 12, 13 | ring 2 |
| 12 | 15 | the apex and ring 1 |
| 13 | 5 (float) | fog start ÷ 700 |
| 14 | 6 (float) | fog end ÷ 700 |
| 15 | 18 | the **cloud layer's colour**, below |
| 16 | 20 | the **scene colour**, below |

Each group of four is a compass: property *k* belongs to the dome segment at
*k* × 90° from +y towards +x.

**Two slots go nowhere.** The record filler copies no field from the runtime
keyframe's `+0x2c`, so **slot 0** never leaves the reader; across the 656
keyframes it holds 18 distinct values that look like heap addresses
(`0x09ab0b57`, `0x0043d7e0`), editor memory saved by accident (*measured*).
**Slot 16** is copied to the record's `+0x50`, and no case of the
interpolation reads that field.

**The clouds' colour.** Only with `AtmCloudsOn` (`0x1007a4cb`), the sky puts
property 15's R, G and B ÷ 255 at `+0x308`..`+0x310` and zeroes
`+0x318`..`+0x320` (`0x1007a4de`–`0x1007a5b3`), then queues the cloud layer
with `+0x304` as its material block (`0x1007ab51`). The draw item keeps that
block at `+0x70`, and the item setup builds the Direct3D material from it
(`0x10030819`): its `+4` is the diffuse colour and its `+0x14` the material's
own emissive, to which the scene colour is added. So the clouds are slot 18
in diffuse, and the scene colour alone in emissive. *Measured*: on 24 of the 27 files whose light varies, slot
18 is at its brightest where the light is — `#f0f5ff` at Mission 01's
brightest keyframe, `#ac2800` as its sun rises. Control: slot 16, 6.

### What the sun does with its seven values

The sun case (`0x1006ac9a`) makes seven values, and `CSun`'s setter stores
them (`0x1007ef10`):

| property | value | `CSun` | used by |
|---:|---|---|---|
| 0–2 | slot 19's R, G, B ÷ 255 × the third float | `+0x84`..`+0x8c` | the main light's colour |
| 3 | the first float | `+0x94` | the sprite's extent across |
| 4 | the second float | `+0x98` | the sprite's extent up |
| 5 | slot 17 | `+0x95c` | its alpha scales the flare's boost of the main light |
| 6 | slot 21 | `+0x964` | the second light's colour |

**The sun object is two directional lights.** Its constructor asks the light
manager (`CreateLightManager`, `0x1007fa40`) for two lights of type 3 —
`D3DLIGHT_DIRECTIONAL` (slot 12, `0x100806b0`) — and flags the first
`0x8000000` and the second `0x10000000` (slot 13, `0x100807a0`). Every takt
(`0x1007d8b0`) it sets the first's colour to the first three values, lifted by
the flare gates as [above](#the-lens-flare), and the second's to slot 21
(slot 3, `0x10080040`). The emboss bump-mapping pass,
`CShade::EmbossBumpMap` (`0x1002ce40`), passes over a `0x10000000` light
(`0x1002cf87`); where else each is used is not traced.

**Their direction is not set by `CSun`.** It calls the light manager at slots
3, 6, 9, 12, 13 and 18 and never at the one that writes a position
(slot 4, `0x100800a0`). A type-3 light's direction is the three floats at
its record's `+0x24`, both where the Direct3D light is built (`0x10030ab3`)
and in the emboss pass (`0x1002d08d`), and no writer of that field has been
found.

The sun's on-screen half-extents are `float 1 × 0.1625 × camera slot 27`
across and `float 2 ×` the same up (`0x1007dfdf`), which the takt tests
against the screen's edges. When the shader's flag bit 0 is set, the sun
passes slot 17 through the shader's slot 5 first, and the sky does the same
to its fog colour (`0x1007d93d`, `0x10079b05`); the sky also sets its colour
mask to `0xff00ff00` in that mode, which reads as a green night-vision filter
(*guess*).

### The dome

The atmosphere builds the sky from a parameter block (`0x1006f1ba`): 10000,
π/4, 1.0, 16, 5, and a flag from the file's last `int32`. The sky copies the
six words to `+0x548` (`0x1007824a`) and reads four of them; the 16 is
replaced by `AtmSkyDetail`, and **nothing reads the flag** — the other
functions that touch offset `0x55c` in `Terrain.dll` belong to larger
objects and read `+0x560` and `+0x564` beside it.

**The shape** (`0x100787f0`) is a spherical cap.

| | value |
|---|---|
| height | *W* = 10000 |
| cap angle | *A* = π/4 |
| sphere radius | *R* = *W* ÷ (2 sin²(*A*/2)) = 34142.1 |
| rings | 5 |
| segments | 2 to the power of the `AtmSkyDetail` setting (`0x1007ac8f`); 16 at the default 4 |

The vertices:

- vertex 0 is the apex, at *z* = *W*;
- ring *r* of segment *j* sits at θ = *r*/5 · *A* and φ = *j* · 2π/segments,
  at (*R* sinθ sinφ, *R* sinθ cosφ, *R* cosθ + *W* − *R*);
- the rim, ring 5, is at *z* = 0 with a radius of 24142.

**Where it is drawn.** The dome is drawn at the camera's position
(`0x1007a17d`), so its rim lies at eye height.

**How it is queued.** The sky's layers go through the shader's slot 16
(`0x10028500`) into render layer 1, with the static render record at
`0x100a7138`, whose flags are 0 — so they take the scene's fog, not their
own. Bits 0 and 1 of one argument become the draw item's `ZENABLE` and
`ZWRITEENABLE` bytes (`+0x12c`, `+0x12d`, each set when its bit is clear,
`0x10028664`), which the item renderer sets as render states 7 and 14
(`0x100302fb`, `0x10030318`):

| draw | material block | matrix | `ZENABLE` | `ZWRITEENABLE` |
|---|---|---|---:|---:|
| `0x1007a37a` | `+0x484` | `0x100a7168` | 0 | 0 |
| `0x1007a408`, `0x1007a49e` | `+0x384`, `+0x404` | at the camera | 1 | 0 |

So the layers drawn at the camera are depth-tested and write no depth.
Neither says how a 34142-radius cap escapes the far plane, or a fog that ends
by 700.

**Its colours** (`0x1007ac60`):

- the apex and ring 1 take property 12;
- rings 2, 3 and 4 take the three compass groups, from the top down;
- within a ring, quadrant *k* runs from group[*k*] to group[*k* + 1], a
  segment at a time;
- the rim takes the fog colour every takt.

**The clouds** use the same cap with its origin 5000 below the camera
(`0x1007a08e`), and carry their own fog, from 5000 to 11380.7 (`0x1007a5d4`).

### Fog

**Fog is on for the whole scene.** `CShade`'s constructor sets `FOGENABLE`
(`0x10042070`); only the 2D overlays switch it off and back on.

**It is linear, range-based and starts at the eye.**

- The sky writes the scene's render record in two places:
  - fog vertex mode 3, `D3DFOG_LINEAR`, when it is built (`0x10078689`);
  - start 700 × property 13 and end 700 × property 14, every update
    (`0x1007bbc5`).
- Each drawn item then applies either its own record's mode, start and end,
  or the scene's (`0x10030620`). Applying a mode also sets `RANGEFOGENABLE`
  (`0x10030f20`).
- With slot 5 always 0, **fog starts at 0 and ends at 700 × slot 6**, which
  is 70 to 700 units (*measured*). On Mission 01 it ends at 420, 490, 525,
  560 or 700.

**It is Direct3D's vertex fog, as far as `Terrain.dll` goes.**

- The only fog states `Terrain.dll`, `World3D.dll` and `Ngi32.dll` set are
  `FOGENABLE`, `FOGCOLOR`, `FOGSTART`, `FOGEND`, `RANGEFOGENABLE` and
  `FOGVERTEXMODE`; none pushes `FOGTABLEMODE` ahead of a call.
- The item renderer submits untransformed vertices — FVF `0x1c2`, `0x2c2`,
  `0x112`, `0x212` and `0x252`, all `D3DFVF_XYZ` (`0x1002f1e0`–`0x1002f800`)
  — for which Direct3D computes the vertex fog itself. The one pre-transformed
  path, FVF `0x1c4` behind the item flag 8 (`0x1002f2c0`), is outside it.
- **`ForceSWFog` is never read.** Its value lands at `CSettings+4`, the first
  entry of the table at `0x100a6cac`; every one of the 41 indexed reads of
  that table in `Terrain.dll` resolves to another setting — `AtmCloudsOn`,
  `AtmSkyDetail` and `LensFlareOn` among them — and nothing reads the table's
  first entry directly. The settings interface is registered with
  `World3D.dll`'s `CreateGameSettings` object under id `0x1e` (`0x1005f5ab`),
  and its getter (slot 3, `0x1005f9c0`) could still hand the value to another
  module.

**Its colour follows the camera's heading** (`0x10079730`):

- the camera's slot 28 (`0x100850f0`) returns three angles of its matrix *m*:
  `atan2(m[4], m[0])`, `atan2(m[0], m[4])` and
  `atan2(m[8], √(m[0]² + m[4]²))` — the compass heading of the matrix's first
  column in two conventions, and that column's pitch;
- the sky takes the second, *b*, and forms degrees as (*b* + π) · 180 ÷ π
  (`0x10077b20` caches 1/π), then *k* = (⌊deg ÷ 90⌋ + 2) mod 4 and
  *f* = (deg mod 90) ÷ 90. The π and the 2 cancel: *k* is the quarter of *b*
  itself, counted from 0, and *f* the fraction through it;
- the colour `lerp(horizon[k], horizon[k+1], f)`, alpha forced to 255,
  becomes `FOGCOLOR` (`0x10079b28`) and the colour of every rim vertex.

So *b* = 0 gives property 0, and *b* grows from +y towards +x. The matrices
here are column-vector (the camera's translation is `m[3]`, `m[7]`, `m[11]`,
`0x1007a17d`), so the first column is one of the camera's own axes in world
space. That it is the axis the camera looks along — which would make the fog
exactly the horizon in the direction you look — is a *guess*: it is the only
axis the angle getter describes.

**Blended geometry fogs to a neutral colour.** For the duration of a draw,
the fog colour is swapped by blend mode (`0x1002ffea`; tables `0x1009a9c8`
and `0x1009a9d0`):

| blend mode | fog colour |
|---|---|
| additive (mode 2) | black |
| mode 3 (`ZERO`/`SRCCOLOR`) | white |
| mode 5 | grey `0x7f7f7f` |

Otherwise glows would pick up fog colour rather than fade out.

### The scene colour is added to every material

The same record carries a colour: property 16, file slot 20
(`0x1007bbc5`). Every drawn material gets **emissive = that colour + the
material's own emissive**, and an ambient term of 0 (`0x100308b8`). It is the
scene's ambient light in all but name — 40/255 grey at Mission 01's brightest
keyframe, a brighter violet at night.

### The render settings

`Terrain.dll` reads 36 settings from `shade.cfg` (`0x1005f652`). No
`shade.cfg` ships (*measured*), so the compiled defaults apply
(`0x1005fa80`):

| setting | default |
|---|---|
| `ForceSWFog` | 1, never read |
| `LightingOn` | 1 |
| `AtmCloudsOn` | 1 |
| `AtmStarsOn` | 1, never read in `Terrain.dll` either |
| `AtmSkyDetail` | 4 |
| `LensFlareOn` | 1 |
| `UseDXLighting` | 0 |

## What the viewer draws

- The **nebula** on the dome, multiplied by a gradient from the keyframe's
  apex (slot 15) to its horizon at heading zero (slot 2), so one draw gives
  "this sky at this hour". The game's dome carries four compass horizons and
  two rings between; the viewer keeps one of each.
- The **stars** over it, additive, fading in as the day's light drops.
- The **clouds** over that, tiled four times and tinted by their own colour,
  slot 18.
- The **sun** and **moon** as billboards, each at its own fixed place, and
  only while the keyframes' opcodes have it up.
- **Rain** while a rain spell runs: the drops are its own `RAIN_DROP` sprite,
  cell 21 of `EFFECT6.0`, falling in a 900-unit box that rides with the
  camera. Snow is marked in the payload and not drawn; lightning is neither.
- The **lens flare**, as a 2D overlay drawn after the scene — it is in the
  lens, not the world, so it takes no depth test. The twelve elements and
  their tables are the engine's, and so is the second gate — full for the sun,
  0.39 for the moon, nothing when neither is up. Only the first gate is
  adapted: its 15° cone is calibrated to the game's field of view and would
  almost never open against an orbiting camera that looks down at the terrain,
  so the same linear-then-squared ramp is driven by the sun's distance from
  the centre of the screen instead.

The time-of-day control walks the first section's keyframes and opens on the
one in force when the mission's clock starts. The scene's light points at
whichever body is up, so the shading and the sky agree; the game's own light
direction is not established.

## Not resolved

- **Where the sun's lights point.** `CSun` makes two directional lights and
  sets only their colours; the field a directional light is drawn with, the
  light record's `+0x24`, has no writer found. Next handle: the light
  manager's other callers and whatever fills its records each frame.
- **What `CSun` does with the lifetime** it is given, and so whether a body
  started before the clock's start keeps its full lifetime from its own
  keyframe or from creation.
- **How the 34142-radius dome escapes the far plane and a fog ending by
  700.** Its layers are depth-tested without depth writes, in render layer 1,
  on a record that takes the scene's fog. Next handle: how layer 1 is drawn —
  its projection, and the fog defaults the render pass copies from the
  shader's slot 7 at `0x1003d9f2`.
- **Whether `ForceSWFog` does anything outside `Terrain.dll`**, through the
  settings interface's getter. Inside it the scene asks Direct3D for linear
  range-based vertex fog on untransformed vertices.
- **The heading's world axis.** The angle is the compass heading of the
  camera matrix's first column, 0 along +y and turning towards +x; that the
  first column is the view direction is a *guess*.
- The sun sprite's extent unit, camera slot 27, and the shader's slot 5
  colour filter and its flag bit 0.
- ~~Which field carries the opcode~~ — the word ahead of slot 0; the three
  dead candidates were one keyframe out.
- ~~What selects between a file's two day cycles~~ — nothing: they play in
  turn.
- ~~When a mission's sky clock starts~~ — at the file's closing time.
- ~~Where a rain shower stops~~ — at its stop opcode.
- ~~The rest of the file header~~ — the section version, a keyframe count,
  a 23:59 nobody reads and the day length; the uninitialised `6939832` is in
  the times' last eight bytes, and marks the editor session that saved the
  file: it is there exactly on the 12 files whose last `int32` is 1.
- ~~Slots 0 and 16; property 15; the sun's seven values~~ — unused, unused,
  the clouds' colour, and two lights' colours and the sprite's size.
- What header bit `0x4000000` does on 81 textures.
