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
- [ ] [M14] The walker's local path and its obstacle contours: how it goes round a tree's or a stone's hole, whether it widens it by the unit's size, how a walker in one walks out, and what it does with a goal in one; the sub-areals' shapes and whether the search measures one from its centre; whether every scenery object reaches the areal map, and the box of a mesh of several parts; how it drops the points a unit has passed (`MWalker::ClearMoverReachedPoint`); how a unit's place comes onto a building's map object and which vertex the search starts from; who calls `MHallWay` slot 11; a hall-way vertex's size gate (the unit's `+0x960`, the record's `+0x28`); the link flags `0x10000` and `0x20000`; how a walker goes to the point it finds off a non-walkable areal, and what it does when its search fails; and how a walk to a door gets past the building's own walls, which cut no areal (the engine: a door more than 20 over the ground under it is passed over, and a straight line into a wall goes round the building's ground contour) ([24-motion](docs/24-motion.md#not-established)).
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

## Mission 02, *The Constructor*

Added on 2026-09-15 against `4f3a16e`, after the Outpost island's landing was fixed in M14.
[34-progression](docs/34-progression.md#not-established), [36-factory](docs/36-factory.md#not-established),
[37-designer](docs/37-designer.md#not-established), [38-designs](docs/38-designs.md#not-established),
[39-boarding](docs/39-boarding.md#not-established), [24-motion](docs/24-motion.md#not-established),
[31-packages](docs/31-packages.md#not-established)

**Where the engine is weaker than the game**

- [ ] The Large Factory's front door: its hall way does not lead to the pod, so the engine's way in is the west side door, which the recording's hero takes.
- [x] ~~The chimney smoke is orange where the recording's plumes are black~~ — closed in M14: an effect sprite takes its material entry's cell as a mesh batch does, and plays its track from its own start, so a puff leaves the chimney on `fire_smoke`'s first, orange cells and is on its later, black ones a quarter of a second on. Its plume's size went with it: a stream's particle walks and grows in metres, which the recording's 29 m over the chimney and 31 m across measure, where the control points' 2.6-long axes had made it 130 and 78. What a sprite's material starts from, and that the effect draw takes the entry's cell at all, are still stand-ins ([07-objects](docs/07-objects.md#how-a-material-reaches-the-device--read-and-measured)).
- [ ] [M12] The medusas stand still. An animal's migration over its clan's pastures is not modelled, so the fight the recording shows from 195 to 218 s, two medusas high over the ground and green acid landing round the hero, never happens. Also not read: what their attack does with the migrate task's figures and circle, and how a flyer's migration point, at the pasture centre's height, meets its flying height.
- [ ] [M14] When `AniMesh.dll` works an agent's sphere and its node sphere out again (`0x10009510`), and at which pose. The engine works both out once, at rest, as the unit is made; they set how high a landed flyer stands, and so where the hero can get out.
- [ ] The minds: the recording's factory shows the hero holding one of the player's two, the same question as Mission 04's which unit holds none.

**The factory screen and production**

- [ ] What puts the capture and the factory screen on one frame.
- [ ] [M11] How the cursor is shown in view mode 5 (the engine: the system's cursor), and the fill colour the resource rows hand their bar (the engine: the weapons list's).
- [ ] Whether the designer pauses the world (the level's flag bit 8), and whether the `Mission` handler runs while a building's screen or the designer is up (the pause byte `+0xe8`).
- [ ] What handing the hero back does to it while the screen is up, and whether the player's keys still move it.
- [ ] The heading the escape leaves a new bot with, and whether a flyer climbs on its way out. (~~[M11] which areals the escape's random points must be on~~ — closed in M14: walkable ones, as the roam's test reads.)
- [ ] What commander pages 1–4 and 6–8 show from first person.

**The warbot designer**

- [ ] [M11] That the factory record's `+0x30`, the grade the chassis page is taken over, is the building's size class.
- [ ] [M11] The part box: `Epower`, the properties behind `regener`, `capacity`, `throughput`, `shotnum` and `blast`, and the formatter that prints one decimal whatever the template asks.
- [ ] [M11] What the turret fit does to guns on a turret it replaces, and the gun fit to a clip on a gun it replaces.
- [ ] [M11] Which destination row a tab selects as it turns on, which tab the panels turn to after a fit (only *seen*), and when fitting a chassis enables Armour (`0x10052491`).
- [ ] [M11] The previews' camera (which way it looks, the axis of its −0.5 rad pitch, which side its 60° field spans, its lights' colours), and how a scan band's green specular lights its strip.
- [ ] What the driven unit's property flag `0x8` gates while the designer is open; how the font's colour slot turns a row's colours into text; who fills the recent projects' count (`+0xb8e0`); the destination panel's own draw and takt; the tooltip's box and timing.

**Boarding, flying and getting out**

- [ ] [M11] The heading the hero is given on leaving, read as (F.x, −F.y) under an assumed matrix layout; the engine turns the hero to face the bot, as the recording shows.
- [ ] [M11] Which of a turret's nodes the boarding test's property `0x52` reads the life of.
- [ ] [M11] The name a bot's gun takes in the weapons list (`0x1008a470`, not followed).
- [ ] Why the recording's bot came to the hero between 236 and 249 s: an order the player gave, or its own behaviour after production.
- [ ] Whether a detached hero stays in the collision manager's or the areal map's lists, and what game messages `0x3f1` and `0x3f2` carry.

**Walking into the Large Factory**

- [ ] [M11] What `CBuilding` does to a door's or a pod's switch word as it files the item, and the capsule a door is measured against (`Terrain.dll:0x1005a27f`).
- [ ] [M11] Where a gathered face's batch word comes from (the engine lets movers **and rounds** through the `DEFAULT` and `PORTAL` materials' faces).
- [ ] [M14] How the basement is triangulated between its two rings, and which of the band's faces the first builder (`Terrain.dll:0x1000bdb0`) makes with the cut landscape face's own texture pair rather than the foundation. What the second builder writes on every face it makes — layer-1 slot 0, no second layer, flags `0x300`, a UV over the world at 0.066 a unit — is read (docs/03, "What a basement face wears"), and it is what the engine lays down.
- [ ] [M11] Whether the slope brake reads a building's stair faces, and who sets a collision object's flags, so which movers keep the floors in their push-out.
- [ ] [M14] How a portal face reaches `CBuilding::PortalDrawNotify` (`Terrain.dll:0x1005a5d0`, an interface slot nothing in the install is found to call) and which node its record names, so which cells a building draws. The lists and the render are read (docs/24, "A building is drawn cell by cell through its portals"); the engine draws every cell and only drops the portal quads, which costs frame time and shows nothing extra.
- [ ] [M14] Where the node matrix an action-3 effect takes as its frame (`Effect.dll:0x1000625a`, property 2) comes from. Its translation is the node's authored origin, which on 68 of `fortif.rlb`'s 112 door sounds stands more than 10 m from the door — 30.8 m on the three factories' side doors — so the sound is all but inaudible in the doorway. The engine stands the effect at the node's level-0 sphere centre instead.

## Mission 03, *The Field Base*

Added on 2026-09-15 against `950af7e`, after M12 made the mission winnable end to end.
[40-command-mode](docs/40-command-mode.md#not-established), [41-commander](docs/41-commander.md#not-established),
[42-selection](docs/42-selection.md#not-established), [32-builder](docs/32-builder.md#not-established),
[23-economy](docs/23-economy.md#not-established), [31-packages](docs/31-packages.md#not-established),
[34-progression](docs/34-progression.md#not-established), [37-designer](docs/37-designer.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M12] The construction sphere's look. Its three effects' records give time mode 0, a value set from outside (slot `0x1c`), and what sets it is not read: they loop on their durations and take the sphere's radius as their size, and the dome only partly shows.
- [ ] [M12] The commander's satellite map (`0x10073830`): its title bar and exit icon beyond their place, and the `+0x230` rectangle the column's click tests first. (Its marks by type are read and drawn since M13.)
- [ ] The warbot designer's save name field and load list: not built, and how they work is not established.
- [x] ~~A Mission 03 transport walks about 33 m/s where docs/23 gives 24~~ — closed in M13: the live limits now come from the unit's engine and load, and it walks 23.98. The recording's round being 13 s longer than two full-speed walks is still open.
- [ ] Why the patrol took about 85 s to reach the base in the recording, when a straight flight is 37–44 s; and how high a patrolling flyer flies, which decides whether its 3D attack limit ever holds.
- [ ] Which fight-module bar a building's guns must clear. With the walker's 0.85 the Small Bunker's flamers only fire at a unit close to its own ground level, never at hovering flyers, so the warbots do the fighting.

**Command mode and the panel**

- [ ] [M12] Whether the character handler sees Esc before its binding leaves command mode (the engine peels the map and the page back first).
- [ ] What view state 4 and the second camera at `+0x68` are for; cursors 7 and 8; the globals `0x1010bf7c`–`0x1010bf80`; the display's slot 12, which picks the system's cursor over the software one.
- [ ] What interface `0x201` slot 9 with (`0x20`, 1) and message (6, 7, 0) do to a bunker left for another view or for telepresence.
- [ ] [M12] What telepresence's auto-driver levels 1 and 2 hand to the AI. (What mode 2 does when its unit dies is read: the removal table rolls modes 1, 2, 5 and 7 back.)
- [ ] [M12] A unit record's `+0x30` and property `0x207`, and a building's `+0x30`: they pick and tint the panel's icons.
- [ ] [M12] What slot 7 of a unit's object does 0.6 s after *Explode!*; the chat overlay; the game menu's screen (mode 7); tooltips.
- [ ] [M12] What `0x10034230` accepts for an Upgrade row, and what Type `0x80000200` is. (The engine: the upgrade task's own target test — a live building of the Type whose scheme has an entry above its own — and, *derived* from Mission 03's recording offering no Upgrade Warehouse over a Small Warehouse whose Medium is unresearched, that the entry above is researched whole.)
- [x] ~~The research panel's contents and controls (page 4)~~ — read and built in M13 ([41-commander](docs/41-commander.md)).
- [ ] [M12] The routine that names a building (strings 6031–6098): the engine picks by Type and the root record's size letter.

**Selecting and ordering**

- [x] ~~[M12] An areal's first flag word (`+0x20`), which decides where a walker may be sent~~ — read in M14: it marks a walkable areal, the only kind the areal map links, and the engine's walker now keeps to them ([24-motion](docs/24-motion.md#the-global-path--read-and-measured)).
- [ ] Why the recording shows `PLACE`, not `GUARD`, over the bunker's roof at 190.5 s.
- [ ] [M12] Which objects the world's classes 3 and 4 are, and the object pick's order and nearest-hit rule (`0x100361a0`).
- [ ] Whether the band is filled or only outlined; double clicks; whether a group sent to one place spreads out.
- [ ] The pending picks not traced: kind 2 (attack-target mode) and the orders kinds 2–5 give.
- [ ] [M12] A unit marker's gap figure (the record's slot 5), its bar frame on page9, the clan's sign, and where its name, icon and bars stand.

**Placing and building**

- [ ] [M12] The site test's path search and its hall-way areal test (interface `0x303`, vertex bit 1). The basement's triangulation is [above](#walking-into-the-large-factory).
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

## Mission 04, *Teleport*

Added on 2026-09-15 against `f26ffd8`, after M13 made the mission winnable end to end along the
briefing's route. [40-command-mode](docs/40-command-mode.md#not-established),
[16-research](docs/16-research.md#not-established), [27-ownership](docs/27-ownership.md#not-established),
[31-packages](docs/31-packages.md#not-established), [34-progression](docs/34-progression.md#not-established),
[24-motion](docs/24-motion.md#not-established), [30-turrets](docs/30-turrets.md#not-established),
[35-hud](docs/35-hud.md), [41-commander](docs/41-commander.md#not-established)

**Where the engine is weaker than the game**

- [ ] [M13] Which of Mission 04's hero, helicopter and HQ holds no mind. The recording's factory shows one free of three once all three are the player's, and Mission 02's shows the hero holding one; the engine lets the HQ taken by Enter hold none, or the build could never start. Mission 03's "4 free, five built" may be the same question.
- [ ] [M3] The ground contact runs once a frame (message `0x1c`, read), but the engine runs it at state steps, and moving it waits on walking from the held ground face. Until then a hero pressed against Mission 03's Small Bunker door as it sinks is pushed down between steps; the bunker walk passes, but only just.
- [ ] The helicopter's capture pace: the recording takes 90 s to the factory and 81 s on to the research centre, the engine 54 s and 42 s at its live 14 m/s. And what held it still near (715, 847) from 362 to 402 s.
- [ ] The briefing's route is checked only by the engine's own test: the recording's player walked to the Teleport, so the large flyer's build, its flight to the HQ, boarding it, flying up to the plateau and setting down there were never compared with the game.
- [ ] Turret pitch and altitude as the user saw it. Measured: pitch moves neither a flyer nor its eye. Explained as rising ground lifting a low flyer, which never sinks back without F (read, `0x1001b3c3`); not confirmed in a window.
- [ ] Tooltips (the research rows' 6251/6252, the batch button's 6243), the maps' route lines and the selected unit's white outline: not drawn.
- [ ] [M13] The research box's name colour and its clip 5; the part preview draws larger than the game's.
- [x] ~~Mission 02's leaving places moved with the joined sphere~~ — closed in M14: the ground contact holds a machine about its node sphere's centre with the joined sphere's radius (read), so a landed L-2f stands 9.67 over flat ground and the whole island lets the hero out. The recording's 11 on landing and the hero's 3 on the same ground fit it ([24-motion](docs/24-motion.md#finding-the-ground--read), [39-boarding](docs/39-boarding.md#against-the-recording--seen)). Open still: see [Mission 02](#mission-02-the-constructor).

**The HQ's command view**

- [ ] [M13] Which bound a unit record's `+0x98` is: 8 × it is how far the camera rides behind the HQ. The engine takes the chassis sphere, 61 m, which the recording favours over the whole bound's 104 m and the cylinder's 38.7 m.
- [ ] [M13] What the game does when an HQ is lost in its own mode 3 (nothing read pops it), and how the stack reads after Enter in telepresence aboard an HQ.
- [ ] Why the HQ reads "[no order]" in its cockpit at 160 s after "[standing]" at 94 s. The engine's capture-standby departure gives it Standby.

**Capture and the maps**

- [ ] [M13] How a walker's path joins the hall way (`MGraph`), so which exit a capturer takes (the engine: the shortest whole way); whether a flyer touches down or hovers at its landing corner; and a flyer against a building's walls outside the hall way (the leg to exit 67 skirts the factory's west side).
- [ ] [M13] The escape's damaged-node test, and which paths a unit leaves a building by (the engine: back along the hall way).
- [x] ~~[M13] A contour vertex's areal flag word, which the engine has no areals for~~ — closed in M14: the engine reads the areal map, and a vertex counts on a walkable areal.
- [ ] [M13] The maps' contact list: the engine marks what lies in a player unit's radar range, not the list the run loop empties and each unit's takt refills.
- [ ] The sphere behind `IBuilding` slot 15.
- [ ] Why the Teleport's map icon looks white at about 412 s, as if selected, when a main teleport's capture selects nothing.

**Research**

- [ ] [M13] Whether a centre's ore take reads the ore it holds or the ore delivered to it.
- [ ] [M13] How often a building's takt runs (the engine: every tick), which bounds how late "Research complete" can come.
- [ ] What becomes of a research when its centre is captured, upgraded or destroyed mid-way (the engine keeps the queue with the building).
- [ ] Whether anything besides the task marks an item researched (scripts, saves), and the AI clans' own research orders (order 16), which are not modelled.

**The Main Teleport**

- [ ] What the teleport's class-25 parts do with the power byte's state `0x20`, and its class-29 parts with 1.
- [ ] What building interface `0xb` slot 16 asks about a place's node; what property `0x208` is (a network mirror flag is a guess); whether anything sets `pTeleFunc`.
- [ ] Whether a player-driven small unit can take the Teleport's pod.
- [ ] [M14] The places besides a dock's and a main teleport's (loading places) do not tick by the place rule yet; the place timer's random source; whether a destroyed generator stays in `World3D.dll`'s queue 3.
- [ ] [M14] What the game drives a dock's `f_recharge_*` glow with, whose own time mode is 0, a value set from outside.
- [ ] [M14] The AI's camouflage in play. Its repair decision and its trip to a dock are in (the engine: `diff_strong`'s 0.8 and 0.9 for `Decision_RepairOn` and `_Off`, since the profile a unit holds is not read; the nearest dock its size class fits, since the pick `0x10023b60` is not read, and never a building's own repair system).
- [ ] Why the hero's panel dims in the pod room, and why the chamber's glow reads greyer than the recording's (the dawn scene colour is the guess).
- [ ] Message 16, the helicopter in route 2, has no test.

**Movement**

- [ ] [M13] Whether the AI's aiming reaches the turret lock's lead, and what spin a unit let go keeps until the Wizard writes one.
- [ ] [M13] What a node reaching its last damage stage takes out of the load (the engine: its own weight and armour).

## Not looked at at all

- [ ] The network protocol.
