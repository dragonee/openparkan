# Mission progression — routes, the Mission handler, messages and objectives

A campaign mission moves on through its player clan's script. That script's
`Mission` handler runs every two seconds. Each run it asks where units stand
and how many robots each clan still has, and from the answers it plays
messages and completes objectives. The mission is won when every primary
objective is complete. This page reads that loop end to end, and walks
Mission 01, *Line of Fire*, through it.

**Every claim is tagged**, as in [15-behaviour.md](15-behaviour.md):
- *measured* is re-derived by `openparkan verify`;
- *read* comes from the disassembly at the address given;
- *derived* follows from the two;
- *guess* fits and is not established.

## Routes are tactical areals — *read*, and *measured*

`data.tma` opens with a list of routes ([04-missions.md](04-missions.md)):
an id and a run of points. They are not paths. They are **polygons on the
ground**, and the areal map calls them *tactical areals*.

- **One areal per route.** `IMission` slot 8 (`MisLoad.dll:0x10001380`) is
  handed the system areal map. It calls `SetTacticArealNum` with the route
  count (`ArealMap.dll` slot 29, `0x10021ec0`). For each areal *i* it then
  gives `SetTacticArealVertexNum` (slot 31, `0x10021f60`) the points of the
  route whose id is *i*.
- **Clan zones are something else.** The same slot hands each clan's zones to
  slot 35 (`0x100220e0`, "Create migration areals for clan"). They are not the
  areas a script tests.
- **The inside test** (`ArealMap.dll:0x10017b90`) counts crossings in x and y
  only:
  - a ray runs from the point toward +x, against the outline closed from its
    last point back to its first;
  - an edge counts when one end is at or above the point's y and the other
    below, and it meets the ray at or to the right of the point;
  - an odd count is inside.
  - `openparkan.mission.Route.contains` is this test.

*Measured*: 34 routes on the 29 missions, and on every mission their ids run
0 to *n* − 1. So every route becomes the tactical areal of its own id.

## Who stands in a route — *read*

A unit's behaviour reports its position, and the areal map keeps a list of
logical ids per tactical areal.

- **The report** (`Behavior.dll:0x1000af70`) runs only for a unit that has a
  logical id. The ticks that call it are listed below.
  - It sends a report when the unit has moved **more than 5 by |dx| + |dy|**
    since its last report, or when its navigation areal has changed.
  - The report is system areal map slot 18 (`ArealMap.dll:0x1001fb40`). It
    takes the id out of every tactical areal's list ("Robot … has been removed
    from tactical areal …"). It then adds the id to the list of each areal
    whose outline holds the unit's x and y ("… has been inserted to tactical
    areal …").
  - After the report, the behaviour sends itself message 6 for each areal
    it stands in (not followed).
- **When it runs.**
  - Once, as soon as the id is attached (`0x10005f91`).
  - After that, on `MBehaviour` message 1, the object takt (`0x100059f5`,
    slot 65 at `0x10006350`). That takt needs only flag 2, set when the id is
    attached (`0x10005fc5`), and not the AI's movement or fight flags.
  - The takt fires on a randomised timer: next = now + 31 × 64 +
    rand8 × 46 × 64 ÷ 256 (the words at `0x100039c2`; the timer is
    `0x1004c550`).
  - In milliseconds that is 1.98 to 4.92 s between reports. That the takt's
    clock is in milliseconds is a *guess*.
- **So the list lags** (*derived*).
  - A unit is re-placed at most every 2 to 5 s, and only once it has moved
    5 m or crossed into another areal.
  - A unit that stops stays on the list where it was last reported.
- **The player's hero reports too** (*derived*). Taking a unit over clears
  only the AI's flags `0x10`, `0x20` and `0x40`
  ([29-weapons.md](29-weapons.md#who-may-drive-a-units-guns--read)), not
  flag 2. And 32 of the 36 route tests the scripts make name the hero, which
  the player drives the whole mission (below).

## Function 32: is a unit in a route — *read*, and *measured*

`fn32(route, id)` asks system areal map slot 33 (`0x10022040`) for tactical
areal *route*'s list, and answers 1 when *id* is on it (`ai.dll:0x1000c446`).

- **Arguments.** The first argument is a route and the second a logical id.
- **A correction.** [15-behaviour.md](15-behaviour.md) once read the function
  as two clan numbers. A clan number could not even cover the calls: Mission
  01's script passes 0 to 4 against four clans.

*Measured* over the 36 calls in scripts that missions name, with each operand
resolved to its pool constant or the literal its handler last wrote to it:

- **The first argument** is one of that mission's route ids on 36.
- **The second** is the logical id of a unit placed in that mission on 36.
- **Whose id.** 32 name the mission's hero. The four others are:
  - `tut4_pl2`'s own second unit;
  - a neutral transformer in `c3m2p`;
  - two enemy walkers in `c3m3p`.

## Function 31: how many robots a clan has — *read*

- **The count.** `fn31(clan, mask)` takes the SuperAI that clan *clan*
  created (the per-clan table `0x10055398`, filled by the constructor at
  `0x100016ae`). It counts the entries of that SuperAI's unit list (`+0x8c`)
  whose logical id is set and whose type shares a bit with *mask*
  (`0x1000c2fd`).
- **How units join.** A placed unit joins its clan's list when it is created:
  `iron3d.dll:0x10077511` sends that clan's SuperAI slot 4 event 1 with the
  unit's id.
- **How units leave** on destruction or capture is not read. A unit that is
  captured changes its SuperAI ([27-ownership.md](27-ownership.md)).
- **What Mission 01 needs** (*derived* from the objectives' text): destroyed
  and captured robots leave their old clan's count, and a captured one joins
  the player's.

*Measured*, on Mission 01:

- **The mask.** `CLASS_ROBOT` (`0x01000000`) is set on all 9 units' `Type`,
  the hero's `0x1020000` included.
- **Robots by clan** at the start:
  - Plr, the hero alone: 1;
  - Trgt, the five targets: 5;
  - Enm, one warrior: 1;
  - Ntrl: 2, both warriors (`tut1_mf1.dat` and `helic.dat`).

## When the Mission handler runs — *read*

`iron3d.dll`'s game frame (`0x1005ed85`) does two things, unless the game is
paused (its byte `+0xe8`), the game's state word `+8` is 3
([After the outcome](#after-the-outcome--read-and-measured)), or the level's
state word `+0x710` is 5:

1. It calls SuperAI slot 9 for **the local player's clan only** (the clan
   record at `+0xad0`).
2. It calls slot 3 for every clan.

- **Slot 9** (`ai.dll:0x10001b80`) runs the `Mission` handler.
  - It runs when `timeGetTime` has reached a next-run time, and then sets that
    time to now + 2000 ms.
  - The next-run time is one word for the whole module (`0x10054c68`). That
    is harmless while the frame calls the slot for one clan. Four more calls
    through a `+0x50` pointer's slot 9 in `iron3d.dll` (`0x10064604`,
    `0x10064629`, `0x10067887`, `0x100a2a6a`) were not followed, and may not
    be on a SuperAI.
- **Slot 3** (`0x10001780`) is the clan's takt.
  - It runs every 7000 + rand % 1000 ms.
  - Each run adds 7 to the seconds clock that function 59 reads
    ([15-behaviour.md](15-behaviour.md)), then runs `Problems<n>`.
- **Slot 5** (`0x10001ae0`) runs `Init`, once.
- **The four handlers** the SuperAI looks up by name at construction
  (`0x1000160b`) are `Mission`, `Problems0`, `Mech_GeneratorFound` and
  `Fort_Task_Complete`. Which of the others run, and when, is not read.

So a player script's `Mission` handler sees the world every 2 s. From the
frame, an enemy's or a neutral's `Mission` handler never runs (*derived*).

The handler's variables belong to the SuperAI (`+0x18`,
[15-behaviour.md](15-behaviour.md#how-a-handler-runs)). They keep their values
from one run to the next, and a save writes them
([17-saves.md](17-saves.md)). Mission 01's latches depend on that.

## Messages — *read*, and *measured*

`messages.cfg` maps a `message_index` to a text and a voice
([21-briefing.md](21-briefing.md#messagescfg--the-in-mission-dialogue)).

**The loader** (`iron3d.dll:0x10094e90`) reads four keys from each object:
- `message_index`, `text_resource` and `voice_resource`;
- an optional `info_system` flag (`0x100950dc`). The message history files a
  message that sets it as kind 4 rather than 3 (`0x10095519`); what that
  changes on screen is not read.
- *Measured*: 46 of the 99 messages set `info_system`. They are all 27
  training-campaign lines named `_H` (the help texts, such as Mission 01's
  `T01_H02`…`T01_H08`), and all 19 messages outside the training campaign.
  No briefing line or instructor line (`_I`) of the training missions sets it.

**Playing one.** `MESSAGE_INFO` hands the id to `0x10094e30`, which finds the
message and plays it (`0x100952d0`).

- **The first time**, the message is marked played and its text goes into the
  message history. Its voice goes to `0x10061ac0`.
  - With no speaker given, as here, `0x10061ac0` passes the voice name as
    written to the sound server's slot 4.
  - With a speaker, it appends `_S` or `_B` to the name by the speaker's kind.
- **Any later time**, nothing is voiced. The history gets `iron3d.dll`'s
  strings 6170 and 6223 instead: *"Recieved message is already in history"*
  and *"Press %s to see it."* (spelling the binary's).

**Voices queue** (`services.dll`, `ISoundServer`, vtable `0x1003a4a4`):

- **Slot 4** (`0x10011ab0`) looks the name up through the resource manager and
  appends the sound to a queue. It starts the sound at once only when the
  queue was empty.
- **Slot 6** (`0x100119e0`) pops the front sound once it has stopped playing,
  and starts the next.
- **Slot 2** (`0x10011bb0`) plays at once, past the queue.
- So two messages asked for in one handler run play one after the other,
  never over each other.

**Where the voice comes from** (*measured*). The name resolves through the
mission's own resource descriptors ([20-resources.md](20-resources.md)). All
80 messages of the four training missions resolve to a member of `voices.lib`:
- through `briefing_sounds` exactly when the message is a line the briefing
  flythrough speaks;
- through `tutorial_voices` otherwise.

In Mission 01:
- ids 0–10 (`T01_T01`…`T01_T11`) are the briefing's lines;
- ids 11–21 (`T01_I01`…`T01_I03`, `T01_H02`…`T01_H08`) are the ones its
  script asks for;
- every text resolves through `TextRes.cfg` ([20-resources.md](20-resources.md)).

## Objectives and the end of a mission — *read*, and *measured*

**The list.** `mission.cfg`'s `primary_objectives` object gives the objectives
in file order, with exempt 0 (`iron3d.dll:0x1006a79b`). `bonus_objectives`
appends its own, with exempt 1 (`0x1006a96a`). A script names an objective by
its position in that combined list.
- `openparkan.mission.objectives` reads the list.
- Until this reading, `load_cfg` took a line like `objective1 = …` for an
  object header, because it starts with "object". The objectives came out
  empty.

**A script's call.** On channel 0 of the mission callback (`0x10060ce0`):

- **`OBJECTIVE_COMPLETE`** on an objective still open:
  - shows string 5040, *"Objective is completed"*;
  - voices `VOICE_OBJ_COMPLETE` (`vc_obj_cpl.wav` through
    `ui/game_resources.cfg`);
  - sets the objective's state to 1 (`0x10060e89`).
- **The completion test** follows every `OBJECTIVE_COMPLETE`, open objective
  or not (`0x10060f5e`). It runs `0x1006b130`: every objective whose exempt
  word is 0 must be at state 1. When the test passes, the callback sends
  itself `SYSTEM_MESSAGE` with `MISSION_COMPLETE`.
- **`MISSION_COMPLETE`** (`0x10060d9f`):
  - voices `VOICE_MISSION_COMPLETE` (`vc_mis_cpl.wav`);
  - records the outcome, setting the game's state word `+8` to 0 and the won
    flag, the parameter block's `+0x157`, to 1.
- **`MISSION_FAILED`** voices `VOICE_MISSION_FAIL` and records the other
  outcome: state word 1, won flag 0.
- What follows either is [After the outcome](#after-the-outcome--read-and-measured).
- **`OBJECTIVE_FAILED`** fetches string 5041, *"Objective has failed"* (not
  followed further).

Bonus objectives never hold the mission back. A mission with no objectives
would pass the test, but only an `OBJECTIVE_COMPLETE` ever runs it.

*Measured*, over the scripts missions name:
- 90 of the 92 literal objective values fit their mission's primary + bonus
  list. The two that do not are `c2m3p`'s `OBJECTIVE_COMPLETE 2` and
  `OBJECTIVE_PROGRESS 2`, against a list of 2.
- Nothing bounds the index (`0x1006b440`). What those two calls touch is
  undefined (*derived*).
- Mission 01 lists three primary objectives and no bonus, and its script
  completes 0, 1 and 2.

## Mission 01, *Line of Fire*, end to end — *derived*

`tut1_pl2`'s `Mission` handler, every 2 s. Every route test is on logical
id 1, the hero, which starts inside route 0 (*measured*: a four-point square
whose corners lie at most 2.6 m from its start).

**The route tests.** A route message plays only on a run in which the hero is
in some route after a run in which it was in none. The latch is
`dcl0`/`dcl1`: `dcl0` marks this run, `dcl1` the one before. If the hero is in
two routes at once, both messages play. Walking straight from one route into a
touching one plays nothing.

| run finds | then | message says |
|---|---|---|
| the hero in route 0, its start | messages 11 and 14 | `T01_I01` welcome; `T01_H02` movement, and the message history |
| the hero in route 1 (x 441–713, y 322–677) | 21 | `T01_H08` zoom, infravision and the map |
| the hero in route 2 (x 671–1020, y 268–667) | 16 | `T01_H03` selecting guns 1–4 and firing |
| the hero in route 3 (x 610–1008, y 666–938; both neutrals stand in it, *measured*) | 18 | `T01_H05` capturing a neutral warbot |
| the hero in route 4 (x 364–969, y 961–1392, the north) | 20 | `T01_H07` ordering an attack |

**The count tests.** Each plays once, held by its own `df` flag.

| run finds | then | message says |
|---|---|---|
| Trgt robots < 5 | 17 | `T01_H04` guided missiles |
| Trgt robots = 0 | objective 0, then 12 | `T01_I02` targets done, find some troops |
| Ntrl robots = 0 and Plr robots ≥ 2 | objective 1, then 13 and 19 | `T01_I03` go north; `T01_H06` the troops menu |
| Enm robots = 0 | objective 2 | — |

The objectives are those of `mission.cfg`: "Destroy all the targets on the
island", "Capture the neutral warbots", "Destroy the enemy warbot".

- **The end.** The mission is complete on the run that completes the last of
  the three, in any order.
- **Capturing alone does not win it.** Objective 1 wants Ntrl to have no
  robots left, both `tut1_mf1` and `helic` gone. It also wants Plr to have
  two: the hero and at least one captured warbot. A capture wins the mission
  only when it is the last of the three objectives done.
- **The voices of a completion** all go through the one queue, in the order
  they are asked for:
  1. `VOICE_OBJ_COMPLETE`;
  2. straight after it, `VOICE_MISSION_COMPLETE`, when that was the last
     objective;
  3. then the messages the handler asks for next.

## After the outcome — *read*, and *measured*

A won or lost mission does not stop. The HUD gives way to a panel that names
the outcome, the world plays on under it, and the mission ends only when the
player presses Esc. The menus then come back, and a win is written to
`dispatcher.ini`.

**The game's state word** is `+8` of the game object (`getIGame`,
`iron3d.dll:0x1005b580`, the object at `0x1010b5f8`):

| value | written by | means |
|---:|---|---|
| 5 | the constructor (`0x1005c330`) | not started |
| 4 | the loop, as it starts (`0x1005e680`) | playing |
| 0 | `MISSION_COMPLETE` (`0x10060dd3`); the game's own complete, `0x10061970` | won |
| 1 | `MISSION_FAILED` (`0x10060e31`); the game's own fail, `0x100618a0` | lost |
| 2 | case 3 of the game's message callback (`0x1005fafc`) | the session lost (string 6225, *derived*) |
| 3 | the exit (`0x10061a30`) | leaving |

- **A repeated outcome.** A second `MISSION_COMPLETE` does nothing: the handler
  tests the word against 0 (`0x10060d8c`). `MISSION_FAILED` tests it only against 1
  (`0x10060de9`), so a failure after a win replaces it (*derived*).
- **The won flag** is `+0x157` of the parameter block the executable hands the
  game (`Run` keeps the block's address at `+4`, `0x1005c680`).

**Play goes on.** The loop (`0x1005e680`) runs until the state word is 3
(`0x1005ef9a`) and tests no other value. The units, the scripts' `Mission`
handler ([When the Mission handler runs](#when-the-mission-handler-runs--read))
and the voices carry on under the panel. No timer ends a mission.

**The panel** (`0x1009f8b0`). The interface pass (`0x100608f0`) draws the HUD
while the state word is 4, and the panel otherwise (`0x10060bae`). It does so on
every second call; a byte at `0x1010b618` flips each call, and the calls between
draw only what the debug keys add. How often the pass is called was not read. The
panel switches on the word for a title and up to four lines:

| state | title | its colour | lines |
|---:|---|---|---|
| 0 | 1012 *"MISSION COMPLETE !"* | green `#64ff64` | 5082 *"Press 'Esc' to continue"* |
| 1 | 1013 *"MISSION FAILED..."* | red `#ff6464` | 5082; 3075 *"Press 'R' to restart mission"*; 3076 *"Press 'L' to load saved game"* |
| 2 | 6225 *"Multiplayer session lost"* | green | 5082 |
| 3 | 5083 *"Exiting..."* | green | — |

- **The lines are grey**, `#f0f0f0`.
- **In a network game** (the game's `+0xe4`, parameter mode 2, *derived* from what
  it drops) lines 3075 and 3076 are cleared. On the session's server (`+0xe7`) a
  fourth line is added in red, 3077 *"WARNING! YOU`RE THE SERVER. IF YOU KILL THE
  GAME YOU'LL KILL OTHER PLAYERS."*, and the lines are spaced 15 wider.
- **Fonts.** The title is in `MENU_FONT` (the game's `+0x14`, set at `0x1005f9df`)
  and the lines in `GAME_FONT` (`+0x10`, `0x1005f9a0`). Each name resolves through
  `ui/menu_resources.cfg`'s font substitutes for the screen size to an entry of
  `ui/font.lib`.
- **Layout**, on the 640 × 480 screen the 2D calls take:
  - every text is centred on x 320;
  - the title sits at y 50;
  - the lines start at y 75, one font height + 2 apart, and an empty line takes
    no room;
  - a font's height and a text's width reach that screen through two scale
    queries of the display object (`0x100cd048` slots `0x10` and `0x14`), not read.
- **The box** is black at alpha `0x99`, 60%.
  - It runs from (x₀, 35) to (640 − x₀, the last line's y + 15). x₀ is the
    leftmost text's x less 30, and never below 0.
  - That these are two corners is *derived* from the box being symmetric.
  - In state 3 the box is the whole screen, (0, 0) to (640, 480).

**Leaving.** The game view's character handler (`0x10070db0`) takes Esc, `0x1b`:

- **A pause is lifted first.** If the interface's pause byte is set,
  `0x1008d850` clears it, and `0x1005f620` clears the game's `+0xe8` and calls
  `World3D`'s `ResumeGameTime`.
- **Once the outcome is recorded,** that is with the state word not 4, the game
  exits with code 1 (`0x10070e2c`).
- **In parameter mode 3,** Esc always exits (`0x10070e11`).
- **While playing,** Esc goes on to its other uses. It skips a briefing
  (`0x10070e75`); the rest were not followed.

After a failure or a lost session, `0x100711f0` takes R and L. It works only
while the state word is 1 or 2, and not in a network game:

| characters | exit code |
|---|---:|
| `R`, `r`, `К`, `к` | 2 |
| `L`, `l`, `Д`, `д` | 3 |

The Cyrillic four are windows-1251 `0xca`, `0xea`, `0xc4` and `0xe4`: the letters
on the same two keys of a Russian layout (*derived*). A win has no R or L.

**The exit** (`0x10061a30`) writes its code to the block's `+0x14c` and sets the
state word to 3. The loop ends and `Run` returns 2 (`0x1005efae`).

**The executable** (`iron_3d.exe:0x401190`) alternates the two DLL objects it
binds by name (`createShell`, `getIShell`, `createGame`, `getIGame` and their
deletes). The shell runs until the player starts a mission, and the game runs
that mission. Then, by the exit code:

- **2:** the game runs again at once with the same parameters, a restart;
- **4:** a load from inside the game (`iron3d.dll:0x100a52d4`, once a file under
  `/save/` is found). The block is reset to mode 1, with `+0x148` 6, `+0x154`
  clear and no mission path, and the game runs again;
- **any other code:** the game is deleted and the shell created afresh, and the
  shell is handed `Run`'s 2 (`0x4012f0`).

The block (`iron_3d.exe:0x406550`) starts as mode 0, `+0x148` −1, `+0x14c` 1,
`+0x150` 2, `+0x154` and `+0x155` 1, and `+0x157` 0 (`0x401000`).

**The shell** (`iron3d.dll:0x10007960`, `IShell` slot 0):

- **It records a win.** Handed 2 with the won flag set (read at `0x10007c2f`), it
  writes `[COMPLETE]` *key* `= 1` to `MISSIONS/dispatcher.ini` (`0x10022be0`,
  through `WritePrivateProfileStringA`).
  - The key is the mission's path, with every character that the CRT's class
    test (`0x100b4a86`) refuses turned into `_` (`0x10022ce0`).
  - [22-settings.md](22-settings.md#iron_3dini-and-dispatcherini--the-players-not-the-games)
    reads the file.
- **It opens by its start switch** (`0x10008b74`). After a game, case 2
  (`0x10009980`) opens the main menu, screen 10, and then:
  - **code 1 in mode 1 or 4:** a mission path holding `campaign` or `CAMPAIGN`
    goes to the campaign branch (`0x10009c10`), and one holding `single` or
    `SINGLE` to the single-mission branch (`0x10009f40`);
  - **code 3 in mode 1, or with the block's `+0x154` clear:** the load-game
    screen, 21 (`0x10012ce0`, `"load_game"`, `"save/"`);
  - **any other code:** the main menu stays.

**A briefing-only mission** (`0x1005ea08`). The briefing plays while the
level's state word `+0x710` (the game's `+0x1c`) is 5. When it ends, or Esc skips it (`0x10070e75`),
the loop reads `mission.cfg`'s `only_briefing` from its `mission` object. If
that is true, the mission is won and exits with code 1 at once.

*Measured*: `only_briefing` is true on `CAMPAIGN.01/Mission.01` and
`CAMPAIGN.05/Mission.02`, false on 25 missions, and absent from `Multi.02` and
`Multi.06`.

**The hero's loss fails the mission** (`0x100751a0`). That function runs as a
unit's record goes: game message 2, for an id whose class nibble is 3, reaches it
through `0x1007d4e0`. That it is a loss is *derived* from the voices it plays.

- **It speaks first.** A unit of the player's clan plays `VOICE_UNIT_LOST`.
  Another clan's plays `VOICE_ENEMY_DESTROY` when that clan's record has `+0x730`
  set.
- **The hero's loss fails the mission.** When the unit is the player's clan's hero
  (`Type` `0x1020000`), driven or not, the view is put on it (`0x100a4e50`) and the
  game fails (`0x10075619`). The panel then reads *"MISSION FAILED..."*.
- **It does nothing** once the state word is 3.
- **In mode 3,** the loss of the driven unit is handled elsewhere.
- **A driven bot's loss** goes to `0x10062ff0`, which was not followed.
- **In a network game,** losing another clan's hero while at most one hero is left
  wins the mission (`0x10075381`; `0x10072cb0` counts `Type` `0x1020000`,
  *derived*).

**For an engine**, on Mission 01:
1. **On `MISSION_COMPLETE`,** keep simulating. Draw the panel in place of the
   HUD:
   - a 60% black box;
   - *"MISSION COMPLETE !"* in green `MENU_FONT` at (320, 50);
   - *"Press 'Esc' to continue"* in grey `GAME_FONT` at (320, 75), both centred
     on a 640 × 480 layout.
2. **On Esc,** end the mission, mark `missions_campaign_campaign_00_mission_01_`
   complete, and return to the menus.
3. **On a failure** (the hero lost, or a script's `MISSION_FAILED`), show the red
   title and the R and L lines. R restarts the mission with the same parameters;
   L goes to the load-game screen.

## Capturing a neutral warbot, for an engine — *read*

The capture is [27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured)'s,
in this order:

1. **Make the warbot the hero's current target.** How the target is picked is
   read elsewhere. The game's own `T01_H05` says the right mouse button
   selects what is in view, and Tab cycles.
2. **Press Enter.** That is `CMD_ENTER_STATE` (730, `SCAN_W_ENTER`). The
   player must be driving the hero.
3. **The target must pass three tests:**
   - its `Type` has no bit outside `0x103e000`;
   - it is within 20 of the hero across the ground;
   - its clan's type is 3, neutral.
4. **`MBehaviour::Capture`** gives it the player's clan, SuperAI and areal
   map. It then joins the player's count and leaves the neutrals'
   (*derived*, above).
5. **If the bot can be boarded**, the hero boards it: its record's `+0x30` is
   4 and its class-1 turret still has life. The view goes to state 1 with that
   bot, and the player drives the bot from then on.

## Ambient sound — *read* in part

- **The theme.** `mission.cfg`'s `ambient_music_loop` `THEME` goes straight to
  the sound server's slot 2 at mission load (`iron3d.dll:0x1005e2e1`). It is a
  type-5 descriptor ([20-resources.md](20-resources.md)); whether that type is
  what makes it loop is not read.
- **The variations.** `ambient_music_variation`'s names are gathered as
  `DEFAULT_`, `DAY_` and `NIGHT_` + `VARIATION1`, `2`, … for as long as the
  key exists (`0x1005f8f1`). When one plays, and how day or night picks among
  them, is not read.

## Not established

- The clock unit of the behaviour takt that times the route reports.
- How a destroyed or captured unit leaves function 31's list.
- Where a message's text is drawn and for how long.
- What `info_system` changes on screen: it makes a history entry kind 4
  rather than 3.
- ~~What the game shows after `MISSION_COMPLETE`.~~ A panel in place of the HUD,
  until Esc exits to the menus, which record the win
  ([After the outcome](#after-the-outcome--read-and-measured)). Still open:
  - what the shell's campaign branch (`0x10009c10`) and single-mission branch
    (`0x10009f40`) show, and so what leads to the next mission;
  - which parameter modes 3 and 4 are;
  - the display's two scale queries that place the panel's text;
  - who sends the game message 3 that sets the state word to 2.
- Whether the hero keeps reporting its route while it sits inside a boarded
  bot. The route tests name the hero's id, not the bot's.
- What the behaviour does with the message 6 it sends itself for each tactical
  areal it is in.
- The ambient variations' schedule.
- ~~A failure on the hero's death.~~ The game fails the mission itself when the
  player's clan's hero is lost (`iron3d.dll:0x10075619`,
  [After the outcome](#after-the-outcome--read-and-measured)), though
  `CLAN_HERO_KILLED` does nothing in this build
  ([21-briefing.md](21-briefing.md#messagescfg--the-in-mission-dialogue)) and
  Mission 01's script never fails. What a driven bot's loss does
  (`0x10062ff0`) is not followed.
