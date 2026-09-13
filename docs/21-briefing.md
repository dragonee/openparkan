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
    EdgeType = "spline"      how it leaves
    WaitType = "continuous"  what it does here
    EdgeTime = 3.70
    WaypointTime = 0.000
    RotateTime = 0.000
    FadeTime = 0.000
    ZoomTime = 0.000
    WaitForText = false      the holds it asks for
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
| `EdgeType` | | how the camera **leaves** this stop: `linear` 243, `spline` 89, `jump` 46 — *read* |
| `WaitType` | | what it does while here: `continuous` 362, `stay` 5, `flyaround` 11 — *read* |
| `EdgeTime` | | seconds the edge leaving this stop takes — *read* |
| `WaypointTime` | | seconds of dwell, when `WaitForTime` is set — *read* |
| `RotateTime` | | seconds per revolution of a `flyaround` — *read*, *measured* |
| `FadeTime`, `ZoomTime` | | seconds to move the fade or the zoom from this stop's value to the next stop's — *read* |
| `WaitForTime` | | whether the dwell lasts `WaypointTime` — *read* |
| `WaitForText/Sound/Click` | | loaded, copied, never consulted — *read* |
| `TextResID`, `SoundResID` | | the subtitle and the voice, resolved below |
| `FadePercent` | | the black overlay's opacity, 0 clear to 100 black — *read*, *measured* |
| `NoisePercent` | | loaded as a fraction, never consulted — *read*; 0 on all 378 |
| `ZoomOn`, `NightVisionOn` | | two camera effects, off on all 378 |
| `LoopIndex` | | the stop to go to after this one, in place of the next — *read*; **−1 on all 378** |

`EdgeTime + WaypointTime` summed over a briefing is a *floor* on its length.
The longest briefing by that floor is `CAMPAIGN.01/Mission.01` at 190 seconds
over 37 waypoints.

## How the player runs it — *read*

`iron3d.dll` loads a waypoint into a 116-byte record (`0x1002d140`) and plays
the list from a per-frame update (`0x1002f480`) that alternates two phases: a
**dwell** at waypoint *i*, then an **edge** out of it.

- **An edge belongs to the waypoint it leaves.** On leaving *i*
  (`0x100308a0`) the camera starts from where it is and heads for the next
  stop — `LoopIndex` if it is not −1, otherwise *i + 1* — and the edge
  (`0x1002f6d0`) runs *i*'s `EdgeType` over *i*'s `EdgeTime`. When it is done
  the destination becomes the current stop (`0x1002f0d0`). The last
  waypoint's edge has nowhere to go: the camera holds for its `EdgeTime` and
  the briefing ends. The reading this document used to give, travel *into* a
  waypoint, was the wrong way round.
- **`linear`** interpolates eye and look-at over `EdgeTime`; **`spline`**
  builds a curve for each (`0x10030a10`) whose end tangent depends on the
  destination — still moving along the edge into a `continuous` stop whose
  own edge is a spline, along the orbit into a `flyaround`, at rest into
  anything else; **`jump`** holds the
  camera where it is for half of `EdgeTime` while the picture goes to black,
  cuts to the destination, and comes back to the destination's `FadePercent`
  over the other half.
- **The dwell** ends at once unless `WaitForTime` is set, and then lasts
  `WaypointTime`. `WaitForText`, `WaitForSound` and `WaitForClick` are copied
  with the record and read by nothing: every function in `iron3d.dll` that
  indexes the waypoint list was checked, with `WaitForTime`'s read as the
  positive control.
- **`flyaround`** (`0x1002fec0`) orbits the waypoint's **target**: level, at
  the camera's height on arrival and its horizontal distance from the target,
  starting from its bearing, turning `2π / RotateTime` radians a second and
  always looking at the target (`0x100305c0` takes the radius, height and
  bearing). The loader refuses a `flyaround` with a `RotateTime` of 0, and
  takes a fifth word, `flyby`, as the same code when `RotateTime` is positive.
  `stay` and `continuous` do nothing during the dwell; they differ only in
  the spline tangent above.
- **The fade** is a full-screen black rectangle whose alpha is the fade level
  × 255 (`0x100315b0`). On arriving at *i* (`0x100305c0`) the player takes
  *i*'s `FadePercent` and the next stop's and moves from one to the other
  over *i*'s `FadeTime` (`0x10030120`); the zoom does the same between
  `ZoomOn` values over `ZoomTime`, and `NightVisionOn` switches its effect on
  arrival.
- The **voice** starts on arrival, from `SoundResID`; the **subtitle** drawn
  is the current waypoint's `TextResID`.
- **`NoisePercent`** is read as a whole number times 0.01 into the record's
  `+0x28` and never read again.

The shipped files agree with every part of that (*measured*):

- `RotateTime` is set on exactly the 11 `flyaround` waypoints and on no other,
  and **each of the 11 dwells for one revolution** — `WaypointTime` within
  0.05 s of `RotateTime`, `WaitForTime` set. All 89 `spline` edges have a
  positive `EdgeTime`, as the loader demands.
- **Every change of `FadePercent` between a waypoint and the next, 76 of 76,
  carries a `FadeTime` on the earlier one.** All 38 fades to black last
  exactly that waypoint's `EdgeTime` — the fade runs over the edge leaving it
  and is black as the camera arrives — and the 38 fades back in are no
  longer. All 46 jumps sit between two clear waypoints and dip on their own.
  All 20 briefings open and close black.
- `NoisePercent` is 0, `ZoomOn` and `NightVisionOn` false and `ZoomTime` 0 on
  all 378.

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
6, one skips 20, and one jumps from 14 to 100.

**The clan scripts ask for a message by that id** (*read*). `iron3d.dll`
builds a map from `message_index` to message when the mission loads
(`0x10094e90`) and plays one by id (`0x10094e30`). It is asked from two
places:

- **The game callback** every clan's SuperAI is created with (`0x10060ce0`,
  pushed at `0x100391a5`). A script calls it with two values, and the first is
  one of `varset.var`'s own `Messages` constants: `SYSTEM_MESSAGE` 0 records
  the mission's result (`MISSION_FAILED` / `MISSION_COMPLETE`),
  **`MESSAGE_INFO` 1 plays the message whose id is the second value**,
  `CLAN_HERO_KILLED` 2 does nothing in this build, and `OBJECTIVE_COMPLETE`,
  `OBJECTIVE_FAILED` and `OBJECTIVE_PROGRESS`, 3 to 5, set an objective's
  state ([17-saves.md](17-saves.md)). In `ai.dll` the call is made by the
  handler stored at the interpreter's `+0x78` (`0x1000c266`), which evaluates
  two operands and passes them on.
- **`iron3d.dll` itself**, in game mode 4: id 22 from the order menu
  (`0x10058015`) and id 100 from `0x100638a9`.

Measured against the scripts: **every call a script makes with a message
constant goes through one function id, `fn30`** — 244 calls, 99 of them
`MESSAGE_INFO` — and **all 22 literal ids** the clan scripts of 13 missions
pass with `MESSAGE_INFO` are `message_index` values of that mission's
`messages.cfg`; the training missions mostly pass a counter, `dMCount`,
instead. The two ids the game asks for itself are each in exactly one file,
both in the training campaign — 22 in `CAMPAIGN.00/Mission.03`, 100 in
`CAMPAIGN.00/Mission.02` — which is why those files skip to them.

## Not established

- ~~What `flyaround` orbits~~ — **the waypoint's target**, level, once per
  `RotateTime` (*read*; 11 of 11 dwell one revolution).
- ~~Which end of an edge `EdgeTime` belongs to~~ — **the end it leaves**
  (*read*; every fade to black lasts it).
- ~~What `RotateTime` rotates~~ — the `flyaround` orbit, and nothing else.
- ~~Whether `NoisePercent` is static or interference~~ — **neither: this build
  never reads it** after loading. What a value was meant to do cannot be
  learned from this binary.
- ~~What `LoopIndex` other than −1 would do~~ — send the camera to that
  waypoint after this one, instead of the next; an index at or before the
  current one loops the flythrough (*read*; no shipped briefing sets one).
- ~~Who asks for a message by `message_index`~~ — the clan script, through
  the SuperAI's game callback with `MESSAGE_INFO`, and the game itself for 22
  and 100.
- **What game mode 4 is.** `iron3d.dll:0x1005c748` compares the settings
  object's first word with 2, 3 and 4 into three flags; mode 4 is the one that
  asks for messages 22 and 100, which only training missions carry, so the
  training campaign is the likely reading — a **guess**.
- **How a briefing is skipped**, and what `WaitForClick` was for; nothing that
  indexes the waypoints reads it.
- **The spline's own curve** (`0x1002cc50`, set up at `0x10030a10`) beyond its
  end tangents.

Everything above is re-derived by `uv run openparkan verify`.
