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

`iron3d.dll`'s game frame (`0x1005ed85`) does two things, unless a byte at
`+0xe8` is set, the frame's state word is 3, or the game's state word `+0x710`
is 5:

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
  - records the outcome, setting the word at `+8` to 0 and a byte at
    `+0x157` to 1.
- **`MISSION_FAILED`** voices `VOICE_MISSION_FAIL` and records the other
  outcome.
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
- What the game shows after `MISSION_COMPLETE`, and what leads to the next
  mission.
- Whether the hero keeps reporting its route while it sits inside a boarded
  bot. The route tests name the hero's id, not the bot's.
- What the behaviour does with the message 6 it sends itself for each tactical
  areal it is in.
- The ambient variations' schedule.
- A failure on the hero's death. `CLAN_HERO_KILLED` does nothing in this build
  ([21-briefing.md](21-briefing.md#messagescfg--the-in-mission-dialogue)), and
  Mission 01's script never fails.
