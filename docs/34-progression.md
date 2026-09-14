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
comes within 20 m of the current pasture's centre, and anything that hurts it
(*derived*). The briefing's `T02_T03` says as much: they attack only those who
trespass on their territory.

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
