# engine

A Rust engine for *Parkan: Iron Strategy* that reads the game's own install.
Its first target is Phase One: load Mission 01, *Line of Fire*, put the hero
at its start, and walk, look and shoot in first person.

Milestones **M0** to **M5** are in, each with the stand-ins listed below:

- the workspace;
- the NRes, mission, `Texm`, `Material.lib`, wear, `Land.msh`, object mesh,
  `objects.rlb`, `.dat`, controller (`.ctl`), input table (`.tbl`), control
  point (`.cpt`), damage table (`.ndp`), explosion (`.exp`), effect (`FXID`)
  and atmosphere (`sky.ske`) readers;
- the golden cross-check;
- a window over Mission 01.

The ground is drawn with its two material layers blended and the file's own
mip levels. Every placed object is drawn from its assembly:

- each part is mounted on its host's socket;
- each node sits at its rest pose;
- level 0 is drawn, and collision hulls never;
- materials draw in the blend mode their flags byte names.

**M3.** The window opens in the hero's cockpit on Mission 01:

- `parkan-sim` plays the chassis controller's states on their own clock. The
  live limits and the velocity and pending-turn integrators run once a state
  step. The body moves by its velocity or by the animation's root stride.
- The planner scales the file's transition costs as the loader does, so the
  run cycle keeps running forward.
- `hero.tbl` drives it through the game's mouse filter.
- The turret's pitch channel tilts the sight. The eye stands at
  `CameraCenter` and looks along `TargetDirect` with a horizontal field of
  view of 1.3 rad.

On Tut_1 the hero holding W runs at 14 m/s. Not drawn yet: the hero itself
and its animation, when seen from outside.

**M4.** The hero fights.

- Its turret's four guns keep the game's clock: a four-step barrel stroke,
  then the interval.
- The fire and number keys reach the selected guns. The cannon and the laser
  start selected.
- A round leaves its muzzle aimed at what the sight meets, and flies with the
  shooter's velocity. Its side speed bleeds off, and its range runs out.
- Each frame a round's segment is tested against the ground, the map box and
  every live object's level-0 triangles.
- A hit does the round's `.exp` damage to the node struck, or blasts every
  node in reach, less armour. A dead node takes its children with it, and
  node 0 takes the object.
- On Mission 01 the laser kills a target in two hits of 250.
- Effects play from `effects.rlb`: each emitter inside its window of effect
  time. The turret's flashes hang on the barrel points, timed by the barrel
  channels; rounds carry their tracers and bolts. A strike plays its `.exp`
  by the surface it met, and a destroyed node plays its own. Sprites and
  particle bursts are drawn in their materials' blend modes.

Not yet: lights, particle streams, animated textures, shields, and what a
dead object leaves behind.

**M5.** The sky is the mission's `sky.ske`, interpolated on its clock:

- the dome around the camera takes its apex, rings and horizon colours;
- linear range fog runs from the eye to 700 × slot 6, in the horizon colour
  of the heading, additive materials fogging to black;
- the scene colour adds to every material's emissive, and the sun or the
  moon lights the scene.

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
every second. With `--fly`, W/A/S/D and Q/E fly, holding
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
mission's atmosphere: 212 dumps.

## Stand-ins

Each stand-in is marked in the code as `// STAND-IN: docs/NN#section` and has
a row here. A row leaves this table when research closes it.

| milestone | what is unknown | stand-in | see |
|---|---|---|---|
| M1 | Whether water is drawn see-through: `WATER`'s material says opaque, and every lake has a `WATER_BOT` bed beneath | opaque, as the material says | [03](../docs/03-terrain.md#terrain-layers-name-materials-not-textures) |
| M2 | Whether a blended material writes depth, and the alpha test's reference value | blended groups draw after opaque ones without writing depth; nothing is discarded (reference 0) | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M1 | Whether `ForceSWFog` is read outside `Terrain.dll`, which asks Direct3D for linear range vertex fog and never reads it | per-pixel linear range fog, 0 to 700 × slot 6 (the two look the same) | [10](../docs/10-sky.md#not-resolved) |
| M1 | How the 34142-radius dome escapes the far plane and a fog ending by 700, and what lies below its rim | draw the dome first at the camera, with no depth, unfogged but for its rim; clear the frame to the fog colour | [10](../docs/10-sky.md#the-dome) |
| M1 | Which camera axis the fog's heading angle measures: the compass heading, 0 at +y towards +x, of the camera matrix's first column | the view direction's heading, 0 along +y, turning towards +x, like the dome's segments | [10](../docs/10-sky.md#not-resolved) |
| M1 | Where the sun object's two directional lights point | a directional light along the sun's fixed direction while its keyframes have it up (the moon's otherwise), coloured by the sky keyframes' slot 19 × the third float, held to 1; a lit colour is held to 1 as fixed-function lighting holds it | [10](../docs/10-sky.md#not-resolved) |
| M1 | The sky's textures: stars, clouds, the sun and moon sprites, the lens flare | not drawn | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M1 | The files' colours in a renderer that decodes textures to linear | sky, fog, scene and light colours decoded from sRGB to linear, so blends match the game's display-space ones | [10](../docs/10-sky.md#the-dome-the-fog-and-the-scene-colour--read-and-measured) |
| M3 | How a body is put back on its ground point, and whether modes 0 and 2 fall | after each step the model's lowest point is set on the highest walkable face within the contact radius above it, as a mission places units | [24](../docs/24-motion.md#not-established) |
| M3 | Collision between objects: shapes and response | none yet: units walk through each other and through buildings | [24](../docs/24-motion.md#not-established) |
| M3 | Walls, the map edge, bridges, jumping | a step that ends over a face steeper than 80°, or off the ground mesh, is undone and the body stops; bridges and buildings are not ground | [24](../docs/24-motion.md#not-established) |
| M3 | The mesh walk inside `FindWorldFace`, and the two query passes | the face under the point, found fresh each step; the water surface is never ground | [24](../docs/24-motion.md#finding-the-ground--read) |
| M4 | Which way the gap to the liquid surface is measured over a bed | `water_level − centre.z < r` (bed damage arrives with damage) | [24](../docs/24-motion.md#not-established) |
| M3 | The divisor D in a blended state's weight | the largest span of the velocity box's switched-on axes | [24](../docs/24-motion.md#not-established) |
| M3 | A state's use count `+0x94` | unlimited | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | Whether a state's step velocity replaces the integrated one | it does not; the boxes test the integrated velocity | [24](../docs/24-motion.md#not-established) |
| M3 | The state a machine starts in, and what plays when nothing is queued | state 0; the current state plays again | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | The game's random source for a jittering step | xorshift | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | The node range the payload counts as the chassis | none: spare payload is the whole payload, r = 1 | [24](../docs/24-motion.md#load--read-and-measured) |
| M3 | What triples 5 and 6 do to the attitude | only the turn about z is applied | [24](../docs/24-motion.md#not-established) |
| M3 | Which way across a slope the mode-2 brake acts | uphill, against the face normal | [24](../docs/24-motion.md#ground-and-slope--read) |
| M3 | The signs `SetInverseMotion` starts with, and the free look's senses on screen | invert (−1, +1): mouse right turns the hull right, mouse up tilts the sight up; free look yaw follows the hull | [14](../docs/14-controls.md#from-a-row-to-a-command--read-and-measured) |
| M3 | Which camera point gives the position and which the direction | position from `CameraCenter`, direction from `TargetDirect`, up the look node's +z | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M3 | How the strafe angle's turn splits between hull and turret, and at what rate | the legs turn at once and the turret holds its heading; the angle follows the keys held, mirrored while backing up | [24](../docs/24-motion.md#not-established) |
| M3 | What the keypad cruise's ramp does | ramp rows do nothing | [24](../docs/24-motion.md#not-established) |
| M3 | A chord with no row of its own, such as Shift+W | the plain row | [14](../docs/14-controls.md#the-table) |
| M3 | The mouse sensitivity handed to `World3D.dll` | `MOUSE_SENS` × 0.01 from `Iron_3D.ini` | [14](../docs/14-controls.md#from-a-row-to-a-command--read-and-measured) |
| M3 | How a follower channel (flag 8) aims its gun's mount | it steps toward the pitch channel's value at its own rate; flag 0x40 copies the previous channel | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M3 | Whether the player's own unit is drawn in first person | it is not | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M4 | What sets the hero's turret target in first-person play | none: the plasma bolt and the missile fly straight | [29](../docs/29-weapons.md#not-established) |
| M4 | How the turret's follower channels set a gun's ready byte | always ready | [29](../docs/29-weapons.md#not-established) |
| M4 | What geometry the sight ray (IWorld slot 7) meets | the ground (less the water surface) and every live object's level-0 mesh, as the hit test does, out to the map's far corner | [29](../docs/29-weapons.md#not-established) |
| M4 | What the class-24 arm states do | selecting a gun unfolds its arm toward frame 48 at the arm channel's rate, deselecting folds it; guns selected at the start begin unfolded | [29](../docs/29-weapons.md#the-button-reaches-the-selected-guns) |
| M4 | How a gun's capacitor refills | full again every tick (the power tick is not modelled) | [23](../docs/23-economy.md#bots-spend-power-through-the-same-code-priced-by-part--read-and-measured) |
| M4 | The point-in-triangle test of the hit test (`0x10011090`), and the landscape's own cell size | an edge test on the triangle's winding; the ground index's 16 m cells | [26](../docs/26-damage.md#the-hit-test--read-and-measured) |
| M4 | Which node flag makes a node vital | the mesh node's `0x200` | [26](../docs/26-damage.md#hit-points--read-and-measured) |
| M4 | Whether vegetation and rock take damage | they stop rounds and take none | [04](../docs/04-missions.md#the-scale) |
| M4 | What a dead object leaves: its explosion, wreck and damage stages | it vanishes | [26](../docs/26-damage.md#hit-points--read-and-measured) |
| M4 | Shields: bubble contacts and sectors | not modelled; no Mission 01 target has one | [26](../docs/26-damage.md#shields-a-generator-a-deflector-six-sectors--read-and-measured) |
| M4 | Poses of other units for the hit test | their rest poses: other units' animation is not played | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M4 | How the two ends of a collision object's sweep differ | the round's position before its move this tick, and after | [26](../docs/26-damage.md#not-established) |
| M4 | The collision radius of a round | the mesh header's bounding sphere | [26](../docs/26-damage.md#not-established) |
| M4 | The rest of the emitter floats: what the fade value scales, what a particle's exponent-shaped triples are, a bolt's widths | sprites (3, 4, 9) move +40→+52 and grow +100→+112 by progress through the window, alpha 1→+24 to the power +28; a bolt (5) is +24 wide and min(+36, +32 × 1000 × s) long behind its origin; a burst (7, 10) is max(1, +16) particles between velocities +44 and +56, spread +68, living +28 of the window, sized +92→+104, fading; streams (8) are not drawn; flag bit 8 ignored | [11](../docs/11-effects.md#not-resolved) |
| M4 | An effect's jitter (flag 1), the owner values of time modes 5–15, and a phase's animated texture frames | no jitter; modes 5–15 read a speed fraction the caller sets; frame 0 of every texture | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M4 | How a sprite whose material says opaque blends | alpha-blended, so its fade shows | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M4 | How the shade lights with a type-1 light's range and attenuation; type 1 is a light in the owner's `CLightManager` | none yet; to come, Direct3D's fixed-function falloff, 1 / (a0 + a1·d + a2·d²) inside the range | [11](../docs/11-effects.md#not-resolved) |
| M4 | Which draw pass draws header-flag-0x800 effects; who sets the manager's target point a bolt starts from | 0x800 effects draw with the rest; a round's target point is where it was fired from | [11](../docs/11-effects.md#not-resolved) |
| M4 | The effect manager's random generator | any uniform generator | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M5 | What the class-24 arms' states 1, 2 and `0x21` play | unfold on select, fold on deselect | [29](../docs/29-weapons.md#not-established) |
| M5 | The camera shake's trigger | no shake | [30](../docs/30-turrets.md#not-established) |
| M5 | How the HUD draws the aim point and the guns | a crosshair at the centre; a slot a gun, lit while selected, with magazine and capacitor bars | [30](../docs/30-turrets.md#not-established) |
| M5 | How a sound falls off between its near and far distances, and how it is panned | linear in distance; panned by its direction against the eye's right | [11](../docs/11-effects.md#emitter-types--read-and-measured) |

### Read since the stand-in was written

Research has closed these, and the code still carries the stand-in. The next
engine pass replaces each with what was read and removes its row.

| milestone | the code's stand-in | what is read | see |
|---|---|---|---|
| M1 | the dome is drawn with no depth | its layers are depth-tested without depth writes (render states 7 and 14, `Terrain.dll:0x100302fb`), in render layer 1, on a record that takes the scene's fog | [10](../docs/10-sky.md#the-dome) |
| M1 | one directional light, coloured by slot 19 × the third float and held to 1 | the sun object is two directional lights: slot 19 × the third float, lifted up to 5× by the flare gates, and slot 21 | [10](../docs/10-sky.md#what-the-sun-does-with-its-seven-values) |
| M1 | a body is up from its first keyframe to its second in clock order; section 0 | a keyframe's opcode is the word ahead of slot 0; a body starts on its opcode and lasts to the first stop at or after it, never wrapping back to section 0; a file's sections play in turn, as one cycle | [10](../docs/10-sky.md#a-bodys-lifetime) |
| M3 | a state's 16-byte conditions are taken as met | they are contacts: `0x100` makes a state need its contact point's node intact and `0x200` destroyed, a walker's limping states | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M4 | start the gun's effect and `_sfx` at step 1 of each barrel stroke, driven 0 to 1 by the barrel channel | nothing starts them: a time-mode-4 effect follows its node's animation value, so the gun effects follow the barrel nodes through each stroke and the `_sfx` follow the arm nodes — arm sounds, not shot sounds | [11](../docs/11-effects.md#time-mode-4-is-a-nodes-animation-value--read-and-measured) |
| M4 | slot 0 of the `.exp` on a unit; the surface's slot on the ground | a unit answers for its material: a strike on a unit plays the slot for the struck batch's material class, `mt` (slot 6) on Mission 01's dummies and hero | [11](../docs/11-effects.md#what-an-explosion-plays--read-and-measured) |
| M4 | a bolt min(+36, +32 × 1000 × s) long; alpha 1 → +24 to the power +28; streams not drawn; flag bit 8 ignored | a bolt runs from its start point to where the effect is now, floor(length / +36) sprites, 1 to +20; a stream emits every lerp(+24, +28) s into a ring of +36; a fade is start + (end − start) × x^power (a sprite's +20, +24, +28); bit 8 draws with the depth test off while the header's tested point is in view | [11](../docs/11-effects.md#bolts-streams-and-fades--read-and-measured) |
| M5 | at noon of its day | at the file's closing time, in section 0: 01:30 on Mission 01, 56 s into its 900-s day | [10](../docs/10-sky.md#where-the-clock-starts) |
