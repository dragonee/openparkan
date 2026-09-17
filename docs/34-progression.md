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
- **A repeated key is still an objective.** The loader walks an objective
  object's property records by index, 40 bytes each (`0x1006a832`–`0x1006a8ec`),
  and never looks up a key. Two files repeat one:
  - Mission 04's `primary_objectives` names `objective4` twice. The game's
    objectives screen lists all six lines (*seen*,
    [below](#mission-04-teleport-end-to-end--derived)).
  - `CAMPAIGN.02/Mission.03`'s `bonus_objectives` names `objective1` twice.
  A reader that keeps one value per key loses a line from each.
  `openparkan.mission.objectives` and the engine's `cfg::objectives` read
  every line (`load_cfg_lines`, `Block::lines`); `load_cfg` and a block's
  `properties` still keep one value per key.
- **Counting every line, all 92** literal objective values fit their mission's
  primary + bonus list. Counting a key once, 90 fit: `c2m3p`'s
  `OBJECTIVE_COMPLETE 2` and `OBJECTIVE_PROGRESS 2` fall past a list of 2,
  where the file has 3.
- Nothing bounds the index (`0x1006b440`).
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

## Mission 02, *The Constructor*, end to end — *derived*

`CAMPAIGN.00/Mission.02` on Tut_2: capture a factory, build a warbot in it,
fly the warbot to an island and capture the Outpost there. The mechanics it
leans on have their own pages — entering and taking a building
([27-ownership.md](27-ownership.md)), the factory's screen and its production
(36-factory.md), the designer (37-designer.md, 38-designs.md), boarding and
flying a bot (39-boarding.md). This section is the mission as its files and
its script drive it.

### What the mission places — *measured*

| clan | index | type | minds | script | zones |
|---|---:|---|---:|---|---|
| `Plr` | 0 | 1, player | 2 | `tut2_pl2` | — |
| `Anml` | 1 | 0, nature | 5 | `tut2_en` | (1068.8, 1076.8) inner 20, outer 40; (915.1, 990.0) inner 20, outer 50 |
| `Ntrl` | 2 | 3, neutral | 5 | `tut2_nt` | — |

`Plr` and `Anml` are hostile to each other both ways; `Ntrl` is neutral to
both, and both to it ([04-missions.md](04-missions.md)).

| logical id | what | clan | where |
|---|---|---|---|
| 1 | the hero, `tut2_p.dat` | `Plr` | (300.3, 556.1) |
| 2, 3 | two medusas, `tent.dat` | `Anml` | (924.3, 987.4), (908.2, 986.5) |
| `CLASS_BUILDING`\|1 | the Large Factory, `lplant01.dat` (`fr_b_plant`, Type `0x80000010`) | `Ntrl` | (392.4, 788.7) |
| `CLASS_BUILDING`\|3 | the Outpost, `shang01.dat` (`fr_l_angar`, Type `0x80000040`, with a shield generator) | `Ntrl` | (1289.1, 793.5), on the island |
| `CLASS_BUILDING`\|5 | a generator, `gener01.dat` | `Plr` | (874.0, 330.4) |

Four trees make up the rest. There is no mine or storage, so the factory
screen's Ore reads 0% the whole mission ([23-economy.md](23-economy.md#what-the-hud-shows--read)).
The player's two minds are one for the hero and one to spare, so the player
can have **one bot besides the hero** ([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).

**The island is ringed by a lake bed.** Tut_2's water is 126 faces, all at
z 150. Under it 103 level-0 faces carry the bed bit, z 115.8 to 150. The
bed-free ground joined to the Outpost's face is 52 faces, x 1219.5–1368.6,
y 729.0–954.4, z 150 to 151.67, and it touches the mainland nowhere: the
shortest way off crosses 49 m of bed. A walker whose sphere meets a bed under
shallow water dies ([24-motion.md](24-motion.md#water-and-lava-beds-kill--read-and-measured)),
so the island can only be reached by air.

### What the scripts ask — *read*, and *measured*

**Only the player's script does anything.** `tut2_en` and `tut2_nt` call
function 19 in `Init` and have no other node. The medusas and the neutrals run
on their behaviour's defaults. `tut2_pl2` calls functions 19, 30, 31, 32 and
**52**, the one the others do not: a building's owner.

**Function 52** (`ai.dll:0x1000e0e4`) asks the clan areal map (the SuperAI's
`+0x3d8`) for the object with the logical id. That map's slot 7 hands the
question to the system areal map's slot 21, the object by logical id
(`ArealMap.dll:0x10001a40` → `0x10021020`). No object gives `ERROR`,
`0xffffffff` (`0x1000e153`); otherwise the answer is the object's slot 17
(`0x1000e165`), its owner word
([15-behaviour.md](15-behaviour.md#65534-is-a-destroyed-objects-owner--read-and-measured)).

- **The argument.** `dT1 = CLASS_BUILDING|1` is the constant `0x80000001`,
  the Large Factory's `LogicalID`, and `CLASS_BUILDING|3` the Outpost's.
- **The answer is a clan's index** (*derived*). The script compares it with
  `d0`, 0, which `varset.var` also names `PLAYER_CLAN`. Players come first in a
  mission's clan list: on all 28 missions with a player clan, index 0 is one
  (*measured*). A destroyed building answers 65534, and only an id with no
  object answers `ERROR`.
- **The failure branches never fire in play** (*derived*). A building that
  dies keeps its object, whose owner word reads 65534
  ([26-damage.md](26-damage.md)), so its id never stops answering. Nor can
  either building die of rounds: a plant's and a hangar's node 0 has no
  level-0 geometry to strike, and that a blast cannot reach it is docs/26's
  *guess*. The hero's death still fails the mission
  ([After the outcome](#after-the-outcome--read-and-measured)).
- **For an engine that answers 0 to a function it does not know**, both
  captures read done on the first run.

**The route tests** are Mission 01's latch, on the hero's id 1: a message
plays on a run that finds the hero in some route after a run that found it in
none. The hero starts inside route 0, a square whose corners lie 9.7–9.9 m
from it.

| run finds | then | message says |
|---|---|---|
| the hero in route 0, its start | 6 | `T02_I01` capture the Large Factory, the building with three pipes |
| the hero in route 1 (x 146–671, y 593–844, about the factory; a notch x 347–438, y 698–823 leaves the building out) | 10 | `T02_H01` capturing buildings: go inside, get on the green platform |
| the hero in route 2 (x 207–616, y 874–1008, north of the factory) | 12 | `T02_H03` the factory's assembly line and repair station |
| the hero in route 3 (x 509–651, y 1054–1236) | 13 | `T02_H04` the animals' habitat and the medusas |
| the hero in route 4 (x 919–1289, y 665–1127, from the medusas' eastern pasture, which it holds, over the lake's western half onto the island's western part) | 14 | `T02_H05` large warbots: Tab, Enter, and Escape to get out |

**The count tests**, each held by its own `df` flag:

| run finds | then | message says |
|---|---|---|
| function 52 of the factory = 0 | objective 0, then 7 and 11 | `T02_I02` the factory runs itself; `T02_H02` the constructor button |
| `Plr` robots = 2 | objective 1, then 8 | `T02_I03` get out: the warbot comes out through the factory gates |
| function 52 of the Outpost = 0 | objective 2 | — |
| `Anml` animals (`CLASS_ANIMAL`) = 0 | objective 3, then 9 | `T02_I04` the enemy is done for |

- **"= 2" is exact, and cannot be overshot.** The hero is one robot and the
  minds allow one more, so the count reaches 2 with the first bot built and
  stays there (*derived*).
- **The objectives** are `mission.cfg`'s three primary ones — "Capture the
  Large Factory", "Build a warbot", "Capture the Outpost on the island" — and
  one bonus, "Destroy hostile animals". The script completes 0 to 3, so the
  mission is won on the run that completes the last of 0, 1 and 2; the medusas
  may be left alive.
- **Messages 0–5** (`T02_T01`…`T02_T06`) are the briefing's lines.
- **Message 100**, `T02_H06` — flying a warbot, PageUp and PageDn — is never
  asked for by the script. `iron3d.dll` asks for it itself (`0x100638a7`) in
  game mode 4 when the unit the player takes over is a flyer: `0x10075f70`
  tests the unit's property `0x207`, its chassis type, for 1. That call is
  part of boarding (39-boarding.md).

### The medusas — *read*, and *measured*

`tent.dat` is one record, `A_L_03`, "L Brainless Tentacle": a `BTLU` creature
with the flyer profile `chas_fly.var`, Type `CLASS_ANIMAL`.

- **Its controller**, `a_a_l3.ctl`: a battery, an engine, a turret, a radar
  of range 500, and one gun. Top speed 13 m/s forward and 2 across and up
  ([24-motion.md](24-motion.md)).
- **The gun** never runs dry and fires every 500 ms. Its round, `ba_a_03`,
  flies at 90 m/s for 200 m and does 400 in a blast of radius 5.
- **Its nodes**: 13; node 0 has 2,500 hit points, eleven have 500, one has 1,
  and node 0 blows up with `selfexp_anl_03a.exp`.

**What they do.** With no order an animal migrates
([31-packages.md](31-packages.md#migrate-an-animals-pasture--read-and-measured)).
Both medusas start 7.7 and 9.6 m from the centre of the western pasture, and
their clan changes pasture every 60–180 s at random. A grazing medusa shoots at
nothing. It attacks a hostile unit — the hero, or the player's warbot — that
comes within 20 m of the current pasture's centre (*derived*), and anything that
hurts it, whatever its radar holds: shot from off its pasture it goes for the
firer for 20 s, held within 70 m of the western pasture's centre or 60 m of the
eastern's (*read*, [31-packages.md](31-packages.md#a-hit-pulls-a-unit-in--read)).
The briefing's `T02_T03` says as much: they attack only those who trespass on
their territory.

**Destroying both** empties `Anml`'s count, which completes the bonus
objective with `T02_I04`. How a dead unit leaves the count is not read; the
recording below shows it happen.

### Seen in a recording

Timings from a 30 fps recording of the mission, played through. Each line is
the first frame that shows it, sampled every 2 s, or every 0.1 to 0.5 s where a
time is given to a tenth.

| time | what |
|---|---|
| 0–69.5 s | the briefing, ending in black |
| 70.5–76.5 s | the objectives screen, "Primary objectives" and "Additional objectives" ([35-hud.md](35-hud.md#the-objectives-screen)) |
| 76.5 s | the cockpit, `T02_I01` already up: asked while the objectives screen hid it |
| 88 s | `T02_H01`, route 1 |
| 106.4 s | the factory screen opens, its message box reading "Building is captured" from the System. Objective 0, `T02_I02` and `T02_H02` are not seen: the designer covers the screen from 107.8 s, and when the factory screen is back the box's 20 s are long past |
| 107.8–155.5 s | the designer |
| 156–157 s | the factory screen, the design listed |
| 178 s | `T02_H03`, route 2 |
| 194 s | `T02_H04`, route 3 |
| ~195–218 s | the hero fights two medusas flying high over the ground; green acid lands around it |
| 219.0 s | objective 1 and `T02_I03`: the warbot exists |
| ~227 s | objective 3 and `T02_I04` |
| 236 s | "Recieved message is already in history": a route the hero had been in before |
| ~249 s | the hero boards the LFW-2 Warrior; `T02_H06`, message 100 |
| 268–286 s | "already in history" again, in flight |
| ~312 s | `T02_H05`, route 4, with the hero inside the warbot over the lake |
| 340.5 s | "Building is captured": the Outpost |
| 341.5 s | "MISSION COMPLETE !" |

Three things follow from it.

- **The hero's route follows the bot it rides.** Route 4 lies 500 m east of the
  factory, where the hero boarded, and its message plays while the hero is
  aboard.
- **The warbot took 63 s** from the design's acceptance (about 156 s) to the
  handler run that counted it. A free bot of a large chassis in a large
  factory is timed at 60 s ([23-economy.md](23-economy.md#construction--read)),
  and the plant has `FreeBotNum` 100; the handler runs every 2 s.
- **The screenshot's "Energy 5%" is a moment in a climb.** As the factory
  screen opens, Energy reads 1% at 106.4 s and 20% at 107.7 s, about 15 points
  a second, while Ore flashes 0% on and off each half second. The display steps
  a point toward its target at most every 0.05 s
  ([23-economy.md](23-economy.md#what-the-hud-shows--read)); if that is tested
  once a frame, at 30 frames a second a step waits every other frame, 15 a
  second (*derived*). The target itself is not derived here.

### For an engine

1. **Answer function 52**: by logical id, the owner's clan index — 65534
   once destroyed, `ERROR` for an id with no object.
2. **Count what the missions count.** A bot the factory builds joins `Plr`,
   and a dead medusa leaves `Anml`.
3. **Report the hero's route from the bot** while it rides one.
4. **Let the medusas graze and defend** as above, from `Anml`'s two zones.
5. **Ask for message 100** the first time the player takes over a flyer, in a
   training mission (game mode 4 is a *guess*, [21-briefing.md](21-briefing.md#not-established)).
6. **Win** when objectives 0, 1 and 2 are all complete, whatever objective 3 is.

## Mission 03, *The Field Base*, end to end — *derived*

`CAMPAIGN.00/Mission.03` on Tut_3. The hero captures a generator and a bunker,
has the builder put up a mine, builds four warbots and destroys the enemy
patrol that comes for the base. The mechanics it leans on have their own pages:
- taking a building by its pod
  ([27-ownership.md](27-ownership.md#capture--read)) and walking in through its
  doors ([24-motion.md](24-motion.md#walking-into-a-building--read-and-measured));
- building a building ([32-builder.md](32-builder.md));
- the factory and the designer (36-factory.md, 37-designer.md);
- command mode, which the bunker opens, and its commander panel (40, 41).

This section is the mission as its files and its script drive it.

### What the mission places — *measured*

| clan | index | type | minds | script | base |
|---|---:|---|---:|---|---|
| `Plr` | 0 | 1, player | 7 | `tut3_pl2` | (521, 978) |
| `Enm` | 1 | 2, enemy | 5 | `tut3_en` | (1458, 1550) |
| `Ntrl` | 2 | 3, neutral | 5 | `tut3_nt` | (1075, 900) |

`Plr` and `Enm` are hostile to each other both ways; `Ntrl` is neutral to
both, and both to it.

| logical id | what | clan | where |
|---|---|---|---|
| 1 | the hero, `tut3_p.dat` | `Plr` | (461.2, 694.6) |
| 6 | the builder, `tut3_b.dat` (SWB-2, a wheeled S-31 with a builder's beam) | `Plr` | (1106.6, 914.4) |
| 7 | the transport, `tut3_t.dat` (SWT-3, the same chassis) | `Plr` | (1032.5, 836.8) |
| 3, 4, 5 | the enemy patrol, `tut3_f1`–`f3.dat`: three S-2f flyers: an autocannon, a laser, and two flame throwers | `Enm` | about (1860, 1885), on the plateau at z 250 |
| `CLASS_BUILDING`\|1 | the Small Warehouse, `sstore01.dat` (`fr_l_store`) | `Plr` | (619.4, 805.3) |
| `CLASS_BUILDING`\|5 | the Large Factory, `lplant01.dat` | `Plr` | (1192.2, 612.3) |
| `CLASS_BUILDING`\|7 | the Small Generator, `gener01.dat` (`fr_l_gener`) | `Ntrl` | (651.8, 1051.2) |
| `CLASS_BUILDING`\|4 | the Small Bunker, `sbunk01.dat` (`fr_l_bunker`) | `Ntrl` | (1260.9, 813.9) |

There is no scenery.

- **One mineral lode**, at (1026.1, 942.7), already marked found. It is 85 m from
  the builder, where the briefing's "fiery plume" stands.
- **The player's minds.** The hero, the builder and the transport take three of
  the seven, so **exactly four bots can be built**
  ([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
- **The prebuilt designs.** `mission.cfg`'s `prebuild` object names
  `tut3_p1.dat` and `tut3_p2.dat`. They are warriors (`Type 0x1008000`) in
  `UNITS\UNITS\PREBLD\`: a walking S-12w with two autocannons, and a wheeled
  S-31 with two guided missile launchers. On a fresh mission start (the
  parameter block's `+0x154` is 1,
  [21-briefing.md](21-briefing.md#when-it-runs--read)),
  the designer's holder loads each `model` key's file into the list of recent
  projects (`iron3d.dll:0x10055a29` → `0x1004dd70`; path
  `units\units\prebld\`, five at most, *read*). The factory screen lists them
  from the first time it opens
  ([36-factory.md](36-factory.md#the-factory-panel)).

**Routes** (x and y extents, rounded):

| route | spans | holds |
|---|---|---|
| 0 | a square about the hero's start, corners 7.1–8.7 m away | the hero |
| 1 | x 436–731, y 620–884 | the Small Warehouse |
| 2 | x 561–782, y 932–1196 | the Small Generator |
| 3 | x 889–1138, y 868–1154 | the lode |
| 4 | x 1079–1129, y 881–940 | the builder |
| 5 | x 995–1079, y 788–877 | the transport |
| 6 | x 1085–1420, y 649–946 | the Small Bunker |

### What the scripts ask — *read*, and *measured*

**`tut3_pl2`** calls functions 15, 19, 30, 31, 32, 34 and 52. `Init` counts
the player's robots into `df5`, then sets `df5 = df5 + 4` and `df6 = df5 − 3`.
The count is 3 (*derived*: the recording plays message 21 with the first bot
built and objective 3 with the fourth, below). So **`df6` is 4 and `df5` is 7**.

**The route tests** are Mission 01's latch on the hero's id 1. A message plays
on a run that finds the hero in a route after a run that found it in none.

| run finds | then | message says |
|---|---|---|
| the hero in route 0, its start | 10 | `T03_I01` welcome; find the generator, north-north-east |
| the hero in route 1 | 15 | `T03_H01` the Warehouse |
| the hero in route 2 | 19 | `T03_H05` the Power Generator |
| the hero in route 3 | 18 | `T03_H04` lodes |
| the hero in route 4 | 17 | `T03_H03` the Builder |
| the hero in route 5 | 16 | `T03_H02` the Transport |
| the hero in route 6 | 23 | `T03_H061` bunkers, the base's headquarters and command mode |

**The count tests**, each held by its own `df` flag:

| run finds | then | messages say |
|---|---|---|
| function 52 of `CLASS_BUILDING`\|7, the generator, = 0 | objective 0, then 11 | `T03_I02` the base has energy; the generator's repair facilities |
| function 52 of `CLASS_BUILDING`\|4, the bunker, = 0 | objective 1, then 24 and 12 | `T03_H062` the bunker's camera; `T03_I03` order the builder to build a mine |
| function 34 of `BUILDING_MINE` = 1 | objective 2, then 13 and 25 | `T03_I04` the mine works slowly; `T03_H09` transporting minerals |
| `Plr` robots ≥ `df6`, 4 | 21 | `T03_H07` controlling warbots in command mode |
| `Plr` robots ≥ `df5`, 7 | objective 3, then 14, then the patrol's orders | `T03_I05` you'll need your army to defend the base |
| `Enm` robots = 0 | objective 4 | — |

- **Either building answering `ERROR`** sends `SYSTEM_MESSAGE` with
  `MISSION_FAILED`. As on Mission 02 it cannot happen in play
  ([above](#what-the-scripts-ask--read-and-measured)).
- **Messages 0–9** (`T03_T01`…`T03_T10`) are the briefing's lines. There is no
  message 20.
- **Message 22**, `T03_H08`, is about placing a building: the model, `<` and
  `>` to turn it, red and green. The script never asks for it. `iron3d.dll`
  asks for it itself, when a build command puts the building's full-size model
  under the cursor. That starter (`0x10057f00`, called from the command
  execution at `0x10079eda`) ends by asking for 22 while the game's byte
  `+0xe6` is set (`0x10058013`), the same gate as Mission 02's message 100.
- **The objectives** are `mission.cfg`'s five primary ones, with no bonus. The
  script completes 0 to 4, so the run that completes the fifth wins.

**The patrol's orders** (function 15, all `INSERT_ORDER_REPLACE` and
`ORDER_ROBOT_PATROL` by place, with the four floats `fSuccess`…`fIndependence`
at their `varset.var` 0.5):

| id | place | lies in |
|---|---|---|
| 3 | (1124, 783) | route 6, 140 m west of the bunker |
| 4 | (606, 993) | route 2, 74 m from the generator |
| 5 | (1124, 783) | route 6 |

A patrol of a place picks a new point within 60 of it every 20 to 30 s, and
fights what it meets
([31-packages.md](31-packages.md#what-each-package-does--read)). The flyers
start 1,325 and 1,549 m away.

**Function 15 orders any clan's unit** (`ai.dll:0x10008054`, *read*).
- **Finding the unit.** It looks the id up through the clan areal map's slot 7,
  as function 52 does (`0x1000835e`). The object may be anyone's, so the
  player's script orders the enemy's patrol. With no object it answers 5
  (`0x10008376`).
- **The packet.** A `TARGET_BY_PLACE` target's two words become floats, x at the
  packet's `+0x10` and y at `+0x14` (`0x100083ea`). The four floats go to
  `+0x12c`.
- **Giving it.** The unit's slot 3 takes the packet and the insert mode
  (`0x100086d4`). The answer is 1 when the unit takes it, 0 when it refuses.

**`tut3_en`'s `Init`** calls function 0, a stub that leaves 1
(`0x10008049`). It then gives ids 3, 4 and 5 `ORDER_ROBOT_SHUTDOWN`,
replacing. A shut-down unit's fire control asks for no target, and its task
lets no engagement through
([31-packages.md](31-packages.md#the-fire-control--read)), so **the patrol waits
on its plateau until the player's fourth bot sends it**. `tut3_nt` calls only
function 19.

**Function 34 counts a type exactly** (`ai.dll:0x10009c30`, *read*). It walks
the running SuperAI's own unit list (`+0x8c`). It counts the entries whose type
equals the argument and whose logical id is set (`0x10009cbb`, `0x10009cd2`).
**A building is on that list.** SuperAI slot 4's event 2 (`0x10001880`) files
it, and also counts it by type, a mine at `+0x3e0` (`0x10001974`). The capture
sends that event with the new clan (`iron3d.dll:0x10032ffa`); which call sends
it for a building a builder puts up is not traced. The recording shows the mine
counted while it is still going up (below). "= 1" is exact, so a second mine
before the handler's next run would miss the test (*derived*).

### Seen in a recording

Timings come from a 30 fps recording of the mission, played through at
960 × 720. Each line is the first frame that shows it, sampled every 2 s, or
more finely where a time is given to a tenth.

| time | what |
|---|---|
| 0–81 s | the briefing: 26 waypoints, 81.5 s of camera |
| 82–88 s | the objectives screen: five objectives, each "in progress" |
| 90 s | the cockpit, `T03_I01` up (route 0) |
| 94 s | `T03_H01`, route 1, passing the Small Warehouse |
| 110 s | `T03_H05`, route 2, at the generator |
| 116 s | "Building is captured": the generator, from its pod inside |
| 118 s | objective 0, `T03_I02` |
| 144 s | `T03_H04`, route 3; the lode's pillar of fire stands beside the transport |
| 164 s | `T03_H061`, route 6, walking down the bunker's ramp |
| 166.8 s | the laser strikes the bunker's closed door; its top edge drops from 167.2 s, and the hero is in the corridor by 171.5 s ([24-motion.md](24-motion.md#a-shot-opens-a-door--read-and-seen)) |
| 178 s | "Building is captured": the bunker; the view is command mode's camera over the bunker's roof, with the icon column and the Energy row |
| 180 s | objective 1, `T03_I03`; `T03_H062` was asked first and is not seen |
| 182 s | `T03_H08`, message 22, as Build Mine is chosen on the Builders page; the model is red at 184 s and green at 186 s |
| 186–192 s | the builder "(building)" |
| 194 s | the Transports page: Transport minerals |
| 196 s | objective 2, `T03_H09`; the builder "(no order)"; the Ore row appears |
| 204–230 s | the builder "(escaping)"; cyan sparks on the ground at the lode, no building standing |
| 212 s | the warbot designer |
| 232 s | a blue construction sphere at the lode |
| 236 s | the Small Mine stands |
| 240 s | the Factory page with a warrior project |
| 288 s | `T03_H07`, message 21: the first bot built |
| 334 s | objective 3, `T03_I05`: the fourth |
| 396–416 s | the patrol's marks cross the satellite map toward the base |
| 418–428 s | the fight at the base |
| 429 s | "MISSION COMPLETE !" over the sky, the command panel gone |
| 432 s | the campaign menu, *The Field Base* done |

What follows from it:

- **The mine counts from its creation.** The builder reached the lode about
  190 s: it starts 85 m away and drives at 82 km/h once the order is given at
  about 185 s. The count took the mine by 196 s, while the building was seen
  standing only at 236 s. Its construction sphere runs in between
  ([32-builder.md](32-builder.md#the-construction-sphere--read-and-measured)).
- **Neither `T03_H03` nor `T03_H02` is seen.** Routes 4 and 5 lie between the
  lode's route and the bunker's. Whether the hero entered them without passing
  through none is not measured.
- **The patrol took about 85 s** from its orders to the fight. Flying straight
  at 0.8 of 160 km/h covers its 1.3 to 1.5 km in 37 to 44 s. Where the rest went
  is not measured: its first point may wait for the patrol's timer, and its
  way down from the plateau is not straight.
- **The win came within 2 s of the last explosion**, the handler's period.

### For an engine

1. **Place the prebuilt designs** among the factory's recent projects at the
   start.
2. **Count robots after the placed units have joined** their clans, so `Init`
   reads 3, and objective 3 needs the four bots the minds allow.
3. **Answer function 34** by exact type over the clan's own list, counting a
   building from the moment it is created.
4. **Answer function 15 for any clan's unit**. Give `SHUTDOWN` (no fire, no
   engagement, no move) and `PATROL` of a place.
5. **Capture the generator and the bunker at their pods**
   ([27-ownership.md](27-ownership.md#the-zones-height-is-the-pod-nodes-parents-box--read-and-measured)).
   The bunker's pod opens command mode.
6. **Ask for message 22** when a build command puts a building's model under the
   cursor.
7. **Win** when objectives 0–4 are complete. Fail on the hero's death.

## Mission 04, *Teleport*, end to end — *derived*

`CAMPAIGN.00/Mission.04` on Tut_4, the last training mission. The steps:

1. The hero takes a neutral mobile HQ.
2. Its command mode sends the helicopter to capture the Large Factory and the
   Research Center.
3. The Research Center develops the missing turret, and the Factory builds a
   large flyer.
4. The hero takes the Main Teleport and goes through it.

The mechanics have their own pages:

- the hero's capture of a unit
  ([27-ownership.md](27-ownership.md#a-neutral-unit-is-taken-by-the-hero--read-and-measured));
- an HQ's command view
  ([30-turrets.md](30-turrets.md#an-hq-unit-in-play--read),
  [40-command-mode.md](40-command-mode.md));
- search and capture
  ([31-packages.md](31-packages.md#what-each-package-does--read));
- the teleport
  ([27-ownership.md](27-ownership.md#the-main-teleport--read-measured-and-seen)).

This section is the mission as its files and its script drive it.

### What the mission places — *measured*

| clan | index | type | minds | script | base |
|---|---:|---|---:|---|---|
| `Plr` | 0 | 1, player | 3 | `tut4_pl2` | (817, 1044) |
| `Ntrl` | 1 | 3, neutral | 5 | `tut4_nt` | (886, 1018) |

`Plr` and `Ntrl` are neutral to each other both ways.

| logical id | what | clan | where |
|---|---|---|---|
| 1 | the mobile HQ, `tut4_hq.dat`: a Large Wheel L-32 with the HQ turret HQL1 and two large flame throwers, "LWC-1 Comm. Center" | `Ntrl` | (422.5, 518.2), 65.7 m from the hero |
| 2 | the hero, `tut4_p.dat` | `Plr` | (483.2, 492.9) |
| 3 | the helicopter, `tut4_f1.dat`: a T-2 Tiny Helicopter with two tiny lasers, "TFW-2 Warrior" | `Plr` | (477.9, 530.6) |
| `CLASS_BUILDING`\|1 | the Main Teleport, `mtp_m_n1.dat` (`fr_m_mtp`) | `Ntrl` | (1153.7, 1265.9, 96.3) |
| `CLASS_BUILDING`\|2 | the Large Factory, `lplant01.dat` | `Ntrl` | (778.1, 990.4) |
| `CLASS_BUILDING`\|3 | the Research Center, `einst01.dat` | `Ntrl` | (959.6, 873.6) |
| `CLASS_BUILDING`\|5 | the Small Generator, `gener01.dat` | `Plr` | (449.5, 412.0) |

- **Scenery and lodes.** One tree stands on the map (`s_tree_58`), and there
  are no lodes.
- **The only generator** is the player's from the start. So the teleport's
  power rule, every generator the hero's clan's, holds as soon as the teleport
  is taken
  ([27-ownership.md](27-ownership.md#teleport-in-0x8000--read)).
- **Robots.** The player starts with 2, the hero and the helicopter, against 3
  minds.

**Routes** (x and y extents, rounded):

| route | spans | holds |
|---|---|---|
| 0 | a square about the hero's start, corners 14–16 m away | the hero |
| 1 | x 846–1341, y 1047–1508 | the Main Teleport |
| 2 | x 844–1048, y 762–977 | the Research Center |

### What the scripts ask — *read*, and *measured*

**`tut4_pl2`** calls functions 0, 19, 30, 31, 32 and 52; `tut4_nt` calls only
19.

- **`Init`** clears `df0`–`df4`, `df8` and `df9`, and sets
  `df5 = fn31(d0, CLASS_ROBOT) + 2`. With the placed units counted first, as on
  Mission 03, that is 2 + 2 = **4**.
- **The route tests** are Mission 01's latch: a message plays on a run that
  finds its unit in a route after a run that found it in none. The hero's
  three tests share one latch (`dcl0`, `dcl1`); the helicopter's has its own
  (`df8`, `df9`).

| run finds | then | message says |
|---|---|---|
| the hero (id 2) in route 0 | 9 | `T04_I01` welcome to the fourth sector; a mobile command center near here, "a Bunker on wheels" |
| the hero in route 1 | 17 | `T04_H03` the Planetary Teleport, "enter the force field located under the Teleport's arc" |
| the hero in route 2 | 16 | `T04_H02` the Field Research Center |
| the helicopter (id 3) in route 2 | 16 | the same |

**The ownership and count tests**, each held by its own `df` flag:

| run finds | then | messages say |
|---|---|---|
| function 52 of id 1, the HQ, = 0 | objective 0, then 10 and 15 | `T04_I02` press Enter to access command mode; `T04_H01` Enter inside the warbot opens command mode, Escape leaves it, and "the mobile command center's camera only moves in sync with the warbot" |
| function 52 of `CLASS_BUILDING`\|2, the Factory, = 0 | objective 1, then 11 | `T04_I03` the Factory lacks parts; the Research Center develops them |
| function 52 of `CLASS_BUILDING`\|3, the Research Center, = 0 | objective 2, then 12 | `T04_I04` "a suitable tower is the only thing that you still need"; look for the Teleport meanwhile |
| function 52 of `CLASS_BUILDING`\|1, the Teleport, = 0 | objective **4**, then 14 | `T04_I06` "dive under the Teleport arc and head on home" |
| `Plr` robots ≥ `df5`, 4 | objective **3**, then 13 | `T04_I05` "rush this thing over to your plateau" |

- **Any of the four answering `ERROR`** sends `MISSION_FAILED`. As on Missions
  02 and 03, that cannot happen in play: a destroyed object answers 65534.
- **`Hero_Teleported`** sends `SYSTEM_MESSAGE` with `MISSION_COMPLETE`. A hero
  at the teleport's out place raises it
  ([27-ownership.md](27-ownership.md#teleport-out-0x4000--read)).
- **Messages 0–8** (`T04_T01`…`T04_T09`) are the briefing's lines, and 15–17 set
  `info_system`.
- **Any robot is the fourth** (*derived*). Function 31 counts every Type with
  `CLASS_ROBOT`, so the large flyer the briefing asks for is not tested for.

**The objectives: six lines, and the script completes five.** `mission.cfg`
names `objective4` twice. The game keeps both lines
([Objectives](#objectives-and-the-end-of-a-mission--read-and-measured)):

| index | line | completed by |
|---:|---|---|
| 0 | 1. Capture the mobile HQ | the HQ's owner |
| 1 | 2. Find and capture the big Factory on the plateau | the Factory's owner |
| 2 | 3. Find and capture the Research Center | its owner |
| 3 | 4. Develop and build a large flying warbot | the robot count |
| 4 | 5. Find and capture the Teleport | the Teleport's owner |
| 5 | 6. Get to the Teleport and leave the training grounds | nothing |

**So the completion test never passes** (*derived*): objective 5 stays open.
The mission is won by `Hero_Teleported` alone, whatever else is done.

A reader that keeps one value per key lists five objectives. It puts "6. Get
to the Teleport…" at index 4. The Teleport's capture would then complete it,
and with 0–3 already done, the mission would be won at the capture, before the
hero goes through. The readers here keep every line.

**The minds.** `Plr` has 3. The hero and the helicopter take two, if each holds
one as on Mission 03
([23-economy.md](23-economy.md#the-bot-limit-is-the-clans-mind-count--read-and-measured)).
Once the HQ is taken, the recording's factory panel still shows **1 free**
(*seen*, below), and then 0 when the build starts. So of the hero, the
helicopter and the HQ, one holds no mind. Which one is not established;
Mission 03's four free of seven, beside three placed units, points at the HQ
taken by Enter.

### Seen in a recording

Timings come from a 30 fps recording of the mission, played through at
960 × 720. Each line is the first frame that shows it, sampled every 1 to
3 s, or more finely where a time is given to a tenth.

| time | what |
|---|---|
| 0–69.3 s | the briefing: 22 waypoints, 68.1 s of camera |
| 69.4 s | the objectives screen: **six** objectives, each "in progress" |
| 76.4 s | the cockpit beside the HQ. The hero starts 65.7 m from it, so the recording is cut here |
| 78.2 s | the HQ's cockpit, "LWC-1 Comm. Center (no order)": captured and boarded |
| 78.6 s | command mode's camera and the icon column; `T04_H01` |
| 83 s | the helicopter "(searching)" on the Battle units page |
| 175 s | "Building is captured": the Factory |
| 176 s | `T04_I03`, objective 1 |
| 236 s | `T04_H02`, route 2 |
| 255 s | "Building is captured": the Research Center |
| 256 s | `T04_I04`, objective 2 |
| 272–276 s | the objectives screen: 1–3 complete, 4–6 in progress |
| about 300–312 s | the Research Center's screen, then "Research complete" |
| 344–347 s | the Factory's screen with an "LFW-X Warrior", an L-2f flyer with no guns: free minds 1, then 0 and 1% as it starts |
| 399 s | `T04_H03`, route 1 |
| 408.6 s | `T04_I05`, objective 3: the fourth robot, 62 s after the build started |
| 410.4 s | "Building is captured": the Teleport |
| 411.0 s | `T04_I06`, objective 4 |
| 416.8 s | the teleport's chamber |
| 421.4 s | "MISSION COMPLETE !" over the chamber's field, the HUD gone |
| 428 s | the campaign menu, *Teleport* done |

What follows from it:

- **The recording's player does not fly.** The hero walks from the Research
  Center to the Teleport. The large flyer is built, but the hero never boards
  it.
- **The win follows `Hero_Teleported`**, 4.6 s after the hero reaches the
  chamber, with objective 5 still open.

### For an engine

1. **Keep a repeated `mission.cfg` key** in the objective lists, so Mission 04
   has six objectives.
2. **Count robots after the placed units have joined** their clans, so `Init`
   reads 2 and objective 3 needs a fourth robot.
3. **Leave a mind free** for the build after the HQ is taken; the recording
   shows one.
4. **Answer function 52** by owner for the HQ, the three buildings and the
   teleport. **Answer function 32** for the hero and the helicopter.
5. **Win only on `Hero_Teleported`**: the teleport's out place, reached through
   its in place once the player holds it
   ([27-ownership.md](27-ownership.md#for-an-engine)).

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
- **A driven bot's loss** goes to `0x10062ff0`, the mode stack's rollback: from
  a bot the hero is put out beside it
  ([39-boarding.md](39-boarding.md#when-the-driven-bot-is-lost--read)).
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
   bot, and the player drives the bot from then on
   ([39-boarding.md](39-boarding.md)).

## Ambient sound — *read* in part

- **The theme.** `mission.cfg`'s `ambient_music_loop` `THEME` goes straight to
  the sound server's slot 2 at mission load (`iron3d.dll:0x1005e2e1`). It is a
  type-5 descriptor ([20-resources.md](20-resources.md)); whether that type is
  what makes it loop is not read.
- **The variations.** `ambient_music_variation`'s names are gathered as
  `DEFAULT_`, `DAY_` and `NIGHT_` + `VARIATION1`, `2`, … for as long as the
  key exists (`0x1005f8f1`). When one plays, and how day or night picks among
  them, is not read.

## Music: the CD's tracks — *read*, and *measured*

The music is not a sound resource. It is **audio CD tracks**, which the Steam
install ships as `MUSIC/Track02.ogg` to `Track10.ogg` (nine files) beside a
`winmm.dll` whose only string about them is `%s\Track%02d.ogg`: a CD track *n*
is `Track`*nn*`.ogg` (*measured*; the `winmm.dll` is third-party and not read).
Track 1, the data track, has no file.

**The player.** `iron3d.dll`'s `createSubsystems` (`0x1005b6d0`) makes a CD
player object (`0x1008e260`, one global at `0x1010b608`):

- it reads `Iron_3D.ini`'s `PLAY_CD_MUSIC` as an integer, and a 0 there turns
  every play into a no-op (`0x1008e2dd`–`0x1008e329`);
- it opens the CD through `Ngi32.dll`'s 3D sound (`0x1008e390`, its slot 9,
  `0x1000cc30`) with `FORCE_CD_SOUND` (`0x1005bb8c`), and keeps the track count
  and whether track 1 is audio (the CD object's slots 6 and 8, `0x1000ef30` and
  `0x1000f050`; `MCI_STATUS_NUMBER_OF_TRACKS`, and `MCI_CDA_STATUS_TYPE_TRACK`
  against `MCI_CDA_TRACK_AUDIO`);
- it reads `SFX_VOLUME` and `CD_VOLUME` as integers and hands each on × 0.01
  (`0x1005ba69`–`0x1005ba80`, `0x1005bb41`–`0x1005bb58`), which `Ngi32.dll` sets
  as that share of a mixer control's range (`0x1000cf5c`–`0x1000cfec`).

The install sets `PLAY_CD_MUSIC=1`, `SFX_VOLUME=12`, `CD_VOLUME=12` and
`FORCE_CD_SOUND=".\MUSIC\"` (*measured*).

**Playing a track** (`Ngi32.dll`'s CD object, vtable `0x1003178c`). It opens
MCI's `cdaudio` in tracks-minutes-seconds-frames (`0x1000ec50`). Play
(`0x1000ed60`) plays track index + 1 from its start to the next track's; its
flag 2 marks the track as looping, and it notes when the track will have run,
now plus the track's length (`0x1000ee25`). Each tick of the 3D sound
(`0x1000ca42`–`0x1000ca85`), 2 s past that time and once MCI's mode reads 2, a
looping track is played again from its start, and any other is marked
finished (`+0x3c`).

**Which track** (`0x1008e4d0`, with a loop flag). With one track that is audio,
track index 0; with two and track 1 not audio, index 1; with more, `rand()` over
the count until the pick differs from the track playing and is not index 0
unless track 1 is audio. So the install's CD plays a random one of tracks 2 to
10, never the same twice running. `rand()` is the C library's
(`0x100b47d0`), seeded once from `timeGetTime()` (`0x1000798f`).

**When** (`iron3d.dll`):

- **At a mission's load**, unless a briefing is running (state word 5), a random
  track (`0x1005e1ca`).
- **At a briefing's start** the CD stops (`0x10031431`), and the `briefing`
  object's `cd_track` in `mission.cfg`, when there is one, plays looping as a track
  index, so CD track `cd_track` + 1 (`0x10031500`; its argument 1 becomes the loop
  flag 2).
- **Every game frame outside a briefing**, when the player's track is marked
  finished, another random track (`0x1005ea34`–`0x1005ea4e`, no loop).

So a mission with no `cd_track` is silent through its briefing and starts a
random track on the first frame after it, and moves to another random track
2 s after each ends (*derived*). *Measured*: only `CAMPAIGN.01/Mission.01`
names a `cd_track` (3), and it is a briefing alone; `ui/shell_ctrls.cfg`'s `main_menu_track` is −1, "play
random track".

**Against Mission 01's recording** (*measured*): `Track07.ogg`'s first 20 s is
found in the recording's audio at 99.65 s (normalised correlation 0.44; no other
track above 0.07), the moment the briefing hands over. Twenty seconds on, the
recording is 9.8 dB under the file's own level over the same span, where a voice
sits 11.8 dB under its file: music and sounds come out alike, as the equal
`CD_VOLUME` and `SFX_VOLUME` say.

## Not established

- The clock unit of the behaviour takt that times the route reports.
- How a destroyed or captured unit leaves function 31's list.
- ~~Where a message's text is drawn and for how long.~~ Answered: in the
  message box at the top, for 20 seconds
  ([35-hud.md](35-hud.md#the-message-box--read-and-measured)).
- ~~What `info_system` changes on screen: it makes a history entry kind 4
  rather than 3.~~ Answered: the box's header, *from: Information assistant*
  rather than *from: Training assistant*
  ([35-hud.md](35-hud.md#the-message-box--read-and-measured)).
- ~~What the game shows after `MISSION_COMPLETE`.~~ A panel in place of the HUD,
  until Esc exits to the menus, which record the win
  ([After the outcome](#after-the-outcome--read-and-measured)). Still open:
  - what the shell's campaign branch (`0x10009c10`) and single-mission branch
    (`0x10009f40`) show, and so what leads to the next mission;
  - which parameter modes 3 and 4 are;
  - ~~the display's two scale queries that place the panel's text~~, answered
    in [35-hud.md](35-hud.md#how-the-radar-draws--read): the screen's width over
    640 and its height over 480;
  - who sends the game message 3 that sets the state word to 2.
- ~~Whether the hero keeps reporting its route while it sits inside a boarded
  bot.~~ It does, where the bot goes: in a recording of Mission 02 route 4's
  message plays while the hero flies the warbot over the lake
  ([Mission 02](#seen-in-a-recording)). How its position reaches the report
  is boarding's (39-boarding.md).
- Whether the `Mission` handler runs while a building's screen or the
  designer is up. The frame's gate is the level's state word at 5 or the
  pause byte `+0xe8`. Only the briefing writes 5 (`iron3d.dll:0x100a2a91`):
  the other eight writes of an immediate store 1 or 4, and all 39 calls of the
  setter (`0x100a4f90`) pass 1, 2, 3, 4 or 6 (*measured*). Whether those
  screens set the pause byte was not read.
- Mission 02's medusas: what the attack does with the figures and the circle
  the migrate task gives it, and how a flyer's migration point, which takes
  the pasture centre's height, meets its flying height.
- What the behaviour does with the message 6 it sends itself for each tactical
  areal it is in.
- The ambient variations' schedule.
- ~~A failure on the hero's death.~~ The game fails the mission itself when the
  player's clan's hero is lost (`iron3d.dll:0x10075619`,
  [After the outcome](#after-the-outcome--read-and-measured)), though
  `CLAN_HERO_KILLED` does nothing in this build
  ([21-briefing.md](21-briefing.md#messagescfg--the-in-mission-dialogue)) and
  Mission 01's script never fails. ~~What a driven bot's loss does
  (`0x10062ff0`) is not followed.~~ It rolls the mode stack back, which puts
  the hero out ([39-boarding.md](39-boarding.md#when-the-driven-bot-is-lost--read)).
