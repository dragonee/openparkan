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

Intensity has two gates. The first is exact: the flare is off once the sun is
more than **15°** off the view axis, ramps linearly to full on-axis, and the
ramp is then squared. Those cosines are cached at load from a constant of 15
degrees, alongside 30 and 60 for a second ramp on a quantity the sun object
carries at `+0x80`, which has not been identified.

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
- The **sun** and **moon** as billboards, opposite each other.
- The **lens flare**, as a 2D overlay drawn after the scene — it is in the
  lens, not the world, so it takes no depth test. The twelve elements and
  their tables are the engine's; both gates are adapted. The 15° cone is
  calibrated to the game's field of view and would almost never open against
  an orbiting camera that looks down at the terrain, so the same
  linear-then-squared ramp is driven by the sun's distance from the centre of
  the screen; and a horizon test stands in for the unidentified second gate,
  which at least has to take the flare away at night.

Where the sun goes is the renderer's own choice: `CSun::Render` builds its
matrix from two angles at `this+0x30` and `this+0x34`, and those have not been
found in `sky.ske`, so the viewer runs the sun along a day arc from the
keyframe's own time — overhead at noon, on the horizon at six, below it at
night — and points the scene's light the same way so the shading and the sky
agree. The time-of-day control walks the keyframes.

## Not resolved

- **Where the sun stands.** `CSun` keeps two angles; they are not in the
  keyframe. No pair of floats in the 88-byte block varies with time the way
  an azimuth and an elevation would.
- **What the flare's second gate measures.** The engine ramps it between the
  cosines of 30° and 60° of a float the sun object keeps at `+0x80`.
- **Snow and rain.** Not for want of a particle system — they are not in
  `effects.rlb` at all, and which missions have them is [answered
  above](#the-weather-and-how-the-engine-reads-a-keyframe). What is missing is
  the *sprite*. `RAIN_DROP` and `SNOWFLAKE` are cells 21 and 20 of
  `EFFECT6.0`, and that texture is not a uniform grid: it holds four wide blue
  streaks, four starbursts, a cyan band, a row of eight green discs and a row
  of small icons, all at different tile sizes. Neither an 8x8 nor a 4x8
  reading of cell 21 lands on anything that looks like a drop. Until the
  atlas's addressing is understood there is no honest way to cut one out, so
  the viewer marks the weather in its panel and draws nothing.
- **The keyframe count of a second section.** Six files have two; their
  72-byte section headers are byte-identical yet hold 27 and 20 keyframes, so
  the count is not in them. The reader takes the second section's keyframes to
  the end of the file, which consumes all six exactly.
- Which field selects the object type.
- The rest of the 124-byte file header. It holds `23, 59` where a time would
  go, `6939832` twice, and a couple of small counts.
- Slots 0, 5, 15–17, 19–21 of the colour block, and the trailer past the time.
