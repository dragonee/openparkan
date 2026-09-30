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
  - [ ] The three heavy warbots never arrive after the HQ is captured
    - The script calls function 57, which runs `mission.cfg`'s `script1`..`script3` - `create(918, 683, 10, 4, 22lwhl1.dat, 0)` and two more - and the engine leaves it unanswered
    - The bonus objective "Destroy enemy patrols" completes without them, since it counts Enm2's robots
    - The same function places the Teleport on C04M02, and is called on C02M03 and C05M01
    - On hold until `create`, `bcreate` and `death` are researched: queued in OPEN-QUESTIONS.md, "AI, scripts, packages and economy"
- Visuals
  - [ ] The last buoy (s_tree_28) is olive and orange, should be lilac with a pink cap and a blue beam (briefing, 10:16)

