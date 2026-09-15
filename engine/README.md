# engine

A Rust engine for *Parkan: Iron Strategy* that reads the game's own install.
Its first target is Phase One: load Mission 01, *Line of Fire*, put the hero
at its start, and walk, look and shoot in first person.

Milestones **M0** to **M5** are in, each with the stand-ins listed below:

- the workspace;
- the NRes, mission, `Texm`, `Material.lib`, wear, `Land.msh`, object mesh,
  `objects.rlb`, `.dat`, controller (`.ctl`), input table (`.tbl`), control
  point (`.cpt`), damage table (`.ndp`), explosion (`.exp`), effect (`FXID`),
  atmosphere (`sky.ske`), RsLi archive and game font (`gamefont.rlb`), behaviour
  script (`.scr`), formula (`.fml`) and variable table (`varset.var`) readers;
- the golden cross-check;
- a window over Mission 01.

The ground is drawn with its two material layers blended and the file's own
mip levels. Every placed object is drawn from its assembly:

- each part is mounted on its host's socket;
- each node sits at its rest pose;
- level 0 is drawn; a node's fifth slot, its cockpit, only in the hero's own view;
- materials draw in the blend mode their flags byte names, two-sided, each
  batch's UVs over 1024 into its entry's cell of the texture, and a material
  whose track has more than one key plays it: the cell and texture step with
  the keys and the masked colours glide.

**M3.** The window opens in the hero's cockpit on Mission 01:

- `parkan-sim` plays the chassis controller's states on their own clock. The
  live limits and the velocity and pending-turn integrators run once a state
  step. The body moves by its velocity, held inside the state's box, or by the
  animation's root stride. Only an anchor plans, and a state applies only to
  its request code.
- The planner scales the file's transition costs as the loader does, so the
  run cycle keeps running forward.
- The ground contact holds the body: a state without bit `0x4` is only lifted
  onto the ground; one with it falls under gravity 10 until a foot lands. The
  map edge clamps the body inside the world box and pushes it back from the
  last 80 m.
- `hero.tbl` drives it through the game's mouse filter. A strafe key sets the
  angle as it goes down; the hull takes the turn and the turret's yaw channel
  takes it back. A held ramp row eases the keypad cruise toward full.
- The turret's pitch channel tilts the sight. The eye stands at
  `CameraCenter`, looks along `TargetDirect` with a horizontal field of view
  of 1.3 rad, takes `CameraCenter`'s own vector as up, and shakes with the
  body's jolts. The hero's own view draws its cockpit.

On Tut_1 the hero holding W runs at 14 m/s. Not drawn yet: the hero itself
and its animation, when seen from outside.

**M4.** The hero fights.

- Its turret's four guns keep the game's clock: a four-step barrel stroke,
  then the interval.
- The fire and number keys reach the selected guns. The cannon and the laser
  start selected, and their arms unfold from folded. Each mount blends from
  its rest toward the pitch by its arm's progress, and a gun fires only once
  its arm is out.
- A round leaves its muzzle aimed at what the sight meets (a sight that passes
  no triangle), and flies with the shooter's velocity. Its side speed bleeds
  off, and its range runs out.
- Each frame a round's segment is tested against the ground, the map box and
  every live object's level-0 triangles.
- A hit does the round's `.exp` damage to the node struck, or blasts every
  node in reach, less armour. A dead node takes its children with it, and
  node 0 takes the object; a building stays, a shell that can still be shot
  apart.
- On Mission 01 the laser kills a target in two hits of 250.
- Effects play from `effects.rlb`: each emitter inside its window of effect
  time. The turret's flashes hang on the barrel points, timed by the barrel
  channels; rounds carry their tracers, bolts and streams. A strike plays its
  `.exp` by the surface it met, or on a unit by the struck batch's material,
  and a destroyed node plays its own. Sprites, bolts, streams and bursts fade
  as their emitters say, in their materials' blend modes; a bit-8 emitter
  draws over the scene while its effect's tested point is in view.

Not yet: lights, animated textures, shields, and what a dead unit leaves
behind.

**M5.** The sky is the mission's `sky.ske`, interpolated on its clock, which
starts at the file's closing time and plays its sections in turn:

- the dome around the camera takes its apex, rings and horizon colours;
- linear range fog runs from the eye to 700 × slot 6, in the horizon colour
  of the heading, additive materials fogging to black;
- the sun and the moon are up from their start keyframe to their stop, and
  while one is up the sun object's two lights shine: slot 19, lifted by the
  flare gates, and slot 21;
- the lit colour, the scene colour and the material's ambient colour (its
  self-light) and diffuse under both lights, is formed in the files' display
  space, held to 1, and decoded; the texture's alpha is scaled by the ambient
  alpha.

Sound plays each effect's sound emitters from `sounds.lib`, WAV and MS ADPCM
through kira, as their effect time passes their trigger. A HUD showed a
crosshair and the guns, until M9 drew the game's own.

Not yet: lightmaps (no Mission 01 mesh has one) and levels of detail beyond 0.

**M6.** The mission moves on, and the hero picks its targets.

- The player clan's `.scr` runs: `Init` once, `Mission` every 2 s, answering
  its route tests, robot counts and message and objective calls. Routes are
  trigger areas that units report into on their takt. Messages voice once
  through a queue from `voices.lib` and show their text; objectives complete
  with the game's own string and voice, and the last primary one wins the
  mission. The mission's theme loops.
- The hero's target list keeps its fitted radar's contacts, drops a dead or
  distant target and picks the nearest hostile, friend or listed object. Tab,
  E, T and the right button pick as `ui_other.man` binds them, and a neutral
  unit makes itself the target once in sensor range.
- A new target reaches the guided guns only. The plasma rifle and the
  missiles hold their fire without a target, out of range or off the barrel,
  count their lock down, and their rounds' seekers steer onto it. Enter
  captures a neutral unit within 20.
- The view holds the heading the hero runs along (`--sway` for the game's).

On Mission 01 the hero is greeted in route 0, destroying the five targets
completes the first objective and capturing both neutral warbots the second.
Not yet: boarding a captured bot, and the ambient variations.

**M7.** The hero sounds, and meets the world.

- A gun's arm sounds as it is put away, and an effect's loop plays while
  inside its window. Each foot that lands runs its contact's group over the
  ground's condition bytes, which restarts its step effect and sound.
- A building's faces are ground, so the hero walks over a bridge on its deck.
  Every other placed object pushes the hero's swept body sphere off its faces;
  a big tree's leaves let it by.

**M8.** Other units, and what they are told.

- Every other unit is a robot: its machine and turret tick, it is drawn node by
  node, and it is struck where it stands.
- The tilde opens the wingman menu on the friendly warbots on the radar
  (Shift+tilde to pick them by number), a digit gives its row's order, and the
  last one chosen acknowledges.
- A wingman carries its order out. The task the order builds tells its walker
  where to go, the walker cuts that into timed points, and the Wizard follows
  them, writing the machine's velocity and turning its hull; a flyer's points
  keep every axis. Standby holds. Follow me keeps within 20 + 20 of the hero.
  Seek and destroy hunts the nearest hostile warrior, builder or transport and
  roams when there is none; Search and capture looks for a building to take and
  roams; Capture building walks to its building; Refit fails with no dock.
  Between orders a unit engages the nearest hostile within 500, and an attack
  circles 50–100 short of its target.
- A wingman's fire control points its turret at the nearest hostile within 500,
  or at what its attack is on. Guns fitted as parts of their own are armed, and
  each fires one shot when its AI timer runs out and its score clears the bar,
  or freely while the unit searches or attacks. On Mission 01 both wingmen,
  sent to seek and destroy, fly to `tut1_e1` and destroy it.
- A unit or building takes damage node by node, as read. A node at or below
  half its life (on a part with a damaged model) draws that damaged block, with
  its explosion; at nothing a part is knocked off and falls, drawn, until it
  meets the ground or three seconds pass, then explodes and goes, and its
  children go with it. When the base
  dies every part still standing goes with its explosion, and the unit is
  deleted its controller's `+92` ms later. A hidden node is not drawn, struck,
  collided with or stood on.
- A lake's bed kills. The ground contact reads the rate of what it touches,
  or of a bed whose water lies less than r below the body sphere's centre, and
  every 250 ms or so a unit's nodes each lose the same share of their life.
  On Mission 01's lake the hero dies 0.6 s in; it stops where it died, and the
  mission fails.
- Once a mission is won or lost, play goes on under the outcome panel in the
  HUD's place: a 60% black box with "MISSION COMPLETE !" in the interface's
  menu font, green, over "Press 'Esc' to continue" in its game font, laid out
  on 640 × 480 as read and as a recording of Mission 01's win shows it; a loss
  is red, and adds R to restart and L to load. Esc then leaves, and R restarts
  the mission.

**M9.** The cockpit's HUD, as `iron3d.dll` draws it on its 640 × 480 layout from
the interface's pages and `ui/compaund.cfg` and `ui/hq.cfg`'s pieces.

- The weapons list at the top right: a row a gun, its key, a lamp by its state
  (black unselected; green, yellow or red by what it reports), its rounds or
  INF, and its name over a bar its charge fills, red, olive or green. A guided
  gun with no target, out of range or off the barrel reads OUT OF RANGE.
- The message box at the top: the newest line, headed by whom it is from, six
  lines at most with "Press F1 to see more", for 20 s; F2 hides it.
- The radar: the target list's contacts in their clan's colours (a flyer a
  cross, a building a bigger square, the target outlined), turning with the
  camera, the view's wedge, north and south, the sweep ring and its ping, the
  range; the altitude over the map's water and the speed in km/h beside it; and
  under it the indicators: repair (G), infrared (N), camouflage (H) and the
  auto-driver's level (Y).
- The target panel at the bottom left and the hero's at the bottom right: the
  unit seen through its panel's camera, each node green whole and red
  destroyed; six shield sectors for a unit with a fight shield and a deflector;
  its life arc and its battery arc; its name ("TFW-2 Warrior", "Human") and,
  for a wingman, its order; the target's distance, and a square about it in
  the world, easing in from the screen's middle.
- The reticle at the middle.
- The HUD's art, text and outcome panel blend in display space, as the game's
  16-bit surfaces did: a weapon bar reads the recording's (74, 146, 92).

**M10.** The mission opens as the game's does, and the rest of its screens.

- A campaign mission's briefing plays first: the camera flies `briefing.cfg`'s
  waypoints (linear, the cubic Hermite spline with its read tangents, jump
  cuts at black and level orbits), 1.04 rad across and never rolling, while
  each stop's voice plays at once. Over the fade, two black bars leave 75 to
  405 of the 640 × 480 screen, with the mission's title from `descr` and the
  wrapped subtitle. Every object but the hero is paused and the clan scripts
  wait; the sky's clock runs. When the path ends, or Esc skips it, the theme
  starts and the cockpit takes over. On Mission 01 its subtitles change within
  0.35 s of a recording's, and its shots frame what the recording's frame.
- The objectives screen opens as the cockpit first shows and closes 7 s after:
  "Primary objectives" and each objective with its state, centred in the menu
  font over a 60% dim, in place of the HUD. F12 opens and closes it, Esc closes
  it, and the wingman menu waits while it is up.
- M opens the satellite map at the top right in the weapons list's place: the
  mission's minimap tinted `#37ff37` at `MAP_ALPHA`, its frame, the compass and
  the hero's mark; ] and [ make it more or less opaque by 12, with its label for
  a second. The message box's frame now places its pieces as read.
- The parts that move by themselves: each generic device and radar steps its
  channels as `Control.dll`'s item does, so the T-2's rotors turn 7.3 times a
  second, every robot turret's dish once in two seconds, and the M-2f's wings
  and engines swing out above half its top speed.
- Water reflects, the `REFLECTION_SHIFTED` way the install's `Iron_3D.ini`
  picks: each frame the dome, the ground and the objects above the water are
  drawn from the eye mirrored in the water plane into a 256 texture covering
  the water's box, clipped half a unit below the water. A water face shows it
  through the drifting environment bump map, times its lit colour; a lake's
  bed is not drawn from above.

Not yet: the view from under the water, which draws only the beds. The lake
comes out bluer than the recording's, whose water is brighter than the sky it
reflects (not established).

**M11.** Mission 02, *The Constructor*: buildings taken and entered, a factory
that builds what the warbot designer draws, and a warbot the hero boards.

- The mission's script is answered: function 52 gives a building's owner, so
  its captures complete only when made. Its briefing's subtitles change within
  0.35 s of a recording's.
- A building's controller gives its doors and its control pod. A unit standing
  on the building is its child: a door opens for a child near it, is open when
  its item stops, lets units through its faces, and closes 5 s after opening
  once free. A building's faces push the units on it too. The pod opens for a
  child in its zone and, when it has opened with the child still there, fires:
  a building of another clan changes owner with "Building is captured" and its
  voice, and for the player's hero a plant opens the factory screen.
- The landscape is cut away inside a building's inner ground-plan ring, from the
  ground queries and the draw; its black doorway and portal quads let a mover
  through, its floors (triangle flag 2) do not push a walker, a push down on a
  unit standing on it is taken whole, and the slope brake is left out on its
  faces, so the hero walks in by the Large Factory's west side door and down its
  ramps and stairs to the pod. Lightmaps light a
  building's lit batches, and each building runs its load group: the Large
  Factory's lamps, screens and chimney smoke, Mission 01's bridge lights.
- The factory screen, laid out as docs/36 reads it, replaces the HUD in view
  mode 5: the Ore and Energy rows, the header and exit, the project box with its
  icons, free minds and turning preview, and the production row. Build and
  batch start `M_Task_Construct`'s budgets (a free bot costs no ore, 1 power and
  its size-table time), and the finished bot appears at the hall way's creation
  vertex, numbered by its clan (LFW-2), escapes, joins its clan's list and
  announces itself.
- The warbot designer, as docs/37 and docs/38 read it: the source and destination
  panels with their tabs, rows, turning previews and part boxes; the project
  view with its prompt, callout and unit box (mass and spare payload, top speed,
  defence, offence, sensor range, red when over); fitting by double click with
  every slot filled from its `_df` part; accept hands the design to the factory.
  The catalogue is the player clan's research tree.
- Enter boards a large bot of the player's clan within 20: the hero leaves the
  world, the bot's own table drives it (R and F climb and sink a flyer), its
  cockpit and camera make the view, and the HUD is the bot's. Esc gets out at
  the first place about it over land, low enough for a flyer, or says "Risk
  area! Landing impossible."

On Mission 02 the factory's pod captures it, the design is built in 60 s, the
hero flies the warbot to the island, gets out and takes the Outpost from its
pod, and the mission is won. Not yet: the front door's hall does not lead to the
pod (the recording's hero takes the west side door); the chimney smoke is orange
where the recording's is black; the designer's save and load;
the own panel's unit while aboard.

**M12.** Mission 03, *The Field Base*.

- A pod's zone is measured as read: a child's bounding-sphere centre within 0.8 of
  the pod node's sphere across the ground, and between the heights of the pod
  node's parent's level-0 box. Every pod of Mission 03 fires for a hero on its
  floor, the Small Generator's too.
- A hit naming a building's node opens the last door filed on that node, whoever
  fired; an area hit carries the node to every building it reaches. Nothing holds
  the door, so it shuts 5 s after opening unless a unit stands near it. A laser
  round on the Small Bunker's door opens it in 3.6 s.
- `mission.cfg`'s `prebuild` designs, from `units\units\prebld\`, go into every
  factory's recent projects at the start, rated, named and priced from the player
  clan's research tree, the last named first: Mission 03's Large Factory offers
  *SWW-X Warrior* and *SSW-X Warrior*.
- Taking a bunker from its pod opens command mode (mode 4): the hero is let go where
  it stands, the bunker selected, and the camera placed over the bunker facing north,
  32.7° down, with a field of 1.04 rad. The arrows move it at up to 125 m/s after a
  half-second ramp, PageUp and PageDown climb and sink at half that, and the cursor
  within 6 of a screen edge turns or tilts it at 1.5 rad/s; it coasts to a stop, its
  height is held 36 to 236 over what is below, and each of x and y within 200 of the
  bunker. Z zooms. The world goes on meanwhile. Esc closes an open map, then turns the
  page back, then leaves: the hero is taken back in the pod.
- The commander panel draws in the cockpit HUD's place, as docs/41 reads it: the Ore
  and Energy rows, the icon column with its lock, its buttons enabled by what the clan
  holds, and the page a button opens. A unit page (battle units, transports, builders)
  shows the selected unit's box (icons, name and status, its rated lines, the drive and
  Explode! buttons), a row a unit with its life, and the order menu the selection is
  offered; a row click selects that unit alone. Standby, Search and capture, Seek and
  destroy, Refit and Transport minerals give their orders at once. The factory page
  draws the selected factory's panel over its rows, and the other building pages a row
  a building, Strategic control moving command mode to a bunker. On Mission 03 the
  builder is offered Build Mine alone of the Build rows. The commander's satellite map
  stands under its title bar and marks the camera in yellow; the message box moves to
  the bottom right.

- The cursor picks as docs/42 reads it, in the world through the command camera and on the
  open satellite map: a click selects one of the player's units or buildings (turning an
  open page to its kind), sends the selection to open ground (a Go), attacks another clan's
  unit, captures or guards a building, and a click on the one selected unit opens its page. A
  drag held 0.35 s draws a band that selects the player's units inside it. The software
  cursor shows the pick: `PICK`, `PLACE`, `TARGET`, `GUARD`, `CAPTURE`, `WRONG_PLACE`. Route
  collects places on the map until the right button gives them as a chain of Go orders; Guard
  takes the next click. The right button undoes the most specific thing open.
- A Build row raises the building's full-size ghost under the cursor, drawn flat, green on a
  good site and red on a bad one (a mine needs a found lode within 20), turned 0.05 rad a
  press by `,` and `.`; a click on a good site orders the builder, and the right button or
  Esc puts it away with *"Building was cancelled by user"*.

- The selected units and the unit under the cursor are bracketed in the world in the marking
  rule's colour, the player's named in green, with their class icon and bars. Each lode's
  plume, `env_mineral`, shows on the ground under it until a building stands within 80.

Not yet: the builder's building, the economy, telepresence, Explode!, tooltips, and the chat
and game menu buttons. The ghost draws over the HUD.

This directory also holds what the rest will follow:

- **`docs/`** is the source of truth. Every behaviour the engine implements is
  in a doc there, labelled *read* or *measured*, and re-derived by
  `uv run openparkan verify`.
- **`design/`** keeps the eight Phase R research notes, R1 to R8. They turn
  the docs into implementable steps. Where a note and `docs/` disagree,
  `docs/` wins, and each note says at its top what has been superseded.
- **The table below** lists the places where the game's behaviour is not
  established and the engine has to choose one. It is the only list of
  guesses.

The plan is Rust + wgpu (winit, glam; kira for sound). It covers one
workspace of `parkan-formats`, `parkan-sim`, `parkan-world`, `parkan-render`
and `parkan`, in milestones M0 to M5.

## Running it

```
cd engine
cargo run --release -p parkan                                  # Mission 01, in the hero's cockpit
cargo run --release -p parkan -- --fly                         # a debug camera instead
cargo run --release -p parkan -- --headless --ticks 180 --hold SCAN_W   # play 3 s holding W, no window
cargo run --release -p parkan -- --screenshot run.png --ticks 120 --hold SCAN_W --mouse 6,-8
cargo run --release -p parkan -- --hold SCAN_W,SCAN_LMOUSE --mouse 4,1 --trace   # the window, hands off
cargo run --release -p parkan -- --screenshot m1.png           # one frame to a PNG, no window
cargo run --release -p parkan -- --screenshot map.png --top-down --size 768x768
cargo run --release -p parkan -- --screenshot tank.png --look 705,885,24,734,906,10   # from X,Y,Z at TX,TY,TZ
cargo run --release -p parkan -- --mission MISSIONS/Single.01  # another mission
```

`--game DIR` or `PARKAN_DIR` points at the install when it is not beside this
repository.

In the cockpit the hero's own `hero.tbl` drives it. W and S walk, A and D
strafe, the mouse turns the hull and tilts the turret, and Shift with the
mouse looks around while the guns hold their aim. Letting Shift go, or leaving
the window, which lets every key up as the game does, centres the view again.
Cmd frees the cursor and lets every key up too, so a macOS shortcut such as
Cmd-Shift-4, which takes the keyboard without the window losing focus, leaves
nothing held.
1 to 4 select guns and the left button fires. The game's
own chords from `ui_other.man` pick targets: Tab the next listed, E the next
or nearest enemy, T a friend, the right button what the view points at; Enter
captures a neutral unit within 20 m. A click grabs the mouse; Escape lets it
go, and quits once it is free. `--ticks N`, `--hold` (scan names) and `--mouse DX,DY` (counts a
tick) play the hero at 60 ticks a second before a screenshot, or with
`--headless` print where it got to. In the window `--hold` keeps its keys down,
`--mouse` adds its counts every tick and `--trace` prints where the hero is
every second. The view holds the heading the hero moves along; `--sway` lets it
swing with the gait as the game's does. A captured bot stands by until it is
ordered; `--capture-idle` leaves it with no order, as the game's capture does. F2
hides the message box or shows it again; G, H and N switch the repair system,
camouflage and infrared, and Y steps the auto-driver. The HUD keeps its layout's
shape on a wide window, its panels on the window's corners; `--stretch-hud`
stretches it as the game's does. `--face NAME,DISTANCE` stands the hero that far
from the mission object whose path ends in NAME, facing it, before `--ticks`
play. With `--fly`, W/A/S/D and Q/E fly, holding
the right mouse button turns and Shift flies faster.

A campaign mission opens on its briefing, and Esc skips it; `--skip-briefing`
starts in the cockpit, and `--screenshot b.png --briefing-at 36.8` draws the
briefing that many seconds in. The objectives screen then shows for 7 s; F12
opens and closes it. M opens the satellite map, and ] and [ change its
opacity. A screenshot draws the cockpit without either, unless `--objectives`
or `--map` is given.

On a factory's screen the warbot constructor button opens the warbot designer:
a double click on a source row (or its preview) fits the part, one on a
destination row takes a chassis, turret or gun out, the tabs page through the
slots, accept hands the design to the factory, and exit or Esc closes it.
`--designer` draws a screenshot with the designer open on the first factory,
and `--design PART,…` fits those parts to it in turn; `accept` among them
clicks accept, leaving the factory screen with the project.

Standing on a building's control pod captures it; `--pod NAME` starts the hero
on the pod of the building whose path ends in NAME (`--pod lplant01.dat` on
Mission 02), and `--at X,Y,YAW` anywhere on the ground. Enter boards a large bot
of the player's clan the hero has targeted within 20 m, and Esc gets out;
`--drive PATH` makes a unit of that design beside the hero and boards it
(`--drive 'UNITS\bld_unit_-2147483647.dat'`).

## Checks

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                 # synthetic bytes only
cargo test --workspace -- --ignored    # against the install
cargo build --release -p parkan-world && uv run openparkan golden
```

`openparkan golden` dumps every file the engine reads through both the Python
readers and `parkan-dump`, and compares the two within 1e-5. At M0 that is
every archive in the install, Mission 01's `data.tma`, `Material.lib`, Tut_1's
`Land.msh`, the textures its ground names, Mission 01's assembly (every
object's parts and their poses) and the 19 meshes they use. From M3 it adds
every controller and control point list, archive by archive, and the three
input tables, and from M6 the key bindings; from M4 every damage table, explosion and effect; from M5 every
mission's atmosphere, both RsLi archives and the game font; from M6 every
`.cfg`, the text and interface string tables, each mission's objectives,
messages and ambient sound resolved, and every behaviour script, its formulas
and `varset.var`: 448 dumps.

## Stand-ins

Each stand-in is marked in the code as `// STAND-IN: docs/NN#section` and has
a row here. A row leaves this table when research closes it.

| milestone | what is unknown | stand-in | see |
|---|---|---|---|
| M1 | Whether the water surface is blended over the frame beneath it | opaque: the terrain draws every material opaque, whatever its blend | [03](../docs/03-terrain.md#not-established) |
| M2 | Whether a blended material writes depth, and the alpha test's reference value | blended groups draw after opaque ones without writing depth; nothing is discarded (reference 0) | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M1 | Whether `ForceSWFog` is read outside `Terrain.dll`, which asks Direct3D for linear range vertex fog and never reads it | per-pixel linear range fog on the distance to the eye, from 700 × slot 5 to 700 × slot 6 | [10](../docs/10-sky.md#not-resolved) |
| M1 | How the 34142-radius dome escapes the far plane and a fog ending by 700, and what lies below its rim | draw the dome first at the camera, depth-tested without writing depth under a projection with no far plane, unfogged but for its rim; clear the frame to the fog colour | [10](../docs/10-sky.md#the-dome) |
| M1 | Which camera axis the fog's heading angle measures: the compass heading, 0 at +y towards +x, of the camera matrix's first column | the view direction's heading, 0 along +y, turning towards +x, like the dome's segments | [10](../docs/10-sky.md#not-resolved) |
| M1 | Where the sun object's two directional lights point | both lights shine from the fixed place of the body that is up; none while no body is up | [10](../docs/10-sky.md#not-resolved) |
| M1 | The sky's textures: stars, clouds, the sun and moon sprites, the lens flare | not drawn | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M1 | The files' colours in a renderer that decodes textures to linear | sky, fog and dome colours and texture tints decoded from sRGB to linear; the lit colour (scene colour, material ambient and diffuse, both lights) formed from the files' values, held to 1, then decoded, so blends match the game's display-space ones | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M3 | What the draw layers 10 and 9 a fifth slot is filed under do (`Terrain.dll:0x1004553b`), and `CShade` slot 15 | the fifth slots draw with the scene, depth-tested, lit and fogged like any model | [07](../docs/07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws) |
| M3 | When the ground contact runs and with what dt, the frames its contact points are placed at, the second sphere's radius r₂, and what lifts a sphere with no face under it | after every state step, with the step as dt; contacts on the step's last frames; r₂ = r; no lift | [24](../docs/24-motion.md#holding-the-body-on-the-ground--read-and-measured) |
| M3 | A state's use count `+0x94` | unlimited | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | The state a machine starts in | state 0 | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | The request code a controller holds before any is sent | none (−1): a state waiting for a code of its own does not apply until one is sent; no Mission 01 state has one | [32](../docs/32-builder.md#the-construction-sphere--read-and-measured) |
| M3 | The game's random source for a jittering step | xorshift | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | What vector a state with righting bits `0x30` stands the hull toward (control `+0x348`, no writer found) | none yet: the hull neither leans nor rights; to come, the ground contact's face normal, and world up for bits `0xC0` as read | [24](../docs/24-motion.md#the-hull-leans-and-rights-itself--read-and-measured) |
| M3 | Which way across a slope the mode-2 brake acts | uphill, against the averaged ground normal of the last landing | [24](../docs/24-motion.md#ground-and-slope--read) |
| M3 | How often `World3D.dll`'s input update runs, which paces the keypad cruise ramp | once a rendered frame, and once a 60 Hz tick where nothing is rendered | [24](../docs/24-motion.md#not-established) |
| M3 | What the walk, strafe and weapon handlers do when an active row runs again each input update while held | only ramp rows run again; any other row runs once, as its key goes down or comes up | [14](../docs/14-controls.md#a-row-that-stays-down--read) |
| M3 | A chord with no row of its own, such as Shift+W | the plain row | [14](../docs/14-controls.md#the-table) |
| M3 | How the camera builds its look-only frame when its up is parallel to the look (`0x10023769`) | any frame about the look; no shipped camera's pitch reaches it | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M4 | Whether a target the hero's AI set before the player took over survives | none: nothing sets it while the player drives, so the plasma bolt and the missile fly straight | [29](../docs/29-weapons.md#not-established) |
| M4 | Whether the landscape is one of the objects the sight ray (IWorld slot 7) walks; it skips no batch or triangle | the ground (less the water surface) and every live object's level-0 mesh, passing no triangle, as far as the map's diagonal and 200 m more | [29](../docs/29-weapons.md#not-established) |
| M4 | What a falling round's mount solves for with no target (the aim triple, `0x10027e07`), which way its lift turns on a hung turret, and whether a player's turret is in `CIS_MANUALCONTROL` | the stored aim triple as the vector, the lift signed by `TurretCenter`'s z, and the solve runs (no hero round falls) | [29](../docs/29-weapons.md#not-established) |
| M4 | How a gun's capacitor refills | full again every tick (the power tick is not modelled) | [23](../docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured) |
| M4 | Whether a round's ground test strikes the water surface | it passes through: the ground index holds no face with `Land.msh` surface bit `0x02`, so a round meets the bed | [26](../docs/26-damage.md#the-hit-test--read-and-measured) |
| M4 | The point-in-triangle test of the hit test (`0x10011090`), and the landscape's own cell size | an edge test on the triangle's winding; the ground index's 16 m cells | [26](../docs/26-damage.md#the-hit-test--read-and-measured) |
| M4 | Which node flag makes a node vital | the mesh node's `0x200` | [26](../docs/26-damage.md#hit-points--read-and-measured) |
| M4 | Whether vegetation and rock take damage | they stop rounds and take none | [04](../docs/04-missions.md#the-scale) |
| M4 | Shields: bubble contacts and sectors | not modelled: no bubble stops a round, a blast skips its shield step and kind 4 does nothing; Mission 01's `tut1_e1`, `tut1_mf1` and `helic` carry fight shields and deflectors | [26](../docs/26-damage.md#shields-a-generator-a-deflector-six-sectors--read-and-measured) |
| M4 | Poses of other units for the hit test | their rest poses: other units' animation is not played | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M4 | The rest of the emitter floats: what the fade value scales, what a particle's exponent-shaped triples are, a bolt's widths | sprites (3, 4, 9) move +40→+52 and grow +100→+112 straight by progress through the window, per-axis powers left out; a burst (7, 10) flies between velocities +44 and +56, spread +68, its age progress over +28, sized +92→+104; a stream (8) particle sits at +88→+100 and grows +136→+148 by its age in seconds; a bolt's sprites are +24 wide, its fade straight across the window; a fade value is the quad's alpha | [11](../docs/11-effects.md#not-resolved) |
| M4 | An effect's jitter (flag 1), the owner values of time modes 5–15, and a phase's animated texture frames | no jitter; modes 5–15 all read the owner's speed over its top speed, set on rounds; frame 0 of every texture | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M4 | How a sprite whose material says opaque blends | alpha-blended, so its fade shows | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M4 | How the shade lights with a type-1 light's range and attenuation; type 1 is a light in the owner's `CLightManager` | none yet; to come, Direct3D's fixed-function falloff, 1 / (a0 + a1·d + a2·d²) inside the range | [11](../docs/11-effects.md#not-resolved) |
| M4 | Which draw pass draws header-flag-0x800 effects; who sets the manager's target point a bolt starts from | 0x800 effects draw with the rest; a bolt starts where its effect started | [11](../docs/11-effects.md#not-resolved) |
| M4 | When a stream emits its first particle | on its first update inside its window | [11](../docs/11-effects.md#bolts-streams-and-fades--read-and-measured) |
| M4 | How often an effect instance tests its point's view, and what the ray through the world meets | every frame, against what a round meets (the ground less its water surface, and every live object's level-0 mesh) | [11](../docs/11-effects.md#bit-8-and-the-tested-point--read-and-measured) |
| M4 | What a building (a `CBuilding` aggregating its agent) answers for a strike's material, and a node's wear base | a strike on a building plays slot 0; the batch's material byte alone indexes the wear | [11](../docs/11-effects.md#what-an-explosion-plays--read-and-measured) |
| M4 | The effect manager's random generator | any uniform generator | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M5 | How a sound falls off between its near and far distances, and how it is panned | linear in distance; panned by its direction against the eye's right | [11](../docs/11-effects.md#emitter-types--read-and-measured) |
| M5 | How the game turns a string's characters into the font's glyph indices | ASCII as its own index; Cyrillic by code page 866, where the font draws it (А–Я at 0x80, а–п at 0xA0, р–я at 0xE0); anything else draws `?` | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How tall a glyph is drawn and how far apart lines are: a record has no bottom edge | the atlas's row pitch, 18 pixels, for both | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How far the pen moves after a glyph | its advance: a recording of Mission 01's win measures it on the interface's menu font (each glyph of "MISSION COMPLETE !" starts its advance after the last, the space's 6 included), and the game font is taken to space the same way | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How the game draws its font: its 8-bit blend table and the text's colour | the atlas, sampled nearest, keyed on black and multiplied by the run's colour in display space, over everything after the HUD (a recording shows the interface font's grey shadow dark) | [12](../docs/12-rsli.md#what-is-inside) |
| M6 | How the loader turns a `varset.var` default into a value | an integer type reads hex after `0x`, a decimal, or a float truncated; a float reads its decimal; anything else is 0 (every shipped default reads either way) | [15](../docs/15-behaviour.md#the-vocabulary-varsetvar) |
| M6 | The formula parser and evaluator, beyond the operator table's priorities | parse by those priorities, left to right among equals, in doubles; `!` is 1 for 0, `&` and `\|` are logical, `N` holds to 0..1, `S` is the sign, `B` is 1 for non-zero, `A` the absolute value; an unknown name is refused (the shipped formulas use numbers, variables, `+ - *` and brackets) | [15](../docs/15-behaviour.md#a-statement) |
| M12 | Whether the character handler sees Esc before its binding leaves command mode | an open satellite map closes first, then a page other than 0 turns to 0, then Esc leaves | [40](../docs/40-command-mode.md#not-established) |
| M12 | A building's property `0x20c`, its construction sphere running | no building builds itself: every building is complete for the column and the pages | [41](../docs/41-commander.md#what-enables-a-button-and-what-lights-it) |
| M12 | What `0x10034230` accepts for an Upgrade row | no Upgrade row is offered | [41](../docs/41-commander.md#not-established) |
| M12 | A builder's beam's life, which the Build rows need intact | a live builder can build | [32](../docs/32-builder.md#building-a-building--read) |
| M12 | A unit's property `0x207` and a building's record `+0x30`, which pick and tint the panel's icons; the width of the piece a row's icons stand in | a unit's second icon is the cell for 1; a building's icon red (2), as the recording shows; each icon piece 19.5 wide on a unit row and 20 on a building row | [41](../docs/41-commander.md#not-established) |
| M12 | The chat overlay and the game menu's screen (mode 7) | not built: their buttons are taken and do nothing | [41](../docs/41-commander.md#what-a-click-on-the-column-does) |
| M12 | The routine that names a building | strings 6031–6098 by Type, by the size letter of its root record (`fr_l_` small, `fr_m_` medium, `fr_b_` large) and a bunker's by its Type | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
| M12 | The commander's satellite map's title bar and exit icon, beyond their place | a page header's pieces 20 tall at (374, 43), the title 5074 centred | [35](../docs/35-hud.md#not-established-4) |
| M12 | An areal's first flag word, which decides where a walker may be sent | the engine keeps no areals: a place is valid where there is ground above any water, and always for flyers | [42](../docs/42-selection.md#a-valid-place--read-and-measured) |
| M12 | Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule | a unit within 0.7 of its radius and a building within all of it, a sphere holding the eye passed over; the nearest centre along the ray wins | [42](../docs/42-selection.md#not-established) |
| M12 | Whether the display's slot 12 answers, so the system's cursor is used | the software cursor's four phases from `new_ui1` are drawn and the system's hidden | [42](../docs/42-selection.md#the-cursor-shows-a-state--read-and-measured) |
| M12 | `IsPlacementValid`'s path, sphere, overlap, areal and slope tests | not modelled: a site is good where a selection may be sent, with a found lode within 20 for a mine | [32](../docs/32-builder.md#placing-a-building--read-measured-and-seen) |
| M12 | The build task (order 7) | not modelled: the order is given and the builder, with no task for it, stops | [32](../docs/32-builder.md#building-a-building-tick-by-tick--read-and-seen) |
| M12 | A marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand | the gap is the unit's projected radius held to 4–44; the name over the left bracket, the class icon right of the right one, a blue box under the left one holding the life bar over a full battery bar | [25](../docs/25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured) |
| M6 | What a node naming a variable, operand, formula or handler that does not exist reads | the node does nothing; a switch to no handler ends the run; a run stops after a million nodes | [15](../docs/15-behaviour.md#how-a-handler-runs) |
| M12 | Function 15's answer: whether the unit takes the order (1) or refuses it (0) | an id some unit or building answers gives 1, and the unit is handed the order after the handler's run | [34](../docs/34-progression.md#what-the-scripts-ask--read-and-measured-1) |
| M12 | A building's contour (property `0x203`), whose vertices a patrol of the building walks | eight points on the building's sphere, pushed out by 30 | [31](../docs/31-packages.md#the-patrol-tick-by-tick--read) |
| M12 | Which areals a walker's patrol points must lie on | none: a walker's point needs no usable areal, as a flyer's does not | [31](../docs/31-packages.md#where-a-search-looks--read-and-measured) |
| M12 | An animal's migration over its clan's pastures, its default order | not modelled: an animal stands and asks its fire control for nothing | [31](../docs/31-packages.md#migrate-an-animals-pasture--read-and-measured) |
| M12 | Which of the fight module's bars a building's guns clear | the walker's, 0.85 | [29](../docs/29-weapons.md#how-the-ai-fires--read) |

### Read since the stand-in was written

Research has closed these, and the code still carries the stand-in. The next
engine pass replaces each with what was read and removes its row.

| milestone | the code's stand-in | what is read | see |
|---|---|---|---|
| M3 | a state's contacts are parsed, but their `0x100`/`0x200` conditions are taken as met (node life is not modelled) | `0x100` makes a state need its contact point's node intact and `0x200` destroyed, a walker's limping states | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | the ground face is found fresh each step, by an up and a down pass over its cell | the walk starts in the held face and crosses at most 24 faces toward the centre, stopping on one too steep | [24](../docs/24-motion.md#finding-the-ground--read) |
| M3 | D is the largest span of the velocity box's switched-on axes | the largest difference between an axis's absolute max and absolute min, over all three velocity axes; D = 0 leaves the weight at 1 (no shipped state's weight changes) | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | the invert constants (−1, +1) with an integrator that does not negate, and free look's yaw negated to undo the mirrored X | the game's mouse X invert is +1 and its integrator negates; on screen the two agree: mouse right turns right, mouse down lowers the sight, and Shift free look's vertical runs opposite | [14](../docs/14-controls.md#from-a-row-to-a-command--read-and-measured) |
| M3 | no spare payload is computed: r = 1 | the chassis is part id 0's nodes, the root object's (the hero's r is about 1 anyway) | [24](../docs/24-motion.md#load--read-and-measured) |
| M3 | only the turn about z is applied | triple 6 is the most the body leans on each axis, from the sources a state's `+0x08` picks; triple 5 is the share of the tilt taken back each step, toward world up (bits `0xC0`) or a vector (`0x30`); no hero state leans | [24](../docs/24-motion.md#the-hull-leans-and-rights-itself--read-and-measured) |
| M4 | every effect instance updates on every tick | the manager updates an instance once 100 ms have passed since its last update | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M6 | The clock unit of the object takt that times a unit's route reports, its rand8, and when a unit's first takt runs | game milliseconds; the engine's own generator; one timer after the unit joins | [34](../docs/34-progression.md#who-stands-in-a-route--read) |
| M6 | A unit also reports when its navigation areal changes | the engine keeps no navigation areals: only a move of more than 5 sends a report | [34](../docs/34-progression.md#who-stands-in-a-route--read) |
| M6 | The `Mission` handler is timed by `timeGetTime`, real time | game time | [34](../docs/34-progression.md#when-the-mission-handler-runs--read) |
| M6 | How a destroyed unit leaves function 31's list | it leaves its clan's count and every route's list | [34](../docs/34-progression.md#function-31-how-many-robots-a-clan-has--read) |
| M6 | What `OBJECTIVE_FAILED` does past fetching string 5041, and what `OBJECTIVE_PROGRESS` shows | the failure shows the string and changes no state; progress shows nothing | [34](../docs/34-progression.md#objectives-and-the-end-of-a-mission--read-and-measured) |
| M6 | The order the resource manager looks a sound's name up in | the first descriptor that binds the name wins, the mission's own before `ui/game_resources.cfg`; every training message resolves to the member its briefing or tutorial descriptor names | [20](../docs/20-resources.md#what-this-does-not-say) |
| M6 | The three signatures a radar weighs against its sensitivities | every live object within the radar's range is detected | [25](../docs/25-sensors.md#a-scan-is-a-sphere-a-falloff-and-three-tests--read) |
| M6 | Whether a guided round's velocity turns with it | it is kept in the round's frame, as a machine's is, and turns with it | [29](../docs/29-weapons.md#guided-rounds-differ-in-how-hard-they-steer--read-and-measured) |
| M6 | What the script functions other than 15, 19, 30, 31, 32, 34 and 52 do in play | a script's other calls do nothing and answer 0; Missions 01 to 03's scripts call none | [15](../docs/15-behaviour.md#what-the-functions-do) |
| M6 | String 6223's key | a repeated message says string 6170 alone | [34](../docs/34-progression.md#not-established) |
| M6 | Whether scenery is among a radar's contacts | a target needs a unit record: trees and rocks are never listed | [25](../docs/25-sensors.md#the-players-target--read-and-measured) |
| M6 | The unit record's `+0x98` and `+0x94`: where the right button's ray starts, and the margin its pick keeps from the unit | both 0 | [25](../docs/25-sensors.md#the-players-target--read-and-measured) |
| M6 | Boarding a captured bot is read, but the engine drives only the hero | the captured unit joins the player's clan and stays where it stands; Enter on the player's own unit does nothing | [27](../docs/27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured) |
| M6 | Whether a type-5 descriptor loops its sound, and when the ambient variations play | the theme loops from the mission's load; the variations are not played | [34](../docs/34-progression.md#ambient-sound--read-in-part) |
| M7 | Which pose the live contact record's height comes from when the loader decides which states plant a foot | the rest pose | [13](../docs/13-control.md#a-footstep-end-to-end--read-and-measured) |
| M7 | A contact's node life, which decides whether a foot can land | every contact is intact | [13](../docs/13-control.md#section-1s-conditions-are-contacts--read-and-measured) |
| M7 | How a playing sound's position, near, far and volume become gain | a sound keeps the linear gain and pan it started with, a loop included | [11](../docs/11-effects.md#type-2-is-a-sound--read-and-measured) |
| M7 | What `0x1000e900` accepts past its first edge test, and the class the small-face stop reads | a face whose plane has the centre in front within the radius, measured to the triangle's nearest point; the hero's class is its size class, 2, so small faces never stop it | [24](../docs/24-motion.md#collision-between-objects--read) |
| M7 | The batch flags 8 and 0x200 a collision's face query passes: a mesh's batch record carries no such word | no batch passes | [24](../docs/24-motion.md#collision-between-objects--read) |
| M7 | Which pairs the collision pass moves besides the hero's | the hero is always the mover and nothing else is pushed | [24](../docs/24-motion.md#collision-between-objects--read) |
| M11 | Who sets a collision object's flags, so which movers carry 8 and keep the floors (triangle flag 2) in their push-out | no mover carries 8: every floor lets a mover by | [24](../docs/24-motion.md#not-established) |
| M8 | The two labels a wingman line draws beside its number, and where the panel and the order menu stand on screen | the unit's name; wingmen down the left 19 apart from (20, 100), rows 19 apart from (220, 250), on a 640 by 480 screen | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | Whether a digit the wingman selector takes also reaches the input table that toggles the hero's guns | it does not | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | What a unit record's `+0x30` is, which picks capturers and the acknowledgement voices | the size class of the chassis's name: t 1, l and h 2, m 3, b 4 | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | The acknowledgement's xorshift: its shifts and seed | 7, 9, 8 from 0xACE1, never repeating the last voice | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | The game's random source for a life update's spread | a 16-bit xorshift (7, 9, 8), seeded apart per unit | [24](../docs/24-motion.md#water-and-lava-beds-kill--read-and-measured) |
| M8 | The shell's menus after a mission, the load-game screen, and `MISSIONS/dispatcher.ini` | Esc after the outcome closes the window, L does nothing, and a win is not written to the install | [34](../docs/34-progression.md#after-the-outcome--read-and-measured) |
| M8 | A material's start stamp, from which its track is played, and the random a mode-3 track jumps by | the world clock's 0 for every material; a hash of the clock | [07](../docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured) |
| M8 | The areal search, the local path and its obstacle contours; the Wizard's heading curve | the straight line to the place, cut into at least three points a second or more apart at the walk's velocity, ending in the read stop; the heading is the curve's velocity's direction | [24](../docs/24-motion.md#not-established) |
| M8 | How a velocity the Wizard writes combines with the velocity integrator, and whether its spin is a rate or a fraction | a driven machine takes the written velocity as its own and turns toward the heading at up to its live yaw rate | [24](../docs/24-motion.md#not-established) |
| M8 | The height a flyer's points are given; who reads `Movement_FlyHeight` | at least `FlyNearLandHeight`, 15, above the ground under the point | [24](../docs/24-motion.md#not-established) |
| M8 | How an engagement scores the radar's contacts through the task (the contact record's three unnamed fields); follow's and refit's priorities for one | the nearest hostile unit within 500 is the best, and for a patrol the one nearest its centre inside its radius; follow and refit answer 0; an attack running is not given another | [31](../docs/31-packages.md#between-orders--read) |
| M8 | The follower's two timers; the behaviour's random source | it measures once a second; a 32-bit xorshift | [31](../docs/31-packages.md#what-each-package-does--read) |
| M8 | A building's pod, the generator's half distance, the construction phase, which areals are usable, and a dock for a refit | a capture walks to the building's placement; the retreat, read to lie off the map, roams; a roam takes the first point tried; no dock is modelled, so a refit always fails at its start | [31](../docs/31-packages.md#where-a-search-looks--read-and-measured) |
| M8 | How the turret turns a traced point into its targets (`0x10028bb0`), its aim stage, and the gun's report codes | each channel moves on by the angle the sight is off, at the rate a small nudge turns it; θ is 0 once both channels reach their targets and π before; a guided gun's θ is its lock left × π | [29](../docs/29-weapons.md#how-the-ai-fires--read) |
| M9 | Where a gun's takt stores its report codes 0 and 3–6, which the weapons list's lamp reads | the code from the gun's state now: 5 no rounds, 6 short of charge, 7 not ready, 3 stroking, 4 waiting its interval; then the gate's 2, 7 or 8; 1 locking; 0 ready | [29](../docs/29-weapons.md#the-guns-takt-a-stroke-then-the-interval) |
| M9 | The charge level of a gun with no capacity, which only a shot sets | its bar shows full | [29](../docs/29-weapons.md#a-gun-is-a-capacitor-a-magazine-and-a-clock--read) |
| M9 | Which caller hands a unit's name its class word, and which robots are *"Tiny Tower"* | each class letter its own word (W *Warrior*, T *Transport*, B *Builder*, C *Comm. Center*); no robot is a Tiny Tower | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
| M9 | The component value `0x400` the *"Dangerous!"* line asks for | no unit is called dangerous | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
| M9 | A panel's battery arc and shield sectors: batteries and shields are not simulated | a unit with a battery reads full and one without empty, so the low battery voice never plays; every sector of a unit with a fight shield and a deflector reads full | [23](../docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured) |
| M9 | A wingman's order queue, which its status line names | the running task names the order: none *no order*, standby *standing*, follow *following*, search *searching* (*capturing* for a capture), refit *refitting*, attack *attacking* | [31](../docs/31-packages.md#the-orders--measured) |
| M9 | The driven unit record's `+0x10 ÷ +0x14` in the scale of the square about the target | the camera's focal length: the square's half-side is two thirds of the target's projected radius, then held as read | [35](../docs/35-hud.md#the-frame-around-the-target-in-the-world--read) |
| M9 | How the panel camera draws a mesh in the colour it is handed in mode 2 | flat, untextured and opaque, the node colour lifted by 0.1 and by 0.1 more with the light: an intact dummy (38, 166, 38) against the recording's (39, 162, 41) | [35](../docs/35-hud.md#the-unit-in-the-middle--read-and-seen) |
| M9 | The interface's `CState` and the landing warning the indicators show; the repair system, camouflage and infrared themselves | the figure is lit and the warning grey; G, H and N turn switches that only the indicators read, and Y steps a level nothing else reads | [35](../docs/35-hud.md#the-indicators--read-and-seen) |
| M9 | What the camera view's property 0, which places the reticle's dot, is | the dot stays at the middle | [35](../docs/35-hud.md#the-reticle--read) |
| M9 | The stage sets a sprite may pick, and how the 2D layer samples | every sprite the default; pages sampled nearest | [35](../docs/35-hud.md#how-the-radar-draws--read) |
| M10 | When a generic device's or a radar's channels are played between its steps: only while one of two countdowns `IControl` slot 8 sets runs, and no caller is found | every tick, so a rotor and a dish turn smoothly | [28](../docs/28-chassis.md#what-a-devices-value-turns--read-and-measured) |
| M10 | Which keys act while a briefing plays, besides Esc | none reaches the hero or the game's commands | [21](../docs/21-briefing.md#not-established) |
| M10 | The default `SUBTITLES` is read with | subtitles show when the key is absent (the install sets 1) | [21](../docs/21-briefing.md#when-it-runs--read) |
| M10 | That a unit record's `+0xd8` and `+0xdc`, which the satellite map's heading line runs along, are its heading | the line runs along the hero's facing | [35](../docs/35-hud.md#not-established-4) |
| M10 | Which clans' units the map marks besides the player's: the player clan record's `+0x54` list | the player's own units only | [25](../docs/25-sensors.md#not-established) |
| M10 | Whether a cull mode changes for the mirrored reflection frame | the faces that face the mirrored eye draw, as a mirror shows them | [03](../docs/03-terrain.md#not-established) |
| M10 | What the reflection camera's pass flags `0x120` leave out | the effects' sprites; the dome, the ground less its water and beds, and every shown object draw | [03](../docs/03-terrain.md#not-established) |
| M10 | How far the water's bump map displaces its lookup | a signed byte stands for −1 to 1 at 127, so the largest offset is 0.01 × 64 ÷ 127 of the box | [03](../docs/03-terrain.md#not-established) |
| M10 | What a device's byte 0 of 1 adds from the machine's list at `+0xc4` | nothing: its channels hold their initial values (no Mission 01 unit has one) | [28](../docs/28-chassis.md#not-established) |
| M11 | What `CBuilding` does to a door's or a computer's switch word as it files the item (the records leave the constructor's 5, which wraps for ever) | a door and a pod start shut, word 0 and progress 0 | [24](../docs/24-motion.md#walking-into-a-building--read-and-measured) |
| M11 | The capsule a door's part is measured against (`Terrain.dll:0x1005a27f`) | the door node's level-0 slot sphere; the holds are worked out from every child each tick rather than on each child's move | [24](../docs/24-motion.md#walking-into-a-building--read-and-measured) |
| M11 | A clan's ore stores and power distribution, which a build draws on | a build's ore request is granted in full and its power taken as available, at efficiency 1 and a use of 1 a second | [23](../docs/23-economy.md#construction--read) |
| M11 | What the factory screen's Ore and Energy rows hold with no economy modelled | Ore 0, as with no mine or storage; Energy the player's generators' share of the map's | [23](../docs/23-economy.md#what-the-hud-shows--read) |
| M11 | The fill colour the resource rows hand their bar | the weapons list's: red under 20%, olive under 80%, green above | [36](../docs/36-factory.md#the-resource-rows) |
| M11 | How the cursor is shown in view mode 5 | the system's cursor, with the grab let go | [36](../docs/36-factory.md#not-established) |
| M11 | Which areals the escape's random points must be on | the first point tried within 150 of the unit, inside the map by 100 | [31](../docs/31-packages.md#the-escape--read) |
| M11 | The heights and textures of the patch and basement faces a building's insertion stitches in | the landscape is left out only inside a building's inner ring, from the ground queries and the draw, and keeps its own faces between the inner and outer rings | [03](../docs/03-terrain.md#for-an-engine) |
| M11 | Where a gathered face's batch word, whose 8 and 0x200 the collision query passes, comes from | the faces of the `DEFAULT`, `PORTAL_001` and `PORTAL_004` materials, a building's black doorway and portal quads, let a mover through, as a recording shows | [24](../docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured) |
| M11 | Whether the slope brake reads a building's stair faces | on a building's faces the slope brake is left out | [24](../docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured) |
| M11 | Which of a turret's nodes the boarding test's property `0x52` reads the life of | the turret part's node 0 | [39](../docs/39-boarding.md#boarding--read) |
| M11 | The heading the hero is given on leaving a bot, read as (F.x, −F.y) under an assumed matrix layout | the hero faces the bot | [39](../docs/39-boarding.md#not-established) |
| M11 | The name a bot's gun takes in the weapons list (`0x1008a470`, not followed) | the gun part's code in the player clan's research tree, or NONAME | [35](../docs/35-hud.md#the-weapons-list--read-and-measured) |
| M11 | That the factory record's `+0x30`, the grade the constructor's chassis page is taken over, is the building's size class | the designer is given the factory's size class | [38](../docs/38-designs.md#not-established) |
| M11 | What the turret fit does to guns on a turret it replaces, and the gun fit to a clip on a gun it replaces | the old turret's guns and clips, and the old gun's clip, go with it | [38](../docs/38-designs.md#not-established) |
| M11 | The part box's `Epower`, the properties behind `regener`, `capacity`, `throughput`, `shotnum` and `blast`, and the formatter that prints one decimal whatever the template asks | `Epower` and `Adfactor` print 0.0; the others are the record values the recording's figures fit (a shield's second value, a repair unit's first, a battery's first ÷ 1000 and its power figure, a magazine, the round's first area blast); every number one decimal | [38](../docs/38-designs.md#not-established) |
| M11 | Which destination row a designer tab selects as it turns on, and which tab the panels turn to after a fit | the first row; *seen*: a chassis turns them to Turrets and a turret to Weapons | [37](../docs/37-designer.md#not-established) |
| M11 | The condition under which fitting a chassis enables the Armour tab (`0x10052491`) | when the chassis has an armour slot | [37](../docs/37-designer.md#not-established) |
| M11 | The designer's save name field and load list | not built: save and load do nothing | [37](../docs/37-designer.md#not-established) |
| M11 | Which way a model view's camera looks, which axis its −0.5 rad pitch turns about, which of the view's sides its 60° field spans, and its two lights' colours | from −y, about x, the narrower side, about the drawn level-0 vertices' sphere; lights grey 0.4 and a scene colour of 0.15 | [37](../docs/37-designer.md#the-previews--read-and-seen) |
| M11 | How a scan band's green specular lights its strip | an added colour of (⅔g, g, ⅔g) over the strip in `0xff009b00` | [37](../docs/37-designer.md#the-scan-bands--read-and-seen) |
| M11 | Which cell of its material an effect sprite samples, and when the material's track plays for a particle | the material's first key's cell, the track not played: `smoke_fr_02`'s puffs show `fire_smoke`'s first cell, orange, where the recording's plumes are its later, black cells | [07](../docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured) |
| M8 | A gun fitted as a part of its own: its ready byte and its barrels' recoil | ready; its recoil is not played | [29](../docs/29-weapons.md#a-gun-is-ready-once-its-arm-is-out--read-and-measured) |
| M8 | Which difficulty profile a wingman holds: `Speed_MaximumFactor` | 1, as four of the five profiles set it | [26](../docs/26-damage.md#the-difficulty-ratio--read-and-measured) |
| M8 | How a knocked-off part flies: its push, its spin, its update, and the world query that ends a flight early | it drops off at 2 m/s away from the unit's centre, falls under gravity turning at 3 rad/s about a level axis across its path, and its flight ends when its sphere meets the ground (the player remembers a part falling about half a second): a dummy's side panel goes 1.3 s after it is knocked off | [26](../docs/26-damage.md#not-established) |
| M8 | A unit's life system holds all its models' nodes under one root | a life per part: a dead unit destroys its other parts' roots, and a part's root, having no parent, is never knocked off | [26](../docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured) |
| M8 | The statuses 4 and 8, a node copying its parent's life fraction or its stage | not modelled; no Mission 01 node carries them | [26](../docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured) |

## Departures

A departure is where the engine does something the game is read *not* to do,
for comfort. Each is marked in the code as `// DEPARTURE: docs/NN#section`,
has a row here, and a switch that restores the game's behaviour.

| what the game does | what the engine does | switch | see |
|---|---|---|---|
| The hero's body node yaws with the gait, ±10° once a run cycle, and the turret, eye, sight and barrels swing with it | node 0 keeps only the part of its turn not about its up axis, so the view, the sight and the barrels hold the heading the body moves along | `--sway` | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| A capture changes only the unit's clan, SuperAI and areal map, and gives it no order, so a captured bot engages a hostile within 500 on its own | a captured bot is given Standby, and holds until the player orders it | `--capture-idle` | [27](../docs/27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured) |
| The HUD's 640 × 480 layout and the outcome panel scale by the screen's width over 640 across and its height over 480 down, so on a wide screen they stretch | the layout scales by the height alone and each element keeps its pin to the screen's edges: the panels in the bottom corners, the weapons at the top right, the radar at the bottom middle, the reticle in the middle; the warbot designer keeps its shape centred, over a black ground across the window | `--stretch-hud` | [35](../docs/35-hud.md#how-the-radar-draws--read) |
