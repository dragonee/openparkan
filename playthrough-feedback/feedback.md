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
  - [x] Interior icons - computer, charging station - should be glowing green, are grey now
  - [x] Lights in Factory do not lie on normals of the building, see (factory-lights.png)
  - [x] The interiors of Factory lie beneath a black color. Until you step in you don't see the next segment
  - [x] A door is not visible from the outside
  - [x] Shooting a door should open the door
  - [x] Door opening sound is barely present on a Factory (works well on Alien Power Plant).
  - [x] Charging station has its own effect and sound
- Bot Designer Screen
  - [x] Quality of life - on each tab (weapons, ammo, systems), selecting an item by choosing an item should cycle the list on the left to the next item. So I pick 1st weapon, left list changes to slot 2

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

## Mission 01 - The Silver Eye

- Mechanics
  - [ ] F7's quick save says nothing and saves nothing
    - At 3:09 the message box reads *"from: System / Game saved..."* (string 6246, beside 6245 *"Quick Save"*) and play goes on
    - `ui_other.man` and `addition.man` bind `CMD_QUICK_SAVE` to F7 and `CMD_QUICK_LOAD` to F8; the engine binds neither (queued: what they write and read)
  - [ ] The Energy row holds still at 50% where the recording's wanders about it
    - With the player holding one of the map's two generators, the recording reads above half at 4:46 (52%), 6:25 (51%), 7:15 (54%) and 7:25 (53%), and 47–48% between; the engine never passes 50.0%, the player's 10.07 of the map's 20.14
    - docs/23 reads the row off each clan's distributor totals, and the step totals `Transfer_Power_Out × dt` over its own 192–255 ms, so the player's share should run 43–57% as the two clans' steps fall (*derived*); the engine divides each clan's totals by its dt first, which pins the share
  - [ ] The enemy plans to capture the player's generator from its first takt
    - `c3m1e`'s `Problems0` compares function 3 with `fPlentyEnergyLevel` 0.5 and, at or below it, raises `PBM_BUILDING_CAPTURE` (0.8) on the generator function 71 finds, the player's at (1271, 494)
    - docs/15 reads function 3 as the clan's power available minus demanded, which the enemy's own generator keeps above 0.5; the engine leaves 3 unanswered (`progress.rs`'s STAND-IN) and answers 0
    - Found checking the scripts; the recording does not show the enemy going for that generator
- Visuals
  - [ ] The burning plant and the smoking volcanoes are drawn cold
    - The recording shows the dish-shaped plant by the bridge (`s_tree_31` at (634, 914)) under an orange flame and a column of smoke (briefing time 34–38 s, 1:21–1:25), and the volcanoes' craters (`s_tree_33`, on the peaks at z 105–160) glowing under black smoke (4:10–4:25, 7:35, 19:20–20:10)
    - The engine starts no effect for a placed tree or stone. docs/13 reads the loader running block entry 0's group whatever the owner: `s_tree_a_31.ctl`'s makes `tree_flame_30`, its sound and `tree_smoke_31`, and `s_tree_a_33.ctl`'s `tree_light_33a` and `_33b`
  - [ ] No weather: the recording's red dust falls the whole mission (the sky's snow spell 00:00–23:59 with `DUST_ADD` in its slot, e.g. 0:55–1:35 and every cockpit frame after), and lightning strikes at 1:55; the engine draws neither (queued: how the weather is drawn)
  - [ ] The engine draws a grey-brown cloud layer with hard, stepped edges over the upper sky, in every briefing shot from briefing time 15 s and in play; the recording's sky is a smooth red, magenta later, with faint wisps at most (0:57–1:25, 2:40, 12:20)
  - [ ] The orb the two standing figures hold is a small orange ball in the recording and a large rayed orange glow in the engine (briefing time 10–19 s, 0:57–1:06), as Campaign 02's generator core
  - [ ] The lava glows bright red in the recording (briefing time 38 s, 1:25; 14:15) and is a dim pink-red in the engine
  - [ ] As on Campaign 02, the engine's haze is far heavier and its ground paler: command mode's first view over the Small Bunker's roof (4:46) is dark, crisp ground in the recording and a pink wash in the engine, and so are the Research Center (briefing time 43–46.5 s) and the far hills of the first cockpit (1:42)
