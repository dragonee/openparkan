# The briefing flythrough — `briefing.cfg`

A campaign mission opens on a camera flying over the map while a voice talks
and subtitles run. That sequence is plain text in the mission's own directory,
in the same `object … end` syntax as `mission.cfg`:

```
object WayPoint0
    CameraX = 767.965        where the camera is
    CameraY = 164.519
    CameraZ = 118.899
    TargetX = 967.457        where it looks
    TargetY = 610.348
    TargetZ = 11.945
    EdgeType = "spline"      how it travels
    WaitType = "continuous"  whether it stops here
    EdgeTime = 3.70
    WaypointTime = 0.000
    RotateTime = 0.000
    FadeTime = 0.000
    ZoomTime = 0.000
    WaitForText = false      what holds the sequence here
    WaitForSound = true
    WaitForTime = true
    WaitForClick = false
    TextResID = "T01_T01"    the subtitle
    SoundResID = "T01_T01"   the voice
    NoisePercent = 0         and the picture
    FadePercent = 100
    ZoomOn = false
    NightVisionOn = false
    LoopIndex = -1
end
```

**All 20 campaign missions carry one, and no other mission does** — the same
20 whose `mission.cfg` declares a `briefing_sounds` descriptor
([20-resources.md](20-resources.md)). Between them: **378 waypoints, every one
carrying all 24 fields.** Read by `openparkan.briefing`, shown by
`uv run openparkan briefing`.

## The path is in world coordinates

This is the structural proof, and it is a strong one. **All 378 camera
positions and all 378 targets fall inside their own map's XY extent** — the
closest 31 units from an edge — and **all 378 cameras sit above the terrain
sampled from `Land.msh`**, a median of 17.3 units of clearance and never less
than 1.0.

A wrong frame, a wrong axis order or a wrong scale would scatter them; instead
the briefing camera lives in exactly the space `data.tma` places objects in
([04-missions.md](04-missions.md)), which means a renderer can fly it as it
stands.

## The fields

| field | | what it is |
|---|---|---|
| `CameraX/Y/Z`, `TargetX/Y/Z` | | eye and look-at, world space — *measured* |
| `EdgeType` | | how the camera travels into this stop: `linear` 243, `spline` 89, `jump` 46 |
| `WaitType` | | `continuous` 362, `stay` 5, `flyaround` 11 |
| `EdgeTime` | | seconds of travel; `WaypointTime` seconds of dwell |
| `RotateTime`, `FadeTime`, `ZoomTime` | | three more timers, each on its own effect |
| `WaitForText/Sound/Time/Click` | | what holds the sequence at this stop; more than one may be set |
| `TextResID`, `SoundResID` | | the subtitle and the voice, resolved below |
| `NoisePercent`, `FadePercent` | | picture treatment, 0–100 |
| `ZoomOn`, `NightVisionOn` | | two camera filters, as booleans |
| `LoopIndex` | | **−1 on all 378** — a field the engine reads and the authors never used |

`EdgeTime + WaypointTime` summed over a briefing is a *floor*, not a duration:
a stop that waits for its voice runs as long as the voice does, and that
length is in the `.wav`, not here. The longest briefing by that floor is
`CAMPAIGN.01/Mission.01` at 190 seconds over 37 waypoints.

## What it says, and in whose voice

Both ids resolve, by the two routes [20-resources.md](20-resources.md)
describes:

```
TextResID  = "T01_T01"  →  DATA/TextRes.cfg  →  string 8 of TextRes.dll
SoundResID = "T01_T01"  →  mission.cfg's briefing_sounds  →  t01_t01.wav in voices.lib
```

**165 of 165 voices resolve**, every one through the mission's own
`briefing_sounds` descriptor — never through `tutorial_voices`, which serves
the in-mission messages instead. The two roles do not overlap.

**275 of 278 subtitles resolve.** The three that do not are all in the final
campaign mission, `CAMPAIGN.05/Mission.02`: `ISFINAL_04`, `ISFINAL_05`, and a
waypoint whose `TextResID` is a lone backtick. `TextRes.cfg` carries
`ISFINAL_01` to `ISFINAL_03` and stops. The **voices for all of them are
present** and resolve, so the finale plays its dialogue with two subtitles
missing — a gap in the shipped English build, not in the reader. The backtick
waypoint has `WaitForText = false` and no sound, so it is a placeholder for
*no text* that was typed wrong.

## `messages.cfg` — the in-mission dialogue

Sixteen of the twenty carry one beside the briefing:

```
object  message1
  message_index   = 0
  text_resource   = "T01_T01"
  voice_resource  = "T01_T01"
end
```

**99 messages, and every text resource resolves.** Those 16 are exactly the
missions whose `mission.cfg` declares `tutorial_voices`.

`message_index` is an **id, not a position** — three files prove it: one skips
6, one skips 20, and one jumps from 14 to 100. So something else in the
mission asks for a message *by number*, and the obvious candidate is the AI
script ([15-behaviour.md](15-behaviour.md)); nothing here tests that, and it
is written down as a lead, not a finding.

## What this does not say

- **What `flyaround` orbits.** Eleven waypoints ask for it; the field says
  neither a radius nor an axis, so it is presumably the camera circling its
  own target.
- **Which end of an edge `EdgeTime` belongs to.** Travel *into* a waypoint is
  the reading used here because the first waypoint of a briefing also carries
  one; that is a **guess**.
- **What `RotateTime` rotates**, and whether `NoisePercent` is static or
  interference. No shipped value contradicts either reading.
- **Whether `LoopIndex` would work.** It is −1 everywhere, so the engine's
  behaviour on any other value is untested by the data.

Everything above is re-derived by `uv run openparkan verify`.
