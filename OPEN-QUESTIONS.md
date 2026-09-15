# Open questions still to research

Compiled on 2026-09-14 against `a5e0188` from every doc's *Not established*,
*Not resolved* and *What is not read here* section, [TODO.md](TODO.md),
[docs/06-open-questions.md](docs/06-open-questions.md) and the unknowns in
[engine/README.md](engine/README.md#stand-ins). Each doc stays the source of
truth; this is the queue.

**[M1]–[M5]** marks a question the engine currently answers with a stand-in,
by milestone. Those come first. Left out: behaviour already read and waiting
only on the engine (the README's *Read since the stand-in was written*), and
engineering such as terrain culling or drawing the sky's textures.

## Sky and rendering

[10-sky](docs/10-sky.md#not-resolved), [02-texm](docs/02-texm.md),
[07-objects](docs/07-objects.md), [06](docs/06-open-questions.md)

- [ ] [M1] Where the sun object's two lights point: nothing writes the light record's `+0x24`.
- [ ] What `CSun` does with a body's lifetime, for a body that started before the clock's start.
- [ ] [M1] How the dome escapes the far plane and the fog (render layer 1's projection, fog defaults at `0x1003d9f2`), and what lies below its rim.
- [ ] [M1] Whether `ForceSWFog` does anything outside `Terrain.dll`.
- [ ] [M1] The fog heading's world axis: that the camera matrix's first column is the view direction is a guess.
- [ ] [M1] Whether water draws see-through (`WATER` says opaque, with a `WATER_BOT` bed beneath).
- [ ] [M2] Whether a blended material writes depth, and the alpha test's reference value.
- [ ] [M3] What draw layers 10 and 9, which a fifth slot is filed under, do (`Terrain.dll:0x1004553b`); and `CShade` slot 15.
- [ ] The sun sprite's extent unit, camera slot 27, and shader slot 5's colour filter and flag bit 0.
- [ ] Texture header bit `0x4000000` (81 textures) and load flag `0x200000`'s effect.
- [ ] Who sets an object's material track (`ILifeSystem` slot 16), and who calls IAnimation slot 27.
- [ ] What reads object face flags 2 (walkable floors) and 16.
- [ ] What IAnimation node mask bit `0x10` does.

## Effects and sound

[11-effects](docs/11-effects.md#not-resolved)

- [ ] [M4] The rest of the emitter floats:
  - [ ] whether a fade scales alpha or colour
  - [ ] the per-axis exponent triples
  - [ ] the sprite powers at `+64` and `+124`
  - [ ] a bolt's widths at `+24` and `+28`
- [ ] [M4] How the shade lights a type-1 light (falloff, attenuation) and the manager flags `0x80000000` and `0x20000000`.
- [ ] [M4] Header flag `0x800` and the rest of the draw path:
  - [ ] who passes the draw-pass argument flag `0x800` waits for
  - [ ] who sets the target point a bolt starts from
  - [ ] draw flag 4 (header `0x2000`)
  - [ ] header flag `0x10000`
- [ ] The four settings groups, the group floats `+0x1084` and `+0x1294`, and the page's `+0x14a4`.
- [ ] [M4] The effect manager's random generator and jitter, the owner values of time modes 5–15, and a phase's animated frames.
- [ ] [M4] When a stream emits its first particle, and where burst and stream particles go.
- [ ] [M4] How often an effect tests its point's view, and what that ray meets.
- [ ] [M4] What a building answers for a strike's material, and a node's wear base.
- [ ] [M5] How a sound falls off between its near and far distances, and how it is panned.

## Motion, ground and controls

[24-motion](docs/24-motion.md#not-established), [13-control](docs/13-control.md#not-established),
[14-controls](docs/14-controls.md)

- [ ] [M3] How interface `0x25` slot 3 turns level-0 triangles into a push, and what slot 2 does with its 0.5.
- [ ] [M3] Which scene nodes are types 1 and 3, which decides whether bridges and buildings are ground; plus a machine's type-3 parent and what message `0x201` returns.
- [ ] [M3] The ground contact's timing and dt, the pose its contact points use, the second sphere's radius r₂, and what lifts a sphere with no face under it.
- [ ] Which `Land.msh` faces carry the world bit `0x8` and class bit 8 that the ground search excludes.
- [ ] Contact record flags 2 (slot `0x7c`) and `0x20`.
- [ ] [M3] How often `World3D.dll`'s input update runs (it paces the cruise ramp), and which screen states set the 0.5 mouse sensitivity.
- [ ] [M3] What handlers do when an active row runs again each update, and who calls the second walk/turn ramp's setter (slot 11).
- [ ] [M3] A state's use count `+0x94`, the state a machine starts in, the game's jitter random source, and a controller's request code before any is sent.
- [ ] [M3] The vector that righting bits `0x30` stand the hull toward (`+0x348`, no writer found).
- [ ] [M3] Which way across a slope the mode-2 brake acts, and which way a positive lean tips the model.
- [ ] [M3] A chord with no row of its own, such as Shift+W.
- [ ] What behaviour flag `0x800` changes besides clearing the walker.
- [ ] The remaining `.ctl` values:
  - [ ] class 3's value 0, the camera's values 3–5, and the hero's arms' values 1 and 4
  - [ ] the section-5 record's int 8
  - [ ] control message 7's arguments (byte 15, `+0x618`)
  - [ ] `IDeviceManager` ids 5 and 6

## Turrets, weapons and camera

[29-weapons](docs/29-weapons.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[28-chassis](docs/28-chassis.md#not-established), [25-sensors](docs/25-sensors.md#not-established)

- [ ] [M4] Whether the landscape is among the objects the sight ray walks.
- [ ] [M4] What a falling round's mount solves for with no target, which way its lift turns on a hung turret, and whether a player's turret is in `CIS_MANUALCONTROL`.
- [ ] [M4] Whether an AI-set target survives the player taking over, and what sets the hero's target field (`+0x38`).
- [ ] [M4] How a gun's capacitor refills (the power tick).
- [ ] [M3] How the camera builds its frame when its up is parallel to the look.
- [ ] [M5] How the HUD draws the aim point, the guns and the player's target; what plays `TARGET_READY` and `TARGET_ZOOM`; and the unit record's `+0x94` and `+0x98`.
- [ ] `e_gun_bl_03` and `e_gun_tl_02` carry a follower and no gun: are they never fitted, or does it pair with a later gun?
- [ ] The AI fight module:
  - [ ] its two aim factors
  - [ ] which gun lends the lead speed
  - [ ] tasks 2, 3 and 5
  - [ ] `MBehaviour+0x614`
  - [ ] the id nibble that frees the winged SSMs
- [ ] An AI turret's state word while it fights (`0x200` or `0x400`).
- [ ] The Large transport's second slot and the Large builder's module socket: what fills the design's slot records.
- [ ] What stops the player building the six free turrets, and which state bits the design screen tests.
- [ ] Where the unit constructor's page item names (`+0xc4`) come from.

## Damage, sensors and ownership

[26-damage](docs/26-damage.md#not-established), [25-sensors](docs/25-sensors.md#not-established),
[27-ownership](docs/27-ownership.md#not-established)

- [ ] [M4] The hit test's point-in-triangle test (`0x10011090`) and the landscape's own cell size.
- [ ] [M4] Which node flag makes a node vital (is AniMesh query `0xe` the mesh node's flags?).
- [ ] [M4] Whether vegetation and rock carry node life.
- [ ] [M4] Whether a round's ground test strikes the water surface.
- [ ] [M4] What a dead unit leaves (wreck, damage stages), and what `iron3d.dll` does with owner word `0xfffe` (37 compares).
- [ ] The `.exp` record's two 1.0 floats, for which no reader was found.
- [ ] What the player's map and radar display show (`iron3d.dll:0x1003fb90` and `0x10073550`).
- [ ] What moves a SuperAI's attitude from one relation band to another.
- [ ] The game view's states 1, 3, 4, 5 and 6, and what pods open on a generator, mine, storage or Outpost; also what `0x10033e40` refuses on a tower.
- [ ] The other four writers of a unit record's `+0xa2`.

## AI, scripts, packages and economy

[15-behaviour](docs/15-behaviour.md#what-is-not-read-here), [31-packages](docs/31-packages.md#not-established),
[23-economy](docs/23-economy.md#not-established), [32-builder](docs/32-builder.md#not-established)

- [ ] What the helpers below the `.scr` handlers compute: a unit's strength, the distance helper, the problem's action record, and SuperAI `+0x40c`.
- [ ] A problem's two raise numbers, which handler runs when, and who writes `dCurrentProblem` and `dCurrentSender`.
- [ ] Channel 2 of the message callback (function 57), and the count function 69 stores.
- [ ] Whether any script depends on a constant landing inside a false block.
- [ ] Five labels no goto aims at, and the label that sits inside a block.
- [ ] The `.fml` operators the corpus never uses.
- [ ] A fire-control request's 0.5, and what sets `+0x5c` and `+0x60` to lock a unit's fire mode.
- [ ] Who sends `MBehaviour` messages `0x19` and `0x1a` (the retaliations).
- [ ] Whether a clan's AI re-orders a build refused for want of a mind.
- [ ] What a mine's ToMine does to its output.

## Mission progression

[34-progression](docs/34-progression.md#not-established)

- [ ] The clock unit that times route reports.
- [ ] How a destroyed or captured unit leaves function 31's list.
- [ ] Where message text is drawn, and for how long.
- [ ] What `info_system` changes on screen.
- [ ] What follows `MISSION_COMPLETE`, and what leads to the next mission.
- [ ] Whether the hero reports its route from inside a boarded bot.
- [ ] What the behaviour does with the message 6 it sends itself.
- [ ] The ambient variations' schedule.
- [ ] A failure on the hero's death.

## Files and formats

[17-saves](docs/17-saves.md#not-established), [16-research](docs/16-research.md#what-is-not-read-here),
[19-descriptions](docs/19-descriptions.md#what-is-not-read-here), [18-vocabulary](docs/18-vocabulary.md),
[21-briefing](docs/21-briefing.md#not-established), [06](docs/06-open-questions.md), [22-settings](docs/22-settings.md)

- [ ] Saves:
  - [ ] most chunks' contents (the control chunk past `+32`, the wizard, behaviour, building and tree chunks)
  - [ ] the clan word before each mind list
  - [ ] the 24-byte records and the `1, id, id` triple
  - [ ] the AI state's layout
  - [ ] whether a mind list's ids are logical ids
- [ ] What `iron3d.dll:0x1008a690` does with a part's derived number, and what else reads a part's `Type`.
- [ ] `objects.dlb`: what the `A` and `N` size letters stand for, and the 37 exceptions to the size grade.
- [ ] Which of the `bb`/`bl`/`bm`/`bp`/`br`/`ba`/`bf`/`bt` building prefixes is which.
- [ ] Briefings: what game mode 4 is, how a briefing is skipped (`WaitForClick`), and the spline's curve.
- [ ] `data.tma`: what reads a building's start flag back, and the word after the map path.
- [ ] What `TRF1`'s directory flag does beyond the debug warning.
- [ ] What the landscape, camera and atmosphere component constructors read.

## Mission 03, *The Field Base*

Added on 2026-09-15 against `950af7e`, after M12 made the mission winnable end to end.
[40-command-mode](docs/40-command-mode.md#not-established), [41-commander](docs/41-commander.md#not-established),
[42-selection](docs/42-selection.md#not-established), [32-builder](docs/32-builder.md#not-established),
[23-economy](docs/23-economy.md#not-established), [31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established), [37-designer](docs/37-designer.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M12] The construction sphere's look. Its three effects' records give time mode 0, a value set from outside (slot `0x1c`), and what sets it is not read: they loop on their durations and take the sphere's radius as their size, and the dome only partly shows.
- [ ] [M12] The commander's satellite map (`0x10073830`): its marks by type (the recording shows building and unit icons, the engine plain squares), its title bar and exit icon beyond their place, and the `+0x230` rectangle the column's click tests first.
- [ ] The warbot designer's save name field and load list: not built, and how they work is not established.
- [ ] A Mission 03 transport walks about 33 m/s where docs/23 gives 24: the walker's pace, not the transport task. The recording's round is also 13 s longer than two full-speed walks.
- [ ] Why the patrol took about 85 s to reach the base in the recording, when a straight flight is 37–44 s; and how high a patrolling flyer flies, which decides whether its 3D attack limit ever holds.
- [ ] Which fight-module bar a building's guns must clear. With the walker's 0.85 the Small Bunker's flamers only fire at a unit close to its own ground level, never at hovering flyers, so the warbots do the fighting.

**Command mode and the panel**

- [ ] [M12] Whether the character handler sees Esc before its binding leaves command mode (the engine peels the map and the page back first).
- [ ] What view state 4 and the second camera at `+0x68` are for; cursors 7 and 8; the globals `0x1010bf7c`–`0x1010bf80`; the display's slot 12, which picks the system's cursor over the software one.
- [ ] What interface `0x201` slot 9 with (`0x20`, 1) and message (6, 7, 0) do to a bunker left for another view or for telepresence.
- [ ] [M12] What telepresence's auto-driver levels 1 and 2 hand to the AI, and what mode 2 does when its unit dies.
- [ ] [M12] A unit record's `+0x30` and property `0x207`, and a building's `+0x30`: they pick and tint the panel's icons.
- [ ] [M12] What slot 7 of a unit's object does 0.6 s after *Explode!*; the chat overlay; the game menu's screen (mode 7); tooltips.
- [ ] [M12] What `0x10034230` accepts for an Upgrade row, and what Type `0x80000200` is.
- [ ] The research panel's contents and controls (page 4).
- [ ] [M12] The routine that names a building (strings 6031–6098): the engine picks by Type and the root record's size letter.

**Selecting and ordering**

- [ ] [M12] An areal's first flag word (`+0x20`), which decides where a walker may be sent, and why the recording shows `PLACE`, not `GUARD`, over the bunker's roof at 190.5 s.
- [ ] [M12] Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule (`0x100361a0`).
- [ ] Whether the band is filled or only outlined; double clicks; whether a group sent to one place spreads out.
- [ ] The pending picks not traced: kind 2 (attack-target mode) and the orders kinds 2–5 give.
- [ ] [M12] A unit marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand.

**Placing and building**

- [ ] [M12] The site test's path search, its hall-way areal test (interface `0x303`, vertex bit 1) and the basement's triangulation.
- [ ] What the pick's query record (first word `0xa`) asks the world for, so which objects stop the cursor's ray.
- [ ] What the game's `+0xe4` byte is, under which a site within 400 of one of the level's records turns red; whether holding `,` or `.` keeps turning the ghost.
- [ ] What an unfinished building looks like before the dome.
- [ ] [M12] Which state each sphere code opens, where an action-5 effect is placed, and which classes the sphere's kill takes.
- [ ] Which call sends SuperAI event 2 for a building a builder puts up (the recording counts the mine at 196 s, and it stands at 236 s).

**Economy**

- [ ] The clan's minds in the recording: the CPU figure reads 4 before any build, yet five warbots (SSW-4 to SSW-8) were built.
- [ ] The direction of the ore a mine's and a storage's loading places move by themselves (`0x10019482`, `0x100195b8`), and unit property `0x208`.
- [ ] [M12] Batteries and efficiency read from a building's root controller only; the economy timers' random source.

**Mission 03's script and recording**

- [ ] Why `T03_H03` and `T03_H02` never show in the recording.
- [ ] Which of the generator's exits the recording's hero used (the south one is inferred), and whether a shot door opens sooner than an approach.
- [ ] What component property `0x200` is (taken to be the door's node: all 56 door components' nodes match their channels').

## Not looked at at all

- [ ] The network protocol.
