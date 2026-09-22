# Selecting and ordering — the cursor in command mode

In command mode the player points: at units to select them, and at a target to
send them there. *The Field Base*'s tip T03_H07 says the cursor's image "will
vary depending on the type of target you select", and that the same can be done
on the satellite map. This page reads that machinery in `iron3d.dll`:

- the mouse's way in;
- the selection;
- the pick under the cursor, and the cursor it shows;
- what a left click, a drag and a right click do;
- the pending picks an order row leaves open.

What the orders do once given is [31-packages.md](31-packages.md). How command
mode is entered, its camera and what it draws is
[40-command-mode.md](40-command-mode.md). The commander panel's pages and rows
are [41-commander.md](41-commander.md). The building ghost a Build row places is
[32-builder.md](32-builder.md). The satellite map's panel is
[35-hud.md](35-hud.md#the-satellite-map), and the marks drawn on selected units
are [25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured).

**Every claim is tagged**, as in [23-economy.md](23-economy.md): *measured* is
re-derived by `openparkan verify`, *read* comes from the disassembly at the
address given, *seen* from the recording of *The Field Base* (960 × 720,
halved here to the game's 640 × 480 layout), *derived* follows from those, and
*guess* fits the evidence but is not established.

## The mouse's way in — *read*

**The game's input listener** is the vtable at `0x100e6490`. The game hands
each event to a chain of listeners, and stops at the first that answers 1:

| slot | chain call | handler | what it is |
|---:|---|---|---|
| 0 | | `0x10070db0` | characters (Esc, digits; [36-factory.md](36-factory.md#what-the-controls-do--read)) |
| 3 | `0x10070ad0`, which first sets `0x1010bf7e` | `0x10071410` | the mouse moving |
| 4 | `0x10070b20` | `0x100714d0` | the left button going down |
| 5 | `0x10070b70`, which first clears the held flag `0x1010bf7c` and its stamp `0x1010bf80` | `0x10071620` | the left button coming up |
| 6 | | `0x100716b0` | the right button (*derived*: what it does, below) |

**Where the cursor is.** `0x1010414c` and `0x10104150` hold its x and y in
window pixels. A handler that works in the 640 × 480 layout divides them by the
display's scales (`IDisplay` slots 4 and 5).

**Only view state 2 picks** (the level's word `+0x710`). The pick below gives
nothing in states 1, 3, 4 and 6 (`0x1008daa4`–`0x1008dac8`), and only state 2
lets a drag become a band (`0x10071476`). So state 2 is the commander's view
(*derived*; how it is entered is [40-command-mode.md](40-command-mode.md)).

**The left button going down** (`0x100714d0`):
1. It sets the held flag `0x1010bf7c`, clears `0x1010bf7e`, and stamps
   `0x1010bf80` with the timer's milliseconds (`ITimer` slot 2).
2. With the game menu on the mode stack's front (mode 7), the interface's
   handler `0x1008d690` takes the click at once.
3. Otherwise nothing happens:
   - while the game is paused (game `+0xe5`);
   - in view state 1 with a driven unit (`+0xa2`, the cockpit);
   - in view state 6 with a building.
4. Otherwise `0x1008d690` gets the click at its layout point. If it does not
   take the click, the band's anchor is set at the window point (`0x100588f0`
   on `0x1010b59c`).

**The interface's handler** (`0x1008d690`) takes the click, in turn:
1. **Swallowed:** while the objectives screen is up (`+0x592`,
   [35-hud.md](35-hud.md#the-objectives-screen)); while a modal widget answers
   (the screens' `+0x1c`, `+4`, slot 3); or in view state 3.
2. **Straight to the world handler** `0x1008fe80` while the cursor is in state 8,
   the building ghost ([below](#the-cursor-shows-a-state--read-and-measured)).
3. **On a commander page** (mode stack front 3, 4 or 5):
   - a click inside (374, 0)–(640, 42), the resource rows, is swallowed;
   - otherwise the commander panel's click `0x100841a0`
     ([41-commander.md](41-commander.md)) may take it.
4. **Anything left** goes to the world handler `0x1008fe80`.

A click on a panel row selects that unit alone, through `0x1007d0a0` with
`VOICE_SELECTED` ([41-commander.md](41-commander.md)). In cursor states 7 and
8 the command camera's edge turns stop
([40-command-mode.md](40-command-mode.md)).

## The selection — *read*

**Units.** A unit is selected when its record's byte `+0x80` is 1; selecting
also writes −1 to `+0x84`. The records are on the level's list `+0x720`.

**`0x10076e70` collects the selection** for everything below. It returns
nothing unless the game's word `+8` is 4, and it takes each record that is:
- selected;
- of the player's clan (level `+0xad0`);
- alive: `+0x48`'s slot 11 does not answer `0xfffe`;
- of a Type that passes the caller's mask. −1 passes everything, and a mask
  must contain the whole Type.

**Buildings** are selected on the level's list `+0x71c`: `0x1007db80` selects
one, `0x1007dd30` clears the list, and `0x1007ddf0` answers the current one
([36-factory.md](36-factory.md#how-the-screen-opens--read)).

**The selection's kind** is `0x1010c384`:

| value | means | set by |
|---:|---|---|
| 0 | nothing selected | a clear |
| 1 | units | `0x1007d0a0`, a band |
| 2 | a building | `0x1007db80` |

**Selecting a unit** (`0x1007d0a0`, with the list, the unit and a replace flag):
1. It does nothing without a unit, or for a hero (Type `0x1020000`).
2. For a unit not selected before, and the game not paused, it says
   `VOICE_SELECTED`.
3. It sets the pick mode `0x1010c388` to 0 (below) and clears the building
   selection.
4. With the replace flag it deselects every other unit. If the kind was 1, the
   kind goes to 0.
5. It selects the unit, appends it to the list, and sets the kind to 1.

**A click in the world always replaces.** It passes the flag as 1
(`0x100901ee`). No path of the world handler or of `0x1007d0a0` reads a key
state, so neither Shift nor Ctrl adds to a selection (*read*, as a search of
both).

**Digits select no group in command mode.** The character handler's cases for
1–9 act only in view states 1 and 3, the wingman selector's
(`0x100710fa`; [31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)).

**Turning the panel to a unit page** keeps a single selected unit of the
page's type. Otherwise it selects the first unit of that type (`0x10084f66`,
`0x10072be0`, replacing). The page itself is [41-commander.md](41-commander.md).

A selected unit gets its marker in the world, and on the map a white outline
([25-sensors.md](25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured)).
*Seen* at 186.5 s: the builder just ordered shows its blue brackets, the name
"SWB-2 Builder" in green, its class icon, and its green and orange bars.

## The band — *read*

A drag with the left button selects the player's units inside a rectangle.

**It starts** in the mouse-move handler (`0x10071410`), which moves the cursor
to state 7 (`0x100714ba`) when all of these hold:
- the view state is 2 and the designer is not up;
- the button is held;
- the pick mode is not Route (5);
- more than **0.35 s** have passed since the button went down: `ITimer` slot 3
  gives the seconds since the stamp, compared with `0x100e64b8` (0.35).

**While it is up** a move writes the band's far corner (`0x1005850b`). Each
frame draws the rectangle from the anchor to the cursor in `0xff19b419`, opaque
(25, 180, 25), converted to the layout, then the arrow over it (`0x10058631`).

**When the button comes up** (`0x10071620`):
1. The corners are sorted (`0x10071b90`).
2. If the band is at least 10 window pixels across *and* down (`0x10071659`,
   `0x1007166e`), `0x10076820` selects inside it. If it selected any, the kind
   is 1.
3. The cursor goes back to state 1 either way.

A band smaller than 10 selects nothing and clears nothing. The click at its
start has already acted, on the way down.

**Selecting inside it** (`0x10076820`):
1. It turns the commander panel to page 0, and clears the building selection
   and the unit selection.
2. **Over the open satellite map**, it maps both corners to the world
   (`0x10074110`). It then takes every unit that stands inside that rectangle
   and is:
   - not selected yet;
   - of the player's clan;
   - alive;
   - not a hero.
3. **Anywhere else**, it takes the same units whose projected point in the
   layout (`0x1007dff0`, which also answers whether the unit is in view) lies
   inside the band.
4. It selects each one (`+0x80` 1, `+0x84` −1) and counts them.
5. If it took any and the game is not paused, it says `VOICE_SELECTED` once.

**A right click during the band** cancels it: the cursor goes to state 1
(`0x1008fb46`).

## The pick under the cursor — *read*

`0x1008da40` fills the **cursor object** at game `+0x24`:
- `[0]`, the unit or building record under the cursor;
- `[4]` and `[8]`, the world x and y;
- `+0x1c`, set when the point came from the map.

It returns a **kind**. The chooser and the click below both call it.

### What it looks at

1. **A placement shortcut.** With the pick mode at 4 or 6 (placing a building)
   it answers 13 at once.
2. **Nothing** (kind 0) in any of these cases:
   - view states 1, 3, 4 and 6;
   - the objectives screen is up (`+0x592`);
   - the warbot designer is up (the screens' `+8`, `+0x2c`);
   - the commander panel's hit test (`0x10084b80`) answers for the cursor;
   - the screens' `+0xc` byte `+0x20` is set;
   - a modal answers;
   - the cursor is in (374, 0)–(640, 42).
3. **On the open satellite map** (`+0x261`; the cursor inside the map's panel,
   `0x10073fd0`):
   - **The world point.** It is (u, v) × L ÷ 256 (`0x10074110`), with the
     ground's height at it. u is the cursor's distance right of the minimap's
     left edge (x₀ + 5), v its distance up from the bottom edge (y₁ − 5), and
     L the map's side ([35-hud.md](35-hud.md#the-panel-in-the-cockpit--read-and-seen)).
     In command mode the panel is the `+0x1f8` one, (374, 63)–(640, 329).
   - **The object.** The first live unit on the level's list whose (x, y) is
     within **40** of the point (`0x10072b70`), or else the first live building
     within **80** (`0x100728e0`). The list's order decides, not the distance.
     The object is dropped unless it passes `0x1007e660` for the player's clan:
     its own, or one listed at the clan record's `+0x54`.
   - **Over the map's frame or title bar** (`0x10074060`) the answer is 0.
4. **In the world:**
   - **The world point** comes from a ray from the camera through the cursor
     (`0x10035e40`). The ray goes into `IWorld` slot 7, the sight ray's query
     ([29-weapons.md](29-weapons.md)). The point must lie inside the map with a
     margin of 0.001 of its side on every edge (`0x10035eee`).
   - **The object** comes from `0x100360f0`, which walks the world's lists of
     object classes 3 and 4 (`0x100361a0`): **the buildings, then the units**
     ([below](#the-object-pick--read)). Its record is found on the unit or the
     building list (`0x10077450`).

### The object pick — *read*

**Classes 3 and 4 are the buildings and the units.** The world keeps a list per
object class (`World3D.dll`'s queue, slot 13 at `0x10007a00`, 3000 objects to a
class from `0x1003a75c`), and its add (`0x100055f0`) files each object under the
class the object itself answers (its slot 11, `0x1000562a`), the class it was
loaded as. That is the class `CreateObjectFromScheme` asks for, 3 for a building
and 4 for a robot ([22-settings.md](22-settings.md)), and the one the add folds
into the object's id (`0x10005636`), whose nibble 3 `iron3d.dll` hands to its
building list and 4 to its unit list
([29-weapons.md](29-weapons.md#how-the-ai-fires--read)).
The name line tests the same answer: 3 is a building
([35-hud.md](35-hud.md#name-and-status--read-and-seen)).

**The walk** (`0x100361a0`, once per class) takes the ray from the eye as a
segment, its start the camera's place and its end on the far side
(`0x100cd6d0`), and for each object on the class's list:
1. its slot 3 must answer non-zero, and its interface `0x16` slot 11 must not
   answer `0xfffe` (alive). The list holds game objects (*inferred*: the add
   reads the class from the same slot 11 docs/35's name line reads off a
   game object), and `IGameObject` slot 3 is the object's parent
   ([31-packages.md](31-packages.md), [39-boarding.md](39-boarding.md)), so an
   object attached to nothing is passed over; what a placed object's parent
   is, and whether the hero has one in command mode, was not traced;
2. its bounding sphere (interface `0x18`, slot 9 with 2) must not lie wholly
   behind any of the view's six planes (`0x10036280`);
3. **the eye must not be inside the sphere**, at its whole radius
   (`0x100362d8`);
4. the radius is then scaled — **by 0.7 for class 3, the buildings, and 1.0
   for class 4, the units** (`0x1003614b`, `0x10036171`) — and the ray's
   *line* must pass within it: the quadratic's discriminant is not negative
   (`0x100924e0`);
5. the centre must be nearer the start than the segment is long
   (`0x1003637f`), and ahead of it: the segment's direction dotted with the way
   to the centre is above 0 (`0x100363a6`);
6. of those, **the one whose centre is nearest the start, in a straight line,
   wins**: its squared distance must be strictly below the best so far
   (`0x100363e3`).

**The two walks share one best distance**, set to `FLT_MAX` before the first
(`0x1003615b`). The buildings are walked first, and `0x100360f0` keeps the
units' answer when it has one (`0x10036188`). A unit is found only when it is
strictly nearer than every building found, so **the nearest object wins, and a
tie keeps the building** (*derived*).

It differs from what this page said before on two counts: the shares were the
other way round — a building's sphere is the shrunken one — and a class-4 hit
is not preferred as such, only when it is the nearer.

### The kinds it answers

The world point and the object then decide the kind. "Valid" means the place
test `0x10076770` passes ([below](#a-valid-place--read-and-measured)).

**Before anything else:**

| when | kind |
|---|---|
| the object is a building whose `0x20c` reads non-zero, its construction sphere running ([32-builder.md](32-builder.md)); the object is dropped | 2 |
| the first selected unit is the player's hero | 2 |
| the pick mode is Route (5): valid / not | 12 / 2 |

**Otherwise, by the selection's kind:**

| selection | pick mode | object | kind |
|---|---|---|---|
| none | — | the player's own unit or building | 7 |
| none | — | anything else | 0 |
| a building | — | an own unit | 7 |
| a building | — | the current building | 17 |
| a building | — | another own building | 7 |
| a building | — | anything else | 0 |
| units | free (0) | an own unit, the one and only selected | 17 |
| units | free | another own unit | 7 |
| units | free | a unit of another clan | 3 |
| units | free | an own building | 10 |
| units | free | a building of another clan, every selected unit of size class 1 or 2 (`+0x30`) | 4 |
| units | free | a building of another clan, any bigger unit selected | 3 |
| units | free | none: valid / not | 1 / 0 |
| units | attack target (1) | anything / none | 16 / 0 |
| units | building (2) | a building of another clan, not a main teleport (`0x80000200`), bridge (`0x80001000`) or ruin (`0x80002000`) | 4 |
| units | building | anything else | 0 |
| units | guard target (3) | anything | 9 |
| units | guard target | none: valid / not | 8 / 0 |

"Another clan" is any clan but the player's: neutral and allied included, as
[27-ownership.md](27-ownership.md#capture--read)'s capture takes them.

**The pick modes** `0x1010c388` are named by the log switch at `0x1005a5cc`
(table `0x1005a828`):

| # | name |
|---:|---|
| 0 | `CState::FREE_MODE` |
| 1 | `CState::SELECT_ATTACK_TARGET_MODE` |
| 2 | `CState::SELECT_BUILDING_MODE` |
| 3 | `CState::SELECT_GUARD_TARGET_MODE` |
| 4 | `CState::SELECT_PLACE_MODE_FB` |
| 5 | `CState::SELECT_WAY_MODE` |
| 6 | `CState::SELECT_PLACE_MODE_FM` |

They are not docs/31's table column "pick mode" (4, 3, 2 there), which the
executor translates ([below](#an-order-row-leaves-a-pick-open--read)).

### A valid place — *read*, and *measured*

`0x10076770` takes the world (x, y) and the selection. It fails when no areal
covers the point (the system areal map's slots 7 and 6, `ArealMap.dll:0x10020370`
and `0x10020310`).

It passes when every selected unit is a flyer: property `0x207`, the chassis
type, is 1 ([34-progression.md](34-progression.md)). Otherwise it passes only
when the areal's first flag word (`+0x20` of its record,
[08-arealmap.md](08-arealmap.md)) is non-zero (`0x10076805`). The areal's
`+4` holds its record's address (`ArealMap.dll:0x10020353`).

*Measured* on `Tut_3`:
- 86 of its 722 areals set that word, 13% of its area.
- Every Mission 03 object on the player's side stands in one: the hero, the
  builder, the transport, and the bunker, factory, warehouse and generator.
- The three enemy warbots on the plateau stand in areals that do not set it.

*Seen*, the recording at 270, 290, 330 and 345 s: the red `WRONG_PLACE` cursor
over the satellite map. The four points it marks, taken to the world at L =
1996.1, lie in areals whose first word is 0.

The word is what makes an areal walkable: the areal map links only areals
that set it, and the walker refuses a goal on one that does not
([24-motion.md](24-motion.md#the-global-path--read-and-measured)).

## The cursor shows a state — *read*, and *measured*

**The cursor's state** is `0x10104148`, 0 to 10. `0x100571a0` sets it:
- leaving state 8 ends the building ghost (`0x10057ec0`, table `0x10057620`);
- entering a state shows that state's cursor (table `0x10057634`).

Eight cursor objects, 0x268 bytes each, are named by their constructors and
filled from `ui/cursor.cfg` (`0x10057660`, then `0x10057860` for each):

| state | cursor object | constructor | name |
|---:|---|---|---|
| 0 | none | | |
| 1 | `0x1010b2d8` | `0x10058a90` | `ARROW` |
| 2 | `0x1010b070` | `0x10058bf0` | `PICK` |
| 3 | `0x1010ae08` | `0x10058d50` | `PLACE` |
| 4 | `0x1010aba0` | `0x10058eb0` | `TARGET` |
| 5 | `0x1010a938` | `0x10059010` | `GUARD` |
| 6 | `0x1010a6d0` | `0x10059170` | `CAPTURE` |
| 7 | `0x1010b2d8` | | `ARROW`, over the band |
| 8 | none: the building ghost is drawn instead ([32-builder.md](32-builder.md)) | | |
| 9 | `0x1010a460` | `0x100592f0` | `WRONG_PLACE` |
| 10 | `0x1010a1f8` | `0x10059450` | `UPGRADE` |

**A cursor object** keeps what `cursor.cfg` gives it:
- the `HARDWARE_CURSOR` handle, from `LoadCursorFromFileA` (`+0xc`);
- `WIDTH` and `HEIGHT` (`+0x250`, `+0x254`);
- four sprites cut from `TEXTURE`, each `WIDTH` wide, stepping right from
  `OFFSET_X`, `OFFSET_Y` on a 256 page (`0x10057b20`);
- `EXTENT_X` and `EXTENT_Y` (`+0x258`, `+0x25c`);
- `PHASE_DELAY` × 0.001 s (`+0x260`), or 0.25 s when the key is absent
  (`0x10057e15`).

**Showing it.** Entering a state places the object at the cursor. If
`IDisplay` slot 12 answers, it calls `SetCursor` with the object's handle; the
software draw below runs only when it does not, so slot 12 is whether the
cursor is the system's (*derived*). The `FORCE_SOFTWARE_CURSOR` setting is read beside it
(`0x100614e8`).

**The software cursor** (`0x10057060`) draws its phase's sprite with its
extent as the hot spot. It moves to the next of the four phases once
`PHASE_DELAY` has passed, wrapping after the fourth.

### Which state — *read*

**Each frame** (`0x10058710`, from the game frame `0x10060c95` and the screens'
draw `0x1008d32f`), while the cursor is shown (`0x1010b5c8`), two things run.

**The chooser** `0x10058740` runs first:
- with no cursor object, state 1;
- in states 7 and 8, the state stays;
- otherwise the pick's kind picks the state (table `0x100587b4`).

**The draw** `0x100585b0` then draws the state's cursor, the band in state 7,
or the ghost in state 8.

| kind | state | cursor |
|---:|---:|---|
| 0, 11, and anything above 12 (13, 16, 17) | 1 | `ARROW` |
| 1, 12 | 3 | `PLACE` |
| 2 | 9 | `WRONG_PLACE` |
| 3 | 4 | `TARGET` |
| 4 | 6 | `CAPTURE` |
| 5, 6, 7 | 2 | `PICK` |
| 8, 9, 10 | 5 | `GUARD` |

So the cursor over an own unit is `PICK`, over an enemy `TARGET`, over a
building a small bot can take `CAPTURE`, over open ground a selection may walk
to `PLACE`, and `WRONG_PLACE` where it may not. `UPGRADE`, state 10, is chosen
by no kind.

### The files — *measured*

`ui/cursor.cfg` holds eight objects. Every one has `TEXTURE` `new_ui1`, 16 × 16
and `PHASE_DELAY` 150:

| object | `HARDWARE_CURSOR` | `OFFSET_X`, `OFFSET_Y` | `EXTENT_X`, `EXTENT_Y` |
|---|---|---|---|
| `ARROW` | `ui/arrow.ani` | 0, 0 | 0, 0 |
| `PICK` | `ui/pick.ani` | 0, 16 | 8, 8 |
| `PLACE` | `ui/place.ani` | 192, 16 | 8, 8 |
| `WRONG_PLACE` | `ui/stop_002.ani` | 64, 32 | 8, 8 |
| `TARGET` | `ui/target_5.ani` | 0, 32 | 8, 8 |
| `CAPTURE` | `ui/capture.ani` | 64, 0 | 8, 8 |
| `GUARD` | `ui/guard.ani` | 128, 0 | 8, 8 |
| `UPGRADE` | `ui/upd_03.ani` | 128, 32 | 8, 8 |

`new_ui1` is `ui/ui.lib`'s member 9, `new_ui1.tex`, a 256 × 256 page. All
32 sprites have ink.

**The eight `.ani` files** are RIFF `ACON` animated cursors. `openparkan.cursors`
reads them. Each frame is a 32 × 32, 4-bit `.cur` (type 2), and every step
lasts 9 jiffies: 9 ÷ 60 s, the 150 ms of `PHASE_DELAY`.

| file | frames | steps | sequence | hot spot | shape (*seen*, drawn from the files) |
|---|---:|---:|---|---|---|
| `Arrow.ani` | 4 | 4 | in order | (1, 1) | a white arrow pointing up-left |
| `Capture.ani` | 4 | 4 | in order | (7, 7) | a red flag |
| `Place.ani` | 4 | 4 | in order | (8, 15) | two green arrows pointing down |
| `Stop_002.ani` | 4 | 4 | in order | (7, 7) | a red circle with a bar |
| `Upd_03.ani` | 4 | 4 | in order | (7, 7) | a green builder's glyph |
| `Guard.ani` | 3 | 4 | 0, 1, 2, 1 | (7, 7) | a green shield, shrinking |
| `Pick.ani` | 3 | 4 | 0, 1, 2, 1 | (7, 7) | green brackets closing |
| `Target_5.ani` | 3 | 4 | 0, 1, 2, 1 | (7, 7) | a red sight, closing |

The software sprites on `new_ui1` draw the same eight shapes at half the size.
The three-frame ones repeat their second frame as the fourth phase.

*Seen*: at 186.5 s, with the builder selected and its mine ordered, the cursor
over open ground is `PLACE`'s green arrows. At 190.5 s it is still `PLACE` over
the bunker's roof, where the table gives an own building `GUARD`
(not established, below).

## A left click in the world — *read*

The world handler `0x1008fe80`:
1. takes the selection;
2. runs the pick;
3. with the cursor in state 8 and kind 13, commits the ghost;
4. otherwise switches on the kind (table `0x10090758`).

**Committing the ghost.** If the site is good (`0x1010b598`), every selected
unit gets:
- `+0x131` set;
- the ghost's 4 × 4 matrix `0x1010b554` copied to its `+0xec`;
- the pick mode set to 0.

A bad site does nothing. What follows, the builder's `ORDER_ROBOT_BUILD` with
target `0x206`, is [32-builder.md](32-builder.md)'s.

**The kinds.** A pick mode written back to 0 closes the pending pick, which
gives the order ([below](#an-order-row-leaves-a-pick-open--read)).

| kind | the click |
|---:|---|
| 1 | **Go**: the place, rounded to whole units, to the dispatcher's case 2 (`0x10078b60` with 2) |
| 3 | **Attack** the unit or building: case 5 (`0x10078820` or `0x100789c0` with 5) |
| 4 | **Capture**. In the building pick mode: each selected unit's `+0x133` set and `+0xe4` the building, and the mode to 0. Otherwise case 6. |
| 7 | **Select** it. A unit: the building selection cleared, the unit selected. If a page is open (`+0x5f0`), the panel turns to the unit's page: builder (`0x1004000`) 3, transport (`0x1002000`) 2, warrior (`0x1008000`) or HQ (`0x1010000`) 1. A building: the unit selection cleared (`0x1007d270`), the building selected. |
| 8 | the place joins each selected unit's point list `+0xc0`, and `+0x131` is set; the mode goes to 0 |
| 9 | in the guard mode: each unit's `+0x132` set and `+0xe8` the object, and the mode to 0. Otherwise **Guard** it, case 7. |
| 10 | in the guard mode: `+0x133` and `+0xe4`, and the mode to 0. Otherwise **Guard** the building, case 7. |
| 12 | the place joins every selected unit's point list `+0xc0`. The mode stays **Route**, so each click adds a point. |
| 16 | a unit: `+0x132` and `+0xe8`; a building: `+0x133` and `+0xe4`; the mode goes to 0 |
| 17 | **Open its page** (`0x10083c20`, then `0x10084d80`). Units: by the clicked unit's Type as for kind 7. A building by its Type: the bunkers `0x80010000`, `0x80020000`, `0x80040000` page 7; the towers `0x80100000`, `0x80200000` page 6; the plant `0x80000010` page 5; the institute `0x80000400` page 4; any other page 8. |
| 0, 2, 5, 6, 11, 13–15 | nothing |

**The dispatcher** `0x10079230` gives every unit of the batch `0x1010c024` its
order, replacing its queue (insert 3; [31-packages.md](31-packages.md)):

| case | order | target | parameter |
|---:|---|---|---|
| 1 | `STAYGROUND` (21) | `0x204` | |
| 2 | `GO` (2), after erasing the unit's point list | `0x202`, the place `0x1010c030`, `0x1010c034` | |
| 3 | `SEARCH` (5) | `0x203` | `0x8017365e` |
| 4 | `SEARCH` (5) | `0x204` | −1 |
| 5 | `0x10078ce0`, the attack | | |
| 6 | `SEARCH` (5) | `0x201`, the first target building's logic id (`+0x34`) | `0x8017365e` |
| 7 | `0x10078df0`, the guard | | |
| 8 | `RELOAD` (8) | `0x204` | |
| 10 | `FOLLOW` (22) | `0x201`, the logic id in `0x1010c030` | 50 |
| 20 | `TRANSPORT` (6) | `0x204` | −1 |
| 30 | `SEARCH` (5) | `0x203` | `0x10001000` |
| 32 | `0x10078f60`, the upgrade | | |

- Case 9 builds a packet and sends nothing, and any other case fails
  (byte map `0x100796c4`).
- After the orders, the batch's last unit answers with its acknowledgement
  voice by size class (`0x1008e840`;
  [31-packages.md](31-packages.md#the-wingman-menu-from-first-person--read-and-measured)).
- `0x10079190` then empties the batch.

**Every selected unit gets the same place** (case 2 reads the one pair). So
"regroup" is not a spread of destinations made here. Whether the go task
spreads them is not established.

## A right click — *read*

`0x1008fb00` does the first that applies:

| when | it |
|---|---|
| the band is up (state 7) | cancels the band, state 1 |
| the pick mode is Route (5) | sets the mode to 0 and each selected unit's `+0x130`: **the route is finished and given** |
| the pick mode is 4 or 6, placing | ends the ghost (`0x10057ec0`), state 1, mode 0, and says 6207 *"Building was cancelled by user"* (`0x1008fe08`) |
| units are selected, view state 2 | clears them (`0x1007d270`), the kind, and the view's unit (`0x100a5660` with 0); a unit page (1–3) goes to page 0 |
| a building is selected | clears it (`0x1007dd30`), the kind, and the view's building (`0x100a5680` with 0); a building page (4–8) goes to page 0; with a building's screen on the mode stack's front (5) it rolls the stack back (`0x10062ff0`) |
| nothing selected, view state 2, a page open | turns the panel to page 0 |
| otherwise, the satellite map open | closes the map (`0x10074100`) |

**Esc cancels a placement too.** Its character case (`0x10070ed1`) and the
mode stack's rollback (`0x10063004`) both call this handler while the ghost is
up.

## An order row leaves a pick open — *read*

An order row's click (`0x1007b510`) plays `BUTTON_CLICK` and hands its command
id to the executor `0x1007b740` (jump table `0x1007bb48`; the rows are
[31-packages.md](31-packages.md#the-commanders-menus--measured-and-read)'s):
- a row with no target calls the dispatcher at once;
- a row with a target writes a **pending pick** into the **first** selected
  unit's record, from `+0xa8`.

**The executor's rows:**

| row | the executor |
|---|---|
| 0 Standby, 2 Search and capture, 3 Seek and destroy, 7 Refit, 8 Transport minerals, 9 Search minerals | dispatcher cases 1, 3, 4, 8, 20, 30 |
| 4 Attack | dispatcher case 5 |
| 1 Route | pending pick 1 |
| 6 Guard | pending pick 4 |
| 5 Capture building | pending pick 5 |
| 10–16 Build | pending pick 3, with the building Type in `+0xb0`: mine `0x80000004`, warehouse `0x80000008`, factory `0x80000010`, outpost `0x80000040`, research centre `0x80000400`, light tower `0x80100000`, heavy tower `0x80200000` |
| 17–23 Upgrade | `0x10078b60` with the Type and case 32 |

**The pending record:**

| field | holds |
|---|---|
| `+0xa8` | the byte that it is pending |
| `+0xac` | its kind, 1 to 5 |
| `+0xb0` | the building Type |
| `+0xb8` | a stage |
| `+0xbc` | the record itself |
| `+0xc0` | the unit's point list |

**The unit's update** runs `0x10079700` while `+0xa8` is set (`0x10075d2b`). Its
kind picks a routine, and each one first sets the pick mode:

| kind | routine | pick mode |
|---:|---|---|
| 1 | `0x10079750` | 5, Route |
| 2 | `0x10079a10` | from `eax`, not traced |
| 3 | `0x10079c40` | 4 or 6 (from `edx`) |
| 4 | `0x10079f40` | 3, guard target |
| 5 | `0x1007a1f0` | 2, building |

It then waits for the clicks above to set the unit's `+0x130`–`+0x133`, and
gives the order.

**Route** (`0x10079750`) is the one read through:
1. **Stage 0** shows the satellite map (`0x100740f0`). It sets the mode to
   Route, clears `+0x130`, and puts the unit's own position first on its point
   list.
2. **Stage 1** waits.
   - If the mode has gone back to 0 without `+0x130`, the pick is dropped. So
     selecting a unit, which zeroes the mode, drops it.
   - When `+0x130` is set, by the right click, it moves to stage 2.
3. **Stage 2** gives a `GO` to every point in turn, `0x202`. The first
   replaces the queue (insert 3) and the rest append (insert 1). The player's
   unit answers with its voice, and the list is emptied.

**So a route goes to the first selected unit only** (*derived*). Clicks add
their points to every selected unit's list (kind 12), but only the pending unit
gives its orders; the others keep their points until a later `GO` erases them.

## Seen in the recording

- **180–186 s.** The Builders page. A click on *Build Mine* raises the red
  ghost, then the green one; the cursor is not drawn while it is up (state 8).
- **186.5 s.** The ordered builder is selected: its marker is in the world, the
  status line reads *building*, and the Build Mine row is gone. The cursor over
  the ground is `PLACE`.
- **262 s.** On the satellite map the factory's mark is outlined in white: the
  building is selected.
- **270–360 s.** The cursor rests low on the satellite map as `WRONG_PLACE`, at
  points in areals whose first flag word is 0 (above).

## For an engine

1. **Input.** Keep the cursor's window position and its layout position (÷ the
   width over 640, the height over 480). Route the left button's down, the
   moves, the up and the right button as above. A click acts on the way down; a
   band acts on the way up.
2. **The selection.**
   - A flag per unit, a single-building selection, and a kind (0, 1, 2).
   - A click replaces, with no modifiers. Say `VOICE_SELECTED` when a unit that
     was not selected is selected.
   - Heroes are never selected. The band never takes a hero, a dead unit or
     another clan's.
3. **The band.**
   - It starts after 0.35 s held in the commander's view, outside Route.
   - Draw it as a rectangle in (25, 180, 25).
   - It needs 10 × 10 window pixels, and replaces the selection. Test each
     unit's projected point, or on the open map its world position.
4. **The pick**, each frame and on each click:
   - **On the map:** (u, v) × L ÷ 256, then the first unit within 40, or else
     the first building within 80, that the player knows.
   - **In the world:** the ground point under the ray, kept 0.001 × L inside
     the map, and the object whose bounding sphere the ray passes (0.7 × r for
     buildings, 1.0 × r for units; not one holding the eye), the centre nearest
     the eye winning and a tie keeping the building.
   - Classify by the kinds table.
5. **Valid places.** An areal must cover the point. Its first flag word must be
   non-zero unless every selected unit is a flyer (chassis type 1).
6. **The cursor.** Kind → state → `cursor.cfg` object, as tabled.
   - Draw the software strip from `new_ui1`: four 16 × 16 phases, a
     `PHASE_DELAY` step, the `EXTENT` as hot spot.
   - Or decode the `.ani` for a system cursor.
   - In state 8 draw no cursor, only the ghost.
7. **Clicks.** Give the orders through one dispatcher, as tabled, replacing
   each selected unit's queue.
8. **Order rows.** A row with a target opens a pick mode on the first selected
   unit.
   - **Route:** points collect on clicks, a right click gives them as a chain
     of `GO`s.
   - **Guard, Capture building and Build:** take the next valid click.
   - **Cancel:** a right click on a placement, or Esc, says string 6207.
9. **The right button** undoes the most specific thing open, in the table's
   order.

## Not established

- ~~**Areal flag word 0.** What it means, beyond deciding where a non-flyer
  selection may be sent~~ — **read**: it marks a walkable areal
  ([24-motion.md](24-motion.md#the-global-path--read-and-measured)). Still
  open: why the recording shows `PLACE`, not `GUARD`,
  over the bunker's roof at 190.5 s. Narrowed by the pick below: a building is
  taken only where the ray passes within **0.7** of its bounding radius, so
  over a roof's outer part the pick finds no object and a valid place gives
  `PLACE` (*inferred*). Whether the recording's cursor stands outside that
  0.7 there, rather than the frame lagging the cursor, was not measured.
- ~~**The object pick.** Which object classes the world's lists 3 and 4 are, and
  the exact order and nearest-hit rule of `0x100361a0` beyond its frustum and
  sphere tests.~~ **Read**: class 3 is the buildings and class 4 the units,
  walked in that order with one shared nearest distance; a building within 0.7
  of its radius and a unit within all of it, not one holding the eye, and the
  centre nearest the eye wins, a tie keeping the building
  ([The object pick](#the-object-pick--read)). Still open there: the walk
  passes over an object with no parent (`IGameObject` slot 3), and whether
  that is what keeps the hero from the pick in command mode was not traced.
- **The band's draw.** Whether `IDisplay` slot 3, which draws the band, fills
  it or outlines it.
- **The pending picks not traced here.** What sets pending kind 2 and so the
  attack-target mode, and the orders kinds 2 to 5 give, read here only in
  outline (their `+0x131`–`+0x133` flags and targets). The Build row's own pick
  is [32-builder.md](32-builder.md)'s: mode 6 for a mine and 4 for any other
  building, and a good click gives `ORDER_ROBOT_BUILD` with target `0x206`.
- **Double clicks.** World3D's message-to-scan converter (`0x10011330`) maps
  `WM_LBUTTONDOWN`/`UP` and the right button, and has no case for
  `WM_LBUTTONDBLCLK`. Whether the window class asks for double clicks was not
  read. Kind 17, a click on the one selected unit, is what opens its page.
- **Spreading a group.** Whether a group sent to one place spreads out, in the
  go task.
- **Telepresence.** Taking over a selected unit from command mode belongs to
  the mode stack's transitions ([40-command-mode.md](40-command-mode.md),
  [39-boarding.md](39-boarding.md)), and is not read here.
