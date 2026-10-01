# Campaign 00

## Terrain

- [ ] Terrain cutout for buildings is jagged now, make it into a nice contour

## Mission 01

- [x] z-layering of leaves (seems reverse - leaves in front are hidden behind further leaves)
  - see 1-layering.png
- [x] left-right strafing should be reversed when moving backwards. Left should go 225 deg, right 135 deg.
- [x] Running on left/right bottom panel is jerky - can you smoothen the movement?
- [x] Laser beam duration is too short - it should fire between 500-800ms with a red beam
  - see 1-laser-beam.png
- [x] Explosion sounds should dampen with distance
- [x] Add Zoom mode (with z key), that zooms in. Images in directory
- [x] Add camera toggle mode (with c) - 5 positions total, right near, right far, left far, left near, back. All with small movement of camera in about 200ms. Images in directory
- [x] Light behaves very strangely on the surfaces, seems to be hidden behind them
- [x] Selecting a guided weapon should show additional homing indicator coming to the crosshair for the duration of the homing, and two sounds should play "homing" and "locked in"
- [x] Tilda should show a stylized menu and not show bots ids. See 1-orders.png
  - [x] The menu should be centerer to the window size
- [x] Bots should play a sound "selected" when captured from neutral clan
- [ ] There is additional unnecessary sound being played on repeat when hero is selected
- [x] There is still no background music being played in the background
- Warbot movement
  - [x] When turret rotates, the yaw axis should slowly turn towards the turret direction
  - [x] Turret pitch should not affect altitude - only altitude up and down buttons (R/F)
  - [x] Speed of the warbot seems too high - 112, when in the video it's only 40
  - [x] There's no sound of the engine when inside the flying warbot

## Mission 02

- Mechanics
  - [x] Charging station does not work inside Factory, when I step on it
  - [ ] Capturing a Factory jumps me to the middle of the control panel, at this moment the building menu should show up. Today it's too long. It should take about 3s.
  - [x] Warbot escaped from factory passing through the wall. Make it respect collisions and door opening
- Visuals
  - [x] Factory chimney animation is too fast and yellow instead of black smoke
    - 2026-09-30: its streams also run at half pace now, and each puff takes its own far end and size, as C03 M01's volcano was measured against Part 5 of the let's play (see Campaign 03)
  - [x] Interior icons - computer, charging station - should be glowing green, are grey now
  - [x] Lights in Factory do not lie on normals of the building, see (factory-lights.png)
  - [x] The interiors of Factory lie beneath a black color. Until you step in you don't see the next segment
  - [x] A door is not visible from the outside
  - [x] Shooting a door should open the door
  - [x] Door opening sound is barely present on a Factory (works well on Alien Power Plant).
  - [x] Charging station has its own effect and sound
- Bot Designer Screen
  - [x] Quality of life - on each tab (weapons, ammo, systems), selecting an item by choosing an item should cycle the list on the left to the next item. So I pick 1st weapon, left list changes to slot 2

## Mission 03

- Visuals
  - [x] The Large Factory's chimney smoke shows dark and crisp through the lode's plume in front of it (seen in play, 2026-09-30)
    - The engine drew every additive effect sprite first and every see-through one after, whatever their distance, and effect sprites write no depth
    - Fixed 2026-09-30: every effect sprite is drawn far to near, as the game's type-3 layer 6 files them (docs/11, "Effect sprites are drawn far to near"), so the nearer plume lies over the smoke

# Campaign 02

Checked against "Let's Play - Parkan: Iron Strategy, Part 3" (UfyUzq8k2kY). Both briefings,
the objectives, the captures, the route triggers, the HQ's command mode and the flyer's
take-over already match the recording.

Missions 03 and 04 checked against "Let's Play - Parkan: Iron Strategy, Part 4" (Qqs8_i9IeUU)
and its bonus "Part 4.5" (C9kZR__WpIQ), which plays Mission 04 again. Both briefings' black
lead-in, camera, cuts and subtitles, both objectives screens, the first cockpits' wingmen and
weapons, the captures and their messages (C02M03's *"Well done!"*, C02M04's road and Outpost
lines), function 57's three flyers, the vacant MTW-1 Warrior announcing itself at 397-399 m,
C02M04's objective completing on the second generator, the plateau tower killing the hero on
foot from 120-137 m, and both wins (the warehouse warbot taken at 5 m, the teleport's field)
already match the recording.

## Mission 01 - The Iron Monster

- Mechanics
  - [x] Factory screen always shows the design being produced - clicking another design's button does not switch to it
    - Capturing a Factory should stop its production (the enemy's SSW-X Warrior keeps building after the capture)
    - The factory should by default show the design produced, and select it on the design button
    - Clicking between designs should show them even while a production is going on
    - Switching to a different command panel and back should go back to the design actually produced
    - In the recording (9:36) the captured Factory shows the prebuild SWW-X Warrior with no production
- Visuals
  - [ ] Outpost (shang03) facade is orange with an arrow emblem, should be bright yellow with a diamond emblem (briefing, 1:22)
  - [ ] Generator (gener01) has a large yellow-green glow at its core, should be a small green light (briefing, 1:27)
  - [ ] Birds are rust-red, should be dark green-grey with glowing spots (4:26)
  - [ ] Clouds have bright yellow streaks and the ground is washed out compared to the recording (both briefings)

## Mission 02 - Ballen's Crossing

- Mechanics
  - [x] The three heavy warbots never arrive after the HQ is captured
    - The script calls function 57, which runs `mission.cfg`'s `script1`..`script3` - `create(918, 683, 10, 4, 22lwhl1.dat, 0)` and two more - and the engine leaves it unanswered
    - The bonus objective "Destroy enemy patrols" completes without them, since it counts Enm2's robots
    - The same function places the Teleport on C04M02, and is called on C02M03 and C05M01
    - Researched and fixed 2026-09-30: the console's `create`, `bcreate` and `death` are read (docs/15, "What the console's `create`, `bcreate` and `death` do"); the three arrive for Enm2 on the ground at (917-919, 683-684), on its list before the bonus objective asks
- Visuals
  - [ ] The last buoy (s_tree_28) is olive and orange, should be lilac with a pink cap and a blue beam (briefing, 10:16)

## Mission 03 - The Lost Key

- Mechanics
  - [x] The warbot hidden in the Warehouse (22mwlk1, id 28) sits sunk in the Warehouse's floor
    - The recording shows it upright on its legs, taller than the hero, in the briefing (1:43, briefing time 71 s) and as the hero walks up to it (15:13-15:15)
    - The engine holds its body 0.16 m over the floor face it stands on (z 36.6), so only the top of its hull shows over the ledge; the same unit put on open ground stands 3.02 m over it, as C02M04's 24mwlke1 does (3.04)
    - So a walker standing on a building's floor loses its legs' height, where on the landscape it keeps it
    - Fixed 2026-09-30 as a stand-in: the Warehouse's ceiling presses its 5.87 m agent sphere down 2.85 m a tick, and the read push lands after the ground contact; the body is now drawn where the contact holds it, and the simulation keeps the push (queued: what keeps it upright in the game)
- Visuals
  - [x] The target panel's name box stays empty for every building
    - The recording names them: "Small Bunker" at the first cockpit (1:50), "Small Generator" (2:45), "Small Warehouse" (15:08), "Light Tower" on C02M04 (16:27)
    - The engine fills the panel's names for units only; docs/35's building names (strings 6031-6098 by Type and size) are used by command mode alone
    - Fixed 2026-09-30: a building is named by its Type and size class on the target panel, with no status under it, a building put up in play too
  - [ ] The Small Factory (splant01) is dim and its pad unlit (briefing time 32 s, 1:04)
    - The recording shows the pad's octagonal border solid lime-green and the factory's roof wing lit yellow-green, crisp through the haze
    - The engine draws the border dark with four yellow lights at its corners and green ones at the door, and the roof orange-brown under heavy haze
  - [ ] As on Mission 01, the clouds carry bright yellow streaks and a sun glare, and the ground is washed out, in every briefing pair

## Mission 04 - The Last Bastion

- Mechanics
  - [x] Capturing a Light Tower does not put the player at its gun
    - In the recording, a second after *"Building is captured"* the view is the tower's gun, the own panel reads "Light Tower", and the player shoots tanks with it until Esc: the valley's tower (mtow01) at 26:04-26:15, the plateau's (mtow02) at 37:23-37:42
    - docs/27 ("Capture", "What the modes show") reads it: standing on the pod of one's own tower switches the view to state 6, the tower's manual control, unless its turret is gone
    - The engine captures the tower and leaves the hero on foot in the pod room: it has no mode 6
    - Fixed 2026-09-30: the pod opens mode 6; the tower is driven from its turret's camera and cockpit with `M1.TBL`, its guns fire at the button, and Esc hands them back; a tower whose turret is gone opens nothing
  - [ ] The valley's Light Tower keeps its gun mast up after its turret is shot off
    - In the recording the target panel's silhouette shows the mast up through the 40 s of the hero's attack from 20:42, the turret red from 21:27, and the mast sunk to a stub by 21:36
    - The engine raises every tower's mast once and keeps it up (a stand-in; queued)
  - [ ] Unexplained: the hero dies in the valley Light Tower's pod room (21:53.5)
    - At full life, walking up to the pod of the tower whose turret it has shot off, it is lost in one white flash and the mission fails; on the reload it captures the same tower (26:04)
    - The engine: nothing harms a hero standing there. What killed it is not read (queued)
- Visuals
  - [x] The target panel's name box stays empty for buildings (see Mission 03): the first cockpit's target reads "Light Tower", 358 m, in the recording (16:27) and nothing in the engine
  - [ ] The Main Teleport's tower and arch look different (briefing time 49.5 s, 16:18.5)
    - The recording shows the tower's mast closed, a blue-violet shaft capped by a blue cylinder, and the arch's opening a pale blue-white face
    - The engine spreads three red vanes on a grey shaft and puts an orange glow at the arch's foot
  - [ ] As on Mission 01: the Outpost's facade carries a teal diamond in the recording and an orange light in the engine (briefing time 11-16.3 s, 15:40-15:45); the Generator's core is a small green light in the recording and a large yellow-green glow in the engine (43.5 s, 16:12.5); and the clouds and washed-out ground

# Campaign 03

Mission 01 checked against "Let's Play - Parkan: Iron Strategy, Part 5" (PfAg6zSe-yM), which plays
it from the campaign menu to its win. The briefing's 4.4 s black lead-in, its camera, both jump
cuts through black (within 0.25 s) and its subtitles; the objectives screen; the first cockpit's
weapons, counters and radar; both *"Vacant vehicle detected..."* (the SWB-1 Builder and the MTW-2
Warrior) and route 1's information line; route 0 waking the Transformer (LSW-1 Warrior); the Small
Factory's capture opening its empty screen and the Small Bunker's opening command mode on the
same framing; the builder's upgrade of the Small Factory to a Medium one; the builder's and the
warbots' captures at pods; the names on the target panel and the pages (Small Factory, Medium
Mine, Medium Res. Center, Tiny Tower, the SWC, MTW, MWW and LSW units); Z's frameless zoom with
its narrowed radar cone; and the win on the Research Center's capture already match the recording.

Mission 02 checked against "Let's Play - Parkan: Iron Strategy, Part 6" (-yNnsqudMzw), played on
the easy level, and its bonus "Part 6.5" (9SBZOCWv_vE), which plays it again from the campaign
menu. The briefing's 4.5 s black lead-in and 4 s fade-in, its camera, both jump cuts through black
and its subtitles; the objectives screen; the first cockpit's weapons, counters and radar;
`c3m2e2`'s two raids on the easy level's clock (the first warning at 12:44, the clock past 622 s,
and the second at 22:32, past 1120 s, at 7.3–7.5 s a takt); the Small Bunker's guns taken from command mode; Enemy 2 taking back the generator
the player captured within about a minute, the player's generator count falling as it does, and
the bonus objective completing once the enemies' robots are destroyed (33:32 in Part 6.5);
*"Vacant vehicle detected..."* by the LSW-1 Warrior, Enter capturing and boarding it, objective 2
with the information assistant's *"Excellent, captain!"* over the *"Objective is completed"* it
replaces, route 0 completing objective 3, and the win once all three primaries are in; and the
*MISSION FAILED* panel's three lines over the fallen hero's camera already match the recording.

## Mission 01 - The Silver Eye

- Mechanics
  - [ ] F7's quick save says nothing and saves nothing
    - At 3:09 the message box reads *"from: System / Game saved..."* (string 6246, beside 6245 *"Quick Save"*) and play goes on
    - `ui_other.man` and `addition.man` bind `CMD_QUICK_SAVE` to F7 and `CMD_QUICK_LOAD` to F8; the engine binds neither (queued: what they write and read)
  - [x] The Energy row holds still at 50% where the recording's wanders about it
    - With the player holding one of the map's two generators, the recording reads above half at 4:46 (52%), 6:25 (51%), 7:15 (54%) and 7:25 (53%), and 47–48% between; the engine never passes 50.0%, the player's 10.07 of the map's 20.14
    - docs/23 reads the row off each clan's distributor totals, and the step totals `Transfer_Power_Out × dt` over its own 192–255 ms, so the player's share should run 43–57% as the two clans' steps fall (*derived*); the engine divides each clan's totals by its dt first, which pins the share
    - Fixed 2026-09-30: each clan's totals are its last step's as they stand; the row runs 43–57% about 50% with the bunker held
  - [x] The enemy plans to capture the player's generator from its first takt
    - `c3m1e`'s `Problems0` compares function 3 with `fPlentyEnergyLevel` 0.5 and, at or below it, raises `PBM_BUILDING_CAPTURE` (0.8) on the generator function 71 finds, the player's at (1271, 494)
    - docs/15 reads function 3 as the clan's power available minus demanded, which the enemy's own generator keeps above 0.5; the engine leaves 3 unanswered (`progress.rs`'s STAND-IN) and answers 0
    - Found checking the scripts; the recording does not show the enemy going for that generator
    - Fixed 2026-09-30: function 3 answers the clan's power available less demanded at its last step; the raise the first takt makes, before any step has given, is not reloaded and runs out
- Visuals
  - [x] The burning plant and the smoking volcanoes are drawn cold
    - The recording shows the dish-shaped plant by the bridge (`s_tree_31` at (634, 914)) under an orange flame and a column of smoke (briefing time 34–38 s, 1:21–1:25), and the volcanoes' craters (`s_tree_33`, on the peaks at z 105–160) glowing under black smoke (4:10–4:25, 7:35, 19:20–20:10)
    - The engine starts no effect for a placed tree or stone. docs/13 reads the loader running block entry 0's group whatever the owner: `s_tree_a_31.ctl`'s makes `tree_flame_30`, its sound and `tree_smoke_31`, and `s_tree_a_33.ctl`'s `tree_light_33a` and `_33b`
    - Fixed 2026-09-30: trees and stones run their load groups as buildings do (222 in 18 missions), and a felled one takes its effects with it
  - [x] The volcano's smoke rises and flickers about twice as fast as the recording's (seen in play, 2026-09-30)
    - Measured frame by frame at 60 fps (4:19.4–4:24.4): the recording's smoke rises at about 0.63 of the engine's speed against its own puffs' width, and its flame beats at 5.3 Hz where the engine's 0.08 s interval beat at 12.5 Hz
    - The engine plays a stream's clock, emission and ageing as `Effect.dll` reads; what slows the game's is not read (queued)
    - Fixed 2026-09-30 as a stand-in: the streams of every load group's effects and of a lode's plume run at half pace; the construction sphere's and every gun's and round's keep theirs
  - [x] The plume climbs one smooth cone where the recording's billows in separate puffs (4:19–4:24)
    - The spawn gives each particle its own far end and size, a uniform in ±half of the block's jitter (`Effect.dll:0x10011e81`–`0x10011f7f`), which the engine left out, so every puff took one line
    - Fixed 2026-09-30: each stream particle draws its own far ends as it leaves; the volcano's smoke ends 100 ± 30 m up and ± 10 m aside
  - [ ] The crater's flame is about twice as large in the recording, and its brightness varies a third as much as the engine's (4:19–4:24; queued with the stream's pace)
  - [ ] No weather: the recording's red dust falls the whole mission (the sky's snow spell 00:00–23:59 with `DUST_ADD` in its slot, e.g. 0:55–1:35 and every cockpit frame after), and lightning strikes at 1:55; the engine draws neither (queued: how the weather is drawn)
  - [ ] The engine draws a grey-brown cloud layer with hard, stepped edges over the upper sky, in every briefing shot from briefing time 15 s and in play; the recording's sky is a smooth red, magenta later, with faint wisps at most (0:57–1:25, 2:40, 12:20)
  - [ ] The orb the two standing figures hold is a small orange ball in the recording and a large rayed orange glow in the engine (briefing time 10–19 s, 0:57–1:06), as Campaign 02's generator core
  - [ ] The lava glows bright red in the recording (briefing time 38 s, 1:25; 14:15) and is a dim pink-red in the engine
  - [ ] As on Campaign 02, the engine's haze is far heavier and its ground paler: command mode's first view over the Small Bunker's roof (4:46) is dark, crisp ground in the recording and a pink wash in the engine, and so are the Research Center (briefing time 43–46.5 s) and the far hills of the first cockpit (1:42)

## Mission 02 - The Convoy

- Mechanics
  - [x] The raid's winged SSM never reaches the bunker from range
    - In the recording the MWW-4 Warrior, raid 1's unit 15, stands 345 to 292 m off the Small Bunker (13:49.5–13:53.5), and its missile's trail runs from it to the bunker, which it strikes at 13:54.2
    - In the engine the same unit fires from 159 m and its missile meets the ground 13 m ahead; with the player at the bunker's guns it drives in to about 20 m first and strikes the bunker. Either way its own 45 m blast kills it in that tick (585 s and 584 s, on the medium level)
    - docs/29 reads a winged SSM (`bm_m_04`, 45 m/s, turning 0.5 rad/s) flying straight at its target's node-sphere centre; what carries the recording's over 300 m of ground is not read (queued)
    - Fixed 2026-10-01 from two reads the engine had short. An attack never nears a building (`Behavior.dll:0x1002777d` clears the nearing flag), so the fire control holds the bunker from the first pick on, where the engine waited until it was within 200 of its point. And a round of more than 10,000 is held for a building, so the raider no longer spends its missiles on the hero it passes. The first missile now leaves 363 m off and lands with the raider 289 m off
  - [x] The hero at a building's guns cannot be hurt
    - In the recording the player is at the Small Bunker's guns (its own panel reads "Small Bunker") when that missile strikes the bunker: one white flash, and *MISSION FAILED* over the fallen hero's camera (13:54.2–13:54.6). The player loads the quick save (14:00)
    - docs/27 reads mode 6 as clearing only the driven unit, with the hero left standing in the pod room. The engine sets `driving` for mode 6, so `hero_away()` holds and the hero's target leaves the world: the missile's blast, 17 m from it in the engine's run, leaves it whole
    - Whether a blast reaches a hero in the room under a building is C02 M04's queued question; this is its second sighting
    - Fixed 2026-10-01: only boarding a bot takes the hero out of the world; at a building's guns or driving a unit from a command view it stands in its room, struck as anything there is, and the raider's missile on the bunker's roof kills it
  - [x] Objective 1 does not reopen when a generator is lost
    - In the recording *"Objective is completed"* shows at 53:57, as the third generator becomes the player's. Enemy 2 takes its generator back by 59:05, and when the player's Comm. Center retakes it, *"Objective is completed"* shows again (59:15–59:17). Part 6.5 does the same: 24:34–24:35, then 32:20–32:21
    - `c3m2p` calls `OBJECTIVE_PROGRESS 0` when `fn34(BUILDING_GENERATOR)` falls from 3, and completes the objective again when it comes back; docs/34 reads `OBJECTIVE_COMPLETE` as acting only on an open objective, so the call reopened it. Nothing shows in either message box as it does (54:30–59:15)
    - The engine leaves `OBJECTIVE_PROGRESS` unanswered (`progression.rs`'s STAND-IN): the objective stays complete through the loss, says nothing when retaken, and the mission can be won with a generator in Enemy 2's hands
    - Fixed 2026-10-01 from the read: `OBJECTIVE_PROGRESS` puts a complete or failed objective back to open without a word (`iron3d.dll:0x10060e44`), `OBJECTIVE_COMPLETE` completes only an open one, and `OBJECTIVE_FAILED` now fails it (state −1) besides showing its string
  - [ ] As on Mission 01, F7's quick save does nothing: the recording saves at 8:38, 12:57, 13:18 and on, and after the failure the panel's L loads the save (13:55–14:00). The engine has neither (queued)
- Visuals
  - [x] The target panel never says *"Dangerous!"*
    - The recording's raider reads *"MWW-4 Warrior"* over a red *"Dangerous!"* at 13:44 and 13:49.5–13:53.5, and the LWW-3 at 30:45
    - docs/35 reads it for any unit not the player's that carries a gun whose node has life left and whose round does at least 10,000. The engine left it unwritten, a stand-in, and wrongly counted among this mission's matches until 2026-10-01
    - Fixed 2026-10-01: a gun keeps its round's damage, and the panel says *"Dangerous!"* under a unit carrying one of 10,000 or more
  - [ ] Every building wears the arrow emblem
    - The recording's emblem follows the owner. Enemy 1's Medium Mine wears cell 6 of the insignia sheet `PG27`, a filled triangle over a bar (briefing time 11 s; 37.4 s in Part 6.5, 0:55 in Part 6). The player's Small Bunker wears cell 0, the arrow (2:15 in Part 6.5)
    - `B_LBL_01`'s eight tracks name cells 0, 6, 5, 4, 3, 2, 1, 7, so Enemy 1, clan 1, wears track 1 and the player, clan 0, track 0. The engine draws track 0 on every building (queued: who sets the track). Campaign 02's Outpost emblem may be the same thing
  - [ ] The Medium Mine's plume and two purple lights are missing in the engine (briefing time 8.5–11 s, 61–62 s)
    - On the Medium Mine, a dense dark plume rises from its tower's top and the platform's underside is lit purple. The engine draws thin grey streaks there and no purple light, and puts a large green rayed glow over the drill at 11 s
    - Enemy 2's Large Factory's mast is lit purple, and the engine lights it with nothing
  - [ ] The generator's core, the light between the horned arch's two horns, is a small yellow light in the recording and a large rayed orange glow in the engine (briefing time 8.5, 11, 41.5, 46.5, 54 s). It is Mission 01's "orb" and Campaign 02's generator core
  - [ ] As on Mission 01: no falling red dust, the hard-edged cloud layer, and the heavier haze and paler ground, in every briefing pair
