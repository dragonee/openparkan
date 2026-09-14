# engine

A Rust engine for *Parkan: Iron Strategy* that reads the game's own install.
Its first target is Phase One: load Mission 01, *Line of Fire*, put the hero
at its start, and walk, look and shoot in first person.

Milestones **M0** to **M5** are in, each with the stand-ins listed below:

- the workspace;
- the NRes, mission, `Texm`, `Material.lib`, wear, `Land.msh`, object mesh,
  `objects.rlb`, `.dat`, controller (`.ctl`), input table (`.tbl`), control
  point (`.cpt`), damage table (`.ndp`), explosion (`.exp`), effect (`FXID`)
  atmosphere (`sky.ske`), RsLi archive and game font (`gamefont.rlb`) readers;
- the golden cross-check;
- a window over Mission 01.

The ground is drawn with its two material layers blended and the file's own
mip levels. Every placed object is drawn from its assembly:

- each part is mounted on its host's socket;
- each node sits at its rest pose;
- level 0 is drawn; a node's fifth slot, its cockpit, only in the hero's own view;
- materials draw in the blend mode their flags byte names.

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
- the lit colour, the scene colour and the material's emissive and diffuse
  under both lights, is formed in the files' display space, held to 1, and
  decoded.

Sound plays each effect's sound emitters from `sounds.lib`, WAV and MS ADPCM
through kira, as their effect time passes their trigger. A HUD shows a
crosshair and the guns: which are selected, their magazines and capacitors.

Not yet: lightmaps (no Mission 01 mesh has one) and levels of detail beyond 0.
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
mouse looks around. A click grabs the mouse; Escape lets it go, and quits once
it is free. `--ticks N`, `--hold` (scan names) and `--mouse DX,DY` (counts a
tick) play the hero at 60 ticks a second before a screenshot, or with
`--headless` print where it got to. In the window `--hold` keeps its keys down,
`--mouse` adds its counts every tick and `--trace` prints where the hero is
every second. The view holds the heading the hero moves along; `--sway` lets it
swing with the gait as the game's does. With `--fly`, W/A/S/D and Q/E fly, holding
the right mouse button turns and Shift flies faster.

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
input tables; from M4 every damage table, explosion and effect; from M5 every
mission's atmosphere, both RsLi archives and the game font: 215 dumps.

## Stand-ins

Each stand-in is marked in the code as `// STAND-IN: docs/NN#section` and has
a row here. A row leaves this table when research closes it.

| milestone | what is unknown | stand-in | see |
|---|---|---|---|
| M1 | Whether water is drawn see-through: `WATER`'s material says opaque, and every lake has a `WATER_BOT` bed beneath | opaque: the terrain draws every material opaque, whatever its blend | [03](../docs/03-terrain.md#terrain-layers-name-materials-not-textures) |
| M2 | Whether a blended material writes depth, and the alpha test's reference value | blended groups draw after opaque ones without writing depth; nothing is discarded (reference 0) | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M1 | Whether `ForceSWFog` is read outside `Terrain.dll`, which asks Direct3D for linear range vertex fog and never reads it | per-pixel linear range fog on the distance to the eye, from 700 × slot 5 to 700 × slot 6 | [10](../docs/10-sky.md#not-resolved) |
| M1 | How the 34142-radius dome escapes the far plane and a fog ending by 700, and what lies below its rim | draw the dome first at the camera, depth-tested without writing depth under a projection with no far plane, unfogged but for its rim; clear the frame to the fog colour | [10](../docs/10-sky.md#the-dome) |
| M1 | Which camera axis the fog's heading angle measures: the compass heading, 0 at +y towards +x, of the camera matrix's first column | the view direction's heading, 0 along +y, turning towards +x, like the dome's segments | [10](../docs/10-sky.md#not-resolved) |
| M1 | Where the sun object's two directional lights point | both lights shine from the fixed place of the body that is up; none while no body is up | [10](../docs/10-sky.md#not-resolved) |
| M1 | The sky's textures: stars, clouds, the sun and moon sprites, the lens flare | not drawn | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M1 | The files' colours in a renderer that decodes textures to linear | sky, fog and dome colours and texture tints decoded from sRGB to linear; the lit colour (scene colour, material emissive and diffuse, both lights) formed from the files' values, held to 1, then decoded, so blends match the game's display-space ones | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M3 | What the draw layers 10 and 9 a fifth slot is filed under do (`Terrain.dll:0x1004553b`), and `CShade` slot 15 | the fifth slots draw with the scene, depth-tested, lit and fogged like any model | [07](../docs/07-objects.md#the-fifth-slot-is-what-the-units-own-view-draws) |
| M3 | How an object's interface `0x25` slot 3 turns its level-0 triangles into a push on a sphere; the rest of the pair response is read | none yet: units walk through each other and through buildings; to come, push the sphere out along the nearest triangle's normal by its penetration | [24](../docs/24-motion.md#collision-between-objects--read) |
| M3 | Which scene objects the walk-face query visits (types 1 and 3), so which bridges and buildings are ground | the landscape alone is ground: bridges and buildings are not | [24](../docs/24-motion.md#not-established) |
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
| M4 | What a dead unit leaves: its wreck and damage stages | its destroyed nodes' explosions play and it is no longer drawn (a building stays as a shell, as read) | [26](../docs/26-damage.md#hit-points--read-and-measured) |
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
| M5 | How the HUD draws the aim point and the guns | a crosshair at the centre; a slot a gun, lit while selected, with magazine and capacitor bars | [30](../docs/30-turrets.md#not-established) |
| M5 | How a sound falls off between its near and far distances, and how it is panned | linear in distance; panned by its direction against the eye's right | [11](../docs/11-effects.md#emitter-types--read-and-measured) |
| M5 | How the game turns a string's characters into the font's glyph indices | ASCII as its own index; Cyrillic by code page 866, where the font draws it (А–Я at 0x80, а–п at 0xA0, р–я at 0xE0); anything else draws `?` | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How tall a glyph is drawn and how far apart lines are: a record has no bottom edge | the atlas's row pitch, 18 pixels, for both | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How far the pen moves after a glyph: a record's advance is one less than its span, so `l` advances 1 | the advance + 1, the span, on every record; a space, a placeholder, moves 9 | [12](../docs/12-rsli.md#what-is-inside) |
| M5 | How the game draws its font: its 8-bit blend table and the text's colour | the atlas's brightness as coverage, sampled nearest, tinted by the run's colour and alpha-blended over everything after the HUD | [12](../docs/12-rsli.md#what-is-inside) |

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

## Departures

A departure is where the engine does something the game is read *not* to do,
for comfort. Each is marked in the code as `// DEPARTURE: docs/NN#section`,
has a row here, and a switch that restores the game's behaviour.

| what the game does | what the engine does | switch | see |
|---|---|---|---|
| The hero's body node yaws with the gait, ±10° once a run cycle, and the turret, eye, sight and barrels swing with it | node 0 keeps only the part of its turn not about its up axis, so the view, the sight and the barrels hold the heading the body moves along | `--sway` | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
