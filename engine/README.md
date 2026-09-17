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

Not yet: lights, animated textures, and what a dead unit leaves
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
  roams when there is none; Search and capture and Capture building take a
  building from its pod (M13); Refit walks to a charging station (M14).
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

**M12.** Mission 03, *The Field Base*: a base commanded from a bunker.

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
- Every clan that is not neutral thinks, the enemy's too: an idle unit engages the best
  hostile within 500 and chases it no further than 1,000 from where it stood; a shut-down
  unit neither moves nor fires; a patrol walks a loop of points about its place, building or
  unit, fights inside its radius and drops an attack that strays past its limit. A building
  with guns aims and fires at units, never moving, once its clan is not neutral. The clans'
  scripts run their `Init`, function 15 orders any clan's unit by its logical id and function
  34 counts a clan's units and buildings of one type, so Mission 03's enemy waits shut down
  until the player's fourth warbot, then patrols to the base.
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
- A build order walks its builder to the site at full speed. On arrival the scheme's first
  building appears for the builder's clan, on its clan's list at once (function 34
  counts it), the builder pays its ore, going below 0, and is left with no order; the
  building runs its 41 s construction sphere: the sign for 5 s, 30 s sending everyone
  out, then the dome and the ray with a kill inside every 250 ms, and the ray stopped a
  second before it is done. A site is tested as `IsPlacementValid` reads it: inside
  the map, clear of the other buildings' spheres, no basement face steeper than acos 0.88
  with a builder, and a mine on a found lode within 20. On Mission 03 `tut3_b` puts the
  Small Mine on the lode for 540, left owing 340, and objective 3 completes. A Route's go
  task walks to its place and is over within 30.
- Ore and power move as docs/23 reads them. Every building has its profile's figures, its
  class-26 efficiency and its batteries: its efficiency draws 0.01 and its task's usage a
  second from them every 250 ms or so, and KPD is the efficiency times the share they
  serve. Every 192–255 ms each clan's generators (the bunkers too) top every other
  building's batteries up by the same share of what they lack, and its mines and storages
  give ore, each `min(held, dt × KPD × 1)`, to every building that asks, by the same share
  of what it asks. A mine digs 50 × KPD a second into a total of at most 500 written over
  what it holds, drawing 1 a second; a factory's build takes of its request what arrived,
  asks again, and draws `Use_Power` until its power is in, its ore cost divided by its KPD.
  The Ore row reads the clan's mines and storages over 4,500, the Energy row its power out
  less its batteries' lack over every clan's. On Mission 03, with the generator and the
  bunker taken, the Large Factory waits on ore and builds an *SSW-X* 25 s after the mine
  starts digging, the rows reading 11% and about 90%.
- Transport minerals sends a transport between its clan's nearest mine with a free loading
  place and its nearest storage: it loads 100 a second at the mine's loading place until
  its 2,000 are aboard (a full mine is full again each takt), walks to the storage's
  unloading place, unloads 100 a second while there is room, waits beside a full storage
  until there is more than 0.5, and goes round again; with no mine or storage the task
  ends. On Mission 03 it carries 2,000 from the new mine to the Small Warehouse, and the
  Ore row reads 56%.
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
- Telepresence: a unit page's three drive buttons take the selected unit over (mode 2) with
  its own cockpit and table while the hero stays in the bunker; Esc goes back to the command
  view with the camera where it was left.
- The hero walks into Mission 03's buildings by their hall ways (docs/24, "The ways into
  Mission 03's Small Generator and Small Bunker"): from the Small Generator's south exit down
  its ramp and through its sliding door to the pod, which captures it 8.2 s after it steps on,
  and down the Small Bunker's sunk ramp, through its door, to the pod, which opens command
  mode 14.6 s after the ramp's top, as the recording takes 8.5 and 14.2 s.

On Mission 03 the hero takes the Small Generator and the Small Bunker from their pods, the
builder puts the Small Mine on the lode from command mode, the transport carries its ore, the
Large Factory builds four *SSW-X* in batch, and they beat the patrol: the mission is won. Not
yet: Explode!, tooltips, the chat and game menu buttons, and the designer's save and load.

**M13.** Mission 04, *Teleport*: a mobile command centre, research, a small warbot's captures, and the Main Teleport.

- Enter on a neutral unit within 20 m captures it and, where the hero can board it (size
  class 4, its turret alive), boards it at once. Mission 04's *LWC-1 Comm. Center* is taken
  and driven with one Enter from its landing point, and its capture completes the first
  objective through `tut4_pl2` with T04_I02 and T04_H01.
- Enter aboard an HQ unit (its turret's flag `0x8000000`), or in telepresence aboard one,
  opens the HQ's command view (mode 3); Enter aboard any other bot does nothing. The HQ is
  let go to its AI with its order and stays selected, the hero stays aboard, and the view is
  command mode's: the panel, the map, the picks, the orders, the HQ ordered from its own page.
  The camera is placed on the HQ facing north and rides with it: a metre a frame it pulls back
  along its look to 8 × the chassis's sphere (61 m on this HQ), each axis snapping onto that
  point within 3 m across and 2 m up and otherwise driven toward it at 125 m/s along the
  world's axes, at least 2.5 over what is below and within the 36–236 band. The arrows and
  PageUp/PageDown do nothing; the cursor at an edge swings the camera round the HQ; Z zooms.
- Esc closes the map, then turns the page back, then returns to the HQ's cockpit with the HQ
  taken at auto-driver level 0; Esc again puts the hero down beside it. Telepresence from
  mode 3 comes back to it with the camera placed on the HQ and pulled out again. A unit box of
  the HQ type shows Strategic control, which opens that HQ's view from a bunker's or another
  HQ's. The hero button rolls the stack back a mode at a time to the hero on foot. A lost
  driven bot rolls the stack back in modes 1 and 2.
- Research as docs/16 reads it. Every clan's tree is its `.trf` as state: an item's three
  bits and its stored progress. A research centre runs its queue of order 14s one at a
  time; a task takes a free technology while the centre has one (costing nothing), else
  the item's research energy and ore, with the centre's `FreeResearchTime` as its time.
  Each takt adds `min(KPD × Use_Ore × dt, ore held)` ore, `KPD × Use_Power × dt` power and
  dt time, and the item's progress grows by the smallest of the three fractions. At 1 −
  0.001 the item is researched, what it unlocks opens, and every newly open free item is
  researched at once. A captured centre researches from its new clan's tree. The
  designer's part lists, the build rows and a building's price read the tree as it stands.
- The research panel, command mode's page 4 and the screen a player's research centre's
  pod opens (mode 5), as docs/41 reads it: the box with the selected item's name, its
  descendants and its part turning, or *No item selected*; the batch button, which
  cancels everything queued and orders every row; the scroll row; and a row per item open
  to research and priced, `name (code)`, its start or stop button and a bar of its
  progress, red, olive or green. An order goes to the end of the queue of the player's
  standing centre that is not building itself and holds the fewest; a cancel keeps the
  progress. Each pass, a queued item now researched is dropped from its queue and
  reported: *"Research complete... (name)"* from System and `VOICE_RSRCH_COMPLETE`.
- On Mission 04 the Enhanced Research Center's pod captures it and opens its screen with
  the one row, *Large Battle Turret (4L1)*; ordered, it is researched free in 5 s, and
  the Large Flying chassis's turret socket then offers `e_tur_bb_01`.
- A small warbot captures buildings (docs/31, "The capture, tick by tick"). A click on
  another clan's building with only size-1 or size-2 units selected searches that one
  building; Search and capture searches by type over `0x8017365e`, taking the nearest
  foreign, finished building across the ground, a generator at half its distance, never a
  bridge or a main teleport. A flyer lands first at the nearest corner of the building's
  contour (its `.bas` outer ring, or an octagon about its construction sphere) and then walks
  in; a walker walks straight in, along the building's hall way to the pod from the exit (or a
  vertex within 5) that makes the whole way shortest, and holds on the pod until it fires. A
  search on one building ends there; a search by type walks back out along the hall way to
  the next building, and roams once none is left. A script's `ORDER_ROBOT_CAPTURE` takes the
  one-building path.
- A unit with no order standing on a live building that is not a ruin is given the escape, and
  one that walked into a building along its hall way walks out the same way.
- A change of owner says what `0x100a48a0` says: string 5039 and a voice by the old clan when
  the player's clan gains a building (the neutral's for a neutral clan or a word of 1, the
  enemy's for 0, else the plain one), and the plain voice alone when it loses one.
- Both satellite maps mark buildings with their `icons` cell, 20 × 20 about their place, in the
  marking rule's colour, white for the player's own selected building; another clan's
  buildings and units show only while one of the player's units has them within its radar's
  range.

On Mission 04 the helicopter ordered Search and capture takes the Large Factory 54 s after the
order, landing at its ring's vertex 9 and walking in by exit 67 as the recording's does, and
the Research Center 42 s after that, their icons turning from grey to light blue; the
recording's took 90 s and 81 s.

- A repeated `mission.cfg` key is kept in the objective list, as the game's loader walks the
  lines by index: Mission 04 lists six objectives, `objective4` twice, and the sixth, which
  nothing completes, holds the win back from the Teleport's capture.
- A repeated `MISSION_COMPLETE` or `MISSION_FAILED` does nothing again; either replaces the
  other.
- A main teleport's in and out places are ticked every 64–128 ms as upright cylinders about
  their vertices, 5 across, 3 up and 2.1 down (3 at an out place). A hero on foot in an in
  place, of the teleport's clan while that clan holds every generator on the map, is put on
  the hall way's `0x10000` vertex in the chamber buried under it, turned as it was; a hero in
  the out place runs the teleport's clan's `Hero_Teleported`, the handler three after
  `Mech_GeneratorFound`, and Mission 04's wins.
- The Main Teleport's pod takes it like any building's, and opens no screen and selects
  nothing. The hero climbs its stair from the third exit to the pod, which fires 2 s after.
- The push-out gathers every face in the sphere, a floor too, drops a face another hides from
  the centre, and only then lets a floor or a see-through face by: the pod computer's wall
  standing under the Teleport's pod room floor no longer stops the hero on the stair. A face
  is touched past its first edge alone, a→b, b→c, c→a in turn, at that edge's foot, start or
  end; the hidden faces go from the farthest, each out at once, behind a hider crossed from
  its front, inside it on the plane of its normal's largest axis, an edge within 1e-5 letting
  it in.
- `--at X,Y,YAW,Z` stands the hero on the highest floor at or below Z, as in the chamber, and
  `--pod` stands the hero on a pod room floor that stands higher over its node than 4.

- A driven bot's hull comes round under its turret, and a unit's live limits come from its
  fitted engine, its load and its body's life (docs/30, "The hull follows the turret";
  docs/24, "What sets the live limits"); its ground contact holds it by its parts' sphere's
  radius (M14: about its node sphere's centre).
- A unit the hero's Enter took holds no mind, so Mission 04's factory has the recording's one
  free mind once the hero, the helicopter and the HQ are the player's.

On Mission 04 the hero walks up the Teleport's stair and takes it, under its arc drops into
the chamber, and walks up it to the field: the mission is won. Played as the briefing means
it, one Enter takes the HQ and another opens its command view; the helicopter, sent to search
and capture, takes the Large Factory and the Research Center; the Large Battle Turret is
researched free in 5 s; the Large Factory builds an L-2f with it in 60 s; routed to the HQ, the
flyer is boarded and flown up onto the Teleport's plateau; and the hero takes the Teleport and
goes through, 318 s in. Not yet: tooltips, the research box's clip, the maps' route lines and
selected outline, the ground contact once a frame (the Small Bunker's sinking door still
presses a hero standing at it down), and which of Mission 04's units holds no mind.

**M14.** Feedback on Mission 01.

- Backing up mirrors the strafe angle, as `World3D.dll`'s handlers read (docs/24, "From input
  to motion"): a strafe key going down while walking turns the hull ½ × π/2 with the sign of
  the way it walks, so S with A goes back and to the left, 225° clockwise from the heading, and
  S with D back and to the right, 135°, whichever went down first. A walk key under a strafe
  key works the angle out again, W or S let go under one leaves the strafe going at full speed,
  and a walk key let go while the other is held walks the other way. A key row without a ramp
  runs once as its key goes down or comes up, as read (docs/14, "A row that stays down").
- A selected guided gun draws its lock (docs/35, "The guided lock"). While its report is
  locking or locked, its round is one of the 20 with a seeker (frame `+116` of 16), it has
  rounds and its share is not 0, four 9 × 9 corners of `page9` close from the HUD's (30, 30)
  and (630, 450) onto 20 about the target's projection, each lock after the first 4 further
  out. The share is 1 − the lock left ÷ value 9, kept every tick and held once locked; the
  corners' green runs from 205 to 255 with it. `TARGET_ZOOM` beeps past 0.35 s since the last
  beep while locking, and `TARGET_READY` past 0.2 s once locked. The locks draw while the
  satellite map is open too. On Mission 01 the missiles, 60 m off a dummy, lock in 4 s.
- The wingman panel as the game draws it (docs/31, "The wingman menu from first person"),
  in place of lines of text. Whenever the driven unit has a wingman, with the selector off
  too, a line per wingman stands at (0, 19 i) out of the compound-control pieces: its number
  in a box, a lamp, the unit's two icons in 19-square pieces, and its name, not its path, over
  its life in a 135 bar. A chosen line's lamp is green and its emitter lit; while picking, an
  unchosen line's lamp is yellow; otherwise it is black with its icons' tint halved; unchosen
  names are grey. While ordering, the menu's rows stand 19 apart from (220, 50), each its key
  in a box and its order in a 150 bar, grey when disabled, with no *Orders* strip. On Mission
  01, with both warbots captured, the tilde draws the two lines and the seven rows where the
  recording's stand at 208 s and 228.7 s. The commander's unit rows' icon pieces are 19 wide,
  as read.
- A beam outlives its round (docs/29, "A beam outlives its round"). A round whose flight is
  over, by a hit, the map's edge or its range, stays where it stopped for its controller's `+92`
  ms, and its effects with it; its hit, edge or range group starts, switches or deletes them by
  id. Every laser, taser and builder round restarts its bolt in time mode 1 as it stops, so the
  hero's laser beam stands from the muzzle to the dummy it struck for 0.75 s, fading straight
  from whole to nothing, as the recording's is gone 0.78 s after its round left. The bolt starts
  at the muzzle the gun handed the round's effects, carried with the shooter's node 0, so the
  beam's near end rides with the hero.
- An effect sprite is coloured by its material: the scene colour plus the material's ambient,
  the self-light nearly every effect material carries, times the texture. The laser's grey
  `LASER.0` draws red under `NE_Laser_R`'s ambient and a yellow-white core under `NE_Laser_Y`'s,
  and a bolt's texture runs along the beam, its bright line down the middle.
- The effects draw after the scene into the frame read without sRGB decoding, so they blend in
  display space as the game's device adds to what its surface holds: over Mission 01's lavender
  sky the laser saturates to the recording's pink band and white core. Not yet: near the camera
  the recording's beam looks broader still.
- `--press N` and `--release N` put the `--hold` keys down and let them up after N of the
  `--ticks`: `--skip-briefing --face l_targ.dat,60 --hold SCAN_LMOUSE --press 3900 --release 3904
  --ticks 3920 --screenshot beam.png` draws one shot's beam under the sky of 65 s in.
- The bottom panels' views stand off the unit as it is drawn this frame, its placement between
  its state step's two poses, not where the step will end (docs/35, "The unit in the middle"):
  the hero running holds still in its own panel, its legs moving, where the camera had jumped
  up to 0.26 m a tick with each stride, and the target panel looks along the line between the
  two as they are drawn.
- Z zooms the driven unit's own view (docs/30, "The zoom"): each update its field steps 0.1
  toward 0.2, eleven updates from the hero's 1.3, and back when Z is pressed again, which it
  takes only at an end; every unit of the player's clan keeps its own zoom, and while the view is
  zoomed the mouse filter's multiplier is 0.5 in place of `MOUSE_SENS` × 0.01.
- C turns the outer camera on the driven unit, on foot, aboard a bot or in telepresence (docs/30,
  "The outer camera"). Each press takes it to the next of four places: right near (0.4 rad off
  straight behind, 2.5 r back), right far (0.2, 4.5 r), left far and left near, then back into
  the eye and off. It stands behind the unit's own eye along its heading turned by the angle,
  0.15 r below the eye (0.45 r above on a flyer), looks where the eye looks with a field of 1.3,
  and eases there: each update takes 1.25 × the seconds since the press of the way left, done
  at 0.4 s (0.24 s back into the eye); a press while it moves does nothing. A camera the
  line from the eye meets something on is brought in front of it. The unit is drawn whole from
  it, a boarded bot too, with the HUD over; Z does nothing, and any change of mode turns it off.
  On Mission 01 the hero stands in the four views where the recording's put it.
- A blended batch draws as the game's draw item does (docs/07, "What a blended batch writes"):
  every blend mode but 0 drops the fragments whose 8-bit alpha is 0, as `ALPHAREF` 1 with
  `GREATEREQUAL` does, and writes depth; only a batch whose ambient alpha is below 1 is
  translucent, drawn after the rest without writing depth. A palm's nearer leaves now hide
  the leaves behind them.
- A beacon light, an effect with header flag 0x400 (docs/11, "A beacon light's glow"), draws
  nothing while its tested point is hidden and, while it is in view, draws every sprite over
  the scene as bit 8 does, so the glow on the bridge's pylons is no longer cut by the faces
  it hangs on.
- The hero's breath is not heard (docs/11, "How a sound is heard"): Mission 01's recording
  has no `H_breath.wav`, so the hero's own turret effects flagged `0x800` play no sound,
  while they update as ever.
- Explosions, shots and steps fade with distance as Direct3D Sound fades them, which the game's
  sound server hands every emitter's sound to (docs/11, "How a sound is heard"): whole within
  the emitter's near distance, then near ÷ distance, no quieter past its far distance, 6 dB a
  doubling where the old straight line from near to far lost barely 1 dB by twice the near
  distance. A one-shot farther than its far distance is not played, and stops once it gets
  there; a loop plays on. Every update hands a playing sound its emitter's place again, so its
  gain and pan follow the emitter and the eye.
- A neutral unit Enter captures and the hero does not board answers as an ordered one does
  (docs/27, "A neutral unit is taken by the hero"): its size class picks `_S`, `_B` or the
  plain set of acknowledgements, never the voice given last, queued behind the voices before
  it. On Mission 01 `tut1_mf1` answers plainly and `helic` in the `_S` voice, as the
  recording's pair does; Mission 04's HQ, boarded at once, says nothing.
- The music plays (docs/34, "Music: the CD's tracks"): the CD's tracks, which the install ships
  as `MUSIC/Track02.ogg` to `Track10.ogg`, streamed through kira's Ogg Vorbis decoder. As
  `iron3d.dll`'s CD player drives them, a mission starts a random track, or with a briefing
  stops the CD and plays its `cd_track` looping and starts a random track on the first frame
  after it; 2 s after a track ends another follows, never the one just played and never the
  data track, `rand()` seeded from the clock. `PLAY_CD_MUSIC=0` keeps it silent.
- A machine's ground contact holds its body by the agent's sphere's radius, the parts' header
  spheres joined and held to 7.5 under 20, about the **node sphere's** centre (docs/24,
  "Finding the ground"): every exterior node's level-0 slot box as a sphere about its diagonal,
  joined the same way. Getting out of a bot tries its places both node spheres' radii out
  (docs/39, "Leaving"). A landed L-2f's origin stands 9.67 over flat ground, not 10.25, so on
  Mission 02's island it reads altitude 11 by the Outpost, as the recording's does, and lets
  the hero out anywhere on the island's flat ground, where M13's model said "Risk area!"; the
  places stand 13.43 m out, and Mission 04's helicopter rests 3.04 over the ground.
- **A dock charges, repairs and rearms who stands in it** (docs/27, "What a dock gives"): the
  hall-way vertices with a bit of `0x620`, the charging stations. They tick with a main
  teleport's places, as upright cylinders about their vertices — 5 across, 3 up and 2.1 down
  indoors, 10 and 12 and 8.4 at a ground-level one — and hold only an occupant moving at most
  2 m/s, so a dock charges nobody driving through it. Each unit standing in one that belongs to
  the building's clan or an ally gains, a second, a tenth of its full life, its destroyed parts
  restored with the rest, and a tenth of each gun's magazine, rounded down but never less than
  a round, and of its capacitor: full in ten seconds, whatever it is. The 16 dock vertices the
  building models carry stand where the models put them — one indoors in each bunker, tower and
  the large ruin, one indoors and one outside on each factory, one outside on the Outpost and
  three on a generator — and a building put up in play gets its own.
- **A charging dock glows, and sounds** (docs/13, "A building's load group"): the `f_recharge_r`
  its load group placed over the field stands still at time 0 until something drives it, and a
  dock that is charging runs it looping and stops it again when it stops. It hangs on the
  field's own `Rech_*` control points, whose direction vectors size it — 15 up and 11 across
  over the Outpost's ground-level dock, 20 over a generator's, 5.7 and 5.2 over an indoor one —
  so the red field it draws covers the charging station it stands on, and `f_recharge.wav`
  sounds as its time crosses 0.1 rising, once every 3 s sweep.
- A building's portal quads are not drawn (docs/24, "The doorways are portal quads"). The
  `DEFAULT`, `PORTAL_001` and `PORTAL_004` faces are the openings between its cells, which
  `CBuilding` draws one at a time and reaches the next through, so a drawn one blacks out the
  room the portal exists to show. Dropping them shows the Large Factory's doors from outside,
  the lit rooms through its windows and the next segment from inside, where before every one
  stood behind a black rectangle.
- A round passes them too, as a mover does, so a shot at a door reaches the door: with the
  quad solid a shot at the Large Factory's entrance struck node 1, `o01`, a metre in front of
  the door, and no door with a doorway in front of it could ever be shot open.
- An action-3 effect stands at the centre of its node's level-0 bounding sphere rather than
  at the node's origin (docs/13, "Where an action-3 effect stands"). 68 of the 112 door
  sounds in `fortif.rlb` hang on a node whose origin is more than 10 m from the door, 30.8 m
  on the three factories' side doors, where a door sound carries 5 to 60 m.
- **Every unit is in the collision pass, not the hero alone** (docs/24, "Collision between
  objects"): after its move and ground contact each robot's swept body sphere is run against
  every placed object whose sphere it meets, its own excepted, and it takes the push as the
  machine does. A warbot is held by a building's walls and by its shut doors, and pushed off
  trees, stones and other units, where before it walked through them.
- **A bot built in a factory leaves by its doors** (docs/31, "The escape"). It is made at the
  factory's creation vertex, inside the building, so it counts as having walked in and its
  escape is routed out along the hall way, which is what the engine's own 20-second check does
  with a unit still on a building ("LEAVE IS TOO !!!"). On Mission 02 the Large Factory's bot
  walks north from the creation vertex, waits 2.25 s at the front door it opened by standing
  near it, and is out on the landscape 1.5 s later; before, the escape's straight line took it
  through the wall and the ground contact put it on the roof.
- **A slope holds an AI unit as it holds the player** (docs/24, "Ground and slope"). The
  mode-2 brake acts on the machine's velocity, and a velocity the Wizard writes replaced that
  every step, so the brake's pull never built up on a unit the AI drives: on a 40 degree face
  of Tut_1 a warbot walked up at 8 m/s where the same chassis under the player does not move
  at all. Where the brake acts, the ground's fraction is now taken off the written velocity
  whole; measured over Tut_1's faces the two agree at 20, 30, 40 and 50 degrees. Mode 0 —
  the Large Walking Chs, the Tiny Spider and every flyer — still ignores slope, as read.
- **An AI unit fires on what its own radar holds** (docs/25, "What the AI does with it"). The
  fire control and the engagement walk the unit's hostile radar list, not the whole map: each
  unit's radar is scanned every 750 ms over its range, and the behaviour may pick only from
  what it returns. Before, a warbot engaged anything hostile within 500 wherever it stood. On
  Mission 01 `tut1_mf1` reaches 350 and `helic` 250 against a `tut1_e1` 474 and 457 away, so
  neither takes it up until it comes nearer. A unit with no radar senses 1 m and picks no
  target of its own, as read. A target an order names is still taken whatever the radar holds,
  and a search still looks over the clan's areal map, which lists without any detection test.
- **A chimney's smoke is black, and the size the recording shows** (docs/07, "How a material
  reaches the device"). An effect sprite took its material's first entry and never played its
  track, so every puff of the Large Factory's plumes drew `fire_smoke`'s first cell, orange.
  A sprite now takes the entry its track is on, from its own start — a stream's particle from
  when it left — so a puff leaves the chimney orange and is on the track's later, black cells a
  quarter of a second on, as the recording's plumes are. A stream's particle walks and grows in
  metres, the frame only turning it: the plume stands 50 m over the chimney and grows from 10
  to 30 m across, against about 29 and 31 measured off the recording, where the control points'
  2.6-long axes had made it 130 and 78.
- **A building's ambience hums on** (docs/11, "Type 2 is a sound"). A loop stops when effect
  time leaves its window, and a looping mode's time comes round to 0 at the end of each period,
  below a window that starts at 0.001. The game's effect manager updates on wall time and lands
  in that millisecond about once in a thousand updates; this one steps an exact 1/60 s from 0
  and landed there at every wrap, so Mission 03's four bunkers, three computers, store and
  generator restarted their hum once a second, over and over. The wrap is no longer taken as
  leaving the window.
- **The wingman menu opens across the middle of the window** (docs/35, "How the radar draws").
  Its rows stand at 220 of the game's 640-wide layout, all but centred on the screens the game
  ran on; pinned to the top left with the lines above them they slid off to one side of a wider
  window. They are pinned to the top now, as the message box over them is.
- **A fit in the warbot designer steps to the next slot** (docs/37, "Fitting"). Fitting a gun,
  armour, a system or a clip left the destination panel on the slot just filled, so every part
  after the first took a click on its slot first. The panels step down the tab's own rows
  instead, and the source panel offers what the next slot takes.
- **A building stands on a band of stone** (docs/03, "What a basement face wears"). The
  landscape was cut away under a building's inner `.bas` ring and nothing put in its place,
  so the ground stopped short of the building and the sky showed through the gap. The cut is
  the **outer** ring now, and the band between the two rings is stitched in as the insertion's
  basement faces: layer-1 slot 0 with no second layer, which is every map's foundation
  material — `B_S0`, texture `B_FOUND`, a grey slab, and no shipped face on any of the 33
  maps names that slot. Its UV is laid over the world at 0.066 a unit, tiling every 3.8, and
  its outer edge drops onto the landscape, flush with it on the contour, while its inner edge
  holds the building's own base. The band is ground: a machine walks it, and a round stops on
  it.

Feedback on the first chapter's *The Arrival* and *Outflanking Maneuver*.

- **The hero is struck** (docs/26, "The hit test"). The hero on foot was no target in the
  battle: no round could meet it and no AI unit's fire control could take it, so an enemy that
  had it on its radar aimed at nothing. The battle now keeps the hero as a target numbered after
  the placed objects, as the radar numbers it, posed where the hero stands each tick and alive
  while it is in the world; its lives stay the hero's, lent to the battle for its frame. A round
  it fires never meets it. A node it loses plays its explosion, and losing it fails the mission.
  On *The Arrival* the first patrol takes the hero up in its ground and its rounds reach it.
- **A clan no relation record names stays hostile** (docs/25, "Clan relations"). The loader
  starts every word at 0 and files each record under the clan its name matches, ignoring case;
  a neutral clan's row and every word towards one become 1, and a clan's word towards itself 2.
  The engine had read a missing name as no relation, and *Outflanking Maneuver*'s records name
  every clan but `player`, so its enemy, towers and bunker took the player for no foe. Every
  relation now comes from the loader's words, the behaviour's own test with it: a neutral
  clan's unit takes nothing as hostile and a nature clan's everything, and neither a nature nor
  a neutral clan's unit is a hostile contact.
- **Shields** (docs/26, "Shields"). Every unit and building with a fight shield and a deflector
  carries six sectors, loaded from its controllers with a fitted generator or deflector taking
  its slot's figures, and the sector maximum at the level ratio: `hero11` 1,850 at 0.9,
  `11tin2` 350 at 0.7, *Outflanking Maneuver*'s Small Bunker 11,000 at 0.36. The bubble is the
  bounding sphere, up while both devices' nodes live. A round meeting it from outside takes the
  sector its hit's dominant axis names in the unit's frame; a round with more life than the
  sector's strength passes, emptying the sector and losing that much life, and any other stops
  on the bubble, its hit stopped by what the sector holds, the sector paying for it over the
  deflector. A blast crossing the bubble from outside is stopped first and the rest reaches the
  nodes; one inside it, and a shot from inside, meet no shield. Kind 4 hits shields alone. Each
  tick the generator recharges its sectors by what each lacks, at most value 1 a second, and a
  building's shield takes its batteries' level: an unpowered one drains and stops nothing. The
  HUD's six sectors read each sector's fill × the deflector's level and condition, red to green.
- **A shield hit flashes** (docs/26, "What a shield hit draws"). A hit on a sector with strength
  plays the generator's effect — `r_shield_r` on a small warbot, `r_shield_b` on the hero — at
  the bubble's centre, turned toward the hit and sized by its radius, on the next of three
  instances: the camera-facing wave, 2.4 radii across, is the translucent sphere, and two
  type-9 emitters draw hemispheres at the point of impact. A type-9 emitter's `+200` now draws
  its dome, 8 × 3, 16 × 6 or 24 × 9 quads, rather than a quad. The flash is the generator's
  colour whatever the sector holds, as read.
- **A hit pulls a unit in** (docs/31, "A hit pulls a unit in"). Every hit tells its victim who
  fired — a direct hit, every object a blast reaches, a round stopped by or passing a bubble —
  and at its next takt the unit asks for an attack on the firer, through the interrupt gate: an
  animal only while it grazes, never a building or a neutral clan's unit, a task answering more
  than 0.3, a pause of 2 s plus up to 3 between interrupts that the engagement shares, a firer
  that is not of its own clan, and a weapon. No radar or relation is asked. A stopped unit's
  attack keeps within 1,000 of where it stood, a patrol's within its guarded ground's limit, a
  follower's within 2 × radius + 20 of its leader, and an attack an interrupt made switches to a
  nearer firer; a migrating animal's runs 10, 20, 25 or 35 s within its pasture's outer radius
  plus 20, 80 or 100, by where it and the firer stand, and fires on the firer within 200.
- **A lobbed round flies as read** (docs/29, "How the AI fires"). The distance score skips the
  distance for a round whose frame flags carry bit `0x10` (1.1) or 8 (1.0), and has no ramp on
  an animal; *Outflanking Maneuver*'s Small Bunker, whose `bf_f_01` carries 8, had scored
  nothing past 33 m at its speed of 45 and never fired, and now fires on the hero 150 m off. A
  mode-3 round falls at 10 and turns along its flight, leaves along its barrel rather than
  converging on the sight, and a follower mount lifts its gun for the point its turret traces,
  a manual turret's taking no lift. The bunker's guns hang on its turret as parts, with no
  mount to lift them; their shells leave on the lower arc to the point the turret traces, and
  the guns hold their fire past the arc's reach, about 200 m on level ground. A building's guns
  also hold their fire on a target below the lowest its turret's sight looks (a departure,
  below), so the Small Bunker leaves alone a unit within about 40 m of it or on its pod.
- **Animals are units** (docs/34, "The medusas"). A medusa is one part whose controller holds its
  turret and gun, and it had never loaded; it does now, its own controller its turret. Shot by
  the hero from off its pasture on *The Arrival*, a medusa turns on the hero and its acid lands
  about it.

More feedback on the first chapter: armour, docks, turrets, batteries and repair.

- **Armour cuts every hit** (docs/26, "Armour"). Each unit's and building's last class-27
  component, a fitted armour part in its chassis's slot, now covers every node of every part:
  a hit becomes `min(d, linear × d + square × d²)`. It was parsed and never attached. The hero
  and *Outflanking Maneuver*'s `12wel2` wear `i_arm_l_02` (0.624, 0.00014), so the hero's 250
  laser bolt takes 165 off a hull of 224 and the warbot stands after one where it fell before;
  its helicopters' `i_arm_t_df` keeps 85%, and its tower's `i_arm_b_05` 22–30%.
- **A dock tops up the shield** (docs/27, "What a dock gives"). The device manager's value 7 the
  dock adds a tenth a second of is the fight shield's mean sector fill: raised, held to 1 and
  written back, which spreads the rise over the sectors by what each lacks, so a spent sector
  fills fastest. A dock also gives a tenth of a full battery a second.
- **A turret shot off takes its guns and radar** (docs/28, "The order parts load in"; docs/26,
  "Children go with their parent"). A gun part's node 0 is the turret's socket in the game's one
  model, so a socket destroyed, or stepping up a stage, now destroys the node 0 of every part
  hanging on it, and that part's walk takes its other nodes: they explode and go. A gun whose
  node has no life starts no stroke and reports 5, a stroke under way finishing, and a radar
  whose node has no life answers no scan. *Outflanking Maneuver*'s first `12tower`, firing on
  the hero, falls silent the moment its turret goes; a `12wel2` shot through one gun keeps
  firing the other two.
- **A unit spends its battery** (docs/23, "Bots spend power through the same code, priced by
  part"). Every unit with a battery runs its power tick every 250 ± 31 ms: the battery gives
  `min(output × charge × condition × dt, capacity × charge × condition)`, and the draws are
  served in the game's order — the engines (power × speed ÷ top speed × the state's factor),
  then channel 0 (the repair system), then the radar, camera, shields, deflector, detection
  shield and armour at one level, then the turret and guns (what each capacitor lacks) — and the
  battery drains by what they used. The shield recharges at the level its group was served, and
  a gun's capacitor fills only from what is left, where both were served whole before. The hero
  idles at 2.39 a second of its 4,080 and camouflage adds 0.3; walking costs nothing, every
  state of its chassis carrying an engine factor of 0, as read (docs/24). The panels' battery
  arcs show the charge, and the own panel says `VOICE_BATT_LOW` under a fifth.
- **G repairs** (docs/26, "Repair"). The repair system, switched by `CICLS_REPAIRSYS`'s rows,
  asks for its idle 0.1 and 0.04 a point for up to 15 × its node's condition points a second,
  never more than the unit's live nodes lack; what its level leaves past the idle draw buys
  points, handed to the nodes in index order, each filled before the next and a destroyed one
  passed over. The cockpit says `VOICE_REPAIR_SYS_ON` and `_OFF` as the player switches it. On
  Mission 01 the hero at half life heals 92 points in ten seconds, faster as its own node mends.
- **A driven unit's guns take the player's target** (docs/29, "Guided rounds differ in how
  hard they steer"). The target list is the driven unit's, and its target goes to that unit's
  guided guns, and their gate measures to it; before, the hero's own guns took it, so a bot the
  player drove, *Ballen's Crossing*'s HQ among them, reported every target out of range. A
  gun's gate and a seeker find a target at its node sphere's centre, not its placement, so the
  HQ's winged missiles, 380 m off a tower on the hill, clear the crest and bring it down.
- **A deleted unit leaves the scripts' view** (docs/15, "65534 is a destroyed object's
  owner"). Function 52 answers 65534 for a dead unit until it is deleted, its controller's
  `+92` ms later, and `ERROR` after, as no object answers its id; a building's shell answers
  65534 for good. So *The Iron Monster*'s heavy warbot objective, which waits for `ERROR` on
  logical id 22, completes once the warbot is destroyed.
- **The walker's global path** (docs/24, "The global path"). The engine reads each map's
  `Land.map` and links its areals as the game does: only a walkable areal, one whose first flag
  word is set, links to a walkable neighbour, at the distance between their centres + 1, and a
  live bridge's hall-way exits join the areals under them and its halves join each other at
  their flag-4 vertices. A non-flyer's walk is an A\* search over those links, each link's cost
  scaled by 1 + a random share up to 0.7, given up after 2048 nodes. Each step puts a waypoint
  where the line to the goal crosses the shared edge, or 3 in from the nearer end. A goal on no
  walkable areal is refused and the unit holds. A unit on one no link leaves makes for the first
  of 50 random points about it that is walkable, in squares growing from 30 by 3 to 500.
  Roaming, a patrol's points (a flyer's excepted), the escape's rings, a capture's landing
  corners and command mode's valid places all keep to walkable areals too. On *The Iron Monster*
  (C02 M01) the warbots that walked straight into the canyon under the bridge now cross over
  its deck, and a goal on the canyon floor is refused. Follow me takes the first of 77 spots
  about the leader on walkable ground, as read, so wingmen follow the hero over the bridge
  rather than stop at its end. A walker's legs are timed across the ground: the bridge's
  hall-way vertices stand 4 m over its deck, and a leg's height had slowed a follower planned
  again just short of one to a standstill at the halves' joint.
- **Trees and stones are cut out of the walkable ground** (docs/24, "A tree or a stone cuts
  the areals it stands on"). Each tree's and stone's footprint, the lower face of its mesh's
  box turned and scaled as it is placed, is taken out of the walkable areals it covers. An
  areal it cuts in two becomes two nodes of the search, one it lies wholly inside keeps it as
  a hole, and a walk goes round both. A goal inside a footprint is moved to the nearest ground
  outside it. On *Ballen's Crossing* (C02 M02) the laser walker's patrol had run straight at
  `s_stone_07`, and it spent most of a minute climbing the stone and sliding back; it now
  walks round it.
- **Refit walks a bot to a charging station, and a hurt one goes by itself** (docs/27, "What
  sends a bot to a dock"). A refit — the wingman menu's row and the commander's, and
  `ORDER_ROBOT_RELOAD` or `_REPARE` from a script — picks the nearest dock of a live, finished
  building of the unit's own clan or an ally, walks there along that building's hall way, holds
  still in the place while the dock charges, repairs and rearms it, and ends once life, charge
  and ammunition are all at 98%. It failed at its start before, on every map. Which docks a unit
  may use goes by its size class, as `MakeInsideDest` does ("TypedSizes missmached"): a tiny or
  small bot fits through a door and takes whichever dock is nearest, indoors or out; a medium or
  large one only a ground-level dock — a generator's two, an Outpost's or a factory's — and with
  none of those on the map its clan holds, its refit fails at its start as every refit did. A unit that needs service sends itself there with no
  order at all: under half its life or its charge, or more than 80% of its guns under 20% of
  their magazine, the trip goes on top of its task, which standby, shutdown, the escape and a
  capture (above 0.2 life and 0.3 charge) refuse. On Mission 02, *The Constructor*, a large
  warbot of the player's clan told to refit flies to the Outpost's ground-level dock, is full
  three seconds later and walks itself back off the building; hurt below half again, it goes
  back with no order. On Mission 03, *The Field Base*, a small warbot's refit walks it down the
  Small Bunker's ramp to the dock inside, while a large one on the same map passes that dock by
  and takes the Large Factory's ground-level one.
- **A warbot repairs itself while it is scratched** (docs/26, "What the AI does with the
  switch"). Every unit the player does not drive now runs the AI's repair decision on its takt,
  unless that takt sent it to a dock or into an attack: it switches its own repair system on
  while it needs service or its life is under `Decision_RepairOn`, provided its charge is over
  30%, and off once it needs none and its life is over `Decision_RepairOff`, or its charge falls
  under 10%. Starting any task turns it off again. Its power tick then pays for the points, as
  the player's G does. Bots left their repair systems off before, however hurt.

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
1 to 4 select guns and the left button fires; a selected plasma rifle or missile launcher
closes its lock's corners on the target and beeps as it locks (`--face l_targ.dat,60 --hold
SCAN_W_4 --ticks 150` draws the missiles half locked). The game's
own chords from `ui_other.man` pick targets: Tab the next listed, E the next
or nearest enemy, T a friend, the right button what the view points at; Enter
captures a neutral unit within 20 m, and the tilde opens the wingman menu over the lines the
wingmen show at the top left (`--wingmen --ticks 90 --tilde` takes both of Mission 01's warbots
and draws the menu). A click grabs the mouse; Escape lets it
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
slots, accept hands the design to the factory, and exit or Esc closes it. A
chassis turns the panels to Turrets and a turret to Weapons; on the tabs they
stay on, a fit steps the destination to the next slot, so the guns, the armour,
the systems and the clips fill one after the other.
`--designer` draws a screenshot with the designer open on the first factory,
and `--design PART,…` fits those parts to it in turn; `accept` among them
clicks accept, leaving the factory screen with the project.

Taking a bunker's pod opens command mode: the arrows and PageUp/PageDown move the camera,
the cursor at a screen edge turns it, Z zooms; the icon column opens the unit and building
pages, a click selects and orders in the world or on the map (M), a drag bands units, the right
button undoes, and Esc peels back the map and the page before leaving. `--page N`,
`--camera-yaw RAD` and `--ghost X,Y` draw command mode's screenshots after `--pod sbunk01.dat
--ticks 400` on Mission 03, and `--build X,Y` orders the first builder to build a mine there
before `--ticks` play. `--capture` gives every unit of the player's clan that may capture
Search and capture before `--ticks` play: `--mission MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.04
--skip-briefing --capture --ticks 3000 --map` draws Mission 04's map with both buildings taken.

On Mission 04 Enter by the HQ takes and boards it, and Enter aboard opens its command view,
whose camera rides with the HQ: the cursor at an edge swings it round, Z zooms, and the arrows
do nothing. Esc steps back to the HQ's cockpit, and again to the hero on foot. `--hq` takes
and boards the mission's first HQ and opens its view before `--ticks` play (`--mission
MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.04 --skip-briefing --hq --ticks 120 --page 1`), and
`--take NAME` then takes over the player's unit whose path ends in NAME (`--take tut4_f1.dat
--hold SCAN_W --ticks 360` flies the helicopter up the valley's west slope).

A research centre's pod opens its research screen once the centre is the player's, as
command mode's page 4 shows it too: a row's button orders or cancels its research, the
batch button orders every row, the arrows scroll, and exit or Esc closes the screen.
`--pod einst01.dat --ticks 400` on Mission 04 draws the screen.

Standing on a building's control pod captures it; `--pod NAME` starts the hero
on the pod of the building whose path ends in NAME (`--pod lplant01.dat` on
Mission 02), and `--at X,Y,YAW` anywhere on the ground. Enter boards a large bot
of the player's clan the hero has targeted within 20 m, and Esc gets out;
`--drive PATH` makes a unit of that design beside the hero and boards it
(`--drive 'UNITS\bld_unit_-2147483647.dat'`). Aboard a bot, or taking one over
from command mode, the mouse turns its turret and the hull comes round under it at
up to 0.7 of its yaw rate; keypad 5 switches the turret lock off and on, and with it
off `,` and `.` spin the hull. R and F climb and sink a flyer, and nothing else does.
Every machine's top speed and turn rate come from its fitted engine, its load and the life
left in its body, recomputed each tick: Mission 03's transport runs at 24 m/s, and a unit shot
to half its body's hit points runs at half speed.

On Mission 04, once the hero has taken the Main Teleport at its pod (`--pod mtp_m_n1.dat`),
walking under its arc drops the hero into the chamber below, and walking up the chamber to its
field wins. `--at X,Y,YAW,Z` stands the hero on the highest floor at or below Z: `--mission
MISSIONS/CAMPAIGN/CAMPAIGN.00/Mission.04 --skip-briefing --at 1172.2,1245.6,-2.434,80
--outcome won --screenshot won.png` draws the field under the won panel.

Z zooms the view of the unit the player drives to a field of 0.2 in eleven updates and slows
the mouse to half; Z again widens it back. `--zoom` presses Z after `--ticks` play and a quarter
of a second more, for a screenshot: `--skip-briefing --face m_targ.dat,150 --ticks 30 --zoom`.
C steps the outer camera round the unit and back into the cockpit; `--outer N` presses C N times
half a second apart after `--ticks` play (`--skip-briefing --ticks 60 --outer 2` draws Mission
01's hero from the right far place).

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
and `varset.var`: 448 dumps. From M14 it adds every map's `Land.map`.

## Stand-ins

Each stand-in is marked in the code as `// STAND-IN: docs/NN#section` and has
a row here. A row leaves this table when research closes it.

| milestone | what is unknown | stand-in | see |
|---|---|---|---|
| M1 | Whether the water surface is blended over the frame beneath it | opaque: the terrain draws every material opaque, whatever its blend | [03](../docs/03-terrain.md#not-established) |
| M1 | Whether `ForceSWFog` is read outside `Terrain.dll`, which asks Direct3D for linear range vertex fog and never reads it | per-pixel linear range fog on the distance to the eye, from 700 × slot 5 to 700 × slot 6 | [10](../docs/10-sky.md#not-resolved) |
| M1 | How the 34142-radius dome escapes the far plane and a fog ending by 700, and what lies below its rim | draw the dome first at the camera, depth-tested without writing depth under a projection with no far plane, unfogged but for its rim; clear the frame to the fog colour | [10](../docs/10-sky.md#the-dome) |
| M1 | Which camera axis the fog's heading angle measures: the compass heading, 0 at +y towards +x, of the camera matrix's first column | the view direction's heading, 0 along +y, turning towards +x, like the dome's segments | [10](../docs/10-sky.md#not-resolved) |
| M1 | Where the sun object's two directional lights point | both lights shine from the fixed place of the body that is up; none while no body is up | [10](../docs/10-sky.md#not-resolved) |
| M1 | The sky's textures: stars, clouds, the sun and moon sprites, the lens flare | not drawn | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M1 | The files' colours in a renderer that decodes textures to linear | sky, fog and dome colours and texture tints decoded from sRGB to linear; the lit colour (scene colour, material ambient and diffuse, both lights) formed from the files' values, held to 1, then decoded, so blends match the game's display-space ones | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M3 | What the draw layers 10 and 9 a fifth slot is filed under do (`Terrain.dll:0x1004553b`), and `CShade` slot 15 | the fifth slots draw with the scene, depth-tested, lit and fogged like any model | [07](../docs/07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws) |
| M3 | The frames the ground contact's points are placed at, the second sphere's radius r₂, and what lifts a sphere with no face under it | contacts on the step's last frames; r₂ = r; no lift | [24](../docs/24-motion.md#holding-the-body-on-the-ground--read-and-measured) |
| M3 | A state's use count `+0x94` | unlimited | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | The state a machine starts in | state 0 | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | The request code a controller holds before any is sent | none (−1): a state waiting for a code of its own does not apply until one is sent; no Mission 01 state has one | [32](../docs/32-builder.md#the-construction-sphere--read-and-measured) |
| M3 | The game's random source for a jittering step | xorshift | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | What vector a state with righting bits `0x30` stands the hull toward (control `+0x348`, no writer found) | none yet: the hull neither leans nor rights; to come, the ground contact's face normal, and world up for bits `0xC0` as read | [24](../docs/24-motion.md#the-hull-leans-and-rights-itself--read-and-measured) |
| M3 | Which way across a slope the mode-2 brake acts | uphill, against the averaged ground normal of the last landing | [24](../docs/24-motion.md#ground-and-slope--read) |
| M3 | How often `World3D.dll`'s input update runs, which paces the keypad cruise ramp | once a rendered frame, and once a 60 Hz tick where nothing is rendered | [24](../docs/24-motion.md#not-established) |
| M3 | A chord with no row of its own, such as Shift+W | the plain row | [14](../docs/14-controls.md#the-table) |
| M3 | How the camera builds its look-only frame when its up is parallel to the look (`0x10023769`) | any frame about the look; no shipped camera's pitch reaches it | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M4 | Whether a target the hero's AI set before the player took over survives | none: nothing sets it while the player drives, so the plasma bolt and the missile fly straight | [29](../docs/29-weapons.md#not-established) |
| M4 | Whether the landscape is one of the objects the sight ray (IWorld slot 7) walks; it skips no batch or triangle | the ground (less the water surface) and every live object's level-0 mesh, passing no triangle, as far as the map's diagonal and 200 m more | [29](../docs/29-weapons.md#not-established) |
| M14 | How a lobbed round's gun fitted as a part on a turret with no follower channel (the Small Bunker's) is raised | its round leaves on the lower arc from its muzzle to the point its turret traces, at its speed, and the gun is ready only while that point is within the arc's reach | [29](../docs/29-weapons.md#not-established) |
| M4 | How a gun's capacitor refills | full again every tick (the power tick is not modelled) | [23](../docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured) |
| M4 | Whether a round's ground test strikes the water surface | it passes through: the ground index holds no face with `Land.msh` surface bit `0x02`, so a round meets the bed | [26](../docs/26-damage.md#the-hit-test--read-and-measured) |
| M4 | The point-in-triangle test of the hit test (`0x10011090`), and the landscape's own cell size | an edge test on the triangle's winding; the ground index's 16 m cells | [26](../docs/26-damage.md#the-hit-test--read-and-measured) |
| M4 | Which node flag makes a node vital | the mesh node's `0x200` | [26](../docs/26-damage.md#hit-points--read-and-measured) |
| M4 | Whether vegetation and rock take damage | they stop rounds and take none | [04](../docs/04-missions.md#the-scale) |
| M4 | Poses of other units for the hit test | their rest poses: other units' animation is not played | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M4 | The rest of the emitter floats: what a particle's exponent-shaped triples are, a bolt's widths | sprites (3, 4, 9) move +40→+52 and grow +100→+112 straight by progress through the window, per-axis powers left out; a burst (7, 10) flies between velocities +44 and +56, spread +68, its age progress over +28, sized +92→+104; a stream (8) particle sits at +88→+100 and grows +136→+148 by its age in seconds, both in metres times the instance's scale, its frame turning them but not sizing them; a bolt's sprites are +24 wide, each spanning its texture's cell once where the texture repeats every +32 along the beam | [11](../docs/11-effects.md#not-resolved) |
| M4 | An effect's jitter (flag 1), the owner values of time modes 5–15, and a phase's animated texture frames | no jitter; modes 5–15 all read the owner's speed over its top speed, set on rounds; frame 0 of every texture | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M4 | How a sprite whose material says opaque blends | alpha-blended, so its fade shows | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M4 | How the shade lights with a type-1 light's range and attenuation; type 1 is a light in the owner's `CLightManager` | none yet; to come, Direct3D's fixed-function falloff, 1 / (a0 + a1·d + a2·d²) inside the range | [11](../docs/11-effects.md#not-resolved) |
| M4 | Which draw pass draws header-flag-0x800 effects | 0x800 effects draw with the rest | [11](../docs/11-effects.md#not-resolved) |
| M4 | When a stream emits its first particle | on its first update inside its window | [11](../docs/11-effects.md#bolts-streams-and-fades--read-and-measured) |
| M4 | How often an effect instance tests its point's view, and what the ray through the world meets | every frame, against what a round meets (the ground less its water surface, and every live object's level-0 mesh) | [11](../docs/11-effects.md#bit-8-and-the-tested-point--read-and-measured) |
| M4 | What a building (a `CBuilding` aggregating its agent) answers for a strike's material, and a node's wear base | a strike on a building plays slot 0; the batch's material byte alone indexes the wear | [11](../docs/11-effects.md#what-an-explosion-plays--read-and-measured) |
| M4 | The effect manager's random generator | any uniform generator | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M5 | How Direct3D Sound places a sound between the speakers | panned by its direction against the eye's right | [11](../docs/11-effects.md#not-resolved) |
| M5 | How the game turns a string's characters into the font's glyph indices | ASCII as its own index; Cyrillic by code page 866, where the font draws it (А–Я at 0x80, а–п at 0xA0, р–я at 0xE0); anything else draws `?` | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How tall a glyph is drawn and how far apart lines are: a record has no bottom edge | the atlas's row pitch, 18 pixels, for both | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How far the pen moves after a glyph | its advance: a recording of Mission 01's win measures it on the interface's menu font (each glyph of "MISSION COMPLETE !" starts its advance after the last, the space's 6 included), and the game font is taken to space the same way | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How the game draws its font: its 8-bit blend table and the text's colour | the atlas, sampled nearest, keyed on black and multiplied by the run's colour in display space, over everything after the HUD (a recording shows the interface font's grey shadow dark) | [12](../docs/12-rsli.md#what-is-inside) |
| M6 | How the loader turns a `varset.var` default into a value | an integer type reads hex after `0x`, a decimal, or a float truncated; a float reads its decimal; anything else is 0 (every shipped default reads either way) | [15](../docs/15-behaviour.md#the-vocabulary-varsetvar) |
| M6 | The formula parser and evaluator, beyond the operator table's priorities | parse by those priorities, left to right among equals, in doubles; `!` is 1 for 0, `&` and `\|` are logical, `N` holds to 0..1, `S` is the sign, `B` is 1 for non-zero, `A` the absolute value; an unknown name is refused (the shipped formulas use numbers, variables, `+ - *` and brackets) | [15](../docs/15-behaviour.md#a-statement) |
| M12 | Whether the character handler sees Esc before its binding leaves command mode: the path the key takes is not traced | an open satellite map closes first, then a page other than 0 turns to 0, then Esc leaves, as Mission 04's recording shows one Esc at a time | [40](../docs/40-command-mode.md#not-established) |
| M12 | What `0x10034230` accepts for an Upgrade row | no Upgrade row is offered | [41](../docs/41-commander.md#not-established) |
| M12 | A builder's beam's life, which the Build rows need intact | a live builder can build | [32](../docs/32-builder.md#building-a-building--read) |
| M12 | A unit's property `0x207` and a building's record `+0x30`, which pick and tint the panel's icons; the width of the piece a building row's icon stands in | a unit's second icon is the cell for 1; a building's icon red (2), as the recording shows; the building row's icon piece 20 wide | [41](../docs/41-commander.md#not-established) |
| M12 | The chat overlay and the game menu's screen (mode 7) | not built: their buttons are taken and do nothing | [41](../docs/41-commander.md#what-a-click-on-the-column-does) |
| M12 | The routine that names a building | strings 6031–6098 by Type, by the size letter of its root record (`fr_l_` small, `fr_m_` medium, `fr_b_` large) and a bunker's by its Type | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
| M12 | The commander's satellite map's title bar and exit icon, beyond their place | a page header's pieces 20 tall at (374, 43), the title 5074 centred | [35](../docs/35-hud.md#not-established-4) |
| M12 | Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule | a unit within 0.7 of its radius and a building within all of it, a sphere holding the eye passed over; the nearest centre along the ray wins | [42](../docs/42-selection.md#not-established) |
| M12 | Whether the display's slot 12 answers, so the system's cursor is used | the software cursor's four phases from `new_ui1` are drawn and the system's hidden | [42](../docs/42-selection.md#the-cursor-shows-a-state--read-and-measured) |
| M12 | A marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand | the gap is the unit's projected radius held to 4–44; the name over the left bracket, the class icon right of the right one, a blue box under the left one holding the life bar over a full battery bar | [25](../docs/25-sensors.md#how-the-game-colours-what-it-marks--read-and-measured) |
| M12 | What telepresence's auto-driver levels 1 and 2 give the AI | the player drives the unit whole at every level | [40](../docs/40-command-mode.md#telepresence-mode-2--read) |
| M6 | What a node naming a variable, operand, formula or handler that does not exist reads | the node does nothing; a switch to no handler ends the run; a run stops after a million nodes | [15](../docs/15-behaviour.md#how-a-handler-runs) |
| M12 | Function 15's answer: whether the unit takes the order (1) or refuses it (0) | an id some unit or building answers gives 1, and the unit is handed the order after the handler's run | [34](../docs/34-progression.md#what-the-scripts-ask--read-and-measured-1) |
| M12 | A building's contour (property `0x203`), whose vertices a patrol of the building walks | eight points on the building's sphere, pushed out by 30 | [31](../docs/31-packages.md#the-patrol-tick-by-tick--read) |
| M12 | An animal's migration over its clan's pastures, its default order | not modelled: an animal stands and asks its fire control for nothing | [31](../docs/31-packages.md#migrate-an-animals-pasture--read-and-measured) |
| M12 | Which of the fight module's bars a building's guns clear | the walker's, 0.85 | [29](../docs/29-weapons.md#how-the-ai-fires--read) |
| M12 | `IsPlacementValid`'s path search from the builder and its hall-way vertices' areal test; how the basement is triangulated between its rings | every site has a path and usable areals; each corner of either ring, against the nearest corner of the other, stands for a face falling along that line | [32](../docs/32-builder.md#the-test-isplacementvalid--read) |
| M12 | Which state each construction-sphere code opens, where and how big an action-5 effect is placed, what drives its time, and which classes the sphere's kill takes | code 1 starts the sign; code 2 the dome and the ray and stops the sign; code 0 stops the ray; each at the sphere's centre sized by its radius, looping on its duration (its records' time mode 0, a value set from outside, is not followed); the kill takes every live robot and the hero inside the sphere | [32](../docs/32-builder.md#the-construction-sphere--read-and-measured) |
| M12 | Whether the go task's 30 about its place is measured in three dimensions | across the ground | [31](../docs/31-packages.md#what-each-package-does--read) |
| M12 | Which controllers a building's control system gathers its batteries and efficiency from | its root record's: the 19.5 to 20 held and 50 to 52 a second put out that docs/23 measures, not the internal parts' (`i_pws_*`) | [23](../docs/23-economy.md#a-power-shortage-lowers-efficiency-once-the-batteries-run-down--read-and-measured) |
| M12 | The random sources of the distribution step's timer (`0..63` ms) and the power tick's jitter (a shift register) | a 32-bit xorshift | [23](../docs/23-economy.md#how-often-and-where-it-settles--read-with-a-derived-settle-point) |
| M12 | When a built mine's order 10 starts: derived to wait behind its construction sphere, while its draw is seen from the moment it appears | it draws its `Use_Power` from its making and digs from the sphere's end | [23](../docs/23-economy.md#a-mine-digs-to-500-and-then-a-draw-does-not-empty-it--read) |
| M12 | Where a transport a full storage turns aside goes: up to 100 random points within 30 of the unloading place, walked at a quarter of its speed | it waits where it stands | [32](../docs/32-builder.md#transporting-ore--read-and-measured) |
| M12 | The ore a mine's loading place or a storage's unloading place moves by itself (`0x10019482`, `0x100195b8`): its direction, property `0x208`, the tick's divisor | left out, as docs/23's model leaves it | [23](../docs/23-economy.md#not-established) |
| M12 | How a building made in play is drawn, and whether the landscape's drawing is cut under it | node by node from level 0 without a lightmap; the ground queries are cut under it, the landscape's drawing is not | [03](../docs/03-terrain.md#for-an-engine) |
| M13 | Which bound an HQ record's `+0x98` is, whose 8 × sets how far its command camera rides behind it | the chassis mesh's authored sphere's radius (8 × 7.653 = 61.2 m on Mission 04's HQ, which the recording favours), else the unit's whole bound | [40](../docs/40-command-mode.md#not-established) |
| M13 | What the game does when an HQ is lost in its own command view: nothing read rolls mode 3 back | the view rolls back to the HQ's cockpit, and the lost bot puts the hero out at (x − 1, y − 1) | [40](../docs/40-command-mode.md#not-established) |
| M13 | How the stack reads after Enter in telepresence aboard an HQ | the telepresence ends and mode 3 takes its place over the command view it came from | [40](../docs/40-command-mode.md#an-hqs-command-mode-mode-3--read-and-seen) |
| M13 | Whether the AI's aiming reaches the turret lock's lead (body `+0x38`) through the component setter, as the player's does | the lead starts from the turret's yaw target when the player takes the unit over | [30](../docs/30-turrets.md#the-hull-follows-the-turret--read-and-measured) |
| M13 | The spin a unit let go keeps until the Wizard writes one | none: letting a unit go clears its spin | [30](../docs/30-turrets.md#the-hull-follows-the-turret--read-and-measured) |
| M13 | Whether a research centre's ore property reads the ore it holds or what the distribution delivered | the take comes out of what the economy says it holds, and its request is what it asks the next step for, as a factory's build does | [16](../docs/16-research.md#not-established) |
| M13 | How often a building's takt runs | a research centre's every tick | [16](../docs/16-research.md#not-established) |
| M13 | The colour the research box's name is drawn in (`GAME_FONT`'s, as whatever drew before left it) | white, as the recording reads | [41](../docs/41-commander.md#not-established) |
| M13 | The research box's clip 5 inside its frame, applied to its preview | the preview's view is not clipped | [41](../docs/41-commander.md#what-it-draws--read) |
| M13 | What "a node reaching its last damage stage" takes out of the load (`0x10011920`) | a destroyed node's own weight and its armour; the devices on it stay | [24](../docs/24-motion.md#what-sets-the-live-limits--read) |
| M13 | How the walker's path joins a building's hall way (`MGraph`), and how it brings a unit to rest on a place in it | straight to the exit, or a hall-way vertex within 5, that makes the whole way to the pod shortest, then along the links; a way in stops on its last vertex rather than half its velocity beyond it | [31](../docs/31-packages.md#not-established) |
| M13 | What the walker does with the pod handed to it again while the unit stands there | within 1.5 of the pod, the go task's arrival at an object, it holds | [31](../docs/31-packages.md#each-tick-slot-7-0x10030300--read) |
| M13 | The building's own paths an escape is routed out by ("LEAVE IS TOO !!!") | a unit sent into a building along its hall way walks out of it along the hall way, from its nearest vertex to the exit that makes the way to its goal shortest, while it stands inside the building's outer ring | [31](../docs/31-packages.md#the-escape--read) |
| M13 | The unit takt escape's node test (a unit on a damaged node is left be) | every node counts as whole | [31](../docs/31-packages.md#the-escape--read) |
| M13 | The clan's contact list the maps mark other clans' objects by, and the scan's signatures | every live object strictly within the radar range of a live unit of the player's clan, the hero among them | [35](../docs/35-hud.md#the-panel-in-the-cockpit--read-and-seen) |
| M13 | The places besides a dock's and a main teleport's: whether a loading or unloading place's cylinder is what a transport's arrival reads | only a dock and the in and out places of a main teleport are ticked; a transport keeps its own arrival | [27](../docs/27-ownership.md#the-places--read-and-measured) |
| M13 | The random source of a place set's and a place's 64 ms share (`0x1004c550`) | a 32-bit xorshift per building | [27](../docs/27-ownership.md#who-stands-in-a-place--read) |
| M13 | Whether a destroyed generator stays in `World3D.dll`'s queue 3, which the in place's power walk reads | the live generators are asked | [27](../docs/27-ownership.md#teleport-in-0x8000--read) |
| M13 | Which of Mission 04's hero, helicopter and HQ holds no mind: the recording's factory shows one free of three once all three are the player's, and Mission 02's shows the hero holding one | a unit the hero's Enter took holds none | [34](../docs/34-progression.md#mission-04-teleport-end-to-end--derived) |
| M14 | The target point the guided lock projects (the target list's `+4`), and whether `getTimer`, which times its beeps, runs on a clock or on `timeGetTime` | the target's sphere centre, as the target panel's frame takes it; game time | [35](../docs/35-hud.md#the-guided-lock--read) |
| M14 | How an effect sprite's pre-lit vertices are coloured (draw flags 4, FVF `0x1e2`) | as a batch's emissive: the scene colour plus the material's ambient, held to 1 and decoded, times the texture; the laser draws red as the recording's does | [11](../docs/11-effects.md#not-resolved) |
| M14 | Which matrix `AniMesh` slot `0x10` hands the effect manager for its argument 2, which carries a beam's muzzle with its shooter | node 0's world pose: the unit's placement and its chassis's node 0 as drawn | [29](../docs/29-weapons.md#not-established) |
| M14 | How often the game frame runs, which paces a unit's zoom by 0.1 an update and the outer camera's ease | once a 60 Hz tick | [30](../docs/30-turrets.md#not-established) |
| M14 | Which objects and faces the outer camera's line through the world meets (mask `0x41a`, `0x208`), and the vector at `+8` of the world's answer 0.75 of which is added to the point met | the ground and every live target but the unit looked at, passing what a round passes; the camera stands 0.75 back toward the eye | [30](../docs/30-turrets.md#not-established) |
| M14 | Which box the unit record's `+0x98`, the outer camera's r, is the half-diagonal of | the chassis mesh's authored box (1.47 m on Mission 01's hero, which its recording favours), else the unit's collision radius; an HQ's command camera keeps its M13 stand-in | [40](../docs/40-command-mode.md#not-established) |
| M14 | Where a batch word's 8 and `0x100`, which file a batch translucent, come from; which sort type each of the render queue's layers has | no batch word makes a batch translucent, and a look is translucent when any of its phases has an ambient alpha below 1, decided once; groups draw mode by mode in their list, unsorted | [07](../docs/07-objects.md#what-a-blended-batch-writes-and-the-alpha-tests-reference--read) |
| M14 | With which depth state the beacon lights (flags 0x400 and 0x800, no bit-8 emitter) draw in the pass flag 0x800 waits for: by the read path their 9 m glow is depth-tested and cut by the faces it hangs on | every sprite of a flag-0x400 effect draws over the scene while its tested point is in view, as a bit-8 emitter's does | [11](../docs/11-effects.md#a-beacon-lights-glow--read-in-part-and-measured) |
| M14 | What silences the hero's breath in its own view: its effect is read to run, its loop inside its near distance, and the recording has none of it | the hero's own turret effects flagged `0x800` (`hero_breath`; `hero_helm_light` is a light) play no sound | [11](../docs/11-effects.md#how-a-sound-is-heard--read-and-measured) |
| M14 | How many tracks the install's `winmm.dll` reports for its `MUSIC` files | the highest `TrackNN.ogg` present; a track with no file is not audio | [34](../docs/34-progression.md#music-the-cds-tracks--read-and-measured) |
| M14 | When `AniMesh.dll` works an agent's sphere and node sphere out again (`0x10009510`), and at which pose | once, at the parts' and nodes' rest poses, as the unit is made | [24](../docs/24-motion.md#finding-the-ground--read) |
| M14 | How `CD_VOLUME` and `SFX_VOLUME`, each a share of a mixer control, become loudness | the music plays at the sounds' level, as the install's equal settings and Mission 01's recording have it | [34](../docs/34-progression.md#music-the-cds-tracks--read-and-measured) |
| M14 | What becomes of a part already knocked off and flying when its dock puts its node back | it is taken out of the air, rather than drawn beside the node it is again | [26](../docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured) |
| M14 | What the game drives a dock's glow with: its own time mode is 0, a value set from outside, and nothing read sets it | a charging dock runs it looping and switches it off again; the `f_recharge_*` nearest a dock's vertex is that dock's | [13](../docs/13-control.md#a-buildings-load-group--read-and-measured) |
| M14 | The masses property `0x7c` gives a unit and a static object, by whose squares a pair shares its push | no push is shared: every unit, not the hero alone, is run as the mover against every placed object and takes the whole push, which is what a building, a tree or a stone gives anyway | [24](../docs/24-motion.md#not-established) |
| M14 | Where a unit built in a factory joins the building's own paths, which the escape's 20-second check routes it out by ("LEAVE IS TOO !!!") | a bot made at a creation vertex counts as having walked in: it leaves along the hall way, as a unit sent in does | [31](../docs/31-packages.md#the-escape--read) |
| M14 | A building's draws beyond its efficiency: its shield, deflector and guns on its batteries | a building's shield takes the level its batteries serve its efficiency at, its shield's and deflector's draws come out of no battery, and its guns' capacitors are full again every tick | [23](../docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured) |
| M14 | When a unit's shield recharges against its power tick: the recharge is read on the power tick | the battery pays for the shield on the unit's power tick and the shield recharges every frame at the level that tick served | [26](../docs/26-damage.md#power--read) |
| M14 | The AI's repair decision (`Behavior.dll:0x10017c70`) and camouflage, and what sends an AI unit short of charge to a dock | a unit the player does not drive keeps its repair system and camouflage off, and nothing sends it to charge | [26](../docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured) |
| M14 | A unit's parts share one life system in the game, the engine keeps a life per part: how a part hanging on a knocked-off socket flies with it | a part whose socket is destroyed, or steps up a stage, has its node 0 destroyed where it stands, so its nodes explode and go there rather than in the air | [26](../docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured) |
| M14 | That a shield flash rides on node 0 | it keeps its direction and follows the bubble's centre | [26](../docs/26-damage.md#what-a-shield-hit-draws--read-and-measured) |
| M14 | Which of the effect frame's axes a type-9 dome's pole ends on, and how its texture runs over it | the first, so a shield flash bulges out of the bubble toward the hit, the round glow a recording shows; u around, v from rim to pole | [11](../docs/11-effects.md#not-resolved) |
| M14 | An animal's migrate: its pasture, its points and its timers | the animal stands; a hit is answered as a migrating animal answers it, about its clan's zone nearest it, and with none as one beyond its pasture | [31](../docs/31-packages.md#migrate-an-animals-pasture--read-and-measured) |
| M14 | How a turret component on a chassis's own controller poses its nodes against the chassis's frames, and how an animal's gun aims, its pitch channel having no point | the turret's channels pose the mesh as a turret part's would; an AI gun with no sight fires straight at the point its fire control traces | [34](../docs/34-progression.md#the-medusas--read-and-measured) |
| M14 | The call for help and the clan attitude a hit lowers | neither is modelled: a hit pulls in its victim alone, and relations stay as the mission gives them | [31](../docs/31-packages.md#a-hit-pulls-a-unit-in--read) |
| M14 | The behaviour's radar module: its two timers, and the hostile and friendly lists it keeps from its machine's radar | the machine's own radar scan stands in for the module, read afresh each takt; the fire control and the engagement pick from it alone, so a unit with no radar picks no target of its own, while a search still looks over the clan's areal map, which the engine keeps whole | [25](../docs/25-sensors.md#what-the-ai-does-with-it--read) |
| M14 | How a unit's place comes to be on a building's map object, and which of its vertices the global path starts or ends at | a unit standing on a bridge's faces takes the bridge's nearest hall-way vertex; a goal over a bridge is refused, as the areal under it is not walkable | [24](../docs/24-motion.md#not-established) |
| M14 | Who calls `MHallWay` slot 11, which links a building's exits to the areals under them and its flag-4 vertices to another's | only a live bridge's hall way joins the search; a way through any other building is left out | [24](../docs/24-motion.md#not-established) |
| M14 | The size gate a hall-way vertex puts on a unit (the unit's `+0x960`, the vertex record's `+0x28`) | every vertex passes | [24](../docs/24-motion.md#not-established) |
| M14 | How a walker goes to the point it finds off a non-walkable areal; what slot 14's `0x20000000`, which doubles the square, is | straight; the square never doubles | [24](../docs/24-motion.md#the-global-path--read-and-measured) |
| M14 | The walker's random source, `rand()`, which scales each link's cost | a 32-bit xorshift the play keeps | [24](../docs/24-motion.md#the-global-path--read-and-measured) |
| M14 | What the walker does when its search fails, out of links or past 2048 nodes | it holds, its queues emptied as `SetTarget` empties them before it searches | [24](../docs/24-motion.md#the-global-path--read-and-measured) |
| M14 | How the walker drops the points a unit has already passed (`MWalker::ClearMoverReachedPoint`) | a way's first vertex is left out of a new plan while the unit stands no farther from the next vertex than it does, so a follower planned again on a bridge goes on rather than back | [24](../docs/24-motion.md#not-established) |
| M14 | Which spot `SetTarget` accepts for Follow me beyond its areal: the path search is read to decide it too | the first of the 77 on a walkable areal (any, for a flyer); one the search then finds no way to is refused by the walker, and the unit holds | [31](../docs/31-packages.md#what-each-package-does--read) |
| M14 | What a roam with no usable point among its 150 does | it takes the last point tried, which the walker refuses | [31](../docs/31-packages.md#where-a-search-looks--read-and-measured) |

### Read since the stand-in was written

Research has closed these, and the code still carries the stand-in. The next
engine pass replaces each with what was read and removes its row.

| milestone | the code's stand-in | what is read | see |
|---|---|---|---|
| M3 | a state's contacts are parsed, but their `0x100`/`0x200` conditions are taken as met (node life is not modelled) | `0x100` makes a state need its contact point's node intact and `0x200` destroyed, a walker's limping states | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | the ground face is found fresh each step, by an up and a down pass over its cell | the walk starts in the held face and crosses at most 24 faces toward the centre, stopping on one too steep | [24](../docs/24-motion.md#finding-the-ground--read) |
| M3 | D is the largest span of the velocity box's switched-on axes | the largest difference between an axis's absolute max and absolute min, over all three velocity axes; D = 0 leaves the weight at 1 (no shipped state's weight changes) | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | the invert constants (−1, +1) with an integrator that does not negate, and free look's yaw negated to undo the mirrored X; a turret's X row takes +1 | the game's mouse X invert is +1 and its integrator negates; on screen the two agree: mouse right turns right, mouse down lowers the sight, and Shift free look's vertical runs opposite | [14](../docs/14-controls.md#from-a-row-to-a-command--read-and-measured) |
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
| M7 | The class the small-face stop reads | the hero's class is its size class, 2, so small faces never stop it | [24](../docs/24-motion.md#collision-between-objects--read) |
| M7 | The batch flags 8 and 0x200 a collision's face query passes, and the batch word's 2 that makes a hider hide both ways: a mesh's batch record carries no such word | no batch passes, and every hider hides from its front only | [24](../docs/24-motion.md#collision-between-objects--read) |
| M11 | Who sets a collision object's flags, so which movers carry 8 and keep the floors (triangle flag 2) in their push-out | no mover carries 8: every floor lets a mover by | [24](../docs/24-motion.md#not-established) |
| M8 | Whether a digit the wingman selector takes also reaches the input table that toggles the hero's guns | it does not | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | What a unit record's `+0x30` is, which picks capturers and the acknowledgement voices | the size class of the chassis's name: t 1, l and h 2, m 3, b 4 | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | The acknowledgement's xorshift: its shifts and seed | 7, 9, 8 from 0xACE1, never repeating the last voice | [31](../docs/31-packages.md#the-wingman-menu-from-first-person--read-and-measured) |
| M8 | The game's random source for a life update's spread | a 16-bit xorshift (7, 9, 8), seeded apart per unit | [24](../docs/24-motion.md#water-and-lava-beds-kill--read-and-measured) |
| M8 | The shell's menus after a mission, the load-game screen, and `MISSIONS/dispatcher.ini` | Esc after the outcome closes the window, L does nothing, and a win is not written to the install | [34](../docs/34-progression.md#after-the-outcome--read-and-measured) |
| M8 | A material's start stamp, from which its track is played, and the random a mode-3 track jumps by | the world clock's 0 for a mesh batch's material, and a sprite's own start for an effect's — a stream's particle from when it left; a hash of the clock | [07](../docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured) |
| M8 | The local path and its obstacle contours; the sub-areals' shapes and centres; how a walker leaves a hole and what it does with a goal in one; the Wizard's heading curve | a leg of the global path that would leave the walkable areals, as a leg across an areal that is not convex can or one across a tree's or a stone's footprint, walks through that areal's triangles pulled straight, the unit's collision radius off every vertex that touches ground that is not walkable; the footprints' edges are cut into the triangles and what lies inside dropped, each areal's triangles that still hang together are a node, measured from the areal's record centre while it is one piece and from its triangles' centre once it is cut apart, and a waypoint on an edge of a cut areal stands a fifth of the edge in from an end; a goal inside a footprint is moved to the nearest ground outside it, and a walker inside one sets out from the nearest; a walker's legs are measured and timed across the ground, its points holding no height, so a hall-way vertex standing over a deck does not slow it; the heading is the curve's velocity's direction | [24](../docs/24-motion.md#not-established) |
| M8 | How a velocity the Wizard writes combines with the velocity integrator, and whether its spin is a rate or a fraction | a driven machine takes the written velocity as its own and turns toward the heading at up to its live yaw rate; since that replaces the machine's own velocity every step, where the mode-2 brake would act the ground's fraction is taken off the written velocity whole, rather than pulled at over several steps as it is on the player's | [24](../docs/24-motion.md#not-established) |
| M8 | The height a flyer's points are given; who reads `Movement_FlyHeight` | at least `FlyNearLandHeight`, 15, above the ground under the point | [24](../docs/24-motion.md#not-established) |
| M8 | How an engagement scores the radar's contacts through the task (the contact record's three unnamed fields); follow's and refit's priorities for one | the nearest hostile unit within 500 is the best, and for a patrol the one nearest its centre inside its radius; follow and refit answer 0; an attack running is not given another | [31](../docs/31-packages.md#between-orders--read) |
| M14 | Each task's priority for a refit, reason 3, beyond the route's, the patrol's and the capture's | every task that moves or fights takes the base's, which lets one through while a dock is reachable; standby, shutdown, the escape and a refit already running answer 0 | [31](../docs/31-packages.md#between-orders--read) |
| M8 | The follower's two timers; the behaviour's random source | it measures once a second; a 32-bit xorshift | [31](../docs/31-packages.md#what-each-package-does--read) |
| M8 | A capture's retreat, read to lie off the map | a plan with nowhere to go roams | [31](../docs/31-packages.md#where-a-search-looks--read-and-measured) |
| M14 | Which dock a refit picks (`0x10023b60`), and when its walk is over | the nearest dock the unit's size fits — any for size class 1 or 2, a ground-level one alone above that, the size rule `MakeInsideDest` routes a unit inside by — walked to along that building's hall way, and it is there once it stands in the place itself, the cylinder that charges it. The game's own refit asks `MakeInsideDest` for the ground-level bit on every dock, so it would never send even a small bot indoors | [27](../docs/27-ownership.md#what-sends-a-bot-to-a-dock--read) |
| M14 | Which difficulty profile a unit's behaviour holds (`+0x8d4`), whose `Decision_RepairOn` and `Decision_RepairOff` the repair decision reads | `diff_strong.var`'s 0.8 and 0.9, so a unit repairs itself while it is only lightly damaged | [26](../docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured) |
| M14 | A building's own repair decision and its repair system | a building never switches one on: only a unit's is modelled | [26](../docs/26-damage.md#repair-a-units-own-repair-unit-switched-on-and-off--read-and-measured) |
| M8 | How the turret turns a traced point into its targets (`0x10028bb0`), its aim stage, and the gun's report codes | each channel moves on by the angle the sight is off, at the rate a small nudge turns it; θ is 0 once both channels reach their targets and π before; a guided gun's θ is its lock left × π | [29](../docs/29-weapons.md#how-the-ai-fires--read) |
| M9 | Where a gun's takt stores its report codes 0 and 3–6, which the weapons list's lamp reads | the code from the gun's state now: 5 no rounds, 6 short of charge, 7 not ready, 3 stroking, 4 waiting its interval; then the gate's 2, 7 or 8; 1 locking; 0 ready | [29](../docs/29-weapons.md#the-guns-takt-a-stroke-then-the-interval) |
| M9 | The charge level of a gun with no capacity, which only a shot sets | its bar shows full | [29](../docs/29-weapons.md#a-gun-is-a-capacitor-a-magazine-and-a-clock--read) |
| M9 | Which caller hands a unit's name its class word, and which robots are *"Tiny Tower"* | each class letter its own word (W *Warrior*, T *Transport*, B *Builder*, C *Comm. Center*); no robot is a Tiny Tower | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
| M9 | The component value `0x400` the *"Dangerous!"* line asks for | no unit is called dangerous | [35](../docs/35-hud.md#name-and-status--read-and-seen) |
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
| M10 | Whether a cull mode changes for the mirrored reflection frame | the faces that face the mirrored eye draw, as a mirror shows them | [03](../docs/03-terrain.md#not-established) |
| M10 | What the reflection camera's pass flags `0x120` leave out | the effects' sprites; the dome, the ground less its water and beds, and every shown object draw | [03](../docs/03-terrain.md#not-established) |
| M10 | How far the water's bump map displaces its lookup | a signed byte stands for −1 to 1 at 127, so the largest offset is 0.01 × 64 ÷ 127 of the box | [03](../docs/03-terrain.md#not-established) |
| M10 | What a device's byte 0 of 1 adds from the machine's list at `+0xc4` | nothing: its channels hold their initial values (no Mission 01 unit has one) | [28](../docs/28-chassis.md#not-established) |
| M11 | What `CBuilding` does to a door's or a computer's switch word as it files the item (the records leave the constructor's 5, which wraps for ever) | a door and a pod start shut, word 0 and progress 0 | [24](../docs/24-motion.md#walking-into-a-building--read-and-measured) |
| M11 | The capsule a door's part is measured against (`Terrain.dll:0x1005a27f`) | the door node's level-0 slot sphere; the holds are worked out from every child each tick rather than on each child's move | [24](../docs/24-motion.md#walking-into-a-building--read-and-measured) |
| M11 | The fill colour the resource rows hand their bar | the weapons list's: red under 20%, olive under 80%, green above | [36](../docs/36-factory.md#the-resource-rows) |
| M11 | How the cursor is shown in view mode 5 | the system's cursor, with the grab let go | [36](../docs/36-factory.md#not-established) |
| M14 | How the basement is triangulated between its two rings, and which of its faces the first builder (`0x1000bdb0`) makes with the ground's own texture pair rather than the foundation | the two rings are walked together and stitched, and every face of the band is the foundation, layer-1 slot 0 with no second layer, as the second builder (`0x1000d9c4`) writes it; the contour is sampled onto the ground every 4 units instead of at each landscape face it crosses, and the band runs 2 units past the contour, sunk 0.5 under the ground there, so the cut's own texel edge shows no sliver of sky | [03](../docs/03-terrain.md#for-an-engine) |
| M11 | Where a gathered face's batch word, whose 8 and 0x200 the collision query passes, comes from | the faces of the `DEFAULT`, `PORTAL_001` and `PORTAL_004` materials, a building's doorway and portal quads, let a mover **and a round** through, as a recording shows and as a shot at a door needs | [24](../docs/24-motion.md#the-doorways-are-portal-quads--measured-read-and-seen) |
| M14 | How a portal face reaches `CBuilding::PortalDrawNotify` and which node it names, and so which cells a building draws | no cell is culled: every node is drawn, and the portal quads themselves are dropped, which is what the cells would have hidden them behind | [24](../docs/24-motion.md#a-building-is-drawn-cell-by-cell-through-its-portals--read) |
| M14 | Where the node matrix an action-3 effect takes as its frame comes from, whose translation is the node's authored origin | an action-3 effect stands at the centre of the node's level-0 bounding sphere, which puts all 112 door sounds on their doors | [13](../docs/13-control.md#a-buildings-load-group--read-and-measured) |
| M11 | Whether the slope brake reads a building's stair faces | on a building's faces the slope brake is left out | [24](../docs/24-motion.md#the-ground-inside-a-building--read-in-part-and-measured) |
| M11 | Which of a turret's nodes the boarding test's property `0x52` reads the life of | the turret part's node 0 | [39](../docs/39-boarding.md#boarding--read) |
| M11 | The heading the hero is given on leaving a bot, read as (F.x, −F.y) under an assumed matrix layout | the hero faces the bot | [39](../docs/39-boarding.md#not-established) |
| M11 | The name a bot's gun takes in the weapons list (`0x1008a470`, not followed) | the gun part's code in the player clan's research tree, or NONAME | [35](../docs/35-hud.md#the-weapons-list--read-and-measured) |
| M11 | That the factory record's `+0x30`, the grade the constructor's chassis page is taken over, is the building's size class | the designer is given the factory's size class | [38](../docs/38-designs.md#not-established) |
| M11 | What the turret fit does to guns on a turret it replaces, and the gun fit to a clip on a gun it replaces | the old turret's guns and clips, and the old gun's clip, go with it | [38](../docs/38-designs.md#not-established) |
| M11 | The part box's `Epower`, the properties behind `regener`, `capacity`, `throughput`, `shotnum` and `blast`, and the formatter that prints one decimal whatever the template asks | `Epower` and `Adfactor` print 0.0; the others are the record values the recording's figures fit (a shield's second value, a repair unit's first, a battery's first ÷ 1000 and its power figure, a magazine, the round's first area blast); every number one decimal | [38](../docs/38-designs.md#not-established) |
| M11 | Which destination row a designer tab selects as it turns on and is left on after a fit, and which tab the panels turn to after one | the first row, and after a fit the next row, wrapping round; *seen*: a chassis turns them to Turrets and a turret to Weapons | [37](../docs/37-designer.md#not-established) |
| M11 | The condition under which fitting a chassis enables the Armour tab (`0x10052491`) | when the chassis has an armour slot | [37](../docs/37-designer.md#not-established) |
| M11 | The designer's save name field and load list | not built: save and load do nothing | [37](../docs/37-designer.md#not-established) |
| M11 | Which way a model view's camera looks, which axis its −0.5 rad pitch turns about, which of the view's sides its 60° field spans, and its two lights' colours | from −y, about x, the narrower side, about the drawn level-0 vertices' sphere; lights grey 0.4 and a scene colour of 0.15 | [37](../docs/37-designer.md#the-previews--read-and-seen) |
| M11 | How a scan band's green specular lights its strip | an added colour of (⅔g, g, ⅔g) over the strip in `0xff009b00` | [37](../docs/37-designer.md#the-scan-bands--read-and-seen) |
| M11 | Which cell of its material an effect sprite samples, and when the material's track plays for a particle | the entry's cell, as a mesh batch takes it, from the key the track is on at the sprite's own age — a stream's particle counting from when it left, everything else from its instance's start; the masked colour lerp between two keys is not drawn | [07](../docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured) |
| M8 | A gun fitted as a part of its own: its ready byte and its barrels' recoil | ready; its recoil is not played | [29](../docs/29-weapons.md#a-gun-is-ready-once-its-arm-is-out--read-and-measured) |
| M8 | Which difficulty profile a wingman holds: `Speed_MaximumFactor` | 1, as four of the five profiles set it | [26](../docs/26-damage.md#the-difficulty-ratio--read-and-measured) |
| M8 | How a knocked-off part flies: its push, its spin, its update, and the world query that ends a flight early | it drops off at 2 m/s away from the unit's centre, falls under gravity turning at 3 rad/s about a level axis across its path, and its flight ends when its sphere meets the ground (the player remembers a part falling about half a second): a dummy's side panel goes 1.3 s after it is knocked off | [26](../docs/26-damage.md#not-established) |
| M8 | A unit's life system holds all its models' nodes under one root | a life per part: a dead unit destroys its other parts' roots, and a part's root, having no parent, is never knocked off | [26](../docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured) |
| M8 | The statuses 4 and 8, a node copying its parent's life fraction or its stage | not modelled; no Mission 01 node carries them | [26](../docs/26-damage.md#what-a-damaged-node-a-destroyed-part-and-a-dead-unit-draw--read-and-measured) |
| M3 | the ground contact runs after every state step, with the step as its dt | once a frame on message `0x1c`, after the collision pass and before a push is taken, with the frame's dt. Held back behind the held-face walk above: a fresh search each frame loses a bed too steep to walk (a hero on Tut_1's lake bed no longer dies) and lifts a hero through stacked floors (the Small Generator's way stops at vertex 7). Meanwhile a sinking door's downward pushes add up between steps: the hero sinks 1.2 m at the Small Bunker's door | [24](../docs/24-motion.md#collision-between-objects--read) |

## Departures

A departure is where the engine does something the game is read *not* to do,
for comfort. Each is marked in the code as `// DEPARTURE: docs/NN#section`,
has a row here, and a switch that restores the game's behaviour.

| what the game does | what the engine does | switch | see |
|---|---|---|---|
| The hero's body node yaws with the gait, ±10° once a run cycle, and the turret, eye, sight and barrels swing with it | node 0 keeps only the part of its turn not about its up axis, so the view, the sight and the barrels hold the heading the body moves along | `--sway` | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| A building's guns fire on whatever its fire control traces, however far below its turret the target stands, so a bunker's lobbed flames reach a unit at its door or inside it | a building's guns hold their fire on a target lower than its turret's pitch channel lets its sight look — −15° on the Small Bunker's, about 40 m out from it — while its turret keeps tracing it | `--fire-below` | [29](../docs/29-weapons.md#how-the-ai-fires--read) |
| A capture changes only the unit's clan, SuperAI and areal map, and gives it no order, so a captured bot engages a hostile within 500 on its own | a captured bot is given Standby, and holds until the player orders it | `--capture-idle` | [27](../docs/27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured) |
| The HUD's 640 × 480 layout and the outcome panel scale by the screen's width over 640 across and its height over 480 down, so on a wide screen they stretch | the layout scales by the height alone and each element keeps its pin to the screen's edges: the panels in the bottom corners, the weapons at the top right, the radar at the bottom middle, the reticle and the wingman menu's rows in the middle; the warbot designer keeps its shape centred, over a black ground across the window | `--stretch-hud` | [35](../docs/35-hud.md#how-the-radar-draws--read) |
