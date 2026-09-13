# engine

A Rust engine for *Parkan: Iron Strategy* that reads the game's own install.
Its first target is Phase One: load Mission 01, *Line of Fire*, put the hero
at its start, and walk, look and shoot in first person.

Milestones **M0**, **M1** and **M2** are in:

- the workspace;
- the NRes, mission, `Texm`, `Material.lib`, wear, `Land.msh`, object mesh,
  `objects.rlb`, `.dat`, controller (`.ctl`) and input table (`.tbl`) readers;
- the golden cross-check;
- a window over Mission 01.

The ground is drawn with its two material layers blended and the file's own
mip levels. Every placed object is drawn from its assembly:

- each part is mounted on its host's socket;
- each node sits at its rest pose;
- level 0 is drawn, and collision hulls never;
- materials draw in the blend mode their flags byte names.

Not yet: animation (M3), lightmaps (no Mission 01 mesh has one) and levels of
detail beyond 0. This
directory also holds what the rest will follow:

- **`docs/`** is the source of truth. Every behaviour the engine implements is
  in a doc there, labelled *read* or *measured*, and re-derived by
  `uv run openparkan verify`.
- **`design/`** keeps the eight Phase R research notes, R1 to R8. They turn
  the docs into implementable steps. Where a note and `docs/` disagree,
  `docs/` wins, and each note says at its top what has been superseded.
- **The table below** lists the places where the game's behaviour is not
  established and the engine has to choose one. It is the only list of
  guesses.

The plan is Rust + wgpu (winit, glam; kira for sound later). It covers one
workspace of `parkan-formats`, `parkan-sim`, `parkan-world`, `parkan-render`
and `parkan`, in milestones M0 to M5.

## Running it

```
cd engine
cargo run --release -p parkan                                  # Mission 01, debug camera
cargo run --release -p parkan -- --screenshot m1.png           # one frame to a PNG, no window
cargo run --release -p parkan -- --screenshot map.png --top-down --size 768x768
cargo run --release -p parkan -- --screenshot tank.png --look 705,885,24,734,906,10   # from X,Y,Z at TX,TY,TZ
cargo run --release -p parkan -- --mission MISSIONS/Single.01  # another mission
```

`--game DIR` or `PARKAN_DIR` points at the install when it is not beside this
repository. In the window W/A/S/D and Q/E fly, holding the right mouse button
turns, Shift flies faster and Escape quits.

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
every controller, archive by archive, and the three input tables: 157 dumps.

## Stand-ins

Each stand-in is marked in the code as `// STAND-IN: docs/NN#section` and has
a row here. A row leaves this table when research closes it.

| milestone | what is unknown | stand-in | see |
|---|---|---|---|
| M1 | Whether water is drawn see-through: `WATER`'s material says opaque, and every lake has a `WATER_BOT` bed beneath | opaque, as the material says | [03](../docs/03-terrain.md#terrain-layers-name-materials-not-textures) |
| M2 | Whether a blended material writes depth, and the alpha test's reference value | blended groups draw after opaque ones without writing depth; nothing is discarded (reference 0) | [07](../docs/07-objects.md#how-a-material-draws-is-in-the-archive-directory) |
| M1 | Whether the scene's fog is Direct3D's vertex fog or the engine's own | per-pixel linear range fog, 0 to 700 × slot 6 (the two look the same) | [10](../docs/10-sky.md#not-resolved) |
| M1 | How the 34142-radius dome escapes the far plane and a fog ending by 700 | draw the dome first, depth writes off, unfogged but for its rim | [10](../docs/10-sky.md#not-resolved) |
| M1 | The heading angle's zero and direction for the fog colour | 0 along +y, turning towards +x, like the dome's segments | [10](../docs/10-sky.md#not-resolved) |
| M1 | What the sun does with its seven values; the scene's light direction | a directional light coloured by slot 19 × the third float, pointing from the sun's fixed place | [10](../docs/10-sky.md#not-resolved) |
| M3 | How a body is put back on its ground point, and whether modes 0 and 2 fall | after integrating, set the sphere centre to ground point + r along the normal | [24](../docs/24-motion.md#not-established) |
| M3 | Collision between objects: shapes and response | spheres pushed apart in xy, the approaching velocity cancelled; faces steeper than 80° remove the velocity into them | [24](../docs/24-motion.md#not-established) |
| M3 | The map edge, bridges, jumping | clamp to the terrain's box inset by r; bridges and buildings as walkable mesh faces | [24](../docs/24-motion.md#not-established) |
| M3 | Which way the gap to the liquid surface is measured over a bed | `water_level − centre.z < r` | [24](../docs/24-motion.md#not-established) |
| M3 | The mesh walk order inside `FindWorldFace` | cross the edge the segment leaves by until the face holds the centre | [24](../docs/24-motion.md#finding-the-ground--read) |
| M3 | The divisor D in a blended state's weight | the speed box's span | [24](../docs/24-motion.md#not-established) |
| M3 | The transition cost's scaling by the gap between two states' boxes | the file's cost unscaled | [24](../docs/24-motion.md#playing-a-state--read-and-measured) |
| M3 | Section 1's 16-byte conditions | always satisfied | [24](../docs/24-motion.md#section-1-is-the-animation-state-graph--read-and-measured) |
| M3 | Whether a state's step velocity replaces the integrated one | it does not; the boxes test the integrated velocity | [24](../docs/24-motion.md#not-established) |
| M3 | The mouse's yaw and pitch signs on screen | mouse right turns right, mouse up looks up; check against the game | [30](../docs/30-turrets.md#not-established) |
| M3 | Which camera point gives the position and which the direction | position from `CameraCenter`, direction from `TargetDirect` | [30](../docs/30-turrets.md#aiming-and-the-camera--read-and-measured) |
| M3 | How the strafe angle's turn splits between hull and turret; the keypad cruise ramp | the legs turn and the turret holds its heading; the ramp adds 0.05 a second to the command | [24](../docs/24-motion.md#not-established) |
| M3 | The mouse sensitivity handed to `World3D.dll` | `MOUSE_SENS` × 0.01 from `Iron_3D.ini` | [14](../docs/14-controls.md#from-a-row-to-a-command--read-and-measured) |
| M4 | What sets the hero's turret target in first-person play | none: the plasma bolt and the missile fly straight | [29](../docs/29-weapons.md#not-established) |
| M4 | What starts the hero's muzzle flash and shot sounds | start the gun's effect and `_sfx` at step 1 of each barrel stroke, driven 0 to 1 by the barrel channel | [29](../docs/29-weapons.md#not-established) |
| M4 | How the turret's follower channels set a gun's ready byte | always ready | [29](../docs/29-weapons.md#not-established) |
| M4 | What geometry the sight ray (IWorld slot 7) meets | terrain and every object's level-0 mesh, as the hit test does | [29](../docs/29-weapons.md#not-established) |
| M4 | How the two ends of a collision object's sweep differ | the round's position before its move this tick, and after | [26](../docs/26-damage.md#not-established) |
| M4 | Whether a unit answers for its material when a round strikes it | slot 0 of the `.exp` on a unit; the surface's slot on the ground | [11](../docs/11-effects.md#not-resolved) |
| M4 | The collision radius of a round | the mesh header's bounding sphere | [26](../docs/26-damage.md#not-established) |
| M4 | Most emitter floats: colour and alpha over life, emission rates, bolt length | per type as in `design/r5-effects.md`; flag bit 8 ignored | [11](../docs/11-effects.md#not-resolved) |
| M4 | Whether emitter type 1 is a light | a point light at the attach point, or nothing until M5 | [11](../docs/11-effects.md#not-resolved) |
| M4 | The effect manager's random generator | any uniform generator | [11](../docs/11-effects.md#how-an-effect-runs--read) |
| M5 | What the class-24 arms' states 1, 2 and `0x21` play | unfold on select, fold on deselect | [29](../docs/29-weapons.md#not-established) |
| M5 | The camera shake's trigger | no shake | [30](../docs/30-turrets.md#not-established) |
| M5 | When a mission's sky clock starts | at 00:00 of the file's first section | [10](../docs/10-sky.md#not-resolved) |
