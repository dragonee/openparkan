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

## The sibling `sky.wea`

Plain text in the same format model wears use, naming the textures:
`ENV_NEBULA_0`, `ENV_STARS`, `ENV_CLOUDS`, `ENV_SUN_3`, `ENV_MOON`,
`ENV_FLARE_00`, `ENV_FLARE_01`, `SNOWFLAKE`, `RAIN_DROP`. The variants differ
per mission — `ENV_NEBULA_1`, `ENV_SUN_6`, `ENV_CLOUDS_2` and so on. Nothing
yet says where in the dome each one is drawn.

## Not resolved

- **The keyframe count of a second section.** Six files have two; their
  72-byte section headers are byte-identical yet hold 27 and 20 keyframes, so
  the count is not in them. The reader takes the second section's keyframes to
  the end of the file, which consumes all six exactly.
- Which field selects the object type.
- The rest of the 124-byte file header. It holds `23, 59` where a time would
  go, `6939832` twice, and a couple of small counts.
- Slots 0, 5, 15–17, 19–21 of the colour block, and the trailer past the time.
