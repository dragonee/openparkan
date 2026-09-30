---
description: Analyse a playthrough video of the game — map its missions, set each against the engine, and write up what differs
argument-hint: "<video path> [missions it covers, e.g. C02M01 C02M02]"
---

# Analyse a playthrough video

Watch a recording of the original game, check the missions it plays against the
engine, and write down what the engine does differently. The deliverable is the
write-up, not a fix: report, and let the user choose what to implement.

`$ARGUMENTS` names the video and, optionally, the missions it covers.

The mechanical work is `analysis/playthrough.py` (read its docstring): contact
sheets, single frames, bursts, the briefing's fade-in, video/engine briefing pairs
and the message box's text. It needs `ffmpeg`, `tesseract`, and a release build of
the engine (`cd engine && cargo build --release -p parkan`). Its outputs go under
`.playthrough/`, which git ignores. **The frames are the game's own imagery and
stay out of the repository**; never copy one into a committed path.

Use `pt` below for `uv run analysis/playthrough.py`.

## 1. Find the game's picture

`pt probe VIDEO`, then `pt frame VIDEO <a time in play>` and look at it. A let's
play puts a facecam or a border beside the game: find the game's own 4:3 picture
in the video's pixels, as `W:H:X:Y`. A column scan of the frame finds a border's
edge (Part 3 of the Iron Strategy let's play: `2872:2154:964:3` of 3840 × 2160).
Pass it once as `pt --crop W:H:X:Y …` and check it with `pt grab`: no border, no
facecam, the HUD's corners whole.

## 2. Map the video's timeline

`pt sheets VIDEO --every 5` makes contact sheets of 16 frames, each stamped with
its time. Read every sheet and write down, per mission:

- the campaign menu and the mission picked;
- the briefing: the loading screen, its black lead-in, its first shot, and its end;
- the objectives screen, the first cockpit frame, and play;
- every screen change: a factory or research screen, command mode, a boarded
  bot's cockpit, the game menu, a reload or restart;
- the end: *MISSION COMPLETE* or *MISSION FAILED*, and what came just before it.

The briefing's title in its top bar names the mission; `uv run openparkan
missions` gives its directory.

## 3. Read what each mission is made of

For each mission, before judging anything in the video:

- `uv run openparkan mission <dir> --list`: the clans, objects with their logical
  ids, routes and objectives;
- `uv run openparkan briefing <dir>`: the waypoints, their durations and subtitles;
- `uv run openparkan behaviour <script>` for **every** clan's script the mission
  names, and the mission's `mission.cfg` (`prebuild`, `script`) and
  `messages.cfg`;
- `uv run openparkan resources --text C0xM0y` for its messages' text.

Turn the scripts into the events the video should show: a building's owner by its
logical id (function 52; a `CLASS_BUILDING|n` constant *is* the id), a unit in a
route (function 32 takes a route and a **logical id**), a count reaching 0
(function 31), a timer (59/60), an order (15), a `mission.cfg` console line (57).
Check the engine answers every function the scripts call: the dispatch is
`engine/crates/parkan-world/src/progress.rs`, and its `STAND-IN` lists what it
leaves unanswered.

## 4. The briefing

The black lead-in lasts the waypoints before the first shot. `pt fade VIDEO <t0>
<t1>` prints the picture's brightness frame by frame: its first rise is that
shot, so the video's time of briefing time 0 is the rise less the lead-in. Then
`pt pairs VIDEO --mission <dir> --offset <that> <one time inside each waypoint>`
puts the video beside the engine's `--briefing-at` at the same moment. Check the
camera and its cuts, the subtitles, and every object in shot — its colours and
emblems, effects and lights — and the sky and the ground.

## 5. The objectives screen and the first cockpit

`pt grab` the objectives screen and the first cockpit frame, and render the
engine's (`--skip-briefing --objectives`, and `--skip-briefing --map --ticks 60`,
`--screenshot`, `--size 1400x1050`). Compare the text, the wingmen list, the
panels' names and orders, the weapons, the radar.

## 6. The event log

`pt ocr VIDEO --box W:H:X:Y` reads the message box once a second and prints a line
whenever it changes. The box stands at the top middle in the cockpit, and at the
bottom right on a factory screen and in command mode: find it in a frame first,
in the video's own pixels. Expect *"Building is captured"*, *"Objective is
completed"*, *"Game saved..."*, the information assistant's lines (match them to
`resources --text`), *"Vacant vehicle detected..."*, *"Risk area! Landing
impossible."*. The wingman menu's rows show through the same box; ignore them.

Tie each line to the script trigger that raised it, and each screen change to what
the hero did. `pt burst VIDEO <t0> <t1> --fps 4` shows a moment frame by frame: a
capture and the screen it opens, a death, a cut.

## 7. Replay each event in the engine

Reproduce each event headless and compare. The engine's switches are in
`engine/README.md` (and `crates/parkan/src/main.rs`); the ones this needs most:
`--mission`, `--skip-briefing`, `--headless --ticks N` (prints the hero, its target,
the objectives' states and every `says:` line), `--god-mode`, `--at X,Y,YAW,Z`,
`--pod <building>.dat`, `--hq`, `--take <unit>.dat`, `--drive <path>`,
`--face <name>,<distance>`, `--look X,Y,Z,TX,TY,TZ`, `--objectives`, `--map`,
`--screenshot`. For each: does the trigger fire, does the objective complete, does
the message play, does the screen open with what the video shows in it?

Two checks before calling anything a difference:

- **Read it at full size.** The game font's digits blur at contact-sheet size — a
  "2" reads as a "3". `pt grab --width 1400` and crop the engine's screenshot the
  same way before comparing names or numbers.
- **Compare like with like.** A factory captured 30 s into the engine is not the
  one the video captured eight minutes in; run the engine to the same state, or say
  what differs between the two.

## 8. Write it up

- **`playthrough-feedback/feedback.md`**: a `# Campaign NN` section naming the
  recording, and `## Mission NN - <title>` with *Mechanics* and *Visuals* checkbox
  lists, each item saying what the recording shows, with its time, and what the
  engine does instead. Leave out what matches; say in one line at the section's
  head what was checked and matched.
- **`OPEN-QUESTIONS.md`**: a difference whose game behaviour is not read yet goes
  in as a new line in its area (*"Raised <date> from checking <mission> against a
  recording"*), with what is known and exactly what is not. Do not implement a
  guess.
- **`COMPLETED-QUESTIONS.md`**: a queue line the recording settles moves there,
  closed as *seen*, citing the recording by its title and time — and a doc's
  *seen* note cites it the same way.

Commit `feedback.md` and the queue separately, staging by path. Then tell the user
what matched, what the engine gets wrong in play, what is only visual, and what is
queued as unread — and ask which to implement.
