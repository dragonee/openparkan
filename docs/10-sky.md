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
and its moon 300 (15:30 to 23:30). What `CSun` does with the lifetime once it
has it (`× 1000` at `+0x2c`) is **pace the body across the sky**: the fraction
of it that has run is θ along the arc
([below](#the-body-travels-that-matrixs-arc--read)). The clock it measures
against is `this+0x28`, stamped by the object's slot 2 from the time its caller
hands it (`0x1007d168`); which time that is, is not traced.

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

## The sun and the moon are drawn

Everything a body's sprite needs is now read, and the engine draws it. Four
pieces come together, each from a different file:

- **Which body, and when.** The start-`SUN` opcode names it, and it is up for
  its lifetime ([above](#a-bodys-lifetime)).
- **Where it stands.** Its own arc, from the fraction of that lifetime that has
  run ([above](#the-body-travels-that-matrixs-arc--read)).
- **What it draws with.** Its `sky.wea` slot — 3 for `sun`, 4 for anything
  else — which is a **material** name, and the material names both a texture
  and a **cell** of it ([above](#a-material-picks-a-sub-image-as-well-as-a-texture)).
- **How big.** The keyframe's first two floats, its extent across and up.

The cell is the piece that is easy to miss and impossible to miss twice.
`ENV_SUN_3` names `SUN1.0`, which holds **four** bodies; a quad taking the whole
sheet draws all four at once, in a square, which is what this engine did until
the picture showed it. *Measured* over the 29 shipped files: every slot 3
resolves to an `ENV_SUN*` material and every slot 4 to an `ENV_MOON*` or
`ENV_SUN*` one, and all of them carry **blend mode 4**,
`SRCALPHA`/`INVSRCALPHA` ([07-objects.md](07-objects.md)), so a body is blended
on its texture's alpha and not added.

**The extents are a factor, not a length.** *Measured* over all 656 keyframes:
across runs 0.4 to 3.3 and up 0.4 to 3.0, and **across is at or above up on all
656**, so a body is never drawn taller than it is wide. Nothing that small is a
world size.

**The base they scale is an angle, and it is 0.325 radians** — *read*. The takt
takes camera slot 27 and multiplies it by 0.325 and then by 0.5
(`0x1007dfdf`); the products are the sprite's two half-extents, and the four
corners it writes at `CSun+0x21c`..`+0x278` are **screen pixels** at *z* =
0.999, the same pre-transformed quad the sky's first draw uses. **Camera slot
27 is pixels per radian**: `CCamera`'s slot 27 (`0x100851c0`, vtable
`0x1009c620` installed at `0x10083a2b`) asks its view for the viewport
rectangle (slot 15, `0x100820c0`, the four ints at view `+0x10`), takes
`right − left`, and divides it by the view's field of view (slot 17,
`0x10082120`, the float at view `+0x234`, which the view's projection puts at
`+0xc` of the 100-byte camera block, `0x10081a92`). That field really is an
angle in radians: `Ngi32` halves it and takes its sine and cosine
(`0x10007055`, `ngiGetSinCos`), writing `cos(fov/2)` into the projection's
`m[0]`, `(width/height)·cos(fov/2)` into `m[5]` and `sin(fov/2)` into `m[11]` —
a cotangent projection with a full horizontal field of view of `fov`. The game
sets it to **1.7 rad**, 97°, at `0x1001ffd7` (view slot 10, `0x10081f60`; the
constructor's default is 1.0).

The screen width and the field of view therefore cancel: a body's **half-width
is `extent × 0.1625 radians`**, whatever the resolution and whatever the view
is set to. A body at extent 1 is **18.6° across**, and the shipped 0.4 to 3.3
span **7.4° to 61°** — the engine's earlier guess of 8° at extent 1 was less
than half of it. The sheets hold **planets** as well as suns, which a sky hangs
large, and the read figure hangs them larger still. The game lays the angle out
linearly in pixels (it is a pixels-per-radian scale, not a tangent), so a body
near the edge of a 97° view is drawn a little smaller than a true projection
would draw it.

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
multiplies, and ORs the other three back unchanged.

**The texture column is `sky.wea`'s slot 5 and slot 6, in that order** (*read*).
`RenderFlare` branches on the column into `CSun`'s material block `+0x11c` when
it is 0 and `+0x19c` otherwise (`0x1007d753`), and the constructor fills those
two from the slot list by index: `push 5` into `+0x11c` (`0x1007cb05`) and
`push 6` into `+0x19c` (`0x1007cc0a`) — the same 0x80 block stride the sky's
own materials use. So the eight ghosts on texture 0 draw `ENV_FLARE_00` and the
four on 1 draw `ENV_FLARE_01`, which the names had already suggested.

Both are cells of `SUN.0`, the same 256-pixel sheet the sun and the moon come
from: cell 1 is a soft disc whose alpha peaks at **25 of 255** and cell 3 a
brighter one peaking at 146 (*measured*). Added over a daylit sky the chain is
faint on purpose — rendering a frame with and without it moves 88579 of the
2359296 bytes of a 1024×576 capture, over 240 rows, by at most 19.

Intensity has two gates, and both are now exact. The first: the flare is off
once the sun is more than **15°** off the view axis, ramps linearly to full
on-axis, and the ramp is then squared. The second ramps on **how high the body
stands at that moment**, which rises and falls as it crosses — see
[below](#where-the-sun-stands-and-it-is-not-in-a-file). Both sets of cosines
are cached at load from constants of 15, 30 and 60 degrees.

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
`CreateAtmosphereObject` hands it to the constructor. Everything below falls
out of those four fields.

**The fourth field is the slot table.** 3 and 4 are `sun` and `moon` in
`SLOT_ROLES` — read out of `sky.wea` quite separately, from the nine slots all
29 missions fill in the same order. The engine and the data agree on the
index without either having been derived from the other.

**The two angles are an azimuth and a tilt.** `CSun::Render` builds
`Rodrigues(axis = (cos A, sin A, 0), B)` and multiplies it by a rotation of
`A` about the vertical, which is `Rz(A) · Rx(B)` (`0x1007d2e3`), and keeps it
at `this+0x38`. Its third column is `(sin A sin B, −cos A sin B, cos B)`. So:

| | azimuth | tilt | third column (game axes, z up) | above the horizon |
|---|---:|---:|---|---:|
| sun | 90° | 30° | (0.5, 0, 0.866) | **60°** |
| moon | 0° | 50° | (0, −0.766, 0.643) | **40°** |

### The body travels that matrix's arc — *read*

The two angles are never rewritten, but they place an **arc**, not a body.
Every takt the sun object's slot 3 (`0x1007ed40`) takes the fraction of its
lifetime that has run — `(now − this+0x28) ÷ this+0x2c`, clamped to 1
(`0x1007ed4c`–`0x1007ed85`) — forms

```
θ = (1.2 × fraction − 0.1) × π
```

from the constants `1.1` and `−0.1` (`0x1009bb48`, and `0x1009c1ac`, which sits
immediately past the object's vtable), writes `(cos θ, 0, −sin θ)` to `this+0x78`
(`0x1007edc9`) and turns it through the matrix at `this+0x38` (`0x1007ede6`).
The result is the direction the body's **light travels**:

```
(cos A cos θ − sin A sin B sin θ,  sin A cos θ + cos A sin B sin θ,  −cos B sin θ)
```

At θ = 90° — halfway through the lifetime — that is the third column negated,
so the table above is where each body stands at the **top** of its arc. The
ends, θ = −18° and θ = 198°, put the body below the horizon: −15.5° for the
sun, −11.5° for the moon, the same at both ends (*measured*, from the two
angles). **The sun rises, crosses and sets**, on a plane the azimuth and the
tilt tip out of the vertical.

**And that is where the flare's second gate comes from.** It ramps on the
height of that same vector — `this+0x78`, whose third component is the `+0x80`
the gate negates and compares — between `cos 60° = 0.5` and `cos 30° = 0.866`.
Negated, the height is `cos B × sin θ`, which peaks at `cos B`: the sun's peak
*is* `cos 30°`, to the last bit, so it reaches exactly the top edge of the ramp
at its zenith and falls off it as it rises and sets; the moon's peak of 0.643
gives 0.390 and never more. The two gate constants were chosen to bracket the
two bodies at their highest, which is what identifies what the gate measures.

That also settles what `CSun` does with the lifetime it is handed: it is the
**span of the crossing**. Mission 01's sun has 525 seconds of a 900-second day
to cross, and its moon 300.

What the mission does choose is **when**, and the data backs the reading.
**33 of the 35 shipped sections hold exactly one sun window and one moon
window**, opcode 0 to opcode 1 — the sun up from about 00:30 to 14:30, the
moon from 15:30 to 23:30. **No section has them up at once**, which is what
makes two arcs only a quarter turn apart coherent: they are never
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
**ambient** colour, to which the scene colour is added to make the emissive.
So the clouds are slot 18 in diffuse, and the scene colour alone in emissive. *Measured*: on 24 of the 27 files whose light varies, slot
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

The sun's on-screen half-extents are `float 1 × 0.1625 × camera slot 27`
across and `float 2 ×` the same up (`0x1007dfdf`), which the takt tests
against the screen's edges; slot 27 is pixels per radian, so the two are
`extent × 0.1625` **radians**
([above](#the-sun-and-the-moon-are-drawn)).

### The colour filter is the camera's infrared, not the shader — *narrowed*

This page used to call it "the shader's slot 5". It is not the shader. `CSun`
keeps the object at `+0x1c` and the sky at `+0x18`, both handed down by
`CAtmosphere` from its own `+0x16c` (`0x1006fde6`, into the five-way body
factory `0x10069dd0` and on to each constructor at `0x1007c817`); the only
writer of `+0x16c` is the setter at `0x10084950`, slot 8 of the same
`CCamera`/`CAtmosphere` wrapper vtable `0x1009c620`. The item renderer holds
the same kind of object in the global `0x100a5e44`, which
`CPrimBuffer::CPrimBuffer` fills by asking the shader component
(`LoadComponent` of `CID_SHADER`) for interface **4** (`0x10032a75`).

Two of its slots are used, and all three callers use them the same way:

- **slot 7** (`+0x1c`) returns a small state block; **bit 0 of that block's
  `+8`** is the flag. The sky tests it at `0x10079786`, the sun's takt at
  `0x1007d923`, the item renderer at `0x1002fe79`.
- **slot 5** (`+0x14`) is a `__fastcall` that takes one `D3DCOLOR` in `edx` and
  returns one. With the flag set the sun passes slot 17's colour through it
  (`0x1007d95f`) and the sky its fog colour (`0x10079b05`).

The sky also sets its colour mask to `0xff00ff00` in that mode (`0x100797a8`,
against `0xffffffff` otherwise) — green only. That names the filter: it is the
camera's **infrared**, `CMD_CAMERA_INFRARED` (35) with `CIS_INFRARED_ON`,
`_OFF` and `_INV` in `World3D.dll` and `NightVisionOn` in `iron3d.dll`, whose
HUD lamp is already read ([35-hud.md](35-hud.md)). **What slot 5 computes is
still not read**: the object's vtable is installed outside `Terrain.dll` and
its class was not found.

### Where the two lights point — *read*

A `CLightSrc` record is **0x5c bytes**, in an array at the manager's `+0xc`
with the count at `+0x10` (`CLightSrc::CheckLightSrcNo`, `0x1008006e`). A
type-3 light's direction is
the three floats at record `+0x24`, both where the Direct3D light is built
(`0x10030ab3`, against `+0x18` and `+0x30` for the other types) and in the
emboss pass (`0x1002d08d`). Its writer is the manager's **slot 9**,
`0x10080550` — `SetDirection(id, space, x, y, z)`: space 0 stores the vector
as given (`0x100805a8`), space 2 first turns it through the manager's own
object's world placement (`0x100805be`–`0x10080629`), and any other space does
nothing.

`CSun` calls it twice every takt, from the same slot 3 that walks the arc
([above](#the-body-travels-that-matrixs-arc--read)): the **first** light
(`this+0x20`) gets the vector at `this+0x78` in space 0 (`0x1007edfd`–
`0x1007ee2a`), and the **second** (`this+0x24`) gets it negated
(`0x1007ee2d`–`0x1007eea8`). So the main light shines the way the body's light
travels and the second shines back at it — a counter light, which is what the
unread setting `ContrLightOn` is named for.

*Measured*, over the whole module: of the **72** sites in `Terrain.dll` that
stride a light record by `0x5c`, the record offsets ever written are 4, 8, 0xc,
0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40,
0x44, 0x48, 0x4c, 0x50, 0x54 and 0x58 — and `+0x24` is written at exactly
**three**: slot 9's two branches, and `CLightManager::CLightManager` at
`0x1007fde1`, which gives every light it makes the default direction
**(1, 0, 0)** and the colour white. Nothing outside the module can reach a
record except through the manager's 23 slots, and no other slot writes `+0x24`.

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
(`0x10028500`, `CShade`'s vtable `0x1009b17c`, installed at `0x10041f94`) into
group 1, with the static render record at `0x100a7138` — its own class's
vtable and flags 0, both written at `0x1007c07d` — so they take the scene's
fog, not their own. The record only reaches the item at
all when the item's own flag word carries `0x10` (`0x100285a2`), which of the
sky's draws only the clouds do, and the clouds hand over a record of their own
with flags `0x40` and fog 5000 to 11380.7 (`0x1007a5c3`–`0x1007a607`). Bits 0
and 1 of another argument become the draw item's `ZENABLE` and `ZWRITEENABLE`
bytes (`+0x12c`, `+0x12d`, each set when its bit is clear, `0x10028664`), which
the item renderer sets as render states 7 and 14 (`0x100302fb`, `0x10030318`):

| draw | material block | matrix | flags | `ZENABLE` | `ZWRITEENABLE` |
|---|---|---|---:|---:|---:|
| `0x1007a37a` | `+0x484` | `0x100a7168` | `0xc` | 0 | 0 |
| `0x1007a408`, `0x1007a49e` | `+0x384`, `+0x404` | at the camera | `4` | 1 | 0 |
| `0x1007ab51` | `+0x304` | dropped 5000 | `0x14` | 1 | 0 |

So the layers drawn at the camera are depth-tested and write no depth. Flag bit
3 of that word is the **pre-transformed** vertex path, FVF `0x1c4`
(`0x10030043` → `0x1002f2c0`), and only the first draw sets it.

**What a render layer is, and which pass the sky gets** — *read*. The two last
arguments of that call are a **group** and a **layer**, and the sky passes
group 1, layer 0 on all four of its draws, the clouds included (`0x1007a32d`,
`0x1007a3a4`, `0x1007a43a`, `0x1007aaeb`). Group 1's layer 0 is
a pass that **overrides the camera to near 700, far 50000 and viewport z 1.0 to
1.0** for its own length. That is how a 34142-radius dome gets drawn at all,
and the rest of this section is how that was read.

The item's group and layer go to `CPrimBuffer::AddItem` (slot 2,
`0x10032f10`), which appends it to `passes[group][layer]` — the array at the
buffer's `+0x14` when the group is 0 and at `+0x1c` otherwise — after asking
the item's own slots 2 and 3 whether it is to be drawn at all. The item itself
comes from the draw-item manager's `Alloc(kind, group)` (slot 0, `0x10032f90`,
vtable `0x1009b13c` installed at `0x100422f2`, the manager in the global
`0x100a6000`): five item kinds, each with a pool per group, so the **pool is
the group**. The sky takes kind 0 (`0x10028508`), the mesh draw kind 1
(`0x100455bc`).

A view then draws in two rounds (`0x10081c70`): it runs the pass list's group 0
(`0x10081ca6`), resets pool 0 (`0x10081cab`), runs group 1 (`0x10081cd5`) and
resets pool 1 (`0x10081ce8`). So group 1 is drawn **after** everything in group
0, and within a group the passes run in order and each pass draws its own
items in the order they were filed.

**The reset touches no render state at all** — *read*, with a control. Slot 1
of the manager (`0x100332b0`, `ret 4`) runs from `0x100332b0` to `0x100336ea`
and contains **no `call` instruction**: it walks the two chunked arrays of the
named pool, zeroes each item's `+4` and then the pool's count. It cannot touch
the fog, or anything else. The control is the same scan over its sibling,
`CPrimBuffer::Draw` (`0x10032c60`), which has six calls in its first 130
instructions.

**A pass carries its own near plane, far plane and depth range** (*read*). At
its start (`0x1003d760`) a pass asks the render device for the current camera
into a 100-byte block of its own (`GetCamera`, `Ngi32.dll:0x100073f0`, which
names the block: size `0x64` at `+0`, near at `+4`, far at `+8`, the field of
view at `+0xc`, the camera matrix pointer at `+0x14`, the viewport rectangle at
`+0x3c` and its min and max z at `+0x4c` and `+0x50`), overwrites the near, the
far and both z bounds from its own six-word descriptor (`slot 5`,
`0x10031bd0`), and hands the block back through the device's `SetCamera` (slot
25, `Ngi32.dll:0x10007170`). At its end (`0x1003d920`) it restores all four and
sets the camera again. A descriptor is `{type 0..5, near, far, min z, max z,
flag}`, 24 bytes, and the type picks which of six pass classes the factory
makes (`0x10031760`; the jump table is at `0x10031ae6`, and type 4 falls to its
default and makes nothing).

**Who calls `SetPasses`** — *read*. Nothing calls `0x10032ae0` directly; it is
slot 0 of `CPrimBuffer`'s vtable `0x1009ae18`, installed by
`CPrimBuffer::CPrimBuffer` at `0x10032a25`. `CShade`'s render setup
(`0x10041370`) makes **two** prim buffers, keeps them in the globals
`0x100a60c0` and `0x100a60c4`, and gives each the same two descriptor arrays
(`0x1004217a`, `0x100421e0`):

```
SetPasses(this, 4, 0x100a6060, 14, 0x100a1d68)
```

— **four** passes in group 0 and **fourteen** in group 1. Both arrays live in
`.data` and are filled at load, group 0's by `0x10040a00` and group 1's by
`0x10040b10`, from the float constants at `0x1009b0ec`..`0x1009b120`. Group 0
is four copies of one pass — type 0, near 0.5, far 700, z 0.1 to 0.99, the flag
byte 1 on the first and 0 on the rest. Group 1 is the interesting one:

| layer | type | near | far | z range | filed by |
|---:|---:|---:|---:|---|---|
| 0 | 2 | **700** | **50000** | **1.0 – 1.0** | the sky, all five draws |
| 1–4 | 2 | 0.5 | 700 | 0.1 – 0.99 | |
| 5 | 1 | 0.5 | 700 | 0.1 – 0.99 | a see-through surface (`0x100455b2`) |
| 6 | 3 | 0.5 | 700 | 0.1 – 0.99 | |
| 7 | 3 | 0.2 | 700 | 0.1 – 0.99 | |
| 8 | 2 | 0.5 | 700 | 0.1 – 0.99 | |
| 9 | 0 | **0.05** | **10** | **0.0 – 0.1** | a fifth slot, ordinary node |
| 10 | 2 | **0.05** | **10** | **0.0 – 0.1** | a fifth slot, the cockpit |
| 11 | 0 | 0.5 | 700 | 0.1 – 0.99 | |
| 12 | 5 | 0.5 | 700 | 0.1 – 0.99 | |
| 13 | 2 | 0.5 | 700 | 0.1 – 0.99 | |

So the depth buffer is cut into three: the first-person passes own 0.0 to 0.1,
the world owns 0.1 to 0.99, and the sky is pinned at 1.0. The sky's pass begins
exactly where the ordinary far plane ends and reaches past the dome's 34142,
and because its whole output lands at 1.0 it fills only the pixels the scene
left — group 1 runs after group 0. The fifth slots are in
[07-objects.md](07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws).

**The descriptor an earlier round found unread is a duplicate.** The static
initialiser at `0x1007c450` fills the same fourteen descriptors at
`0x100a3828`, in the same order and with the same values, and four more at
`0x100a7410`; nothing reads either, and they stay written and unused. The live
arrays are `0x100a6060` and `0x100a1d68`, and a raw search for their addresses
finds only the two `push`es apiece, because the initialisers write each field
by its own absolute address (`0x100a1d6c`, `0x100a1d70`, …) and never name the
array's base. The control that round used pointed at the wrong kind of object
too: `0x100a3800`, "the blob the dome's own draw pushes", is not a camera
descriptor at all — it is the quad's **vertex format**, which the queue stores
at the draw item's `+0x194` between the vertex count at `+0x190` and the index
count at `+0x198` (`0x100286ab`–`0x100286c9`).

**Its colours** (`0x1007ac60`):

- the apex and ring 1 take property 12;
- rings 2, 3 and 4 take the three compass groups, from the top down;
- within a ring, quadrant *k* runs from group[*k*] to group[*k* + 1], a
  segment at a time;
- the rim takes the fog colour every takt.

**The clouds** use the same cap with its origin 5000 below the camera
(`0x1007a08e`), and carry their own fog, from 5000 to 11380.7 (`0x1007a5d4`).

### The sky's first draw is a screen-wide quad, and it is usually skipped — *read*

The draw at `0x1007a37a` is built from the viewport rectangle the view hands
back (`0x1007a1a6`): four corners at *z* = 1.0 written into the sky's own
`+0x254`, `+0x260`, `+0x26c` and `+0x278`, six indices, the vertex format at
`0x100a3800`, item flags `0xc` — pre-transformed vertices, the depth test and
the depth write both off. It is filed into group 1 layer 0 like the rest of the
sky and, being filed first, is drawn first within that pass.

**It carries no colour of its own.** Its material block is the sky's `+0x484`,
and the only fields anything ever writes in it are `+0x20` (1.0, the ambient
alpha, `0x10078614`), `+0x48` and `+0x4c` (both 0, so no texture), `+0x58` (0)
and `+0x5c` (a format id from `CShade`'s five-entry table at `+0xbfc`,
`0x10046a98`). Its diffuse at `+4` and its ambient colour at `+0x14` are never
written anywhere in `Terrain.dll` — a scan of every `[reg + 0x484..0x504]`
operand and every `add`/`lea` by a constant in that span over the whole module
returns the six sites above and the one `add ecx, 0x484` that pushes the block.
So what it would put on the screen is the **scene colour** alone, which is
what every material's emissive gets added
([below](#the-scene-colour-is-added-to-every-material)).

**But the draw is gated** (`0x1007a325`): the sky asks the view (interface
`0x12`) for its mode — slot 24, `0x10083010`, the field `+0x280` that slot 23
sets and the view's constructor leaves 0 — and **skips the quad when the mode
is 1**. Which views carry which mode is not read. The inference is that the
mode that draws the world is 1: group 1 runs *after* group 0, so a screen-wide
quad with the depth test off would paint over the finished scene.

### The three layers and their texture coordinates — *read*

The cap builder (`0x100787f0`) lays out, per vertex, one position array
(`+0x34`, 12 bytes) and one colour array (`+0x38`, 4) — and then **three**
eight-byte arrays, `+0x40`, `+0x44` and `+0x48`. They are texture coordinates,
and all three are the same thing: the vertex **projected straight down on to
the horizontal plane, in radii**, times one constant each
(`0x10078f2f`, `0x10078fa9`, `0x10079023`):

```
u = x / R * K     v = y / R * K
```

| array | *K* | drawn by |
|---|---:|---|
| `+0x48` | 1 | the nebula |
| `+0x44` | 3 | the clouds |
| `+0x40` | 15 | nothing |

The rim is at *R* sin(π/4), so a set reaches 0.707 *K* at the horizon: the
nebula never leaves one tile of its sheet and the clouds cross three.

**Which layer takes which** falls out of the vertex-stream blocks the draws
name. The builder fills four of them, each a run of `{pointer, stride}` pairs
(`0x100792fc` onwards):

| block | position | colour | texture coordinates |
|---|---|---|---|
| `+0xd4` | `+0x34` | `+0x38`, per vertex | none |
| `+0x74` | `+0x34` | a constant white | `+0x40`, *K* 15 |
| `+0x134` | `+0x34` | `+0x3c`, per vertex | `+0x44`, *K* 3 |
| `+0x194` | `+0x34` | a constant white | `+0x48`, *K* 1 |

and the sky's four draws, in the order it queues them:

| draw | material | from | matrix | vertices |
|---|---|---|---|---|
| `0x1007a37a` | `+0x484` | — | fixed | a four-vertex quad, `+0x1f4` |
| `0x1007a408` | `+0x384` | **slot 0**, the nebula | at the camera | `+0x194`, *K* 1 |
| `0x1007a49e` | `+0x404` | — | at the camera | `+0xd4`, no texture |
| `0x1007ab51` | `+0x304` | **slot 2**, the clouds | 5000 below | `+0x134`, *K* 3 |

The material blocks are filled from `sky.wea` by index, each a copy into its
own 0x80-byte block: index 1 to `+0x284` (`0x1007828c`), index 2 to `+0x304`
(`0x1007838e`), index 0 to `+0x384` (`0x10078496`). `+0x404` and `+0x484` take
no slot; their texture handles come from the render device (`0x100785a2`,
`0x100785f5`), so those two draws are untextured — the vertex-colour dome and
a flat quad over the viewport.

So **the nebula is drawn first and the coloured dome over it**, which is what
makes the dome's alpha matter.

### The dome's alpha shows the nebula through — *read*, and *measured*

The colour slots are BGRA, and their alpha is not padding. *Measured* over all
656 shipped keyframes: the apex is **alpha 0 on 355** of them and the four
horizon colours are **255 on 2594 of their 2624**. The control is in the same
read — slot 20, the scene colour, is 255 on all 656.

Read against a mission's clock it is a day. `CAMPAIGN.00/Mission.01`:

| clock | apex | ring 2 | ring 3 | horizon |
|---|---:|---:|---:|---:|
| 00:00 | 0 | 0 | 120 | 255 |
| 01:30 | 45 | 0 | 140 | 255 |
| 03:40 – 11:00 | 255 | 255 | 255 | 255 |
| 13:15 | 45 | 70 | 230 | 255 |
| 15:00 | 0 | 0 | 120 | 255 |

— clear overhead at night, solid by day, and solid at the rim throughout.
That is a nebula that comes out after dark and is washed out at noon, which is
what the four `ENV_NEBULA_*` sheets are for; and the four are **256-pixel
`RGB565` images with no alpha at all** (*measured*), so the layer under the
dome is a backdrop, not a wash. `CAMPAIGN.01/Mission.01` keeps its apex at 0
on every one of its five keyframes, so its nebula is up the whole mission.

### The stars are built and never drawn — *read*, and *measured*

Slot 1 is `ENV_STARS` on all 29 missions, its material is copied into the
sky's `+0x284`, and the cap carries a third set of texture coordinates at
*K* 15 in the block at `+0x74`. Nothing draws either of them. `0x284` as a
32-bit immediate appears in `Terrain.dll`'s code at exactly one site inside
the sky, the setup at `0x1007828c`; the controls are the other two slots'
blocks, which the same search finds at both their setup **and** their draw
(`+0x384` at `0x10078496` and `0x1007a3e5`, `+0x304` at `0x1007838e` and
`0x1007ab2c`). `AtmStarsOn` agrees: it is one of the four settings
[nothing reads](#nobody-reads-forceswfog--read-and-measured), while
`AtmCloudsOn` gates the cloud draw (`0x1007a4cd`), `AtmSkyDetail` the segment
count (`0x1007ac8f`) and `LensFlareOn` the flare (`0x1007d523`).

`STAR0.0` is drawn and shipped — 256 pixels square, `ARGB8888`, alpha 0 on
65441 of its 65536 pixels, a sparse star field. It just never reaches the
screen.

### What each sky material blends with — *measured*

Every `sky.wea` slot of all 29 missions, through `Material.lib`:

| slot | material | texture | cell | blend |
|---:|---|---|---|---|
| 0 | `ENV_NEBULA_*` | `NEBULA_0..3.0` | whole | 4, `SRCALPHA`/`INVSRCALPHA` |
| 1 | `ENV_STARS` | `STAR0.0` | whole | 4 |
| 2 | `ENV_CLOUDS*` | `S_03/05/06.0`, `TOK51.0` | whole | 4 |
| 3 | `ENV_SUN*` | `SUN*.0` | 0, 3, 4 or whole | 4 |
| 4 | `ENV_MOON*`, `ENV_SUN*` | `SUN.0`, `SUN3.0`, `SUN4.0` | 2, 3, 4, 5 | 4 |
| 5 | `ENV_FLARE_00` | `SUN.0` | **1** | **2, `SRCALPHA`/`ONE`** |
| 6 | `ENV_FLARE_01` | `SUN.0` | **3** | **2** |
| 7 | `SNOWFLAKE`, `DUST_ADD` | `EFFECT6.0`, `DUST.0` | 20, 0 | 4, 2 |
| 8 | `RAIN_DROP` | `EFFECT6.0` | 21 | 4 |

So the sky blends on alpha everywhere but the flare, which is added. `ENV_STARS`
is the one material in the game with **bit 0** of the flags byte set, and what
that bit does is still not read — it does not change the mode.

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
- **`ForceSWFog` is never read, in `Terrain.dll` or anywhere else.** See
  [below](#nobody-reads-forceswfog--read-and-measured).

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

So *b* = 0 gives property 0, and *b* grows from +y towards +x.

**The first column is the axis the camera looks along — *read*.** The matrix
the angle getter reads is the camera object's world placement: slot 28
(`0x100850f0`) asks the `CGameObject` interface at the camera's `+4` for
placement type 2 (`CGameObject::GetPlacement`, `0x1008ae50`, which returns the
world matrix at the object's `+0x60`) and takes `m[0]`, `m[4]` and `m[8]`.
The same call, with the same type, is what the view hands the renderer: the
view's placement getter (`0x10082f60`) forwards type 2 to its attached
object, the view's projection setup puts the pointer it returns at `+0x14` of
a 100-byte camera block (`0x10081a58`), and the render device's slot 25 copies
all sixteen floats into its own slot (`Ngi32.dll:0x100073a5`) and builds the
Direct3D view from them (`0x10009450`):

- the view's **depth** is `(m[0], m[4], m[8]) · (p − E)`;
- its **x** is `−(m[1], m[5], m[9]) · (p − E)`, its **y** `(m[2], m[6], m[10])`;
- with the eye *E* = `(m[3], m[7], m[11])`, which is how the dome is placed at
  the camera (`0x1007a17d`) and how a camera's position is read elsewhere
  ([03-terrain.md](03-terrain.md#when-it-draws--read)).

So the columns are forward, left, up and the eye, the bottom row is
`(0, 0, 0, 1)`, and *b* is the compass heading of the **view direction**: the
fog is exactly the horizon in the direction you look. The second consumer
agrees — `Control.dll` builds a unit's first-person frame as a matrix whose
columns are look, side, up and eye and hands it out as the same placement type
2 ([30-turrets.md](30-turrets.md#aiming-and-the-camera--read-and-measured)).

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
material's ambient colour**, and an ambient term of 0 (`0x100308b8`). It is the
scene's ambient light in all but name — 40/255 grey at Mission 01's brightest
keyframe, a brighter violet at night. An earlier draft said the material's own
emissive; the block offsets `+0x14..+0x1c` are the entry's ambient, and the
entry's emissive is never read. Nothing sets `D3DRS_AMBIENT`, so the zero
ambient term is moot: a material's ambient colour is its self-light. See
[07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured).

### The render settings

`Terrain.dll` reads 36 settings from `shade.cfg` (`0x1005f652`). No
`shade.cfg` ships (*measured*), so the compiled defaults apply
(`0x1005fa80`). They are one page, in a fixed order: `CSettings` is the
global at `0x100a6ca8`, its values the 36 dwords from `+4`, and its names the
36-byte descriptors from `0x100a6798` that `0x1005eb10` fills at load. The
index is what every read of the page names, so it is worth having whole:

| # | setting | default | reads |
|---:|---|---|---:|
| 0 | `ForceSWFog` | 1 | **0** |
| 1 | `LightingOn` | 1 | 1 |
| 2 | `SpecularsOn` | 1 | 1 |
| 3 | `EmulatePointLight` | 1 | 1 |
| 4 | `MicroTexturingOn` | 1 | 1 |
| 5 | `MicroTexScale` | 0.05 | **0** |
| 6 | `MaxShadowsQty` | 20 | 1 |
| 7 | `RobotDetail` | 1.0 | 1 |
| 8 | `RobotBestLOD` | 1 | 1 |
| 9 | `RobShadowsOn` | 1 | 1 |
| 10 | `RobShadowDetail` | 1.0 | 1 |
| 11 | `RobShadowBestLOD` | 1 | 1 |
| 12 | `RobShdwRefreshMask` | 1 | 1 |
| 13 | `RobShadowSmooth` | 1 | 1 |
| 14 | `BuildingDetail` | 1.0 | 1 |
| 15 | `BldShadowsOn` | 0 | 1 |
| 16 | `BldShadowDetail` | 1.0 | 1 |
| 17 | `BldShadowBestLOD` | 1 | 1 |
| 18 | `BldShdwRefreshMask` | 3 | 1 |
| 19 | `BldShadowSmooth` | 0 | 1 |
| 20 | `AtmCloudsOn` | 1 | 1 |
| 21 | `AtmStarsOn` | 1 | **0** |
| 22 | `AtmSkyDetail` | 4 | 1 |
| 23 | `LensFlareOn` | 1 | 1 |
| 24 | `ContrLightOn` | 1 | **0** |
| 25 | `UseEmbossBump` | 1 | 1 |
| 26 | `UseReflections` | 1 | 5 |
| 27 | `PortalNearDist` | 75 | 1 |
| 28 | `PortalFarDist` | 95 | 1 |
| 29 | `UseDXLighting` | 0 | 2 |
| 30 | `UseEMBMReflections` | 0 | 5 |
| 31 | `EMBMCoeff00` | 0.01 | 1 |
| 32 | `EMBMCoeff11` | 0.01 | 1 |
| 33 | `EMBMMaxVal` | 64 | 1 |
| 34 | `EMBMBumpTile` | 100.0 | 1 |
| 35 | `EMBMBumpMove` | 10000 | 1 |

The water's settings, `UseReflections` to `EMBMBumpMove`, are in [03-terrain.md](03-terrain.md#water-reflects--read-and-measured).

### Nobody reads `ForceSWFog` — *read*, and *measured*

`ForceSWFog` is the page's entry 0, at `0x100a6cac`. Inside `Terrain.dll`
a setting is read as `[index * 4 + 0x100a6cac]`, an absolute address the
loader relocates, so the whole set of reads is countable rather than
searchable: **41 sites name the page, all 41 resolve to a constant index, and
none of them is 0** (*measured*, from the module's relocation table). The
control is in the same 41: they cover 32 of the 36 settings, including entries
1 and 2 either side of it. Four are never read — `ForceSWFog`,
`MicroTexScale`, `AtmStarsOn` and `ContrLightOn`.

Outside it, the page is reachable only through `World3D.dll`'s settings
registry. `Terrain.dll`'s `InitializeSettings` builds `CSettings`, asks it for
interface 7 — which hands back the object itself (`0x1005ff20`) — and
registers it with the registry under id `0x1e` (slot 9, `0x1005f5ab`). The
registry is a table of 8-byte `{uint16 id, interface}` records at
`World3D.dll:0x10122658`, at most `0x80` of them ("Too many Settings.").
Its getter, slot 3 (`0x1000a400`), takes one dword key: it matches the key's
**low word against the id**, then tail-calls that interface's own slot 3 with
the key's **high word as the index** (`0x1000a43c` rebuilds the argument in
place, and `CSettings`' getter at `0x1005f9c0` masks it to 16 bits and returns
`[this + index * 4 + 4]`). So a cross-module read of `ForceSWFog` is the key
`0x0000001e`, and of `RobotBestLOD` the key `0x0008001e`.

Only the registry hands that interface out, and only
`World3D.dll!CreateGameSettings` hands the registry out. **5 of the 21 shipped
modules import it** — `AniMesh.dll`, `Control.dll`, `Effect.dll`,
`Terrain.dll` and `iron3d.dll` — and each calls it from exactly one site
(*measured*):

| module | site | what it does |
|---|---|---|
| `Terrain.dll` | `0x1005f569` | registers id `0x1e` and drops the pointer |
| `Control.dll` | `0x10032260` | registers id `0x15` and drops the pointer |
| `Effect.dll` | `0x10014091` | registers id `0x14` and drops the pointer |
| `AniMesh.dll` | `0x1000719c` | **reads** key `0x0008001e` — `RobotBestLOD` |
| `iron3d.dll` | `0x1005bc86` | keeps the registry at `0x1010b60c` |

`iron3d.dll` hands it out through one accessor (`0x1005b570`) called twice,
both on the `iron_3d.ini` path: it **writes** `UseReflections` (26),
`UseEmbossBump` (25) and `UseEMBMReflections` (30) from `REFLECTIONS`,
`EMBOSS_BUMP` and `EMBM` (`0x10061736`, `0x1006177b`, `0x10061795`), and
otherwise touches only interface `0xa`. `World3D.dll`'s own uses of the record
table are the registry's own methods and its constructor.

So: **`ForceSWFog` is read by nothing in the shipped install.** Its key is
never built, no read of the page resolves to index 0, the name is a string in
`Terrain.dll` and in no other module (*measured*), `Iron_3D.ini` carries no
fog key, and no `shade.cfg` ships — so it holds its compiled 1 for the whole
game and changes nothing. `Ngi32.dll`, the module that would have to implement
a software fog, cannot even reach the settings registry: it does not import
`CreateGameSettings`.

Two controls, both from the same searches: `AniMesh.dll` does read this very
page across a module boundary, and `iron3d.dll` does write three of its
entries; the scan that finds nothing at index 0 finds both.

## What the viewer draws

- The **nebula** on the dome, multiplied by a gradient from the keyframe's
  apex (slot 15) to its horizon at heading zero (slot 2), so one draw gives
  "this sky at this hour". The game's dome carries four compass horizons and
  two rings between; the viewer keeps one of each.
- The **stars** over it, additive, fading in as the day's light drops. This one
  is the viewer's own invention: the game
  [never draws them](#the-stars-are-built-and-never-drawn--read-and-measured),
  and their material is not additive either.
- The **clouds** over that, tiled four times and tinted by their own colour,
  slot 18. The game tiles them **three** times, and over a cap dropped 5000.
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
one in force when the mission's clock starts. The scene's light points where
the body up at that moment stands on its arc, and the second light back at it,
which is what the game does.

## Not resolved

- ~~Where the sun's lights point~~ — the light manager's slot 9
  (`0x10080550`) writes the record's `+0x24`, and `CSun` calls it twice every
  takt with the body's travel and its negation; the only other writer of the
  field is the manager's constructor, whose default is (1, 0, 0), and no
  other of its 23 slots touches it ([Where the two lights
  point](#where-the-two-lights-point--read)).
- ~~What `CSun` does with the lifetime~~ — it is the span of the body's
  crossing: the fraction of it that has run is θ along the arc
  ([The body travels that matrix's arc](#the-body-travels-that-matrixs-arc--read)).
  Whether a body started before the clock's start keeps its full lifetime from
  its own keyframe or from creation still turns on what stamps `this+0x28`
  (`0x1007d168`), which is not traced.
- ~~How the 34142-radius dome escapes the far plane and a fog ending by 700~~ —
  it is drawn in a pass of its own. `CShade`'s render setup gives both prim
  buffers `SetPasses(4, 0x100a6060, 14, 0x100a1d68)` (`0x1004217a`,
  `0x100421e0`), the sky files every draw under group 1, layer 0, and that
  pass overrides the camera to **near 700, far 50000, viewport z 1.0 to 1.0**
  for its own length. The descriptor an earlier round found "unread" at
  `0x100a3828` is a dead duplicate of the live array, and `0x100a3800`, the
  control it used, is the quad's vertex format and not a camera descriptor at
  all ([The dome](#the-dome)). The item-pool reset does nothing to the fog:
  slot 1 of the manager (`0x100332b0`, `0x100332b0`–`0x100336ea`) holds **no
  `call` instruction** and only zeroes counts; the control is the six calls in
  the first 130 instructions of its sibling `CPrimBuffer::Draw`
  (`0x10032c60`).
- ~~Whether `ForceSWFog` does anything outside `Terrain.dll`~~ — nothing
  reads it anywhere: 0 of `Terrain.dll`'s 41 reads of the settings page name
  entry 0, and of the four other modules that can reach the page only
  `AniMesh.dll` reads it at all, for `RobotBestLOD`
  ([Nobody reads `ForceSWFog`](#nobody-reads-forceswfog--read-and-measured)).
- ~~The heading's world axis~~ — the view direction. The matrix is the camera
  object's world placement (`GetPlacement` type 2), and the same matrix, by
  the same call, is what `Ngi32.dll` turns into the Direct3D view: its first
  column is the view's depth axis (`0x10009450`), its last the eye
  ([Fog](#fog)).
- ~~The sun sprite's **extent unit** and camera slot 27~~ — slot 27
  (`CCamera`, `0x100851c0`) is the viewport's width in pixels divided by the
  view's field of view in radians, the sprite's corners are screen pixels, and
  the two cancel: a body's half-width is `extent × 0.1625` radians, so extent 1
  is **18.6° across** and the shipped 0.4 to 3.3 span 7.4° to 61°
  ([The sun and the moon are drawn](#the-sun-and-the-moon-are-drawn)).
- The **colour filter's slot 5**, and its flag bit 0. *Narrowed.* It is not the
  shader: it is an object `CAtmosphere` hands its bodies from `+0x16c` and the
  item renderer gets from the shader component's interface 4, and it is the
  camera's infrared. Its slot 7 returns a block whose `+8` bit 0 is the flag,
  and slot 5 maps one `D3DCOLOR`; what slot 5 computes is not read, because the
  object's vtable is installed outside `Terrain.dll`
  ([The colour filter is the camera's infrared, not the
  shader](#the-colour-filter-is-the-cameras-infrared-not-the-shader--narrowed)).
- ~~Which of the flare's pair the engine calls texture 0~~ — slot 5; the column
  branches into the material blocks the constructor fills from slots 5 and 6
  ([The lens flare](#the-lens-flare)).
- ~~What the sky's fourth draw puts on the screen~~ — the **scene colour**, and
  usually nothing at all. Its material block `+0x484` has no texture and no
  colour of its own: over the whole module the only fields written in it are
  the ambient alpha at `+0x20`, the two texture fields at `+0x48` and `+0x4c`
  (both 0) and a format id at `+0x5c`, so the only colour it can carry is the
  scene colour every emissive gets. And the draw is skipped whenever the view's
  mode (slot 24, `0x10083010`) is 1 (`0x1007a325`). Which views carry which
  mode is **not read**; the inference is that the world's view is 1, because
  group 1 runs after group 0 and a screen-wide quad with the depth test off
  would paint over the finished scene
  ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad-and-it-is-usually-skipped--read)).
- **What lies below the dome's rim.** The rim is at eye height and the terrain
  covers what is under it, but what the frame is cleared to before either is
  drawn is still not read.
- **What flag bit 0 of a material's blend byte does.** It is set on `ENV_STARS`
  and on nothing else in the game, and it does not change the blend mode.
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
  the clouds' colour (which tints their layer, now drawn), and two lights'
  colours and the sprite's size.
- What header bit `0x4000000` does on 81 textures.
