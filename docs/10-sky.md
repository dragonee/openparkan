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
- **What colour.** One for all four corners: the sprite's stream block gives its
  diffuse as `CSun +0x95c` with a stride of 0 (`0x1007cd98`–`0x1007cdaa`), which
  is property 5, **slot 17**, and its specular as the `0xff000000` at `+0x960`
  (`0x1007cd7e`), no fog. The item is filed with flags `0xc` (`0x1007e69d`),
  pre-transformed and unlit like the sky's quad, so that colour is all it has
  beside its texture. *Measured*: slot 17 is black on 31 of the 576 keyframes
  that lie inside a body's window and on 53 of the 80 that lie outside one;
  the commonest of the rest are a mauve `(130, 115, 162)`, an orange
  `(255, 74, 0)` and a pale `(255, 235, 210)`. Its alpha is the one the flare's
  lift of the main light reads ([below](#the-lens-flare)), 255 on 197 of the 656
  and 50 or less on 249.
- **And the colour and the alpha both reach the frame.** The constructor gives the
  sprite's material block render phase **2** (`0x1007cabf`, the shade's slot 7,
  `0x10042850`, whose `+0x192c` is 2 on a device with that phase and 1 otherwise,
  `0x10041198`–`0x100411ad`); slot 16 of the shade copies the block to the item
  (`0x1002852f`), and the item's `+0xc8` is what `SetPhase` is handed (`0x10030386`). Phase 2
  is `Ngi32.dll`'s records 2 and 3 (`0x10034cb0`, `0x10034b30`), and either way the stage
  **modulates both**: colour texture × diffuse, alpha texture × diffuse. So under the
  material's blend 4 the disc is

  ```
  frame = T.rgb × slot 17's rgb × a + frame behind × (1 − a)        a = T.a × slot 17's alpha
  ```

  which is what the keyframes are written for: on C00 M01 the sun comes up an orange at 50
  of 255 and stands (255, 245, 230) at 255 by mid-morning. A black slot 17 never paints a
  black disc (*measured*): of its 84 keyframes the 54 at 255 lie outside every body's start
  and lifetime, and the 30 others carry an alpha of 0. (By that window, start and lifetime,
  12 of 541 are black inside one and 72 of 115 outside; the 31 of 576 above was counted some
  other way, which was not recovered.) The flare's two blocks take the same phase (`0x1007cbc4`, `0x1007ccc9`), which is the control: its intensity
  is an alpha on its vertices and nothing else (below), and it would do nothing otherwise.

  *Seen*: C02 M03's sun at briefing time 10 s, slot 17 at (228, 239, 39) and 114 of 255
  between its keyframes, against "Part 4" (Qqs8_i9IeUU, 0:42), mean colours over the same
  areas with the video's own matrix, the sky's as the control. Before, the engine drew the
  texture's own colours:

  | area | recording | before | after |
  |---|---|---|---|
  | the disc's core | (143, 183, 34) | (202, 238, 147) | (142, 185, 34) |
  | its lower edge | (136, 173, 36) | (203, 226, 159) | (143, 181, 36) |
  | its left limb | (114, 161, 33) | (149, 199, 78) | (123, 170, 34) |
  | the brightest pixel in it | (162, 192, 44) | (255, 255, 255) | (163, 192, 39) |
  | the sky to the right | (106, 152, 35) | (109, 154, 38) | (109, 154, 38) |

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

**And it needs a clear line to the body** — *read*, found 2026-10-31 by tracing the face
filters [07-objects.md](07-objects.md#the-filters-the-first-enumeration-could-not-see--read)
could not see. The draw asks the world's slot 7 for the first face from half a metre out of
the eye (`0x1007e2a0`–`0x1007e3f4`, the direction made unit by `g_FastProc`'s `+0x34`, the 0.5
at `0x1009a1fc`) towards a point along the body's way (`0x1007dd5c`–`0x1007dd8c`,
`0x1007de64`), with a request of its own, {`0x41a`, 0, 0, 0, 0, `0x20`}, kept at `CSun +0x9c8`
(`0x1007c8d7`–`0x1007c913`) and translated into the query's filter (`0x1007e377`–`0x1007e3a1`).
Any face takes the flare away and the lift with it: the answer's 0 goes to the same plain
colour the infrared takes (`0x1007e4ca`, `0x1007e7d4`, `0x1007eba3`). So a sun behind a hill,
a building or a tree throws no flare and lights the scene at its own colour. How far the
point stands is half a figure this round did not finish tracing (`ebp − 0x60c`); the engine
asks 10000 along, past every map's edge, a stand-in.

**Only a scene camera's own frame gets it.** `RenderFlare` returns at any
camera type but 0 (`0x1007d558`), and the sun's draw returns at once in a
camera of type 2, body and all (`0x1007d8f6`): the water's mirror has no flare
and, as the install runs it, no body
([below](#the-skys-first-draw-is-a-screen-wide-quad--read)). And with the
camera's infrared on the draw puts the body up and then passes over the whole
of the rest — the gates, the flare and the lift — handing the light manager
the plain colour instead (`0x1007e7ee`–`0x1007e7f9`, `0x1007eba3`,
[What the infrared does to a frame](#what-the-infrared-does-to-a-frame--read-and-seen)).

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
block at `+0x70`: its `+4` is the diffuse colour and its `+0x14` the material's
**ambient** colour. So the clouds' material is slot 18 in diffuse and nothing
in ambient, and the layer is lit: what a vertex of it shows is in
[The clouds are lit](#the-clouds-are-lit-and-fogged-on-a-range-of-their-own--read-and-measured).
The slot's alpha is not read; the block's ambient alpha stays the entry's.
*Measured*: on 24 of the 27 files whose light varies, slot
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

### The colour filter is the camera's infrared, in the shader component — *read*

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
HUD lamp is already read ([35-hud.md](35-hud.md)).

**The object is the shader component's one state, and slot 5 greys a colour into
green** — *read*. `Comp.ini` binds `CID_SHADER` (6) to `terrain.dll CreateShader`,
and `CreateShader` (`0x1004b910`) builds its object once and hands the same one back
ever after (the global `0x100a60e8`), so whoever loads the component gets the one
object. Of the fourteen modules only `Terrain.dll` imports `LoadComponent`, and its
four calls all load component 6 (`0x10032a69` the prim buffer, `0x10041fcd`
`CShade`, `0x1006ef14` the atmosphere, `0x10083b72` the camera). The constructor
(`0x1004b9a0`) installs interface 4 at `+4`, vtable `0x1009b2fc` — in this module
after all; the earlier search missed it for looking at the camera's vtables. Its
**slot 5** (`0x1004f900`) takes 0.33 of each of red, green and blue, sums them,
multiplies by 7, holds the result to 255 and returns it as the green alone, the
alpha kept and red and blue 0: green = min(255, 2.31 (R + G + B)). Its **slot 7**
(`0x1004fa70`) returns the state block at `+0x38`, whose flag word is `+8`; the
camera's frame render sets bit 0 every frame from its own flag word's `0x20`
(`0x10084514`–`0x1008456a`, [The scene colour](#the-scene-colour-is-added-to-every-material)).

### What the infrared does to a frame — *read*, and *seen*

**The flag lasts one camera's render.** `CCamera::Render` (`0x100844c0`) sends
the state a block of mask 1 whose bit 0 is its own flag word's `0x20`
(`0x10084514`–`0x1008456a`), sends it again after each of the three renders it
nests (`0x100845c8`, `0x10084674`, `0x100846c7`), and before it returns sends
the same block with the bit **clear** (`0x1008481a`–`0x10084851`). So the
infrared is on for what that camera draws and for nothing drawn after it: the
HUD, its model views, the next camera. The water's reflection is the one
exception by design — the reflection draw gives the reflection camera the scene
camera's flag word (`0x10083f0e`–`0x10083f20`), so the lake mirrors a green
world.

**Who asks.** The state's bit is read in nine places, all in `Terrain.dll`:

| reader | at | what it does with the bit set |
|---|---|---|
| the shade's lighter, slot 8 | `0x1004e0ad` | filters the item's material and the scene colour, below |
| the shade's indexed lighter, slot 3 | `0x1004c6bd` | the same, on the same four colours (`0x1004c6dd`–`0x1004c800`) |
| the effect sprites' colour, slot 4 | `0x1004f8cc` | puts the sprite's diffuse through slot 5 (`0x1004f8ee`) |
| the sky's draw | `0x10079786` | the fog colour and every dome colour through slot 5, below |
| the sun's draw | `0x1007d923` | slot 17's colour through slot 5 (`0x1007d95f`), and no flare |
| the snow's draw | `0x10075ae0` | its one colour through slot 5 (`0x10075b2b`) |
| the rain's draw | `0x10076460` | the same (`0x100764ab`) |
| the item renderer, twice | `0x1002f920`, `0x1002fe79` | keeps the item off the device-lighting road, which the shipped `UseDXLighting` 0 never takes anyway (`0x1002f8fd`, `0x1002fe50`) |

The first three read the block where it lies, at the interface's `+0x6c` and
`+0x3c`; the other six ask slot 7 for it. *Measured*: the install holds **414**
calls through a slot 7 (`+0x1c`) and **658** through a slot 5 (`+0x14`); of the
414, **six** read the answer's `+8` and mask bit 0 in the next five
instructions, the six of the table; and of the 658, **six** pass a colour in
`edx` to the state they have just tested — one each for the snow, the rain
and the sun, two for the sky, and slot 4's on itself — beside one in the item
renderer that is its vertex buffer's (`0x100302bc`). `0x1004f900` has no direct
caller. The control is what the sweep was known to owe: it finds the three
callers this page had already named — the sky, the sun and the item renderer —
and three it had not, the snow, the rain and the item renderer's second. The
three inlined copies of the filter are found by its constant instead:
`0x1009b360`, the 0.33, is named at 27 sites — twelve in slot 3, twelve in
slot 8 and three in slot 5 — and nowhere else in the module.

**What goes through it.**

- **Every lit item's material, and the scene colour.** Each lighter, before it
  sums a light, turns four colours of its own copies into the filter's green,
  in floats and **not held to 1**: the material block's diffuse (`+4`), its
  specular (`+0x24`) and its ambient (`+0x14`), and the merged state's scene
  colour (`+0x78`). Green becomes 7 × 0.33 × (R + G + B) and red and blue 0;
  the alphas are left. So the lit sum is green alone — a light gives a surface
  its **green** channel and nothing of its red or blue — and the floor under
  it is 2.31 times the scene colour's sum: 1.09 for Mission 01's 40-of-255
  grey, 2.39 for C02 M04's (86, 111, 67). A floor past 1 is kneed like any lit
  component and what is over goes to the specular
  ([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)),
  so **under the infrared no lit surface is darker than its texture's own
  green**. That is every mesh batch, the landscape, the clouds and the water.
- **The sky.** The fog colour, after its alpha is forced
  (`0x10079afa`–`0x10079b21`), so the device's `FOGCOLOR`, the dome's rim and
  the screen-wide quad are all the filtered colour; then each of the dome's
  vertex colours in place (`0x10079b4f`–`0x10079bb6`), their alphas kept, the
  rim's written after from the fog colour (`0x10079bb8`); and the sky's
  constant white, the nebula's vertex colour, becomes `0xff00ff00`
  (`0x100797a8`) — the same white through the same filter.
- **The sprites.** An effect's sprite, the snow, the rain and a body's sprite
  each carry one colour, and each goes through slot 5.

**What does not.** A light's colour: no reader filters one, and the sun's two
keep slot 19's and slot 21's. A texture: the `0xff00ff00` is a vertex colour
and not a write mask, so a texel's red and blue are lost only by being
multiplied by a diffuse that has none. The fog's range. And everything drawn
outside the camera's render.

**It brightens, by seven.** The filter is not a hue: 0.33 of the sum is the
colour's mean and 7 the gain on it, so a stored colour whose mean is over 36
of 255 comes out at full green, and a lit material at up to 6.93 times a white
one's light. Night vision is what the name in `iron3d.dll` calls it.

**No flare.** With the flag set the sun's draw skips from `0x1007e7ee` to
`0x1007eba3`: both gates, the twelve ghosts and the lift of the main light.

*Seen*, once. In C02 M04, on foot ("Let's Play - Parkan: Iron Strategy, Part
4.5", C9kZR__WpIQ, 3:14.9 to 3:15.5), the HUD's infrared lamp goes white and
the whole view goes green for seven tenths of a second:

| | a second before | under the infrared |
|---|---|---|
| sky | (181, 214, 64) | (0, 253, 0) |
| sunlit ground | (101, 111, 23) | (0, 252, 0), (0, 248, 0) |
| a hill's shaded face | (44, 64, 26) | (1, 226, 1) |
| the tanks far off | — | (1, 238, 1), dark shapes in it |

Red and blue are 0 or 1 everywhere in the view; the ground keeps a faint
mottling and the hill its texture's grain, a plant stands out darker; and the
HUD is untouched — the radar's enemy dot stays (220, 46, 4), the weapon list's
*OUT OF RANGE* red, the two model views their own greens. The sky's 2.31 × 459
and the ground's lit sum are both far past the byte, which is the saturation;
the hill, off the sun, is its texel's green on a diffuse of 1 plus the
specular's share. The engine at the same hour of the same mission, 150 s in —
sky (176, 210, 55), ground (101, 103, 33) — reads (0, 253, 0) and (0, 252, 0)
with its switch on.

*Measured*, how often: one frame every 3 s of all 19 recordings, **8107
frames**, of which **4995** show the cockpit's indicator row (the figure or the
palm lit, [35-hud.md](35-hud.md#the-indicators--read-and-seen)). The infrared's
lamp is lit beside dark repair and camouflage lamps in **1**, that one, and it
is the only frame whose view is green alone: seven others are more than 0.6
green-only and none more than 0.66, five of them cockpit frames with the lamp
dark and two with no cockpit — the one looked at, Part 10's at 9:33, is a room
under a green light. The control is the reader's other lamps: repair lit on 48
and camouflage on 48 of training mission 1's 49 cockpit frames, where the
tutorial turns both on. A flip shorter than 3 s between two samples is not
ruled out.

### Where the two lights point — *read*

A `CLightSrc` record is **0x5c bytes**, in an array at the manager's `+0xc`
with the count at `+0x10` (`CLightSrc::CheckLightSrcNo`, `0x1008006e`). A
type-3 light's direction is
the three floats at record `+0x24`, both where the Direct3D light is built
(`0x10030ab3`, against `+0x18` and `+0x30` for the other types) and in the
emboss pass (`0x1002d08d`). Its writer is the manager's **slot 9**,
`0x10080550` — `SetDirection(id, space, x, y, z)`: space 0 stores the vector
as given (`0x100805a8`), space 2 first takes it **into** the frame of the
manager's own object, by the transpose of that object's world placement
(`0x100805be`–`0x10080629`, `g_FastProc`'s `+0x8c` at `0x1008060f`), and any
other space does nothing. This page had space 2 turning the vector *through*
the placement; it is the other way, a world vector brought into the owner's
frame, and the gather brings it out again through `+0x1c` (`0x100803e4`), the
transpose of `+0x8c`
([37-designer.md](37-designer.md#the-previews--read-and-seen)). The sun's own
manager is the atmosphere's, the one object of class 7
([35-hud.md](35-hud.md#the-sky-is-the-one-object-of-kind-7--read)).

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
vtable and flags 0, both written at `0x1007c07d`. The record only reaches the
item at all when the item's own flag word carries `0x10`, the lit flag
(`0x100285a2`), which of the sky's draws only the clouds do, and the clouds
hand over a record of their own with flags `0x40` and fog 5000 to 11380.7
(`0x1007a5c3`–`0x1007a607`). The other draws are unlit, so the shade never
sees them, and they carry their fog factor themselves: a constant specular of
`0xff000000` (`0x100792bc`), alpha 255, which is **no fog at all**
([Fog](#fog)). Bits 0
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
0, and within a group the passes run in order. What a pass does with its own
items is its type: a type-2 or type-5 pass draws them in the order they were
filed, a type-1 or type-3 pass far to near, and a type-0 pass by the vertex
buffer they share
([11-effects.md](11-effects.md#effect-sprites-are-drawn-far-to-near--read-and-seen)).
The sky's pass is type 2, so its five draws go down as it files them.

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
| 1–4 | 2 | 0.5 | 700 | 0.1 – 0.99 | layer 1: a water batch, unless `UseReflections` is on with `UseEMBMReflections` off (`0x1004453b`) |
| 5 | 1 | 0.5 | 700 | 0.1 – 0.99 | a see-through surface (`0x100455b2`) |
| 6 | 3 | 0.5 | 700 | 0.1 – 0.99 | every effect sprite but the camera's own unit's |
| 7 | 3 | 0.2 | 700 | 0.1 – 0.99 | the sprites of the unit the camera rides (`CShade` slot 8, `0x1004294b`) |
| 8 | 2 | 0.5 | 700 | 0.1 – 0.99 | |
| 9 | 0 | **0.05** | **10** | **0.0 – 0.1** | a fifth slot, ordinary node |
| 10 | 2 | **0.05** | **10** | **0.0 – 0.1** | a fifth slot, the cockpit |
| 11 | 0 | 0.5 | 700 | 0.1 – 0.99 | |
| 12 | 5 | 0.5 | 700 | 0.1 – 0.99 | a water batch with `UseReflections` on and `UseEMBMReflections` off (`0x1004452f`); the layer is drawn only while `UseReflections` is on |
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
(`0x1007a08e`), and carry their own fog, from 5000 to 11380.7 (`0x1007a5d4`):
[below](#the-clouds-are-lit-and-fogged-on-a-range-of-their-own--read-and-measured).

### The sky's first draw is a screen-wide quad — *read*

The draw at `0x1007a37a` is built from the viewport rectangle the view hands
back (`0x1007a1a6`): four corners at *z* = 1.0 written into the sky's own
`+0x254`, `+0x260`, `+0x26c` and `+0x278`, six indices, the vertex format at
`0x100a3800`, item flags `0xc` — pre-transformed vertices, the depth test and
the depth write both off. It is filed into group 1 layer 0 like the rest of the
sky and, being filed first, is drawn first within that pass.

**It paints the fog colour** — *read*, and *seen*. The colour is in its vertex
stream, not its material. The quad's stream block is the sky's `+0x1f4`,
filled with the other four (`0x10079661`–`0x10079708`): the positions at
`+0x254` with a stride of `0xc`, **the diffuse at the sky's `+0x70` with a
stride of 0** (`0x10079698`), the specular at `+0x5c`, the `0xff000000` the
unfogged layers share, and one pair of texture coordinates at `+0x64`. `+0x70`
has one writer, the sky's draw itself (`0x10079b3f`), three instructions after
it has made the same value the device's `FOGCOLOR` (`0x10079b33`): the
horizon's colour in the camera's heading, its alpha forced to 255
([Fog](#fog)), after the infrared has had it where that is on. The item's
flags carry no `0x10`, so the shade never sees it and no material colour
reaches it: a pre-transformed item is drawn in the diffuse it brings
(`0x10030043` → `0x1002f2c0`).

Its material block, the sky's `+0x484`, gives it what is left. The only fields
anything writes in it are `+0x20` (1.0, the ambient alpha, `0x10078614`),
`+0x48` and `+0x4c` (both 0, so no texture), `+0x58` (0) and `+0x5c`, the blend
mode: `CShade`'s slot 3 asked with `edx` 0 (`0x100785f5`), index 0 of its
translate table, **mode 0, opaque** — where the gradient's block beside it asks
with 1 and gets `SRCALPHA`/`INVSRCALPHA` (`0x100785a2`). An earlier round
derived the **scene colour** from that block having no colour written in it
anywhere, and it does not: the block's colours never reach an unlit item, and
the colour stream was not looked at.

**It is all the clearing the frame gets** — *read*. `World3D.dll!stdRenderGame`
(`0x10013bd0`) opens a frame with the render device's slot 10 and `edx` 0
(`0x10013ca1`) and then its begin-scene (slot 12, `0x10013cac`), and slot 10
(`Ngi32.dll:0x10007ca0`) is one `IDirect3DDevice7::Clear` with flags **2**,
`D3DCLEAR_ZBUFFER`, a colour of 0 it does not use and z 1.0
(`0x10007d24`–`0x10007d32`). `CCamera::Render` calls the same slot after its
reflection draw. *Measured* over `Ngi32.dll`, the one module that holds the
device: three calls through `+0x28`, of which that one is the device's `Clear`
— the control, since the sweep does find it — and the other two are the
DirectDraw object's as it shuts down (`0x100049a6`) and the sound listener's
(`0x1000ca06`); and of its eleven calls through `+0x14`, three are a surface's
blit, two the frame's presentation with `DDBLT_WAIT` alone (`0x10006e1b`,
`0x10006f51`) and one a copy handed no effects block (`0x10008a58`), which a
colour fill needs. So **the colour buffer is never cleared**, and the quad —
opaque, over the whole viewport, first of the frame — is what stands where
nothing else is drawn. That is also why the one view that draws over a
finished frame has to skip it.

*Seen*, with the frames converted by the video's own matrix. C02 M04's briefing
at 43.5 s ("Part 4", Qqs8_i9IeUU, 16:12.5) looks down about 4° from a height
of 158, by its waypoints, so the dome's rim, at eye height, stands some 67
pixels above the middle of a 1400-pixel frame, and the far ground's edge 100
pixels below the rim at the frame's left and 60 at its right. That band is
**(106, 172, 22)**, row after row, and so is the sky above the rim; the file's
horizon ahead is (107, 172, 25) and its scene colour **(86, 111, 67)**. C03
M02's opening shot ("Part 6", -yNnsqudMzw, 0:49.7), about 12° down from a
height of 100, is one red, (108, 1, 19), from the top of the picture through
the rim to the fogged ground some 140 pixels under it — the shot is still
fading in, at 0.66 of its (164, 1, 30) — where the scene colour would put
(84, 54, 73).

**The draw is gated** (`0x1007a325`): the sky asks the view (interface
`0x12`, `ICamera2`) for its type — slot 24, `0x10083010`, the field `+0x280`
that slot 23 sets and the view's constructor leaves 0 — and **skips the quad
when the type is 1**.

**A camera's type is its name** — *read*. This page called it a mode and could
not find who set it. `CBufferingCamera`'s own word for it is *type* —
*"Invalid camera type"* is the panic of its slot 30, `GetObjectMask`
(`0x100830b0`), which hands out one of three masks by it — and it is set once,
by the camera that owns the view. `CCamera::CCamera` (`0x100839c0`) keeps at
its `+0x1a0` a number it makes of the name it is built with (`0x10083a54`–
`0x10083aa6`): **2** for `REFLECTION_SHIFTED`, **1** for `REFLECTION`, **0**
for any other name or none. Slot 14 of its game-object interface
(`0x100852e0`), called with an argument that is not null, hands that number to
its view's slot 23 (`0x10085365`) beside the render device and the prim buffer,
and ends by zeroing the camera's flag word (`0x1008542e`).

**Who calls slot 14** — *read*. Two callers, and one of them matters.
`World3D.dll` calls slot 14 of every object it makes: `CreateObject` with 0
once the object is loaded (`0x10007d1a`) and again, before it returns, with
the word `SetGameTime` keeps at `0x10032a38` (`0x10007e6e`, the store at
`0x10005144`), and `AddObjectToGame` and its siblings likewise (`0x100080a9`,
`0x10008580`, `0x100092ba`, `0x10009717`). A scene camera is such an object,
class 5 ([22-settings.md](22-settings.md#what-the-loaders-do-and-who-asks-for-them--read)),
and the type it hands its view is the 0 the view was made with. The mirror's
camera is no object of `World3D.dll`'s. The **landscape's** own slot 14
(`0x1001fcd0`, the entry at `0x1009a4a0`) builds it by name (`0x1001fe75`,
`0x1001fec9`), keeps it at `0x100a6004`, calls **its** slot 14 with the
argument it was called with itself (`0x1001ff0e`) — the one call in the game
that gives a view a type other than 0 — and then sets its flag word to 1
(`0x1001fff7`). So:

| type | camera | the sky's quad | the weather | a body | its flare |
|---:|---|---|---|---|---|
| 0 | every scene camera | drawn | drawn | drawn | drawn |
| 1 | `REFLECTION`, the water's mirror drawn into the frame | **skipped** (`0x1007a328`) | not drawn | drawn | not drawn |
| 2 | `REFLECTION_SHIFTED`, the mirror drawn into a texture, which the install runs ([03-terrain.md](03-terrain.md#the-reflection-camera--read)) | drawn | not drawn | **not drawn** (`0x1007d8f6`) | not drawn |

The weather's two draws and `RenderFlare` return at any type but 0
(`0x100759d3`, `0x10076353`, `0x1007d558`), and `CShade` keeps the type of the
camera it is drawing for at its `+0xbcc` (`0x10045f53`–`0x10045f65`), which is
the 0 its water tests ask for
([03-terrain.md](03-terrain.md#how-a-water-face-draws--read)).

A type-2 camera looks straight up out of the water, so its own heading means
nothing: the sky takes the fog colour's heading from the view it was handed as
its parent instead (slot 31, `0x10083120`, the field the reflection draw sets
through slot 32 at `0x10083f2e`; asked at `0x100797ca`–`0x10079812`). And
every camera, whatever its type, runs the sky's `0x800` pass unless its flag
word carries 8 (`0x100845eb`, `0x10084643`–`0x10084655`), which is how the
mirror has a sky at all.

**What sets flag 8** — *read*, and *measured*. The flag word is `CCamera`'s
`+0x164`. `ICamera` — interface 8, the object itself — sets it whole through
slot 13 (`0x10084a70`) and reads it through slot 20 (`0x10084ad0`), and
`Terrain.dll` stores to it in three places and in no constructor: the setter
(`0x10084a79`), the render's own clearing of `0x200` (`0x10084738`), and slot
14's zero above, which is `+0x160` of the interface it is entered on. The
render tests bit 8 twice: set, it skips the sky's pass (`0x100845eb`) and then
the world and the weather's pass with it (`0x10084701`, to `0x100847eb`).
**One call in the install sets it**, `iron3d.dll:0x10055c9b`: the warbot
designer's opening, which writes 9 — 8, and the 1 that lets the eye place the
view — to the game's own scene camera, and its four closings take the 8 off
again (`0x1005149b`, `0x1005183e`, `0x10055d22`, `0x10055eff`;
[37-designer.md](37-designer.md#opening-and-closing--read)). So flag 8 marks
no kind of camera: it is the scene's, for as long as the designer's black
screen stands over it. The mirror's draw is not gated by the bit
(`0x1008458f`–`0x100845aa`, `0x1008468e`–`0x100846a9`), but it begins by
handing the mirror's camera the scene camera's whole word
(`0x10083f0e`–`0x10083f20`), so behind the designer the mirror draws no sky
and no world either.

*Measured*: the install holds 375 calls through `+0x34`. 29 have a constant
carrying 8 pushed, ORed in or loaded since the call before them, and **one**
of those pushes two values, the constant and the receiver, which is all the
setter takes (`ret 8`, `0x10084ac7`); 25 push three or four and the other
three one, six and eight, other interfaces' slot 13
(`analysis/camflags.py`). The control is `0x200`: the same sweep returns
eight, and the one with two values pushed is `CBuilding::Render`'s
`or ah, 2` (`Terrain.dll:0x100599f9`,
[The weather](#the-weather--read-and-seen)).

**The engine** draws the world under the designer and then the designer's
black ground over the whole window, so its picture is the game's; it does not
skip the draw.

*Measured*: the install holds **38** calls through a slot 23 (`+0x5c`). Slot 23
of `ICamera2` takes its value in `edx` and nothing on the stack — it ends in a
bare `ret` (`0x10083002`) — and **36** of the 38 push at least one argument
first, 35 since the call before them and one, `World3D.dll:0x100075bb`, ahead
of a branch it shares with a `+0x58` call: 13 are calls into `Ngi32.dll`'s
`g_FastProc` table with a matrix in `edx` (five in `AniMesh.dll`, three each in
`Terrain.dll` and `MisLoad.dll`, two in `Effect.dll`), ten are `World3D.dll`'s
own, and the rest COM-style calls with `this` pushed. Of the two that push
nothing, `Control.dll:0x10007cda` calls its own object from a jump table with
only the vtable in `edx`, and `Terrain.dll:0x10085365` is the one
(`analysis/slotcalls.py --bare 23`). It carries no constant because the value
is a field. The earlier searches missed it twice over: the view is kept from a
`QueryInterface` whose answer's address is made with an `add`, not the `lea`
[`analysis/slotcalls.py`](../analysis/slotcalls.py) looks for (`0x10083acd`),
and the call reads it at `+0x198` of the interface the method was entered on,
where the constructor wrote `+0x19c` ([09-method.md](09-method.md#searches-that-do-not-discriminate-so-nobody-repeats-them)).
The control is slot 24, the getter, on the same view: of the install's **65**
calls through `+0x60` the same sweep by arity keeps **8**, all in
`Terrain.dll` and all this getter — the sky's two, the snow's, the rain's, the
sun's two, the flare's and `CShade`'s (`--bare 24`).

**The scene's own camera is type 0, so there the quad is drawn.** An earlier
round inferred the opposite, that the world's view is the type-1 one, because
group 1 runs after group 0 and a screen-wide quad with the depth test off
would paint over the finished scene. `CCamera::Render` runs the sky's `0x800`
pass **and ends that render** before it draws the world at all
([03-terrain.md](03-terrain.md#when-it-draws--read)): the quad is the first
thing of the frame, and the world is drawn over it. The `REFLECTION` camera is
the one that cannot have it — its render comes after the scene's own sky pass,
into the same frame (`0x1008468e`–`0x100846a9`), and a quad would wipe it.

### The three layers and their texture coordinates — *read*

The cap builder (`0x100787f0`) lays out, per vertex, one position array
(`+0x34`, 12 bytes), one colour array (`+0x38`, 4) and one **normal** array
(`+0x3c`, 4: the unit vector out from the sphere's centre as three signed
bytes at 127, `0x10078c39`–`0x10078edd` — (0, 0, 1) at the apex) — and then
**three** eight-byte arrays, `+0x40`, `+0x44` and `+0x48`. They are texture coordinates,
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

| block | position | normal | colour | specular | texture coordinates |
|---|---|---|---|---|---|
| `+0xd4` | `+0x34` | none | `+0x38`, per vertex | `0xff000000` | none |
| `+0x74` | `+0x34` | none | a constant white | `0xff000000` | `+0x40`, *K* 15 |
| `+0x134` | `+0x34` | `+0x3c`, per vertex | none | none | `+0x44`, *K* 3 |
| `+0x194` | `+0x34` | none | a constant white | `0xff000000` | `+0x48`, *K* 1 |

A block's pairs are, in order, the position, the normal (`+8`), the diffuse
colour (`+0x10`), the specular colour (`+0x18`) and the two texture stages'
coordinates (`+0x20`, `+0x28`), which is how the shade reads them
(`0x1004e743`–`0x1004e781`). An earlier reading here had `+0x3c` as the
clouds' colours: it sits in the **normal** place of its block
(`0x100794ac`–`0x100794bb`, pointer at `+0x13c`, stride 4), and the block has
no colour stream at all — the shade makes one for it. The constant white is
the sky's `+0x6c` (`0x100792f5`) and the constant specular its `+0x5c`
(`0x100792bc`), each handed over with a stride of 0.

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
no slot: their texture fields are zeroed and only their blend modes asked for,
of `CShade`'s translate table at index 1 and index 0 (`0x100785a2`,
`0x100785f5`; an earlier reading here took those two calls for texture
handles), so those two draws are untextured — the vertex-colour dome, blended
on its alpha, and an opaque quad over the viewport.

So **the nebula is drawn first and the coloured dome over it**, which is what
makes the dome's alpha matter.

### The clouds are lit, and fogged on a range of their own — *read*, and *measured*

The cloud layer is the one sky draw filed **lit** — item flags `0x14`
(`0x1007aaf1`) — so it goes through the shade like a mesh
([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)):

- its **material** is `sky.wea`'s slot 2 with the diffuse overwritten by the
  keyframe's cloud colour and the ambient zeroed (`0x1007a4de`–`0x1007a5b3`).
  The ambient alpha is the entry's own, and *measured* it is 1.0 on all three
  cloud materials the 29 missions name (`ENV_CLOUDS` on 12, `ENV_CLOUDS_2` on
  10, `ENV_CLOUDS_4` on 6; the 29th names `TOK51`, a 16-pixel texture whose
  alpha is 0 throughout, which is no clouds at all);
- its **normals** are the cap's own, out of the sphere and so upward. A vertex
  takes whichever of the sun object's two lights comes down at it — the main
  one while the body is above the horizon, the second while it is below — on
  the cloud colour, and the scene colour is the floor under that;
- its **lights** are whatever list the shade last gathered: the item is
  filed with the list the shade hands out (`0x1002863d`, slot 24), and the
  sky's draws are filed after the world's (`CCamera::Render` asks the world
  first, `0x1008477e`, and the queue's second drawable with pass flags
  `0x1000` after it, `0x100847e8`). That list is the last drawn object's or
  cell's, with the sun's two lights in it; *which* object's it is, and so
  which point lights ride along, is **not read** — and *measured*, it does
  not matter to what is drawn. The lighter passes over a vertex past a
  light's range (`Ngi32.dll:0x100165bb`), and the cap's nearest vertices stand
  5000 over the eye, the apex, and 4580 over it 5341 out, the first ring.
  Of the 618 light blocks the largest range is 300 (`r_lightray_w`) and no
  other block's passes 162.5 with its header's scale; an instance's frame
  stretches it, and of the lights standing on C03 M02 at briefing time 61.8 s
  the largest comes to 214, `mineglow`'s 7 through a drill frame 30.6 long.
  **Only the lightning's comes near**: `env_lightning`'s 100 ± 5
  through a bolt frame 40 wide is 4000 ± 200 as the flash starts, on a light
  standing 300 over the ground it strikes
  ([Lightning](#lightning--read-measured-and-seen)). That reaches the apex
  only from ground 500 above the eye at the eye's own place, where the eye
  could not be, and a first-ring vertex only from a strike 5341 away on
  ground more than 80 above the eye, and there it gives the square of what is
  left of its range: under a hundredth of its colour for 300 of relief. The
  clouds' material has no specular either, power 0 on all three, so no
  highlight reaches them;
- its **fog** is its own record's: from 5000 — the dome's height less the
  drop, `0x10077c43` — to a third of the sphere's radius, 11380.7
  (`0x10077c73`), toward the frame's fog colour, the horizon ahead. The
  distances are from the eye, which stands 5000 over the cap's origin:

| vertex | distance | fog keeps |
|---|---:|---:|
| apex | 5000 | 1 |
| ring 1, 40.6° up | 7035 | 0.77 |
| ring 2, 17.5° up | 11063 | 0.07 |
| ring 3, 4.7° up, and out | 15553 + | 0 |

  So the layer is itself overhead and **the horizon's own colour from the
  cap's third ring out to the rim**, and between the rings the device carries
  the two vertex colours across the face.

Its texture is blended on its own alpha, mode 4. *Measured*: `S_03.0`,
`S_05.0` and `S_06.0` are 256-pixel `ARGB4444`, so the alpha has 16 levels —
`S_05` is 0 on 66% of its texels and 35 of 255 in the mean, `S_06` is never 0
and 207 in the mean, a nearly solid sheet. A fogged vertex keeps its alpha,
so below the second ring a solid sheet **paints the fog colour over the
dome's gradient**, and the lower sky is one smooth colour.

*Seen*, with the frames converted by the video's own matrix. C02 M04's
briefing at 43.5 s ("Part 4", Qqs8_i9IeUU, 16:12.5) looks level, and its sky
is one green, (105, 169, 22), where the fog colour ahead is (107, 172, 25);
C03 M02's at 29 s ("Part 6", -yNnsqudMzw, 1:11.7) is one red, (158, 8, 21).
Drawn unlit and unfogged, as the engine had it, the first showed yellow
streaks with stepped edges across that green and the second a grey-brown
band; lit and fogged the engine's sky reads (107, 171, 24) and (161, 9, 22).

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

### The frame holds what the files hold — *read*, and *measured*

The game is Direct3D 7's fixed function, and nothing in it decodes or encodes
a colour: a texel, a vertex colour, the fog colour and the frame are all the
bytes as stored, and every multiply, add and blend is done on them. So the
arithmetic of a fogged, textured vertex is, with *T* the texel, *D* and *S*
the vertex's diffuse and specular, *F* the fog colour and *k* what the fog
keeps:

```
frame = k × min(T × D + S, 1) + (1 − k) × F          all of them stored values
```

The engine used to draw the scene into an sRGB frame from sRGB textures —
decoding *T*, *D* and *F* to linear, blending there, and letting the frame
encode the result — and held that this gave "the game's display-space
blends". It does not. A product nearly survives the curve, since
*x*^2.2 × *y*^2.2 = (*xy*)^2.2, and a sum does not:

| *T*, *D* | *F*, *k* | the game writes | decoded, blended, encoded |
|---|---|---:|---:|
| 0.3, 0.5 | —, 1 | 0.150 | 0.132 |
| 0.3, 0.5 | 0.675, 0.5 | 0.4125 | 0.500 |

— a dark ground texel half lost in C02 M04's green fog came out **22 levels of
255 paler**, and unfogged 5 darker. Alpha blends go the same way: a see-through
surface over a dark one is lighter blended in linear.

The scene is now drawn as the HUD and the effects already were: into the
frame read without sRGB decoding, from textures read as stored, with the
files' colours as they are.

### The lit colour is the game's own — *read*, and *measured*

**No device material is ever built, and no device light.** A draw item that
asks to be lit carries `0x10` in its flags: every mesh batch
([11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)),
the landscape's cells (`0x414`, `0x10044627`,
[03-terrain.md](03-terrain.md#how-the-landscape-is-lit-and-fogged--read-and-measured))
and the clouds (`0x14`). `CStridedPrimitive::RenderVB` (`0x1002fab0`) tests
the flag (`0x1002fe3d`) and then the setting `UseDXLighting`, entry 29 of the
page (`0x1002fe50`). **It is 0** — compiled so (`0x1005fcce`), and nothing
writes it ([The render settings](#the-render-settings)) — so the render calls
slot 8 of the shader component, `CShade::ShadeIndexedStrided` (`0x1004df70`,
named by its panic strings), clears the flag (`0x1002ffbc`), and the device
draws the item with its lighting off, from a buffer of position, diffuse,
specular and texture coordinates (`0x1002f995`). `Ngi32.dll`'s reset leaves
`SPECULARENABLE` at 1 (`0x10006c38`, the state's cache at `+0x190` taking the
1 in `ebx`), so what the device does with a vertex is: texture stages by the
diffuse, **plus the specular**, fogged by the specular's alpha.

**What the shade writes on a vertex.** It merges the item's own record into
the scene's (`0x1004df7b`), copies the material's diffuse, ambient, specular
and power out of the item's block (`0x1004e026`–`0x1004e0a7`), puts the eye
into the item's frame, and for each vertex keeps the unit vector to the eye
and its length (`0x1004e7b1`–`0x1004e8ea`). Then:

1. **The lights**, while the state's lighting flag is on and the material has
   a diffuse that is not black or a specular with a power (`0x1004e99f`–
   `0x1004ea30`): each light of the item's list, a point light through
   `g_FastProc`'s `+0xc4` and a directional one through `+0xc8`. The
   directional routine (`Ngi32.dll:0x10016070`) takes the cosine of the
   vertex's normal — three signed bytes at 127 — against the way back along
   the light's travel, passes over a vertex it is not above 0 on, and adds
   **light colour × material diffuse × cosine** to the vertex's diffuse sum.
   Where the item wants highlights it mirrors the light's travel in the
   normal, takes that against the vector to the eye, and adds **light colour ×
   material specular × that cosine squared `power − 1` times** to the
   specular sum (`0x1001616f`–`0x100161e9`): exponents 4, 8, 16 and 32 for
   the shipped powers 3, 4, 5 and 6. With the lighting off, or nothing to
   light, the diffuse sum is the material's diffuse itself (`0x1004ee3a`).
   **A point light leaves a highlight the same way** (`Ngi32.dll:0x100166b5`–
   `0x10016750`, read through): with *L* the vector from the vertex to the
   light and *n* its normal, the routine forms 2 (*n* · *L*) *n* − *L*, the way
   to the light mirrored in the normal, takes it against the vertex's unit
   vector to the eye, passes over a vertex it is not above 0 on (`0x10016705`),
   divides by the length of *L* (`0x10016715`), squares it `power − 1` times
   (`0x10016719`–`0x10016727`) and multiplies by the light's falloff
   (`0x1001672c`) before the light's colour times the material's specular is
   added in (`0x1001674a`). So a point light's highlight fades with its range
   as its diffuse does, and a directional light's does not fade.
   **Which lights leave one.** The shade asks a highlight of a light only
   where the item wants one and the light's active word `+0x4c` has bit 1
   clear (`0x1004ec03`–`0x1004ec38` for a point light, `0x1004ed81`–`0x1004edb7`
   for a directional one), and a light carrying the bit is passed over
   altogether on an item that takes no diffuse (`0x1004ea78`–`0x1004ea97`).
   The bit is `D3DLIGHT_NO_SPECULAR`; the light manager's slot 18
   (`0x10080a60`) clears it for a non-zero argument (`0x10080ab8`) and sets it
   for 0 (`0x10080aea`), and a light is made with the word 0 (`0x1008076a`).
   **`CSun`'s constructor calls slot 18 with 0 on its second light and on no
   other** (`0x1007d023`–`0x1007d03b`, between the second light's flags at
   `0x1007d020` and the two switches at `0x1007d056` and `0x1007d071`), and
   `Effect.dll` never calls it — its light emitter's calls on the manager are
   slots 12, 13, 6, 11, 8, 9, 3 and 4 (`0x1000f694`–`0x1000fbb8`), and the
   module holds no call through `+0x48` at all, where the same sweep finds 91
   through `+0x18` and `+0x34`
   ([11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).
   So **the sun's first light and every effect's light leave a highlight, and
   the counter light lights a diffuse and nothing else.**
   **A light's direction goes into the item's frame by its 3 × 3 alone.**
   Where the frame's kind word (`+0x6a`) is 1 the direction is used as the
   record holds it (`0x1004ed1d`); otherwise it goes through `g_FastProc`'s
   `+0x8c` (`0x1004ed7b`), which multiplies it by the transpose of the
   frame's 3 × 3 and does nothing else. **All five builds of the slot are
   read**, and none takes a root or a reciprocal: x87
   `Ngi32.dll:0x10024260` and P6 `0x1001f910` are
   nine products, six sums and three stores, 3DNow! `0x1001ba50` the same in
   pairs, and the two SSE ones, `0x1001ca90` for an aligned matrix and
   `0x1001ca00` for any, three `mulps` of the vector's components by the
   matrix's first three rows and two `addps`. It is the inverse of a
   rotation, and on a 3 × 3 scaled by *s* it would be a vector *s* long,
   which the directional routine takes its cosine against as it stands
   (`0x1001612c`–`0x10016143`). **No scaled object's 3 × 3 is scaled**,
   though: the frame keeps its scale beside its matrix, and one pose key in
   the install is all that stretches one (below).
   A point light's position goes through the frame's inverse on a frame of
   kind 4 or 8 — `+0x9c`, `Ngi32.dll:0x100154e0`: the frame's translation
   taken off, the same transpose, and a factor an axis from the frame's
   `+0x5c` — through `0x1002e0c0` on kind 2, and as it stands on kind 1 (the
   table at `0x1004f6cf`); its range is multiplied by the frame's `+0x5c`
   (`0x1004ead5`) and the range's reciprocal by its `+0x50` (`0x1004ec5f`).
   Kinds 3, 5, 6 and 7 convert no position at all, and no frame of those
   kinds is ever made (below).
2. **The ambient and the floor**: `g_FastProc`'s `+0x50` with the sum, the
   material's ambient and the state block's colour (`0x1004f225`–`0x1004f23d`).
   All four builds of it — x87 `Ngi32.dll:0x100248a0`, 3DNow! `0x1001bdd0`,
   SSE `0x1001d980`, P6 `0x1001ffc0` — **add the ambient to the sum and then
   take the larger of that and the scene colour**: `fadd` then a compare and
   a store of the third argument where the sum is below it; `addps` then
   `maxps`. So

   ```
   lit = max(material ambient + Σ light colour × material diffuse × cos θ,  scene colour)
   ```

3. **The knee**: a channel is as it is up to 1, *c* ÷ 6 + 5 ÷ 6 up to 7, and
   2 beyond (`0x1004f26d`–`0x1004f3cd`; the constants at `0x100a2354`,
   `0x100a2360` and `0x1009b35c`).
4. **The overbright goes to the specular**: what a channel then has over 1 is
   added to the specular sum and the diffuse held at 1
   (`0x1004f3e5`–`0x1004f4a5`); and the specular sum is scaled in its turn,
   0.8 of it up to 1, 0.1 of it plus 0.7 up to 3, and 1 beyond
   (`0x1004f4c3`–`0x1004f648`).
5. **The alphas**: the diffuse's is the material's ambient alpha times the
   item's own factor (`0x1004f3d0`) — 1, a distance fade, the vertex's alpha
   stream or their product, by the record's mode (`0x1004f0b8`, the table at
   `0x1004f6ef`); the specular's is the fog factor ([Fog](#fog)).

The two colours are packed to bytes (`+0x54`, called at `0x1004f69f`) into
streams the shade gives the item (`0x1004e246`–`0x1004e3a1`), so the colour is
a **vertex's** and the device carries it across the face. The packing
multiplies each component by 255 (`Ngi32.dll:0x10037230`, four of them) and
converts it to a whole number by the processor's rounding, to the nearest and
a tie to the even one. The SSE builds then hold it to 0..255 (`cvtps2pi`,
`packssdw`, `packuswb`; `0x1001d940` and `0x1001d8f0`), and the x87 build
keeps the low byte of what `fistp` stored (`0x100247e0`), which is the same
byte for everything the knees let through: the diffuse is held at 1 and the
specular's knee ends at 1.

A lit vertex is therefore never darker than the scene colour and never
brighter for it; a light past 1 whitens what it falls on, by up to 0.8 of a
channel; and a batch whose lightmap has made its diffuse its self-light
([07-objects.md](07-objects.md#how-a-lightmapped-batch-is-drawn--read-and-measured))
shows the larger of that colour and the scene's.

**The frame keeps its scale beside its matrix** — *read*. What the shade is
handed as an item's frame is the class the binary's one panic for it names
`N3DSCALEDMATRIX` (*"Invalid matrix class in N3DSCALEDMATRIX"*, `0x100a1644`):
a pointer to sixteen floats, a scale an axis at `+0x50`, its reciprocals at
`+0x5c`, a word at `+0x68` and the **class** at `+0x6a`, the kind word above.
Its constructor from a matrix and a class (`0x100291a0`; the sky's draw
carries a copy inline, `0x1007a62e`–`0x1007aae7`) measures the length of each
of the matrix's three axes on class 8, or of the first alone on class 4, keeps
them as the scale and **divides them out of the matrix**, each axis by its
reciprocal (`0x10029416`–`0x100294e4`; `0x10029900` is an axis by a factor),
so the 3 × 3 it leaves has unit axes. Four classes are ever made:

| class | made by | its 3 × 3 |
|---:|---|---|
| 1 | two statics, `0x100a5f80` (its class set at `0x10049073`), which `CShade`'s cell draw copies into a landscape cell's item (`0x1004486f`–`0x100448dd`), and `0x100a60f8` (`0x10054b83`), copied by slot 4 of the vtable at `0x1009b160` (`0x10052630`, at `0x1005410f`–`0x1005417d`), a routine on the shade's own fields whose draw is not read here; and three built in place (`0x1003c60f`, `0x1003cb26`, `0x1003d096`) | the identity |
| 2 | the sky's draw, for its two dome layers (`0x1007a398`, `0x1007a42e`, the matrix at `0x100a71d8`) and the clouds (`0x1007a62e`, the matrix at `0x100a7218`); `CShade`'s slot 32 (`0x10029107`) | the identity: both sky matrices are made the identity once (`0x1007a0be`–`0x1007a16e`, `0x10079fb4`–`0x1007a064`) and take three floats of translation a frame (`0x1007a183`–`0x1007a1a0`, `0x1007a079`–`0x1007a097`) |
| 4 | `CShade`'s mesh draw, slot 21 (`0x10044ea0`), through a constructor of its own from a matrix and one float (`0x10048f50`, called at `0x10044eeb`): the matrix **as it is handed**, the float as all three of the scale, its reciprocal beside it | the mesh node's |
| 8 | `CShade::RenderEffect` (`0x100288ab`) | unit axes, the instance's size divided out; an effect's sprite is no lit item ([11-effects.md](11-effects.md#how-an-effect-sprite-is-coloured--read-and-measured)) |

A word at `+0x6a` has 81 accesses in `Terrain.dll` — five of them in a
library routine `iron3d.dll` and `Behavior.dll` carry too — and none in
`Ngi32.dll` or `AniMesh.dll`; its stores there are those and five copies from
another frame (`0x1002859b`, `0x1002879b`, `0x100299ca`, `0x1002beee`,
`0x10082dfa`). `Effect.dll` carries a copy of the constructor (`0x1000c5f0`),
whose one call makes a class 2 (`0x1000bece`); the two other stores of such a
word in the install, `ai.dll:0x10002a5e` and `Behavior.dll:0x10036311`, are
in modules that hand the shade nothing and were not read. So classes 3, 5, 6
and 7, for which the shade converts no light's position, do not occur.

**A scaled mesh's matrix is not scaled** — *read*. The mesh's draw hands
slot 21 a node's matrix, the node record's `+0x20`, and a pointer to the
mesh's own scale (`AniMesh.dll:0x10014fad`–`0x10014fc0`), of which the shade
takes the first float (`0x10044ec0`). That scale is three floats at the mesh's
`+0x158`, with a byte at `+0x154` that says they are not (1, 1, 1): the
constructor makes them (1, 1, 1) (`0x10006970`–`0x10006999`, from a static
its initialiser fills, `0x10007cd0`), and `SetScale` (`0x10014770`, slot 15 of
the interface at the mesh's `+4`) is their one writer. `SetScale` and the pose
walk multiply **the translation of each node's matrices** by the scale and
touch nothing else (`0x10014893`–`0x1001493d`; `0x10008c9d`–`0x10008d83`,
`0x10008ee7`–`0x10008fcd`): a tree placed at 3 has its nodes three times as
far apart and each node's 3 × 3 the rotation it had. The frame's scale is what
sizes it, and what the shade wants of it is a point light's reach — a range
times `+0x5c`, a position through the inverse and `+0x5c`. A directional light
has no use for it, so **a scaled object is lit as an unscaled one**. That is
a read: no recording's frame of a scaled tree or stone was measured for it.

**What a node's 3 × 3 is made of** — *read*, and *measured*. A node's matrix is
its pose keys' rotations down the chain, under the object's placement. A key's
four `int16` are multiplied by 1 ÷ 32767 (`AniMesh.dll:0x10012880`, the factor
made at `0x10013030`); two keys are blended by a spherical interpolation that
turns linear where their cosine is within 0.00001 of 1 (`g_FastProc`'s
`+0x58`, `Ngi32.dll:0x10017f50`); and the result is made a matrix by `+0x38`
(`0x10017e60`), 1 − 2(y² + z²) on the diagonal and 2(xy ± wz) off it, which
**normalises nothing**. For a quaternion of squared length *n* that is

```
M = (1 − n) I + n R        R the rotation of the same quaternion made unit
```

which leaves the axis alone and, across it, turns by the angle of
(1 − n) + n e^(iθ) and stretches by its length, √(1 − 2n(1 − n)(1 − cos θ)).
Over the install's **34049 keys in 435 meshes**:

| keys | | the game's matrix |
|---:|---|---|
| 34038 | within 0.1% of unit length | a rotation, to within 0.00017 |
| 9 | all zero | the identity |
| 1 | `R_M_03.msh` key 78, (0, 0, 71, 0) | the identity, to within 0.00001 |
| 1 | `o_tur_ba_09.msh` key 20, (−32767, 0, 0, −32767) | **turned 116.57° about z and stretched √5 = 2.236 across it** |

All eleven stand where a full turn wraps round — the six wheels of the Medium
chassis `R_M_03` at frame 19 of its 21, the dishes of `R_l_06`,
`o_tur_la_06` and `o_tur_ba_10` — and the last is the node `BTHQ_m1o1` of
`o_tur_ba_09`, the mesh of the Large turret 09 (`e_tur_bb_09`, `e_tur_bt_09`),
at frame 47 of its 102, between a (0, 0, 0, −32767) and a (0, 0, 0, 32767). It
is **the one shipped frame whose 3 × 3 carries a scale**: round that frame the
node would be drawn up to 2.236 times its size across its own z, and take that
much of a directional light. The control is the same formula on a quaternion of
squared length 4, which gives the 5 its matrix's first axis measures
(`openparkan verify`).

**No controller plays it, or any of the eleven** — *measured*. A node with a
channel plays that channel's frames from its `first` to its `last`
([07-objects.md](07-objects.md#how-the-engine-plays-it--read)); what else a mesh plays
is its controller's state pairs. The Large turret 09 has two records, and both
controllers give `BTHQ_m1o1` one channel of frames **42 to 46**, the radar's dish
([25-sensors.md](25-sensors.md#the-dish-turns-while-the-radar-runs--read-and-measured)),
and no state at all. Frame 46 is a key of its own, (0, 0, 0, −32767), so the walk's "takes
that key or the next one where *f* equals its time" never interpolates on towards 47. The
frames of the run after the dish's are its author's closing of the turn: all eleven keys
stand **one frame past the channel that turns their node**, the six wheels of `R_M_03` at 19
against 14 to 18, `Radr_m1o1` of `R_l_06` at 11 against 6 to 10, `o_tur_ba_10`'s two at 6
against 1 to 5 and `o_tur_la_06`'s at 7 against 2 to 6. Of the 15 pairs of a key and a
record that names its mesh, **0** lie within a channel's or a state's frames, where the frame
before each lies within one on all 15, the control (`openparkan verify`).

**So no drawn frame's 3 × 3 is stretched, and the turn the readers take of those keys is
never drawn either.** The rest of a scale's ways in:

- **The missions' 864 placements** put no scale in a matrix: the loader's is a turn about z
  and a translation, and the scale 216 of them carry — 134 trees and 82 stones, 0.2 to 21 —
  goes beside it to `SetScale` ([04-missions.md](04-missions.md#the-scale)).
- **`SetScale` is called on scenery alone** — *read*. Its slot takes its object on the stack
  and a pointer after it. Of the install's 117 calls at `+0x3c`, 39 have that shape; those on
  an interface `0x18` are four, one in each of `World3D.dll`'s `AddNewObjectToGame`
  (`0x1000842a`, the published one and so the control), `CreateMirrorObject` (`0x100091bd`),
  `AddMirrorObjectToGame` (`0x100093ab`) and `AddNewMirrorToGame` (`0x100095d8`), each of the
  scale in a block its last argument points at. The three mirrors no module imports. Of
  `AddNewObjectToGame`'s six callers, all in `iron3d.dll`, three pass no block (`0x100366b4`,
  `0x100a2322`, `0x100a24b5`), one passes (1, 1, 1) for a type 11 (`0x10039249`), and the two
  that pass a mission's scale are the two kinds of scenery's, type 10 (`0x100a4334`,
  `0x100a43e4`). The building's slot only hands on to its mesh's (`Terrain.dll:0x10056bf0`).
  **Nothing calls `SetScale` on a unit.**
- **The placements `Control.dll` gives a unit and its parts** — *read*. `PlaceObject` is
  `IGameObject` slot 7, `+0x1c` (`AniMesh.dll:0x10017790`, table `0x10020d28`), and of the
  install's calls of its shape nine are in `Control.dll`. Six change only a matrix's
  translation: the machine's own `+0x2e4` as it steps and is pushed (`0x10005a85`, `0x1000c93c`,
  `0x1000cb6b`), the body's follow of the ground (`0x10015f0f`) and a copy of another's placement
  moved by a point (`0x10030a54`, `0x10030ab4`). Two are a camera's (`0x10023879`, `0x10023896`).
  The ninth is the walker's turn (`0x100194e7`), a pivot's translations about the matrix
  `g_FastProc`'s `+0x38` makes of a quaternion built from an angle's sine and cosine. No site
  multiplies an axis by anything; the `+0x2e4` itself is `GetPlacement`'s answer or one of
  these (`0x10007a48`, `0x1000c898`, `0x100194cd`, `0x1003189f`), or what `0x10004690` is
  handed whole, whose callers this round did not follow.
- **The two other stores of a word at `+0x6a`** are no frame the shade is handed — *read*:
  `ai.dll:0x10002a5e` is a static identity of class 1, and `Behavior.dll:0x10036311` no matrix
  at all, a 7 beside a `0xf` in an object of −1s and zeros (`0x10036290`).

*Not set against a frame*: that a scaled object is lit as an unscaled one. C00 M01's hero
starts 304 and 319 m from its stones at 16 and 20, under a main light of (0.7, 0.11, 0) and a
second of (0.45, 0.45, 0.63) as the mission opens; multiplied by 16 either saturates, and
`STONE00`'s texels stand at (127, 114, 89) on the mean, where the rock at the left of the frame
at 6:40 of "Part 1" (9_rcJkRzgZQ) measures (33, 20, 21) over its area. But which rock that is
was not established against the map: it does not plainly wear `STONE00`'s picture, and may be
the landscape's own.

**The engine** took a unit normal against a unit direction whatever the scale,
which is the game's on every scaled object, so nothing changed there. Its
readers normalised a key that was not unit, and that made a **half turn about
y** of (0, 0, 71, 0), the left middle wheel of `R_M_03` at the frame where its
five siblings hold a zero and the game's matrix is the identity. Both readers
now take the turn of the matrix the game builds, the identity there and
116.57° on the turret's key. The stretch they leave out, and no controller
plays it.

*Measured*, the engine before and after following this — with the fog and the
[ground's materials](03-terrain.md#the-uv-unit-is-1024--read-and-measured)
beside it — against the recordings' frames converted by the video's own
matrix, mean colours over the same areas:

| frame | area | recording | before | after |
|---|---|---|---|---|
| C02 M04 briefing 43.5 s | near hill | (76, 115, 26) | (90, 129, 30) | (78, 118, 28) |
| | the hill behind it | (35, 57, 11) | (91, 136, 27) | (37, 61, 13) |
| | far hill | (77, 123, 20) | (99, 160, 24) | (79, 126, 21) |
| | sky | (105, 169, 22) | (116, 176, 22) | (107, 171, 24) |
| C02 M01 briefing 57.5 s | sky | (95, 150, 20) | (105, 164, 24) | (96, 151, 22) |
| C03 M02 briefing 54 s | cracked ground | (59, 7, 10) | (89, 8, 15) | (57, 11, 14) |
| | sky | (127, 12, 23) | (158, 12, 27) | (133, 14, 26) |
| C03 M02 briefing 29 s | sky | (158, 8, 21) | (102, 20, 20) | (161, 9, 22) |
| C03 M02 cockpit, 79 s in | sky | (169, 38, 41) | (129, 62, 53) | (157, 39, 53) |

**The counter light's highlight was this engine's, not the game's** —
*measured*. C03 M02's sun has just risen through its briefing: at 54 s the
first light is (0.74, 0.03, 0.11) travelling north along the ground and the
second (0.79, 0.46, 0.82) travelling south, over a scene colour of
(160, 42, 62). The engine took a highlight from both, and on every material
with a specular the second's came out lavender on whatever faced north with
the eye in its mirror. Without it, and with the point lights' highlights the
shade does take, against the recording ("Let's Play - Parkan: Iron Strategy,
Part 6", -yNnsqudMzw, 1:31.6, 1:36.7 and 1:44.5), mean colours over the same
areas:

| frame | area | recording | before | after |
|---|---|---|---|---|
| briefing 54 s | the slope Enemy 1's Medium Mine turns to the camera | (139, 13, 23) | (191, 44, 74) | (142, 17, 27) |
| | its tower | (97, 11, 41) | (116, 21, 58) | (102, 13, 44) |
| | the cracked ground before it | (84, 8, 10) | (86, 10, 12) | (86, 10, 12) |
| briefing 61.8 s | Enemy 2's Large Factory, first mast, its north face | (94, 23, 43) | (192, 78, 138) | (99, 26, 47) |
| | the face the camera looks at | (142, 15, 26) | (168, 15, 27) | (168, 15, 26) |
| | its south face | (224, 90, 47) | (150, 15, 25) | (230, 89, 44) |
| briefing 48.9 s | Enemy 1's Large Factory, first mast, its north-west face | (103, 25, 46) | (180, 70, 122) | (105, 28, 51) |
| | its south face | (236, 116, 51) | (140, 14, 24) | (236, 95, 44) |

The mine's slope is the face an earlier round's queue called the Outpost's;
the building is `mmine01.dat`, and 25 of its mesh `fr_m_mine`'s 305 batches
wear a skin whose specular is (255, 245, 239) at power 3, `B_PLT_02` and
`B_PLT_06` among them, where 236 carry (10, 10, 10) or (11, 11, 11). Which
batch the slope is was not picked out. The masts' own skin is `B_PLT_01`, with
the same specular, and what turns their sun side yellow is not the sun
([11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).

**Not followed here**: the engine gathers one list of point lights a frame
([11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)).
It leaves out the one stretched pose key's stretch, and rightly: the Large turret
09's `BTHQ_m1o1` is drawn and lit at its own size through its channel, which stops short of
its one stretched key.

### Fog

**Fog is on for the whole scene.** `CShade`'s constructor sets `FOGENABLE`
(`0x10042070`); only the 2D overlays switch it off and back on.

**It is the shade's own, and linear in the squared distance.** The device is
never given a fog mode, a start or an end: those calls sit in the routine that
builds a device material (`0x10030620`, `0x10030f20`), which has one caller
(`0x1002ff27`) behind `UseDXLighting`, and that setting is 0
([below](#the-lit-colour-is-the-games-own--read-and-measured)). With no vertex
or table mode set, the device takes each vertex's fog factor from **the alpha
of its specular colour**, and the shade writes it there
(`0x1004f187`–`0x1004f213`, into the vector packed at `0x1004f64b`):

```
keep = 1                                          d² < start²
       0                                          d² > end²
       1 − (d² − start²) ÷ (end² − start²)        between
```

with *d* the vertex's distance from the eye, which the shade has already
measured for every vertex (`0x1004e7f8`–`0x1004e8ea`), and the two squares and
the reciprocal of their difference kept by `0x1004bfb0`. The start and the end
are the item's own record's where it carries mask `0x40`, the clouds' alone,
and the scene's otherwise:

- the sky writes the scene's record in two places: the word at its `+0x24`,
  3, when it is built (`0x10078689`) — the shade asks only that it is not 0
  (`0x1004f187`) — and start 700 × property 13 and end 700 × property 14,
  every update (`0x1007bbc5`);
- with slot 5 always 0, **fog starts at 0 and ends at 700 × slot 6**, which
  is 70 to 700 units (*measured*). On Mission 01 it ends at 420, 490, 525,
  560 or 700.

So the fog keeps 1 − (*d* ÷ end)²: at half the range three quarters of a
surface, where a fog linear in the distance keeps half; at three quarters of
it 0.44 against 0.25. An earlier reading here had the device's own linear
vertex fog, `D3DFOG_LINEAR` with `RANGEFOGENABLE`: that is what `0x10030f20`
would ask for, on the path the shipped setting never takes. An item that is
not lit skips the shade and carries its factor in its own specular stream:
the effects' sprites one the shader component makes them, by the same
squares ([11-effects.md](11-effects.md#how-an-effect-sprite-is-coloured--read-and-measured)),
the dome's gradient and the nebula a constant 255, no fog.

- The only fog states `Terrain.dll`, `World3D.dll` and `Ngi32.dll` set are
  `FOGENABLE`, `FOGCOLOR`, `FOGSTART`, `FOGEND`, `RANGEFOGENABLE` and
  `FOGVERTEXMODE`; none pushes `FOGTABLEMODE` ahead of a call. Of
  `FOGVERTEXMODE`'s nine sites in `Terrain.dll` one is `0x10030f41`, on the
  path not taken, and the other eight are the four pass renders saving the
  state as they begin and putting it back as they end (`0x1003dab0` and
  `0x1003dc93`, `0x1003de30` and `0x1003e013`, `0x1003e290` and `0x1003ef31`,
  `0x1003f1c9` and `0x1003f3fb`).
- **`ForceSWFog` is never read, in `Terrain.dll` or anywhere else.** See
  [below](#nobody-reads-forceswfog--read-and-measured). It has nothing to
  switch: the software fog is the only one.

*Measured* against C02 M04's briefing at 43.5 s ("Part 4", Qqs8_i9IeUU,
16:12.5), where the camera stands about 290 units from the generator it
looks at and the fog ends at 498.5 in a colour of (107, 172, 25). A fog
linear in the distance would keep 0.42 there, and leave no surface a green
under 0.58 × 169 = 98; the generator's body in the recording is (64, 77, 16),
(57, 68, 15) and (65, 90, 16) over three patches. The squared fog keeps 0.66,
a floor of 57. The engine's linear fog gave (94, 142, 21) on the same
patches, and with the squares it gives (70, 84, 18), (61, 70, 17) and
(64, 83, 18).

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

<a id="the-scene-colour-is-added-to-every-material"></a>

### The scene colour is a floor under every lit vertex — *read*, and *measured*

The same record carries a colour: property 16, file slot 20
(`0x1007bbc5`). **It is not added to anything.** The shade adds the material's
ambient colour to what the lights gave the vertex and then takes, channel by
channel, **the larger of that sum and the scene colour**
([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)).
So it is the darkest a lit surface can be — 40/255 grey at Mission 01's
brightest keyframe, a brighter violet at night — and a surface the lights
reach past it shows nothing of it. This page used to say "emissive = that
colour + the material's ambient colour", which is what the device material
would be given (`0x100308b8`) on the path `UseDXLighting` 0 never takes; and
an engine that adds it draws every lit surface paler by the scene colour,
which is what the recordings showed of this one. The block offsets
`+0x14..+0x1c` are the entry's ambient, and the entry's emissive is never
read: a material's ambient colour is its self-light. See
[07-objects.md](07-objects.md#how-a-material-reaches-the-device--read-and-measured).

**There is one scene colour, the shader component's, and a model view takes it
too** — *read*. The colour is the state block's, at its `+0x14`: the shade
reads it out of the block it merges for each item (`0x1004df7b`–`0x1004df8b`,
the colour at the interface's `+0x78`), and the item renderer, one global
object at `0x100a5d30` (`0x1002ff22`), keeps a copy of the same block at its
`+0x18`. Each prim buffer's render copies that
block in, asking the state object it holds at `+0x9c` for it (that object's slot 7;
`0x1003d990`–`0x1003da6c`, and the three renders like it at `0x1003dd59`,
`0x1003e230`, `0x1003f169`), and
`CShade` hands every prim buffer the same object, its `+0xbd0`, the shader
component's interface 4 ([above](#the-colour-filter-is-the-cameras-infrared-in-the-shader-component--read)),
through their slot 8 (`0x10046824`–`0x100468f8` → `0x10031bb0`). The block is built
with the colour (0.2, 0.2, 0.2) (`0x1004bbcd`), and interface 4's slot 6
(`0x1004f9a0`) merges a block into it field by field as its `+4` mask says
(`0x1004c080`): bit `0x10` the colour, `0x40` the fog's start and end, `0x20` the
word at `+0x24`, 1 and 2 the flag bits. **Only the sky sends the colour**: of the
five places in `Terrain.dll` that fill a block's mask and send it, the sky's is the
one with `0x10` — `0x50`, the colour with the fog range (`0x1007bbc5`); the others
are `CShade`'s `0x180` (`0x10046c31`), `0x20` (`0x10078689`), the camera's 1, the
infrared flag it sends every frame (`0x10084514`), and the sky's `0x40`, which rides
with its own draw item rather than being sent (`0x1007a5d4`). The sky sends it from
its slot 3 (`0x1007ac60`), the last step the atmosphere's property hand-over takes
for each body (`0x10070b3f`), and that runs on the atmosphere's takt message (event
6, 1, `0x10070040`–`0x100702fa`), not in the draw. So the scene colour follows the
sky's clock whether or not the world is drawn — behind the warbot designer too —
and every view's materials take it, a HUD or designer model view's as much as the
world's ([37-designer.md](37-designer.md#the-previews--read-and-seen)); before any
sky has sent one it is 0.2 grey.

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

## The weather — *read*, and *seen*

The factory's three weather classes are `CRain` (0xc0 bytes, constructor
`Terrain.dll:0x10075d00`), `CSnow` (0xb8, `0x10075560`) and `CLightning`
(0xa0, `0x10071990`) — the "one more" of [the ten opcodes](#the-ten-opcodes).
Each has the same six slots: its interfaces, its delete, a start time (slot 2),
an update with the game time (slot 3), **a draw with a view (slot 4)** and its
type; and behind interface 7 a setter that takes a parameter's index and value.

**What the atmosphere hands them.** Every takt, for every object
(`0x10070a20`): `CAtmData` lerps the type's parameters between the two keyframes
round the clock (the switch at `0x1006cf64`), the setter takes each by its index,
and slot 3 takes the time.

| type | parameter 0 | parameter 1 |
|---|---|---|
| `RAIN` (`0x1006ce00`), `SNOW` (`0x1006cd81`) | the fourth float | slot 19 with each of R, G and B held at `0x50` or above and the alpha set to `0x96` (`0x1006ce63`–`0x1006ceb6`, `0x1006d120`) |
| `LIGHTNING` (`0x1006cef8`) | the fourth float | — |

So rain and snow are both drawn in **the sun light's colour, never darker than
(80, 80, 80), at an alpha of 150**: grey while slot 19 is black, as at the start
of `CAMPAIGN.03`'s day, and red once its sun is up.

**When they draw.** `CAtmosphere`'s draw (`0x10070b90`) takes the pass flags the
world hands it. With `0x800` it draws the objects of type 1, the sky; otherwise
with `0x1000` those of types 2, 3 and 4. The scene camera runs the `0x800` pass
before the world and the `0x1000` pass after it
([03-terrain.md](03-terrain.md#when-it-draws--read)), so **the weather is drawn
after the whole world**. A reflection camera runs the `0x800` pass like any
other, hands the world `0x120`, and is not handed the `0x1000` pass at all
(`0x10084781`).

**In which views.** The rain's and the snow's draws begin alike (`0x100759b5`,
`0x10076335`): they ask the view for interface `0x12`, `ICamera2`, and return
if its slot 24, the camera's type
([the sky's first draw](#the-skys-first-draw-is-a-screen-wide-quad--read)),
is not 0 — which only the water's two mirrors are. Nothing else in them gates
anything: the two draws and the particle system under them call the view, the
render device, the shader's colour filter, the material manager and the game
clock, and **nothing of the landscape, of a building or of the player's unit**.
So the weather falls in every view a scene camera draws: *seen* in every frame
of `CAMPAIGN.03`'s briefings ("Let's Play - Parkan: Iron Strategy, Part 5",
PfAg6zSe-yM, from 0:55; "Part 6", -yNnsqudMzw, 0:55), of the cockpit, of
command mode ("Part 6.5", 9SBZOCWv_vE, 7:05) and of a bunker's guns (9:47).

**But not in a room** — *read*, and *seen*. The gate is not in the weather; it
is on the pass. `CCamera::Render` clears its flag word's `0x200` before it
draws the world (`0x10084732`) and hands the `0x1000` pass over only if the
bit is still clear after it (`0x10084790`–`0x1008479e`); and a building's own
draw sets it. `CBuilding::Render` (`0x10058b90`), for a camera inside its
sphere, turns the camera into each of its rooms' frames and asks whether it
stands in one ([24-motion.md](24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read)):
the eye must lie in the room's level-0 box, corner to corner
(`0x100592af`–`0x1005934f`), and the room's mesh must then say it holds it —
interface `0x20` slot 7 (`AniMesh.dll:0x10010550`), which sends a line through
the point across the room's triangles, counts the crossings to each side
(`0x10010995`, `0x100109a5`) and answers whether either count is odd. With the
camera in a room, and pass flags without `0x100` — any camera but a mirror —
the building takes the view's flags, ORs in `0x200` and sets them
(`0x100599dc`–`0x10059a06`). It is the one writer found: across the install,
that is the only call through a slot 13 (`+0x34`) with an `or` of `0x200` in
the ten instructions before it.

**The line is the room node's own z** — *read*, and *measured*. An earlier
reading here had the mesh count crossings "along the direction at its
`+0x150`". That triple is no direction: it is the mesh's **scale**, `+0x158`
of the object seen from the interface the slot is entered on, eight bytes in
([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)),
and the three floats slot 7 compares it with, zeros in the file at
`0x10026b40`, are (1, 1, 1) once their initialiser has run (`0x100121f0`): the
test asks whether the mesh is scaled. The walk's node visitor (`0x100106d0`)
brings the point into the node's frame — the node's matrix at `+0x20`, its
translation taken off and the transpose of its 3 × 3, `g_FastProc`'s `+0x84` —
divides it by the scale where there is one (`0x10010736`–`0x1001074f`) and
answers no outside the slot's sphere (`0x10010756`–`0x1001078e`). The
triangle visitor (`0x100107a0`) passes a face whose stored normal has no z
(`0x10010839`), asks whether the point's x and y lie to one side of all three
edges (`0x10011530`; the axes 0 and 1 at `0x100209d8` and `0x100209e4`, the
next corner (1, 2, 0) at `0x100209e8`), finds the height of the face's plane
there off its normal and first corner (`0x10010947`–`0x1001097f`), and counts
the face under the point or over it. The filter it walks with shuts no face
out ([07-objects.md](07-objects.md#who-fills-a-face-querys-filter--read)).
`CBuilding::Render` hands it the eye as the camera has it and the room's node
(`0x10059355`–`0x10059373`), having tested the box through the same `+0x84` on
that node's placement (`0x1005922d`–`0x10059293`). So the line runs along the
z of the room's own frame, which is what the engine took.

*Measured* (`openparkan verify`): the install has **177 rooms** — nodes past
the first whose flags carry 4 and 1 — in 21 meshes. At 125 points in each
room's box, 22125 in all, 13345 are inside, and the game's reading — the
sphere, the stored normal, the plane's height — and the engine's barycentric
weights agree on every one. So would a line along x or along y, on all 22125:
a room's level-0 slot is a closed shell, and the axis never mattered. The
control opens the shells: with each room's most level face left out, a line
along x and one along z part on 201 of the points.

So **neither rain nor snow is drawn while the camera stands in a room of a
building**, any building, the pod room among them; at a doorway, in the
exterior's cell, it still is. Nothing else stops for it: the particles are
only moved by the draw, the rain's sound is set in the takt, and the lightning
strikes and lights the hall through its doorways as before. *Seen* on
`CAMPAIGN.03`'s Small Bunker (Part 6): at 28:16 the hero is on the threshold,
the frame is the doorway's black and the dust is all over it; at 28:20 and at
28:34, inside, there is none, nor in the frames between at two-second steps;
at 28:42, outside again, it is back.

### One particle system, run two ways — *read*

Rain and snow each own one `0x8e14`-byte system (`0x10072b40`), made with two
flags: snow's are (1, 0), rain's (0, 1) (`0x1007563d`, `0x10075ddd`). The first
gives the particles a flutter, the second draws them as streaks.

**Where they live.** The particles are **points in the world** (the array at
`+0`), and the box they are kept in is the camera's. Each draw builds it from
the view (`0x100759dc`–`0x10075ab4`, and the same at `0x1007635c`):

| | |
|---|---|
| depth | from **2** to **50** ahead of the camera |
| across | ± 50 × tan(*f* ÷ 2), *f* the view's field of view |
| up | ± 50 × tan(*f* × *h* ÷ *w* ÷ 2), *w* × *h* the viewport |

It is a box and not a frustum: as wide at 2 as at 50, so about a third of it
is in view.

**How many.** `1000 × intensity × min(1, V ÷ V₀)`, rounded (`0x10075b3a`–
`0x10075b8f`, `0x100764ba`–`0x1007650f`): *V* is the box's volume and *V₀* that
of the box at a field of 1.3 across and 0.975 up (`0x100757c0`, `0x1009be14`,
`0x1009be18`). So a thousand at full intensity in a 4:3 view 1.3 across or
wider, 722 at 16:9, and 19 zoomed to a field of 0.2. The system's pools hold
1000 screen positions (`0x10072cad`), and no shipped keyframe asks for more
([below](#which-missions-fall-what--measured)).

**Each draw** (`0x10073c10`):

1. if the box is not the one the last draw kept at `+0x8de4`, every particle is
   spawned again in the new one (`0x10073c92`);
2. every particle moves by the system's velocity times the seconds since the
   last draw, the clock being the game queue's (`World3D.dll!GetQueue`, slot 14);
   with the flutter flag it moves again by (*r*, *r*, *r*) times those seconds,
   *r* in 0..1 off the generator at `+0x5e78` — three steps of it, and the
   value after the third on all three axes (`0x1007371a`–`0x100738bd`). The
   generator is `Effect.dll`'s ([11-effects.md](11-effects.md#the-generator--read-and-measured)),
   seeded from `NGI32.dll` ordinal 52;
3. every particle is brought into the camera's frame, and one outside the box
   is **wrapped**: moved a whole number of box extents along each axis
   (`0x10073410`, by `floor`). A particle is never deleted and spawned for
   leaving; it comes back in at the far side;
4. the count is brought to the one asked: new particles are spawned anywhere
   in the box, each coordinate `rand() ÷ 32767` of its extent (`0x10073200`;
   the C runtime's `rand`, `0x1008e98c`), and a surplus is dropped from the end.

So the fall follows the camera only by wrapping: turn, and the particles stay
where they are in the world while the box turns under them.

**What is drawn.** The particles are projected to the screen and each becomes a
quad of **pre-transformed vertices**. A particle is left out unless its centre
is inside the viewport less half a full-sized particle, rounded up, on every
side; at most **600** quads are made (`0x258`, `0x100748d2`, `0x10075401`). The
size is the caller's factor times the camera's slot 27, the viewport's width
over the field of view — so a factor of *k* is a particle *k* ÷ *f* of the
screen's width across at the near face — and it shrinks with depth by
1 − (*d* − 2) ÷ 48.

- **A snowflake** (`0x100748ea`) is centred on its particle, its half-size the
  factor's half times the fade, the fade held at **0.1** or above
  (`0x1009be28`). Its index turns it: the low two bits mirror it across and up
  (`0x100a35f8`, `0x100a3608`) and turn its texture a quarter each
  (`0x1009bde4`…), and bit 2 picks a square; without bit 2 it is a diamond,
  the same square on its corner (× √2, `0x100a70a8`).
- **A raindrop** (`0x1007416c`) is a quad from where the particle was drawn
  before the last move to where it is now, its half-width the factor's half
  times the fade, with no floor; both ends must be inside the viewport. A drop
  that has just wrapped has no length.

The quads go to the shader's slot 16 (`0x10028500`, called at `0x10075546`)
with item flags `0xd`, the depth argument 3 — **`ZENABLE` and `ZWRITEENABLE`
both off** — and **group 1, layer 8**: after the effects' layers 3 and 6 and a
doorway's fade quad in layer 5, before the cockpit's layers 9 and 10
([The dome](#the-dome)). One colour stands for every vertex (a stream of
stride 0): parameter 1, put through the shader's colour filter while the
camera's infrared is on (`0x10075b0b`), with a specular of `0xff000000`, whose
alpha is the fog factor: **no fog**. The flags carry no `0x10`, so no device
material is built and the material's own colours do not enter: the texture
times that colour. The material is the `sky.wea` slot the start carries, asked
for at time 0 every draw (`0x10075870`), so its first entry — `DUST_ADD`'s four
entries, cells 0 to 3 of `DUST.0`, are not played, and the dust is cell 0, a
puff 64 texels square.

### Snow — *read*, and *seen*

`CSnow::Draw` (`0x100759a0`): a size factor of **0.0195** (`0x1009be70`), a
velocity of **(0.5, 0, −4)** world units a second (`0x10075b92`), the flutter on.
A flake falls 4 a second and drifts 0.5 along x, and the flutter adds between
0 and 1 a second along each axis, the same on all three. At a field of 1.3 the
largest flake is 1.5% of the screen across and the smallest a tenth of that.

*Seen* on `CAMPAIGN.03`, whose snow is `DUST_ADD`: soft squares and diamonds,
two to fifteen pixels across on a 1400-pixel frame, some thirty-five in a
500 × 320 region of sky, grey through the briefing and pink-red in play
(Part 5, 1:55; Part 6, 0:55, 1:50, 2:00), and pink under the moon, whose slot 19
is (244, 154, 183) to (255, 205, 224) (Part 6, 48:40). The engine's frames of
the same mission match in count, size, shape and colour.

**The colour is all that changes with the hour.** The material is the one slot
the start carries, its first entry, and the size a constant, so the dust is the
same puff by night as by day. The white many-pointed stars in the night frames
of Part 6 (48:05, 48:40) are not it: they stand about the player's own muzzle
flash and about the unit being shot twelve metres ahead, not in the sky, where
the dust goes on falling pink. `DUST.0`'s star cells, pages 32 to 37, are worn
by `splash5`, `plas_shoot` and `taser_shoot` — the sprites of the bullet-hit
effects `exp_b_be`, `exp_b_cb` and `exp_b_hc`, of `exp_m_pls_b` and of
`gunf_builder` (*measured*) — and by `SHIELD1`, which nothing names.

### Rain — *read*

`CRain::Draw` (`0x10076320`): a factor of **0.0065** (`0x1009beb8`), a velocity
of **(0.5, 0, −60)** (`0x1007654b`), no flutter, streaks. At 60 frames a second
a drop 26 ahead is a streak a fortieth of the screen's width long and a
four-hundredth wide.

**Its sound.** The constructor opens `sounds.lib`, finds the name the start
carries — *"Unable to find sound"* if it is not there — makes a sound of it
with flags `0x102`, sets its volume to 0 and plays it (`0x10075e37`–
`0x10075fca`). Slot 3, every takt, sets the volume to
`10000 × (intensity^(1/16) − 1)` hundredths of a decibel — four square roots
(`0x100765e5`–`0x10076636`): silence at 0, −4.2 dB at 0.5, full at 1. The
destructor stops and releases it (`0x100761a0`).

**It is a loop, and it is nowhere** — *read*. The call goes to the 3D sound
object itself, not to a sound world: the atmosphere keeps `niGet3DSound`'s
answer at its `+0x164` (`0x1006efee`) and hands it to the rain's constructor,
which calls its slot 7 (`Ngi32.dll:0x1000cab0`). That makes a plain sound, a
`0x2c`-byte object with the vtable `0x1003175c` (`0x1000e980`), and the flags
are its own:

| flag | what it does |
|---|---|
| `0x100` | the buffer is made with `DSBCAPS_CTRLVOLUME`, and the volume slot does nothing without it (`0x1000e9db`, `0x1000eb7e`) |
| `0x200`, `0x400` | `DSBCAPS_CTRLFREQUENCY` and `DSBCAPS_CTRLPAN`, by the same shift (`0x1000e9c9`–`0x1000e9e2`) |
| `0x2` | kept with the others at the object's `+0xc` (`0x1000ea1e`), and handed to the buffer's `Play` as its flags, `DSBPLAY_LOOPING` (`0x1000eb15`–`0x1000eb25`) |

The buffer's description comes out as `0x82` for the rain: static, its volume
settable, and **no `DSBCAPS_CTRL3D`** — no flag of this object asks for one —
so nothing places it and it is heard alike from everywhere. It plays on in a
room, where the drops are not drawn.

No recording of rain is at hand. *Measured*: 8 missions rain, all with
`RAIN_DROP`, cell 21 of `EFFECT6.0`, and `atm_rain1.wav`.

### Lightning — *read*, *measured*, and *seen*

`CLightning` draws nothing: its slot 4 is an empty routine (`0x100721a0`). It
keeps **one instance of its effect** in the landscape's effect manager, made
and switched off in its constructor (`0x10071a8d`–`0x10071af6`), and its
update decides when to play it.

**When** (`0x10071cb0`, three states):

1. after a strike, nothing for **6000 ms** (`0x10071ce8`);
2. then, the intensity above 0, the next strike's time is drawn
   (`0x10071920`): now + `rand() ÷ 32767` × (1 − intensity) × **60000 ms**,
   the intensity held at **0.95**;
3. at that time, a strike.

So a bolt every 6 to 9 seconds at full intensity, every 6 to 36 at 0.5, and
at full intensity the first within three seconds of the object's start. *Measured* on the
recording: over four minutes of Part 6.5 from a bunker's guns the ground
lights up at 9:09.7, 9:17.5, 9:25.6, 9:32.1, 9:46.9, 9:54.9, 10:41.2, 10:48.0,
10:54.5, 11:50.9, 11:57.0, 12:03.3, 12:10.9 and 12:25.2 — nine gaps between
**6.1 and 8.1 s**, two of 14.3 and 14.8 with a strike unseen between, and the
longer gaps where gunfire hides the count.

**Where** (`0x100720b0`): a point drawn anywhere over the landscape's box —
the corners its `IMesh2` hands out at start (`0x10071c9a`), x and y each
`rand() ÷ 32767` of the box's extent. It is the whole map, not the camera's
surroundings. `CLightning::Init`'s two panics, *"Root object is not a
landscape"* and *"Landscape does not support IMesh2"*, are that request. The
corners are the first 96 bytes of `Land.msh`'s stream 2 as the landscape read
them (`0x1001781c`): the point's x runs from corner 0 to corner 1, its y from
corner 0 to corner 3, and **its z is corner 0's** (`0x1007211e`–`0x1007213d`).
*Measured* on all 33 maps: corner 0 is (0, 0, the lowest vertex's z), corner 1
that moved along x and corner 3 along y. So the strike is looked for from the
map box's floor.

**The ground under it** — *read*, and *measured*. The strike asks the world,
not the landscape: `IWorld` slot 8 (`0x10025b40`, the world from `0x10024ac0`)
with 2 in the register, the z axis and neither direction bit, and a query
whose first word is the static at `0x100a6f54` and whose other seven are 0
(`0x10071d95`–`0x10071e32`). The static is zero in the file and `0xfff` once
the initialiser `0x100723d0` has run — the OR of the twelve bit constants at
`0x1009bb4c`, the same default the other modules build
([24-motion.md](24-motion.md#finding-the-ground--read)). So: **all twelve
classes, nothing required and nothing shut out.** Slot 8 starts the answer's
gap at `FLT_MAX` (`0x10025b6e`) and walks the world's tree from its root
(`0x10025ba0`), asking every object whose class the mask holds for interface
`0x18` slot 7 and keeping an answer only where its squared gap to the point is
the smaller (`0x10025c3b`–`0x10025c46`).

The landscape's answer (`0x10021860`) has a turn of its own. A query that
neither asks for nor shuts out world flag `0x200`, the liquid's surface, nor
`0x400`, its bed (`0x100218de`–`0x1002197a`), is run **twice** — once with the
surface shut out and once with the bed (`0x100219b3`, `0x10021a65`–`0x10021a7a`)
— and the second is kept only where its gap is the smaller (`0x10021aaf`). Each
run takes the **first** face of the cell's run, in the file's order, whose
triangle holds the point and whose plane is not on edge, either side of the
point (`0x10021cc6`–`0x100220fa`); the direction bits 4 and 8 are what would
have refused one side (`0x10021ff6`, `0x1002201b`).

*Measured* on the 33 maps, a 60 × 60 lattice over each box (`openparkan
verify`): **every one of the 118800 points has a landscape face**, 111446 one
and 7353 two, a liquid's sheet and its bed on all 7353; one more lies on the
edge two dry faces share, at one height. No point has two dry faces at two
heights, so "first in the run" never has to choose. From the floor the bed is
never the farther — under the sheet on 7341 of the pairs and level with it on
12, ten of them on maps 11 and FINAL, which lay a sheet flat on its bed — so
**a strike over a lake or over lava is measured to the bed**: the effect stands 300 over the
bottom, and its 600 reach up through the sheet. An object answers too, a
building's deck or a warbot's back, and wins only where its face is nearer the
map's floor than the ground's own — under the ground, that is.

**What a strike does** (`0x10071d70`): it asks the world for the ground at
the point, puts the effect **300 above it** in a frame of the world's own axes,
its second axis turned back one time in two (`0x10071f8a`), starts it in time
mode 1, switches it on, and sizes it **(40, 40, 600)** (`0x10072058`). Those
are its only calls: the height query and four slots of the effect manager.
**A bolt hurts nothing.**

**`env_lightning`** is three blocks over **0.75 s**, its time jittered by ±0.2
(header flag 1, spread 0.4) and its sprites' fog factor held at 1 (flag
`0x2000`, on this effect alone):

| block | | |
|---|---|---|
| type 3, a sprite | material `env_lightning`: cell 23 of `EFFECT6.0`, a bolt drawn lying down, 192 × 32 texels, additive | fading 1 → 0; sprite mode **1**, direction (0, 0, 1) |
| type 1, a light | kind 7: a point light, flags `0x20000000` | colour (7, 7, 10) → 0, range 100 → 0, falloff terms (0, 0, 1) |
| type 2, a sound | `atm_light1.wav` | heard whole within 100, to 1500 |

*The sprite.* The block's sprite mode is 1 and its direction (0, 0, 1), and a
mode-1 sprite is a streak with the texture's u along its direction, turned
about it to face the eye
([11-effects.md](11-effects.md#a-sprites-mode--read-measured-and-seen)). So the
bolt's picture runs **up the frame's third axis**: 600 tall and 40 across, from
the ground up. *Seen* (Part 5, 1:55.0–1:55.6): a jagged white-pink bolt with a
fainter branch, from the top of the frame down behind a hill.

*The light.* It reaches the scene through **the shade's own lighter**, which
[11-effects.md](11-effects.md#what-a-light-does-to-a-surface--read-and-measured)
reads. An item that builds a device material takes one of two roads
(`0x1002fe3d`–`0x1002ffa1`). With setting 29, `UseDXLighting`, on, the builder
at `0x10030620` fills a `D3DLIGHT7` from each of the item's first eight light
records and sets it on the device (`0x10030be6`); the setting's default is 0
(`0x1005fcce`, and the [table](#the-render-settings) above), so **that road is
not taken**. An earlier version of this section took it for the live one. With
the setting off the item's light list goes to the shader's slot 8, whose point
light (`Ngi32.dll:0x100165bb`–`0x100166ac`) leaves out a vertex past the range
or facing away, and scales the rest by `a0 + a1 x + a2 x²` on
*x* = (range − *d*) ÷ range, the three terms the block's own in its own order
(`Effect.dll:0x1000fb9a` into `Terrain.dll:0x100800b0`, record `+0x38`,
`+0x3c`, `+0x40`). `env_lightning`'s (0, 0, 1) is *x*², and its range starts at
100 × the frame's 40 = 4000
([11-effects.md](11-effects.md#type-1-is-a-light--read-and-measured) has the
range's scale): at 1000 from the bolt the light is still 0.56 of (7, 7, 10),
far past what a surface can show, and what is over 1 the lighter moves into
the specular colour, which is added after the texture and whitens it. Its flag `0x20000000` only keeps it out of
the landscape's emulated discs.

*Seen*: for the bolt's half second **every lit surface within reach goes to
its texture's own colours** — the ground, the hills and a unit's own cockpit
pale and white-lilac, slopes facing away still dark, the sky, the dust and the
HUD untouched, the gate's two towers dark (Part 5, 1:55.0–1:55.5; Part 6.5,
9:46.9–9:47.3). A lightmapped batch takes no light at all
([07-objects.md](07-objects.md#how-a-lightmapped-batch-is-drawn--read-and-measured)).
Neither the sky nor the scene colour flashes. The flash is there whether or
not the bolt is in view.

### Which missions fall what — *measured*

All 29 atmosphere files, through their `sky.wea`:

| slot | material | files | of them falling |
|---:|---|---:|---|
| 7, snow | `SNOWFLAKE` (`EFFECT6.0` cell 20, blend 4) | 23 | 1: `CAMPAIGN.05/Mission.01`, three spells |
| 7, snow | `DUST_ADD` (`DUST.0` cell 0, blend 2) | 6 | all 6, the whole day |
| 8, rain | `RAIN_DROP` (`EFFECT6.0` cell 21, blend 4) | 29 | 8 |

15 of the 29 start some weather: 7 snow, 8 rain, 8 lightning. The fourth float
is at most 1.0 on all 656 keyframes. From a spell's start to its stop it is
1.0 on 96 of snow's 104 keyframes, 0.7 on 2 and 0 on 6, the starts and stops
of `CAMPAIGN.05/Mission.01`'s three; on rain's 57 it is 1.0 on 24, 0.8 on 10,
0.5 to 0.7 on 5 and 0 on 18, its nine starts and nine stops, so **rain comes
on and goes off by degrees**; on lightning's 96 it is 1.0 on 90 and 0.6 to
0.8 on 6.

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
  camera — the viewer's own figures, not [the game's](#the-weather--read-and-seen).
  Snow is marked in the payload and not drawn; lightning is neither.
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

- ~~Whether the dome and its layers take the scene's fog~~ — the gradient and
  the nebula carry a specular of `0xff000000`, a fog factor of 1, and take
  none; the clouds are a lit item with a fog of their own, 5000 to 11380.7
  ([The clouds are lit](#the-clouds-are-lit-and-fogged-on-a-range-of-their-own--read-and-measured)).
- ~~How the files' colours meet the textures~~ — as stored values, every
  multiply, add and blend
  ([The frame holds what the files hold](#the-frame-holds-what-the-files-hold--read-and-measured)).
- ~~**Whose light list the clouds are lit with.**~~ — the last drawn
  object's or cell's, and *which* that is stays unread; what is *measured* is
  that no point light in any list lights them. The lighter passes over a
  vertex past a light's range, the cap's nearest vertices stand 5000 and 4580
  over the eye, the largest of the 618 blocks' ranges is 300, and the one
  light a frame stretches that far, the lightning's 4000 ± 200, reaches a
  first-ring vertex only from ground more than 80 above the eye 5341 away,
  for under a hundredth of its colour
  ([The clouds are lit](#the-clouds-are-lit-and-fogged-on-a-range-of-their-own--read-and-measured)).
- ~~**Whether a light's direction is put into an item's frame as a unit
  vector.**~~ — it is not: the shade multiplies it by the transpose of the
  frame's 3 × 3 (`g_FastProc`'s `+0x8c`, `Ngi32.dll:0x10024260`) and neither
  it nor the directional routine normalises the result, so a frame scaled by
  *s* takes *s* times the light
  ([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)).
  ~~Still open: which shipped frames carry a scale, and so which objects the
  engine, which normalises, lights otherwise.~~ **One frame, and no scaled
  object.** The frame keeps its scale beside its matrix (`N3DSCALEDMATRIX`),
  so a scaled tree's or stone's 3 × 3 is the rotation it had: 216 of the 864
  placements carry a scale and none carries it in a matrix. Of the 34049 pose
  keys a node's 3 × 3 is made of, 34038 give a rotation to within 0.00017, ten
  the identity, and one — the Large turret 09's `BTHQ_m1o1` at frame 47 — a
  turn of 116.57° stretched 2.236 across its z. All five builds of the slot
  are read, and none normalises. The engine's unit light is the game's on
  every scaled object; its readers made a half turn of a key the game's
  matrix makes the identity of, which is fixed, and the one stretch is left
  out. ~~Not enumerated: the placements `Control.dll` gives a unit's parts.~~
  **No controller plays the stretched key, nor any of the eleven**: both records'
  dish runs 42 to 46 against its 47, and all eleven stand one frame past their
  node's channel. `SetScale` reaches scenery alone, `Control.dll`'s nine placements
  multiply no axis, and the two other `+0x6a` stores are no frame. What stays
  open is the frame ([The lit colour is the game's
  own](#the-lit-colour-is-the-games-own--read-and-measured)).
- ~~**One face of C03 M02's Outpost.**~~ — the building is Enemy 1's Medium
  Mine, and the lavender was the **second light's highlight** on its skin's
  specular, which the game does not take: `CSun`'s constructor flags the
  counter light `D3DLIGHT_NO_SPECULAR` through the manager's slot 18
  (`0x1007d03b`, `0x10080aea`), and the shade asks no highlight of a light
  that carries it. Without it the engine's slope reads (142, 17, 27) against
  the recording's (139, 13, 23), where it read (191, 44, 74)
  ([The lit colour is the game's own](#the-lit-colour-is-the-games-own--read-and-measured)).
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
- ~~The **colour filter's slot 5**, and its flag bit 0.~~ **Read**: the object is
  the shader component's one state (`CreateShader`, a singleton; interface 4's vtable
  `0x1009b2fc` is in `Terrain.dll` after all), slot 5 (`0x1004f900`) returns
  green = min(255, 7 × 0.33 (R + G + B)) with red and blue 0 and the alpha kept, and
  bit 0 is set every frame by the camera from its flag word's infrared `0x20`
  ([The colour filter is the camera's infrared, in the shader
  component](#the-colour-filter-is-the-cameras-infrared-in-the-shader-component--read)).
- ~~What scene colour a view that is not the world's takes~~ — the one there is:
  the shader state's, which only the sky sends, every takt, whether or not the world
  is drawn; 0.2 grey before it has
  ([The scene colour is added to every material](#the-scene-colour-is-added-to-every-material)).
- ~~Which of the flare's pair the engine calls texture 0~~ — slot 5; the column
  branches into the material blocks the constructor fills from slots 5 and 6
  ([The lens flare](#the-lens-flare)).
- ~~What the sky's fourth draw puts on the screen~~ — ~~the scene colour~~ the
  **fog colour**, over the whole viewport and opaque. Its material block
  `+0x484` has no texture and no colour of its own, which an earlier round took
  to leave it the scene colour; the item is unlit, and its colour is the one
  its vertex stream carries, the sky's `+0x70`, written with the fog colour
  every frame (`0x10079698`, `0x10079b3f`). The draw is skipped whenever the
  view's type (slot 24, `0x10083010`) is 1 (`0x1007a325`).
  ~~Which views carry which mode~~ — the scene's own camera is type 0, where
  the quad is drawn, first of all and before the world; type 1 is the
  `REFLECTION` camera, drawn into the frame after it, and type 2 the
  `REFLECTION_SHIFTED` one
  ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad--read)).
- ~~**What lies below the dome's rim.**~~ The sky's screen-wide quad, drawn
  before the dome and the world in every scene camera's frame.
  ~~That it paints the scene colour is derived and not checked against a
  frame; what the device is cleared to under it is still not read.~~ It paints
  the **fog colour**, *seen* under the rim of C02 M04's briefing at
  (106, 172, 22) against a scene colour of (86, 111, 67); and the device's
  colour buffer is cleared to nothing at all — `Ngi32.dll`'s one `Clear` is the
  depth's (`0x10007d32`, flags 2), so the quad is the frame's clear
  ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad--read)).
- ~~What the three weather objects draw~~ — rain and snow are one particle
  system of up to a thousand points kept in the camera's box, 2 to 50 ahead,
  and drawn as screen-space quads over the finished world with the depth test
  off; lightning draws nothing and plays `env_lightning` at a point drawn over
  the whole map every 6 s and up to 60 more
  ([The weather](#the-weather--read-and-seen)).
- ~~**The flags the rain's sound is made with**, `0x102` (`0x10075f8c`): that it
  loops is inferred from its being played once and never again.~~ **Read**:
  the sound is the 3D sound object's plain kind (`Ngi32.dll:0x1000cab0`,
  `0x1000e980`); `0x2` is the `DSBPLAY_LOOPING` its play hands the buffer
  (`0x1000eb15`), `0x100` the `DSBCAPS_CTRLVOLUME` its volume slot waits for
  (`0x1000eb7e`), and with no 3D capability asked the rain is heard alike from
  everywhere ([Rain](#rain--read)).
- ~~**Which faces the lightning's ground query takes.** It is the landscape's
  slot 8 with kind 2 and the default filter (`0x10071e19`); the filter's mask
  at `0x100a6f54` is not read.~~ **Read**, and *measured*: it is the world's
  slot 8, the mask is `0xfff` once its initialiser has run (`0x100723d0`), all
  twelve classes with nothing shut out, and the point looks from the map box's
  floor, so the answer is the face nearest that floor — the landscape's one
  face on 111446 of 118800 lattice points and a liquid's **bed** on 7353 of
  the rest ([Lightning](#lightning--read-measured-and-seen)).
- ~~**Who gives a camera a mode other than 0.** `ICamera2`'s slot 23
  (`0x10082fd0`) is the only writer of `+0x280` besides the constructor, and
  its caller is not found.~~ **Read**: `CCamera` itself, from slot 14 of its
  game-object interface (`0x10085365`), with the type its constructor made of its name — 2
  for `REFLECTION_SHIFTED`, 1 for `REFLECTION`, 0 otherwise (`0x10083a54`). Of
  the install's 38 calls through `+0x5c` it is the one that passes a value in
  `edx` and nothing on the stack
  ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad--read)).
- ~~Whether the weather draws in a room~~ — it does not: a building's draw
  sets the view's flag `0x200` while the camera stands in one of its rooms
  (`0x100599f9`), and a view carrying it is not handed the weather's pass
  (`0x10084790`); *seen* on C03 M02's Small Bunker
  ([The weather](#the-weather--read-and-seen)).
- ~~What the infrared changes in a frame~~ — every lit item's material and the
  scene colour, the fog and the dome, and each sprite's one colour, all to
  seven times their mean as green alone; no light, no texture and nothing of
  the HUD; and it skips the lens flare. *Seen* once, for 0.7 s of Part 4.5
  ([What the infrared does to a frame](#what-the-infrared-does-to-a-frame--read-and-seen)).
- ~~**Whether slot 17's alpha reaches a body's sprite.** The sprite's four
  vertices carry slot 17's colour (`0x1007cd98`), alpha and all, and the alpha
  is 50 of 255 or less on 249 of the 656 keyframes; what the device does with
  it is not checked against a frame, and the engine draws a body in its
  texture's own colours.~~ **It does, and the colour with it**: `CSun` gives the
  sprite render phase 2 (`0x1007cabf`), whose stage multiplies both the texture's
  colour and its alpha by the vertex's, under blend 4. *Seen* on C02 M03's sun at
  114 of 255: the disc's core is (143, 183, 34) in the recording and (142, 185, 34)
  in the engine now, where it was (202, 238, 147)
  ([The sun and the moon are drawn](#the-sun-and-the-moon-are-drawn)).
- ~~**The line a room's mesh is asked along.** The room test's second half
  (`AniMesh.dll:0x10010550`) counts crossings along the direction at the mesh's
  `+0x150`, whose writer is not read; the engine takes the room's own z.~~
  **Read**, and *measured*: the room node's own z, as the engine had it. The
  triple at `+0x150` is the mesh's scale, written by `SetScale`
  (`0x10014770`) and (1, 1, 1) otherwise, which the point is divided by; the
  triangle test passes a face whose normal has no z and counts the others
  under or over the point by the height of their planes (`0x100107a0`). Over
  the install's 177 rooms the game's reading and the engine's agree on all
  22125 points tried, and so would a line along x or y: the rooms are closed
  shells ([The weather](#the-weather--read-and-seen)).
- ~~**Camera flag 8.** A camera whose flag word carries it draws neither the
  sky's pass nor the world (`0x100845eb`, `0x10084701`); what sets it is not
  read.~~ **Read**, and *measured*: the warbot designer's opening, which
  writes 9 to the game's scene camera (`iron3d.dll:0x10055c9b`), and nothing
  else — of the install's 375 calls through the setter's slot, 29 carry a
  constant with the bit and that one alone passes it as the setter's one
  value. The mirror's camera takes the scene camera's word each draw, 8 with
  it ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad--read)).
- ~~**Who calls `CCamera`'s slot 14.**~~ `World3D.dll`, for every object it
  makes and adds, a scene camera among them (`CreateObject`, `0x10007d1a`,
  `0x10007e6e`); and for the mirror's camera the landscape's own slot 14,
  with the argument it was handed itself (`Terrain.dll:0x1001ff0e`)
  ([The sky's first draw is a screen-wide quad](#the-skys-first-draw-is-a-screen-wide-quad--read)).
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
