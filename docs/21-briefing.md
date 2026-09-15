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
  waypoint, was the wrong way round. Esc skips the rest (`iron3d.dll:0x10070e75`),
  and a mission whose `mission.cfg` says `only_briefing` is won the moment its
  briefing ends ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
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

## How the briefing is shown — *read*, and *measured*

What follows is enough to play a briefing frame for frame: when it starts and
ends, what it draws on the 640 × 480 screen the 2D calls take
([35-hud.md](35-hud.md#everything-is-drawn-on-a-640--480-screen--read)), the
camera it looks through, the exact curve of every edge, and what the rest of
the world does meanwhile. *Measured* here means against a 960 × 720 recording
of Mission 01, 1.5 × that screen, whose first frame of the briefing is at
1.20 s.

### When it runs — *read*

**Starting** (`iron3d.dll:0x100a29f3`–`0x100a2aa5`). Unless a saved game is
being loaded (the game's `+0xe5`), the mission set-up creates the briefing
object (`0x10030dd0`, one global at `0x1010a148`) and starts it
(`0x10031130`):

1. It looks up `mission.cfg`'s `briefing` object and its `filename`, and loads
   the waypoints ([above](#how-the-player-runs-it--read)). **A mission without
   one, or a file that fails to load, has no briefing**: the object is
   destroyed and play starts at once.
2. It copies the game's `+0xd4` string, **the first line of the mission
   directory's `descr` file**, as the title. The mission loader reads that line
   with `fgets` into 128 bytes (`0x1005dfb1`–`0x1005e08a`); Mission 01's is
   *Line of Fire*.
3. It reads `Iron_3D.ini`'s `[CS] SUBTITLES` as an integer: subtitles show
   when it is not 0.
4. It stops the CD music (`0x1008e620`) and plays `mission.cfg`'s `cd_track`
   if there is one.
5. It creates the camera (`0x100364a0`) and aims it at waypoint 0: at its
   `CameraXYZ`, looking at its `TargetXYZ` (`0x10030220`).

Then every object but the hero is paused (below), a shown cursor is hidden
(`0x100a2aaa`), and the level's state word `+0x710` becomes **5**.

**The first frame.** The per-frame update does nothing while no waypoint is
current. The first time the screen is drawn (`0x100315b0`, its `+0x1a9`
latch) it sends the player to waypoint 0 at the timer's time now (`ITimer`
slot 2, milliseconds). **The clock of the flythrough starts on that frame.**

**Each frame in state 5.** The game frame updates the player with the timer's
time (`0x100a55c0`, the state-5 case) and the screen layer draws the briefing
screen **and nothing else** (`0x1008d302`): no HUD, no message box, no
objectives screen, no map.

**Ending.** The player sets its finished byte (`+0x1a4`) when the last edge
is done, and Esc sets it at any time (`0x10070e75`). The next game frame
(`0x1005e7a4`):

1. destroys the briefing object (`0x10030ed0`). Its destructor
   (`0x10030f10`) asks the resource manager to drop its cache (slot 10,
   `services.dll:0x10008e70`), calls slot 11 of `Ngi32.dll`'s 3D sound object
   (`niGet3DSound`, not followed), and **plays the mission's `THEME`** through
   the sound server's slot 2;
2. sets the driven unit's record (`+0xaec`) back to `+0xa2` = 1 and `+0x9c` =
   0, and calls its `+0x50` object's slot 10 (the start called slot 9; neither
   followed);
3. un-pauses every object (below) and sets the state word to **1**;
4. wins the mission if `mission.cfg` says `only_briefing`
   ([34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).

A skip is the same ending one frame after Esc. **The fade goes with the
screen**: it is drawn only in state 5.

**The objectives screen that follows** is not the briefing's. The mission
set-up opens it before the briefing starts when not loading a save and the
parameter block's `+0x154` is 1 (`0x1005e11f`–`0x1005e147`; the executable
sets it, and a load from inside the game clears it,
[34-progression.md](34-progression.md#after-the-outcome--read-and-measured)).
It shows once the state word is no longer 5. In the recording it is up at 99.6 s, 0.4 s
after the model's end (*measured*).

### The world meanwhile — *read*, and *derived*

- **Every object but the hero is paused.** `0x100a1ca0` walks the world's
  objects and sets property `0x20a` to 1 on each whose `Type` is not
  `0x1020000`, and to 0 at the end. `MBehaviour` keeps it at `+0xa08`
  (`Behavior.dll:0x1000ab0d`). Setting it stops the unit (`0x1003c540` on its
  walker). Clearing it sends the unit's task planner to where it stands
  (`0x10034930`). While it is set, the behaviour takt returns early
  (`0x10004f20`): no default or restored orders, no places, **no radar, no
  unit or building takt and no fire control**.
- **The clan scripts wait.** The game frame calls no SuperAI `Mission` handler
  and no clan takt in state 5
  ([34-progression.md](34-progression.md#when-the-mission-handler-runs--read)),
  so no mission message plays until the briefing is over (*derived*). It also
  skips the driven unit's loss test (`0x1005ed75`).
- **Nothing in the briefing moves a unit.** The hero stands at its spawn: the
  camera of waypoints 8–12 looks at (433.119, 477.063, 14.099), the hero's
  (433.0, 477.1, 14.1), and the recording shows it there.
- **The sky's clock runs.** *Measured*: the sky above the recording's view is
  (110, 61, 82) at 9.5 s, (95, 66, 90) at 30 s, (166, 154, 199) at 70 s and
  (182, 169, 208) at 85 s. Mission 01's day is 900 s and starts at 01:30
  ([10-sky.md](10-sky.md#where-the-clock-starts)), so the briefing's 98 s
  carry it to about 04:10: from `#642d3c` at 01:30 through `#aa7896` at 02:40
  to `#beaad2` at 03:40. The early purple is dawn (*derived*).
- **Music.** No `THEME` plays until the end. In the recording the sound is
  near silent, some 40 dB under the voices, from 94.5 s to 99.5 s, and music
  starts at 99.65 s (*measured*).

### The screen — *read*, and *measured*

The screen draw (`0x100315b0`) makes these GUI-server calls
([35-hud.md](35-hud.md#how-the-radar-draws--read)'s table), in this order:

| # | call | arguments | what |
|---:|---|---|---|
| 1 | filled rectangle, alpha kept | (0, 0) – (display width, display height), `0x00000000` + alpha round(fade × 255) | the fade, over everything drawn before it |
| 2 | filled rectangle | (0, 0) – (640, 75), `0xff000000` | the top bar |
| 3 | filled rectangle | (0, 405) – (640, 480), `0xff000000` | the bottom bar |
| 4 | text with a shadow | `MENU_FONT` (the game's `+0x14`), the title, (20, 20), `0xff808080` | the title |
| 5 | text with a shadow, per line | `GAME_FONT` (`+0x10`), line *i*, (10, 410 + *i* × *l*), `0xff808080` | the subtitle, only when `SUBTITLES` is on |

- **The fade is under the bars and the words**, which are drawn after it. The
  recording shows the title and the first subtitle over black from 1.20 s.
- **The line step** *l* is round((`GAME_FONT`'s height + 1) ÷ *sy*), the height
  in pixels, *sy* the display's height ÷ 480 (`0x10031763`–`0x10031786`).
  Every line of the text is drawn; there is no cap.
- **The text** is the current waypoint's `TextResID`, looked up through the
  resource manager, and is re-wrapped only when that id changes
  (`0x10031840`). An empty id clears it. The wrap (`0x10093140`, the same one
  the message box uses) fills a line while its words fit **0.98 × 640 × *sx***
  pixels (*sx* the display's width ÷ 640), and breaks at a newline.
- After drawing, the draw puts the 3D viewport back to (0, 75) – (display
  width, display height − 75) in pixels (`0x100317ee`), the rectangle the
  camera was created with (`0x100319e0`). At 640 × 480 that is exactly the gap
  between the bars; above that it reaches under them.

*Measured* at 960 × 720:
- the bars cover y < 112 and y ≥ 608;
- the title's ink is x 30–140, y 31–47, 1 px below its pen at (30, 30);
  "MISSION COMPLETE !" sits the same way below its pen in the same recording;
- the subtitle's lines start at x 15 and y 616, 626, 636, 648, 658 and 668,
  which is 10.5 px (a step of 7) rounded;
- the widest of its lines ends at x 943, inside 15 + 0.98 × 960;
- both texts are grey, ink up to (147, 147, 147).

### The camera — *read*, and *measured*

**What it sees.** `0x10031130` creates the view with a near and far of **3
and 700** and an angle of **1.04 rad** (`0x10031532`–`0x1003153c`). Those land
in the view's parameter block at `+0xc`, `+0x10` and `+0x14`, where the unit
camera puts its 0.1, 1000 and 1.3 ([30-turrets.md](30-turrets.md)), so the
angle is horizontal. That near and far are what the two numbers are is the
same *guess* as there. Each frame the zoom sets the angle to
(1 − *z*) × 1.04 + 0.2 × *z* (`0x10036c90`), and *z* is 0 on every shipped
waypoint.

*Measured*, from the waypoint-1 camera at 9.5 s:
- the targets `M_targ` 15 and 16 and the trees at (771.0, 394.9) and
  (875.3, 568.8) project, through 1.04 rad across 960 pixels centred on
  (480, 360), within about 10 px of where the recording shows their bases;
- 1.3 rad would put `M_targ` 15 80 px nearer the middle;
- **the pixels are square**: the vertical scale is the horizontal one, and the
  letterbox only clips.

**How it is aimed** (`0x1002f6d0`, `0x1002fec0`, `0x10030220`, `0x100303d0`,
all *read*). With the eye *E* and the look-at point *A*:
- F = normalise(*A* − *E*);
- R = normalise((0, 0, 1) × F);
- U = F × R.

The camera's matrix columns are F, R, U and *E*: x forward, y left, z up. **The
world's z is always up, so a briefing camera never rolls**, and one looking
straight up or down has no R.

### Every frame of the path, exactly — *read*

The fields are the record's ([above](#how-the-player-runs-it--read)). Times are
the timer's milliseconds × 0.001. The player keeps:
- the time of arrival *t*ₐ and of leaving *t*ₗ;
- the eye *P*₀ when it left;
- the look-at *A*₀, which is **the leaving waypoint's own `TargetXYZ`**, not
  where the camera was looking;
- the destination's camera *P*₁ and target *A*₁, which are *P*₀ and *A*₀ again
  when there is no next stop;
- a velocity *V*.

The update (`0x1002f480`) does this each frame:

1. **The fade and the zoom** (`0x10030120`), by *t* − *t*ₐ:
   - level = *f*ᵢ + (*f*ₙ − *f*ᵢ) × (*t* − *t*ₐ) ÷ `FadeTime` while that is
     under 1;
   - after that, or with `FadeTime` 0, the next stop's *f*ₙ at once;
   - on a `jump` waypoint the level is *f*ᵢ, and the edge overrides it.
2. **The dwell.**
   - A `flyaround` places the eye (below).
   - The dwell ends when `WaitForTime` is clear, or when *t* − *t*ₐ ≥
     `WaypointTime`.
   - Leaving (`0x100308a0`) takes *P*₀ from the camera, *A*₀, *P*₁, *A*₁ and
     *t*ₗ = *t*, builds the spline if the edge is one, and runs the edge in
     the same frame (`0x1002f600`).
3. **The edge**, with *s* = *t* − *t*ₗ and *T* = `EdgeTime`. When *s* ≥ *T*
   the destination is arrived at in the same frame (`0x1002f0d0`):
   - its voice, *t*ₐ = *t*, *f*ᵢ and *f*ₙ, the orbit;
   - then its fade and dwell;
   - with no destination, the player stops and the briefing ends.
   - Arrival runs its dwell, and its edge if the dwell is already over, at
     once. If that edge is over in the same frame, which only an `EdgeTime` of
     0 would do, the player stops and the briefing ends there; a zero edge
     left in a later frame goes straight on to the next stop instead. No
     shipped waypoint has a zero `EdgeTime` (*measured*, 0 of 378).

**The edges** (`0x1002f6d0`):

- **`linear`**, while 0 ≤ *s* < *T* (and *T* > 0):
  - with *u* = *s* ÷ *T*, eye = *P*₀ + *u*(*P*₁ − *P*₀) and look-at = *A*₀ +
    *u*(*A*₁ − *A*₀);
  - otherwise eye *P*₁ and look-at *A*₁;
  - *V* = (*k*, *k*, *k*) with *k* = |*P*₁ − *P*₀| ÷ *T* whenever *T* > 0, at
    and past the end too, **one speed in all three components**, not a vector;
    0 when *T* is 0.
- **`jump`**, *V* = 0. *f*ᵢ and *f*ₙ are this waypoint's and the next's
  `FadePercent`, and *h* = *T* ÷ 2:
  - for *s* < *h*: eye *P*₀, look-at *A*₀, fade *f*ᵢ + (1 − *f*ᵢ) *s* ÷ *h*;
  - for *h* ≤ *s* < *T*: eye *P*₁, look-at *A*₁, fade 1 + (*f*ₙ − 1)(*s* − *h*) ÷ *h*;
  - after that: *P*₁, *A*₁ and *f*ₙ.
- **`spline`** (`0x1002cc50` builds, `0x1002ce90` evaluates, `0x1002cfa0`
  differentiates):
  - the curve H(*p*₀, *p*₁, *a*, *b*; *u*) = *p*₀ + *a u* + *c*₂ *u*² + *c*₃ *u*³;
  - *c*₂ = 3(*p*₁ − *p*₀) − 2*a* − *b* and *c*₃ = *a* + *b* − 2(*p*₁ − *p*₀);
  - *u* = *s* ÷ *T*, clamped to the ends: a cubic Hermite with end tangents
    *a* and *b*;
  - eye = H(*P*₀, *P*₁, *T V*₀, *T V*₁; *u*);
  - look-at = H(*A*₀, *A*₁, 0, 0; *u*), which eases in and out;
  - *V* = (*a* + 2*c*₂*u* + 3*c*₃*u*²) ÷ *T* inside the edge, but *T V*₀ at its
    start and ***T V*₁ at its end**.

  *V*₀ is whatever *V* the previous phase left. *V*₁ is set by the destination
  (`0x10030a10`):
  - a `continuous` destination whose own edge is a `spline` gets
    normalise(â + b̂) × (|*a*| + |*b*|) ÷ (*T* + *T*ₙ), with *a* = *P*₁ − *P*₀,
    *b* = the stop after's camera − *P*₁, *T*ₙ its `EdgeTime`, and unit
    vectors of 0 for lengths of 0;
  - a `continuous` destination whose own edge is `linear` and timed gets
    (the stop after's camera − *P*₁) ÷ *T*ₙ;
  - a `flyaround` destination gets its orbit's tangent (−ω *d*ᵧ, ω *d*ₓ, 0),
    with *d* = its camera − its target and ω = 2π ÷ `RotateTime` (the loader
    keeps ω at `+0x6c`);
  - anything else, and the last stop, gets 0.
- **`flyaround`**, during the dwell (`0x1002fec0`). Arrival keeps the eye's
  bearing about the target, θ₀ = atan2(*E*ᵧ − *A*ᵧ, *E*ₓ − *A*ₓ) (0 when
  closer than 0.0001), its horizontal distance *r* and its height *z*. Then:
  - θ = θ₀ + ω(*t* − *t*ₐ);
  - eye = (*A*ₓ + *r* cos θ, *A*ᵧ + *r* sin θ, *z*), looking at the target;
  - *V* = (−ω*r* sin θ, ω*r* cos θ, 0);
  - so the orbit turns anticlockwise seen from above.

**The voice** starts on arrival through the sound server's slot 2
(`0x10030836`), which plays it at once past the message queue and stops
nothing ([34-progression.md](34-progression.md#messages--read-and-measured)).

- *Measured*: under the model, 164 of the 165 briefing voices end before the
  next voice starts or the briefing ends. The one that does not,
  `CAMPAIGN.02/Mission.03`'s `C02M03_T01`, runs 0.05 s over. So whether a
  voice cuts another never shows.
- In the recording, the sound rises 0.08 to 0.17 s after the subtitle changes on
  nine of the ten voiced waypoints after the first, and 0.47 s before it on
  T01_T03. The recording's sound and picture are not locked closer than that;
  that is a *guess*.

### Against the recording — *measured*

Mission 01 lasts **97.965 s** under the model. The recording's events, found
by frame differencing, against the model's arrival times + 1.20 s:

| waypoint | what changes | model | recording |
|---:|---|---:|---:|
| 0 | title and T01_T01 over black | 1.20 | 1.20 |
| 1 | T01_T02; fade in over 3.024 s | 4.90 | 4.93; the picture rises from 5.03 to 8.0 |
| 2 | `jump`: black at its middle | 10.98 | 11.07 |
| 5 | fade out over 2.071 s | 33.14 – 35.21 | black by 35.27 |
| 7 | fade in over 2.038 s | 35.71 | rising from 35.83 |
| 8 | T01_T03 | 37.74 | 37.83 |
| 12 | T01_T04 | 60.05 | 60.17 |
| 13 | `jump`, subtitle cleared | 64.11 | 64.30 |
| 14 | MIS_OBJ, *Battle Mission:* | 66.18 | 66.37 |
| 15–17 | T01_T05, T01_T06, T01_T08 | 68.23, 70.79, 73.29 | 68.43, 70.97, 73.47 |
| 18 | `jump`, cleared | 77.31 | 77.60 |
| 19, 20 | T01_T09, T01_T10 | 78.84, 85.92 | 79.17, 86.23 |
| 21 | `jump`, cleared | 88.97 | 89.27 |
| 22 | T01_T11 | 90.53 | 90.87 |
| 24 | cleared; fade out over 2.052 s | 96.61 | 96.97; black by 99.0 |
| — | the end | 99.17 | the objectives screen at 99.6 |

**The recording falls behind by about 14 ms a waypoint**, 0.35 s over 25. That
is *derived* from the update. A phase ends on the first frame past its time,
and the next phase counts from that frame, so each change loses part of a
frame; 14 ms is half a frame at about 35 frames a second.

**Mission 04 needs nothing the first two did not** (*measured*).

- **The path.** Its 22 waypoints use `spline`, `linear` and `jump` edges, and
  `continuous` and `flyaround` dwells, all of which Missions 01 and 02 already
  use. The two `flyaround`s, 17 and 19, dwell one turn each: 5.50 and 5.54 s
  against `RotateTime` 5.52 and 5.55.
- **The length.** It lasts **68.1 s**.
- **Against its recording.** The darkest frame of each of its five `jump`s
  falls at the jump's middle, the model's arrival + 1.2 s + half the edge:

  | waypoint | model | recording |
  |---:|---:|---:|
  | 4 | 22.31 | 22.23 |
  | 8 | 30.06 | 29.97 |
  | 14 | 44.40 | 44.40 |
  | 16 | 50.04 | 50.00 |
  | 18 | 59.60 | 59.70 |

- **What follows.** The objectives screen is up at 69.4 s, 0.1 s after the
  model's end, and the cockpit follows 7.0 s later.

### For an engine

1. Load the waypoints, the title from `descr` and the subtitle switch. Set
   property `0x20a` on every object but the hero: stop it, and give it no
   orders, radar, fire or scripts. Skip the clan scripts' `Mission` and takts.
   Keep the sky's clock running. Draw no HUD.
2. On the first drawn frame, arrive at waypoint 0 at the time now.
3. Each frame run the update above:
   - draw the world through the eye and look-at with a 1.04 rad horizontal
     angle, square pixels, z up and no roll;
   - over it, the fade, the two bars, the title and the wrapped subtitle.
4. When the last edge ends, or on Esc:
   - un-pause the objects;
   - start `THEME`;
   - open the objectives screen if it was asked for;
   - hand the view to the cockpit.

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
- **`iron3d.dll` itself**, in game mode 4: id 22 as a build command puts the
  building's model under the cursor (`0x10058015`, in `0x10057f00`,
  [34-progression.md](34-progression.md#what-the-scripts-ask--read-and-measured-1))
  and id 100 from `0x100638a9`.

**How a message plays** is [34-progression.md](34-progression.md#messages--read-and-measured)'s
(*read*):
- the first request voices it through the sound server's queue, so voices
  play one after another, and files its text in the message history;
- a repeat only adds "Recieved message is already in history";
- a message can set a fourth key, `info_system`, on 46 of the 99.

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
  training campaign is the likely reading — a **guess**. Message 100 is asked
  when the unit the player takes over is a flyer (`0x10075f70`), and a
  recording of Mission 02 played from the campaign shows it as the hero boards
  its warbot ([34-progression.md](34-progression.md#seen-in-a-recording)).
- ~~How a briefing is skipped~~ — Esc sets the player's finished byte and
  the next frame ends it as if the path had run out
  ([How the briefing is shown](#when-it-runs--read)). **What `WaitForClick`
  was for** stays open; nothing that indexes the waypoints reads it.
- ~~The spline's own curve~~ — **a cubic Hermite over `EdgeTime`**, the
  look-at's with zero end tangents
  ([Every frame of the path](#every-frame-of-the-path-exactly--read)).
- **Whether a voice still speaking stops** when the briefing ends or is
  skipped. The destructor drops the resource manager's cache and calls slot
  11 of `Ngi32.dll`'s 3D sound object; neither was followed.
- **What the driven unit's record does with its `+0x50` object's slots 9 and
  10**, called as the briefing starts and ends, and what its `+0xa2` and
  `+0x9c` hold.
- **Which keys do anything in state 5** besides Esc, and whether the timer the
  player reads (`ITimer` slot 2) stops when the game is paused.
- **The first twelve bytes of the briefing view's parameter block**, which
  `0x100364a0` does not write before handing it over, and the names of the
  near and far planes (a *guess*, as for the unit camera).
- **Whether the ambient variations play** during the briefing. The game
  frame's random 10–20 s sound timer (`0x1005eb44`) is not gated on state 5,
  but what it plays was not followed, and the recording does not settle it.

Everything above is re-derived by `uv run openparkan verify`.
